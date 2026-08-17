use reqwest::Client;
use scraper::{Html, Selector};
use regex::Regex;

use crate::models::{AccountConfig, RemoteData, Submission, SyncError};
use super::{browser_headers, get_text, polite_sleep, with_referer, now_epoch};

pub async fn fetch(client: &Client, account: &AccountConfig, full: bool, cursor: i64) -> Result<RemoteData, SyncError> {
    let uid = account.account.trim();
    if uid.is_empty() || !uid.chars().all(|c| c.is_ascii_digit()) { return Err(SyncError::error("牛客目前需要数字 User ID（个人主页 users/ 后面的数字）")); }
    let base = format!("https://ac.nowcoder.com/acm/contest/profile/{uid}/practice-coding");
    let mut out = Vec::new();
    let mut page = 1_i64;
    let mut max_seen = cursor;
    let cutoff = if full { 0 } else { cursor.saturating_sub(5) };
    loop {
        if page > 5000 { return Err(SyncError::error("牛客分页过多，已中止")); }
        let url = format!("{base}?languageCategoryFilter=-1&orderType=DESC&page={page}&pageSize=200&search=&statusTypeFilter=5");
        let html = get_text(client, &url, with_referer(browser_headers(), "https://ac.nowcoder.com/")).await?;
        let text = Html::parse_document(&html).root_element().text().collect::<String>();
        if text.contains("登录") && !text.contains("提交时间") { return Err(SyncError::auth("牛客页面需要登录或当前账号不可公开访问")); }
        let rows = parse_rows(&html, uid);
        if rows.is_empty() { break; }
        let mut old = false;
        for s in rows {
            max_seen = max_seen.max(s.epoch_second);
            if !full && s.epoch_second <= cutoff { old = true; continue; }
            out.push(s);
        }
        if old || !has_next_page(&html, page) { break; }
        page += 1;
        polite_sleep(260).await;
    }
    Ok(RemoteData {
        platform: "nowcoder".into(), account: uid.into(), submissions: out, aggregates: vec![], solved_count: None, difficulty: vec![], activity_only: false,
        notes: vec!["牛客竞赛站公开练习提交页 · statusTypeFilter=5".into()], cursor_epoch: max_seen.max(now_epoch().saturating_sub(2)), replace_submissions: full, replace_aggregates: full,
    })
}

fn parse_rows(html: &str, uid: &str) -> Vec<Submission> {
    let doc = Html::parse_document(html);
    let tr = Selector::parse("tr").unwrap();
    let td = Selector::parse("td").unwrap();
    let a_sel = Selector::parse("a").unwrap();
    let re_problem = Regex::new(r"/acm/problem/(\d+)").unwrap();
    let re_contest = Regex::new(r"/acm/contest/(\d+)/([^/?#]+)").unwrap();
    let mut out = Vec::new();
    for row in doc.select(&tr) {
        let cells: Vec<_> = row.select(&td).collect();
        if cells.len() < 9 { continue; }
        let verdict = cells[2].text().collect::<String>();
        if !(verdict.contains("答案正确") || verdict.to_ascii_lowercase().contains("accepted") || verdict.trim() == "AC") { continue; }
        let link = match cells[1].select(&a_sel).next().and_then(|a| a.value().attr("href")) { Some(x) => x.to_string(), None => continue };
        let problem_name = cells[1].text().collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ");
        let language = cells[7].text().collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ");
        let time_text = cells[8].text().collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ");
        let ts = parse_china_time(&time_text); if ts <= 0 { continue; }
        let absolute = if link.starts_with("http") { link.clone() } else { format!("https://ac.nowcoder.com{link}") };
        let pid = re_problem.captures(&absolute).map(|c| c[1].to_string()).or_else(|| re_contest.captures(&absolute).map(|c| format!("{}/{}", &c[1], &c[2]))).unwrap_or_else(|| absolute.clone());
        let id = cells[0].text().collect::<String>().chars().filter(|c| c.is_ascii_digit()).collect::<String>();
        out.push(Submission { platform: "nowcoder".into(), submission_id: if id.is_empty() { format!("{uid}-{ts}-{pid}") } else { id }, problem_key: pid.clone(), problem_id: pid, problem_name, problem_url: absolute, epoch_second: ts, language, difficulty: None });
    }
    out
}

fn has_next_page(html: &str, current: i64) -> bool {
    let needle = format!("page={}", current + 1);
    html.contains(&needle)
}

fn parse_china_time(s: &str) -> i64 {
    let re = Regex::new(r"(\d{4})-(\d{2})-(\d{2})\s+(\d{2}):(\d{2}):(\d{2})").unwrap();
    let Some(c) = re.captures(s) else { return 0; };
    let text = format!("{}-{}-{}T{}:{}:{}+08:00", &c[1], &c[2], &c[3], &c[4], &c[5], &c[6]);
    chrono::DateTime::parse_from_rfc3339(&text).map(|x| x.timestamp()).unwrap_or(0)
}
