use reqwest::Client;
use scraper::{Html, Selector};
use regex::Regex;

use crate::models::{AccountConfig, RemoteData, Submission, SyncError};
use super::{browser_headers, get_text, polite_sleep, with_cookie, now_epoch};

pub async fn fetch(client: &Client, account: &AccountConfig, full: bool, cursor: i64) -> Result<RemoteData, SyncError> {
    let user = account.account.trim();
    if user.is_empty() { return Err(SyncError::error("QOJ 用户名为空")); }
    if account.secret.trim().is_empty() {
        return Err(SyncError::auth("QOJ 当前要求登录后才能查看完整提交列表；请在设置中填写 UOJSESSID Cookie"));
    }
    let mut out = Vec::new();
    let cutoff = if full { 0 } else { cursor.saturating_sub(5) };
    let mut max_seen = cursor;
    for page in 1..=5000 {
        let url = format!("https://qoj.ac/submissions?submitter={}&min_score=100&max_score=100&page={page}", urlencoding::encode(user));
        let html = get_text(client, &url, with_cookie(browser_headers(), &account.secret)).await?;
        if looks_like_login(&html) { return Err(SyncError::auth("QOJ 登录态无效或已过期，请更新 UOJSESSID Cookie")); }
        let rows = parse_rows(&html, user);
        if rows.is_empty() { if page == 1 { return Err(SyncError::error("QOJ 提交页没有解析到记录；站点表格结构可能已变化")); } break; }
        let mut reached_old = false;
        for s in rows {
            max_seen = max_seen.max(s.epoch_second);
            if !full && s.epoch_second <= cutoff { reached_old = true; continue; }
            out.push(s);
        }
        if reached_old || !has_next_page(&html, page) { break; }
        polite_sleep(350).await;
    }
    Ok(RemoteData {
        platform: "qoj".into(), account: user.into(), submissions: out, aggregates: vec![], solved_count: None, difficulty: vec![], activity_only: false,
        notes: vec!["QOJ 完整提交列表当前需要登录；本地通过 UOJSESSID 读取".into()], cursor_epoch: max_seen.max(now_epoch().saturating_sub(2)), replace_submissions: full, replace_aggregates: full,
    })
}

fn looks_like_login(html: &str) -> bool {
    let low = html.to_ascii_lowercase();
    (low.contains("name=\"password\"") || low.contains("name='password'")) && (low.contains("login") || html.contains("登录"))
}

fn parse_rows(html: &str, user: &str) -> Vec<Submission> {
    let doc = Html::parse_document(html);
    let tr = Selector::parse("tr").unwrap(); let td = Selector::parse("td").unwrap(); let a_sel = Selector::parse("a").unwrap();
    let re_problem = Regex::new(r"/problem/(\d+)").unwrap();
    let re_sub = Regex::new(r"(?:/submission/|#)(\d+)").unwrap();
    let mut out = Vec::new();
    for row in doc.select(&tr) {
        let cells: Vec<_> = row.select(&td).collect();
        if cells.len() < 8 { continue; }
        let texts: Vec<String> = cells.iter().map(|c| c.text().collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ")).collect();
        if texts.iter().any(|x| x == user) == false && cells.len() >= 3 { continue; }
        let verdict_idx = if cells.len() >= 9 { 3 } else { 2 };
        let verdict = texts.get(verdict_idx).cloned().unwrap_or_default();
        if !(verdict.contains("100") || verdict.eq_ignore_ascii_case("AC") || verdict.to_ascii_lowercase().contains("accepted")) { continue; }
        let problem_cell = cells.get(1); let Some(problem_cell) = problem_cell else { continue; };
        let href = problem_cell.select(&a_sel).next().and_then(|a| a.value().attr("href")).unwrap_or("");
        let Some(cap) = re_problem.captures(href) else { continue; };
        let pid = cap[1].to_string();
        let problem_name = texts.get(1).cloned().unwrap_or_else(|| format!("#{pid}"));
        let time_text = texts.last().cloned().unwrap_or_default();
        let ts = parse_qoj_time(&time_text); if ts <= 0 { continue; }
        let language = if cells.len() >= 9 { texts.get(6).cloned().unwrap_or_default() } else { String::new() };
        let id_text = texts.first().cloned().unwrap_or_default();
        let id = re_sub.captures(&id_text).map(|c| c[1].to_string()).unwrap_or_else(|| format!("{user}-{ts}-{pid}"));
        out.push(Submission { platform: "qoj".into(), submission_id: id, problem_key: pid.clone(), problem_id: format!("#{pid}"), problem_name, problem_url: format!("https://qoj.ac/problem/{pid}"), epoch_second: ts, language, difficulty: None });
    }
    out
}

fn has_next_page(html: &str, current: usize) -> bool { html.contains(&format!("page={}", current + 1)) }

fn parse_qoj_time(s: &str) -> i64 {
    let re = Regex::new(r"(\d{4})-(\d{2})-(\d{2})\s+(\d{2}):(\d{2}):(\d{2})").unwrap();
    let Some(c) = re.captures(s) else { return 0; };
    let text = format!("{}-{}-{}T{}:{}:{}+08:00", &c[1], &c[2], &c[3], &c[4], &c[5], &c[6]);
    chrono::DateTime::parse_from_rfc3339(&text).map(|x| x.timestamp()).unwrap_or(0)
}
