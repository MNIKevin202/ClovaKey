use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    // Embed the build time (UNIX seconds) so the app can flag a system clock
    // that is set earlier than the build — a strong hint that TOTP codes will
    // fail. Non-sensitive.
    let build_ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    println!("cargo:rustc-env=CLOVAKEY_BUILD_TS={build_ts}");
    println!("cargo:rerun-if-changed=build.rs");

    tauri_build::build()
}
