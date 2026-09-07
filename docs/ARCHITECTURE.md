# ClovaKey — Architecture

## Overview

ClovaKey is a Tauri 2 desktop app. It has two clean halves:

- **`clovakey-core`** — a pure-Rust library with the security- and standards-critical
  logic (OTP, crypto, vault, imports, backups). It has **no Tauri or UI dependency** and
  is exhaustively unit-tested in isolation.
- **`clovakey`** — the Tauri application: a thin layer that exposes the core to the UI
  via commands and owns desktop concerns (tray, clipboard, idle-lock, autostart).
- **`src/`** — the React + TypeScript frontend.

```
┌────────────────────────────────────────────────────────────┐
│  React UI (src/)                                             │
│  views · components · zustand store · typed IPC (lib/ipc.ts) │
└───────────────▲───────────────────────────┬─────────────────┘
                │  metadata + ephemeral codes │  commands (invoke)
                │                             ▼
┌───────────────┴─────────────────────────────────────────────┐
│  Tauri app (src-tauri/src/)                                  │
│  commands · state · tray · clipboard · idle-lock · settings  │
└───────────────▲──────────────────────────────────────────────┘
                │  Rust API (secrets stay below this line)
                ▼
┌──────────────────────────────────────────────────────────────┐
│  clovakey-core (src-tauri/core/)                               │
│  otp · crypto · storage(vault) · keystore · import · backup    │
└──────────────────────────────▲─────────────────────────────────┘
                                │
                    SQLite (encrypted secrets)  +  OS keychain (master key)
```

## `clovakey-core` modules

| Module | Responsibility |
|---|---|
| `otp` | RFC 4226 HOTP, RFC 6238 TOTP, tolerant Base32, `otpauth://` parse/build |
| `crypto` | XChaCha20-Poly1305 AEAD, Argon2id KDF, CSPRNG, zeroizing key type |
| `keystore` | `KeyStore` trait; OS keychain impl + in-memory impl for tests |
| `storage` | The `Vault`: encrypted SQLite, lock/unlock, accounts/groups/settings, migrations |
| `model` | Non-sensitive account/group/config types (never hold a secret) |
| `dedupe` | Keyed duplicate-detection fingerprints |
| `import::google` | `otpauth-migration://` protobuf decode + multi-QR batch collector |
| `import::protobuf` | Minimal bounded protobuf wire reader |
| `qr` | Local QR decode (rqrr + image) and SVG encode (qrcode) |
| `backup` | Versioned, passphrase-encrypted `.clovakey` format |

## The security boundary

The single most important rule: **the frontend never receives an OTP secret during
normal use.** It calls `codes_generate_all` / `code_generate` and gets back a
`GeneratedCode` (the current code + timing). Secrets are decrypted transiently inside the
core to compute a code and are dropped immediately.

Raw secret material crosses the boundary only when:

- **Creating** an account (`account_add_manual` / `account_add_uri` / scan) — the user
  supplied it; the core validates, encrypts, and stores it, returning only metadata.
- **Revealing/exporting** an account (`account_reveal`) — an explicit action, gated
  behind passphrase re-entry when a passphrase is set, with a clear warning in the UI.

Google-import staging keeps decoded secrets in a Rust-side session; the UI sees only a
metadata preview (issuer/account/type + a duplicate flag) until the user commits.

## Data model

`accounts` (metadata + `secret_nonce`/`secret_ct` blobs + keyed `secret_fp`),
`groups`, `settings` (non-sensitive key/value), and `vault_meta` (protection mode,
wrapped key, KDF params, check value). Schema migrations run from day one via
`PRAGMA user_version` (see `storage/migrations.rs`).

## Frontend

- **State**: a single `zustand` store (`src/state/store.ts`) holds vault status,
  accounts, groups, codes, settings, and UI state. A 1 s ticker updates the clock and
  regenerates codes when a TOTP window ends.
- **Countdown ring**: a CSS animation on `stroke-dashoffset`, synced to the window
  boundary via a negative `animation-delay` and re-keyed each window — smooth, with no
  per-frame JS, and it respects reduced-motion.
- **IPC**: every call goes through `src/lib/ipc.ts`, the only place the UI talks to Rust.
- **Styling**: bespoke CSS with design tokens (`src/styles/tokens.css`); light/dark/
  system themes; no UI framework.

## Cross-platform & the future

- Desktop-only plugins (autostart, single-instance, updater/process) are gated to
  non-mobile targets, and the app entry uses the Tauri lib+`run()` pattern, so mobile or
  Windows ARM64 can be added without restructuring.
- Biometric unlock is behind a documented abstraction (`platform/biometric.rs`).
