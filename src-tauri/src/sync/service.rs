use crate::app::state::AppState;
use crate::db;
use crate::infrastructure::logging::log_event;
use crate::models::SyncResult;

pub(crate) async fn sync_platform(
    state: &AppState,
    platform: &str,
    full: bool,
) -> Result<SyncResult, String> {
    let _operation = state.operations.enter()?;
    let accounts = {
        let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
        db::get_accounts(&conn)?
            .into_iter()
            .filter(|entry| entry.platform == platform && !entry.account.trim().is_empty())
            .collect::<Vec<_>>()
    };
    if accounts.is_empty() {
        return Err(format!("{} 尚未填写账号", platform));
    }

    let mut inserted = 0;
    let mut updated = 0;
    let mut succeeded = 0;
    let mut partial = false;
    let mut failures = Vec::new();
    let mut advisories = Vec::new();

    for account in accounts {
        let cursor = {
            let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
            let cursor = if full {
                0
            } else {
                db::get_cursor(&conn, platform, &account.account)?
            };
            db::mark_syncing(&conn, platform, &account.account)?;
            cursor
        };
        log_event(
            &state.log_dir,
            platform,
            if full {
                "full rebuild started"
            } else {
                "incremental sync started"
            },
            &account.secret,
        );

        match super::fetch_platform(
            &state.client,
            &account,
            full,
            cursor,
            &state.data_dir.join("public-cache"),
        )
        .await
        {
            Ok(mut remote) => {
                partial |= remote.activity_only && platform != "luogu";
                if remote.ratings.is_none()
                    && (platform == "codeforces"
                        || platform == "atcoder"
                        || (platform == "leetcode"
                            && !account.account.to_ascii_lowercase().starts_with("cn:")))
                {
                    remote.notes.push(
                        "警告：Rating 暂未更新，已有 Rating 缓存保留；提交同步不受影响"
                            .into(),
                    );
                }
                // The configured identifier is the stable local account key. Some
                // providers return a display name, which must not split one account.
                remote.account = account.account.trim().to_string();
                for submission in &mut remote.submissions {
                    submission.account = remote.account.clone();
                }
                let counts = {
                    let mut conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
                    db::apply_remote(&mut conn, &remote)
                };
                let counts = match counts {
                    Ok(value) => value,
                    Err(message) => {
                        let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
                        db::mark_failed(&conn, platform, &account.account, "error", &message)?;
                        failures.push(format!("{}：{}", account.account, message));
                        log_event(&state.log_dir, platform, &message, &account.secret);
                        continue;
                    }
                };
                for note in &remote.notes {
                    if let Some(warning) = note.strip_prefix("警告：") {
                        advisories.push(format!("{}：{}", account.account, warning));
                    }
                }
                inserted += counts.0;
                updated += counts.1;
                succeeded += 1;
                log_event(
                    &state.log_dir,
                    platform,
                    &format!(
                        "sync completed account={} inserted={} updated={}",
                        account.account, counts.0, counts.1
                    ),
                    &account.secret,
                );
            }
            Err(err) => {
                let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
                let _ =
                    db::mark_failed(&conn, platform, &account.account, &err.status, &err.message);
                failures.push(format!("{}：{}", account.account, err.message));
                log_event(
                    &state.log_dir,
                    platform,
                    &format!("sync failed status={} message={}", err.status, err.message),
                    &account.secret,
                );
            }
        }
    }

    if succeeded == 0 {
        let message = failures.join("；");
        let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
        conn.execute(
            "UPDATE sync_state SET status='error',message=? WHERE platform=?",
            rusqlite::params![message, platform],
        )
        .map_err(|e| e.to_string())?;
        return Err(message);
    }

    let suffix = if failures.is_empty() {
        String::new()
    } else {
        format!(" · {} 个账号失败", failures.len())
    };
    let status = if failures.is_empty() && advisories.is_empty() {
        "ok"
    } else {
        "warning"
    };
    let mut message = format!("同步成功 · 新增 {inserted}，更新 {updated}{suffix}");
    for detail in failures.iter().chain(advisories.iter()) {
        message.push_str(&format!("；{detail}"));
    }
    {
        let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
        conn.execute(
            "UPDATE sync_state SET status=?,message=? WHERE platform=?",
            rusqlite::params![status, message, platform],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(SyncResult {
        platform: platform.into(),
        inserted,
        updated,
        message,
        status: status.into(),
        partial,
    })
}

pub(crate) async fn sync_all(state: &AppState) -> Result<Vec<SyncResult>, String> {
    let mut configured = {
        let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
        db::get_accounts(&conn)?
            .into_iter()
            .filter(|account| !account.account.trim().is_empty())
            .map(|account| account.platform)
            .collect::<Vec<_>>()
    };
    configured.sort();
    configured.dedup();

    let mut out = Vec::new();
    for platform in configured {
        match sync_platform(state, &platform, false).await {
            Ok(result) => out.push(result),
            Err(message) => out.push(SyncResult {
                platform,
                inserted: 0,
                updated: 0,
                message,
                status: "error".into(),
                partial: false,
            }),
        }
    }
    Ok(out)
}
