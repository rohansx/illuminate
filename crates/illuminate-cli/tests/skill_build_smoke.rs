//! Smoke test for `illuminate skill build` — seeds a real `graph.db` with at
//! least one decision episode, runs the verb against it, and asserts the
//! emitted SKILL.md is a well-formed Claude Code skill pack: a YAML
//! front-matter block (delimited by `---` lines) carrying `name` and
//! `description` keys, followed by markdown sections summarizing the team's
//! decisions/patterns/failures and an instruction to query illuminate before
//! acting. A seeded decision title must appear in the body.
//!
//! No mocks: the graph is a real on-disk SQLite database seeded via the
//! library `Graph`, exactly as the sibling `onboard_smoke` does. The empty
//! graph case asserts a well-formed skeleton (front-matter + a "no decisions
//! captured yet" line) and a clean exit. The `--out <path>` path is exercised
//! to prove the SKILL.md can be written to a file deterministically.

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use illuminate::Episode;

fn cargo_bin() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_illuminate"))
}

fn run(repo: &Path, args: &[&str]) -> std::process::Output {
    Command::new(cargo_bin())
        .args(args)
        .current_dir(repo)
        // Force the extraction pipeline off so the test doesn't depend on
        // ONNX models existing on the host.
        .env("ILLUMINATE_MODELS_DIR", "/nonexistent/illuminate/models")
        .env("HOME", repo)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("subprocess must run")
}

/// Initialize `.illuminate/` + `illuminate.toml`; returns the repo root.
fn init_repo(repo: &Path) {
    fs::create_dir_all(repo.join(".illuminate")).unwrap();
    fs::write(
        repo.join(".illuminate/illuminate.toml"),
        "[project]\nname = 'skill-smoke'\n",
    )
    .unwrap();
}

/// Seed one decision episode directly via the library `Graph` (mirrors
/// `onboard_smoke`), creating `.illuminate/graph.db`.
fn seed_decision(repo: &Path) {
    let db = repo.join(".illuminate/graph.db");
    let graph = illuminate::Graph::open_or_create(&db).expect("open graph");
    let episode = Episode::builder(
        "[dec-use-postgres] Use Postgres for the billing service\n\nChose Postgres over MongoDB for the billing service after a vendor review.",
    )
    .source("wiki:dec/use-postgres")
    .build();
    graph.add_episode(episode).expect("add decision episode");
}

/// Split a SKILL.md document into (front_matter, body) using the leading
/// `---` fenced YAML block. Panics if the front-matter delimiters are absent —
/// the assertion the smoke test relies on.
fn split_front_matter(doc: &str) -> (&str, &str) {
    let rest = doc
        .strip_prefix("---\n")
        .unwrap_or_else(|| panic!("SKILL.md must open with a `---` front-matter delimiter:\n{doc}"));
    let end = rest
        .find("\n---")
        .unwrap_or_else(|| panic!("SKILL.md front-matter must be closed by a `---` line:\n{doc}"));
    let front = &rest[..end];
    let body = rest[end + 4..].trim_start();
    (front, body)
}

#[test]
fn skill_build_emits_front_matter_and_seeded_decision() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);
    seed_decision(repo);

    let out = run(repo, &["skill", "build"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "skill build must exit 0 on a populated graph; stdout: {stdout}\nstderr: {stderr}"
    );

    // YAML front-matter delimiters + the two required keys.
    let (front, body) = split_front_matter(&stdout);
    assert!(
        front.lines().any(|l| l.trim_start().starts_with("name:")),
        "front-matter must carry a `name:` key:\n{front}"
    );
    assert!(
        front
            .lines()
            .any(|l| l.trim_start().starts_with("description:")),
        "front-matter must carry a `description:` key:\n{front}"
    );

    // The seeded decision title must appear in the body.
    assert!(
        body.contains("Use Postgres for the billing service"),
        "expected the seeded decision title in the SKILL.md body:\n{body}"
    );

    // The skill must instruct the agent to query illuminate before acting.
    let lower = stdout.to_lowercase();
    assert!(
        lower.contains("illuminate"),
        "SKILL.md must instruct the agent to query illuminate:\n{stdout}"
    );
}

#[test]
fn skill_build_empty_graph_emits_skeleton_and_exits_zero() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);
    // Create an empty graph.db with no episodes.
    let db = repo.join(".illuminate/graph.db");
    illuminate::Graph::open_or_create(&db).expect("open empty graph");

    let out = run(repo, &["skill", "build"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "empty-graph skill build must exit 0; stdout: {stdout}\nstderr: {stderr}"
    );

    // Still a well-formed skeleton: front-matter with name/description.
    let (front, _body) = split_front_matter(&stdout);
    assert!(
        front.lines().any(|l| l.trim_start().starts_with("name:")),
        "skeleton front-matter must carry a `name:` key:\n{front}"
    );
    assert!(
        front
            .lines()
            .any(|l| l.trim_start().starts_with("description:")),
        "skeleton front-matter must carry a `description:` key:\n{front}"
    );
    assert!(
        stdout.to_lowercase().contains("no decisions captured yet"),
        "empty-graph SKILL.md must carry a `no decisions captured yet` line:\n{stdout}"
    );
}

#[test]
fn skill_build_out_writes_deterministic_file() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);
    seed_decision(repo);

    let out_path = repo.join("nested/dir/SKILL.md");
    let out = run(
        repo,
        &["skill", "build", "--out", out_path.to_str().unwrap()],
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "skill build --out must exit 0; stderr: {stderr}"
    );

    let first = fs::read_to_string(&out_path).expect("SKILL.md must be written to --out path");
    assert!(
        first.starts_with("---\n"),
        "written SKILL.md must open with the front-matter delimiter:\n{first}"
    );
    assert!(
        first.contains("Use Postgres for the billing service"),
        "written SKILL.md must carry the seeded decision title:\n{first}"
    );

    // Determinism: a second run over the same graph yields byte-identical output.
    let out2 = run(
        repo,
        &["skill", "build", "--out", out_path.to_str().unwrap()],
    );
    assert!(out2.status.success(), "second skill build --out must exit 0");
    let second = fs::read_to_string(&out_path).expect("re-read SKILL.md");
    assert_eq!(
        first, second,
        "skill build output must be deterministic for a fixed graph state"
    );
}
