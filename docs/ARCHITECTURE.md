# Architecture

Wallscape is a Windows desktop application built with Tauri 2, Rust, SvelteKit, and SQLite.

## Frontend

The Svelte frontend under `src/` provides the application views, components, stores, and typed wrappers for Tauri commands. `src/routes/+page.svelte` coordinates top-level application behavior; reusable UI and workflows live under `src/lib/`.

## Backend

The Rust application under `src-tauri/` owns filesystem access, SQLite persistence, downloads, settings, monitor detection, Windows integration, and wallpaper playback. Tauri command adapters live in `src-tauri/src/commands/` and delegate to focused modules.

## Wallpaper runtime

Static wallpapers use Windows wallpaper APIs. Video wallpapers use libmpv rendered into a native child window attached to the Windows desktop host. The runtime runs on a dedicated Win32 thread and manages playback, placement, pause state, and cleanup.

## Data

Wallpaper metadata, preferences, collections, saved filters, assignments, and history are stored locally in SQLite. Imported media and downloaded wallpapers remain on the user's machine.

## External services

Wallhaven and MotionBGS requests are user initiated. Results are normalized by the Rust backend before being exposed to the frontend. These integrations are optional and must remain isolated from local-library functionality.
