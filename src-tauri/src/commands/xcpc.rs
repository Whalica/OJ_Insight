use tauri::State;

use crate::app::state::AppState;
use crate::db;
use crate::models::XcpcContest;
use crate::xcpc;

#[tauri::command]
pub(crate) async fn get_xcpc_contests(
    state: State<'_, AppState>,
    force_refresh: Option<bool>,
    refresh_ratings: Option<bool>,
) -> Result<Vec<XcpcContest>, String> {
    let cookie = {
        let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
        let accounts = db::get_accounts(&conn)?;
        accounts
            .iter()
            .find(|entry| entry.platform == "qoj" && !entry.secret.trim().is_empty())
            .or_else(|| accounts.iter().find(|entry| entry.platform == "qoj"))
            .map(|entry| entry.secret.clone())
            .unwrap_or_default()
    };
    let mut contests = xcpc::load_catalog(
        &state.client,
        &state.data_dir.join("xcpc-catalog.json"),
        &cookie,
        force_refresh.unwrap_or(false),
    )
    .await?;
    // Tags are optional metadata. Keep the contest tracker usable when the
    // upstream repository is temporarily unavailable, and reuse the last
    // successful local manifest on subsequent opens.
    let _ = xcpc::apply_problem_tags(
        &state.client,
        &state.data_dir.join("xcpc-problem-types.json"),
        &mut contests,
        force_refresh.unwrap_or(false),
    )
    .await;
    if refresh_ratings.unwrap_or(false) {
        xcpc::sync_public_ratings(
            &state.client,
            &state.data_dir.join("xcpc-catalog.json"),
            &mut contests,
        )
        .await?;
    }
    let solved = {
        let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
        db::apply_qoj_problem_ratings(&conn, &contests)?;
        db::solved_problem_keys(&conn, "qoj")?
    };
    for contest in &mut contests {
        for problem in &mut contest.problems {
            problem.solved = solved.contains(&problem.problem_id);
        }
    }
    Ok(contests)
}
