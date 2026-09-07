# Google Authenticator Migration Import

ClovaKey imports accounts from Google Authenticator's **Export accounts** feature by
decoding its migration QR codes **entirely on the device**. No external service is
involved, and secrets never leave the machine.

## The pipeline

```
QR code  →  otpauth-migration://offline?data=<url-encoded base64>
         →  URL-decode the `data` parameter
         →  Base64-decode
         →  Protocol Buffers  (MigrationPayload)
         →  one or more OTP account records
```

All of this is implemented in `clovakey-core::import::google` and
`clovakey-core::import::protobuf`. We implement the protobuf structure directly (a small,
bounded wire-format reader) rather than depending on a general protobuf toolchain, so the
parser is self-contained and defensive.

## MigrationPayload structure

```proto
message MigrationPayload {
  message OtpParameters {
    bytes    secret    = 1;   // RAW key bytes — NOT Base32 text
    string   name      = 2;
    string   issuer    = 3;
    Algorithm algorithm = 4;  // SHA1=1, SHA256=2, SHA512=3
    DigitCount digits  = 5;   // SIX=1, EIGHT=2
    OtpType  type      = 6;   // HOTP=1, TOTP=2
    int64    counter   = 7;
  }
  repeated OtpParameters otp_parameters = 1;
  int32 version     = 2;
  int32 batch_size  = 3;
  int32 batch_index = 4;
  int32 batch_id    = 5;
}
```

**Critical correctness point:** the `secret` field is **raw key material**, not Base32
text. ClovaKey stores those bytes directly (encrypted); it never mistakenly re-decodes
them as Base32. A dedicated test asserts a raw secret round-trips byte-for-byte.

## Multiple QR codes (batches)

Large exports are split across several QR codes. Each carries `batch_size`,
`batch_index`, and a shared `batch_id`. ClovaKey's `BatchCollector`:

- tracks progress ("QR codes scanned 2 of 4"),
- **rejects codes from a different export** (mismatched `batch_id` or `batch_size`),
- rejects out-of-range indices,
- and only produces the account list once every batch index has been scanned.

## Import UX

1. **Import → Google Authenticator** (or Add account → Import from Google Authenticator).
2. Add the export QR by camera scan, QR image, or by pasting the migration link. For a
   multi-QR export, keep adding codes until the batch is complete.
3. **Preview** all decoded accounts. Likely duplicates (matching an existing account by
   keyed fingerprint) are flagged and unchecked by default. You can rename issuer/account
   per row, assign a group, and Select all / none.
4. **Import selected.** A summary reports how many were imported, skipped, or grouped.

Throughout, the UI shows only metadata; decoded secrets stay in a Rust-side session and
are zeroized if you cancel.

## Safety

- Input is capped (512 KiB payload, ≤ 10 000 records) and every varint/length is
  bounds-checked, so malformed or hostile data is rejected cleanly rather than crashing.
- The migration QR is treated as a credential: it is never logged, printed, or written to
  a temporary file.

## Test fixtures

Tests build **synthetic** migration payloads with a small in-test encoder (see
`import/google.rs` tests) — never real credentials. If you hand ClovaKey a real export
for manual testing, treat it as sensitive: do not commit it, print its seeds, or leave it
in logs.
