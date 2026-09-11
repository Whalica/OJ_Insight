use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

use regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};

use crate::models::{XcpcContest, XcpcProblem};
use crate::sync::{browser_headers, get_text, with_cookie};

const ROOT_CATEGORIES: [(&str, usize); 3] = [("21", 1), ("205", 1), ("212", 1)];
const CATALOG_CACHE_VERSION: u32 = 2;

#[derive(serde::Serialize, serde::Deserialize)]
struct CatalogCache {
    version: u32,
    contests: Vec<XcpcContest>,
}

pub async fn load_catalog(client: &Client, cache_path: &Path, cookie: &str, force_refresh: bool) -> Result<Vec<XcpcContest>, String> {
    let cached = std::fs::read_to_string(cache_path).ok()
        .and_then(|text| serde_json::from_str::<CatalogCache>(&text).ok())
        .filter(|cache| cache.version == CATALOG_CACHE_VERSION)
        .map(|cache| cache.contests)
        .filter(|items| !items.is_empty());
    if !force_refresh {
        if let Some(items) = cached.as_ref() { return Ok(items.clone()); }
    }
    let mut items = fetch_catalog(client, cookie).await?;
    if let Some(cached) = cached {
        let cached_by_id: HashMap<_, _> = cached.into_iter().map(|contest| (contest.id.clone(), contest)).collect();
        for contest in &mut items {
            if let Some(previous) = cached_by_id.get(&contest.id) {
                if cached_problems_are_well_formed(&previous.problems) {
                    contest.problems = previous.problems.clone();
                }
                if contest.board_source.is_none() { contest.board_source = previous.board_source.clone(); }
            }
        }
    }
    if !cookie.trim().is_empty() {
        enrich_contest_problems(client, cookie, &mut items).await;
    }
    let json = serde_json::to_string(&CatalogCache { version: CATALOG_CACHE_VERSION, contests: items.clone() })
        .map_err(|e| format!("序列化 XCPC 目录失败：{e}"))?;
    std::fs::write(cache_path, json).map_err(|e| format!("保存 XCPC 目录失败：{e}"))?;
    Ok(items)
}

async fn fetch_catalog(client: &Client, cookie: &str) -> Result<Vec<XcpcContest>, String> {
    let mut contests = HashMap::<String, XcpcContest>::new();
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();
    for (id, depth) in ROOT_CATEGORIES { queue.push_back((id.to_string(), depth)); }
    while !queue.is_empty() {
        let mut batch = Vec::new();
        while let Some((category_id, depth)) = queue.pop_front() {
            if visited.insert(category_id.clone()) { batch.push((category_id, depth)); }
        }
        for chunk in batch.chunks(8) {
            let mut tasks = tokio::task::JoinSet::new();
            for (category_id, depth) in chunk.iter().cloned() {
                let client = client.clone();
                let cookie = cookie.to_string();
                tasks.spawn(async move {
                    let url = format!("https://qoj.ac/category/{category_id}");
                    let html = get_text(&client, &url, with_cookie(browser_headers(), &cookie)).await
                        .map_err(|e| format!("更新 XCPC 目录失败：{e}"))?;
                    Ok::<_, String>((depth, parse_category(&html)))
                });
            }
            while let Some(result) = tasks.join_next().await {
                let (depth, page) = result.map_err(|e| format!("更新 XCPC 目录任务失败：{e}"))??;
                for contest in page.contests { contests.entry(contest.id.clone()).or_insert(contest); }
                if depth > 0 {
                    for child in page.child_categories {
                        if !visited.contains(&child) { queue.push_back((child, depth - 1)); }
                    }
                }
            }
        }
    }
    let mut items: Vec<_> = contests.into_values().collect();
    items.retain(|contest| !is_warmup(&contest.name));
    items.sort_by(|a, b| b.year.cmp(&a.year).then_with(|| numeric_id(&b.id).cmp(&numeric_id(&a.id))).then_with(|| a.name.cmp(&b.name)));
    if items.is_empty() {
        // QOJ occasionally serves the category table without the problem-column
        // markup. Keep the tracker usable by falling back to its public contest list.
        items = fetch_contest_list(client, cookie).await?;
    }
    if items.is_empty() { return Err("QOJ 页面已返回，但没有识别到 XCPC 比赛；请稍后重试".into()); }
    Ok(items)
}

async fn enrich_contest_problems(client: &Client, cookie: &str, contests: &mut [XcpcContest]) {
    let missing: Vec<_> = contests.iter().enumerate()
        .filter(|(_, contest)| contest.problems.is_empty())
        .map(|(index, contest)| (index, contest.id.clone()))
        .collect();
    for chunk in missing.chunks(8) {
        let mut tasks = tokio::task::JoinSet::new();
        for (index, contest_id) in chunk.iter().cloned() {
            let client = client.clone();
            let cookie = cookie.to_string();
            tasks.spawn(async move {
                let url = format!("https://qoj.ac/contest/{contest_id}");
                let problems = fetch_contest_problems(&client, &url, &cookie).await?;
                Ok::<_, String>((index, problems))
            });
        }
        while let Some(result) = tasks.join_next().await {
            if let Ok(Ok((index, problems))) = result {
                if !problems.is_empty() { contests[index].problems = problems; }
            }
        }
    }
}

async fn fetch_contest_problems(client: &Client, url: &str, cookie: &str) -> Result<Vec<XcpcProblem>, String> {
    let html = get_text(client, url, with_cookie(browser_headers(), cookie)).await
        .map_err(|e| e.to_string())?;
    let doc = Html::parse_document(&html);
    let anchor_sel = Selector::parse("a[href]").unwrap();
    let problem_re = Regex::new(r"^(?:https?://qoj\.ac)?/(?:contest/\d+/)?problem/(\d+)(?:$|[/?#])").unwrap();
    let mut problems = Vec::new();
    let mut seen = HashSet::new();
    for anchor in doc.select(&anchor_sel) {
        let Some(href) = anchor.value().attr("href") else { continue };
        let Some(problem_id) = problem_re.captures(href).map(|capture| capture[1].to_string()) else { continue };
        if !seen.insert(problem_id.clone()) { continue; }
        let raw = text_of(&anchor);
        let index = fallback_problem_index(problems.len());
        let (_, parsed_name) = problem_label(&raw, problems.len());
        let title = anchor.value().attr("title").or_else(|| anchor.value().attr("data-original-title")).unwrap_or_default().trim();
        let name = if !title.is_empty() { clean_problem_name(title, &index) } else { parsed_name };
        problems.push(XcpcProblem { index, name, url: format!("https://qoj.ac/problem/{problem_id}"), problem_id, tier: None, accepted_teams: None, total_teams: None, solved: false });
    }
    Ok(problems)
}

struct ParsedCategory { contests: Vec<XcpcContest>, child_categories: Vec<String> }

fn parse_category(html: &str) -> ParsedCategory {
    let doc = Html::parse_document(html);
    let row_sel = Selector::parse("table tr").unwrap();
    let anchor_sel = Selector::parse("a[href]").unwrap();
    let contest_re = Regex::new(r"^(?:https?://qoj\.ac)?/contest/(\d+)(?:$|[/?#])").unwrap();
    let problem_re = Regex::new(r"^(?:https?://qoj\.ac)?/(?:contest/\d+/)?problem/(\d+)(?:$|[/?#])").unwrap();
    let category_re = Regex::new(r"^(?:https?://qoj\.ac)?/category/(\d+)(?:$|[/?#])").unwrap();
    let mut contests = Vec::new();
    let mut child_categories = Vec::new();
    for row in doc.select(&row_sel) {
        let anchors: Vec<_> = row.select(&anchor_sel).collect();
        let contest_anchor = anchors.iter().find(|anchor| anchor.value().attr("href").is_some_and(|href| contest_re.is_match(href)));
        let Some(contest_anchor) = contest_anchor else {
            for anchor in anchors {
                if let Some(id) = anchor.value().attr("href").and_then(|href| category_re.captures(href)).map(|captures| captures[1].to_string()) { child_categories.push(id); }
            }
            continue;
        };
        let contest_href = contest_anchor.value().attr("href").unwrap_or_default();
        let Some(contest_id) = contest_re.captures(contest_href).map(|captures| captures[1].to_string()) else { continue };
        let name = text_of(contest_anchor);
        if name.is_empty() { continue; }
        let mut problems = Vec::new();
        for anchor in anchors {
            let Some(href) = anchor.value().attr("href") else { continue };
            let Some(problem_id) = problem_re.captures(href).map(|captures| captures[1].to_string()) else { continue };
            let raw_index = text_of(&anchor);
            let index = fallback_problem_index(problems.len());
            let (_, parsed_name) = problem_label(&raw_index, problems.len());
            let title = anchor.value().attr("title").or_else(|| anchor.value().attr("data-original-title")).or_else(|| anchor.value().attr("aria-label")).unwrap_or_default().trim();
            let problem_name = if !title.is_empty() { clean_problem_name(title, &index) } else { parsed_name };
            problems.push(XcpcProblem { index, name: problem_name, url: format!("https://qoj.ac/problem/{problem_id}"), problem_id, tier: None, accepted_teams: None, total_teams: None, solved: false });
        }
        let year = extract_year(&name);
        contests.push(XcpcContest {
            id: contest_id.clone(), name: name.clone(), short_name: short_name(&name, &year), url: format!("https://qoj.ac/contest/{contest_id}"),
            date: String::new(), year, series: classify_series(&name), stage: classify_stage(&name), site: classify_site(&name), board_source: None, problems,
        });
    }
    let parsed = ParsedCategory { contests, child_categories };
    if parsed.contests.is_empty() || parsed.contests.iter().all(|contest| contest.problems.is_empty()) {
        let fallback = parse_category_rows_from_html(html);
        if !fallback.contests.is_empty() || !fallback.child_categories.is_empty() { return fallback; }
    }
    parsed
}

fn parse_category_rows_from_html(html: &str) -> ParsedCategory {
    let row_re = Regex::new(r"(?is)<tr\b[^>]*>(.*?)</tr>").unwrap();
    let anchor_re = Regex::new(r#"(?is)<a\b[^>]*href\s*=\s*["']([^"']+)["'][^>]*>(.*?)</a>"#).unwrap();
    let tag_re = Regex::new(r"(?is)<[^>]+>").unwrap();
    let contest_re = Regex::new(r"^(?:https?://qoj\.ac)?/contest/(\d+)(?:$|[/?#])").unwrap();
    let problem_re = Regex::new(r"^(?:https?://qoj\.ac)?/(?:contest/\d+/)?problem/(\d+)(?:$|[/?#])").unwrap();
    let category_re = Regex::new(r"^(?:https?://qoj\.ac)?/category/(\d+)(?:$|[/?#])").unwrap();
    let mut contests = Vec::new();
    let mut child_categories = Vec::new();
    for capture in row_re.captures_iter(html) {
        let row = capture.get(1).map(|match_| match_.as_str()).unwrap_or_default();
        let anchors: Vec<_> = anchor_re.captures_iter(row).collect();
        let contest = anchors.iter().find(|anchor| contest_re.is_match(&anchor[1]));
        let Some(contest) = contest else {
            for anchor in anchors { if let Some(id) = category_re.captures(&anchor[1]).map(|capture| capture[1].to_string()) { child_categories.push(id); } }
            continue;
        };
        let Some(id) = contest_re.captures(&contest[1]).map(|capture| capture[1].to_string()) else { continue };
        let name = tag_re.replace_all(&contest[2], " ").split_whitespace().collect::<Vec<_>>().join(" ");
        if name.is_empty() { continue; }
        let mut problems = Vec::new();
        for anchor in anchors {
            let Some(problem_id) = problem_re.captures(&anchor[1]).map(|capture| capture[1].to_string()) else { continue };
            let raw = tag_re.replace_all(&anchor[2], " ").split_whitespace().collect::<Vec<_>>().join(" ");
            let index = fallback_problem_index(problems.len());
            let (_, name) = problem_label(&raw, problems.len());
            problems.push(XcpcProblem { index, name, url: format!("https://qoj.ac/problem/{problem_id}"), problem_id, tier: None, accepted_teams: None, total_teams: None, solved: false });
        }
        let year = extract_year(&name);
        contests.push(XcpcContest { id: id.clone(), name: name.clone(), short_name: short_name(&name, &year), url: format!("https://qoj.ac/contest/{id}"), date: String::new(), year, series: classify_series(&name), stage: classify_stage(&name), site: classify_site(&name), board_source: None, problems });
    }
    ParsedCategory { contests, child_categories }
}

async fn fetch_contest_list(client: &Client, cookie: &str) -> Result<Vec<XcpcContest>, String> {
    let html = get_text(client, "https://qoj.ac/contests?tab=icpc", with_cookie(browser_headers(), cookie))
        .await
        .map_err(|e| format!("更新 XCPC 目录失败：{e}"))?;
    let doc = Html::parse_document(&html);
    let row_sel = Selector::parse("table tr").unwrap();
    let anchor_sel = Selector::parse("a[href]").unwrap();
    let contest_re = Regex::new(r"^(?:https?://qoj\.ac)?/contest/(\d+)(?:$|[/?#])").unwrap();
    let mut items = Vec::new();
    for row in doc.select(&row_sel) {
        let Some(anchor) = row.select(&anchor_sel).find(|a| a.value().attr("href").is_some_and(|h| contest_re.is_match(h))) else { continue };
        let Some(id) = anchor.value().attr("href").and_then(|h| contest_re.captures(h)).map(|c| c[1].to_string()) else { continue };
        let name = text_of(&anchor);
        if name.is_empty() { continue; }
        let year = extract_year(&name);
        items.push(XcpcContest { id: id.clone(), name: name.clone(), short_name: short_name(&name, &year), url: format!("https://qoj.ac/contest/{id}"), date: String::new(), year, series: classify_series(&name), stage: classify_stage(&name), site: classify_site(&name), board_source: None, problems: Vec::new() });
    }
    items.retain(|contest| !is_warmup(&contest.name));
    items.sort_by(|a, b| b.year.cmp(&a.year).then_with(|| numeric_id(&b.id).cmp(&numeric_id(&a.id))));
    Ok(items)
}

fn text_of(element: &scraper::ElementRef<'_>) -> String { element.text().collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ") }
fn fallback_problem_index(position: usize) -> String {
    if position < 26 { ((b'A' + position as u8) as char).to_string() } else { (position + 1).to_string() }
}
fn problem_label(raw: &str, position: usize) -> (String, String) {
    let text = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let label_re = Regex::new(r"(?i)^(?:problem\s+)?([a-z][a-z0-9]{0,2})(?:\s*[.．:：]\s*|\s+)(.+)$").unwrap();
    if let Some(capture) = label_re.captures(&text) {
        return (capture[1].to_ascii_uppercase(), capture[2].trim().to_string());
    }
    if !text.is_empty() && text.len() <= 3 && text.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        return (text.to_ascii_uppercase(), String::new());
    }
    (fallback_problem_index(position), text)
}
fn clean_problem_name(raw: &str, index: &str) -> String {
    let text = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let prefix = Regex::new(&format!(r"(?i)^(?:problem\s+)?{}\s*[.．:：]\s*", regex::escape(index))).unwrap();
    prefix.replace(&text, "").trim().to_string()
}
fn cached_problems_are_well_formed(problems: &[XcpcProblem]) -> bool {
    !problems.is_empty() && problems.iter().enumerate().all(|(position, problem)| {
        problem.index == fallback_problem_index(position) && clean_problem_name(&problem.name, &problem.index) == problem.name
    })
}
fn extract_year(name: &str) -> String { Regex::new(r"(?:19|20)\d{2}").unwrap().find(name).map(|value| value.as_str().to_string()).unwrap_or_else(|| "未知".into()) }

fn classify_series(name: &str) -> Vec<String> {
    let low = name.to_ascii_lowercase();
    let mut values = Vec::new();
    if low.contains("icpc") { values.push("ICPC".into()); }
    if low.contains("ccpc") || name.contains("中国大学生程序设计竞赛") { values.push("CCPC".into()); }
    if low.contains("provincial") || low.contains("province programming") || name.contains("省赛") || name.contains("省大学生") { values.push("省赛".into()); }
    if values.is_empty() { values.push("其他".into()); }
    values
}

fn classify_stage(name: &str) -> String {
    let low = name.to_ascii_lowercase();
    if low.contains("online") || name.contains("网络") { "网络赛" }
    else if low.contains("invitational") || name.contains("邀请赛") { "邀请赛" }
    else if low.contains("provincial") || low.contains("province programming") || name.contains("省赛") || name.contains("省大学生") { "省赛" }
    else if low.contains("final") || name.contains("总决赛") { "总决赛" }
    else if low.contains("regional") { "区域赛" }
    else if low.contains("site") || name.contains('站') { "分站赛" }
    else { "其他" }.into()
}

fn classify_site(name: &str) -> String {
    const SITES: [(&str, &str); 35] = [
        ("Beijing", "北京"), ("北京", "北京"), ("Changchun", "长春"), ("长春", "长春"), ("Chengdu", "成都"), ("成都", "成都"),
        ("Chongqing", "重庆"), ("重庆", "重庆"), ("Fujian", "福建"), ("Guangdong", "广东"), ("Guangzhou", "广州"), ("广州", "广州"),
        ("Hangzhou", "杭州"), ("Harbin", "哈尔滨"), ("哈尔滨", "哈尔滨"), ("Hefei", "合肥"), ("Hong Kong", "香港"),
        ("Jinan", "济南"), ("济南", "济南"), ("Kunming", "昆明"), ("Nanchang", "南昌"), ("Nanjing", "南京"), ("南京", "南京"),
        ("Qingdao", "青岛"), ("Shanghai", "上海"), ("上海", "上海"), ("Shenyang", "沈阳"), ("沈阳", "沈阳"),
        ("Wuhan", "武汉"), ("武汉", "武汉"), ("Xi'an", "西安"), ("西安", "西安"), ("Zhengzhou", "郑州"), ("郑州", "郑州"), ("Zhejiang", "浙江")
    ];
    SITES.iter().find(|(needle, _)| name.contains(needle)).map(|(_, label)| (*label).to_string()).unwrap_or_else(|| "全国".into())
}

fn short_name(name: &str, year: &str) -> String {
    let series = if name.to_ascii_lowercase().contains("ccpc") || name.contains("中国大学生程序设计竞赛") { "CCPC" } else if name.to_ascii_lowercase().contains("icpc") { "ICPC" } else { "XCPC" };
    format!("{year} {series} {}{}", classify_site(name), classify_stage(name))
}
fn is_warmup(name: &str) -> bool { let low = name.to_ascii_lowercase(); low.contains("warm up") || low.contains("warm-up") || low.contains("practice") || name.contains("热身") }
fn numeric_id(id: &str) -> i64 { id.parse().unwrap_or_default() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_realistic_category_rows() {
        let html = r#"<table><tr><td><a href="/contest/2513">The 2025 ICPC Asia Nanjing Regional Contest</a></td><td><a title="A. Array" href="/contest/2513/problem/14001/statement/zh_cn">A. Array</a><a href="/problem/14002">B. Bitset</a></td></tr><tr><td><a href="/category/84">Nanjing</a></td></tr></table>"#;
        let parsed = parse_category(html);
        assert_eq!(parsed.child_categories, vec!["84"]);
        assert_eq!(parsed.contests.len(), 1);
        assert_eq!(parsed.contests[0].problems[0].name, "Array");
        assert_eq!(parsed.contests[0].problems[1].index, "B");
        assert_eq!(parsed.contests[0].problems[1].name, "Bitset");
        assert_eq!(parsed.contests[0].problems[1].url, "https://qoj.ac/problem/14002");
    }
}
