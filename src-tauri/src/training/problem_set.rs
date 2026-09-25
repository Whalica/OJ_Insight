use crate::models::PLATFORMS;

use super::{CanonicalProblem, ProblemSet, ProblemSetInput, ProblemSetProblem};

const ROLES: [&str; 6] = ["Warmup", "Stable", "Core", "Weakness", "Observation", "Stretch"];
const TAG_VISIBILITY: [&str; 3] = ["never", "before_solving", "after_ac"];

pub(crate) fn normalize_problem(mut problem: CanonicalProblem) -> Result<CanonicalProblem, String> {
    problem.platform = problem.platform.trim().to_ascii_lowercase();
    problem.problem_key = problem.problem_key.trim().to_string();
    if !PLATFORMS.contains(&problem.platform.as_str()) {
        return Err(format!("不支持的平台：{}", problem.platform));
    }
    if problem.problem_key.is_empty() { return Err("题目标识不能为空；粘贴受支持平台的题目链接可以自动识别".into()); }
    problem.canonical_id = format!("{}:{}", problem.platform, problem.problem_key);
    problem.problem_id = problem.problem_id.trim().to_string();
    problem.name = problem.name.trim().to_string();
    problem.url = problem.url.trim().to_string();
    if !problem.url.is_empty() && !problem.url.starts_with("https://") && !problem.url.starts_with("http://") { return Err("题目链接必须以 http:// 或 https:// 开头".into()); }
    problem.tags = problem.tags.into_iter().map(|tag| tag.trim().to_string()).filter(|tag| !tag.is_empty()).collect();
    problem.tags.sort();
    problem.tags.dedup();
    Ok(problem)
}

pub(crate) fn validate_role(role: &str) -> Result<(), String> {
    if ROLES.contains(&role) { Ok(()) } else { Err(format!("无效的题目角色：{role}")) }
}

pub(crate) fn validate_tag_visibility(value: &str) -> Result<(), String> {
    if TAG_VISIBILITY.contains(&value) { Ok(()) } else { Err("无效的标签可见性".into()) }
}

pub(crate) fn normalize_problem_set(mut input: ProblemSetInput) -> Result<ProblemSetInput, String> {
    input.title = input.title.trim().to_string();
    input.description = input.description.trim().to_string();
    if input.title.is_empty() { return Err("题单名称不能为空".into()); }
    if input.set_type != "static" { return Err("MVP 仅支持 Static Set".into()); }
    validate_tag_visibility(&input.tag_visibility)?;
    if input.problems.is_empty() { return Err("题单至少需要一道题".into()); }
    let mut seen = std::collections::HashSet::new();
    for (index, entry) in input.problems.iter_mut().enumerate() {
        entry.position = index as i64;
        entry.problem = normalize_problem(entry.problem.clone())?;
        validate_role(&entry.role)?;
        if !seen.insert(entry.problem.canonical_id.clone()) { return Err(format!("题单包含重复题目：{}", entry.problem.canonical_id)); }
    }
    Ok(input)
}

pub(crate) fn problem_set_manifest(set: &ProblemSet) -> Result<String, String> {
    let value = serde_json::json!({
        "schema": "com.ojinsight.problem-set",
        "schemaVersion": 1,
        "title": set.title,
        "description": set.description,
        "setType": set.set_type,
        "tagVisibility": set.tag_visibility,
        "sourceSetId": set.source_set_id.or(Some(set.id)),
        "sourceUrl": set.source_url,
        "problems": set.problems,
    });
    serde_json::to_string_pretty(&value).map(|text| format!("{text}\n")).map_err(|e| e.to_string())
}

pub(crate) fn import_problem_set(data: &str) -> Result<ProblemSetInput, String> {
    let value: serde_json::Value = serde_json::from_str(data).map_err(|e| format!("题单 JSON 无效：{e}"))?;
    if value.get("schema").and_then(|v| v.as_str()) != Some("com.ojinsight.problem-set") { return Err("这不是 OJ Insight 题单 JSON".into()); }
    if value.get("schemaVersion").and_then(|v| v.as_i64()) != Some(1) { return Err("不支持的题单 JSON 版本".into()); }
    let input = ProblemSetInput {
        id: None,
        title: value.get("title").and_then(|v| v.as_str()).unwrap_or_default().into(),
        description: value.get("description").and_then(|v| v.as_str()).unwrap_or_default().into(),
        set_type: value.get("setType").and_then(|v| v.as_str()).unwrap_or("static").into(),
        tag_visibility: value.get("tagVisibility").and_then(|v| v.as_str()).unwrap_or("after_ac").into(),
        source_set_id: value.get("sourceSetId").and_then(|v| v.as_i64()),
        source_url: value.get("sourceUrl").and_then(|v| v.as_str()).map(str::to_string),
        problems: serde_json::from_value::<Vec<ProblemSetProblem>>(value.get("problems").cloned().unwrap_or_default()).map_err(|e| format!("题目列表无效：{e}"))?,
    };
    normalize_problem_set(input)
}
