use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{DateTime, Duration, LocalResult, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::models::*;

mod connection;

pub use connection::open;

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

pub fn get_watched_people(conn: &Connection) -> Result<Vec<WatchedPerson>, String> {
    let mut stmt = conn
        .prepare("SELECT id,platform,account,nickname,relationship,secret,enabled,initialized,status,message,last_checked,last_success FROM watched_people ORDER BY created_at,id")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(WatchedPerson {
                id: r.get(0)?,
                platform: r.get(1)?,
                account: r.get(2)?,
                nickname: r.get(3)?,
                relationship: r.get(4)?,
                secret: r.get(5)?,
                enabled: r.get::<_, i64>(6)? != 0,
                initialized: r.get::<_, i64>(7)? != 0,
                status: r.get(8)?,
                message: r.get(9)?,
                last_checked: r.get(10)?,
                last_success: r.get(11)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

fn read_watched_events(conn: &Connection, sql: &str, limit: i64) -> Result<Vec<WatchedAcEvent>, String> {
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([limit], |r| {
            Ok(WatchedAcEvent {
                id: r.get(0)?,
                person_id: r.get(1)?,
                platform: r.get(2)?,
                account: r.get(3)?,
                nickname: r.get(4)?,
                relationship: r.get(5)?,
                submission_id: r.get(6)?,
                problem_id: r.get(7)?,
                problem_name: r.get(8)?,
                problem_url: r.get(9)?,
                epoch_second: r.get(10)?,
                language: r.get(11)?,
                difficulty: r.get(12)?,
                created_at: r.get(13)?,
                dismissed: r.get::<_, i64>(14)? != 0,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn get_watched_events(conn: &Connection, retention: u32) -> Result<Vec<WatchedAcEvent>, String> {
    let limit = retention.clamp(1, 100) as i64;
    conn.execute(
        "DELETE FROM watched_events WHERE id NOT IN (SELECT id FROM watched_events ORDER BY created_at DESC,id DESC LIMIT ?)",
        [limit],
    ).map_err(|e| e.to_string())?;
    read_watched_events(
        conn,
        "SELECT id,person_id,platform,account,nickname,relationship,submission_id,problem_id,problem_name,problem_url,epoch_second,language,difficulty,created_at,dismissed FROM watched_events ORDER BY created_at DESC,id DESC LIMIT ?",
        limit,
    )
}

pub fn get_pending_watched_notifications(conn: &Connection) -> Result<Vec<WatchedAcEvent>, String> {
    read_watched_events(
        conn,
        "SELECT e.id,e.person_id,e.platform,e.account,e.nickname,e.relationship,e.submission_id,e.problem_id,e.problem_name,e.problem_url,e.epoch_second,e.language,e.difficulty,e.created_at,e.dismissed FROM watched_events e JOIN watched_notifications n ON n.event_id=e.id ORDER BY n.created_at DESC,e.id DESC LIMIT ?",
        100,
    )
}

pub fn save_watched_person(
    conn: &mut Connection,
    platform: &str,
    account: &str,
    nickname: &str,
    relationship: &str,
    secret: &str,
) -> Result<(), String> {
    save_watched_people(
        conn,
        nickname,
        relationship,
        &[WatchedBindingInput {
            platform: platform.to_string(),
            account: account.to_string(),
            secret: secret.to_string(),
        }],
    )
}

pub fn save_watched_people(
    conn: &mut Connection,
    nickname: &str,
    relationship: &str,
    bindings: &[WatchedBindingInput],
) -> Result<(), String> {
    if nickname.trim().is_empty() {
        return Err("称呼不能为空".into());
    }
    if bindings.is_empty() {
        return Err("请至少填写一个平台账号".into());
    }
    let now = Utc::now().timestamp();
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for binding in bindings {
        let platform = binding.platform.trim();
        let account = binding.account.trim();
        if platform.is_empty() || account.is_empty() {
            return Err("关系人的平台和账号不能为空".into());
        }
        let already_added: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM watched_people WHERE platform=?1 AND lower(trim(account))=lower(?2))",
                params![platform, account],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        if already_added {
            return Err(format!("该用户已经被添加了：{} · {}", platform, account));
        }
        tx.execute(
            "INSERT INTO watched_people(platform,account,nickname,relationship,secret,enabled,initialized,status,message,cursor_epoch,last_checked,last_success,created_at,updated_at) VALUES(?,?,?,?,?,1,0,'idle','尚未检查',0,NULL,NULL,?,?)",
            params![platform, account, nickname.trim(), relationship.trim(), binding.secret.trim(), now, now],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())
}

pub fn edit_watched_person(
    conn: &mut Connection,
    person_ids: &[i64],
    nickname: &str,
    relationship: &str,
    bindings: &[WatchedBindingInput],
) -> Result<(), String> {
    if nickname.trim().is_empty() {
        return Err("称呼不能为空".into());
    }
    if person_ids.is_empty() {
        return Err("关注账号不存在".into());
    }
    if bindings.is_empty() {
        return Err("请至少填写一个平台账号".into());
    }
    if bindings
        .iter()
        .any(|binding| binding.platform.trim().is_empty() || binding.account.trim().is_empty())
    {
        return Err("关系人的平台和账号不能为空".into());
    }
    let now = Utc::now().timestamp();
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let mut existing_accounts = Vec::with_capacity(person_ids.len());
    for person_id in person_ids {
        let existing: Option<(String, String)> = tx.query_row(
            "SELECT platform,account FROM watched_people WHERE id=?",
            [person_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional().map_err(|e| e.to_string())?;
        let Some((platform, account)) = existing else {
            return Err("关注账号不存在".into());
        };
        existing_accounts.push((*person_id, platform, account));
        tx.execute(
            "UPDATE watched_people SET nickname=?,relationship=?,updated_at=? WHERE id=?",
            params![nickname.trim(), relationship.trim(), now, person_id],
        ).map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE watched_events SET nickname=?,relationship=? WHERE person_id=?",
            params![nickname.trim(), relationship.trim(), person_id],
        ).map_err(|e| e.to_string())?;
    }
    for binding in bindings {
        let platform = binding.platform.trim();
        let account = binding.account.trim();
        if let Some((person_id, _, _)) = existing_accounts.iter().find(|(_, existing_platform, existing_account)| {
            existing_platform == platform && existing_account.eq_ignore_ascii_case(account)
        }) {
            tx.execute(
                "UPDATE watched_people SET secret=?,updated_at=? WHERE id=?",
                params![binding.secret.trim(), now, person_id],
            ).map_err(|e| e.to_string())?;
            continue;
        }
        if person_ids.len() == 1 && bindings.len() == 1 && existing_accounts[0].1 == platform {
            let person_id = existing_accounts[0].0;
            let already_added: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM watched_people WHERE platform=?1 AND lower(trim(account))=lower(?2) AND id<>?3)",
                    params![platform, account, person_id],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;
            if already_added {
                return Err(format!("该用户已经被添加了：{} · {}", platform, account));
            }
            tx.execute("DELETE FROM watched_submissions WHERE person_id=?", [person_id])
                .map_err(|e| e.to_string())?;
            tx.execute(
                "UPDATE watched_events SET submission_id='archived:' || id WHERE person_id=?",
                [person_id],
            )
            .map_err(|e| e.to_string())?;
            tx.execute(
                "UPDATE watched_people SET account=?,secret=?,initialized=0,status='idle',message='尚未检查',cursor_epoch=0,last_checked=NULL,last_success=NULL,updated_at=? WHERE id=?",
                params![account, binding.secret.trim(), now, person_id],
            )
            .map_err(|e| e.to_string())?;
            continue;
        }
        let already_added: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM watched_people WHERE platform=?1 AND lower(trim(account))=lower(?2))",
                params![platform, account],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        if already_added {
            return Err(format!("该用户已经被添加了：{} · {}", platform, account));
        }
        tx.execute(
            "INSERT INTO watched_people(platform,account,nickname,relationship,secret,enabled,initialized,status,message,cursor_epoch,last_checked,last_success,created_at,updated_at) VALUES(?,?,?,?,?,1,0,'idle','尚未检查',0,NULL,NULL,?,?)",
            params![platform, account, nickname.trim(), relationship.trim(), binding.secret.trim(), now, now],
        ).map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())
}

pub fn delete_watched_person(conn: &mut Connection, person_id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM watched_people WHERE id=?", [person_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn watched_cursor(conn: &Connection, person_id: i64) -> Result<i64, String> {
    conn.query_row(
        "SELECT cursor_epoch FROM watched_people WHERE id=?",
        [person_id],
        |r| r.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "关系人不存在".into())
}

pub fn mark_watched_checking(conn: &Connection, person_id: i64) -> Result<(), String> {
    let now = Utc::now().timestamp();
    conn.execute(
        "UPDATE watched_people SET status='checking',message='正在检查',last_checked=?,updated_at=? WHERE id=?",
        params![now, now, person_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn mark_watched_success(
    conn: &Connection,
    person_id: i64,
    cursor_epoch: i64,
    status: &str,
    message: &str,
) -> Result<(), String> {
    let now = Utc::now().timestamp();
    conn.execute(
        "UPDATE watched_people SET initialized=1,cursor_epoch=?,status=?,message=?,last_checked=?,last_success=?,updated_at=? WHERE id=?",
        params![cursor_epoch, status, message, now, now, now, person_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn mark_watched_failed(conn: &Connection, person_id: i64, message: &str) -> Result<(), String> {
    let now = Utc::now().timestamp();
    conn.execute(
        "UPDATE watched_people SET status='error',message=?,last_checked=?,updated_at=? WHERE id=?",
        params![message, now, now, person_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn apply_watched_remote(
    conn: &mut Connection,
    person_id: i64,
    remote: &RemoteData,
) -> Result<Vec<WatchedAcEvent>, String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let (platform, account, nickname, relationship, initialized): (String, String, String, String, i64) = tx
        .query_row(
            "SELECT platform,account,nickname,relationship,initialized FROM watched_people WHERE id=?",
            [person_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "关系人不存在，丢弃此次检查结果".to_string())?;
    if remote.platform != platform || remote.submissions.iter().any(|s| s.platform != platform) {
        return Err("检查数据的平台归属不一致".into());
    }

    let baseline = initialized == 0;
    let now = Utc::now().timestamp();
    let mut events = Vec::new();
    for submission in &remote.submissions {
        let exists: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM watched_submissions WHERE person_id=? AND submission_id=?)",
                params![person_id, submission.submission_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO watched_submissions(person_id,platform,account,source,source_day,submission_id,problem_key,problem_id,problem_name,problem_url,epoch_second,language,difficulty) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?) ON CONFLICT(person_id,submission_id) DO UPDATE SET platform=excluded.platform,account=excluded.account,source=excluded.source,source_day=excluded.source_day,problem_key=excluded.problem_key,problem_id=excluded.problem_id,problem_name=excluded.problem_name,problem_url=excluded.problem_url,epoch_second=excluded.epoch_second,language=excluded.language,difficulty=COALESCE(excluded.difficulty,watched_submissions.difficulty)",
            params![person_id, platform, account, submission.source, submission.source_day, submission.submission_id, submission.problem_key, submission.problem_id, submission.problem_name, submission.problem_url, submission.epoch_second, submission.language, submission.difficulty],
        )
        .map_err(|e| e.to_string())?;
        if baseline || exists {
            continue;
        }
        let inserted = tx.execute(
            "INSERT OR IGNORE INTO watched_events(person_id,platform,account,nickname,relationship,submission_id,problem_id,problem_name,problem_url,epoch_second,language,difficulty,created_at,dismissed) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,0)",
            params![person_id, platform, account, nickname, relationship, submission.submission_id, submission.problem_id, submission.problem_name, submission.problem_url, submission.epoch_second, submission.language, submission.difficulty, now],
        ).map_err(|e| e.to_string())?;
        if inserted > 0 {
            let event_id = tx.last_insert_rowid();
            tx.execute(
                "INSERT INTO watched_notifications(event_id,created_at) VALUES(?,?)",
                params![event_id, now],
            ).map_err(|e| e.to_string())?;
            events.push(WatchedAcEvent {
                id: event_id,
                person_id,
                platform: platform.clone(),
                account: account.clone(),
                nickname: nickname.clone(),
                relationship: relationship.clone(),
                submission_id: submission.submission_id.clone(),
                problem_id: submission.problem_id.clone(),
                problem_name: submission.problem_name.clone(),
                problem_url: submission.problem_url.clone(),
                epoch_second: submission.epoch_second,
                language: submission.language.clone(),
                difficulty: submission.difficulty.clone(),
                created_at: now,
                dismissed: false,
            });
        }
    }
    tx.execute(
        "UPDATE watched_people SET initialized=1,cursor_epoch=?,updated_at=? WHERE id=?",
        params![remote.cursor_epoch, now, person_id],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(events)
}

pub fn dismiss_watched_event(conn: &Connection, event_id: i64) -> Result<(), String> {
    conn.execute(
        "UPDATE watched_events SET dismissed=1 WHERE id=?",
        [event_id],
    )
    .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM watched_notifications WHERE event_id=?", [event_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn save_account(
    conn: &mut Connection,
    platform: &str,
    account: &str,
    secret: &str,
) -> Result<(), String> {
    let mut entries: Vec<_> = get_accounts(conn)?
        .into_iter()
        .filter(|entry| entry.platform == platform)
        .collect();
    if account.trim().is_empty() {
        entries.clear();
    } else if let Some(entry) = entries
        .iter_mut()
        .find(|entry| entry.account == account.trim())
    {
        entry.secret = secret.trim().into();
    } else {
        entries.push(AccountConfig {
            platform: platform.into(),
            account: account.trim().into(),
            secret: secret.trim().into(),
        });
    }
    replace_accounts(conn, platform, &entries)
}

pub fn replace_all_accounts(
    conn: &mut Connection,
    accounts: &[AccountConfig],
) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for platform in PLATFORMS {
        let entries: Vec<_> = accounts
            .iter()
            .filter(|entry| entry.platform == platform)
            .cloned()
            .collect();
        replace_accounts_tx(&tx, platform, &entries)?;
    }
    tx.commit().map_err(|e| e.to_string())
}

pub fn replace_accounts(
    conn: &mut Connection,
    platform: &str,
    accounts: &[AccountConfig],
) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    replace_accounts_tx(&tx, platform, accounts)?;
    tx.commit().map_err(|e| e.to_string())
}

fn replace_accounts_tx(
    tx: &Transaction<'_>,
    platform: &str,
    accounts: &[AccountConfig],
) -> Result<(), String> {
    let previous: HashSet<String> = {
        let mut stmt = tx
            .prepare("SELECT account FROM account_entries WHERE platform=?")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([platform], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())?
    };
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
    let mut removed = 0;
    for table in [
        "submissions",
        "daily_aggregates_accounts",
        "difficulty_stats_accounts",
        "knowledge_stats_accounts",
        "platform_stats_accounts",
        "rating_history",
        "account_sync_state",
    ] {
        removed += tx.execute(&format!(
            "DELETE FROM {table} WHERE platform=? AND NOT EXISTS (SELECT 1 FROM account_entries e WHERE e.platform={table}.platform AND e.account={table}.account)"
        ), [platform]).map_err(|e| e.to_string())?;
    }
    // Retired single-account caches must never reappear on restart/downgrade.
    for table in ["daily_aggregates", "difficulty_stats", "platform_stats"] {
        tx.execute(&format!("DELETE FROM {table} WHERE platform=?"), [platform])
            .map_err(|e| e.to_string())?;
    }
    if previous != seen || removed > 0 {
        if platform_activity_only(tx, platform, None) {
            recompute_aggregate_daily(tx, platform)?;
        } else {
            recompute_raw_daily(tx, platform)?;
        }
        tx.execute(
            "UPDATE sync_state SET status='idle',message='账号已更新；移除账号的本地记录已清理',last_attempt=NULL,
             last_success=(SELECT MAX(last_success) FROM account_sync_state WHERE platform=?),
             cursor_epoch=COALESCE((SELECT MAX(cursor_epoch) FROM account_sync_state WHERE platform=?),0) WHERE platform=?",
            params![platform,platform,platform],
        ).map_err(|e| e.to_string())?;
    }
    Ok(())
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
    let configured: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM account_entries WHERE platform=? AND account=?)",
            params![remote.platform, remote.account],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if !configured {
        return Err("账号已移除，丢弃此次同步结果".into());
    }
    if remote
        .submissions
        .iter()
        .any(|s| s.platform != remote.platform || s.account != remote.account)
    {
        return Err("同步数据的账号归属不一致".into());
    }
    let previous_aggregates = if remote.activity_only && !remote.aggregates.is_empty() {
        let mut stmt = tx
            .prepare("SELECT day,metric,count FROM daily_aggregates_accounts WHERE platform=? AND account=?")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![remote.platform, remote.account], |row| {
                Ok((
                    (row.get::<_, String>(0)?, row.get::<_, String>(1)?),
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut values = HashMap::new();
        for row in rows {
            let (key, count) = row.map_err(|e| e.to_string())?;
            values.insert(key, count);
        }
        values
    } else {
        HashMap::new()
    };
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
    let mut submission_inserted = 0_i64;
    let mut submission_updated = 0_i64;
    for s in &remote.submissions {
        let exists: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM submissions WHERE platform=? AND account=? AND submission_id=?)",
                params![s.platform, s.account, s.submission_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let tags = serde_json::to_string(&s.tags).unwrap_or_else(|_| "[]".into());
        let changed = tx.execute(r#"INSERT INTO submissions(platform,account,source,source_day,submission_id,problem_key,problem_id,problem_name,problem_url,epoch_second,language,difficulty,participant_type,tags)
VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?) ON CONFLICT(platform,account,submission_id) DO UPDATE SET account=excluded.account,source=excluded.source,source_day=excluded.source_day,problem_key=excluded.problem_key,problem_id=excluded.problem_id,problem_name=excluded.problem_name,problem_url=excluded.problem_url,epoch_second=excluded.epoch_second,language=excluded.language,difficulty=COALESCE(excluded.difficulty,submissions.difficulty),participant_type=CASE WHEN excluded.participant_type='' THEN submissions.participant_type ELSE excluded.participant_type END,tags=CASE WHEN excluded.tags='[]' THEN submissions.tags ELSE excluded.tags END
WHERE submissions.source IS NOT excluded.source
   OR submissions.source_day IS NOT excluded.source_day
   OR submissions.problem_key IS NOT excluded.problem_key
   OR submissions.problem_id IS NOT excluded.problem_id
   OR submissions.problem_name IS NOT excluded.problem_name
   OR submissions.problem_url IS NOT excluded.problem_url
   OR submissions.epoch_second IS NOT excluded.epoch_second
   OR submissions.language IS NOT excluded.language
   OR (excluded.difficulty IS NOT NULL AND submissions.difficulty IS NOT excluded.difficulty)
   OR (excluded.participant_type<>'' AND submissions.participant_type IS NOT excluded.participant_type)
   OR (excluded.tags<>'[]' AND submissions.tags IS NOT excluded.tags)"#,
            params![s.platform,s.account,s.source,s.source_day,s.submission_id,s.problem_key,s.problem_id,s.problem_name,s.problem_url,s.epoch_second,s.language,s.difficulty,s.participant_type,tags]).map_err(|e| e.to_string())?;
        if exists && changed > 0 {
            submission_updated += 1;
        } else if !exists && changed > 0 {
            submission_inserted += 1;
        }
    }
    let mut aggregate_inserted = 0_i64;
    let mut aggregate_updated = 0_i64;
    for a in &remote.aggregates {
        if remote.activity_only {
            let previous = previous_aggregates
                .get(&(a.day.clone(), a.metric.clone()))
                .copied()
                .unwrap_or(0);
            let delta = a.count - previous;
            if delta > 0 {
                aggregate_inserted += delta;
            } else if delta < 0 {
                aggregate_updated += 1;
            }
        }
        tx.execute("INSERT INTO daily_aggregates_accounts(platform,account,day,metric,count,note,epoch_second) VALUES(?,?,?,?,?,?,?) ON CONFLICT(platform,account,day,metric) DO UPDATE SET count=excluded.count,note=excluded.note,epoch_second=excluded.epoch_second", params![remote.platform,remote.account,a.day,a.metric,a.count,a.note,a.epoch_second]).map_err(|e| e.to_string())?;
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
    }
    if let Some(knowledge) = &remote.knowledge {
        tx.execute(
            "DELETE FROM knowledge_stats_accounts WHERE platform=? AND account=?",
            params![remote.platform, remote.account],
        )
        .map_err(|e| e.to_string())?;
        for item in knowledge {
            tx.execute(
                "INSERT INTO knowledge_stats_accounts(platform,account,axis,count) VALUES(?,?,?,?)",
                params![remote.platform, remote.account, item.axis, item.count],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    if let Some(ratings) = &remote.ratings {
        tx.execute(
            "DELETE FROM rating_history WHERE platform=? AND account=?",
            params![remote.platform, remote.account],
        )
        .map_err(|e| e.to_string())?;
        for rating in ratings {
            tx.execute(
                "INSERT INTO rating_history(platform,account,contest_id,contest_name,epoch_second,old_rating,new_rating,rank) VALUES(?,?,?,?,?,?,?,?) ON CONFLICT(platform,account,contest_id) DO UPDATE SET contest_name=excluded.contest_name,epoch_second=excluded.epoch_second,old_rating=excluded.old_rating,new_rating=excluded.new_rating,rank=excluded.rank",
                params![remote.platform, remote.account, rating.contest_id, rating.contest_name, rating.epoch_second, rating.old_rating, rating.new_rating, rating.rank],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    if remote.ratings.is_some() {
        set_account_stat_tx(
            &tx,
            &remote.platform,
            &remote.account,
            "rating_synced_at",
            &Utc::now().timestamp().to_string(),
        )?;
    }
    if remote.platform == "codeforces" && remote.replace_submissions {
        set_account_stat_tx(
            &tx,
            &remote.platform,
            &remote.account,
            "metadata_backfill_v1",
            "1",
        )?;
    }
    if remote.platform == "nowcoder" && remote.replace_submissions {
        set_account_stat_tx(
            &tx,
            &remote.platform,
            &remote.account,
            "tracker_difficulty_backfill_v2",
            "1",
        )?;
    }
    if let Some(display_name) = remote
        .display_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        set_account_stat_tx(
            &tx,
            &remote.platform,
            &remote.account,
            "display_name",
            display_name,
        )?;
    }
    set_account_stat_tx(
        &tx,
        &remote.platform,
        &remote.account,
        "rating_stale",
        if remote.ratings.is_some() { "0" } else { "1" },
    )?;
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
    let (inserted, updated) = if remote.activity_only && !remote.aggregates.is_empty() {
        (aggregate_inserted, aggregate_updated)
    } else {
        (submission_inserted, submission_updated)
    };
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
        let day = day_in_time_zone(ts, "Asia/Shanghai");
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

pub fn clear_platform(conn: &mut Connection, platform: &str) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    clear_platform_tx(&tx, platform)?;
    tx.commit().map_err(|e| e.to_string())
}

fn clear_platform_tx(conn: &Connection, platform: &str) -> Result<(), String> {
    for sql in [
        "DELETE FROM submissions WHERE platform=?",
        "DELETE FROM daily_counts WHERE platform=?",
        "DELETE FROM daily_aggregates WHERE platform=?",
        "DELETE FROM daily_aggregates_accounts WHERE platform=?",
        "DELETE FROM platform_stats WHERE platform=?",
        "DELETE FROM platform_stats_accounts WHERE platform=?",
        "DELETE FROM difficulty_stats WHERE platform=?",
        "DELETE FROM difficulty_stats_accounts WHERE platform=?",
        "DELETE FROM knowledge_stats_accounts WHERE platform=?",
        "DELETE FROM rating_history WHERE platform=?",
        "DELETE FROM account_sync_state WHERE platform=?",
    ] {
        conn.execute(sql, [platform]).map_err(|e| e.to_string())?;
    }
    conn.execute("UPDATE sync_state SET status='idle',message='本地记录已清空',last_attempt=NULL,last_success=NULL,cursor_epoch=0 WHERE platform=?",[platform]).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn clear_all(conn: &mut Connection) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for p in PLATFORMS {
        clear_platform_tx(&tx, p)?;
    }
    tx.commit().map_err(|e| e.to_string())
}

fn parse_time_zone(value: &str) -> Tz {
    value.parse::<Tz>().unwrap_or(chrono_tz::Asia::Shanghai)
}

pub fn day_in_time_zone(ts: i64, time_zone: &str) -> String {
    let tz = parse_time_zone(time_zone);
    DateTime::<Utc>::from_timestamp(ts, 0)
        .unwrap_or_else(Utc::now)
        .with_timezone(&tz)
        .format("%Y-%m-%d")
        .to_string()
}

fn today_in_time_zone(time_zone: &str) -> NaiveDate {
    Utc::now()
        .with_timezone(&parse_time_zone(time_zone))
        .date_naive()
}

fn local_day_start(date: NaiveDate, time_zone: &str) -> Option<i64> {
    let tz = parse_time_zone(time_zone);
    for hour in 0..=6 {
        let local = date.and_hms_opt(hour, 0, 0)?;
        match tz.from_local_datetime(&local) {
            LocalResult::Single(value) => return Some(value.timestamp()),
            LocalResult::Ambiguous(first, second) => {
                return Some(first.timestamp().min(second.timestamp()))
            }
            LocalResult::None => {}
        }
    }
    None
}

fn day_epoch_range(day: &str, time_zone: &str) -> Option<(i64, i64)> {
    let date = NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()?;
    let start = local_day_start(date, time_zone)?;
    let next = local_day_start(date.succ_opt()?, time_zone)?;
    Some((start, next.saturating_sub(1)))
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
        "SELECT COUNT(*) FROM (SELECT account,problem_key FROM submissions WHERE platform=? AND (?='' OR account=?) AND (?='' OR source=?) GROUP BY account,problem_key)",
        params![platform, account, account, source, source],
        |r| r.get(0),
    )
    .ok()
}

fn ratings_for_platform(
    conn: &Connection,
    platform: &str,
    account: Option<&str>,
) -> Result<Vec<RatingSummary>, String> {
    let account_filter = account.unwrap_or("");
    let mut stmt = conn
        .prepare(
            "SELECT account,contest_id,contest_name,epoch_second,old_rating,new_rating,rank FROM rating_history WHERE platform=? AND (?='' OR account=?) ORDER BY account,epoch_second,contest_id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![platform, account_filter, account_filter], |row| {
            Ok((
                row.get::<_, String>(0)?,
                RatingHistoryPoint {
                    contest_id: row.get(1)?,
                    contest_name: row.get(2)?,
                    epoch_second: row.get(3)?,
                    old_rating: row.get(4)?,
                    new_rating: row.get(5)?,
                    rank: row.get(6)?,
                },
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut grouped: BTreeMap<String, Vec<RatingHistoryPoint>> = BTreeMap::new();
    for row in rows {
        let (account, point) = row.map_err(|e| e.to_string())?;
        grouped.entry(account).or_default().push(point);
    }
    let mut summaries = Vec::new();
    for (account, history) in grouped {
        if let Some(last) = history.last() {
            let last_updated = conn.query_row("SELECT value FROM platform_stats_accounts WHERE platform=? AND account=? AND key='rating_synced_at'", params![platform,account], |row| row.get::<_,String>(0))
                .optional().map_err(|e| e.to_string())?.and_then(|value| value.parse::<i64>().ok());
            let stale = conn.query_row("SELECT value FROM platform_stats_accounts WHERE platform=? AND account=? AND key='rating_stale'", params![platform,account], |row| row.get::<_,String>(0))
                .optional().map_err(|e| e.to_string())?.as_deref() != Some("0");
            let display_name = conn.query_row("SELECT value FROM platform_stats_accounts WHERE platform=? AND account=? AND key='display_name'", params![platform,account], |row| row.get::<_,String>(0))
                .optional().map_err(|e| e.to_string())?.filter(|value| !value.trim().is_empty()).unwrap_or_else(|| account.clone());
            summaries.push(RatingSummary {
                last_updated,
                stale,
                platform: platform.to_string(),
                account,
                display_name,
                current: last.new_rating,
                maximum: history
                    .iter()
                    .map(|point| point.new_rating)
                    .max()
                    .unwrap_or(last.new_rating),
                last_change: last.new_rating - last.old_rating,
                contest_count: history.len() as i64,
                last_contest_epoch: last.epoch_second,
                history,
            });
        }
    }
    Ok(summaries)
}

pub fn snapshot(
    conn: &Connection,
    platform: Option<&str>,
    start_day: Option<&str>,
    end_day: Option<&str>,
    metric: &str,
    account_filter: Option<&str>,
    source_filter: Option<&str>,
    time_zone: &str,
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
    let mut nowcoder_daily_difficulty = Vec::new();
    let mut difficulty_daily = Vec::new();
    let mut knowledge = Vec::new();
    let mut ratings = Vec::new();
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
            time_zone,
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
            time_zone,
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
            time_zone,
        )?;
        ac_sub_range += acs.iter().map(|x| x.1).sum::<i64>();
        let active_days = daily.iter().filter(|x| x.1 > 0).count() as i64;
        let today_key = day_in_time_zone(Utc::now().timestamp(), time_zone);
        let today_count = load_daily(
            conn,
            p,
            "activity",
            Some(&today_key),
            Some(&today_key),
            platform_account_filter,
            platform_source_filter,
            time_zone,
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
            time_zone,
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
            None,
            None,
            20,
            platform_account_filter,
            platform_source_filter,
            time_zone,
        )?);
        let difficulty_source_filter = if p == "nowcoder" && platform_source_filter.is_none() {
            Some("oj")
        } else {
            platform_source_filter
        };
        difficulty.extend(difficulty_for_platform(
            conn,
            p,
            start_day,
            end_day,
            platform_account_filter,
            difficulty_source_filter,
        )?);
        if platform == Some("nowcoder") && platform_source_filter.is_none() {
            nowcoder_daily_difficulty.extend(difficulty_for_platform(
                conn,
                p,
                start_day,
                end_day,
                platform_account_filter,
                Some("daily"),
            )?);
        }
        difficulty_daily.extend(difficulty_daily_for_platform(
            conn,
            p,
            start_day,
            end_day,
            platform_account_filter,
            platform_source_filter,
            time_zone,
        )?);
        knowledge.extend(knowledge_for_platform(conn, p, platform_account_filter)?);
        ratings.extend(ratings_for_platform(conn, p, platform_account_filter)?);
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
    let stats = stats_for_map(&combined, solved_range, ac_sub_range, end_day, time_zone);
    let career = stats_for_map(&career_daily, career_solved, career_ac_sub, None, time_zone);
    Ok(Snapshot {
        stats,
        career,
        daily: daily_vec,
        platforms,
        difficulty,
        nowcoder_daily_difficulty,
        difficulty_daily,
        knowledge,
        ratings,
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
    time_zone: &str,
) -> SnapshotStats {
    let active_days = map.values().filter(|&&c| c > 0).count() as i64;
    let (longest, current) = streaks(map, end, time_zone);
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
    time_zone: &str,
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
                for (day, count) in load_daily(
                    conn,
                    p,
                    metric,
                    start,
                    end,
                    Some(account),
                    source,
                    time_zone,
                )? {
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
        let mut stmt=conn.prepare("SELECT day,epoch_second,count FROM daily_aggregates_accounts WHERE platform=? AND metric=? AND (?='' OR account=?) ORDER BY day").map_err(|e|e.to_string())?;
        let rows = stmt
            .query_map(params![p, metric, account, account], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<i64>>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut out = BTreeMap::new();
        for row in rows {
            let (source_day, epoch, count) = row.map_err(|e| e.to_string())?;
            let day = epoch
                .map(|value| day_in_time_zone(value, time_zone))
                .unwrap_or(source_day);
            if day.as_str() >= s && day.as_str() <= e {
                *out.entry(day).or_default() += count;
            }
        }
        return Ok(out.into_iter().collect());
    }
    let account = account.unwrap_or("");
    let source = source.unwrap_or("");
    let mut stmt=conn.prepare("SELECT problem_key,epoch_second,source_day FROM submissions WHERE platform=? AND (?='' OR account=?) AND (?='' OR source=?) ORDER BY epoch_second,submission_id").map_err(|e|e.to_string())?;
    let rows = stmt
        .query_map(params![p, account, account, source, source], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut first_seen = HashSet::new();
    let mut daily_seen = HashSet::new();
    let mut first = BTreeMap::new();
    let mut unique = BTreeMap::new();
    let mut subs = BTreeMap::new();
    for row in rows {
        let (problem, ts, source_day) = row.map_err(|e| e.to_string())?;
        let day = source_day.unwrap_or_else(|| day_in_time_zone(ts, time_zone));
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
    time_zone: &str,
) -> Result<Vec<Submission>, String> {
    let start_ts = start
        .and_then(|day| day_epoch_range(day, time_zone).map(|range| range.0))
        .unwrap_or(0);
    let end_ts = end
        .and_then(|day| day_epoch_range(day, time_zone).map(|range| range.1))
        .unwrap_or(i64::MAX / 2);
    let account = account.unwrap_or("");
    let source = source.unwrap_or("");
    let mut stmt=conn.prepare("SELECT platform,account,source,source_day,submission_id,problem_key,problem_id,problem_name,problem_url,epoch_second,language,difficulty,participant_type,tags FROM submissions WHERE platform=? AND ((source_day IS NULL AND epoch_second>=? AND epoch_second<=?) OR (source_day IS NOT NULL AND source_day>=? AND source_day<=?)) AND (?='' OR account=?) AND (?='' OR source=?) ORDER BY epoch_second DESC LIMIT ?").map_err(|e|e.to_string())?;
    let start_day = start.unwrap_or("0000-00-00");
    let end_day = end.unwrap_or("9999-99-99");
    let rows = stmt
        .query_map(
            params![
                p, start_ts, end_ts, start_day, end_day, account, account, source, source, limit
            ],
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
        source_day: r.get(3)?,
        submission_id: r.get(4)?,
        problem_key: r.get(5)?,
        problem_id: r.get(6)?,
        problem_name: r.get(7)?,
        problem_url: r.get(8)?,
        epoch_second: r.get(9)?,
        language: r.get(10)?,
        difficulty: r.get(11)?,
        participant_type: r.get(12)?,
        tags: serde_json::from_str(&r.get::<_, String>(13)?).unwrap_or_default(),
    })
}

const KNOWLEDGE_AXES: [&str; 8] = [
    "基础与模拟",
    "数据结构",
    "图论与树",
    "动态规划",
    "数学",
    "字符串",
    "搜索与构造",
    "贪心与思维",
];

fn knowledge_axis(tag: &str) -> Option<&'static str> {
    let tag = tag.trim().to_lowercase();
    if tag.is_empty() {
        return None;
    }
    if [
        "data structures",
        "data structure",
        "array",
        "hash",
        "stack",
        "queue",
        "heap",
        "linked list",
        "segment tree",
        "fenwick",
        "dsu",
        "数据结构",
    ]
    .iter()
    .any(|value| tag.contains(value))
    {
        return Some("数据结构");
    }
    if [
        "graph",
        "tree",
        "shortest path",
        "mst",
        "topological",
        "图论",
        "树",
    ]
    .iter()
    .any(|value| tag.contains(value))
    {
        return Some("图论与树");
    }
    if ["dynamic programming", "dp", "动态规划"]
        .iter()
        .any(|value| tag == *value || tag.contains(value))
    {
        return Some("动态规划");
    }
    if [
        "math",
        "number theory",
        "combinatorics",
        "geometry",
        "probability",
        "数学",
        "几何",
    ]
    .iter()
    .any(|value| tag.contains(value))
    {
        return Some("数学");
    }
    if ["string", "trie", "字符串"]
        .iter()
        .any(|value| tag.contains(value))
    {
        return Some("字符串");
    }
    if [
        "binary search",
        "brute force",
        "backtracking",
        "dfs",
        "bfs",
        "constructive",
        "search",
        "搜索",
        "构造",
    ]
    .iter()
    .any(|value| tag.contains(value))
    {
        return Some("搜索与构造");
    }
    if [
        "greedy",
        "two pointers",
        "sliding window",
        "divide and conquer",
        "sort",
        "贪心",
        "思维",
    ]
    .iter()
    .any(|value| tag.contains(value))
    {
        return Some("贪心与思维");
    }
    if [
        "implementation",
        "simulation",
        "basic",
        "基础",
        "模拟",
        "算法策略",
    ]
    .iter()
    .any(|value| tag.contains(value))
    {
        return Some("基础与模拟");
    }
    None
}

fn knowledge_level(platform: &str, difficulty: &str) -> Option<f64> {
    let label = difficulty.trim().to_lowercase();
    match platform {
        "codeforces" => label.parse::<f64>().ok().filter(|rating| *rating > 0.0),
        "leetcode" => match label.as_str() {
            "hard" | "困难" => Some(86.0),
            "medium" | "中等" => Some(65.0),
            "easy" | "简单" => Some(42.0),
            _ => None,
        },
        "qoj" if label.contains("gold") || label.contains('金') => Some(90.0),
        "qoj" if label.contains("silver") || label.contains('银') => Some(76.0),
        "qoj" if label.contains("bronze") || label.contains('铜') => Some(58.0),
        "qoj" if label.contains("iron") || label.contains('铁') => Some(38.0),
        _ => None,
    }
}

fn knowledge_recency_weight(epoch_second: i64) -> f64 {
    if epoch_second <= 0 {
        return 0.65;
    }
    let age_days = (Utc::now().timestamp() - epoch_second).max(0) as f64 / 86_400.0;
    0.65 + 0.35 * (-age_days / 730.0).exp()
}

fn weighted_quantile(items: &[(f64, f64)], quantile: f64) -> Option<f64> {
    let mut values: Vec<_> = items
        .iter()
        .copied()
        .filter(|(_, weight)| *weight > 0.0)
        .collect();
    values.sort_by(|left, right| {
        left.0
            .partial_cmp(&right.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let target = values.iter().map(|(_, weight)| weight).sum::<f64>() * quantile.clamp(0.0, 1.0);
    let mut seen = 0.0;
    for (value, weight) in &values {
        seen += weight;
        if seen >= target {
            return Some(*value);
        }
    }
    values.last().map(|item| item.0)
}

fn robust_knowledge_estimate(
    representative: f64,
    prior: f64,
    evidence: f64,
    platform: &str,
) -> f64 {
    let lambda =
        evidence.max(0.0) / (evidence.max(0.0) + if platform == "codeforces" { 8.0 } else { 6.0 });
    lambda * representative + (1.0 - lambda) * prior
}

fn knowledge_buckets(
    platform: &str,
    values: Vec<(&'static str, i64, f64)>,
) -> Vec<KnowledgeBucket> {
    let mut ranked = values
        .iter()
        .filter(|(_, count, _)| *count > 0)
        .map(|(_, _, estimate)| *estimate)
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    let center = ranked
        .get(ranked.len() / 2)
        .copied()
        .unwrap_or(if platform == "codeforces" {
            1200.0
        } else {
            50.0
        });
    values
        .into_iter()
        .map(|(axis, count, estimate)| {
            let score = if count <= 0 {
                0
            } else if platform == "codeforces" {
                let absolute = 20.0 + 0.05 * (estimate - 800.0);
                let relative = 50.0 + (estimate - center) / 10.0;
                (absolute * 0.4 + relative * 0.6).round().clamp(5.0, 95.0) as i64
            } else {
                estimate.round().clamp(5.0, 95.0) as i64
            };
            KnowledgeBucket {
                platform: platform.into(),
                axis: axis.into(),
                count,
                score,
            }
        })
        .collect()
}

fn codeforces_rating_prior(conn: &Connection, account: &str) -> Result<f64, String> {
    let mut stmt = conn.prepare(
        "SELECT new_rating FROM rating_history WHERE platform='codeforces' AND (?='' OR account=?) ORDER BY epoch_second DESC LIMIT 5"
    ).map_err(|error| error.to_string())?;
    let rows = stmt
        .query_map(params![account, account], |row| row.get::<_, i64>(0))
        .map_err(|error| error.to_string())?;
    let mut values = Vec::new();
    for row in rows {
        values.push(row.map_err(|error| error.to_string())? as f64);
    }
    if values.is_empty() {
        return Ok(1200.0);
    }
    values.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    Ok(values[values.len() / 2])
}

pub fn needs_tag_backfill(
    conn: &Connection,
    platform: &str,
    account: &str,
) -> Result<bool, String> {
    if platform != "codeforces" {
        return Ok(false);
    }
    let completed: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM platform_stats_accounts WHERE platform=? AND account=? AND key='metadata_backfill_v1' AND value='1')",
        params![platform, account], |row| row.get(0),
    ).map_err(|error| error.to_string())?;
    if completed {
        return Ok(false);
    }
    let (total, enriched): (i64, i64) = conn.query_row(
        "SELECT COUNT(DISTINCT problem_key),COUNT(DISTINCT CASE WHEN participant_type<>'' THEN problem_key END)
         FROM submissions WHERE platform=? AND account=?",
        params![platform, account], |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(|error| error.to_string())?;
    Ok(total > 0 && enriched * 100 < total * 90)
}

pub fn needs_nowcoder_difficulty_backfill(
    conn: &Connection,
    platform: &str,
    account: &str,
) -> Result<bool, String> {
    if platform != "nowcoder" {
        return Ok(false);
    }
    let completed: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM platform_stats_accounts WHERE platform=? AND account=? AND key='tracker_difficulty_backfill_v2' AND value='1')",
            params![platform, account],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    Ok(!completed)
}

fn knowledge_for_platform(
    conn: &Connection,
    platform: &str,
    account: Option<&str>,
) -> Result<Vec<KnowledgeBucket>, String> {
    if !matches!(platform, "codeforces" | "leetcode" | "qoj") {
        return Ok(Vec::new());
    }
    let account = account.unwrap_or("");
    let mut aggregate_stmt = conn.prepare(
        "SELECT axis,SUM(count) FROM knowledge_stats_accounts WHERE platform=? AND (?='' OR account=?) GROUP BY axis"
    ).map_err(|error| error.to_string())?;
    let aggregate_rows = aggregate_stmt
        .query_map(params![platform, account, account], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|error| error.to_string())?;
    let mut aggregate_counts = HashMap::new();
    for row in aggregate_rows {
        let (axis, count) = row.map_err(|error| error.to_string())?;
        aggregate_counts.insert(axis, count);
    }
    if aggregate_counts.values().any(|count| *count > 0) {
        let mut difficulty_stmt = conn.prepare(
            "SELECT label,SUM(count) FROM difficulty_stats_accounts WHERE platform=? AND (?='' OR account=?) GROUP BY label"
        ).map_err(|error| error.to_string())?;
        let difficulty_rows = difficulty_stmt
            .query_map(params![platform, account, account], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(|error| error.to_string())?;
        let mut difficulty_evidence = Vec::new();
        for row in difficulty_rows {
            let (label, count) = row.map_err(|error| error.to_string())?;
            if let Some(level) = knowledge_level(platform, &label) {
                difficulty_evidence.push((level, count.max(0) as f64));
            }
        }
        let representative = weighted_quantile(&difficulty_evidence, 0.75).unwrap_or(50.0);
        let prior = if platform == "codeforces" {
            codeforces_rating_prior(conn, account)?
        } else {
            50.0
        };
        let values = KNOWLEDGE_AXES
            .iter()
            .map(|axis| {
                let count = aggregate_counts.get(*axis).copied().unwrap_or(0);
                let evidence = (count.max(0) as f64).min(20.0);
                let estimate = robust_knowledge_estimate(representative, prior, evidence, platform);
                (*axis, count, estimate)
            })
            .collect();
        return Ok(knowledge_buckets(platform, values));
    }
    let mut stmt = conn.prepare(
        "SELECT problem_key,MAX(tags),MAX(COALESCE(difficulty,'')),MAX(epoch_second),MAX(CASE participant_type WHEN 'CONTESTANT' THEN 4 WHEN 'VIRTUAL' THEN 3 WHEN 'OUT_OF_COMPETITION' THEN 2 WHEN 'PRACTICE' THEN 1 ELSE 0 END) FROM submissions WHERE platform=? AND (?='' OR account=?) AND tags<>'[]' GROUP BY problem_key"
    ).map_err(|error| error.to_string())?;
    let rows = stmt
        .query_map(params![platform, account, account], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })
        .map_err(|error| error.to_string())?;
    let mut evidence: HashMap<&'static str, Vec<(f64, f64, bool)>> = HashMap::new();
    for row in rows {
        let (_, raw, difficulty, epoch_second, participant_rank) =
            row.map_err(|error| error.to_string())?;
        let tags: Vec<String> = serde_json::from_str(&raw).unwrap_or_default();
        let axes: HashSet<_> = tags.iter().filter_map(|tag| knowledge_axis(tag)).collect();
        let Some(level) = knowledge_level(platform, &difficulty) else {
            continue;
        };
        if axes.is_empty() {
            continue;
        }
        let kind_weight = match participant_rank {
            4 => 1.0,
            3 => 0.65,
            2 => 0.5,
            1 => 0.25,
            _ => {
                if platform == "codeforces" {
                    0.25
                } else {
                    1.0
                }
            }
        };
        let weight = kind_weight * knowledge_recency_weight(epoch_second) / axes.len() as f64;
        let practice = platform == "codeforces" && participant_rank <= 1;
        for axis in axes {
            evidence
                .entry(axis)
                .or_default()
                .push((level, weight, practice));
        }
    }
    if evidence.values().all(|items| items.is_empty()) {
        return Ok(Vec::new());
    }
    let prior = if platform == "codeforces" {
        codeforces_rating_prior(conn, account)?
    } else {
        let all = evidence
            .values()
            .flatten()
            .map(|(level, weight, _)| (*level, *weight))
            .collect::<Vec<_>>();
        weighted_quantile(&all, 0.75).unwrap_or(50.0)
    };
    let values = KNOWLEDGE_AXES
        .iter()
        .map(|axis| {
            let items = evidence.get(axis).cloned().unwrap_or_default();
            let count = items.len() as i64;
            let timed = items
                .iter()
                .filter(|(_, _, practice)| !practice)
                .map(|(level, weight, _)| (*level, *weight))
                .collect::<Vec<_>>();
            let practice = items
                .iter()
                .filter(|(_, _, practice)| *practice)
                .map(|(level, weight, _)| (*level, *weight))
                .collect::<Vec<_>>();
            let timed_p75 = weighted_quantile(&timed, 0.75);
            let practice_p75 = weighted_quantile(&practice, 0.75);
            let representative = match (timed_p75, practice_p75) {
                (Some(timed), Some(practice)) => 0.75 * timed + 0.25 * practice,
                (Some(value), None) | (None, Some(value)) => value,
                _ => prior,
            };
            let timed_sum = timed
                .iter()
                .map(|(_, weight)| weight)
                .sum::<f64>()
                .min(20.0);
            let practice_evidence = (practice.len() as f64 * 0.1).min(5.0);
            let effective = timed_sum + practice_evidence;
            let estimate = robust_knowledge_estimate(representative, prior, effective, platform);
            (*axis, count, estimate)
        })
        .collect();
    Ok(knowledge_buckets(platform, values))
}

const UNRATED_LABEL: &str = "未评级";
const UNRATED_ORDER: i64 = -1;

fn difficulty_for_platform(
    conn: &Connection,
    p: &str,
    _start: Option<&str>,
    _end: Option<&str>,
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
    if explicit_count > 0 && source.is_empty() {
        let mut stmt=conn.prepare("SELECT label,SUM(count),sort_order FROM difficulty_stats_accounts WHERE platform=? AND (?='' OR account=?) GROUP BY label,sort_order ORDER BY sort_order,label").map_err(|e|e.to_string())?;
        let rows = stmt
            .query_map(params![p, account, account], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })
            .map_err(|e| e.to_string())?;
        let mut buckets: BTreeMap<(i64, String), i64> = BTreeMap::new();
        for row in rows {
            let (raw_label, count) = row.map_err(|e| e.to_string())?;
            let (order, label) = bucket_label(p, &raw_label);
            *buckets.entry((order, label)).or_default() += count;
        }
        let solved: i64 = conn.query_row(
            "SELECT COALESCE(SUM(CAST(value AS INTEGER)),0) FROM platform_stats_accounts WHERE platform=? AND key='solved_count' AND (?='' OR account=?)",
            params![p, account, account],
            |row| row.get(0),
        ).unwrap_or(0);
        let rated = buckets
            .iter()
            .filter(|((order, _), _)| *order != UNRATED_ORDER)
            .map(|(_, count)| *count)
            .sum::<i64>();
        if solved > rated {
            let unrated = buckets
                .entry((UNRATED_ORDER, UNRATED_LABEL.into()))
                .or_default();
            *unrated = (*unrated).max(solved - rated);
        }
        return Ok(buckets
            .into_iter()
            .filter(|(_, count)| *count > 0)
            .map(|((order, label), count)| DifficultyBucket {
                platform: p.into(),
                label,
                count,
                order,
            })
            .collect());
    }
    let mut stmt=conn.prepare("SELECT account,problem_key,difficulty FROM submissions WHERE platform=? AND (?='' OR account=?) AND (?='' OR source=?) ORDER BY epoch_second,submission_id").map_err(|e|e.to_string())?;
    let rows = stmt
        .query_map(params![p, account, account, source, source], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut seen = HashSet::new();
    let mut bucket: BTreeMap<(i64, String), i64> = BTreeMap::new();
    for row in rows {
        let (row_account, problem, difficulty) = row.map_err(|e| e.to_string())?;
        if !seen.insert(format!("{row_account}\0{problem}")) {
            continue;
        }
        let (order, label) = bucket_label(p, difficulty.as_deref().unwrap_or(""));
        *bucket.entry((order, label)).or_default() += 1;
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

pub fn solved_problem_keys(conn: &Connection, platform: &str) -> Result<HashSet<String>, String> {
    let mut stmt = conn
        .prepare("SELECT DISTINCT problem_key FROM submissions WHERE platform=?")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([platform], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<HashSet<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn apply_qoj_problem_ratings(
    conn: &Connection,
    contests: &[XcpcContest],
) -> Result<usize, String> {
    let mut updated = 0;
    for problem in contests.iter().flat_map(|contest| &contest.problems) {
        let label = match problem.tier.as_deref() {
            Some("gold") => Some("金题"),
            Some("silver") => Some("银题"),
            Some("bronze") => Some("铜题"),
            Some("iron") => Some("铁题"),
            _ => None,
        };
        let mut tags = problem.tag_axes.clone();
        tags.extend(problem.tags.iter().cloned());
        tags.sort();
        tags.dedup();
        let tags = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".into());
        updated += conn.execute(
            "UPDATE submissions SET difficulty=COALESCE(?,difficulty),tags=CASE WHEN ?='[]' THEN tags ELSE ? END WHERE platform='qoj' AND problem_key=?",
            params![label, tags, tags, problem.problem_id],
        ).map_err(|e| e.to_string())?;
    }
    Ok(updated)
}

fn bucket_label(p: &str, difficulty: &str) -> (i64, String) {
    let d = difficulty.trim();
    if d.is_empty() || d.eq_ignore_ascii_case("unknown") || d.eq_ignore_ascii_case("unrated") {
        return (UNRATED_ORDER, UNRATED_LABEL.into());
    }
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
    if p == "leetcode" {
        return match d.to_ascii_lowercase().as_str() {
            "easy" => (1, "Easy".into()),
            "medium" => (2, "Medium".into()),
            "hard" => (3, "Hard".into()),
            _ => (UNRATED_ORDER, UNRATED_LABEL.into()),
        };
    }
    if p == "qoj" {
        return match d {
            "铁题" | "iron" => (1, "铁题".into()),
            "铜题" | "bronze" => (2, "铜题".into()),
            "银题" | "silver" => (3, "银题".into()),
            "金题" | "gold" => (4, "金题".into()),
            _ => (UNRATED_ORDER, UNRATED_LABEL.into()),
        };
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
            _ => UNRATED_ORDER,
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
            _ => UNRATED_LABEL,
        };
        return (order, label.into());
    }
    (UNRATED_ORDER, UNRATED_LABEL.into())
}

fn difficulty_daily_for_platform(
    conn: &Connection,
    p: &str,
    start: Option<&str>,
    end: Option<&str>,
    account: Option<&str>,
    source: Option<&str>,
    time_zone: &str,
) -> Result<Vec<DifficultyDayPoint>, String> {
    let s = start.unwrap_or("0000-00-00");
    let e = end.unwrap_or("9999-99-99");
    let account = account.unwrap_or("");
    let source = source.unwrap_or("");
    let mut stmt = conn.prepare("SELECT epoch_second,difficulty,source_day FROM submissions WHERE platform=? AND (?='' OR account=?) AND (?='' OR source=?) ORDER BY epoch_second").map_err(|e|e.to_string())?;
    let rows = stmt
        .query_map(params![p, account, account, source, source], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut days: BTreeMap<String, (i64, String)> = BTreeMap::new();
    for row in rows {
        let (ts, difficulty, source_day) = row.map_err(|e| e.to_string())?;
        let day = source_day.unwrap_or_else(|| day_in_time_zone(ts, time_zone));
        if day.as_str() < s || day.as_str() > e {
            continue;
        }
        let (order, label) = bucket_label(p, difficulty.as_deref().unwrap_or(""));
        let rank = if order == UNRATED_ORDER { -1 } else { order };
        match days.get(&day) {
            Some((current, _))
                if (if *current == UNRATED_ORDER {
                    -1
                } else {
                    *current
                }) >= rank => {}
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

fn streaks(map: &BTreeMap<String, i64>, end: Option<&str>, time_zone: &str) -> (i64, i64) {
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
    let today = today_in_time_zone(time_zone);
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
    time_zone: &str,
) -> Result<DayDetail, String> {
    let (start, end) =
        day_epoch_range(day, time_zone).ok_or_else(|| "日期或时区格式错误".to_string())?;
    let mut items = Vec::new();
    let mut aggs: Vec<AggregateDetail> = Vec::new();
    let ps: Vec<&str> = platform
        .map(|p| vec![p])
        .unwrap_or_else(|| PLATFORMS.into_iter().collect());
    for p in ps {
        let account = account.unwrap_or("");
        let source = source.unwrap_or("");
        let mut stmt=conn.prepare("SELECT platform,account,source,source_day,submission_id,problem_key,problem_id,problem_name,problem_url,epoch_second,language,difficulty,participant_type,tags FROM submissions WHERE platform=? AND ((source_day IS NULL AND epoch_second>=? AND epoch_second<=?) OR source_day=?) AND (?='' OR account=?) AND (?='' OR source=?) ORDER BY epoch_second DESC").map_err(|e|e.to_string())?;
        let rows = stmt
            .query_map(
                params![p, start, end, day, account, account, source, source],
                row_submission,
            )
            .map_err(|e| e.to_string())?;
        for r in rows {
            items.push(r.map_err(|e| e.to_string())?);
        }
        let mut st=conn.prepare("SELECT platform,metric,count,note,day,epoch_second FROM daily_aggregates_accounts WHERE platform=? AND (?='' OR account=?) ORDER BY metric").map_err(|e|e.to_string())?;
        let rs = st
            .query_map(params![p, account, account], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, Option<i64>>(5)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        for r in rs {
            let (row_platform, metric, count, note, source_day, epoch) =
                r.map_err(|e| e.to_string())?;
            let converted = epoch
                .map(|value| day_in_time_zone(value, time_zone))
                .unwrap_or(source_day);
            if converted != day {
                continue;
            }
            if let Some(existing) = aggs
                .iter_mut()
                .find(|item| item.platform == row_platform && item.metric == metric)
            {
                existing.count += count;
                if !note.is_empty() && !existing.note.contains(&note) {
                    if !existing.note.is_empty() {
                        existing.note.push_str(" · ");
                    }
                    existing.note.push_str(&note);
                }
            } else {
                aggs.push(AggregateDetail {
                    platform: row_platform,
                    metric,
                    count,
                    note,
                });
            }
        }
    }
    items.sort_by_key(|x| std::cmp::Reverse(x.epoch_second));
    Ok(DayDetail {
        day: day.into(),
        items,
        aggregates: aggs,
    })
}

pub fn difficulty_detail(
    conn: &Connection,
    platform: &str,
    label: &str,
    account: Option<&str>,
    source: Option<&str>,
) -> Result<DifficultyDetail, String> {
    let account = account.unwrap_or("");
    let source = source.unwrap_or("");
    let explicit_count = if source.is_empty() {
        difficulty_for_platform(
            conn,
            platform,
            None,
            None,
            (!account.is_empty()).then_some(account),
            None,
        )?
        .into_iter()
        .find(|bucket| bucket.label == label)
        .map(|bucket| bucket.count)
        .unwrap_or(0)
    } else {
        0
    };
    let mut stmt = conn.prepare(
        "SELECT platform,account,source,source_day,submission_id,problem_key,problem_id,problem_name,problem_url,epoch_second,language,difficulty,participant_type,tags FROM submissions WHERE platform=? AND (?='' OR account=?) AND (?='' OR source=?) ORDER BY epoch_second DESC,submission_id DESC"
    ).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(
            params![platform, account, account, source, source],
            row_submission,
        )
        .map_err(|e| e.to_string())?;
    let mut seen = HashSet::new();
    let mut items = Vec::new();
    for row in rows {
        let item = row.map_err(|e| e.to_string())?;
        let matches = bucket_label(platform, item.difficulty.as_deref().unwrap_or("")).1 == label;
        if matches && seen.insert(format!("{}\0{}", item.account, item.problem_key)) {
            items.push(item);
        }
    }
    let item_count = items.len() as i64;
    let count = explicit_count.max(item_count);
    let note = if explicit_count > item_count {
        Some(format!(
            "当前数据源只提供部分逐题记录：共 {explicit_count} 题，可显示 {item_count} 题。"
        ))
    } else {
        None
    };
    Ok(DifficultyDetail {
        platform: platform.into(),
        label: label.into(),
        count,
        items,
        note,
    })
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
    use super::*;
    use std::path::Path;

    fn entry(platform: &str, account: &str) -> AccountConfig {
        AccountConfig {
            platform: platform.into(),
            account: account.into(),
            secret: String::new(),
        }
    }

    fn remote(platform: &str, account: &str) -> RemoteData {
        RemoteData {
            platform: platform.into(),
            account: account.into(),
            display_name: None,
            submissions: vec![Submission {
                platform: platform.into(),
                account: account.into(),
                source: "oj".into(),
                source_day: None,
                submission_id: "shared-id".into(),
                problem_key: "A".into(),
                problem_id: "A".into(),
                problem_name: "A".into(),
                problem_url: String::new(),
                epoch_second: 1_767_196_800,
                language: "C++".into(),
                difficulty: Some("1200".into()),
                participant_type: "CONTESTANT".into(),
                tags: vec![],
            }],
            aggregates: vec![AggregateDay {
                day: "2026-01-01".into(),
                epoch_second: None,
                metric: "activity".into(),
                count: 3,
                note: String::new(),
            }],
            solved_count: Some(1),
            difficulty: vec![DifficultyStat {
                label: "1200".into(),
                count: 1,
                order: 1200,
            }],
            knowledge: Some(vec![KnowledgeStat {
                axis: "图论与树".into(),
                count: 1,
            }]),
            ratings: Some(vec![RatingPoint {
                contest_id: "1".into(),
                contest_name: "Round 1".into(),
                epoch_second: 1_767_196_800,
                old_rating: 1200,
                new_rating: 1300,
                rank: Some(100),
            }]),
            activity_only: false,
            notes: vec![],
            cursor_epoch: 123,
            replace_submissions: false,
            replace_aggregates: false,
        }
    }

    #[test]
    fn watched_people_establish_a_baseline_then_emit_each_new_ac_once() {
        let mut conn = open(Path::new(":memory:")).unwrap();
        save_watched_person(&mut conn, "codeforces", "teammate", "小明", "队友", "").unwrap();
        let person_id = get_watched_people(&conn).unwrap()[0].id;

        let mut initial = remote("codeforces", "teammate");
        initial.cursor_epoch = 200;
        assert!(apply_watched_remote(&mut conn, person_id, &initial)
            .unwrap()
            .is_empty());
        assert!(get_watched_events(&conn, 20).unwrap().is_empty());
        assert!(get_watched_people(&conn).unwrap()[0].initialized);

        let mut next = initial.clone();
        let mut newer = next.submissions[0].clone();
        newer.submission_id = "new-ac".into();
        newer.problem_id = "B".into();
        newer.problem_key = "B".into();
        newer.problem_name = "New AC".into();
        newer.epoch_second += 60;
        next.submissions.push(newer);
        next.cursor_epoch = 300;
        let events = apply_watched_remote(&mut conn, person_id, &next).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].problem_name, "New AC");
        assert_eq!(get_watched_events(&conn, 20).unwrap().len(), 1);

        assert!(apply_watched_remote(&mut conn, person_id, &next)
            .unwrap()
            .is_empty());
        let event_id = get_watched_events(&conn, 20).unwrap()[0].id;
        dismiss_watched_event(&conn, event_id).unwrap();
        assert!(get_watched_events(&conn, 20).unwrap()[0].dismissed);
        assert!(get_pending_watched_notifications(&conn).unwrap().is_empty());

        delete_watched_person(&mut conn, person_id).unwrap();
        assert!(get_watched_events(&conn, 20).unwrap().is_empty());
        assert!(get_watched_people(&conn).unwrap().is_empty());
    }

    #[test]
    fn watched_people_batch_save_is_atomic() {
        let mut conn = open(Path::new(":memory:")).unwrap();
        let bindings = vec![
            WatchedBindingInput {
                platform: "codeforces".into(),
                account: "cf-user".into(),
                secret: String::new(),
            },
            WatchedBindingInput {
                platform: "atcoder".into(),
                account: "at-user".into(),
                secret: String::new(),
            },
        ];
        save_watched_people(&mut conn, "小明", "队友", &bindings).unwrap();
        assert_eq!(get_watched_people(&conn).unwrap().len(), 2);

        let invalid = vec![
            WatchedBindingInput {
                platform: "luogu".into(),
                account: "lg-user".into(),
                secret: String::new(),
            },
            WatchedBindingInput {
                platform: "qoj".into(),
                account: String::new(),
                secret: String::new(),
            },
        ];
        assert!(save_watched_people(&mut conn, "小明", "队友", &invalid).is_err());
        let people = get_watched_people(&conn).unwrap();
        assert_eq!(people.len(), 2);
        assert!(!people.iter().any(|person| person.platform == "luogu"));
    }

    #[test]
    fn editing_watched_person_can_add_a_platform_with_add_validation() {
        let mut conn = open(Path::new(":memory:")).unwrap();
        save_watched_person(&mut conn, "codeforces", "cf-user", "小明", "队友", "").unwrap();
        save_watched_person(&mut conn, "luogu", "taken-user", "小红", "", "").unwrap();
        let people = get_watched_people(&conn).unwrap();
        let person_id = people
            .iter()
            .find(|person| person.platform == "codeforces")
            .unwrap()
            .id;

        let existing = WatchedBindingInput {
            platform: "codeforces".into(),
            account: "cf-user".into(),
            secret: String::new(),
        };
        let duplicate_addition = WatchedBindingInput {
            platform: "luogu".into(),
            account: " TAKEN-USER ".into(),
            secret: String::new(),
        };
        let error = edit_watched_person(
            &mut conn,
            &[person_id],
            "小明改名",
            "",
            &[existing.clone(), duplicate_addition],
        )
        .unwrap_err();
        assert!(error.contains("该用户已经被添加了"));
        assert!(edit_watched_person(&mut conn, &[person_id], "小明", "队友", &[]).is_err());

        let new_platform = WatchedBindingInput {
            platform: "atcoder".into(),
            account: "at-user".into(),
            secret: String::new(),
        };
        edit_watched_person(
            &mut conn,
            &[person_id],
            "小明",
            "队友",
            &[existing, new_platform],
        )
        .unwrap();
        let people = get_watched_people(&conn).unwrap();
        assert_eq!(people.len(), 3);
        assert_eq!(
            people
                .iter()
                .filter(|person| person.nickname == "小明")
                .count(),
            2
        );
        assert!(!people.iter().any(|person| person.nickname == "小明改名"));
        assert!(people
            .iter()
            .any(|person| person.platform == "atcoder" && person.account == "at-user"));
    }

    #[test]
    fn editing_one_watched_account_only_changes_that_platform_and_resets_its_baseline() {
        let mut conn = open(Path::new(":memory:")).unwrap();
        save_watched_person(&mut conn, "codeforces", "cf-user", "小明", "队友", "").unwrap();
        save_watched_person(&mut conn, "atcoder", "at-user", "小明", "队友", "").unwrap();
        let people = get_watched_people(&conn).unwrap();
        let atcoder_id = people.iter().find(|person| person.platform == "atcoder").unwrap().id;
        let codeforces_id = people.iter().find(|person| person.platform == "codeforces").unwrap().id;
        mark_watched_checking(&conn, atcoder_id).unwrap();
        edit_watched_person(
            &mut conn,
            &[atcoder_id],
            "小明",
            "队友",
            &[WatchedBindingInput { platform: "atcoder".into(), account: "new-at-user".into(), secret: String::new() }],
        ).unwrap();
        let people = get_watched_people(&conn).unwrap();
        assert_eq!(people.len(), 2);
        let updated = people.iter().find(|person| person.id == atcoder_id).unwrap();
        assert_eq!(updated.account, "new-at-user");
        assert!(!updated.initialized);
        assert_eq!(updated.status, "idle");
        assert_eq!(people.iter().find(|person| person.id == codeforces_id).unwrap().account, "cf-user");
    }

    fn count(conn: &Connection, table: &str, platform: &str, account: &str) -> i64 {
        conn.query_row(
            &format!("SELECT COUNT(*) FROM {table} WHERE platform=? AND account=?"),
            params![platform, account],
            |r| r.get(0),
        )
        .unwrap()
    }

    #[test]
    fn removing_one_id_purges_all_owned_records_and_preserves_others() {
        let mut conn = open(Path::new(":memory:")).unwrap();
        replace_all_accounts(
            &mut conn,
            &[
                entry("codeforces", "alice"),
                entry("codeforces", "bob"),
                entry("atcoder", "alice"),
            ],
        )
        .unwrap();
        for (platform, account) in [
            ("codeforces", "alice"),
            ("codeforces", "bob"),
            ("atcoder", "alice"),
        ] {
            apply_remote(&mut conn, &remote(platform, account)).unwrap();
        }
        // Same submission id must coexist across accounts.
        assert_eq!(count(&conn, "submissions", "codeforces", "alice"), 1);
        replace_accounts(&mut conn, "codeforces", &[entry("codeforces", "bob")]).unwrap();
        for table in [
            "submissions",
            "daily_aggregates_accounts",
            "difficulty_stats_accounts",
            "knowledge_stats_accounts",
            "platform_stats_accounts",
            "rating_history",
            "account_sync_state",
        ] {
            assert_eq!(count(&conn, table, "codeforces", "alice"), 0, "{table}");
            assert!(count(&conn, table, "codeforces", "bob") > 0, "{table}");
            assert!(count(&conn, table, "atcoder", "alice") > 0, "{table}");
        }
        assert_eq!(get_cursor(&conn, "codeforces", "alice").unwrap(), 0);
        assert_eq!(
            ratings_for_platform(&conn, "codeforces", None)
                .unwrap()
                .len(),
            1
        );
        let detail = day_detail(
            &conn,
            "2026-01-01",
            Some("codeforces"),
            None,
            None,
            "Asia/Shanghai",
        )
        .unwrap();
        assert_eq!(detail.items.len(), 1);
        assert_eq!(detail.items[0].account, "bob");
    }

    #[test]
    fn renamed_or_removed_ids_cannot_accept_late_sync_results() {
        let mut conn = open(Path::new(":memory:")).unwrap();
        replace_accounts(&mut conn, "codeforces", &[entry("codeforces", "old")]).unwrap();
        apply_remote(&mut conn, &remote("codeforces", "old")).unwrap();
        replace_accounts(&mut conn, "codeforces", &[entry("codeforces", "new")]).unwrap();
        assert!(apply_remote(&mut conn, &remote("codeforces", "old")).is_err());
        assert_eq!(get_cursor(&conn, "codeforces", "new").unwrap(), 0);
        save_account(&mut conn, "codeforces", "", "").unwrap();
        assert!(get_accounts(&conn).unwrap().is_empty());
        assert!(ratings_for_platform(&conn, "codeforces", None)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn bulk_account_save_rolls_back_all_platforms_on_error() {
        let mut conn = open(Path::new(":memory:")).unwrap();
        replace_all_accounts(
            &mut conn,
            &[entry("codeforces", "alice"), entry("atcoder", "bob")],
        )
        .unwrap();
        apply_remote(&mut conn, &remote("codeforces", "alice")).unwrap();
        conn.execute_batch("CREATE TRIGGER fail_save BEFORE INSERT ON account_entries WHEN NEW.account='fail' BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap();
        assert!(replace_all_accounts(&mut conn, &[entry("atcoder", "fail")]).is_err());
        assert_eq!(get_accounts(&conn).unwrap().len(), 2);
        assert_eq!(count(&conn, "submissions", "codeforces", "alice"), 1);
    }

    #[test]
    fn clearing_records_keeps_accounts_and_resets_cursors_and_ratings() {
        let mut conn = open(Path::new(":memory:")).unwrap();
        replace_accounts(&mut conn, "codeforces", &[entry("codeforces", "alice")]).unwrap();
        apply_remote(&mut conn, &remote("codeforces", "alice")).unwrap();
        clear_all(&mut conn).unwrap();
        assert_eq!(get_accounts(&conn).unwrap().len(), 1);
        assert_eq!(get_cursor(&conn, "codeforces", "alice").unwrap(), 0);
        assert!(ratings_for_platform(&conn, "codeforces", None)
            .unwrap()
            .is_empty());
        assert_eq!(
            statuses(&conn)
                .unwrap()
                .iter()
                .map(|s| s.cached_records)
                .sum::<i64>(),
            0
        );
    }

    #[test]
    fn failed_rating_refresh_preserves_cache_but_empty_success_clears_it() {
        let mut conn = open(Path::new(":memory:")).unwrap();
        replace_accounts(&mut conn, "codeforces", &[entry("codeforces", "alice")]).unwrap();
        let mut data = remote("codeforces", "alice");
        apply_remote(&mut conn, &data).unwrap();
        data.ratings = None;
        apply_remote(&mut conn, &data).unwrap();
        assert_eq!(count(&conn, "rating_history", "codeforces", "alice"), 1);
        data.ratings = Some(vec![]);
        apply_remote(&mut conn, &data).unwrap();
        assert_eq!(count(&conn, "rating_history", "codeforces", "alice"), 0);
    }

    #[test]
    fn unchanged_incremental_overlap_is_not_reported_as_new_or_updated() {
        let mut conn = open(Path::new(":memory:")).unwrap();
        replace_accounts(&mut conn, "codeforces", &[entry("codeforces", "alice")]).unwrap();
        assert_eq!(
            apply_remote(&mut conn, &remote("codeforces", "alice")).unwrap(),
            (1, 0)
        );
        assert_eq!(
            apply_remote(&mut conn, &remote("codeforces", "alice")).unwrap(),
            (0, 0)
        );
    }

    #[test]
    fn codeforces_metadata_backfill_runs_only_once_after_success() {
        let mut conn = open(Path::new(":memory:")).unwrap();
        replace_accounts(&mut conn, "codeforces", &[entry("codeforces", "alice")]).unwrap();
        let mut data = remote("codeforces", "alice");
        data.submissions[0].participant_type.clear();
        apply_remote(&mut conn, &data).unwrap();
        assert!(needs_tag_backfill(&conn, "codeforces", "alice").unwrap());
        data.submissions[0].participant_type = "CONTESTANT".into();
        data.replace_submissions = true;
        apply_remote(&mut conn, &data).unwrap();
        assert!(!needs_tag_backfill(&conn, "codeforces", "alice").unwrap());
    }

    #[test]
    fn duplicate_provider_rating_rows_are_safely_collapsed() {
        let mut conn = open(Path::new(":memory:")).unwrap();
        replace_accounts(&mut conn, "nowcoder", &[entry("nowcoder", "10001")]).unwrap();
        let mut data = remote("nowcoder", "10001");
        let mut newer = data.ratings.as_ref().unwrap()[0].clone();
        newer.new_rating = 1400;
        data.ratings.as_mut().unwrap().push(newer);
        apply_remote(&mut conn, &data).unwrap();
        assert_eq!(count(&conn, "rating_history", "nowcoder", "10001"), 1);
    }

    #[test]
    fn difficulty_detail_returns_each_account_problem_once() {
        let mut conn = open(Path::new(":memory:")).unwrap();
        replace_accounts(&mut conn, "codeforces", &[entry("codeforces", "alice")]).unwrap();
        let mut data = remote("codeforces", "alice");
        let mut duplicate = data.submissions[0].clone();
        duplicate.submission_id = "second-ac".into();
        duplicate.epoch_second += 60;
        data.submissions.push(duplicate);
        apply_remote(&mut conn, &data).unwrap();
        let detail = difficulty_detail(&conn, "codeforces", "1200", None, None).unwrap();
        assert_eq!(detail.count, 1);
        assert_eq!(detail.items.len(), 1);
        assert_eq!(detail.items[0].submission_id, "second-ac");
    }

    #[test]
    fn difficulty_includes_unrated_problems() {
        let mut conn = open(Path::new(":memory:")).unwrap();
        replace_accounts(&mut conn, "codeforces", &[entry("codeforces", "alice")]).unwrap();
        let mut data = remote("codeforces", "alice");
        let mut unrated = data.submissions[0].clone();
        unrated.submission_id = "unrated-ac".into();
        unrated.problem_key = "B".into();
        unrated.problem_id = "B".into();
        unrated.problem_name = "Unrated".into();
        unrated.difficulty = None;
        data.submissions.push(unrated);
        data.solved_count = Some(2);
        apply_remote(&mut conn, &data).unwrap();
        let buckets = difficulty_for_platform(&conn, "codeforces", None, None, None, None).unwrap();
        assert_eq!(
            buckets.first().map(|item| item.label.as_str()),
            Some(UNRATED_LABEL)
        );
        assert_eq!(
            buckets
                .iter()
                .find(|item| item.label == UNRATED_LABEL)
                .map(|item| item.count),
            Some(1)
        );
        let detail = difficulty_detail(&conn, "codeforces", UNRATED_LABEL, None, None).unwrap();
        assert_eq!(detail.count, 1);
        assert_eq!(detail.items.len(), 1);
    }

    #[test]
    fn daily_difficulty_prefers_rated_problem_over_unrated_problem() {
        let mut conn = open(Path::new(":memory:")).unwrap();
        replace_accounts(&mut conn, "codeforces", &[entry("codeforces", "alice")]).unwrap();
        let mut data = remote("codeforces", "alice");
        let mut unrated = data.submissions[0].clone();
        unrated.submission_id = "unrated-ac".into();
        unrated.problem_key = "B".into();
        unrated.problem_id = "B".into();
        unrated.problem_name = "Unrated".into();
        unrated.difficulty = None;
        data.submissions.push(unrated);
        data.solved_count = Some(2);
        apply_remote(&mut conn, &data).unwrap();
        let daily = difficulty_daily_for_platform(
            &conn,
            "codeforces",
            None,
            None,
            None,
            None,
            "Asia/Shanghai",
        )
        .unwrap();
        assert_eq!(daily.len(), 1);
        assert_eq!(daily[0].order, 1200);
        assert_eq!(daily[0].label, "1200");
    }

    #[test]
    fn reopening_does_not_reimport_legacy_caches() {
        let path = std::env::temp_dir().join(format!(
            "oj-insight-test-{}-{}.sqlite3",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap()
        ));
        {
            let mut conn = open(&path).unwrap();
            replace_accounts(
                &mut conn,
                "codeforces",
                &[entry("codeforces", "alice"), entry("codeforces", "removed")],
            )
            .unwrap();
            apply_remote(&mut conn, &remote("codeforces", "removed")).unwrap();
            // Simulate v0.4 removing just the config, leaving account caches.
            conn.execute("DELETE FROM account_entries WHERE account='removed'", [])
                .unwrap();
            conn.execute_batch("INSERT INTO daily_aggregates VALUES('codeforces','2026-01-01','activity',99,'legacy',NULL);").unwrap();
        }
        {
            let mut conn = open(&path).unwrap();
            assert_eq!(
                count(&conn, "daily_aggregates_accounts", "codeforces", "alice"),
                0
            );
            assert_eq!(count(&conn, "submissions", "codeforces", "removed"), 0);
            assert_eq!(count(&conn, "rating_history", "codeforces", "removed"), 0);
            replace_accounts(&mut conn, "codeforces", &[]).unwrap();
        }
        {
            let conn = open(&path).unwrap();
            assert!(get_accounts(&conn).unwrap().is_empty());
            assert_eq!(
                statuses(&conn)
                    .unwrap()
                    .iter()
                    .map(|s| s.cached_records)
                    .sum::<i64>(),
                0
            );
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn utc8_day_changes_at_china_midnight() {
        assert_eq!(
            day_in_time_zone(1_767_196_799, "Asia/Shanghai"),
            "2025-12-31"
        );
        assert_eq!(
            day_in_time_zone(1_767_196_800, "Asia/Shanghai"),
            "2026-01-01"
        );
    }

    #[test]
    fn day_range_follows_dst_timezone() {
        let (start, end) = day_epoch_range("2026-03-08", "America/New_York").unwrap();
        assert_eq!(end - start + 1, 23 * 3600);
    }

    #[test]
    fn difficulty_buckets_follow_platform_levels() {
        assert_eq!(bucket_label("codeforces", "1350"), (1300, "1300".into()));
        assert_eq!(bucket_label("luogu", "7"), (7, "省选/NOI-".into()));
        assert_eq!(bucket_label("luogu", "提高"), (5, "提高".into()));
        assert_eq!(bucket_label("leetcode", "Medium"), (2, "Medium".into()));
        assert_eq!(bucket_label("qoj", "bronze"), (2, "铜题".into()));
        assert_eq!(bucket_label("qoj", "金题"), (4, "金题".into()));
        assert_eq!(
            bucket_label("codeforces", ""),
            (UNRATED_ORDER, UNRATED_LABEL.into())
        );
        assert_eq!(
            bucket_label("atcoder", "unknown"),
            (UNRATED_ORDER, UNRATED_LABEL.into())
        );
    }

    #[test]
    fn knowledge_estimate_values_difficulty_and_uses_evidence_as_confidence() {
        let easy = robust_knowledge_estimate(
            knowledge_level("leetcode", "Easy").unwrap(),
            50.0,
            5.0,
            "leetcode",
        );
        let hard = robust_knowledge_estimate(
            knowledge_level("leetcode", "Hard").unwrap(),
            50.0,
            5.0,
            "leetcode",
        );
        assert!(hard > easy);

        let one = robust_knowledge_estimate(86.0, 50.0, 1.0, "leetcode");
        let two = robust_knowledge_estimate(86.0, 50.0, 2.0, "leetcode");
        let three = robust_knowledge_estimate(86.0, 50.0, 3.0, "leetcode");
        assert!(two > one && three > two);
        assert!(three - two < two - one);
        assert!(three < 95.0);
    }
}
