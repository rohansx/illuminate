//! End-to-end integration test.
//!
//! Spawns the `illuminate` binary against a tempdir repo and asserts the
//! bootstrap → wiki rebuild → audit flow produces the expected exit codes.

use std::fs;
use std::process::{Command, Stdio};

fn cargo_bin() -> std::path::PathBuf {
    // Built by `cargo test`; CARGO_BIN_EXE_<name> is set automatically.
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
        "[project]\nname = 'e2e'\n\n[policies.no_redis]\nrule = \"rejected_pattern\"\npattern = \"Redis\"\nreason = \"deployment target disallows stateful sidecars\"\nseverity = \"error\"\n",
    )
    .unwrap();
    fs::write(
        repo.join("CLAUDE.md"),
        "## Caching\n\nWe use Memcached. Never use Redis. Do not introduce stateful sidecars.\n\n## Style\n\nUse 2-space indentation.\n",
    )
    .unwrap();
}

#[test]
fn bootstrap_then_rebuild_then_audit_violation_and_pass() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_repo(repo);

    // 1. bootstrap
    let bootstrap = run(repo, &["bootstrap"]);
    assert!(
        bootstrap.status.success(),
        "bootstrap stderr: {}",
        String::from_utf8_lossy(&bootstrap.stderr)
    );
    let stdout = String::from_utf8_lossy(&bootstrap.stdout);
    assert!(
        stdout.contains("pages written"),
        "bootstrap stdout: {stdout}"
    );

    // 2. wiki rebuild
    let rebuild = run(repo, &["wiki", "rebuild"]);
    assert!(
        rebuild.status.success(),
        "wiki rebuild stderr: {}",
        String::from_utf8_lossy(&rebuild.stderr)
    );

    // 3. audit a Redis plan — expect violation (exit 2)
    let bad = run(repo, &["audit", "add Redis caching to billing service"]);
    let bad_stdout = String::from_utf8_lossy(&bad.stdout);
    let bad_stderr = String::from_utf8_lossy(&bad.stderr);
    assert_eq!(
        bad.status.code(),
        Some(2),
        "expected exit 2 for violation; got {:?}\nstdout: {bad_stdout}\nstderr: {bad_stderr}",
        bad.status.code()
    );
    assert!(
        bad_stdout.contains("Redis") || bad_stdout.contains("Violations"),
        "expected violation evidence in stdout: {bad_stdout}"
    );

    // 4. audit a Memcached plan — expect pass (exit 0)
    let good = run(repo, &["audit", "add Memcached caching to billing service"]);
    assert_eq!(
        good.status.code(),
        Some(0),
        "expected exit 0 for pass; got {:?}\nstdout: {}\nstderr: {}",
        good.status.code(),
        String::from_utf8_lossy(&good.stdout),
        String::from_utf8_lossy(&good.stderr)
    );
}
