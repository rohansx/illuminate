# Plan: v0.7 — close remaining doc-vs-code drift

**Date:** 2026-05-08 (continuation after v0.6.0)
**Workspace:** `/home/rsx/Desktop/projx/illuminate` master, 509 tests passing.

## Goals

The v0.6 coordinator listed seven still-open drift items. This batch closes the three highest-leverage:

1. **Bootstrap git-history source** (HIGH leverage) — `illuminate-bootstrap` ships only ADR + agent-files importers; docs require git-history backfill. The git-history extractor exists in `illuminate-watch::git` (`ingest_commits`) — connect it to bootstrap as a third source.
2. **CLI exit-code alignment** (MEDIUM leverage) — `audit` and `audit-diff` return `0/1/2`; docs say `0/2/3`. One-line change but breaks CI integrators if rolled wrong; document the change in CHANGELOG with a migration note.
3. **Audit response richer surface** (MEDIUM leverage) — `wiki_url`, `policies_applied`, `trace_id` per `docs/AUDIT.md`. Skip `confidence` and `evidence` (those need more thought).

Plus small cleanups:

4. MCP `illuminate_get_wiki_page` response shape per docs.
5. `[trail]` and `[extraction]` config sections parsed and applied.

## Tasks

### Task CA — Bootstrap git-history source

**File:** `crates/illuminate-bootstrap/src/git_history.rs` (new), `crates/illuminate-bootstrap/src/orchestrate.rs` (extend).

**Goal.** When `illuminate bootstrap` runs, in addition to ADRs and agent files, walk `git log` over the last 6 months (default; configurable via `[bootstrap] history_months`) and extract decision-shaped commits as candidates.

**What counts as a "decision-shaped commit":**
- Commits whose message matches `(?i)^(decision|adopt|reject|switch|migrate|deprecate|remove|chose|use)\b` OR contain a body line matching that.
- Commits with conventional types `feat:` AND signal-bearing keywords ("instead of", "rather than", "we decided", "after debate", etc.).
- Skip merges (`is_merge_commit`).

**Implementation:**
1. Use `git2` crate (already in workspace deps for `illuminate-watch`). Iterate commits via `Repository::open(repo_root)?.revwalk()?`.
2. For each commit in the window, compute a `BootstrapCandidate` (the existing struct in `crates/illuminate-bootstrap/src/candidate.rs`):
   - `id_slug = format!("dec-bs-git-{}", &commit.id().to_string()[..8])`
   - `title = commit.summary()`
   - `body = commit.message_body()` (strip leading metadata)
   - `source_kind = "git_history"`
   - `confidence = 0.6` (lower than ADR; needs review)
   - `created_at = commit.time().seconds() → DateTime<Utc>`
3. Run through the existing dedup pipeline (`orchestrate.rs` already has content-hash dedup).
4. Pages with `confidence < auto_merge_threshold` route to `_review/` (existing behavior).

**Tests** in `crates/illuminate-bootstrap/tests/git_history_test.rs` (new):
- `extracts_decision_shaped_commits` — Set up tempdir git repo with 3 commits: one matching ("Decision: switch to PostgreSQL"), one not ("fix typo"), one merge. Assert only 1 candidate.
- `respects_history_months_window` — Commits older than the window are skipped.
- `git_history_candidates_route_to_review_below_threshold` — Default `confidence=0.6 < 0.7 threshold` → `_review/`.

**Deliverable.** Commit `feat(bootstrap): git-history source for decision-shaped commits`.

### Task CB — CLI exit codes 0/2/3 per docs

**Files:** `crates/illuminate-cli/src/commands/audit.rs`, `crates/illuminate-cli/src/commands/audit_diff.rs`.

**Current:** `Pass=0, Warning=1, Violation=2`.
**Target:** `Pass=0, Violation=2, Warning=3` (docs spec).

**Why this order?** `Violation` is the strict-blocker (CI fails); `Warning` is informational (CI may or may not block). Exit `2` traditionally means "blocking issue" (mirrors `git diff --check`). Exit `3` for warnings keeps it distinguishable.

**Implementation:**
1. Update both `commands/audit.rs::run` and `commands/audit_diff.rs::run` exit branches.
2. Update tests that assert exit codes (search the test files for `exit_code == 1` or `code(1)` or similar).
3. CHANGELOG note: BREAKING for CI integrators.

**Tests:** Existing exit-code tests get updated. Add or update:
- `audit_warning_exits_three`
- `audit_violation_exits_two`
- `audit_pass_exits_zero`

**Deliverable.** Commit `fix(cli): align audit exit codes with docs (warn=3 violation=2)`.

### Task CC — Audit response richer surface

**File:** `crates/illuminate-audit/src/response.rs`, `crates/illuminate-audit/src/lib.rs`.

**Goal.** Add three new optional fields:
- `wiki_url: Option<String>` — derived from policy_violations and decision_conflicts; the URL of the wiki page for the most-relevant decision (or None if no decision matched).
- `policies_applied: Vec<String>` — names of policies that were checked (regardless of whether they fired). Useful for debugging "why didn't my policy match?"
- `trace_id: String` — UUID v4 generated per audit call. Useful for correlating CI/MCP audits with logs.

```rust
pub struct AuditResult {
    ...existing fields,
    #[serde(default)]
    pub wiki_url: Option<String>,
    #[serde(default)]
    pub policies_applied: Vec<String>,
    #[serde(default)]
    pub trace_id: String,
}
```

**Implementation:**
1. Generate `trace_id` at the top of `Auditor::audit` via `uuid::Uuid::new_v4().to_string()`. Add `uuid` to `illuminate-audit/Cargo.toml` deps if not present.
2. Track applied policies: `policies_applied = self.policies.iter().map(|p| p.name().to_string()).collect()`. Need a `name()` method on `IntentPolicy` enum if not present.
3. `wiki_url`: from the first `policy_violation` or first `relevant_decision`, look at the wiki page id and construct `"<repo>/.illuminate/wiki/decisions/<id>.md"` or similar. For v0.7, just use the file path as the URL (no actual HTTP server). If no match, leave `None`.

**Tests:**
- `audit_response_includes_trace_id` — every audit call returns a non-empty `trace_id`.
- `audit_response_lists_applied_policies` — set up auditor with 2 policies, audit, assert `policies_applied.len() == 2`.

**Deliverable.** Commit `feat(audit): add trace_id, policies_applied, wiki_url to AuditResult`.

### Task CD — MCP `illuminate_get_wiki_page` response shape per docs

**File:** `crates/illuminate-mcp/src/tools.rs`.

**Goal.** Current shape: `{ id, content, path }`. Docs spec: `{ id, type, title, front_matter, body }`.

**Implementation:**
1. Parse the wiki page front-matter via `illuminate_wiki::page::parse_page` (already exists).
2. Return JSON: `{ id, type, title, front_matter: <object>, body: <markdown> }`.
3. Keep `path` as an additional field for debugging (not required by spec but useful).

**Tests:** Update existing `get_wiki_page_returns_markdown_content` test in `missing_tools_test.rs` to also check `front_matter.title` is present.

**Deliverable.** Commit `fix(mcp): align get_wiki_page response shape with docs/MCP.md`.

### Task CE — `[trail]` and `[extraction]` config sections

**Files:** `crates/illuminate-audit/src/policy.rs` (extend `parse_audit_config` to also parse `[trail]` / `[extraction]`, OR add `parse_trail_config` and `parse_extraction_config` siblings).

**Sections to support:**
```toml
[trail]
enabled = true
purge_after_days = 90
exclude_patterns = ["*.env", "secrets/**"]

[extraction]
signal_threshold = 0.7
confidence_threshold = 0.5
```

**Implementation:**
1. Add `TrailConfig { enabled, purge_after_days, exclude_patterns }` with sensible defaults (enabled=true, purge=180, no excludes).
2. Add `ExtractionConfig { signal_threshold, confidence_threshold }` with defaults (0.7, 0.5).
3. Add `parse_trail_config(toml)` and `parse_extraction_config(toml)` siblings to `parse_audit_config`. Same tolerance pattern: missing fields → defaults, wrong types → defaults + warn.
4. Wire into the right consumers:
   - `TrailConfig`: trail watcher should respect `enabled` and `exclude_patterns`. Find `trail watch` in CLI and check.
   - `ExtractionConfig`: pipeline construction should use `confidence_threshold` instead of hardcoded `0.5`. The existing `try_attach_extraction` calls `load_extraction_pipeline` — pass through.
5. For v0.7, **at minimum just parse and emit**. Wiring to consumers can be a separate task if the surfaces are messy.

**Tests:**
- `parse_trail_config_reads_enabled_and_purge`
- `parse_extraction_config_reads_thresholds`

**Deliverable.** Commit `feat(audit): parse [trail] and [extraction] config sections`.

### Task CF — Final coordinator + tag v0.7.0

Architect agent reviews everything. Confirms the loop still works. Writes CHANGELOG section. Tags v0.7.0.

## Conventions

- Rust 2024. `cargo fmt --all` clean. `cargo clippy --workspace --all-targets -- -D warnings` clean. CI uses `--all-targets` so always run that locally.
- TDD strict.
- Single-line lowercase commit messages, no Co-Authored-By, push to `origin/master` after each task.
- Pre-write step: `./target/release/illuminate audit "<plan>"` before any source modification.

## Order

CA → CB → CC → CD → CE → CF.

CA is the biggest. CB-CE are smaller cleanups but each closes a real doc-promise. CF tags + ships.
