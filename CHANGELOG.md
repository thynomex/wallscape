# Changelog

Notable changes to Wallscape are documented here. The project follows [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.3.0] - 2026-10-31

- The window is now the card itself: the transparent margin around it is gone, so clicks next to the window reach the desktop, snapped windows have no gap, and the resize edge is the visible edge. A maximized window also drops its rounded corners.
- Detail view speed and fit controls now change only the displays playing that wallpaper and no longer overwrite the defaults in Settings. They appear whenever the wallpaper is playing on any display, including a second one.
- "View Wallpaper" on Home opens the detail view instead of applying the wallpaper to every display.
- Setting a Wallhaven or MotionBGS wallpaper from its detail view shows download progress, and a failed download no longer hides the Discover results.
- Fixed cards and carousel thumbnails not lifting on hover, and the navigation and hero glass not blurring the image behind it.
- The detail view and Settings keep keyboard focus inside them and return it on close. Detail shortcuts no longer fire while the display picker or a button has focus, and the favorite button on cards is a separate, visible control.
- Confirmations use native Windows dialogs.
- The Library filter is one row (All, Favorites, collections) and is kept, along with "Show more", when switching views. Cancelling a collection delete keeps it selected, and failed creates or saves keep what you typed.
- Import progress shows in the Library, and the Upload button is now Import.
- History entries can be clicked to re-apply them, and the popover closes on Escape or an outside click.
- Toasts are announced to screen readers, can be dismissed, pause while hovered, and errors stay for 6 seconds. The footer error clears with its toast or can be dismissed.
- Settings shows "Saved" next to the changed control instead of a toast per click, and only that control is disabled while it saves.
- Home's logo, hero text, and sections share one left edge, and the navigation pills are centered. Loading placeholders use the same columns as the cards, and the carousel scrolls the selected thumbnail into view.
- Metadata is consistent across the app: no "0fps" or "0×0", MotionBGS previews no longer claim 60fps, and Wallhaven titles match between cards and the detail view.

## [0.2.1] - 2026-10-30

- Replaced the bundled libmpv runtime with a 37.5 MB slim build (down from 120.9 MB) made from pinned sources. It keeps the FFmpeg `select` filter for the FPS cap, D3D11 hardware decoding, and the containers and codecs Wallscape imports. The installer drops from 55.1 MB to 22.5 MB.
- Fixed short views scrolling slightly and hiding part of the status bar.
- Fixed the selected Home carousel thumbnail's outline being cut off at the edge of the carousel.

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
