use chrono::{Datelike, NaiveDate, TimeZone};
use regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

use super::{
    browser_headers, get_json, get_text, now_epoch, polite_sleep, with_raw_cookie, with_referer,
};
use crate::models::{
    AccountConfig, DifficultyStat, RatingPoint, RemoteData, Submission, SyncError,
};

pub async fn fetch(
    client: &Client,
    account: &AccountConfig,
    full: bool,
    cursor: i64,
) -> Result<RemoteData, SyncError> {
    let uid = account.account.trim();
    if uid.is_empty() || !uid.chars().all(|c| c.is_ascii_digit()) {
        return Err(SyncError::error(
            "牛客目前需要数字 User ID（个人主页 users/ 后面的数字）",
        ));
    }
    let base = format!("https://ac.nowcoder.com/acm/contest/profile/{uid}/practice-coding");
    let mut out = Vec::new();
    let mut page = 1_i64;
    let mut max_seen = cursor;
    let mut display_name = None;
    let cutoff = if full {
        0
    } else {
        cursor.saturating_sub(48 * 3600)
    };
    loop {
        if page > 5000 {
            return Err(SyncError::error("牛客分页过多，已中止"));
        }
        let url = format!("{base}?languageCategoryFilter=-1&orderType=DESC&page={page}&pageSize=200&search=&statusTypeFilter=5");
        let html = get_text(
            client,
            &url,
            with_referer(browser_headers(), "https://ac.nowcoder.com/"),
        )
        .await?;
        let text = Html::parse_document(&html)
            .root_element()
            .text()
            .collect::<String>();
        if display_name.is_none() {
            display_name = parse_display_name(&html);
        }
        if text.contains("登录") && !text.contains("提交时间") {
            return Err(SyncError::auth("牛客页面需要登录或当前账号不可公开访问"));
        }
        let rows = parse_rows(&html, uid);
        if rows.is_empty() {
            break;
        }
        let mut old = false;
        for s in rows {
            max_seen = max_seen.max(s.epoch_second);
            if !full && s.epoch_second <= cutoff {
                old = true;
                continue;
            }
            out.push(s);
        }
        if old || !has_next_page(&html, page) {
            break;
        }
        page += 1;
        polite_sleep(260).await;
    }
    let (tracker, tracker_note) = match fetch_tracker_catalog(client).await {
        Ok(items) => (items, "已读取牛客 Tracker 题目 Rating".to_string()),
        Err(error) => (
            TrackerCatalog::default(),
            format!("警告：牛客 Tracker 暂不可用（{}）", error.message),
        ),
    };
    let mut tracker_matches = 0;
    for submission in &mut out {
        let item = tracker.find_submission(submission);
        if let Some(item) = item {
            if submission.problem_name.trim().is_empty() && !item.title.is_empty() {
                submission.problem_name = item.title.clone();
            }
            if submission.difficulty.is_none() {
                submission.difficulty = item.difficulty.clone();
            }
            tracker_matches += 1;
        }
    }

    let (mut daily_catalog, daily_note) = match fetch_tracker_problems(client).await {
        Ok(items) => (items, "已读取牛客每日一题日历".to_string()),
        Err(error) => (
            TrackerCatalog::default(),
            format!("警告：牛客每日一题日历暂不可用（{}）", error.message),
        ),
    };
    let cookie = account.secret.trim();
    let (completed_days, completion_verified, completion_note) = if cookie.is_empty() {
        (
            HashSet::new(),
            false,
            "未填写 Cookie；普通提交与 Tracker 题目 Rating 正常统计，每日一题打卡记录需登录 Cookie"
                .to_string(),
        )
    } else {
        match fetch_tracker_completed_days(client, cookie).await {
            Ok(days) => {
                let count = days.len();
                (days, true, format!("每日一题登录打卡记录 {count} 天"))
            }
            Err(error) => (
                HashSet::new(),
                false,
                format!("警告：牛客每日一题 Cookie 未生效（{}）", error.message),
            ),
        }
    };
    let daily_difficulty_count =
        enrich_tracker_difficulties(client, &mut daily_catalog, &completed_days, &out).await;
    let mut daily_matches = 0;
    let mut matched_days = HashSet::new();
    for submission in &mut out {
        let Some(item) = daily_catalog.find_submission(submission) else {
            continue;
        };
        let real_day = china_day(submission.epoch_second);
        let confirmed = !completion_verified
            || completed_days.contains(&item.day)
            || completed_days.contains(&real_day);
        if !confirmed {
            continue;
        }
        submission.source = "daily".into();
        submission.source_day = Some(item.day.clone());
        if submission.problem_name.trim().is_empty() && !item.title.is_empty() {
            submission.problem_name = item.title.clone();
        }
        // 每日一题使用独立的 1–5 难度体系，不能沿用 Tracker Rating。
        if item.difficulty.is_some() {
            submission.difficulty = item.difficulty.clone();
        }
        daily_matches += 1;
        matched_days.insert(item.day.clone());
    }
    let mut date_only_daily = 0;
    for day in &completed_days {
        if matched_days.contains(day) {
            continue;
        }
        let Some(item) = daily_catalog.by_day.get(day) else {
            continue;
        };
        out.push(date_only_tracker_submission(uid, item));
        date_only_daily += 1;
    }

    let mut difficulty_counts = HashMap::<String, i64>::new();
    let mut counted_problems = HashSet::new();
    for submission in &out {
        if let Some(label) = &submission.difficulty {
            if counted_problems.insert(submission.problem_key.clone()) {
                *difficulty_counts.entry(label.clone()).or_default() += 1;
            }
        }
    }
    let mut difficulty = difficulty_counts
        .into_iter()
        .map(|(label, count)| DifficultyStat {
            order: label.parse::<i64>().unwrap_or(i64::MAX),
            label,
            count,
        })
        .collect::<Vec<_>>();
    difficulty.sort_by_key(|item| item.order);
    let ratings = fetch_rating_history(client, uid).await.ok();
    Ok(RemoteData {
        platform: "nowcoder".into(),
        account: uid.into(),
        display_name,
        submissions: out,
        aggregates: vec![],
        solved_count: None,
        difficulty,
        knowledge: None,
        solved_inventory: None,
        ratings,
        activity_only: false,
        notes: vec![
            "牛客竞赛站公开练习提交页 · statusTypeFilter=5".into(),
            format!("{tracker_note} · 为 {tracker_matches} 条真实 AC 匹配 Tracker 题目 Rating"),
            format!(
                "{daily_note} · {completion_note} · 读取 {daily_difficulty_count} 道每日题难度 · 匹配 {daily_matches} 条真实 AC，补充 {date_only_daily} 条仅有打卡日期的记录"
            ),
        ],
        cursor_epoch: max_seen.max(now_epoch().saturating_sub(48 * 3600)),
        replace_submissions: full,
        replace_aggregates: full,
    })
}

async fn fetch_rating_history(client: &Client, uid: &str) -> Result<Vec<RatingPoint>, SyncError> {
    let url = format!("https://ac.nowcoder.com/acm/contest/rating-history?uid={uid}");
    let referer = format!("https://ac.nowcoder.com/acm/contest/profile/{uid}");
    let payload = get_json(client, &url, with_referer(browser_headers(), &referer)).await?;
    if payload.get("code").and_then(Value::as_i64) != Some(0) {
        return Err(SyncError::error(
            payload
                .get("msg")
                .and_then(Value::as_str)
                .unwrap_or("牛客 Rating 历史暂不可用"),
        ));
    }
    Ok(parse_rating_history(&payload))
}

fn parse_rating_history(payload: &Value) -> Vec<RatingPoint> {
    let mut by_contest = HashMap::<String, RatingPoint>::new();
    let rows = payload.get("data").and_then(|data| {
        data.as_array().or_else(|| {
            ["list", "records", "ratingHistory", "ratingList"]
                .iter()
                .find_map(|key| data.get(*key).and_then(Value::as_array))
        })
    });
    for item in rows.into_iter().flatten() {
        let Some(new_rating) = ["rating", "ratingValue", "newRating"]
            .iter()
            .find_map(|key| item.get(*key).and_then(number_value))
            .map(|value| value.round() as i64)
        else {
            continue;
        };
        let change = ["changeValue", "change", "ratingChange"]
            .iter()
            .find_map(|key| item.get(*key).and_then(number_value))
            .unwrap_or(0.0)
            .round() as i64;
        let Some(contest_id) = item
            .get("contestId")
            .or_else(|| item.get("contest_id"))
            .or_else(|| item.get("id"))
            .and_then(value_string)
        else {
            continue;
        };
        let raw_time = ["time", "contestTime", "startTime", "contestStartTime"]
            .iter()
            .find_map(|key| {
                item.get(*key)
                    .and_then(|value| value.as_i64().or_else(|| value.as_str()?.parse().ok()))
            })
            .unwrap_or(0);
        let point = RatingPoint {
            contest_id,
            contest_name: item
                .get("contestName")
                .or_else(|| item.get("name"))
                .and_then(Value::as_str)
                .unwrap_or("牛客 Rating 赛")
                .to_string(),
            epoch_second: if raw_time > 10_000_000_000 {
                raw_time / 1000
            } else {
                raw_time
            },
            old_rating: item
                .get("oldRating")
                .and_then(number_value)
                .map(|value| value.round() as i64)
                .unwrap_or(new_rating - change),
            new_rating,
            rank: item
                .get("rank")
                .and_then(number_value)
                .map(|value| value.round() as i64),
        };
        let replace = by_contest
            .get(&point.contest_id)
            .map_or(true, |existing| point.epoch_second >= existing.epoch_second);
        if replace {
            by_contest.insert(point.contest_id.clone(), point);
        }
    }
    let mut points = by_contest.into_values().collect::<Vec<_>>();
    points.sort_by_key(|point| point.epoch_second);
    points
}

fn number_value(value: &Value) -> Option<f64> {
    value.as_f64().or_else(|| value.as_str()?.parse().ok())
}

#[derive(Clone)]
struct TrackerProblem {
    day: String,
    problem_id: String,
    title: String,
    url: String,
    difficulty: Option<String>,
}

#[derive(Default)]
struct TrackerCatalog {
    by_key: HashMap<String, TrackerProblem>,
    by_day: HashMap<String, TrackerProblem>,
}

impl TrackerCatalog {
    fn find_submission(&self, submission: &Submission) -> Option<&TrackerProblem> {
        submission_keys(submission)
            .into_iter()
            .find_map(|key| self.by_key.get(&key))
    }
}

async fn fetch_tracker_catalog(client: &Client) -> Result<TrackerCatalog, SyncError> {
    let mut result = TrackerCatalog::default();
    let mut page = 1_i64;
    let limit = 200_i64;
    let mut seen_pages = HashSet::new();
    loop {
        let url = format!(
            "https://www.nowcoder.com/problem/tracker/list?contestType=0&page={page}&pageSize={limit}&limit={limit}"
        );
        let payload = get_json(
            client,
            &url,
            with_referer(
                browser_headers(),
                "https://www.nowcoder.com/problem/tracker",
            ),
        )
        .await?;
        if !matches!(
            payload.get("code").and_then(Value::as_i64),
            Some(0) | Some(200)
        ) {
            return Err(SyncError::error(
                payload
                    .get("msg")
                    .or_else(|| payload.get("message"))
                    .and_then(Value::as_str)
                    .unwrap_or("Tracker 题库暂不可用"),
            ));
        }
        let papers = ["/data/papers", "/data/list", "/data/records", "/data/result"]
            .iter()
            .find_map(|path| payload.pointer(path).and_then(Value::as_array))
            .cloned()
            .unwrap_or_default();
        if papers.is_empty() {
            break;
        }
        let page_key = serde_json::to_string(&papers).unwrap_or_else(|_| page.to_string());
        if !seen_pages.insert(page_key) {
            break;
        }
        for paper in papers {
            let contest_id = paper
                .get("contestId")
                .or_else(|| paper.get("contest_id"))
                .or_else(|| paper.get("competitionId"))
                .or_else(|| paper.get("id"))
                .and_then(value_string)
                .unwrap_or_default();
            let questions = paper
                .get("questions")
                .or_else(|| paper.get("problems"))
                .or_else(|| paper.get("questionList"))
                .or_else(|| paper.get("problemList"))
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            for question in questions {
                let problem_id = question
                    .get("problemId")
                    .or_else(|| question.get("questionId"))
                    .or_else(|| question.get("id"))
                    .and_then(value_string)
                    .unwrap_or_default();
                if problem_id.is_empty() {
                    continue;
                }
                let title = question
                    .get("title")
                    .or_else(|| question.get("questionTitle"))
                    .or_else(|| question.get("name"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let raw_difficulty = question
                    .get("difficulty")
                    .or_else(|| question.get("difficultyScore"))
                    .or_else(|| question.get("rating"))
                    .and_then(|value| value.as_i64().or_else(|| value.as_str()?.parse().ok()));
                let difficulty = raw_difficulty
                    .filter(|value| *value > 0)
                    .map(|value| tracker_difficulty_score(value).to_string());
                let mut url = question
                    .get("questionUrl")
                    .or_else(|| question.get("problemUrl"))
                    .or_else(|| question.get("url"))
                    .or_else(|| question.get("link"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                if url.is_empty() && !contest_id.is_empty() {
                    if let Some(index) = question
                        .get("index")
                        .or_else(|| question.get("problemIndex"))
                        .or_else(|| question.get("questionIndex"))
                        .and_then(value_string)
                    {
                        url = format!("https://ac.nowcoder.com/acm/contest/{contest_id}/{index}");
                    }
                }
                let problem = TrackerProblem {
                    day: String::new(),
                    problem_id: problem_id.clone(),
                    title,
                    url: url.clone(),
                    difficulty,
                };
                let mut keys = vec![problem_id];
                if let Some(value) = question.get("questionId").and_then(value_string) {
                    keys.push(value);
                }
                keys.extend(url_keys(&url));
                keys.sort();
                keys.dedup();
                for key in keys {
                    result.by_key.insert(key, problem.clone());
                }
            }
        }
        if page >= 100 {
            break;
        }
        page += 1;
        polite_sleep(60).await;
    }
    Ok(result)
}

fn tracker_difficulty_score(value: i64) -> i64 {
    match value {
        1 => 800,
        2 => 1200,
        3 => 1600,
        4 => 2000,
        5 => 2400,
        6 => 2800,
        7 => 3000,
        8 => 3200,
        9 => 3400,
        10 => 3500,
        _ => value,
    }
}

async fn fetch_tracker_problems(client: &Client) -> Result<TrackerCatalog, SyncError> {
    let now = chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap());
    let mut year = now.year();
    let mut month = now.month() as i32;
    let mut result = TrackerCatalog::default();
    for _ in 0..18 {
        let url = format!(
            "https://www.nowcoder.com/problem/tracker/clock/monthinfo?year={year}&month={month}"
        );
        let payload = get_json(
            client,
            &url,
            with_referer(browser_headers(), "https://www.nowcoder.com/"),
        )
        .await?;
        if payload.get("code").and_then(Value::as_i64).unwrap_or(-1) == 0 {
            if let Some(items) = tracker_problem_rows(&payload) {
                for item in items {
                    let problem_id = item
                        .get("problemId")
                        .or_else(|| item.get("questionId"))
                        .and_then(|value| {
                            value
                                .as_i64()
                                .map(|x| x.to_string())
                                .or_else(|| value.as_str().map(str::to_string))
                        })
                        .unwrap_or_default();
                    if problem_id.is_empty() {
                        continue;
                    }
                    let day = item
                        .get("createTime")
                        .and_then(Value::as_i64)
                        .map(|value| {
                            china_day(if value > 10_000_000_000 {
                                value / 1000
                            } else {
                                value
                            })
                        })
                        .or_else(|| {
                            item.get("date")
                                .or_else(|| item.get("day"))
                                .and_then(Value::as_str)
                                .and_then(normalize_day)
                        })
                        .unwrap_or_default();
                    if day.is_empty() {
                        continue;
                    }
                    let title = item
                        .get("questionTitle")
                        .or_else(|| item.get("title"))
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let difficulty = item
                        .get("difficultyScore")
                        .or_else(|| item.get("difficulty"))
                        .and_then(|value| {
                            value.as_i64().map(|x| x.to_string()).or_else(|| {
                                value
                                    .as_str()
                                    .filter(|x| !x.is_empty() && *x != "N/A")
                                    .map(str::to_string)
                            })
                        });
                    let url = item
                        .get("questionUrl")
                        .or_else(|| item.get("url"))
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let mut keys = vec![problem_id.clone()];
                    if let Some(value) = item.get("questionId").and_then(value_string) {
                        keys.push(value);
                    }
                    if let Some(value) = item.get("problemId").and_then(value_string) {
                        keys.push(value);
                    }
                    keys.extend(url_keys(&url));
                    keys.sort();
                    keys.dedup();
                    let problem = TrackerProblem {
                        day: day.clone(),
                        problem_id,
                        title,
                        url,
                        difficulty,
                    };
                    for key in keys {
                        result.by_key.insert(key, problem.clone());
                    }
                    result.by_day.insert(day, problem);
                }
            }
        }
        month -= 1;
        if month == 0 {
            month = 12;
            year -= 1;
        }
        polite_sleep(90).await;
    }
    Ok(result)
}

fn tracker_problem_rows(payload: &Value) -> Option<&Vec<Value>> {
    payload
        .get("data")
        .and_then(Value::as_array)
        .or_else(|| payload.pointer("/data/list").and_then(Value::as_array))
        .or_else(|| payload.pointer("/data/records").and_then(Value::as_array))
        .or_else(|| payload.pointer("/data/result").and_then(Value::as_array))
}

async fn fetch_tracker_completed_days(
    client: &Client,
    cookie: &str,
) -> Result<HashSet<String>, SyncError> {
    let now = chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap());
    let mut year = now.year();
    let mut month = now.month() as i32;
    let mut result = HashSet::new();
    for _ in 0..18 {
        let url = format!(
            "https://www.nowcoder.com/problem/tracker/clock/list?year={year}&month={month}"
        );
        let payload = get_json(
            client,
            &url,
            with_raw_cookie(
                with_referer(
                    browser_headers(),
                    "https://www.nowcoder.com/problem/tracker",
                ),
                cookie,
            ),
        )
        .await?;
        let code = payload.get("code").and_then(Value::as_i64).unwrap_or(-1);
        if code != 0 {
            let message = payload
                .get("msg")
                .or_else(|| payload.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("请重新登录牛客并更新 Cookie");
            return Err(SyncError::auth(message));
        }
        if let Some(data) = payload.get("data") {
            collect_days(data, &mut result);
        }
        month -= 1;
        if month == 0 {
            month = 12;
            year -= 1;
        }
        polite_sleep(90).await;
    }
    Ok(result)
}

fn collect_days(value: &Value, out: &mut HashSet<String>) {
    match value {
        Value::String(text) => {
            if let Some(day) = normalize_day(text) {
                out.insert(day);
            }
        }
        Value::Number(number) => {
            if let Some(raw) = number.as_i64() {
                let epoch = if raw > 10_000_000_000 {
                    raw / 1000
                } else {
                    raw
                };
                if epoch > 1_500_000_000 {
                    out.insert(china_day(epoch));
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_days(item, out);
            }
        }
        Value::Object(map) => {
            for (key, item) in map {
                if let Some(day) = normalize_day(key) {
                    if item.as_bool().unwrap_or(true) {
                        out.insert(day);
                    }
                }
                collect_days(item, out);
            }
        }
        _ => {}
    }
}

fn value_string(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::to_string)
        .or_else(|| value.as_i64().map(|x| x.to_string()))
        .filter(|value| !value.is_empty())
}

fn normalize_day(value: &str) -> Option<String> {
    let head = value.get(0..10)?;
    NaiveDate::parse_from_str(head, "%Y-%m-%d")
        .ok()
        .map(|day| day.format("%Y-%m-%d").to_string())
}

fn china_day(epoch: i64) -> String {
    chrono::Utc
        .timestamp_opt(epoch, 0)
        .single()
        .unwrap_or_else(chrono::Utc::now)
        .with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap())
        .format("%Y-%m-%d")
        .to_string()
}

fn url_keys(url: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let normalized = url.trim().trim_end_matches('/');
    if normalized.is_empty() {
        return keys;
    }
    keys.push(normalized.to_string());
    if let Some(path) = normalized
        .split("//")
        .nth(1)
        .and_then(|value| value.find('/').map(|index| &value[index..]))
    {
        keys.push(
            path.split('?')
                .next()
                .unwrap_or(path)
                .trim_end_matches('/')
                .to_string(),
        );
    }
    let re_practice = Regex::new(r"/practice/([^/?#]+)").unwrap();
    if let Some(found) = re_practice.captures(normalized) {
        keys.push(found[1].to_string());
    }
    let re_problem = Regex::new(r"/acm/problem/(\d+)").unwrap();
    if let Some(found) = re_problem.captures(normalized) {
        keys.push(found[1].to_string());
    }
    let re_contest = Regex::new(r"/acm/contest/(\d+)/([^/?#]+)").unwrap();
    if let Some(found) = re_contest.captures(normalized) {
        keys.push(format!("{}/{}", &found[1], &found[2]));
    }
    keys
}

fn submission_keys(submission: &Submission) -> Vec<String> {
    let mut keys = vec![
        submission.problem_key.clone(),
        submission.problem_id.clone(),
    ];
    keys.extend(url_keys(&submission.problem_url));
    keys.sort();
    keys.dedup();
    keys
}

fn date_only_tracker_submission(uid: &str, item: &TrackerProblem) -> Submission {
    let epoch_second = NaiveDate::parse_from_str(&item.day, "%Y-%m-%d")
        .ok()
        .and_then(|day| day.and_hms_opt(12, 0, 0))
        .and_then(|local| {
            chrono::FixedOffset::east_opt(8 * 3600)?
                .from_local_datetime(&local)
                .single()
        })
        .map(|value| value.timestamp())
        .unwrap_or(0);
    Submission {
        platform: "nowcoder".into(),
        account: uid.into(),
        source: "daily".into(),
        source_day: Some(item.day.clone()),
        submission_id: format!("tracker-{uid}-{}", item.day),
        problem_key: format!("tracker:{}", item.problem_id),
        problem_id: item.problem_id.clone(),
        problem_name: item.title.clone(),
        problem_url: item.url.clone(),
        epoch_second,
        language: "Tracker 来源日期".into(),
        difficulty: item.difficulty.clone(),
        participant_type: String::new(),
        tags: vec![],
    }
}

fn parse_rows(html: &str, uid: &str) -> Vec<Submission> {
    let doc = Html::parse_document(html);
    let tr = Selector::parse("tr").unwrap();
    let td = Selector::parse("td").unwrap();
    let a_sel = Selector::parse("a").unwrap();
    let re_problem = Regex::new(r"/acm/problem/(\d+)").unwrap();
    let re_contest = Regex::new(r"/acm/contest/(\d+)/([^/?#]+)").unwrap();
    let re_practice = Regex::new(r"/practice/([^/?#]+)").unwrap();
    let mut out = Vec::new();
    for row in doc.select(&tr) {
        let cells: Vec<_> = row.select(&td).collect();
        if cells.len() < 9 {
            continue;
        }
        let verdict = cells[2].text().collect::<String>();
        if !(verdict.contains("答案正确")
            || verdict.to_ascii_lowercase().contains("accepted")
            || verdict.trim() == "AC")
        {
            continue;
        }
        let link = match cells[1]
            .select(&a_sel)
            .next()
            .and_then(|a| a.value().attr("href"))
        {
            Some(x) => x.to_string(),
            None => continue,
        };
        let problem_name = cells[1]
            .text()
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let language = cells[7]
            .text()
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let time_text = cells[8]
            .text()
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let ts = parse_china_time(&time_text);
        if ts <= 0 {
            continue;
        }
        let absolute = if link.starts_with("http") {
            link.clone()
        } else {
            format!("https://ac.nowcoder.com{link}")
        };
        let pid = re_problem
            .captures(&absolute)
            .map(|c| c[1].to_string())
            .or_else(|| re_practice.captures(&absolute).map(|c| c[1].to_string()))
            .or_else(|| {
                re_contest
                    .captures(&absolute)
                    .map(|c| format!("{}/{}", &c[1], &c[2]))
            })
            .unwrap_or_else(|| absolute.clone());
        let id = cells[0]
            .text()
            .collect::<String>()
            .chars()
            .filter(|c| c.is_ascii_digit())
            .collect::<String>();
        out.push(Submission {
            platform: "nowcoder".into(),
            account: uid.into(),
            source: "oj".into(),
            source_day: None,
            submission_id: if id.is_empty() {
                format!("{uid}-{ts}-{pid}")
            } else {
                id
            },
            problem_key: pid.clone(),
            problem_id: pid,
            problem_name,
            problem_url: absolute,
            epoch_second: ts,
            language,
            difficulty: None,
            participant_type: String::new(),
            tags: vec![],
        });
    }
    out
}

async fn enrich_tracker_difficulties(
    client: &Client,
    catalog: &mut TrackerCatalog,
    completed_days: &HashSet<String>,
    submissions: &[Submission],
) -> usize {
    let mut targets = HashMap::<String, String>::new();
    for day in completed_days {
        if let Some(item) = catalog.by_day.get(day) {
            targets.insert(item.problem_id.clone(), item.url.clone());
        }
    }
    for submission in submissions {
        if let Some(item) = catalog.find_submission(submission) {
            targets.insert(item.problem_id.clone(), item.url.clone());
        }
    }
    let difficulty_re = Regex::new(r#"difficulty_var\s*:\s*['\"](\d+)['\"]"#).unwrap();
    let mut found = HashMap::<String, String>::new();
    for (problem_id, path) in targets {
        let url = if path.starts_with("http") {
            path
        } else {
            format!("https://www.nowcoder.com{path}")
        };
        if let Ok(html) = get_text(
            client,
            &url,
            with_referer(
                browser_headers(),
                "https://www.nowcoder.com/problem/tracker",
            ),
        )
        .await
        {
            if let Some(value) = difficulty_re
                .captures(&html)
                .and_then(|capture| capture.get(1))
            {
                found.insert(problem_id, value.as_str().to_string());
            }
        }
        polite_sleep(70).await;
    }
    for item in catalog.by_day.values_mut() {
        if let Some(value) = found.get(&item.problem_id) {
            item.difficulty = Some(value.clone());
        }
    }
    for item in catalog.by_key.values_mut() {
        if let Some(value) = found.get(&item.problem_id) {
            item.difficulty = Some(value.clone());
        }
    }
    found.len()
}

fn parse_display_name(html: &str) -> Option<String> {
    let doc = Html::parse_document(html);
    let selector = Selector::parse(".coder-name").ok()?;
    let name = doc
        .select(&selector)
        .next()?
        .text()
        .collect::<String>()
        .trim()
        .to_string();
    (!name.is_empty()).then_some(name)
}

fn has_next_page(html: &str, current: i64) -> bool {
    let needle = format!("page={}", current + 1);
    html.contains(&needle)
}

fn parse_china_time(s: &str) -> i64 {
    let re = Regex::new(r"(\d{4})-(\d{2})-(\d{2})\s+(\d{2}):(\d{2}):(\d{2})").unwrap();
    let Some(c) = re.captures(s) else {
        return 0;
    };
    let text = format!(
        "{}-{}-{}T{}:{}:{}+08:00",
        &c[1], &c[2], &c[3], &c[4], &c[5], &c[6]
    );
    chrono::DateTime::parse_from_rfc3339(&text)
        .map(|x| x.timestamp())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracker_urls_match_practice_problem_keys() {
        let keys = url_keys("https://www.nowcoder.com/practice/abc-123?tpId=37");
        assert!(keys.iter().any(|key| key == "abc-123"));
        assert!(keys.iter().any(|key| key == "/practice/abc-123"));
    }

    #[test]
    fn public_profile_name_is_read_separately_from_numeric_uid() {
        let html = r#"<a class="coder-name rate-score2" data-title="Whalica">Whalica</a>"#;
        assert_eq!(parse_display_name(html).as_deref(), Some("Whalica"));
    }

    #[test]
    fn tracker_date_only_rows_keep_source_day() {
        let item = TrackerProblem {
            day: "2026-08-24".into(),
            problem_id: "42".into(),
            title: "每日一题".into(),
            url: "https://www.nowcoder.com/practice/example".into(),
            difficulty: None,
        };
        let row = date_only_tracker_submission("10001", &item);
        assert_eq!(row.source_day.as_deref(), Some("2026-08-24"));
        assert_eq!(china_day(row.epoch_second), "2026-08-24");
    }

    #[test]
    fn duplicate_rating_contests_keep_the_latest_row() {
        let payload = serde_json::json!({ "data": [
            { "contestId": 42, "contestName": "旧记录", "rating": 1500, "changeValue": 20, "time": 1000 },
            { "contestId": 42, "contestName": "新记录", "rating": 1510, "changeValue": 30, "time": 2000 }
        ]});
        let points = parse_rating_history(&payload);
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].contest_name, "新记录");
        assert_eq!(points[0].new_rating, 1510);
    }
}
