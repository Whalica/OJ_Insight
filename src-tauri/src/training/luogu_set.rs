use std::collections::HashSet;

use regex::Regex;
use reqwest::{header::{HeaderMap, HeaderValue, ACCEPT, COOKIE, REFERER, USER_AGENT}, Client, Url};
use serde_json::Value;

use super::{CanonicalProblem, ProblemSetInput, ProblemSetProblem};

fn training_id(input: &str) -> Result<String, String> {
    let url = Url::parse(input.trim()).map_err(|_| "请输入洛谷题单链接".to_string())?;
    if url.scheme() != "https" || url.host_str() != Some("www.luogu.com.cn") && url.host_str() != Some("luogu.com.cn") {
        return Err("仅支持 https://www.luogu.com.cn/training/数字 题单链接".into());
    }
    let segments: Vec<_> = url.path_segments().ok_or("题单链接无效")?.filter(|part| !part.is_empty()).collect();
    if segments.len() != 2 || segments[0] != "training" || !segments[1].chars().all(|ch| ch.is_ascii_digit()) {
        return Err("仅支持洛谷题单详情页链接".into());
    }
    Ok(segments[1].into())
}

fn valid_problem_id(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else { return false; };
    first.is_ascii_alphabetic() && value.len() <= 32 && value.bytes().any(|byte| byte.is_ascii_digit()) && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn problem_from_value(value: &Value) -> Option<(String, String)> {
    if let Some(id) = value.as_str().filter(|id| valid_problem_id(id)) { return Some((id.to_ascii_uppercase(), String::new())); }
    let object = value.as_object()?;
    let id = object.get("pid").or_else(|| object.get("problemId")).or_else(|| object.get("problem_id"))?.as_str()?;
    if !valid_problem_id(id) { return None; }
    let name = object.get("title").or_else(|| object.get("name")).and_then(Value::as_str).unwrap_or_default();
    Some((id.to_ascii_uppercase(), name.to_string()))
}

fn collect_problems(value: &Value, output: &mut Vec<(String, String)>) {
    if let Some(problem) = problem_from_value(value) { output.push(problem); return; }
    if let Some(array) = value.as_array() { for row in array { collect_problems(row, output); } return; }
    if let Some(object) = value.as_object() {
        for key in ["problems", "problemList", "problem_list", "sections", "groups", "items"] {
            if let Some(child) = object.get(key) { collect_problems(child, output); }
        }
    }
}

fn parse_payload(text: &str) -> Option<Value> {
    if let Ok(value) = serde_json::from_str(text) { return Some(value); }
    let capture = Regex::new(r#"decodeURIComponent\(("(?:[^"\\]|\\.)*")\)"#).ok()?.captures(text)?;
    let encoded = serde_json::from_str::<String>(capture.get(1)?.as_str()).ok()?;
    serde_json::from_str(&urlencoding::decode(&encoded).ok()?).ok()
}

fn parse_html_links(text: &str, output: &mut Vec<(String, String)>) {
    let Ok(link) = Regex::new(r#"(?is)<a\b[^>]*href=["'](?:https?://(?:www\.)?luogu\.com\.cn)?/problem/([A-Za-z][A-Za-z0-9_]*)[^"']*["'][^>]*>(.*?)</a>"#) else { return; };
    let tags = Regex::new(r"(?s)<[^>]+>").expect("static HTML tag regex");
    for capture in link.captures_iter(text) {
        let id = capture.get(1).map(|value| value.as_str()).unwrap_or_default();
        if valid_problem_id(id) {
            let name = capture.get(2).map(|value| tags.replace_all(value.as_str(), "").trim().to_string()).unwrap_or_default();
            output.push((id.to_ascii_uppercase(), name));
        }
    }
}

fn canonical_problem(key: String, name: String) -> CanonicalProblem {
    let (platform, problem_key, problem_id, url) = if let Some(rest) = key.strip_prefix("CF") {
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        let (contest, index) = rest.split_at(digits);
        if !contest.is_empty() && !index.is_empty() && index.len() <= 3
            && index.starts_with(|ch: char| ch.is_ascii_uppercase())
            && index.chars().all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit()) {
            ("codeforces", format!("{contest}:{index}"), format!("{contest}{index}"), format!("https://codeforces.com/contest/{contest}/problem/{index}"))
        } else { ("luogu", key.clone(), key.clone(), format!("https://www.luogu.com.cn/problem/{key}")) }
    } else if let Some(task) = key.strip_prefix("AT_") {
        let task = task.to_ascii_lowercase();
        if let Some((contest, suffix)) = task.rsplit_once('_') {
            let standard_contest = ["abc", "arc", "agc", "ahc"].iter().any(|prefix| contest.strip_prefix(prefix).is_some_and(|number| !number.is_empty() && number.bytes().all(|ch| ch.is_ascii_digit())));
            if !suffix.is_empty() && (standard_contest || matches!(contest, "dp" | "typical90")) {
                ("atcoder", task.clone(), task.clone(), format!("https://atcoder.jp/contests/{contest}/tasks/{task}"))
            } else { ("luogu", key.clone(), key.clone(), format!("https://www.luogu.com.cn/problem/{key}")) }
        } else { ("luogu", key.clone(), key.clone(), format!("https://www.luogu.com.cn/problem/{key}")) }
    } else { ("luogu", key.clone(), key.clone(), format!("https://www.luogu.com.cn/problem/{key}")) };
    let canonical_id = format!("{platform}:{problem_key}");
    let fallback_name = match platform { "codeforces" => format!("CF {problem_id}"), "atcoder" => format!("AtCoder {problem_id}"), _ => format!("洛谷 {key}") };
    CanonicalProblem { canonical_id, platform: platform.into(), problem_key, problem_id, name: if name.is_empty() { fallback_name } else { name }, url, difficulty: None, tags: Vec::new(), training_suitability: None, observation_dependency: None, implementation_load: None, knowledge_dependency: None, interactive: false, output_only: false }
}

pub(crate) fn preview_from_page(id: &str, text: &str) -> Result<ProblemSetInput, String> {
    let url = format!("https://www.luogu.com.cn/training/{id}");
    let mut title = format!("洛谷题单 {id}");
    let mut problems = Vec::new();
    let mut expected = None;
    if let Some(payload) = parse_payload(text) {
        let data = payload.get("currentData").or_else(|| payload.get("data")).unwrap_or(&payload);
        let training = data.get("training").unwrap_or(data);
        title = training.get("title").or_else(|| training.get("name")).and_then(Value::as_str).filter(|value| !value.trim().is_empty()).unwrap_or(&title).to_string();
        expected = training.get("problemCount").or_else(|| training.get("problem_count")).and_then(Value::as_u64);
        collect_problems(training, &mut problems);
    }
    if problems.is_empty() { parse_html_links(text, &mut problems); }
    let mut seen = HashSet::new();
    problems.retain(|(key, _)| seen.insert(key.clone()));
    if problems.is_empty() { return Err("未从洛谷页面读到题目；请检查题单可见性及洛谷 Cookie，或使用批量粘贴题目链接。".into()); }
    if expected.is_some_and(|count| count > problems.len() as u64) { return Err(format!("洛谷题单标记了 {} 道题，但只读取到 {} 道；为避免漏题，已取消导入。", expected.unwrap_or_default(), problems.len())); }
    let entries = problems.into_iter().enumerate().map(|(position, (key, name))| ProblemSetProblem {
        position: position as i64, role: "Core".into(), note: String::new(),
        problem: canonical_problem(key, name),
    }).collect();
    Ok(ProblemSetInput { id: None, title, description: format!("来源：{url}"), set_type: "static".into(), tag_visibility: "after_ac".into(), source_set_id: None, source_url: Some(url), problems: entries })
}

pub(crate) async fn fetch_preview(client: &Client, input: &str, cookie: &str) -> Result<ProblemSetInput, String> {
    let id = training_id(input)?;
    let url = format!("https://www.luogu.com.cn/training/{id}");
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 OJ-Insight training import"));
    headers.insert(ACCEPT, HeaderValue::from_static("application/json,text/html;q=0.9,*/*;q=0.8"));
    headers.insert(REFERER, HeaderValue::from_static("https://www.luogu.com.cn/"));
    headers.insert("x-lentille-request", HeaderValue::from_static("content-only"));
    if !cookie.trim().is_empty() {
        headers.insert(COOKIE, HeaderValue::from_str(cookie.trim()).map_err(|_| "洛谷 Cookie 格式无效".to_string())?);
    }
    let response = client.get(&url).headers(headers).send().await.map_err(|error| format!("读取洛谷题单失败：{error}"))?;
    if response.url().path().starts_with("/auth/") || response.status().as_u16() == 401 || response.status().as_u16() == 403 {
        return Err("洛谷要求登录或拒绝访问；请在当前导入框填写有效的洛谷 Cookie。".into());
    }
    let response = response.error_for_status().map_err(|error| format!("洛谷题单请求失败：{error}"))?;
    if response.content_length().is_some_and(|size| size > 2_000_000) { return Err("洛谷题单页面过大".into()); }
    let bytes = response.bytes().await.map_err(|error| format!("读取洛谷题单失败：{error}"))?;
    if bytes.len() > 2_000_000 { return Err("洛谷题单页面过大".into()); }
    preview_from_page(&id, &String::from_utf8_lossy(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_ordered_problems_and_ignores_duplicates() {
        let page = r#"{"currentData":{"training":{"title":"入门题单","problemCount":2,"problems":[{"pid":"P1421","title":"小玉买文具"},{"pid":"B2002","title":"Hello"},{"pid":"P1421","title":"重复"}]}}}"#;
        let preview = preview_from_page("42", page).unwrap();
        assert_eq!(preview.problems.len(), 2);
        assert_eq!(preview.problems[0].problem.name, "小玉买文具");
        assert_eq!(preview.problems[1].problem.problem_key, "B2002");
    }

    #[test]
    fn rejects_partial_page() {
        let page = r#"{"training":{"problemCount":3,"problems":[{"pid":"P1421"}]}}"#;
        assert!(preview_from_page("42", page).is_err());
    }

    #[test]
    fn maps_supported_remote_judge_problems_to_original_sites() {
        let page = r#"{"training":{"problemCount":4,"problems":[{"pid":"CF1234D","title":"Distinct Characters Queries"},{"pid":"AT_abc376_e","title":"Max × Sum"},{"pid":"AT_s8pc_4_h","title":"Unknown contest mapping"},{"pid":"UVA100","title":"The 3n + 1 problem"}]}}"#;
        let preview = preview_from_page("42", page).unwrap();
        assert_eq!(preview.problems[0].problem.platform, "codeforces");
        assert_eq!(preview.problems[0].problem.problem_key, "1234:D");
        assert_eq!(preview.problems[1].problem.platform, "atcoder");
        assert_eq!(preview.problems[1].problem.problem_key, "abc376_e");
        assert_eq!(preview.problems[2].problem.platform, "luogu");
        assert_eq!(preview.problems[3].problem.platform, "luogu");
    }
}
