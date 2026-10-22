# Changelog

Notable changes to Wallscape are documented here. The project follows [Semantic Versioning](https://semver.org/).

## [Unreleased]

## 0.2.0

- Improved video FPS limiting and short-loop timing across pause, speed, and fit changes.
- Kept active playback controls separate from defaults saved in Settings.
- Preserved new imports and downloads when favorites or rotation change during a library refresh.
- Streamed downloads to temporary files with bounded concurrency, cancellation cleanup, and startup recovery for interrupted downloads.
- Reduced background polling and intermediate download-progress updates.
- Made player replacement, pause transitions, settings persistence, and inaccessible-file cleanup more reliable.
- Repaired launch-at-startup registrations when the application executable moves.
- Disabled diagnostic uploads and removed the bundled upload command and credential.
- Added frontend race regressions, runtime tests, generated-binding drift checks, and Windows CI.
- Updated vulnerable frontend build dependencies within their existing version ranges.
- Limited local log retention and reduced release-build logging noise.

The bundled libmpv runtime now includes frame filters needed for FPS limiting. This increases installer size compared with earlier slim-runtime releases.

## [0.1.5] - 2026-07-11

- Updated the maintained Tauri, Svelte, SvelteKit, Vite, and validation toolchain.
- Updated `anyhow` and `tauri-build` to their latest compatible patch releases.
- Enabled GitHub dependency vulnerability alerts.
- Kept major TypeScript, Node type, scraper, windows-rs, and rusqlite migrations out of this release pending dedicated compatibility reviews.

## [0.1.4] - 2026-07-11

- Added image and video wallpaper management.
- Added local imports, collections, favorites, history, rotation, and restore flows.
- Added Wallhaven and MotionBGS discovery integrations.
- Added multi-monitor controls, tray behavior, startup settings, and resource-use options.
