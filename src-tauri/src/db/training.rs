use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::training::{CanonicalProblem, ProblemSet, ProblemSetInput, ProblemSetProblem, TrainingMatch, TrainingMatchProblem};

const TRAINING_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS canonical_problems (
  platform TEXT NOT NULL,
  problem_key TEXT NOT NULL,
  problem_id TEXT NOT NULL DEFAULT '',
  name TEXT NOT NULL DEFAULT '',
  url TEXT NOT NULL DEFAULT '',
  difficulty TEXT,
  tags TEXT NOT NULL DEFAULT '[]',
  training_suitability REAL,
  observation_dependency REAL,
  implementation_load REAL,
  knowledge_dependency REAL,
  interactive INTEGER NOT NULL DEFAULT 0,
  output_only INTEGER NOT NULL DEFAULT 0,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY(platform, problem_key)
);
CREATE TABLE IF NOT EXISTS problem_sets (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  title TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  set_type TEXT NOT NULL DEFAULT 'static',
  tag_visibility TEXT NOT NULL DEFAULT 'after_ac',
  source_set_id INTEGER,
  source_url TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS problem_set_problems (
  set_id INTEGER NOT NULL,
  position INTEGER NOT NULL,
  platform TEXT NOT NULL,
  problem_key TEXT NOT NULL,
  role TEXT NOT NULL DEFAULT 'Core',
  note TEXT NOT NULL DEFAULT '',
  PRIMARY KEY(set_id, position),
  UNIQUE(set_id, platform, problem_key),
  FOREIGN KEY(set_id) REFERENCES problem_sets(id) ON DELETE CASCADE,
  FOREIGN KEY(platform, problem_key) REFERENCES canonical_problems(platform, problem_key)
);
CREATE TABLE IF NOT EXISTS training_matches (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  problem_set_id INTEGER,
  title TEXT NOT NULL,
  mode TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'running',
  tag_visibility TEXT NOT NULL DEFAULT 'after_ac',
  target_solve_rate_min REAL NOT NULL,
  target_solve_rate_max REAL NOT NULL,
  duration_minutes INTEGER NOT NULL,
  started_at INTEGER NOT NULL,
  ended_at INTEGER,
  created_at INTEGER NOT NULL,
  FOREIGN KEY(problem_set_id) REFERENCES problem_sets(id) ON DELETE SET NULL
);
CREATE TABLE IF NOT EXISTS training_match_problems (
  match_id INTEGER NOT NULL,
  position INTEGER NOT NULL,
  platform TEXT NOT NULL,
  problem_key TEXT NOT NULL,
  role TEXT NOT NULL,
  note TEXT NOT NULL DEFAULT '',
  solved INTEGER NOT NULL DEFAULT 0,
  solved_at INTEGER,
  feedback TEXT,
  PRIMARY KEY(match_id, position),
  UNIQUE(match_id, platform, problem_key),
  FOREIGN KEY(match_id) REFERENCES training_matches(id) ON DELETE CASCADE,
  FOREIGN KEY(platform, problem_key) REFERENCES canonical_problems(platform, problem_key)
);
CREATE INDEX IF NOT EXISTS idx_training_match_status ON training_matches(status, started_at);
"#;

pub(super) fn initialize_training_schema(tx: &Transaction<'_>) -> Result<(), String> {
    tx.execute_batch(TRAINING_SCHEMA).map_err(|e| format!("初始化 Training 数据库失败：{e}"))
}

fn tags(value: &str) -> Vec<String> { serde_json::from_str(value).unwrap_or_default() }

fn row_problem(row: &rusqlite::Row<'_>, offset: usize) -> rusqlite::Result<CanonicalProblem> {
    let platform: String = row.get(offset)?;
    let problem_key: String = row.get(offset + 1)?;
    Ok(CanonicalProblem {
        canonical_id: format!("{platform}:{problem_key}"), platform, problem_key,
        problem_id: row.get(offset + 2)?, name: row.get(offset + 3)?, url: row.get(offset + 4)?, difficulty: row.get(offset + 5)?,
        tags: tags(&row.get::<_, String>(offset + 6)?), training_suitability: row.get(offset + 7)?, observation_dependency: row.get(offset + 8)?,
        implementation_load: row.get(offset + 9)?, knowledge_dependency: row.get(offset + 10)?, interactive: row.get::<_, i64>(offset + 11)? != 0,
        output_only: row.get::<_, i64>(offset + 12)? != 0,
    })
}

fn upsert_problem(tx: &Transaction<'_>, problem: &CanonicalProblem, now: i64) -> Result<(), String> {
    let tags = serde_json::to_string(&problem.tags).map_err(|e| e.to_string())?;
    tx.execute("INSERT INTO canonical_problems(platform,problem_key,problem_id,name,url,difficulty,tags,training_suitability,observation_dependency,implementation_load,knowledge_dependency,interactive,output_only,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?) ON CONFLICT(platform,problem_key) DO UPDATE SET problem_id=excluded.problem_id,name=excluded.name,url=excluded.url,difficulty=excluded.difficulty,tags=excluded.tags,training_suitability=excluded.training_suitability,observation_dependency=excluded.observation_dependency,implementation_load=excluded.implementation_load,knowledge_dependency=excluded.knowledge_dependency,interactive=excluded.interactive,output_only=excluded.output_only,updated_at=excluded.updated_at",
        params![problem.platform,problem.problem_key,problem.problem_id,problem.name,problem.url,problem.difficulty,tags,problem.training_suitability,problem.observation_dependency,problem.implementation_load,problem.knowledge_dependency,problem.interactive as i64,problem.output_only as i64,now]).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn save_problem_set(conn: &mut Connection, input: &ProblemSetInput) -> Result<ProblemSet, String> {
    let now = chrono::Utc::now().timestamp();
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let id = if let Some(id) = input.id {
        let changed = tx.execute("UPDATE problem_sets SET title=?,description=?,set_type=?,tag_visibility=?,source_set_id=?,source_url=?,updated_at=? WHERE id=?", params![input.title,input.description,input.set_type,input.tag_visibility,input.source_set_id,input.source_url,now,id]).map_err(|e| e.to_string())?;
        if changed == 0 { return Err("题单不存在".into()); }
        tx.execute("DELETE FROM problem_set_problems WHERE set_id=?", [id]).map_err(|e| e.to_string())?;
        id
    } else {
        tx.execute("INSERT INTO problem_sets(title,description,set_type,tag_visibility,source_set_id,source_url,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?)", params![input.title,input.description,input.set_type,input.tag_visibility,input.source_set_id,input.source_url,now,now]).map_err(|e| e.to_string())?;
        tx.last_insert_rowid()
    };
    for entry in &input.problems {
        upsert_problem(&tx, &entry.problem, now)?;
        tx.execute("INSERT INTO problem_set_problems(set_id,position,platform,problem_key,role,note) VALUES(?,?,?,?,?,?)", params![id,entry.position,entry.problem.platform,entry.problem.problem_key,entry.role,entry.note]).map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    get_problem_set(conn, id)
}

pub fn list_problem_sets(conn: &Connection) -> Result<Vec<ProblemSet>, String> {
    let mut stmt = conn.prepare("SELECT id FROM problem_sets ORDER BY updated_at DESC,id DESC").map_err(|e| e.to_string())?;
    let ids = stmt.query_map([], |row| row.get::<_, i64>(0)).map_err(|e| e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e| e.to_string())?;
    ids.into_iter().map(|id| get_problem_set(conn, id)).collect()
}

pub fn get_problem_set(conn: &Connection, id: i64) -> Result<ProblemSet, String> {
    let mut set = conn.query_row("SELECT id,title,description,set_type,tag_visibility,source_set_id,source_url,created_at,updated_at FROM problem_sets WHERE id=?", [id], |row| Ok(ProblemSet { id:row.get(0)?,title:row.get(1)?,description:row.get(2)?,set_type:row.get(3)?,tag_visibility:row.get(4)?,source_set_id:row.get(5)?,source_url:row.get(6)?,created_at:row.get(7)?,updated_at:row.get(8)?,problems:Vec::new() })).optional().map_err(|e| e.to_string())?.ok_or_else(|| "题单不存在".to_string())?;
    let mut stmt = conn.prepare("SELECT p.position,p.role,p.note,c.platform,c.problem_key,c.problem_id,c.name,c.url,c.difficulty,c.tags,c.training_suitability,c.observation_dependency,c.implementation_load,c.knowledge_dependency,c.interactive,c.output_only FROM problem_set_problems p JOIN canonical_problems c ON c.platform=p.platform AND c.problem_key=p.problem_key WHERE p.set_id=? ORDER BY p.position").map_err(|e| e.to_string())?;
    set.problems = stmt.query_map([id], |row| Ok(ProblemSetProblem { position:row.get(0)?,role:row.get(1)?,note:row.get(2)?,problem:row_problem(row,3)? })).map_err(|e| e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e| e.to_string())?;
    Ok(set)
}

pub fn delete_problem_set(conn: &Connection, id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM problem_sets WHERE id=?", [id]).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn is_problem_solved(conn: &Connection, platform: &str, problem_key: &str) -> Result<bool, String> {
    conn.query_row("SELECT EXISTS(SELECT 1 FROM submissions WHERE platform=? AND problem_key=?)", params![platform,problem_key], |row| row.get(0)).map_err(|e| e.to_string())
}

pub fn create_training_match(conn: &mut Connection, set_id: Option<i64>, title: &str, mode: &str, tag_visibility: &str, min: f64, max: f64, duration: i64, problems: &[ProblemSetProblem]) -> Result<TrainingMatch, String> {
    let now = chrono::Utc::now().timestamp();
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute("INSERT INTO training_matches(problem_set_id,title,mode,status,tag_visibility,target_solve_rate_min,target_solve_rate_max,duration_minutes,started_at,created_at) VALUES(?,?,?,'running',?,?,?,?,?,?)", params![set_id,title,mode,tag_visibility,min,max,duration,now,now]).map_err(|e| e.to_string())?;
    let id = tx.last_insert_rowid();
    for entry in problems {
        upsert_problem(&tx, &entry.problem, now)?;
        tx.execute("INSERT INTO training_match_problems(match_id,position,platform,problem_key,role,note) VALUES(?,?,?,?,?,?)", params![id,entry.position,entry.problem.platform,entry.problem.problem_key,entry.role,entry.note]).map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    get_training_match(conn, id)
}

pub fn refresh_training_match(conn: &Connection, id: i64) -> Result<TrainingMatch, String> {
    conn.execute("UPDATE training_match_problems SET solved=EXISTS(SELECT 1 FROM submissions s JOIN training_matches m ON m.id=training_match_problems.match_id WHERE s.platform=training_match_problems.platform AND s.problem_key=training_match_problems.problem_key AND s.epoch_second>=m.started_at), solved_at=(SELECT MIN(s.epoch_second) FROM submissions s JOIN training_matches m ON m.id=training_match_problems.match_id WHERE s.platform=training_match_problems.platform AND s.problem_key=training_match_problems.problem_key AND s.epoch_second>=m.started_at) WHERE match_id=?", [id]).map_err(|e| e.to_string())?;
    get_training_match(conn, id)
}

pub fn refresh_running_training_matches(conn: &Connection) -> Result<(), String> {
    let mut stmt = conn.prepare("SELECT id FROM training_matches WHERE status='running'").map_err(|e| e.to_string())?;
    let ids = stmt.query_map([], |row| row.get::<_, i64>(0)).map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    drop(stmt);
    for id in ids { refresh_training_match(conn, id)?; }
    Ok(())
}

pub fn training_profile_markdown(conn: &Connection) -> Result<String, String> {
    let mut stmt = conn.prepare("SELECT platform,COUNT(DISTINCT problem_key),COUNT(DISTINCT CASE WHEN epoch_second>=? THEN problem_key END) FROM submissions GROUP BY platform ORDER BY platform").map_err(|e| e.to_string())?;
    let since = chrono::Utc::now().timestamp() - 90 * 86_400;
    let rows = stmt.query_map([since], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?))).map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    drop(stmt);
    let mut output = String::from("# Profile\n\nLocal solved-problem summary. Difficulty systems remain platform-specific.\n\n| Platform | Career solved | Last 90 days |\n|---|---:|---:|\n");
    if rows.is_empty() { output.push_str("| No synced data | 0 | 0 |\n"); }
    for (platform, career, recent) in rows { output.push_str(&format!("| {platform} | {career} | {recent} |\n")); }
    let mut match_stmt = conn.prepare("SELECT m.mode,COUNT(DISTINCT m.id),SUM(p.solved),COUNT(p.position) FROM training_matches m JOIN training_match_problems p ON p.match_id=m.id WHERE m.status='finished' GROUP BY m.mode ORDER BY m.mode").map_err(|e| e.to_string())?;
    let history = match_stmt.query_map([], |row| Ok((row.get::<_,String>(0)?,row.get::<_,i64>(1)?,row.get::<_,i64>(2)?,row.get::<_,i64>(3)?))).map_err(|e| e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
    output.push_str("\n## Training history\n\n| Mode | Matches | Solved | Problems |\n|---|---:|---:|---:|\n");
    if history.is_empty() { output.push_str("| No finished matches | 0 | 0 | 0 |\n"); }
    for (mode, matches, solved, total) in history { output.push_str(&format!("| {mode} | {matches} | {solved} | {total} |\n")); }
    Ok(output)
}

pub fn finish_training_match(conn: &Connection, id: i64) -> Result<TrainingMatch, String> {
    refresh_training_match(conn, id)?;
    conn.execute("UPDATE training_matches SET status='finished',ended_at=COALESCE(ended_at,?) WHERE id=?", params![chrono::Utc::now().timestamp(),id]).map_err(|e| e.to_string())?;
    get_training_match(conn, id)
}

pub fn list_training_matches(conn: &Connection) -> Result<Vec<TrainingMatch>, String> {
    let mut stmt = conn.prepare("SELECT id FROM training_matches ORDER BY started_at DESC,id DESC").map_err(|e| e.to_string())?;
    let ids = stmt.query_map([], |row| row.get::<_,i64>(0)).map_err(|e| e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e| e.to_string())?;
    ids.into_iter().map(|id| get_training_match(conn,id)).collect()
}

pub fn get_training_match(conn: &Connection, id: i64) -> Result<TrainingMatch, String> {
    let mut item = conn.query_row("SELECT id,problem_set_id,title,mode,status,tag_visibility,target_solve_rate_min,target_solve_rate_max,duration_minutes,started_at,ended_at,created_at FROM training_matches WHERE id=?", [id], |row| Ok(TrainingMatch { id:row.get(0)?,problem_set_id:row.get(1)?,title:row.get(2)?,mode:row.get(3)?,status:row.get(4)?,tag_visibility:row.get(5)?,target_solve_rate_min:row.get(6)?,target_solve_rate_max:row.get(7)?,duration_minutes:row.get(8)?,started_at:row.get(9)?,ended_at:row.get(10)?,created_at:row.get(11)?,problems:Vec::new() })).optional().map_err(|e| e.to_string())?.ok_or_else(|| "训练赛不存在".to_string())?;
    let mut stmt=conn.prepare("SELECT p.position,p.role,p.note,p.solved,p.solved_at,c.platform,c.problem_key,c.problem_id,c.name,c.url,c.difficulty,c.tags,c.training_suitability,c.observation_dependency,c.implementation_load,c.knowledge_dependency,c.interactive,c.output_only FROM training_match_problems p JOIN canonical_problems c ON c.platform=p.platform AND c.problem_key=p.problem_key WHERE p.match_id=? ORDER BY p.position").map_err(|e|e.to_string())?;
    item.problems=stmt.query_map([id],|row|Ok(TrainingMatchProblem{position:row.get(0)?,role:row.get(1)?,note:row.get(2)?,solved:row.get::<_,i64>(3)?!=0,solved_at:row.get(4)?,problem:row_problem(row,5)?})).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
    Ok(item)
}
