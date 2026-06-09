//! illuminate-publish: explicit publish gesture for curated trail sessions.
//!
//! Stage 4 of the v3 four-stage pipeline (enrich → generate → capture → curate).
//! Given a trail jsonl from `.illuminate/trail/`, writes a structured markdown
//! page to a configured team-repo path under `sessions/<date>-<slug>.md`. The
//! per-session redaction level (full / summary / decision / discard) is chosen
//! by the caller; nothing is published implicitly.
//!
//! ## Trust-model invariants (enforced)
//!
//! - **`illuminate-publish` is the only crate that writes outside `.illuminate/`.**
//!   No other crate in this workspace has a network-or-foreign-FS write path,
//!   and even this crate refuses to write anywhere the caller has not
//!   explicitly named in `req.team_repo`.
//! - **No network calls.** v3.0 ships [`TeamRepoTarget::LocalPath`] only. The
//!   planned [`TeamRepoTarget::GitRemote`] variant is deliberately gated for
//!   v3.1 with a paired `illuminate trust check` config-linter pass.
//! - **`Discard` writes nothing.** A request with `redaction: Discard` returns
//!   an empty `PublishResponse` and never touches the filesystem or graph.
//!
//! See `docs/trust-model.md` and `docs/PRODUCT_OVERVIEW.md` for the user-facing
//! framing.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use illuminate::{Episode, Graph};
use illuminate_trail::TrailRecord;

pub mod as_doc;
pub use as_doc::{draft_design_doc, write_design_doc};

/// How much of the captured session to share with the team.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RedactionLevel {
    /// Full transcript (every prompt + response).
    Full,
    /// First user prompt, the assistant's final response, and `files_touched`.
    /// The default in the pre-commit hook flow — low friction, high signal.
    Summary,
    /// Front-matter only. Body is the literal string `(decision-only publish)`.
    /// Useful when the team only needs to know "X was decided" without the
    /// surrounding chatter.
    Decision,
    /// Do nothing — return an empty response.
    Discard,
}

impl RedactionLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Summary => "summary",
            Self::Decision => "decision",
            Self::Discard => "discard",
        }
    }

    /// Parse from a CLI-friendly string. Unknown values yield `None`.
    ///
    /// Named `parse` (not `from_str`) so it doesn't shadow `std::str::FromStr`
    /// — callers who want the trait can wire it up themselves; we want a
    /// fallible parser that doesn't allocate a `Box<dyn Error>`.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "full" => Some(Self::Full),
            "summary" => Some(Self::Summary),
            "decision" => Some(Self::Decision),
            "discard" => Some(Self::Discard),
            _ => None,
        }
    }
}

/// Where the published session lands. v3.0 supports a local-path target only.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum TeamRepoTarget {
    /// A directory on the local filesystem. The crate writes
    /// `<path>/sessions/<filename>.md` — directories are created as needed.
    LocalPath(PathBuf),
    // GitRemote { url, branch } — planned for v3.1 once `illuminate trust check`
    // is in place to gate network writes via explicit config + first-use prompt.
}

/// A request to publish one captured session.
#[derive(Debug, Clone)]
pub struct PublishRequest {
    /// Path to the source trail jsonl (typically `.illuminate/trail/<file>.jsonl`).
    pub trail_path: PathBuf,
    pub redaction: RedactionLevel,
    /// Optional git commit SHA this session produced — recorded in front-matter
    /// so future readers can jump from the decision page to the code.
    pub commit_sha: Option<String>,
    pub team_repo: TeamRepoTarget,
}

/// Result of a publish call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishResponse {
    pub session_id: String,
    pub redaction: RedactionLevel,
    /// All filesystem paths the call wrote. Empty for `Discard`.
    pub written_paths: Vec<PathBuf>,
    /// The graph episode registered for this published session, if any.
    /// `None` for `Discard`.
    pub graph_episode_id: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum PublishError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("trail parse error: {0}")]
    Parse(String),
    #[error("graph error: {0}")]
    Graph(#[from] illuminate::IlluminateError),
}

pub type Result<T, E = PublishError> = std::result::Result<T, E>;

/// Publish one captured session. Pure transformation + filesystem write —
/// no network, no implicit upload. `Discard` is a no-op.
///
/// Side effects:
/// - Writes a markdown page under `<team_repo>/sessions/`.
/// - Registers a graph episode in `graph` with `source = "published:<agent>"`.
///
/// Both effects are skipped for `redaction: Discard`.
pub fn publish(graph: &mut Graph, req: &PublishRequest) -> Result<PublishResponse> {
    if req.redaction == RedactionLevel::Discard {
        // Read the trail just enough to surface the session_id in the response
        // — useful for telemetry / "you skipped session X" feedback. No write.
        let session_id = read_trail(&req.trail_path)
            .map(|r| r.session_id)
            .unwrap_or_default();
        return Ok(PublishResponse {
            session_id,
            redaction: RedactionLevel::Discard,
            written_paths: Vec::new(),
            graph_episode_id: None,
        });
    }

    let trail = read_trail(&req.trail_path)?;

    let filename = build_filename(&trail);
    let body = render_body(&trail, req.redaction);
    let front_matter = render_front_matter(&trail, req, &filename);

    let target_path = match &req.team_repo {
        TeamRepoTarget::LocalPath(root) => root.join("sessions").join(&filename),
    };
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let contents = format!("{front_matter}\n{body}\n");
    let mut f = fs::File::create(&target_path)?;
    f.write_all(contents.as_bytes())?;

    let episode_id = register_episode(graph, &trail, req, &target_path, &contents)?;

    Ok(PublishResponse {
        session_id: trail.session_id,
        redaction: req.redaction,
        written_paths: vec![target_path],
        graph_episode_id: Some(episode_id),
    })
}

/// Install a `.git/hooks/pre-commit` script that calls `illuminate publish` on
/// the most-recent trail file. Returns the path written.
pub fn install_pre_commit_hook(repo_root: &Path, team_repo: &Path) -> Result<PathBuf> {
    let hooks_dir = repo_root.join(".git").join("hooks");
    fs::create_dir_all(&hooks_dir)?;
    let hook_path = hooks_dir.join("pre-commit");

    let script = format!(
        r#"#!/usr/bin/env bash
# illuminate pre-commit publish hook — added by `illuminate publish --install-hook`.
# Publishes the most recent trail session as a summary into the team repo.
# Skip with `git commit --no-verify` or by deleting this file.

set -euo pipefail

LATEST_TRAIL="$(ls -1t .illuminate/trail/*.jsonl 2>/dev/null | head -n1 || true)"
if [[ -z "${{LATEST_TRAIL}}" ]]; then
    exit 0
fi

# Default to `summary`. Override by setting ILLUMINATE_PUBLISH_REDACTION
# (full|summary|decision|discard) in the env.
REDACTION="${{ILLUMINATE_PUBLISH_REDACTION:-summary}}"
TEAM_REPO="{team_repo}"
COMMIT_SHA="$(git rev-parse HEAD 2>/dev/null || echo "")"

illuminate publish \
    --trail "${{LATEST_TRAIL}}" \
    --redaction "${{REDACTION}}" \
    --team-repo "${{TEAM_REPO}}" \
    --commit-sha "${{COMMIT_SHA}}" \
    || echo "illuminate publish: skipped (non-fatal)"
"#,
        team_repo = team_repo.display()
    );

    let mut f = fs::File::create(&hook_path)?;
    f.write_all(script.as_bytes())?;
    set_executable(&hook_path)?;
    Ok(hook_path)
}

/// Read a single-line trail jsonl file into a [`TrailRecord`].
///
/// The canonical reader for `.illuminate/trail/<file>.jsonl` (one JSON document
/// per file). Exposed so sibling paths like [`write_design_doc`] share the same
/// parse semantics instead of re-implementing them. Read-only — no write.
pub fn read_trail_file(path: &Path) -> Result<TrailRecord> {
    read_trail(path)
}

// ─────────────────────────── internals ───────────────────────────

fn read_trail(path: &Path) -> Result<TrailRecord> {
    let bytes = fs::read(path)?;
    let trimmed = std::str::from_utf8(&bytes)
        .map_err(|e| PublishError::Parse(format!("utf-8: {e}")))?
        .trim_end();
    // Each trail file is a single-line JSON document.
    serde_json::from_str(trimmed).map_err(|e| PublishError::Parse(e.to_string()))
}

fn build_filename(trail: &TrailRecord) -> String {
    let date = trail.started_at.format("%Y-%m-%d");
    let slug = slugify(first_user_prompt(trail).unwrap_or("session"));
    format!("{date}-{slug}.md")
}

fn render_front_matter(trail: &TrailRecord, req: &PublishRequest, filename: &str) -> String {
    let id = filename.trim_end_matches(".md");
    let agent = agent_str(&trail.agent);
    let files: Vec<String> = trail
        .files_touched
        .iter()
        .map(|p| p.display().to_string())
        .collect();
    let files_yaml = if files.is_empty() {
        "[]".to_string()
    } else {
        let inner = files
            .iter()
            .map(|f| format!("\"{}\"", f.replace('"', "\\\"")))
            .collect::<Vec<_>>()
            .join(", ");
        format!("[{inner}]")
    };
    let commit_sha = req.commit_sha.clone().unwrap_or_default();
    format!(
        "---\nid: ses-{id}\npage_type: session\nsession_id: {session_id}\nagent: {agent}\nmodel: {model}\nredaction: {redaction}\ncommit_sha: \"{commit_sha}\"\nfiles_touched: {files_yaml}\ncreated: {created}\nsource: published:{agent}\n---",
        id = id,
        session_id = trail.session_id,
        agent = agent,
        model = trail.model,
        redaction = req.redaction.as_str(),
        commit_sha = commit_sha,
        files_yaml = files_yaml,
        created = trail.started_at.to_rfc3339(),
    )
}

fn render_body(trail: &TrailRecord, level: RedactionLevel) -> String {
    match level {
        RedactionLevel::Discard => String::new(),
        RedactionLevel::Decision => {
            "(decision-only publish — see front-matter for context)".to_string()
        }
        RedactionLevel::Summary => render_summary(trail),
        RedactionLevel::Full => render_full(trail),
    }
}

fn render_summary(trail: &TrailRecord) -> String {
    let mut s = String::new();
    s.push_str("## Prompt\n\n");
    s.push_str(first_user_prompt(trail).unwrap_or("(no user prompt captured)"));
    s.push_str("\n\n## Final response\n\n");
    s.push_str(last_assistant_response(trail).unwrap_or("(no assistant response captured)"));
    if !trail.files_touched.is_empty() {
        s.push_str("\n\n## Files touched\n\n");
        for f in &trail.files_touched {
            s.push_str(&format!("- `{}`\n", f.display()));
        }
    }
    s
}

fn render_full(trail: &TrailRecord) -> String {
    let mut s = String::new();
    s.push_str("## Full transcript\n\n");
    for (i, m) in trail.messages.iter().enumerate() {
        let role = match m.role {
            illuminate_trail::MessageRole::User => "User",
            illuminate_trail::MessageRole::Assistant => "Assistant",
            illuminate_trail::MessageRole::System => "System",
            illuminate_trail::MessageRole::Tool => "Tool",
        };
        s.push_str(&format!("### {i}. {role}\n\n{}\n\n", m.text));
    }
    if !trail.files_touched.is_empty() {
        s.push_str("## Files touched\n\n");
        for f in &trail.files_touched {
            s.push_str(&format!("- `{}`\n", f.display()));
        }
    }
    s
}

fn register_episode(
    graph: &mut Graph,
    trail: &TrailRecord,
    req: &PublishRequest,
    path: &Path,
    contents: &str,
) -> Result<String> {
    let agent = agent_str(&trail.agent);
    let source = format!("published:{agent}");
    let summary = first_user_prompt(trail).unwrap_or("(no prompt)");
    let metadata = serde_json::json!({
        "session_id": trail.session_id,
        "redaction": req.redaction.as_str(),
        "commit_sha": req.commit_sha,
        "path": path.display().to_string(),
        "files_touched": trail.files_touched.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
    });
    // Episode content is just the summary line + a path back to the markdown.
    // The full body of the published session lives on disk; the graph keeps
    // a lightweight pointer episode that audit / enrich can FTS-match against.
    let content = format!(
        "[ses-{}] {} (published from {})\n\nsee {}",
        truncate(&path.file_stem().unwrap_or_default().to_string_lossy(), 40),
        summary,
        agent,
        path.display(),
    );
    let mut builder = Episode::builder(&content).source(&source);
    if let serde_json::Value::Object(map) = metadata {
        for (k, v) in map {
            builder = builder.meta(&k, v);
        }
    }
    let episode = builder.build();
    let result = graph.add_episode(episode)?;
    let _ = contents; // contents is kept for future hashing / signing — unused for now
    Ok(result.episode_id)
}

fn first_user_prompt(trail: &TrailRecord) -> Option<&str> {
    trail
        .messages
        .iter()
        .find(|m| m.role == illuminate_trail::MessageRole::User)
        .map(|m| m.text.as_str())
}

fn last_assistant_response(trail: &TrailRecord) -> Option<&str> {
    trail
        .messages
        .iter()
        .rev()
        .find(|m| m.role == illuminate_trail::MessageRole::Assistant)
        .map(|m| m.text.as_str())
}

fn agent_str(a: &illuminate_trail::AgentKind) -> &'static str {
    match a {
        illuminate_trail::AgentKind::ClaudeCode => "claude-code",
        illuminate_trail::AgentKind::Cursor => "cursor",
        illuminate_trail::AgentKind::Codex => "codex",
    }
}

fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_dash = true;
    for c in s.chars().take(120) {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "session".to_string()
    } else {
        trimmed
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut cut = max;
    while !s.is_char_boundary(cut) && cut > 0 {
        cut -= 1;
    }
    s[..cut].to_string()
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perm = fs::metadata(path)?.permissions();
    perm.set_mode(0o755);
    fs::set_permissions(path, perm)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use illuminate_trail::{AgentKind, Message, MessageRole};

    fn sample_trail() -> TrailRecord {
        TrailRecord {
            session_id: "test-session-123".to_string(),
            agent: AgentKind::ClaudeCode,
            model: "claude-opus-4-7".to_string(),
            started_at: Utc.with_ymd_and_hms(2026, 5, 25, 10, 0, 0).unwrap(),
            ended_at: Utc.with_ymd_and_hms(2026, 5, 25, 10, 30, 0).unwrap(),
            repo_path: PathBuf::from("/tmp/fake-repo"),
            messages: vec![
                Message {
                    role: MessageRole::User,
                    timestamp: Utc::now(),
                    text: "add caching to the txn endpoint".to_string(),
                },
                Message {
                    role: MessageRole::Assistant,
                    timestamp: Utc::now(),
                    text: "Use LRU with 30s TTL — Redis is rejected per dec-no-redis.".to_string(),
                },
            ],
            files_touched: vec![PathBuf::from("src/payments/txn.rs")],
            tool_invocations: Vec::new(),
            input_tokens: None,
            output_tokens: None,
            cache_creation_input_tokens: None,
            cache_read_input_tokens: None,
        }
    }

    fn write_sample_trail(dir: &Path) -> PathBuf {
        let path = dir.join("trail.jsonl");
        let json = serde_json::to_string(&sample_trail()).unwrap();
        fs::write(&path, json).unwrap();
        path
    }

    fn open_temp_graph(dir: &Path) -> Graph {
        Graph::open_or_create(&dir.join("graph.db")).expect("open graph")
    }

    #[test]
    fn slugify_drops_non_alnum_and_lowercases() {
        assert_eq!(slugify("Add Redis caching!"), "add-redis-caching");
        assert_eq!(slugify("/path/to/file.rs"), "path-to-file-rs");
        assert_eq!(slugify(""), "session");
    }

    #[test]
    fn redaction_level_round_trips_via_str() {
        for lvl in [
            RedactionLevel::Full,
            RedactionLevel::Summary,
            RedactionLevel::Decision,
            RedactionLevel::Discard,
        ] {
            assert_eq!(RedactionLevel::parse(lvl.as_str()), Some(lvl));
        }
    }

    #[test]
    fn discard_writes_nothing_and_no_episode() {
        let dir = tempfile::tempdir().unwrap();
        let team = dir.path().join("team-illuminate");
        let trail_path = write_sample_trail(dir.path());
        let mut graph = open_temp_graph(dir.path());

        let req = PublishRequest {
            trail_path,
            redaction: RedactionLevel::Discard,
            commit_sha: None,
            team_repo: TeamRepoTarget::LocalPath(team.clone()),
        };
        let resp = publish(&mut graph, &req).expect("publish");
        assert_eq!(resp.session_id, "test-session-123");
        assert!(resp.written_paths.is_empty());
        assert!(resp.graph_episode_id.is_none());
        assert!(!team.exists(), "team dir should not exist after Discard");
    }

    #[test]
    fn full_writes_complete_transcript() {
        let dir = tempfile::tempdir().unwrap();
        let team = dir.path().join("team-illuminate");
        let trail_path = write_sample_trail(dir.path());
        let mut graph = open_temp_graph(dir.path());

        let req = PublishRequest {
            trail_path,
            redaction: RedactionLevel::Full,
            commit_sha: Some("abc123".to_string()),
            team_repo: TeamRepoTarget::LocalPath(team.clone()),
        };
        let resp = publish(&mut graph, &req).expect("publish");
        assert_eq!(resp.written_paths.len(), 1);
        let body = fs::read_to_string(&resp.written_paths[0]).unwrap();
        assert!(body.contains("page_type: session"));
        assert!(body.contains("redaction: full"));
        assert!(body.contains("commit_sha: \"abc123\""));
        assert!(body.contains("agent: claude-code"));
        assert!(body.contains("## Full transcript"));
        assert!(body.contains("add caching to the txn endpoint"));
        assert!(body.contains("Use LRU with 30s TTL"));
        assert!(body.contains("`src/payments/txn.rs`"));
    }

    #[test]
    fn summary_includes_first_prompt_and_files_touched_only() {
        let dir = tempfile::tempdir().unwrap();
        let team = dir.path().join("team-illuminate");
        let trail_path = write_sample_trail(dir.path());
        let mut graph = open_temp_graph(dir.path());

        let req = PublishRequest {
            trail_path,
            redaction: RedactionLevel::Summary,
            commit_sha: None,
            team_repo: TeamRepoTarget::LocalPath(team),
        };
        let resp = publish(&mut graph, &req).expect("publish");
        let body = fs::read_to_string(&resp.written_paths[0]).unwrap();
        assert!(body.contains("redaction: summary"));
        assert!(body.contains("## Prompt"));
        assert!(body.contains("add caching to the txn endpoint"));
        assert!(body.contains("## Final response"));
        assert!(body.contains("Use LRU with 30s TTL"));
        assert!(body.contains("`src/payments/txn.rs`"));
        // Summary must NOT include the full transcript heading.
        assert!(!body.contains("## Full transcript"));
    }

    #[test]
    fn decision_only_writes_front_matter_with_empty_body() {
        let dir = tempfile::tempdir().unwrap();
        let team = dir.path().join("team-illuminate");
        let trail_path = write_sample_trail(dir.path());
        let mut graph = open_temp_graph(dir.path());

        let req = PublishRequest {
            trail_path,
            redaction: RedactionLevel::Decision,
            commit_sha: None,
            team_repo: TeamRepoTarget::LocalPath(team),
        };
        let resp = publish(&mut graph, &req).expect("publish");
        let body = fs::read_to_string(&resp.written_paths[0]).unwrap();
        assert!(body.contains("redaction: decision"));
        assert!(body.contains("(decision-only publish"));
        assert!(!body.contains("## Prompt"));
        assert!(!body.contains("## Full transcript"));
    }

    #[test]
    fn filename_uses_date_and_slug_under_sessions_subdir() {
        let dir = tempfile::tempdir().unwrap();
        let team = dir.path().join("team-illuminate");
        let trail_path = write_sample_trail(dir.path());
        let mut graph = open_temp_graph(dir.path());

        let req = PublishRequest {
            trail_path,
            redaction: RedactionLevel::Summary,
            commit_sha: None,
            team_repo: TeamRepoTarget::LocalPath(team.clone()),
        };
        let resp = publish(&mut graph, &req).expect("publish");
        let written = &resp.written_paths[0];
        // sessions/<date>-<slug>.md
        assert!(written.starts_with(&team), "written under team_repo");
        assert!(written.to_string_lossy().contains("/sessions/"));
        assert!(written.to_string_lossy().contains("2026-05-25"));
        assert!(!written.to_string_lossy().contains("add-redis-caching")); // sanity
        let name = written.file_name().unwrap().to_string_lossy();
        assert!(name.starts_with("2026-05-25-"));
        assert!(name.ends_with(".md"));
    }

    #[test]
    fn publish_registers_a_graph_episode_with_published_source() {
        let dir = tempfile::tempdir().unwrap();
        let team = dir.path().join("team-illuminate");
        let trail_path = write_sample_trail(dir.path());
        let mut graph = open_temp_graph(dir.path());

        let req = PublishRequest {
            trail_path,
            redaction: RedactionLevel::Summary,
            commit_sha: None,
            team_repo: TeamRepoTarget::LocalPath(team),
        };
        let resp = publish(&mut graph, &req).expect("publish");
        let episode_id = resp.graph_episode_id.expect("episode id");

        let ep = graph
            .get_episode(&episode_id)
            .expect("get_episode")
            .expect("episode exists");
        assert_eq!(ep.source.as_deref(), Some("published:claude-code"));
        assert!(ep.content.contains("add caching to the txn endpoint"));
        // Metadata recorded session_id and redaction.
        let meta = ep.metadata.expect("metadata");
        assert_eq!(meta["session_id"], "test-session-123");
        assert_eq!(meta["redaction"], "summary");
    }

    #[test]
    fn install_hook_writes_executable_pre_commit() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        let hook = install_pre_commit_hook(dir.path(), &PathBuf::from("../team-illuminate"))
            .expect("install");
        assert!(hook.ends_with(".git/hooks/pre-commit"));
        let content = fs::read_to_string(&hook).unwrap();
        assert!(content.contains("illuminate publish"));
        assert!(content.contains("ILLUMINATE_PUBLISH_REDACTION"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&hook).unwrap().permissions().mode();
            assert_eq!(mode & 0o111, 0o111, "hook must be executable");
        }
    }
}
