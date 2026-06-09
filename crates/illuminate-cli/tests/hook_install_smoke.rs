//! Smoke test for `illuminate hook install --agent <cursor|codex|claude>`.
//!
//! The installer writes a *local* agent config file (no network) into a
//! caller-supplied directory so the host agent invokes `illuminate audit-hook`
//! before/after edits, mirroring the existing Claude `PreToolUse` wiring. It
//! must be idempotent (a second run leaves the file unchanged) and refuse
//! unknown agents with a clear, non-zero error.
//!
//! No mocks: every assertion runs the real `illuminate` binary against a
//! `tempfile::tempdir()` config directory.

use std::fs;
use std::process::{Command, Output};

fn cargo_bin() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_illuminate"))
}

fn run_install(agent: &str, dir: &std::path::Path) -> Output {
    Command::new(cargo_bin())
        .arg("hook")
        .arg("install")
        .arg("--agent")
        .arg(agent)
        .arg("--dir")
        .arg(dir)
        .output()
        .expect("spawn must succeed")
}

#[test]
fn cursor_install_writes_hooks_json_with_illuminate() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run_install("cursor", tmp.path());
    assert_eq!(
        out.status.code(),
        Some(0),
        "cursor install must exit 0; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let cfg = tmp.path().join(".cursor/hooks.json");
    assert!(cfg.is_file(), "cursor config must exist at .cursor/hooks.json");

    let body = fs::read_to_string(&cfg).unwrap();
    let json: serde_json::Value = serde_json::from_str(&body).expect("valid json");
    assert_eq!(json["version"], 1, "cursor hooks schema version");

    // The illuminate audit-hook invocation must appear under an edit-time event.
    let after_edit = json["hooks"]["afterFileEdit"]
        .as_array()
        .expect("afterFileEdit array present");
    let has_illuminate = after_edit.iter().any(|h| {
        h["command"]
            .as_str()
            .is_some_and(|c| c.contains("illuminate") && c.contains("audit-hook"))
    });
    assert!(
        has_illuminate,
        "cursor afterFileEdit must invoke `illuminate audit-hook`: {body}"
    );
}

#[test]
fn cursor_install_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = tmp.path().join(".cursor/hooks.json");

    let first = run_install("cursor", tmp.path());
    assert_eq!(first.status.code(), Some(0));
    let after_first = fs::read_to_string(&cfg).unwrap();

    let second = run_install("cursor", tmp.path());
    assert_eq!(second.status.code(), Some(0));
    let after_second = fs::read_to_string(&cfg).unwrap();

    assert_eq!(
        after_first, after_second,
        "second cursor install must leave the file byte-identical"
    );

    // And there must be exactly ONE illuminate entry (no duplicates).
    let json: serde_json::Value = serde_json::from_str(&after_second).unwrap();
    let count = json["hooks"]["afterFileEdit"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|h| {
            h["command"]
                .as_str()
                .is_some_and(|c| c.contains("illuminate"))
        })
        .count();
    assert_eq!(count, 1, "exactly one illuminate hook entry after re-run");
}

#[test]
fn codex_install_writes_config_with_illuminate() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run_install("codex", tmp.path());
    assert_eq!(
        out.status.code(),
        Some(0),
        "codex install must exit 0; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let cfg = tmp.path().join(".codex/hooks.json");
    assert!(cfg.is_file(), "codex config must exist at .codex/hooks.json");

    let body = fs::read_to_string(&cfg).unwrap();
    let json: serde_json::Value = serde_json::from_str(&body).expect("valid json");

    let pre = json["hooks"]["PreToolUse"]
        .as_array()
        .expect("PreToolUse array present");
    let has_illuminate = pre.iter().any(|group| {
        group["hooks"]
            .as_array()
            .map(|inner| {
                inner.iter().any(|h| {
                    h["command"]
                        .as_str()
                        .is_some_and(|c| c.contains("illuminate") && c.contains("audit-hook"))
                })
            })
            .unwrap_or(false)
    });
    assert!(
        has_illuminate,
        "codex PreToolUse must invoke `illuminate audit-hook`: {body}"
    );
}

#[test]
fn codex_install_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = tmp.path().join(".codex/hooks.json");

    let first = run_install("codex", tmp.path());
    assert_eq!(first.status.code(), Some(0));
    let after_first = fs::read_to_string(&cfg).unwrap();

    let second = run_install("codex", tmp.path());
    assert_eq!(second.status.code(), Some(0));
    let after_second = fs::read_to_string(&cfg).unwrap();

    assert_eq!(
        after_first, after_second,
        "second codex install must leave the file byte-identical"
    );
}

#[test]
fn claude_install_writes_settings_pretooluse() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run_install("claude", tmp.path());
    assert_eq!(
        out.status.code(),
        Some(0),
        "claude install must exit 0; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let cfg = tmp.path().join(".claude/settings.json");
    assert!(
        cfg.is_file(),
        "claude config must exist at .claude/settings.json"
    );

    let body = fs::read_to_string(&cfg).unwrap();
    let json: serde_json::Value = serde_json::from_str(&body).expect("valid json");
    let pre = json["hooks"]["PreToolUse"]
        .as_array()
        .expect("PreToolUse array present");
    let has_illuminate = pre.iter().any(|h| {
        h["command"]
            .as_str()
            .is_some_and(|c| c.contains("illuminate") && c.contains("audit-hook"))
    });
    assert!(
        has_illuminate,
        "claude PreToolUse must invoke `illuminate audit-hook`: {body}"
    );

    // Idempotent: re-run leaves it byte-identical.
    let second = run_install("claude", tmp.path());
    assert_eq!(second.status.code(), Some(0));
    let after_second = fs::read_to_string(&cfg).unwrap();
    assert_eq!(body, after_second, "second claude install unchanged");
}

#[test]
fn unknown_agent_is_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run_install("windsurf-typo", tmp.path());
    assert_ne!(
        out.status.code(),
        Some(0),
        "unknown agent must produce a non-zero exit"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.to_lowercase().contains("unknown agent")
            || stderr.to_lowercase().contains("unsupported"),
        "error must clearly name the unknown agent: {stderr}"
    );
    // Nothing should have been written for the bogus agent.
    assert!(
        !tmp.path().join(".cursor/hooks.json").exists(),
        "no cursor config should be written for an unknown agent"
    );
}
