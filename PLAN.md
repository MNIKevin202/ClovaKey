# ClovaKey — Implementation Plan

ClovaKey is an offline-first desktop authenticator (TOTP/HOTP) for macOS and Windows,
part of the Clova suite. Built with Tauri 2 + Rust core + React/TypeScript/Vite.

## Guiding principles

1. **The Rust core is the security boundary.** Secrets live in Rust; the frontend asks
   "generate the code for account X" and receives only ephemeral codes + metadata.
2. **No invented crypto.** Audited RustCrypto primitives only.
3. **Offline-first.** No account, no cloud, no telemetry. A fresh install with no network
   can add an account and generate codes.
4. **Least privilege.** Tauri capabilities are scoped tightly.
5. **Cross-platform from day one** — never "make Windows work later".

## Technology decisions (audited)

| Concern | Choice | Why |
|---|---|---|
| Shell | Tauri 2 | Native, small, Rust core, cross-platform |
| Frontend | React 19 + TS + Vite | Requested; mature |
| OTP | Hand-written HOTP/TOTP over RustCrypto `hmac`+`sha1`/`sha2` | Minimal trusted surface, RFC-verifiable |
| Base32 | `data-encoding` | Audited, RFC4648, no padding foot-guns |
| AEAD | `chacha20poly1305` (XChaCha20-Poly1305) | 192-bit nonce → random nonces are safe |
| KDF | `argon2` (Argon2id) | Modern password KDF for backups & passphrase lock |
| Zeroize | `zeroize` | Wipe key material from memory |
| CSPRNG | `rand` / `getrandom` (OsRng) | OS entropy for keys/nonces/salts |
| OS key store | `keyring` v3 | macOS Keychain / Windows Credential Manager (DPAPI) |
| DB | `rusqlite` (bundled) | Embedded, no external SQLite, migrations via user_version |
| Google migration protobuf | Hand-written bounded decoder | Self-contained, oversized/malformed-safe, testable |
| QR decode (images) | `rqrr` + `image` | Pure-Rust, local decode, no upload |
| QR encode (export) | `qrcode` | Render account/backup as QR (SVG) |
| Camera scan | webview `getUserMedia` + `jsQR` | Only practical cross-platform camera path; decoded locally, URI validated in Rust |
| Frontend state | `zustand` | Small, clean |
| UI icons | `lucide-react` | Restrained, consistent |
| Styling | Bespoke CSS + design tokens | Full control of premium look; no Bootstrap/Material |

**Stronghold vs. Keychain:** we use an encrypted SQLite vault whose master key is protected by
the OS keychain (`keyring`), with an optional Argon2id passphrase layer. This is the design the
brief itself outlines, is fully cross-platform, and avoids the heavier Stronghold plugin while
keeping direct control of the crypto. Documented in `docs/SECURITY.md`.

## Security model (summary)

- Master key = 32 random bytes (CSPRNG), generated on first run.
- Each account's OTP secret encrypted individually with XChaCha20-Poly1305, AAD = account id
  (prevents ciphertext swapping). Metadata (issuer, name, algo, digits…) stays searchable.
- Master key stored in OS keychain. Optional passphrase wraps it via Argon2id for a hard lock.
- Lock drops & zeroizes the key from memory → codes cannot be generated while locked.
- Backups: portable, password-based. Argon2id → XChaCha20-Poly1305, versioned `.clovakey`.

## Cargo workspace

- `clovakey-core` (lib): otp, crypto, model, import (otpauth + google), backup, vault/storage,
  keystore abstraction. Pure Rust, no Tauri — fully unit-tested against RFC vectors & fixtures.
- `clovakey` (bin): Tauri app — commands, app state, tray, clipboard, updater, autostart.

## Milestones

- **M1** Foundation: workspace, branding/icons, app shell, navigation, authenticator UI skeleton.
- **M2** OTP engine: TOTP/HOTP, otpauth URI parser, RFC test vectors.
- **M3** Secure vault: encrypted storage, keychain, lock/unlock, passphrase.
- **M4** Account management: add/edit/delete, favorites, groups, search.
- **M5** QR support: image import, clipboard image, URI paste, camera.
- **M6** Google Authenticator migration: protobuf decoder, single + multi-batch, preview, dedup.
- **M7** Backup & restore: encrypted `.clovakey`, cross-platform.
- **M8** Desktop integration: tray, clipboard auto-clear, startup, full settings.
- **M9** Polish: accessibility, animation, large lists, edge cases, self security audit.
- **M10** Release: CI, fmt/clippy/tests, macOS DMG, Windows NSIS, updater foundation, docs.

Each milestone: run tests, `cargo fmt`, `cargo clippy -D warnings`, fix, commit.
