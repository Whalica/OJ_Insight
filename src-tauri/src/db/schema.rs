const INITIAL_SCHEMA: &str = r#"
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
CREATE TABLE IF NOT EXISTS solved_inventory (
  platform TEXT NOT NULL,
  account TEXT NOT NULL,
  problem_key TEXT NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY(platform,account,problem_key)
);
CREATE INDEX IF NOT EXISTS idx_solved_inventory_problem ON solved_inventory(platform,problem_key);
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
"#;

fn initialize_schema(tx: &rusqlite::Transaction<'_>) -> Result<(), String> {
    tx.execute_batch(INITIAL_SCHEMA)
        .map_err(|e| format!("初始化 SQLite 失败：{e}"))
}
