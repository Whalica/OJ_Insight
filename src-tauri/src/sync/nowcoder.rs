use chrono::{Datelike, NaiveDate, TimeZone};
use regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

use super::{
    browser_headers, get_json, get_text, now_epoch, polite_sleep, with_raw_cookie, with_referer,
};
use crate::models::{AccountConfig, RemoteData, Submission, SyncError};

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
    let (tracker, tracker_note) = match fetch_tracker_problems(client).await {
        Ok(items) => (items, "已读取牛客 Tracker 题目日历".to_string()),
        Err(error) => (
            TrackerCatalog::default(),
            format!("警告：牛客 Tracker 暂不可用（{}）", error.message),
        ),
    };
    let cookie = account.secret.trim();
    let (completed_days, completion_verified, completion_note) = if cookie.is_empty() {
        (
            HashSet::new(),
            false,
            "未填写 Cookie；普通 OJ 提交正常统计，Tracker 完成日需登录 Cookie".to_string(),
        )
    } else {
        match fetch_tracker_completed_days(client, cookie).await {
            Ok(days) => {
                let count = days.len();
                (days, true, format!("Tracker 登录记录 {count} 天"))
            }
            Err(error) => (
                HashSet::new(),
                false,
                format!("警告：Tracker Cookie 未生效（{}）", error.message),
            ),
        }
    };
    let mut daily_matches = 0;
    let mut matched_days = HashSet::new();
    for submission in &mut out {
        let item = tracker.find_submission(submission);
        if let Some(item) = item {
            let real_day = china_day(submission.epoch_second);
            let confirmed = !completion_verified
                || completed_days.contains(&item.day)
                || completed_days.contains(&real_day);
            if !confirmed {
                continue;
            }
            submission.source = "daily".into();
            if submission.problem_name.trim().is_empty() && !item.title.is_empty() {
                submission.problem_name = item.title.clone();
            }
            if submission.difficulty.is_none() {
                submission.difficulty = item.difficulty.clone();
            }
            daily_matches += 1;
            matched_days.insert(item.day.clone());
        }
    }
    let mut date_only = 0;
    for day in &completed_days {
        if matched_days.contains(day) {
            continue;
        }
        let Some(item) = tracker.by_day.get(day) else {
            continue;
        };
        out.push(date_only_tracker_submission(uid, item));
        date_only += 1;
    }
    Ok(RemoteData {
        platform: "nowcoder".into(),
        account: uid.into(),
        submissions: out,
        aggregates: vec![],
        solved_count: None,
        difficulty: vec![],
        ratings: None,
        activity_only: false,
        notes: vec![
            "牛客竞赛站公开练习提交页 · statusTypeFilter=5".into(),
            format!(
                "{tracker_note} · {completion_note} · 匹配 {daily_matches} 条真实 AC，补充 {date_only} 条仅有来源日期的记录"
            ),
        ],
        cursor_epoch: max_seen.max(now_epoch().saturating_sub(48 * 3600)),
        replace_submissions: full,
        replace_aggregates: full,
    })
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

async fn fetch_tracker_problems(
    client: &Client,
) -> Result<TrackerCatalog, SyncError> {
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
                        .map(|value| china_day(if value > 10_000_000_000 { value / 1000 } else { value }))
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
                with_referer(browser_headers(), "https://www.nowcoder.com/problem/tracker"),
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
                let epoch = if raw > 10_000_000_000 { raw / 1000 } else { raw };
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
    if let Some(path) = normalized.split("//").nth(1).and_then(|value| value.find('/').map(|index| &value[index..])) {
        keys.push(path.split('?').next().unwrap_or(path).trim_end_matches('/').to_string());
    }
    let re_practice = Regex::new(r"/practice/([^/?#]+)").unwrap();
    if let Some(found) = re_practice.captures(normalized) {
        keys.push(found[1].to_string());
    }
    let re_problem = Regex::new(r"/acm/problem/(\d+)").unwrap();
    if let Some(found) = re_problem.captures(normalized) {
        keys.push(found[1].to_string());
    }
    keys
}

fn submission_keys(submission: &Submission) -> Vec<String> {
    let mut keys = vec![submission.problem_key.clone(), submission.problem_id.clone()];
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
        });
    }
    out
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
}
