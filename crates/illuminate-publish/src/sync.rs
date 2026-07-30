//! `illuminate sync` — move published knowledge between a developer's machine
//! and the team's git remote.
//!
//! ## The gap this closes
//!
//! Before this, "team" meant "a shared folder on local disk". [`crate::publish`]
//! wrote a page into a directory and stopped; nothing carried it to a teammate,
//! and nothing brought a teammate's decisions back. Every competing company-brain
//! product gets team scale for free by being a cloud service — illuminate's
//! local-first design means it has to be built, and this is it.
//!
//! ## Shape
//!
//! Planning is pure ([`plan_sync`]) and execution goes through the [`GitOps`]
//! trait, so the whole orchestration is unit-testable without a network or a
//! git binary. The only implementation that touches the world is
//! [`SystemGit`], which shells out.
//!
//! ## Trust model
//!
//! Sync is the *only* place in illuminate that reaches the network on the
//! write path, so the gate is explicit and mirrors what
//! `illuminate trust check` enforces on `illuminate.toml`:
//!
//! - `consent: false` refuses before any step runs.
//! - `--dry-run` plans and prints without executing anything.
//! - `--no-push` fetches and merges but never uploads, so a developer can
//!   consume the team's knowledge without publishing their own.
//! - Pull is fast-forward only. Sync will not invent a merge commit in someone
//!   else's repository.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{PublishError, Result};

/// Where the team repo lives and whether we are allowed to talk to it.
#[derive(Debug, Clone)]
pub struct SyncConfig {
    pub url: String,
    pub branch: String,
    /// Local working clone — the same directory `publish` writes into.
    pub local_clone: PathBuf,
    /// Explicit opt-in for off-host network access.
    pub consent: bool,
}

/// Caller-chosen modifiers.
#[derive(Debug, Clone, Copy, Default)]
pub struct SyncOptions {
    /// Plan only; execute nothing.
    pub dry_run: bool,
    /// Fetch and merge, but never push local commits.
    pub no_push: bool,
}

/// One unit of work. Ordered: you cannot push what you have not merged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncStep {
    /// Update remote-tracking refs.
    Fetch,
    /// Fast-forward the local branch onto the remote.
    FastForward,
    /// Upload local commits.
    Push,
}

impl SyncStep {
    pub fn as_str(self) -> &'static str {
        match self {
            SyncStep::Fetch => "fetch",
            SyncStep::FastForward => "fast-forward",
            SyncStep::Push => "push",
        }
    }
}

/// A validated, ordered plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncPlan {
    pub steps: Vec<SyncStep>,
    pub local_clone: PathBuf,
    pub branch: String,
    pub url: String,
    pub dry_run: bool,
}

/// What actually happened.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncReport {
    pub executed: Vec<SyncStep>,
    pub skipped_dry_run: bool,
    /// True when a push was planned but there was nothing local to send.
    pub nothing_to_push: bool,
}

/// Build the ordered plan for a sync, validating the target first.
///
/// Pure: no I/O, no clock. The same config and options always give the same
/// plan, which is what makes `--dry-run` a trustworthy preview of the real run.
pub fn plan_sync(cfg: &SyncConfig, opts: SyncOptions) -> Result<SyncPlan> {
    if cfg.url.trim().is_empty() {
        return Err(PublishError::InvalidTarget(
            "sync target has an empty url".to_string(),
        ));
    }
    if cfg.branch.trim().is_empty() {
        return Err(PublishError::InvalidTarget(
            "sync target has an empty branch".to_string(),
        ));
    }
    if !cfg.consent {
        return Err(PublishError::ConsentRequired(cfg.url.clone()));
    }

    let mut steps = vec![SyncStep::Fetch, SyncStep::FastForward];
    if !opts.no_push {
        steps.push(SyncStep::Push);
    }

    Ok(SyncPlan {
        steps,
        local_clone: cfg.local_clone.clone(),
        branch: cfg.branch.clone(),
        url: cfg.url.clone(),
        dry_run: opts.dry_run,
    })
}

/// The git operations sync needs. Injected so the orchestration can be tested
/// without a git binary, a network, or a real repository.
pub trait GitOps {
    fn fetch(&self, dir: &Path, branch: &str) -> Result<()>;
    /// Fast-forward only. Must fail rather than create a merge commit.
    fn fast_forward(&self, dir: &Path, branch: &str) -> Result<()>;
    fn push(&self, dir: &Path, branch: &str) -> Result<()>;
    /// Whether the local branch has commits the remote does not.
    fn has_unpushed_commits(&self, dir: &Path, branch: &str) -> Result<bool>;
}

/// Execute `plan` against `git`.
///
/// Steps run in order and the first failure aborts the rest — a failed fetch
/// must never be followed by a push, or a developer could upload commits based
/// on a stale view of the team's history.
pub fn run_sync(git: &dyn GitOps, plan: &SyncPlan) -> Result<SyncReport> {
    let mut report = SyncReport::default();

    if plan.dry_run {
        report.skipped_dry_run = true;
        return Ok(report);
    }

    for step in &plan.steps {
        match step {
            SyncStep::Fetch => git.fetch(&plan.local_clone, &plan.branch)?,
            SyncStep::FastForward => git.fast_forward(&plan.local_clone, &plan.branch)?,
            SyncStep::Push => {
                // Asking git to push nothing is harmless but noisy; more
                // importantly, reporting "pushed" when nothing moved is a lie.
                if !git.has_unpushed_commits(&plan.local_clone, &plan.branch)? {
                    report.nothing_to_push = true;
                    continue;
                }
                git.push(&plan.local_clone, &plan.branch)?;
            }
        }
        report.executed.push(*step);
    }
    Ok(report)
}

/// [`GitOps`] backed by the `git` binary.
pub struct SystemGit;

impl SystemGit {
    fn run(dir: &Path, args: &[&str]) -> Result<std::process::Output> {
        let out = Command::new("git")
            .current_dir(dir)
            .args(args)
            .output()
            .map_err(PublishError::Io)?;
        if !out.status.success() {
            return Err(PublishError::InvalidTarget(format!(
                "git {} failed in {}: {}",
                args.join(" "),
                dir.display(),
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
        Ok(out)
    }
}

impl GitOps for SystemGit {
    fn fetch(&self, dir: &Path, branch: &str) -> Result<()> {
        SystemGit::run(dir, &["fetch", "origin", branch]).map(|_| ())
    }

    fn fast_forward(&self, dir: &Path, branch: &str) -> Result<()> {
        // `--ff-only` is the trust guarantee: sync never fabricates a merge in
        // a repository it does not own.
        SystemGit::run(dir, &["merge", "--ff-only", &format!("origin/{branch}")]).map(|_| ())
    }

    fn push(&self, dir: &Path, branch: &str) -> Result<()> {
        SystemGit::run(dir, &["push", "origin", branch]).map(|_| ())
    }

    fn has_unpushed_commits(&self, dir: &Path, branch: &str) -> Result<bool> {
        let out = SystemGit::run(
            dir,
            &["rev-list", "--count", &format!("origin/{branch}..{branch}")],
        )?;
        let count: usize = String::from_utf8_lossy(&out.stdout)
            .trim()
            .parse()
            .unwrap_or(0);
        Ok(count > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// Records calls instead of running git. Lets the orchestration be tested
    /// exactly — order, abort-on-failure, and the no-op push path.
    struct RecordingGit {
        calls: RefCell<Vec<String>>,
        unpushed: bool,
        fail_on: Option<&'static str>,
    }

    impl RecordingGit {
        fn new() -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
                unpushed: true,
                fail_on: None,
            }
        }
        fn with_nothing_to_push() -> Self {
            Self {
                unpushed: false,
                ..Self::new()
            }
        }
        fn failing_on(op: &'static str) -> Self {
            Self {
                fail_on: Some(op),
                ..Self::new()
            }
        }
        fn record(&self, op: &str) -> Result<()> {
            self.calls.borrow_mut().push(op.to_string());
            if self.fail_on == Some(op) {
                return Err(PublishError::InvalidTarget(format!("{op} failed")));
            }
            Ok(())
        }
        fn calls(&self) -> Vec<String> {
            self.calls.borrow().clone()
        }
    }

    impl GitOps for RecordingGit {
        fn fetch(&self, _: &Path, _: &str) -> Result<()> {
            self.record("fetch")
        }
        fn fast_forward(&self, _: &Path, _: &str) -> Result<()> {
            self.record("fast_forward")
        }
        fn push(&self, _: &Path, _: &str) -> Result<()> {
            self.record("push")
        }
        fn has_unpushed_commits(&self, _: &Path, _: &str) -> Result<bool> {
            Ok(self.unpushed)
        }
    }

    fn cfg(consent: bool) -> SyncConfig {
        SyncConfig {
            url: "https://git.invalid/acme/team.git".to_string(),
            branch: "main".to_string(),
            local_clone: PathBuf::from("/tmp/clone"),
            consent,
        }
    }

    // ------------------------------------------------------------ plan ---

    #[test]
    fn a_full_plan_fetches_then_fast_forwards_then_pushes() {
        let plan = plan_sync(&cfg(true), SyncOptions::default()).expect("plans");
        assert_eq!(
            plan.steps,
            vec![SyncStep::Fetch, SyncStep::FastForward, SyncStep::Push]
        );
    }

    #[test]
    fn no_push_plans_a_consume_only_sync() {
        let plan = plan_sync(
            &cfg(true),
            SyncOptions {
                no_push: true,
                ..Default::default()
            },
        )
        .expect("plans");
        assert_eq!(plan.steps, vec![SyncStep::Fetch, SyncStep::FastForward]);
        assert!(!plan.steps.contains(&SyncStep::Push));
    }

    #[test]
    fn syncing_without_consent_is_refused_before_any_step() {
        let err = plan_sync(&cfg(false), SyncOptions::default()).expect_err("must refuse");
        assert!(matches!(err, PublishError::ConsentRequired(_)), "{err:?}");
    }

    #[test]
    fn an_empty_url_or_branch_is_rejected() {
        let mut c = cfg(true);
        c.url = String::new();
        assert!(matches!(
            plan_sync(&c, SyncOptions::default()),
            Err(PublishError::InvalidTarget(_))
        ));

        let mut c = cfg(true);
        c.branch = "  ".to_string();
        assert!(matches!(
            plan_sync(&c, SyncOptions::default()),
            Err(PublishError::InvalidTarget(_))
        ));
    }

    #[test]
    fn planning_is_deterministic() {
        let a = plan_sync(&cfg(true), SyncOptions::default()).unwrap();
        let b = plan_sync(&cfg(true), SyncOptions::default()).unwrap();
        assert_eq!(a, b);
    }

    // ------------------------------------------------------------- run ---

    #[test]
    fn a_dry_run_executes_nothing() {
        let git = RecordingGit::new();
        let plan = plan_sync(
            &cfg(true),
            SyncOptions {
                dry_run: true,
                ..Default::default()
            },
        )
        .unwrap();
        let report = run_sync(&git, &plan).expect("dry run");
        assert!(report.skipped_dry_run);
        assert!(report.executed.is_empty());
        assert!(git.calls().is_empty(), "dry run must touch nothing");
    }

    #[test]
    fn a_real_run_executes_the_steps_in_order() {
        let git = RecordingGit::new();
        let plan = plan_sync(&cfg(true), SyncOptions::default()).unwrap();
        let report = run_sync(&git, &plan).expect("run");
        assert_eq!(git.calls(), vec!["fetch", "fast_forward", "push"]);
        assert_eq!(
            report.executed,
            vec![SyncStep::Fetch, SyncStep::FastForward, SyncStep::Push]
        );
    }

    #[test]
    fn a_failed_fetch_aborts_before_pushing() {
        // Pushing on top of a stale view of the team's history is exactly the
        // situation sync exists to prevent.
        let git = RecordingGit::failing_on("fetch");
        let plan = plan_sync(&cfg(true), SyncOptions::default()).unwrap();
        assert!(run_sync(&git, &plan).is_err());
        assert_eq!(
            git.calls(),
            vec!["fetch"],
            "nothing may run after a failure"
        );
    }

    #[test]
    fn a_failed_fast_forward_aborts_before_pushing() {
        let git = RecordingGit::failing_on("fast_forward");
        let plan = plan_sync(&cfg(true), SyncOptions::default()).unwrap();
        assert!(run_sync(&git, &plan).is_err());
        assert_eq!(git.calls(), vec!["fetch", "fast_forward"]);
    }

    #[test]
    fn nothing_to_push_is_reported_rather_than_faked() {
        let git = RecordingGit::with_nothing_to_push();
        let plan = plan_sync(&cfg(true), SyncOptions::default()).unwrap();
        let report = run_sync(&git, &plan).expect("run");
        assert!(report.nothing_to_push);
        assert!(!report.executed.contains(&SyncStep::Push));
        assert_eq!(git.calls(), vec!["fetch", "fast_forward"]);
    }

    #[test]
    fn a_consume_only_run_never_calls_push() {
        let git = RecordingGit::new();
        let plan = plan_sync(
            &cfg(true),
            SyncOptions {
                no_push: true,
                ..Default::default()
            },
        )
        .unwrap();
        run_sync(&git, &plan).expect("run");
        assert_eq!(git.calls(), vec!["fetch", "fast_forward"]);
    }
}
