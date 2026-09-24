async fn sync_xcpcio_ratings(
    client: &Client,
    contests: &mut [XcpcContest],
) -> Result<usize, String> {
    let tree_text = get_text(
        client,
        "https://api.github.com/repos/xcpcio/board-data/git/trees/main?recursive=1",
        browser_headers(),
    )
    .await
    .map_err(|error| format!("读取 XCPCIO 榜单目录失败：{error}"))?;
    let tree: serde_json::Value = serde_json::from_str(&tree_text)
        .map_err(|error| format!("解析 XCPCIO 榜单目录失败：{error}"))?;
    let boards: Vec<_> = tree
        .get("tree")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            let path = item.get("path")?.as_str()?;
            if !path.starts_with("data/")
                || !path.ends_with("/config.json")
                || path.contains("warmup")
            {
                return None;
            }
            let directory = path.trim_end_matches("/config.json").to_string();
            Some(XcpcioBoard {
                text: directory.replace(['/', '-', '_'], " "),
                directory,
            })
        })
        .collect();
    let targets: Vec<_> = contests
        .iter()
        .enumerate()
        .filter(|(_, contest)| !contest.problems.is_empty())
        .filter_map(|(index, contest)| {
            best_xcpcio_board(contest, &boards).map(|board| (index, board.clone()))
        })
        .collect();
    let mut updated = 0;
    let results = crate::fetch_queue::collect(targets, 6, |(index, board)| {
        let client = client.clone();
        async move {
            fetch_xcpcio_stats(&client, &board)
                .await
                .map(|result| (index, result))
        }
    })
    .await;
    for result in results {
        let Ok((index, (stats, date))) = result else {
            continue;
        };
        let contest = &mut contests[index];
        if contest.stage == "网络赛"
            && (stats.len() != contest.problems.len()
                || contest
                    .problems
                    .iter()
                    .any(|problem| !stats.contains_key(&problem.index)))
        {
            continue;
        }
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
            if contest.date.is_empty() {
                contest.date = date;
            }
            updated += 1;
        }
    }
    Ok(updated)
}

fn best_xcpcio_board<'a>(
    contest: &XcpcContest,
    boards: &'a [XcpcioBoard],
) -> Option<&'a XcpcioBoard> {
    let mut scored: Vec<_> = boards
        .iter()
        .filter_map(|board| xcpcio_match_score(contest, board).map(|score| (score, board)))
        .collect();
    scored.sort_by(|left, right| right.0.cmp(&left.0));
    match scored.as_slice() {
        // A national ICPC/CCPC board scores 5 (year) + 4 (series) without a
        // provincial site or stage qualifier. Keep that common case eligible.
        [(score, board), ..] if *score >= 9 && (scored.len() == 1 || *score > scored[1].0) => {
            Some(*board)
        }
        _ => None,
    }
}

fn xcpcio_match_score(contest: &XcpcContest, board: &XcpcioBoard) -> Option<i32> {
    if is_warmup(&board.text) {
        return None;
    }
    let path = normalize_match_text(&board.text);
    let year = contest.year.parse::<i32>().ok()?;
    let edition = if contest.series.iter().any(|series| series == "ICPC") {
        Some(year - 1975)
    } else if contest.series.iter().any(|series| series == "CCPC") {
        Some(year - 2014)
    } else {
        None
    };
    let edition_matches = edition.is_some_and(|edition| {
        edition > 0
            && ["st", "nd", "rd", "th"]
                .iter()
                .any(|suffix| path.contains(&format!("{edition}{suffix}")))
    });
    if !path.contains(&contest.year) && !edition_matches {
        return None;
    }
    let mut score = 5 + round_match_score(contest, &board.text)?;
    if contest.series.iter().any(|series| series == "ICPC") {
        if !path.contains("icpc") {
            return None;
        }
        score += 4;
    } else if contest.series.iter().any(|series| series == "CCPC") {
        if !path.contains("ccpc") {
            return None;
        }
        score += 4;
    } else if contest.series.iter().any(|series| series == "省赛") {
        if !path.contains("provincialcontest") {
            return None;
        }
        score += 3;
    }
    if contest.site != "全国" {
        if site_match_aliases(&contest.site)
            .iter()
            .any(|alias| path.contains(alias))
        {
            score += 6;
        } else {
            return None;
        }
    }
    let stage_terms: &[&str] = match contest.stage.as_str() {
        "网络赛" => &["onlinequalification", "qualificationround", "online"],
        "邀请赛" => &["invitational"],
        "总决赛" => &["final", "finals"],
        _ => &[],
    };
    if !stage_terms.is_empty() {
        if stage_terms.iter().any(|term| path.contains(term)) {
            score += 4;
        } else {
            return None;
        }
    } else if ["warmup", "onlinequalification", "qualificationround"]
        .iter()
        .any(|term| path.contains(term))
    {
        return None;
    }
    Some(score)
}

fn json_id(value: &serde_json::Value) -> Option<String> {
    value
        .as_str()
        .map(str::to_string)
        .or_else(|| value.as_i64().map(|id| id.to_string()))
}

async fn fetch_xcpcio_stats(
    client: &Client,
    board: &XcpcioBoard,
) -> Result<(HashMap<String, (i64, i64, String)>, String), String> {
    let base = format!(
        "https://raw.githubusercontent.com/xcpcio/board-data/main/{}/",
        board.directory
    );
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
    let included = if official.is_empty() {
        xcpcio_team_ids(&all_teams, false)
    } else {
        official
    };
    let total = included.len() as i64;
    if total <= 0 {
        return Ok((HashMap::new(), String::new()));
    }
    let labels: HashMap<_, _> = config
        .get("problems")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|problem| {
            Some((
                json_id(problem.get("id")?)?,
                problem.get("label")?.as_str()?.to_ascii_uppercase(),
            ))
        })
        .collect();
    let stats = xcpcio_problem_stats(&runs, &labels, &included, total);
    let date = config
        .get("start_time")
        .and_then(serde_json::Value::as_i64)
        .map(|timestamp| {
            if timestamp > 10_000_000_000 {
                timestamp / 1000
            } else {
                timestamp
            }
        })
        .and_then(|timestamp| chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp, 0))
        .map(|value| value.format("%Y-%m-%d").to_string())
        .unwrap_or_default();
    Ok((stats, date))
}

async fn fetch_json_file(client: &Client, url: &str) -> Result<serde_json::Value, String> {
    let text = get_text(client, url, browser_headers())
        .await
        .map_err(|error| error.to_string())?;
    serde_json::from_str(&text).map_err(|error| error.to_string())
}

fn xcpcio_team_ids(teams: &[&serde_json::Value], official_only: bool) -> HashSet<String> {
    teams
        .iter()
        .filter(|team| {
            !official_only
                || team
                    .get("official")
                    .is_some_and(|value| value.as_i64().unwrap_or(1) != 0)
                || team
                    .get("group")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|groups| {
                        groups
                            .iter()
                            .any(|group| group.as_str() == Some("official"))
                    })
        })
        .filter_map(|team| team.get("team_id").and_then(json_id))
        .collect()
}

fn xcpcio_problem_stats(
    runs: &serde_json::Value,
    labels: &HashMap<String, String>,
    included: &HashSet<String>,
    total: i64,
) -> HashMap<String, (i64, i64, String)> {
    let mut solved = HashMap::<String, HashSet<String>>::new();
    for run in runs.as_array().into_iter().flatten() {
        let status = run
            .get("status")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        if !status.eq_ignore_ascii_case("accepted") && !status.eq_ignore_ascii_case("correct") {
            continue;
        }
        let Some(team_id) = run.get("team_id").and_then(json_id) else {
            continue;
        };
        if !included.contains(&team_id) {
            continue;
        }
        let Some(problem_id) = run.get("problem_id").and_then(json_id) else {
            continue;
        };
        solved.entry(problem_id).or_default().insert(team_id);
    }
    labels
        .iter()
        .map(|(problem_id, label)| {
            let accepted = solved
                .get(problem_id)
                .map(|teams| teams.len() as i64)
                .unwrap_or_default();
            (
                label.clone(),
                (accepted, total, rating_tier(accepted, total).into()),
            )
        })
        .collect()
}
