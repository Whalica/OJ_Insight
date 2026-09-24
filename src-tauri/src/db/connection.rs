use std::path::Path;

use rusqlite::Connection;

use super::{get_accounts, replace_all_accounts};
use crate::models::PLATFORMS;

include!("schema.rs");
include!("migrations.rs");

pub fn open(path: &Path) -> Result<Connection, String> {
    let mut conn = Connection::open(path).map_err(|e| format!("打开 SQLite 失败：{e}"))?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| e.to_string())?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| e.to_string())?;
    let had_multi_accounts: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='account_entries')",
        [], |row| row.get(0),
    ).map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    initialize_schema(&tx)?;
    run_migrations(&tx, had_multi_accounts)?;
    tx.commit().map_err(|e| e.to_string())?;
    for p in PLATFORMS {
        conn.execute("INSERT OR IGNORE INTO sync_state(platform) VALUES(?)", [p])
            .map_err(|e| e.to_string())?;
    }
    // Repair orphaned records left by older account-removal implementations.
    // Only records with no configured owner are removed; retained IDs survive.
    let accounts = get_accounts(&conn)?;
    replace_all_accounts(&mut conn, &accounts)?;
    conn.execute("UPDATE sync_state SET status='idle',message='上次同步被中断，请重新同步' WHERE status='syncing'", [])
        .map_err(|e| e.to_string())?;
    Ok(conn)
}
