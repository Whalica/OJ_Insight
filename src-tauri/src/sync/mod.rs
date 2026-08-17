use reqwest::{header::{HeaderMap, HeaderValue, ACCEPT, COOKIE, REFERER, USER_AGENT}, Client};
use serde_json::Value;
use tokio::time::{sleep, Duration};

use crate::models::{AccountConfig, RemoteData, SyncError};

mod atcoder;
mod codeforces;
mod leetcode;
mod luogu;
mod nowcoder;
mod qoj;

pub async fn fetch_platform(client: &Client, account: &AccountConfig, full: bool, cursor: i64) -> Result<RemoteData, SyncError> {
    match account.platform.as_str() {
        "atcoder" => atcoder::fetch(client, account, full, cursor).await,
        "codeforces" => codeforces::fetch(client, account, full, cursor).await,
        "luogu" => luogu::fetch(client, account, full, cursor).await,
        "nowcoder" => nowcoder::fetch(client, account, full, cursor).await,
        "qoj" => qoj::fetch(client, account, full, cursor).await,
        "leetcode" => leetcode::fetch(client, account, full, cursor).await,
        _ => Err(SyncError::error("不支持的平台")),
    }
}

pub fn browser_headers() -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/151 Safari/537.36 OJ-Insight/0.1"));
    h.insert(ACCEPT, HeaderValue::from_static("text/html,application/xhtml+xml,application/json;q=0.9,*/*;q=0.8"));
    h
}

pub fn with_cookie(mut h: HeaderMap, cookie: &str) -> HeaderMap {
    if !cookie.trim().is_empty() {
        if let Ok(v) = HeaderValue::from_str(cookie.trim()) { h.insert(COOKIE, v); }
    }
    h
}

pub fn with_referer(mut h: HeaderMap, referer: &'static str) -> HeaderMap {
    h.insert(REFERER, HeaderValue::from_static(referer));
    h
}

pub async fn get_text(client: &Client, url: &str, headers: HeaderMap) -> Result<String, SyncError> {
    let resp = client.get(url).headers(headers).send().await.map_err(|e| SyncError::error(format!("网络请求失败：{e}")))?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| SyncError::error(format!("读取上游响应失败：{e}")))?;
    if !status.is_success() { return Err(SyncError::error(format!("上游 HTTP {}：{}", status.as_u16(), host(url)))); }
    Ok(text)
}

pub async fn get_json(client: &Client, url: &str, headers: HeaderMap) -> Result<Value, SyncError> {
    let text = get_text(client, url, headers).await?;
    serde_json::from_str(&text).map_err(|_| SyncError::error(format!("上游返回的不是有效 JSON：{}", host(url))))
}

pub async fn post_json(client: &Client, url: &str, headers: HeaderMap, body: Value) -> Result<Value, SyncError> {
    let resp = client.post(url).headers(headers).json(&body).send().await.map_err(|e| SyncError::error(format!("网络请求失败：{e}")))?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| SyncError::error(format!("读取上游响应失败：{e}")))?;
    if !status.is_success() {
        let hint = if status.as_u16() == 403 { "（可能触发 Cloudflare / 登录校验）" } else { "" };
        return Err(SyncError::error(format!("上游 HTTP {}：{}{}", status.as_u16(), host(url), hint)));
    }
    serde_json::from_str(&text).map_err(|_| SyncError::error(format!("上游返回的不是有效 JSON：{}", host(url))))
}

pub fn host(url: &str) -> &str {
    url.split("//").nth(1).and_then(|x| x.split('/').next()).unwrap_or(url)
}

pub async fn polite_sleep(ms: u64) { sleep(Duration::from_millis(ms)).await; }

pub fn now_epoch() -> i64 { chrono::Utc::now().timestamp() }
