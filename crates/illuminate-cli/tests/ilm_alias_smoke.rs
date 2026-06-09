//! Smoke test for the `ilm` shorthand alias (v3.2 roadmap).
//!
//! The `illuminate-cli` crate emits a second binary, `ilm`, that is
//! behaviorally identical to `illuminate` — same clap command tree, same
//! dispatch — so `ilm ask` / `ilm onboard` / `ilm ingest` resolve exactly as
//! their `illuminate` counterparts. This is achieved by a `[[bin]]` target
//! that shares the existing command dispatch (no duplicated logic), not by a
//! re-implementation, so the two binaries can never drift.
//!
//! No mocks: both binaries are the real cargo-built artifacts, and the graph
//! seeded for the parity check is a real on-disk SQLite database (mirroring the
//! sibling `onboard_smoke`).
//!
//! Asserted here:
//! 1. `ilm --help` lists the same top-level subcommands as `illuminate --help`
//!    (both contain onboard / ingest / ask / audit, and the full set matches).
//! 2. `ilm onboard` on a seeded tempdir graph is byte-identical to
//!    `illuminate onboard` on the same graph.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use illuminate::Episode;

fn illuminate_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_illuminate"))
}

/// The aliased binary. Cargo exports `CARGO_BIN_EXE_ilm` for the second
/// `[[bin]]` target named `ilm`; if that target is missing this test will not
/// compile, which is exactly the RED state we want before the alias exists.
fn ilm_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_ilm"))
}

fn run_bin(bin: &Path, repo: &Path, args: &[&str]) -> Output {
    Command::new(bin)
        .args(args)
        .current_dir(repo)
        // Force the extraction pipeline off so the test doesn't depend on ONNX
        // models existing on the host (mirrors onboard_smoke).
        .env("ILLUMINATE_MODELS_DIR", "/nonexistent/illuminate/models")
        .env("HOME", repo)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("subprocess must run")
}

/// Initialize `.illuminate/` + `illuminate.toml`; returns nothing (mutates dir).
fn init_repo(repo: &Path) {
    fs::create_dir_all(repo.join(".illuminate")).unwrap();
    fs::write(
        repo.join(".illuminate/illuminate.toml"),
        "[project]\nname = 'ilm-alias-smoke'\n",
    )
    .unwrap();
}

/// Seed one decision episode directly via the library `Graph`, creating
/// `.illuminate/graph.db` (mirrors `onboard_smoke::seed_decision`).
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

/// Extract the set of top-level subcommand names from a clap `--help` dump.
///
/// clap renders subcommands under a `Commands:` section, one per line, indented,
/// with the command name as the first token. We collect every first-token after
/// the `Commands:` header until the next un-indented section header (e.g.
/// `Options:`), skipping blank lines and continuation lines.
fn subcommands_from_help(help: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut in_commands = false;
    for line in help.lines() {
        let trimmed = line.trim_end();
        if trimmed.trim_start().starts_with("Commands:") {
            in_commands = true;
            continue;
        }
        if !in_commands {
            continue;
        }
        // A new non-indented section (Options:, Arguments:, etc.) ends the list.
        if !trimmed.is_empty() && !trimmed.starts_with(char::is_whitespace) {
            break;
        }
        if trimmed.trim().is_empty() {
            continue;
        }
        let first = trimmed.split_whitespace().next().unwrap_or("");
        // The synthetic `help` subcommand clap appends is part of the tree for
        // both binaries, so keeping it is fine — it stays consistent.
        if !first.is_empty() {
            names.insert(first.to_string());
        }
    }
    names
}

#[test]
fn ilm_help_lists_same_top_level_subcommands_as_illuminate() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();

    let illuminate_help = run_bin(&illuminate_bin(), repo, &["--help"]);
    let ilm_help = run_bin(&ilm_bin(), repo, &["--help"]);

    assert!(
        illuminate_help.status.success(),
        "`illuminate --help` must exit 0; stderr: {}",
        String::from_utf8_lossy(&illuminate_help.stderr)
    );
    assert!(
        ilm_help.status.success(),
        "`ilm --help` must exit 0; stderr: {}",
        String::from_utf8_lossy(&ilm_help.stderr)
    );

    let illuminate_cmds = subcommands_from_help(&String::from_utf8_lossy(&illuminate_help.stdout));
    let ilm_cmds = subcommands_from_help(&String::from_utf8_lossy(&ilm_help.stdout));

    // Sanity: the parser actually found a meaningful command tree.
    for expected in ["onboard", "ingest", "ask", "audit"] {
        assert!(
            illuminate_cmds.contains(expected),
            "`illuminate --help` should list `{expected}`; found {illuminate_cmds:?}"
        );
        assert!(
            ilm_cmds.contains(expected),
            "`ilm --help` should list `{expected}`; found {ilm_cmds:?}"
        );
    }

    // The full top-level subcommand set must be identical between the two.
    assert_eq!(
        illuminate_cmds, ilm_cmds,
        "ilm must expose the exact same top-level subcommands as illuminate"
    );
}

#[test]
fn ilm_onboard_is_byte_identical_to_illuminate_onboard() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);
    seed_decision(repo);

    let illuminate_out = run_bin(&illuminate_bin(), repo, &["onboard"]);
    let ilm_out = run_bin(&ilm_bin(), repo, &["onboard"]);

    assert!(
        illuminate_out.status.success(),
        "`illuminate onboard` must exit 0; stderr: {}",
        String::from_utf8_lossy(&illuminate_out.stderr)
    );
    assert!(
        ilm_out.status.success(),
        "`ilm onboard` must exit 0; stderr: {}",
        String::from_utf8_lossy(&ilm_out.stderr)
    );

    assert_eq!(
        illuminate_out.stdout, ilm_out.stdout,
        "`ilm onboard` stdout must be byte-identical to `illuminate onboard` on the same graph"
    );
    assert_eq!(
        illuminate_out.status.code(),
        ilm_out.status.code(),
        "`ilm onboard` exit code must match `illuminate onboard`"
    );
}
