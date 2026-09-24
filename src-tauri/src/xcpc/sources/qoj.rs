async fn fetch_catalog(client: &Client, cookie: &str) -> Result<Vec<XcpcContest>, String> {
    let mut contests = HashMap::<String, XcpcContest>::new();
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();
    for (id, depth) in ROOT_CATEGORIES {
        queue.push_back((id.to_string(), depth));
    }
    while !queue.is_empty() {
        let mut batch = Vec::new();
        while let Some((category_id, depth)) = queue.pop_front() {
            if visited.insert(category_id.clone()) {
                batch.push((category_id, depth));
            }
        }
        for chunk in batch.chunks(8) {
            let mut tasks = tokio::task::JoinSet::new();
            for (category_id, depth) in chunk.iter().cloned() {
                let client = client.clone();
                let cookie = cookie.to_string();
                tasks.spawn(async move {
                    let url = format!("https://qoj.ac/category/{category_id}");
                    let html = get_text(&client, &url, with_cookie(browser_headers(), &cookie))
                        .await
                        .map_err(|e| format!("更新 ICPC/CCPC 目录失败：{e}"))?;
                    Ok::<_, String>((depth, parse_category(&html)))
                });
            }
            while let Some(result) = tasks.join_next().await {
                let (depth, page) =
                    result.map_err(|e| format!("更新 ICPC/CCPC 目录任务失败：{e}"))??;
                for contest in page.contests {
                    contests.entry(contest.id.clone()).or_insert(contest);
                }
                if depth > 0 {
                    for child in page.child_categories {
                        if !visited.contains(&child) {
                            queue.push_back((child, depth - 1));
                        }
                    }
                }
            }
        }
    }
    let mut items: Vec<_> = contests.into_values().collect();
    items.retain(|contest| !is_warmup(&contest.name));
    items.sort_by(|a, b| {
        b.year
            .cmp(&a.year)
            .then_with(|| numeric_id(&b.id).cmp(&numeric_id(&a.id)))
            .then_with(|| a.name.cmp(&b.name))
    });
    if items.is_empty() {
        // QOJ occasionally serves the category table without the problem-column
        // markup. Keep the tracker usable by falling back to its public contest list.
        items = fetch_contest_list(client, cookie).await?;
    }
    if items.is_empty() {
        return Err("QOJ 页面已返回，但没有识别到 ICPC/CCPC 比赛；请稍后重试".into());
    }
    Ok(items)
}

async fn enrich_contest_problems(client: &Client, cookie: &str, contests: &mut [XcpcContest]) {
    let missing: Vec<_> = contests
        .iter()
        .enumerate()
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
                    let previous: HashMap<_, _> = contests[index]
                        .problems
                        .iter()
                        .map(|problem| (problem.problem_id.clone(), problem.clone()))
                        .collect();
                    contests[index].problems = problems
                        .into_iter()
                        .map(|mut problem| {
                            if let Some(old) = previous.get(&problem.problem_id) {
                                problem.tier = old.tier.clone();
                                problem.accepted_teams = old.accepted_teams;
                                problem.total_teams = old.total_teams;
                                problem.solved = old.solved;
                            }
                            problem
                        })
                        .collect();
                }
            }
        }
    }
}

fn contest_needs_problem_details(contest: &XcpcContest) -> bool {
    contest.problems.is_empty()
        || contest
            .problems
            .iter()
            .any(|problem| problem_name_is_missing(&problem.name))
}

fn problem_name_is_missing(raw: &str) -> bool {
    let name = clean_problem_name(raw);
    name.is_empty() || name == "题目" || name.eq_ignore_ascii_case("problem")
}

async fn fetch_contest_problems(
    client: &Client,
    url: &str,
    cookie: &str,
) -> Result<Vec<XcpcProblem>, String> {
    let html = get_text(client, url, with_cookie(browser_headers(), cookie))
        .await
        .map_err(|e| e.to_string())?;
    let doc = Html::parse_document(&html);
    let anchor_sel = Selector::parse("a[href]").unwrap();
    let problem_re =
        Regex::new(r"^(?:https?://qoj\.ac)?/(?:contest/\d+/)?problem/(\d+)(?:$|[/?#])").unwrap();
    let mut problems: Vec<XcpcProblem> = Vec::new();
    let mut positions = HashMap::<String, usize>::new();
    for anchor in doc.select(&anchor_sel) {
        let Some(href) = anchor.value().attr("href") else {
            continue;
        };
        let Some(problem_id) = problem_re
            .captures(href)
            .map(|capture| capture[1].to_string())
        else {
            continue;
        };
        let raw = text_of(&anchor);
        let (mut index, parsed_name) = problem_label(&raw, problems.len());
        let title = anchor
            .value()
            .attr("title")
            .or_else(|| anchor.value().attr("data-original-title"))
            .unwrap_or_default()
            .trim();
        let name = if let Some((title_index, title_name)) =
            explicit_problem_label(title, problems.len())
        {
            index = title_index;
            title_name
        } else if !title.is_empty() {
            clean_problem_name(title)
        } else {
            parsed_name
        };
        let mut candidate = XcpcProblem {
            index,
            name,
            url: format!("https://qoj.ac/problem/{problem_id}"),
            problem_id: problem_id.clone(),
            tier: None,
            accepted_teams: None,
            total_teams: None,
            tag_axes: vec![],
            tags: vec![],
            solved: false,
        };
        if let Some(position) = positions.get(&problem_id).copied() {
            if problem_name_is_missing(&problems[position].name)
                && !problem_name_is_missing(&candidate.name)
            {
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

struct ParsedCategory {
    contests: Vec<XcpcContest>,
    child_categories: Vec<String>,
}

fn parse_category(html: &str) -> ParsedCategory {
    let doc = Html::parse_document(html);
    let row_sel = Selector::parse("table tr").unwrap();
    let anchor_sel = Selector::parse("a[href]").unwrap();
    let contest_re = Regex::new(r"^(?:https?://qoj\.ac)?/contest/(\d+)(?:$|[/?#])").unwrap();
    let problem_re =
        Regex::new(r"^(?:https?://qoj\.ac)?/(?:contest/\d+/)?problem/(\d+)(?:$|[/?#])").unwrap();
    let category_re = Regex::new(r"^(?:https?://qoj\.ac)?/category/(\d+)(?:$|[/?#])").unwrap();
    let mut contests = Vec::new();
    let mut child_categories = Vec::new();
    for row in doc.select(&row_sel) {
        let anchors: Vec<_> = row.select(&anchor_sel).collect();
        let contest_anchor = anchors.iter().find(|anchor| {
            anchor
                .value()
                .attr("href")
                .is_some_and(|href| contest_re.is_match(href))
        });
        let Some(contest_anchor) = contest_anchor else {
            for anchor in anchors {
                if let Some(id) = anchor
                    .value()
                    .attr("href")
                    .and_then(|href| category_re.captures(href))
                    .map(|captures| captures[1].to_string())
                {
                    child_categories.push(id);
                }
            }
            continue;
        };
        let contest_href = contest_anchor.value().attr("href").unwrap_or_default();
        let Some(contest_id) = contest_re
            .captures(contest_href)
            .map(|captures| captures[1].to_string())
        else {
            continue;
        };
        let name = text_of(contest_anchor);
        if name.is_empty() {
            continue;
        }
        let mut problems = Vec::new();
        for anchor in anchors {
            let Some(href) = anchor.value().attr("href") else {
                continue;
            };
            let Some(problem_id) = problem_re
                .captures(href)
                .map(|captures| captures[1].to_string())
            else {
                continue;
            };
            let raw_index = text_of(&anchor);
            let (mut index, parsed_name) = problem_label(&raw_index, problems.len());
            let title = anchor
                .value()
                .attr("title")
                .or_else(|| anchor.value().attr("data-original-title"))
                .or_else(|| anchor.value().attr("aria-label"))
                .unwrap_or_default()
                .trim();
            let problem_name = if let Some((title_index, title_name)) =
                explicit_problem_label(title, problems.len())
            {
                index = title_index;
                title_name
            } else if !title.is_empty() {
                clean_problem_name(title)
            } else {
                parsed_name
            };
            problems.push(XcpcProblem {
                index,
                name: problem_name,
                url: format!("https://qoj.ac/problem/{problem_id}"),
                problem_id,
                tier: None,
                accepted_teams: None,
                total_teams: None,
                tag_axes: vec![],
                tags: vec![],
                solved: false,
            });
        }
        sort_problems(&mut problems);
        let year = extract_year(&name);
        contests.push(XcpcContest {
            id: contest_id.clone(),
            name: name.clone(),
            short_name: short_name(&name, &year),
            url: format!("https://qoj.ac/contest/{contest_id}"),
            date: String::new(),
            year,
            series: classify_series(&name),
            stage: classify_stage(&name),
            site: classify_site(&name),
            board_source: None,
            ratings_stale: false,
            problems,
        });
    }
    let parsed = ParsedCategory {
        contests,
        child_categories,
    };
    if parsed.contests.is_empty()
        || parsed
            .contests
            .iter()
            .all(|contest| contest.problems.is_empty())
    {
        let fallback = parse_category_rows_from_html(html);
        if !fallback.contests.is_empty() || !fallback.child_categories.is_empty() {
            return fallback;
        }
    }
    parsed
}

fn parse_category_rows_from_html(html: &str) -> ParsedCategory {
    let row_re = Regex::new(r"(?is)<tr\b[^>]*>(.*?)</tr>").unwrap();
    let anchor_re =
        Regex::new(r#"(?is)<a\b[^>]*href\s*=\s*["']([^"']+)["'][^>]*>(.*?)</a>"#).unwrap();
    let title_re =
        Regex::new(r#"(?is)\b(?:title|data-original-title|aria-label)\s*=\s*["']([^"']+)["']"#)
            .unwrap();
    let tag_re = Regex::new(r"(?is)<[^>]+>").unwrap();
    let contest_re = Regex::new(r"^(?:https?://qoj\.ac)?/contest/(\d+)(?:$|[/?#])").unwrap();
    let problem_re =
        Regex::new(r"^(?:https?://qoj\.ac)?/(?:contest/\d+/)?problem/(\d+)(?:$|[/?#])").unwrap();
    let category_re = Regex::new(r"^(?:https?://qoj\.ac)?/category/(\d+)(?:$|[/?#])").unwrap();
    let mut contests = Vec::new();
    let mut child_categories = Vec::new();
    for capture in row_re.captures_iter(html) {
        let row = capture
            .get(1)
            .map(|match_| match_.as_str())
            .unwrap_or_default();
        let anchors: Vec<_> = anchor_re.captures_iter(row).collect();
        let contest = anchors
            .iter()
            .find(|anchor| contest_re.is_match(&anchor[1]));
        let Some(contest) = contest else {
            for anchor in anchors {
                if let Some(id) = category_re
                    .captures(&anchor[1])
                    .map(|capture| capture[1].to_string())
                {
                    child_categories.push(id);
                }
            }
            continue;
        };
        let Some(id) = contest_re
            .captures(&contest[1])
            .map(|capture| capture[1].to_string())
        else {
            continue;
        };
        let name = tag_re
            .replace_all(&contest[2], " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if name.is_empty() {
            continue;
        }
        let mut problems = Vec::new();
        for anchor in anchors {
            let Some(problem_id) = problem_re
                .captures(&anchor[1])
                .map(|capture| capture[1].to_string())
            else {
                continue;
            };
            let raw = tag_re
                .replace_all(&anchor[2], " ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            let (mut index, parsed_name) = problem_label(&raw, problems.len());
            let title = title_re
                .captures(&anchor[0])
                .map(|capture| capture[1].trim().to_string())
                .unwrap_or_default();
            let name = if let Some((title_index, title_name)) =
                explicit_problem_label(&title, problems.len())
            {
                index = title_index;
                title_name
            } else if !title.is_empty() {
                clean_problem_name(&title)
            } else {
                parsed_name
            };
            problems.push(XcpcProblem {
                index,
                name,
                url: format!("https://qoj.ac/problem/{problem_id}"),
                problem_id,
                tier: None,
                accepted_teams: None,
                total_teams: None,
                tag_axes: vec![],
                tags: vec![],
                solved: false,
            });
        }
        sort_problems(&mut problems);
        let year = extract_year(&name);
        contests.push(XcpcContest {
            id: id.clone(),
            name: name.clone(),
            short_name: short_name(&name, &year),
            url: format!("https://qoj.ac/contest/{id}"),
            date: String::new(),
            year,
            series: classify_series(&name),
            stage: classify_stage(&name),
            site: classify_site(&name),
            board_source: None,
            ratings_stale: false,
            problems,
        });
    }
    ParsedCategory {
        contests,
        child_categories,
    }
}

async fn fetch_contest_list(client: &Client, cookie: &str) -> Result<Vec<XcpcContest>, String> {
    let html = get_text(
        client,
        "https://qoj.ac/contests?tab=icpc",
        with_cookie(browser_headers(), cookie),
    )
    .await
    .map_err(|e| format!("更新 ICPC/CCPC 目录失败：{e}"))?;
    let doc = Html::parse_document(&html);
    let row_sel = Selector::parse("table tr").unwrap();
    let anchor_sel = Selector::parse("a[href]").unwrap();
    let contest_re = Regex::new(r"^(?:https?://qoj\.ac)?/contest/(\d+)(?:$|[/?#])").unwrap();
    let mut items = Vec::new();
    for row in doc.select(&row_sel) {
        let Some(anchor) = row.select(&anchor_sel).find(|a| {
            a.value()
                .attr("href")
                .is_some_and(|h| contest_re.is_match(h))
        }) else {
            continue;
        };
        let Some(id) = anchor
            .value()
            .attr("href")
            .and_then(|h| contest_re.captures(h))
            .map(|c| c[1].to_string())
        else {
            continue;
        };
        let name = text_of(&anchor);
        if name.is_empty() {
            continue;
        }
        let year = extract_year(&name);
        items.push(XcpcContest {
            id: id.clone(),
            name: name.clone(),
            short_name: short_name(&name, &year),
            url: format!("https://qoj.ac/contest/{id}"),
            date: String::new(),
            year,
            series: classify_series(&name),
            stage: classify_stage(&name),
            site: classify_site(&name),
            board_source: None,
            ratings_stale: false,
            problems: Vec::new(),
        });
    }
    items.retain(|contest| !is_warmup(&contest.name));
    items.sort_by(|a, b| {
        b.year
            .cmp(&a.year)
            .then_with(|| numeric_id(&b.id).cmp(&numeric_id(&a.id)))
    });
    Ok(items)
}

fn text_of(element: &scraper::ElementRef<'_>) -> String {
    element
        .text()
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
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
    if text.eq_ignore_ascii_case(&expected) {
        return (expected, String::new());
    }
    if let Some((_, name)) = explicit_problem_label(&text, position) {
        return (expected, name);
    }
    (expected, text)
}
fn explicit_problem_label(raw: &str, position: usize) -> Option<(String, String)> {
    let text = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let expected = fallback_problem_index(position);
    // A prefix is a problem label only when it agrees with the position in the
    // contest. This keeps real titles such as "MOD. Modular" and "GG: Game"
    // intact instead of trying to maintain a fragile list of exceptions.
    let label_re = Regex::new(
        r"(?i)^(?:(?:problem\s+([a-z]+)(?:\s*[.．:：]\s*|\s+))|(?:([a-z]+)\s*[.．:：]\s*))(.+)$",
    )
    .unwrap();
    let capture = label_re.captures(&text)?;
    let label = capture.get(1).or_else(|| capture.get(2))?.as_str();
    if !label.eq_ignore_ascii_case(&expected) {
        return None;
    }
    Some((expected, capture[3].trim().to_string()))
}
fn clean_problem_name(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}
fn problem_index_rank(index: &str) -> u32 {
    let upper = index.to_ascii_uppercase();
    if !upper.is_empty() && upper.chars().all(|ch| ch.is_ascii_uppercase()) {
        return upper.bytes().fold(0u32, |value, ch| {
            value
                .saturating_mul(26)
                .saturating_add((ch - b'A' + 1) as u32)
        });
    }
    upper.parse::<u32>().unwrap_or(u32::MAX)
}
fn sort_problems(problems: &mut [XcpcProblem]) {
    problems.sort_by(|a, b| {
        problem_index_rank(&a.index)
            .cmp(&problem_index_rank(&b.index))
            .then_with(|| a.problem_id.cmp(&b.problem_id))
    });
}
fn cached_problems_are_well_formed(problems: &[XcpcProblem]) -> bool {
    !problems.is_empty()
        && problems.iter().enumerate().all(|(position, problem)| {
            problem.index == fallback_problem_index(position)
                && clean_problem_name(&problem.name) == problem.name
        })
}
