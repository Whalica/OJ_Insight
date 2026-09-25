use tauri::State;

use crate::app::state::AppState;
use crate::{db, training};
use training::{CandidatePool, CanonicalProblem, ProblemSet, ProblemSetInput, TrainingMatch};

#[tauri::command]
pub(crate) fn list_problem_sets(state: State<'_, AppState>) -> Result<Vec<ProblemSet>, String> {
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::list_problem_sets(&conn)
}

#[tauri::command]
pub(crate) fn save_problem_set(state: State<'_, AppState>, input: ProblemSetInput) -> Result<ProblemSet, String> {
    let input = training::normalize_problem_set(input)?;
    let mut conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::save_problem_set(&mut conn, &input)
}

#[tauri::command]
pub(crate) fn delete_problem_set(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::delete_problem_set(&conn, id)
}

#[tauri::command]
pub(crate) fn export_problem_set(state: State<'_, AppState>, id: i64) -> Result<String, String> {
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    training::problem_set_manifest(&db::get_problem_set(&conn, id)?)
}

#[tauri::command]
pub(crate) fn import_problem_set(state: State<'_, AppState>, data: String) -> Result<ProblemSet, String> {
    let input = training::import_problem_set(&data)?;
    let mut conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::save_problem_set(&mut conn, &input)
}

#[tauri::command]
pub(crate) fn filter_training_candidates(state: State<'_, AppState>, candidates: Vec<CanonicalProblem>) -> Result<Vec<CanonicalProblem>, String> {
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    training::filter_candidates(&conn, candidates)
}

#[tauri::command]
pub(crate) fn start_training_match(state: State<'_, AppState>, problem_set_id: i64, mode: String, duration_minutes: i64, target_min: Option<f64>, target_max: Option<f64>) -> Result<TrainingMatch, String> {
    let mut conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    training::start_match(&mut conn, problem_set_id, &mode, duration_minutes, target_min, target_max)
}

#[tauri::command]
pub(crate) fn list_training_matches(state: State<'_, AppState>) -> Result<Vec<TrainingMatch>, String> {
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::list_training_matches(&conn)
}

#[tauri::command]
pub(crate) fn refresh_training_match(state: State<'_, AppState>, id: i64) -> Result<TrainingMatch, String> {
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::refresh_training_match(&conn, id)
}

#[tauri::command]
pub(crate) fn finish_training_match(state: State<'_, AppState>, id: i64) -> Result<TrainingMatch, String> {
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::finish_training_match(&conn, id)
}

#[tauri::command]
pub(crate) fn delete_training_match(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    db::delete_training_match(&conn, id)
}

#[tauri::command]
pub(crate) async fn generate_training_candidates(
    state: State<'_, AppState>,
    platforms: Vec<String>,
    mode: String,
    candidate_count: usize,
) -> Result<CandidatePool, String> {
    let cookie = {
        let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
        db::get_accounts(&conn)?.into_iter()
            .find(|entry| entry.platform == "qoj" && !entry.secret.trim().is_empty())
            .map(|entry| entry.secret).unwrap_or_default()
    };
    let pool = training::build_candidate_pool(
        &state.client,
        &state.data_dir.join("xcpc-catalog.json"),
        &cookie,
        &platforms,
        &mode,
        candidate_count,
    ).await?;
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    training::finalize_candidate_pool(&conn, pool)
}

#[tauri::command]
pub(crate) fn export_ai_training_pack(
    state: State<'_, AppState>,
    candidates: Vec<CanonicalProblem>,
    mode: String,
) -> Result<Vec<u8>, String> {
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    let candidates = training::filter_candidates(&conn, candidates)?;
    if candidates.is_empty() { return Err("没有可导出的候选题".into()); }
    let profile = db::training_profile_markdown(&conn)?;
    let pack = training::build_training_pack(&candidates, &mode, profile)?;
    training::training_pack_zip(&pack)
}

#[tauri::command]
pub(crate) fn export_training_pack(state: State<'_, AppState>, problem_set_id: i64, mode: String) -> Result<String, String> {
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    let mut set = db::get_problem_set(&conn, problem_set_id)?;
    set.problems = training::filter_problem_entries(&conn, set.problems)?;
    if set.problems.is_empty() { return Err("题单中没有可导出的未做候选题".into()); }
    let profile = db::training_profile_markdown(&conn)?;
    let candidates = set.problems.into_iter().map(|entry| entry.problem).collect::<Vec<_>>();
    let pack = training::build_training_pack(&candidates, &mode, profile)?;
    serde_json::to_string_pretty(&pack).map(|value| format!("{value}\n")).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn import_match_manifest(state: State<'_, AppState>, data: String) -> Result<TrainingMatch, String> {
    let mut conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    training::import_match_manifest(&mut conn, &data)
}
