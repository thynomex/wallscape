#!/usr/bin/env python3
"""Inspect a Windows libmpv DLL without third-party Python packages.

Run with matching-bitness Python on Windows for runtime queries. PE and hash
inspection also works on other platforms. Does not play media or change files
other than the requested JSON output.
"""
import argparse
import ctypes as C
import hashlib
import json
import os
from pathlib import Path
import platform
import struct


def fingerprint(path):
    path = Path(path).resolve()
    digest = hashlib.sha256()
    with path.open('rb') as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b''):
            digest.update(chunk)
    return {'path': str(path), 'bytes': path.stat().st_size, 'sha256': digest.hexdigest()}


def inspect_pe(path):
    data = Path(path).read_bytes()
    def u16(offset): return struct.unpack_from('<H', data, offset)[0]
    def u32(offset): return struct.unpack_from('<I', data, offset)[0]
    pe = u32(0x3c)
    if data[:2] != b'MZ' or data[pe:pe + 4] != b'PE\0\0':
        raise ValueError('Not a PE file')
    coff = pe + 4
    optional = coff + 20
    magic = u16(optional)
    if magic not in (0x10b, 0x20b):
        raise ValueError('Unsupported PE optional header')
    directory = optional + (112 if magic == 0x20b else 96)
    sections = []
    section_table = optional + u16(coff + 16)
    for index in range(u16(coff + 2)):
        offset = section_table + index * 40
        sections.append((u32(offset + 12), max(u32(offset + 8), u32(offset + 16)), u32(offset + 20)))
    def rva(value):
        for start, size, raw in sections:
            if start <= value < start + size:
                return raw + value - start
        if value < u32(optional + 60): return value
        raise ValueError(f'Unmapped RVA {value:#x}')
    def string(offset):
        return data[offset:data.index(b'\0', offset)].decode('ascii', errors='replace')
    imports = []
    import_rva = u32(directory + 8)
    if import_rva:
        offset = rva(import_rva)
        while any(struct.unpack_from('<IIIII', data, offset)):
            imports.append(string(rva(u32(offset + 12))))
            offset += 20
    exports = []
    export_rva = u32(directory)
    ordinal_exports = 0
    if export_rva:
        offset = rva(export_rva)
        names = u32(offset + 24)
        ordinal_exports = u32(offset + 20) - names
        table = rva(u32(offset + 32))
        exports = [string(rva(u32(table + index * 4))) for index in range(names)]
    machine = u16(coff)
    return {'machine': {0x8664: 'AMD64', 0x14c: 'I386', 0xaa64: 'ARM64'}.get(machine, hex(machine)),
            'optional_header': 'PE32+' if magic == 0x20b else 'PE32',
            'imports': sorted(imports, key=str.lower), 'exports': sorted(exports),
            'ordinal_only_export_count': ordinal_exports}


class Node(C.Structure): pass
class NodeList(C.Structure): pass
class NodeValue(C.Union):
    _fields_ = [('string', C.c_char_p), ('flag', C.c_int), ('int64', C.c_int64),
                ('double', C.c_double), ('list', C.POINTER(NodeList)), ('pointer', C.c_void_p)]
Node._fields_ = [('u', NodeValue), ('format', C.c_int)]
NodeList._fields_ = [('num', C.c_int), ('values', C.POINTER(Node)), ('keys', C.POINTER(C.c_char_p))]


def decode_node(node):
    fmt = node.format
    if fmt == 0: return None
    if fmt == 1: return node.u.string.decode('utf-8', errors='replace') if node.u.string else None
    if fmt == 3: return bool(node.u.flag)
    if fmt == 4: return node.u.int64
    if fmt == 5: return node.u.double
    if fmt in (7, 8):
        items = node.u.list.contents
        values = [decode_node(items.values[index]) for index in range(items.num)]
        if fmt == 7: return values
        return {items.keys[index].decode('utf-8', errors='replace'): value for index, value in enumerate(values)}
    return {'unhandled_mpv_format': fmt}


def query_runtime(path):
    if os.name != 'nt': return {'error': 'Runtime queries require Windows'}
    lib = C.CDLL(str(Path(path).resolve()))
    def bind(name, result, *args):
        function = getattr(lib, name)
        function.restype = result
        function.argtypes = list(args)
        return function
    version = bind('mpv_client_api_version', C.c_ulong)()
    create = bind('mpv_create', C.c_void_p)
    set_option = bind('mpv_set_option_string', C.c_int, C.c_void_p, C.c_char_p, C.c_char_p)
    initialize = bind('mpv_initialize', C.c_int, C.c_void_p)
    get = bind('mpv_get_property', C.c_int, C.c_void_p, C.c_char_p, C.c_int, C.c_void_p)
    free = bind('mpv_free_node_contents', None, C.POINTER(Node))
    destroy = bind('mpv_terminate_destroy', None, C.c_void_p)
    error = bind('mpv_error_string', C.c_char_p, C.c_int)
    handle = create()
    if not handle: raise RuntimeError('mpv_create returned NULL')
    result = {'client_api_version': {'raw': version, 'major': version >> 16, 'minor': version & 0xffff},
              'options': {}, 'properties': {}}
    try:
        for key, value in [('config', 'no'), ('vo', 'null'), ('audio', 'no'), ('terminal', 'no'), ('idle', 'yes')]:
            code = set_option(handle, key.encode(), value.encode())
            result['options'][key] = {'value': value, 'status': code}
        status = initialize(handle)
        result['initialize_status'] = status
        if status < 0:
            result['error'] = error(status).decode()
            return result
        for name in ['mpv-version', 'ffmpeg-version', 'ffmpeg-configuration', 'libass-version',
                     'mpv-configuration', 'options', 'property-list', 'decoder-list', 'demuxer-lavf-list',
                     'protocol-list', 'vf', 'hwdec', 'vo', 'current-vo',
                     'option-info/vf', 'option-info/vo', 'option-info/hwdec', 'option-info/gpu-api']:
            node = Node()
            code = get(handle, name.encode(), 6, C.byref(node))
            if code >= 0:
                try: result['properties'][name] = decode_node(node)
                finally: free(C.byref(node))
            else:
                result['properties'][name] = {'error_code': code, 'error': error(code).decode()}
    finally:
        destroy(handle)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--dll', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--artifact', action='append', default=[], type=Path,
                        help='Additional file to hash, repeatable')
    parser.add_argument('--skip-runtime', action='store_true')
    args = parser.parse_args()
    report = {'schema_version': 1, 'python': platform.python_version(),
              'dll': fingerprint(args.dll), 'pe': inspect_pe(args.dll),
              'artifacts': [fingerprint(path) for path in args.artifact]}
    if not args.skip_runtime:
        try: report['runtime'] = query_runtime(args.dll)
        except Exception as exc: report['runtime'] = {'error': f'{type(exc).__name__}: {exc}'}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    print(f"{args.output}: {report['dll']['bytes']} bytes, SHA256 {report['dll']['sha256']}")
    if 'runtime' in report and 'error' in report['runtime']:
        print('Runtime query failed: ' + report['runtime']['error'])
        return 1
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
