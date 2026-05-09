//! End-to-end golden-path smoke test.
//!
//! Documents and exercises the developer's full happy path through the CLI:
//!
//! 1. Set up a tempdir repo with `.illuminate/illuminate.toml` (one
//!    rejected_pattern policy), `.illuminate/interview.yaml` (one
//!    decision), `CLAUDE.md`, and a tiny `src/lib.rs` with a single
//!    function so there's a real symbol to discover.
//! 2. `illuminate index` — builds the code symbol index.
//! 3. `illuminate bootstrap` + `illuminate wiki rebuild` — extracts
//!    decisions from agent files and interview.yaml, materializes wiki
//!    pages, and registers them as episodes in `.illuminate/graph.db`.
//! 4. `illuminate audit "add Redis caching to payments"` — should exit
//!    2 (violation) and surface the `no_redis` policy hit. Re-run with
//!    `--json` to verify the structured response shape.
//! 5. `illuminate audit "Add a small refactor of process_payment" --json
//!    --index-db ... src/lib.rs` — should exit 0 (pass), with `impact`
//!    populated from the symbol index.
//! 6. `illuminate explain src/lib.rs` — must not crash on the tempdir DB.
//!
//! The whole thing is best-effort: when fastembed models or ONNX
//! extraction models aren't installed we relax the relevant assertions
//! rather than fail the test, since the golden path must remain green
//! for first-install users without any model weights cached.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn cargo_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_illuminate"))
}

fn run(repo: &Path, args: &[&str]) -> std::process::Output {
    Command::new(cargo_bin())
        .args(args)
        .current_dir(repo)
        // Disable embed loading by default — the golden path must work on
        // machines without `~/.cache/fastembed`. Individual assertions that
        // depend on embedding can override this when a model is available.
        .env("ILLUMINATE_NO_EMBED", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("subprocess must run")
}

/// Plant the four files that drive the golden path.
fn setup_repo(repo: &Path) {
    // `git init` keeps the project signal honest (some bootstrap sources
    // walk git history). Best-effort — older `git` may print warnings.
    let _ = Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(repo)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output();

    fs::create_dir_all(repo.join("src")).unwrap();
    fs::write(
        repo.join("src/lib.rs"),
        "use std::collections::HashMap;\n\npub fn process_payment() {\n    let _: HashMap<String, u32> = HashMap::new();\n}\n",
    )
    .unwrap();

    fs::create_dir_all(repo.join(".illuminate/wiki/decisions")).unwrap();
    fs::create_dir_all(repo.join(".illuminate/wiki/patterns")).unwrap();
    fs::create_dir_all(repo.join(".illuminate/wiki/failures")).unwrap();
    fs::create_dir_all(repo.join(".illuminate/wiki/modules")).unwrap();

    fs::write(
        repo.join(".illuminate/illuminate.toml"),
        "[project]\nname = 'golden-path'\n\n[policies.no_redis]\nrule = \"rejected_pattern\"\npattern = \"Redis\"\nreason = \"Use in-memory LRU instead\"\nseverity = \"error\"\ndecision_ref = \"dec-no-redis\"\n",
    )
    .unwrap();

    fs::write(
        repo.join(".illuminate/interview.yaml"),
        "language: \"Rust 2024\"\navoid:\n  - \"Redis for caching\"\n",
    )
    .unwrap();

    fs::write(
        repo.join("CLAUDE.md"),
        "# project conventions\n\nWe use in-memory caching.\n",
    )
    .unwrap();
}

/// Count rows in the `symbols` table of the supplied SQLite file.
/// Returns 0 if the file is missing or the table doesn't exist —
/// extraction failure must not crash the test.
fn symbol_count(index_db: &Path) -> i64 {
    let Ok(conn) = rusqlite::Connection::open(index_db) else {
        return 0;
    };
    conn.query_row("SELECT COUNT(*) FROM symbols", [], |r| r.get::<_, i64>(0))
        .unwrap_or(0)
}

#[test]
fn golden_path_init_index_bootstrap_audit_explain() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_repo(repo);

    // -----------------------------------------------------------------
    // Step 2: illuminate index
    // -----------------------------------------------------------------
    let out = run(repo, &["index"]);
    assert!(
        out.status.success(),
        "index must succeed; stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let index_db = repo.join(".illuminate/index.db");
    assert!(
        index_db.exists(),
        "index.db should exist after `illuminate index`",
    );
    let symbols = symbol_count(&index_db);
    assert!(
        symbols >= 1,
        "expected at least one symbol row after indexing src/lib.rs, got {symbols}",
    );

    // -----------------------------------------------------------------
    // Step 3: illuminate bootstrap (+ wiki rebuild to populate graph.db).
    //
    // bootstrap materializes wiki pages from the agent-file and interview
    // sources; `wiki rebuild` then scans the wiki directory and registers
    // each page as an episode in `graph.db`. Both commands together
    // mirror what `illuminate init` would have done in a real
    // first-install flow.
    // -----------------------------------------------------------------
    let out = run(repo, &["bootstrap"]);
    assert!(
        out.status.success(),
        "bootstrap must succeed; stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let bootstrap_stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        bootstrap_stdout.contains("pages written") || bootstrap_stdout.contains("complete"),
        "bootstrap should print a summary line; stdout: {bootstrap_stdout}",
    );

    let out = run(repo, &["wiki", "rebuild"]);
    assert!(
        out.status.success(),
        "wiki rebuild must succeed; stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );

    assert!(
        repo.join(".illuminate/graph.db").exists(),
        "wiki rebuild must create .illuminate/graph.db",
    );
    assert!(
        repo.join(".illuminate/wiki/decisions").exists(),
        "wiki/decisions dir must exist",
    );
    // The interview source should have produced at least one decision page.
    let decisions_dir = repo.join(".illuminate/wiki/decisions");
    let decision_pages: Vec<_> = std::fs::read_dir(&decisions_dir)
        .map(|it| it.filter_map(|e| e.ok()).collect())
        .unwrap_or_default();
    assert!(
        !decision_pages.is_empty(),
        "bootstrap should have written at least one decision page from interview.yaml",
    );

    // -----------------------------------------------------------------
    // Step 4a: audit a Redis plan — expect exit 2 (violation).
    // -----------------------------------------------------------------
    let out = run(repo, &["audit", "add Redis caching to payments"]);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    assert_eq!(
        out.status.code(),
        Some(2),
        "violating audit must exit 2; stdout: {stdout}\nstderr: {stderr}",
    );
    let lower = stdout.to_lowercase();
    assert!(
        lower.contains("redis") || lower.contains("violation") || lower.contains("no_redis"),
        "audit output should mention redis / no_redis / violation; stdout: {stdout}",
    );

    // -----------------------------------------------------------------
    // Step 4b: same audit with --json — verify the structured shape.
    // -----------------------------------------------------------------
    let out = run(repo, &["audit", "add Redis caching to payments", "--json"]);
    assert_eq!(
        out.status.code(),
        Some(2),
        "JSON audit must also exit 2; stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let json: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("audit --json must emit valid JSON: {e}\nstdout: {stdout}"));

    assert_eq!(
        json["status"].as_str(),
        Some("violation"),
        "expected status=violation; json: {json}",
    );
    let violations = json["policy_violations"]
        .as_array()
        .expect("policy_violations must be an array");
    assert!(
        !violations.is_empty(),
        "expected at least one policy violation; json: {json}",
    );

    // The first policy hit must reference our `no_redis` policy at full
    // confidence (rejected_pattern matches are deterministic string hits).
    let first = &violations[0];
    assert_eq!(
        first["policy_name"].as_str(),
        Some("no_redis"),
        "first policy violation should be no_redis; json: {first}",
    );
    if let Some(c) = first["confidence"].as_f64() {
        assert!(
            (c - 1.0).abs() < 0.01,
            "rejected_pattern confidence should be 1.0, got {c}",
        );
    }

    // wiki_url should resolve to the policy's `decision_ref` page.
    if let Some(url) = json["wiki_url"].as_str() {
        assert!(
            url.contains("dec-no-redis"),
            "wiki_url should reference dec-no-redis, got {url}",
        );
    }

    // -----------------------------------------------------------------
    // Step 5: benign audit with file arg — exit 0, impact populated.
    // -----------------------------------------------------------------
    let index_db_str = index_db.to_str().unwrap();
    let out = run(
        repo,
        &[
            "audit",
            "Add a small refactor of process_payment",
            "--index-db",
            index_db_str,
            "src/lib.rs",
            "--json",
        ],
    );
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    assert_eq!(
        out.status.code(),
        Some(0),
        "benign audit must exit 0; stdout: {stdout}\nstderr: {stderr}",
    );

    let json: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!("benign audit --json must emit valid JSON: {e}\nstdout: {stdout}")
    });
    assert_eq!(
        json["status"].as_str(),
        Some("pass"),
        "benign audit should pass; json: {json}",
    );

    // The impact section should reference src/lib.rs as a seed. seed_symbols
    // is qualified-name format ("file::src/lib.rs" or similar), so we accept
    // any entry that mentions the file path.
    let seeds = json["impact"]["seed_symbols"]
        .as_array()
        .expect("impact.seed_symbols must be an array");
    let has_file_seed = seeds
        .iter()
        .filter_map(|v| v.as_str())
        .any(|s| s.contains("src/lib.rs"));
    let defined = json["impact"]["defined_symbols"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mentions_symbol = defined
        .iter()
        .filter_map(|v| v.as_str())
        .any(|s| s.contains("process_payment") || s.contains("src/lib.rs"));
    assert!(
        has_file_seed || mentions_symbol,
        "expected impact section to reference src/lib.rs or process_payment; impact: {}",
        json["impact"],
    );

    // -----------------------------------------------------------------
    // Step 6: explain src/lib.rs — smoke check (must not crash).
    // -----------------------------------------------------------------
    let out = run(repo, &["explain", "src/lib.rs"]);
    assert!(
        out.status.code().is_some(),
        "explain must terminate normally; stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    // We don't pin exit 0 because explain returns the no-anchors warning
    // when the graph has no anchors for the file — that's still useful
    // output, not a crash. But the run must succeed (exit 0): there are
    // no failure paths once the graph opens.
    assert_eq!(
        out.status.code(),
        Some(0),
        "explain on a file with no anchors must still exit 0; stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("src/lib.rs") || stdout.contains("No"),
        "explain should mention the file path or report no anchors; stdout: {stdout}",
    );
}
