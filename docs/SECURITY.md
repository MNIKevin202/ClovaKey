# ClovaKey — Security Model

ClovaKey treats OTP secrets as what they are: account credentials. This document
describes how they are protected, the threat model, and the results of a focused
self-review.

## Principles

1. **The Rust core is the trust boundary.** All secret handling lives in the
   `clovakey-core` library. The UI asks the core to *generate the code for account
   `id`* and receives only the ephemeral code plus non-sensitive metadata. Raw secrets
   cross the boundary in only two places: **creation** (the user typed/scanned it) and
   an **explicit, re-authenticated reveal/export**.
2. **No invented cryptography.** Only audited RustCrypto primitives are used.
3. **Offline-first, zero telemetry.** ClovaKey makes no network requests for
   authenticator data and has no analytics. It runs fully offline.

## Data at rest

The vault is an SQLite database in the OS app-data directory
(`~/Library/Application Support/com.clova.clovakey` on macOS;
`%APPDATA%\com.clova.clovakey` on Windows).

- Each account's OTP secret is encrypted **individually** with
  **XChaCha20-Poly1305** (a 256-bit key, 192-bit random nonce). The account id is used
  as the AEAD *associated data*, so a ciphertext cannot be moved or swapped between
  records without detection.
- Everything else (issuer, account name, algorithm, digits, period, counter, group,
  favorite, sort order) is non-sensitive, searchable metadata stored in plain columns.
- A keyed duplicate-detection fingerprint (`HMAC-SHA256(master_key, secret ‖ config)`)
  is stored per account. Being keyed, it is meaningless without the unlocked vault and,
  being a one-way MAC, never exposes the secret.

## The master key

A random 256-bit master key encrypts every secret. It is protected in one of two modes
(`vault_meta.protection`):

- **`keychain`** (default) — the master key is stored in the OS credential store
  (macOS Keychain / Windows Credential Manager, via the `keyring` crate). Unlocking
  re-fetches it. This protects an offline copy of the database (the DB is useless
  without the key) and ties the key to the OS user account.
- **`passphrase`** — the master key is wrapped with a key derived from a user
  passphrase via **Argon2id** (64 MiB, t=3, p=1) and stored (sealed) in the database.
  Unlocking requires the passphrase. This is a **hard lock**: while locked, the master
  key is not in memory, so no secret can be decrypted and no code can be generated.

The master key exists in memory only while unlocked and is zeroized (via `zeroize`) on
lock. A known-plaintext check value, sealed under the master key, is verified on unlock
to detect a corrupt or mismatched key.

### App lock

- Manual **Lock Now** (⌘/Ctrl+L, tray, or sidebar).
- **Lock on start** (setting) — the vault always begins locked; with this on, keychain
  mode also shows the lock screen instead of auto-unlocking.
- **Idle auto-lock** — a Rust background task locks the vault after a configurable
  inactivity timeout.
- Planned: lock on system sleep / screen lock (see *Roadmap* below).

## Backups

Portable `.clovakey` backups are a versioned JSON envelope:

- Key derived from a user passphrase via **Argon2id**; the derived key encrypts the
  serialized payload (accounts with Base32 secrets, groups, settings) with
  **XChaCha20-Poly1305**.
- The envelope header (format, version, KDF parameters) is bound as AEAD associated
  data — tampering with it fails authentication.
- KDF parameters read from an untrusted file are clamped to safe bounds before use, so a
  malicious header cannot force a multi-terabyte memory allocation.
- A wrong passphrase or any tampering fails as an authentication error.

Backups are password-based and independent of the machine key, so they restore on
another computer. See [`BACKUP_FORMAT.md`](BACKUP_FORMAT.md).

## Threat model

**In scope / mitigated**

- *Theft of the database file* — secrets are encrypted; the key is in the OS keychain
  or behind an Argon2id passphrase.
- *Malicious import/backup payloads* — bounded parsers with size and structure caps
  (see the review below).
- *Clipboard leakage* — copied codes auto-clear after a configurable delay, without
  erasing anything the user copied afterwards.
- *Shoulder-surfing* — optional "hide codes until hovered".

**Out of scope**

- A compromised OS user session with the vault unlocked (memory scraping, a keylogger).
  ClovaKey reduces exposure (zeroization, hard lock) but cannot defend a fully
  compromised host.
- Physical access with the passphrase, or a backup file plus its passphrase.

## Self-review checklist

| Concern | Status |
|---|---|
| Plaintext seed persistence | **None.** Per-secret AEAD; a test scans the DB file to assert the plaintext seed never appears on disk. |
| Secrets in logs | **None.** Errors are redacted; `SymmetricKey`'s `Debug` is `<redacted>`; no logging of secret values. |
| Secrets in frontend state | Only ephemeral codes + metadata are sent; raw secret only via gated reveal/export. |
| Unsafe temporary files | None — imports decode in memory; backups write directly to the chosen path. |
| Insecure backup encryption | Argon2id + XChaCha20-Poly1305, authenticated, header bound as AAD. |
| Hard-coded keys | None. Master key is CSPRNG-generated; AAD constants are domain separators, not keys. |
| Predictable salts/nonces | All from the OS CSPRNG (`getrandom`). |
| Overbroad Tauri permissions | Capability is minimal: no network/fs/shell; one clipboard-write, dialogs, read-only OS info, autostart. Rust performs all file IO. |
| Path traversal | Backup paths come from the OS file dialog; the core performs no path concatenation on untrusted input. |
| Malicious backup/import payloads | Size caps (64 MiB backup, 512 KiB migration), clamped KDF params, authenticated decryption. |
| QR parser crashes | Image decoded with strict width/height/alloc limits; decoder returns `Result`, never panics on bad input. |
| Protobuf parser abuse | Hand-written bounded reader; rejects truncated varints and lengths past the buffer; caps record count; tested against garbage. |
| Oversized payload attacks | Covered by the caps above. |
| Clipboard race conditions | A monotonic token + exact-value check ensure auto-clear only removes ClovaKey's own value and never a newer one. |
| Dangerous IPC commands | Commands validate input; reveal/export/wipe are re-authenticated when a passphrase is set. |
| Frontend XSS → privileged commands | Strict CSP (`script-src 'self'`, no remote origins); the only `dangerouslySetInnerHTML` is a QR SVG generated by the `qrcode` crate (user text becomes QR modules, not markup). |

## Roadmap

- **Biometric unlock** (Touch ID / Windows Hello). The abstraction exists in
  `src-tauri/src/platform/biometric.rs` and the passphrase-wrapping design already
  supports gating an unlock behind a biometric check; a reliable native implementation
  is deferred rather than shipped fragile.
- **Lock on system sleep / screen lock** via platform power/session notifications.
