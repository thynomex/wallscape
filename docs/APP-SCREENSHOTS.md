# App screenshots

The README images render Wallscape's existing Svelte UI with a curated landscape
library. The capture uses an isolated Chromium browser and sample app data,
including two displays. It does not run the native wallpaper engine or read your
installed library. These images show the interface, not a native playback test.

## Regenerate the images

Run once from the repository root:

```powershell
npm install
npm --prefix scripts/screenshots ci
npm --prefix scripts/screenshots run install-browser
```

Then capture all seven images:

```powershell
npm run screenshots
```

The screenshot tools have their own locked dependencies. They do not add a
browser or image processor to the app's production bundle. Installation needs
internet access. Capture uses the checked-in artwork and blocks external browser
requests.

| Image | Export size | What it shows |
| --- | --- | --- |
| `docs/images/home.webp` | 2400 × 1248 | Featured landscape and six wallpaper previews |
| `docs/images/library.webp` | 2400 × 1800 | Library, collections, favorites, and rotation controls |
| `docs/images/wallpaper-detail.webp` | 2400 × 1500 | Wallpaper preview and target display selector |
| `docs/images/discover.webp` | 2400 wide, fits content | Wallhaven search, filters, results, and pager |
| `docs/images/discover-motionbgs.webp` | 2400 wide, fits content | MotionBGS feed with 4K/HD quality selectors |
| `docs/images/explore.webp` | 2400 wide, fits content | Library search for "Mountains" |
| `docs/images/settings.webp` | 2400 × 1500 | Settings panel over Explore, top of the list |

Discover results reuse the six local images with sample Wallhaven and MotionBGS
IDs, counts, and file sizes. Capture never contacts either service.

PNG originals are written to `scripts/screenshots/.output/`, which Git ignores.
Commit the WebP files for the README. The PNGs are useful for further editing or
exporting to another site.

## Capture settings

- 1600-pixel-wide app viewport, rendered at twice the pixel density. Each view
  gets enough height to show its controls without cutting through a row.
  Discover and Explore grow the viewport until the whole view fits.
- 3200-pixel-wide PNG originals and 2400-pixel-wide WebP exports under 1 MiB each.
- Dark appearance, reduced motion, English labels, and UTC.
- Existing window frame and layout, with a neutral background behind transparent pixels.
- Fonts and visible images finish loading before capture. The Home carousel stays on the first wallpaper.

The script copies the frontend into a temporary workspace inside the ignored
`.output/` directory and uses its own Vite server on `127.0.0.1:4179`. It closes
the browser and server and removes that workspace afterward. The normal dev
server, Svelte caches, Rust build, and native app data are separate.

If port 4179 is occupied, choose another port:

```powershell
$env:WALLSCAPE_SCREENSHOT_PORT = "4180"
npm run screenshots
Remove-Item Env:WALLSCAPE_SCREENSHOT_PORT
```

Capture fails on missing artwork, failed visible images, browser errors,
unexpected native commands, or an oversized export. Inspect all seven images
after UI changes. Check for clipped controls, poor contrast, and crowded layouts
at the width GitHub displays them.

## Artwork and staging

The six source images and their titles, dimensions, tags, and collection names
live in `scripts/screenshots/assets.json` and `scripts/screenshots/assets/`.
The first entry supplies the featured wallpaper and detail view.

Photographer credits, source pages, and licensing are recorded in
[ARTWORK.md](../scripts/screenshots/ARTWORK.md). The demo photographs belong only
to the documentation workflow. The app installer does not include them.

To change the visual direction, replace the six assets and update their manifest
and credits. Use images you can redistribute. Keep the set visually consistent,
with enough variation to make the library useful to look at.

For a native desktop or live playback recording, capture the installed Windows
app separately. Use the same artwork and window proportions, move the cursor away
from controls, and restore the prior wallpaper afterward. A browser capture
cannot demonstrate WorkerW placement, video decoding, or actual multi-monitor
playback.
