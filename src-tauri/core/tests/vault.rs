//! Integration tests for the encrypted vault.
//!
//! These use an in-memory key store and a temp-file database — never the real
//! OS keychain and never real credentials. Secrets are synthetic.

use clovakey_core::keystore::MemoryKeyStore;
use clovakey_core::model::{AccountPatch, NewAccount, OtpConfig};
use clovakey_core::otp::{base32, Algorithm, OtpType};
use clovakey_core::storage::Vault;
use rusqlite::Connection;

fn temp_vault() -> Vault {
    let conn = Connection::open_in_memory().unwrap();
    Vault::from_connection(conn, Box::new(MemoryKeyStore::new())).unwrap()
}

fn totp_account(name: &str) -> NewAccount {
    let mut a = NewAccount::new(name);
    a.issuer = Some("GitHub".into());
    a.otp = OtpConfig::default();
    a
}

// Synthetic RFC test seed, Base32-encoded.
const SEED_B32: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ"; // "12345678901234567890"

#[test]
fn setup_unlock_lock_cycle_keychain() {
    let v = temp_vault();
    assert!(!v.is_initialized().unwrap());
    v.setup(None).unwrap();
    assert!(v.is_initialized().unwrap());
    assert!(!v.is_locked());
    v.lock();
    assert!(v.is_locked());
    v.unlock(None).unwrap();
    assert!(!v.is_locked());
}

#[test]
fn passphrase_hard_lock() {
    let v = temp_vault();
    v.setup(Some("correct horse battery staple")).unwrap();
    v.lock();
    // Wrong passphrase fails.
    assert!(v.unlock(Some("wrong")).is_err());
    assert!(v.is_locked());
    // No passphrase fails in passphrase mode.
    assert!(v.unlock(None).is_err());
    // Correct passphrase unlocks.
    v.unlock(Some("correct horse battery staple")).unwrap();
    assert!(!v.is_locked());
}

#[test]
fn add_and_generate_totp() {
    let v = temp_vault();
    v.setup(None).unwrap();
    let secret = base32::decode_secret(SEED_B32).unwrap();
    let acct = v
        .add_account(&totp_account("kevin@example.com"), &secret)
        .unwrap();
    assert_eq!(acct.account_name, "kevin@example.com");

    let code = v.generate_code(&acct.id).unwrap();
    assert_eq!(code.code.len(), 6);
    assert_eq!(code.otp_type, OtpType::Totp);
    assert!(code.seconds_remaining <= 30);
}

#[test]
fn codes_unavailable_while_locked() {
    let v = temp_vault();
    v.setup(Some("pw")).unwrap();
    let secret = base32::decode_secret(SEED_B32).unwrap();
    let acct = v.add_account(&totp_account("a"), &secret).unwrap();
    v.lock();
    // Listing metadata still works; generating a code must not.
    assert!(v.generate_code(&acct.id).is_err());
}

#[test]
fn hotp_counter_advances_and_persists() {
    let v = temp_vault();
    v.setup(None).unwrap();
    let secret = base32::decode_secret(SEED_B32).unwrap();
    let mut new = NewAccount::new("bob");
    new.otp = OtpConfig {
        otp_type: OtpType::Hotp,
        algorithm: Algorithm::Sha1,
        digits: 6,
        period: 30,
        counter: 0,
    };
    let acct = v.add_account(&new, &secret).unwrap();

    // Counter 0 → RFC 4226 first value.
    let c0 = v.generate_code(&acct.id).unwrap();
    assert_eq!(c0.code, "755224");
    // Advance → counter 1 value, and it persists.
    let c1 = v.advance_hotp(&acct.id).unwrap();
    assert_eq!(c1.code, "287082");
    let again = v.generate_code(&acct.id).unwrap();
    assert_eq!(again.code, "287082");
}

#[test]
fn secret_is_encrypted_at_rest() {
    // The raw seed bytes must never appear in the database file.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vault.db");
    {
        let v = Vault::open(&path, "com.clova.clovakey.test-encrypt").unwrap_or_else(|_| {
            // If the OS keychain is unavailable in this environment, fall back
            // to a memory store but keep the on-disk DB for inspection.
            let conn = Connection::open(&path).unwrap();
            Vault::from_connection(conn, Box::new(MemoryKeyStore::new())).unwrap()
        });
        v.setup(Some("pw")).unwrap();
        let secret = base32::decode_secret(SEED_B32).unwrap();
        assert_eq!(&secret[..], b"12345678901234567890");
        v.add_account(&totp_account("a"), &secret).unwrap();
    }
    let bytes = std::fs::read(&path).unwrap();
    // Look for the ASCII seed anywhere in the file.
    let needle = b"12345678901234567890";
    let found = bytes.windows(needle.len()).any(|w| w == needle);
    assert!(
        !found,
        "plaintext seed must not be present in the vault file"
    );
}

#[test]
fn update_delete_and_reorder() {
    let v = temp_vault();
    v.setup(None).unwrap();
    let secret = base32::decode_secret(SEED_B32).unwrap();
    let a = v.add_account(&totp_account("a"), &secret).unwrap();
    let b = v.add_account(&totp_account("b"), &secret).unwrap();

    let patch = AccountPatch {
        favorite: Some(true),
        account_name: Some("renamed".into()),
        ..Default::default()
    };
    let updated = v.update_account(&a.id, &patch).unwrap();
    assert!(updated.favorite);
    assert_eq!(updated.account_name, "renamed");

    v.reorder_accounts(&[b.id.clone(), a.id.clone()]).unwrap();
    v.delete_account(&b.id).unwrap();
    assert_eq!(v.list_accounts().unwrap().len(), 1);
    assert!(v.delete_account(&b.id).is_err());
}

#[test]
fn groups_crud_and_assignment() {
    let v = temp_vault();
    v.setup(None).unwrap();
    let g = v.create_group("Work").unwrap();
    assert_eq!(v.list_groups().unwrap().len(), 1);

    let secret = base32::decode_secret(SEED_B32).unwrap();
    let a = v.add_account(&totp_account("a"), &secret).unwrap();
    let patch = AccountPatch {
        group_id: Some(Some(g.id.clone())),
        ..Default::default()
    };
    let updated = v.update_account(&a.id, &patch).unwrap();
    assert_eq!(updated.group_id.as_deref(), Some(g.id.as_str()));

    // Deleting the group nulls the account's group_id (ON DELETE SET NULL).
    v.delete_group(&g.id).unwrap();
    assert!(v.get_account(&a.id).unwrap().group_id.is_none());
}

#[test]
fn duplicate_fingerprints_detected() {
    let v = temp_vault();
    v.setup(None).unwrap();
    let secret = base32::decode_secret(SEED_B32).unwrap();
    let a = v.add_account(&totp_account("a"), &secret).unwrap();
    let _ = a;

    let existing = v.existing_fingerprints().unwrap();
    let candidate = v.fingerprint_for(&secret, &OtpConfig::default()).unwrap();
    assert!(existing
        .iter()
        .any(|fp| clovakey_core::dedupe::fingerprints_match(fp, &candidate)));

    // A different secret does not collide.
    let other = base32::decode_secret("JBSWY3DPEHPK3PXP").unwrap();
    let other_fp = v.fingerprint_for(&other, &OtpConfig::default()).unwrap();
    assert!(!existing
        .iter()
        .any(|fp| clovakey_core::dedupe::fingerprints_match(fp, &other_fp)));
}

#[test]
fn export_account_uri_round_trips() {
    let v = temp_vault();
    v.setup(None).unwrap();
    let secret = base32::decode_secret(SEED_B32).unwrap();
    let acct = v
        .add_account(&totp_account("kevin@example.com"), &secret)
        .unwrap();
    let uri = v.export_account_uri(&acct.id).unwrap();
    assert!(uri.starts_with("otpauth://totp/"));
    let parsed = clovakey_core::otp::uri::parse(&uri).unwrap();
    assert_eq!(&parsed.secret[..], &secret[..]);
    assert_eq!(parsed.issuer.as_deref(), Some("GitHub"));
}

#[test]
fn backup_export_restore_cross_vault() {
    use clovakey_core::backup;

    // Source vault with two accounts and a group.
    let src = temp_vault();
    src.setup(None).unwrap();
    let g = src.create_group("Work").unwrap();
    let secret = base32::decode_secret(SEED_B32).unwrap();
    let a = src
        .add_account(&totp_account("kevin@example.com"), &secret)
        .unwrap();
    src.update_account(
        &a.id,
        &AccountPatch {
            group_id: Some(Some(g.id.clone())),
            favorite: Some(true),
            ..Default::default()
        },
    )
    .unwrap();
    let other = base32::decode_secret("JBSWY3DPEHPK3PXP").unwrap();
    src.add_account(&totp_account("second"), &other).unwrap();

    // Export → seal → bytes.
    let payload = src.export_payload().unwrap();
    let bytes = backup::seal_payload(&payload, "portable-backup-pw").unwrap();

    // Wrong password fails.
    assert!(backup::open_payload(&bytes, "nope").is_err());

    // Restore into a brand-new vault (simulating another machine).
    let dst = temp_vault();
    dst.setup(Some("different-local-pw")).unwrap();
    let restored = backup::open_payload(&bytes, "portable-backup-pw").unwrap();
    let summary = dst
        .import_payload(&restored, backup::DuplicatePolicy::Skip)
        .unwrap();
    assert_eq!(summary.imported, 2);
    assert_eq!(summary.groups_created, 1);

    // The restored account generates the same code as the source.
    let dst_accounts = dst.list_accounts().unwrap();
    let kevin = dst_accounts
        .iter()
        .find(|a| a.account_name == "kevin@example.com")
        .unwrap();
    assert!(kevin.favorite);
    assert!(kevin.group_id.is_some());
    let src_code = src.generate_code(&a.id).unwrap().code;
    let dst_code = dst.generate_code(&kevin.id).unwrap().code;
    assert_eq!(src_code, dst_code);

    // Re-importing the same backup with Skip imports nothing new.
    let again = backup::open_payload(&bytes, "portable-backup-pw").unwrap();
    let summary2 = dst
        .import_payload(&again, backup::DuplicatePolicy::Skip)
        .unwrap();
    assert_eq!(summary2.imported, 0);
    assert_eq!(summary2.skipped, 2);
}

#[test]
fn passphrase_enable_disable_change() {
    let v = temp_vault();
    v.setup(None).unwrap();
    // keychain → passphrase
    v.enable_passphrase("first").unwrap();
    v.lock();
    v.unlock(Some("first")).unwrap();
    // change
    v.change_passphrase("first", "second").unwrap();
    v.lock();
    assert!(v.unlock(Some("first")).is_err());
    v.unlock(Some("second")).unwrap();
    // passphrase → keychain
    v.disable_passphrase().unwrap();
    v.lock();
    v.unlock(None).unwrap();
}
