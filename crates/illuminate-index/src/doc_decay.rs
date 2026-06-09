//! Doc-decay detection (knowledge-layer v3.3).
//!
//! Deterministic, network-free scan that flags references in ingested markdown
//! docs to code symbols that no longer exist in the [`CodeIndex`]. A doc that
//! cites a function/type which was since deleted or renamed is a *stale*
//! reference — surfacing those keeps prose honest with the code.
//!
//! The scan is intentionally conservative to keep precision high: it only
//! considers inline-code spans (`` `like this` ``) and only treats a span as a
//! symbol reference when it is shaped like code (an identifier containing an
//! underscore or a CamelCase boundary, or a `path::Symbol` / `module::Symbol`
//! form). Plain backticked prose words (`` `note` ``, `` `the` ``) are ignored
//! so they never produce false positives.

use crate::indexer::CodeIndex;

/// A markdown reference to a code symbol that is absent from the index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaleRef {
    /// Doc file the reference appears in (as passed by the caller).
    pub file: String,
    /// 1-based line number of the reference within the doc.
    pub line: usize,
    /// The symbol name that could not be found in the index.
    pub symbol: String,
    /// The raw inline-code span the symbol was extracted from (for context).
    pub raw: String,
}

/// A candidate symbol reference extracted from a doc, before index lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocRef {
    /// 1-based line number.
    pub line: usize,
    /// Symbol name (the trailing segment for a `path::Symbol` form).
    pub symbol: String,
    /// File-path prefix when the reference was `path::Symbol` and the prefix
    /// looks like a source file (e.g. `src/widget.rs`); `None` otherwise.
    pub file_hint: Option<String>,
    /// The raw inline-code span.
    pub raw: String,
}

/// Extract candidate code-symbol references from markdown `text`.
///
/// Walks each line, ignores fenced code blocks (```` ``` ````-delimited), and
/// collects inline-code spans that look like code symbols. Pure and
/// deterministic — no IO, no allocation beyond the returned vector.
pub fn extract_doc_refs(text: &str) -> Vec<DocRef> {
    let mut refs = Vec::new();
    let mut in_fence = false;

    for (idx, line) in text.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }

        for span in inline_code_spans(line) {
            if let Some(doc_ref) = classify_span(span, line_no) {
                refs.push(doc_ref);
            }
        }
    }

    refs
}

/// Scan markdown `text` (from doc `file`) against `index`, returning one
/// [`StaleRef`] per referenced symbol that is absent from the index. Returns an
/// empty vector when every referenced symbol is present (a clean doc).
pub fn scan_markdown_against_index(file: &str, text: &str, index: &CodeIndex) -> Vec<StaleRef> {
    let mut stale = Vec::new();

    for doc_ref in extract_doc_refs(text) {
        if symbol_present(index, &doc_ref) {
            continue;
        }
        stale.push(StaleRef {
            file: file.to_string(),
            line: doc_ref.line,
            symbol: doc_ref.symbol,
            raw: doc_ref.raw,
        });
    }

    stale
}

/// True when `doc_ref`'s symbol can be resolved in the index.
///
/// For a `path::Symbol` reference whose prefix is a source file, membership is
/// checked via [`CodeIndex::lookup_file`] (the symbol must be defined in that
/// file). Otherwise existence is checked via [`CodeIndex::lookup_symbol`] with
/// an exact-name match (the underlying lookup is a prefix match, so we filter).
fn symbol_present(index: &CodeIndex, doc_ref: &DocRef) -> bool {
    if let Some(ref file) = doc_ref.file_hint
        && let Ok(syms) = index.lookup_file(file)
    {
        // A readable, indexed file: membership is authoritative. An
        // unreadable/missing file (Err) falls through to the name-based
        // lookup below rather than crashing.
        return syms.iter().any(|s| s.name == doc_ref.symbol);
    }

    match index.lookup_symbol(&doc_ref.symbol, 64) {
        Ok(syms) => syms.iter().any(|s| s.name == doc_ref.symbol),
        Err(_) => false,
    }
}

/// Yield the contents of every inline-code span (text between single
/// backticks) on `line`. Backticks inside a fenced block are handled by the
/// caller; this only splits a single line on `` ` ``.
fn inline_code_spans(line: &str) -> Vec<&str> {
    let mut spans = Vec::new();
    let mut rest = line;
    while let Some(open) = rest.find('`') {
        let after_open = &rest[open + 1..];
        match after_open.find('`') {
            Some(close) => {
                let span = &after_open[..close];
                if !span.is_empty() {
                    spans.push(span);
                }
                rest = &after_open[close + 1..];
            }
            None => break,
        }
    }
    spans
}

/// Classify an inline-code `span` into a [`DocRef`], or `None` when it does not
/// look like a code-symbol reference.
fn classify_span(span: &str, line: usize) -> Option<DocRef> {
    let span = span.trim();
    if span.is_empty() {
        return None;
    }

    // path::Symbol / module::Symbol form.
    if let Some(pos) = span.rfind("::") {
        let prefix = &span[..pos];
        let symbol = &span[pos + 2..];
        if !is_identifier(symbol) {
            return None;
        }
        let file_hint = if looks_like_source_path(prefix) {
            Some(prefix.to_string())
        } else {
            None
        };
        return Some(DocRef {
            line,
            symbol: symbol.to_string(),
            file_hint,
            raw: span.to_string(),
        });
    }

    // Bare identifier — only treat as a symbol reference when it is shaped
    // like code (snake_case or CamelCase) to avoid flagging prose words.
    if is_identifier(span) && is_code_like(span) {
        return Some(DocRef {
            line,
            symbol: span.to_string(),
            file_hint: None,
            raw: span.to_string(),
        });
    }

    None
}

/// True when `s` is a single identifier token: `[A-Za-z_][A-Za-z0-9_]*`.
fn is_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// True when an identifier looks like a code symbol rather than a prose word:
/// it contains an underscore, or has an internal/leading uppercase letter
/// (CamelCase / PascalCase). Plain lowercase words (`note`, `the`) are excluded.
fn is_code_like(s: &str) -> bool {
    if s.contains('_') {
        return true;
    }
    let mut chars = s.chars();
    let first_upper = chars
        .next()
        .map(|c| c.is_ascii_uppercase())
        .unwrap_or(false);
    let has_inner_upper = s.chars().skip(1).any(|c| c.is_ascii_uppercase());
    first_upper || has_inner_upper
}

/// True when `prefix` looks like a path to a source file (has a known
/// source-file extension), so it should be resolved via `lookup_file`.
fn looks_like_source_path(prefix: &str) -> bool {
    matches!(
        prefix.rsplit('.').next(),
        Some(
            "rs" | "go"
                | "ts"
                | "tsx"
                | "js"
                | "jsx"
                | "py"
                | "java"
                | "c"
                | "h"
                | "cpp"
                | "cc"
                | "cxx"
                | "hpp"
        )
    ) && prefix.contains('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_snake_case_inline_code() {
        let refs = extract_doc_refs("Call `make_widget` here.");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].symbol, "make_widget");
        assert_eq!(refs[0].line, 1);
        assert!(refs[0].file_hint.is_none());
    }

    #[test]
    fn ignores_plain_prose_words() {
        // `note` and `the` are lowercase prose, not code-like → not refs.
        let refs = extract_doc_refs("A `note` about `the` widget.");
        assert!(refs.is_empty(), "prose words must not be refs: {refs:?}");
    }

    #[test]
    fn extracts_camelcase_identifier() {
        let refs = extract_doc_refs("See `MakeWidget` and `widgetCount`.");
        let names: Vec<_> = refs.iter().map(|r| r.symbol.as_str()).collect();
        assert_eq!(names, vec!["MakeWidget", "widgetCount"]);
    }

    #[test]
    fn splits_path_qualified_form() {
        let refs = extract_doc_refs("Path: `src/widget.rs::make_widget`.");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].symbol, "make_widget");
        assert_eq!(refs[0].file_hint.as_deref(), Some("src/widget.rs"));
    }

    #[test]
    fn module_path_has_no_file_hint() {
        // `widget::make_widget` — module path, not a source file.
        let refs = extract_doc_refs("`widget::make_widget`");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].symbol, "make_widget");
        assert!(refs[0].file_hint.is_none(), "module path is not a file");
    }

    #[test]
    fn skips_fenced_code_blocks() {
        let doc = "intro `keep_me`\n```\nfn skip_me() {}\n`also_skip`\n```\noutro `keep_two`";
        let names: Vec<_> = extract_doc_refs(doc)
            .into_iter()
            .map(|r| r.symbol)
            .collect();
        assert_eq!(names, vec!["keep_me", "keep_two"]);
    }

    #[test]
    fn line_numbers_are_one_based() {
        let doc = "line one\n\nthird line has `deleted_fn`";
        let refs = extract_doc_refs(doc);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].line, 3);
    }

    #[test]
    fn unterminated_backtick_is_ignored() {
        let refs = extract_doc_refs("dangling `open_only without close");
        assert!(refs.is_empty(), "no closing backtick → no span: {refs:?}");
    }

    #[test]
    fn is_identifier_rejects_spaces_and_symbols() {
        assert!(is_identifier("make_widget"));
        assert!(is_identifier("Widget"));
        assert!(!is_identifier("make widget"));
        assert!(!is_identifier("3foo"));
        assert!(!is_identifier(""));
    }

    #[test]
    fn looks_like_source_path_detects_extensions() {
        assert!(looks_like_source_path("src/widget.rs"));
        assert!(looks_like_source_path("pkg/main.go"));
        assert!(!looks_like_source_path("widget"));
        assert!(!looks_like_source_path("crate::module"));
    }
}
