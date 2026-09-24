pub async fn sync_public_ratings(
    client: &Client,
    cache_path: &Path,
    contests: &mut [XcpcContest],
) -> Result<usize, String> {
    let mut updated = 0;
    let mut errors = Vec::new();
    // Prefer XCPCIO's official-team data, then let RankLand fill individual
    // problems that are absent from that board instead of skipping the contest.
    // Rebuild ratings separately so RankLand can replace stale values while
    // preserving the last successful cache for contests whose downloads fail.
    let mut refreshed = contests.to_vec();
    for contest in &mut refreshed {
        clear_board_stats(contest);
    }
    match sync_xcpcio_ratings(client, &mut refreshed).await {
        Ok(count) => updated += count,
        Err(error) => errors.push(error),
    }
    match sync_rankland_ratings(client, &mut refreshed).await {
        Ok(count) => updated += count,
        Err(error) => errors.push(error),
    }
    if updated == 0 && errors.len() == 2 {
        return Err(format!("公开榜单源均不可用：{}", errors.join("；")));
    }
    merge_refreshed_ratings(contests, refreshed);
    save_catalog(cache_path, contests)?;
    Ok(updated)
}

fn clear_board_stats(contest: &mut XcpcContest) {
    contest.board_source = None;
    contest.date.clear();
    for problem in &mut contest.problems {
        problem.tier = None;
        problem.accepted_teams = None;
        problem.total_teams = None;
    }
}

fn merge_refreshed_ratings(contests: &mut [XcpcContest], refreshed: Vec<XcpcContest>) {
    for (contest, mut next) in contests.iter_mut().zip(refreshed) {
        if next.board_source.is_some() {
            if !contest.ratings_stale {
                let mut kept_previous = false;
                for problem in next
                    .problems
                    .iter_mut()
                    .filter(|problem| problem.tier.is_none())
                {
                    if let Some(previous) = contest.problems.iter().find(|previous| {
                        previous.problem_id == problem.problem_id
                            && previous.index == problem.index
                            && previous.tier.is_some()
                    }) {
                        problem.tier = previous.tier.clone();
                        problem.accepted_teams = previous.accepted_teams;
                        problem.total_teams = previous.total_teams;
                        kept_previous = true;
                    }
                }
                if kept_previous {
                    for source in contest
                        .board_source
                        .as_deref()
                        .unwrap_or_default()
                        .split(" + ")
                        .filter(|source| !source.is_empty())
                    {
                        append_board_source(&mut next, source);
                    }
                }
                if next.date.is_empty() {
                    next.date = contest.date.clone();
                }
            }
            next.ratings_stale = false;
            *contest = next;
        }
    }
}

fn append_board_source(contest: &mut XcpcContest, source: &str) -> bool {
    let sources: Vec<_> = contest
        .board_source
        .as_deref()
        .unwrap_or_default()
        .split(" + ")
        .collect();
    if sources.iter().any(|current| *current == source) {
        return false;
    }
    contest.board_source = Some(if sources.iter().all(|current| current.is_empty()) {
        source.to_string()
    } else {
        format!(
            "{} + {source}",
            contest.board_source.as_deref().unwrap_or_default()
        )
    });
    true
}

fn rating_tier(accepted: i64, total: i64) -> &'static str {
    let scaled = accepted.max(0).saturating_mul(100);
    if scaled <= total.saturating_mul(10) {
        "gold"
    } else if scaled <= total.saturating_mul(30) {
        "silver"
    } else if scaled <= total.saturating_mul(60) {
        "bronze"
    } else {
        "iron"
    }
}
