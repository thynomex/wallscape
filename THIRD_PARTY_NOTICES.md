# Third-party notices

Wallscape is distributed under the MIT License, but it depends on third-party software distributed under separate licenses. Copyright in those projects remains with their respective owners.

## Bundled video runtime

Release 0.2.0 installers include [mpv/libmpv](https://github.com/mpv-player/mpv) from the upstream Windows package `mpv-dev-x86_64-20261005-git-c152964208.7z`, distributed by [mpv-winbuild-cmake release 20261005](https://github.com/shinchiro/mpv-winbuild-cmake/releases/tag/20261005). Its enabled dependencies include FFmpeg and other upstream libraries. The package SHA-256 is `7D9DD506DA6B77E6054F56052F7423D9F7EED764F85154A6221925ABA37DBFB2`. mpv and FFmpeg licensing depends on their build configuration.

The current package provenance, build requirements, and runtime behavior are recorded in `docs/LIBMPV-SLIM-BUILD.md`. The earlier slim-runtime customization remains recorded in `docs/libmpv-slim-mpv-winbuild-cmake.patch` for reference. The runtime must be distributed with the applicable upstream license texts and corresponding source information.

## Rust and JavaScript dependencies

Rust crates are locked in `src-tauri/Cargo.lock`; JavaScript packages are locked in `package-lock.json`. Each dependency retains its own license. Release builds should generate and archive a dependency license report or software bill of materials.

## Content services

Wallscape can access [Wallhaven](https://wallhaven.cc/) and [MotionBGS](https://motionbgs.com/) at the user's request. These projects are not affiliated with or endorsed by Wallscape. Wallpaper copyrights and service terms remain with their respective owners.

Wallscape does not grant permission to redistribute media downloaded through third-party services. Users are responsible for complying with applicable licenses and terms.
