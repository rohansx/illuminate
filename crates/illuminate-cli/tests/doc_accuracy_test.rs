//! Doc-accuracy guard tests.
//!
//! These tests pin `docs/CLI.md` and `docs/CRATES.md` to the actual shipped
//! surface so the docs cannot drift back into documenting phantom commands or
//! a stale crate count.
//!
//! Ground truth:
//! - The set of real CLI commands is derived from `--help` output of the built
//!   `illuminate` binary (top-level subcommands) plus the known nested
//!   subcommand verbs (`models download`, `wiki ...`, `trail ...`,
//!   `decisions for`, `failure log`), so the test does not need to parse the
//!   Rust `Commands` enum by hand.
//! - The crate count is the number of directories under `crates/`.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn cargo_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_illuminate"))
}

/// Repo root = two levels above this crate's manifest (`crates/illuminate-cli`).
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate manifest has a grandparent (repo root)")
        .to_path_buf()
}

fn read_doc(rel: &str) -> String {
    let path = repo_root().join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Top-level command names the binary actually exposes, scraped from
/// `illuminate --help`. Returns the lowercase verbs (e.g. `audit`,
/// `audit-diff`, `wiki`).
fn top_level_commands() -> BTreeSet<String> {
    let out = Command::new(cargo_bin())
        .arg("--help")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("illuminate --help runs");
    assert!(
        out.status.success(),
        "--help failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let help = String::from_utf8_lossy(&out.stdout);

    // clap renders a "Commands:" block where each line begins with two spaces
    // then the command name then more spaces then the about text.
    let mut cmds = BTreeSet::new();
    let mut in_commands = false;
    for line in help.lines() {
        let trimmed_lower = line.trim().to_lowercase();
        if trimmed_lower == "commands:" {
            in_commands = true;
            continue;
        }
        if in_commands {
            // Section ends at a blank line or a new header like "Options:".
            if line.trim().is_empty() || line.trim_end().ends_with(':') && !line.starts_with("  ") {
                break;
            }
            // A command line is indented and the first token is the verb.
            if line.starts_with("  ")
                && let Some(first) = line.split_whitespace().next()
            {
                cmds.insert(first.to_lowercase());
            }
        }
    }
    assert!(
        !cmds.is_empty(),
        "expected to scrape commands from --help, got none:\n{help}"
    );
    cmds
}

/// All `### illuminate <cmd...>` headings in CLI.md, returned as the command
/// path lowercased (e.g. `audit-pr`, `wiki rebuild`, `forget`).
fn cli_headings(doc: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in doc.lines() {
        let l = line.trim_start();
        // Headings look like: ### `illuminate audit-pr`
        if let Some(rest) = l.strip_prefix("### ") {
            let inner = rest.trim().trim_matches('`').trim();
            if let Some(cmd) = inner.strip_prefix("illuminate ") {
                out.push(cmd.trim().to_lowercase());
            }
        }
    }
    out
}

#[test]
fn cli_md_has_no_phantom_command_headings() {
    let doc = read_doc("docs/CLI.md");
    for phantom in ["forget", "purge", "migrate", "completions"] {
        let needle = format!("### `illuminate {phantom}`");
        assert!(
            !doc.contains(&needle),
            "docs/CLI.md still documents the non-existent `illuminate {phantom}` command \
             (no `{}` variant in the Commands enum)",
            phantom_variant(phantom)
        );
    }
}

fn phantom_variant(cmd: &str) -> &'static str {
    match cmd {
        "forget" => "Forget",
        "purge" => "Purge",
        "migrate" => "Migrate",
        "completions" => "Completions",
        _ => "<unknown>",
    }
}

#[test]
fn cli_md_does_not_mention_completions_subcommand() {
    // The `completions` verb has no enum variant; there must be no usage
    // example invoking it either (e.g. `illuminate completions bash`).
    let doc = read_doc("docs/CLI.md");
    assert!(
        !doc.contains("illuminate completions"),
        "docs/CLI.md still references the non-existent `illuminate completions` command"
    );
}

#[test]
fn every_cli_md_heading_maps_to_a_real_command() {
    let doc = read_doc("docs/CLI.md");
    let top = top_level_commands();

    // Nested subcommand verbs that are documented as their own heading but
    // live under a parent command. The parent must itself be a real command.
    let nested_parents: &[(&str, &str)] = &[
        ("models download", "models"),
        ("audit-diff", "audit-diff"),
        ("audit-pr", "audit-pr"),
        ("failure log", "failure"),
        ("decisions for", "decisions"),
        ("wiki rebuild", "wiki"),
        ("wiki serve", "wiki"),
        ("wiki lint", "wiki"),
        ("wiki review", "wiki"),
        ("wiki redact", "wiki"),
        ("trail list", "trail"),
        ("trail show", "trail"),
        ("trail purge", "trail"),
        ("trail install-service", "trail"),
    ];

    for heading in cli_headings(&doc) {
        // Direct top-level match (e.g. `audit`, `status`, `index`).
        let head_verb = heading.split_whitespace().next().unwrap_or("").to_string();
        if top.contains(&heading) || top.contains(&head_verb) {
            continue;
        }
        // Nested verb whose parent is a real top-level command.
        if let Some((_, parent)) = nested_parents.iter().find(|(h, _)| *h == heading) {
            assert!(
                top.contains(*parent),
                "heading `illuminate {heading}` maps to parent `{parent}` which is not a real command"
            );
            continue;
        }
        panic!(
            "docs/CLI.md heading `### illuminate {heading}` does not correspond to any real \
             command. Real top-level commands: {top:?}"
        );
    }
}

#[test]
fn crates_md_count_matches_disk() {
    let crates_dir = repo_root().join("crates");
    let on_disk = fs::read_dir(&crates_dir)
        .expect("read crates/")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .count();
    assert_eq!(
        on_disk, 17,
        "expected 17 crate dirs on disk, found {on_disk}"
    );

    let doc = read_doc("docs/CRATES.md");
    assert!(
        !doc.contains("14 crates") && !doc.contains("Fourteen crates"),
        "docs/CRATES.md still claims a stale crate count (14 / Fourteen); disk has {on_disk}"
    );
    assert!(
        doc.contains(&format!("{on_disk} crates")),
        "docs/CRATES.md should state the real crate count ({on_disk} crates)"
    );
}

#[test]
fn readme_has_no_stale_fourteen_crate_claim() {
    let doc = read_doc("README.md");
    assert!(
        !doc.contains("14 crates") && !doc.contains("Fourteen crates"),
        "README.md still claims a stale 14-crate count"
    );
}

/// Every command the binary exposes via `--help` must have a documented
/// `### illuminate <cmd>` heading in CLI.md — so newly-shipped verbs cannot
/// silently land without docs. Nested-only parents (`failure`, `decisions`,
/// `skill`, `hook`, `models`) are documented under a nested subcommand heading
/// (e.g. `### illuminate skill build`) and are covered by the "first token of
/// a heading" match below, so they are not required to have a bare parent
/// heading. A small allow-list covers commands that are intentionally
/// undocumented in the user-facing reference (internal hook entry points and
/// the legacy aliases that share a documented surface).
#[test]
fn every_real_command_is_documented_in_cli_md() {
    let doc = read_doc("docs/CLI.md");
    let top = top_level_commands();

    // Heading verbs present in CLI.md, by first token (so `wiki rebuild`
    // counts as documenting `wiki`, `skill build` documents `skill`, etc.).
    let documented_first_tokens: BTreeSet<String> = cli_headings(&doc)
        .iter()
        .filter_map(|h| h.split_whitespace().next().map(str::to_string))
        .collect();

    // Commands that are deliberately not given their own user-facing heading.
    // - `help` is clap's built-in.
    // - `mcp` / `serve` are the same server; `serve` is documented, `mcp` is
    //   its raw alias.
    // - `log` / `query` / `entities` / `export` / `watch` are legacy/low-level
    //   verbs documented elsewhere (SCHEMA/ARCHITECTURE) or via `decisions` /
    //   `search` / `trail`.
    // - `audit-hook` is an internal PreToolUse stdin entry point, wired by
    //   `hook install` / `init --hooks`, not invoked by hand.
    // - `reflect` is the MCP-facing reflexion recorder (see MCP.md), surfaced
    //   to humans as `failure log`.
    let exempt: BTreeSet<&str> = [
        "help",
        "mcp",
        "log",
        "query",
        "entities",
        "export",
        "watch",
        "audit-hook",
        "reflect",
    ]
    .into_iter()
    .collect();

    let mut missing = Vec::new();
    for cmd in &top {
        if exempt.contains(cmd.as_str()) {
            continue;
        }
        if !documented_first_tokens.contains(cmd) {
            missing.push(cmd.clone());
        }
    }
    assert!(
        missing.is_empty(),
        "these real commands have no `### illuminate <cmd>` heading in docs/CLI.md: {missing:?}"
    );
}

/// The Phase-H verbs plus the `ask --synthesize` flag must be documented.
/// This is the explicit H6 acceptance: each new verb has a backtick-wrapped
/// `### illuminate <cmd>` heading, and the synthesize flag is documented under
/// the existing ask section.
#[test]
fn phase_h_verbs_and_flags_are_documented() {
    let doc = read_doc("docs/CLI.md");
    let headings: BTreeSet<String> = cli_headings(&doc).into_iter().collect();

    // Phase-H + Phase-G verbs that landed this session must each own a heading
    // (by first token, so `skill build` / `hook install` count).
    let first_tokens: BTreeSet<String> = headings
        .iter()
        .filter_map(|h| h.split_whitespace().next().map(str::to_string))
        .collect();
    for verb in [
        "diagram",
        "oncall",
        "audit-docs",
        "onboard",
        "doc-decay",
        "skill",
        "hook",
    ] {
        assert!(
            first_tokens.contains(verb),
            "docs/CLI.md is missing a `### illuminate {verb}` heading for the shipped `{verb}` command"
        );
    }

    // The `ask --synthesize` flag must be documented in the ask section.
    let ask_start = doc
        .find("### `illuminate ask`")
        .expect("CLI.md has an `### illuminate ask` heading");
    // The ask section runs until the next `### ` heading.
    let rest = &doc[ask_start + "### `illuminate ask`".len()..];
    let ask_section_end = rest.find("\n### ").map(|i| i + 1).unwrap_or(rest.len());
    let ask_section = &rest[..ask_section_end];
    assert!(
        ask_section.contains("--synthesize"),
        "docs/CLI.md `### illuminate ask` section does not document the `--synthesize` flag"
    );
}

/// Returns the body of the `### \`illuminate <heading>\`` section in `doc`,
/// from just after the heading line up to (but not including) the next
/// `### ` heading (or end of file).
fn section_body<'a>(doc: &'a str, heading: &str) -> &'a str {
    let marker = format!("### `illuminate {heading}`");
    let start = doc
        .find(&marker)
        .unwrap_or_else(|| panic!("CLI.md has a `{marker}` heading"));
    let rest = &doc[start + marker.len()..];
    let end = rest.find("\n### ").map(|i| i + 1).unwrap_or(rest.len());
    &rest[..end]
}

/// The Phase-I surface that landed this batch must be documented:
/// - the new `illuminate trust check` verb has its own heading,
/// - the `ilm` shorthand-alias note is present (no separate binary heading,
///   but the alias relationship must be stated so users can find it),
/// - the `onboard` section documents the `cookbook` brief output that I1
///   added to both the human render and the `--json` envelope.
#[test]
fn phase_i_verbs_and_flags_are_documented() {
    let doc = read_doc("docs/CLI.md");
    let first_tokens: BTreeSet<String> = cli_headings(&doc)
        .iter()
        .filter_map(|h| h.split_whitespace().next().map(str::to_string))
        .collect();

    // The new `trust check` verb owns a heading (first token `trust`).
    assert!(
        first_tokens.contains("trust"),
        "docs/CLI.md is missing a `### illuminate trust check` heading for the shipped \
         `trust check` command"
    );

    // The `ilm` shorthand alias must be documented (it is a real shipped
    // binary that shares the command tree).
    assert!(
        doc.contains("`ilm`"),
        "docs/CLI.md does not document the `ilm` shorthand alias for `illuminate`"
    );

    // I1 added a prompt-cookbook section to `onboard` (human render + a
    // `cookbook[]` entry in the `--json` envelope) — the doc must reflect it.
    let onboard = section_body(&doc, "onboard");
    assert!(
        onboard.contains("cookbook"),
        "docs/CLI.md `### illuminate onboard` section does not document the `cookbook` \
         brief output (added in I1: a `cookbook[]` JSON array + a \"Prompt cookbook\" section)"
    );
}
