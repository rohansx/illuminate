//! Smoke test for `illuminate diagram` — builds a real code index over a
//! tempdir repo (with a cross-file import) via the real `illuminate index`
//! subprocess, then asserts `illuminate diagram` exits 0 and prints a Mermaid
//! `flowchart` header with the file nodes + an import edge, and that two runs
//! are byte-identical. Also asserts `--out` writes the diagram to a file and a
//! missing index produces the documented "run `illuminate index` first" error.
//! No mocks: the index is built by the real binary over real source files.

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

/// Create a repo with `.illuminate/`, two Rust files (`main.rs` imports the
/// `widget` module), and build the on-disk code index via `illuminate index`.
fn setup_indexed_repo(repo: &Path) {
    fs::create_dir_all(repo.join(".illuminate")).unwrap();
    fs::write(
        repo.join(".illuminate/illuminate.toml"),
        "[project]\nname = 'diagram-smoke'\n",
    )
    .unwrap();
    fs::create_dir_all(repo.join("src")).unwrap();
    fs::write(
        repo.join("src/widget.rs"),
        "pub fn make_widget() -> u32 { 1 }\n",
    )
    .unwrap();
    fs::write(
        repo.join("src/main.rs"),
        "use crate::widget;\npub fn run() -> u32 { widget::make_widget() }\n",
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
fn diagram_prints_mermaid_header_and_exits_zero() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_indexed_repo(repo);

    let out = run(repo, &["diagram"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "diagram must exit 0; stdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stdout.starts_with("flowchart TD"),
        "expected a mermaid header; stdout: {stdout}"
    );
    assert!(
        stdout.contains("[\"src/main.rs\"]"),
        "main.rs file node must render; stdout: {stdout}"
    );
    assert!(
        stdout.contains("[\"src/widget.rs\"]"),
        "widget.rs file node must render; stdout: {stdout}"
    );
    assert!(
        stdout.contains(" --> "),
        "expected at least one `A --> B` import edge; stdout: {stdout}"
    );
}

#[test]
fn diagram_two_runs_are_byte_identical() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_indexed_repo(repo);

    let first = run(repo, &["diagram"]);
    let second = run(repo, &["diagram"]);
    assert!(first.status.success() && second.status.success());
    assert_eq!(
        first.stdout, second.stdout,
        "two diagram runs over the same index must be byte-identical"
    );
}

#[test]
fn diagram_out_writes_to_file() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_indexed_repo(repo);

    let out = run(repo, &["diagram", "--out", "build/arch.mmd"]);
    assert!(
        out.status.success(),
        "diagram --out must exit 0; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let written = repo.join("build/arch.mmd");
    assert!(
        written.exists(),
        "diagram --out must create the file (and parent dir)"
    );
    let body = fs::read_to_string(&written).unwrap();
    assert!(
        body.starts_with("flowchart TD"),
        "the written file must contain a mermaid header; body: {body}"
    );
}

#[test]
fn diagram_without_index_reports_run_index_first() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    fs::create_dir_all(repo.join(".illuminate")).unwrap();
    fs::write(
        repo.join(".illuminate/illuminate.toml"),
        "[project]\nname = 'no-index'\n",
    )
    .unwrap();

    let out = run(repo, &["diagram"]);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !out.status.success(),
        "a missing index must produce a nonzero exit; output: {combined}"
    );
    assert!(
        combined.contains("illuminate index"),
        "error must tell the user to run `illuminate index` first; output: {combined}"
    );
}
