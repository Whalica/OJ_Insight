use crate::app::state::AppState;
use crate::db;
use crate::infrastructure::logging::log_event;
use crate::models::{AccountConfig, WatchedAcEvent, WatchedSyncResult};

/// Check the separate relationship-person cache. Their submissions never enter
/// the user's own analytics tables; only newly observed accepted records become
/// notification events.
pub(crate) async fn sync(
    state: &AppState,
    only_person: Option<i64>,
) -> Result<WatchedSyncResult, String> {
    let today_start=chrono::Local::now().date_naive().and_hms_opt(0,0,0)
        .and_then(|day|day.and_local_timezone(chrono::Local).earliest())
        .map(|day|day.timestamp()).unwrap_or_else(||chrono::Utc::now().timestamp()-86_400);
    let _operation = state.operations.enter()?;
    let people = {
        let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
        db::get_watched_people(&conn)?
            .into_iter()
            .filter(|person| only_person == Some(person.id) || person.enabled)
            .filter(|person| only_person.is_none() || only_person == Some(person.id))
            .collect::<Vec<_>>()
    };

    let mut checked = 0_i64;
    let mut events = Vec::<WatchedAcEvent>::new();
    let mut failures = Vec::new();

    for person in people {
        let cursor = {
            let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
            db::watched_cursor(&conn, person.id)?
        };
        let first_sync = !person.initialized || cursor == 0;
        {
            let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
            db::mark_watched_checking(&conn, person.id)?;
        }

        let account = AccountConfig {
            platform: person.platform.clone(),
            account: person.account.clone(),
            secret: person.secret.clone(),
        };
        log_event(
            &state.log_dir,
            &person.platform,
            &format!("relationship check started account={}", person.account),
            &person.secret,
        );

        match super::fetch_platform(
            &state.client,
            &account,
            false,
            today_start,
            &state.data_dir.join("public-cache"),
        )
        .await
        {
            Ok(mut remote) => {
                // The configured identifier is the stable relationship key even
                // when a provider returns a display name or a cn: prefix.
                remote.account = person.account.clone();
                for submission in &mut remote.submissions {
                    submission.account = person.account.clone();
                }
                let applied = {
                    let mut conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
                    db::apply_watched_today(&mut conn, person.id, &remote)
                };
                let new_events = match applied {
                    Ok(value) => value,
                    Err(message) => {
                        let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
                        db::mark_watched_failed(&conn, person.id, &message)?;
                        failures.push(format!("{}：{}", person.nickname_or_account(), message));
                        log_event(
                            &state.log_dir,
                            &person.platform,
                            &format!("relationship check failed: {message}"),
                            &person.secret,
                        );
                        continue;
                    }
                };
                let mut message = if first_sync {
                    format!("已获取今天的 AC，共 {} 条新记录",new_events.len())
                } else if new_events.is_empty() {
                    "检查完成，没有新的 AC".to_string()
                } else {
                    format!("检查完成，发现 {} 条新 AC", new_events.len())
                };
                let status = if remote.activity_only {
                    message.push_str(" · 该平台只提供部分逐题记录");
                    "warning"
                } else {
                    "ok"
                };
                {
                    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
                    db::mark_watched_success(
                        &conn,
                        person.id,
                        remote.cursor_epoch,
                        status,
                        &message,
                    )?;
                }
                checked += 1;
                let new_event_count = new_events.len();
                events.extend(new_events);
                log_event(
                    &state.log_dir,
                    &person.platform,
                    &format!(
                        "relationship check completed account={} events={}",
                        person.account, new_event_count
                    ),
                    &person.secret,
                );
            }
            Err(error) => {
                let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
                db::mark_watched_failed(&conn, person.id, &error.message)?;
                failures.push(format!(
                    "{}：{}",
                    person.nickname_or_account(),
                    error.message
                ));
                log_event(
                    &state.log_dir,
                    &person.platform,
                    &format!(
                        "relationship check failed status={} message={}",
                        error.status, error.message
                    ),
                    &person.secret,
                );
            }
        }
    }

    events.sort_by(|a, b| {
        b.epoch_second
            .cmp(&a.epoch_second)
            .then_with(|| b.id.cmp(&a.id))
    });
    Ok(WatchedSyncResult {
        checked,
        inserted_events: events.len() as i64,
        events,
        failures,
    })
}

trait WatchedPersonLabel {
    fn nickname_or_account(&self) -> String;
}

impl WatchedPersonLabel for crate::models::WatchedPerson {
    fn nickname_or_account(&self) -> String {
        if self.nickname.trim().is_empty() {
            self.account.clone()
        } else {
            self.nickname.clone()
        }
    }
}
