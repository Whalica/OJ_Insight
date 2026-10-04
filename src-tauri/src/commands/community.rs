use serde::{Deserialize, Serialize};
use tauri::State;

use crate::app::state::AppState;
use crate::{db, training};
use training::ProblemSet;

const BASE: &str = "https://raw.githubusercontent.com/Whalica/OJ_Insight-Community/main";
const FRESH_CATALOG: &str = "https://api.github.com/repos/Whalica/OJ_Insight-Community/contents/catalog.json?ref=main";
const MAX_CATALOG_BYTES: usize = 1_000_000;
const MAX_ENTRY_BYTES: usize = 512_000;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommunityAuthor {
    pub name: String,
    pub url: Option<String>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommunityListing {
    pub id: String,
    #[serde(rename = "type")]
    pub content_type: String,
    pub title: String,
    pub summary: String,
    pub author: CommunityAuthor,
    pub categories: Vec<String>,
    pub license: String,
    pub path: String,
    pub problem_count: usize,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommunityCatalog {
    pub schema: String,
    pub schema_version: u32,
    pub entries: Vec<CommunityListing>,
    #[serde(default, skip_deserializing)]
    pub cached: bool,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommunityEntry {
    pub schema: String,
    pub schema_version: u32,
    pub id: String,
    #[serde(rename = "type")]
    pub content_type: String,
    pub title: String,
    pub summary: String,
    pub author: CommunityAuthor,
    pub categories: Vec<String>,
    pub license: String,
    pub source_url: Option<String>,
    pub content: serde_json::Value,
    #[serde(default, skip_deserializing)]
    pub solved_keys: Vec<String>,
    #[serde(default, skip_deserializing)]
    pub cached: bool,
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 100 && id.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn valid_path(path: &str, id: &str) -> bool {
    let Some(relative) = path.strip_prefix("content/problem-sets/") else { return false; };
    let parts: Vec<_> = relative.split('/').collect();
    path.len() <= 512
        && parts.len() <= 12
        && parts.last().is_some_and(|file| *file == format!("{id}.json"))
        && parts[..parts.len() - 1].iter().all(|folder| {
            !folder.is_empty() && folder.len() <= 80 && folder.trim() == *folder
                && *folder != "." && *folder != ".."
                && folder.chars().all(|ch| ch.is_alphanumeric() || matches!(ch, ' ' | '_' | '.' | '-'))
        })
}

fn content_url(path: &str, github: bool) -> String {
    let base = if github { "https://github.com/Whalica/OJ_Insight-Community/blob/main" } else { BASE };
    let mut url = reqwest::Url::parse(base).expect("static community base URL");
    url.path_segments_mut().expect("HTTP URL has path segments").extend(path.split('/'));
    url.to_string()
}

fn validate_listing(item: &CommunityListing) -> bool {
    valid_id(&item.id)
        && item.content_type == "problem-set"
        && valid_path(&item.path, &item.id)
        && !item.title.trim().is_empty()
        && item.problem_count > 0
}

async fn download(state: &AppState, url: &str, limit: usize) -> Result<Vec<u8>, String> {
    let mut request = state.client.get(url);
    if url == FRESH_CATALOG {
        request = request.header(reqwest::header::ACCEPT, "application/vnd.github.raw+json");
    }
    let response = request.send().await.map_err(|error| format!("社区连接失败：{error}"))?
        .error_for_status().map_err(|error| format!("社区文件请求失败：{error}"))?;
    if response.content_length().is_some_and(|size| size > limit as u64) { return Err("社区文件过大".into()); }
    let bytes = response.bytes().await.map_err(|error| format!("读取社区文件失败：{error}"))?;
    if bytes.len() > limit { return Err("社区文件过大".into()); }
    Ok(bytes.to_vec())
}

fn parse_catalog(data: &[u8]) -> Result<CommunityCatalog, String> {
    let catalog: CommunityCatalog = serde_json::from_slice(data).map_err(|error| format!("社区目录无效：{error}"))?;
    if catalog.schema != "com.ojinsight.community-catalog" || catalog.schema_version != 1 || catalog.entries.len() > 5000 || catalog.entries.iter().any(|item| !validate_listing(item)) {
        return Err("社区目录格式不受支持".into());
    }
    Ok(catalog)
}

fn parse_entry(data: &[u8], id: &str) -> Result<CommunityEntry, String> {
    let entry: CommunityEntry = serde_json::from_slice(data).map_err(|error| format!("社区题单无效：{error}"))?;
    if entry.schema != "com.ojinsight.community-entry" || entry.schema_version != 1 || entry.id != id || entry.content_type != "problem-set" || entry.title.trim().is_empty() {
        return Err("社区题单格式不受支持".into());
    }
    training::import_problem_set(&entry.content.to_string())?;
    Ok(entry)
}

#[tauri::command]
pub(crate) async fn get_community_catalog(state: State<'_, AppState>, fresh: bool) -> Result<CommunityCatalog, String> {
    let cache = state.data_dir.join("community-catalog.json");
    if fresh {
        if let Ok(data) = download(&state, FRESH_CATALOG, MAX_CATALOG_BYTES).await {
            if let Ok(catalog) = parse_catalog(&data) {
                let _ = std::fs::write(&cache, data);
                return Ok(catalog);
            }
        }
    }
    if let Ok(data) = download(&state, &format!("{BASE}/catalog.json"), MAX_CATALOG_BYTES).await {
        if let Ok(catalog) = parse_catalog(&data) {
            let _ = std::fs::write(&cache, data);
            return Ok(catalog);
        }
    }
    let data = std::fs::read(&cache).map_err(|_| "社区目录不可用，且没有可用的本地缓存".to_string())?;
    let mut catalog = parse_catalog(&data)?;
    catalog.cached = true;
    Ok(catalog)
}

#[tauri::command]
pub(crate) async fn get_community_problem_set(state: State<'_, AppState>, id: String, path: String) -> Result<CommunityEntry, String> {
    if !valid_id(&id) || !valid_path(&path, &id) { return Err("题单路径无效".into()); }
    let url = content_url(&path, false);
    let cache_dir = state.data_dir.join("community-entries");
    let cache = cache_dir.join(format!("{id}.json"));
    let (mut entry, cache_fallback) = if let Ok(data) = download(&state, &url, MAX_ENTRY_BYTES).await {
        if let Ok(entry) = parse_entry(&data, &id) {
            let _ = std::fs::create_dir_all(&cache_dir);
            let _ = std::fs::write(&cache, data);
            (entry, false)
        } else {
            let cached = std::fs::read(&cache).map_err(|_| "社区题单无效，且没有可用的本地缓存".to_string())?;
            (parse_entry(&cached, &id)?, true)
        }
    } else {
        let cached = std::fs::read(&cache).map_err(|_| "社区题单不可用，且没有可用的本地缓存".to_string())?;
        (parse_entry(&cached, &id)?, true)
    };
    if cache_fallback {
        entry.cached = true;
    }
    let input = training::import_problem_set(&entry.content.to_string())?;
    {
        let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
        for row in &input.problems {
            if db::is_problem_solved(&conn, &row.problem.platform, &row.problem.problem_key)? {
                entry.solved_keys.push(format!("{}:{}", row.problem.platform, row.problem.problem_key));
            }
        }
    }
    Ok(entry)
}

#[tauri::command]
pub(crate) fn save_community_problem_set(state: State<'_, AppState>, entry: CommunityEntry, path: String) -> Result<ProblemSet, String> {
    if entry.schema != "com.ojinsight.community-entry" || entry.schema_version != 1 || entry.content_type != "problem-set" || !valid_id(&entry.id) || !valid_path(&path, &entry.id) {
        return Err("社区题单格式不受支持".into());
    }
    let mut input = training::import_problem_set(&entry.content.to_string())?;
    input.source_set_id = None;
    input.source_url = Some(content_url(&path, true));
    let mut conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::save_problem_set(&mut conn, &input)
}

#[cfg(test)]
mod tests {
    use super::valid_path;

    #[test]
    fn accepts_nested_community_sets_and_rejects_path_escape() {
        assert!(valid_path("content/problem-sets/example.json", "example"));
        assert!(valid_path("content/problem-sets/算法基础/图论/example.json", "example"));
        assert!(!valid_path("content/problem-sets/../example.json", "example"));
        assert!(!valid_path("content/problem-sets/%2e%2e/example.json", "example"));
        assert!(!valid_path("content/problem-sets/算法基础/other.json", "example"));
    }
}
