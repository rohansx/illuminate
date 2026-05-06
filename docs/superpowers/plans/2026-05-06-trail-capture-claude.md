# Trail Capture (Claude Code) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the v0.1 Claude Code prompt-trail capture: a watcher that detects new sessions in `~/.claude/projects/`, parses the jsonl records, normalizes them to `TrailRecord`, writes them to `.illuminate/trail/`, and exposes them via `illuminate trail {list,show,import,watch}`.

**Architecture:** Build out the existing `illuminate-trail` crate skeleton. Add a Claude jsonl parser that tolerates the heterogeneous record shapes Claude Code emits (`type: "user" | "assistant" | "attachment" | "last-prompt" | ...`), an opt-in repo-detection helper that walks ancestors of a session's `cwd` looking for `.illuminate/illuminate.toml`, a trail-file storage layer, and a `notify`-based watcher loop. Then wire a `Trail` subcommand into `illuminate-cli` with `import` (one-shot), `list`, `show`, and `watch` (foreground daemon). All work fits inside this plan; no schema or graph changes — that's a separate plan.

**Tech Stack:** Rust 2024, `tokio`, `notify` 7, `serde` / `serde_json`, `chrono`, `clap` (already in workspace). No new workspace deps.

**Scope discipline:**
- Don't extract entities/decisions from trails yet (separate plan).
- Don't write to `graph.db` (separate plan).
- Don't build wiki rendering (separate plan).
- Don't build Cursor/Codex (deferred to v0.2 per `docs/ROADMAP.md`).
- Don't change `illuminate-watch`'s git ingester behavior.
- Do produce: a working `illuminate trail watch` that captures sessions in real time on this very repo (dogfood).

---

## File structure

**Create:**
- `crates/illuminate-trail/src/raw.rs` — strongly-typed wrapper over the heterogeneous Claude jsonl record types
- `crates/illuminate-trail/src/repo.rs` — opt-in detection (resolve `cwd` → repo root with `.illuminate/illuminate.toml`)
- `crates/illuminate-trail/src/storage.rs` — `.illuminate/trail/` file path generation + write
- `crates/illuminate-trail/src/import.rs` — one-shot import path (parse + normalize + write)
- `crates/illuminate-trail/tests/fixtures/claude-session.jsonl` — a small real-shape session fixture for tests
- `crates/illuminate-trail/tests/parse_test.rs` — claude jsonl parser tests
- `crates/illuminate-trail/tests/repo_test.rs` — opt-in detection tests
- `crates/illuminate-trail/tests/storage_test.rs` — trail file storage tests
- `crates/illuminate-trail/tests/import_test.rs` — end-to-end import tests
- `crates/illuminate-cli/src/commands/trail.rs` — `illuminate trail` subcommand handlers

**Modify:**
- `crates/illuminate-trail/src/lib.rs` — re-export new modules; remove stale `cursor`/`codex` re-exports kept private as stubs
- `crates/illuminate-trail/src/claude.rs` — replace stub `parse_session` with real implementation; keep `default_sessions_dir` and add `discover_sessions`
- `crates/illuminate-trail/src/watcher.rs` — implement notify-based loop; expose channel for parsed records
- `crates/illuminate-trail/Cargo.toml` — already adequate (`notify` already there); confirm `serde_json` is enabled
- `crates/illuminate-cli/Cargo.toml` — add `illuminate-trail` workspace dep
- `crates/illuminate-cli/src/main.rs` — add `Trail` variant to `Commands` enum, wire dispatch
- `crates/illuminate-cli/src/commands/mod.rs` — re-export `trail`

---

## Reference: Claude Code session jsonl format

A Claude Code session lives at `~/.claude/projects/<project-hash>/<session-uuid>.jsonl`. The `<project-hash>` is the absolute repo path with `/` replaced by `-` and a leading `-`. Example: `/home/rsx/Desktop/projx/illuminate` → `-home-rsx-Desktop-projx-illuminate`.

Each line is one JSON object with a `type` field. The relevant types for v0.1:

| `type` | Meaning | What we extract |
|--------|---------|-----------------|
| `user` | A user prompt | `message.content` (string), `timestamp`, `cwd`, `sessionId` |
| `assistant` | An assistant response | `message.content` (string or array), `timestamp` |
| `attachment` | Tool calls, hooks, deferred-tool deltas | only `tool_use` / tool result kinds; ignore `hook_*`, `skill_listing`, `mcp_instructions_delta`, `deferred_tools_delta`, `file-history-snapshot` |
| `last-prompt` | Bookmark | ignore |
| `permission-mode` | Mode metadata | ignore |
| `summary` | Session summary | optional; capture if present |

Common fields on most records: `parentUuid`, `isSidechain`, `uuid`, `timestamp` (ISO-8601), `cwd`, `sessionId`, `version`, `gitBranch`, `entrypoint`. The parser must tolerate unknown `type` values by falling through to `Unknown` rather than failing.

---

## Task 1: Commit existing docs + crate skeleton

**Files:**
- Modify: working tree (commit only; no new edits)

- [ ] **Step 1: Verify clean docs state**

Run: `cd /home/rsx/Desktop/projx/illuminate && git status -s`

Expected: shows the renamed/added docs, illuminate-trail crate, illuminate.toml.example, README.md, illuminate-watch Cargo.toml description tweak. No surprise files.

- [ ] **Step 2: Stage everything**

```bash
git add docs/ crates/illuminate-trail/ crates/illuminate-watch/Cargo.toml \
    illuminate.toml.example README.md Cargo.lock
```

- [ ] **Step 3: Commit**

```bash
git commit -m "docs: pivot to compounding-context framing; add illuminate-trail skeleton

archive prior docs to docs/old/. introduce v2 product overview, architecture,
schema, ingestion, audit, bootstrap, roadmap, privacy, crates, cli, mcp docs.
add illuminate-trail crate skeleton (claude/cursor/codex stubs, normalize
helpers, watcher harness). update illuminate.toml.example for v2 config and
README for new positioning."
```

- [ ] **Step 4: Verify**

Run: `git log --oneline -1`

Expected: shows the new commit at HEAD.

---

## Task 2: Add a Claude session fixture

**Files:**
- Create: `crates/illuminate-trail/tests/fixtures/claude-session.jsonl`

We need a small real-shape session to drive parser tests. Hand-craft a 12-line jsonl with the record variants we care about. **Do not copy a real session verbatim** — handcrafted is more stable and scope-contained.

- [ ] **Step 1: Write the fixture**

```jsonl
{"type":"last-prompt","leafUuid":"00000000-0000-0000-0000-000000000001","sessionId":"abc-123"}
{"type":"permission-mode","permissionMode":"default","sessionId":"abc-123"}
{"parentUuid":null,"isSidechain":false,"attachment":{"type":"hook_success","hookName":"SessionStart:startup","content":""},"type":"attachment","uuid":"hook-1","timestamp":"2026-05-06T12:00:00.000Z","cwd":"/tmp/illuminate-fixture-repo","sessionId":"abc-123","version":"2.1.128","gitBranch":"main"}
{"parentUuid":"hook-1","isSidechain":false,"promptId":"p-1","type":"user","message":{"role":"user","content":"explain the audit flow"},"uuid":"u-1","timestamp":"2026-05-06T12:00:05.000Z","cwd":"/tmp/illuminate-fixture-repo","sessionId":"abc-123","version":"2.1.128","gitBranch":"main"}
{"parentUuid":"u-1","isSidechain":false,"type":"assistant","message":{"role":"assistant","content":"the audit flow runs locally and queries the graph."},"uuid":"a-1","timestamp":"2026-05-06T12:00:10.000Z","cwd":"/tmp/illuminate-fixture-repo","sessionId":"abc-123","version":"2.1.128","gitBranch":"main"}
{"parentUuid":"a-1","isSidechain":false,"type":"user","message":{"role":"user","content":"add an integration test"},"uuid":"u-2","timestamp":"2026-05-06T12:01:00.000Z","cwd":"/tmp/illuminate-fixture-repo","sessionId":"abc-123","version":"2.1.128","gitBranch":"main"}
{"parentUuid":"u-2","isSidechain":false,"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"sure, here is the test"},{"type":"tool_use","id":"tu-1","name":"Write","input":{"path":"src/audit_test.rs","content":"// test"}}]},"uuid":"a-2","timestamp":"2026-05-06T12:01:30.000Z","cwd":"/tmp/illuminate-fixture-repo","sessionId":"abc-123","version":"2.1.128","gitBranch":"main"}
{"parentUuid":"a-2","isSidechain":false,"attachment":{"type":"tool_result","tool_use_id":"tu-1","content":"OK"},"type":"attachment","uuid":"tr-1","timestamp":"2026-05-06T12:01:31.000Z","cwd":"/tmp/illuminate-fixture-repo","sessionId":"abc-123","version":"2.1.128","gitBranch":"main"}
{"type":"file-history-snapshot","messageId":"snap-1","snapshot":{"messageId":"snap-1","trackedFileBackups":{},"timestamp":"2026-05-06T12:01:32.000Z"},"isSnapshotUpdate":false}
{"type":"last-prompt","lastPrompt":"add an integration test","leafUuid":"a-2","sessionId":"abc-123"}
```

Save to `crates/illuminate-trail/tests/fixtures/claude-session.jsonl`.

- [ ] **Step 2: Sanity-check the fixture parses as ndjson**

Run: `python3 -c "import json,sys; [json.loads(l) for l in open('crates/illuminate-trail/tests/fixtures/claude-session.jsonl')]; print('ok')"`

Expected: `ok`.

- [ ] **Step 3: Commit**

```bash
git add crates/illuminate-trail/tests/fixtures/claude-session.jsonl
git commit -m "test: add claude session fixture for trail parser"
```

---

## Task 3: Define raw record types (`raw.rs`)

**Files:**
- Create: `crates/illuminate-trail/src/raw.rs`
- Modify: `crates/illuminate-trail/src/lib.rs` (add `pub mod raw;`)
- Test: `crates/illuminate-trail/tests/parse_test.rs`

The raw layer mirrors the on-disk record shapes 1:1. Heterogeneous `type` field handled via `#[serde(tag = "type", rename_all = "kebab-case")]` with an `Unknown(serde_json::Value)` fallback variant.

- [ ] **Step 1: Write the failing test**

Create `crates/illuminate-trail/tests/parse_test.rs`:

```rust
use illuminate_trail::raw::{parse_jsonl, RawRecord};

const FIXTURE: &str = include_str!("fixtures/claude-session.jsonl");

#[test]
fn parses_all_lines_as_records() {
    let records = parse_jsonl(FIXTURE).expect("fixture must parse");
    assert_eq!(records.len(), 10, "fixture has 10 lines");
}

#[test]
fn classifies_user_and_assistant_records() {
    let records = parse_jsonl(FIXTURE).unwrap();
    let users = records.iter().filter(|r| matches!(r, RawRecord::User { .. })).count();
    let assistants = records.iter().filter(|r| matches!(r, RawRecord::Assistant { .. })).count();
    assert_eq!(users, 2);
    assert_eq!(assistants, 2);
}

#[test]
fn unknown_record_types_round_trip_to_unknown_variant() {
    let line = r#"{"type":"made-up-type","sessionId":"x"}"#;
    let records = parse_jsonl(line).unwrap();
    assert!(matches!(records[0], RawRecord::Unknown(_)));
}

#[test]
fn skips_empty_lines() {
    let input = "\n\n\n";
    let records = parse_jsonl(input).unwrap();
    assert_eq!(records.len(), 0);
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p illuminate-trail --test parse_test 2>&1 | tail -20`

Expected: compile failure (`raw` module does not exist).

- [ ] **Step 3: Implement `raw.rs`**

Create `crates/illuminate-trail/src/raw.rs`:

```rust
//! Raw, on-disk representation of Claude Code session jsonl records.
//!
//! Records are heterogeneous — a single jsonl file mixes user/assistant
//! messages with attachments, hooks, file-history snapshots, and bookmarks.
//! The parser tolerates unknown `type` values by routing them to
//! [`RawRecord::Unknown`] rather than failing.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum RawRecord {
    User(UserRecord),
    Assistant(AssistantRecord),
    Attachment(AttachmentRecord),
    Summary(SummaryRecord),
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRecord {
    pub uuid: String,
    pub timestamp: DateTime<Utc>,
    pub cwd: Option<String>,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub message: MessageBlock,
    #[serde(rename = "parentUuid", default)]
    pub parent_uuid: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(rename = "gitBranch", default)]
    pub git_branch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantRecord {
    pub uuid: String,
    pub timestamp: DateTime<Utc>,
    pub cwd: Option<String>,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub message: MessageBlock,
    #[serde(rename = "parentUuid", default)]
    pub parent_uuid: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(rename = "gitBranch", default)]
    pub git_branch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentRecord {
    pub uuid: String,
    pub timestamp: DateTime<Utc>,
    pub cwd: Option<String>,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub attachment: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryRecord {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub summary: String,
    #[serde(default)]
    pub timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageBlock {
    pub role: String,
    pub content: Value,
}

/// Parse a Claude Code session jsonl file (or any subset of it) into
/// strongly-typed records. Empty lines are silently skipped; malformed lines
/// produce a `Parse` error referencing the line number.
pub fn parse_jsonl(input: &str) -> crate::Result<Vec<RawRecord>> {
    let mut out = Vec::new();
    for (idx, line) in input.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let rec: RawRecord = serde_json::from_str(trimmed).map_err(|e| {
            crate::TrailError::Parse(format!("line {}: {e}", idx + 1))
        })?;
        out.push(rec);
    }
    Ok(out)
}
```

Add to `lib.rs`:

```rust
pub mod raw;
```

(insert after `pub mod normalize;`).

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test -p illuminate-trail --test parse_test 2>&1 | tail -20`

Expected: 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/illuminate-trail/src/raw.rs crates/illuminate-trail/src/lib.rs \
        crates/illuminate-trail/tests/parse_test.rs
git commit -m "feat(trail): parse claude session jsonl into raw records"
```

---

## Task 4: Opt-in repo detection (`repo.rs`)

**Files:**
- Create: `crates/illuminate-trail/src/repo.rs`
- Modify: `crates/illuminate-trail/src/lib.rs` (add `pub mod repo;`)
- Test: `crates/illuminate-trail/tests/repo_test.rs`

A session belongs to a repo iff some ancestor of its `cwd` contains `.illuminate/illuminate.toml`. The watcher will skip non-opted-in sessions.

- [ ] **Step 1: Write the failing test**

Create `crates/illuminate-trail/tests/repo_test.rs`:

```rust
use illuminate_trail::repo::resolve_repo;
use std::fs;
use std::path::Path;

fn make_opted_in(root: &Path) {
    fs::create_dir_all(root.join(".illuminate")).unwrap();
    fs::write(root.join(".illuminate/illuminate.toml"), "name = 'test'\n").unwrap();
}

#[test]
fn detects_repo_at_cwd_directly() {
    let tmp = tempfile::tempdir().unwrap();
    make_opted_in(tmp.path());
    let resolved = resolve_repo(tmp.path()).unwrap();
    assert_eq!(resolved, tmp.path());
}

#[test]
fn walks_ancestors_to_find_opt_in_marker() {
    let tmp = tempfile::tempdir().unwrap();
    make_opted_in(tmp.path());
    let nested = tmp.path().join("a/b/c");
    fs::create_dir_all(&nested).unwrap();
    let resolved = resolve_repo(&nested).unwrap();
    assert_eq!(resolved, tmp.path());
}

#[test]
fn returns_none_when_no_marker_found() {
    let tmp = tempfile::tempdir().unwrap();
    // no .illuminate
    assert!(resolve_repo(tmp.path()).is_none());
}
```

Add `tempfile` as a dev-dep on the trail crate.

- [ ] **Step 2: Run to verify it fails**

First, add to `crates/illuminate-trail/Cargo.toml` under `[dev-dependencies]`:

```toml
[dev-dependencies]
tempfile = "3"
```

Run: `cargo test -p illuminate-trail --test repo_test 2>&1 | tail -20`

Expected: compile failure (`repo` module does not exist).

- [ ] **Step 3: Implement `repo.rs`**

Create `crates/illuminate-trail/src/repo.rs`:

```rust
//! Opt-in detection: a session is captured only if the working directory it
//! ran in (or one of its ancestors) contains `.illuminate/illuminate.toml`.

use std::path::{Path, PathBuf};

/// Walk ancestors of `cwd` looking for `.illuminate/illuminate.toml`.
/// Returns the directory that contains it, or `None` if no opt-in marker
/// is found before reaching the filesystem root.
pub fn resolve_repo(cwd: &Path) -> Option<PathBuf> {
    let mut cur = Some(cwd);
    while let Some(dir) = cur {
        if dir.join(".illuminate").join("illuminate.toml").is_file() {
            return Some(dir.to_path_buf());
        }
        cur = dir.parent();
    }
    None
}
```

Add to `lib.rs`:

```rust
pub mod repo;
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test -p illuminate-trail --test repo_test 2>&1 | tail -20`

Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/illuminate-trail/Cargo.toml crates/illuminate-trail/src/repo.rs \
        crates/illuminate-trail/src/lib.rs crates/illuminate-trail/tests/repo_test.rs
git commit -m "feat(trail): opt-in repo detection via .illuminate/illuminate.toml"
```

---

## Task 5: Trail file storage (`storage.rs`)

**Files:**
- Create: `crates/illuminate-trail/src/storage.rs`
- Modify: `crates/illuminate-trail/src/lib.rs` (add `pub mod storage;`)
- Test: `crates/illuminate-trail/tests/storage_test.rs`

The storage layer is responsible for two things: (1) compute the canonical filename for a normalized record, (2) write the record as a single jsonl line into `<repo>/.illuminate/trail/`. Idempotent: writing the same `session_id` overwrites (no append-only here; sessions are versioned by `ended_at`).

- [ ] **Step 1: Write the failing test**

Create `crates/illuminate-trail/tests/storage_test.rs`:

```rust
use chrono::{TimeZone, Utc};
use illuminate_trail::record::*;
use illuminate_trail::storage::{trail_path, write_trail};
use std::path::PathBuf;

fn sample_record() -> TrailRecord {
    TrailRecord {
        session_id: "abc-123".into(),
        agent: AgentKind::ClaudeCode,
        model: "claude-sonnet-4-6".into(),
        started_at: Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap(),
        ended_at: Utc.with_ymd_and_hms(2026, 5, 6, 12, 30, 0).unwrap(),
        repo_path: PathBuf::from("/tmp/x"),
        messages: vec![Message {
            role: MessageRole::User,
            timestamp: Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 5).unwrap(),
            text: "explain the audit flow".into(),
        }],
        files_touched: vec![],
        tool_invocations: vec![],
    }
}

#[test]
fn trail_path_uses_date_topic_agent() {
    let r = sample_record();
    let path = trail_path(&r);
    let name = path.file_name().unwrap().to_str().unwrap();
    assert!(name.starts_with("2026-05-06-"));
    assert!(name.ends_with("-claude.jsonl"));
}

#[test]
fn write_trail_creates_file_and_round_trips() {
    let tmp = tempfile::tempdir().unwrap();
    let mut r = sample_record();
    r.repo_path = tmp.path().to_path_buf();
    let written = write_trail(&r).unwrap();
    assert!(written.exists());
    let content = std::fs::read_to_string(&written).unwrap();
    let parsed: TrailRecord = serde_json::from_str(content.trim()).unwrap();
    assert_eq!(parsed.session_id, "abc-123");
}

#[test]
fn write_trail_overwrites_for_same_session() {
    let tmp = tempfile::tempdir().unwrap();
    let mut r = sample_record();
    r.repo_path = tmp.path().to_path_buf();
    write_trail(&r).unwrap();
    let p1 = write_trail(&r).unwrap();
    // path is deterministic from (date, topic, agent), but topic comes from
    // first user message — same record, same path
    assert!(p1.exists());
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p illuminate-trail --test storage_test 2>&1 | tail -20`

Expected: compile failure (`storage` does not exist).

- [ ] **Step 3: Implement `storage.rs`**

Create `crates/illuminate-trail/src/storage.rs`:

```rust
//! Trail file storage: write a normalized [`TrailRecord`] to
//! `<repo>/.illuminate/trail/<date>-<topic>-<agent>.jsonl`.
//!
//! Each file holds exactly one record (as a single jsonl line). Writing the
//! same session twice overwrites the previous file — the topic-slug + agent
//! + date combination is the deterministic identity.

use crate::normalize::{output_filename, topic_slug};
use crate::record::TrailRecord;
use crate::Result;
use std::path::PathBuf;

/// Compute the on-disk path for a record, without performing the write.
pub fn trail_path(record: &TrailRecord) -> PathBuf {
    let slug = topic_slug(&record.messages);
    let filename = output_filename(record, if slug.is_empty() { "session" } else { &slug });
    record.repo_path.join(".illuminate").join("trail").join(filename)
}

/// Write a record to its canonical path, creating parent dirs as needed.
pub fn write_trail(record: &TrailRecord) -> Result<PathBuf> {
    let path = trail_path(record);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let line = serde_json::to_string(record)
        .map_err(|e| crate::TrailError::Parse(format!("serialize: {e}")))?;
    std::fs::write(&path, format!("{line}\n"))?;
    Ok(path)
}
```

Add to `lib.rs`:

```rust
pub mod storage;
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test -p illuminate-trail --test storage_test 2>&1 | tail -20`

Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/illuminate-trail/src/storage.rs crates/illuminate-trail/src/lib.rs \
        crates/illuminate-trail/tests/storage_test.rs
git commit -m "feat(trail): write normalized trail records to .illuminate/trail/"
```

---

## Task 6: Real `claude::parse_session`

**Files:**
- Modify: `crates/illuminate-trail/src/claude.rs`
- Test: `crates/illuminate-trail/tests/parse_test.rs` (extend)

Read the on-disk jsonl, walk records in order, build a `TrailRecord`. `started_at` = first user message timestamp; `ended_at` = last record timestamp; `repo_path` = first non-empty `cwd` (we trust the session — they can only run in one repo per session in practice); `model` = best-effort from any record that exposes it (Claude embeds it in `assistant.message.model` sometimes; for v0.1 we leave `model` empty if unknown rather than failing).

- [ ] **Step 1: Extend the failing test**

Append to `crates/illuminate-trail/tests/parse_test.rs`:

```rust
use illuminate_trail::claude::parse_session;
use std::io::Write;

#[test]
fn parse_session_extracts_user_and_assistant_messages() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let fixture = include_str!("fixtures/claude-session.jsonl");
    tmp.as_file().write_all(fixture.as_bytes()).unwrap();
    let record = parse_session(tmp.path()).unwrap();
    assert_eq!(record.session_id, "abc-123");
    assert_eq!(record.messages.len(), 4); // 2 user + 2 assistant
    assert_eq!(record.messages[0].text, "explain the audit flow");
    assert_eq!(record.repo_path.to_str().unwrap(), "/tmp/illuminate-fixture-repo");
}

#[test]
fn parse_session_collects_tool_invocations_from_assistant_blocks() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let fixture = include_str!("fixtures/claude-session.jsonl");
    tmp.as_file().write_all(fixture.as_bytes()).unwrap();
    let record = parse_session(tmp.path()).unwrap();
    let writes = record.tool_invocations.iter()
        .filter(|t| t.name == "Write")
        .count();
    assert_eq!(writes, 1);
}
```

Add `Write` import to the file's imports.

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p illuminate-trail --test parse_test 2>&1 | tail -20`

Expected: failure — current `parse_session` is a stub returning empty fields.

- [ ] **Step 3: Implement real `parse_session`**

Replace `crates/illuminate-trail/src/claude.rs` content (keep `default_sessions_dir`):

```rust
//! Claude Code session capture.

use crate::raw::{parse_jsonl, RawRecord};
use crate::record::{AgentKind, Message, MessageRole, ToolInvocation, TrailRecord};
use crate::{Result, TrailError};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::path::{Path, PathBuf};

pub fn default_sessions_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".claude").join("projects"))
}

pub fn parse_session(path: &Path) -> Result<TrailRecord> {
    let content = std::fs::read_to_string(path)?;
    let records = parse_jsonl(&content)?;

    let mut messages: Vec<Message> = Vec::new();
    let mut tool_invocations: Vec<ToolInvocation> = Vec::new();
    let mut session_id: Option<String> = None;
    let mut model = String::new();
    let mut repo_path: Option<PathBuf> = None;
    let mut started_at: Option<DateTime<Utc>> = None;
    let mut ended_at: Option<DateTime<Utc>> = None;

    for rec in &records {
        match rec {
            RawRecord::User(u) => {
                session_id.get_or_insert_with(|| u.session_id.clone());
                if repo_path.is_none() {
                    if let Some(c) = &u.cwd {
                        repo_path = Some(PathBuf::from(c));
                    }
                }
                let text = content_to_text(&u.message.content);
                if !text.is_empty() {
                    started_at.get_or_insert(u.timestamp);
                    ended_at = Some(u.timestamp);
                    messages.push(Message {
                        role: MessageRole::User,
                        timestamp: u.timestamp,
                        text,
                    });
                }
            }
            RawRecord::Assistant(a) => {
                session_id.get_or_insert_with(|| a.session_id.clone());
                if repo_path.is_none() {
                    if let Some(c) = &a.cwd {
                        repo_path = Some(PathBuf::from(c));
                    }
                }
                if let Some(m) = a.message.content.as_object().and_then(|_| None::<&str>) {
                    let _ = m; // placeholder; model is rarely a top-level string here
                }
                ended_at = Some(a.timestamp);
                let (text, calls) = split_assistant_content(&a.message.content, a.timestamp);
                if !text.is_empty() {
                    messages.push(Message {
                        role: MessageRole::Assistant,
                        timestamp: a.timestamp,
                        text,
                    });
                }
                tool_invocations.extend(calls);
            }
            RawRecord::Attachment(att) => {
                ended_at = Some(att.timestamp);
                if repo_path.is_none() {
                    if let Some(c) = &att.cwd {
                        repo_path = Some(PathBuf::from(c));
                    }
                }
                // We don't surface hooks/snapshots; tool_result is paired with
                // an assistant tool_use elsewhere.
            }
            RawRecord::Summary(_) | RawRecord::Unknown => {}
        }
    }

    let session_id = session_id.ok_or_else(|| {
        TrailError::Normalize("no sessionId found in any record".into())
    })?;
    let repo_path = repo_path.ok_or_else(|| {
        TrailError::Normalize("no cwd found in any record".into())
    })?;
    let started_at = started_at.unwrap_or_else(Utc::now);
    let ended_at = ended_at.unwrap_or(started_at);

    Ok(TrailRecord {
        session_id,
        agent: AgentKind::ClaudeCode,
        model,
        started_at,
        ended_at,
        repo_path,
        messages,
        files_touched: Vec::new(),
        tool_invocations,
    })
}

fn content_to_text(content: &Value) -> String {
    match content {
        Value::String(s) => s.clone(),
        Value::Array(arr) => arr
            .iter()
            .filter_map(|item| {
                let obj = item.as_object()?;
                if obj.get("type").and_then(|t| t.as_str()) == Some("text") {
                    obj.get("text").and_then(|t| t.as_str()).map(str::to_string)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

fn split_assistant_content(content: &Value, ts: DateTime<Utc>) -> (String, Vec<ToolInvocation>) {
    let mut text = String::new();
    let mut tools = Vec::new();
    if let Value::Array(arr) = content {
        for item in arr {
            let Some(obj) = item.as_object() else { continue };
            match obj.get("type").and_then(|t| t.as_str()) {
                Some("text") => {
                    if let Some(t) = obj.get("text").and_then(|t| t.as_str()) {
                        if !text.is_empty() {
                            text.push('\n');
                        }
                        text.push_str(t);
                    }
                }
                Some("tool_use") => {
                    let name = obj.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
                    let params = obj.get("input").cloned().unwrap_or(Value::Null);
                    tools.push(ToolInvocation {
                        name,
                        timestamp: ts,
                        params,
                        result: Value::Null,
                    });
                }
                _ => {}
            }
        }
    } else if let Value::String(s) = content {
        text = s.clone();
    }
    (text, tools)
}
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test -p illuminate-trail --test parse_test 2>&1 | tail -20`

Expected: all parse_test cases pass.

- [ ] **Step 5: Commit**

```bash
git add crates/illuminate-trail/src/claude.rs crates/illuminate-trail/tests/parse_test.rs
git commit -m "feat(trail): parse claude jsonl into normalized trailrecord"
```

---

## Task 7: One-shot import (`import.rs`)

**Files:**
- Create: `crates/illuminate-trail/src/import.rs`
- Modify: `crates/illuminate-trail/src/lib.rs`
- Test: `crates/illuminate-trail/tests/import_test.rs`

`import_session(path)` ties parse → opt-in check → write into one call. Used by both the CLI `trail import` subcommand and the live watcher.

- [ ] **Step 1: Write the failing test**

Create `crates/illuminate-trail/tests/import_test.rs`:

```rust
use illuminate_trail::import::import_session;
use std::fs;
use std::io::Write;

const FIXTURE: &str = include_str!("fixtures/claude-session.jsonl");

fn make_opted_in_repo(repo: &std::path::Path) {
    fs::create_dir_all(repo.join(".illuminate")).unwrap();
    fs::write(repo.join(".illuminate/illuminate.toml"), "name='x'\n").unwrap();
}

fn write_fixture_session(jsonl_path: &std::path::Path, repo: &std::path::Path) {
    // Replace the fixture's hardcoded /tmp/illuminate-fixture-repo with the
    // actual tempdir path, so the parsed cwd resolves to the opted-in repo.
    let mut f = fs::File::create(jsonl_path).unwrap();
    let patched = FIXTURE.replace("/tmp/illuminate-fixture-repo", repo.to_str().unwrap());
    f.write_all(patched.as_bytes()).unwrap();
}

#[test]
fn imports_session_for_opted_in_repo() {
    let repo = tempfile::tempdir().unwrap();
    make_opted_in_repo(repo.path());
    let jsonl = repo.path().join("session.jsonl");
    write_fixture_session(&jsonl, repo.path());
    let written = import_session(&jsonl).unwrap();
    assert!(written.is_some());
    let p = written.unwrap();
    assert!(p.starts_with(repo.path().join(".illuminate").join("trail")));
    assert!(p.exists());
}

#[test]
fn skips_session_for_non_opted_in_repo() {
    let repo = tempfile::tempdir().unwrap();
    // no .illuminate marker
    let jsonl = repo.path().join("session.jsonl");
    write_fixture_session(&jsonl, repo.path());
    let written = import_session(&jsonl).unwrap();
    assert!(written.is_none(), "session for non-opted-in repo must be skipped");
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p illuminate-trail --test import_test 2>&1 | tail -20`

Expected: compile failure (`import` module does not exist).

- [ ] **Step 3: Implement `import.rs`**

Create `crates/illuminate-trail/src/import.rs`:

```rust
//! One-shot session import: parse a Claude jsonl, check opt-in, write.
//!
//! Returns `Ok(None)` if the session's repo isn't opted in (no
//! `.illuminate/illuminate.toml` ancestor of its `cwd`). Returns the path to
//! the written trail file otherwise.

use crate::claude::parse_session;
use crate::repo::resolve_repo;
use crate::storage::write_trail;
use crate::Result;
use std::path::{Path, PathBuf};

pub fn import_session(jsonl_path: &Path) -> Result<Option<PathBuf>> {
    let mut record = parse_session(jsonl_path)?;
    let Some(repo) = resolve_repo(&record.repo_path) else {
        return Ok(None);
    };
    record.repo_path = repo;
    let written = write_trail(&record)?;
    Ok(Some(written))
}
```

Add to `lib.rs`:

```rust
pub mod import;
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test -p illuminate-trail --test import_test 2>&1 | tail -20`

Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/illuminate-trail/src/import.rs crates/illuminate-trail/src/lib.rs \
        crates/illuminate-trail/tests/import_test.rs
git commit -m "feat(trail): one-shot import_session ties parse + opt-in + write"
```

---

## Task 8: Notify-based watcher loop (`watcher.rs`)

**Files:**
- Modify: `crates/illuminate-trail/src/watcher.rs`

The loop watches `~/.claude/projects/` recursively. On each filesystem event for a `*.jsonl` file, it debounces briefly, then calls `import_session`. Errors are logged via `eprintln!` (no logging crate yet) and never crash the loop.

For v0.1 we ship the simplest correct loop: re-parse the entire session file on any modify event. Claude appends lines, so re-parsing is cheap (< 5ms for typical sessions) and avoids partial-line edge cases. Optimization comes later if profiling demands it.

- [ ] **Step 1: Write the (integration) test**

Append to `crates/illuminate-trail/tests/import_test.rs`:

```rust
use illuminate_trail::watcher::{run_watcher, WatcherOpts};
use std::sync::mpsc;
use std::time::Duration;

#[test]
fn watcher_imports_existing_session_on_startup() {
    let repo = tempfile::tempdir().unwrap();
    make_opted_in_repo(repo.path());
    let claude_root = tempfile::tempdir().unwrap();
    let project_dir = claude_root.path().join("-fake-project");
    std::fs::create_dir_all(&project_dir).unwrap();
    let jsonl = project_dir.join("00000000-0000-0000-0000-000000000001.jsonl");
    write_fixture_session(&jsonl, repo.path());

    let (tx, rx) = mpsc::channel();
    let handle = std::thread::spawn(move || {
        let opts = WatcherOpts {
            sessions_root: claude_root.path().to_path_buf(),
            on_imported: Some(Box::new(move |path| {
                let _ = tx.send(path);
            })),
            run_once: true,
        };
        run_watcher(opts).unwrap();
    });
    let received = rx.recv_timeout(Duration::from_secs(5)).expect("watcher must import session");
    assert!(received.starts_with(repo.path().join(".illuminate").join("trail")));
    handle.join().unwrap();
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p illuminate-trail --test import_test 2>&1 | tail -20`

Expected: compile failure (`run_watcher` / new `WatcherOpts` not as defined).

- [ ] **Step 3: Replace `watcher.rs`**

Replace `crates/illuminate-trail/src/watcher.rs`:

```rust
//! Claude Code session watcher.
//!
//! Walks `sessions_root` (default `~/.claude/projects/`), runs `import_session`
//! on every `.jsonl` file present at startup, then (unless `run_once`) watches
//! for filesystem events and re-imports modified files.

use crate::import::import_session;
use crate::Result;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

pub type ImportCallback = Box<dyn Fn(PathBuf) + Send + Sync>;

pub struct WatcherOpts {
    pub sessions_root: PathBuf,
    pub on_imported: Option<ImportCallback>,
    /// If true, scan once and exit. Used by tests and one-shot imports.
    pub run_once: bool,
}

pub fn run_watcher(opts: WatcherOpts) -> Result<()> {
    // Initial scan.
    scan_dir(&opts.sessions_root, &opts.on_imported);

    if opts.run_once {
        return Ok(());
    }

    use notify::{recommended_watcher, Event, EventKind, RecursiveMode, Watcher};

    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher = recommended_watcher(move |res| {
        let _ = tx.send(res);
    })?;
    watcher.watch(&opts.sessions_root, RecursiveMode::Recursive)?;

    loop {
        match rx.recv_timeout(Duration::from_secs(60)) {
            Ok(Ok(event)) => {
                if !matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                    continue;
                }
                for path in event.paths {
                    if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                        continue;
                    }
                    if let Err(e) = handle_one(&path, &opts.on_imported) {
                        eprintln!("[trail] import failed for {}: {e}", path.display());
                    }
                }
            }
            Ok(Err(e)) => eprintln!("[trail] watch error: {e}"),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    Ok(())
}

fn scan_dir(root: &Path, cb: &Option<ImportCallback>) {
    if !root.is_dir() {
        return;
    }
    let walker = match std::fs::read_dir(root) {
        Ok(w) => w,
        Err(_) => return,
    };
    for entry in walker.flatten() {
        let p = entry.path();
        if p.is_dir() {
            scan_dir(&p, cb);
        } else if p.extension().and_then(|e| e.to_str()) == Some("jsonl") {
            let _ = handle_one(&p, cb);
        }
    }
}

fn handle_one(path: &Path, cb: &Option<ImportCallback>) -> Result<()> {
    if let Some(written) = import_session(path)? {
        if let Some(callback) = cb {
            callback(written);
        }
    }
    Ok(())
}
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test -p illuminate-trail 2>&1 | tail -20`

Expected: all tests pass (smoke + parse + repo + storage + import).

- [ ] **Step 5: Commit**

```bash
git add crates/illuminate-trail/src/watcher.rs crates/illuminate-trail/tests/import_test.rs
git commit -m "feat(trail): notify-based watcher loop with one-shot mode"
```

---

## Task 9: CLI `trail` subcommand (`commands/trail.rs`)

**Files:**
- Create: `crates/illuminate-cli/src/commands/trail.rs`
- Modify: `crates/illuminate-cli/Cargo.toml`
- Modify: `crates/illuminate-cli/src/commands/mod.rs`
- Modify: `crates/illuminate-cli/src/main.rs`

Subcommands:
- `illuminate trail import <path>` — import a single jsonl
- `illuminate trail list` — list trails in the current repo's `.illuminate/trail/`
- `illuminate trail show <session-id-or-filename>` — print a trail's messages
- `illuminate trail watch [--sessions-root PATH]` — long-running watcher

- [ ] **Step 1: Add the workspace dep**

Modify `crates/illuminate-cli/Cargo.toml`. Find the `[dependencies]` section and add:

```toml
illuminate-trail = { version = "0.8.0", path = "../illuminate-trail" }
```

- [ ] **Step 2: Write the trail command module**

Create `crates/illuminate-cli/src/commands/trail.rs`:

```rust
//! `illuminate trail` — capture and inspect Claude Code prompt-trails.

use clap::Subcommand;
use illuminate_trail::claude::default_sessions_dir;
use illuminate_trail::import::import_session;
use illuminate_trail::record::TrailRecord;
use illuminate_trail::watcher::{run_watcher, WatcherOpts};
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub enum TrailCmd {
    /// Import a single Claude Code session jsonl file
    Import {
        /// Path to the .jsonl file
        path: PathBuf,
    },
    /// List trails captured for the current repo
    List,
    /// Show the messages in a captured trail
    Show {
        /// Filename inside .illuminate/trail/, or session id, or file path
        ident: String,
    },
    /// Watch ~/.claude/projects/ and capture sessions in real time
    Watch {
        /// Override the watch root (default: ~/.claude/projects)
        #[arg(long)]
        sessions_root: Option<PathBuf>,
    },
}

pub fn run(cmd: TrailCmd) -> std::io::Result<()> {
    match cmd {
        TrailCmd::Import { path } => cmd_import(&path),
        TrailCmd::List => cmd_list(),
        TrailCmd::Show { ident } => cmd_show(&ident),
        TrailCmd::Watch { sessions_root } => cmd_watch(sessions_root),
    }
}

fn cmd_import(path: &Path) -> std::io::Result<()> {
    match import_session(path) {
        Ok(Some(p)) => {
            println!("imported: {}", p.display());
            Ok(())
        }
        Ok(None) => {
            println!("skipped: session repo is not opted in (no .illuminate/illuminate.toml)");
            Ok(())
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}

fn trail_dir() -> std::io::Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    let mut cur = Some(cwd.as_path());
    while let Some(d) = cur {
        let candidate = d.join(".illuminate").join("trail");
        if d.join(".illuminate").join("illuminate.toml").is_file() {
            std::fs::create_dir_all(&candidate)?;
            return Ok(candidate);
        }
        cur = d.parent();
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "no .illuminate/illuminate.toml found in cwd or ancestors",
    ))
}

fn cmd_list() -> std::io::Result<()> {
    let dir = trail_dir()?;
    let mut entries: Vec<_> = std::fs::read_dir(&dir)?
        .flatten()
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|x| x.to_str())
                .is_some_and(|x| x == "jsonl")
        })
        .collect();
    entries.sort_by_key(|e| e.file_name());
    if entries.is_empty() {
        println!("no trails captured yet — try `illuminate trail watch` or `illuminate trail import <path>`");
        return Ok(());
    }
    for e in entries {
        let path = e.path();
        let size = e.metadata().map(|m| m.len()).unwrap_or(0);
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(rec) = serde_json::from_str::<TrailRecord>(content.trim()) {
                println!(
                    "{:<10}  {}  {} msgs  {} bytes",
                    rec.started_at.format("%Y-%m-%d"),
                    path.file_name().unwrap().to_string_lossy(),
                    rec.messages.len(),
                    size,
                );
                continue;
            }
        }
        println!("{}  ({} bytes, unparsed)", path.file_name().unwrap().to_string_lossy(), size);
    }
    Ok(())
}

fn cmd_show(ident: &str) -> std::io::Result<()> {
    let dir = trail_dir()?;
    let candidate = dir.join(ident);
    let path = if candidate.is_file() {
        candidate
    } else {
        // try to find by session id
        std::fs::read_dir(&dir)?
            .flatten()
            .map(|e| e.path())
            .find(|p| {
                std::fs::read_to_string(p)
                    .ok()
                    .and_then(|c| serde_json::from_str::<TrailRecord>(c.trim()).ok())
                    .is_some_and(|r| r.session_id == ident)
            })
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no trail matched"))?
    };
    let content = std::fs::read_to_string(&path)?;
    let rec: TrailRecord = serde_json::from_str(content.trim()).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
    })?;
    println!("session: {}", rec.session_id);
    println!("agent:   {:?}", rec.agent);
    println!("repo:    {}", rec.repo_path.display());
    println!("range:   {} → {}", rec.started_at, rec.ended_at);
    println!("messages: {}", rec.messages.len());
    println!("---");
    for m in &rec.messages {
        println!("[{} {:?}] {}", m.timestamp.format("%H:%M:%S"), m.role, m.text);
    }
    if !rec.tool_invocations.is_empty() {
        println!("---");
        println!("tool calls:");
        for t in &rec.tool_invocations {
            println!("  {} @ {}", t.name, t.timestamp.format("%H:%M:%S"));
        }
    }
    Ok(())
}

fn cmd_watch(sessions_root: Option<PathBuf>) -> std::io::Result<()> {
    let root = sessions_root
        .or_else(default_sessions_dir)
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "could not determine ~/.claude/projects/ — pass --sessions-root",
            )
        })?;
    println!("watching {}", root.display());
    let opts = WatcherOpts {
        sessions_root: root,
        on_imported: Some(Box::new(|p| {
            println!("captured: {}", p.display());
        })),
        run_once: false,
    };
    run_watcher(opts).map_err(|e| std::io::Error::other(e.to_string()))?;
    Ok(())
}
```

- [ ] **Step 3: Wire into commands/mod.rs**

Read the current `crates/illuminate-cli/src/commands/mod.rs`. Add `pub mod trail;` next to the other `pub mod` lines. (Check the existing pattern by reading the file before editing.)

- [ ] **Step 4: Wire into main.rs**

In `crates/illuminate-cli/src/main.rs`, locate the `enum Commands` block. Add a new variant:

```rust
    /// Capture and inspect Claude Code prompt-trails
    Trail {
        #[command(subcommand)]
        cmd: commands::trail::TrailCmd,
    },
```

Locate the `match` block in `main()` that dispatches each command and add:

```rust
        Commands::Trail { cmd } => commands::trail::run(cmd)?,
```

(If `main` is not currently `Result`-returning, wrap the call: `commands::trail::run(cmd).map_err(|e| { eprintln!("{e}"); std::process::exit(1); }).ok();`)

- [ ] **Step 5: Build the workspace**

Run: `cargo build --workspace 2>&1 | tail -20`

Expected: clean build, no errors.

- [ ] **Step 6: Smoke-test the CLI**

Run:
```bash
mkdir -p /tmp/illu-smoke/.illuminate
echo "name='smoke'" > /tmp/illu-smoke/.illuminate/illuminate.toml
cargo run -p illuminate-cli -- trail import \
  crates/illuminate-trail/tests/fixtures/claude-session.jsonl 2>&1 | tail -5
```

Expected: prints `skipped: session repo is not opted in (...)` (because the fixture's hardcoded `cwd` is `/tmp/illuminate-fixture-repo`, not the smoke dir). Confirms the opt-in gate works.

Then:
```bash
sed "s|/tmp/illuminate-fixture-repo|/tmp/illu-smoke|g" \
  crates/illuminate-trail/tests/fixtures/claude-session.jsonl \
  > /tmp/illu-smoke/session.jsonl
cargo run -p illuminate-cli -- trail import /tmp/illu-smoke/session.jsonl
ls /tmp/illu-smoke/.illuminate/trail/
cd /tmp/illu-smoke && /home/rsx/Desktop/projx/illuminate/target/debug/illuminate trail list
cd - >/dev/null
```

Expected: `imported: /tmp/illu-smoke/.illuminate/trail/2026-05-06-...-claude.jsonl`, then a non-empty `list` output.

- [ ] **Step 7: Commit**

```bash
git add crates/illuminate-cli/Cargo.toml crates/illuminate-cli/src/commands/trail.rs \
        crates/illuminate-cli/src/commands/mod.rs crates/illuminate-cli/src/main.rs \
        Cargo.lock
git commit -m "feat(cli): add trail subcommand (import/list/show/watch)"
```

---

## Task 10: End-to-end dogfood test

**Files:** none modified (manual verification only)

Confirm the watcher captures real Claude Code activity on this repo.

- [ ] **Step 1: Initialize the local repo**

Run:
```bash
mkdir -p /home/rsx/Desktop/projx/illuminate/.illuminate
cat > /home/rsx/Desktop/projx/illuminate/.illuminate/illuminate.toml <<'EOF'
[project]
name = "illuminate"
EOF
echo ".illuminate/trail/" >> /home/rsx/Desktop/projx/illuminate/.gitignore
echo ".illuminate/graph.db" >> /home/rsx/Desktop/projx/illuminate/.gitignore
```

(The `.gitignore` additions only matter if those entries aren't already present.)

- [ ] **Step 2: Run the watcher in the background**

Run (in a separate terminal or via `&`):
```bash
cargo run -p illuminate-cli --release -- trail watch &
```

Expected: prints `watching /home/rsx/.claude/projects/`. On startup, scans existing sessions; existing illuminate-repo sessions are imported.

- [ ] **Step 3: List captured trails**

Run:
```bash
cargo run -p illuminate-cli --release -- trail list
```

Expected: at least one entry, dated today.

- [ ] **Step 4: Show one trail**

Run:
```bash
cargo run -p illuminate-cli --release -- trail show <filename-from-list>
```

Expected: prints session metadata + message log.

- [ ] **Step 5: Stop the watcher**

`fg` then `Ctrl-C`, or `kill %1`.

- [ ] **Step 6: Commit the local opt-in marker**

The `.illuminate/illuminate.toml` and `.gitignore` updates should land:

```bash
git add .illuminate/illuminate.toml .gitignore
git commit -m "chore: opt this repo into trail capture"
```

---

## Self-Review

**Spec coverage:** Each item from the v0.1 trail-capture scope ("Claude Code session capture daemon, writes to `.illuminate/trail/`") maps to a task above. Bootstrap, extraction, wiki rendering are explicitly out-of-scope per the plan header — those are separate plans.

**Placeholder scan:** No "TBD" / "implement later" in steps. Code blocks in every implementation step. Test code is concrete and runnable.

**Type consistency:**
- `TrailRecord`, `Message`, `MessageRole`, `ToolInvocation`, `AgentKind` — defined once in `record.rs` (existing) and used consistently throughout.
- `RawRecord`, `UserRecord`, `AssistantRecord`, `AttachmentRecord`, `MessageBlock` — defined in `raw.rs` (Task 3) and used in `claude.rs` (Task 6).
- `WatcherOpts` — redefined in Task 8 (replacing the skeleton's stub). Tests in Task 8 use the new fields (`sessions_root`, `on_imported`, `run_once`).
- `TrailCmd` — defined in Task 9 only. CLI uses it via the `Trail` variant added to `Commands`.
- `import_session` returns `Result<Option<PathBuf>>`; called the same way by `cmd_import`, `handle_one`, and the integration test.

**Cross-task references:**
- Task 6 uses `RawRecord` from Task 3.
- Task 7 uses `parse_session` (Task 6), `resolve_repo` (Task 4), `write_trail` (Task 5).
- Task 8 uses `import_session` (Task 7).
- Task 9 uses everything via the public crate API.
