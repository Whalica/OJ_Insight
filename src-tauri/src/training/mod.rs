mod candidate;
#[path = "match.rs"]
mod match_service;
mod model;
mod pack;
mod problem_set;
mod template;

pub(crate) use candidate::{build_candidate_pool, filter_candidates, filter_problem_entries, finalize_candidate_pool};
pub(crate) use match_service::{import_match_manifest, start_match};
pub(crate) use model::*;
pub(crate) use pack::{build_training_pack, training_pack_zip};
pub(crate) use problem_set::{import_problem_set, normalize_problem_set, problem_set_manifest};
pub(crate) use template::target_solve_rate;

pub(crate) fn validate_contest(input: &mut ContestInput) -> Result<(), String> {
    input.title = input.title.trim().into();
    if input.title.is_empty() || input.problems.is_empty() { return Err("比赛需要名称和至少一道题".into()); }
    if !(15..=480).contains(&input.duration_minutes) { return Err("比赛时长必须在 15–480 分钟之间".into()); }
    template::target_solve_rate(&input.mode)?;
    problem_set::validate_tag_visibility(&input.tag_visibility)?;
    if input.target_solve_rate_min < 0.0 || input.target_solve_rate_max > 1.0 || input.target_solve_rate_min > input.target_solve_rate_max { return Err("目标完成比例无效".into()); }
    let mut seen=std::collections::HashSet::new();
    for (index,entry) in input.problems.iter_mut().enumerate(){
        entry.position=index as i64;
        entry.problem=problem_set::normalize_problem(entry.problem.clone())?;
        problem_set::validate_role(&entry.role)?;
        if !seen.insert(entry.problem.canonical_id.clone()){return Err("比赛中有重复题目".into());}
    }
    Ok(())
}

pub(crate) fn contest_from_manifest(data:&str)->Result<ContestInput,String>{
    let value:serde_json::Value=serde_json::from_str(data).map_err(|e|format!("比赛 JSON 无效：{e}"))?;
    let schema=value.get("schema").and_then(|v|v.as_str()).unwrap_or("");
    if schema!="com.ojinsight.generated-contest" && schema!="com.ojinsight.match-manifest" && schema!="com.ojinsight.problem-set" { return Err("不是 OJ Insight 生成比赛 JSON".into()); }
    if value.get("schemaVersion").and_then(|v|v.as_i64())!=Some(1){return Err("不支持的比赛 JSON 版本".into());}
    let mode=value.get("mode").and_then(|v|v.as_str()).unwrap_or("balanced").to_string();
    let (min,max)=template::target_solve_rate(&mode)?;
    Ok(ContestInput{id:None,title:value.get("title").and_then(|v|v.as_str()).unwrap_or("").into(),description:value.get("description").and_then(|v|v.as_str()).unwrap_or("").into(),origin:"ai".into(),source_set_id:None,mode,duration_minutes:value.get("durationMinutes").and_then(|v|v.as_i64()).unwrap_or(120),tag_visibility:value.get("tagVisibility").and_then(|v|v.as_str()).unwrap_or("after_ac").into(),target_solve_rate_min:value.get("targetSolveRateMin").and_then(|v|v.as_f64()).unwrap_or(min),target_solve_rate_max:value.get("targetSolveRateMax").and_then(|v|v.as_f64()).unwrap_or(max),problems:serde_json::from_value(value.get("problems").cloned().unwrap_or_default()).map_err(|e|format!("比赛题目无效：{e}"))?})
}
