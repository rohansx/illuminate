//! Gated synthesis step for `illuminate ask --synthesize` (v3.3).
//!
//! This module is degrade-only: it assembles a *deterministic* synthesis prompt
//! from the retrieval [`AskResponse`] and detects whether an LLM provider is
//! configured purely from the environment. When none is configured the caller
//! prints the `synthesis unavailable` notice and exits 0 — no network call is
//! ever made here. Keeping it separate from `ask.rs` keeps both files small and
//! lets the prompt assembly be unit-tested in isolation.

use std::io::Write;

use super::ask::AskResponse;

/// Detect a configured LLM provider purely from the environment. Returns the
/// provider name when one is configured, `None` otherwise. This is the ONLY
/// gate the degrade path consults — no network probe is ever performed.
pub(crate) fn llm_provider() -> Option<String> {
    // An explicit provider selection wins.
    if let Ok(p) = std::env::var("ILLUMINATE_LLM_PROVIDER")
        && !p.trim().is_empty()
    {
        return Some(p.trim().to_string());
    }
    // Otherwise infer from a known API-key env var.
    if std::env::var("ANTHROPIC_API_KEY").is_ok_and(|v| !v.trim().is_empty()) {
        return Some("anthropic".to_string());
    }
    if std::env::var("OPENAI_API_KEY").is_ok_and(|v| !v.trim().is_empty()) {
        return Some("openai".to_string());
    }
    None
}

/// Build the deterministic synthesis prompt from a retrieval [`AskResponse`].
///
/// Pure and side-effect-free: the same `AskResponse` always yields a
/// byte-identical prompt. The prompt embeds the original question and, for each
/// non-empty kind (in the response's already-stable hit order), the hit titles
/// and snippets grouped under their section heading — exactly the context an
/// LLM rewrite step would consume. No network, no clock, no randomness.
pub(crate) fn assemble_synthesis_prompt(resp: &AskResponse) -> String {
    let mut p = String::new();
    p.push_str(
        "You are illuminate's synthesis step. Using ONLY the retrieved context \
         below, write a concise, well-grounded answer to the question. Cite the \
         hit titles you rely on; do not invent facts that are absent from the \
         context.\n\n",
    );
    p.push_str(&format!("Question: {}\n\n", resp.question));
    p.push_str("Retrieved context:\n");

    if resp.hits.is_empty() {
        p.push_str("(no matching context was retrieved)\n");
    } else {
        let mut last_kind = None;
        for h in &resp.hits {
            if Some(h.kind) != last_kind {
                p.push_str(&format!("\n## {}\n", h.kind.heading()));
                last_kind = Some(h.kind);
            }
            p.push_str(&format!("- {}\n  {}\n", h.title, h.snippet));
        }
    }
    p
}

/// Render the clearly-marked degrade notice. Printed AFTER the retrieval report
/// so that report is never lost; the command still exits 0.
pub(crate) fn render_synthesis_unavailable<W: Write>(out: &mut W) -> std::io::Result<()> {
    writeln!(
        out,
        "\n── synthesis unavailable (no LLM provider configured) ──"
    )?;
    writeln!(
        out,
        "Set ANTHROPIC_API_KEY / OPENAI_API_KEY (or ILLUMINATE_LLM_PROVIDER) to \
         enable the synthesis step. The retrieval report above is unchanged."
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::ask::{Hit, HitKind};

    fn hit(kind: HitKind, id: &str, title: &str, snippet: &str) -> Hit {
        Hit {
            kind,
            id: id.to_string(),
            title: title.to_string(),
            snippet: snippet.to_string(),
            source: Some(format!("wiki:{id}")),
            score_bucket: "high".to_string(),
        }
    }

    fn sample_response() -> AskResponse {
        AskResponse {
            question: "How do we store billing data?".to_string(),
            hits: vec![
                hit(
                    HitKind::Decision,
                    "dec-use-postgres",
                    "Use Postgres for the billing service",
                    "Chose Postgres over MongoDB after a vendor review.",
                ),
                hit(
                    HitKind::Failure,
                    "fail-cache-stampede",
                    "Cache stampede on cold start",
                    "No jitter on the TTL so every key expired at once.",
                ),
            ],
            hit_count: 2,
            empty_kinds: vec![],
        }
    }

    #[test]
    fn synthesis_prompt_embeds_question_and_a_hit_title() {
        let resp = sample_response();
        let prompt = assemble_synthesis_prompt(&resp);
        assert!(
            prompt.contains("How do we store billing data?"),
            "prompt must embed the question; got: {prompt}"
        );
        assert!(
            prompt.contains("Use Postgres for the billing service"),
            "prompt must embed at least one hit title; got: {prompt}"
        );
    }

    #[test]
    fn synthesis_prompt_is_byte_identical_across_calls() {
        let resp = sample_response();
        let a = assemble_synthesis_prompt(&resp);
        let b = assemble_synthesis_prompt(&resp);
        assert_eq!(a, b, "synthesis prompt must be deterministic");
    }

    #[test]
    fn synthesis_prompt_groups_by_kind_heading() {
        let resp = sample_response();
        let prompt = assemble_synthesis_prompt(&resp);
        assert!(
            prompt.contains("## Decisions") && prompt.contains("## Failures"),
            "prompt must group hits under their kind headings; got: {prompt}"
        );
    }

    #[test]
    fn synthesis_prompt_handles_empty_hits() {
        let resp = AskResponse {
            question: "anything?".to_string(),
            hits: vec![],
            hit_count: 0,
            empty_kinds: vec![],
        };
        let prompt = assemble_synthesis_prompt(&resp);
        assert!(prompt.contains("anything?"));
        assert!(prompt.contains("no matching context"));
    }
}
