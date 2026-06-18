//! Parser for the illuminate policy DSL.
//!
//! Grammar (one rule per non-empty, non-comment line):
//!
//! ```text
//! <verb> if <expression> [=> "message"];
//! ```
//!
//! `<verb>` ∈ {`allow`, `deny`, `ask`}. `<expression>` is a Rhai boolean
//! expression compiled once and reused. An optional `=> "message"` suffix
//! attaches a human-readable reason that surfaces to the agent when the rule
//! fires (e.g. the policy hook's `permissionDecisionReason`); `=>` is chosen as
//! the delimiter because it can't appear in a valid Rhai boolean expression
//! outside a string literal. Comments start with `//`. Blank / comment-only
//! lines are skipped.
//!
//! Ported from `homn-policy` (the author's own project), relicensed MIT.

use std::path::{Path, PathBuf};

use rhai::AST;

use crate::Engine;

/// Errors that can occur while parsing a policy file.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    /// Failed to read the file from disk.
    #[error("io error reading {path}: {source}")]
    Io {
        /// Path that failed.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// A rule line doesn't match the expected `<verb> if <expr>;` shape.
    #[error("{file}:{line}: malformed rule: {message}")]
    Malformed {
        /// File name (display only).
        file: String,
        /// 1-indexed line number.
        line: u32,
        /// What went wrong.
        message: String,
    },
    /// The expression after `if` doesn't compile as Rhai.
    #[error("{file}:{line}: rhai compile error: {message}")]
    BadExpression {
        /// File name (display only).
        file: String,
        /// 1-indexed line number.
        line: u32,
        /// The error from Rhai.
        message: String,
    },
}

/// One compiled rule.
#[derive(Debug)]
pub struct CompiledRule {
    verb: Verb,
    file_name: String,
    line: u32,
    source_text: String,
    message: Option<String>,
    ast: AST,
}

impl CompiledRule {
    /// Borrow the compiled expression AST.
    pub fn ast(&self) -> &AST {
        &self.ast
    }
    /// Display-friendly file name.
    pub fn file_name(&self) -> &str {
        &self.file_name
    }
    /// 1-indexed line number.
    pub fn line(&self) -> u32 {
        self.line
    }
    /// The full source text of the rule (verb + expr + any message), for
    /// audit-log snapshots.
    pub fn source_text(&self) -> &str {
        &self.source_text
    }
    /// The human-readable message from an `=> "…"` suffix, if any. Surfaced to
    /// the agent as the reason a rule fired.
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verb {
    Allow,
    Deny,
    Ask,
}

/// A parsed, compiled ruleset, grouped by verb for evaluation order.
#[derive(Debug)]
pub struct RuleSet {
    deny_rules: Vec<CompiledRule>,
    ask_rules: Vec<CompiledRule>,
    allow_rules: Vec<CompiledRule>,
}

impl RuleSet {
    /// Parse a ruleset from in-memory source.
    pub fn parse(engine: &Engine, source: &str, file_name: &str) -> Result<Self, ParseError> {
        let mut deny = Vec::new();
        let mut ask = Vec::new();
        let mut allow = Vec::new();

        for (idx, raw_line) in source.lines().enumerate() {
            let line_no = (idx + 1) as u32;
            let line = strip_comment(raw_line).trim();
            if line.is_empty() {
                continue;
            }
            let rule = parse_rule(engine, line, file_name, line_no)?;
            match rule.verb {
                Verb::Deny => deny.push(rule),
                Verb::Ask => ask.push(rule),
                Verb::Allow => allow.push(rule),
            }
        }

        Ok(Self {
            deny_rules: deny,
            ask_rules: ask,
            allow_rules: allow,
        })
    }

    /// Load and parse a ruleset from disk.
    pub fn load(engine: &Engine, path: &Path) -> Result<Self, ParseError> {
        let source = std::fs::read_to_string(path).map_err(|source| ParseError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        Self::parse(engine, &source, &name)
    }

    /// Rules with verb `deny`, in source order.
    pub fn deny_rules(&self) -> impl Iterator<Item = &CompiledRule> {
        self.deny_rules.iter()
    }
    /// Rules with verb `ask`, in source order.
    pub fn ask_rules(&self) -> impl Iterator<Item = &CompiledRule> {
        self.ask_rules.iter()
    }
    /// Rules with verb `allow`, in source order.
    pub fn allow_rules(&self) -> impl Iterator<Item = &CompiledRule> {
        self.allow_rules.iter()
    }

    /// Total compiled rule count across all verbs.
    pub fn len(&self) -> usize {
        self.deny_rules.len() + self.ask_rules.len() + self.allow_rules.len()
    }

    /// Whether the ruleset has no rules.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

fn strip_comment(line: &str) -> &str {
    // Strip a trailing `// …` comment, but keep `//` that appears inside a
    // double-quoted string literal intact. Required because users embed `//`
    // inside regex patterns and URLs — e.g. `url.regex("^https?://...")`.
    let bytes = line.as_bytes();
    let mut in_string = false;
    let mut escape = false;
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if escape {
            escape = false;
        } else if in_string && c == b'\\' {
            escape = true;
        } else if c == b'"' {
            in_string = !in_string;
        } else if !in_string && c == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            return &line[..i];
        }
        i += 1;
    }
    line
}

fn parse_rule(
    engine: &Engine,
    line: &str,
    file_name: &str,
    line_no: u32,
) -> Result<CompiledRule, ParseError> {
    let trimmed = line.trim_end_matches(';').trim();

    let (verb_word, rest) = trimmed
        .split_once(' ')
        .ok_or_else(|| ParseError::Malformed {
            file: file_name.to_owned(),
            line: line_no,
            message: "expected '<verb> if <expression>'".to_owned(),
        })?;

    let verb = match verb_word {
        "allow" => Verb::Allow,
        "deny" => Verb::Deny,
        "ask" => Verb::Ask,
        other => {
            return Err(ParseError::Malformed {
                file: file_name.to_owned(),
                line: line_no,
                message: format!("unknown verb `{other}` (expected allow/deny/ask)"),
            });
        }
    };

    let rest = rest.trim_start();
    let expr = rest
        .strip_prefix("if")
        .ok_or_else(|| ParseError::Malformed {
            file: file_name.to_owned(),
            line: line_no,
            message: "expected `if` after verb".to_owned(),
        })?;
    // Peel off an optional `=> "message"` suffix before compiling the boolean
    // expression. `=>` is unambiguous: it can't appear in a valid Rhai boolean
    // expression except inside a string literal, which `split_message` skips.
    let (expr, message) = split_message(expr.trim());
    let expr = expr.trim();
    if expr.is_empty() {
        return Err(ParseError::Malformed {
            file: file_name.to_owned(),
            line: line_no,
            message: "missing expression after `if`".to_owned(),
        });
    }

    let ast = engine
        .rhai()
        .compile_expression(expr)
        .map_err(|err| ParseError::BadExpression {
            file: file_name.to_owned(),
            line: line_no,
            message: err.to_string(),
        })?;

    Ok(CompiledRule {
        verb,
        file_name: file_name.to_owned(),
        line: line_no,
        source_text: line.to_owned(),
        message,
        ast,
    })
}

/// Split a rule body into `(boolean_expr, message)` at the first top-level `=>`
/// (one that is NOT inside a double-quoted string). The message is the text
/// after `=>`; surrounding double quotes are stripped and `\"`/`\\` unescaped.
/// With no `=>`, the whole input is the expression and the message is `None`.
fn split_message(body: &str) -> (&str, Option<String>) {
    let bytes = body.as_bytes();
    let mut in_string = false;
    let mut escape = false;
    let mut i = 0;
    while i + 1 < bytes.len() {
        let c = bytes[i];
        if escape {
            escape = false;
        } else if in_string && c == b'\\' {
            escape = true;
        } else if c == b'"' {
            in_string = !in_string;
        } else if !in_string && c == b'=' && bytes[i + 1] == b'>' {
            let expr = &body[..i];
            let msg = unquote(body[i + 2..].trim());
            return (expr, Some(msg));
        }
        i += 1;
    }
    (body, None)
}

/// Strip surrounding double quotes from a message literal and unescape `\"` /
/// `\\`. A bare (unquoted) message is returned trimmed and verbatim.
fn unquote(s: &str) -> String {
    let inner = s
        .strip_prefix('"')
        .and_then(|t| t.strip_suffix('"'))
        .unwrap_or(s);
    inner.replace("\\\"", "\"").replace("\\\\", "\\")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_rule() {
        let eng = Engine::new();
        let rs = RuleSet::parse(&eng, "allow if tool == \"Bash\";", "test").unwrap();
        assert_eq!(rs.deny_rules().count(), 0);
        assert_eq!(rs.ask_rules().count(), 0);
        assert_eq!(rs.allow_rules().count(), 1);
        assert_eq!(rs.len(), 1);
    }

    #[test]
    fn comments_and_blank_lines_are_skipped() {
        let eng = Engine::new();
        let src = "// header\n\n  // indented\nallow if tool == \"x\";\n";
        let rs = RuleSet::parse(&eng, src, "t").unwrap();
        assert_eq!(rs.allow_rules().count(), 1);
    }

    #[test]
    fn unknown_verb_is_rejected_with_line_number() {
        let eng = Engine::new();
        let src = "allow if tool == \"a\";\nyolo if tool == \"b\";\n";
        let err = RuleSet::parse(&eng, src, "t").unwrap_err();
        match err {
            ParseError::Malformed { line, message, .. } => {
                assert_eq!(line, 2);
                assert!(message.contains("yolo"), "got: {message}");
            }
            other => panic!("expected Malformed, got {other:?}"),
        }
    }

    #[test]
    fn bad_expression_is_rejected_with_line_number() {
        let eng = Engine::new();
        let src = "allow if tool == ;";
        let err = RuleSet::parse(&eng, src, "t").unwrap_err();
        match err {
            ParseError::BadExpression { line, .. } => assert_eq!(line, 1),
            other => panic!("expected BadExpression, got {other:?}"),
        }
    }

    #[test]
    fn comment_stripper_preserves_double_slash_inside_strings() {
        let eng = Engine::new();
        let src =
            r#"ask if tool == "WebFetch" && url.regex("^https?://prod\\.");  // trailing comment"#;
        let rs = RuleSet::parse(&eng, src, "t").expect("rule with // inside string must parse");
        assert_eq!(rs.ask_rules().count(), 1);
    }

    #[test]
    fn rule_without_message_has_none() {
        let eng = Engine::new();
        let rs = RuleSet::parse(&eng, r#"ask if tool == "Bash";"#, "t").unwrap();
        let rule = rs.ask_rules().next().unwrap();
        assert_eq!(rule.message(), None);
    }

    #[test]
    fn rule_with_message_captures_it_and_still_compiles_expr() {
        let eng = Engine::new();
        let src = r#"ask if tool == "Bash" && cmd.regex("cargo\\s+add") => "new dependency — stdlib first?";"#;
        let rs = RuleSet::parse(&eng, src, "t").unwrap();
        let rule = rs.ask_rules().next().unwrap();
        assert_eq!(rule.message(), Some("new dependency — stdlib first?"));
    }

    #[test]
    fn arrow_inside_a_string_is_not_a_message_delimiter() {
        // A literal `=>` inside the regex must NOT be treated as the suffix.
        let eng = Engine::new();
        let src = r#"deny if tool == "Bash" && cmd.regex("a=>b") => "blocked";"#;
        let rs = RuleSet::parse(&eng, src, "t").unwrap();
        let rule = rs.deny_rules().next().unwrap();
        assert_eq!(rule.message(), Some("blocked"));
    }

    #[test]
    fn missing_if_is_rejected() {
        let eng = Engine::new();
        let src = "allow tool == \"x\";";
        let err = RuleSet::parse(&eng, src, "t").unwrap_err();
        match err {
            ParseError::Malformed { message, .. } => {
                assert!(message.contains("`if`"), "got: {message}");
            }
            other => panic!("expected Malformed, got {other:?}"),
        }
    }
}
