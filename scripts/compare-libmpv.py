#!/usr/bin/env python3
"""Compare two libmpv DLL builds, ignoring the fields that record build time.

Exits 0 when the files are identical apart from the COFF and export/debug
directory timestamps and the PE checksum, and 1 otherwise.

    python scripts/compare-libmpv.py <published libmpv-2.dll> <rebuilt libmpv-2.dll>
"""
import struct
import sys
from pathlib import Path


def build_time_fields(data):
    """Return {offset: label} for the 4-byte fields that change with build time."""
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    if data[pe:pe + 4] != b"PE\0\0":
        raise ValueError("not a PE file")
    coff = pe + 4
    opt = coff + 20
    data_dirs = opt + (112 if struct.unpack_from("<H", data, opt)[0] == 0x20B else 96)
    count = struct.unpack_from("<H", data, coff + 2)[0]
    sections_at = opt + struct.unpack_from("<H", data, coff + 16)[0]
    sections = [struct.unpack_from("<IIII", data, sections_at + i * 40 + 8) for i in range(count)]

    def offset(rva):
        for vsize, vaddr, rawsize, rawptr in sections:
            if vaddr <= rva < vaddr + max(vsize, rawsize):
                return rawptr + rva - vaddr
        return None

    fields = {coff + 4: "COFF timestamp", opt + 64: "PE checksum"}
    export_rva = struct.unpack_from("<I", data, data_dirs)[0]
    if export_rva and offset(export_rva) is not None:
        fields[offset(export_rva) + 4] = "export directory timestamp"
    debug_rva, debug_size = struct.unpack_from("<II", data, data_dirs + 6 * 8)
    if debug_rva and offset(debug_rva) is not None:
        for i in range(debug_size // 28):
            fields[offset(debug_rva) + i * 28 + 4] = f"debug directory {i} timestamp"
    return fields


def main():
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    a, b = (Path(p).read_bytes() for p in sys.argv[1:])
    if len(a) != len(b):
        print(f"DIFFERENT: sizes {len(a):,} and {len(b):,} bytes")
        return 1
    fields = build_time_fields(a)
    ignored = {start + i for start in fields for i in range(4)}
    unexplained = 0
    chunk = 1 << 20
    for start in range(0, len(a), chunk):
        if a[start:start + chunk] != b[start:start + chunk]:
            unexplained += sum(1 for i in range(start, min(start + chunk, len(a)))
                               if a[i] != b[i] and i not in ignored)
    changed = [label for start, label in sorted(fields.items()) if a[start:start + 4] != b[start:start + 4]]
    if unexplained:
        print(f"DIFFERENT: {unexplained:,} bytes differ outside build-time fields")
        return 1
    print("MATCH: identical" + (f" apart from {', '.join(changed)}" if changed else ""))
    return 0


if __name__ == "__main__":
    sys.exit(main())
