use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use chrono::{DateTime, Duration, FixedOffset, NaiveDate, TimeZone, Utc};
use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::models::*;

pub fn open(path: &Path) -> Result<Connection, String> {
    let conn = Connection::open(path).map_err(|e| format!("打开 SQLite 失败：{e}"))?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| e.to_string())?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| e.to_string())?;
    conn.execute_batch(
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
CREATE TABLE IF NOT EXISTS account_sync_state (
  platform TEXT NOT NULL,
  account TEXT NOT NULL,
  cursor_epoch INTEGER NOT NULL DEFAULT 0,
  last_success INTEGER,
  PRIMARY KEY(platform, account)
);
CREATE TABLE IF NOT EXISTS submissions (
  platform TEXT NOT NULL,
  submission_id TEXT NOT NULL,
  problem_key TEXT NOT NULL,
  problem_id TEXT NOT NULL DEFAULT '',
  problem_name TEXT NOT NULL DEFAULT '',
  problem_url TEXT NOT NULL DEFAULT '',
  epoch_second INTEGER NOT NULL,
  language TEXT NOT NULL DEFAULT '',
  difficulty TEXT,
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
  PRIMARY KEY(platform, day, metric)
);
CREATE TABLE IF NOT EXISTS daily_aggregates_accounts (
  platform TEXT NOT NULL,
  account TEXT NOT NULL,
  day TEXT NOT NULL,
  metric TEXT NOT NULL,
  count INTEGER NOT NULL,
  note TEXT NOT NULL DEFAULT '',
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
    ensure_column(&conn, "submissions", "account", "TEXT NOT NULL DEFAULT ''")?;
    ensure_column(&conn, "submissions", "source", "TEXT NOT NULL DEFAULT 'oj'")?;
    conn.execute("INSERT OR IGNORE INTO account_entries(platform,account,secret,updated_at) SELECT platform,account,secret,updated_at FROM accounts WHERE TRIM(account)<>''", []).map_err(|e| e.to_string())?;
    conn.execute("UPDATE submissions SET account=COALESCE((SELECT account FROM accounts a WHERE a.platform=submissions.platform),'') WHERE account=''", []).map_err(|e| e.to_string())?;
    conn.execute("INSERT OR IGNORE INTO daily_aggregates_accounts(platform,account,day,metric,count,note) SELECT d.platform,COALESCE(a.account,''),d.day,d.metric,d.count,d.note FROM daily_aggregates d LEFT JOIN accounts a ON a.platform=d.platform", []).map_err(|e| e.to_string())?;
    conn.execute("INSERT OR IGNORE INTO difficulty_stats_accounts(platform,account,label,count,sort_order) SELECT d.platform,COALESCE(a.account,''),d.label,d.count,d.sort_order FROM difficulty_stats d LEFT JOIN accounts a ON a.platform=d.platform", []).map_err(|e| e.to_string())?;
    conn.execute("INSERT OR IGNORE INTO platform_stats_accounts(platform,account,key,value) SELECT p.platform,COALESCE(a.account,''),p.key,p.value FROM platform_stats p LEFT JOIN accounts a ON a.platform=p.platform", []).map_err(|e| e.to_string())?;
    for p in PLATFORMS {
        conn.execute("INSERT OR IGNORE INTO sync_state(platform) VALUES(?)", [p])
            .map_err(|e| e.to_string())?;
    }
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

pub fn get_accounts(conn: &Connection) -> Result<Vec<AccountConfig>, String> {
    let mut stmt = conn
        .prepare("SELECT platform, account, secret FROM account_entries ORDER BY CASE platform WHEN 'codeforces' THEN 1 WHEN 'atcoder' THEN 2 WHEN 'luogu' THEN 3 WHEN 'nowcoder' THEN 4 WHEN 'qoj' THEN 5 WHEN 'leetcode' THEN 6 ELSE 99 END, updated_at, account")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(AccountConfig {
                platform: r.get(0)?,
                account: r.get(1)?,
                secret: r.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

pub fn save_account(
    conn: &Connection,
    platform: &str,
    account: &str,
    secret: &str,
) -> Result<(), String> {
    let now = Utc::now().timestamp();
    if account.trim().is_empty() {
        conn.execute("DELETE FROM account_entries WHERE platform=?", [platform])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM accounts WHERE platform=?", [platform])
            .map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE sync_state SET account='' WHERE platform=?",
            [platform],
        )
        .map_err(|e| e.to_string())?;
        return Ok(());
    }
    conn.execute("INSERT INTO account_entries(platform,account,secret,updated_at) VALUES(?,?,?,?) ON CONFLICT(platform,account) DO UPDATE SET secret=excluded.secret,updated_at=excluded.updated_at", params![platform, account.trim(), secret.trim(), now]).map_err(|e| e.to_string())?;
    conn.execute("INSERT INTO accounts(platform,account,secret,updated_at) VALUES(?,?,?,?) ON CONFLICT(platform) DO UPDATE SET account=excluded.account,secret=excluded.secret,updated_at=excluded.updated_at", params![platform, account.trim(), secret.trim(), now]).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE sync_state SET account=? WHERE platform=?",
        params![account.trim(), platform],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn replace_accounts(
    conn: &mut Connection,
    platform: &str,
    accounts: &[AccountConfig],
) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM account_entries WHERE platform=?", [platform])
        .map_err(|e| e.to_string())?;
    let now = Utc::now().timestamp();
    let mut seen = HashSet::new();
    for (index, entry) in accounts.iter().enumerate() {
        let account = entry.account.trim();
        if account.is_empty() || !seen.insert(account.to_string()) {
            continue;
        }
        tx.execute(
            "INSERT INTO account_entries(platform,account,secret,updated_at) VALUES(?,?,?,?)",
            params![platform, account, entry.secret.trim(), now + index as i64],
        )
        .map_err(|e| e.to_string())?;
    }
    if let Some(first) = accounts.iter().find(|x| !x.account.trim().is_empty()) {
        tx.execute("INSERT INTO accounts(platform,account,secret,updated_at) VALUES(?,?,?,?) ON CONFLICT(platform) DO UPDATE SET account=excluded.account,secret=excluded.secret,updated_at=excluded.updated_at", params![platform,first.account.trim(),first.secret.trim(),now]).map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE sync_state SET account=? WHERE platform=?",
            params![first.account.trim(), platform],
        )
        .map_err(|e| e.to_string())?;
    } else {
        tx.execute("DELETE FROM accounts WHERE platform=?", [platform])
            .map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE sync_state SET account='' WHERE platform=?",
            [platform],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())
}

pub fn get_cursor(conn: &Connection, platform: &str, account: &str) -> Result<i64, String> {
    conn.query_row(
        "SELECT cursor_epoch FROM account_sync_state WHERE platform=? AND account=?",
        params![platform, account],
        |r| r.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())
    .map(|x| x.unwrap_or(0))
}

pub fn mark_syncing(conn: &Connection, platform: &str, account: &str) -> Result<(), String> {
    conn.execute("INSERT INTO sync_state(platform,account,status,message,last_attempt) VALUES(?,?, 'syncing','正在同步',?) ON CONFLICT(platform) DO UPDATE SET account=excluded.account,status='syncing',message='正在同步',last_attempt=excluded.last_attempt", params![platform, account, Utc::now().timestamp()]).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn mark_failed(
    conn: &Connection,
    platform: &str,
    account: &str,
    status: &str,
    message: &str,
) -> Result<(), String> {
    conn.execute("INSERT INTO sync_state(platform,account,status,message,last_attempt) VALUES(?,?,?,?,?) ON CONFLICT(platform) DO UPDATE SET account=excluded.account,status=excluded.status,message=excluded.message,last_attempt=excluded.last_attempt", params![platform, account, status, message, Utc::now().timestamp()]).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn apply_remote(conn: &mut Connection, remote: &RemoteData) -> Result<(i64, i64), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    if remote.replace_submissions {
        tx.execute(
            "DELETE FROM submissions WHERE platform=? AND account=?",
            params![remote.platform, remote.account],
        )
        .map_err(|e| e.to_string())?;
    }
    if remote.replace_aggregates {
        tx.execute(
            "DELETE FROM daily_aggregates_accounts WHERE platform=? AND account=?",
            params![remote.platform, remote.account],
        )
        .map_err(|e| e.to_string())?;
    }
    let mut inserted = 0_i64;
    let mut updated = 0_i64;
    for s in &remote.submissions {
        let exists: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM submissions WHERE platform=? AND submission_id=?)",
                params![s.platform, s.submission_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        tx.execute(r#"INSERT INTO submissions(platform,account,source,submission_id,problem_key,problem_id,problem_name,problem_url,epoch_second,language,difficulty)
VALUES(?,?,?,?,?,?,?,?,?,?,?) ON CONFLICT(platform,submission_id) DO UPDATE SET account=excluded.account,source=excluded.source,problem_key=excluded.problem_key,problem_id=excluded.problem_id,problem_name=excluded.problem_name,problem_url=excluded.problem_url,epoch_second=excluded.epoch_second,language=excluded.language,difficulty=excluded.difficulty"#,
            params![s.platform,s.account,s.source,s.submission_id,s.problem_key,s.problem_id,s.problem_name,s.problem_url,s.epoch_second,s.language,s.difficulty]).map_err(|e| e.to_string())?;
        if exists {
            updated += 1;
        } else {
            inserted += 1;
        }
    }
    for a in &remote.aggregates {
        tx.execute("INSERT INTO daily_aggregates_accounts(platform,account,day,metric,count,note) VALUES(?,?,?,?,?,?) ON CONFLICT(platform,account,day,metric) DO UPDATE SET count=excluded.count,note=excluded.note", params![remote.platform,remote.account,a.day,a.metric,a.count,a.note]).map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO daily_aggregates(platform,day,metric,count,note) VALUES(?,?,?,?,?) ON CONFLICT(platform,day,metric) DO UPDATE SET count=excluded.count,note=excluded.note", params![remote.platform,a.day,a.metric,a.count,a.note]).map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO daily_counts(platform,day,metric,count) VALUES(?,?,?,?) ON CONFLICT(platform,day,metric) DO UPDATE SET count=excluded.count", params![remote.platform,a.day,a.metric,a.count]).map_err(|e| e.to_string())?;
    }
    tx.execute(
        "DELETE FROM difficulty_stats_accounts WHERE platform=? AND account=?",
        params![remote.platform, remote.account],
    )
    .map_err(|e| e.to_string())?;
    for d in &remote.difficulty {
        tx.execute(
            "INSERT INTO difficulty_stats_accounts(platform,account,label,count,sort_order) VALUES(?,?,?,?,?)",
            params![remote.platform, remote.account, d.label, d.count, d.order],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO difficulty_stats(platform,label,count,sort_order) VALUES(?,?,?,?) ON CONFLICT(platform,label) DO UPDATE SET count=excluded.count,sort_order=excluded.sort_order",
            params![remote.platform, d.label, d.count, d.order],
        )
        .map_err(|e| e.to_string())?;
    }
    set_account_stat_tx(
        &tx,
        &remote.platform,
        &remote.account,
        "activity_only",
        if remote.activity_only { "1" } else { "0" },
    )?;
    set_account_stat_tx(
        &tx,
        &remote.platform,
        &remote.account,
        "notes",
        &serde_json::to_string(&remote.notes).unwrap_or_default(),
    )?;
    if let Some(solved) = remote.solved_count {
        set_account_stat_tx(
            &tx,
            &remote.platform,
            &remote.account,
            "solved_count",
            &solved.to_string(),
        )?;
        set_stat_tx(&tx, &remote.platform, "solved_count", &solved.to_string())?;
    } else if !remote.activity_only {
        tx.execute(
            "DELETE FROM platform_stats_accounts WHERE platform=? AND account=? AND key='solved_count'",
            params![remote.platform, remote.account],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM platform_stats WHERE platform=? AND key='solved_count'",
            [&remote.platform],
        )
        .map_err(|e| e.to_string())?;
    }
    if !remote.activity_only {
        recompute_raw_daily(&tx, &remote.platform)?;
    } else {
        recompute_aggregate_daily(&tx, &remote.platform)?;
    }
    let now = Utc::now().timestamp();
    let warning = remote
        .notes
        .iter()
        .find_map(|note| note.strip_prefix("警告："));
    let message = match warning {
        Some(warning) => format!("同步成功 · 新增 {inserted}，更新 {updated} · {warning}"),
        None => format!("同步成功 · 新增 {inserted}，更新 {updated}"),
    };
    tx.execute("INSERT INTO sync_state(platform,account,status,message,last_attempt,last_success,cursor_epoch) VALUES(?,?, 'ok', ?, ?, ?, ?) ON CONFLICT(platform) DO UPDATE SET account=excluded.account,status='ok',message=excluded.message,last_attempt=excluded.last_attempt,last_success=excluded.last_success,cursor_epoch=MAX(sync_state.cursor_epoch,excluded.cursor_epoch)", params![remote.platform,remote.account,message,now,now,remote.cursor_epoch]).map_err(|e| e.to_string())?;
    tx.execute("INSERT INTO account_sync_state(platform,account,cursor_epoch,last_success) VALUES(?,?,?,?) ON CONFLICT(platform,account) DO UPDATE SET cursor_epoch=MAX(account_sync_state.cursor_epoch,excluded.cursor_epoch),last_success=excluded.last_success", params![remote.platform,remote.account,remote.cursor_epoch,now]).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok((inserted, updated))
}

fn set_stat_tx(tx: &Transaction<'_>, platform: &str, key: &str, value: &str) -> Result<(), String> {
    tx.execute("INSERT INTO platform_stats(platform,key,value) VALUES(?,?,?) ON CONFLICT(platform,key) DO UPDATE SET value=excluded.value", params![platform,key,value]).map_err(|e| e.to_string())?;
    Ok(())
}

fn set_account_stat_tx(
    tx: &Transaction<'_>,
    platform: &str,
    account: &str,
    key: &str,
    value: &str,
) -> Result<(), String> {
    tx.execute("INSERT INTO platform_stats_accounts(platform,account,key,value) VALUES(?,?,?,?) ON CONFLICT(platform,account,key) DO UPDATE SET value=excluded.value", params![platform,account,key,value]).map_err(|e| e.to_string())?;
    Ok(())
}

fn recompute_aggregate_daily(tx: &Transaction<'_>, platform: &str) -> Result<(), String> {
    tx.execute("DELETE FROM daily_counts WHERE platform=?", [platform])
        .map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO daily_counts(platform,day,metric,count) SELECT platform,day,metric,SUM(count) FROM daily_aggregates_accounts WHERE platform=? GROUP BY platform,day,metric",
        [platform],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

fn recompute_raw_daily(tx: &Transaction<'_>, platform: &str) -> Result<(), String> {
    tx.execute("DELETE FROM daily_counts WHERE platform=?", [platform])
        .map_err(|e| e.to_string())?;
    let mut stmt = tx.prepare("SELECT problem_key, epoch_second FROM submissions WHERE platform=? ORDER BY epoch_second, submission_id").map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([platform], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })
        .map_err(|e| e.to_string())?;
    let mut first_seen = HashSet::new();
    let mut daily_seen = HashSet::new();
    let mut first: BTreeMap<String, i64> = BTreeMap::new();
    let mut unique: BTreeMap<String, i64> = BTreeMap::new();
    let mut subs: BTreeMap<String, i64> = BTreeMap::new();
    for row in rows {
        let (problem, ts) = row.map_err(|e| e.to_string())?;
        let day = day_utc8(ts);
        *subs.entry(day.clone()).or_default() += 1;
        if daily_seen.insert(format!("{day}\0{problem}")) {
            *unique.entry(day.clone()).or_default() += 1;
        }
        if first_seen.insert(problem) {
            *first.entry(day).or_default() += 1;
        }
    }
    insert_counts(tx, platform, "accepted_submissions", &subs)?;
    insert_counts(tx, platform, "activity", &subs)?;
    insert_counts(tx, platform, "daily_unique", &unique)?;
    insert_counts(tx, platform, "first_ac", &first)?;
    Ok(())
}

fn insert_counts(
    tx: &Transaction<'_>,
    platform: &str,
    metric: &str,
    map: &BTreeMap<String, i64>,
) -> Result<(), String> {
    for (day, count) in map {
        tx.execute(
            "INSERT INTO daily_counts(platform,day,metric,count) VALUES(?,?,?,?)",
            params![platform, day, metric, count],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn statuses(conn: &Connection) -> Result<Vec<SyncStatus>, String> {
    let mut stmt = conn.prepare("SELECT s.platform, COALESCE((SELECT GROUP_CONCAT(e.account,' · ') FROM account_entries e WHERE e.platform=s.platform),''), s.status,s.message,s.last_attempt,s.last_success, (SELECT COUNT(*) FROM submissions x WHERE x.platform=s.platform) + (SELECT COUNT(*) FROM daily_aggregates_accounts d WHERE d.platform=s.platform) FROM sync_state s ORDER BY CASE s.platform WHEN 'codeforces' THEN 1 WHEN 'atcoder' THEN 2 WHEN 'luogu' THEN 3 WHEN 'nowcoder' THEN 4 WHEN 'qoj' THEN 5 WHEN 'leetcode' THEN 6 ELSE 99 END").map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(SyncStatus {
                platform: r.get(0)?,
                account: r.get(1)?,
                status: r.get(2)?,
                message: r.get(3)?,
                last_attempt: r.get(4)?,
                last_success: r.get(5)?,
                cached_records: r.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

pub fn clear_platform(conn: &Connection, platform: &str) -> Result<(), String> {
    for sql in [
        "DELETE FROM submissions WHERE platform=?",
        "DELETE FROM daily_counts WHERE platform=?",
        "DELETE FROM daily_aggregates WHERE platform=?",
        "DELETE FROM daily_aggregates_accounts WHERE platform=?",
        "DELETE FROM platform_stats WHERE platform=?",
        "DELETE FROM platform_stats_accounts WHERE platform=?",
        "DELETE FROM difficulty_stats WHERE platform=?",
        "DELETE FROM difficulty_stats_accounts WHERE platform=?",
        "DELETE FROM account_sync_state WHERE platform=?",
    ] {
        conn.execute(sql, [platform]).map_err(|e| e.to_string())?;
    }
    conn.execute("UPDATE sync_state SET status='idle',message='本地记录已清空',last_attempt=NULL,last_success=NULL,cursor_epoch=0 WHERE platform=?",[platform]).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn clear_all(conn: &Connection) -> Result<(), String> {
    for p in PLATFORMS {
        clear_platform(conn, p)?;
    }
    Ok(())
}

pub fn day_utc8(ts: i64) -> String {
    let tz = FixedOffset::east_opt(8 * 3600).unwrap();
    DateTime::<Utc>::from_timestamp(ts, 0)
        .unwrap_or_else(Utc::now)
        .with_timezone(&tz)
        .format("%Y-%m-%d")
        .to_string()
}

fn platform_activity_only(conn: &Connection, platform: &str, account: Option<&str>) -> bool {
    let account = account.unwrap_or("");
    let raw_accounts: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM platform_stats_accounts WHERE platform=? AND key='activity_only' AND value='0' AND (?='' OR account=?)",
            params![platform, account, account],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if raw_accounts > 0 {
        return false;
    }
    conn.query_row(
        "SELECT COUNT(*) FROM platform_stats_accounts WHERE platform=? AND key='activity_only' AND value='1' AND (?='' OR account=?)",
        params![platform, account, account],
        |r| r.get::<_, i64>(0),
    )
    .unwrap_or(0)
        > 0
}

fn platform_solved_lifetime(
    conn: &Connection,
    platform: &str,
    account: Option<&str>,
    source: Option<&str>,
) -> Option<i64> {
    let account = account.unwrap_or("");
    let source = source.unwrap_or("");
    if source.is_empty() {
        let explicit: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(CAST(value AS INTEGER)),0) FROM platform_stats_accounts WHERE platform=? AND key='solved_count' AND (?='' OR account=?)",
                params![platform, account, account],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if explicit > 0 {
            return Some(explicit);
        }
    }
    conn.query_row(
        "SELECT COUNT(DISTINCT problem_key) FROM submissions WHERE platform=? AND (?='' OR account=?) AND (?='' OR source=?)",
        params![platform, account, account, source, source],
        |r| r.get(0),
    )
    .ok()
}

pub fn snapshot(
    conn: &Connection,
    platform: Option<&str>,
    start_day: Option<&str>,
    end_day: Option<&str>,
    metric: &str,
    account_filter: Option<&str>,
    source_filter: Option<&str>,
) -> Result<Snapshot, String> {
    let selected: Vec<&str> = match platform {
        Some(p) => vec![p],
        None => PLATFORMS.into_iter().collect(),
    };
    let mut combined: BTreeMap<String, i64> = BTreeMap::new();
    let mut warnings = Vec::new();
    let mut metric_available = false;
    let mut platforms = Vec::new();
    let mut recent = Vec::new();
    let mut difficulty = Vec::new();
    let mut difficulty_daily = Vec::new();
    let mut solved_range = 0_i64;
    let mut ac_sub_range = 0_i64;
    let mut career_solved = 0_i64;
    let mut career_ac_sub = 0_i64;
    let mut career_daily: BTreeMap<String, i64> = BTreeMap::new();
    let statuses_map: HashMap<String, SyncStatus> = statuses(conn)?
        .into_iter()
        .map(|s| (s.platform.clone(), s))
        .collect();

    for p in selected {
        let platform_account_filter = if platform == Some(p) {
            account_filter
        } else {
            None
        };
        let platform_source_filter = if platform == Some(p) {
            source_filter
        } else {
            None
        };
        let activity_only = platform_activity_only(conn, p, platform_account_filter);
        let account = if let Some(value) = platform_account_filter {
            value.to_string()
        } else {
            let mut stmt = conn
                .prepare("SELECT account FROM account_entries WHERE platform=? ORDER BY updated_at,account")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([p], |r| r.get::<_, String>(0))
                .map_err(|e| e.to_string())?;
            let mut names = Vec::new();
            for row in rows {
                names.push(row.map_err(|e| e.to_string())?);
            }
            names.join(" · ")
        };
        let status = statuses_map.get(p).cloned().unwrap_or(SyncStatus {
            platform: p.into(),
            account: account.clone(),
            status: "idle".into(),
            message: "".into(),
            last_attempt: None,
            last_success: None,
            cached_records: 0,
        });
        let daily = load_daily(
            conn,
            p,
            metric,
            start_day,
            end_day,
            platform_account_filter,
            platform_source_filter,
        )?;
        if !activity_only || metric == "activity" {
            metric_available = true;
        }
        for (day, count) in &daily {
            *combined.entry(day.clone()).or_default() += *count;
        }
        let first = load_daily(
            conn,
            p,
            "first_ac",
            start_day,
            end_day,
            platform_account_filter,
            platform_source_filter,
        )?;
        solved_range += first.iter().map(|x| x.1).sum::<i64>();
        let acs = load_daily(
            conn,
            p,
            "accepted_submissions",
            start_day,
            end_day,
            platform_account_filter,
            platform_source_filter,
        )?;
        ac_sub_range += acs.iter().map(|x| x.1).sum::<i64>();
        let active_days = daily.iter().filter(|x| x.1 > 0).count() as i64;
        let today_key = day_utc8(Utc::now().timestamp());
        let today_count = load_daily(
            conn,
            p,
            "activity",
            Some(&today_key),
            Some(&today_key),
            platform_account_filter,
            platform_source_filter,
        )?
        .first()
        .map(|item| item.1)
        .unwrap_or(0);
        let solved_lifetime =
            platform_solved_lifetime(conn, p, platform_account_filter, platform_source_filter);
        let account_value = platform_account_filter.unwrap_or("");
        let source_value = platform_source_filter.unwrap_or("");
        let ac_lifetime: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM submissions WHERE platform=? AND (?='' OR account=?) AND (?='' OR source=?)",
                params![p, account_value, account_value, source_value, source_value],
                |r| r.get(0),
            )
            .unwrap_or(0);
        career_solved += solved_lifetime.unwrap_or(0);
        career_ac_sub += ac_lifetime;
        for (day, count) in load_daily(
            conn,
            p,
            "activity",
            None,
            None,
            platform_account_filter,
            platform_source_filter,
        )? {
            *career_daily.entry(day).or_default() += count;
        }
        platforms.push(PlatformSummary {
            platform: p.into(),
            account,
            solved: solved_lifetime,
            accepted_submissions: ac_lifetime,
            active_days,
            today_count,
            last_success: status.last_success,
            status: status.status,
            message: status.message,
            activity_only,
            cached_records: status.cached_records,
            last_attempt: status.last_attempt,
        });
        if activity_only && metric != "activity" {
            warnings.push(format!(
                "{} 只有平台公开的日期活动计数，无法还原“{}”逐日口径。",
                platform_name(p),
                metric_label(metric)
            ));
        }
        recent.extend(load_recent(
            conn,
            p,
            start_day,
            end_day,
            20,
            platform_account_filter,
            platform_source_filter,
        )?);
        difficulty.extend(difficulty_for_platform(
            conn,
            p,
            start_day,
            end_day,
            platform_account_filter,
            platform_source_filter,
        )?);
        difficulty_daily.extend(difficulty_daily_for_platform(
            conn,
            p,
            start_day,
            end_day,
            platform_account_filter,
            platform_source_filter,
        )?);
    }
    recent.sort_by_key(|x| std::cmp::Reverse(x.epoch_second));
    recent.truncate(20);
    let daily_vec: Vec<DailyPoint> = combined
        .iter()
        .map(|(d, c)| DailyPoint {
            day: d.clone(),
            count: *c,
        })
        .collect();
    let stats = stats_for_map(&combined, solved_range, ac_sub_range, end_day);
    let career = stats_for_map(&career_daily, career_solved, career_ac_sub, None);
    Ok(Snapshot {
        stats,
        career,
        daily: daily_vec,
        platforms,
        difficulty,
        difficulty_daily,
        recent,
        metric_available,
        warnings,
    })
}

fn stats_for_map(
    map: &BTreeMap<String, i64>,
    solved: i64,
    accepted_submissions: i64,
    end: Option<&str>,
) -> SnapshotStats {
    let active_days = map.values().filter(|&&c| c > 0).count() as i64;
    let (longest, current) = streaks(map, end);
    let (peak_day, peak_count) = map
        .iter()
        .max_by_key(|(_, c)| *c)
        .map(|(d, c)| (Some(d.clone()), *c))
        .unwrap_or((None, 0));
    SnapshotStats {
        solved,
        accepted_submissions,
        active_days,
        longest_streak: longest,
        current_streak: current,
        peak_day,
        peak_count,
    }
}

fn load_daily(
    conn: &Connection,
    p: &str,
    metric: &str,
    start: Option<&str>,
    end: Option<&str>,
    account: Option<&str>,
    source: Option<&str>,
) -> Result<Vec<(String, i64)>, String> {
    let s = start.unwrap_or("0000-00-00");
    let e = end.unwrap_or("9999-99-99");
    if account.is_none() {
        let mut stmt = conn
            .prepare(
                "SELECT account FROM account_entries WHERE platform=? ORDER BY updated_at,account",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([p], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        let mut accounts = Vec::new();
        for row in rows {
            accounts.push(row.map_err(|e| e.to_string())?);
        }
        if !accounts.is_empty() {
            let mut combined = BTreeMap::new();
            for account in &accounts {
                for (day, count) in load_daily(conn, p, metric, start, end, Some(account), source)?
                {
                    *combined.entry(day).or_default() += count;
                }
            }
            return Ok(combined.into_iter().collect());
        }
    }
    if platform_activity_only(conn, p, account) {
        if metric != "activity" {
            return Ok(Vec::new());
        }
        let account = account.unwrap_or("");
        let mut stmt=conn.prepare("SELECT day,SUM(count) FROM daily_aggregates_accounts WHERE platform=? AND metric=? AND day>=? AND day<=? AND (?='' OR account=?) GROUP BY day ORDER BY day").map_err(|e|e.to_string())?;
        let rows = stmt
            .query_map(params![p, metric, s, e, account, account], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| e.to_string())?);
        }
        return Ok(out);
    }
    let account = account.unwrap_or("");
    let source = source.unwrap_or("");
    let mut stmt=conn.prepare("SELECT problem_key,epoch_second FROM submissions WHERE platform=? AND (?='' OR account=?) AND (?='' OR source=?) ORDER BY epoch_second,submission_id").map_err(|e|e.to_string())?;
    let rows = stmt
        .query_map(params![p, account, account, source, source], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })
        .map_err(|e| e.to_string())?;
    let mut first_seen = HashSet::new();
    let mut daily_seen = HashSet::new();
    let mut first = BTreeMap::new();
    let mut unique = BTreeMap::new();
    let mut subs = BTreeMap::new();
    for row in rows {
        let (problem, ts) = row.map_err(|e| e.to_string())?;
        let day = day_utc8(ts);
        *subs.entry(day.clone()).or_default() += 1;
        if daily_seen.insert(format!("{day}\0{problem}")) {
            *unique.entry(day.clone()).or_default() += 1;
        }
        if first_seen.insert(problem) {
            *first.entry(day).or_default() += 1;
        }
    }
    let chosen = match metric {
        "first_ac" => first,
        "daily_unique" => unique,
        "accepted_submissions" | "activity" => subs,
        _ => BTreeMap::new(),
    };
    Ok(chosen
        .into_iter()
        .filter(|(day, _)| day.as_str() >= s && day.as_str() <= e)
        .collect())
}

fn load_recent(
    conn: &Connection,
    p: &str,
    start: Option<&str>,
    end: Option<&str>,
    limit: i64,
    account: Option<&str>,
    source: Option<&str>,
) -> Result<Vec<Submission>, String> {
    let start_ts = day_start_epoch(start.unwrap_or("1970-01-01")).unwrap_or(0);
    let end_ts = day_end_epoch(end.unwrap_or("2999-12-31")).unwrap_or(i64::MAX / 2);
    let account = account.unwrap_or("");
    let source = source.unwrap_or("");
    let mut stmt=conn.prepare("SELECT platform,account,source,submission_id,problem_key,problem_id,problem_name,problem_url,epoch_second,language,difficulty FROM submissions WHERE platform=? AND epoch_second>=? AND epoch_second<=? AND (?='' OR account=?) AND (?='' OR source=?) ORDER BY epoch_second DESC LIMIT ?").map_err(|e|e.to_string())?;
    let rows = stmt
        .query_map(
            params![p, start_ts, end_ts, account, account, source, source, limit],
            row_submission,
        )
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| e.to_string())?);
    }
    Ok(out)
}
fn row_submission(r: &rusqlite::Row<'_>) -> rusqlite::Result<Submission> {
    Ok(Submission {
        platform: r.get(0)?,
        account: r.get(1)?,
        source: r.get(2)?,
        submission_id: r.get(3)?,
        problem_key: r.get(4)?,
        problem_id: r.get(5)?,
        problem_name: r.get(6)?,
        problem_url: r.get(7)?,
        epoch_second: r.get(8)?,
        language: r.get(9)?,
        difficulty: r.get(10)?,
    })
}

fn difficulty_for_platform(
    conn: &Connection,
    p: &str,
    start: Option<&str>,
    end: Option<&str>,
    account: Option<&str>,
    source: Option<&str>,
) -> Result<Vec<DifficultyBucket>, String> {
    let account = account.unwrap_or("");
    let source = source.unwrap_or("");
    let explicit_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM difficulty_stats_accounts WHERE platform=? AND (?='' OR account=?)",
            params![p, account, account],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let raw_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM submissions WHERE platform=? AND (?='' OR account=?) AND (?='' OR source=?)",
        params![p,account,account,source,source], |r| r.get(0)).unwrap_or(0);
    let explicit_only = platform_activity_only(
        conn,
        p,
        if account.is_empty() {
            None
        } else {
            Some(account)
        },
    );
    if explicit_count > 0 && (explicit_only || raw_count == 0) && source.is_empty() {
        let mut stmt=conn.prepare("SELECT label,SUM(count),sort_order FROM difficulty_stats_accounts WHERE platform=? AND (?='' OR account=?) GROUP BY label,sort_order ORDER BY sort_order,label").map_err(|e|e.to_string())?;
        let rows = stmt
            .query_map(params![p, account, account], |r| {
                Ok(DifficultyBucket {
                    platform: p.into(),
                    label: r.get(0)?,
                    count: r.get(1)?,
                    order: r.get(2)?,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        return Ok(out);
    }
    let mut stmt=conn.prepare("SELECT problem_key,epoch_second,difficulty FROM submissions WHERE platform=? AND (?='' OR account=?) AND (?='' OR source=?) ORDER BY epoch_second,submission_id").map_err(|e|e.to_string())?;
    let rows = stmt
        .query_map(params![p, account, account, source, source], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let s = start.unwrap_or("0000-00-00");
    let e = end.unwrap_or("9999-99-99");
    let mut seen = HashSet::new();
    let mut bucket: BTreeMap<(i64, String), i64> = BTreeMap::new();
    for row in rows {
        let (problem, ts, diff) = row.map_err(|e| e.to_string())?;
        if !seen.insert(problem) {
            continue;
        }
        let day = day_utc8(ts);
        if day.as_str() < s || day.as_str() > e {
            continue;
        }
        if let Some(d) = diff {
            let (order, label) = bucket_label(p, &d);
            *bucket.entry((order, label)).or_default() += 1;
        }
    }
    Ok(bucket
        .into_iter()
        .map(|((order, label), count)| DifficultyBucket {
            platform: p.into(),
            label,
            count,
            order,
        })
        .collect())
}
fn bucket_label(p: &str, d: &str) -> (i64, String) {
    if p == "codeforces" {
        if let Ok(x) = d.parse::<i64>() {
            let rating = (x / 100) * 100;
            return (rating, rating.to_string());
        }
    }
    if p == "atcoder" {
        if let Ok(x) = d.parse::<i64>() {
            let lo = (x.max(0) / 400) * 400;
            return (lo, format!("{}–{}", lo, lo + 399));
        }
    }
    if p == "nowcoder" {
        if let Ok(x) = d.parse::<i64>() {
            return (x, x.to_string());
        }
    }
    if p == "luogu" {
        let order = match d {
            "入门" | "1" => 1,
            "普及-" | "2" => 2,
            "普及" | "3" => 3,
            "普及+/提高-" | "4" => 4,
            "提高" | "5" => 5,
            "提高+/省选-" | "6" => 6,
            "省选/NOI-" | "7" => 7,
            "NOI/NOI+/CTS" | "8" => 8,
            _ => 0,
        };
        let label = match order {
            1 => "入门",
            2 => "普及-",
            3 => "普及",
            4 => "普及+/提高-",
            5 => "提高",
            6 => "提高+/省选-",
            7 => "省选/NOI-",
            8 => "NOI/NOI+/CTS",
            _ => d,
        };
        return (order, label.into());
    }
    (9999, d.into())
}

fn difficulty_daily_for_platform(
    conn: &Connection,
    p: &str,
    start: Option<&str>,
    end: Option<&str>,
    account: Option<&str>,
    source: Option<&str>,
) -> Result<Vec<DifficultyDayPoint>, String> {
    let s = start.unwrap_or("0000-00-00");
    let e = end.unwrap_or("9999-99-99");
    let account = account.unwrap_or("");
    let source = source.unwrap_or("");
    let mut stmt = conn.prepare("SELECT epoch_second,difficulty FROM submissions WHERE platform=? AND difficulty IS NOT NULL AND TRIM(difficulty)<>'' AND (?='' OR account=?) AND (?='' OR source=?) ORDER BY epoch_second").map_err(|e|e.to_string())?;
    let rows = stmt
        .query_map(params![p, account, account, source, source], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?;
    let mut days: BTreeMap<String, (i64, String)> = BTreeMap::new();
    for row in rows {
        let (ts, difficulty) = row.map_err(|e| e.to_string())?;
        let day = day_utc8(ts);
        if day.as_str() < s || day.as_str() > e {
            continue;
        }
        let (order, label) = bucket_label(p, &difficulty);
        if order == 9999 {
            continue;
        }
        match days.get(&day) {
            Some((current, _)) if *current >= order => {}
            _ => {
                days.insert(day, (order, label));
            }
        }
    }
    Ok(days
        .into_iter()
        .map(|(day, (order, label))| DifficultyDayPoint {
            platform: p.into(),
            day,
            label,
            order,
        })
        .collect())
}

fn streaks(map: &BTreeMap<String, i64>, end: Option<&str>) -> (i64, i64) {
    let days: Vec<NaiveDate> = map
        .iter()
        .filter(|(_, c)| **c > 0)
        .filter_map(|(d, _)| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        .collect();
    if days.is_empty() {
        return (0, 0);
    }
    let mut best = 1;
    let mut cur = 1;
    for i in 1..days.len() {
        if days[i - 1] + Duration::days(1) == days[i] {
            cur += 1;
            best = best.max(cur);
        } else {
            cur = 1;
        }
    }
    let today = Utc::now()
        .with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap())
        .date_naive();
    let target = end
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        .map(|d| d.min(today))
        .unwrap_or(today);
    let mut current = 0;
    let mut d = target;
    loop {
        let key = d.format("%Y-%m-%d").to_string();
        if map.get(&key).copied().unwrap_or(0) > 0 {
            current += 1;
            d -= Duration::days(1);
        } else {
            break;
        }
    }
    (best, current)
}

pub fn day_detail(
    conn: &Connection,
    day: &str,
    platform: Option<&str>,
    account: Option<&str>,
    source: Option<&str>,
) -> Result<DayDetail, String> {
    let start = day_start_epoch(day).ok_or_else(|| "日期格式错误".to_string())?;
    let end = day_end_epoch(day).ok_or_else(|| "日期格式错误".to_string())?;
    let mut items = Vec::new();
    let mut aggs = Vec::new();
    let ps: Vec<&str> = platform
        .map(|p| vec![p])
        .unwrap_or_else(|| PLATFORMS.into_iter().collect());
    for p in ps {
        let account = account.unwrap_or("");
        let source = source.unwrap_or("");
        let mut stmt=conn.prepare("SELECT platform,account,source,submission_id,problem_key,problem_id,problem_name,problem_url,epoch_second,language,difficulty FROM submissions WHERE platform=? AND epoch_second>=? AND epoch_second<=? AND (?='' OR account=?) AND (?='' OR source=?) ORDER BY epoch_second DESC").map_err(|e|e.to_string())?;
        let rows = stmt
            .query_map(
                params![p, start, end, account, account, source, source],
                row_submission,
            )
            .map_err(|e| e.to_string())?;
        for r in rows {
            items.push(r.map_err(|e| e.to_string())?);
        }
        let mut st=conn.prepare("SELECT platform,metric,SUM(count),COALESCE(GROUP_CONCAT(NULLIF(note,''),' · '),'') FROM daily_aggregates_accounts WHERE platform=? AND day=? AND (?='' OR account=?) GROUP BY platform,metric ORDER BY metric").map_err(|e|e.to_string())?;
        let rs = st
            .query_map(params![p, day, account, account], |r| {
                Ok(AggregateDetail {
                    platform: r.get(0)?,
                    metric: r.get(1)?,
                    count: r.get(2)?,
                    note: r.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?;
        for r in rs {
            aggs.push(r.map_err(|e| e.to_string())?);
        }
    }
    items.sort_by_key(|x| std::cmp::Reverse(x.epoch_second));
    Ok(DayDetail {
        day: day.into(),
        items,
        aggregates: aggs,
    })
}

fn day_start_epoch(day: &str) -> Option<i64> {
    let date = NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()?;
    let tz = FixedOffset::east_opt(8 * 3600)?;
    Some(
        tz.from_local_datetime(&date.and_hms_opt(0, 0, 0)?)
            .single()?
            .timestamp(),
    )
}
fn day_end_epoch(day: &str) -> Option<i64> {
    day_start_epoch(day).map(|x| x + 86399)
}
fn platform_name(p: &str) -> &'static str {
    match p {
        "codeforces" => "Codeforces",
        "atcoder" => "AtCoder",
        "luogu" => "Luogu",
        "nowcoder" => "NowCoder",
        "qoj" => "QOJ",
        "leetcode" => "LeetCode",
        _ => "OJ",
    }
}
fn metric_label(m: &str) -> &str {
    match m {
        "first_ac" => "首次 AC",
        "daily_unique" => "当日去重 AC",
        "accepted_submissions" => "AC 提交",
        "activity" => "平台活动",
        _ => m,
    }
}

#[cfg(test)]
mod tests {
    use super::{bucket_label, day_utc8};

    #[test]
    fn utc8_day_changes_at_china_midnight() {
        assert_eq!(day_utc8(1_767_196_799), "2025-12-31");
        assert_eq!(day_utc8(1_767_196_800), "2026-01-01");
    }

    #[test]
    fn difficulty_buckets_follow_platform_levels() {
        assert_eq!(bucket_label("codeforces", "1350"), (1300, "1300".into()));
        assert_eq!(bucket_label("luogu", "7"), (7, "省选/NOI-".into()));
        assert_eq!(bucket_label("luogu", "提高"), (5, "提高".into()));
    }
}
