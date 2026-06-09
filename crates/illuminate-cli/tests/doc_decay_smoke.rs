//! Smoke test for `illuminate doc-decay` — builds a real code index over a
//! tempdir repo, then asserts the verb exits 0 with a "no stale doc
//! references" message when every referenced symbol exists, and exits nonzero
//! with a clearly-marked report (file + line + missing symbol) when a doc
//! references a deleted symbol. No mocks: the index is built by the real
//! `illuminate index` subprocess over real source files on disk.

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

fn cargo_bin() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_illuminate"))
}

fn run(repo: &Path, args: &[&str]) -> std::process::Output {
    Command::new(cargo_bin())
        .args(args)
        .current_dir(repo)
        // Keep the test independent of host ONNX models.
        .env("ILLUMINATE_MODELS_DIR", "/nonexistent/illuminate/models")
        .env("HOME", repo)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("subprocess must run")
}

/// Create a repo with `.illuminate/`, a source file defining `make_widget` and
/// `paint_widget`, and build the on-disk code index via `illuminate index`.
fn setup_indexed_repo(repo: &Path) {
    fs::create_dir_all(repo.join(".illuminate")).unwrap();
    fs::write(
        repo.join(".illuminate/illuminate.toml"),
        "[project]\nname = 'doc-decay-smoke'\n",
    )
    .unwrap();
    fs::create_dir_all(repo.join("src")).unwrap();
    fs::write(
        repo.join("src/widget.rs"),
        "pub fn make_widget() -> u32 { 1 }\npub fn paint_widget(x: u32) -> u32 { x }\n",
    )
    .unwrap();

    let out = run(repo, &["index"]);
    assert!(
        out.status.success(),
        "`illuminate index` must succeed; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        repo.join(".illuminate/index.db").exists(),
        "index.db must be written"
    );
}

#[test]
fn clean_repo_reports_no_stale_references_and_exits_zero() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_indexed_repo(repo);

    fs::create_dir_all(repo.join("docs")).unwrap();
    fs::write(
        repo.join("docs/widget.md"),
        "# Widget\n\nUse `make_widget` then `paint_widget`.\n",
    )
    .unwrap();

    let out = run(repo, &["doc-decay"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "clean repo must exit 0; stdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stdout.to_lowercase().contains("no stale doc references"),
        "expected the clean message; stdout: {stdout}"
    );
}

#[test]
fn deleted_symbol_reference_is_flagged_with_nonzero_exit() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_indexed_repo(repo);

    fs::create_dir_all(repo.join("docs")).unwrap();
    fs::write(
        repo.join("docs/widget.md"),
        "# Widget\n\nUse `make_widget`.\n\nThe `delete_widget` helper was removed.\n",
    )
    .unwrap();

    let out = run(repo, &["doc-decay"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let combined = format!("{}{}", stdout, String::from_utf8_lossy(&out.stderr));
    assert!(
        !out.status.success(),
        "a stale reference must produce a nonzero exit; stdout: {stdout}"
    );
    assert!(
        combined.contains("delete_widget"),
        "report must name the deleted symbol; output: {combined}"
    );
    assert!(
        combined.contains("docs/widget.md"),
        "report must name the doc file; output: {combined}"
    );
    // The existing symbol must NOT be flagged.
    assert!(
        !combined.contains("make_widget"),
        "make_widget exists and must not be flagged; output: {combined}"
    );
}

#[test]
fn json_output_lists_stale_references() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_indexed_repo(repo);

    fs::create_dir_all(repo.join("docs")).unwrap();
    fs::write(
        repo.join("docs/api.md"),
        "Removed: `delete_widget` and `resize_widget`.\n",
    )
    .unwrap();

    let out = run(repo, &["doc-decay", "--json"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("--json output must be valid JSON");
    let arr = parsed
        .get("stale")
        .and_then(|v| v.as_array())
        .expect("expected a `stale` array key");
    let names: Vec<&str> = arr
        .iter()
        .filter_map(|r| r.get("symbol").and_then(|s| s.as_str()))
        .collect();
    assert!(
        names.contains(&"delete_widget") && names.contains(&"resize_widget"),
        "expected both deleted symbols in JSON: {stdout}"
    );
}
