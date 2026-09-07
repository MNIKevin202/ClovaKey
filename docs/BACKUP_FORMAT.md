# ClovaKey Backup Format (`.clovakey`)

A ClovaKey backup is a portable, passphrase-encrypted file. It is versioned from day one
and authenticated, and it never contains plaintext secrets.

## Envelope

The file is UTF-8 JSON:

```json
{
  "format": "clovakey-backup",
  "formatVersion": 1,
  "kdf": {
    "algorithm": "argon2id",
    "memoryKib": 65536,
    "iterations": 3,
    "parallelism": 1,
    "salt": "<base64>"
  },
  "cipher": "xchacha20poly1305",
  "nonce": "<base64, 24 bytes>",
  "ciphertext": "<base64>"
}
```

## Cryptography

- **Key derivation**: `key = Argon2id(passphrase, salt, params)` → 32 bytes. `params`
  are recorded in the header so they can evolve without breaking old backups. On
  restore they are validated against safe bounds (8 KiB–2 GiB memory, 1–20 iterations,
  1–16 lanes) before use.
- **Encryption**: `ciphertext = XChaCha20-Poly1305(key, nonce, plaintext, aad)`.
- **Associated data (AAD)**: the canonical string
  `clovakey-backup|v1|argon2id|m=<memoryKib>|t=<iterations>|p=<parallelism>|xchacha20poly1305`.
  Because the header parameters are authenticated, editing them invalidates the file.

## Plaintext payload (inside the ciphertext)

```json
{
  "formatVersion": 1,
  "exportedAt": 1730000000,
  "accounts": [
    {
      "issuer": "GitHub",
      "accountName": "you@example.com",
      "type": "totp",
      "algorithm": "SHA1",
      "digits": 6,
      "period": 30,
      "counter": 0,
      "secret": "<base32>",
      "favorite": true,
      "group": "Personal",
      "icon": null,
      "sortOrder": 0
    }
  ],
  "groups": [{ "name": "Personal", "sortOrder": 0 }],
  "settings": [["theme", "dark"]]
}
```

Secrets are Base32 (the portable standard). Groups are referenced **by name** so restore
can remap them into (or create them in) the target vault. The decrypted payload is held
in a zeroizing buffer and the secret strings are wiped on drop.

## Restore behaviour

- A wrong passphrase or any tampering fails as an authentication error — the restore
  simply reports an incorrect passphrase.
- Duplicate detection uses the same keyed fingerprint as import; by default duplicates
  are skipped, or the user can choose to import them anyway.
- Unknown future top-level fields are ignored (forward-compatible via `serde(default)`).

## Versioning

`formatVersion` starts at 1. A reader refuses a version it does not understand
(`unsupported backup version: N`) rather than guessing. New versions add a branch in
`backup::open_payload`.
