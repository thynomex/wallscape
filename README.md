# Wallscape

Wallscape is a free and open-source live wallpaper manager for Windows. It can apply local images and videos, organize a wallpaper library, and discover optional content from Wallhaven and MotionBGS.

> Wallscape is currently an early release. Back up important settings and report reproducible problems through GitHub Issues.

## Features

- Image and video wallpapers on the Windows desktop
- Local imports with wallpaper metadata and thumbnails
- Wallpaper history, favorites, collections, rotation, and restore controls
- Multi-monitor detection and per-monitor assignment
- Optional Wallhaven and MotionBGS discovery
- Hardware-accelerated video playback through libmpv
- Pause, resume, tray, startup, and resource-use settings

## Download

Download the latest MSI from [GitHub Releases](https://github.com/thynomex/wallscape/releases/latest).

Wallscape supports Windows 10 and Windows 11. The Microsoft Edge WebView2 Runtime is required and is normally already installed. Unsigned development releases may trigger a Microsoft Defender SmartScreen warning; verify that the installer came from this repository and compare its SHA-256 checksum with the release files.

## Privacy and network access

Wallscape stores its library and settings locally. It does not include wallpaper files in the installer.

Network requests occur when you use Wallhaven or MotionBGS discovery, open an external source page, or download the libmpv development package while building from source. Wallscape does not send automatic diagnostic reports.

Wallhaven and MotionBGS are independent third-party services. Their content belongs to the respective owners and is subject to each service's terms. See [Third-party content](docs/THIRD-PARTY-CONTENT.md).

## Build from source

Prerequisites:

- Windows 10 or later
- Node.js 22.12 or later, or Node.js 20.19 or later
- Rust stable with the MSVC toolchain
- Visual Studio Build Tools with Desktop development with C++
- WebView2 Runtime

```powershell
npm install
npm run setup:mpv
npm run tauri dev
```

Create an MSI:

```powershell
npm run setup:mpv
npm run tauri build
```

Build output is written under `src-tauri/target/release/bundle/msi/`.

## Development checks

```powershell
npm run check
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets
```

See [Development](docs/DEVELOPMENT.md), [Architecture](docs/ARCHITECTURE.md), and [Troubleshooting](docs/TROUBLESHOOTING.md).

## Contributing and support

Contributions are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request. For help, see [SUPPORT.md](SUPPORT.md). Security vulnerabilities should be reported according to [SECURITY.md](SECURITY.md).

## License

Wallscape source code is licensed under the [MIT License](LICENSE). Bundled dependencies and media integrations have their own terms; see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
