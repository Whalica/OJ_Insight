use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{DateTime, Duration, LocalResult, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::models::*;

mod connection;

pub use connection::open;


include!("accounts.rs");
include!("relationships.rs");
include!("submissions.rs");
include!("analytics.rs");
include!("ratings.rs");

mod training;

pub use training::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn entry(platform: &str, account: &str) -> AccountConfig {
        AccountConfig {
            platform: platform.into(),
            account: account.into(),
            secret: String::new(),
        }
    }

    fn remote(platform: &str, account: &str) -> RemoteData {
        RemoteData {
            platform: platform.into(),
            account: account.into(),
            display_name: None,
            submissions: vec![Submission {
                platform: platform.into(),
                account: account.into(),
                source: "oj".into(),
                source_day: None,
                submission_id: "shared-id".into(),
                problem_key: "A".into(),
                problem_id: "A".into(),
                problem_name: "A".into(),
                problem_url: String::new(),
                epoch_second: 1_767_196_800,
                language: "C++".into(),
                difficulty: Some("1200".into()),
                participant_type: "CONTESTANT".into(),
                tags: vec![],
            }],
            aggregates: vec![AggregateDay {
                day: "2026-01-01".into(),
                epoch_second: None,
                metric: "activity".into(),
                count: 3,
                note: String::new(),
            }],
            solved_count: Some(1),
            difficulty: vec![DifficultyStat {
                label: "1200".into(),
                count: 1,
                order: 1200,
            }],
            knowledge: Some(vec![KnowledgeStat {
                axis: "图论与树".into(),
                count: 1,
            }]),
            ratings: Some(vec![RatingPoint {
                contest_id: "1".into(),
                contest_name: "Round 1".into(),
                epoch_second: 1_767_196_800,
                old_rating: 1200,
                new_rating: 1300,
                rank: Some(100),
            }]),
            activity_only: false,
            notes: vec![],
            cursor_epoch: 123,
            replace_submissions: false,
            replace_aggregates: false,
        }
    }

    #[test]
    fn watched_people_establish_a_baseline_then_emit_each_new_ac_once() {
        let mut conn = open(Path::new(":memory:")).unwrap();
        save_watched_person(&mut conn, "codeforces", "teammate", "小明", "队友", "").unwrap();
        let person_id = get_watched_people(&conn).unwrap()[0].id;

        let mut initial = remote("codeforces", "teammate");
        initial.cursor_epoch = 200;
        assert!(apply_watched_remote(&mut conn, person_id, &initial)
            .unwrap()
            .is_empty());
        assert!(get_watched_events(&conn, 20).unwrap().is_empty());
        assert!(get_watched_people(&conn).unwrap()[0].initialized);

        let mut next = initial.clone();
        let mut newer = next.submissions[0].clone();
        newer.submission_id = "new-ac".into();
        newer.problem_id = "B".into();
        newer.problem_key = "B".into();
        newer.problem_name = "New AC".into();
        newer.epoch_second += 60;
        next.submissions.push(newer);
        next.cursor_epoch = 300;
        let events = apply_watched_remote(&mut conn, person_id, &next).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].problem_name, "New AC");
        assert_eq!(get_watched_events(&conn, 20).unwrap().len(), 1);

        assert!(apply_watched_remote(&mut conn, person_id, &next)
            .unwrap()
            .is_empty());
        let event_id = get_watched_events(&conn, 20).unwrap()[0].id;
        dismiss_watched_event(&conn, event_id).unwrap();
        assert!(get_watched_events(&conn, 20).unwrap()[0].dismissed);
        assert!(get_pending_watched_notifications(&conn).unwrap().is_empty());

        delete_watched_person(&mut conn, person_id).unwrap();
        assert!(get_watched_events(&conn, 20).unwrap().is_empty());
        assert!(get_watched_people(&conn).unwrap().is_empty());
    }

    #[test]
    fn watched_people_batch_save_is_atomic() {
        let mut conn = open(Path::new(":memory:")).unwrap();
        let bindings = vec![
            WatchedBindingInput {
                platform: "codeforces".into(),
                account: "cf-user".into(),
                secret: String::new(),
            },
            WatchedBindingInput {
                platform: "atcoder".into(),
                account: "at-user".into(),
                secret: String::new(),
            },
        ];
        save_watched_people(&mut conn, "小明", "队友", &bindings).unwrap();
        assert_eq!(get_watched_people(&conn).unwrap().len(), 2);

        let invalid = vec![
            WatchedBindingInput {
                platform: "luogu".into(),
                account: "lg-user".into(),
                secret: String::new(),
            },
            WatchedBindingInput {
                platform: "qoj".into(),
                account: String::new(),
                secret: String::new(),
            },
        ];
        assert!(save_watched_people(&mut conn, "小明", "队友", &invalid).is_err());
        let people = get_watched_people(&conn).unwrap();
        assert_eq!(people.len(), 2);
        assert!(!people.iter().any(|person| person.platform == "luogu"));
    }

    #[test]
    fn editing_watched_person_can_add_a_platform_with_add_validation() {
        let mut conn = open(Path::new(":memory:")).unwrap();
        save_watched_person(&mut conn, "codeforces", "cf-user", "小明", "队友", "").unwrap();
        save_watched_person(&mut conn, "luogu", "taken-user", "小红", "", "").unwrap();
        let people = get_watched_people(&conn).unwrap();
        let person_id = people
            .iter()
            .find(|person| person.platform == "codeforces")
            .unwrap()
            .id;

        let existing = WatchedBindingInput {
            platform: "codeforces".into(),
            account: "cf-user".into(),
            secret: String::new(),
        };
        let duplicate_addition = WatchedBindingInput {
            platform: "luogu".into(),
            account: " TAKEN-USER ".into(),
            secret: String::new(),
        };
        let error = edit_watched_person(
            &mut conn,
            &[person_id],
            "小明改名",
            "",
            &[existing.clone(), duplicate_addition],
        )
        .unwrap_err();
        assert!(error.contains("该用户已经被添加了"));
        assert!(edit_watched_person(&mut conn, &[person_id], "小明", "队友", &[]).is_err());

        let new_platform = WatchedBindingInput {
            platform: "atcoder".into(),
            account: "at-user".into(),
            secret: String::new(),
        };
        edit_watched_person(
            &mut conn,
            &[person_id],
            "小明",
            "队友",
            &[existing, new_platform],
        )
        .unwrap();
        let people = get_watched_people(&conn).unwrap();
        assert_eq!(people.len(), 3);
        assert_eq!(
            people
                .iter()
                .filter(|person| person.nickname == "小明")
                .count(),
            2
        );
        assert!(!people.iter().any(|person| person.nickname == "小明改名"));
        assert!(people
            .iter()
            .any(|person| person.platform == "atcoder" && person.account == "at-user"));
    }

    #[test]
    fn editing_one_watched_account_only_changes_that_platform_and_resets_its_baseline() {
        let mut conn = open(Path::new(":memory:")).unwrap();
        save_watched_person(&mut conn, "codeforces", "cf-user", "小明", "队友", "").unwrap();
        save_watched_person(&mut conn, "atcoder", "at-user", "小明", "队友", "").unwrap();
        let people = get_watched_people(&conn).unwrap();
        let atcoder_id = people.iter().find(|person| person.platform == "atcoder").unwrap().id;
        let codeforces_id = people.iter().find(|person| person.platform == "codeforces").unwrap().id;
        mark_watched_checking(&conn, atcoder_id).unwrap();
        edit_watched_person(
            &mut conn,
            &[atcoder_id],
            "小明",
            "队友",
            &[WatchedBindingInput { platform: "atcoder".into(), account: "new-at-user".into(), secret: String::new() }],
        ).unwrap();
        let people = get_watched_people(&conn).unwrap();
        assert_eq!(people.len(), 2);
        let updated = people.iter().find(|person| person.id == atcoder_id).unwrap();
        assert_eq!(updated.account, "new-at-user");
        assert!(!updated.initialized);
        assert_eq!(updated.status, "idle");
        assert_eq!(people.iter().find(|person| person.id == codeforces_id).unwrap().account, "cf-user");
    }

    fn count(conn: &Connection, table: &str, platform: &str, account: &str) -> i64 {
        conn.query_row(
            &format!("SELECT COUNT(*) FROM {table} WHERE platform=? AND account=?"),
            params![platform, account],
            |r| r.get(0),
        )
        .unwrap()
    }

    #[test]
    fn removing_one_id_purges_all_owned_records_and_preserves_others() {
        let mut conn = open(Path::new(":memory:")).unwrap();
        replace_all_accounts(
            &mut conn,
            &[
                entry("codeforces", "alice"),
                entry("codeforces", "bob"),
                entry("atcoder", "alice"),
            ],
        )
        .unwrap();
        for (platform, account) in [
            ("codeforces", "alice"),
            ("codeforces", "bob"),
            ("atcoder", "alice"),
        ] {
            apply_remote(&mut conn, &remote(platform, account)).unwrap();
        }
        // Same submission id must coexist across accounts.
        assert_eq!(count(&conn, "submissions", "codeforces", "alice"), 1);
        replace_accounts(&mut conn, "codeforces", &[entry("codeforces", "bob")]).unwrap();
        for table in [
            "submissions",
            "daily_aggregates_accounts",
            "difficulty_stats_accounts",
            "knowledge_stats_accounts",
            "platform_stats_accounts",
            "rating_history",
            "account_sync_state",
        ] {
            assert_eq!(count(&conn, table, "codeforces", "alice"), 0, "{table}");
            assert!(count(&conn, table, "codeforces", "bob") > 0, "{table}");
            assert!(count(&conn, table, "atcoder", "alice") > 0, "{table}");
        }
        assert_eq!(get_cursor(&conn, "codeforces", "alice").unwrap(), 0);
        assert_eq!(
            ratings_for_platform(&conn, "codeforces", None)
                .unwrap()
                .len(),
            1
        );
        let detail = day_detail(
            &conn,
            "2026-01-01",
            Some("codeforces"),
            None,
            None,
            "Asia/Shanghai",
        )
        .unwrap();
        assert_eq!(detail.items.len(), 1);
        assert_eq!(detail.items[0].account, "bob");
    }

    #[test]
    fn renamed_or_removed_ids_cannot_accept_late_sync_results() {
        let mut conn = open(Path::new(":memory:")).unwrap();
        replace_accounts(&mut conn, "codeforces", &[entry("codeforces", "old")]).unwrap();
        apply_remote(&mut conn, &remote("codeforces", "old")).unwrap();
        replace_accounts(&mut conn, "codeforces", &[entry("codeforces", "new")]).unwrap();
        assert!(apply_remote(&mut conn, &remote("codeforces", "old")).is_err());
        assert_eq!(get_cursor(&conn, "codeforces", "new").unwrap(), 0);
        save_account(&mut conn, "codeforces", "", "").unwrap();
        assert!(get_accounts(&conn).unwrap().is_empty());
        assert!(ratings_for_platform(&conn, "codeforces", None)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn bulk_account_save_rolls_back_all_platforms_on_error() {
        let mut conn = open(Path::new(":memory:")).unwrap();
        replace_all_accounts(
            &mut conn,
            &[entry("codeforces", "alice"), entry("atcoder", "bob")],
        )
        .unwrap();
        apply_remote(&mut conn, &remote("codeforces", "alice")).unwrap();
        conn.execute_batch("CREATE TRIGGER fail_save BEFORE INSERT ON account_entries WHEN NEW.account='fail' BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap();
        assert!(replace_all_accounts(&mut conn, &[entry("atcoder", "fail")]).is_err());
        assert_eq!(get_accounts(&conn).unwrap().len(), 2);
        assert_eq!(count(&conn, "submissions", "codeforces", "alice"), 1);
    }

    #[test]
    fn clearing_records_keeps_accounts_and_resets_cursors_and_ratings() {
        let mut conn = open(Path::new(":memory:")).unwrap();
        replace_accounts(&mut conn, "codeforces", &[entry("codeforces", "alice")]).unwrap();
        apply_remote(&mut conn, &remote("codeforces", "alice")).unwrap();
        clear_all(&mut conn).unwrap();
        assert_eq!(get_accounts(&conn).unwrap().len(), 1);
        assert_eq!(get_cursor(&conn, "codeforces", "alice").unwrap(), 0);
        assert!(ratings_for_platform(&conn, "codeforces", None)
            .unwrap()
            .is_empty());
        assert_eq!(
            statuses(&conn)
                .unwrap()
                .iter()
                .map(|s| s.cached_records)
                .sum::<i64>(),
            0
        );
    }

    #[test]
    fn failed_rating_refresh_preserves_cache_but_empty_success_clears_it() {
        let mut conn = open(Path::new(":memory:")).unwrap();
        replace_accounts(&mut conn, "codeforces", &[entry("codeforces", "alice")]).unwrap();
        let mut data = remote("codeforces", "alice");
        apply_remote(&mut conn, &data).unwrap();
        data.ratings = None;
        apply_remote(&mut conn, &data).unwrap();
        assert_eq!(count(&conn, "rating_history", "codeforces", "alice"), 1);
        data.ratings = Some(vec![]);
        apply_remote(&mut conn, &data).unwrap();
        assert_eq!(count(&conn, "rating_history", "codeforces", "alice"), 0);
    }

    #[test]
    fn unchanged_incremental_overlap_is_not_reported_as_new_or_updated() {
        let mut conn = open(Path::new(":memory:")).unwrap();
        replace_accounts(&mut conn, "codeforces", &[entry("codeforces", "alice")]).unwrap();
        assert_eq!(
            apply_remote(&mut conn, &remote("codeforces", "alice")).unwrap(),
            (1, 0)
        );
        assert_eq!(
            apply_remote(&mut conn, &remote("codeforces", "alice")).unwrap(),
            (0, 0)
        );
    }

    #[test]
    fn codeforces_metadata_backfill_runs_only_once_after_success() {
        let mut conn = open(Path::new(":memory:")).unwrap();
        replace_accounts(&mut conn, "codeforces", &[entry("codeforces", "alice")]).unwrap();
        let mut data = remote("codeforces", "alice");
        data.submissions[0].participant_type.clear();
        apply_remote(&mut conn, &data).unwrap();
        assert!(needs_tag_backfill(&conn, "codeforces", "alice").unwrap());
        data.submissions[0].participant_type = "CONTESTANT".into();
        data.replace_submissions = true;
        apply_remote(&mut conn, &data).unwrap();
        assert!(!needs_tag_backfill(&conn, "codeforces", "alice").unwrap());
    }

    #[test]
    fn duplicate_provider_rating_rows_are_safely_collapsed() {
        let mut conn = open(Path::new(":memory:")).unwrap();
        replace_accounts(&mut conn, "nowcoder", &[entry("nowcoder", "10001")]).unwrap();
        let mut data = remote("nowcoder", "10001");
        let mut newer = data.ratings.as_ref().unwrap()[0].clone();
        newer.new_rating = 1400;
        data.ratings.as_mut().unwrap().push(newer);
        apply_remote(&mut conn, &data).unwrap();
        assert_eq!(count(&conn, "rating_history", "nowcoder", "10001"), 1);
    }

    #[test]
    fn difficulty_detail_returns_each_account_problem_once() {
        let mut conn = open(Path::new(":memory:")).unwrap();
        replace_accounts(&mut conn, "codeforces", &[entry("codeforces", "alice")]).unwrap();
        let mut data = remote("codeforces", "alice");
        let mut duplicate = data.submissions[0].clone();
        duplicate.submission_id = "second-ac".into();
        duplicate.epoch_second += 60;
        data.submissions.push(duplicate);
        apply_remote(&mut conn, &data).unwrap();
        let detail = difficulty_detail(&conn, "codeforces", "1200", None, None).unwrap();
        assert_eq!(detail.count, 1);
        assert_eq!(detail.items.len(), 1);
        assert_eq!(detail.items[0].submission_id, "second-ac");
    }

    #[test]
    fn difficulty_includes_unrated_problems() {
        let mut conn = open(Path::new(":memory:")).unwrap();
        replace_accounts(&mut conn, "codeforces", &[entry("codeforces", "alice")]).unwrap();
        let mut data = remote("codeforces", "alice");
        let mut unrated = data.submissions[0].clone();
        unrated.submission_id = "unrated-ac".into();
        unrated.problem_key = "B".into();
        unrated.problem_id = "B".into();
        unrated.problem_name = "Unrated".into();
        unrated.difficulty = None;
        data.submissions.push(unrated);
        data.solved_count = Some(2);
        apply_remote(&mut conn, &data).unwrap();
        let buckets = difficulty_for_platform(&conn, "codeforces", None, None, None, None).unwrap();
        assert_eq!(
            buckets.first().map(|item| item.label.as_str()),
            Some(UNRATED_LABEL)
        );
        assert_eq!(
            buckets
                .iter()
                .find(|item| item.label == UNRATED_LABEL)
                .map(|item| item.count),
            Some(1)
        );
        let detail = difficulty_detail(&conn, "codeforces", UNRATED_LABEL, None, None).unwrap();
        assert_eq!(detail.count, 1);
        assert_eq!(detail.items.len(), 1);
    }

    #[test]
    fn daily_difficulty_prefers_rated_problem_over_unrated_problem() {
        let mut conn = open(Path::new(":memory:")).unwrap();
        replace_accounts(&mut conn, "codeforces", &[entry("codeforces", "alice")]).unwrap();
        let mut data = remote("codeforces", "alice");
        let mut unrated = data.submissions[0].clone();
        unrated.submission_id = "unrated-ac".into();
        unrated.problem_key = "B".into();
        unrated.problem_id = "B".into();
        unrated.problem_name = "Unrated".into();
        unrated.difficulty = None;
        data.submissions.push(unrated);
        data.solved_count = Some(2);
        apply_remote(&mut conn, &data).unwrap();
        let daily = difficulty_daily_for_platform(
            &conn,
            "codeforces",
            None,
            None,
            None,
            None,
            "Asia/Shanghai",
        )
        .unwrap();
        assert_eq!(daily.len(), 1);
        assert_eq!(daily[0].order, 1200);
        assert_eq!(daily[0].label, "1200");
    }

    #[test]
    fn reopening_does_not_reimport_legacy_caches() {
        let path = std::env::temp_dir().join(format!(
            "oj-insight-test-{}-{}.sqlite3",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap()
        ));
        {
            let mut conn = open(&path).unwrap();
            replace_accounts(
                &mut conn,
                "codeforces",
                &[entry("codeforces", "alice"), entry("codeforces", "removed")],
            )
            .unwrap();
            apply_remote(&mut conn, &remote("codeforces", "removed")).unwrap();
            // Simulate v0.4 removing just the config, leaving account caches.
            conn.execute("DELETE FROM account_entries WHERE account='removed'", [])
                .unwrap();
            conn.execute_batch("INSERT INTO daily_aggregates VALUES('codeforces','2026-01-01','activity',99,'legacy',NULL);").unwrap();
        }
        {
            let mut conn = open(&path).unwrap();
            assert_eq!(
                count(&conn, "daily_aggregates_accounts", "codeforces", "alice"),
                0
            );
            assert_eq!(count(&conn, "submissions", "codeforces", "removed"), 0);
            assert_eq!(count(&conn, "rating_history", "codeforces", "removed"), 0);
            replace_accounts(&mut conn, "codeforces", &[]).unwrap();
        }
        {
            let conn = open(&path).unwrap();
            assert!(get_accounts(&conn).unwrap().is_empty());
            assert_eq!(
                statuses(&conn)
                    .unwrap()
                    .iter()
                    .map(|s| s.cached_records)
                    .sum::<i64>(),
                0
            );
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn utc8_day_changes_at_china_midnight() {
        assert_eq!(
            day_in_time_zone(1_767_196_799, "Asia/Shanghai"),
            "2025-12-31"
        );
        assert_eq!(
            day_in_time_zone(1_767_196_800, "Asia/Shanghai"),
            "2026-01-01"
        );
    }

    #[test]
    fn day_range_follows_dst_timezone() {
        let (start, end) = day_epoch_range("2026-03-08", "America/New_York").unwrap();
        assert_eq!(end - start + 1, 23 * 3600);
    }

    #[test]
    fn difficulty_buckets_follow_platform_levels() {
        assert_eq!(bucket_label("codeforces", "1350"), (1300, "1300".into()));
        assert_eq!(bucket_label("luogu", "7"), (7, "省选/NOI-".into()));
        assert_eq!(bucket_label("luogu", "提高"), (5, "提高".into()));
        assert_eq!(bucket_label("leetcode", "Medium"), (2, "Medium".into()));
        assert_eq!(bucket_label("qoj", "bronze"), (2, "铜题".into()));
        assert_eq!(bucket_label("qoj", "金题"), (4, "金题".into()));
        assert_eq!(
            bucket_label("codeforces", ""),
            (UNRATED_ORDER, UNRATED_LABEL.into())
        );
        assert_eq!(
            bucket_label("atcoder", "unknown"),
            (UNRATED_ORDER, UNRATED_LABEL.into())
        );
    }

    #[test]
    fn knowledge_estimate_values_difficulty_and_uses_evidence_as_confidence() {
        let easy = robust_knowledge_estimate(
            knowledge_level("leetcode", "Easy").unwrap(),
            50.0,
            5.0,
            "leetcode",
        );
        let hard = robust_knowledge_estimate(
            knowledge_level("leetcode", "Hard").unwrap(),
            50.0,
            5.0,
            "leetcode",
        );
        assert!(hard > easy);

        let one = robust_knowledge_estimate(86.0, 50.0, 1.0, "leetcode");
        let two = robust_knowledge_estimate(86.0, 50.0, 2.0, "leetcode");
        let three = robust_knowledge_estimate(86.0, 50.0, 3.0, "leetcode");
        assert!(two > one && three > two);
        assert!(three - two < two - one);
        assert!(three < 95.0);
    }
}
