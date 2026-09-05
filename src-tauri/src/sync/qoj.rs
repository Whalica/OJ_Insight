use regex::Regex;
use reqwest::Client;
use scraper::{ElementRef, Html, Selector};

use super::{browser_headers, get_text, now_epoch, polite_sleep, with_cookie};
use crate::models::{AccountConfig, RemoteData, Submission, SyncError};

pub async fn fetch(
    client: &Client,
    account: &AccountConfig,
    full: bool,
    cursor: i64,
) -> Result<RemoteData, SyncError> {
    let user = account.account.trim();
    if user.is_empty() {
        return Err(SyncError::error("QOJ 用户名为空"));
    }
    if account.secret.trim().is_empty() {
        return Err(SyncError::auth(
            "QOJ 当前要求登录后才能查看完整提交列表；请在设置中填写 UOJSESSID Cookie",
        ));
    }
    let mut out = Vec::new();
    let cutoff = if full {
        0
    } else {
        cursor.saturating_sub(48 * 3600)
    };
    let mut max_seen = cursor;
    for page in 1..=5000 {
        let url = format!(
            "https://qoj.ac/submissions?submitter={}&min_score=100&max_score=100&page={page}",
            urlencoding::encode(user)
        );
        let html = get_text(
            client,
            &url,
            with_cookie(browser_headers(), &account.secret),
        )
        .await?;
        if looks_like_login(&html) {
            return Err(SyncError::auth(
                "QOJ 未登录或 UOJSESSID 已过期；可填写完整 UOJSESSID=value，也可只填写 value",
            ));
        }
        let rows = parse_rows(&html, user);
        if rows.is_empty() {
            if page == 1 {
                if is_valid_empty_page(&html) {
                    return Ok(RemoteData {
                        platform: "qoj".into(),
                        account: user.into(),
                        submissions: vec![],
                        aggregates: vec![],
                        solved_count: Some(0),
                        difficulty: vec![],
                        ratings: None,
                        activity_only: false,
                        notes: vec!["QOJ 已登录，当前筛选下没有 AC 提交".into()],
                        cursor_epoch: now_epoch(),
                        replace_submissions: full,
                        replace_aggregates: full,
                    });
                }
                return Err(SyncError::error(
                    "QOJ 页面已返回，但未识别到提交表结构；请查看 logs/oj-insight.log 后更新应用",
                ));
            }
            break;
        }
        let mut reached_old = false;
        for s in rows {
            max_seen = max_seen.max(s.epoch_second);
            if !full && s.epoch_second <= cutoff {
                reached_old = true;
                continue;
            }
            out.push(s);
        }
        if reached_old || !has_next_page(&html, page) {
            break;
        }
        polite_sleep(350).await;
    }
    Ok(RemoteData {
        platform: "qoj".into(),
        account: user.into(),
        submissions: out,
        aggregates: vec![],
        solved_count: None,
        difficulty: vec![],
        ratings: None,
        activity_only: false,
        notes: vec!["QOJ 完整提交列表当前需要登录；本地通过 UOJSESSID 读取".into()],
        cursor_epoch: max_seen.max(now_epoch().saturating_sub(48 * 3600)),
        replace_submissions: full,
        replace_aggregates: full,
    })
}

fn looks_like_login(html: &str) -> bool {
    let low = html.to_ascii_lowercase();
    ((low.contains("name=\"password\"") || low.contains("name='password'"))
        && (low.contains("login") || html.contains("登录")))
        || low.contains("must now be logged in to view submissions")
}

fn is_valid_empty_page(html: &str) -> bool {
    let low = html.to_ascii_lowercase();
    (low.contains("submission")
        && low.contains("problem")
        && (low.contains("submit time") || low.contains("提交时间")))
        && (low.contains("no submissions")
            || low.contains("no records")
            || low.contains("没有提交")
            || low.contains("暂无记录")
            || Html::parse_document(html)
                .select(&Selector::parse("table tbody tr").unwrap())
                .next()
                .is_none())
}

fn parse_rows(html: &str, user: &str) -> Vec<Submission> {
    let doc = Html::parse_document(html);
    let tr = Selector::parse("table tr").unwrap();
    let td = Selector::parse("td").unwrap();
    let a_sel = Selector::parse("a[href]").unwrap();
    let re_problem = Regex::new(r"/problem/(\d+)").unwrap();
    let re_sub = Regex::new(r"(?:/submission/|#)(\d+)").unwrap();
    let mut out = Vec::new();
    for row in doc.select(&tr) {
        let cells: Vec<_> = row.select(&td).collect();
        if cells.is_empty() {
            continue;
        }
        let texts: Vec<String> = cells
            .iter()
            .map(|c| {
                c.text()
                    .collect::<String>()
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect();
        let row_text = texts.join(" ");
        let row_low = row_text.to_ascii_lowercase();
        if !(row_text.contains("100")
            || row_low.contains("accepted")
            || row_low.split_whitespace().any(|x| x == "ac"))
        {
            continue;
        }
        let anchors: Vec<ElementRef<'_>> = row.select(&a_sel).collect();
        let problem_anchor = anchors.iter().find(|a| {
            a.value()
                .attr("href")
                .is_some_and(|h| re_problem.is_match(h))
        });
        let Some(problem_anchor) = problem_anchor else {
            continue;
        };
        let href = problem_anchor.value().attr("href").unwrap_or("");
        let Some(cap) = re_problem.captures(href) else {
            continue;
        };
        let pid = cap[1].to_string();
        let problem_name = problem_anchor
            .text()
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let ts = parse_qoj_time(&row_text);
        if ts <= 0 {
            continue;
        }
        let id = anchors
            .iter()
            .filter_map(|a| a.value().attr("href"))
            .find_map(|h| re_sub.captures(h).map(|c| c[1].to_string()))
            .unwrap_or_else(|| format!("{user}-{ts}-{pid}"));
        let language = texts
            .iter()
            .find(|x| {
                let l = x.to_ascii_lowercase();
                l.contains("c++")
                    || l.contains("python")
                    || l.contains("rust")
                    || l.contains("java")
            })
            .cloned()
            .unwrap_or_default();
        out.push(Submission {
            platform: "qoj".into(),
            account: user.into(),
            source: "oj".into(),
            source_day: None,
            submission_id: id,
            problem_key: pid.clone(),
            problem_id: format!("#{pid}"),
            problem_name,
            problem_url: format!("https://qoj.ac/problem/{pid}"),
            epoch_second: ts,
            language,
            difficulty: None,
        });
    }
    out
}

fn has_next_page(html: &str, current: usize) -> bool {
    html.contains(&format!("page={}", current + 1))
}

fn parse_qoj_time(s: &str) -> i64 {
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
