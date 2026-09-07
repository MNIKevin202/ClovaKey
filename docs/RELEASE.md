# Releasing ClovaKey

## Versioning

ClovaKey uses semantic versioning. The version lives in two places that must match:

- `package.json` → `version`
- `src-tauri/tauri.conf.json` → `version` (the Cargo workspace inherits this)

Tag releases as `vMAJOR.MINOR.PATCH` (e.g. `v0.1.0`).

## Continuous integration

`.github/workflows/ci.yml` runs on every push and PR:

- **Frontend** — `npm ci`, `npm run lint`, `npm run typecheck`, `npm run build`.
- **Rust** — `cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`,
  `cargo test --workspace` (the core's RFC vectors, crypto, vault, import, and backup
  tests).
- **Bundle smoke build** — `tauri build` on macOS and Windows runners to catch
  packaging regressions.

## Release workflow

`.github/workflows/release.yml` runs when a `v*` tag is pushed. It builds signed (if
credentials are present) bundles on macOS and Windows and attaches them to a GitHub
Release: the macOS `.dmg` and the Windows NSIS `.exe`.

```bash
# cut a release
npm version 0.1.0 --no-git-tag-version      # or edit both version fields by hand
#   …also set src-tauri/tauri.conf.json "version": "0.1.0"
git commit -am "release: v0.1.0"
git tag v0.1.0
git push origin main --tags
```

## Signing & notarization — required secrets

No signing material is committed. Provide these as GitHub Actions **repository secrets**;
the workflow passes them through to `tauri build`.

### macOS (Developer ID + notarization)

| Secret | What it is |
|---|---|
| `APPLE_CERTIFICATE` | base64 of your Developer ID Application `.p12` |
| `APPLE_CERTIFICATE_PASSWORD` | the `.p12` export password |
| `APPLE_SIGNING_IDENTITY` | e.g. `Developer ID Application: Your Name (TEAMID)` |
| `APPLE_ID` | Apple ID email used for notarization |
| `APPLE_PASSWORD` | an app-specific password for that Apple ID |
| `APPLE_TEAM_ID` | your 10-character Apple Team ID |

With these present, Tauri signs the `.app`/`.dmg` and submits them to Apple's
notary service. Without them, the workflow still produces an **unsigned** `.dmg`
(users must right-click → Open on first launch).

### Windows (Authenticode)

Provide a code-signing certificate and configure `bundle.windows.certificateThumbprint`
(or the `signCommand`) in `tauri.conf.json`, and supply the certificate to the runner.
For an EV/HSM or Azure Trusted Signing setup, wire the appropriate `signCommand`. Without
a certificate the installer is produced **unsigned** (SmartScreen will warn on first
run).

## Auto-update (foundation)

ClovaKey is architected for [Tauri's updater](https://v2.tauri.app/plugin/updater/), but
it ships **disabled** so the app makes no network calls until an update endpoint and
signing key are provisioned. A failure to reach an update server must never block access
to local codes — the updater is entirely separate from the vault.

To enable updates:

1. Generate an update signing key (keep the private key secret; never commit it):

   ```bash
   npm run tauri signer generate -- -w ./clovakey-updater.key
   ```

2. Add the updater plugin config to `tauri.conf.json`:

   ```jsonc
   "plugins": {
     "updater": {
       "pubkey": "<contents of clovakey-updater.key.pub>",
       "endpoints": ["https://releases.example.com/clovakey/{{target}}/{{arch}}/{{current_version}}"]
     }
   }
   ```

3. Build with the updater enabled and register the plugin:
   - build the `clovakey` crate with `--features updater`,
   - add `tauri_plugin_updater::Builder::new().build()` and
     `tauri_plugin_process::init()` to the builder (both are already dependencies, gated
     behind the `updater` feature),
   - add `updater:default` and `process:allow-restart` to the capability.

4. Set `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` as release
   secrets so the workflow signs update artifacts.

Update requests contain only the target/arch/current-version — never account names, OTP
values, seeds, issuer names, or any vault metadata.
