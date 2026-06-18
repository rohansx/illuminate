//! Rhai-based policy engine — illuminate's **gatekeeper**.
//!
//! Where the audit linter *advises* (surfaces past decisions/failures) and the
//! graph *remembers*, this engine *decides*: it parses `.rhai` policy files
//! (lines of `<verb> if <expr>;`) and evaluates them in **deny → ask → allow**
//! order. The first matching rule in priority order wins; if no rule matches,
//! the default decision is `Ask` — never a silent allow.
//!
//! Ported from `homn-policy` (the author's own Apache-2.0 project), relicensed
//! MIT for inclusion here. The daemon-only hot-reload watcher is intentionally
//! omitted — illuminate evaluates cold per tool-call (no resident process).

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use rhai::Engine as RhaiEngine;
use serde::{Deserialize, Serialize};

mod parse;

pub use parse::{ParseError, RuleSet};
/// Re-exported so callers can register extra helpers via [`Engine::with_helpers`]
/// without taking a direct `rhai` dependency.
pub use rhai;

/// The decision a policy reaches for a tool call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Decision {
    /// Block the call outright.
    Deny,
    /// Defer to the human (the agent prompts natively).
    Ask,
    /// Permit the call silently.
    Allow,
}

impl Decision {
    /// Lowercase verb (`"deny"` / `"ask"` / `"allow"`).
    pub fn as_str(&self) -> &'static str {
        match self {
            Decision::Deny => "deny",
            Decision::Ask => "ask",
            Decision::Allow => "allow",
        }
    }
}

/// Where a rule lives — `file:line` — for explainability and the audit ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuleSourceLocation {
    /// The policy file the rule came from.
    pub file: PathBuf,
    /// 1-indexed line number.
    pub line: u32,
}

/// The outcome of a single policy evaluation.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    /// The decision the engine reached.
    pub decision: Decision,
    /// File + line of the rule that fired, if any.
    pub rule: Option<RuleSourceLocation>,
    /// Snapshot of the rule's text (for retro-readability in the audit ledger).
    pub rule_text: Option<String>,
    /// Human-readable reason from the rule's `=> "…"` suffix, if any. Surfaced
    /// to the agent (e.g. the hook's `permissionDecisionReason`) so a decision
    /// explains itself rather than echoing raw rule source.
    pub message: Option<String>,
}

impl Outcome {
    /// Build the "no match → default ask" outcome.
    pub fn default_ask() -> Self {
        Self {
            decision: Decision::Ask,
            rule: None,
            rule_text: None,
            message: None,
        }
    }
}

/// One rule's contribution to a [`Trace`].
#[derive(Debug, Clone, PartialEq)]
pub struct RuleTrace {
    /// The decision this rule produces when it fires.
    pub verb: Decision,
    /// File + line of the rule.
    pub location: RuleSourceLocation,
    /// The rule's full source text.
    pub source_text: String,
    /// Whether the rule matched this request.
    pub matched: bool,
    /// Whether this rule decided the outcome — the first match in
    /// deny → ask → allow order. At most one rule in a trace is decisive.
    pub decisive: bool,
}

/// A full evaluation trace: every rule, in evaluation order, plus the outcome.
#[derive(Debug, Clone)]
pub struct Trace {
    /// Every rule, evaluated in deny → ask → allow order.
    pub rules: Vec<RuleTrace>,
    /// The decision the engine reached.
    pub outcome: Outcome,
}

/// Per-tool-call evaluation context, bound into Rhai's scope.
#[derive(Debug, Clone, Default)]
pub struct EvalRequest {
    /// Tool name (e.g. `"Bash"`).
    pub tool: String,
    /// For `Bash`: the command. Empty otherwise.
    pub cmd: String,
    /// For `Read` / `Edit` / `Write`: the file path. Empty otherwise.
    pub path: String,
    /// For `WebFetch`: the URL. Empty otherwise.
    pub url: String,
    /// Working directory of the calling session.
    pub cwd: String,
    /// `$HOME`.
    pub home: String,
    /// Session id from the host agent.
    pub session_id: String,
}

impl EvalRequest {
    /// Build a request from a host-agent `tool_name` + `tool_input` JSON object,
    /// populating `cmd` / `path` / `url` from the conventional fields.
    pub fn from_tool_call(tool_name: &str, tool_input: &serde_json::Value, cwd: &str) -> Self {
        let field = |k: &str| {
            tool_input
                .get(k)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned()
        };
        // Read/Edit/Write use `file_path` in Claude Code; accept `path` too.
        let path = {
            let p = field("file_path");
            if p.is_empty() { field("path") } else { p }
        };
        Self {
            tool: tool_name.to_owned(),
            cmd: field("command"),
            path,
            url: field("url"),
            cwd: cwd.to_owned(),
            home: std::env::var("HOME").unwrap_or_default(),
            session_id: String::new(),
        }
    }
}

/// The policy engine. Wraps a single Rhai engine with sandbox limits + the
/// `matches` (glob) and `regex` helpers.
#[derive(Clone)]
pub struct Engine {
    inner: std::sync::Arc<RhaiEngine>,
}

impl Engine {
    /// Build an engine with default sandbox limits + the built-in `matches` /
    /// `regex` helpers.
    pub fn new() -> Self {
        Self::with_helpers(|_| {})
    }

    /// Like [`new`](Self::new) but lets the caller register additional Rhai
    /// helper functions before the engine is sealed — e.g. graph-backed
    /// predicates (`recently_edited(path)`, `decisions_referencing(entity)`) so
    /// policy rules can reason over what illuminate knows. The CLI wires those
    /// in (keeping `illuminate-policy` free of any `illuminate-core` dependency).
    pub fn with_helpers<F: FnOnce(&mut RhaiEngine)>(extra: F) -> Self {
        let mut inner = RhaiEngine::new();
        inner.set_max_operations(100_000);
        inner.set_max_call_levels(32);
        inner.set_max_string_size(8 * 1024);
        inner.set_max_array_size(1024);
        inner.set_max_modules(8);
        inner.set_max_expr_depths(64, 64);
        register_helpers(&mut inner);
        extra(&mut inner);
        Self {
            inner: std::sync::Arc::new(inner),
        }
    }

    /// Borrow the underlying Rhai engine (used by `parse::compile_rule`).
    pub(crate) fn rhai(&self) -> &RhaiEngine {
        &self.inner
    }

    /// Evaluate a [`RuleSet`] against a request in deny → ask → allow order;
    /// first matching rule wins. No match → [`Outcome::default_ask`]. Rules that
    /// fail to evaluate (e.g. operation budget exhausted) are treated as
    /// non-matches.
    pub fn eval(&self, rules: &RuleSet, req: &EvalRequest) -> Outcome {
        for rule in rules.deny_rules() {
            if self.fires(rule, req) {
                return self.outcome(Decision::Deny, rule);
            }
        }
        for rule in rules.ask_rules() {
            if self.fires(rule, req) {
                return self.outcome(Decision::Ask, rule);
            }
        }
        for rule in rules.allow_rules() {
            if self.fires(rule, req) {
                return self.outcome(Decision::Allow, rule);
            }
        }
        Outcome::default_ask()
    }

    /// Like [`eval`](Self::eval) but evaluates *every* rule (no short-circuit) so
    /// a human can see exactly what did and didn't fire — backs `explain`.
    pub fn trace(&self, rules: &RuleSet, req: &EvalRequest) -> Trace {
        let ordered = rules
            .deny_rules()
            .map(|r| (Decision::Deny, r))
            .chain(rules.ask_rules().map(|r| (Decision::Ask, r)))
            .chain(rules.allow_rules().map(|r| (Decision::Allow, r)));

        let mut traced = Vec::new();
        let mut outcome: Option<Outcome> = None;

        for (verb, rule) in ordered {
            let matched = self.fires(rule, req);
            let decisive = matched && outcome.is_none();
            if decisive {
                outcome = Some(self.outcome(verb, rule));
            }
            traced.push(RuleTrace {
                verb,
                location: RuleSourceLocation {
                    file: PathBuf::from(rule.file_name()),
                    line: rule.line(),
                },
                source_text: rule.source_text().to_owned(),
                matched,
                decisive,
            });
        }

        Trace {
            rules: traced,
            outcome: outcome.unwrap_or_else(Outcome::default_ask),
        }
    }

    fn fires(&self, rule: &parse::CompiledRule, req: &EvalRequest) -> bool {
        let mut scope = rhai::Scope::new();
        scope.push_constant("tool", req.tool.clone());
        scope.push_constant("cmd", req.cmd.clone());
        scope.push_constant("path", req.path.clone());
        scope.push_constant("url", req.url.clone());
        scope.push_constant("cwd", req.cwd.clone());
        scope.push_constant("home", req.home.clone());
        scope.push_constant("session_id", req.session_id.clone());

        match self
            .inner
            .eval_ast_with_scope::<bool>(&mut scope, rule.ast())
        {
            Ok(b) => b,
            Err(err) => {
                // A rule that can't evaluate is a non-match (fail safe — an
                // ask/deny is never silently downgraded by a broken rule).
                eprintln!(
                    "illuminate-policy: rule {}:{} failed to evaluate ({err}); treating as non-match",
                    rule.file_name(),
                    rule.line()
                );
                false
            }
        }
    }

    fn outcome(&self, decision: Decision, rule: &parse::CompiledRule) -> Outcome {
        Outcome {
            decision,
            rule: Some(RuleSourceLocation {
                file: PathBuf::from(rule.file_name()),
                line: rule.line(),
            }),
            rule_text: Some(rule.source_text().to_owned()),
            message: rule.message().map(str::to_owned),
        }
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

/// The bundled default policy (conservative deny → ask → allow). Used when a
/// repo has no `.illuminate/policy.rhai` of its own.
pub const DEFAULT_POLICY: &str = include_str!("default.rhai");

/// Parse the bundled [`DEFAULT_POLICY`] with a fresh engine.
pub fn default_ruleset(engine: &Engine) -> RuleSet {
    RuleSet::parse(engine, DEFAULT_POLICY, "default.rhai")
        .expect("bundled default.rhai must always parse")
}

/// The minimalism layer — `ask` rules that pause before an agent reaches for a
/// new dependency, embodying ponytail's "the best code is the code you never
/// write" ladder. Stacks *on top of* [`DEFAULT_POLICY`] (see
/// [`minimalist_policy_source`]); since rules group by verb, these `ask`s beat
/// the default's broad `allow` on package managers.
pub const MINIMALIST_POLICY: &str = include_str!("minimalist.rhai");

/// A complete, ready-to-use minimalist policy: the conservative
/// [`DEFAULT_POLICY`] plus the [`MINIMALIST_POLICY`] dependency-restraint layer.
/// Written verbatim by `illuminate policy template minimalist`.
pub fn minimalist_policy_source() -> String {
    format!("{DEFAULT_POLICY}\n{MINIMALIST_POLICY}")
}

/// Register the custom helpers: `matches` (glob) and `regex` (RE2-flavoured).
fn register_helpers(engine: &mut RhaiEngine) {
    engine.register_fn("matches", |s: &str, pattern: &str| glob_match(s, pattern));
    engine.register_fn("matches", |s: String, pattern: &str| {
        glob_match(&s, pattern)
    });
    engine.register_fn("regex", |s: &str, pattern: &str| -> bool {
        regex::Regex::new(pattern)
            .map(|r| r.is_match(s))
            .unwrap_or(false)
    });
    engine.register_fn("regex", |s: String, pattern: &str| -> bool {
        regex::Regex::new(pattern)
            .map(|r| r.is_match(&s))
            .unwrap_or(false)
    });
}

/// Shell-style glob: `*` matches any chars, `?` matches one char, else literal.
fn glob_match(text: &str, pattern: &str) -> bool {
    let mut regex_src = String::from("^");
    for ch in pattern.chars() {
        match ch {
            '*' => regex_src.push_str(".*"),
            '?' => regex_src.push('.'),
            c if "\\.+()[]{}|^$".contains(c) => {
                regex_src.push('\\');
                regex_src.push(c);
            }
            c => regex_src.push(c),
        }
    }
    regex_src.push('$');
    regex::Regex::new(&regex_src)
        .map(|r| r.is_match(text))
        .unwrap_or(false)
}

/// Load a [`RuleSet`] from disk using a default [`Engine`].
pub fn load_ruleset(path: impl AsRef<Path>) -> Result<RuleSet, ParseError> {
    RuleSet::load(&Engine::new(), path.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(tool: &str, cmd: &str) -> EvalRequest {
        EvalRequest {
            tool: tool.into(),
            cmd: cmd.into(),
            home: "/home/rsx".into(),
            cwd: "/home/rsx/dev/x".into(),
            ..Default::default()
        }
    }

    fn ruleset(engine: &Engine, src: &str) -> RuleSet {
        RuleSet::parse(engine, src, "test.rhai").expect("ruleset parses")
    }

    #[test]
    fn glob_match_handles_star_and_question() {
        assert!(glob_match("git push origin main", "git push * main"));
        assert!(glob_match("npm run build", "npm run *"));
        assert!(!glob_match("cargo build", "npm *"));
        assert!(glob_match("a", "?"));
        assert!(!glob_match("ab", "?"));
    }

    #[test]
    fn empty_ruleset_yields_default_ask() {
        let eng = Engine::new();
        let rs = ruleset(&eng, "");
        let out = eng.eval(&rs, &req("Bash", "ls"));
        assert_eq!(out.decision, Decision::Ask);
        assert!(out.rule.is_none());
    }

    #[test]
    fn deny_beats_allow_in_priority_order() {
        let eng = Engine::new();
        let src = r#"
            allow if tool == "Bash";
            deny if tool == "Bash" && cmd.contains("rm -rf");
        "#;
        let rs = ruleset(&eng, src);
        let out = eng.eval(&rs, &req("Bash", "rm -rf ~/scratch"));
        assert_eq!(out.decision, Decision::Deny);
        assert_eq!(out.rule.unwrap().line, 3);
    }

    #[test]
    fn no_match_falls_through_to_ask() {
        let eng = Engine::new();
        let rs = ruleset(&eng, r#"allow if tool == "Read";"#);
        let out = eng.eval(&rs, &req("WebFetch", ""));
        assert_eq!(out.decision, Decision::Ask);
        assert!(out.rule.is_none());
    }

    #[test]
    fn helpers_work_inside_rules() {
        let eng = Engine::new();
        let rs = ruleset(
            &eng,
            r#"allow if tool == "Bash" && cmd.regex("^git (status|log)( |$)");"#,
        );
        assert_eq!(
            eng.eval(&rs, &req("Bash", "git status")).decision,
            Decision::Allow
        );
        let rs2 = ruleset(
            &eng,
            r#"allow if tool == "Bash" && cmd.matches("npm run *");"#,
        );
        assert_eq!(
            eng.eval(&rs2, &req("Bash", "npm run build")).decision,
            Decision::Allow
        );
    }

    #[test]
    fn trace_marks_exactly_one_decisive_rule() {
        let eng = Engine::new();
        let src = r#"
            allow if tool == "Bash";
            deny if tool == "Bash" && cmd.contains("rm -rf");
        "#;
        let rs = ruleset(&eng, src);
        let trace = eng.trace(&rs, &req("Bash", "rm -rf /x"));
        assert_eq!(trace.outcome.decision, Decision::Deny);
        assert_eq!(trace.rules.iter().filter(|r| r.decisive).count(), 1);
        // the allow rule matched but lost on priority
        let allow = trace
            .rules
            .iter()
            .find(|r| r.verb == Decision::Allow)
            .unwrap();
        assert!(allow.matched && !allow.decisive);
    }

    #[test]
    fn default_policy_parses_and_gates_the_dev_loop() {
        let eng = Engine::new();
        let rs = default_ruleset(&eng);
        assert!(!rs.is_empty());
        // normal dev loop → allow
        assert_eq!(
            eng.eval(&rs, &req("Bash", "cargo build --release"))
                .decision,
            Decision::Allow
        );
        assert_eq!(eng.eval(&rs, &req("Read", "")).decision, Decision::Allow);
        // dangerous → deny
        let mut danger = req("Bash", "rm -rf /etc");
        danger.cwd = "/home/rsx/dev/x".into();
        assert_eq!(eng.eval(&rs, &danger).decision, Decision::Deny);
        // outward → ask
        assert_eq!(
            eng.eval(&rs, &req("Bash", "git push origin main")).decision,
            Decision::Ask
        );
        // genuinely unknown → default ask (no rule)
        let out = eng.eval(&rs, &req("Bash", "frobnicate the widget"));
        assert_eq!(out.decision, Decision::Ask);
        assert!(out.rule.is_none());
    }

    #[test]
    fn decision_serializes_lowercase() {
        assert_eq!(serde_json::to_string(&Decision::Deny).unwrap(), "\"deny\"");
    }

    #[test]
    fn outcome_carries_rule_message() {
        let eng = Engine::new();
        let rs = ruleset(
            &eng,
            r#"ask if tool == "Bash" && cmd.contains("danger") => "explain yourself";"#,
        );
        let out = eng.eval(&rs, &req("Bash", "do danger now"));
        assert_eq!(out.decision, Decision::Ask);
        assert_eq!(out.message.as_deref(), Some("explain yourself"));
    }

    #[test]
    fn minimalist_policy_parses_and_gates_dependency_growth() {
        let eng = Engine::new();
        let rs = RuleSet::parse(&eng, &minimalist_policy_source(), "minimalist.rhai")
            .expect("composed minimalist policy must parse");

        // Adding a new dependency → ask, with the ladder message.
        for (cmd, what) in [
            ("npm install left-pad", "npm add"),
            ("npm i react", "npm i add"),
            ("yarn add lodash", "yarn add"),
            ("pnpm add zod", "pnpm add"),
            ("cargo add serde", "cargo add"),
            ("pip install requests", "pip add"),
            ("go get github.com/x/y", "go get"),
            ("npx create-react-app foo", "scaffolder"),
        ] {
            let out = eng.eval(&rs, &req("Bash", cmd));
            assert_eq!(out.decision, Decision::Ask, "{what}: `{cmd}` should ask");
            assert!(out.message.is_some(), "{what}: ask should carry a message");
        }

        // Syncing from an existing manifest adds nothing → still allowed.
        // (npm/cargo/yarn are in the default allow-list, so they stay allow.)
        for cmd in ["npm install", "npm ci", "cargo build --release", "yarn"] {
            assert_eq!(
                eng.eval(&rs, &req("Bash", cmd)).decision,
                Decision::Allow,
                "`{cmd}` syncs an existing manifest — must stay allowed"
            );
        }

        // The `[^-\s]` discrimination: a flag-led manifest sync must NOT trip
        // the "new dependency" ask. (pip isn't in the default allow-list, so it
        // is a default ask either way — the proof is the ABSENCE of a message.)
        let sync = eng.eval(&rs, &req("Bash", "pip install -r requirements.txt"));
        assert!(
            sync.message.is_none(),
            "pip -r manifest sync must not carry the new-dependency message"
        );

        // The conservative default still holds underneath.
        let mut danger = req("Bash", "rm -rf /etc");
        danger.cwd = "/home/rsx/dev/x".into();
        assert_eq!(eng.eval(&rs, &danger).decision, Decision::Deny);
    }
}
