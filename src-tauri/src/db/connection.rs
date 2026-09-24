use std::path::Path;

use rusqlite::Connection;

use super::{get_accounts, replace_all_accounts};
use crate::models::PLATFORMS;

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
    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS accounts (
  platform TEXT PRIMARY KEY,
  account TEXT NOT NULL DEFAULT '',
  secret TEXT NOT NULL DEFAULT '',
  updated_at INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS account_entries (
  platform TEXT NOT NULL,
  account TEXT NOT NULL,
  secret TEXT NOT NULL DEFAULT '',
  updated_at INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY(platform, account)
);
CREATE TABLE IF NOT EXISTS watched_people (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  platform TEXT NOT NULL,
  account TEXT NOT NULL,
  nickname TEXT NOT NULL DEFAULT '',
  relationship TEXT NOT NULL DEFAULT '',
  secret TEXT NOT NULL DEFAULT '',
  enabled INTEGER NOT NULL DEFAULT 1,
  initialized INTEGER NOT NULL DEFAULT 0,
  status TEXT NOT NULL DEFAULT 'idle',
  message TEXT NOT NULL DEFAULT '',
  cursor_epoch INTEGER NOT NULL DEFAULT 0,
  last_checked INTEGER,
  last_success INTEGER,
  created_at INTEGER NOT NULL DEFAULT 0,
  updated_at INTEGER NOT NULL DEFAULT 0,
  UNIQUE(platform, account)
);
CREATE TABLE IF NOT EXISTS watched_submissions (
  person_id INTEGER NOT NULL,
  platform TEXT NOT NULL,
  account TEXT NOT NULL DEFAULT '',
  source TEXT NOT NULL DEFAULT 'oj',
  source_day TEXT,
  submission_id TEXT NOT NULL,
  problem_key TEXT NOT NULL,
  problem_id TEXT NOT NULL DEFAULT '',
  problem_name TEXT NOT NULL DEFAULT '',
  problem_url TEXT NOT NULL DEFAULT '',
  epoch_second INTEGER NOT NULL,
  language TEXT NOT NULL DEFAULT '',
  difficulty TEXT,
  PRIMARY KEY(person_id, submission_id),
  FOREIGN KEY(person_id) REFERENCES watched_people(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_watched_submissions_person_time
  ON watched_submissions(person_id, epoch_second);
CREATE TABLE IF NOT EXISTS watched_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  person_id INTEGER NOT NULL,
  platform TEXT NOT NULL,
  account TEXT NOT NULL DEFAULT '',
  nickname TEXT NOT NULL DEFAULT '',
  relationship TEXT NOT NULL DEFAULT '',
  submission_id TEXT NOT NULL,
  problem_id TEXT NOT NULL DEFAULT '',
  problem_name TEXT NOT NULL DEFAULT '',
  problem_url TEXT NOT NULL DEFAULT '',
  epoch_second INTEGER NOT NULL,
  language TEXT NOT NULL DEFAULT '',
  difficulty TEXT,
  created_at INTEGER NOT NULL,
  dismissed INTEGER NOT NULL DEFAULT 0,
  UNIQUE(person_id, submission_id),
  FOREIGN KEY(person_id) REFERENCES watched_people(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_watched_events_state_time
  ON watched_events(dismissed, created_at);
CREATE TABLE IF NOT EXISTS watched_notifications (
  event_id INTEGER PRIMARY KEY,
  created_at INTEGER NOT NULL,
  FOREIGN KEY(event_id) REFERENCES watched_events(id) ON DELETE CASCADE
);
INSERT OR IGNORE INTO watched_notifications(event_id,created_at)
  SELECT id,created_at FROM watched_events WHERE dismissed=0;
CREATE TABLE IF NOT EXISTS account_sync_state (
  platform TEXT NOT NULL,
  account TEXT NOT NULL,
  cursor_epoch INTEGER NOT NULL DEFAULT 0,
  last_success INTEGER,
  PRIMARY KEY(platform, account)
);
CREATE TABLE IF NOT EXISTS submissions (
  platform TEXT NOT NULL,
  account TEXT NOT NULL DEFAULT '',
  source TEXT NOT NULL DEFAULT 'oj',
  source_day TEXT,
  submission_id TEXT NOT NULL,
  problem_key TEXT NOT NULL,
  problem_id TEXT NOT NULL DEFAULT '',
  problem_name TEXT NOT NULL DEFAULT '',
  problem_url TEXT NOT NULL DEFAULT '',
  epoch_second INTEGER NOT NULL,
  language TEXT NOT NULL DEFAULT '',
  difficulty TEXT,
  participant_type TEXT NOT NULL DEFAULT '',
  tags TEXT NOT NULL DEFAULT '[]',
  PRIMARY KEY(platform, submission_id)
);
CREATE INDEX IF NOT EXISTS idx_submissions_platform_time ON submissions(platform, epoch_second);
CREATE INDEX IF NOT EXISTS idx_submissions_platform_problem ON submissions(platform, problem_key);
CREATE TABLE IF NOT EXISTS daily_counts (
  platform TEXT NOT NULL,
  day TEXT NOT NULL,
  metric TEXT NOT NULL,
  count INTEGER NOT NULL,
  PRIMARY KEY(platform, day, metric)
);
CREATE INDEX IF NOT EXISTS idx_daily_counts_range ON daily_counts(platform, metric, day);
CREATE TABLE IF NOT EXISTS daily_aggregates (
  platform TEXT NOT NULL,
  day TEXT NOT NULL,
  metric TEXT NOT NULL,
  count INTEGER NOT NULL,
  note TEXT NOT NULL DEFAULT '',
  epoch_second INTEGER,
  PRIMARY KEY(platform, day, metric)
);
CREATE TABLE IF NOT EXISTS daily_aggregates_accounts (
  platform TEXT NOT NULL,
  account TEXT NOT NULL,
  day TEXT NOT NULL,
  metric TEXT NOT NULL,
  count INTEGER NOT NULL,
  note TEXT NOT NULL DEFAULT '',
  epoch_second INTEGER,
  PRIMARY KEY(platform, account, day, metric)
);
CREATE TABLE IF NOT EXISTS platform_stats (
  platform TEXT NOT NULL,
  key TEXT NOT NULL,
  value TEXT NOT NULL,
  PRIMARY KEY(platform, key)
);
CREATE TABLE IF NOT EXISTS platform_stats_accounts (
  platform TEXT NOT NULL,
  account TEXT NOT NULL,
  key TEXT NOT NULL,
  value TEXT NOT NULL,
  PRIMARY KEY(platform, account, key)
);
CREATE TABLE IF NOT EXISTS difficulty_stats (
  platform TEXT NOT NULL,
  label TEXT NOT NULL,
  count INTEGER NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY(platform, label)
);
CREATE TABLE IF NOT EXISTS difficulty_stats_accounts (
  platform TEXT NOT NULL,
  account TEXT NOT NULL,
  label TEXT NOT NULL,
  count INTEGER NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY(platform, account, label)
);
CREATE TABLE IF NOT EXISTS knowledge_stats_accounts (
  platform TEXT NOT NULL,
  account TEXT NOT NULL,
  axis TEXT NOT NULL,
  count INTEGER NOT NULL,
  PRIMARY KEY(platform, account, axis)
);
CREATE TABLE IF NOT EXISTS rating_history (
  platform TEXT NOT NULL,
  account TEXT NOT NULL,
  contest_id TEXT NOT NULL,
  contest_name TEXT NOT NULL DEFAULT '',
  epoch_second INTEGER NOT NULL,
  old_rating INTEGER NOT NULL,
  new_rating INTEGER NOT NULL,
  rank INTEGER,
  PRIMARY KEY(platform, account, contest_id)
);
CREATE INDEX IF NOT EXISTS idx_rating_history_account_time
  ON rating_history(platform, account, epoch_second);
CREATE TABLE IF NOT EXISTS sync_state (
  platform TEXT PRIMARY KEY,
  account TEXT NOT NULL DEFAULT '',
  status TEXT NOT NULL DEFAULT 'idle',
  message TEXT NOT NULL DEFAULT '',
  last_attempt INTEGER,
  last_success INTEGER,
  cursor_epoch INTEGER NOT NULL DEFAULT 0
);
"#,
    )
    .map_err(|e| format!("初始化 SQLite 失败：{e}"))?;
    ensure_column(&tx, "submissions", "account", "TEXT NOT NULL DEFAULT ''")?;
    ensure_column(&tx, "submissions", "source", "TEXT NOT NULL DEFAULT 'oj'")?;
    ensure_column(&tx, "submissions", "source_day", "TEXT")?;
    ensure_column(
        &tx,
        "submissions",
        "participant_type",
        "TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(&tx, "submissions", "tags", "TEXT NOT NULL DEFAULT '[]'")?;
    ensure_column(&tx, "daily_aggregates", "epoch_second", "INTEGER")?;
    ensure_column(&tx, "daily_aggregates_accounts", "epoch_second", "INTEGER")?;
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

fn ensure_column(
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
