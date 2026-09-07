# Building ClovaKey

## Prerequisites

- **Rust** (stable) via [rustup](https://rustup.rs).
- **Node.js ≥ 20** and npm.
- **macOS**: Xcode Command Line Tools (`xcode-select --install`).
- **Windows**: the [WebView2 runtime](https://developer.microsoft.com/microsoft-edge/webview2/)
  (preinstalled on Windows 11) and the *Desktop development with C++* workload from the
  Visual Studio Build Tools.

Install JS dependencies (this also brings in the Tauri CLI):

```bash
npm install
```

## Develop

```bash
npm run tauri:dev      # app with hot-reloading frontend
```

Frontend-only checks:

```bash
npm run typecheck      # tsc --noEmit
npm run lint           # ESLint
npm run format         # Prettier (write)
npm run build          # typecheck + production bundle to dist/
```

Rust checks (run from `src-tauri/`):

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt
```

The security/OTP core can be tested on its own, with no GUI:

```bash
cargo test -p clovakey-core
```

## Production build

```bash
npm run tauri:build
```

Artifacts land in `src-tauri/target/release/bundle/`.

### macOS

- Produces `ClovaKey.app` and a `.dmg`.
- Building on Apple Silicon yields an `aarch64` app. For Intel, add the target and build
  for it, or produce a universal binary:

  ```bash
  rustup target add x86_64-apple-darwin
  npm run tauri:build -- --target universal-apple-darwin
  ```

- `minimumSystemVersion` is 10.15 (see `tauri.conf.json`).
- Camera QR scanning needs `NSCameraUsageDescription`, which is included via
  `src-tauri/Info.plist` and applied to the bundled `.app`. (In `tauri dev` the app runs
  as a bare binary without that plist, so the camera path is only guaranteed in a
  packaged build; image/paste import work everywhere.)

### Windows

- Produces an **NSIS** installer (`.exe`) under `bundle/nsis/`. Switch to MSI by adding
  `"msi"` to `bundle.targets` in `tauri.conf.json` (requires the WiX toolset).
- Install mode is per-machine (`bundle.windows.nsis.installMode`).
- **Windows ARM64**: add `rustup target add aarch64-pc-windows-msvc` and build with
  `--target aarch64-pc-windows-msvc`. The project is structured so this needs no code
  changes.

## Signing & notarization

Release builds are unsigned by default. See [`RELEASE.md`](RELEASE.md) for the exact
credentials required and how the CI workflow consumes them.
