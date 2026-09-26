fn run_migrations(
    tx: &rusqlite::Transaction<'_>,
    had_multi_accounts: bool,
) -> Result<(), String> {
    ensure_column(tx, "submissions", "account", "TEXT NOT NULL DEFAULT ''")?;
    ensure_column(tx, "submissions", "source", "TEXT NOT NULL DEFAULT 'oj'")?;
    ensure_column(tx, "submissions", "source_day", "TEXT")?;
    ensure_column(
        tx,
        "submissions",
        "participant_type",
        "TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(tx, "submissions", "tags", "TEXT NOT NULL DEFAULT '[]'")?;
    ensure_column(tx, "daily_aggregates", "epoch_second", "INTEGER")?;
    ensure_column(tx, "daily_aggregates_accounts", "epoch_second", "INTEGER")?;
    // Import legacy single-account caches only on the first upgrade. Repeating
    // this import can assign a deleted account's cache to another account.
    if !had_multi_accounts {
        tx.execute_batch("
INSERT OR IGNORE INTO account_entries SELECT platform,account,secret,updated_at FROM accounts WHERE TRIM(account)<>'';
UPDATE submissions SET account=COALESCE((SELECT account FROM accounts a WHERE a.platform=submissions.platform),'') WHERE account='';
INSERT OR IGNORE INTO daily_aggregates_accounts SELECT d.platform,a.account,d.day,d.metric,d.count,d.note,d.epoch_second FROM daily_aggregates d JOIN accounts a ON a.platform=d.platform WHERE TRIM(a.account)<>'';
INSERT OR IGNORE INTO difficulty_stats_accounts SELECT d.platform,a.account,d.label,d.count,d.sort_order FROM difficulty_stats d JOIN accounts a ON a.platform=d.platform WHERE TRIM(a.account)<>'';
INSERT OR IGNORE INTO platform_stats_accounts SELECT p.platform,a.account,p.key,p.value FROM platform_stats p JOIN accounts a ON a.platform=p.platform WHERE TRIM(a.account)<>'';
").map_err(|e| e.to_string())?;
    }
    // Luogu training-list cookies are supplied for a single import, not stored with accounts.
    tx.execute("UPDATE account_entries SET secret='' WHERE platform='luogu' AND secret<>''", []).map_err(|e| e.to_string())?;
    tx.execute("UPDATE accounts SET secret='' WHERE platform='luogu' AND secret<>''", []).map_err(|e| e.to_string())?;
    // Submission IDs are not necessarily unique across accounts/sites.
    let account_pk: i64 = tx
        .query_row(
            "SELECT pk FROM pragma_table_info('submissions') WHERE name='account'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if account_pk == 0 {
        tx.execute_batch("
ALTER TABLE submissions RENAME TO submissions_v4;
CREATE TABLE submissions (
 platform TEXT NOT NULL, account TEXT NOT NULL DEFAULT '', source TEXT NOT NULL DEFAULT 'oj',
 source_day TEXT, submission_id TEXT NOT NULL, problem_key TEXT NOT NULL,
 problem_id TEXT NOT NULL DEFAULT '', problem_name TEXT NOT NULL DEFAULT '',
 problem_url TEXT NOT NULL DEFAULT '', epoch_second INTEGER NOT NULL,
 language TEXT NOT NULL DEFAULT '', difficulty TEXT, participant_type TEXT NOT NULL DEFAULT '', tags TEXT NOT NULL DEFAULT '[]',
 PRIMARY KEY(platform,account,submission_id)
);
INSERT INTO submissions(platform,account,source,source_day,submission_id,problem_key,problem_id,problem_name,problem_url,epoch_second,language,difficulty,participant_type,tags) SELECT platform,account,source,source_day,submission_id,problem_key,problem_id,problem_name,problem_url,epoch_second,language,difficulty,'',tags FROM submissions_v4;
DROP TABLE submissions_v4;
CREATE INDEX idx_submissions_platform_time ON submissions(platform,epoch_second);
CREATE INDEX idx_submissions_platform_problem ON submissions(platform,problem_key);
").map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub(super) fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<(), String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|e| e.to_string())?;
    let columns = stmt
        .query_map([], |r| r.get::<_, String>(1))
        .map_err(|e| e.to_string())?;
    for name in columns {
        if name.map_err(|e| e.to_string())? == column {
            return Ok(());
        }
    }
    conn.execute(
        &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
        [],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
