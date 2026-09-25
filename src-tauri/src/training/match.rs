use rusqlite::Connection;

use super::{MatchManifest, ProblemSetProblem, TrainingMatch};

pub(crate) fn start_match(conn: &mut Connection, set_id: i64, mode: &str, duration_minutes: i64, target_min: Option<f64>, target_max: Option<f64>) -> Result<TrainingMatch, String> {
    let set = crate::db::get_problem_set(conn, set_id)?;
    let unsolved = super::filter_problem_entries(conn, set.problems)?;
    if unsolved.is_empty() { return Err("题单中没有可开始的未做题".into()); }
    let defaults = super::template::target_solve_rate(mode)?;
    let min = target_min.unwrap_or(defaults.0).clamp(0.0, 1.0);
    let max = target_max.unwrap_or(defaults.1).clamp(min, 1.0);
    crate::db::create_training_match(conn, Some(set_id), &set.title, mode, &set.tag_visibility, min, max, duration_minutes.clamp(15, 480), &unsolved)
}

pub(crate) fn import_match_manifest(conn: &mut Connection, data: &str) -> Result<TrainingMatch, String> {
    let mut manifest: MatchManifest = serde_json::from_str(data).map_err(|e| format!("Match Manifest JSON 无效：{e}"))?;
    if manifest.schema != "com.ojinsight.match-manifest" || manifest.schema_version != 1 { return Err("不支持的 Match Manifest schema".into()); }
    if manifest.title.trim().is_empty() { return Err("Match 名称不能为空".into()); }
    super::problem_set::validate_tag_visibility(&manifest.tag_visibility)?;
    let defaults = super::template::target_solve_rate(&manifest.mode)?;
    let min = manifest.target_solve_rate_min.unwrap_or(defaults.0).clamp(0.0, 1.0);
    let max = manifest.target_solve_rate_max.unwrap_or(defaults.1).clamp(min, 1.0);
    let mut problems = Vec::<ProblemSetProblem>::new();
    let mut seen = std::collections::HashSet::new();
    for (index, mut entry) in manifest.problems.drain(..).enumerate() {
        entry.position = index as i64;
        entry.problem = super::problem_set::normalize_problem(entry.problem)?;
        super::problem_set::validate_role(&entry.role)?;
        if !seen.insert(entry.problem.canonical_id.clone()) { return Err(format!("Manifest 包含重复题目：{}", entry.problem.canonical_id)); }
        if crate::db::is_problem_solved(conn, &entry.problem.platform, &entry.problem.problem_key)? { continue; }
        problems.push(entry);
    }
    let problems = super::filter_problem_entries(conn, problems)?;
    if problems.is_empty() { return Err("Manifest 中没有可用的未做题".into()); }
    crate::db::create_training_match(conn, None, manifest.title.trim(), &manifest.mode, &manifest.tag_visibility, min, max, manifest.duration_minutes.clamp(15, 480), &problems)
}
