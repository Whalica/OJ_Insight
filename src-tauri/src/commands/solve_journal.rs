use serde::{Deserialize, Serialize};
use rusqlite::{params, Connection};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

use crate::app::state::AppState;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SolveRecord {
    id: String,
    title: String,
    url: String,
    platform: String,
    status: String,
    outcome: String,
    created_at: i64,
    updated_at: i64,
    ended_at: Option<i64>,
    elapsed_ms: i64,
    note: String,
    tags: Vec<String>,
    mistakes: Vec<SolveMistake>,
    #[serde(default)]
    source_group: String,
    #[serde(default)]
    source_batch_id: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SolveMistake {
    id: String,
    at: i64,
    reason: String,
    note: String,
    lost_minutes: Option<i64>,
}

fn validate(record: &SolveRecord) -> Result<(), String> {
    if record.id.len() > 100 || record.id.is_empty() || !record.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err("无效的做题记录 ID".into());
    }
    if !["draft", "finished"].contains(&record.status.as_str()) || !["pending", "solved", "unfinished"].contains(&record.outcome.as_str()) {
        return Err("无效的做题记录状态".into());
    }
    if record.title.len() > 1000 || record.url.len() > 4000 || record.note.len() > 1_000_000 || record.tags.len() > 100 || record.mistakes.len() > 1000 || record.elapsed_ms < 0 || record.source_group.len() > 1000 || record.source_batch_id.len() > 200 {
        return Err("做题记录超出大小限制".into());
    }
    if record.mistakes.iter().any(|m| m.lost_minutes.is_some_and(|value| value < 0 || value > 100_000)) {
        return Err("失误耗时无效".into());
    }
    Ok(())
}

pub(crate) struct CompanionDraftInput {
    pub(crate) title: String,
    pub(crate) url: String,
    pub(crate) group: String,
    pub(crate) batch_id: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CompanionDraftResult {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) created: bool,
}

fn normalized_url(url: &str) -> String {
    url.split('#').next().unwrap_or(url).trim_end_matches('/').to_string()
}

fn platform_for_url(url: &reqwest::Url) -> &'static str {
    let host = url.host_str().unwrap_or("").to_ascii_lowercase();
    if host == "codeforces.com" || host.ends_with(".codeforces.com") { "codeforces" }
    else if host == "atcoder.jp" || host.ends_with(".atcoder.jp") { "atcoder" }
    else if host == "luogu.com.cn" || host.ends_with(".luogu.com.cn") { "luogu" }
    else if host == "ac.nowcoder.com" || host == "nowcoder.com" { "nowcoder" }
    else if host == "qoj.ac" || host.ends_with(".qoj.ac") { "qoj" }
    else if host == "leetcode.com" || host == "leetcode.cn" { "leetcode" }
    else { "other" }
}

static DRAFT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(crate) fn store_companion_draft(conn: &mut Connection, input: CompanionDraftInput) -> Result<CompanionDraftResult, String> {
    let parsed = reqwest::Url::parse(input.url.trim()).map_err(|_| "题目链接无效".to_string())?;
    if !["http", "https"].contains(&parsed.scheme()) || parsed.host_str().is_none() { return Err("仅支持 HTTP 或 HTTPS 题目链接".into()); }
    let url = normalized_url(parsed.as_str());
    let title = input.title.trim();
    if title.is_empty() || title.len() > 1000 || url.len() > 4000 || input.group.len() > 1000 || input.batch_id.len() > 200 { return Err("题目数据不完整或过长".into()); }
    {
        let mut stmt = conn.prepare("SELECT payload FROM solve_records WHERE status='draft' ORDER BY updated_at DESC")
            .map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0)).map_err(|e| e.to_string())?;
        for row in rows {
            let record: SolveRecord = serde_json::from_str(&row.map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
            if !record.url.is_empty() && normalized_url(&record.url) == url {
                return Ok(CompanionDraftResult { id: record.id, title: record.title, created: false });
            }
        }
    }
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?;
    let millis = now.as_millis().min(i64::MAX as u128) as i64;
    let id = format!("cc-{}-{}-{}", std::process::id(), now.as_nanos(), DRAFT_SEQUENCE.fetch_add(1, Ordering::Relaxed));
    let record = SolveRecord {
        id: id.clone(), title: title.to_string(), url, platform: platform_for_url(&parsed).to_string(),
        status: "draft".into(), outcome: "pending".into(), created_at: millis, updated_at: millis,
        ended_at: None, elapsed_ms: 0, note: String::new(), tags: Vec::new(), mistakes: Vec::new(),
        source_group: input.group.trim().to_string(), source_batch_id: input.batch_id,
    };
    validate(&record)?;
    let payload = serde_json::to_string(&record).map_err(|e| e.to_string())?;
    conn.execute("INSERT INTO solve_records(id, status, updated_at, payload) VALUES(?1, 'draft', ?2, ?3)", params![record.id, record.updated_at, payload])
        .map_err(|e| e.to_string())?;
    Ok(CompanionDraftResult { id, title: record.title, created: true })
}

#[tauri::command]
pub(crate) fn list_solve_records(state: State<'_, AppState>) -> Result<Vec<SolveRecord>, String> {
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    let mut stmt = conn.prepare("SELECT payload FROM solve_records ORDER BY updated_at DESC")
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0)).map_err(|e| e.to_string())?;
    rows.map(|row| serde_json::from_str(&row.map_err(|e| e.to_string())?)
        .map_err(|e| format!("读取做题记录失败：{e}"))).collect()
}

#[tauri::command]
pub(crate) fn save_solve_record(state: State<'_, AppState>, record: SolveRecord) -> Result<(), String> {
    validate(&record)?;
    let payload = serde_json::to_string(&record).map_err(|e| e.to_string())?;
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    conn.execute("INSERT INTO solve_records(id, status, updated_at, payload) VALUES(?1, ?2, ?3, ?4) ON CONFLICT(id) DO UPDATE SET status=excluded.status, updated_at=excluded.updated_at, payload=excluded.payload WHERE excluded.updated_at >= solve_records.updated_at",
        rusqlite::params![record.id, record.status, record.updated_at, payload]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) fn delete_solve_record(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    conn.execute("DELETE FROM solve_records WHERE id=?1", [id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn companion_reuses_draft_without_erasing_work_and_creates_new_attempt_after_finish() {
        let mut conn = crate::db::open(Path::new(":memory:")).unwrap();
        let input = || CompanionDraftInput {
            title: "A. Example".into(),
            url: "https://codeforces.com/problemset/problem/1/A#statement".into(),
            group: "Codeforces - Round 1".into(),
            batch_id: "batch-1".into(),
        };
        let first = store_companion_draft(&mut conn, input()).unwrap();
        assert!(first.created);
        let payload: String = conn.query_row("SELECT payload FROM solve_records WHERE id=?1", [&first.id], |row| row.get(0)).unwrap();
        let mut record: SolveRecord = serde_json::from_str(&payload).unwrap();
        assert_eq!(record.platform, "codeforces");
        assert_eq!(record.source_group, "Codeforces - Round 1");
        record.note = "已有笔记".into();
        conn.execute("UPDATE solve_records SET payload=?1 WHERE id=?2", params![serde_json::to_string(&record).unwrap(), first.id]).unwrap();
        let second = store_companion_draft(&mut conn, input()).unwrap();
        assert!(!second.created);
        assert_eq!(second.id, record.id);
        let preserved: String = conn.query_row("SELECT payload FROM solve_records WHERE id=?1", [&record.id], |row| row.get(0)).unwrap();
        assert!(preserved.contains("已有笔记"));
        record.status = "finished".into();
        conn.execute("UPDATE solve_records SET status='finished', payload=?1 WHERE id=?2", params![serde_json::to_string(&record).unwrap(), record.id]).unwrap();
        let third = store_companion_draft(&mut conn, input()).unwrap();
        assert!(third.created);
        assert_ne!(third.id, record.id);
    }

    #[test]
    fn companion_rejects_non_http_urls() {
        let mut conn = crate::db::open(Path::new(":memory:")).unwrap();
        let input = CompanionDraftInput { title: "Unsafe".into(), url: "file:///tmp/example".into(), group: String::new(), batch_id: String::new() };
        assert!(store_companion_draft(&mut conn, input).is_err());
    }
}
