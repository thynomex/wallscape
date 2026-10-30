# Video runtime build notes

## Current runtime (2026-10-24)

`npm run setup:mpv` installs the slim development package
`mpv-dev-slim-x86_64-c152964208.zip` (14,450,282 bytes, SHA-256
`B22D705DD7CA51EC0505F6F1FB50AA069E079F5A3D390E41C95D241BFF40117B`), published
at the [`libmpv-slim-20261024`](https://github.com/thynomex/wallscape/releases/tag/libmpv-slim-20261024)
release. Its `libmpv-2.dll` is 37,483,190 bytes with SHA-256
`BD434862360E546B07CADC773A9F3CC435CD28D4064C6B91EF1A5621B4A4804A`. It is
built from the same mpv (`c152964208`) and FFmpeg (`a35c8799`) revisions as the
previous upstream package, so playback behavior matches; only the compiled-in
feature set differs.

| | Slim (current) | Upstream package | Earlier slim (2026-06-22) |
| --- | --- | --- | --- |
| `libmpv-2.dll` | 37,483,190 | 120,863,744 | 32,507,918 |
| Dev archive | 14,450,282 | 31,530,655 | 13,518,356 |
| `mpv.lib` | 50,062 | 53,308 | 13,206 |
| `select` filter (FPS cap) | yes | yes | **no** |
| D3D11 hardware decode | yes | yes | yes |
| Installer (MSI) | 22,470,656 | 55,091,200 | 20,463,616 |

The DLL is 69.0% smaller than the upstream runtime and 15.3% larger than the
2026-06-22 slim build. The growth buys back the FFmpeg `select` filter that the
FPS cap needs, plus the GPU-shared D3D11 hwaccels that keep `hwdec=auto` on
`d3d11va` instead of falling back to `dxva2-copy`.

The archive installs into `.vendor/mpv-dev-slim`, and `src-tauri/build.rs`
resolves that directory first, then `.vendor/mpv-dev-filters`, then
`.vendor/mpv-dev`, then `C:\mpv-dev`. Setup takes the archive from
`.vendor/cache` or downloads the public release asset, and verifies its SHA-256
before extracting. The slim package is a
zip, so 7-Zip is only needed for the upstream package.

If the slim archive cannot be obtained, setup prints a warning and installs the
upstream package instead, which produces a ~55 MB MSI. `-FullRuntime` forces the
upstream package and removes `.vendor/mpv-dev-slim`, because `build.rs` would
otherwise keep resolving and restaging the slim DLL. A checksum mismatch, an
explicit `-Source`, or `-FullRuntime` never falls back silently.

### FPS cap and frame timing

The video FPS cap requires FFmpeg's `select` filter. The cap keeps the first
frame in each timestamp interval, preserves the original timestamps, and adjusts
its interval for playback speed. It does not add frames to videos already below
the cap. Changing or removing the cap replaces/removes only the `wallscape-fps`
filter. `display-fps-override` is a display timing calibration option and is not
used as a cap.

Filtered video frames lose their final-frame duration in this libmpv version.
The wallpaper thread therefore holds capped videos at EOF until their original
media duration has elapsed, freezes that clock while paused, and adjusts it on
speed changes. Short loops share the same frame-rate clock; loops without an
eligible output frame retain the previous image. Filter replacement and seek
run as one mpv command list to avoid rendering an extra frame from the old loop.
Videos with unavailable duration reject a cap rather than change loop speed.

## Building the current runtime

```powershell
# One-time: MSYS2 UCRT64 toolchain and pinned sources into an isolated workspace
powershell -ExecutionPolicy Bypass -File scripts\build-libmpv-slim.ps1 -Stage prepare

# Build libplacebo, FFmpeg, and libmpv; package the dev archive
powershell -ExecutionPolicy Bypass -File scripts\build-libmpv-slim.ps1

# Make the result the one `npm run setup:mpv` installs
Copy-Item <workspace>\candidate\mpv-dev-slim-x86_64-c152964208.zip .vendor\cache\
npm run setup:mpv
```

The workspace defaults to `%TEMP%\wallscape-libmpv-slim`; pass `-Root` to keep
it elsewhere. The script:

- Downloads MSYS2's self-extracting base archive and verifies it against
  MSYS2's published SHA-256. It needs no `tar` or `xz`.
- Installs the UCRT64 toolchain (GCC, binutils, Meson, shaderc, SPIRV-Cross,
  dav1d, libass, and their dependencies) at the exact versions in
  `scripts/libmpv-slim-toolchain.lock`, downloaded from MSYS2's package
  archive. The lock was recorded from the validated build's own package list;
  all 80 versions were still downloadable on 2026-10-30.
- Stops if an existing workspace's UCRT64 packages differ from the lock.
  `-AllowToolchainDrift` builds anyway, and the result is then a new candidate
  that needs the full validation below.
- Clones mpv, FFmpeg, and libplacebo at the pinned revisions and writes the
  candidate into `<workspace>\candidate\` with its archive and `.sha256` files.

Nothing outside that workspace is modified, and the script does not touch the
release assets or the staged `src-tauri/bundle` DLL.

### Feature selection

FFmpeg is configured with `--disable-everything` plus explicit lists, so every
retained component is named in `scripts/build-libmpv-slim.sh`:

- Demuxers: MP4/MOV, Matroska/WebM, AVI, ASF/WMV, FLV, MPEG-TS/PS, raw video,
  GIF, image2, and yuv4mpegpipe (tests).
- Decoders: H.264, HEVC, VP8, VP9, AV1 (native and dav1d), MPEG-1/2/4, MS-MPEG4,
  WMV1/2/3, VC-1, MJPEG, FLV, GIF, PNG, rawvideo, plus edit-friendly codecs
  (ProRes, DNxHD, FFV1, HuffYUV, Ut Video, QuickTime RLE, MS RLE).
- Hardware: `d3d11va`, `dxva2`, and both the `*_d3d11va` (system-memory) and
  `*_d3d11va2` (GPU-surface) hwaccels. The `2` variants are mandatory: without
  them `hwdec=auto` degrades to `dxva2-copy`.
- Filters: `select` (cap), `metadata` and `showinfo` (frame-count tests),
  `format`, `scale`, `null`, `hwdownload`, `hwupload`.
- Protocol: `file` only. No network, capture devices, encoders, muxers, audio
  output, disc/archive input, Lua/JavaScript, Vulkan, OpenGL, or libarchive.

mpv is built with `-Dlibmpv=true -Dcplayer=false`, D3D11 output, D3D11 hardware
acceleration, shaderc, SPIRV-Cross, zlib, and win32 threads. libplacebo is built
with D3D11 and shaderc only.

MSYS2's `shaderc_combined` archive references SPIRV-Tools and glslang symbols but
its `.pc` file lists only itself. The build script writes `shaderc.pc`,
`shaderc_combined.pc`, and `shaderc_static.pc` under the local prefix with the
full `--start-group` link closure, and wipes Meson's dependency cache before
configuring mpv so the corrected arguments take effect.

### Validation

The video checks need synthetic media. Generate it with an FFmpeg build that
includes libx264, libx265, libvpx, and libaom, then point the tests at it:

```powershell
scripts\make-libmpv-fixtures.ps1 -OutputDir <fixtures>
$env:WALLSCAPE_MPV_FIXTURES = "<fixtures>"
```

Run from `src-tauri/`; `build.rs` selects the runtime as described above:

- `cargo test` — 158 passed, 3 ignored.
- `cargo test wallpaper::video::validation::compressed_container_codec_matrix_and_invalid_media -- --ignored`
  — 14 container/codec fixtures × capped and uncapped, plus the unsupported-media
  error path.
- `WALLSCAPE_DESKTOP_SMOKE=1 cargo test wallpaper::video::validation::desktop_hardware_decode_d3d11_and_monitor_replacement -- --ignored`
  — real WorkerW playback with `hwdec=d3d11va`, `vo=gpu`,
  `current-gpu-context=d3d11`, capped and uncapped, fit/fill/stretch, pause and
  2× speed, per-monitor replacement, failed replacement recovery, clean
  shutdown, and the original wallpaper restored afterward. It sets a video
  wallpaper for a few seconds, so run it in an interactive session.

`scripts/inspect-libmpv.py` records a DLL's size, hash, PE machine, imports, and
exports as JSON for comparison with the previous runtime.

On 2026-10-24 the final DLL (`BD434862…`) passed all three in an isolated source
copy. On 2026-10-30 `cargo test`, the codec matrix, `cargo fmt --check`, and
`cargo clippy --locked --all-targets -- -D warnings` passed again in the main
checkout with `build.rs` resolving `.vendor/mpv-dev-slim` to that DLL. Host:
Ryzen 7 7800X3D, GeForce RTX 2080 Ti, Windows 10 19045, 2560×1440.

The MSI and main-checkout results recorded on 2026-10-24 came from an earlier
candidate (`C46D0898…`, the same size but different code). On 2026-10-30 the
installer was rebuilt from a fresh copy of the source tree whose `npm run
setup:mpv` downloaded the published archive. The 22,470,656-byte MSI contains a
`libmpv-2.dll` matching `BD434862…` by SHA-256. Its executable imports only
Windows system DLLs and the bundled `libmpv-2.dll`; the DLL imports only Windows
system DLLs. The MSI was inspected, not installed on a clean machine, because
Windows Sandbox is unavailable on the build host. The desktop smoke test was not
repeated on 2026-10-30 because Wallscape was running on that desktop; the DLL is
unchanged from its 2026-10-24 pass.

### Reproducibility

The build script sets `SOURCE_DATE_EPOCH` to the pinned mpv commit time, so
binutils writes the same PE timestamps on every build. A linker timestamp flag
is not used: mpv embeds its Meson options in the DLL, and changing them shifts
3.7 MB of code and data.

On 2026-10-30, two consecutive rebuilds of the mpv stage produced byte-identical
DLLs. Each matches the published `BD434862…` except for its COFF and export
timestamps and PE checksum, which record the published build's link time. Check
a rebuild with:

```powershell
python scripts\compare-libmpv.py .vendor\mpv-dev-slim\libmpv-2.dll <workspace>\candidate\mpv-dev-slim-x86_64-c152964208\libmpv-2.dll
```

`-Stage prepare` in an empty workspace downloaded MSYS2, installed exactly the 80
locked UCRT64 package versions, and checked out the pinned sources.

The DLL contains the workspace path (in mpv's embedded options), so rebuild in a
workspace at the same path to compare byte for byte. The dev archive is not
byte-reproducible because `zip` records file times; compare the DLL instead.

### Rollback

```powershell
npm run setup:mpv -- -FullRuntime   # upstream 120 MB runtime, same checksum guard
```

This installs `.vendor/mpv-dev-filters`, stages its DLL, and removes
`.vendor/mpv-dev-slim` so the next build uses the upstream runtime. Run
`npm run setup:mpv` again to return to the slim runtime. `MPV_DEV_DIR` overrides
the resolution order for one-off comparisons. Keep the `libmpv-slim-20261024`
release published even after a rollback, because commits that pin its checksum
download it during setup.

## History

Releases 0.1.4 to 0.1.6 used a smaller slim build from 2026-06-22 that omitted
the `select` filter, and 0.2.0 used the upstream package to regain the FPS cap.
`docs/libmpv-slim-mpv-winbuild-cmake.patch` records the 2026-06-22 recipe for
reference only; `scripts/build-libmpv-slim.ps1` replaces it.
