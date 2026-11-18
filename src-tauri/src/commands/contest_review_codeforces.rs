use std::{collections::HashMap, fmt::Write, time::Duration};

use base64::{engine::general_purpose::STANDARD, Engine};
use ring::{digest, rand::{SecureRandom, SystemRandom}};
use serde::Deserialize;

use super::{element_text, get_text, Html, ReviewContest, ReviewProblem, ReviewSubmission, Selector, Utc, Value};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Credentials {
    api_key: String,
    api_secret: String,
}

const API: &str = "https://codeforces.com/api";
const PAGE_SIZE: usize = 100;
const SCAN_LIMIT: usize = 1000;

async fn request(client: &reqwest::Client, url: &str, endpoint: &str) -> Result<Value, String> {
    let response = client.get(url).send().await
        .map_err(|_| format!("无法连接 Codeforces（{endpoint}）"))?;
    if !response.status().is_success() {
        return Err(format!("Codeforces {endpoint} 请求失败（HTTP {}）", response.status()));
    }
    let payload = response.json::<Value>().await
        .map_err(|_| format!("无法读取 Codeforces {endpoint} 响应"))?;
    if payload["status"] != "OK" || payload["result"].is_null() {
        return Err(format!("Codeforces {endpoint} 未返回数据"));
    }
    Ok(payload["result"].clone())
}

fn signed_url(account: &str, from: usize, credentials: &Credentials) -> Result<String, String> {
    let mut nonce = [0; 3];
    SystemRandom::new().fill(&mut nonce).map_err(|_| "无法生成 Codeforces 签名")?;
    let prefix = hex(&nonce);
    let query = format!(
        "apiKey={}&count={PAGE_SIZE}&from={from}&handle={}&includeSources=true&time={}",
        urlencoding::encode(credentials.api_key.trim()),
        urlencoding::encode(account),
        Utc::now().timestamp()
    );
    let input = format!("{prefix}/user.status?{query}#{}", credentials.api_secret.trim());
    let signature = hex(digest::digest(&digest::SHA512, input.as_bytes()).as_ref());
    Ok(format!("{API}/user.status?{query}&apiSig={prefix}{signature}"))
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn decode_source(item: &Value) -> Option<String> {
    item["sourceBase64"].as_str().filter(|value| !value.is_empty())
        .and_then(|value| STANDARD.decode(value).ok())
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .or_else(|| ["source", "sourceCode"].into_iter()
            .find_map(|key| item[key].as_str().filter(|text| !text.is_empty()).map(str::to_owned)))
}

async fn sources(
    client: &reqwest::Client,
    account: &str,
    submissions: &mut [ReviewSubmission],
    credentials: &Credentials,
) -> Result<(), String> {
    let mut missing: HashMap<_, _> = submissions.iter_mut()
        .map(|submission| (submission.id.clone(), submission)).collect();
    for from in (1..=SCAN_LIMIT).step_by(PAGE_SIZE) {
        if missing.is_empty() {
            break;
        }
        let url = signed_url(account, from, credentials)?;
        let data = request(client, &url, "user.status").await?;
        let items = data.as_array().ok_or("无法读取 Codeforces 源码列表")?;
        for item in items {
            if let Some(id) = item["id"].as_u64() {
                if let Some(submission) = missing.remove(&id.to_string()) {
                    submission.source = decode_source(item);
                }
            }
        }
        if items.len() < PAGE_SIZE {
            break;
        }
        if !missing.is_empty() && from + PAGE_SIZE <= SCAN_LIMIT {
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }
    Ok(())
}

fn submission(item: &Value, contest_id: &str, include_post_contest: bool) -> Option<ReviewSubmission> {
    let role = item["author"]["participantType"].as_str()?;
    let post_contest = role == "PRACTICE";
    if role != "CONTESTANT" && !(post_contest && include_post_contest) {
        return None;
    }
    let id = item["id"].as_u64()?.to_string();
    Some(ReviewSubmission {
        url: format!("https://codeforces.com/contest/{contest_id}/submission/{id}"),
        id,
        problem_id: item["problem"]["index"].as_str()?.to_owned(),
        epoch_second: item["creationTimeSeconds"].as_i64()?,
        relative_seconds: if post_contest { None } else { item["relativeTimeSeconds"].as_i64() },
        language: item["programmingLanguage"].as_str().unwrap_or_default().to_owned(),
        verdict: item["verdict"].as_str().unwrap_or("未知").to_owned(),
        time_ms: item["timeConsumedMillis"].as_i64(),
        memory_bytes: item["memoryConsumedBytes"].as_i64(),
        score: None,
        source: None,
        post_contest,
    })
}

pub(super) async fn load(
    client: &reqwest::Client,
    account: &str,
    contest_id: &str,
    include_post_contest: bool,
    credentials: Option<Credentials>,
) -> Result<ReviewContest, String> {
    let handle = urlencoding::encode(account);
    let standings = request(client, &format!("{API}/contest.standings?contestId={contest_id}"), "contest.standings").await?;
    let contest = &standings["contest"];
    let name = contest["name"].as_str().ok_or("Codeforces 未返回比赛名称")?;
    let url = format!("https://codeforces.com/contest/{contest_id}");
    let row = standings["rows"].as_array().into_iter().flatten().find(|row| {
        row["party"]["members"].as_array().into_iter().flatten().any(|member| {
            member["handle"].as_str().is_some_and(|handle| handle.eq_ignore_ascii_case(account))
        })
    });
    let data = request(client, &format!("{API}/contest.status?contestId={contest_id}&handle={handle}"), "contest.status").await?;
    let mut submissions: Vec<_> = data.as_array().ok_or("Codeforces 未返回提交列表")?
        .iter().filter_map(|item| submission(item, contest_id, include_post_contest)).collect();
    submissions.sort_by_key(|item| item.epoch_second);

    let mut notes = Vec::new();
    if submissions.is_empty() {
        notes.push("该账号在所选范围内没有可见提交。".into());
    } else {
        match credentials {
            Some(credentials) if !credentials.api_key.trim().is_empty() && !credentials.api_secret.trim().is_empty() => {
                if let Err(error) = sources(client, account, &mut submissions, &credentials).await {
                    notes.push(format!("{error}。请检查 API Key、Secret 和本机时间。"));
                } else if submissions.iter().any(|item| item.source.is_none()) {
                    notes.push(format!("部分源码不可用，已最多扫描最近 {SCAN_LIMIT} 次提交。"));
                }
            }
            _ => notes.push("所选账号未保存完整的 API Key 和 Secret。可通过「前往配置」补齐后获取源码。".into()),
        }
    }

    let mut problems = Vec::new();
    for item in standings["problems"].as_array().ok_or("Codeforces 未返回题目列表")? {
        let id = item["index"].as_str().ok_or("Codeforces 题目缺少编号")?;
        let problem_url = format!("{url}/problem/{}", urlencoding::encode(id));
        let mut problem = ReviewProblem {
            id: id.into(),
            name: item["name"].as_str().unwrap_or(id).into(),
            url: problem_url.clone(),
            statement: String::new(),
            time_limit: String::new(),
            memory_limit: String::new(),
        };
        if let Ok(html) = get_text(client, &problem_url, "").await {
            let document = Html::parse_document(&html);
            let extract = |selector: &str| {
                document.select(&Selector::parse(selector).unwrap()).next()
                    .map(element_text).unwrap_or_default()
            };
            problem.statement = extract(".problem-statement");
            problem.time_limit = extract(".problem-statement .time-limit");
            problem.memory_limit = extract(".problem-statement .memory-limit");
        }
        problems.push(problem);
    }
    if problems.iter().any(|problem| problem.statement.is_empty()) {
        notes.push("部分题面未能获取，复盘包保留了原题链接。".into());
    }
    Ok(ReviewContest {
        platform: "codeforces".into(),
        id: contest_id.into(),
        name: name.into(),
        url,
        account: account.into(),
        start_epoch: contest["startTimeSeconds"].as_i64(),
        duration_seconds: contest["durationSeconds"].as_i64(),
        rank: row.and_then(|row| row["rank"].as_i64()),
        score: row.and_then(|row| row["points"].as_f64()),
        penalty: row.and_then(|row| row["penalty"].as_i64()),
        problems,
        submissions,
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn decodes_exporter_source_fields() {
        assert_eq!(decode_source(&json!({"sourceBase64": "YSAmIGI="})).as_deref(), Some("a & b"));
        assert_eq!(decode_source(&json!({"sourceBase64": "!", "sourceCode": "return 0;"})).as_deref(), Some("return 0;"));
        assert_eq!(decode_source(&json!({})), None);
    }

    #[test]
    fn includes_failed_official_submissions_and_keeps_practice_separate() {
        let mut item = json!({
            "id": 123,
            "author": {"participantType": "CONTESTANT"},
            "problem": {"index": "B"},
            "creationTimeSeconds": 1000,
            "relativeTimeSeconds": 120,
            "verdict": "WRONG_ANSWER"
        });
        let official = submission(&item, "42", false).unwrap();
        assert_eq!(official.verdict, "WRONG_ANSWER");
        assert_eq!(official.relative_seconds, Some(120));
        item["author"]["participantType"] = json!("PRACTICE");
        assert!(submission(&item, "42", false).is_none());
        assert!(submission(&item, "42", true).unwrap().post_contest);
        item["author"]["participantType"] = json!("VIRTUAL");
        assert!(submission(&item, "42", true).is_none());
    }
}
