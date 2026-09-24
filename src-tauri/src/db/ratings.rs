const KNOWLEDGE_AXES: [&str; 8] = [
    "基础与模拟",
    "数据结构",
    "图论与树",
    "动态规划",
    "数学",
    "字符串",
    "搜索与构造",
    "贪心与思维",
];

fn knowledge_axis(tag: &str) -> Option<&'static str> {
    let tag = tag.trim().to_lowercase();
    if tag.is_empty() {
        return None;
    }
    if [
        "data structures",
        "data structure",
        "array",
        "hash",
        "stack",
        "queue",
        "heap",
        "linked list",
        "segment tree",
        "fenwick",
        "dsu",
        "数据结构",
    ]
    .iter()
    .any(|value| tag.contains(value))
    {
        return Some("数据结构");
    }
    if [
        "graph",
        "tree",
        "shortest path",
        "mst",
        "topological",
        "图论",
        "树",
    ]
    .iter()
    .any(|value| tag.contains(value))
    {
        return Some("图论与树");
    }
    if ["dynamic programming", "dp", "动态规划"]
        .iter()
        .any(|value| tag == *value || tag.contains(value))
    {
        return Some("动态规划");
    }
    if [
        "math",
        "number theory",
        "combinatorics",
        "geometry",
        "probability",
        "数学",
        "几何",
    ]
    .iter()
    .any(|value| tag.contains(value))
    {
        return Some("数学");
    }
    if ["string", "trie", "字符串"]
        .iter()
        .any(|value| tag.contains(value))
    {
        return Some("字符串");
    }
    if [
        "binary search",
        "brute force",
        "backtracking",
        "dfs",
        "bfs",
        "constructive",
        "search",
        "搜索",
        "构造",
    ]
    .iter()
    .any(|value| tag.contains(value))
    {
        return Some("搜索与构造");
    }
    if [
        "greedy",
        "two pointers",
        "sliding window",
        "divide and conquer",
        "sort",
        "贪心",
        "思维",
    ]
    .iter()
    .any(|value| tag.contains(value))
    {
        return Some("贪心与思维");
    }
    if [
        "implementation",
        "simulation",
        "basic",
        "基础",
        "模拟",
        "算法策略",
    ]
    .iter()
    .any(|value| tag.contains(value))
    {
        return Some("基础与模拟");
    }
    None
}

fn knowledge_level(platform: &str, difficulty: &str) -> Option<f64> {
    let label = difficulty.trim().to_lowercase();
    match platform {
        "codeforces" => label.parse::<f64>().ok().filter(|rating| *rating > 0.0),
        "leetcode" => match label.as_str() {
            "hard" | "困难" => Some(86.0),
            "medium" | "中等" => Some(65.0),
            "easy" | "简单" => Some(42.0),
            _ => None,
        },
        "qoj" if label.contains("gold") || label.contains('金') => Some(90.0),
        "qoj" if label.contains("silver") || label.contains('银') => Some(76.0),
        "qoj" if label.contains("bronze") || label.contains('铜') => Some(58.0),
        "qoj" if label.contains("iron") || label.contains('铁') => Some(38.0),
        _ => None,
    }
}

fn knowledge_recency_weight(epoch_second: i64) -> f64 {
    if epoch_second <= 0 {
        return 0.65;
    }
    let age_days = (Utc::now().timestamp() - epoch_second).max(0) as f64 / 86_400.0;
    0.65 + 0.35 * (-age_days / 730.0).exp()
}

fn weighted_quantile(items: &[(f64, f64)], quantile: f64) -> Option<f64> {
    let mut values: Vec<_> = items
        .iter()
        .copied()
        .filter(|(_, weight)| *weight > 0.0)
        .collect();
    values.sort_by(|left, right| {
        left.0
            .partial_cmp(&right.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let target = values.iter().map(|(_, weight)| weight).sum::<f64>() * quantile.clamp(0.0, 1.0);
    let mut seen = 0.0;
    for (value, weight) in &values {
        seen += weight;
        if seen >= target {
            return Some(*value);
        }
    }
    values.last().map(|item| item.0)
}

fn robust_knowledge_estimate(
    representative: f64,
    prior: f64,
    evidence: f64,
    platform: &str,
) -> f64 {
    let lambda =
        evidence.max(0.0) / (evidence.max(0.0) + if platform == "codeforces" { 8.0 } else { 6.0 });
    lambda * representative + (1.0 - lambda) * prior
}

fn knowledge_buckets(
    platform: &str,
    values: Vec<(&'static str, i64, f64)>,
) -> Vec<KnowledgeBucket> {
    let mut ranked = values
        .iter()
        .filter(|(_, count, _)| *count > 0)
        .map(|(_, _, estimate)| *estimate)
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    let center = ranked
        .get(ranked.len() / 2)
        .copied()
        .unwrap_or(if platform == "codeforces" {
            1200.0
        } else {
            50.0
        });
    values
        .into_iter()
        .map(|(axis, count, estimate)| {
            let score = if count <= 0 {
                0
            } else if platform == "codeforces" {
                let absolute = 20.0 + 0.05 * (estimate - 800.0);
                let relative = 50.0 + (estimate - center) / 10.0;
                (absolute * 0.4 + relative * 0.6).round().clamp(5.0, 95.0) as i64
            } else {
                estimate.round().clamp(5.0, 95.0) as i64
            };
            KnowledgeBucket {
                platform: platform.into(),
                axis: axis.into(),
                count,
                score,
            }
        })
        .collect()
}

fn codeforces_rating_prior(conn: &Connection, account: &str) -> Result<f64, String> {
    let mut stmt = conn.prepare(
        "SELECT new_rating FROM rating_history WHERE platform='codeforces' AND (?='' OR account=?) ORDER BY epoch_second DESC LIMIT 5"
    ).map_err(|error| error.to_string())?;
    let rows = stmt
        .query_map(params![account, account], |row| row.get::<_, i64>(0))
        .map_err(|error| error.to_string())?;
    let mut values = Vec::new();
    for row in rows {
        values.push(row.map_err(|error| error.to_string())? as f64);
    }
    if values.is_empty() {
        return Ok(1200.0);
    }
    values.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    Ok(values[values.len() / 2])
}

pub fn needs_tag_backfill(
    conn: &Connection,
    platform: &str,
    account: &str,
) -> Result<bool, String> {
    if platform != "codeforces" {
        return Ok(false);
    }
    let completed: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM platform_stats_accounts WHERE platform=? AND account=? AND key='metadata_backfill_v1' AND value='1')",
        params![platform, account], |row| row.get(0),
    ).map_err(|error| error.to_string())?;
    if completed {
        return Ok(false);
    }
    let (total, enriched): (i64, i64) = conn.query_row(
        "SELECT COUNT(DISTINCT problem_key),COUNT(DISTINCT CASE WHEN participant_type<>'' THEN problem_key END)
         FROM submissions WHERE platform=? AND account=?",
        params![platform, account], |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(|error| error.to_string())?;
    Ok(total > 0 && enriched * 100 < total * 90)
}

pub fn needs_nowcoder_difficulty_backfill(
    conn: &Connection,
    platform: &str,
    account: &str,
) -> Result<bool, String> {
    if platform != "nowcoder" {
        return Ok(false);
    }
    let completed: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM platform_stats_accounts WHERE platform=? AND account=? AND key='tracker_difficulty_backfill_v2' AND value='1')",
            params![platform, account],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    Ok(!completed)
}

fn knowledge_for_platform(
    conn: &Connection,
    platform: &str,
    account: Option<&str>,
) -> Result<Vec<KnowledgeBucket>, String> {
    if !matches!(platform, "codeforces" | "leetcode" | "qoj") {
        return Ok(Vec::new());
    }
    let account = account.unwrap_or("");
    let mut aggregate_stmt = conn.prepare(
        "SELECT axis,SUM(count) FROM knowledge_stats_accounts WHERE platform=? AND (?='' OR account=?) GROUP BY axis"
    ).map_err(|error| error.to_string())?;
    let aggregate_rows = aggregate_stmt
        .query_map(params![platform, account, account], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|error| error.to_string())?;
    let mut aggregate_counts = HashMap::new();
    for row in aggregate_rows {
        let (axis, count) = row.map_err(|error| error.to_string())?;
        aggregate_counts.insert(axis, count);
    }
    if aggregate_counts.values().any(|count| *count > 0) {
        let mut difficulty_stmt = conn.prepare(
            "SELECT label,SUM(count) FROM difficulty_stats_accounts WHERE platform=? AND (?='' OR account=?) GROUP BY label"
        ).map_err(|error| error.to_string())?;
        let difficulty_rows = difficulty_stmt
            .query_map(params![platform, account, account], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(|error| error.to_string())?;
        let mut difficulty_evidence = Vec::new();
        for row in difficulty_rows {
            let (label, count) = row.map_err(|error| error.to_string())?;
            if let Some(level) = knowledge_level(platform, &label) {
                difficulty_evidence.push((level, count.max(0) as f64));
            }
        }
        let representative = weighted_quantile(&difficulty_evidence, 0.75).unwrap_or(50.0);
        let prior = if platform == "codeforces" {
            codeforces_rating_prior(conn, account)?
        } else {
            50.0
        };
        let values = KNOWLEDGE_AXES
            .iter()
            .map(|axis| {
                let count = aggregate_counts.get(*axis).copied().unwrap_or(0);
                let evidence = (count.max(0) as f64).min(20.0);
                let estimate = robust_knowledge_estimate(representative, prior, evidence, platform);
                (*axis, count, estimate)
            })
            .collect();
        return Ok(knowledge_buckets(platform, values));
    }
    let mut stmt = conn.prepare(
        "SELECT problem_key,MAX(tags),MAX(COALESCE(difficulty,'')),MAX(epoch_second),MAX(CASE participant_type WHEN 'CONTESTANT' THEN 4 WHEN 'VIRTUAL' THEN 3 WHEN 'OUT_OF_COMPETITION' THEN 2 WHEN 'PRACTICE' THEN 1 ELSE 0 END) FROM submissions WHERE platform=? AND (?='' OR account=?) AND tags<>'[]' GROUP BY problem_key"
    ).map_err(|error| error.to_string())?;
    let rows = stmt
        .query_map(params![platform, account, account], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })
        .map_err(|error| error.to_string())?;
    let mut evidence: HashMap<&'static str, Vec<(f64, f64, bool)>> = HashMap::new();
    for row in rows {
        let (_, raw, difficulty, epoch_second, participant_rank) =
            row.map_err(|error| error.to_string())?;
        let tags: Vec<String> = serde_json::from_str(&raw).unwrap_or_default();
        let axes: HashSet<_> = tags.iter().filter_map(|tag| knowledge_axis(tag)).collect();
        let Some(level) = knowledge_level(platform, &difficulty) else {
            continue;
        };
        if axes.is_empty() {
            continue;
        }
        let kind_weight = match participant_rank {
            4 => 1.0,
            3 => 0.65,
            2 => 0.5,
            1 => 0.25,
            _ => {
                if platform == "codeforces" {
                    0.25
                } else {
                    1.0
                }
            }
        };
        let weight = kind_weight * knowledge_recency_weight(epoch_second) / axes.len() as f64;
        let practice = platform == "codeforces" && participant_rank <= 1;
        for axis in axes {
            evidence
                .entry(axis)
                .or_default()
                .push((level, weight, practice));
        }
    }
    if evidence.values().all(|items| items.is_empty()) {
        return Ok(Vec::new());
    }
    let prior = if platform == "codeforces" {
        codeforces_rating_prior(conn, account)?
    } else {
        let all = evidence
            .values()
            .flatten()
            .map(|(level, weight, _)| (*level, *weight))
            .collect::<Vec<_>>();
        weighted_quantile(&all, 0.75).unwrap_or(50.0)
    };
    let values = KNOWLEDGE_AXES
        .iter()
        .map(|axis| {
            let items = evidence.get(axis).cloned().unwrap_or_default();
            let count = items.len() as i64;
            let timed = items
                .iter()
                .filter(|(_, _, practice)| !practice)
                .map(|(level, weight, _)| (*level, *weight))
                .collect::<Vec<_>>();
            let practice = items
                .iter()
                .filter(|(_, _, practice)| *practice)
                .map(|(level, weight, _)| (*level, *weight))
                .collect::<Vec<_>>();
            let timed_p75 = weighted_quantile(&timed, 0.75);
            let practice_p75 = weighted_quantile(&practice, 0.75);
            let representative = match (timed_p75, practice_p75) {
                (Some(timed), Some(practice)) => 0.75 * timed + 0.25 * practice,
                (Some(value), None) | (None, Some(value)) => value,
                _ => prior,
            };
            let timed_sum = timed
                .iter()
                .map(|(_, weight)| weight)
                .sum::<f64>()
                .min(20.0);
            let practice_evidence = (practice.len() as f64 * 0.1).min(5.0);
            let effective = timed_sum + practice_evidence;
            let estimate = robust_knowledge_estimate(representative, prior, effective, platform);
            (*axis, count, estimate)
        })
        .collect();
    Ok(knowledge_buckets(platform, values))
}

const UNRATED_LABEL: &str = "未评级";
const UNRATED_ORDER: i64 = -1;

fn difficulty_for_platform(
    conn: &Connection,
    p: &str,
    _start: Option<&str>,
    _end: Option<&str>,
    account: Option<&str>,
    source: Option<&str>,
) -> Result<Vec<DifficultyBucket>, String> {
    let account = account.unwrap_or("");
    let source = source.unwrap_or("");
    let explicit_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM difficulty_stats_accounts WHERE platform=? AND (?='' OR account=?)",
            params![p, account, account],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if explicit_count > 0 && source.is_empty() {
        let mut stmt=conn.prepare("SELECT label,SUM(count),sort_order FROM difficulty_stats_accounts WHERE platform=? AND (?='' OR account=?) GROUP BY label,sort_order ORDER BY sort_order,label").map_err(|e|e.to_string())?;
        let rows = stmt
            .query_map(params![p, account, account], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })
            .map_err(|e| e.to_string())?;
        let mut buckets: BTreeMap<(i64, String), i64> = BTreeMap::new();
        for row in rows {
            let (raw_label, count) = row.map_err(|e| e.to_string())?;
            let (order, label) = bucket_label(p, &raw_label);
            *buckets.entry((order, label)).or_default() += count;
        }
        let solved: i64 = conn.query_row(
            "SELECT COALESCE(SUM(CAST(value AS INTEGER)),0) FROM platform_stats_accounts WHERE platform=? AND key='solved_count' AND (?='' OR account=?)",
            params![p, account, account],
            |row| row.get(0),
        ).unwrap_or(0);
        let rated = buckets
            .iter()
            .filter(|((order, _), _)| *order != UNRATED_ORDER)
            .map(|(_, count)| *count)
            .sum::<i64>();
        if solved > rated {
            let unrated = buckets
                .entry((UNRATED_ORDER, UNRATED_LABEL.into()))
                .or_default();
            *unrated = (*unrated).max(solved - rated);
        }
        return Ok(buckets
            .into_iter()
            .filter(|(_, count)| *count > 0)
            .map(|((order, label), count)| DifficultyBucket {
                platform: p.into(),
                label,
                count,
                order,
            })
            .collect());
    }
    let mut stmt=conn.prepare("SELECT account,problem_key,difficulty FROM submissions WHERE platform=? AND (?='' OR account=?) AND (?='' OR source=?) ORDER BY epoch_second,submission_id").map_err(|e|e.to_string())?;
    let rows = stmt
        .query_map(params![p, account, account, source, source], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut seen = HashSet::new();
    let mut bucket: BTreeMap<(i64, String), i64> = BTreeMap::new();
    for row in rows {
        let (row_account, problem, difficulty) = row.map_err(|e| e.to_string())?;
        if !seen.insert(format!("{row_account}\0{problem}")) {
            continue;
        }
        let (order, label) = bucket_label(p, difficulty.as_deref().unwrap_or(""));
        *bucket.entry((order, label)).or_default() += 1;
    }
    Ok(bucket
        .into_iter()
        .map(|((order, label), count)| DifficultyBucket {
            platform: p.into(),
            label,
            count,
            order,
        })
        .collect())
}

pub fn solved_problem_keys(conn: &Connection, platform: &str) -> Result<HashSet<String>, String> {
    let mut stmt = conn
        .prepare("SELECT DISTINCT problem_key FROM submissions WHERE platform=?")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([platform], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<HashSet<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn apply_qoj_problem_ratings(
    conn: &Connection,
    contests: &[XcpcContest],
) -> Result<usize, String> {
    let mut updated = 0;
    for problem in contests.iter().flat_map(|contest| &contest.problems) {
        let label = match problem.tier.as_deref() {
            Some("gold") => Some("金题"),
            Some("silver") => Some("银题"),
            Some("bronze") => Some("铜题"),
            Some("iron") => Some("铁题"),
            _ => None,
        };
        let mut tags = problem.tag_axes.clone();
        tags.extend(problem.tags.iter().cloned());
        tags.sort();
        tags.dedup();
        let tags = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".into());
        updated += conn.execute(
            "UPDATE submissions SET difficulty=COALESCE(?,difficulty),tags=CASE WHEN ?='[]' THEN tags ELSE ? END WHERE platform='qoj' AND problem_key=?",
            params![label, tags, tags, problem.problem_id],
        ).map_err(|e| e.to_string())?;
    }
    Ok(updated)
}

fn bucket_label(p: &str, difficulty: &str) -> (i64, String) {
    let d = difficulty.trim();
    if d.is_empty() || d.eq_ignore_ascii_case("unknown") || d.eq_ignore_ascii_case("unrated") {
        return (UNRATED_ORDER, UNRATED_LABEL.into());
    }
    if p == "codeforces" {
        if let Ok(x) = d.parse::<i64>() {
            let rating = (x / 100) * 100;
            return (rating, rating.to_string());
        }
    }
    if p == "atcoder" {
        if let Ok(x) = d.parse::<i64>() {
            let lo = (x.max(0) / 400) * 400;
            return (lo, format!("{}–{}", lo, lo + 399));
        }
    }
    if p == "nowcoder" {
        if let Ok(x) = d.parse::<i64>() {
            return (x, x.to_string());
        }
    }
    if p == "leetcode" {
        return match d.to_ascii_lowercase().as_str() {
            "easy" => (1, "Easy".into()),
            "medium" => (2, "Medium".into()),
            "hard" => (3, "Hard".into()),
            _ => (UNRATED_ORDER, UNRATED_LABEL.into()),
        };
    }
    if p == "qoj" {
        return match d {
            "铁题" | "iron" => (1, "铁题".into()),
            "铜题" | "bronze" => (2, "铜题".into()),
            "银题" | "silver" => (3, "银题".into()),
            "金题" | "gold" => (4, "金题".into()),
            _ => (UNRATED_ORDER, UNRATED_LABEL.into()),
        };
    }
    if p == "luogu" {
        let order = match d {
            "入门" | "1" => 1,
            "普及-" | "2" => 2,
            "普及" | "3" => 3,
            "普及+/提高-" | "4" => 4,
            "提高" | "5" => 5,
            "提高+/省选-" | "6" => 6,
            "省选/NOI-" | "7" => 7,
            "NOI/NOI+/CTS" | "8" => 8,
            _ => UNRATED_ORDER,
        };
        let label = match order {
            1 => "入门",
            2 => "普及-",
            3 => "普及",
            4 => "普及+/提高-",
            5 => "提高",
            6 => "提高+/省选-",
            7 => "省选/NOI-",
            8 => "NOI/NOI+/CTS",
            _ => UNRATED_LABEL,
        };
        return (order, label.into());
    }
    (UNRATED_ORDER, UNRATED_LABEL.into())
}

fn difficulty_daily_for_platform(
    conn: &Connection,
    p: &str,
    start: Option<&str>,
    end: Option<&str>,
    account: Option<&str>,
    source: Option<&str>,
    time_zone: &str,
) -> Result<Vec<DifficultyDayPoint>, String> {
    let s = start.unwrap_or("0000-00-00");
    let e = end.unwrap_or("9999-99-99");
    let account = account.unwrap_or("");
    let source = source.unwrap_or("");
    let mut stmt = conn.prepare("SELECT epoch_second,difficulty,source_day FROM submissions WHERE platform=? AND (?='' OR account=?) AND (?='' OR source=?) ORDER BY epoch_second").map_err(|e|e.to_string())?;
    let rows = stmt
        .query_map(params![p, account, account, source, source], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut days: BTreeMap<String, (i64, String)> = BTreeMap::new();
    for row in rows {
        let (ts, difficulty, source_day) = row.map_err(|e| e.to_string())?;
        let day = source_day.unwrap_or_else(|| day_in_time_zone(ts, time_zone));
        if day.as_str() < s || day.as_str() > e {
            continue;
        }
        let (order, label) = bucket_label(p, difficulty.as_deref().unwrap_or(""));
        let rank = if order == UNRATED_ORDER { -1 } else { order };
        match days.get(&day) {
            Some((current, _))
                if (if *current == UNRATED_ORDER {
                    -1
                } else {
                    *current
                }) >= rank => {}
            _ => {
                days.insert(day, (order, label));
            }
        }
    }
    Ok(days
        .into_iter()
        .map(|(day, (order, label))| DifficultyDayPoint {
            platform: p.into(),
            day,
            label,
            order,
        })
        .collect())
}
