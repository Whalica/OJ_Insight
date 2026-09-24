pub async fn load_catalog(
    client: &Client,
    cache_path: &Path,
    cookie: &str,
    force_refresh: bool,
) -> Result<Vec<XcpcContest>, String> {
    let cached = std::fs::read_to_string(cache_path)
        .ok()
        .and_then(|text| serde_json::from_str::<CatalogCache>(&text).ok())
        .filter(|cache| cache.version == 4 || cache.version == CATALOG_CACHE_VERSION)
        .map(|mut cache| {
            // These fields are derived from the title. Refresh old cached
            // labels without discarding problem details or public ratings.
            for contest in &mut cache.contests {
                contest.year = extract_year(&contest.name);
                contest.series = classify_series(&contest.name);
                contest.stage = classify_stage(&contest.name);
                contest.site = classify_site(&contest.name);
                contest.short_name = short_name(&contest.name, &contest.year);
                // Version 4 could attach another round's board to online contests.
                // Keep problems and AC progress, but never display those counts.
                if cache.version < CATALOG_CACHE_VERSION
                    && contest.stage == "网络赛"
                    && !contest.problems.is_empty()
                {
                    clear_board_stats(contest);
                    contest.ratings_stale = true;
                }
            }
            cache.contests
        })
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
        let cached_by_id: HashMap<_, _> = cached
            .into_iter()
            .map(|contest| (contest.id.clone(), contest))
            .collect();
        for contest in &mut items {
            if let Some(previous) = cached_by_id.get(&contest.id) {
                if cached_problems_are_well_formed(&previous.problems) {
                    contest.problems = previous.problems.clone();
                }
                if contest.board_source.is_none() {
                    contest.board_source = previous.board_source.clone();
                }
                contest.ratings_stale = previous.ratings_stale;
                contest.date = previous.date.clone();
            }
        }
    }
    if !cookie.trim().is_empty() {
        enrich_contest_problems(client, cookie, &mut items).await;
    }
    let json = serde_json::to_string(&CatalogCache {
        version: CATALOG_CACHE_VERSION,
        contests: items.clone(),
    })
    .map_err(|e| format!("序列化 ICPC/CCPC 目录失败：{e}"))?;
    std::fs::write(cache_path, json).map_err(|e| format!("保存 ICPC/CCPC 目录失败：{e}"))?;
    Ok(items)
}
