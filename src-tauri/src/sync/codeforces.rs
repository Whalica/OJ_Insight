use reqwest::Client;
use serde_json::Value;

use crate::models::{AccountConfig, RemoteData, Submission, SyncError};
use super::{browser_headers, get_json, polite_sleep, now_epoch};

pub async fn fetch(client: &Client, account: &AccountConfig, full: bool, cursor: i64) -> Result<RemoteData, SyncError> {
    let handle = account.account.trim();
    if handle.is_empty() { return Err(SyncError::error("Codeforces Handle 为空")); }
    let mut from = 1_i64;
    let page_size = 10_000_i64;
    let mut out = Vec::new();
    let mut max_seen = cursor;
    let stop_cursor = if full { 0 } else { cursor.saturating_sub(5) };
    let mut page = 0;

    loop {
        page += 1;
        if page > 200 { return Err(SyncError::error("Codeforces 分页过多，已中止")); }
        let url = format!("https://codeforces.com/api/user.status?handle={}&from={from}&count={page_size}", urlencoding::encode(handle));
        let payload = get_json(client, &url, browser_headers()).await?;
        if payload.get("status").and_then(Value::as_str) != Some("OK") {
            return Err(SyncError::error(payload.get("comment").and_then(Value::as_str).unwrap_or("Codeforces API 返回失败")));
        }
        let rows = payload.get("result").and_then(Value::as_array).ok_or_else(|| SyncError::error("Codeforces result 格式异常"))?;
        if rows.is_empty() { break; }
        let mut reached_old = false;
        for s in rows {
            let ts = s.get("creationTimeSeconds").and_then(Value::as_i64).unwrap_or(0);
            max_seen = max_seen.max(ts);
            if !full && ts <= stop_cursor { reached_old = true; continue; }
            if s.get("verdict").and_then(Value::as_str) != Some("OK") { continue; }
            let problem = s.get("problem").unwrap_or(&Value::Null);
            let contest_id = problem.get("contestId").or_else(|| s.get("contestId")).and_then(Value::as_i64);
            let index = problem.get("index").and_then(Value::as_str).unwrap_or("");
            let name = problem.get("name").and_then(Value::as_str).unwrap_or(index);
            let key = contest_id.map(|id| format!("{id}:{index}")).unwrap_or_else(|| format!("{name}:{index}"));
            let pid = contest_id.map(|id| format!("{id}{index}")).unwrap_or_else(|| key.clone());
            let url = contest_id.map(|id| {
                if id >= 100_000 { format!("https://codeforces.com/gym/{id}/problem/{index}") }
                else { format!("https://codeforces.com/contest/{id}/problem/{index}") }
            }).unwrap_or_else(|| "https://codeforces.com/problemset".to_string());
            out.push(Submission {
                platform: "codeforces".into(),
                submission_id: s.get("id").and_then(Value::as_i64).unwrap_or(ts).to_string(),
                problem_key: key,
                problem_id: pid,
                problem_name: name.to_string(),
                problem_url: url,
                epoch_second: ts,
                language: s.get("programmingLanguage").and_then(Value::as_str).unwrap_or("").to_string(),
                difficulty: problem.get("rating").and_then(Value::as_i64).map(|x| x.to_string()),
            });
        }
        if reached_old || rows.len() < page_size as usize { break; }
        from += rows.len() as i64;
        polite_sleep(2100).await;
    }

    Ok(RemoteData {
        platform: "codeforces".into(), account: handle.into(), submissions: out, aggregates: vec![], solved_count: None,
        difficulty: vec![], activity_only: false, notes: vec!["Codeforces 官方 user.status API".into()],
        cursor_epoch: max_seen.max(now_epoch().saturating_sub(2)), replace_submissions: full, replace_aggregates: full,
    })
}
