# Third-party notices

Wallscape is distributed under the MIT License, but it depends on third-party software distributed under separate licenses. Copyright in those projects remains with their respective owners.

## Bundled video runtime

Release installers include a custom Windows build of [mpv/libmpv](https://github.com/mpv-player/mpv) and its enabled dependencies, including FFmpeg, libplacebo, libass, shaderc, and SPIRV-Cross. mpv and FFmpeg licensing depends on their build configuration. The Wallscape runtime build must be distributed with the applicable upstream license texts and corresponding source information.

The reproducible customization is recorded in `docs/libmpv-slim-mpv-winbuild-cmake.patch`. Release maintainers must verify the exact upstream revisions and include the complete notices/source offer with every binary release.

## Rust and JavaScript dependencies

Rust crates are locked in `src-tauri/Cargo.lock`; JavaScript packages are locked in `package-lock.json`. Each dependency retains its own license. Release builds should generate and archive a dependency license report or software bill of materials.

## Content services

Wallscape can access [Wallhaven](https://wallhaven.cc/) and [MotionBGS](https://motionbgs.com/) at the user's request. These projects are not affiliated with or endorsed by Wallscape. Wallpaper copyrights and service terms remain with their respective owners.

Wallscape does not grant permission to redistribute media downloaded through third-party services. Users are responsible for complying with applicable licenses and terms.
