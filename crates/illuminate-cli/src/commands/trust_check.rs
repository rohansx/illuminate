//! `illuminate trust check` — deterministic config linter for off-host writes.
//!
//! The v3.0 trust-model exit criterion (see `docs/ROADMAP.md` and
//! `docs/trust-model.md`): a default `illuminate.toml` returns 0 with a clean
//! notice, and a config that names a **remote / off-host write target** (a
//! `TeamRepoTarget::GitRemote`-style `[publish]` entry, or a `[cloud.sync]`
//! block) WITHOUT a paired explicit `consent = true` flag returns non-zero
//! with a clearly-marked report that names the offending target.
//!
//! Pure + deterministic: reads `illuminate.toml` only (ancestor-walk from cwd,
//! same resolution as the audit policy loader). No network, no graph access.

use std::env;

use serde::Serialize;

/// One trust-linter finding. Stable JSON shape: `{target, message, key}`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TrustFinding {
    /// The offending write target (a git remote URL, a `[cloud.sync]` marker,
    /// etc.) — named so the dev can locate it in `illuminate.toml`.
    pub target: String,
    /// Human-readable explanation of why this target tripped the linter.
    pub message: String,
    /// The config key that holds the off-host target (e.g. `publish`).
    pub key: String,
}

/// The full report. Stable JSON envelope: `{ok, findings:[...]}`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TrustReport {
    pub ok: bool,
    pub findings: Vec<TrustFinding>,
}

/// Run the `trust check` subcommand. Returns `Ok(())` after printing; the
/// process exit code is set here (non-zero when findings are present) so the
/// caller's generic error path is not triggered for a clean linter failure —
/// mirrors the `audit` command's exit-code handling.
pub fn run(json: bool) -> illuminate::Result<()> {
    let config = load_config_text()?;
    let report = lint(config.as_deref());

    if json {
        // Stable, pretty JSON envelope — `{ok, findings:[...]}`.
        let rendered = serde_json::to_string_pretty(&report)
            .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;
        println!("{rendered}");
    } else {
        print_human(&report);
    }

    if !report.ok {
        // Non-zero, distinct from the generic error exit (1) and from audit's
        // violation (2) / warning (3): use 4 for "trust check failed".
        std::process::exit(4);
    }

    Ok(())
}

/// Lint an optional config body. A missing config (`None`) is treated the same
/// as an empty/default config — clean, since no off-host target is declared.
pub fn lint(config: Option<&str>) -> TrustReport {
    let Some(body) = config else {
        return TrustReport {
            ok: true,
            findings: Vec::new(),
        };
    };

    let value: toml::Value = match body.parse() {
        Ok(v) => v,
        Err(_) => {
            // Unparseable config — surface it as a finding rather than crashing,
            // so the linter degrades gracefully and stays deterministic.
            return TrustReport {
                ok: false,
                findings: vec![TrustFinding {
                    target: "illuminate.toml".to_string(),
                    message: "could not parse illuminate.toml as TOML".to_string(),
                    key: "illuminate.toml".to_string(),
                }],
            };
        }
    };

    let mut findings = Vec::new();
    findings.extend(check_publish_target(&value));
    findings.extend(check_cloud_sync(&value));

    TrustReport {
        ok: findings.is_empty(),
        findings,
    }
}

/// Lint the `[publish]` write target. An off-host `kind = "git_remote"` (or any
/// `url`-bearing remote shape) requires a paired `consent = true`.
fn check_publish_target(root: &toml::Value) -> Vec<TrustFinding> {
    let Some(publish) = root.get("publish").and_then(|v| v.as_table()) else {
        return Vec::new();
    };

    let kind = publish.get("kind").and_then(|v| v.as_str());
    let url = publish.get("url").and_then(|v| v.as_str());

    // A target is off-host when its kind is git_remote (matching
    // `TeamRepoTarget::GitRemote`) or it carries a remote `url`. A
    // `local_path` kind (or a bare `path`) is on-host and always clean.
    let is_off_host = matches!(kind, Some("git_remote") | Some("remote")) || url.is_some();
    if !is_off_host {
        return Vec::new();
    }

    if has_consent(publish) {
        return Vec::new();
    }

    // Name the offending target: prefer the concrete URL, else the kind.
    let target = url
        .map(str::to_string)
        .or_else(|| kind.map(|k| format!("[publish] kind = \"{k}\"")))
        .unwrap_or_else(|| "[publish]".to_string());

    vec![TrustFinding {
        target,
        message: "off-host publish target requires a paired `consent = true` flag in [publish]"
            .to_string(),
        key: "publish".to_string(),
    }]
}

/// Lint the `[cloud.sync]` block (v3-cloud). `enabled = true` without a paired
/// `consent = true` trips the linter — per `docs/trust-model.md` no auto-upload.
fn check_cloud_sync(root: &toml::Value) -> Vec<TrustFinding> {
    let sync = root
        .get("cloud")
        .and_then(|v| v.as_table())
        .and_then(|t| t.get("sync"))
        .and_then(|v| v.as_table());
    let Some(sync) = sync else {
        return Vec::new();
    };

    let enabled = sync
        .get("enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !enabled || has_consent(sync) {
        return Vec::new();
    }

    vec![TrustFinding {
        target: "[cloud.sync]".to_string(),
        message: "[cloud.sync] enabled requires a paired `consent = true` flag".to_string(),
        key: "cloud.sync".to_string(),
    }]
}

/// True when the given table carries an explicit `consent = true`.
fn has_consent(table: &toml::value::Table) -> bool {
    table.get("consent").and_then(|v| v.as_bool()) == Some(true)
}

fn print_human(report: &TrustReport) {
    if report.ok {
        println!("✓ trust check: OK — no off-host write target configured without consent");
        return;
    }

    println!("✗ trust check: off-host write target(s) need explicit consent");
    for f in &report.findings {
        println!("\n  Target: {}", f.target);
        println!("  Key:    [{}]", f.key);
        println!("  Reason: {}", f.message);
    }
    println!(
        "\nAdd `consent = true` to the named section in illuminate.toml to acknowledge \
         the off-host write, or switch to a local-path target."
    );
}

/// Read `illuminate.toml` text via the same ancestor-walk the audit policy
/// loader uses: nearest `.illuminate/illuminate.toml`, then `./illuminate.toml`.
/// Returns `Ok(None)` when no config exists — a default repo is clean.
fn load_config_text() -> illuminate::Result<Option<String>> {
    let cwd = env::current_dir().map_err(illuminate::IlluminateError::Io)?;
    let mut cur = Some(cwd.as_path());
    while let Some(d) = cur {
        let candidate = d.join(".illuminate").join("illuminate.toml");
        if candidate.is_file() {
            return Ok(Some(
                std::fs::read_to_string(&candidate).map_err(illuminate::IlluminateError::Io)?,
            ));
        }
        cur = d.parent();
    }

    let legacy = cwd.join("illuminate.toml");
    if legacy.is_file() {
        return Ok(Some(
            std::fs::read_to_string(&legacy).map_err(illuminate::IlluminateError::Io)?,
        ));
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_config_is_clean() {
        let r = lint(None);
        assert!(r.ok);
        assert!(r.findings.is_empty());
    }

    #[test]
    fn default_project_only_is_clean() {
        let r = lint(Some("[project]\nname = 'x'\n"));
        assert!(r.ok, "{r:?}");
    }

    #[test]
    fn local_path_target_is_clean() {
        let r = lint(Some("[publish]\nkind = 'local_path'\npath = '../team'\n"));
        assert!(r.ok, "{r:?}");
    }

    #[test]
    fn git_remote_without_consent_fails_and_names_url() {
        let r = lint(Some(
            "[publish]\nkind = 'git_remote'\nurl = 'git@host:team.git'\n",
        ));
        assert!(!r.ok);
        assert_eq!(r.findings.len(), 1);
        assert_eq!(r.findings[0].target, "git@host:team.git");
        assert_eq!(r.findings[0].key, "publish");
    }

    #[test]
    fn git_remote_with_consent_is_clean() {
        let r = lint(Some(
            "[publish]\nkind = 'git_remote'\nurl = 'git@host:team.git'\nconsent = true\n",
        ));
        assert!(r.ok, "{r:?}");
    }

    #[test]
    fn bare_url_without_kind_is_off_host() {
        let r = lint(Some("[publish]\nurl = 'https://x/team.git'\n"));
        assert!(!r.ok);
        assert!(r.findings[0].target.contains("https://x/team.git"));
    }

    #[test]
    fn cloud_sync_enabled_without_consent_fails() {
        let r = lint(Some("[cloud.sync]\nenabled = true\n"));
        assert!(!r.ok);
        assert_eq!(r.findings[0].key, "cloud.sync");
    }

    #[test]
    fn cloud_sync_enabled_with_consent_is_clean() {
        let r = lint(Some("[cloud.sync]\nenabled = true\nconsent = true\n"));
        assert!(r.ok, "{r:?}");
    }

    #[test]
    fn unparseable_config_is_a_finding_not_a_panic() {
        let r = lint(Some("this is = = not valid toml ]["));
        assert!(!r.ok);
        assert_eq!(r.findings.len(), 1);
    }
}
