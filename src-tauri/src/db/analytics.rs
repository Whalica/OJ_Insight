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
    let mut today_problems = Vec::new();
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
        let today_metric = if activity_only { "activity" } else { "daily_unique" };
        let today_count = load_daily(
            conn,
            p,
            today_metric,
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
        if !activity_only {
            today_problems.extend(load_recent(
                conn,
                p,
                Some(&today_key),
                Some(&today_key),
                10_000,
                platform_account_filter,
                platform_source_filter,
                time_zone,
            )?);
        }
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
    today_problems.sort_by_key(|x| std::cmp::Reverse(x.epoch_second));
    let mut seen_today = HashSet::new();
    today_problems.retain(|item| seen_today.insert(format!("{}\0{}", item.platform, item.problem_key)));
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
        today_problems,
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
