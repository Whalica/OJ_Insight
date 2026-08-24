use reqwest::{
    header::{HeaderMap, HeaderValue, ACCEPT, REFERER, USER_AGENT},
    Client,
};
use serde_json::Value;
use std::collections::HashMap;

use super::{get_json, get_text, now_epoch, polite_sleep};
use crate::models::{
    AccountConfig, AggregateDay, DifficultyStat, RemoteData, Submission, SyncError,
};

fn base_headers() -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(
        USER_AGENT,
        HeaderValue::from_static("OJ-Insight/0.4 local analytics"),
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
    full: bool,
    cursor: i64,
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
    let mut aggregates = Vec::new();
    if let Some(obj) = data.get("dailyCounts").and_then(Value::as_object) {
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
                epoch_second: None,
                metric: "activity".into(),
                count,
                note: "洛谷公开个人页 dailyCounts；仅有日期计数，无当天逐题明细".into(),
            });
        }
    }

    let mut solved_count = None;
    let mut difficulty = Vec::new();
    let mut problem_difficulties = HashMap::new();
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
                let mut buckets = [0_i64; 9];
                for p in passed {
                    if let Some(d) = p.get("difficulty").and_then(Value::as_i64) {
                        if (1..=8).contains(&d) {
                            buckets[d as usize] += 1;
                            if let Some(pid) = p.get("pid").and_then(Value::as_str) {
                                if let Some(label) = luogu_difficulty(d) {
                                    problem_difficulties.insert(pid.to_string(), label);
                                }
                            }
                        }
                    }
                }
                let labels = [
                    "未评定",
                    "入门",
                    "普及-",
                    "普及",
                    "普及+/提高-",
                    "提高",
                    "提高+/省选-",
                    "省选/NOI-",
                    "NOI/NOI+/CTS",
                ];
                for (i, c) in buckets.into_iter().enumerate().skip(1) {
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

    let records = fetch_records(
        client,
        &uid,
        input,
        full,
        cursor,
        &problem_difficulties,
    )
    .await;
    let (submissions, record_note, record_available) = match records {
        Ok(items) => (
            items,
            "洛谷公开提交记录 · 使用原始 submitTime".to_string(),
            true,
        ),
        Err(error) => (
            Vec::new(),
            format!(
                "警告：提交记录暂不可用，已回退活动计数（{}）",
                error.message
            ),
            false,
        ),
    };
    let activity_only = !record_available;
    if activity_only && aggregates.is_empty() {
        return Err(SyncError::error("洛谷没有返回可用的提交记录或逐日活动数据"));
    }

    Ok(RemoteData {
        platform: "luogu".into(),
        account: display,
        submissions,
        aggregates,
        solved_count,
        difficulty,
        activity_only,
        notes: vec![format!("洛谷个人页热度图 · UID {uid}"), record_note],
        cursor_epoch: now_epoch().saturating_sub(48 * 3600),
        replace_submissions: full,
        replace_aggregates: true,
    })
}

async fn fetch_records(
    client: &Client,
    uid: &str,
    account: &str,
    full: bool,
    cursor: i64,
    known_difficulties: &HashMap<String, String>,
) -> Result<Vec<Submission>, SyncError> {
    let cutoff = if full {
        0
    } else {
        cursor.saturating_sub(48 * 3600)
    };
    let mut out = Vec::new();
    for page in 1..=5000 {
        let url = format!(
            "https://www.luogu.com.cn/record/list?user={}&status=12&orderBy=0&page={page}",
            urlencoding::encode(uid)
        );
        let text = get_text(client, &url, lentille_headers()).await?;
        let payload = parse_payload(&text)?;
        let data = payload
            .get("data")
            .or_else(|| payload.get("currentData"))
            .unwrap_or(&payload);
        let records = data
            .pointer("/records/result")
            .or_else(|| data.pointer("/records/results"))
            .or_else(|| data.get("records"))
            .and_then(|value| {
                value
                    .as_array()
                    .or_else(|| value.get("result").and_then(Value::as_array))
            })
            .ok_or_else(|| SyncError::error("record/list 未返回记录列表"))?;
        if records.is_empty() {
            break;
        }
        let mut reached_old = false;
        for record in records {
            let ts = record
                .get("submitTime")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            if ts <= 0 {
                continue;
            }
            if !full && ts <= cutoff {
                reached_old = true;
                continue;
            }
            let problem = record.get("problem").unwrap_or(&Value::Null);
            let pid = problem.get("pid").and_then(Value::as_str).unwrap_or("");
            if pid.is_empty() {
                continue;
            }
            let difficulty = problem
                .get("difficulty")
                .and_then(Value::as_i64)
                .and_then(luogu_difficulty)
                .or_else(|| known_difficulties.get(pid).cloned());
            let id = record
                .get("id")
                .and_then(Value::as_i64)
                .map(|x| x.to_string())
                .unwrap_or_else(|| format!("{uid}-{ts}-{pid}"));
            out.push(Submission {
                platform: "luogu".into(),
                account: account.into(),
                source: "oj".into(),
                source_day: None,
                submission_id: id,
                problem_key: pid.into(),
                problem_id: pid.into(),
                problem_name: problem
                    .get("title")
                    .and_then(Value::as_str)
                    .unwrap_or(pid)
                    .into(),
                problem_url: format!("https://www.luogu.com.cn/problem/{pid}"),
                epoch_second: ts,
                language: record
                    .get("language")
                    .and_then(Value::as_i64)
                    .map(|x| x.to_string())
                    .unwrap_or_default(),
                difficulty,
            });
        }
        if reached_old {
            break;
        }
        let count = data
            .pointer("/records/count")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let per_page = data
            .pointer("/records/perPage")
            .and_then(Value::as_i64)
            .unwrap_or(records.len() as i64);
        if per_page <= 0 || page as i64 * per_page >= count {
            break;
        }
        polite_sleep(320).await;
    }
    Ok(out)
}

fn luogu_difficulty(value: i64) -> Option<String> {
    Some(
        match value {
            1 => "入门",
            2 => "普及-",
            3 => "普及",
            4 => "普及+/提高-",
            5 => "提高",
            6 => "提高+/省选-",
            7 => "省选/NOI-",
            8 => "NOI/NOI+/CTS",
            _ => return None,
        }
        .into(),
    )
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
