use reqwest::{
    header::{HeaderMap, HeaderValue, ACCEPT, REFERER, USER_AGENT},
    Client,
};
use serde_json::Value;

use super::{get_json, get_text, now_epoch};
use crate::models::{AccountConfig, AggregateDay, DifficultyStat, RatingPoint, RemoteData, SyncError};

fn base_headers() -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(
        USER_AGENT,
        HeaderValue::from_static(concat!("OJ-Insight/", env!("CARGO_PKG_VERSION"), " local analytics")),
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
    let chosen = candidates.iter().find(|u| {
        let name = u
            .get("name")
            .or_else(|| u.get("username"))
            .and_then(Value::as_str)
            .unwrap_or("");
        name.eq_ignore_ascii_case(input)
    });
    let u = chosen.ok_or_else(|| SyncError::error("未找到完全匹配的洛谷用户；请填写数字 UID"))?;
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

fn parse_rating_history(data: &Value) -> Option<Vec<RatingPoint>> {
    let entries = data.get("elo")?.as_array()?;
    let mut history = Vec::new();
    for entry in entries {
        let Some(new_rating) = entry.get("rating").and_then(Value::as_i64) else { continue };
        let Some(mut epoch_second) = entry.get("time").and_then(Value::as_i64) else { continue };
        if epoch_second > 10_000_000_000 { epoch_second /= 1000; }
        if epoch_second <= 0 || new_rating <= 0 { continue; }
        let contest = entry.get("contest");
        let contest_id = contest.and_then(|item| item.get("id")).and_then(|value| value.as_i64().map(|value| value.to_string()).or_else(|| value.as_str().map(str::to_string)))
            .unwrap_or_else(|| format!("rating-{epoch_second}"));
        let contest_name = contest.and_then(|item| item.get("name")).and_then(Value::as_str)
            .filter(|name| !name.trim().is_empty()).unwrap_or("洛谷 Rated 比赛").to_string();
        let change = entry.get("prevDiff").and_then(Value::as_i64).unwrap_or(0);
        history.push(RatingPoint {
            contest_id, contest_name, epoch_second,
            old_rating: (new_rating - change).max(0), new_rating,
            rank: entry.get("rank").and_then(Value::as_i64),
        });
    }
    history.sort_by_key(|point| point.epoch_second);
    Some(history)
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
    let ratings = parse_rating_history(data);
    let mut aggregates = Vec::new();
    let has_daily_counts = data.get("dailyCounts").and_then(Value::as_object).is_some();
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
    let mut solved_inventory = None;
    let mut difficulty = None;
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
                solved_inventory = Some(passed.iter().filter_map(passed_problem_id).collect());
                let mut buckets = [0_i64; 9];
                let mut recognized_difficulty = passed.is_empty();
                for p in passed {
                    if let Some(d) = p.get("difficulty").and_then(Value::as_i64) {
                        if (0..=8).contains(&d) {
                            buckets[d as usize] += 1;
                            recognized_difficulty = true;
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
                let mut stats = Vec::new();
                for (i, c) in buckets.into_iter().enumerate() {
                    if c > 0 {
                        stats.push(DifficultyStat {
                            label: labels[i].into(),
                            count: c,
                            order: i as i64,
                        });
                    }
                }
                if recognized_difficulty { difficulty = Some(stats); }
            }
        }
    }

    if !has_daily_counts && solved_count.is_none() {
        return Err(SyncError::error(
            "洛谷没有返回可用的提交、活动或题目统计数据",
        ));
    }

    let missing_difficulty = difficulty.is_none();
    Ok(RemoteData {
        platform: "luogu".into(),
        account: display,
        display_name: None,
        submissions: Vec::new(),
        aggregates,
        solved_count,
        difficulty,
        knowledge: None,
        solved_inventory,
        ratings,
        activity_only: true,
        notes: {
            let mut notes = vec!["遵循洛谷规则，不请求提交记录；活动数据通常仅覆盖近期，且没有逐题明细".into()];
            if has_daily_counts { notes.push(format!("洛谷个人页 dailyCounts · UID {uid}")); }
            else { notes.push("警告：个人页未返回 dailyCounts，已保留旧活动数据".into()); }
            if missing_difficulty { notes.push("警告：本次未获取到难度分布，已保留旧难度数据".into()); }
            if data.get("elo").is_none() { notes.push("警告：本次未获取到比赛等级分，已保留旧 Rating 数据".into()); }
            notes
        },
        cursor_epoch: now_epoch().saturating_sub(48 * 3600),
        replace_submissions: false,
        // The public calendar may contain only recent days. Merge observed days
        // so older activity remains in the local long-term history.
        replace_aggregates: false,
    })
}

fn passed_problem_id(value: &Value) -> Option<String> {
    let raw = value.as_str().or_else(|| {
        ["pid", "problemId", "id"].iter().find_map(|field| value.get(*field).and_then(Value::as_str))
    })?;
    let id = raw.trim().to_ascii_uppercase();
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        None
    } else {
        Some(id)
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passed_problem_ids_accept_string_and_object_rows() {
        assert_eq!(passed_problem_id(&serde_json::json!("p1421")), Some("P1421".into()));
        assert_eq!(passed_problem_id(&serde_json::json!({"pid":"B2002"})), Some("B2002".into()));
        assert_eq!(passed_problem_id(&serde_json::json!({"pid":"../../x"})), None);
    }

    #[test]
    fn rating_history_uses_contest_elo_not_guzhi() {
        let data = serde_json::json!({
            "gu": {"rating": 135},
            "elo": [
                {"rating": 972, "prevDiff": -54, "time": 1752600000, "contest": {"id": 232, "name": "月赛"}},
                {"rating": 1026, "prevDiff": 26, "time": 1750000000, "contest": {"id": 220, "name": "练习赛"}}
            ]
        });
        let history = parse_rating_history(&data).unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].new_rating, 1026);
        assert_eq!(history[1].old_rating, 1026);
        assert_eq!(history[1].contest_id, "232");
    }
}
