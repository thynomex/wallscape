# Contributing to Wallscape

Thank you for helping improve Wallscape.

## Before opening an issue

- Search existing issues.
- Use the latest release or current `main` branch.
- Remove personal file paths and private media names from logs.

## Development setup

Wallscape requires Windows, Node.js, Rust with the MSVC toolchain, Visual Studio C++ Build Tools, and WebView2.

```powershell
npm install
npm run setup:mpv
npm run tauri dev
```

## Pull requests

Keep changes focused and explain the user-visible behavior. Add or update tests when practical. Before submitting, run:

```powershell
npm run check
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets
```

Do not commit build output, downloaded wallpaper media, `.vendor/`, credentials, webhook URLs, or personal diagnostics. By contributing, you agree that your contribution is licensed under the MIT License.
