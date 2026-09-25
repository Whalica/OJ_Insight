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
        "SELECT id,person_id,platform,account,nickname,relationship,submission_id,problem_id,problem_name,problem_url,epoch_second,language,difficulty,created_at,dismissed FROM watched_events WHERE epoch_second>=CAST(strftime('%s','now','localtime','start of day','utc') AS INTEGER) ORDER BY epoch_second DESC,id DESC LIMIT ?",
        limit,
    )
}

pub fn get_pending_watched_notifications(conn: &Connection) -> Result<Vec<WatchedAcEvent>, String> {
    read_watched_events(
        conn,
        "SELECT e.id,e.person_id,e.platform,e.account,e.nickname,e.relationship,e.submission_id,e.problem_id,e.problem_name,e.problem_url,e.epoch_second,e.language,e.difficulty,e.created_at,e.dismissed FROM watched_events e JOIN watched_notifications n ON n.event_id=e.id WHERE e.epoch_second>=CAST(strftime('%s','now','localtime','start of day','utc') AS INTEGER) ORDER BY e.epoch_second DESC,e.id DESC LIMIT ?",
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
    apply_watched_remote_inner(conn,person_id,remote,false)
}

pub fn apply_watched_today(conn:&mut Connection,person_id:i64,remote:&RemoteData)->Result<Vec<WatchedAcEvent>,String>{
    apply_watched_remote_inner(conn,person_id,remote,true)
}

fn apply_watched_remote_inner(conn:&mut Connection,person_id:i64,remote:&RemoteData,today_only:bool)->Result<Vec<WatchedAcEvent>,String>{
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
    let today_start = chrono::Local::now().date_naive().and_hms_opt(0,0,0)
        .and_then(|day|day.and_local_timezone(chrono::Local).earliest())
        .map(|day|day.timestamp()).unwrap_or(now-86_400);
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
        if (today_only && submission.epoch_second < today_start) || (!today_only && (baseline || exists)) {
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
