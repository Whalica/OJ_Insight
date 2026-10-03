use std::collections::{BTreeSet, VecDeque};
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension};

use crate::app::state::AppState;
use crate::db;
use crate::training::nowcoder_identity::{fetch_nowcoder_aliases, nowcoder_identity_client, nowcoder_identity_url};
use crate::training::ProblemSetProblem;

fn lookup_due(conn: &Connection, key: &str) -> Result<bool, String> {
    if db::has_nowcoder_problem_alias(conn, key)? { return Ok(false); }
    let attempted_at: Option<i64> = conn.query_row(
        "SELECT attempted_at FROM nowcoder_alias_attempts WHERE problem_key=?", [key], |row| row.get(0),
    ).optional().map_err(|error| error.to_string())?;
    Ok(attempted_at.is_none_or(|at| chrono::Utc::now().timestamp() - at >= 120))
}

fn pending_keys(conn: &Connection, problems: &[ProblemSetProblem]) -> Result<VecDeque<String>, String> {
    let mut pending = BTreeSet::new();
    let mut unresolved = false;
    for row in problems.iter().filter(|row| row.problem.platform == "nowcoder") {
        let key = &row.problem.problem_key;
        if nowcoder_identity_url(key).is_none() || db::is_problem_solved(conn, "nowcoder", key)? {
            continue;
        }
        unresolved = true;
        if lookup_due(conn, key)? { pending.insert(key.clone()); }
    }
    if !unresolved { return Ok(VecDeque::new()); }

    // Some older syncs stored a contest/practice key. Resolve those too, so a
    // numeric-ID problem set can match an existing AC in the opposite direction.
    let mut statement = conn.prepare("SELECT problem_key FROM submissions WHERE platform='nowcoder' UNION SELECT problem_key FROM solved_inventory WHERE platform='nowcoder'")
        .map_err(|error| error.to_string())?;
    let keys = statement.query_map([], |row| row.get::<_, String>(0)).map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>().map_err(|error| error.to_string())?;
    if keys.is_empty() { return Ok(VecDeque::new()); }
    for key in keys {
        if !key.bytes().all(|byte| byte.is_ascii_digit()) && nowcoder_identity_url(&key).is_some()
            && lookup_due(conn, &key)? {
            pending.insert(key);
        }
    }
    let mut ordered = Vec::new();
    for key in pending {
        let attempted_at: Option<i64> = conn.query_row(
            "SELECT attempted_at FROM nowcoder_alias_attempts WHERE problem_key=?", [&key], |row| row.get(0),
        ).optional().map_err(|error| error.to_string())?;
        ordered.push((attempted_at, key));
    }
    // Even after the cooldown expires, a failing prefix must not starve keys
    // that have never been tried. Option::None sorts before known timestamps.
    ordered.sort();
    Ok(ordered.into_iter().map(|(_, key)| key).collect())
}

pub(super) async fn ensure_problem_aliases(state: &AppState, problems: &[ProblemSetProblem]) -> Result<(), String> {
    let mut pending = {
        let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
        pending_keys(&conn, problems)?
    };
    if pending.is_empty() { return Ok(()); }
    let Ok(client) = nowcoder_identity_client() else { return Ok(()); };

    // Alias discovery is optional enrichment. Offline/changed pages must never
    // prevent browsing or importing, and no database lock is held over an await.
    // Cache successes in SQLite; cap concurrent requests and the total wait.
    let mut tasks = tokio::task::JoinSet::new();
    let work = async {
        loop {
            while tasks.len() < 4 {
                let Some(key) = pending.pop_front() else { break; };
                {
                    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
                    if !lookup_due(&conn, &key)? { continue; }
                    // Record before dispatch, including requests cancelled by the
                    // overall deadline. Later reads can advance past a slow prefix
                    // and cannot repeatedly block on failed/in-flight requests.
                    conn.execute("INSERT INTO nowcoder_alias_attempts(problem_key,attempted_at) VALUES(?,?) ON CONFLICT(problem_key) DO UPDATE SET attempted_at=excluded.attempted_at",
                        rusqlite::params![key, chrono::Utc::now().timestamp()]).map_err(|error| error.to_string())?;
                }
                let client = client.clone();
                tasks.spawn(async move { fetch_nowcoder_aliases(&client, &key).await });
            }
            let Some(result) = tasks.join_next().await else { break; };
            if let Ok(Some(aliases)) = result {
                let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
                // Conflicting identities are ignored, never merged by title or
                // used to overwrite a previously verified relationship.
                let _ = db::save_nowcoder_problem_aliases(&conn, &aliases.problem_id, &aliases.keys);
            }
        }
        Ok::<(), String>(())
    };
    if let Ok(result) = tokio::time::timeout(Duration::from_secs(12), work).await { result?; }
    tasks.abort_all();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::training::ProblemSetInput;

    fn entry(key: &str) -> ProblemSetProblem {
        serde_json::from_value(serde_json::json!({
            "problem": {"platform":"nowcoder", "problemKey":key, "name":"Same title"}
        })).unwrap()
    }

    #[test]
    fn exact_ac_and_cached_aliases_do_not_trigger_page_requests() {
        let conn = db::open(std::path::Path::new(":memory:")).unwrap();
        assert!(pending_keys(&conn, &[entry("78807/D")]).unwrap().is_empty());
        conn.execute("INSERT INTO solved_inventory(platform,account,problem_key,updated_at) VALUES('nowcoder','test','269161',0)", []).unwrap();
        assert!(pending_keys(&conn, &[entry("269161")]).unwrap().is_empty());
        assert_eq!(pending_keys(&conn, &[entry("78807/D")]).unwrap(), ["78807/D".to_string()]);
        db::save_nowcoder_problem_aliases(&conn, "269161", &["78807/D".into()]).unwrap();
        assert!(pending_keys(&conn, &[entry("78807/D")]).unwrap().is_empty());
    }

    #[test]
    fn existing_sets_and_reverse_submission_keys_are_discovered_without_reimport() {
        let mut conn = db::open(std::path::Path::new(":memory:")).unwrap();
        let input: ProblemSetInput = serde_json::from_value(serde_json::json!({
            "title":"Existing set", "problems":[entry("269161")]
        })).unwrap();
        let saved = db::save_problem_set(&mut conn, &input).unwrap();
        conn.execute("INSERT INTO solved_inventory(platform,account,problem_key,updated_at) VALUES('nowcoder','test','78807/D',0)", []).unwrap();
        let sets = db::list_problem_sets(&conn).unwrap();
        assert_eq!(pending_keys(&conn, &sets[0].problems).unwrap(), ["269161".to_string(), "78807/D".to_string()]);
        db::save_nowcoder_problem_aliases(&conn, "269161", &["78807/D".into()]).unwrap();
        assert_eq!(db::get_problem_set(&conn, saved.id).unwrap().solved_keys, ["nowcoder:269161"]);
    }

    #[test]
    fn failed_or_inflight_lookups_back_off_without_starving_later_keys() {
        let conn = db::open(std::path::Path::new(":memory:")).unwrap();
        conn.execute("INSERT INTO solved_inventory(platform,account,problem_key,updated_at) VALUES('nowcoder','test','269161',0)", []).unwrap();
        conn.execute("INSERT INTO nowcoder_alias_attempts(problem_key,attempted_at) VALUES('78807/A',?)", [chrono::Utc::now().timestamp()]).unwrap();
        let problems = [entry("78807/A"), entry("78807/D")];
        assert_eq!(pending_keys(&conn, &problems).unwrap(), ["78807/D".to_string()]);
        assert!(!db::is_problem_solved(&conn, "nowcoder", "78807/A").unwrap());
        conn.execute("UPDATE nowcoder_alias_attempts SET attempted_at=attempted_at-121", []).unwrap();
        assert_eq!(pending_keys(&conn, &problems).unwrap(), ["78807/D".to_string(), "78807/A".to_string()]);
    }
}
