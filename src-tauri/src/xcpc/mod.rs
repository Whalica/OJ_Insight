use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::sync::OnceLock;

use regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};

use crate::models::{XcpcContest, XcpcProblem};
use crate::sync::{browser_headers, get_text, with_cookie};

mod tags;

pub use tags::apply_problem_tags;


include!("model.rs");
include!("cache.rs");
include!("matcher.rs");
include!("rating.rs");
include!("sources/rankland.rs");
include!("sources/xcpcio.rs");
include!("sources/qoj.rs");
include!("catalog.rs");

#[cfg(test)]
mod tests {
    use super::*;

    fn online_contest(round: &str, problem_count: usize) -> XcpcContest {
        let name = format!("The 2026 ICPC Asia East Continent Online Contest ({round})");
        XcpcContest {
            id: round.into(),
            short_name: short_name(&name, "2026"),
            name,
            url: String::new(),
            date: String::new(),
            year: "2026".into(),
            series: vec!["ICPC".into()],
            stage: "网络赛".into(),
            site: "全国".into(),
            board_source: None,
            ratings_stale: false,
            problems: (0..problem_count)
                .map(|position| XcpcProblem {
                    index: fallback_problem_index(position),
                    name: "Problem".into(),
                    url: String::new(),
                    problem_id: format!("{round}-{position}"),
                    tier: None,
                    accepted_teams: None,
                    total_teams: None,
                    tag_axes: vec![],
                    tags: vec![],
                    solved: position == 0,
                })
                .collect(),
        }
    }

    fn rankland_board(round: u32) -> RanklandBoard {
        let uk = format!("icpc2026preliminary-{round}");
        RanklandBoard {
            text: format!("{uk} 2026 ICPC Asia EC网络预选赛 - 第{round}场"),
            uk,
            file_id: round.to_string(),
            direct_url: None,
            date: String::new(),
        }
    }

    fn temp_catalog_path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "oj-insight-xcpc-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn round_numbers_match_across_qoj_rankland_and_xcpcio() {
        for (text, expected) in [
            (
                "The 2026 ICPC Asia East Continent Online Contest (I)",
                Some(1),
            ),
            ("Online Contest (II)", Some(2)),
            ("Online Contest 2", Some(2)),
            (
                "official/icpc/icpc2026/icpc2026preliminary-1.srk.json",
                Some(1),
            ),
            ("data icpc 49th online qualification 2", Some(2)),
            ("2026 ICPC Asia EC网络预选赛 - 第二场", Some(2)),
            ("第十一届 CCPC 网络赛", None),
            ("CCPC Online 2026", None),
            ("Online Contest (2026)", None),
            ("icpc2026preliminary-1 网络赛第二场", None),
        ] {
            assert_eq!(contest_round(text), expected, "{text}");
        }
    }

    #[test]
    fn matches_online_rounds_independently_of_board_order() {
        let mut rankland = vec![rankland_board(2), rankland_board(1)];
        let mut xcpcio: Vec<_> = [2, 1]
            .into_iter()
            .map(|round| {
                let directory = format!("data/icpc/51st/online-qualification-{round}");
                XcpcioBoard {
                    text: directory.replace(['/', '-'], " "),
                    directory,
                }
            })
            .collect();
        for _ in 0..2 {
            for (label, round) in [("I", 1), ("II", 2)] {
                let contest = online_contest(label, 14);
                assert_eq!(
                    best_rankland_board(&contest, &rankland).unwrap().uk,
                    format!("icpc2026preliminary-{round}")
                );
                assert_eq!(
                    best_xcpcio_board(&contest, &xcpcio).unwrap().directory,
                    format!("data/icpc/51st/online-qualification-{round}")
                );
            }
            rankland.reverse();
            xcpcio.reverse();
        }
    }

    #[test]
    fn rejects_missing_wrong_or_ambiguous_rounds() {
        let contest = online_contest("I", 14);
        assert!(best_rankland_board(&contest, &[rankland_board(2)]).is_none());
        let mut board = rankland_board(1);
        board.text = "icpc2026 preliminary online 网络赛".into();
        assert!(best_rankland_board(&contest, &[board]).is_none());
        let mut duplicate = rankland_board(1);
        duplicate.uk = "another-board".into();
        duplicate.file_id = "different-file".into();
        assert!(best_rankland_board(&contest, &[rankland_board(1), duplicate]).is_none());
        let mut warmup = rankland_board(1);
        warmup.text.push_str(" warmup");
        assert!(best_rankland_board(&contest, &[warmup]).is_none());
        let board = XcpcioBoard {
            directory: "data/icpc/51st/online-qualification-1".into(),
            text: "data icpc 51st online qualification 1".into(),
        };
        assert!(best_xcpcio_board(&contest, &[board.clone(), board]).is_none());
    }

    #[test]
    fn parses_rankland_team_counts_and_omits_missing_statistics() {
        for (accepted, total, date, tier) in [
            (1019, 2535, "2026-09-06", "bronze"),
            (115, 2636, "2026-09-12", "gold"),
        ] {
            let srk = serde_json::json!({
                "contest": { "startAt": format!("{date}T13:00:00+08:00") },
                "rows": vec![serde_json::json!({}); total],
                "problems": [
                    { "alias": "A", "statistics": { "accepted": accepted } },
                    { "alias": "B", "statistics": { "submitted": 50 } },
                    { "alias": "C", "statistics": { "accepted": 0 } },
                    { "alias": "D", "statistics": { "accepted": -1 } },
                ]
            });
            let (stats, actual_date) = parse_rankland_stats(&srk);
            assert_eq!(stats["A"], (accepted, total as i64, tier.into()));
            assert_eq!(stats["C"].0, 0);
            assert!(!stats.contains_key("B"));
            assert!(!stats.contains_key("D"));
            assert_eq!(actual_date, date);
        }
    }

    #[test]
    fn successful_refresh_replaces_old_counts_and_failed_refresh_keeps_cache() {
        let mut old = online_contest("I", 14);
        old.board_source = Some("RankLand".into());
        old.date = "2026-09-12".into();
        old.problems[0].accepted_teams = Some(115);
        old.problems[0].total_teams = Some(2636);
        old.problems[0].tier = Some("gold".into());
        let mut refreshed = old.clone();
        clear_board_stats(&mut refreshed);
        let mut contests = vec![old.clone()];
        merge_refreshed_ratings(&mut contests, vec![refreshed.clone()]);
        assert_eq!(
            serde_json::to_value(&contests[0]).unwrap(),
            serde_json::to_value(&old).unwrap()
        );
        refreshed.board_source = Some("RankLand".into());
        refreshed.date = "2026-09-06".into();
        refreshed.problems[0].accepted_teams = Some(1019);
        refreshed.problems[0].total_teams = Some(2535);
        refreshed.problems[0].tier = Some("bronze".into());
        merge_refreshed_ratings(&mut contests, vec![refreshed]);
        assert_eq!(contests[0].problems[0].accepted_teams, Some(1019));
        assert_eq!(contests[0].date, "2026-09-06");
        assert!(contests[0].problems[0].solved);
    }

    #[test]
    fn partial_refresh_keeps_previous_valid_problem_statistics() {
        let mut old = online_contest("I", 2);
        old.board_source = Some("XCPCIO".into());
        old.problems[0].tier = Some("gold".into());
        old.problems[0].accepted_teams = Some(5);
        let mut next = old.clone();
        clear_board_stats(&mut next);
        next.board_source = Some("RankLand".into());
        next.problems[1].tier = Some("bronze".into());
        next.problems[1].accepted_teams = Some(1019);
        let mut contests = vec![old];
        merge_refreshed_ratings(&mut contests, vec![next]);
        assert_eq!(contests[0].problems[0].accepted_teams, Some(5));
        assert_eq!(contests[0].problems[1].accepted_teams, Some(1019));
        assert_eq!(
            contests[0].board_source.as_deref(),
            Some("RankLand + XCPCIO")
        );
    }

    #[test]
    fn migrates_old_online_ratings_without_discarding_problems_or_other_contests() {
        let path = temp_catalog_path();
        let mut online = online_contest("I", 14);
        online.board_source = Some("RankLand".into());
        online.date = "2026-09-12".into();
        online.problems[0].accepted_teams = Some(115);
        online.problems[0].total_teams = Some(2636);
        online.problems[0].tier = Some("gold".into());
        let mut regional = online.clone();
        regional.id = "regional".into();
        regional.name = "The 2026 ICPC Asia Nanjing Regional Contest".into();
        let empty_online = online_contest("II", 0);
        let mut cache =
            serde_json::json!({ "version": 4, "contests": [online, regional, empty_online] });
        for item in cache["contests"].as_array_mut().unwrap() {
            item.as_object_mut().unwrap().remove("ratingsStale");
        }
        std::fs::write(&path, serde_json::to_string(&cache).unwrap()).unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let items = runtime
            .block_on(load_catalog(&Client::new(), &path, "", false))
            .unwrap();
        assert!(items[0].ratings_stale);
        assert!(items[0].board_source.is_none());
        assert!(items[0].date.is_empty());
        assert_eq!(items[0].problems.len(), 14);
        assert!(items[0].problems[0].solved);
        assert!(items[0]
            .problems
            .iter()
            .all(|problem| problem.tier.is_none()
                && problem.accepted_teams.is_none()
                && problem.total_teams.is_none()));
        assert!(!items[1].ratings_stale);
        assert_eq!(items[1].problems[0].accepted_teams, Some(115));
        assert!(!items[2].ratings_stale);
        save_catalog(&path, &items).unwrap();
        let persisted = runtime
            .block_on(load_catalog(&Client::new(), &path, "", false))
            .unwrap();
        assert!(persisted[0].ratings_stale);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    #[ignore = "fetches the public 2026 Online I/II standings from the network"]
    fn live_online_rounds_have_their_own_counts_and_dates() {
        let path = temp_catalog_path();
        let mut contests = vec![online_contest("I", 14), online_contest("II", 12)];
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap();
        runtime
            .block_on(sync_public_ratings(&client, &path, &mut contests))
            .unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(contests[0].problems[0].accepted_teams, Some(1019));
        assert_eq!(contests[0].problems[0].total_teams, Some(2535));
        assert_eq!(contests[0].date, "2026-09-06");
        assert_eq!(contests[1].problems[0].accepted_teams, Some(115));
        assert_eq!(contests[1].date, "2026-09-12");
        assert!(contests[0]
            .problems
            .iter()
            .all(|problem| problem.accepted_teams.is_some()));
        println!("Online I: A = 1019 / 2535, date = 2026-09-06; Online II: A = 115 / 2636, date = 2026-09-12");
    }

    #[test]
    fn parses_realistic_category_rows() {
        let html = r#"<table><tr><td><a href="/contest/2513">The 2025 ICPC Asia Nanjing Regional Contest</a></td><td><a title="A. Array" href="/contest/2513/problem/14001/statement/zh_cn">A. Array</a><a href="/problem/14002">B. Bitset</a></td></tr><tr><td><a href="/category/84">Nanjing</a></td></tr></table>"#;
        let parsed = parse_category(html);
        assert_eq!(parsed.child_categories, vec!["84"]);
        assert_eq!(parsed.contests.len(), 1);
        assert_eq!(parsed.contests[0].problems[0].name, "Array");
        assert_eq!(parsed.contests[0].problems[1].index, "B");
        assert_eq!(parsed.contests[0].problems[1].name, "Bitset");
        assert_eq!(
            parsed.contests[0].problems[1].url,
            "https://qoj.ac/problem/14002"
        );
    }

    #[test]
    fn abbreviates_contests_without_losing_rounds_or_divisions() {
        for (name, expected) in [
            (
                "The 2026 ICPC Asia East Continent Online Contest (I)",
                "2026 ICPC 亚洲东区网络赛 (I)",
            ),
            (
                "The 2026 ICPC Asia East Continent Online Contest (II)",
                "2026 ICPC 亚洲东区网络赛 (II)",
            ),
            (
                "The 2025 ICPC Asia East Continent Final Contest",
                "2025 ICPC 亚洲东区总决赛",
            ),
            ("The 2025 ICPC World Finals", "2025 ICPC 全球总决赛"),
            (
                "第十一届中国大学生程序设计竞赛 女生专场（CCPC 2025 Women's Division）",
                "2025 CCPC 女生赛",
            ),
            (
                "第十一届中国大学生程序设计竞赛 高职专场（CCPC 2025 Vocational Division）",
                "2025 CCPC 高职赛",
            ),
            (
                "第十一届中国大学生程序设计竞赛网络预选赛（CCPC Online 2025）",
                "2025 CCPC 网络赛",
            ),
            (
                "The 2025 Hunan Collegiate Programming Contest",
                "2025 湖南省赛",
            ),
            ("2025 年上海市大学生程序设计竞赛", "2025 上海市赛"),
            (
                "The 2023 ICPC Asia Xi’an Regional Contest",
                "2023 ICPC 西安区域赛",
            ),
            (
                "The 2020 ICPC Asia Yinchuan Regional Contest",
                "2020 ICPC 银川区域赛",
            ),
            (
                "The 2015 ICPC Asia Fuzhou Regional Contest",
                "2015 ICPC 福州区域赛",
            ),
            (
                "第八届 CCPC 河南省大学生程序设计竞赛",
                "第8届 CCPC 河南省赛",
            ),
            ("第二十届东北地区大学生程序设计竞赛", "第20届 东北地区赛"),
            (
                "The 10th Hebei Collegiate Programming Contest",
                "第10届 河北省赛",
            ),
            ("第十一届中国大学生程序设计竞赛总决赛", "第11届 CCPC 总决赛"),
        ] {
            assert_eq!(short_name(name, &extract_year(name)), expected, "{name}");
        }
    }

    #[test]
    fn preserves_titles_when_abbreviations_would_be_ambiguous() {
        for name in [
            "CCPC 河南省大学生程序设计竞赛",
            "ACM-HK Programming Contest 2026",
            "The 2026 ICPC Asia Unknown City Regional Contest",
            "2026 CCPC 全国邀请赛（南昌）暨第三届江西省赛",
        ] {
            assert_eq!(short_name(name, &extract_year(name)), name);
        }
    }

    #[test]
    fn recognizes_chinese_and_case_insensitive_sites_and_local_contests() {
        assert_eq!(
            classify_site("2026 CCPC 全国邀请赛（南昌）暨第三届江西省赛"),
            "南昌"
        );
        assert_eq!(classify_site("2025 ICPC hangzhou Regional Contest"), "杭州");
        assert_eq!(classify_site("2025 ICPC 福州区域赛"), "福州");
        assert_eq!(
            classify_site("The 2017 ACM-ICPC Asia Ürümqi Regional Contest"),
            "乌鲁木齐"
        );
        assert_eq!(
            classify_site("The 2025 ICPC Asia Xiangtan Regional Contest"),
            "全国"
        );
        assert_eq!(
            classify_series("The 2025 Hunan Collegiate Programming Contest"),
            vec!["省赛"]
        );
        assert_eq!(classify_stage("CCPC 2024 北京市赛"), "市赛");
    }

    #[test]
    fn refreshes_cached_names_offline_without_losing_ratings() {
        let path = std::env::temp_dir().join(format!(
            "oj-insight-xcpc-cache-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let contest = XcpcContest {
            id: "1".into(),
            name: "The 2026 ICPC Asia East Continent Online Contest (II)".into(),
            short_name: "2026 ICPC 全国网络赛".into(),
            url: String::new(),
            date: "2026-09-13".into(),
            year: "2026".into(),
            series: vec!["ICPC".into()],
            stage: "网络赛".into(),
            site: "全国".into(),
            board_source: Some("XCPCIO".into()),
            ratings_stale: false,
            problems: vec![XcpcProblem {
                index: "A".into(),
                name: "Array".into(),
                url: String::new(),
                problem_id: "2".into(),
                tier: Some("gold".into()),
                accepted_teams: Some(5),
                total_teams: Some(100),
                tag_axes: vec![],
                tags: vec![],
                solved: true,
            }],
        };
        save_catalog(&path, &[contest.clone()]).unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = runtime.block_on(load_catalog(&Client::new(), &path, "", false));
        std::fs::remove_file(&path).unwrap();
        let items = result.unwrap();
        assert_eq!(items[0].short_name, "2026 ICPC 亚洲东区网络赛 (II)");
        assert_eq!(items[0].board_source, contest.board_source);
        assert_eq!(items[0].date, contest.date);
        assert_eq!(
            serde_json::to_value(&items[0].problems).unwrap(),
            serde_json::to_value(&contest.problems).unwrap()
        );
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
        assert_eq!(
            explicit_problem_label("D. Dynamic Graph", 3).unwrap().1,
            "Dynamic Graph"
        );
        assert_eq!(
            explicit_problem_label("Problem K: Knowledge", 10)
                .unwrap()
                .0,
            "K"
        );
        assert!(explicit_problem_label("MOD. Modular Arithmetic", 0).is_none());
    }

    #[test]
    fn keeps_indefinite_article_as_part_of_problem_name() {
        assert_eq!(
            problem_label("A Perfect Match", 3),
            ("D".into(), "A Perfect Match".into())
        );
        assert_eq!(
            problem_label("A Perfect Match", 0),
            ("A".into(), "A Perfect Match".into())
        );
        assert_eq!(
            problem_label("A. Perfect Match", 0),
            ("A".into(), "Perfect Match".into())
        );
        assert_eq!(
            problem_label("Problem B A Long Journey", 1),
            ("B".into(), "A Long Journey".into())
        );
        assert_eq!(problem_label("MOD", 0), ("A".into(), "MOD".into()));
        assert_eq!(problem_label("GG", 1), ("B".into(), "GG".into()));
        assert_eq!(
            problem_label("MOD. Modular Arithmetic", 0),
            ("A".into(), "MOD. Modular Arithmetic".into())
        );
        assert_eq!(fallback_problem_index(26), "AA");
    }

    #[test]
    fn requests_details_for_placeholder_problem_names() {
        let contest = XcpcContest {
            id: "1".into(),
            name: "Contest".into(),
            short_name: "Contest".into(),
            url: String::new(),
            date: String::new(),
            year: "2026".into(),
            series: vec!["ICPC".into()],
            stage: "区域赛".into(),
            site: "全国".into(),
            board_source: None,
            ratings_stale: false,
            problems: vec![XcpcProblem {
                index: "B".into(),
                name: "题目".into(),
                url: String::new(),
                problem_id: "2".into(),
                tier: None,
                accepted_teams: None,
                total_teams: None,
                tag_axes: vec![],
                tags: vec![],
                solved: false,
            }],
        };
        assert!(contest_needs_problem_details(&contest));
    }

    #[test]
    fn matches_xcpcio_edition_paths_without_a_calendar_year() {
        let contest = XcpcContest {
            id: "1".into(),
            name: "The 2023 ICPC Asia Xi'an Regional Contest".into(),
            short_name: String::new(),
            url: String::new(),
            date: String::new(),
            year: "2023".into(),
            series: vec!["ICPC".into()],
            stage: "区域赛".into(),
            site: "西安".into(),
            board_source: None,
            ratings_stale: false,
            problems: Vec::new(),
        };
        let board = XcpcioBoard {
            directory: "data/icpc/48th/xian".into(),
            text: "data icpc 48th xian".into(),
        };
        assert!(xcpcio_match_score(&contest, &board).is_some());
    }
}
