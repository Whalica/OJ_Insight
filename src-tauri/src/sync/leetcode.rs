use std::collections::{BTreeMap, BTreeSet};

use chrono::Datelike;
use reqwest::{
    header::{
        HeaderMap, HeaderName, HeaderValue, ACCEPT, CONTENT_TYPE, ORIGIN, REFERER, USER_AGENT,
    },
    Client,
};
use serde_json::{json, Value};

use super::{now_epoch, polite_sleep, post_json};
use crate::models::{AccountConfig, AggregateDay, DifficultyStat, RemoteData, SyncError};

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

fn headers(site: &LeetCodeSite) -> HeaderMap {
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
    h
}

const GLOBAL_QUERY: &str = r#"query userProfileCalendar($username: String!, $year: Int) { matchedUser(username: $username) { username submitStatsGlobal { acSubmissionNum { difficulty count submissions } } userCalendar(year: $year) { activeYears streak totalActiveDays submissionCalendar } } }"#;
const CN_PROGRESS_QUERY: &str = r#"query userQuestionProgress($userSlug: String!) { userProfileUserQuestionProgress(userSlug: $userSlug) { numAcceptedQuestions { count difficulty } } }"#;
const CN_PROGRESS_V2_QUERY: &str = r#"query userProfileUserQuestionProgressV2($userSlug: String!) { userProfileUserQuestionProgressV2(userSlug: $userSlug) { numAcceptedQuestions { count difficulty } } }"#;

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
    if site.kind == SiteKind::China {
        let empty = Value::Null;
        let (solved_count, difficulty) = profile_stats(client, user, &site, &empty).await?;
        return Ok(RemoteData {
            platform: "leetcode".into(),
            account: format!("cn:{user}"),
            submissions: vec![],
            aggregates: vec![],
            solved_count,
            difficulty,
            activity_only: true,
            notes: vec![
                "LeetCode 中国站公开个人资料使用独立 GraphQL schema".into(),
                "中国站目前没有稳定可用的公开 Activity 日历接口；解题总数与难度正常同步，已有日期缓存不会被清空。".into(),
            ],
            cursor_epoch: now_epoch(),
            replace_submissions: false,
            replace_aggregates: false,
        });
    }

    let (first, aggregates) = load_calendar(client, user, &site).await?;
    let (solved_count, difficulty) = profile_stats(client, user, &site, &first).await?;
    Ok(RemoteData {
        platform: "leetcode".into(),
        account: user.into(),
        submissions: vec![],
        aggregates,
        solved_count,
        difficulty,
        activity_only: true,
        notes: vec![
            format!("{} 独立 GraphQL provider", site.label),
            "逐日 calendar 是提交活动，不提供完整历史逐题 AC 明细".into(),
        ],
        cursor_epoch: now_epoch(),
        replace_submissions: true,
        replace_aggregates: true,
    })
}

async fn load_calendar(
    client: &Client,
    user: &str,
    site: &LeetCodeSite,
) -> Result<(Value, Vec<AggregateDay>), SyncError> {
    let current = chrono::Utc::now().year();
    let first = calendar_request(client, user, current, site).await?;
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
            calendar_request(client, user, year, site).await?
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
                day: crate::db::day_utc8(ts),
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
) -> Result<Value, SyncError> {
    let body = json!({"operationName":"userProfileCalendar","query":GLOBAL_QUERY,"variables":{"username":user,"year":year}});
    post_json(client, site.endpoint, headers(site), body).await
}

async fn profile_stats(
    client: &Client,
    user: &str,
    site: &LeetCodeSite,
    first: &Value,
) -> Result<(Option<i64>, Vec<DifficultyStat>), SyncError> {
    let rows = if site.kind == SiteKind::China {
        let v1 = json!({"operationName":"userQuestionProgress","query":CN_PROGRESS_QUERY,"variables":{"userSlug":user}});
        let value = match post_json(client, site.endpoint, headers(site), v1).await {
            Ok(value) => value,
            Err(v1_error) => {
                let v2 = json!({"operationName":"userProfileUserQuestionProgressV2","query":CN_PROGRESS_V2_QUERY,"variables":{"userSlug":user}});
                post_json(client, site.endpoint, headers(site), v2)
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
