use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::path::Path;

use reqwest::Client;
use rusqlite::Connection;
use serde_json::Value;

use super::{CandidatePool, CandidateSourceStatus, CanonicalProblem, ProblemSetProblem};

pub(crate) fn filter_candidates(conn: &Connection, candidates: Vec<CanonicalProblem>) -> Result<Vec<CanonicalProblem>, String> {
    filter_candidates_with_count(conn, candidates).map(|value| value.0)
}

fn filter_candidates_with_count(conn: &Connection, candidates: Vec<CanonicalProblem>) -> Result<(Vec<CanonicalProblem>, usize), String> {
    let mut out = Vec::new();
    let mut excluded_solved = 0;
    let mut seen = HashSet::new();
    let mut solved = HashSet::new();
    let mut statement = conn.prepare("SELECT platform,problem_key FROM submissions UNION SELECT platform,problem_key FROM solved_inventory").map_err(|error| error.to_string())?;
    for row in statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).map_err(|error| error.to_string())? {
        solved.insert(row.map_err(|error| error.to_string())?);
    }
    for candidate in candidates {
        let problem = super::problem_set::normalize_problem(candidate)?;
        let unsuitable_tag = problem.tags.iter().any(|tag| tag.eq_ignore_ascii_case("interactive") || tag.eq_ignore_ascii_case("*special"));
        if !seen.insert(problem.canonical_id.clone()) || problem.interactive || problem.output_only || unsuitable_tag
            || problem.training_suitability.is_some_and(|score| score < 0.35)
            || problem.observation_dependency.is_some_and(|score| score > 0.8) { continue; }
        if solved.contains(&(problem.platform.clone(), problem.problem_key.clone())) {
            excluded_solved += 1;
        } else {
            out.push(problem);
        }
    }
    Ok((out, excluded_solved))
}

pub(crate) fn filter_problem_entries(conn: &Connection, entries: Vec<ProblemSetProblem>) -> Result<Vec<ProblemSetProblem>, String> {
    let mut out = Vec::new();
    for mut entry in entries {
        let filtered = filter_candidates(conn, vec![entry.problem])?;
        if let Some(problem) = filtered.into_iter().next() {
            entry.problem = problem;
            entry.position = out.len() as i64;
            out.push(entry);
        }
    }
    Ok(out)
}

pub(crate) async fn build_candidate_pool(
    client: &Client,
    xcpc_cache_path: &Path,
    qoj_cookie: &str,
    platforms: &[String],
    mode: &str,
    requested_count: usize,
) -> Result<CandidatePool, String> {
    super::template::target_solve_rate(mode)?;
    let requested_count = requested_count.clamp(12, 120);
    let selected: HashSet<&str> = platforms.iter().map(String::as_str).collect();
    let mut all = Vec::new();
    let mut sources = Vec::new();

    if selected.contains("codeforces") {
        match fetch_codeforces(client).await {
            Ok(items) => { sources.push(source_ok("codeforces", items.len())); all.extend(items); }
            Err(message) => sources.push(source_error("codeforces", message)),
        }
    }
    if selected.contains("atcoder") {
        match fetch_atcoder(client).await {
            Ok(items) => { sources.push(source_ok("atcoder", items.len())); all.extend(items); }
            Err(message) => sources.push(source_error("atcoder", message)),
        }
    }
    if selected.contains("qoj") {
        match fetch_qoj(client, xcpc_cache_path, qoj_cookie).await {
            Ok(items) => { sources.push(source_ok("qoj", items.len())); all.extend(items); }
            Err(message) => sources.push(source_error("qoj", message)),
        }
    }
    for platform in ["luogu", "nowcoder", "leetcode"] {
        if selected.contains(platform) {
            sources.push(CandidateSourceStatus { platform: platform.into(), available: false, problem_count: 0, message: "暂时没有足够可靠的完整题目目录，未参与自动筛题".into() });
        }
    }
    if all.is_empty() { return Err("所选平台当前没有可用的可靠候选目录".into()); }

    Ok(CandidatePool { generated_at: chrono::Utc::now().timestamp(), mode: mode.into(), requested_count, excluded_solved: 0, selection_basis: String::new(), candidates: all, sources })
}

pub(crate) fn finalize_candidate_pool(conn: &Connection, mut pool: CandidatePool) -> Result<CandidatePool, String> {
    let (candidates, excluded_solved) = filter_candidates_with_count(conn, pool.candidates)?;
    let targets = profile_targets(conn, &pool.mode)?;
    let mut groups: BTreeMap<String, Vec<CanonicalProblem>> = BTreeMap::new();
    for problem in candidates { groups.entry(problem.platform.clone()).or_default().push(problem); }
    let mut groups: BTreeMap<String, VecDeque<CanonicalProblem>> = groups.into_iter().map(|(platform, mut problems)| {
        let target = targets.get(&platform).copied();
        problems.sort_by_key(|problem| {
            let distance = match (target, problem.difficulty.as_deref().and_then(|value| value.parse::<f64>().ok())) {
                (Some(center), Some(level)) if level.is_finite() => ((level - center).abs() * 10.0) as u64,
                (Some(_), _) => u64::MAX / 4,
                _ => 0,
            };
            (distance, stable_order(&problem.canonical_id))
        });
        (platform, problems.into())
    }).collect();
    let mut candidates = Vec::new();
    while candidates.len() < pool.requested_count {
        let mut added = false;
        for group in groups.values_mut() {
            if let Some(problem) = group.pop_front() { candidates.push(problem); added = true; }
            if candidates.len() == pool.requested_count { break; }
        }
        if !added { break; }
    }
    if candidates.is_empty() { return Err("候选目录中的题目均已完成或不适合本次训练".into()); }
    pool.selection_basis = if targets.is_empty() { "尚无足够的分平台难度记录；按平台均衡取样，已排除已做题。".into() } else { format!("根据 {} 个平台的已通过题目难度估计训练区间，并保持跨平台覆盖；无难度记录的平台使用固定顺序取样。", targets.len()) };
    pool.candidates = candidates;
    pool.excluded_solved = excluded_solved;
    Ok(pool)
}

fn profile_targets(conn: &Connection, mode: &str) -> Result<HashMap<String, f64>, String> {
    let mut statement = conn.prepare("SELECT platform,difficulty FROM submissions WHERE difficulty IS NOT NULL AND difficulty<>'' GROUP BY platform,problem_key").map_err(|error| error.to_string())?;
    let mut values: HashMap<String, Vec<f64>> = HashMap::new();
    for row in statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).map_err(|error| error.to_string())? {
        let (platform, difficulty) = row.map_err(|error| error.to_string())?;
        if let Ok(value) = difficulty.parse::<f64>() { if value.is_finite() && value >= 0.0 { values.entry(platform).or_default().push(value); } }
    }
    let percentile = match mode { "relaxed" => 0.40, "pressure" => 0.75, _ => 0.60 };
    Ok(values.into_iter().filter_map(|(platform, mut levels)| {
        if levels.len() < 8 { return None; }
        levels.sort_by(|left, right| left.total_cmp(right));
        let index = ((levels.len() - 1) as f64 * percentile).round() as usize;
        Some((platform, levels[index]))
    }).collect())
}

fn source_ok(platform: &str, count: usize) -> CandidateSourceStatus {
    CandidateSourceStatus { platform: platform.into(), available: true, problem_count: count, message: "可靠题目目录已加载".into() }
}

fn source_error(platform: &str, message: String) -> CandidateSourceStatus {
    CandidateSourceStatus { platform: platform.into(), available: false, problem_count: 0, message }
}

fn stable_order(value: &str) -> u64 {
    value.bytes().fold(1_469_598_103_934_665_603, |hash, byte| (hash ^ u64::from(byte)).wrapping_mul(1_099_511_628_211))
}

async fn fetch_codeforces(client: &Client) -> Result<Vec<CanonicalProblem>, String> {
    let value = client.get("https://codeforces.com/api/problemset.problems").send().await
        .map_err(|e| format!("Codeforces 题目目录请求失败：{e}"))?.error_for_status()
        .map_err(|e| format!("Codeforces 题目目录：{e}"))?.json::<Value>().await
        .map_err(|e| format!("Codeforces 题目目录解析失败：{e}"))?;
    if value.get("status").and_then(Value::as_str) != Some("OK") { return Err("Codeforces 题目目录返回异常".into()); }
    let rows = value.pointer("/result/problems").and_then(Value::as_array).ok_or_else(|| "Codeforces 题目目录格式异常".to_string())?;
    Ok(rows.iter().filter_map(|item| {
        let contest_id = item.get("contestId")?.as_i64()?;
        let index = item.get("index")?.as_str()?;
        let tags: Vec<String> = item.get("tags").and_then(Value::as_array)
            .map(|items| items.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default();
        Some(CanonicalProblem {
            canonical_id: String::new(), platform: "codeforces".into(), problem_key: format!("{contest_id}:{index}"),
            problem_id: format!("{contest_id}{index}"), name: item.get("name").and_then(Value::as_str).unwrap_or(index).into(),
            url: if contest_id >= 100_000 { format!("https://codeforces.com/gym/{contest_id}/problem/{index}") } else { format!("https://codeforces.com/contest/{contest_id}/problem/{index}") },
            difficulty: item.get("rating").and_then(Value::as_i64).map(|value| value.to_string()),
            interactive: tags.iter().any(|tag| tag.eq_ignore_ascii_case("interactive")),
            output_only: item.get("type").and_then(Value::as_str) == Some("OUTPUT_ONLY"), tags,
            training_suitability: None, observation_dependency: None, implementation_load: None, knowledge_dependency: None,
        })
    }).collect())
}

async fn fetch_atcoder(client: &Client) -> Result<Vec<CanonicalProblem>, String> {
    let problems = client.get("https://kenkoooo.com/atcoder/resources/problems.json").send().await
        .map_err(|e| format!("AtCoder 题目目录请求失败：{e}"))?.error_for_status()
        .map_err(|e| format!("AtCoder 题目目录：{e}"))?.json::<Value>().await
        .map_err(|e| format!("AtCoder 题目目录解析失败：{e}"))?;
    let models = match client.get("https://kenkoooo.com/atcoder/resources/problem-models.json").send().await {
        Ok(response) => match response.error_for_status() { Ok(response) => response.json::<Value>().await.unwrap_or(Value::Null), Err(_) => Value::Null },
        Err(_) => Value::Null,
    };
    let rows = problems.as_array().ok_or_else(|| "AtCoder 题目目录格式异常".to_string())?;
    Ok(rows.iter().filter_map(|item| {
        let id = item.get("id")?.as_str()?;
        let contest_id = item.get("contest_id")?.as_str()?;
        let difficulty = models.get(id).and_then(|model| model.get("difficulty")).and_then(Value::as_f64).map(|value| value.round().to_string());
        Some(CanonicalProblem {
            canonical_id: String::new(), platform: "atcoder".into(), problem_key: id.into(), problem_id: id.into(),
            name: item.get("name").and_then(Value::as_str).unwrap_or(id).into(), url: format!("https://atcoder.jp/contests/{contest_id}/tasks/{id}"), difficulty,
            tags: Vec::new(), training_suitability: None, observation_dependency: None, implementation_load: None, knowledge_dependency: None,
            interactive: false, output_only: false,
        })
    }).collect())
}

async fn fetch_qoj(client: &Client, cache_path: &Path, cookie: &str) -> Result<Vec<CanonicalProblem>, String> {
    let contests = crate::xcpc::load_catalog(client, cache_path, cookie, false).await?;
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for contest in contests {
        for problem in contest.problems {
            if problem.problem_id.is_empty() || !seen.insert(problem.problem_id.clone()) { continue; }
            out.push(CanonicalProblem {
                canonical_id: String::new(), platform: "qoj".into(), problem_key: problem.problem_id.clone(), problem_id: problem.problem_id.clone(),
                name: problem.name, url: if problem.url.is_empty() { format!("https://qoj.ac/problem/{}", problem.problem_id) } else { problem.url },
                difficulty: problem.tier, tags: problem.tags, training_suitability: None, observation_dependency: None,
                implementation_load: None, knowledge_dependency: None, interactive: false, output_only: false,
            });
        }
    }
    Ok(out)
}
