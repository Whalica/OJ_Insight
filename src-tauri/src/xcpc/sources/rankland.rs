async fn sync_rankland_ratings(
    client: &Client,
    contests: &mut [XcpcContest],
) -> Result<usize, String> {
    let index_text = get_text(
        client,
        "https://rl.algoux.cn/api/v2/public/contests",
        browser_headers(),
    )
    .await
    .map_err(|e| format!("读取 RankLand 榜单索引失败：{e}"))?;
    let index: serde_json::Value = serde_json::from_str(&index_text)
        .map_err(|e| format!("解析 RankLand 榜单索引失败：{e}"))?;
    let boards: Vec<_> = index
        .pointer("/data/contests")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            let uk = item.get("uk")?.as_str()?.to_string();
            let file_id = item.get("srkFileID")?.as_str()?.to_string();
            let mut labels = vec![
                uk.clone(),
                item.get("name")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            ];
            if let Some(title) = item.get("title").and_then(serde_json::Value::as_object) {
                labels.extend(
                    title
                        .values()
                        .filter_map(serde_json::Value::as_str)
                        .map(str::to_string),
                );
            }
            let date = item
                .get("startAt")
                .and_then(serde_json::Value::as_str)
                .and_then(|value| value.get(..10))
                .unwrap_or_default()
                .to_string();
            Some(RanklandBoard {
                uk,
                file_id,
                direct_url: None,
                text: labels.join(" "),
                date,
            })
        })
        .collect();
    let mut updated = fill_from_srk_boards(client, contests, &boards, "RankLand", false).await?;

    // RankLand's public index can lag behind its open-source collection. Read
    // the collection tree as an independent fallback so newly contributed and
    // older boards can cover otherwise-unrated contests immediately.
    if let Ok(tree_text) = get_text(
        client,
        "https://api.github.com/repos/algoux/srk-collection/git/trees/master?recursive=1",
        browser_headers(),
    )
    .await
    {
        if let Ok(tree) = serde_json::from_str::<serde_json::Value>(&tree_text) {
            let collection: Vec<_> = tree
                .get("tree")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|item| {
                    let path = item.get("path")?.as_str()?;
                    if !path.starts_with("official/")
                        || !path.ends_with(".srk.json")
                        || path.to_ascii_lowercase().contains("warmup")
                    {
                        return None;
                    }
                    Some(RanklandBoard {
                        uk: path.to_string(),
                        file_id: String::new(),
                        direct_url: Some(format!(
                            "https://raw.githubusercontent.com/algoux/srk-collection/master/{path}"
                        )),
                        text: path.replace(['/', '-', '_'], " "),
                        date: String::new(),
                    })
                })
                .collect();
            updated +=
                fill_from_srk_boards(client, contests, &collection, "SRK Collection", true).await?;
        }
    }
    Ok(updated)
}

async fn fill_from_srk_boards(
    client: &Client,
    contests: &mut [XcpcContest],
    boards: &[RanklandBoard],
    source: &str,
    fallback_only: bool,
) -> Result<usize, String> {
    let targets: Vec<_> = contests
        .iter()
        .enumerate()
        .filter(|(_, contest)| !contest.problems.is_empty())
        .filter(|(_, contest)| {
            !fallback_only
                || (contest
                    .problems
                    .iter()
                    .any(|problem| problem.tier.is_none())
                    && !contest
                        .board_source
                        .as_deref()
                        .unwrap_or_default()
                        .contains("RankLand"))
        })
        .filter_map(|(index, contest)| {
            best_rankland_board(contest, boards).map(|board| (index, board.clone()))
        })
        .collect();
    let mut updated = 0;
    let results = crate::fetch_queue::collect(targets, 6, |(index, board)| {
        let client = client.clone();
        async move {
            fetch_rankland_stats(&client, &board)
                .await
                .map(|stats| (index, board, stats))
        }
    })
    .await;
    for result in results {
        let Ok((index, board, (stats, date))) = result else {
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
        let mut matched = false;
        for problem in &mut contest.problems {
            if let Some((accepted, total, tier)) = stats.get(&problem.index) {
                matched = true;
                if problem.tier.is_none() {
                    problem.accepted_teams = Some(*accepted);
                    problem.total_teams = Some(*total);
                    problem.tier = Some(tier.clone());
                    filled = true;
                }
            }
        }
        let source_added = matched && append_board_source(contest, source);
        if filled || source_added {
            if contest.date.is_empty() {
                contest.date = if date.is_empty() { board.date } else { date };
            }
            updated += 1;
        }
    }
    Ok(updated)
}

fn best_rankland_board<'a>(
    contest: &XcpcContest,
    boards: &'a [RanklandBoard],
) -> Option<&'a RanklandBoard> {
    let mut scored: Vec<_> = boards
        .iter()
        .filter_map(|board| board_match_score(contest, board).map(|score| (score, board)))
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    match scored.as_slice() {
        [(score, board), ..] if *score >= 10 && (scored.len() == 1 || *score > scored[1].0) => {
            Some(*board)
        }
        _ => None,
    }
}

fn board_match_score(contest: &XcpcContest, board: &RanklandBoard) -> Option<i32> {
    if is_warmup(&board.text) {
        return None;
    }
    let text = normalize_match_text(&board.text);
    if contest.year == "未知" || !text.contains(&contest.year) {
        return None;
    }
    let mut score = 5 + round_match_score(contest, &board.text)?;
    if contest.series.iter().any(|value| value == "ICPC") {
        if !text.contains("icpc") {
            return None;
        }
        score += 4;
    } else if contest.series.iter().any(|value| value == "CCPC") {
        if !text.contains("ccpc") {
            return None;
        }
        score += 4;
    }
    if contest.site != "全国" {
        let aliases = site_match_aliases(&contest.site);
        if aliases.iter().any(|alias| text.contains(alias)) {
            score += 6;
        } else {
            return None;
        }
    }
    let stage_terms: &[&str] = match contest.stage.as_str() {
        "网络赛" => &["preliminary", "online", "网络", "预选"],
        "邀请赛" => &["invitational", "邀请"],
        "总决赛" => &["final", "总决赛", "总决"],
        "省赛" => &["provincial", "省赛", "大学生程序设计"],
        _ => &[],
    };
    if !stage_terms.is_empty() {
        if stage_terms
            .iter()
            .any(|term| text.contains(&normalize_match_text(term)))
        {
            score += 4;
        } else {
            return None;
        }
    } else if [
        "preliminary",
        "online",
        "网络",
        "预选",
        "invitational",
        "邀请",
        "final",
        "总决",
    ]
    .iter()
    .any(|term| text.contains(term))
    {
        return None;
    }
    let qoj_name = normalize_match_text(&contest.name);
    if qoj_name.contains(&normalize_match_text(&board.uk)) || text.contains(&qoj_name) {
        score += 3;
    }
    Some(score)
}

async fn fetch_rankland_stats(
    client: &Client,
    board: &RanklandBoard,
) -> Result<(HashMap<String, (i64, i64, String)>, String), String> {
    let file_url = if let Some(url) = board.direct_url.as_deref() {
        url.to_string()
    } else {
        let metadata_url = format!("https://rl.algoux.cn/api/v2/public/files/{}", board.file_id);
        let metadata_text = get_text(client, &metadata_url, browser_headers())
            .await
            .map_err(|e| e.to_string())?;
        let metadata: serde_json::Value =
            serde_json::from_str(&metadata_text).map_err(|e| e.to_string())?;
        metadata
            .pointer("/data/url")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "RankLand 榜单缺少下载地址".to_string())?
            .to_string()
    };
    let srk_text = get_text(client, &file_url, browser_headers())
        .await
        .map_err(|e| e.to_string())?;
    let srk: serde_json::Value = serde_json::from_str(&srk_text).map_err(|e| e.to_string())?;
    Ok(parse_rankland_stats(&srk))
}

fn parse_rankland_stats(srk: &serde_json::Value) -> (HashMap<String, (i64, i64, String)>, String) {
    let date = srk
        .pointer("/contest/startAt")
        .and_then(serde_json::Value::as_str)
        .and_then(|value| value.get(..10))
        .unwrap_or_default()
        .to_string();
    let total = srk
        .get("rows")
        .and_then(serde_json::Value::as_array)
        .map(|rows| rows.len() as i64)
        .unwrap_or_default();
    if total <= 0 {
        return (HashMap::new(), date);
    }
    let mut stats = HashMap::new();
    for problem in srk
        .get("problems")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(alias) = problem.get("alias").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let Some(accepted) = problem
            .pointer("/statistics/accepted")
            .and_then(serde_json::Value::as_i64)
            .filter(|value| *value >= 0 && *value <= total)
        else {
            continue;
        };
        stats.insert(
            alias.to_ascii_uppercase(),
            (accepted, total, rating_tier(accepted, total).into()),
        );
    }
    (stats, date)
}
