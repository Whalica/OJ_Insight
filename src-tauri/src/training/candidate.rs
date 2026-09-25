use rusqlite::Connection;

use super::{CanonicalProblem, ProblemSetProblem};

pub(crate) fn filter_candidates(conn: &Connection, candidates: Vec<CanonicalProblem>) -> Result<Vec<CanonicalProblem>, String> {
    let mut out = Vec::new();
    for candidate in candidates {
        let problem = super::problem_set::normalize_problem(candidate)?;
        if problem.interactive || problem.output_only || problem.training_suitability.is_some_and(|score| score < 0.35) || problem.observation_dependency.is_some_and(|score| score > 0.8) { continue; }
        let solved: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM submissions WHERE platform=? AND problem_key=?)", rusqlite::params![problem.platform, problem.problem_key], |row| row.get(0)).map_err(|e| e.to_string())?;
        if !solved { out.push(problem); }
    }
    Ok(out)
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
