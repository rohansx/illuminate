//! `illuminate install` — wire illuminate into a coding agent in one command.
//!
//! Before this, turning illuminate on for an agent meant several scattered
//! steps: register the MCP server, wire the policy gatekeeper hook, drop the
//! pre-write directive, maybe scaffold a policy. `install` composes them for a
//! chosen agent, idempotently and project-local, so a freshly-cloned repo is
//! one command away from a fully-wired agent:
//!
//! - **MCP server** (`illuminate serve`) into the agent's MCP config so the
//!   context tools (`illuminate_audit`, `_search`, `_enrich`, …) are available.
//! - **Policy gatekeeper hook** (`illuminate policy hook`) into the agent's
//!   PreToolUse hooks (Claude / Codex), so deny/ask/allow runs on every call.
//! - **CLAUDE.md directive** (Claude) so the agent audits before writing.
//! - **`--minimalist`**: scaffold the "write-less" policy template.
//!
//! Reuses the wiring already proven by `policy install` / `policy template` /
//! `init` rather than duplicating it.

use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// Agents whose MCP config is a project-local `mcpServers` JSON file.
fn mcp_config_path(agent: &str, root: &Path) -> Option<PathBuf> {
    match agent {
        // Claude Code reads project-scoped MCP servers from `.mcp.json`.
        "claude" => Some(root.join(".mcp.json")),
        "cursor" => Some(root.join(".cursor").join("mcp.json")),
        "windsurf" => Some(root.join(".windsurf").join("mcp.json")),
        // Codex's MCP config is global TOML (~/.codex/config.toml); we wire its
        // hook here and print guidance for the MCP entry rather than editing a
        // global file from a project command.
        _ => None,
    }
}

pub fn run(agent: &str, minimalist: bool, dir: Option<PathBuf>) -> std::io::Result<()> {
    let agent = agent.trim().to_lowercase();
    if !matches!(agent.as_str(), "claude" | "cursor" | "windsurf" | "codex") {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("unknown agent '{agent}': expected claude | cursor | windsurf | codex"),
        ));
    }
    let root = dir.unwrap_or(std::env::current_dir()?);

    if !root.join(".illuminate").is_dir() {
        eprintln!(
            "note: no .illuminate/ found under {} — run `illuminate init` first so the graph + wiki exist.",
            root.display()
        );
    }

    let mut wired: Vec<String> = Vec::new();
    let mut notes: Vec<String> = Vec::new();

    // 1. MCP server registration.
    match mcp_config_path(&agent, &root) {
        Some(path) => {
            merge_mcp_server(&path)?;
            wired.push(format!("MCP server → {}", rel(&root, &path)));
        }
        None => notes.push(
            "Codex MCP is global — add illuminate to ~/.codex/config.toml under [mcp_servers] (command = \"illuminate\", args = [\"serve\"])".to_string(),
        ),
    }

    // 2. Policy gatekeeper hook (only Claude / Codex have the PreToolUse
    //    permission protocol; Cursor / Windsurf get MCP only).
    if agent == "claude" || agent == "codex" {
        super::policy::wire_policy_hook(&agent, &root)?;
        wired.push("policy gatekeeper PreToolUse hook".to_string());
    } else {
        notes.push(format!(
            "{agent} has no PreToolUse permission hook — the policy gate is Claude/Codex only"
        ));
    }

    // 3. Pre-write audit directive (Claude reads CLAUDE.md).
    if agent == "claude" {
        super::init::write_claude_directive(&root.join("CLAUDE.md"))
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        wired.push("CLAUDE.md pre-write audit directive".to_string());
    }

    // 4. Optional minimalist policy — never clobber an existing policy.
    if minimalist {
        let policy = root.join(".illuminate").join("policy.rhai");
        if policy.exists() {
            notes.push(
                "left existing .illuminate/policy.rhai as-is (replace with `illuminate policy template minimalist --force`)".to_string(),
            );
        } else {
            super::policy::write_template("minimalist", &root, false)?;
            wired.push("minimalist \"write-less\" policy".to_string());
        }
    }

    // Summary.
    println!("installed illuminate for {agent}:");
    for w in &wired {
        println!("  ✓ {w}");
    }
    for n in &notes {
        println!("  · {n}");
    }
    println!();
    println!("next: `illuminate serve` (or restart {agent}) to load the MCP tools.");
    Ok(())
}

/// Merge `mcpServers.illuminate = { command: "illuminate", args: ["serve"] }`
/// into a JSON MCP config, preserving any other servers / keys. Creates the
/// file (and parent dir) when absent. Idempotent.
fn merge_mcp_server(path: &Path) -> std::io::Result<()> {
    let mut cfg: Value = match std::fs::read_to_string(path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_else(|_| json!({})),
        Err(_) => json!({}),
    };
    if !cfg.is_object() {
        cfg = json!({});
    }
    let servers = cfg
        .as_object_mut()
        .expect("object")
        .entry("mcpServers")
        .or_insert_with(|| json!({}));
    if !servers.is_object() {
        *servers = json!({});
    }
    servers.as_object_mut().expect("object").insert(
        "illuminate".to_string(),
        json!({ "command": "illuminate", "args": ["serve"] }),
    );

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(
        path,
        serde_json::to_string_pretty(&cfg).map_err(std::io::Error::other)?,
    )
}

/// Display a path relative to `root` when possible (else the full path).
fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn read_json(path: &Path) -> Value {
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn claude_install_wires_mcp_hook_and_directive() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".illuminate")).unwrap();

        run("claude", false, Some(tmp.path().to_path_buf())).unwrap();

        // MCP server in .mcp.json
        let mcp = read_json(&tmp.path().join(".mcp.json"));
        assert_eq!(mcp["mcpServers"]["illuminate"]["command"], "illuminate");
        assert_eq!(mcp["mcpServers"]["illuminate"]["args"][0], "serve");

        // Policy gatekeeper hook in .claude/settings.json
        let settings = read_json(&tmp.path().join(".claude").join("settings.json"));
        let pre = settings["hooks"]["PreToolUse"].as_array().unwrap();
        assert!(
            pre.iter().any(|e| e["command"]
                .as_str()
                .is_some_and(|c| c.contains("policy hook"))),
            "expected the policy hook to be wired"
        );

        // CLAUDE.md directive
        let claude_md = std::fs::read_to_string(tmp.path().join("CLAUDE.md")).unwrap();
        assert!(claude_md.contains("illuminate: required pre-write step"));
    }

    #[test]
    fn install_is_idempotent_and_preserves_other_servers() {
        let tmp = tempfile::tempdir().unwrap();
        // Pre-existing unrelated MCP server must survive.
        std::fs::write(
            tmp.path().join(".mcp.json"),
            r#"{"mcpServers":{"other":{"command":"x"}}}"#,
        )
        .unwrap();

        run("cursor", false, Some(tmp.path().to_path_buf())).unwrap();
        run("cursor", false, Some(tmp.path().to_path_buf())).unwrap(); // second run: stable

        // cursor writes .cursor/mcp.json; the root .mcp.json is untouched here.
        let cursor = read_json(&tmp.path().join(".cursor").join("mcp.json"));
        assert_eq!(cursor["mcpServers"]["illuminate"]["command"], "illuminate");

        // And a merge into a file with an existing server keeps it.
        run("claude", false, Some(tmp.path().to_path_buf())).unwrap();
        let mcp = read_json(&tmp.path().join(".mcp.json"));
        assert_eq!(mcp["mcpServers"]["other"]["command"], "x");
        assert_eq!(mcp["mcpServers"]["illuminate"]["command"], "illuminate");
    }

    #[test]
    fn minimalist_flag_scaffolds_policy() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".illuminate")).unwrap();
        run("claude", true, Some(tmp.path().to_path_buf())).unwrap();
        let policy =
            std::fs::read_to_string(tmp.path().join(".illuminate").join("policy.rhai")).unwrap();
        assert!(policy.contains("new crate") || policy.contains("new dependency"));
    }

    #[test]
    fn unknown_agent_is_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let err = run("emacs", false, Some(tmp.path().to_path_buf())).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    }
}
