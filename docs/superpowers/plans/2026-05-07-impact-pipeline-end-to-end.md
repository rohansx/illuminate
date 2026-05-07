# Plan: end-to-end the impact pipeline

**Date:** 2026-05-07 (continuation — second autonomous batch)
**Workspace:** `/home/rsx/Desktop/projx/illuminate` master
**Predecessor:** plan `2026-05-07-cross-agent-coverage-and-edges.md` shipped 5 tasks (Cursor + Codex parsers, edges schema, Rust import edges, audit impact_radius integration). The final whole-branch reviewer flagged the impact pipeline as fully implemented and tested but **not reachable from any agent surface**: the CLI doesn't pass `--index-db`, the MCP `illuminate_audit` handler doesn't call `audit_with_files`, and the index rebuild doesn't call `index_file_with_edges` — so the edges table is always empty in production runs.

## Goals

1. Trigger edge extraction during the existing `illuminate rebuild` (or whichever indexing entrypoint exists) so `edges` populates alongside `symbols`.
2. Wire `Auditor::with_index` + `audit_with_files` into the CLI `illuminate audit` command via `--index-db` flag + positional file args.
3. Wire the same into the MCP `illuminate_audit` tool handler so agents see blast-radius.
4. Replace `eprintln!` calls in `crates/illuminate-trail/src/watcher.rs` with `tracing::warn!` / `tracing::error!` to match the `illuminate-audit` convention.
5. Fix `ARCHITECTURE.md` Capture section: Cursor uses `state.vscdb` (SQLite), not `~/.cursor/conversations/<session>.json`.

## Non-goals

- Per-language edge extraction beyond Rust (deferred).
- Token-count surfacing on `TrailRecord` (deferred).
- Splitting grouped `use std::{io, fs};` into multiple edges (deferred).
- Symbol-level (vs file-level) seeds for `impact_radius` (deferred — the file-level seeds match the import-edge producer).

## Conventions

- Rust 2024. `cargo fmt`, `cargo clippy --tests -- -D warnings` clean per-crate.
- TDD: tests first.
- Single-line lowercase commit messages, no Co-Authored-By, push to `origin/master` after each task.
- Pre-write step: `./target/release/illuminate audit "<plan>"` per CLAUDE.md.

## Tasks

### Task A — Trigger edge extraction during indexing

**File:** `crates/illuminate-index/src/indexer.rs` (and any callers).

**What to build.** Wherever the existing indexer iterates files and calls `index_file()`, switch to `index_file_with_edges()` and persist both the symbols AND the edges via `storage::upsert_symbols` + `storage::upsert_edges`. Don't break the existing API; if `index_file()` is exported and tested, leave it as a thin wrapper or keep it as the symbols-only path and add a new `index_with_edges()` driver alongside.

**Tests** (extend `crates/illuminate-index/tests/index_tests.rs` or add a new file):
- `rebuild_populates_edges_table` — index a tempdir containing a Rust file with `use foo::bar;` and assert `edges` table has at least 1 row.
- `rebuild_populates_no_edges_for_non_rust_file` — index a `.go` file (no extractor yet), assert `edges` empty but `symbols` non-empty.

**Deliverable.** Commit `feat(index): populate edges table during rebuild`. ≤ 1 day.

### Task B — CLI `illuminate audit --index-db` and file args

**Files:** `crates/illuminate-cli/src/commands/audit.rs` (existing CLI handler).

**What to build.** Read the existing handler. Add:
- `--index-db <PATH>` flag (default: `<repo>/.illuminate/index.db` if it exists, else `None`)
- Positional file args: `illuminate audit "plan text" file1.rs file2.rs ...`
- When index-db resolves and files are provided, construct `Auditor::with_index(...)` and call `audit_with_files(...)`. Otherwise fall back to existing `Auditor::new` + `audit(...)`.
- Print the impact section in human-readable form when non-empty (e.g., "Blast radius: N impacted symbols across M files: ..."). Don't change exit codes.

**Tests** (`crates/illuminate-cli/tests/`): one integration test that builds a tempdir repo with `.illuminate/index.db` populated, runs `illuminate audit "plan" file.rs`, asserts the output contains `impacted` (or whatever phrasing you choose).

**Deliverable.** Commit `feat(cli): audit with --index-db and file args surfaces blast radius`. ≤ half day.

### Task C — MCP `illuminate_audit` uses `audit_with_files`

**Files:** `crates/illuminate-mcp/src/tools.rs` (or wherever `illuminate_audit` is handled).

**What to build.** Find the handler. The MCP tool schema already accepts a `files` parameter (the architecture doc shows it in the audit lifecycle). The handler currently ignores `files` per the final reviewer. Wire:
- Read `files` from the MCP request params (array of strings, optional).
- If non-empty AND `index.db` is reachable, build `Auditor::with_index(...)` and call `audit_with_files(...)`.
- Include `impact: ImpactInfo` in the JSON response (already serializable via `AuditResult`).

**Tests.** If the MCP crate has handler tests (mock JSON-RPC envelopes), add one that exercises `files` param and asserts the response includes `impact`. Otherwise add a smoke test that just round-trips the schema.

**Deliverable.** Commit `feat(mcp): illuminate_audit surfaces impact for files arg`. ≤ half day.

### Task D — Watcher logging migration

**File:** `crates/illuminate-trail/src/watcher.rs`.

**What to build.** Add `tracing = { workspace = true }` to `illuminate-trail/Cargo.toml`. Replace the four `eprintln!` calls in `watcher.rs` with `tracing::warn!` (for recoverable errors) or `tracing::error!` (for genuine failures). Run existing trail tests to confirm nothing breaks.

**Deliverable.** Commit `refactor(trail): watcher uses tracing instead of eprintln`. < 1 hour.

### Task E — ARCHITECTURE.md Cursor path fix

**File:** `docs/ARCHITECTURE.md`.

**What to fix.** The Capture section (around line 447-450) shows Cursor's session storage as `~/.cursor/conversations/<session-id>.json (poll)`. Reality (per the just-shipped `cursor.rs`): Cursor uses `state.vscdb` — a single SQLite database under `~/Library/Application Support/Cursor/User/globalStorage/` on macOS, `~/.config/Cursor/User/globalStorage/` on Linux, `~/AppData/Roaming/Cursor/User/globalStorage/` on Windows.

Update the bullet to match reality and note that capture is via SQLite read (no inotify on the DB file itself; the watcher polls).

**Deliverable.** Commit `docs(architecture): cursor capture uses state.vscdb sqlite`. < 30 minutes.

### Task F — Final coordinator pass

Architect agent re-reads `ARCHITECTURE.md` (post-Task E) and the new code from Tasks A-D. Reports whether the impact pipeline is now reachable end-to-end. Read-only.

## Commit cadence

One commit per task. Push after each. Continue through all 6 without check-ins.

## Blocked / escalation triggers

- Indexer/CLI/MCP handler shape doesn't match what the prompt assumes → halt, surface specifics.
- Existing tests break in a way that suggests architectural drift → halt.
- Audit policy fires `block` for any task plan → halt.
