use serde::{Deserialize, Serialize};
use tauri::State;

use crate::app::state::AppState;
use crate::{db, training};
use training::ProblemSet;

const BASE: &str = "https://raw.githubusercontent.com/Whalica/OJ_Insight-Community/main";
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

fn validate_listing(item: &CommunityListing) -> bool {
    valid_id(&item.id)
        && item.content_type == "problem-set"
        && item.path == format!("content/problem-sets/{}.json", item.id)
        && !item.title.trim().is_empty()
        && item.problem_count > 0
}

async fn download(state: &AppState, url: &str, limit: usize) -> Result<Vec<u8>, String> {
    let response = state.client.get(url).send().await.map_err(|error| format!("社区连接失败：{error}"))?
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
pub(crate) async fn get_community_catalog(state: State<'_, AppState>) -> Result<CommunityCatalog, String> {
    let cache = state.data_dir.join("community-catalog.json");
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
pub(crate) async fn get_community_problem_set(state: State<'_, AppState>, id: String) -> Result<CommunityEntry, String> {
    if !valid_id(&id) { return Err("题单 ID 无效".into()); }
    let url = format!("{BASE}/content/problem-sets/{id}.json");
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
    super::problem_aliases::ensure_problem_aliases(&state, &input.problems).await?;
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
pub(crate) async fn save_community_problem_set(state: State<'_, AppState>, entry: CommunityEntry) -> Result<ProblemSet, String> {
    if entry.schema != "com.ojinsight.community-entry" || entry.schema_version != 1 || entry.content_type != "problem-set" || !valid_id(&entry.id) {
        return Err("社区题单格式不受支持".into());
    }
    let mut input = training::import_problem_set(&entry.content.to_string())?;
    super::problem_aliases::ensure_problem_aliases(&state, &input.problems).await?;
    input.source_set_id = None;
    input.source_url = Some(format!("https://github.com/Whalica/OJ_Insight-Community/blob/main/content/problem-sets/{}.json", entry.id));
    let mut conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::save_problem_set(&mut conn, &input)
}
