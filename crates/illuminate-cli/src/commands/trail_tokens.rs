//! Shared loader for captured prompt-trails, used by `illuminate stats`
//! (token panel) and `illuminate wiki serve` (dashboard savings tile).
//!
//! Both consumers need the same thing: parse every `.illuminate/trail/*.jsonl`
//! file in the current repo into a `Vec<TrailRecord>` so it can be folded by
//! [`illuminate_trail::savings::aggregate_tokens`]. This module centralises the
//! directory discovery + tolerant parse so the two call sites stay tiny.

use illuminate_trail::record::TrailRecord;
use std::path::{Path, PathBuf};

/// Locate the repo's `.illuminate/trail/` directory by walking up from `start`.
///
/// Returns the path only if an opted-in repo (`.illuminate/illuminate.toml`)
/// is found AND a `trail/` directory already exists under it. Returns `None`
/// when no opted-in repo is found or the trail directory has never been
/// created — both are normal "no token data yet" states, not errors.
fn find_trail_dir(start: &Path) -> Option<PathBuf> {
    let mut cur = Some(start);
    while let Some(d) = cur {
        if d.join(".illuminate").join("illuminate.toml").is_file() {
            let candidate = d.join(".illuminate").join("trail");
            return candidate.is_dir().then_some(candidate);
        }
        cur = d.parent();
    }
    None
}

/// Parse every `*.jsonl` trail in `dir` into a [`TrailRecord`].
///
/// Each file holds a single JSON object (the watcher writes one record per
/// file). Unparseable or non-`.jsonl` entries are skipped silently — a stray
/// file must never abort the token panel. Ordering follows the directory's
/// natural read order, which is irrelevant because `aggregate_tokens` is a
/// commutative fold.
fn parse_trail_dir(dir: &Path) -> Vec<TrailRecord> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("jsonl"))
        .filter_map(|p| std::fs::read_to_string(&p).ok())
        .filter_map(|c| serde_json::from_str::<TrailRecord>(c.trim()).ok())
        .collect()
}

/// Load every captured [`TrailRecord`] for the repo containing `start`.
///
/// Returns an empty vec when there is no opted-in repo, no `trail/` directory,
/// or no parseable trails — the consumers treat an empty vec as "no token data
/// captured yet" and still succeed.
pub fn load_records_from(start: &Path) -> Vec<TrailRecord> {
    match find_trail_dir(start) {
        Some(dir) => parse_trail_dir(&dir),
        None => Vec::new(),
    }
}

/// Convenience wrapper over [`load_records_from`] anchored at the current
/// working directory. Returns an empty vec if the cwd can't be read.
pub fn load_records() -> Vec<TrailRecord> {
    match std::env::current_dir() {
        Ok(cwd) => load_records_from(&cwd),
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use illuminate_trail::record::AgentKind;

    fn rec(input: Option<u64>, cache_read: Option<u64>) -> TrailRecord {
        let now = Utc::now();
        TrailRecord {
            session_id: "s".into(),
            agent: AgentKind::ClaudeCode,
            model: "m".into(),
            started_at: now,
            ended_at: now,
            repo_path: PathBuf::from("/tmp"),
            messages: vec![],
            files_touched: vec![],
            tool_invocations: vec![],
            input_tokens: input,
            output_tokens: Some(10),
            cache_creation_input_tokens: None,
            cache_read_input_tokens: cache_read,
        }
    }

    fn opt_in(root: &Path) {
        std::fs::create_dir_all(root.join(".illuminate").join("trail")).unwrap();
        std::fs::write(
            root.join(".illuminate").join("illuminate.toml"),
            "[project]\nname='t'\n",
        )
        .unwrap();
    }

    #[test]
    fn no_repo_yields_empty() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(load_records_from(tmp.path()).is_empty());
    }

    #[test]
    fn opted_in_but_no_trails_yields_empty() {
        let tmp = tempfile::tempdir().unwrap();
        opt_in(tmp.path());
        assert!(load_records_from(tmp.path()).is_empty());
    }

    #[test]
    fn parses_jsonl_skips_garbage() {
        let tmp = tempfile::tempdir().unwrap();
        opt_in(tmp.path());
        let dir = tmp.path().join(".illuminate").join("trail");
        let json = serde_json::to_string(&rec(Some(100), Some(50))).unwrap();
        std::fs::write(dir.join("a.jsonl"), json).unwrap();
        std::fs::write(dir.join("broken.jsonl"), "{ not json").unwrap();
        std::fs::write(dir.join("ignore.txt"), "{}").unwrap();

        let records = load_records_from(tmp.path());
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].input_tokens, Some(100));
        assert_eq!(records[0].cache_read_input_tokens, Some(50));
    }

    #[test]
    fn finds_trail_dir_from_subdir() {
        let tmp = tempfile::tempdir().unwrap();
        opt_in(tmp.path());
        let dir = tmp.path().join(".illuminate").join("trail");
        std::fs::write(
            dir.join("a.jsonl"),
            serde_json::to_string(&rec(Some(7), None)).unwrap(),
        )
        .unwrap();
        let sub = tmp.path().join("src").join("deep");
        std::fs::create_dir_all(&sub).unwrap();
        assert_eq!(load_records_from(&sub).len(), 1);
    }
}
