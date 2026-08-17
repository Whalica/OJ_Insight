use reqwest::{
    header::{HeaderMap, HeaderValue, ACCEPT, REFERER, USER_AGENT},
    Client,
};
use serde_json::Value;

use super::{get_json, get_text, now_epoch};
use crate::models::{AccountConfig, AggregateDay, DifficultyStat, RemoteData, SyncError};

fn base_headers() -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(
        USER_AGENT,
        HeaderValue::from_static("OJ-Insight/0.2 local analytics"),
    );
    h.insert(
        ACCEPT,
        HeaderValue::from_static("application/json,text/plain,*/*"),
    );
    h.insert(
        REFERER,
        HeaderValue::from_static("https://www.luogu.com.cn/"),
    );
    h
}
fn lentille_headers() -> HeaderMap {
    let mut h = base_headers();
    h.insert(
        "x-lentille-request",
        HeaderValue::from_static("content-only"),
    );
    h
}

fn parse_payload(text: &str) -> Result<Value, SyncError> {
    if let Ok(v) = serde_json::from_str(text) {
        return Ok(v);
    }

    // Compatibility fallback for Luogu's older loader page:
    // decodeURIComponent("%7B...%7D")
    if let Ok(re) = regex::Regex::new(r#"decodeURIComponent\(("(?:[^"\\]|\\.)*")\)"#) {
        if let Some(caps) = re.captures(text) {
            if let Some(raw) = caps.get(1) {
                if let Ok(encoded) = serde_json::from_str::<String>(raw.as_str()) {
                    if let Ok(decoded) = urlencoding::decode(&encoded) {
                        if let Ok(v) = serde_json::from_str::<Value>(&decoded) {
                            return Ok(v);
                        }
                    }
                }
            }
        }
    }

    let plain = text.replace(['\n', '\r', '\t'], " ");
    let lower = plain.to_lowercase();
    if plain.contains("访问")
        || plain.contains("频繁")
        || plain.contains("验证码")
        || lower.contains("captcha")
        || lower.contains("forbidden")
        || lower.contains("challenge")
    {
        return Err(SyncError::error("洛谷触发访问限制或验证页面"));
    }
    Err(SyncError::error("洛谷返回格式异常"))
}

async fn resolve_uid(client: &Client, input: &str) -> Result<(String, String), SyncError> {
    if input.chars().all(|c| c.is_ascii_digit()) {
        return Ok((input.into(), input.into()));
    }
    let url = format!(
        "https://www.luogu.com.cn/api/user/search?keyword={}",
        urlencoding::encode(input)
    );
    let payload = get_json(client, &url, base_headers()).await.map_err(|e| {
        SyncError::error(format!(
            "洛谷用户名搜索失败；可直接填写数字 UID。{}",
            e.message
        ))
    })?;
    let candidates = payload
        .get("users")
        .or_else(|| payload.pointer("/data/users"))
        .or_else(|| payload.pointer("/currentData/users"))
        .or_else(|| payload.get("result"))
        .and_then(Value::as_array)
        .ok_or_else(|| SyncError::error("未找到洛谷用户；可改填数字 UID"))?;
    let mut chosen = candidates.first();
    for u in candidates {
        let name = u
            .get("name")
            .or_else(|| u.get("username"))
            .and_then(Value::as_str)
            .unwrap_or("");
        if name.eq_ignore_ascii_case(input) {
            chosen = Some(u);
            break;
        }
    }
    let u = chosen.ok_or_else(|| SyncError::error("未找到洛谷用户"))?;
    let uid = u
        .get("uid")
        .or_else(|| u.get("id"))
        .and_then(|x| {
            x.as_i64()
                .map(|n| n.to_string())
                .or_else(|| x.as_str().map(str::to_string))
        })
        .ok_or_else(|| SyncError::error("洛谷用户名解析失败；可改填数字 UID"))?;
    let name = u
        .get("name")
        .or_else(|| u.get("username"))
        .and_then(Value::as_str)
        .unwrap_or(input)
        .to_string();
    Ok((uid, name))
}

pub async fn fetch(
    client: &Client,
    account: &AccountConfig,
    _full: bool,
    _cursor: i64,
) -> Result<RemoteData, SyncError> {
    let input = account.account.trim();
    if input.is_empty() {
        return Err(SyncError::error("洛谷用户名/UID 为空"));
    }
    let (uid, display) = resolve_uid(client, input).await?;
    let profile_text = get_text(
        client,
        &format!("https://www.luogu.com.cn/user/{uid}"),
        lentille_headers(),
    )
    .await?;
    let payload = parse_payload(&profile_text)?;
    let data = payload
        .get("data")
        .or_else(|| payload.get("currentData"))
        .unwrap_or(&payload);
    let daily = data.get("dailyCounts").ok_or_else(|| {
        SyncError::error("洛谷个人页未返回 dailyCounts；可能是接口变更或当前用户不可公开访问")
    })?;
    let mut aggregates = Vec::new();
    if let Some(obj) = daily.as_object() {
        for (raw_day, raw) in obj {
            let count = if let Some(a) = raw.as_array() {
                a.first().and_then(Value::as_i64).unwrap_or(0)
            } else if let Some(o) = raw.as_object() {
                o.get("count")
                    .or_else(|| o.get("value"))
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
            } else {
                raw.as_i64().unwrap_or(0)
            };
            if count <= 0 {
                continue;
            }
            let day = normalize_day(raw_day);
            if day.is_empty() {
                continue;
            }
            aggregates.push(AggregateDay {
                day,
                metric: "activity".into(),
                count,
                note: "洛谷公开个人页 dailyCounts；仅有日期计数，无当天逐题明细".into(),
            });
        }
    }

    let mut solved_count = None;
    let mut difficulty = Vec::new();
    if let Ok(practice_text) = get_text(
        client,
        &format!("https://www.luogu.com.cn/user/{uid}/practice"),
        lentille_headers(),
    )
    .await
    {
        if let Ok(practice) = parse_payload(&practice_text) {
            let pd = practice
                .get("data")
                .or_else(|| practice.get("currentData"))
                .unwrap_or(&practice);
            if let Some(passed) = pd.get("passed").and_then(Value::as_array) {
                solved_count = Some(passed.len() as i64);
                let mut buckets = [0_i64; 8];
                for p in passed {
                    if let Some(d) = p.get("difficulty").and_then(Value::as_i64) {
                        if (0..8).contains(&d) {
                            buckets[d as usize] += 1;
                        }
                    }
                }
                let labels = [
                    "入门",
                    "普及-",
                    "普及/提高-",
                    "普及+/提高",
                    "提高+/省选-",
                    "省选/NOI-",
                    "NOI/NOI+/CTSC",
                    "未知/特殊",
                ];
                for (i, c) in buckets.into_iter().enumerate() {
                    if c > 0 {
                        difficulty.push(DifficultyStat {
                            label: labels[i].into(),
                            count: c,
                            order: i as i64,
                        });
                    }
                }
            }
        }
    }

    Ok(RemoteData {
        platform: "luogu".into(),
        account: display,
        submissions: vec![],
        aggregates,
        solved_count,
        difficulty,
        activity_only: true,
        notes: vec![
            format!("洛谷个人页热度图 · UID {uid}"),
            "record/list 匿名访问容易触发限制，因此 Activity 使用 dailyCounts".into(),
        ],
        cursor_epoch: now_epoch(),
        replace_submissions: true,
        replace_aggregates: true,
    })
}

fn normalize_day(raw: &str) -> String {
    let s = raw.trim().replace('/', "-");
    let parts: Vec<_> = s.split('-').collect();
    if parts.len() != 3 {
        return String::new();
    }
    let y = parts[0].parse::<i32>().ok();
    let m = parts[1].parse::<u32>().ok();
    let d = parts[2].parse::<u32>().ok();
    match (y, m, d) {
        (Some(y), Some(m), Some(d)) if (1..=12).contains(&m) && (1..=31).contains(&d) => {
            format!("{y:04}-{m:02}-{d:02}")
        }
        _ => String::new(),
    }
}
