use std::{collections::HashMap, path::Path};

use chrono::{DateTime, Utc};
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue, COOKIE, REFERER};
use scraper::{Html, Selector};
use serde::Serialize;
use serde_json::Value;
use tauri::State;

use crate::app::state::AppState;
use crate::db;
use crate::sync::browser_headers;

#[path = "contest_review_codeforces.rs"]
mod codeforces;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewPreview {
    platform: String,
    contest_id: String,
    contest_name: String,
    contest_url: String,
    account: String,
    start_epoch: Option<i64>,
    duration_seconds: Option<i64>,
    problem_count: usize,
    submission_count: usize,
    code_available: bool,
    completeness: String,
    notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewExportResult {
    path: String,
    contest_name: String,
    problem_count: usize,
    submission_count: usize,
    code_count: usize,
    completeness: String,
    notes: Vec<String>,
}

#[derive(Debug, Clone)]
struct ReviewContest {
    platform: String,
    id: String,
    name: String,
    url: String,
    account: String,
    start_epoch: Option<i64>,
    duration_seconds: Option<i64>,
    rank: Option<i64>,
    score: Option<f64>,
    penalty: Option<i64>,
    problems: Vec<ReviewProblem>,
    submissions: Vec<ReviewSubmission>,
    notes: Vec<String>,
}

#[derive(Debug, Clone)]
struct ReviewProblem {
    id: String,
    name: String,
    url: String,
    statement: String,
    time_limit: String,
    memory_limit: String,
}

#[derive(Debug, Clone)]
struct ReviewSubmission {
    id: String,
    problem_id: String,
    epoch_second: i64,
    relative_seconds: Option<i64>,
    language: String,
    verdict: String,
    time_ms: Option<i64>,
    memory_bytes: Option<i64>,
    score: Option<f64>,
    url: String,
    source: Option<String>,
    post_contest: bool,
}

fn account_secret(state: &AppState, platform: &str, account: &str) -> Result<String, String> {
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::get_accounts(&conn)?
        .into_iter()
        .find(|entry| entry.platform == platform && entry.account == account)
        .map(|entry| entry.secret)
        .ok_or_else(|| "找不到所选账号，请先在设置中保存账号".to_string())
}

fn normalize_contest_id(platform: &str, input: &str) -> Result<String, String> {
    let value = input.trim();
    if value.is_empty() {
        return Err("请输入比赛 ID 或链接".into());
    }
    let (path_pattern, id_pattern) = match platform {
        "codeforces" => (
            r"^https?://(?:www\.)?codeforces\.com/(?:contest|problemset/problem)/([1-9][0-9]*)(?:[/#?]|$)",
            r"^[1-9][0-9]*$",
        ),
        "atcoder" => (
            r"/contests/([A-Za-z0-9_-]+)(?:[/#?]|$)",
            r"^[A-Za-z0-9_-]+$",
        ),
        _ => return Err("当前版本比赛复盘支持 AtCoder 和 Codeforces".into()),
    };
    if let Some(id) = Regex::new(path_pattern)
        .map_err(|e| e.to_string())?
        .captures(value)
        .and_then(|captures| captures.get(1))
    {
        return Ok(id.as_str().to_string());
    }
    if Regex::new(id_pattern)
        .map_err(|e| e.to_string())?
        .is_match(value)
    {
        return Ok(value.to_string());
    }
    Err("无法识别比赛 ID，请输入 ID 或完整比赛链接".to_string())
}

fn cookie_headers(secret: &str, referer: &str) -> HeaderMap {
    let mut headers = browser_headers();
    let secret = secret.trim();
    let secret = secret.strip_prefix("Cookie:").unwrap_or(secret).trim();
    if !secret.is_empty() {
        if let Ok(mut value) = HeaderValue::from_str(secret) {
            value.set_sensitive(true);
            headers.insert(COOKIE, value);
        }
    }
    if let Ok(value) = HeaderValue::from_str(referer) {
        headers.insert(REFERER, value);
    }
    headers
}

async fn get_text(client: &reqwest::Client, url: &str, secret: &str) -> Result<String, String> {
    let response = client
        .get(url)
        .headers(cookie_headers(secret, url))
        .send()
        .await
        .map_err(|e| format!("请求失败：{e}"))?;
    if response.url().host_str() == Some("atcoder.jp") && response.url().path() == "/login" {
        return Err("AtCoder 要求登录，请在「前往配置」中填写或更新所选账号的 Cookie。".into());
    }
    if !response.status().is_success() {
        return Err(format!("请求 {url} 失败：HTTP {}", response.status()));
    }
    response
        .text()
        .await
        .map_err(|e| format!("读取页面失败：{e}"))
}

async fn get_source(
    client: &reqwest::Client,
    url: &str,
    secret: &str,
) -> Option<String> {
    if let Ok(html) = get_text(client, url, secret).await {
        if let Some(source) = extract_source(&html) {
            return Some(source);
        }
    }
    None
}

async fn load_contest(
    state: &AppState,
    platform: &str,
    account: &str,
    input: &str,
    include_post_contest: bool,
    with_source: bool,
) -> Result<ReviewContest, String> {
    let contest_id = normalize_contest_id(platform, input)?;
    let secret = account_secret(state, platform, account)?;
    match platform {
        "atcoder" => {
            load_atcoder(
                &state.client,
                account,
                &contest_id,
                &secret,
                include_post_contest,
                with_source,
            )
            .await
        }
        "codeforces" => {
            let credentials = serde_json::from_str(&secret).ok();
            codeforces::load(&state.client, account, &contest_id, include_post_contest, credentials).await
        }
        _ => unreachable!(),
    }
}

#[tauri::command]
pub(crate) async fn inspect_contest_review(
    state: State<'_, AppState>,
    platform: String,
    account: String,
    contest_input: String,
    include_post_contest: Option<bool>,
) -> Result<ReviewPreview, String> {
    let mut contest = load_contest(
        &state,
        &platform,
        &account,
        &contest_input,
        include_post_contest.unwrap_or(false),
        false,
    )
    .await?;
    let mut code_available = contest.submissions.iter().any(|item| item.source.is_some());
    if platform == "atcoder" && !code_available && !contest.submissions.is_empty() {
        let secret = account_secret(&state, &platform, &account)?;
        for submission in contest.submissions.iter().take(3) {
            if get_source(&state.client, &submission.url, &secret).await.is_some() {
                code_available = true;
                break;
            }
        }
    }
    if platform == "atcoder" && !contest.submissions.is_empty() && !code_available {
        contest
            .notes
            .push("抽查未读到提交源码。可在「前往配置」中检查 AtCoder Cookie，生成时会逐条尝试。".into());
    }
    Ok(ReviewPreview {
        platform: contest.platform,
        contest_id: contest.id,
        contest_name: contest.name,
        contest_url: contest.url,
        account: contest.account,
        start_epoch: contest.start_epoch,
        duration_seconds: contest.duration_seconds,
        problem_count: contest.problems.len(),
        submission_count: contest.submissions.len(),
        code_available,
        completeness: if contest.notes.is_empty() {
            "complete".into()
        } else {
            "partial".into()
        },
        notes: contest.notes,
    })
}

#[tauri::command]
pub(crate) async fn generate_contest_review(
    state: State<'_, AppState>,
    platform: String,
    account: String,
    contest_input: String,
    include_post_contest: Option<bool>,
    path: String,
) -> Result<ReviewExportResult, String> {
    let include_post_contest = include_post_contest.unwrap_or(false);
    let contest = load_contest(
        &state,
        &platform,
        &account,
        &contest_input,
        include_post_contest,
        true,
    )
    .await?;
    let code_count = contest
        .submissions
        .iter()
        .filter(|item| item.source.is_some())
        .count();
    let mut notes = contest.notes.clone();
    if code_count < contest.submissions.len() {
        notes.push(format!(
            "有 {} 次提交未能取得源代码",
            contest.submissions.len() - code_count
        ));
    }
    let completeness = if notes.is_empty() {
        "complete"
    } else {
        "partial"
    }
    .to_string();
    let documents = render_documents(&contest, include_post_contest, &completeness, &notes);
    write_zip(Path::new(&path), &documents)?;
    Ok(ReviewExportResult {
        path,
        contest_name: contest.name,
        problem_count: contest.problems.len(),
        submission_count: contest.submissions.len(),
        code_count,
        completeness,
        notes,
    })
}

fn extract_source(html: &str) -> Option<String> {
    let document = Html::parse_document(html);
    for pattern in [
        "#program-source-text",
        "#submission-code",
        "pre.prettyprint",
        "pre.code",
        "textarea[name='code']",
    ] {
        if let Ok(selector) = Selector::parse(pattern) {
            if let Some(element) = document.select(&selector).next() {
                let value = element.text().collect::<String>();
                if !value.trim().is_empty() {
                    return Some(value);
                }
            }
        }
    }
    None
}

fn element_text(element: scraper::ElementRef<'_>) -> String {
    element
        .text()
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

async fn load_atcoder(
    client: &reqwest::Client,
    account: &str,
    contest_id: &str,
    secret: &str,
    include_post_contest: bool,
    with_source: bool,
) -> Result<ReviewContest, String> {
    let base = format!("https://atcoder.jp/contests/{contest_id}");
    let contest_html = get_text(client, &base, secret).await?;
    let (name, contest_times) = {
        let document = Html::parse_document(&contest_html);
        let title_selector = Selector::parse("title").unwrap();
        let name = document
            .select(&title_selector)
            .next()
            .map(element_text)
            .unwrap_or_else(|| contest_id.to_string())
            .replace(" - AtCoder", "");
        (name, parse_atcoder_times(&document))
    };
    let start_epoch = contest_times.first().copied();
    let duration_seconds = contest_times
        .get(1)
        .zip(start_epoch)
        .map(|(end, start)| end - start)
        .filter(|value| *value > 0);
    let tasks_url = format!("{base}/tasks");
    let tasks_html = get_text(client, &tasks_url, secret).await?;
    let task_stubs: Vec<(String, String, String)> = {
        let tasks_doc = Html::parse_document(&tasks_html);
        let row_selector = Selector::parse("table tbody tr").unwrap();
        let link_selector = Selector::parse("a[href*='/tasks/']").unwrap();
        tasks_doc
            .select(&row_selector)
            .filter_map(|row| {
                let links: Vec<_> = row.select(&link_selector).collect();
                let first = links.first()?;
                let id = first.text().collect::<String>().trim().to_string();
                let name = links
                    .last()
                    .map(|item| item.text().collect::<String>().trim().to_string())
                    .unwrap_or_else(|| id.clone());
                let href = first.value().attr("href").unwrap_or("");
                let url = if href.starts_with("http") {
                    href.to_string()
                } else {
                    format!("https://atcoder.jp{href}")
                };
                Some((id, name, url))
            })
            .collect()
    };
    let mut problems = Vec::new();
    for (id, name, url) in task_stubs {
        let statement = get_text(client, &url, secret)
            .await
            .ok()
            .map(|html| extract_atcoder_statement(&html))
            .unwrap_or_default();
        problems.push(ReviewProblem {
            id,
            name,
            url,
            statement,
            time_limit: String::new(),
            memory_limit: String::new(),
        });
    }
    let problem_labels = problems
        .iter()
        .filter_map(|problem| {
            problem
                .url
                .rsplit('/')
                .next()
                .map(|slug| (slug.to_string(), problem.id.clone()))
        })
        .collect::<HashMap<_, _>>();
    let mut submissions = Vec::new();
    let mut notes = Vec::new();
    let mut page = 1;
    loop {
        let url = format!(
            "{base}/submissions?f.User={}&page={page}",
            urlencoding::encode(account)
        );
        let html = match get_text(client, &url, secret).await {
            Ok(html) => html,
            Err(error) => {
                notes.push(format!("未能完整读取 AtCoder 提交列表。{error}"));
                break;
            }
        };
        let parsed = parse_atcoder_submissions(&html, start_epoch, duration_seconds);
        if parsed.is_empty() {
            break;
        }
        let count = parsed.len();
        submissions.extend(parsed);
        if count < 20 || page >= 100 {
            break;
        }
        page += 1;
    }
    if submissions.is_empty() {
        if let Ok(fallback) = fetch_atcoder_api_submissions(
            client,
            account,
            contest_id,
            start_epoch,
            duration_seconds,
            &problem_labels,
        )
        .await
        {
            if !fallback.is_empty() {
                submissions = fallback;
            }
        }
    }
    if !include_post_contest {
        submissions.retain(|item| !item.post_contest);
    }
    if with_source {
        for submission in &mut submissions {
            submission.source = get_source(client, &submission.url, secret).await;
        }
    }
    submissions.sort_by_key(|item| item.epoch_second);
    if problems.iter().any(|item| item.statement.is_empty()) {
        notes.push("部分 AtCoder 题面未能获取，请使用题目链接补充查看".into());
    }
    if submissions.is_empty() && notes.is_empty() {
        notes.push("未找到该账号在本场比赛的提交；请检查 AtCoder 用户名大小写和比赛 ID".into());
    }
    Ok(ReviewContest {
        platform: "atcoder".into(),
        id: contest_id.into(),
        name,
        url: base,
        account: account.into(),
        start_epoch,
        duration_seconds,
        rank: None,
        score: None,
        penalty: None,
        problems,
        submissions,
        notes,
    })
}

async fn fetch_atcoder_api_submissions(
    client: &reqwest::Client,
    account: &str,
    contest_id: &str,
    start_epoch: Option<i64>,
    duration_seconds: Option<i64>,
    problem_labels: &HashMap<String, String>,
) -> Result<Vec<ReviewSubmission>, String> {
    let from_second = start_epoch.unwrap_or(0).saturating_sub(1);
    let url = format!(
        "https://kenkoooo.com/atcoder/atcoder-api/v3/user/submissions?user={}&from_second={from_second}",
        urlencoding::encode(account)
    );
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("读取 AtCoder 公开提交数据失败：{e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "读取 AtCoder 公开提交数据失败：HTTP {}",
            response.status()
        ));
    }
    let rows = response
        .json::<Vec<Value>>()
        .await
        .map_err(|e| format!("解析 AtCoder 公开提交数据失败：{e}"))?;
    let end_epoch = start_epoch
        .zip(duration_seconds)
        .map(|(start, duration)| start + duration);
    let mut result = rows
        .into_iter()
        .filter(|item| item.get("contest_id").and_then(Value::as_str) == Some(contest_id))
        .filter_map(|item| {
            let id = item.get("id")?.as_i64()?.to_string();
            let epoch_second = item.get("epoch_second")?.as_i64()?;
            let task_slug = item
                .get("problem_id")
                .and_then(Value::as_str)
                .unwrap_or("?");
            Some(ReviewSubmission {
                id: id.clone(),
                problem_id: problem_labels
                    .get(task_slug)
                    .cloned()
                    .unwrap_or_else(|| task_slug.to_string()),
                epoch_second,
                relative_seconds: start_epoch.map(|start| epoch_second - start),
                language: item
                    .get("language")
                    .and_then(Value::as_str)
                    .unwrap_or("未知")
                    .to_string(),
                verdict: item
                    .get("result")
                    .and_then(Value::as_str)
                    .unwrap_or("UNKNOWN")
                    .to_string(),
                time_ms: item.get("execution_time").and_then(Value::as_i64),
                memory_bytes: None,
                score: item.get("point").and_then(Value::as_f64),
                url: format!("https://atcoder.jp/contests/{contest_id}/submissions/{id}"),
                source: None,
                post_contest: end_epoch.is_some_and(|end| epoch_second > end),
            })
        })
        .collect::<Vec<_>>();
    result.sort_by_key(|item| item.epoch_second);
    Ok(result)
}

fn parse_atcoder_times(document: &Html) -> Vec<i64> {
    let Ok(selector) = Selector::parse("time.fixtime-full, time.fixtime, time.fixtime-second")
    else {
        return Vec::new();
    };
    document
        .select(&selector)
        .filter_map(|element| {
            element
                .value()
                .attr("data-time")
                .and_then(parse_atcoder_time)
                .or_else(|| parse_atcoder_time(&element.text().collect::<String>()))
        })
        .collect()
}

fn parse_atcoder_time(value: &str) -> Option<i64> {
    let value = value.trim();
    DateTime::parse_from_rfc3339(value)
        .ok()
        .or_else(|| DateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S%z").ok())
        .or_else(|| DateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S %z").ok())
        .map(|time| time.timestamp())
}

fn extract_atcoder_statement(html: &str) -> String {
    let document = Html::parse_document(html);
    let selector = Selector::parse("#task-statement").unwrap();
    document
        .select(&selector)
        .next()
        .map(element_text)
        .unwrap_or_default()
}

fn parse_atcoder_submissions(
    html: &str,
    start_epoch: Option<i64>,
    duration_seconds: Option<i64>,
) -> Vec<ReviewSubmission> {
    let document = Html::parse_document(html);
    let row_selector = Selector::parse("table tbody tr").unwrap();
    let cell_selector = Selector::parse("td").unwrap();
    let link_selector = Selector::parse("a").unwrap();
    let time_selector = Selector::parse("time").unwrap();
    let end_epoch = start_epoch
        .zip(duration_seconds)
        .map(|(start, duration)| start + duration);
    document
        .select(&row_selector)
        .filter_map(|row| {
            let cells: Vec<_> = row.select(&cell_selector).collect();
            if cells.len() < 7 {
                return None;
            }
            let link = row.select(&link_selector).find(|item| {
                item.value()
                    .attr("href")
                    .is_some_and(|href| href.contains("/submissions/"))
            })?;
            let href = link.value().attr("href")?;
            let id = href.rsplit('/').next()?.to_string();
            let epoch_second = row
                .select(&time_selector)
                .next()
                .and_then(|item| {
                    item.value()
                        .attr("data-time")
                        .and_then(parse_atcoder_time)
                        .or_else(|| parse_atcoder_time(&item.text().collect::<String>()))
                })
                .unwrap_or(0);
            let post_contest = end_epoch.is_some_and(|end| epoch_second > end);
            let text = |index: usize| {
                cells
                    .get(index)
                    .map(|cell| element_text(*cell))
                    .unwrap_or_default()
            };
            let problem_id = cells
                .get(1)
                .and_then(|cell| cell.select(&link_selector).next())
                .map(element_text)
                .unwrap_or_else(|| "?".into());
            let verdict = text(6);
            let time_value = text(7);
            let memory_value = text(8);
            let time_ms = Regex::new(r"(\d+)\s*ms")
                .ok()
                .and_then(|re| re.captures(&time_value))
                .and_then(|caps| caps.get(1))
                .and_then(|item| item.as_str().parse().ok());
            let memory_bytes = Regex::new(r"(\d+)\s*KB")
                .ok()
                .and_then(|re| re.captures(&memory_value))
                .and_then(|caps| caps.get(1))
                .and_then(|item| item.as_str().parse::<i64>().ok())
                .map(|value| value * 1024);
            Some(ReviewSubmission {
                id,
                problem_id,
                epoch_second,
                relative_seconds: start_epoch.map(|start| epoch_second - start),
                language: text(3),
                verdict,
                time_ms,
                memory_bytes,
                score: text(4).parse().ok(),
                url: format!("https://atcoder.jp{href}"),
                source: None,
                post_contest,
            })
        })
        .collect()
}

fn render_documents(
    contest: &ReviewContest,
    include_post_contest: bool,
    completeness: &str,
    notes: &[String],
) -> Vec<(&'static str, String)> {
    let generated = Utc::now().to_rfc3339();
    let status = if completeness == "complete" {
        "完整"
    } else {
        "部分完整"
    };
    let missing = if notes.is_empty() {
        "无已知缺失。".into()
    } else {
        notes
            .iter()
            .map(|note| format!("- {note}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let start = contest
        .start_epoch
        .map(format_epoch)
        .unwrap_or_else(|| "未知".into());
    let start_doc = format!(
        r#"# 请从这里开始

你收到的是 OJ Insight 生成的比赛复盘包。请先读完本压缩包中的四份 Markdown 文档，然后直接开始复盘，不要等待用户补充提示词。若用户另外说明关注点，以用户说明为准。

## 复盘要求

1. 总结整场表现、关键转折、选题顺序和时间分配。
2. 逐题还原提交过程，对比失败代码与通过代码的变化。
3. 区分思路、复杂度、边界、实现、调试和比赛策略问题。
4. 分析未完成题目，但数据不足时必须明确说明，不得凭空推断。
5. 给出最重要的能力短板和低压力、可执行的后续训练建议。
6. 所有结论尽量引用题号、提交 ID、相对比赛时间和代码作为依据。

## 数据摘要

- OJ：{}
- 比赛：{}（{}）
- 账号：{}
- 开始时间：{}
- 题目数：{}
- 提交数：{}
- 包含赛后补题：{}
- 完整性：{}

## 完整性说明

{}

生成时间：{}
"#,
        contest.platform,
        contest.name,
        contest.id,
        contest.account,
        start,
        contest.problems.len(),
        contest.submissions.len(),
        if include_post_contest { "是" } else { "否" },
        status,
        missing,
        generated
    );

    let mut contest_doc = format!("# 比赛信息\n\n- 名称：{}\n- OJ：{}\n- 比赛 ID：{}\n- 链接：{}\n- 账号：{}\n- 开始时间：{}\n- 时长：{}\n- 排名：{}\n- 得分：{}\n- 罚时：{}\n\n## 提交时间线\n\n", contest.name, contest.platform, contest.id, contest.url, contest.account, start, contest.duration_seconds.map(format_duration).unwrap_or_else(|| "未知".into()), optional_number(contest.rank), optional_float(contest.score), optional_number(contest.penalty));
    if contest.submissions.is_empty() {
        contest_doc.push_str("未找到该账号的比赛提交。\n");
    }
    for item in &contest.submissions {
        if item.post_contest {
            contest_doc.push_str(&format!(
                "- {} · {} · {} · 赛后补题\n",
                format_epoch(item.epoch_second),
                item.problem_id,
                item.verdict
            ));
        } else {
            contest_doc.push_str(&format!(
                "- {} · {} · {} · {}\n",
                format_epoch(item.epoch_second),
                format_relative(item.relative_seconds),
                item.problem_id,
                item.verdict
            ));
        }
    }

    let mut problems_doc = String::from("# 题目信息\n");
    for problem in &contest.problems {
        problems_doc.push_str(&format!(
            "\n## {}. {}\n\n- 链接：{}\n- 时间限制：{}\n- 空间限制：{}\n\n",
            problem.id,
            problem.name,
            problem.url,
            empty_as_unknown(&problem.time_limit),
            empty_as_unknown(&problem.memory_limit)
        ));
        if problem.statement.is_empty() {
            problems_doc.push_str("> 题面未能获取，请通过原始链接查看。\n");
        } else {
            problems_doc.push_str(&problem.statement);
            problems_doc.push('\n');
        }
    }

    let mut submissions_doc = String::from("# 全部提交与代码\n");
    for problem in &contest.problems {
        let items: Vec<_> = contest
            .submissions
            .iter()
            .filter(|item| item.problem_id == problem.id)
            .collect();
        submissions_doc.push_str(&format!("\n## {}. {}\n", problem.id, problem.name));
        if items.is_empty() {
            submissions_doc.push_str("\n本题没有提交。\n");
            continue;
        }
        for item in items {
            let time = if item.post_contest {
                format_epoch(item.epoch_second)
            } else {
                format!("{}（{}）", format_epoch(item.epoch_second), format_relative(item.relative_seconds))
            };
            submissions_doc.push_str(&format!("\n### 提交 {} · {}\n\n- 时间：{}\n- 阶段：{}\n- 结果：{}\n- 语言：{}\n- 用时：{}\n- 内存：{}\n- 得分：{}\n- 链接：{}\n\n", item.id, item.verdict, time, if item.post_contest { "赛后补题" } else { "正式比赛" }, item.verdict, item.language, item.time_ms.map(|value| format!("{value} ms")).unwrap_or_else(|| "未知".into()), item.memory_bytes.map(|value| format!("{} KB", value / 1024)).unwrap_or_else(|| "未知".into()), optional_float(item.score), item.url));
            if let Some(source) = &item.source {
                submissions_doc.push_str(&code_block(source, &item.language));
            } else {
                submissions_doc.push_str("> 本次提交的源代码未能获取。\n");
            }
        }
    }
    for item in contest.submissions.iter().filter(|item| {
        !contest
            .problems
            .iter()
            .any(|problem| problem.id == item.problem_id)
    }) {
        submissions_doc.push_str(&format!(
            "\n## 未识别题目 {} · 提交 {}\n\n结果：{}\n\n{}",
            item.problem_id,
            item.id,
            item.verdict,
            item.source
                .as_ref()
                .map(|source| code_block(source, &item.language))
                .unwrap_or_else(|| "> 源代码未能获取。\n".into())
        ));
    }
    vec![
        ("00-START-HERE.md", start_doc),
        ("01-CONTEST.md", contest_doc),
        ("02-PROBLEMS.md", problems_doc),
        ("03-SUBMISSIONS.md", submissions_doc),
    ]
}

fn write_zip(path: &Path, documents: &[(&str, String)]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建导出目录失败：{e}"))?;
    }
    // A tiny standards-compliant ZIP writer is enough here: the package always
    // contains four UTF-8 Markdown files, so storing them without compression
    // avoids another native dependency and keeps export deterministic.
    let mut archive = Vec::new();
    let mut central = Vec::new();
    for (name, content) in documents {
        let name = name.as_bytes();
        let data = content.as_bytes();
        let name_len = u16::try_from(name.len()).map_err(|_| "复盘包文件名过长".to_string())?;
        let data_len =
            u32::try_from(data.len()).map_err(|_| "单份复盘文档超过 ZIP 格式限制".to_string())?;
        let offset =
            u32::try_from(archive.len()).map_err(|_| "复盘包超过 ZIP 格式限制".to_string())?;
        let crc = crc32(data);
        push_u32(&mut archive, 0x0403_4b50);
        push_u16(&mut archive, 20);
        push_u16(&mut archive, 0x0800); // UTF-8 file names.
        push_u16(&mut archive, 0); // Stored, no compression.
        push_u16(&mut archive, 0);
        push_u16(&mut archive, 0);
        push_u32(&mut archive, crc);
        push_u32(&mut archive, data_len);
        push_u32(&mut archive, data_len);
        push_u16(&mut archive, name_len);
        push_u16(&mut archive, 0);
        archive.extend_from_slice(name);
        archive.extend_from_slice(data);

        push_u32(&mut central, 0x0201_4b50);
        push_u16(&mut central, 20);
        push_u16(&mut central, 20);
        push_u16(&mut central, 0x0800);
        push_u16(&mut central, 0);
        push_u16(&mut central, 0);
        push_u16(&mut central, 0);
        push_u32(&mut central, crc);
        push_u32(&mut central, data_len);
        push_u32(&mut central, data_len);
        push_u16(&mut central, name_len);
        push_u16(&mut central, 0);
        push_u16(&mut central, 0);
        push_u16(&mut central, 0);
        push_u16(&mut central, 0);
        push_u32(&mut central, 0);
        push_u32(&mut central, offset);
        central.extend_from_slice(name);
    }
    let central_offset =
        u32::try_from(archive.len()).map_err(|_| "复盘包超过 ZIP 格式限制".to_string())?;
    let central_len =
        u32::try_from(central.len()).map_err(|_| "复盘包索引超过 ZIP 格式限制".to_string())?;
    let entries = u16::try_from(documents.len()).map_err(|_| "复盘包文件过多".to_string())?;
    archive.extend_from_slice(&central);
    push_u32(&mut archive, 0x0605_4b50);
    push_u16(&mut archive, 0);
    push_u16(&mut archive, 0);
    push_u16(&mut archive, entries);
    push_u16(&mut archive, entries);
    push_u32(&mut archive, central_len);
    push_u32(&mut archive, central_offset);
    push_u16(&mut archive, 0);
    std::fs::write(path, archive).map_err(|e| format!("写入复盘包失败：{e}"))
}

fn push_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_le_bytes());
}
fn push_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn format_epoch(value: i64) -> String {
    DateTime::from_timestamp(value, 0)
        .map(|time| time.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| value.to_string())
}
fn format_duration(value: i64) -> String {
    format!("{} 小时 {} 分钟", value / 3600, value % 3600 / 60)
}
fn format_relative(value: Option<i64>) -> String {
    value
        .map(|seconds| {
            if seconds < 0 {
                format!("赛前 {}", format_duration(-seconds))
            } else {
                format!("比赛开始后 {}", format_duration(seconds))
            }
        })
        .unwrap_or_else(|| "相对时间未知".into())
}
fn optional_number(value: Option<i64>) -> String {
    value
        .map(|item| item.to_string())
        .unwrap_or_else(|| "未知".into())
}
fn optional_float(value: Option<f64>) -> String {
    value
        .map(|item| {
            if item.fract() == 0.0 {
                format!("{item:.0}")
            } else {
                item.to_string()
            }
        })
        .unwrap_or_else(|| "未知".into())
}
fn empty_as_unknown(value: &str) -> &str {
    if value.trim().is_empty() {
        "未知"
    } else {
        value
    }
}
fn code_block(source: &str, language: &str) -> String {
    let longest = source
        .split(|character| character != '`')
        .map(str::len)
        .max()
        .unwrap_or(0);
    let fence = "`".repeat(longest.max(3) + 1);
    format!(
        "{fence}{}\n{}\n{fence}\n",
        language_hint(language),
        source.trim_end()
    )
}
fn language_hint(language: &str) -> &'static str {
    let value = language.to_ascii_lowercase();
    if value.contains("c++") || value.contains("gcc") {
        "cpp"
    } else if value.contains("python") || value.contains("pypy") {
        "python"
    } else if value.contains("java") {
        "java"
    } else if value.contains("rust") {
        "rust"
    } else if value.contains("kotlin") {
        "kotlin"
    } else if value.contains("javascript") {
        "javascript"
    } else {
        "text"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_contest_ids_and_links() {
        assert_eq!(
            normalize_contest_id("atcoder", "https://atcoder.jp/contests/abc380/tasks").unwrap(),
            "abc380"
        );
        assert_eq!(normalize_contest_id("codeforces", "2030").unwrap(), "2030");
        assert_eq!(
            normalize_contest_id("codeforces", "https://codeforces.com/contest/2030/problem/A").unwrap(),
            "2030"
        );
        assert!(normalize_contest_id("codeforces", "https://example.com/contest/2030").is_err());
        assert!(normalize_contest_id("nowcoder", "11244").is_err());
        assert!(normalize_contest_id("qoj", "123").is_err());
    }

    #[test]
    fn source_extractor_decodes_html() {
        let html = r#"<pre id="submission-code">if (a &lt; b) return "&amp;lt;";</pre>"#;
        assert_eq!(extract_source(html).as_deref(), Some(r#"if (a < b) return "&lt;";"#));
    }

    #[test]
    fn code_fence_survives_embedded_backticks() {
        let value = code_block("```\nvalue", "C++17");
        assert!(value.starts_with("````cpp"));
    }

    #[test]
    fn zip_writer_keeps_the_four_stable_document_names() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        let path = std::env::temp_dir().join(format!(
            "oj-insight-review-{}-{}.zip",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let documents = vec![
            ("00-START-HERE.md", "start".to_string()),
            ("01-CONTEST.md", "contest".to_string()),
            ("02-PROBLEMS.md", "problems".to_string()),
            ("03-SUBMISSIONS.md", "submissions".to_string()),
        ];
        write_zip(&path, &documents).unwrap();
        let archive = std::fs::read(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert!(archive.starts_with(b"PK\x03\x04"));
        assert_eq!(
            archive
                .windows(4)
                .filter(|value| *value == b"PK\x01\x02")
                .count(),
            4
        );
        for (name, _) in documents {
            assert!(archive
                .windows(name.len())
                .any(|value| value == name.as_bytes()));
        }
    }
}
