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
