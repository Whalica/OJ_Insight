use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

use regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};

use crate::models::{XcpcContest, XcpcProblem};
use crate::sync::{browser_headers, get_text, with_cookie};

const ROOT_CATEGORIES: [(&str, usize); 3] = [("21", 1), ("205", 1), ("212", 1)];
const CATALOG_CACHE_VERSION: u32 = 4;

#[derive(serde::Serialize, serde::Deserialize)]
struct CatalogCache {
    version: u32,
    contests: Vec<XcpcContest>,
}

#[derive(Clone)]
struct RanklandBoard {
    uk: String,
    file_id: String,
    direct_url: Option<String>,
    text: String,
    date: String,
}

#[derive(Clone)]
struct XcpcioBoard {
    directory: String,
    text: String,
}

pub async fn load_catalog(client: &Client, cache_path: &Path, cookie: &str, force_refresh: bool) -> Result<Vec<XcpcContest>, String> {
    let cached = std::fs::read_to_string(cache_path).ok()
        .and_then(|text| serde_json::from_str::<CatalogCache>(&text).ok())
        .filter(|cache| cache.version == CATALOG_CACHE_VERSION)
        .map(|cache| cache.contests)
        .filter(|items| !items.is_empty());
    if !force_refresh {
        if let Some(items) = cached.as_ref() {
            let mut items = items.clone();
            if !cookie.trim().is_empty() && items.iter().any(contest_needs_problem_details) {
                enrich_contest_problems(client, cookie, &mut items).await;
                save_catalog(cache_path, &items)?;
            }
            return Ok(items);
        }
    }
    let mut items = fetch_catalog(client, cookie).await?;
    if let Some(cached) = cached {
        let cached_by_id: HashMap<_, _> = cached.into_iter().map(|contest| (contest.id.clone(), contest)).collect();
        for contest in &mut items {
            if let Some(previous) = cached_by_id.get(&contest.id) {
                if cached_problems_are_well_formed(&previous.problems) {
                    contest.problems = previous.problems.clone();
                }
                if contest.board_source.is_none() { contest.board_source = previous.board_source.clone(); }
            }
        }
    }
    if !cookie.trim().is_empty() {
        enrich_contest_problems(client, cookie, &mut items).await;
    }
    let json = serde_json::to_string(&CatalogCache { version: CATALOG_CACHE_VERSION, contests: items.clone() })
        .map_err(|e| format!("序列化 XCPC 目录失败：{e}"))?;
    std::fs::write(cache_path, json).map_err(|e| format!("保存 XCPC 目录失败：{e}"))?;
    Ok(items)
}

pub async fn sync_rankland_ratings(client: &Client, cache_path: &Path, contests: &mut [XcpcContest]) -> Result<usize, String> {
    let index_text = get_text(client, "https://rl.algoux.cn/api/v2/public/contests", browser_headers()).await
        .map_err(|e| format!("读取 RankLand 榜单索引失败：{e}"))?;
    let index: serde_json::Value = serde_json::from_str(&index_text).map_err(|e| format!("解析 RankLand 榜单索引失败：{e}"))?;
    let boards: Vec<_> = index.pointer("/data/contests").and_then(serde_json::Value::as_array).into_iter().flatten().filter_map(|item| {
        let uk = item.get("uk")?.as_str()?.to_string();
        let file_id = item.get("srkFileID")?.as_str()?.to_string();
        let mut labels = vec![uk.clone(), item.get("name").and_then(serde_json::Value::as_str).unwrap_or_default().to_string()];
        if let Some(title) = item.get("title").and_then(serde_json::Value::as_object) {
            labels.extend(title.values().filter_map(serde_json::Value::as_str).map(str::to_string));
        }
        let date = item.get("startAt").and_then(serde_json::Value::as_str).and_then(|value| value.get(..10)).unwrap_or_default().to_string();
        Some(RanklandBoard { uk, file_id, direct_url: None, text: labels.join(" "), date })
    }).collect();
    let mut updated = fill_from_srk_boards(client, cache_path, contests, &boards, "RankLand").await?;

    // RankLand's public index can lag behind its open-source collection. Read
    // the collection tree as an independent fallback so newly contributed and
    // older boards can cover otherwise-unrated contests immediately.
    if let Ok(tree_text) = get_text(client, "https://api.github.com/repos/algoux/srk-collection/git/trees/master?recursive=1", browser_headers()).await {
        if let Ok(tree) = serde_json::from_str::<serde_json::Value>(&tree_text) {
            let collection: Vec<_> = tree.get("tree").and_then(serde_json::Value::as_array).into_iter().flatten().filter_map(|item| {
                let path = item.get("path")?.as_str()?;
                if !path.starts_with("official/") || !path.ends_with(".srk.json") || path.to_ascii_lowercase().contains("warmup") { return None; }
                Some(RanklandBoard {
                    uk: path.to_string(),
                    file_id: String::new(),
                    direct_url: Some(format!("https://raw.githubusercontent.com/algoux/srk-collection/master/{path}")),
                    text: path.replace(['/', '-', '_'], " "),
                    date: String::new(),
                })
            }).collect();
            updated += fill_from_srk_boards(client, cache_path, contests, &collection, "SRK Collection").await?;
        }
    }
    Ok(updated)
}

async fn fill_from_srk_boards(client: &Client, cache_path: &Path, contests: &mut [XcpcContest], boards: &[RanklandBoard], source: &str) -> Result<usize, String> {
    let targets: Vec<_> = contests.iter().enumerate()
        .filter(|(_, contest)| !contest.problems.is_empty() && contest.problems.iter().any(|problem| problem.tier.is_none()))
        .filter_map(|(index, contest)| best_rankland_board(contest, boards).map(|board| (index, board.clone())))
        .collect();
    let mut updated = 0;
    for chunk in targets.chunks(6) {
        let mut tasks = tokio::task::JoinSet::new();
        for (index, board) in chunk.iter().cloned() {
            let client = client.clone();
            tasks.spawn(async move { fetch_rankland_stats(&client, &board).await.map(|stats| (index, board, stats)) });
        }
        while let Some(result) = tasks.join_next().await {
            let Ok(Ok((index, board, stats))) = result else { continue };
            let contest = &mut contests[index];
            let mut filled = false;
            for problem in &mut contest.problems {
                if problem.tier.is_none() {
                    let Some((accepted, total, tier)) = stats.get(&problem.index) else { continue };
                    problem.accepted_teams = Some(*accepted);
                    problem.total_teams = Some(*total);
                    problem.tier = Some(tier.clone());
                    filled = true;
                }
            }
            if filled {
                append_board_source(contest, source);
                if contest.date.is_empty() { contest.date = board.date; }
                updated += 1;
            }
        }
        save_catalog(cache_path, contests)?;
    }
    Ok(updated)
}

pub async fn sync_public_ratings(client: &Client, cache_path: &Path, contests: &mut [XcpcContest]) -> Result<usize, String> {
    let mut updated = 0;
    let mut errors = Vec::new();
    // Prefer XCPCIO's official-team data, then let RankLand fill individual
    // problems that are absent from that board instead of skipping the contest.
    match sync_xcpcio_ratings(client, cache_path, contests).await {
        Ok(count) => updated += count,
        Err(error) => errors.push(error),
    }
    match sync_rankland_ratings(client, cache_path, contests).await {
        Ok(count) => updated += count,
        Err(error) => errors.push(error),
    }
    if updated == 0 && errors.len() == 2 && !contests.iter().any(|contest| contest.board_source.is_some()) {
        return Err(format!("公开榜单源均不可用：{}", errors.join("；")));
    }
    save_catalog(cache_path, contests)?;
    Ok(updated)
}

fn save_catalog(cache_path: &Path, contests: &[XcpcContest]) -> Result<(), String> {
    let json = serde_json::to_string(&CatalogCache { version: CATALOG_CACHE_VERSION, contests: contests.to_vec() })
        .map_err(|e| format!("序列化 XCPC 目录失败：{e}"))?;
    std::fs::write(cache_path, json).map_err(|e| format!("保存 XCPC 目录失败：{e}"))
}

fn append_board_source(contest: &mut XcpcContest, source: &str) {
    let sources: Vec<_> = contest.board_source.as_deref().unwrap_or_default().split(" + ").collect();
    if sources.iter().any(|current| *current == source) { return; }
    contest.board_source = Some(if sources.iter().all(|current| current.is_empty()) {
        source.to_string()
    } else {
        format!("{} + {source}", contest.board_source.as_deref().unwrap_or_default())
    });
}

async fn sync_xcpcio_ratings(client: &Client, cache_path: &Path, contests: &mut [XcpcContest]) -> Result<usize, String> {
    let tree_text = get_text(client, "https://api.github.com/repos/xcpcio/board-data/git/trees/main?recursive=1", browser_headers()).await
        .map_err(|error| format!("读取 XCPCIO 榜单目录失败：{error}"))?;
    let tree: serde_json::Value = serde_json::from_str(&tree_text).map_err(|error| format!("解析 XCPCIO 榜单目录失败：{error}"))?;
    let boards: Vec<_> = tree.get("tree").and_then(serde_json::Value::as_array).into_iter().flatten().filter_map(|item| {
        let path = item.get("path")?.as_str()?;
        if !path.starts_with("data/") || !path.ends_with("/config.json") || path.contains("warmup") { return None; }
        let directory = path.trim_end_matches("/config.json").to_string();
        Some(XcpcioBoard { text: directory.replace(['/', '-', '_'], " "), directory })
    }).collect();
    let targets: Vec<_> = contests.iter().enumerate()
        .filter(|(_, contest)| !contest.problems.is_empty())
        .filter_map(|(index, contest)| best_xcpcio_board(contest, &boards).map(|board| (index, board.clone())))
        .collect();
    let mut updated = 0;
    for chunk in targets.chunks(6) {
        let mut tasks = tokio::task::JoinSet::new();
        for (index, board) in chunk.iter().cloned() {
            let client = client.clone();
            tasks.spawn(async move { fetch_xcpcio_stats(&client, &board).await.map(|result| (index, result)) });
        }
        while let Some(result) = tasks.join_next().await {
            let Ok(Ok((index, (stats, date)))) = result else { continue };
            let contest = &mut contests[index];
            let mut filled = false;
            for problem in &mut contest.problems {
                if let Some((accepted, total, tier)) = stats.get(&problem.index) {
                    problem.accepted_teams = Some(*accepted);
                    problem.total_teams = Some(*total);
                    problem.tier = Some(tier.clone());
                    filled = true;
                }
            }
            if filled {
                append_board_source(contest, "XCPCIO");
                if contest.date.is_empty() { contest.date = date; }
                updated += 1;
            }
        }
        save_catalog(cache_path, contests)?;
    }
    Ok(updated)
}

fn best_xcpcio_board<'a>(contest: &XcpcContest, boards: &'a [XcpcioBoard]) -> Option<&'a XcpcioBoard> {
    let mut scored: Vec<_> = boards.iter().filter_map(|board| xcpcio_match_score(contest, board).map(|score| (score, board))).collect();
    scored.sort_by(|left, right| right.0.cmp(&left.0));
    match scored.as_slice() {
        // A national ICPC/CCPC board scores 5 (year) + 4 (series) without a
        // provincial site or stage qualifier. Keep that common case eligible.
        [(score, board), ..] if *score >= 9 && (*score >= 13 || scored.len() == 1 || *score > scored[1].0) => Some(*board),
        _ => None,
    }
}

fn xcpcio_match_score(contest: &XcpcContest, board: &XcpcioBoard) -> Option<i32> {
    let path = normalize_match_text(&board.text);
    let year = contest.year.parse::<i32>().ok()?;
    let edition = if contest.series.iter().any(|series| series == "ICPC") {
        Some(year - 1975)
    } else if contest.series.iter().any(|series| series == "CCPC") {
        Some(year - 2014)
    } else {
        None
    };
    let edition_matches = edition.is_some_and(|edition| edition > 0 &&
        ["st", "nd", "rd", "th"].iter().any(|suffix| path.contains(&format!("{edition}{suffix}"))));
    if !path.contains(&contest.year) && !edition_matches { return None; }
    let mut score = 5;
    if contest.series.iter().any(|series| series == "ICPC") {
        if !path.contains("icpc") { return None; }
        score += 4;
    } else if contest.series.iter().any(|series| series == "CCPC") {
        if !path.contains("ccpc") { return None; }
        score += 4;
    } else if contest.series.iter().any(|series| series == "省赛") {
        if !path.contains("provincialcontest") { return None; }
        score += 3;
    }
    if contest.site != "全国" {
        if site_match_aliases(&contest.site).iter().any(|alias| path.contains(alias)) { score += 6; } else { return None; }
    }
    let stage_terms: &[&str] = match contest.stage.as_str() {
        "网络赛" => &["onlinequalification", "qualificationround", "online"],
        "邀请赛" => &["invitational"],
        "总决赛" => &["final", "finals"],
        _ => &[],
    };
    if !stage_terms.is_empty() {
        if stage_terms.iter().any(|term| path.contains(term)) { score += 4; } else { return None; }
    } else if ["warmup", "onlinequalification", "qualificationround"].iter().any(|term| path.contains(term)) {
        return None;
    }
    Some(score)
}

fn json_id(value: &serde_json::Value) -> Option<String> {
    value.as_str().map(str::to_string).or_else(|| value.as_i64().map(|id| id.to_string()))
}

async fn fetch_xcpcio_stats(client: &Client, board: &XcpcioBoard) -> Result<(HashMap<String, (i64, i64, String)>, String), String> {
    let base = format!("https://raw.githubusercontent.com/xcpcio/board-data/main/{}/", board.directory);
    let config: serde_json::Value = fetch_json_file(client, &format!("{base}config.json")).await?;
    let teams: serde_json::Value = fetch_json_file(client, &format!("{base}team.json")).await?;
    let runs: serde_json::Value = fetch_json_file(client, &format!("{base}run.json")).await?;
    let all_teams: Vec<&serde_json::Value> = if let Some(items) = teams.as_array() {
        items.iter().collect()
    } else if let Some(items) = teams.as_object() {
        items.values().collect()
    } else {
        return Err("XCPCIO 队伍数据格式异常".to_string());
    };
    let official = xcpcio_team_ids(&all_teams, true);
    let included = if official.is_empty() { xcpcio_team_ids(&all_teams, false) } else { official };
    let total = included.len() as i64;
    if total <= 0 { return Ok((HashMap::new(), String::new())); }
    let labels: HashMap<_, _> = config.get("problems").and_then(serde_json::Value::as_array).into_iter().flatten().filter_map(|problem| {
        Some((json_id(problem.get("id")?)?, problem.get("label")?.as_str()?.to_ascii_uppercase()))
    }).collect();
    let stats = xcpcio_problem_stats(&runs, &labels, &included, total);
    let date = config.get("start_time").and_then(serde_json::Value::as_i64)
        .map(|timestamp| if timestamp > 10_000_000_000 { timestamp / 1000 } else { timestamp })
        .and_then(|timestamp| chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp, 0))
        .map(|value| value.format("%Y-%m-%d").to_string()).unwrap_or_default();
    Ok((stats, date))
}

async fn fetch_json_file(client: &Client, url: &str) -> Result<serde_json::Value, String> {
    let text = get_text(client, url, browser_headers()).await.map_err(|error| error.to_string())?;
    serde_json::from_str(&text).map_err(|error| error.to_string())
}

fn xcpcio_team_ids(teams: &[&serde_json::Value], official_only: bool) -> HashSet<String> {
    teams.iter().filter(|team| {
        !official_only || team.get("official").is_some_and(|value| value.as_i64().unwrap_or(1) != 0)
            || team.get("group").and_then(serde_json::Value::as_array)
                .is_some_and(|groups| groups.iter().any(|group| group.as_str() == Some("official")))
    }).filter_map(|team| team.get("team_id").and_then(json_id)).collect()
}

fn xcpcio_problem_stats(
    runs: &serde_json::Value,
    labels: &HashMap<String, String>,
    included: &HashSet<String>,
    total: i64,
) -> HashMap<String, (i64, i64, String)> {
    let mut solved = HashMap::<String, HashSet<String>>::new();
    for run in runs.as_array().into_iter().flatten() {
        let status = run.get("status").and_then(serde_json::Value::as_str).unwrap_or_default();
        if !status.eq_ignore_ascii_case("accepted") && !status.eq_ignore_ascii_case("correct") { continue; }
        let Some(team_id) = run.get("team_id").and_then(json_id) else { continue };
        if !included.contains(&team_id) { continue; }
        let Some(problem_id) = run.get("problem_id").and_then(json_id) else { continue };
        solved.entry(problem_id).or_default().insert(team_id);
    }
    labels.iter().map(|(problem_id, label)| {
        let accepted = solved.get(problem_id).map(|teams| teams.len() as i64).unwrap_or_default();
        (label.clone(), (accepted, total, rating_tier(accepted, total).into()))
    }).collect()
}

fn best_rankland_board<'a>(contest: &XcpcContest, boards: &'a [RanklandBoard]) -> Option<&'a RanklandBoard> {
    let mut scored: Vec<_> = boards.iter().filter_map(|board| board_match_score(contest, board).map(|score| (score, board))).collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    match scored.as_slice() {
        [(score, board), ..] if *score >= 10 && (*score >= 13 || scored.len() == 1 || *score > scored[1].0) => Some(*board),
        _ => None,
    }
}

fn board_match_score(contest: &XcpcContest, board: &RanklandBoard) -> Option<i32> {
    let text = normalize_match_text(&board.text);
    if contest.year == "未知" || !text.contains(&contest.year) { return None; }
    let mut score = 5;
    if contest.series.iter().any(|value| value == "ICPC") {
        if !text.contains("icpc") { return None; }
        score += 4;
    } else if contest.series.iter().any(|value| value == "CCPC") {
        if !text.contains("ccpc") { return None; }
        score += 4;
    }
    if contest.site != "全国" {
        let aliases = site_match_aliases(&contest.site);
        if aliases.iter().any(|alias| text.contains(alias)) { score += 6; } else { return None; }
    }
    let stage_terms: &[&str] = match contest.stage.as_str() {
        "网络赛" => &["preliminary", "online", "网络", "预选"],
        "邀请赛" => &["invitational", "邀请"],
        "总决赛" => &["final", "总决赛", "总决"],
        "省赛" => &["provincial", "省赛", "大学生程序设计"],
        _ => &[],
    };
    if !stage_terms.is_empty() {
        if stage_terms.iter().any(|term| text.contains(&normalize_match_text(term))) { score += 4; } else { return None; }
    } else if ["preliminary", "online", "网络", "预选", "invitational", "邀请", "final", "总决"].iter().any(|term| text.contains(term)) {
        return None;
    }
    let qoj_name = normalize_match_text(&contest.name);
    if qoj_name.contains(&normalize_match_text(&board.uk)) || text.contains(&qoj_name) { score += 3; }
    Some(score)
}

fn normalize_match_text(value: &str) -> String {
    value.chars().flat_map(char::to_lowercase).filter(|ch| ch.is_alphanumeric()).collect()
}

fn site_match_aliases(site: &str) -> Vec<String> {
    let english = match site {
        "北京" => "beijing", "长春" => "changchun", "成都" => "chengdu", "重庆" => "chongqing", "福建" => "fujian",
        "广东" => "guangdong", "广州" => "guangzhou", "杭州" => "hangzhou", "哈尔滨" => "harbin", "合肥" => "hefei",
        "香港" => "hongkong", "济南" => "jinan", "昆明" => "kunming", "南昌" => "nanchang", "南京" => "nanjing",
        "青岛" => "qingdao", "上海" => "shanghai", "沈阳" => "shenyang", "武汉" => "wuhan", "西安" => "xian",
        "郑州" => "zhengzhou", "浙江" => "zhejiang", "安徽" => "anhui", "澳门" => "macau", "甘肃" => "gansu",
        "广西" => "guangxi", "贵州" => "guizhou", "海南" => "hainan", "河北" => "hebei", "河南" => "henan",
        "黑龙江" => "heilongjiang", "湖北" => "hubei", "湖南" => "hunan", "吉林" => "jilin", "江苏" => "jiangsu",
        "江西" => "jiangxi", "辽宁" => "liaoning", "内蒙古" => "inner mongolia", "宁夏" => "ningxia", "青海" => "qinghai",
        "山东" => "shandong", "山西" => "shanxi", "陕西" => "shaanxi", "四川" => "sichuan", "天津" => "tianjin",
        "西藏" => "tibet", "新疆" => "xinjiang", "云南" => "yunnan", "深圳" => "shenzhen", "长沙" => "changsha",
        "桂林" => "guilin", "秦皇岛" => "qinhuangdao", "徐州" => "xuzhou", "威海" => "weihai", _ => "",
    };
    [normalize_match_text(site), normalize_match_text(english)].into_iter().filter(|value| !value.is_empty()).collect()
}

async fn fetch_rankland_stats(client: &Client, board: &RanklandBoard) -> Result<HashMap<String, (i64, i64, String)>, String> {
    let file_url = if let Some(url) = board.direct_url.as_deref() {
        url.to_string()
    } else {
        let metadata_url = format!("https://rl.algoux.cn/api/v2/public/files/{}", board.file_id);
        let metadata_text = get_text(client, &metadata_url, browser_headers()).await.map_err(|e| e.to_string())?;
        let metadata: serde_json::Value = serde_json::from_str(&metadata_text).map_err(|e| e.to_string())?;
        metadata.pointer("/data/url").and_then(serde_json::Value::as_str).ok_or_else(|| "RankLand 榜单缺少下载地址".to_string())?.to_string()
    };
    let srk_text = get_text(client, &file_url, browser_headers()).await.map_err(|e| e.to_string())?;
    let srk: serde_json::Value = serde_json::from_str(&srk_text).map_err(|e| e.to_string())?;
    let total = srk.get("rows").and_then(serde_json::Value::as_array).map(|rows| rows.len() as i64).unwrap_or_default();
    if total <= 0 { return Ok(HashMap::new()); }
    let mut stats = HashMap::new();
    for problem in srk.get("problems").and_then(serde_json::Value::as_array).into_iter().flatten() {
        let Some(alias) = problem.get("alias").and_then(serde_json::Value::as_str) else { continue };
        let accepted = problem.pointer("/statistics/accepted").and_then(serde_json::Value::as_i64).unwrap_or_default();
        stats.insert(alias.to_ascii_uppercase(), (accepted, total, rating_tier(accepted, total).into()));
    }
    Ok(stats)
}

fn rating_tier(accepted: i64, total: i64) -> &'static str {
    let scaled = accepted.max(0).saturating_mul(100);
    if scaled <= total.saturating_mul(10) { "gold" }
    else if scaled <= total.saturating_mul(30) { "silver" }
    else if scaled <= total.saturating_mul(60) { "bronze" }
    else { "iron" }
}

async fn fetch_catalog(client: &Client, cookie: &str) -> Result<Vec<XcpcContest>, String> {
    let mut contests = HashMap::<String, XcpcContest>::new();
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();
    for (id, depth) in ROOT_CATEGORIES { queue.push_back((id.to_string(), depth)); }
    while !queue.is_empty() {
        let mut batch = Vec::new();
        while let Some((category_id, depth)) = queue.pop_front() {
            if visited.insert(category_id.clone()) { batch.push((category_id, depth)); }
        }
        for chunk in batch.chunks(8) {
            let mut tasks = tokio::task::JoinSet::new();
            for (category_id, depth) in chunk.iter().cloned() {
                let client = client.clone();
                let cookie = cookie.to_string();
                tasks.spawn(async move {
                    let url = format!("https://qoj.ac/category/{category_id}");
                    let html = get_text(&client, &url, with_cookie(browser_headers(), &cookie)).await
                        .map_err(|e| format!("更新 XCPC 目录失败：{e}"))?;
                    Ok::<_, String>((depth, parse_category(&html)))
                });
            }
            while let Some(result) = tasks.join_next().await {
                let (depth, page) = result.map_err(|e| format!("更新 XCPC 目录任务失败：{e}"))??;
                for contest in page.contests { contests.entry(contest.id.clone()).or_insert(contest); }
                if depth > 0 {
                    for child in page.child_categories {
                        if !visited.contains(&child) { queue.push_back((child, depth - 1)); }
                    }
                }
            }
        }
    }
    let mut items: Vec<_> = contests.into_values().collect();
    items.retain(|contest| !is_warmup(&contest.name));
    items.sort_by(|a, b| b.year.cmp(&a.year).then_with(|| numeric_id(&b.id).cmp(&numeric_id(&a.id))).then_with(|| a.name.cmp(&b.name)));
    if items.is_empty() {
        // QOJ occasionally serves the category table without the problem-column
        // markup. Keep the tracker usable by falling back to its public contest list.
        items = fetch_contest_list(client, cookie).await?;
    }
    if items.is_empty() { return Err("QOJ 页面已返回，但没有识别到 XCPC 比赛；请稍后重试".into()); }
    Ok(items)
}

async fn enrich_contest_problems(client: &Client, cookie: &str, contests: &mut [XcpcContest]) {
    let missing: Vec<_> = contests.iter().enumerate()
        .filter(|(_, contest)| contest_needs_problem_details(contest))
        .map(|(index, contest)| (index, contest.id.clone()))
        .collect();
    for chunk in missing.chunks(8) {
        let mut tasks = tokio::task::JoinSet::new();
        for (index, contest_id) in chunk.iter().cloned() {
            let client = client.clone();
            let cookie = cookie.to_string();
            tasks.spawn(async move {
                let url = format!("https://qoj.ac/contest/{contest_id}");
                let problems = fetch_contest_problems(&client, &url, &cookie).await?;
                Ok::<_, String>((index, problems))
            });
        }
        while let Some(result) = tasks.join_next().await {
            if let Ok(Ok((index, problems))) = result {
                if !problems.is_empty() {
                    let previous: HashMap<_, _> = contests[index].problems.iter()
                        .map(|problem| (problem.problem_id.clone(), problem.clone()))
                        .collect();
                    contests[index].problems = problems.into_iter().map(|mut problem| {
                        if let Some(old) = previous.get(&problem.problem_id) {
                            problem.tier = old.tier.clone();
                            problem.accepted_teams = old.accepted_teams;
                            problem.total_teams = old.total_teams;
                            problem.solved = old.solved;
                        }
                        problem
                    }).collect();
                }
            }
        }
    }
}

fn contest_needs_problem_details(contest: &XcpcContest) -> bool {
    contest.problems.is_empty() || contest.problems.iter().any(|problem| problem_name_is_missing(&problem.name))
}

fn problem_name_is_missing(raw: &str) -> bool {
    let name = clean_problem_name(raw);
    name.is_empty() || name == "题目" || name.eq_ignore_ascii_case("problem")
}

async fn fetch_contest_problems(client: &Client, url: &str, cookie: &str) -> Result<Vec<XcpcProblem>, String> {
    let html = get_text(client, url, with_cookie(browser_headers(), cookie)).await
        .map_err(|e| e.to_string())?;
    let doc = Html::parse_document(&html);
    let anchor_sel = Selector::parse("a[href]").unwrap();
    let problem_re = Regex::new(r"^(?:https?://qoj\.ac)?/(?:contest/\d+/)?problem/(\d+)(?:$|[/?#])").unwrap();
    let mut problems: Vec<XcpcProblem> = Vec::new();
    let mut positions = HashMap::<String, usize>::new();
    for anchor in doc.select(&anchor_sel) {
        let Some(href) = anchor.value().attr("href") else { continue };
        let Some(problem_id) = problem_re.captures(href).map(|capture| capture[1].to_string()) else { continue };
        let raw = text_of(&anchor);
        let (mut index, parsed_name) = problem_label(&raw, problems.len());
        let title = anchor.value().attr("title").or_else(|| anchor.value().attr("data-original-title")).unwrap_or_default().trim();
        let name = if let Some((title_index, title_name)) = explicit_problem_label(title, problems.len()) {
            index = title_index;
            title_name
        } else if !title.is_empty() {
            clean_problem_name(title)
        } else {
            parsed_name
        };
        let mut candidate = XcpcProblem { index, name, url: format!("https://qoj.ac/problem/{problem_id}"), problem_id: problem_id.clone(), tier: None, accepted_teams: None, total_teams: None, solved: false };
        if let Some(position) = positions.get(&problem_id).copied() {
            if problem_name_is_missing(&problems[position].name) && !problem_name_is_missing(&candidate.name) {
                candidate.index = problems[position].index.clone();
                problems[position] = candidate;
            }
        } else {
            positions.insert(problem_id, problems.len());
            problems.push(candidate);
        }
    }
    sort_problems(&mut problems);
    Ok(problems)
}

struct ParsedCategory { contests: Vec<XcpcContest>, child_categories: Vec<String> }

fn parse_category(html: &str) -> ParsedCategory {
    let doc = Html::parse_document(html);
    let row_sel = Selector::parse("table tr").unwrap();
    let anchor_sel = Selector::parse("a[href]").unwrap();
    let contest_re = Regex::new(r"^(?:https?://qoj\.ac)?/contest/(\d+)(?:$|[/?#])").unwrap();
    let problem_re = Regex::new(r"^(?:https?://qoj\.ac)?/(?:contest/\d+/)?problem/(\d+)(?:$|[/?#])").unwrap();
    let category_re = Regex::new(r"^(?:https?://qoj\.ac)?/category/(\d+)(?:$|[/?#])").unwrap();
    let mut contests = Vec::new();
    let mut child_categories = Vec::new();
    for row in doc.select(&row_sel) {
        let anchors: Vec<_> = row.select(&anchor_sel).collect();
        let contest_anchor = anchors.iter().find(|anchor| anchor.value().attr("href").is_some_and(|href| contest_re.is_match(href)));
        let Some(contest_anchor) = contest_anchor else {
            for anchor in anchors {
                if let Some(id) = anchor.value().attr("href").and_then(|href| category_re.captures(href)).map(|captures| captures[1].to_string()) { child_categories.push(id); }
            }
            continue;
        };
        let contest_href = contest_anchor.value().attr("href").unwrap_or_default();
        let Some(contest_id) = contest_re.captures(contest_href).map(|captures| captures[1].to_string()) else { continue };
        let name = text_of(contest_anchor);
        if name.is_empty() { continue; }
        let mut problems = Vec::new();
        for anchor in anchors {
            let Some(href) = anchor.value().attr("href") else { continue };
            let Some(problem_id) = problem_re.captures(href).map(|captures| captures[1].to_string()) else { continue };
            let raw_index = text_of(&anchor);
            let (mut index, parsed_name) = problem_label(&raw_index, problems.len());
            let title = anchor.value().attr("title").or_else(|| anchor.value().attr("data-original-title")).or_else(|| anchor.value().attr("aria-label")).unwrap_or_default().trim();
            let problem_name = if let Some((title_index, title_name)) = explicit_problem_label(title, problems.len()) {
                index = title_index;
                title_name
            } else if !title.is_empty() {
                clean_problem_name(title)
            } else {
                parsed_name
            };
            problems.push(XcpcProblem { index, name: problem_name, url: format!("https://qoj.ac/problem/{problem_id}"), problem_id, tier: None, accepted_teams: None, total_teams: None, solved: false });
        }
        sort_problems(&mut problems);
        let year = extract_year(&name);
        contests.push(XcpcContest {
            id: contest_id.clone(), name: name.clone(), short_name: short_name(&name, &year), url: format!("https://qoj.ac/contest/{contest_id}"),
            date: String::new(), year, series: classify_series(&name), stage: classify_stage(&name), site: classify_site(&name), board_source: None, problems,
        });
    }
    let parsed = ParsedCategory { contests, child_categories };
    if parsed.contests.is_empty() || parsed.contests.iter().all(|contest| contest.problems.is_empty()) {
        let fallback = parse_category_rows_from_html(html);
        if !fallback.contests.is_empty() || !fallback.child_categories.is_empty() { return fallback; }
    }
    parsed
}

fn parse_category_rows_from_html(html: &str) -> ParsedCategory {
    let row_re = Regex::new(r"(?is)<tr\b[^>]*>(.*?)</tr>").unwrap();
    let anchor_re = Regex::new(r#"(?is)<a\b[^>]*href\s*=\s*["']([^"']+)["'][^>]*>(.*?)</a>"#).unwrap();
    let title_re = Regex::new(r#"(?is)\b(?:title|data-original-title|aria-label)\s*=\s*["']([^"']+)["']"#).unwrap();
    let tag_re = Regex::new(r"(?is)<[^>]+>").unwrap();
    let contest_re = Regex::new(r"^(?:https?://qoj\.ac)?/contest/(\d+)(?:$|[/?#])").unwrap();
    let problem_re = Regex::new(r"^(?:https?://qoj\.ac)?/(?:contest/\d+/)?problem/(\d+)(?:$|[/?#])").unwrap();
    let category_re = Regex::new(r"^(?:https?://qoj\.ac)?/category/(\d+)(?:$|[/?#])").unwrap();
    let mut contests = Vec::new();
    let mut child_categories = Vec::new();
    for capture in row_re.captures_iter(html) {
        let row = capture.get(1).map(|match_| match_.as_str()).unwrap_or_default();
        let anchors: Vec<_> = anchor_re.captures_iter(row).collect();
        let contest = anchors.iter().find(|anchor| contest_re.is_match(&anchor[1]));
        let Some(contest) = contest else {
            for anchor in anchors { if let Some(id) = category_re.captures(&anchor[1]).map(|capture| capture[1].to_string()) { child_categories.push(id); } }
            continue;
        };
        let Some(id) = contest_re.captures(&contest[1]).map(|capture| capture[1].to_string()) else { continue };
        let name = tag_re.replace_all(&contest[2], " ").split_whitespace().collect::<Vec<_>>().join(" ");
        if name.is_empty() { continue; }
        let mut problems = Vec::new();
        for anchor in anchors {
            let Some(problem_id) = problem_re.captures(&anchor[1]).map(|capture| capture[1].to_string()) else { continue };
            let raw = tag_re.replace_all(&anchor[2], " ").split_whitespace().collect::<Vec<_>>().join(" ");
            let (mut index, parsed_name) = problem_label(&raw, problems.len());
            let title = title_re.captures(&anchor[0]).map(|capture| capture[1].trim().to_string()).unwrap_or_default();
            let name = if let Some((title_index, title_name)) = explicit_problem_label(&title, problems.len()) {
                index = title_index;
                title_name
            } else if !title.is_empty() {
                clean_problem_name(title)
            } else {
                parsed_name
            };
            problems.push(XcpcProblem { index, name, url: format!("https://qoj.ac/problem/{problem_id}"), problem_id, tier: None, accepted_teams: None, total_teams: None, solved: false });
        }
        sort_problems(&mut problems);
        let year = extract_year(&name);
        contests.push(XcpcContest { id: id.clone(), name: name.clone(), short_name: short_name(&name, &year), url: format!("https://qoj.ac/contest/{id}"), date: String::new(), year, series: classify_series(&name), stage: classify_stage(&name), site: classify_site(&name), board_source: None, problems });
    }
    ParsedCategory { contests, child_categories }
}

async fn fetch_contest_list(client: &Client, cookie: &str) -> Result<Vec<XcpcContest>, String> {
    let html = get_text(client, "https://qoj.ac/contests?tab=icpc", with_cookie(browser_headers(), cookie))
        .await
        .map_err(|e| format!("更新 XCPC 目录失败：{e}"))?;
    let doc = Html::parse_document(&html);
    let row_sel = Selector::parse("table tr").unwrap();
    let anchor_sel = Selector::parse("a[href]").unwrap();
    let contest_re = Regex::new(r"^(?:https?://qoj\.ac)?/contest/(\d+)(?:$|[/?#])").unwrap();
    let mut items = Vec::new();
    for row in doc.select(&row_sel) {
        let Some(anchor) = row.select(&anchor_sel).find(|a| a.value().attr("href").is_some_and(|h| contest_re.is_match(h))) else { continue };
        let Some(id) = anchor.value().attr("href").and_then(|h| contest_re.captures(h)).map(|c| c[1].to_string()) else { continue };
        let name = text_of(&anchor);
        if name.is_empty() { continue; }
        let year = extract_year(&name);
        items.push(XcpcContest { id: id.clone(), name: name.clone(), short_name: short_name(&name, &year), url: format!("https://qoj.ac/contest/{id}"), date: String::new(), year, series: classify_series(&name), stage: classify_stage(&name), site: classify_site(&name), board_source: None, problems: Vec::new() });
    }
    items.retain(|contest| !is_warmup(&contest.name));
    items.sort_by(|a, b| b.year.cmp(&a.year).then_with(|| numeric_id(&b.id).cmp(&numeric_id(&a.id))));
    Ok(items)
}

fn text_of(element: &scraper::ElementRef<'_>) -> String { element.text().collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ") }
fn fallback_problem_index(position: usize) -> String {
    let mut value = position + 1;
    let mut label = String::new();
    while value > 0 {
        value -= 1;
        label.insert(0, (b'A' + (value % 26) as u8) as char);
        value /= 26;
    }
    label
}
fn problem_label(raw: &str, position: usize) -> (String, String) {
    let text = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let expected = fallback_problem_index(position);
    if text.eq_ignore_ascii_case(&expected) { return (expected, String::new()); }
    if let Some((_, name)) = explicit_problem_label(&text, position) { return (expected, name); }
    (expected, text)
}
fn explicit_problem_label(raw: &str, position: usize) -> Option<(String, String)> {
    let text = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let expected = fallback_problem_index(position);
    // A prefix is a problem label only when it agrees with the position in the
    // contest. This keeps real titles such as "MOD. Modular" and "GG: Game"
    // intact instead of trying to maintain a fragile list of exceptions.
    let label_re = Regex::new(r"(?i)^(?:(?:problem\s+([a-z]+)(?:\s*[.．:：]\s*|\s+))|(?:([a-z]+)\s*[.．:：]\s*))(.+)$").unwrap();
    let capture = label_re.captures(&text)?;
    let label = capture.get(1).or_else(|| capture.get(2))?.as_str();
    if !label.eq_ignore_ascii_case(&expected) { return None; }
    Some((expected, capture[3].trim().to_string()))
}
fn clean_problem_name(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}
fn problem_index_rank(index: &str) -> u32 {
    let upper = index.to_ascii_uppercase();
    if !upper.is_empty() && upper.chars().all(|ch| ch.is_ascii_uppercase()) {
        return upper.bytes().fold(0u32, |value, ch| value.saturating_mul(26).saturating_add((ch - b'A' + 1) as u32));
    }
    upper.parse::<u32>().unwrap_or(u32::MAX)
}
fn sort_problems(problems: &mut [XcpcProblem]) {
    problems.sort_by(|a, b| problem_index_rank(&a.index).cmp(&problem_index_rank(&b.index)).then_with(|| a.problem_id.cmp(&b.problem_id)));
}
fn cached_problems_are_well_formed(problems: &[XcpcProblem]) -> bool {
    !problems.is_empty() && problems.iter().enumerate().all(|(position, problem)| {
        problem.index == fallback_problem_index(position) && clean_problem_name(&problem.name) == problem.name
    })
}
fn extract_year(name: &str) -> String { Regex::new(r"(?:19|20)\d{2}").unwrap().find(name).map(|value| value.as_str().to_string()).unwrap_or_else(|| "未知".into()) }

fn classify_series(name: &str) -> Vec<String> {
    let low = name.to_ascii_lowercase();
    let mut values = Vec::new();
    if low.contains("icpc") { values.push("ICPC".into()); }
    if low.contains("ccpc") || name.contains("中国大学生程序设计竞赛") { values.push("CCPC".into()); }
    if low.contains("provincial") || low.contains("province programming") || name.contains("省赛") || name.contains("省大学生") { values.push("省赛".into()); }
    if values.is_empty() { values.push("其他".into()); }
    values
}

fn classify_stage(name: &str) -> String {
    let low = name.to_ascii_lowercase();
    if low.contains("online") || name.contains("网络") { "网络赛" }
    else if low.contains("invitational") || name.contains("邀请赛") { "邀请赛" }
    else if low.contains("provincial") || low.contains("province programming") || name.contains("省赛") || name.contains("省大学生") { "省赛" }
    else if low.contains("final") || name.contains("总决赛") { "总决赛" }
    else if low.contains("regional") { "区域赛" }
    else if low.contains("site") || name.contains('站') { "分站赛" }
    else { "其他" }.into()
}

fn classify_site(name: &str) -> String {
    const SITES: &[(&str, &str)] = &[
        ("Beijing", "北京"), ("北京", "北京"), ("Changchun", "长春"), ("长春", "长春"), ("Chengdu", "成都"), ("成都", "成都"),
        ("Chongqing", "重庆"), ("重庆", "重庆"), ("Fujian", "福建"), ("Guangdong", "广东"), ("Guangzhou", "广州"), ("广州", "广州"),
        ("Hangzhou", "杭州"), ("Harbin", "哈尔滨"), ("哈尔滨", "哈尔滨"), ("Hefei", "合肥"), ("Hong Kong", "香港"),
        ("Jinan", "济南"), ("济南", "济南"), ("Kunming", "昆明"), ("Nanchang", "南昌"), ("Nanjing", "南京"), ("南京", "南京"),
        ("Qingdao", "青岛"), ("Shanghai", "上海"), ("上海", "上海"), ("Shenyang", "沈阳"), ("沈阳", "沈阳"),
        ("Wuhan", "武汉"), ("武汉", "武汉"), ("Xi'an", "西安"), ("西安", "西安"), ("Zhengzhou", "郑州"), ("郑州", "郑州"), ("Zhejiang", "浙江"),
        ("Shenzhen", "深圳"), ("深圳", "深圳"), ("Changsha", "长沙"), ("长沙", "长沙"), ("Guilin", "桂林"), ("桂林", "桂林"),
        ("Qinhuangdao", "秦皇岛"), ("秦皇岛", "秦皇岛"), ("Xuzhou", "徐州"), ("徐州", "徐州"), ("Weihai", "威海"), ("威海", "威海"),
        ("Anhui", "安徽"), ("安徽", "安徽"), ("Fuzhou", "福建"), ("福建", "福建"), ("Gansu", "甘肃"), ("甘肃", "甘肃"),
        ("Guangxi", "广西"), ("广西", "广西"), ("Guizhou", "贵州"), ("贵州", "贵州"), ("Hainan", "海南"), ("海南", "海南"),
        ("Hebei", "河北"), ("河北", "河北"), ("Henan", "河南"), ("河南", "河南"), ("Heilongjiang", "黑龙江"), ("黑龙江", "黑龙江"),
        ("Hubei", "湖北"), ("湖北", "湖北"), ("Hunan", "湖南"), ("湖南", "湖南"), ("Jilin", "吉林"), ("吉林", "吉林"),
        ("Jiangsu", "江苏"), ("江苏", "江苏"), ("Jiangxi", "江西"), ("江西", "江西"), ("Liaoning", "辽宁"), ("辽宁", "辽宁"),
        ("Inner Mongolia", "内蒙古"), ("内蒙古", "内蒙古"), ("Ningxia", "宁夏"), ("宁夏", "宁夏"), ("Qinghai", "青海"), ("青海", "青海"),
        ("Shandong", "山东"), ("山东", "山东"), ("Shanxi", "山西"), ("山西", "山西"), ("Shaanxi", "陕西"), ("陕西", "陕西"),
        ("Sichuan", "四川"), ("四川", "四川"), ("Tianjin", "天津"), ("天津", "天津"), ("Tibet", "西藏"), ("西藏", "西藏"),
        ("Xinjiang", "新疆"), ("新疆", "新疆"), ("Yunnan", "云南"), ("云南", "云南"), ("Macau", "澳门"), ("澳门", "澳门")
    ];
    SITES.iter().find(|(needle, _)| name.contains(needle)).map(|(_, label)| (*label).to_string()).unwrap_or_else(|| "全国".into())
}

fn short_name(name: &str, year: &str) -> String {
    let series = if name.to_ascii_lowercase().contains("ccpc") || name.contains("中国大学生程序设计竞赛") { "CCPC" } else if name.to_ascii_lowercase().contains("icpc") { "ICPC" } else { "XCPC" };
    format!("{year} {series} {}{}", classify_site(name), classify_stage(name))
}
fn is_warmup(name: &str) -> bool { let low = name.to_ascii_lowercase(); low.contains("warm up") || low.contains("warm-up") || low.contains("practice") || name.contains("热身") }
fn numeric_id(id: &str) -> i64 { id.parse().unwrap_or_default() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_realistic_category_rows() {
        let html = r#"<table><tr><td><a href="/contest/2513">The 2025 ICPC Asia Nanjing Regional Contest</a></td><td><a title="A. Array" href="/contest/2513/problem/14001/statement/zh_cn">A. Array</a><a href="/problem/14002">B. Bitset</a></td></tr><tr><td><a href="/category/84">Nanjing</a></td></tr></table>"#;
        let parsed = parse_category(html);
        assert_eq!(parsed.child_categories, vec!["84"]);
        assert_eq!(parsed.contests.len(), 1);
        assert_eq!(parsed.contests[0].problems[0].name, "Array");
        assert_eq!(parsed.contests[0].problems[1].index, "B");
        assert_eq!(parsed.contests[0].problems[1].name, "Bitset");
        assert_eq!(parsed.contests[0].problems[1].url, "https://qoj.ac/problem/14002");
    }

    #[test]
    fn assigns_rating_tiers_from_acceptance_ratio() {
        assert_eq!(rating_tier(10, 100), "gold");
        assert_eq!(rating_tier(30, 100), "silver");
        assert_eq!(rating_tier(60, 100), "bronze");
        assert_eq!(rating_tier(61, 100), "iron");
        assert_eq!(rating_tier(11, 101), "silver");
    }

    #[test]
    fn removes_only_the_expected_problem_label_from_titles() {
        assert_eq!(explicit_problem_label("D. Dynamic Graph", 3).unwrap().1, "Dynamic Graph");
        assert_eq!(explicit_problem_label("Problem K: Knowledge", 10).unwrap().0, "K");
        assert!(explicit_problem_label("MOD. Modular Arithmetic", 0).is_none());
    }

    #[test]
    fn keeps_indefinite_article_as_part_of_problem_name() {
        assert_eq!(problem_label("A Perfect Match", 3), ("D".into(), "A Perfect Match".into()));
        assert_eq!(problem_label("A Perfect Match", 0), ("A".into(), "A Perfect Match".into()));
        assert_eq!(problem_label("A. Perfect Match", 0), ("A".into(), "Perfect Match".into()));
        assert_eq!(problem_label("Problem B A Long Journey", 1), ("B".into(), "A Long Journey".into()));
        assert_eq!(problem_label("MOD", 0), ("A".into(), "MOD".into()));
        assert_eq!(problem_label("GG", 1), ("B".into(), "GG".into()));
        assert_eq!(problem_label("MOD. Modular Arithmetic", 0), ("A".into(), "MOD. Modular Arithmetic".into()));
        assert_eq!(fallback_problem_index(26), "AA");
    }

    #[test]
    fn requests_details_for_placeholder_problem_names() {
        let contest = XcpcContest {
            id: "1".into(), name: "Contest".into(), short_name: "Contest".into(), url: String::new(),
            date: String::new(), year: "2026".into(), series: vec!["ICPC".into()], stage: "区域赛".into(),
            site: "全国".into(), board_source: None,
            problems: vec![XcpcProblem { index: "B".into(), name: "题目".into(), url: String::new(), problem_id: "2".into(), tier: None, accepted_teams: None, total_teams: None, solved: false }],
        };
        assert!(contest_needs_problem_details(&contest));
    }

    #[test]
    fn matches_xcpcio_edition_paths_without_a_calendar_year() {
        let contest = XcpcContest {
            id: "1".into(), name: "The 2023 ICPC Asia Xi'an Regional Contest".into(), short_name: String::new(), url: String::new(),
            date: String::new(), year: "2023".into(), series: vec!["ICPC".into()], stage: "区域赛".into(),
            site: "西安".into(), board_source: None, problems: Vec::new(),
        };
        let board = XcpcioBoard { directory: "data/icpc/48th/xian".into(), text: "data icpc 48th xian".into() };
        assert!(xcpcio_match_score(&contest, &board).is_some());
    }
}
