//! Dev-only helper to populate a vault with SYNTHETIC demo accounts so the UI
//! can be viewed populated. Never uses real credentials. Not part of the app.
//!
//! Run: `cargo run -p clovakey-core --example seed`

use clovakey_core::model::{NewAccount, OtpConfig};
use clovakey_core::otp::{base32, Algorithm, OtpType};
use clovakey_core::Vault;

fn main() {
    let dir = dirs_app_data();
    std::fs::create_dir_all(&dir).unwrap();
    let db = dir.join("vault.db");
    println!("seeding vault at {}", db.display());

    let vault = Vault::open(&db, "com.clova.clovakey").expect("open vault");
    if !vault.is_initialized().unwrap() {
        vault.setup(Some("demo1234")).expect("setup");
    } else {
        vault.unlock(Some("demo1234")).expect("unlock");
    }

    if !vault.list_accounts().unwrap().is_empty() {
        println!("vault already has accounts; leaving as-is");
        return;
    }

    let work = vault.create_group("Work").unwrap();
    let personal = vault.create_group("Personal").unwrap();

    // (issuer, account, base32 secret [synthetic], favorite, group, otp)
    let demos: Vec<(&str, &str, &str, bool, Option<&str>, OtpConfig)> = vec![
        ("GitHub", "kevin@example.com", "JBSWY3DPEHPK3PXP", true, Some(&personal.id), totp(Algorithm::Sha1, 6)),
        ("Google", "kevin.poulos@gmail.com", "GEZDGNBVGY3TQOJQ", true, Some(&personal.id), totp(Algorithm::Sha1, 6)),
        ("Amazon Web Services", "kevin-admin", "KRSXG5CTMVRXEZLUGEZA", false, Some(&work.id), totp(Algorithm::Sha256, 8)),
        ("Microsoft", "kevin@outlook.com", "MFRGGZDFMZTWQ2LK", false, Some(&work.id), totp(Algorithm::Sha1, 6)),
        ("Stripe", "acct_1Qk2Zx", "NBSWY3DPEB3W64TM", false, Some(&work.id), totp(Algorithm::Sha1, 6)),
        ("Discord", "kevin", "ONSWG4TFOQFA====", false, Some(&personal.id), totp(Algorithm::Sha1, 6)),
        ("Cloudflare", "kevin@example.com", "PEBGC5DFEBSGKZDF", false, None, totp(Algorithm::Sha1, 6)),
        ("Proton", "kevin@proton.me", "QFXHIYLDMVXHIYLB", false, Some(&personal.id), totp(Algorithm::Sha512, 6)),
        ("Legacy VPN", "token-7781", "RFYHA3DPEHPK3PXP", false, Some(&work.id), hotp()),
    ];

    for (issuer, account, secret_b32, fav, group, otp) in demos {
        let secret = base32::decode_secret(secret_b32).expect("valid synthetic secret");
        let new = NewAccount {
            issuer: Some(issuer.to_string()),
            account_name: account.to_string(),
            otp,
            group_id: group.map(|s| s.to_string()),
            favorite: fav,
            icon: None,
        };
        vault.add_account(&new, &secret).expect("add");
        println!("  + {issuer} / {account}");
    }

    println!("done. {} accounts.", vault.list_accounts().unwrap().len());
}

fn totp(algorithm: Algorithm, digits: u8) -> OtpConfig {
    OtpConfig { otp_type: OtpType::Totp, algorithm, digits, period: 30, counter: 0 }
}

fn hotp() -> OtpConfig {
    OtpConfig { otp_type: OtpType::Hotp, algorithm: Algorithm::Sha1, digits: 6, period: 30, counter: 0 }
}

fn dirs_app_data() -> std::path::PathBuf {
    let home = std::env::var("HOME").expect("HOME");
    std::path::PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("com.clova.clovakey")
}
