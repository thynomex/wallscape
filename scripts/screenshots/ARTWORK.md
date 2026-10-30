# Screenshot artwork

These six photographs are demo content for the GitHub screenshots. They are not bundled with the Wallscape application.

All photographs are by [Sam Ferrara](https://stocksnap.io/author/34723). Each linked StockSnap page identified the photographer and the [CC0 1.0 Universal](https://creativecommons.org/publicdomain/zero/1.0/) license when checked on October 24, 2026. StockSnap also publishes a [license summary](https://stocksnap.io/license). The names below are short display titles chosen for the demo, not the photographer's original titles or verified place names.

| Demo title | Source photograph | Original dimensions | Export |
| --- | --- | --- | --- |
| Glacier at Dusk | [Nature landscape · 0QSHE1I3PL](https://stocksnap.io/photo/nature-landscape-0QSHE1I3PL) | 8192 × 5461 | `assets/01-glacier-dusk.jpg` |
| Alpine Forest | [Nature landscape · 0R847P5TS8](https://stocksnap.io/photo/nature-landscape-0R847P5TS8) | 7486 × 4149 | `assets/02-alpine-forest.jpg` |
| Blue Ridge | [Nature landscape · 05DUUIO7UT](https://stocksnap.io/photo/nature-landscape-05DUUIO7UT) | 7570 × 5059 | `assets/03-blue-ridge.jpg` |
| Amber Valley | [Mountain landscape · D3QTMYLQH9](https://stocksnap.io/photo/mountain-landscape-D3QTMYLQH9) | 3840 × 2735 | `assets/04-amber-valley.jpg` |
| Meltwater | [Mountain landscape · TCRBATOT1S](https://stocksnap.io/photo/mountain-landscape-TCRBATOT1S) | 3840 × 2555 | `assets/05-meltwater.jpg` |
| Last Light | [Mountain landscape · D9QN84B8D8](https://stocksnap.io/photo/mountain-landscape-D9QN84B8D8) | 5399 × 3037 | `assets/06-last-light.jpg` |

The exports are 2560 × 1600 progressive JPEGs, each smaller than 1 MB. They use a centered crop and Lanczos resizing, with no color edits. The first image supplies the Home and wallpaper detail screenshots. Its glacier curve and dark foreground leave room for the app's text. The set keeps cool blues and greens across most tiles, with an amber valley and a sunset for variety.

[assets.json](assets.json) records each source URL, photographer, license, original dimensions, processing settings, byte count, and SHA-256 of both the source download and final JPEG. The source pages' JSON-LD supplied the attribution and license fields. The decoded image files supplied the dimensions.

## Verify or restore the images

Run from this directory with Python 3.10 or later. Pillow 12.2.0 produced the checked-in JPEGs.

```powershell
python -m pip install requests beautifulsoup4 Pillow==12.2.0
python download-assets.py --verify
```

To restore a missing or changed asset, run `python download-assets.py`. Valid local files are kept. The script downloads through each source page's Free Download form, checks the original SHA-256, reproduces the crop, and requires the result to match the recorded SHA-256 before replacing a local file. A changed source or encoder output stops that asset from being replaced.

Suggested credit when reusing the screenshots: "Demo photography by Sam Ferrara via StockSnap, CC0."
