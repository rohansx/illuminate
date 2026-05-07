# Plan: cross-agent capture + edge extraction + audit integration

**Date:** 2026-05-07
**Workspace:** `/home/rsx/Desktop/projx/illuminate` (master, dirty allowed during execution)
**Authoring context:** Continuation of v0.1 work after commits `e39a5b7` (two-graph doc) and `b8a7995` (edges schema + impact_radius). User asked to keep working autonomously and run a periodic coordinator pass against the architecture docs.

## Goals

1. Lift Cursor and Codex session capture out of stub status, with format knowledge ported (not copied) from codeburn (MIT).
2. Wire the new `edges` table to a real producer: per-file Rust import edges extracted at index time. This proves the schema end-to-end without taking on call-resolution complexity yet.
3. Connect `impact_radius()` into `illuminate-audit` so an audit request reports blast-radius information for the files an agent proposes to touch.
4. Run a coordinator agent every ~2 tasks that reads the architecture docs and confirms the direction is still consistent with the spec.

## Non-goals

- Cross-language edge extraction. Rust imports first, others later. Calls/inheritance are deliberately deferred.
- MCP tool exposure of `impact_radius` directly. The audit lifecycle already routes through MCP; we wire impact through audit, not as a new top-level tool.
- Schema migrations for existing on-disk `index.db`. New columns are append-only via `CREATE TABLE IF NOT EXISTS` — fresh DBs only.

## Conventions

- Rust 2024 edition. `cargo fmt`, `cargo clippy --tests -- -D warnings` clean.
- TDD: tests first, must fail, then implementation.
- Single-line lowercase commit messages, no Co-Authored-By trailers, push to `origin/master` after each task that lands a commit.
- Format knowledge attribution: code-review-graph (MIT) for graph patterns, codeburn (MIT) for session parsers. Keep a one-line credit comment near the relevant code; the `docs/ARCHITECTURE.md` Related Projects section already covers the longer-form attribution.
- Before every source-file write, the implementer must run `./target/release/illuminate audit "<plan>"` per `CLAUDE.md` and surface the result. If the result is a violation, escalate.
- Do not use Redis. The repo has a policy that rejects it. (See `.illuminate/illuminate.toml`.)

## Tasks

### Task 1 — Cursor parser port (illuminate-trail/src/cursor.rs)

**File:** `crates/illuminate-trail/src/cursor.rs` (currently a 19-line stub).

**What to build.** A `parse_state_db(path: &Path) -> Result<Vec<TrailRecord>>` function that:

1. Opens the Cursor SQLite database at `path` (typically `~/Library/Application Support/Cursor/User/globalStorage/state.vscdb` on macOS; `~/.config/Cursor/User/globalStorage/state.vscdb` on Linux; `~/AppData/Roaming/Cursor/User/globalStorage/state.vscdb` on Windows). The default-path resolution lives in a small helper `default_state_db_path() -> Option<PathBuf>` so callers and tests can pass an explicit path.
2. Verifies the schema by checking that `cursorDiskKV` exists with at least one `key LIKE 'bubbleId:%'` row; returns `Err(TrailError::Parse("cursor schema not detected"))` otherwise.
3. Reads the `cursorDiskKV` table and decodes each `key LIKE 'bubbleId:%'` row's `value` JSON. Extract:
   - `tokenCount.inputTokens`, `tokenCount.outputTokens`
   - `modelInfo.modelName`
   - `createdAt`
   - `conversationId`
   - `text` (first 500 chars only — long bubbles need truncation per codeburn's note)
   - `type` (1 = user, otherwise assistant)
4. Reconstructs message ordering by `conversationId` + arrival `ROWID` and emits one `TrailRecord` per conversation.
5. Honors a `LOOKBACK_DAYS = 180` floor (filter `createdAt > now - 180d`).
6. For DBs over 250k bubble rows, uses the ROWID-cutoff trick from codeburn (`SELECT MIN(rid) FROM (SELECT ROWID rid FROM cursorDiskKV WHERE key LIKE 'bubbleId:%' ORDER BY ROWID DESC LIMIT 250000)`) so old sessions are skipped rather than the parse stalling for 30+ seconds.

**Tests** (in `crates/illuminate-trail/tests/cursor_tests.rs`, new file):

- `parses_minimal_two_message_conversation` — write a tempfile state.vscdb with two `bubbleId:<conv>:001` rows, verify two messages emitted in arrival order.
- `parses_skips_unrelated_keys` — DB has `agentKv:blob:*` and `bubbleId:*` rows; only bubbles surface.
- `unrecognized_schema_returns_parse_error` — DB has no `cursorDiskKV` table.
- `truncates_text_to_500_chars` — bubble with 1000-char text emits a record whose preview is ≤ 500 chars.
- `respects_lookback_days_floor` — bubble with createdAt 200 days ago is excluded.
- `bubble_type_1_marks_user_role` — conversation with type=1 user bubble produces a record where the user role is recognized.

**Reference.** `/tmp/codeburn/src/providers/cursor.ts` lines 100-260 for SQL queries and ROWID cutoff. Do not copy code — reimplement in idiomatic Rust using `rusqlite`.

**Deliverable.** `cursor.rs` with documented `parse_state_db`, default path helper, all 6 tests passing, clippy clean. Commit with message `feat(trail): cursor session capture from cursorDiskKV`.

### Task 2 — Codex parser port (illuminate-trail/src/codex.rs)

**File:** `crates/illuminate-trail/src/codex.rs` (currently a 13-line stub).

**What to build.** A `parse_session(path: &Path) -> Result<TrailRecord>` function and a `discover_sessions(codex_dir: &Path) -> Result<Vec<PathBuf>>` helper that:

1. Walks `<codex_dir>/sessions/YYYY/MM/DD/` and returns every `rollout-*.jsonl` file.
2. For each candidate, the parser reads the first JSONL line, validates it's a `payload.type == "session_meta"` record whose `payload.originator` starts with `"codex"`, and extracts `payload.cwd`. Reject otherwise with `TrailError::Parse`.
3. Streams remaining lines as JSONL, decoding each into the existing `RawRecord` machinery (User / Assistant / Attachment / Summary). Treat unknown record types as `RawRecord::Unknown(value)`.
4. Builds a `TrailRecord` with `agent: AgentKind::Codex`, the `cwd` from session_meta, and the message list.
5. The default codex dir resolves to `$CODEX_HOME` if set, else `~/.codex`. Provide a `default_codex_dir() -> Option<PathBuf>`.

**Tests** (`crates/illuminate-trail/tests/codex_tests.rs`):

- `discovers_rollout_files_in_dated_dirs` — create `sessions/2026/05/07/rollout-abc.jsonl` and `sessions/2026/05/06/rollout-def.jsonl`, verify both surface.
- `ignores_files_outside_yyyy_mm_dd_pattern` — `sessions/scratch/foo.jsonl` ignored.
- `parses_minimal_session` — file with session_meta line + 2 user/assistant lines yields a `TrailRecord` with 2 messages and cwd populated.
- `rejects_non_codex_session_meta` — first line has `originator: "claude-code"`, parse returns Err.
- `handles_unknown_record_types` — line with `type: "tool_use"` falls through to Unknown without crashing.

**Reference.** `/tmp/codeburn/src/providers/codex.ts` lines 70-180 for directory layout and validation logic.

**Deliverable.** `codex.rs` with documented `parse_session` + `discover_sessions`, all 5 tests passing, clippy clean. Commit `feat(trail): codex session capture from rollout-*.jsonl`.

### Task 3 — Rust import edges (illuminate-index)

**File:** `crates/illuminate-index/src/edge_extract.rs` (new). Extends existing symbol extraction with edge production for Rust files.

**What to build.** A pure function `extract_rust_edges(tree: &tree_sitter::Tree, source: &[u8], file_path: &str) -> Vec<Edge>` that:

1. Walks the AST looking for `use_declaration` nodes.
2. For each, computes the `source_qualified` as `file::<file_path>` (file-level node — we don't yet have function-scoped imports).
3. Computes the `target_qualified` as the dotted path text of the use statement (e.g., `std::collections::HashMap`).
4. Emits an `Edge { kind: EdgeKind::Imports, source_qualified, target_qualified, file_path, line }`.
5. Re-exports a top-level `index_file_with_edges()` from `lib.rs` that returns `(Vec<Symbol>, Vec<Edge>)` for callers.

**Tests** (`crates/illuminate-index/tests/edge_extract_tests.rs`):

- `extracts_single_use_decl` — `use foo::bar;` → 1 edge with target `foo::bar`.
- `extracts_multiple_use_decls` — three `use` lines → 3 edges.
- `handles_grouped_use` — `use std::{io, fs};` → at least 1 edge whose target contains `std`.
- `no_use_decls_yields_no_edges` — file with only a function → 0 edges.
- `index_file_with_edges_returns_both` — combined helper returns symbols and edges from same parse.

**Out of scope.** Per-language edge extraction for Go/Python/etc. is deferred. Only `Language::Rust` needs to wire through; other languages return empty edges.

**Deliverable.** `edge_extract.rs`, exported helper in `lib.rs`, 5 tests passing, clippy clean. Commit `feat(index): rust import edge extraction`.

### Task 4 — Coordinator pass (architect agent)

**Not a code task.** Read `docs/PRODUCT_OVERVIEW.md`, `docs/ARCHITECTURE.md` (especially the new "Two Graphs, One Audit" section), and the current state of `crates/illuminate-trail/`, `crates/illuminate-index/`, `crates/illuminate-audit/`. Report whether tasks 1-3 are still aligned with the architecture, and flag any drift. Output is text only — no code changes. This task runs after tasks 1, 2, 3 are complete (or before task 5 if they fail).

### Task 5 — Wire impact_radius into illuminate-audit

**File:** `crates/illuminate-audit/` (existing crate).

**What to build.** When an audit plan provides file paths, look those files up in `index.db`, gather symbols touching each file, and call `storage::impact_radius()` to compute blast-radius. Add the impacted symbol set as a new field on the audit response so the caller (CLI / MCP) can surface "this change ripples to N other symbols."

**Constraints.** Don't break existing audit semantics. The blast-radius is informational — it doesn't change pass/warn/block status. Wire as an optional field; if `index.db` is missing or the file isn't indexed, return an empty impact set, not an error.

**Tests.** Whatever the audit crate's existing test pattern is, mirror it. At minimum: an audit that touches a file with N known callers in the index DB returns those callers in the impact set.

**Deliverable.** Audit crate exposes blast-radius. Test added. Commit `feat(audit): include impact_radius in audit response`.

## Coordinator schedule

After **task 2** completes (Cursor + Codex done, before edges and audit work): dispatch architect agent with the read-only summary task above. If it surfaces drift, escalate to user before proceeding to task 3.

After **task 5** completes: final code-reviewer pass on the whole branch.

## Commit cadence

One commit per task. Push to `origin/master` after each commit. Don't batch.

## Blocked / escalation triggers

- Implementer cannot resolve a real bug in existing code → halt, surface to user.
- Tests fail in a way that suggests architectural drift (not just bugs) → halt, dispatch coordinator early, escalate.
- Audit policy fires `block` for any task plan → halt, surface to user.
