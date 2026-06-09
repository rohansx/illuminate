//! `--as-doc`: deterministically draft a design-doc markdown from a captured
//! session — **template-based, no LLM**.
//!
//! Stage 4's session-publish path ([`crate::publish`]) writes a *session* page
//! under `<team_repo>/sessions/`. This module is the sibling "design doc"
//! gesture: given a captured [`TrailRecord`], it renders a coherent, headed
//! design document (front-matter + `Context` / `Decision` / `Reasoning` /
//! `Files touched`) derived purely from the trail's first user prompt, final
//! assistant response, and `files_touched`. No network, no model call — the
//! same `(trail, target)` input yields byte-identical output.
//!
//! ## Trust-model invariant (preserved)
//!
//! [`write_design_doc`] writes **only** to the caller-named `target` path
//! (creating its parent directories). It never derives a `sessions/` layout or
//! any path the caller did not name — the same "no write outside the named
//! target" rule the session-publish path follows.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use illuminate_trail::{AgentKind, MessageRole, TrailRecord};

use crate::Result;

/// Render a deterministic, template-based design-doc markdown from a captured
/// session. Pure — no I/O, no network, no LLM. Two calls with the same
/// `trail` return byte-identical strings.
///
/// Structure:
/// - YAML front-matter (`page_type: design_doc`, `session_id`, `agent`,
///   `model`, `created`, `files_touched`).
/// - `## Context` — derived from the first user prompt.
/// - `## Decision` — derived from the final assistant response.
/// - `## Reasoning` — the assistant's final response verbatim (the rationale).
/// - `## Files touched` — a bullet list (or an explicit "none recorded" note).
pub fn draft_design_doc(trail: &TrailRecord) -> String {
    let front_matter = render_front_matter(trail);
    let context = first_user_prompt(trail).unwrap_or("(no user prompt captured)");
    let decision = last_assistant_response(trail).unwrap_or("(no assistant response captured)");

    let mut s = String::new();
    s.push_str(&front_matter);
    s.push('\n');
    s.push_str("# Design doc\n\n");
    s.push_str("## Context\n\n");
    s.push_str(context);
    s.push_str("\n\n## Decision\n\n");
    s.push_str(decision);
    s.push_str("\n\n## Reasoning\n\n");
    s.push_str(decision);
    s.push_str("\n\n## Files touched\n\n");
    s.push_str(&render_files(trail));
    s.push('\n');
    s
}

/// Write the drafted design-doc markdown to the caller-named `target` path.
///
/// Creates `target`'s parent directories as needed and writes the bytes
/// returned by [`draft_design_doc`] — and nothing else, anywhere else. Returns
/// the path written (always exactly `target`).
pub fn write_design_doc(trail: &TrailRecord, target: &Path) -> Result<PathBuf> {
    let contents = draft_design_doc(trail);
    if let Some(parent) = target.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    let mut f = fs::File::create(target)?;
    f.write_all(contents.as_bytes())?;
    Ok(target.to_path_buf())
}

// ─────────────────────────── internals ───────────────────────────

fn render_front_matter(trail: &TrailRecord) -> String {
    let files_yaml = files_yaml(trail);
    format!(
        "---\npage_type: design_doc\nsession_id: {session_id}\nagent: {agent}\nmodel: {model}\ncreated: {created}\nfiles_touched: {files_yaml}\nsource: design_doc:{agent}\n---",
        session_id = trail.session_id,
        agent = agent_str(&trail.agent),
        model = trail.model,
        created = trail.started_at.to_rfc3339(),
        files_yaml = files_yaml,
    )
}

fn files_yaml(trail: &TrailRecord) -> String {
    if trail.files_touched.is_empty() {
        return "[]".to_string();
    }
    let inner = trail
        .files_touched
        .iter()
        .map(|p| format!("\"{}\"", p.display().to_string().replace('"', "\\\"")))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{inner}]")
}

fn render_files(trail: &TrailRecord) -> String {
    if trail.files_touched.is_empty() {
        return "(no files recorded)".to_string();
    }
    let mut s = String::new();
    for f in &trail.files_touched {
        s.push_str(&format!("- `{}`\n", f.display()));
    }
    // Trim the trailing newline so the document ends with a single `\n` added
    // by the caller — keeps output stable regardless of file count.
    s.trim_end().to_string()
}

fn first_user_prompt(trail: &TrailRecord) -> Option<&str> {
    trail
        .messages
        .iter()
        .find(|m| m.role == MessageRole::User)
        .map(|m| m.text.as_str())
}

fn last_assistant_response(trail: &TrailRecord) -> Option<&str> {
    trail
        .messages
        .iter()
        .rev()
        .find(|m| m.role == MessageRole::Assistant)
        .map(|m| m.text.as_str())
}

fn agent_str(a: &AgentKind) -> &'static str {
    match a {
        AgentKind::ClaudeCode => "claude-code",
        AgentKind::Cursor => "cursor",
        AgentKind::Codex => "codex",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use illuminate_trail::Message;

    fn trail_no_files() -> TrailRecord {
        TrailRecord {
            session_id: "s-1".to_string(),
            agent: AgentKind::Cursor,
            model: "gpt".to_string(),
            started_at: Utc.with_ymd_and_hms(2026, 1, 2, 3, 4, 5).unwrap(),
            ended_at: Utc.with_ymd_and_hms(2026, 1, 2, 3, 10, 5).unwrap(),
            repo_path: std::path::PathBuf::from("/tmp/r"),
            messages: vec![Message {
                role: MessageRole::User,
                timestamp: Utc.with_ymd_and_hms(2026, 1, 2, 3, 4, 5).unwrap(),
                text: "do a thing".to_string(),
            }],
            files_touched: Vec::new(),
            tool_invocations: Vec::new(),
            input_tokens: None,
            output_tokens: None,
            cache_creation_input_tokens: None,
            cache_read_input_tokens: None,
        }
    }

    #[test]
    fn empty_files_renders_explicit_none_note_and_empty_yaml() {
        let t = trail_no_files();
        let md = draft_design_doc(&t);
        assert!(md.contains("files_touched: []"));
        assert!(md.contains("(no files recorded)"));
        // No assistant message → the documented fallback string.
        assert!(md.contains("(no assistant response captured)"));
    }

    #[test]
    fn agent_str_maps_all_kinds() {
        assert_eq!(agent_str(&AgentKind::ClaudeCode), "claude-code");
        assert_eq!(agent_str(&AgentKind::Cursor), "cursor");
        assert_eq!(agent_str(&AgentKind::Codex), "codex");
    }
}
