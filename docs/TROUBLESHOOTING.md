# Troubleshooting

## The installer is blocked by SmartScreen

Early releases may be unsigned. Download only from this repository's GitHub Releases page and compare the installer SHA-256 value with `SHA256SUMS.txt`.

## The application does not open

Install or repair the Microsoft Edge WebView2 Runtime, restart Wallscape, and check `%LOCALAPPDATA%\Wallscape\logs\wallscape.log`.

## Video wallpapers do not play

Confirm that the release directory contains `libmpv-2.dll`. Developers should rerun `npm run setup:mpv`. Some uncommon containers and codecs are intentionally absent from the slim runtime; try an MP4 containing H.264 or H.265 video.

## Wallpaper placement is incorrect

Include the Windows display layout, scaling percentage, monitor resolutions, and which display is primary in a bug report. Restart Windows Explorer before reporting a persistent WorkerW/desktop-host problem.

## High resource use

Enable pause behavior in Settings, test with a lower-resolution or lower-frame-rate video, and record whether the problem occurs while the desktop is visible or covered.

Before attaching logs to an issue, remove personal file paths, wallpaper names, and other private information.
