# Development

## Requirements

- Windows 10 or Windows 11
- Node.js 22.12+ or 20.19+
- Rust stable with the MSVC target
- Visual Studio Build Tools with Desktop development with C++
- WebView2 Runtime

## Setup

```powershell
npm install
npm run setup:mpv
npm run tauri dev
```

`setup:mpv` downloads a pinned archive, verifies its SHA-256 hash, installs development files under `.vendor/`, and stages the runtime DLL for Tauri packaging.

## Checks

```powershell
npm run check
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets
```

Regenerate TypeScript bindings after changing exported Rust DTOs:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml export_typescript_bindings
```

## Release build

Keep the versions in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json` synchronized. Tag releases as `vMAJOR.MINOR.PATCH`; GitHub Actions builds and publishes the MSI, checksum, license, and third-party notices.

Never commit `.vendor/`, `src-tauri/bundle/`, `target/`, credentials, diagnostic logs, or downloaded media.
