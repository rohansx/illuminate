//! Smoke test for `illuminate audit` with `--index-db` and positional file
//! arguments — verifies the CLI surfaces blast-radius information when an
//! index.db is supplied.

use std::fs;
use std::process::{Command, Stdio};

use illuminate_index::edges::{Edge, EdgeKind};
use illuminate_index::storage::{create_schema, upsert_edges};
use rusqlite::Connection;

fn cargo_bin() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_illuminate"))
}

fn run(repo: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(cargo_bin())
        .args(args)
        .current_dir(repo)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("subprocess must run")
}

fn setup_repo(repo: &std::path::Path) {
    fs::create_dir_all(repo.join(".illuminate/wiki/decisions")).unwrap();
    fs::create_dir_all(repo.join(".illuminate/wiki/patterns")).unwrap();
    fs::create_dir_all(repo.join(".illuminate/wiki/failures")).unwrap();
    fs::create_dir_all(repo.join(".illuminate/wiki/modules")).unwrap();
    fs::write(
        repo.join(".illuminate/illuminate.toml"),
        "[project]\nname = 'impact-smoke'\n",
    )
    .unwrap();
    fs::write(
        repo.join("CLAUDE.md"),
        "## Style\n\nUse 2-space indentation.\n",
    )
    .unwrap();
}

/// Populate a minimal index.db with edges so a seed file has both incoming
/// and outgoing impact.
fn populate_index(index_db: &std::path::Path) {
    let conn = Connection::open(index_db).unwrap();
    create_schema(&conn).unwrap();

    let billing_to_payments = Edge {
        source_qualified: "file::src/billing.rs".to_string(),
        target_qualified: "file::src/payments.rs".to_string(),
        kind: EdgeKind::Imports,
        file_path: "src/billing.rs".to_string(),
        line: 3,
    };
    let api_to_billing = Edge {
        source_qualified: "file::src/api.rs".to_string(),
        target_qualified: "file::src/billing.rs".to_string(),
        kind: EdgeKind::Imports,
        file_path: "src/api.rs".to_string(),
        line: 5,
    };

    upsert_edges(&conn, "src/billing.rs", &[billing_to_payments]).unwrap();
    upsert_edges(&conn, "src/api.rs", &[api_to_billing]).unwrap();
}

#[test]
fn audit_surfaces_blast_radius_when_index_db_supplied() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_repo(repo);

    // bootstrap + rebuild produce graph.db so `audit` can open it.
    let bootstrap = run(repo, &["bootstrap"]);
    assert!(
        bootstrap.status.success(),
        "bootstrap stderr: {}",
        String::from_utf8_lossy(&bootstrap.stderr)
    );
    let rebuild = run(repo, &["wiki", "rebuild"]);
    assert!(
        rebuild.status.success(),
        "wiki rebuild stderr: {}",
        String::from_utf8_lossy(&rebuild.stderr)
    );

    // Manually populate index.db at the standard location.
    let index_db = repo.join(".illuminate/index.db");
    populate_index(&index_db);

    // Run audit with positional file arg pointing at billing.rs.
    let out = run(repo, &["audit", "refactor billing", "src/billing.rs"]);
    assert!(
        out.status.success(),
        "audit must pass; stdout: {} stderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lower = stdout.to_lowercase();
    assert!(
        lower.contains("blast") || lower.contains("radius"),
        "expected blast-radius section in stdout: {stdout}"
    );
    assert!(
        stdout.contains("file::src/payments.rs") || stdout.contains("file::src/api.rs"),
        "expected impacted symbol(s) in stdout: {stdout}"
    );
}

#[test]
fn audit_back_compat_no_files_no_flag() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_repo(repo);

    let bootstrap = run(repo, &["bootstrap"]);
    assert!(bootstrap.status.success());
    let rebuild = run(repo, &["wiki", "rebuild"]);
    assert!(rebuild.status.success());

    // Old positional form: just `audit "plan"` with no files, no --index-db.
    let out = run(repo, &["audit", "refactor billing layer"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "back-compat audit must succeed; stdout: {} stderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.to_lowercase().contains("blast radius"),
        "no impact section without files: {stdout}"
    );
}

#[test]
fn audit_explicit_index_db_flag() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_repo(repo);

    let bootstrap = run(repo, &["bootstrap"]);
    assert!(bootstrap.status.success());
    let rebuild = run(repo, &["wiki", "rebuild"]);
    assert!(rebuild.status.success());

    // Place the index.db under a non-default location; the explicit flag
    // must override the ancestor walk.
    let index_db = repo.join("custom-index.db");
    populate_index(&index_db);

    let out = run(
        repo,
        &[
            "audit",
            "refactor billing",
            "src/billing.rs",
            "--index-db",
            index_db.to_str().unwrap(),
        ],
    );
    assert!(
        out.status.success(),
        "audit with --index-db must pass; stdout: {} stderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lower = stdout.to_lowercase();
    assert!(
        lower.contains("blast") || lower.contains("radius"),
        "expected blast-radius section: {stdout}"
    );
}
