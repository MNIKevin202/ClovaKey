//! Schema migrations, tracked with SQLite's `PRAGMA user_version`.
//!
//! Migrations are applied in order and are idempotent with respect to the
//! recorded version. New migrations append to [`MIGRATIONS`] and bump
//! [`LATEST_VERSION`]; existing entries must never be edited once shipped.

use crate::error::Result;
use rusqlite::Connection;

/// The schema version this build expects.
pub const LATEST_VERSION: i64 = 1;

/// Ordered migration steps. Index `i` migrates from version `i` to `i + 1`.
const MIGRATIONS: &[&str] = &[
    // ── v0 → v1: initial schema ──────────────────────────────────────────
    r#"
    CREATE TABLE vault_meta (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );

    CREATE TABLE groups (
        id         TEXT PRIMARY KEY,
        name       TEXT NOT NULL,
        sort_order INTEGER NOT NULL DEFAULT 0
    );

    CREATE TABLE accounts (
        id           TEXT PRIMARY KEY,
        issuer       TEXT,
        account_name TEXT NOT NULL,
        otp_type     TEXT NOT NULL,
        algorithm    TEXT NOT NULL,
        digits       INTEGER NOT NULL,
        period       INTEGER NOT NULL,
        counter      INTEGER NOT NULL DEFAULT 0,
        secret_nonce BLOB NOT NULL,
        secret_ct    BLOB NOT NULL,
        secret_fp    BLOB,
        group_id     TEXT REFERENCES groups(id) ON DELETE SET NULL,
        favorite     INTEGER NOT NULL DEFAULT 0,
        sort_order   INTEGER NOT NULL DEFAULT 0,
        icon         TEXT,
        created_at   INTEGER NOT NULL,
        updated_at   INTEGER NOT NULL
    );

    CREATE INDEX idx_accounts_sort ON accounts(sort_order);
    CREATE INDEX idx_accounts_group ON accounts(group_id);
    CREATE INDEX idx_accounts_fp ON accounts(secret_fp);

    CREATE TABLE settings (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
    "#,
];

/// Bring `conn` up to [`LATEST_VERSION`], applying any pending migrations.
pub fn migrate(conn: &Connection) -> Result<()> {
    // Enforce foreign keys for this connection.
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;

    let mut version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

    while (version as usize) < MIGRATIONS.len() {
        let sql = MIGRATIONS[version as usize];
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(sql)?;
        version += 1;
        tx.execute_batch(&format!("PRAGMA user_version = {version};"))?;
        tx.commit()?;
    }

    debug_assert_eq!(version, LATEST_VERSION);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_fresh_db_to_latest() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        let v: i64 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(v, LATEST_VERSION);
    }

    #[test]
    fn migrate_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        // Running again applies nothing and does not error.
        migrate(&conn).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='accounts'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
}
