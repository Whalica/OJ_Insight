use reqwest::{
    header::{HeaderMap, HeaderValue, ACCEPT, COOKIE, REFERER, SET_COOKIE, USER_AGENT},
    Client,
};
use serde_json::Value;
use tokio::time::{sleep, Duration};

use crate::models::{AccountConfig, RemoteData, SyncError};

mod atcoder;
mod codeforces;
mod leetcode;
mod luogu;
mod metadata_cache;
mod nowcoder;
mod qoj;
pub(crate) mod service;
pub(crate) mod relationships;

const QOJ_SESSION_COOKIE_FALLBACK: &str = "__Host-UOJSESSID";

pub async fn fetch_platform(
    client: &Client,
    account: &AccountConfig,
    full: bool,
    cursor: i64,
    cache_dir: &std::path::Path,
) -> Result<RemoteData, SyncError> {
    match account.platform.as_str() {
        "atcoder" => atcoder::fetch(client, account, full, cursor, cache_dir).await,
        "codeforces" => codeforces::fetch(client, account, full, cursor).await,
        "luogu" => luogu::fetch(client, account, full, cursor).await,
        "nowcoder" => nowcoder::fetch(client, account, full, cursor, cache_dir).await,
        "qoj" => qoj::fetch(client, account, full, cursor).await,
        "leetcode" => leetcode::fetch(client, account, full, cursor).await,
        _ => Err(SyncError::error("不支持的平台")),
    }
}

pub fn browser_headers() -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/151 Safari/537.36 OJ-Insight/0.5"));
    h.insert(
        ACCEPT,
        HeaderValue::from_static(
            "text/html,application/xhtml+xml,application/json;q=0.9,*/*;q=0.8",
        ),
    );
    h
}

pub fn with_cookie(mut h: HeaderMap, cookie: &str) -> HeaderMap {
    let cookie = cookie.trim();
    if !cookie.is_empty() {
        if let Ok(v) = normalize_qoj_cookie(cookie).and_then(|value| HeaderValue::from_str(value).map_err(|_| "QOJ Cookie 格式无效".to_string())) {
            h.insert(COOKIE, v);
        }
    }
    h
}

pub fn normalize_qoj_cookie(cookie: &str) -> Result<&str, String> {
    let cookie = cookie.trim();
    let cookie = if cookie.get(..7).is_some_and(|prefix| prefix.eq_ignore_ascii_case("Cookie:")) {
        cookie[7..].trim()
    } else { cookie };
    if cookie.is_empty() {
        return Err("请填写从浏览器复制的完整 QOJ Cookie（名称=值）".into());
    }
    if HeaderValue::from_str(cookie).is_err()
        || cookie.split(';').any(|part| {
            let Some((name, _)) = part.trim().split_once('=') else { return true; };
            name.is_empty() || name.bytes().any(|byte| byte <= b' ' || byte == b';' || byte == b'=')
        })
    {
        return Err("QOJ Cookie 格式无效；请复制完整的名称=值，多个 Cookie 用分号分隔".into());
    }
    Ok(cookie)
}

fn qoj_cookie_value_only(cookie: &str) -> Result<&str, String> {
    let value = cookie.trim();
    if value.is_empty() || value.len() > 4096 || value.bytes().any(|byte| byte <= b' ' || byte == b';')
        || HeaderValue::from_str(value).is_err()
    {
        return Err("QOJ Cookie 值无效；请重新复制浏览器中的 Cookie 值".into());
    }
    Ok(value)
}

fn qoj_session_name(headers: &HeaderMap) -> Option<String> {
    let names: Vec<String> = headers.get_all(SET_COOKIE).iter()
        .filter_map(|header| header.to_str().ok()?.split(';').next()?.split_once('=').map(|(name, _)| name.trim().to_string()))
        .filter(|name| {
            let lower = name.to_ascii_lowercase();
            !name.is_empty() && !lower.starts_with("cf_") && !lower.starts_with("__cf")
                && !lower.starts_with("_ga")
                && name.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte))
        })
        .collect();
    let session: Vec<_> = names.iter().filter(|name| {
        let lower = name.to_ascii_lowercase();
        lower.contains("sess") || lower.ends_with("sid")
    }).collect();
    if session.len() == 1 { return Some((*session[0]).clone()); }
    None
}

fn qoj_cookie_is_header(input: &str) -> bool {
    input.contains(';') || input.get(..7).is_some_and(|prefix| prefix.eq_ignore_ascii_case("Cookie:"))
        || input.split_once('=').is_some_and(|(name, value)| !name.is_empty() && !value.is_empty() && value != "=")
}

pub async fn resolve_qoj_cookie(client: &Client, cookie: &str) -> Result<String, String> {
    let input = cookie.trim();
    if qoj_cookie_is_header(input) {
        return normalize_qoj_cookie(input).map(str::to_string);
    }
    let value = qoj_cookie_value_only(input)?;
    let mut name = None;
    for url in ["https://qoj.ac/login", "https://qoj.ac/"] {
        if let Ok(response) = client.get(url).headers(browser_headers()).send().await {
            name = qoj_session_name(response.headers());
            if name.is_some() { break; }
        }
    }
    let name = name.unwrap_or_else(|| QOJ_SESSION_COOKIE_FALLBACK.to_string());
    let resolved = format!("{name}={value}");
    normalize_qoj_cookie(&resolved)?;
    Ok(resolved)
}

#[cfg(test)]
mod qoj_cookie_tests {
    use super::{browser_headers, normalize_qoj_cookie, qoj_cookie_is_header, qoj_cookie_value_only, qoj_session_name, with_cookie, COOKIE, SET_COOKIE, QOJ_SESSION_COOKIE_FALLBACK};
    use reqwest::header::{HeaderMap, HeaderValue};

    #[test]
    fn sends_current_cookie_name_without_rewriting_it() {
        let cookie = "Cookie: new_session=abc123; preference=dark";
        assert_eq!(normalize_qoj_cookie(cookie).unwrap(), "new_session=abc123; preference=dark");
        let headers = with_cookie(browser_headers(), cookie);
        assert_eq!(headers.get(COOKIE).unwrap().to_str().unwrap(), "new_session=abc123; preference=dark");
    }

    #[test]
    fn rejects_bare_value_and_malformed_pairs() {
        assert!(normalize_qoj_cookie("abc123").is_err());
        assert!(normalize_qoj_cookie("session=abc123; missing_pair").is_err());
        assert!(normalize_qoj_cookie("session=abc123\r\nInjected: yes").is_err());
    }

    #[test]
    fn discovers_session_name_without_using_a_fixed_cookie_name() {
        let mut headers = HeaderMap::new();
        headers.append(SET_COOKIE, HeaderValue::from_static("cf_clearance=challenge; Path=/"));
        headers.append(SET_COOKIE, HeaderValue::from_static("QOJ_SESSION=guest; Path=/; HttpOnly"));
        assert_eq!(qoj_session_name(&headers).as_deref(), Some("QOJ_SESSION"));
        assert_eq!(qoj_cookie_value_only("abc123").unwrap(), "abc123");
        assert!(!qoj_cookie_is_header("abc123=="));
        assert!(qoj_cookie_is_header("QOJ_SESSION=abc123=="));
        let mut challenge_only = HeaderMap::new();
        challenge_only.append(SET_COOKIE, HeaderValue::from_static("cf_clearance=challenge; Path=/"));
        assert!(qoj_session_name(&challenge_only).is_none());
        let cookie = format!("{QOJ_SESSION_COOKIE_FALLBACK}={}", qoj_cookie_value_only("abc123").unwrap());
        assert_eq!(normalize_qoj_cookie(&cookie).unwrap(), cookie);
        let headers = with_cookie(browser_headers(), &cookie);
        assert_eq!(headers.get(COOKIE).unwrap().to_str().unwrap(), cookie);
        let mut preference_only = HeaderMap::new();
        preference_only.append(SET_COOKIE, HeaderValue::from_static("theme=dark; Path=/"));
        assert!(qoj_session_name(&preference_only).is_none());
    }
}

pub fn with_raw_cookie(mut h: HeaderMap, cookie: &str) -> HeaderMap {
    let cookie = cookie.trim();
    if !cookie.is_empty() {
        if let Ok(v) = HeaderValue::from_str(cookie) {
            h.insert(COOKIE, v);
        }
    }
    h
}

pub fn with_referer(mut h: HeaderMap, referer: &str) -> HeaderMap {
    if let Ok(value) = HeaderValue::from_str(referer) {
        h.insert(REFERER, value);
    }
    h
}

pub async fn get_text(client: &Client, url: &str, headers: HeaderMap) -> Result<String, SyncError> {
    let resp = client
        .get(url)
        .headers(headers)
        .send()
        .await
        .map_err(|e| SyncError::error(format!("网络请求失败：{e}")))?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| SyncError::error(format!("读取上游响应失败：{e}")))?;
    if !status.is_success() {
        return Err(SyncError::error(format!(
            "上游 HTTP {}：{}",
            status.as_u16(),
            host(url)
        )));
    }
    Ok(text)
}

pub async fn get_json(client: &Client, url: &str, headers: HeaderMap) -> Result<Value, SyncError> {
    let text = get_text(client, url, headers).await?;
    serde_json::from_str(&text)
        .map_err(|_| SyncError::error(format!("上游返回的不是有效 JSON：{}", host(url))))
}

pub async fn post_json(
    client: &Client,
    url: &str,
    headers: HeaderMap,
    body: Value,
) -> Result<Value, SyncError> {
    let resp = client
        .post(url)
        .headers(headers)
        .json(&body)
        .send()
        .await
        .map_err(|e| SyncError::error(format!("网络请求失败：{e}")))?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| SyncError::error(format!("读取上游响应失败：{e}")))?;
    if !status.is_success() {
        let hint = if status.as_u16() == 403 {
            "（可能触发 Cloudflare / 登录校验）"
        } else {
            ""
        };
        let operation = body
            .get("operationName")
            .and_then(Value::as_str)
            .unwrap_or("GraphQL");
        let detail = text
            .chars()
            .filter(|c| !c.is_control())
            .take(240)
            .collect::<String>();
        return Err(SyncError::error(format!(
            "{operation} HTTP {}：{}{} · {}",
            status.as_u16(),
            host(url),
            hint,
            detail
        )));
    }
    let value: Value = serde_json::from_str(&text)
        .map_err(|_| SyncError::error(format!("上游返回的不是有效 JSON：{}", host(url))))?;
    if let Some(errors) = value.get("errors").and_then(Value::as_array) {
        let message = errors
            .iter()
            .filter_map(|e| e.get("message").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("; ");
        if !message.is_empty() {
            return Err(SyncError::error(format!(
                "{} GraphQL：{}",
                host(url),
                message
            )));
        }
    }
    Ok(value)
}

pub fn host(url: &str) -> &str {
    url.split("//")
        .nth(1)
        .and_then(|x| x.split('/').next())
        .unwrap_or(url)
}

pub async fn polite_sleep(ms: u64) {
    sleep(Duration::from_millis(ms)).await;
}

pub fn now_epoch() -> i64 {
    chrono::Utc::now().timestamp()
}
