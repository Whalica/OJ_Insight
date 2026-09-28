use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use reqwest::Client;

use crate::models::XcpcContest;
use crate::sync::{browser_headers, get_text};

const PROBLEM_TYPES_URL: &str = "https://raw.githubusercontent.com/Hei-MaoM/xcpcrating/main/data/problem-types/2023-present-all-qoj-v2.json";

pub async fn apply_problem_tags(
    client: &Client,
    cache_path: &Path,
    contests: &mut [XcpcContest],
    force_refresh: bool,
) -> Result<usize, String> {
    let cached = std::fs::read_to_string(cache_path).ok();
    let text = if force_refresh || cached.is_none() {
        match get_text(client, PROBLEM_TYPES_URL, browser_headers()).await {
            Ok(text) => {
                if serde_json::from_str::<serde_json::Value>(&text).is_ok() {
                    let _ = std::fs::write(cache_path, &text);
                    text
                } else {
                    cached.ok_or_else(|| "ICPC/CCPC 标签数据格式异常".to_string())?
                }
            }
            Err(error) => cached.ok_or_else(|| format!("读取 ICPC/CCPC 标签数据失败：{error}"))?,
        }
    } else {
        cached.unwrap_or_default()
    };
    let payload: serde_json::Value = serde_json::from_str(&text)
        .map_err(|error| format!("解析 ICPC/CCPC 标签数据失败：{error}"))?;
    let mut by_qoj_id = HashMap::<String, (Vec<String>, BTreeMap<String, f64>, Vec<String>)>::new();
    for entry in payload
        .get("problems")
        .and_then(serde_json::Value::as_object)
        .into_iter()
        .flatten()
        .map(|(_, value)| value)
    {
        let Some(canonical) = entry.get("canonicalId").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let Some(problem_id) = canonical.strip_prefix("qoj:") else {
            continue;
        };
        let mut weights = BTreeMap::<String, f64>::new();
        for (key, weight) in entry
            .get("labels")
            .and_then(serde_json::Value::as_object)
            .into_iter()
            .flatten()
        {
            let weight = weight.as_f64().unwrap_or(0.0);
            if !weight.is_finite() || weight <= 0.0 {
                continue;
            }
            let label = match key.as_str() {
                "adhoc" | "technique" => "基础与模拟",
                "search" => "搜索与构造",
                "offline" | "random" => "贪心与思维",
                "dataStructure" => "数据结构",
                "graph" | "flow" => "图论与树",
                "dp" => "动态规划",
                "math" | "probability" | "geometry" => "数学",
                "string" => "字符串",
                "basic" => "基础与模拟",
                _ => continue,
            };
            *weights.entry(label.to_string()).or_default() += weight;
        }
        let strongest = weights.values().copied().fold(0.0, f64::max);
        weights.retain(|_, weight| *weight >= 0.25 && *weight >= strongest * 0.5);
        let total = weights.values().sum::<f64>();
        if total > 0.0 {
            for weight in weights.values_mut() { *weight /= total; }
        }
        let axes = weights.keys().cloned().collect();
        let tags = entry
            .get("detailTags")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .map(str::to_string)
            .collect();
        by_qoj_id.insert(problem_id.to_string(), (axes, weights, tags));
    }
    let mut matched = 0;
    for problem in contests
        .iter_mut()
        .flat_map(|contest| &mut contest.problems)
    {
        let Some((axes, weights, tags)) = by_qoj_id.get(&problem.problem_id) else {
            continue;
        };
        problem.tag_axes = axes.clone();
        problem.tag_weights = weights.clone();
        problem.tags = tags.clone();
        matched += 1;
    }
    Ok(matched)
}
