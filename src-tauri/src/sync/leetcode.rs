use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Datelike, Utc};
use reqwest::{
    header::{
        HeaderMap, HeaderName, HeaderValue, ACCEPT, CONTENT_TYPE, ORIGIN, REFERER, USER_AGENT,
    },
    Client,
};
use serde_json::{json, Value};

use super::{now_epoch, polite_sleep, post_json, with_raw_cookie};
use crate::models::{
    AccountConfig, AggregateDay, DifficultyStat, RatingPoint, RemoteData, Submission, SyncError,
};

#[derive(Clone, Copy, PartialEq)]
enum SiteKind {
    Global,
    China,
}

struct LeetCodeSite {
    endpoint: &'static str,
    origin: &'static str,
    referer: &'static str,
    label: &'static str,
    kind: SiteKind,
}

fn parse_account(raw: &str) -> (&str, LeetCodeSite) {
    let raw = raw.trim();
    if let Some(user) = raw.strip_prefix("cn:").or_else(|| raw.strip_prefix("CN:")) {
        (
            user.trim(),
            LeetCodeSite {
                endpoint: "https://leetcode.cn/graphql/",
                origin: "https://leetcode.cn",
                referer: "https://leetcode.cn/",
                label: "LeetCode CN",
                kind: SiteKind::China,
            },
        )
    } else {
        (
            raw,
            LeetCodeSite {
                endpoint: "https://leetcode.com/graphql",
                origin: "https://leetcode.com",
                referer: "https://leetcode.com/",
                label: "LeetCode",
                kind: SiteKind::Global,
            },
        )
    }
}

fn headers(site: &LeetCodeSite, cookie: &str) -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(
        USER_AGENT,
        HeaderValue::from_static(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/151 Safari/537.36",
        ),
    );
    h.insert(ACCEPT, HeaderValue::from_static("application/json"));
    h.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    h.insert(ORIGIN, HeaderValue::from_str(site.origin).unwrap());
    h.insert(REFERER, HeaderValue::from_str(site.referer).unwrap());
    h.insert(
        HeaderName::from_static("x-requested-with"),
        HeaderValue::from_static("XMLHttpRequest"),
    );
    with_raw_cookie(h, cookie)
}

const GLOBAL_QUERY: &str = r#"query userProfileCalendar($username: String!, $year: Int) { matchedUser(username: $username) { username submitStatsGlobal { acSubmissionNum { difficulty count submissions } } userCalendar(year: $year) { activeYears streak totalActiveDays submissionCalendar } } recentAcSubmissionList(username: $username, limit: 20) { id title titleSlug timestamp } }"#;
const RATING_QUERY: &str = r#"query userContestRankingHistory($username: String!) { userContestRankingHistory(username: $username) { attended rating ranking contest { title startTime } } }"#;
const CN_PROGRESS_QUERY: &str = r#"query userQuestionProgress($userSlug: String!) { userProfileUserQuestionProgress(userSlug: $userSlug) { numAcceptedQuestions { count difficulty } } }"#;
const CN_PROGRESS_V2_QUERY: &str = r#"query userProfileUserQuestionProgressV2($userSlug: String!) { userProfileUserQuestionProgressV2(userSlug: $userSlug) { numAcceptedQuestions { count difficulty } } }"#;
const CN_CALENDAR_QUERY: &str = r#"query userProfileCalendar($userSlug: String!, $year: Int) { userProfileCalendar(userSlug: $userSlug, year: $year) { activeYears streak totalActiveDays submissionCalendar } }"#;
const CN_RECENT_QUERY: &str = r#"query recentACSubmissions($userSlug: String!) { recentACSubmissions(userSlug: $userSlug) { submissionId submitTime question { questionFrontendId title titleSlug translatedTitle } } }"#;

pub async fn fetch(
    client: &Client,
    account: &AccountConfig,
    _full: bool,
    _cursor: i64,
) -> Result<RemoteData, SyncError> {
    let (user, site) = parse_account(&account.account);
    if user.is_empty() {
        return Err(SyncError::error("LeetCode 用户名为空"));
    }
    let cookie = account.secret.trim();
    if site.kind == SiteKind::China {
        let empty = Value::Null;
        let (solved_count, difficulty) = profile_stats(client, user, &site, &empty, cookie).await?;
        let (aggregates, submissions, calendar_note) = load_cn_activity(client, user, &site, cookie).await;
        let has_aggregates = !aggregates.is_empty();
        return Ok(RemoteData {
            platform: "leetcode".into(),
            account: format!("cn:{user}"),
            submissions,
            aggregates,
            solved_count,
            difficulty,
            ratings: None,
            activity_only: true,
            notes: vec![
                "LeetCode 中国站公开个人资料使用独立 GraphQL schema".into(),
                calendar_note,
            ],
            cursor_epoch: now_epoch(),
            replace_submissions: false,
            replace_aggregates: has_aggregates,
        });
    }

    let (first, aggregates) = load_calendar(client, user, &site, cookie).await?;
    let (solved_count, difficulty) = profile_stats(client, user, &site, &first, cookie).await?;
    let submissions = recent_submissions(user, &first);
    let ratings = post_json(client, site.endpoint, headers(&site, cookie),
        json!({"operationName":"userContestRankingHistory","query":RATING_QUERY,"variables":{"username":user}}))
        .await.ok().and_then(|payload| contest_rating_history(&payload));
    Ok(RemoteData {
        platform: "leetcode".into(),
        account: user.into(),
        submissions,
        aggregates,
        solved_count,
        difficulty,
        ratings,
        activity_only: true,
        notes: vec![
            format!("{} 独立 GraphQL provider", site.label),
            "逐日 calendar 是提交活动，不提供完整历史逐题 AC 明细".into(),
        ],
        cursor_epoch: now_epoch(),
        replace_submissions: false,
        replace_aggregates: true,
    })
}

fn contest_rating_history(payload: &Value) -> Option<Vec<RatingPoint>> {
    if payload.get("errors").and_then(Value::as_array).is_some_and(|errors| !errors.is_empty()) { return None; }
    let history = payload.pointer("/data/userContestRankingHistory")?.as_array()?;
    if history.iter().any(|row| row.get("attended").and_then(Value::as_bool).is_none()) { return None; }
    let mut rows: Vec<_> = history.iter()
        .filter(|row| row.get("attended").and_then(Value::as_bool) == Some(true)).collect();
    rows.sort_by_key(|row| row.pointer("/contest/startTime").and_then(Value::as_i64).unwrap_or(0));
    let mut previous = 0_i64;
    let mut points = Vec::new();
    for row in rows {
        if !row.get("attended").and_then(Value::as_bool).unwrap_or(false) {
            continue;
        }
        let contest = row.get("contest");
        let epoch_second = row.pointer("/contest/startTime")?.as_i64()?;
        if epoch_second <= 0 { return None; }
        let new_rating = row.get("rating").and_then(Value::as_f64)?.round() as i64;
        let title = contest
            .and_then(|value| value.get("title"))
            .and_then(Value::as_str)
            .unwrap_or("LeetCode Contest");
        points.push(RatingPoint {
            contest_id: format!("{epoch_second}-{title}"),
            contest_name: title.to_string(),
            epoch_second,
            old_rating: if previous == 0 { new_rating } else { previous },
            new_rating,
            rank: row.get("ranking").and_then(Value::as_i64),
        });
        previous = new_rating;
    }
    Some(points)
}

fn recent_submissions(user: &str, payload: &Value) -> Vec<Submission> {
    payload
        .pointer("/data/recentAcSubmissionList")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let slug = row.get("titleSlug").and_then(Value::as_str)?;
                    let ts = row.get("timestamp").and_then(|value| {
                        value
                            .as_i64()
                            .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
                    })?;
                    let id = row
                        .get("id")
                        .and_then(|value| {
                            value
                                .as_str()
                                .map(str::to_string)
                                .or_else(|| value.as_i64().map(|x| x.to_string()))
                        })
                        .unwrap_or_else(|| format!("{user}-{ts}-{slug}"));
                    Some(Submission {
                        platform: "leetcode".into(),
                        account: user.into(),
                        source: "oj".into(),
                        source_day: None,
                        submission_id: id,
                        problem_key: slug.into(),
                        problem_id: slug.into(),
                        problem_name: row
                            .get("title")
                            .and_then(Value::as_str)
                            .unwrap_or(slug)
                            .into(),
                        problem_url: format!("https://leetcode.com/problems/{slug}/"),
                        epoch_second: ts,
                        language: String::new(),
                        difficulty: None,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

async fn load_calendar(
    client: &Client,
    user: &str,
    site: &LeetCodeSite,
    cookie: &str,
) -> Result<(Value, Vec<AggregateDay>), SyncError> {
    let current = chrono::Utc::now().year();
    let first = calendar_request(client, user, current, site, cookie).await?;
    let initial_calendar = calendar_node(&first)
        .filter(|v| !v.is_null())
        .ok_or_else(|| {
            SyncError::error(format!(
                "{} 用户不存在，或 userCalendar 未返回数据",
                site.label
            ))
        })?;
    let mut years = BTreeSet::new();
    if let Some(a) = initial_calendar
        .get("activeYears")
        .and_then(Value::as_array)
    {
        for y in a {
            if let Some(n) = y.as_i64() {
                years.insert(n as i32);
            }
        }
    }
    years.insert(current);
    let mut calendar: BTreeMap<String, i64> = BTreeMap::new();
    for year in years {
        let payload = if year == current {
            first.clone()
        } else {
            calendar_request(client, user, year, site, cookie).await?
        };
        if let Some(s) = calendar_node(&payload)
            .and_then(|v| v.get("submissionCalendar"))
            .and_then(Value::as_str)
        {
            if let Ok(map) = serde_json::from_str::<BTreeMap<String, i64>>(s) {
                for (epoch, count) in map {
                    if count > 0 {
                        calendar.insert(epoch, count);
                    }
                }
            }
        }
        polite_sleep(160).await;
    }
    let aggregates = calendar
        .into_iter()
        .filter_map(|(epoch, count)| {
            epoch.parse::<i64>().ok().map(|ts| AggregateDay {
                day: DateTime::from_timestamp(ts, 0)
                    .unwrap_or_else(Utc::now)
                    .format("%Y-%m-%d")
                    .to_string(),
                epoch_second: Some(ts),
                metric: "activity".into(),
                count,
                note: format!(
                    "{} submissionCalendar：提交活动计数，不等同于首次 AC",
                    site.label
                ),
            })
        })
        .collect();
    Ok((first, aggregates))
}

fn calendar_node(payload: &Value) -> Option<&Value> {
    payload.pointer("/data/matchedUser/userCalendar")
}

async fn calendar_request(
    client: &Client,
    user: &str,
    year: i32,
    site: &LeetCodeSite,
    cookie: &str,
) -> Result<Value, SyncError> {
    let body = json!({"operationName":"userProfileCalendar","query":GLOBAL_QUERY,"variables":{"username":user,"year":year}});
    post_json(client, site.endpoint, headers(site, cookie), body).await
}

async fn load_cn_activity(
    client: &Client,
    user: &str,
    site: &LeetCodeSite,
    cookie: &str,
) -> (Vec<AggregateDay>, Vec<Submission>, String) {
    let current = chrono::Utc::now().year();
    let body = json!({
        "operationName": "userProfileCalendar",
        "query": CN_CALENDAR_QUERY,
        "variables": { "userSlug": user, "year": current }
    });
    let first = match post_cn_activity(client, site, cookie, body).await {
        Ok(value) => value,
        Err(error) => {
            return (
                Vec::new(),
                Vec::new(),
                format!(
                    "活动日历接口暂不可用（{}）；已有活动砖缓存会保留。{}",
                    error.message,
                    if cookie.is_empty() { "可在账号设置中填写对应站点 Cookie 后重试。" } else { "已携带 Cookie 请求。" }
                ),
            )
        }
    };
    let mut years = BTreeSet::new();
    if let Some(items) = first
        .pointer("/data/userProfileCalendar/activeYears")
        .and_then(Value::as_array)
    {
        for item in items {
            if let Some(year) = item.as_i64() {
                years.insert(year as i32);
            }
        }
    }
    years.insert(current);
    let mut calendar = BTreeMap::new();
    for year in years {
        let payload = if year == current {
            first.clone()
        } else {
            let body = json!({
                "operationName": "userProfileCalendar",
                "query": CN_CALENDAR_QUERY,
                "variables": { "userSlug": user, "year": year }
            });
            match post_cn_activity(client, site, cookie, body).await {
                Ok(value) => value,
                Err(_) => continue,
            }
        };
        if let Some(serialized) = payload
            .pointer("/data/userProfileCalendar/submissionCalendar")
            .and_then(Value::as_str)
        {
            if let Ok(rows) = serde_json::from_str::<BTreeMap<String, i64>>(serialized) {
                calendar.extend(rows.into_iter().filter(|(_, count)| *count > 0));
            }
        }
        polite_sleep(120).await;
    }
    let aggregates = calendar
        .into_iter()
        .filter_map(|(epoch, count)| {
            epoch.parse::<i64>().ok().map(|ts| AggregateDay {
                day: DateTime::from_timestamp(ts, 0)
                    .unwrap_or_else(Utc::now)
                    .format("%Y-%m-%d")
                    .to_string(),
                epoch_second: Some(ts),
                metric: "activity".into(),
                count,
                note: "LeetCode CN submissionCalendar：提交活动计数".into(),
            })
        })
        .collect::<Vec<_>>();

    let recent_body = json!({
        "operationName": "recentACSubmissions",
        "query": CN_RECENT_QUERY,
        "variables": { "userSlug": user }
    });
    let submissions = post_cn_activity(client, site, cookie, recent_body)
        .await
        .ok()
        .and_then(|payload| payload.pointer("/data/recentACSubmissions").cloned())
        .and_then(|value| value.as_array().cloned())
        .map(|rows| {
            rows.into_iter()
                .filter_map(|row| {
                    let question = row.get("question")?;
                    let slug = question.get("titleSlug").and_then(Value::as_str)?;
                    let ts = row.get("submitTime").and_then(|value| {
                        value
                            .as_i64()
                            .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
                    })?;
                    let id = row
                        .get("submissionId")
                        .and_then(|value| {
                            value
                                .as_str()
                                .map(str::to_string)
                                .or_else(|| value.as_i64().map(|number| number.to_string()))
                        })
                        .unwrap_or_else(|| format!("cn-{user}-{ts}-{slug}"));
                    Some(Submission {
                        platform: "leetcode".into(),
                        account: format!("cn:{user}"),
                        source: "oj".into(),
                        source_day: None,
                        submission_id: id,
                        problem_key: slug.into(),
                        problem_id: question
                            .get("questionFrontendId")
                            .and_then(Value::as_str)
                            .unwrap_or(slug)
                            .into(),
                        problem_name: question
                            .get("translatedTitle")
                            .or_else(|| question.get("title"))
                            .and_then(Value::as_str)
                            .unwrap_or(slug)
                            .into(),
                        problem_url: format!("https://leetcode.cn/problems/{slug}/"),
                        epoch_second: ts,
                        language: String::new(),
                        difficulty: None,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let note = format!(
        "活动日历 {} 天，最近 AC {} 条；日期来自官方日历 epoch，并按所选时区显示。",
        aggregates.len(),
        submissions.len()
    );
    (aggregates, submissions, note)
}

async fn post_cn_activity(
    client: &Client,
    site: &LeetCodeSite,
    cookie: &str,
    body: Value,
) -> Result<Value, SyncError> {
    match post_json(client, site.endpoint, headers(site, cookie), body.clone()).await {
        Ok(value) => Ok(value),
        Err(primary) => post_json(
            client,
            "https://leetcode.cn/graphql/noj-go/",
            headers(site, cookie),
            body,
        )
        .await
        .map_err(|fallback| {
            SyncError::error(format!(
                "标准接口：{}；noj-go 回退：{}",
                primary.message, fallback.message
            ))
        }),
    }
}

async fn profile_stats(
    client: &Client,
    user: &str,
    site: &LeetCodeSite,
    first: &Value,
    cookie: &str,
) -> Result<(Option<i64>, Vec<DifficultyStat>), SyncError> {
    let rows = if site.kind == SiteKind::China {
        let v1 = json!({"operationName":"userQuestionProgress","query":CN_PROGRESS_QUERY,"variables":{"userSlug":user}});
        let value = match post_json(client, site.endpoint, headers(site, cookie), v1).await {
            Ok(value) => value,
            Err(v1_error) => {
                let v2 = json!({"operationName":"userProfileUserQuestionProgressV2","query":CN_PROGRESS_V2_QUERY,"variables":{"userSlug":user}});
                post_json(client, site.endpoint, headers(site, cookie), v2)
                    .await
                    .map_err(|v2_error| {
                        SyncError::error(format!(
                            "LeetCode CN 难度统计请求失败（V1：{}；V2：{}）",
                            v1_error.message, v2_error.message
                        ))
                    })?
            }
        };
        value
            .pointer("/data/userProfileUserQuestionProgress/numAcceptedQuestions")
            .or_else(|| {
                value.pointer("/data/userProfileUserQuestionProgressV2/numAcceptedQuestions")
            })
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    } else {
        first
            .pointer("/data/matchedUser/submitStatsGlobal/acSubmissionNum")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    };
    let mut solved = None;
    let mut difficulty = Vec::new();
    let mut sum = 0;
    for (i, row) in rows.iter().enumerate() {
        let label = row.get("difficulty").and_then(Value::as_str).unwrap_or("");
        let count = row.get("count").and_then(Value::as_i64).unwrap_or(0);
        if label.eq_ignore_ascii_case("all") {
            solved = Some(count);
        } else if !label.is_empty() {
            sum += count;
            difficulty.push(DifficultyStat {
                label: label.into(),
                count,
                order: i as i64,
            });
        }
    }
    if solved.is_none() && !difficulty.is_empty() {
        solved = Some(sum);
    }
    Ok((solved, difficulty))
}
