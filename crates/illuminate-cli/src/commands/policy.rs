//! `illuminate policy` — the gatekeeper. Evaluates a tool call against the
//! repo's Rhai policy (`.illuminate/policy.rhai`, or the bundled conservative
//! default) in deny → ask → allow order, records every decision to a local
//! ledger, and — as a Claude Code PreToolUse hook — returns the allow/deny/ask
//! verdict in the host-agent protocol so illuminate sits in the live tool-call
//! path, not just advising after the fact.

use clap::Subcommand;
use illuminate_policy::{Decision, Engine, EvalRequest, RuleSet, default_ruleset};
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub enum PolicyCmd {
    /// Evaluate a single tool call and print the decision (allow/deny/ask)
    Check {
        /// Tool name (e.g. Bash, Read, WebFetch)
        tool: String,
        /// For Bash: the command
        #[arg(long)]
        cmd: Option<String>,
        /// For Read/Edit/Write: the file path
        #[arg(long)]
        path: Option<String>,
        /// For WebFetch: the URL
        #[arg(long)]
        url: Option<String>,
        /// Emit JSON instead of a human line
        #[arg(long)]
        json: bool,
    },
    /// Show the full rule-by-rule trace for a tool call (why a decision was reached)
    Trace {
        /// Tool name
        tool: String,
        #[arg(long)]
        cmd: Option<String>,
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        url: Option<String>,
    },
    /// PreToolUse hook: read the tool call from stdin, decide, emit the
    /// host-agent permission verdict (allow/deny/ask) on stdout
    Hook,
    /// Show the most recent policy decisions from the ledger
    Recent {
        /// Max rows
        #[arg(long, default_value_t = 20)]
        limit: usize,
        /// Emit JSON
        #[arg(long)]
        json: bool,
    },
    /// Wire the policy PreToolUse hook into a host agent's config (one-command setup)
    Install {
        /// Host agent: claude or codex (both have a PreToolUse permission hook)
        #[arg(long, default_value = "claude")]
        agent: String,
        /// Config root (default: current directory)
        #[arg(long)]
        dir: Option<PathBuf>,
    },
    /// Scaffold `.illuminate/policy.rhai` from a named template
    Template {
        /// Template: `minimalist` (default rules + "write less" dependency
        /// restraint) or `default` (the conservative baseline)
        name: String,
        /// Repo root (default: current directory)
        #[arg(long)]
        dir: Option<PathBuf>,
        /// Overwrite an existing policy.rhai
        #[arg(long)]
        force: bool,
    },
}

pub fn run(cmd: PolicyCmd) -> std::io::Result<()> {
    match cmd {
        PolicyCmd::Check {
            tool,
            cmd,
            path,
            url,
            json,
        } => cmd_check(tool, cmd, path, url, json),
        PolicyCmd::Trace {
            tool,
            cmd,
            path,
            url,
        } => cmd_trace(tool, cmd, path, url),
        PolicyCmd::Hook => cmd_hook(),
        PolicyCmd::Recent { limit, json } => cmd_recent(limit, json),
        PolicyCmd::Install { agent, dir } => cmd_install(&agent, dir),
        PolicyCmd::Template { name, dir, force } => cmd_template(&name, dir, force),
    }
}

/// Scaffold `.illuminate/policy.rhai` from a bundled template. `minimalist`
/// stacks the "write less" dependency-restraint layer on the conservative
/// default; `default` writes just the baseline. Refuses to clobber an existing
/// policy without `--force`, and validates that what it wrote parses.
fn cmd_template(name: &str, dir: Option<PathBuf>, force: bool) -> std::io::Result<()> {
    let root = dir.unwrap_or(std::env::current_dir()?);
    write_template(name, &root, force)
}

/// Write a named policy template to `<root>/.illuminate/policy.rhai`. Shared by
/// `policy template` and `illuminate install --minimalist`. Validates the
/// source parses before writing; refuses to clobber without `force`.
pub(crate) fn write_template(name: &str, root: &Path, force: bool) -> std::io::Result<()> {
    let source = match name.trim().to_lowercase().as_str() {
        "minimalist" | "minimal" => illuminate_policy::minimalist_policy_source(),
        "default" => illuminate_policy::DEFAULT_POLICY.to_string(),
        other => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("unknown template '{other}': expected `minimalist` or `default`"),
            ));
        }
    };

    let target = root.join(".illuminate").join("policy.rhai");
    if target.exists() && !force {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!(
                "{} already exists — re-run with --force to overwrite",
                target.display()
            ),
        ));
    }

    // Validate before writing so a template can never leave a broken policy.
    let rules = RuleSet::parse(&Engine::new(), &source, "policy.rhai")
        .map_err(|e| std::io::Error::other(format!("template failed to parse: {e}")))?;

    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&target, &source)?;
    println!(
        "wrote {name} policy ({} rules) → {}",
        rules.len(),
        target.display()
    );
    if name.trim().eq_ignore_ascii_case("minimalist") {
        println!("the gate now asks before an agent adds a new dependency.");
    }
    println!("wire it into your agent with: illuminate policy install --agent claude");
    Ok(())
}

/// The command the installed hook runs (the PreToolUse entry point).
const POLICY_HOOK_CMD: &str = "illuminate policy hook";
/// Tool-name matcher for the policy hook — the tools the policy reasons about.
const POLICY_MATCHER: &str = "Bash|Read|Edit|Write|MultiEdit|WebFetch";

/// Wire `illuminate policy hook` into a host agent's PreToolUse hooks. Idempotent
/// — re-running never duplicates the entry. Claude uses a flat `{matcher, command}`;
/// Codex nests `{matcher, hooks: [{type, command}]}`.
fn cmd_install(agent: &str, dir: Option<PathBuf>) -> std::io::Result<()> {
    let root = dir.unwrap_or(std::env::current_dir()?);
    wire_policy_hook(agent, &root)
}

/// Wire `illuminate policy hook` into a host agent's PreToolUse hooks under
/// `root`. Shared by `policy install` and `illuminate install`. Idempotent.
pub(crate) fn wire_policy_hook(agent: &str, root: &Path) -> std::io::Result<()> {
    let (path, nested) = match agent.trim().to_lowercase().as_str() {
        "claude" => (root.join(".claude").join("settings.json"), false),
        "codex" => (root.join(".codex").join("hooks.json"), true),
        other => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "unknown agent '{other}': policy hook supports claude or codex (PreToolUse)"
                ),
            ));
        }
    };

    let mut cfg = match std::fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_else(|_| json!({})),
        Err(_) => json!({}),
    };
    let pre = cfg
        .as_object_mut()
        .expect("object")
        .entry("hooks")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .expect("object")
        .entry("PreToolUse")
        .or_insert_with(|| json!([]));
    let arr = pre.as_array_mut().expect("array");

    if !arr.iter().any(entry_runs_policy_hook) {
        if nested {
            arr.push(json!({ "matcher": POLICY_MATCHER, "hooks": [{ "type": "command", "command": POLICY_HOOK_CMD }] }));
        } else {
            arr.push(json!({ "matcher": POLICY_MATCHER, "command": POLICY_HOOK_CMD }));
        }
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&cfg).map_err(std::io::Error::other)?,
    )?;
    println!(
        "wired `{POLICY_HOOK_CMD}` into {agent} PreToolUse → {}",
        path.display()
    );
    println!("the policy gate now runs on every {POLICY_MATCHER} call (allow/deny/ask).");
    Ok(())
}

/// True if a PreToolUse entry already runs the policy hook (flat or Codex-nested).
fn entry_runs_policy_hook(entry: &Value) -> bool {
    let has = |c: Option<&str>| c.is_some_and(|c| c.contains("policy hook"));
    if has(entry.get("command").and_then(|v| v.as_str())) {
        return true;
    }
    entry
        .get("hooks")
        .and_then(|h| h.as_array())
        .is_some_and(|inner| {
            inner
                .iter()
                .any(|h| has(h.get("command").and_then(|v| v.as_str())))
        })
}

fn cmd_recent(limit: usize, as_json: bool) -> std::io::Result<()> {
    let rows = recent_decisions(&repo_root(), limit);
    if as_json {
        println!("{}", Value::Array(rows));
        return Ok(());
    }
    if rows.is_empty() {
        println!("(no policy decisions recorded yet)");
        return Ok(());
    }
    for r in &rows {
        let s = |k: &str| r.get(k).and_then(|v| v.as_str()).unwrap_or("");
        let detail = [s("cmd"), s("path"), s("url")]
            .into_iter()
            .find(|x| !x.is_empty())
            .unwrap_or("");
        println!(
            "{:<6} {:<8} {}  [{}]",
            s("decision").to_uppercase(),
            s("tool"),
            detail,
            s("rule")
        );
    }
    Ok(())
}

/// Find the repo root (nearest ancestor with `.illuminate/`), defaulting to cwd.
fn repo_root() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut cur = Some(cwd.as_path());
    while let Some(d) = cur {
        if d.join(".illuminate").is_dir() {
            return d.to_path_buf();
        }
        cur = d.parent();
    }
    cwd
}

/// The repo's policy ruleset: `.illuminate/policy.rhai` if present, else the
/// bundled conservative default.
fn ruleset_for(engine: &Engine, root: &Path) -> RuleSet {
    let custom = root.join(".illuminate").join("policy.rhai");
    if custom.is_file() {
        match RuleSet::load(engine, &custom) {
            Ok(rs) => return rs,
            Err(e) => eprintln!("illuminate: policy.rhai parse error ({e}); using default policy"),
        }
    }
    default_ruleset(engine)
}

/// Build a policy engine wired with graph-backed Rhai helpers so rules can
/// reason over what illuminate knows about this repo — the fusion of the
/// gatekeeper with the decision graph + git history:
///
/// - `recently_edited(path)` → was `path` touched in git in the last 14 days?
/// - `decisions_referencing(text)` → how many graph episodes mention `text`?
///
/// e.g. `ask if tool == "Edit" && decisions_referencing(path) > 0;` (pause
/// before editing a file the team has recorded decisions about).
fn build_engine(root: &Path) -> Engine {
    let r1 = root.to_path_buf();
    let r2 = root.to_path_buf();
    Engine::with_helpers(move |e| {
        e.register_fn("recently_edited", move |path: &str| {
            recently_edited(&r1, path)
        });
        e.register_fn("decisions_referencing", move |text: &str| {
            decisions_referencing(&r2, text)
        });
    })
}

/// True if `path` was modified in git within the last 14 days. Best-effort:
/// any git failure (not a repo, no history) is a non-match (false).
fn recently_edited(root: &Path, path: &str) -> bool {
    if path.is_empty() {
        return false;
    }
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "log",
            "-1",
            "--since=14 days ago",
            "--format=%H",
            "--",
            path,
        ])
        .output();
    matches!(out, Ok(o) if o.status.success() && !o.stdout.is_empty())
}

/// Count graph episodes that reference `text` (decisions/patterns/failures the
/// team has recorded touching this concept). Best-effort: 0 if the graph can't
/// be opened or the query fails.
fn decisions_referencing(root: &Path, text: &str) -> i64 {
    if text.trim().is_empty() {
        return 0;
    }
    let db = root.join(".illuminate").join("graph.db");
    match illuminate::Graph::open(&db) {
        Ok(graph) => graph
            .search(text, 50)
            .map(|hits| hits.len() as i64)
            .unwrap_or(0),
        Err(_) => 0,
    }
}

fn make_request(
    tool: &str,
    cmd: Option<String>,
    path: Option<String>,
    url: Option<String>,
    root: &Path,
) -> EvalRequest {
    EvalRequest {
        tool: tool.to_string(),
        cmd: cmd.unwrap_or_default(),
        path: path.unwrap_or_default(),
        url: url.unwrap_or_default(),
        cwd: root.display().to_string(),
        home: std::env::var("HOME").unwrap_or_default(),
        session_id: String::new(),
    }
}

fn cmd_check(
    tool: String,
    cmd: Option<String>,
    path: Option<String>,
    url: Option<String>,
    as_json: bool,
) -> std::io::Result<()> {
    let root = repo_root();
    let engine = build_engine(&root);
    let rules = ruleset_for(&engine, &root);
    let req = make_request(&tool, cmd, path, url, &root);
    let out = engine.eval(&rules, &req);
    append_ledger(
        &root,
        &req,
        &out.decision,
        out.rule
            .as_ref()
            .map(|r| (r.file.display().to_string(), r.line)),
        out.rule_text.as_deref(),
    );

    if as_json {
        let v = json!({
            "decision": out.decision.as_str(),
            "rule": out.rule.map(|r| json!({ "file": r.file.display().to_string(), "line": r.line })),
            "rule_text": out.rule_text,
            "message": out.message,
        });
        println!("{v}");
    } else {
        let badge = match out.decision {
            Decision::Deny => "DENY",
            Decision::Ask => "ASK",
            Decision::Allow => "ALLOW",
        };
        match &out.rule {
            Some(r) => println!(
                "{badge}  ({}:{})  {}",
                r.file.display(),
                r.line,
                out.rule_text.as_deref().unwrap_or_default()
            ),
            None => println!("{badge}  (no rule matched → default)"),
        }
        if let Some(m) = &out.message {
            println!("       → {m}");
        }
    }
    Ok(())
}

fn cmd_trace(
    tool: String,
    cmd: Option<String>,
    path: Option<String>,
    url: Option<String>,
) -> std::io::Result<()> {
    let root = repo_root();
    let engine = build_engine(&root);
    let rules = ruleset_for(&engine, &root);
    let req = make_request(&tool, cmd, path, url, &root);
    let trace = engine.trace(&rules, &req);

    println!("decision: {}", trace.outcome.decision.as_str());
    println!("rules (deny → ask → allow order):");
    for r in &trace.rules {
        let mark = if r.decisive {
            "▶"
        } else if r.matched {
            "·"
        } else {
            " "
        };
        println!(
            "  {mark} [{}] {}:{}  {}",
            r.verb.as_str(),
            r.location.file.display(),
            r.location.line,
            r.source_text
        );
    }
    Ok(())
}

/// PreToolUse hook. Reads the Claude Code hook payload from stdin, evaluates the
/// policy, records the decision, and emits the permission verdict on stdout in
/// the host-agent's `hookSpecificOutput.permissionDecision` protocol (allow /
/// deny / ask). Honored-decisions-only: an `ask` lets the agent prompt natively.
fn cmd_hook() -> std::io::Result<()> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    let payload: Value = serde_json::from_str(&input).unwrap_or_default();

    let tool = payload
        .get("tool_name")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let tool_input = payload.get("tool_input").cloned().unwrap_or_default();
    let root = repo_root();

    let mut req = EvalRequest::from_tool_call(tool, &tool_input, &root.display().to_string());
    if let Some(sid) = payload.get("session_id").and_then(|v| v.as_str()) {
        req.session_id = sid.to_string();
    }

    let engine = build_engine(&root);
    let rules = ruleset_for(&engine, &root);
    let out = engine.eval(&rules, &req);
    append_ledger(
        &root,
        &req,
        &out.decision,
        out.rule
            .as_ref()
            .map(|r| (r.file.display().to_string(), r.line)),
        out.rule_text.as_deref(),
    );

    // Prefer the rule's human `=> "message"` when present (e.g. the minimalist
    // ladder), falling back to the raw rule source, then a generic default.
    let reason = match (
        &out.decision,
        out.message.as_deref(),
        out.rule_text.as_deref(),
    ) {
        (Decision::Allow, _, _) => "illuminate policy: allowed".to_string(),
        (_, Some(m), _) => format!("illuminate policy: {m}"),
        (_, None, Some(t)) => format!("illuminate policy: {t}"),
        (_, None, None) => "illuminate policy: default".to_string(),
    };
    let verdict = json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": out.decision.as_str(),
            "permissionDecisionReason": reason,
        }
    });
    let mut stdout = std::io::stdout();
    writeln!(stdout, "{verdict}")?;
    Ok(())
}

/// Append one decision to `.illuminate/policy/ledger.jsonl` (best-effort — a
/// ledger write must never block a tool call).
fn append_ledger(
    root: &Path,
    req: &EvalRequest,
    decision: &Decision,
    rule: Option<(String, u32)>,
    rule_text: Option<&str>,
) {
    let dir = root.join(".illuminate").join("policy");
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let entry = json!({
        "ts": chrono::Utc::now().to_rfc3339(),
        "tool": req.tool,
        "cmd": req.cmd,
        "path": req.path,
        "url": req.url,
        "decision": decision.as_str(),
        "rule": rule.map(|(f, l)| format!("{f}:{l}")),
        "rule_text": rule_text,
    });
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("ledger.jsonl"))
    {
        let _ = writeln!(f, "{entry}");
    }
}

/// Read the most recent ledger decisions (newest first), up to `limit`. Used by
/// the `recent_decisions` surface.
pub fn recent_decisions(root: &Path, limit: usize) -> Vec<Value> {
    let path = root.join(".illuminate").join("policy").join("ledger.jsonl");
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut rows: Vec<Value> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .collect();
    rows.reverse();
    rows.truncate(limit);
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ledger_roundtrips_recent_decisions() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".illuminate")).unwrap();
        let req = EvalRequest {
            tool: "Bash".into(),
            cmd: "rm -rf /etc".into(),
            ..Default::default()
        };
        append_ledger(
            tmp.path(),
            &req,
            &Decision::Deny,
            Some(("default.rhai".into(), 5)),
            Some("deny if ..."),
        );
        append_ledger(tmp.path(), &req, &Decision::Allow, None, None);

        let recent = recent_decisions(tmp.path(), 10);
        assert_eq!(recent.len(), 2);
        // newest first
        assert_eq!(recent[0]["decision"], "allow");
        assert_eq!(recent[1]["decision"], "deny");
        assert_eq!(recent[1]["rule"], "default.rhai:5");
    }

    #[test]
    fn install_is_idempotent_and_preserves_other_hooks() {
        let tmp = tempfile::tempdir().unwrap();
        let settings = tmp.path().join(".claude").join("settings.json");
        std::fs::create_dir_all(settings.parent().unwrap()).unwrap();
        // Pre-existing unrelated PreToolUse hook must survive.
        std::fs::write(
            &settings,
            r#"{"hooks":{"PreToolUse":[{"matcher":"Write","command":"prettier"}]}}"#,
        )
        .unwrap();

        cmd_install("claude", Some(tmp.path().to_path_buf())).unwrap();
        cmd_install("claude", Some(tmp.path().to_path_buf())).unwrap(); // second run: no dup

        let cfg: Value =
            serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
        let pre = cfg["hooks"]["PreToolUse"].as_array().unwrap();
        // prettier preserved + exactly one policy-hook entry
        assert!(pre.iter().any(|e| e["command"] == "prettier"));
        let policy_entries = pre.iter().filter(|e| entry_runs_policy_hook(e)).count();
        assert_eq!(policy_entries, 1, "idempotent: exactly one policy hook");
    }

    #[test]
    fn template_minimalist_writes_a_parsing_policy() {
        let tmp = tempfile::tempdir().unwrap();
        cmd_template("minimalist", Some(tmp.path().to_path_buf()), false).unwrap();
        let path = tmp.path().join(".illuminate").join("policy.rhai");
        let src = std::fs::read_to_string(&path).unwrap();
        // It carries both the conservative baseline and the minimalism layer.
        assert!(
            src.contains("rm -rf"),
            "should include the default baseline"
        );
        assert!(src.contains("new dependency") || src.contains("new crate"));
        // And the written file actually parses + gates a `cargo add`.
        let eng = Engine::new();
        let rs = RuleSet::load(&eng, &path).unwrap();
        let req = EvalRequest {
            tool: "Bash".into(),
            cmd: "cargo add serde".into(),
            ..Default::default()
        };
        assert_eq!(eng.eval(&rs, &req).decision, Decision::Ask);
    }

    #[test]
    fn template_refuses_to_clobber_without_force() {
        let tmp = tempfile::tempdir().unwrap();
        cmd_template("default", Some(tmp.path().to_path_buf()), false).unwrap();
        // Second run without --force must error rather than overwrite.
        let err = cmd_template("minimalist", Some(tmp.path().to_path_buf()), false).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
        // With --force it succeeds and swaps the content.
        cmd_template("minimalist", Some(tmp.path().to_path_buf()), true).unwrap();
        let src =
            std::fs::read_to_string(tmp.path().join(".illuminate").join("policy.rhai")).unwrap();
        assert!(src.contains("new dependency") || src.contains("new crate"));
    }

    #[test]
    fn template_rejects_unknown_name() {
        let tmp = tempfile::tempdir().unwrap();
        let err = cmd_template("kitchen-sink", Some(tmp.path().to_path_buf()), false).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn entry_runs_policy_hook_detects_flat_and_nested() {
        assert!(entry_runs_policy_hook(
            &json!({ "command": "illuminate policy hook" })
        ));
        assert!(entry_runs_policy_hook(
            &json!({ "hooks": [{ "type": "command", "command": "illuminate policy hook" }] })
        ));
        assert!(!entry_runs_policy_hook(
            &json!({ "command": "illuminate audit-hook" })
        ));
    }
}
