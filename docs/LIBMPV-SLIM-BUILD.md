# Video runtime build notes

## Current runtime requirement (2026-10-05)

`npm run setup:mpv` now installs the pinned upstream development package
`mpv-dev-x86_64-20261005-git-c152964208.7z` from
[`shinchiro/mpv-winbuild-cmake`, release `20261005`](https://github.com/shinchiro/mpv-winbuild-cmake/releases/tag/20261005).
Its archive SHA-256 is
`7D9DD506DA6B77E6054F56052F7423D9F7EED764F85154A6221925ABA37DBFB2`.
Extraction requires 7-Zip or NanaZip. The setup script finds `7z`/`7za` on PATH
or 7-Zip in Program Files. It installs into `.vendor/mpv-dev-filters`, leaving
the older `.vendor/mpv-dev` directory intact. Use `-SkipBundleStaging` when
preparing this dependency alongside an existing build without replacing its
staged DLL. Normal setup stages it for packaging.

The video FPS cap requires FFmpeg's `select` filter, which the slim build below
removed. The cap keeps the first frame in each timestamp interval, preserves
the original timestamps, and adjusts its interval for playback speed. It does
not add frames to videos already below the cap. Changing or removing the cap
replaces/removes only the `wallscape-fps` filter. `display-fps-override` is a
display timing calibration option and is no longer used as a cap.

Filtered video frames lose their final-frame duration in this libmpv version.
The wallpaper thread therefore holds capped videos at EOF until their original
media duration has elapsed, freezes that clock while paused, and adjusts it on
speed changes. Short loops share the same frame-rate clock; loops without an
eligible output frame retain the previous image. Filter replacement and seek
run as one mpv command list to avoid rendering an extra frame from the old loop.
Videos with unavailable duration reject a cap rather than change loop speed.

The full DLL is 120,863,744 bytes, compared with the old slim DLL's 32,507,918
bytes. This increases the runtime by 88,355,826 bytes; the download is 31,530,655
bytes. The upstream COFF import archive is also installed as `mpv.lib` for the
MSVC linker. Restoring a small runtime will require a new reproducible slim
build with `--enable-filter=select`, plus updated archive checksums. Hardware
decoding remains enabled; the timestamp-only filter does not request software
pixel conversion. Headless frame-count tests cover filtering, not D3D11 output.
