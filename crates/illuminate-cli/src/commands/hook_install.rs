//! `illuminate hook install --agent <cursor|codex|claude>` — write a *local*
//! agent config file that invokes `illuminate audit-hook` on edits, mirroring
//! the existing Claude `PreToolUse` wiring in [`super::init`].
//!
//! This is a purely local operation: it only writes a config file under the
//! caller-supplied directory (defaulting to the current directory). It performs
//! no network access. It is idempotent — re-running it never duplicates an
//! illuminate entry and leaves an already-configured file byte-identical. An
//! unknown agent is refused with a clear, non-zero error.
//!
//! Config targets, all rooted at `<dir>`:
//!
//! | agent    | file                    | event           |
//! |----------|-------------------------|-----------------|
//! | `cursor` | `.cursor/hooks.json`    | `afterFileEdit` |
//! | `codex`  | `.codex/hooks.json`     | `PreToolUse`    |
//! | `claude` | `.claude/settings.json` | `PreToolUse`    |
//!
//! All three invoke the same command, [`HOOK_COMMAND`], so a project that
//! switches host agents keeps the identical audit gate.

use std::path::{Path, PathBuf};

use illuminate::IlluminateError;

/// The command every installed hook runs. Mirrors the literal wired by
/// `illuminate init --hooks` so all agents share one audit entry point.
const HOOK_COMMAND: &str = "illuminate audit-hook";

/// Edit-time tool matcher reused by the Codex/Claude `PreToolUse` event — the
/// same `Write|Edit|MultiEdit` set the audit hook itself gates on.
const EDIT_MATCHER: &str = "Write|Edit|MultiEdit";

/// Dispatch entry for `illuminate hook install`.
///
/// `agent` is matched case-insensitively. `dir` is the config root; `None`
/// means the current working directory.
pub fn run(agent: &str, dir: Option<PathBuf>) -> illuminate::Result<()> {
    let root = match dir {
        Some(d) => d,
        None => std::env::current_dir().map_err(IlluminateError::Io)?,
    };

    let written = match agent.trim().to_lowercase().as_str() {
        "cursor" => install_cursor(&root)?,
        "codex" => install_codex(&root)?,
        "claude" => install_claude(&root)?,
        other => {
            return Err(IlluminateError::Extraction(format!(
                "unknown agent '{other}': supported agents are cursor, codex, claude"
            )));
        }
    };

    println!(
        "installed illuminate audit hook for {agent} → {}",
        written.display()
    );
    Ok(())
}

/// Read + parse a JSON config file, returning an empty object when the file is
/// absent or unparseable (the latter mirrors `init.rs`'s tolerant behaviour:
/// a corrupt file is treated as empty rather than aborting the install).
fn read_json(path: &Path) -> serde_json::Value {
    match std::fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({})),
        Err(_) => serde_json::json!({}),
    }
}

/// Serialize `value` pretty-printed and write it to `path`, creating parent
/// directories as needed.
fn write_json(path: &Path, value: &serde_json::Value) -> illuminate::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(IlluminateError::Io)?;
    }
    let json_str = serde_json::to_string_pretty(value)
        .map_err(|e| IlluminateError::Extraction(e.to_string()))?;
    std::fs::write(path, json_str).map_err(IlluminateError::Io)?;
    Ok(())
}

/// True if any element of `arr` is an object whose `command` string contains
/// "illuminate" — the idempotency probe shared by all flat-array installers.
fn array_has_illuminate(arr: &[serde_json::Value]) -> bool {
    arr.iter().any(|h| {
        h.get("command")
            .and_then(|c| c.as_str())
            .is_some_and(|c| c.contains("illuminate"))
    })
}

/// Cursor: `<dir>/.cursor/hooks.json` with an `afterFileEdit` entry.
fn install_cursor(root: &Path) -> illuminate::Result<PathBuf> {
    let path = root.join(".cursor").join("hooks.json");
    let mut config = read_json(&path);

    let obj = config
        .as_object_mut()
        .expect("read_json always yields an object");
    obj.entry("version").or_insert_with(|| serde_json::json!(1));
    let hooks = obj.entry("hooks").or_insert_with(|| serde_json::json!({}));
    let after_edit = hooks
        .as_object_mut()
        .expect("hooks is an object")
        .entry("afterFileEdit")
        .or_insert_with(|| serde_json::json!([]));

    if let Some(arr) = after_edit.as_array_mut()
        && !array_has_illuminate(arr)
    {
        arr.push(serde_json::json!({ "command": HOOK_COMMAND }));
    }

    write_json(&path, &config)?;
    Ok(path)
}

/// Claude: `<dir>/.claude/settings.json` with a `PreToolUse` entry — identical
/// in shape to what `illuminate init --hooks` writes.
fn install_claude(root: &Path) -> illuminate::Result<PathBuf> {
    let path = root.join(".claude").join("settings.json");
    let mut config = read_json(&path);

    let hooks = config
        .as_object_mut()
        .expect("read_json always yields an object")
        .entry("hooks")
        .or_insert_with(|| serde_json::json!({}));
    let pre = hooks
        .as_object_mut()
        .expect("hooks is an object")
        .entry("PreToolUse")
        .or_insert_with(|| serde_json::json!([]));

    if let Some(arr) = pre.as_array_mut()
        && !array_has_illuminate(arr)
    {
        arr.push(serde_json::json!({
            "matcher": EDIT_MATCHER,
            "command": HOOK_COMMAND,
        }));
    }

    write_json(&path, &config)?;
    Ok(path)
}

/// Codex: `<dir>/.codex/hooks.json` with a `PreToolUse` matcher group whose
/// inner `hooks` array runs the command. Codex nests one level deeper than
/// Claude: `PreToolUse -> [ { matcher, hooks: [ { type, command } ] } ]`.
fn install_codex(root: &Path) -> illuminate::Result<PathBuf> {
    let path = root.join(".codex").join("hooks.json");
    let mut config = read_json(&path);

    let hooks = config
        .as_object_mut()
        .expect("read_json always yields an object")
        .entry("hooks")
        .or_insert_with(|| serde_json::json!({}));
    let pre = hooks
        .as_object_mut()
        .expect("hooks is an object")
        .entry("PreToolUse")
        .or_insert_with(|| serde_json::json!([]));

    if let Some(arr) = pre.as_array_mut()
        && !codex_has_illuminate(arr)
    {
        arr.push(serde_json::json!({
            "matcher": EDIT_MATCHER,
            "hooks": [
                {
                    "type": "command",
                    "command": HOOK_COMMAND,
                }
            ],
        }));
    }

    write_json(&path, &config)?;
    Ok(path)
}

/// Codex idempotency probe: scan each matcher group's nested `hooks` array for
/// an illuminate command.
fn codex_has_illuminate(groups: &[serde_json::Value]) -> bool {
    groups.iter().any(|group| {
        group
            .get("hooks")
            .and_then(|h| h.as_array())
            .map(|inner| array_has_illuminate(inner))
            .unwrap_or(false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn array_has_illuminate_detects_command() {
        let arr = vec![
            serde_json::json!({ "command": "prettier --write" }),
            serde_json::json!({ "command": "illuminate audit-hook" }),
        ];
        assert!(array_has_illuminate(&arr));
    }

    #[test]
    fn array_has_illuminate_false_when_absent() {
        let arr = vec![serde_json::json!({ "command": "eslint" })];
        assert!(!array_has_illuminate(&arr));
    }

    #[test]
    fn codex_has_illuminate_scans_nested_hooks() {
        let groups = vec![serde_json::json!({
            "matcher": "Write|Edit",
            "hooks": [ { "type": "command", "command": "illuminate audit-hook" } ],
        })];
        assert!(codex_has_illuminate(&groups));

        let other = vec![serde_json::json!({
            "matcher": "Bash",
            "hooks": [ { "type": "command", "command": "shellcheck" } ],
        })];
        assert!(!codex_has_illuminate(&other));
    }

    #[test]
    fn cursor_install_preserves_existing_unrelated_hook() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(".cursor").join("hooks.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            r#"{"version":1,"hooks":{"afterFileEdit":[{"command":"prettier"}]}}"#,
        )
        .unwrap();

        install_cursor(tmp.path()).unwrap();

        let body = std::fs::read_to_string(&path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        let arr = json["hooks"]["afterFileEdit"].as_array().unwrap();
        // existing prettier hook preserved + illuminate appended
        assert_eq!(arr.len(), 2);
        assert!(array_has_illuminate(arr));
        assert!(
            arr.iter()
                .any(|h| h["command"].as_str() == Some("prettier")),
            "existing hook must survive"
        );
    }

    #[test]
    fn unknown_agent_returns_clear_error() {
        let tmp = tempfile::tempdir().unwrap();
        let err = run("nope", Some(tmp.path().to_path_buf())).unwrap_err();
        let msg = err.to_string().to_lowercase();
        assert!(msg.contains("unknown agent"), "got: {msg}");
        // and nothing was written
        assert!(!tmp.path().join(".cursor/hooks.json").exists());
    }
}
