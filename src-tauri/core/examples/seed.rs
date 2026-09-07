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

    struct Demo {
        issuer: &'static str,
        account: &'static str,
        secret: &'static str, // synthetic Base32
        favorite: bool,
        group: Option<String>,
        otp: OtpConfig,
    }

    let demos = vec![
        Demo {
            issuer: "GitHub",
            account: "kevin@example.com",
            secret: "JBSWY3DPEHPK3PXP",
            favorite: true,
            group: Some(personal.id.clone()),
            otp: totp(Algorithm::Sha1, 6),
        },
        Demo {
            issuer: "Google",
            account: "kevin.poulos@gmail.com",
            secret: "GEZDGNBVGY3TQOJQ",
            favorite: true,
            group: Some(personal.id.clone()),
            otp: totp(Algorithm::Sha1, 6),
        },
        Demo {
            issuer: "Amazon Web Services",
            account: "kevin-admin",
            secret: "KRSXG5CTMVRXEZLUGEZA",
            favorite: false,
            group: Some(work.id.clone()),
            otp: totp(Algorithm::Sha256, 8),
        },
        Demo {
            issuer: "Microsoft",
            account: "kevin@outlook.com",
            secret: "MFRGGZDFMZTWQ2LK",
            favorite: false,
            group: Some(work.id.clone()),
            otp: totp(Algorithm::Sha1, 6),
        },
        Demo {
            issuer: "Stripe",
            account: "acct_1Qk2Zx",
            secret: "NBSWY3DPEB3W64TM",
            favorite: false,
            group: Some(work.id.clone()),
            otp: totp(Algorithm::Sha1, 6),
        },
        Demo {
            issuer: "Discord",
            account: "kevin",
            secret: "ONSWG4TFOQFA",
            favorite: false,
            group: Some(personal.id.clone()),
            otp: totp(Algorithm::Sha1, 6),
        },
        Demo {
            issuer: "Cloudflare",
            account: "kevin@example.com",
            secret: "PEBGC5DFEBSGKZDF",
            favorite: false,
            group: None,
            otp: totp(Algorithm::Sha1, 6),
        },
        Demo {
            issuer: "Proton",
            account: "kevin@proton.me",
            secret: "QFXHIYLDMVXHIYLB",
            favorite: false,
            group: Some(personal.id.clone()),
            otp: totp(Algorithm::Sha512, 6),
        },
        Demo {
            issuer: "Legacy VPN",
            account: "token-7781",
            secret: "RFYHA3DPEHPK3PXP",
            favorite: false,
            group: Some(work.id.clone()),
            otp: hotp(),
        },
    ];

    for d in demos {
        let secret = base32::decode_secret(d.secret).expect("valid synthetic secret");
        let new = NewAccount {
            issuer: Some(d.issuer.to_string()),
            account_name: d.account.to_string(),
            otp: d.otp,
            group_id: d.group,
            favorite: d.favorite,
            icon: None,
        };
        vault.add_account(&new, &secret).expect("add");
        println!("  + {} / {}", d.issuer, d.account);
    }

    println!("done. {} accounts.", vault.list_accounts().unwrap().len());
}

fn totp(algorithm: Algorithm, digits: u8) -> OtpConfig {
    OtpConfig {
        otp_type: OtpType::Totp,
        algorithm,
        digits,
        period: 30,
        counter: 0,
    }
}

fn hotp() -> OtpConfig {
    OtpConfig {
        otp_type: OtpType::Hotp,
        algorithm: Algorithm::Sha1,
        digits: 6,
        period: 30,
        counter: 0,
    }
}

fn dirs_app_data() -> std::path::PathBuf {
    let home = std::env::var("HOME").expect("HOME");
    std::path::PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("com.clova.clovakey")
}
