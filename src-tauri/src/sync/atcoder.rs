use chrono::DateTime;
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;

use super::metadata_cache::{self, Resource};
use super::{browser_headers, get_json, get_text, now_epoch, polite_sleep};
use crate::models::{AccountConfig, RatingPoint, RemoteData, Submission, SyncError};

pub async fn fetch(
    client: &Client,
    account: &AccountConfig,
    full: bool,
    cursor: i64,
    cache_dir: &std::path::Path,
) -> Result<RemoteData, SyncError> {
    let user = account.account.trim();
    if user.is_empty() {
        return Err(SyncError::error("AtCoder 用户名为空"));
    }
    let profile_url = format!("https://atcoder.jp/users/{}", urlencoding::encode(user));
    let _ = get_text(client, &profile_url, browser_headers()).await?;

    let mut notes = vec![
        "AtCoder Problems submission API；使用原始 epoch_second".into(),
        "增量同步回看 7 天，避免上游延迟入库造成漏记".into(),
    ];
    let problems = reference_data(
        client,
        cache_dir,
        Resource::Problems,
        full,
        "题目名称",
        &mut notes,
    )
    .await;
    let mut titles: HashMap<String, (String, String)> = HashMap::new();
    if let Some(rows) = problems.as_array() {
        for p in rows {
            if let Some(id) = p.get("id").and_then(Value::as_str) {
                titles.insert(
                    id.to_string(),
                    (
                        p.get("title")
                            .and_then(Value::as_str)
                            .unwrap_or(id)
                            .to_string(),
                        p.get("contest_id")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                    ),
                );
            }
        }
    }
    let models = reference_data(
        client,
        cache_dir,
        Resource::Models,
        full,
        "题目难度",
        &mut notes,
    )
    .await;

    let mut from_second = if full {
        0
    } else {
        cursor.saturating_sub(7 * 24 * 3600).max(0)
    };
    let mut out = Vec::new();
    let mut max_seen = cursor;
    for _ in 0..5000 {
        let url = format!("https://kenkoooo.com/atcoder/atcoder-api/v3/user/submissions?user={}&from_second={from_second}", urlencoding::encode(user));
        let payload = get_json(client, &url, browser_headers()).await?;
        let rows = payload
            .as_array()
            .ok_or_else(|| SyncError::error("AtCoder Problems 返回格式异常"))?;
        if rows.is_empty() {
            break;
        }
        for s in rows {
            let ts = s.get("epoch_second").and_then(Value::as_i64).unwrap_or(0);
            max_seen = max_seen.max(ts);
            if s.get("result").and_then(Value::as_str) != Some("AC") {
                continue;
            }
            let problem_id = s.get("problem_id").and_then(Value::as_str).unwrap_or("");
            let contest_id = s.get("contest_id").and_then(Value::as_str).unwrap_or("");
            let (title, fallback_contest) = titles
                .get(problem_id)
                .cloned()
                .unwrap_or((problem_id.into(), contest_id.into()));
            let cid = if contest_id.is_empty() {
                fallback_contest
            } else {
                contest_id.into()
            };
            let difficulty = models
                .get(problem_id)
                .and_then(|v| v.get("difficulty"))
                .and_then(Value::as_f64)
                .map(atcoder_display_difficulty)
                .map(|x| x.to_string());
            out.push(Submission {
                platform: "atcoder".into(),
                account: user.into(),
                source: "oj".into(),
                source_day: None,
                submission_id: s
                    .get("id")
                    .and_then(Value::as_i64)
                    .unwrap_or(ts)
                    .to_string(),
                problem_key: problem_id.into(),
                problem_id: problem_id.into(),
                problem_name: title,
                problem_url: if cid.is_empty() {
                    "https://atcoder.jp/contests/".into()
                } else {
                    format!("https://atcoder.jp/contests/{cid}/tasks/{problem_id}")
                },
                epoch_second: ts,
                language: s
                    .get("language")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .into(),
                difficulty,
                participant_type: String::new(),
                tags: vec![],
            });
        }
        if rows.len() < 500 {
            break;
        }
        let last = rows
            .last()
            .and_then(|v| v.get("epoch_second"))
            .and_then(Value::as_i64)
            .ok_or_else(|| SyncError::error("AtCoder 分页缺少时间字段"))?;
        let next = last + 1;
        if next <= from_second {
            return Err(SyncError::error("AtCoder 分页游标未前进"));
        }
        from_second = next;
        polite_sleep(1100).await;
    }

    let ratings = fetch_rating_history(client, user).await.ok();

    Ok(RemoteData {
        platform: "atcoder".into(),
        account: user.into(),
        display_name: None,
        submissions: out,
        aggregates: vec![],
        solved_count: None,
        difficulty: vec![],
        knowledge: None,
        solved_inventory: None,
        ratings,
        activity_only: false,
        notes,
        cursor_epoch: max_seen.max(now_epoch().saturating_sub(7 * 24 * 3600)),
        replace_submissions: full,
        replace_aggregates: full,
    })
}

async fn reference_data(
    client: &Client,
    directory: &std::path::Path,
    resource: Resource,
    force: bool,
    label: &str,
    notes: &mut Vec<String>,
) -> Value {
    match metadata_cache::get(client, directory, resource, force).await {
        Ok(data) => {
            if data.stale {
                notes.push(format!(
                    "警告：{label}暂未更新，使用最近 7 天内的公共题库缓存"
                ));
            }
            data.value
        }
        Err(_) => {
            notes.push(format!("警告：{label}暂不可用，提交记录仍会同步"));
            Value::Null
        }
    }
}

async fn fetch_rating_history(client: &Client, user: &str) -> Result<Vec<RatingPoint>, SyncError> {
    let url = format!(
        "https://atcoder.jp/users/{}/history/json",
        urlencoding::encode(user)
    );
    let payload = get_json(client, &url, browser_headers()).await?;
    let rows = payload
        .as_array()
        .ok_or_else(|| SyncError::error("AtCoder Rating 历史格式异常"))?;
    if rows
        .iter()
        .any(|row| row.get("IsRated").and_then(Value::as_bool).is_none())
    {
        return Err(SyncError::error("AtCoder Rating 标识不完整，保留旧缓存"));
    }
    rows.iter()
        .filter(|row| row.get("IsRated").and_then(Value::as_bool).unwrap_or(false))
        .map(|row| {
            let epoch_second = DateTime::parse_from_rfc3339(row.get("EndTime")?.as_str()?)
                .ok()?
                .timestamp();
            let contest_id = row
                .get("ContestScreenName")
                .and_then(Value::as_str)
                .unwrap_or("")
                .split('.')
                .next()
                .unwrap_or("");
            if contest_id.is_empty() {
                return None;
            }
            Some(RatingPoint {
                contest_id: contest_id.to_string(),
                contest_name: row
                    .get("ContestName")
                    .and_then(Value::as_str)
                    .unwrap_or(contest_id)
                    .to_string(),
                epoch_second,
                old_rating: row.get("OldRating")?.as_i64()?,
                new_rating: row.get("NewRating")?.as_i64()?,
                rank: row.get("Place").and_then(Value::as_i64),
            })
        })
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| SyncError::error("Rating 历史存在不完整记录，保留旧缓存"))
}

fn atcoder_display_difficulty(value: f64) -> i64 {
    let adjusted = if value < 400.0 {
        (400.0 / ((400.0 - value) / 400.0).exp()).round() as i64
    } else {
        value.round() as i64
    };
    adjusted.max(0)
}
