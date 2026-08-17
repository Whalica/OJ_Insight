use std::collections::{BTreeMap, BTreeSet};

use chrono::Datelike;
use reqwest::{
    header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE, ORIGIN, REFERER, USER_AGENT},
    Client,
};
use serde_json::{json, Value};

use crate::models::{AccountConfig, AggregateDay, DifficultyStat, RemoteData, SyncError};
use super::{now_epoch, polite_sleep, post_json};

struct LeetCodeSite {
    endpoint: &'static str,
    origin: &'static str,
    referer: &'static str,
    label: &'static str,
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
    if let Ok(v) = HeaderValue::from_str(site.origin) {
        h.insert(ORIGIN, v);
    }
    if let Ok(v) = HeaderValue::from_str(site.referer) {
        h.insert(REFERER, v);
    }
    h
}

const QUERY: &str = r#"
query userProfileCalendar($username: String!, $year: Int) {
  matchedUser(username: $username) {
    username
    submitStatsGlobal { acSubmissionNum { difficulty count submissions } }
    userCalendar(year: $year) { activeYears streak totalActiveDays submissionCalendar }
  }
}
"#;

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

    let first = post_json(
        client,
        site.endpoint,
        headers(&site),
        json!({"query": QUERY, "variables": {"username": user, "year": Value::Null}}),
    )
    .await?;
    let matched = first
        .pointer("/data/matchedUser")
        .filter(|v| !v.is_null())
        .ok_or_else(|| {
            SyncError::error(format!(
                "{} 用户不存在，或 GraphQL 未返回 matchedUser",
                site.label
            ))
        })?;

    let mut years = BTreeSet::new();
    if let Some(a) = matched
        .pointer("/userCalendar/activeYears")
        .and_then(Value::as_array)
    {
        for y in a {
            if let Some(n) = y.as_i64() {
                years.insert(n as i32);
            }
        }
    }
    years.insert(chrono::Utc::now().year());

    let mut calendar: BTreeMap<String, i64> = BTreeMap::new();
    for (idx, year) in years.into_iter().enumerate() {
        let payload = if idx == 0
            && chrono::Utc::now().year() == year
            && matched
                .pointer("/userCalendar/submissionCalendar")
                .is_some()
        {
            first.clone()
        } else {
            post_year(client, user, year, &site).await?
        };

        if let Some(s) = payload
            .pointer("/data/matchedUser/userCalendar/submissionCalendar")
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
        polite_sleep(180).await;
    }

    let mut aggregates = Vec::new();
    for (epoch, count) in calendar {
        if let Ok(ts) = epoch.parse::<i64>() {
            let day = crate::db::day_utc8(ts);
            aggregates.push(AggregateDay {
                day,
                metric: "activity".into(),
                count,
                note: format!(
                    "{} submissionCalendar：提交活动计数，不等同于首次 AC",
                    site.label
                ),
            });
        }
    }

    let mut solved_count = None;
    let mut difficulty = Vec::new();
    if let Some(arr) = matched
        .pointer("/submitStatsGlobal/acSubmissionNum")
        .and_then(Value::as_array)
    {
        for (i, row) in arr.iter().enumerate() {
            let label = row
                .get("difficulty")
                .and_then(Value::as_str)
                .unwrap_or("");
            let count = row.get("count").and_then(Value::as_i64).unwrap_or(0);
            if label == "All" {
                solved_count = Some(count);
            } else if !label.is_empty() {
                difficulty.push(DifficultyStat {
                    label: label.into(),
                    count,
                    order: i as i64,
                });
            }
        }
    }

    Ok(RemoteData {
        platform: "leetcode".into(),
        account: if site.label == "LeetCode CN" {
            format!("cn:{user}")
        } else {
            user.into()
        },
        submissions: vec![],
        aggregates,
        solved_count,
        difficulty,
        activity_only: true,
        notes: vec![
            format!("{} GraphQL userCalendar + submitStatsGlobal", site.label),
            "逐日 calendar 是提交活动，不提供完整历史逐题 AC 明细".into(),
        ],
        cursor_epoch: now_epoch(),
        replace_submissions: true,
        replace_aggregates: true,
    })
}

async fn post_year(
    client: &Client,
    user: &str,
    year: i32,
    site: &LeetCodeSite,
) -> Result<Value, SyncError> {
    post_json(
        client,
        site.endpoint,
        headers(site),
        json!({"query": QUERY, "variables": {"username": user, "year": year}}),
    )
    .await
}
