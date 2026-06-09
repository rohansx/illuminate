//! Real-binary tempdir smoke for `illuminate trust check` — the v3.0 trust
//! linter that vets off-host (remote) write targets in `illuminate.toml`.
//!
//! No mocks: every fixture is a real `illuminate.toml` written to a tempdir,
//! and every assertion runs the actual built `illuminate` binary as a
//! subprocess. Matches the v3.0 ROADMAP exit criterion: exit 0 on a default /
//! local-path config, non-zero when a remote `TeamRepoTarget::GitRemote`-style
//! target is configured WITHOUT a paired explicit consent flag.

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
        .env("ILLUMINATE_NO_EMBED", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("subprocess must run")
}

/// Write a `.illuminate/illuminate.toml` with the given body and return the
/// repo root. The trust linter is a pure config reader, so no graph.db is
/// needed.
fn write_config(repo: &Path, body: &str) {
    fs::create_dir_all(repo.join(".illuminate")).unwrap();
    fs::write(repo.join(".illuminate/illuminate.toml"), body).unwrap();
}

/// (1) A default config (project block only — no off-host write target) must
/// pass: exit 0 + a clearly-marked clean notice.
#[test]
fn default_config_passes_clean() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    write_config(repo, "[project]\nname = 'trust-default'\n");

    let out = run(repo, &["trust", "check"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "default config must exit 0; stdout: {stdout}\nstderr: {stderr}"
    );
    let lower = stdout.to_lowercase();
    assert!(
        lower.contains("trust")
            && (lower.contains("ok") || lower.contains("pass") || lower.contains("clean")),
        "expected a clean trust notice; stdout: {stdout}"
    );
}

/// A local-path team-repo target is on-host → still a clean pass (exit 0).
#[test]
fn local_path_target_passes_clean() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    write_config(
        repo,
        "[project]\nname = 'trust-local'\n\n[publish]\nkind = 'local_path'\npath = '../team-illuminate'\n",
    );

    let out = run(repo, &["trust", "check"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "local-path target must exit 0; stdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// (2) A remote / off-host target WITHOUT a paired consent flag must FAIL:
/// non-zero exit, and the report must NAME the offending target.
#[test]
fn remote_target_without_consent_fails_and_names_target() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    write_config(
        repo,
        "[project]\nname = 'trust-remote'\n\n[publish]\nkind = 'git_remote'\nurl = 'git@github.com:acme/team-illuminate.git'\n",
    );

    let out = run(repo, &["trust", "check"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !out.status.success(),
        "remote target without consent must exit non-zero; stdout: {stdout}"
    );
    // The report must name the offending target so the dev can find it.
    assert!(
        stdout.contains("git@github.com:acme/team-illuminate.git") || stdout.contains("[publish]"),
        "report must name the offending off-host target; stdout: {stdout}"
    );
    let lower = stdout.to_lowercase();
    assert!(
        lower.contains("consent"),
        "report must explain the missing consent flag; stdout: {stdout}"
    );
}

/// A remote target WITH a paired explicit consent flag is allowed → exit 0.
#[test]
fn remote_target_with_consent_passes() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    write_config(
        repo,
        "[project]\nname = 'trust-remote-ok'\n\n[publish]\nkind = 'git_remote'\nurl = 'git@github.com:acme/team-illuminate.git'\nconsent = true\n",
    );

    let out = run(repo, &["trust", "check"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "remote target WITH consent must exit 0; stdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// (3) `--json` must emit a stable `{ok, findings:[...]}` envelope. On the
/// failing config, `ok` is false and at least one finding names the target.
#[test]
fn json_envelope_is_stable_on_failure() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    write_config(
        repo,
        "[project]\nname = 'trust-json'\n\n[publish]\nkind = 'git_remote'\nurl = 'https://example.com/team.git'\n",
    );

    let out = run(repo, &["trust", "check", "--json"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !out.status.success(),
        "json failing config must still exit non-zero; stdout: {stdout}"
    );
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("invalid json: {e}\n{stdout}"));

    assert_eq!(
        parsed.get("ok").and_then(|v| v.as_bool()),
        Some(false),
        "envelope must carry ok:false on a failing config; {stdout}"
    );
    let findings = parsed
        .get("findings")
        .and_then(|v| v.as_array())
        .expect("envelope must carry a findings array");
    assert!(
        !findings.is_empty(),
        "findings must be non-empty on a failing config; {stdout}"
    );
    // Each finding carries a stable shape and names the target.
    let first = &findings[0];
    for field in ["target", "message"] {
        assert!(
            first.get(field).is_some(),
            "finding must carry `{field}`; {stdout}"
        );
    }
    assert!(
        findings.iter().any(|f| f
            .get("target")
            .and_then(|t| t.as_str())
            .map(|s| s.contains("example.com"))
            .unwrap_or(false)),
        "a finding must name the off-host target; {stdout}"
    );
}

/// `--json` on a clean config emits `{ok:true, findings:[]}` and exits 0.
#[test]
fn json_envelope_is_stable_on_pass() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    write_config(repo, "[project]\nname = 'trust-json-ok'\n");

    let out = run(repo, &["trust", "check", "--json"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "json clean config must exit 0; stdout: {stdout}"
    );
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("invalid json: {e}\n{stdout}"));
    assert_eq!(
        parsed.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "{stdout}"
    );
    assert_eq!(
        parsed
            .get("findings")
            .and_then(|v| v.as_array())
            .map(|a| a.len()),
        Some(0),
        "clean config must have zero findings; {stdout}"
    );
}
