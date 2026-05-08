//! `illuminate audit-pr <pr-number>` — audit a GitHub PR by shelling out
//! to the `gh` CLI for PR metadata + changed files, then running the same
//! `Auditor::audit_with_files` pipeline `audit-diff` uses.
//!
//! The PR title is used as the plan text. Output is markdown by default,
//! ready to drop into a PR comment when `--comment` is passed; `--format json`
//! mirrors the AuditResult JSON envelope plus PR metadata.
//!
//! Exit codes match `audit` / `audit-diff`: Pass=0, Violation=2, Warning=3.
//!
//! `gh` is required — bails with a clear error if the binary isn't installed.
//! Repo is auto-detected from `git remote get-url origin` when `--repo` is not
//! supplied.
//!
//! See `docs/CLI.md` for the canonical specification of this command.

use std::path::PathBuf;
use std::process::Command;

use illuminate_audit::Auditor;
use illuminate_audit::resolve_index_db_from_cwd;
use illuminate_audit::resolve_repo_root_from_cwd;
use illuminate_audit::response::{AuditResult, AuditStatus};
use serde::Serialize;
use serde_json::Value;

use super::audit::{load_audit_config, load_policies};
use super::open_graph;

/// Cap on impacted-symbol entries shown in markdown output. Mirrors
/// `audit_diff::HUMAN_IMPACT_LIMIT` so the two views read alike.
const MD_IMPACT_LIMIT: usize = 10;

/// Cap on relevant-decision entries (each is a multi-line preview).
const MD_RELEVANT_LIMIT: usize = 5;

/// Run the `audit-pr` subcommand.
pub fn run(
    pr_number: u64,
    repo: Option<String>,
    token_env: Option<String>,
    comment: bool,
    format: Option<String>,
) -> illuminate::Result<()> {
    // Verify `gh` is on PATH. `gh --version` is the cheapest sentinel and
    // matches what `gh` itself uses for plumbing detection.
    if Command::new("gh").arg("--version").output().is_err() {
        return Err(illuminate::IlluminateError::Extraction(
            "gh CLI not installed; install via https://cli.github.com/".into(),
        ));
    }

    // Resolve repo (CLI flag wins; fall back to origin URL parsing).
    let repo = match repo {
        Some(r) => r,
        None => detect_repo_from_origin()?,
    };

    // Fetch PR metadata + changed files via `gh`.
    let pr_data = fetch_pr_data(&repo, pr_number)?;
    let title = pr_data
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let url = pr_data
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let files = fetch_changed_files(&repo, pr_number)?;

    // Wire up the auditor mirroring `audit::run` / `audit_diff::run`.
    let graph = open_graph()?;
    let policies = load_policies()?;
    let audit_config = load_audit_config()?;
    let resolved_index = resolve_index_db_from_cwd(None);
    let resolved_root = resolve_repo_root_from_cwd();
    let embed = super::audit::try_load_embed_pub();

    let result = match resolved_index {
        Some(path) => {
            let auditor = Auditor::with_index_root_and_embed(
                graph,
                policies,
                path,
                resolved_root,
                embed,
                audit_config.semantic_top_k,
                audit_config.semantic_threshold,
            );
            auditor
                .audit_with_files(&title, &files)
                .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?
        }
        None => {
            let auditor = match embed {
                Some(e) => Auditor::with_index_root_and_embed(
                    graph,
                    policies,
                    PathBuf::from("/nonexistent/illuminate-audit-no-index.db"),
                    None::<PathBuf>,
                    Some(e),
                    audit_config.semantic_top_k,
                    audit_config.semantic_threshold,
                ),
                None => Auditor::new(graph, policies),
            };
            auditor
                .audit_with_files(&title, &files)
                .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?
        }
    };

    // Format and print to stdout.
    let format_str = format.as_deref().unwrap_or("markdown");
    let output = match format_str {
        "json" => format_json(pr_number, &title, &url, &result, &files)?,
        _ => format_markdown(pr_number, &title, &url, &result, &files),
    };
    println!("{output}");

    // `--comment` posts the markdown body to the PR. JSON is preserved on
    // stdout for tooling but we never post raw JSON as a PR comment.
    if comment {
        let body = if format_str == "json" {
            // Re-render markdown for the comment so JSON consumers and
            // human reviewers both get useful output.
            format_markdown(pr_number, &title, &url, &result, &files)
        } else {
            output
        };
        post_pr_comment(&repo, pr_number, &body, token_env.as_deref())?;
    }

    match result.status {
        AuditStatus::Pass => Ok(()),
        AuditStatus::Warning => std::process::exit(3),
        AuditStatus::Violation => std::process::exit(2),
    }
}

/// Resolve `OWNER/REPO` from `git remote get-url origin`. Returns an
/// `Extraction` error when the remote is missing or the URL doesn't look
/// like a GitHub remote — caller should re-run with `--repo` in that case.
fn detect_repo_from_origin() -> illuminate::Result<String> {
    let out = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .map_err(illuminate::IlluminateError::Io)?;
    if !out.status.success() {
        return Err(illuminate::IlluminateError::Extraction(
            "could not resolve `origin` remote; supply --repo OWNER/REPO".into(),
        ));
    }
    let url = String::from_utf8_lossy(&out.stdout).trim().to_string();
    parse_github_repo(&url).ok_or_else(|| {
        illuminate::IlluminateError::Extraction(format!(
            "could not parse GitHub repo from origin URL `{url}`; supply --repo OWNER/REPO"
        ))
    })
}

/// Parse `owner/repo` out of common GitHub remote URL forms:
/// - `git@github.com:owner/repo.git`
/// - `https://github.com/owner/repo[.git]`
/// - `http://github.com/owner/repo[.git]`
/// - `ssh://git@github.com/owner/repo[.git]`
///
/// Returns `None` for non-GitHub hosts so callers can fall back to a
/// `--repo` flag.
pub(crate) fn parse_github_repo(url: &str) -> Option<String> {
    let trimmed = url.trim().trim_end_matches('/').trim_end_matches(".git");
    for prefix in [
        "git@github.com:",
        "https://github.com/",
        "http://github.com/",
        "ssh://git@github.com/",
    ] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            // Reject if the remainder isn't `owner/repo` shaped.
            let parts: Vec<&str> = rest.split('/').filter(|s| !s.is_empty()).collect();
            if parts.len() == 2 {
                return Some(format!("{}/{}", parts[0], parts[1]));
            }
            return None;
        }
    }
    None
}

/// Fetch PR metadata via `gh pr view --json baseRefName,headRefName,title,number,url`.
fn fetch_pr_data(repo: &str, pr: u64) -> illuminate::Result<Value> {
    let out = Command::new("gh")
        .args([
            "pr",
            "view",
            &pr.to_string(),
            "--repo",
            repo,
            "--json",
            "baseRefName,headRefName,title,number,url",
        ])
        .output()
        .map_err(illuminate::IlluminateError::Io)?;
    if !out.status.success() {
        return Err(illuminate::IlluminateError::Extraction(format!(
            "gh pr view failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    serde_json::from_slice(&out.stdout)
        .map_err(|e| illuminate::IlluminateError::Extraction(format!("gh json parse: {e}")))
}

/// Fetch the list of changed files via `gh pr diff --name-only`.
fn fetch_changed_files(repo: &str, pr: u64) -> illuminate::Result<Vec<PathBuf>> {
    let out = Command::new("gh")
        .args(["pr", "diff", &pr.to_string(), "--repo", repo, "--name-only"])
        .output()
        .map_err(illuminate::IlluminateError::Io)?;
    if !out.status.success() {
        return Err(illuminate::IlluminateError::Extraction(format!(
            "gh pr diff failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .collect())
}

/// JSON envelope for `--format json`. Mirrors `AuditResult` plus PR metadata
/// so consumers don't need to cross-reference a separate `gh pr view` call.
#[derive(Serialize)]
struct JsonOutput<'a> {
    pr_number: u64,
    title: &'a str,
    url: &'a str,
    status: AuditStatus,
    files_changed: &'a [PathBuf],
    audit_result: &'a AuditResult,
}

fn format_json(
    pr: u64,
    title: &str,
    url: &str,
    result: &AuditResult,
    files: &[PathBuf],
) -> illuminate::Result<String> {
    let payload = JsonOutput {
        pr_number: pr,
        title,
        url,
        status: result.status,
        files_changed: files,
        audit_result: result,
    };
    serde_json::to_string_pretty(&payload)
        .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))
}

/// Render the audit result as a self-contained markdown block suitable for
/// dropping into a PR comment. Status emoji is the load-bearing visual cue;
/// blocks below are emitted only when the corresponding result field is
/// non-empty so green builds produce clean comments.
pub(crate) fn format_markdown(
    pr: u64,
    title: &str,
    url: &str,
    result: &AuditResult,
    files: &[PathBuf],
) -> String {
    let (emoji, status_text) = match result.status {
        AuditStatus::Pass => ("✅", "Pass"),
        AuditStatus::Warning => ("⚠️", "Warning"),
        AuditStatus::Violation => ("❌", "Violation"),
    };

    let mut out = String::new();
    out.push_str(&format!(
        "## {emoji} Illuminate audit — PR #{pr}: {title}\n\n"
    ));
    out.push_str(&format!("**Status:** {status_text}\n\n"));
    out.push_str(&format!("**Files audited:** {}\n\n", files.len()));

    if !result.policy_violations.is_empty() {
        out.push_str("### Policy violations\n\n");
        for v in &result.policy_violations {
            out.push_str(&format!(
                "- **{}** ({:?}): {}\n",
                v.policy_name, v.severity, v.reason
            ));
        }
        out.push('\n');
    }

    if !result.violations.is_empty() {
        out.push_str("### Decision conflicts\n\n");
        for v in &result.violations {
            out.push_str(&format!("- {} ({:?}): ", v.plan_entity, v.violation_type));
            if let Some(d) = &v.conflicting_decision {
                let preview: String = d.content.chars().take(120).collect();
                out.push_str(&preview.replace('\n', " "));
            }
            out.push('\n');
        }
        out.push('\n');
    }

    if !result.relevant_decisions.is_empty() {
        out.push_str("### Related decisions\n\n");
        for d in result.relevant_decisions.iter().take(MD_RELEVANT_LIMIT) {
            let preview: String = d.content_preview.chars().take(100).collect();
            let preview = preview.replace('\n', " ");
            let label = d.source.as_deref().unwrap_or(&d.episode_id);
            out.push_str(&format!("- `{label}`: {preview}\n"));
        }
        out.push('\n');
    }

    if !result.impact.impacted_symbols.is_empty() {
        out.push_str(&format!(
            "### Blast radius\n\n{} symbols impacted across changes:\n\n",
            result.impact.impacted_symbols.len()
        ));
        for s in result.impact.impacted_symbols.iter().take(MD_IMPACT_LIMIT) {
            out.push_str(&format!("- `{s}`\n"));
        }
        if result.impact.impacted_symbols.len() > MD_IMPACT_LIMIT {
            out.push_str(&format!(
                "- _… ({} more)_\n",
                result.impact.impacted_symbols.len() - MD_IMPACT_LIMIT
            ));
        }
        out.push('\n');
    }

    out.push_str("---\n");
    if url.is_empty() {
        out.push_str("_Audited with `illuminate audit-pr`._\n");
    } else {
        out.push_str(&format!(
            "_Audited with `illuminate audit-pr`. PR: {url}_\n"
        ));
    }
    out
}

/// Post `body` as a PR comment via `gh pr comment --body-file`. The body is
/// written to a tempfile (cleaner than `--body "..."` which has shell
/// quoting limits and bash arg-length caps).
///
/// `token_env` is the env-var NAME (e.g. `"GITHUB_TOKEN"`); when set and
/// resolvable we forward the token to `gh` via `GH_TOKEN`. `gh` falls back
/// to its own auth otherwise so this is best-effort.
fn post_pr_comment(
    repo: &str,
    pr: u64,
    body: &str,
    token_env: Option<&str>,
) -> illuminate::Result<()> {
    let dir = tempfile::tempdir().map_err(illuminate::IlluminateError::Io)?;
    let body_path = dir.path().join("comment.md");
    std::fs::write(&body_path, body)?;

    let mut cmd = Command::new("gh");
    cmd.args([
        "pr",
        "comment",
        &pr.to_string(),
        "--repo",
        repo,
        "--body-file",
    ]);
    cmd.arg(&body_path);
    if let Some(env_name) = token_env
        && let Ok(token) = std::env::var(env_name)
    {
        cmd.env("GH_TOKEN", token);
    }

    let out = cmd.output().map_err(illuminate::IlluminateError::Io)?;
    if !out.status.success() {
        return Err(illuminate::IlluminateError::Extraction(format!(
            "gh pr comment failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    eprintln!("posted comment to PR #{pr}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use illuminate_audit::policy::PolicyViolation;
    use illuminate_audit::response::{AuditResult, AuditStatus, ImpactInfo, Severity};

    fn empty_result(status: AuditStatus) -> AuditResult {
        AuditResult {
            status,
            violations: Vec::new(),
            policy_violations: Vec::new(),
            reflexions: Vec::new(),
            impact: ImpactInfo::default(),
            relevant_decisions: Vec::new(),
            trace_id: String::new(),
            policies_applied: Vec::new(),
            wiki_url: None,
        }
    }

    #[test]
    fn parse_github_repo_handles_ssh_url() {
        assert_eq!(
            parse_github_repo("git@github.com:foo/bar.git"),
            Some("foo/bar".to_string())
        );
    }

    #[test]
    fn parse_github_repo_handles_https_url() {
        assert_eq!(
            parse_github_repo("https://github.com/foo/bar.git"),
            Some("foo/bar".to_string())
        );
        // No trailing `.git` is also fine.
        assert_eq!(
            parse_github_repo("https://github.com/foo/bar"),
            Some("foo/bar".to_string())
        );
    }

    #[test]
    fn parse_github_repo_returns_none_for_other_hosts() {
        assert_eq!(parse_github_repo("git@gitlab.com:foo/bar.git"), None);
        assert_eq!(parse_github_repo("https://bitbucket.org/foo/bar"), None);
        assert_eq!(parse_github_repo(""), None);
    }

    #[test]
    fn format_markdown_has_status_emoji() {
        let result = empty_result(AuditStatus::Pass);
        let md = format_markdown(42, "fix flaky test", "https://x", &result, &[]);
        assert!(md.contains("✅"), "expected pass emoji, got: {md}");
        assert!(md.contains("PR #42"));
        assert!(md.contains("fix flaky test"));
    }

    #[test]
    fn format_markdown_includes_policy_violations() {
        let mut result = empty_result(AuditStatus::Violation);
        result.policy_violations.push(PolicyViolation {
            policy_name: "no-redis".into(),
            expected: Some("in-memory cache".into()),
            found: Some("redis".into()),
            reason: "we never use redis".into(),
            severity: Severity::Error,
            decision_ref: None,
            evidence: None,
            confidence: 1.0,
        });
        let md = format_markdown(7, "add redis cache", "https://x", &result, &[]);
        assert!(md.contains("❌"), "expected violation emoji");
        assert!(md.contains("Policy violations"));
        assert!(md.contains("no-redis"));
        assert!(md.contains("we never use redis"));
    }

    #[test]
    fn format_markdown_warning_status_uses_warning_emoji() {
        let result = empty_result(AuditStatus::Warning);
        let md = format_markdown(3, "soft conflict", "", &result, &[]);
        assert!(md.contains("⚠️"), "expected warning emoji, got: {md}");
        // Empty url path should fall through to the no-link footer.
        assert!(md.contains("Audited with `illuminate audit-pr`"));
        assert!(!md.contains("PR: "));
    }
}
