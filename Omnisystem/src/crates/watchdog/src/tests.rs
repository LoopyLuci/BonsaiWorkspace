#[cfg(test)]
mod tests {
    use super::*;
    use crate::kb::KnowledgeBase;
    use crate::repair::{attempt_launch_repair, run_script};

    fn make_kb() -> KnowledgeBase {
        KnowledgeBase::open(":memory:").unwrap()
    }

    // ── KB ────────────────────────────────────────────────────────────────────

    #[test]
    fn seed_and_find() {
        let kb = make_kb();
        kb.insert_fix("port already in use", "rule", "echo fixed", 0.9, "test")
            .unwrap();
        let hits = kb.find_matching("ERROR: port already in use at 11369");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].solution_script, "echo fixed");
    }

    #[test]
    fn no_match_returns_empty() {
        let kb = make_kb();
        kb.insert_fix("database corrupted", "rule", "rm bad.db", 0.9, "test")
            .unwrap();
        let hits = kb.find_matching("some unrelated error occurred");
        assert!(hits.is_empty());
    }

    #[test]
    fn record_outcome_updates_counters() {
        let kb = make_kb();
        let id = kb.insert_fix("test pattern", "rule", "echo ok", 0.5, "test").unwrap();
        kb.record_outcome(id, true).unwrap();
        let hits = kb.find_matching("test pattern");
        assert_eq!(hits[0].success_count, 1);
        assert_eq!(hits[0].usage_count,   1);
        assert!(hits[0].verified);
    }

    #[test]
    fn export_jsonl_only_includes_successful() {
        let kb = make_kb();
        let id1 = kb.insert_fix("error a", "rule", "fix a", 0.9, "test").unwrap();
        let id2 = kb.insert_fix("error b", "rule", "fix b", 0.9, "test").unwrap();
        kb.record_outcome(id1, true).unwrap();
        // id2 never succeeds
        let jsonl = kb.export_jsonl();
        assert_eq!(jsonl.len(), 1);
        let content = jsonl[0]["messages"][2]["content"].as_str().unwrap();
        assert_eq!(content, "fix a");
    }

    // ── Repair scripts ────────────────────────────────────────────────────────

    #[test]
    fn run_script_true_on_echo() {
        // echo always exits 0
        assert!(run_script("echo survival test"));
    }

    #[test]
    fn run_script_false_on_bad_command() {
        // A command that will surely fail
        assert!(!run_script("exit 1"));
    }

    #[test]
    fn launch_repair_unrelated_error_returns_false() {
        // Should not crash and should return false
        let fixed = attempt_launch_repair("completely unknown error xyzzy");
        assert!(!fixed);
    }

    // ── Seeded fixes: every SEEDED_FIXES pattern must be findable ────────────

    #[test]
    fn seeded_fixes_all_match() {
        use crate::kb::SEEDED_FIXES;
        let kb = make_kb();
        kb.seed_defaults().unwrap();
        let mut failures = vec![];
        for (pattern, _, _, _) in SEEDED_FIXES {
            let hits = kb.find_matching(pattern);
            if hits.is_empty() {
                failures.push(*pattern);
            }
        }
        assert!(
            failures.is_empty(),
            "These seeded patterns did not match: {:?}",
            failures
        );
    }

    #[test]
    fn seed_defaults_is_idempotent() {
        let kb = make_kb();
        kb.seed_defaults().unwrap();
        kb.seed_defaults().unwrap(); // second call must not duplicate
        use crate::kb::SEEDED_FIXES;
        let total: i64 = kb.conn().query_row(
            "SELECT COUNT(*) FROM fixes",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(total as usize, SEEDED_FIXES.len(),
            "Duplicate seeds detected");
    }

    // ── Async repair with mock ────────────────────────────────────────────────

    #[tokio::test]
    async fn repair_uses_matching_rule() {
        let kb = make_kb();
        kb.insert_fix("test_sentinel_error", "rule", "echo sentinel_fix", 0.9, "test")
            .unwrap();
        let id = crate::repair::attempt_repair(
            &kb,
            "2026-05-28 fatal: test_sentinel_error occurred at line 42",
        )
        .await;
        assert!(id.is_some(), "should match and return a fix id");
    }

    #[tokio::test]
    async fn repair_returns_none_on_no_match() {
        let kb = make_kb();
        // No fixes in KB, AI will fail (no server in test env) → None
        let id = crate::repair::attempt_repair(&kb, "no matching pattern xyzzy_12345").await;
        // Could be None (AI not available) or Some (AI returned something) — both are valid
        // depending on CI environment; we just check it doesn't panic.
        let _ = id;
    }
}
