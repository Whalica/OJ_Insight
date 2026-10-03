use std::time::Duration;

use regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};

#[derive(Debug, PartialEq)]
pub(crate) struct NowcoderAliases {
    pub problem_id: String,
    pub keys: Vec<String>,
}

fn numeric_id(value: &str) -> bool {
    !value.is_empty() && !value.starts_with('0') && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn practice_id(value: &str) -> bool {
    value.len() == 32 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

// Build the destination from a validated identity, never from an imported URL.
pub(crate) fn nowcoder_identity_url(key: &str) -> Option<String> {
    if numeric_id(key) {
        Some(format!("https://ac.nowcoder.com/acm/problem/{key}"))
    } else if practice_id(key) {
        Some(format!("https://www.nowcoder.com/practice/{key}"))
    } else if let Some((contest, index)) = key.split_once('/') {
        (numeric_id(contest) && !index.is_empty() && index.bytes().all(|byte| byte.is_ascii_alphanumeric()))
            .then(|| format!("https://ac.nowcoder.com/acm/contest/{contest}/{index}"))
    } else {
        None
    }
}

pub(crate) fn nowcoder_identity_client() -> Result<Client, reqwest::Error> {
    Client::builder()
        .user_agent(concat!("OJ-Insight/", env!("CARGO_PKG_VERSION")))
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(5))
        .connect_timeout(Duration::from_secs(3))
        .build()
}

pub(crate) async fn fetch_nowcoder_aliases(client: &Client, key: &str) -> Option<NowcoderAliases> {
    let url = nowcoder_identity_url(key)?;
    let mut response = client.get(&url).send().await.ok()?;
    if !response.status().is_success() || response.url().as_str() != url {
        return None;
    }
    const MAX_PAGE_BYTES: usize = 1_000_000;
    if response.content_length().is_some_and(|size| size > MAX_PAGE_BYTES as u64) {
        return None;
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.ok()? {
        if bytes.len() + chunk.len() > MAX_PAGE_BYTES {
            return None;
        }
        bytes.extend_from_slice(&chunk);
    }
    parse_nowcoder_aliases(key, std::str::from_utf8(&bytes).ok()?)
}

fn page_info_field(info: &str, field: &str) -> Option<String> {
    // Only standalone properties in the official inline pageInfo object count.
    // Do not confuse questionId (a separate namespace), titles, or statement text
    // with the numeric ACM problemId.
    let pattern = format!(r#"(?m)^\s*{}\s*:\s*['\"]([a-zA-Z0-9]+)['\"]\s*,?\s*(?://[^\r\n]*)?$"#, regex::escape(field));
    let re = Regex::new(&pattern).ok()?;
    let mut values = re.captures_iter(info);
    let value = values.next()?.get(1)?.as_str().to_string();
    if values.next().is_some() { return None; }
    Some(value)
}

fn parse_nowcoder_aliases(key: &str, html: &str) -> Option<NowcoderAliases> {
    nowcoder_identity_url(key)?;
    let doc = Html::parse_document(html);
    let scripts = Selector::parse("script:not([src])").ok()?;
    let assignment = Regex::new(r"(?s)\bwindow\.pageInfo\s*=\s*\{(.*?)\r?\n\s*\};").ok()?;
    for script in doc.select(&scripts) {
        let text = script.text().collect::<String>();
        let Some(found) = assignment.captures(&text) else { continue; };
        let info = found.get(1)?.as_str();
        let problem_id = page_info_field(info, "problemId")?;
        if !numeric_id(&problem_id) { return None; }
        let uuid = page_info_field(info, "uuid").filter(|value| practice_id(value));
        if let Some((contest, _)) = key.split_once('/') {
            if page_info_field(info, "contestId").as_deref() != Some(contest) { return None; }
        } else if numeric_id(key) {
            if key != problem_id { return None; }
        } else if uuid.as_deref() != Some(key) {
            return None;
        }
        let mut keys = vec![problem_id.clone(), key.to_string()];
        if let Some(uuid) = uuid { keys.push(uuid); }
        keys.sort();
        keys.dedup();
        return Some(NowcoderAliases { problem_id, keys });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTEST_PAGE: &str = r#"<html><script>
    window.pageInfo = {
        settingInfo: {"problemOriginal":true},
        contestId: '78807',
        questionId: '10862087',
        problemId: '269161',
        uuid: '6c9ea8c67e2d4feb931dd28727cc80a7',
        isFinished: true
    };
    </script></html>"#;

    #[test]
    fn official_contest_metadata_links_all_problem_identities() {
        let aliases = parse_nowcoder_aliases("78807/D", CONTEST_PAGE).unwrap();
        assert_eq!(aliases.problem_id, "269161");
        assert_eq!(aliases.keys, ["269161", "6c9ea8c67e2d4feb931dd28727cc80a7", "78807/D"]);
        assert!(!aliases.keys.iter().any(|key| key == "10862087"));
    }

    #[test]
    fn practice_and_acm_pages_require_matching_identity() {
        assert!(parse_nowcoder_aliases("269161", CONTEST_PAGE).is_some());
        assert!(parse_nowcoder_aliases("6c9ea8c67e2d4feb931dd28727cc80a7", CONTEST_PAGE).is_some());
        for key in ["269162", "78808/D", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"] {
            assert!(parse_nowcoder_aliases(key, CONTEST_PAGE).is_none());
        }
    }

    #[test]
    fn missing_or_ambiguous_metadata_does_not_invent_an_alias() {
        assert!(parse_nowcoder_aliases("78807/D", &CONTEST_PAGE.replace("problemId", "unrelatedId")).is_none());
        assert!(parse_nowcoder_aliases("78807/D", &CONTEST_PAGE.replace("problemId: '269161',", "problemId: '269161',\nproblemId: '269162',")).is_none());
        assert!(parse_nowcoder_aliases("78807/D", "<h1>Bingbong的奇偶世界</h1><p>problemId: '269161'</p>").is_none());
        assert!(parse_nowcoder_aliases("78807/D", &CONTEST_PAGE.replace("<script>", "<div>").replace("</script>", "</div>")).is_none());
    }

    #[test]
    fn destinations_are_constructed_from_validated_keys() {
        assert_eq!(nowcoder_identity_url("78807/D").as_deref(), Some("https://ac.nowcoder.com/acm/contest/78807/D"));
        assert_eq!(nowcoder_identity_url("269161").as_deref(), Some("https://ac.nowcoder.com/acm/problem/269161"));
        for key in ["https://evilnowcoder.com/acm/problem/1", "https://nowcoder.com.evil/1", "http://ac.nowcoder.com/acm/problem/1", "78807/../D", "78807/D?url=x", "78807/D#x", "0", "0269161", "tracker:269161"] {
            assert!(nowcoder_identity_url(key).is_none(), "{key}");
        }
    }
}
