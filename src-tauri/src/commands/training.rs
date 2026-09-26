use tauri::State;

use crate::app::state::AppState;
use crate::{db, training};
use training::{CandidatePool, CanonicalProblem, Contest, ContestInput, ProblemSet, ProblemSetInput, TrainingMatch, VpSubmission};

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
pub(crate) async fn preview_luogu_problem_set(state: State<'_, AppState>, url: String, cookie: String) -> Result<ProblemSetInput, String> {
    training::preview_luogu_problem_set(&state.client, &url, &cookie).await
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
    extra_requirements: Option<String>,
) -> Result<Vec<u8>, String> {
    let conn = state.db.lock().map_err(|_| "数据库锁异常".to_string())?;
    let candidates = training::filter_candidates(&conn, candidates)?;
    if candidates.is_empty() { return Err("没有可导出的候选题".into()); }
    let profile = db::training_profile_markdown(&conn)?;
    let mut pack = training::build_training_pack(&candidates, &mode, profile)?;
    if let Some(extra)=extra_requirements.filter(|value|!value.trim().is_empty()) {
        pack.files.push(training::TrainingPackFile{name:"EXTRA-REQUIREMENTS.md".into(),content:extra});
    }
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

#[tauri::command]
pub(crate) fn list_contests(state: State<'_, AppState>) -> Result<Vec<Contest>, String> {
    let conn=state.db.lock().map_err(|_|"数据库锁异常".to_string())?;
    db::list_contests(&conn)
}

#[tauri::command]
pub(crate) fn save_contest(state: State<'_, AppState>, mut input: ContestInput) -> Result<Contest, String> {
    training::validate_contest(&mut input)?;
    let conn=state.db.lock().map_err(|_|"数据库锁异常".to_string())?;
    db::save_contest(&conn,&input)
}

#[tauri::command]
pub(crate) fn delete_contest(state: State<'_, AppState>, id:i64)->Result<(),String>{
    let conn=state.db.lock().map_err(|_|"数据库锁异常".to_string())?;
    db::delete_contest(&conn,id)
}

#[tauri::command]
pub(crate) fn contest_from_set(state: State<'_, AppState>, set_id:i64, mode:String, duration_minutes:i64)->Result<Contest,String>{
    let conn=state.db.lock().map_err(|_|"数据库锁异常".to_string())?;
    let set=db::get_problem_set(&conn,set_id)?;
    let (min,max)=training::target_solve_rate(&mode)?;
    let mut input=ContestInput{id:None,title:set.title,description:set.description,origin:"problem_set".into(),source_set_id:Some(set_id),mode,duration_minutes,tag_visibility:set.tag_visibility,target_solve_rate_min:min,target_solve_rate_max:max,problems:set.problems};
    training::validate_contest(&mut input)?;
    db::save_contest(&conn,&input)
}

#[tauri::command]
pub(crate) fn import_generated_contest(state: State<'_, AppState>, data:String)->Result<Contest,String>{
    let mut input=training::contest_from_manifest(&data)?;
    training::validate_contest(&mut input)?;
    let conn=state.db.lock().map_err(|_|"数据库锁异常".to_string())?;
    for entry in &input.problems { if db::is_problem_solved(&conn,&entry.problem.platform,&entry.problem.problem_key)? { return Err(format!("生成结果包含已做题：{}，请重新生成或移除",entry.problem.name)); } }
    db::save_contest(&conn,&input)
}

#[tauri::command]
pub(crate) fn queue_contest(state: State<'_, AppState>, id:i64, countdown_seconds:i64)->Result<TrainingMatch,String>{
    let mut conn=state.db.lock().map_err(|_|"数据库锁异常".to_string())?;
    db::queue_contest(&mut conn,id,countdown_seconds)
}

#[tauri::command]
pub(crate) fn start_vp(state: State<'_, AppState>, id:i64)->Result<TrainingMatch,String>{
    let conn=state.db.lock().map_err(|_|"数据库锁异常".to_string())?;
    db::start_vp(&conn,id)
}

#[tauri::command]
pub(crate) fn schedule_vp(state: State<'_, AppState>, id:i64, countdown_seconds:i64)->Result<TrainingMatch,String>{
    let conn=state.db.lock().map_err(|_|"数据库锁异常".to_string())?;
    db::schedule_vp(&conn,id,countdown_seconds)
}

#[tauri::command]
pub(crate) fn pause_vp(state: State<'_, AppState>, id:i64)->Result<TrainingMatch,String>{
    let conn=state.db.lock().map_err(|_|"数据库锁异常".to_string())?;
    db::pause_vp(&conn,id)
}

#[tauri::command]
pub(crate) fn resume_vp(state: State<'_, AppState>, id:i64)->Result<TrainingMatch,String>{
    let conn=state.db.lock().map_err(|_|"数据库锁异常".to_string())?;
    db::resume_vp(&conn,id)
}

#[tauri::command]
pub(crate) fn save_vp_note(state: State<'_, AppState>, id:i64, position:Option<i64>, note:String)->Result<TrainingMatch,String>{
    let conn=state.db.lock().map_err(|_|"数据库锁异常".to_string())?;
    db::save_vp_note(&conn,id,position,&note)
}

#[tauri::command]
pub(crate) fn list_vp_submissions(state: State<'_, AppState>, id:i64)->Result<Vec<VpSubmission>,String>{
    let conn=state.db.lock().map_err(|_|"数据库锁异常".to_string())?;
    db::list_vp_submissions(&conn,id)
}

#[tauri::command]
pub(crate) async fn refresh_vp_submissions(state: State<'_, AppState>, id:i64)->Result<Vec<VpSubmission>,String>{
    let item={let conn=state.db.lock().map_err(|_|"数据库锁异常".to_string())?;db::get_training_match(&conn,id)?};
    if item.started_at == 0 { return Ok(Vec::new()); }
    let accounts={let conn=state.db.lock().map_err(|_|"数据库锁异常".to_string())?;db::get_accounts(&conn)?};
    let until=item.ended_at.unwrap_or_else(||chrono::Utc::now().timestamp());
    let mut found=Vec::new();
    if item.problems.iter().any(|p|p.problem.platform=="codeforces") {
        for account in accounts.iter().filter(|a|a.platform=="codeforces") {
            let url=format!("https://codeforces.com/api/user.status?handle={}&from=1&count=10000",urlencoding::encode(&account.account));
            if let Ok(response)=state.client.get(&url).send().await {
                if let Ok(value)=response.json::<serde_json::Value>().await {
                    if let Some(rows)=value.pointer("/result").and_then(|v|v.as_array()) {
                        for row in rows {
                            let ts=row.get("creationTimeSeconds").and_then(|v|v.as_i64()).unwrap_or(0);
                            if ts<item.started_at || ts>until {continue;}
                            let problem=&row["problem"];
                            let Some(contest_id)=problem.get("contestId").and_then(|v|v.as_i64()) else {continue};
                            let Some(index)=problem.get("index").and_then(|v|v.as_str()) else {continue};
                            let key=format!("{contest_id}:{index}");
                            if !item.problems.iter().any(|p|p.problem.platform=="codeforces"&&p.problem.problem_key==key){continue;}
                            let submission_id=row.get("id").and_then(|v|v.as_i64()).unwrap_or(0);
                            found.push(VpSubmission{platform:"codeforces".into(),problem_key:key,submitted_at:ts,verdict:row.get("verdict").and_then(|v|v.as_str()).unwrap_or("TESTING").into(),source_url:format!("https://codeforces.com/contest/{contest_id}/submission/{submission_id}")});
                        }
                    }
                }
            }
        }
    }
    if item.problems.iter().any(|p|p.problem.platform=="atcoder") {
        for account in accounts.iter().filter(|a|a.platform=="atcoder") {
            let url=format!("https://kenkoooo.com/atcoder/atcoder-api/v3/user/submissions?user={}&from_second={}",urlencoding::encode(&account.account),item.started_at);
            if let Ok(response)=state.client.get(&url).send().await {
                if let Ok(rows)=response.json::<Vec<serde_json::Value>>().await {
                    for row in rows {
                        let ts=row.get("epoch_second").and_then(|v|v.as_i64()).unwrap_or(0);
                        if ts>until {continue;}
                        let Some(key)=row.get("problem_id").and_then(|v|v.as_str()) else {continue};
                        if !item.problems.iter().any(|p|p.problem.platform=="atcoder"&&p.problem.problem_key==key){continue;}
                        let contest_id=row.get("contest_id").and_then(|v|v.as_str()).unwrap_or("");
                        let submission_id=row.get("id").and_then(|v|v.as_i64()).unwrap_or(0);
                        found.push(VpSubmission{platform:"atcoder".into(),problem_key:key.into(),submitted_at:ts,verdict:row.get("result").and_then(|v|v.as_str()).unwrap_or("未知").into(),source_url:format!("https://atcoder.jp/contests/{contest_id}/submissions/{submission_id}")});
                    }
                }
            }
        }
    }
    let mut conn=state.db.lock().map_err(|_|"数据库锁异常".to_string())?;
    db::save_vp_submissions(&mut conn,id,&found)?;
    db::list_vp_submissions(&conn,id)
}

#[tauri::command]
pub(crate) fn bind_vp_code(state: State<'_, AppState>, id:i64, position:i64, path:String)->Result<(),String>{
    let source=std::path::Path::new(&path);
    let name=source.file_name().and_then(|v|v.to_str()).ok_or("无效的代码文件名")?;
    let content=std::fs::read(source).map_err(|e|format!("读取代码文件失败：{e}"))?;
    if content.len()>2_000_000{return Err("代码文件超过 2 MB".into());}
    let mut conn=state.db.lock().map_err(|_|"数据库锁异常".to_string())?;
    db::bind_vp_code(&mut conn,id,position,name,&content)
}

#[tauri::command]
pub(crate) fn delete_vp_code(state: State<'_, AppState>, id:i64, position:i64)->Result<(),String>{
    let conn=state.db.lock().map_err(|_|"数据库锁异常".to_string())?;
    db::delete_vp_code(&conn,id,position)
}

#[tauri::command]
pub(crate) fn list_vp_code_files(state: State<'_, AppState>, id:i64)->Result<Vec<(i64,String)>,String>{
    let conn=state.db.lock().map_err(|_|"数据库锁异常".to_string())?;
    db::list_vp_code_names(&conn,id)
}

#[tauri::command]
pub(crate) fn export_vp_review_pack(state: State<'_, AppState>, id:i64)->Result<Vec<u8>,String>{
    use std::io::{Cursor,Write};
    let conn=state.db.lock().map_err(|_|"数据库锁异常".to_string())?;
    let item=db::get_training_match(&conn,id)?;
    if item.status!="finished"{return Err("请先结束比赛再导出复盘包".into());}
    let submissions=db::list_vp_submissions(&conn,id)?;
    let code=db::list_vp_code(&conn,id)?;
    let cursor=Cursor::new(Vec::new());
    let mut zip=zip::ZipWriter::new(cursor);
    let options=zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    let mut add=|name:&str,data:&[u8]|->Result<(),String>{zip.start_file(name,options).map_err(|e|e.to_string())?;zip.write_all(data).map_err(|e|e.to_string())};
    let intro="请只根据本 ZIP 中的资料分析这场 VP：总结选题与时间安排、每题思路及失误、下一步训练建议。提交结果和代码可能不完整；不要推断缺失的 WA 或编造未保存的代码。";
    add("START-HERE.md",intro.as_bytes())?;
    let manifest=serde_json::json!({"schema":"com.ojinsight.vp-review","schemaVersion":1,"match":&item,"submissions":&submissions,"codeFiles":code.iter().map(|(position,name,_)|serde_json::json!({"position":position,"name":name})).collect::<Vec<_>>()});
    add("REVIEW.json",serde_json::to_string_pretty(&manifest).map_err(|e|e.to_string())?.as_bytes())?;
    let mut notes=format!("# {}\n\n## 整场总评\n\n{}\n",item.title,item.general_note);
    for problem in &item.problems {notes.push_str(&format!("\n## {}. {}\n\n{}\n",problem.position+1,problem.problem.name,problem.solution_note));}
    add("NOTES.md",notes.as_bytes())?;
    for (index,(position,name,content)) in code.iter().enumerate(){
        let safe=name.chars().map(|c|if c.is_ascii_alphanumeric()||"._-".contains(c){c}else{'_'}).collect::<String>();
        add(&format!("code/{}-{}-{safe}",position+1,index+1),content)?;
    }
    drop(add);
    zip.finish().map(|cursor|cursor.into_inner()).map_err(|e|e.to_string())
}

#[tauri::command]
pub(crate) async fn lookup_problem_metadata(state: State<'_, AppState>, mut problem: CanonicalProblem)->Result<CanonicalProblem,String>{
    use rusqlite::OptionalExtension;
    use serde_json::Value;
    let cached={
        let conn=state.db.lock().map_err(|_|"数据库锁异常".to_string())?;
        conn.query_row("SELECT problem_name,difficulty,tags FROM submissions WHERE platform=? AND problem_key=? ORDER BY epoch_second DESC LIMIT 1",rusqlite::params![&problem.platform,&problem.problem_key],|row|Ok((row.get::<_,String>(0)?,row.get::<_,Option<String>>(1)?,row.get::<_,String>(2)?))).optional().map_err(|e|e.to_string())?
    };
    if let Some((name,difficulty,tags))=cached {
        if !name.trim().is_empty() && name!=problem.name {
            problem.name=name;
            problem.difficulty=difficulty;
            problem.tags=serde_json::from_str(&tags).unwrap_or_default();
            return Ok(problem);
        }
    }
    match problem.platform.as_str(){
        "codeforces"=>{
            let (contest,index)=problem.problem_key.split_once(':').ok_or("Codeforces 题目标识无效")?;
            let url=format!("https://codeforces.com/api/contest.standings?contestId={contest}&from=1&count=1");
            let api=async {
                state.client.get(url).send().await.map_err(|e|e.to_string())?.error_for_status().map_err(|e|e.to_string())?.json::<Value>().await.map_err(|e|e.to_string())
            }.await;
            let row=api.as_ref().ok().and_then(|value|value.pointer("/result/problems")).and_then(Value::as_array).and_then(|rows|rows.iter().find(|row|row.get("index").and_then(Value::as_str)==Some(index)));
            if let Some(row)=row {
                problem.name=row.get("name").and_then(Value::as_str).map(str::to_string).ok_or("CF 接口没有返回题目标题")?;
                problem.tags=row.get("tags").and_then(Value::as_array).map(|rows|rows.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default();
                problem.difficulty=row.get("rating").and_then(Value::as_i64).map(|v|v.to_string());
            } else {
                let html=state.client.get(&problem.url).send().await.map_err(|e|format!("CF 接口不可用，题目页也无法访问：{e}"))?.error_for_status().map_err(|e|format!("CF 接口不可用，题目页返回：{e}"))?.text().await.map_err(|e|e.to_string())?;
                let doc=scraper::Html::parse_document(&html);
                let selector=scraper::Selector::parse(".problem-statement .title").map_err(|e|e.to_string())?;
                problem.name=doc.select(&selector).next().map(|element|element.text().collect::<String>().trim().to_string()).filter(|name|!name.is_empty()).ok_or("CF 接口与题目页都没有返回标题，请稍后重试")?;
            }
        }
        "qoj"=>{
            let html=state.client.get(&problem.url).send().await.map_err(|e|e.to_string())?.error_for_status().map_err(|e|e.to_string())?.text().await.map_err(|e|e.to_string())?;
            let doc=scraper::Html::parse_document(&html);
            let heading=scraper::Selector::parse("h1, h2, title").map_err(|e|e.to_string())?;
            let name=doc.select(&heading).map(|element|element.text().collect::<String>().trim().to_string()).find(|value|!value.is_empty() && !value.eq_ignore_ascii_case("QOJ"));
            problem.name=name.ok_or("QOJ 题目页没有返回标题，请稍后重试")?;
            let tags=scraper::Selector::parse("a[href*='/tag/'], .problem-tag, .tags a").map_err(|e|e.to_string())?;
            problem.tags=doc.select(&tags).map(|element|element.text().collect::<String>().trim().to_string()).filter(|value|!value.is_empty()).collect();
        }
        "atcoder"=>{
            let html=state.client.get(&problem.url).send().await.map_err(|e|e.to_string())?.error_for_status().map_err(|e|e.to_string())?.text().await.map_err(|e|e.to_string())?;
            let doc=scraper::Html::parse_document(&html);
            let selector=scraper::Selector::parse("span.h2, title").map_err(|e|e.to_string())?;
            problem.name=doc.select(&selector).map(|element|element.text().collect::<String>().trim().to_string()).find(|value|!value.is_empty()).ok_or("AtCoder 题目页没有返回标题，请稍后重试")?;
        }
        _=>{
            let html=state.client.get(&problem.url).send().await.map_err(|e|e.to_string())?.error_for_status().map_err(|e|e.to_string())?.text().await.map_err(|e|e.to_string())?;
            let doc=scraper::Html::parse_document(&html);
            let selector=scraper::Selector::parse("h1, title").map_err(|e|e.to_string())?;
            problem.name=doc.select(&selector).map(|element|element.text().collect::<String>().trim().to_string()).find(|value|!value.is_empty()).ok_or("题目页没有返回标题，请稍后重试")?;
        }
    }
    Ok(problem)
}
