# Third-party notices

Wallscape is distributed under the MIT License, but it depends on third-party software distributed under separate licenses. Copyright in those projects remains with their respective owners.

## Bundled video runtime

Release 0.2.1 installers include `libmpv-2.dll` from the slim package `mpv-dev-slim-x86_64-c152964208.zip`, published at the [libmpv-slim-20261024 release](https://github.com/thynomex/wallscape/releases/tag/libmpv-slim-20261024) with SHA-256 `B22D705DD7CA51EC0505F6F1FB50AA069E079F5A3D390E41C95D241BFF40117B`. It is built by `scripts/build-libmpv-slim.ps1` from [mpv](https://github.com/mpv-player/mpv) `c1529642089b`, [FFmpeg](https://ffmpeg.org/) `a35c8799920a`, and [libplacebo](https://github.com/haasn/libplacebo) `92b5ac6db79f`, and statically links them with dav1d, libass, shaderc, SPIRV-Cross, glslang, zlib, and their MSYS2 dependencies at the versions in `scripts/libmpv-slim-toolchain.lock`.

mpv is GPLv2+ with parts under LGPL, FFmpeg is configured without `--enable-gpl` and is LGPLv2.1+, and libplacebo is LGPLv2.1+, so the combined DLL is distributed under the GPL. The package includes these license texts and its build provenance. Build requirements, enabled components, and runtime behavior are recorded in `docs/LIBMPV-SLIM-BUILD.md`. The runtime must be distributed with the applicable upstream license texts and corresponding source information.

Release 0.2.0 used the upstream Windows package `mpv-dev-x86_64-20261005-git-c152964208.7z` from [mpv-winbuild-cmake release 20261005](https://github.com/shinchiro/mpv-winbuild-cmake/releases/tag/20261005), which `npm run setup:mpv -- -FullRuntime` still installs.

## Rust and JavaScript dependencies

Rust crates are locked in `src-tauri/Cargo.lock`; JavaScript packages are locked in `package-lock.json`. Each dependency retains its own license. Release builds should generate and archive a dependency license report or software bill of materials.

## Content services

Wallscape can access [Wallhaven](https://wallhaven.cc/) and [MotionBGS](https://motionbgs.com/) at the user's request. These projects are not affiliated with or endorsed by Wallscape. Wallpaper copyrights and service terms remain with their respective owners.

Wallscape does not grant permission to redistribute media downloaded through third-party services. Users are responsible for complying with applicable licenses and terms.
