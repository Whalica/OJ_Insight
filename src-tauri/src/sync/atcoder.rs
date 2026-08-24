use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;

use super::{browser_headers, get_json, get_text, now_epoch, polite_sleep};
use crate::models::{AccountConfig, RemoteData, Submission, SyncError};

pub async fn fetch(
    client: &Client,
    account: &AccountConfig,
    full: bool,
    cursor: i64,
) -> Result<RemoteData, SyncError> {
    let user = account.account.trim();
    if user.is_empty() {
        return Err(SyncError::error("AtCoder 用户名为空"));
    }
    let profile_url = format!("https://atcoder.jp/users/{}", urlencoding::encode(user));
    let _ = get_text(client, &profile_url, browser_headers()).await?;

    let problems = get_json(
        client,
        "https://kenkoooo.com/atcoder/resources/problems.json",
        browser_headers(),
    )
    .await
    .unwrap_or(Value::Array(vec![]));
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
    let models = get_json(
        client,
        "https://kenkoooo.com/atcoder/resources/problem-models.json",
        browser_headers(),
    )
    .await
    .unwrap_or(Value::Object(Default::default()));

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

    Ok(RemoteData {
        platform: "atcoder".into(),
        account: user.into(),
        submissions: out,
        aggregates: vec![],
        solved_count: None,
        difficulty: vec![],
        activity_only: false,
        notes: vec![
            "AtCoder Problems submission API；使用原始 epoch_second".into(),
            "增量同步回看 7 天，避免上游延迟入库造成漏记".into(),
        ],
        cursor_epoch: max_seen.max(now_epoch().saturating_sub(7 * 24 * 3600)),
        replace_submissions: full,
        replace_aggregates: full,
    })
}

fn atcoder_display_difficulty(value: f64) -> i64 {
    let adjusted = if value < 400.0 {
        (400.0 / ((400.0 - value) / 400.0).exp()).round() as i64
    } else {
        value.round() as i64
    };
    adjusted.max(0)
}
