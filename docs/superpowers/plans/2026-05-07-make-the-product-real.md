# Plan: make the product real — close the docs-vs-code gap

**Date:** 2026-05-07
**Workspace:** `/home/rsx/Desktop/projx/illuminate` master, 483 tests passing, v0.5.0 tagged.

## Context

A docs audit (architect, prior turn) found one HIGH-leverage gap and two MEDIUM ones:

1. **HIGH:** `illuminate trail register` and `failures register` skip NER extraction. The marquee capture mechanism stores raw episodes only, so the graph never gets entities the audit can match against. The semantic-similarity audit path is also unwired (`Auditor` has no `EmbedEngine`).
2. **MEDIUM:** MCP tool surface drifts from `docs/MCP.md` (missing `illuminate_decisions_for`, `illuminate_failures_for`, `illuminate_get_wiki_page`).
3. **MEDIUM:** CLI surface drifts from `docs/CLI.md` (missing `audit-diff`, `decisions for <path>`).

This plan closes the HIGH gap first (the loop becomes real) then ships the cheap MEDIUM cleanups.

## Tasks

### Task BA — Wire `ExtractionPipeline` into `trail register` and `failures register`

**Files:**
- `crates/illuminate-cli/src/commands/trail.rs::cmd_register`
- `crates/illuminate-cli/src/commands/failures.rs::cmd_register`
- `crates/illuminate-cli/Cargo.toml` — add `illuminate-extract = { path = "../illuminate-extract" }` if not already present.

**Goal.** When `illuminate trail register <file>` runs, it should:
1. Resolve the models directory (mirror the helper in `crates/illuminate-cli/src/commands/mcp.rs::find_models_dir`).
2. Build an `ExtractionPipeline::with_defaults(models_dir)`. If models are absent, log a `tracing::warn!` and fall back to the existing raw-episode storage path. **Don't fail.**
3. For each `Message` in the trail record (or for the concatenated body, whichever the existing path uses), call `pipeline.extract(text, reference_time)`. Aggregate `entities` and `relations` across messages.
4. Persist via the existing graph API: the entities/relations from extraction need to land as `Episode` metadata or via `Graph::add_extracted` (or whichever method exists; check `crates/illuminate-core/src/graph.rs` for the right call). If no such method exists, fall through to a manual append using `add_episode` with the raw text PLUS the extracted entities serialized into the episode's metadata.
5. Same logic for `failures::cmd_register`.

**Reference helper.** Look at `crates/illuminate-cli/src/commands/mcp.rs::find_models_dir` (around line 20-50) — it ascends from cwd looking for `~/.cache/illuminate/models`, then `<repo>/.illuminate/models`. Reuse that pattern verbatim. Extract to a shared helper in `crates/illuminate-cli/src/commands/mod.rs` if both `trail.rs`, `failures.rs`, and `mcp.rs` need it.

**Read context first.**
- `crates/illuminate-cli/src/commands/trail.rs` — `cmd_register` (around lines 200-280).
- `crates/illuminate-cli/src/commands/failures.rs` — `cmd_register`.
- `crates/illuminate-cli/src/commands/mcp.rs::find_models_dir` (lines 19-50) for the resolver pattern.
- `crates/illuminate-extract/src/pipeline.rs` — `ExtractionPipeline::with_defaults`, `extract`.
- `crates/illuminate-core/src/graph.rs` — find an `add_episode_with_extraction` or similar; otherwise `add_episode` + manual entity insert.

**Tests.**
- `trail_register_with_models_extracts_entities` — Skip if no models present locally (`if !models_dir.is_dir() { return; }`). Otherwise, register a small TrailRecord with text containing "Redis" / "PostgreSQL" / etc. (entities from the default schema). Assert post-register graph has at least one entity matching.
- `trail_register_without_models_falls_back_silently` — Force a missing-models path (point the resolver to a tempdir). Assert exit 0 and the raw episode lands.
- Same two for `failures::cmd_register`.

**Deliverable.** Commit `feat(trail,failures): wire extraction pipeline into register commands`.

### Task BB — Add `EmbedEngine` to `Auditor` for semantic top-k

**Files:**
- `crates/illuminate-audit/src/lib.rs`
- `crates/illuminate-audit/src/response.rs`
- `crates/illuminate-audit/Cargo.toml` — add `illuminate-embed = { path = "../illuminate-embed" }` if not present.
- `crates/illuminate-cli/src/commands/audit.rs` — wire embed engine.
- `crates/illuminate-mcp/src/tools.rs` — wire embed engine (it already has one for find_precedents).

**Goal.** Plan-text → embed → top-k decisions/patterns/failures. Merge into `AuditResult.relevant_decisions` (new field).

**Schema additions in `response.rs`:**
```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RelevantDecision {
    pub episode_id: String,
    pub content_preview: String,
    pub source: Option<String>,
    pub recorded_at: chrono::DateTime<chrono::Utc>,
    pub similarity: f32,
}

pub struct AuditResult {
    ...
    #[serde(default)]
    pub relevant_decisions: Vec<RelevantDecision>,
}
```

**`Auditor` API additions:**
```rust
pub fn with_index_root_and_embed(
    graph, policies, index_db_path, repo_root,
    embed: Option<Arc<EmbedEngine>>,
    semantic_top_k: usize,           // 0 disables
    semantic_threshold: f32,         // similarity floor (e.g., 0.6)
) -> Self
```

The existing constructors stay back-compat (forward to this with `embed: None`, `top_k: 0`).

**Audit logic:**
1. After existing decision-conflict check, if `embed.is_some()` AND `semantic_top_k > 0`: embed plan, call `Graph::search_fused(plan, top_k)` (or whatever the existing semantic search is — confirm; `find_precedents` in MCP uses it).
2. Filter results above `semantic_threshold`.
3. Convert to `RelevantDecision` and stuff into `AuditResult.relevant_decisions`.
4. **Doesn't change `status`** — informational only, like impact.

**CLI/MCP wiring.**
- CLI `audit.rs::run`: build `EmbedEngine` if models dir resolves; pass into `Auditor::with_index_root_and_embed`. Print "Related decisions" section in human output (mirror the existing FTS5 fallback block).
- MCP `tools.rs::illuminate_audit`: same — build embed engine at handler init (or reuse from `ToolContext`).

**Tests.** In `crates/illuminate-audit/tests/`:
- `audit_with_embed_surfaces_relevant_decisions` — set up graph with a decision episode containing "Redis caching", call audit with plan "add caching layer", assert `relevant_decisions` non-empty (skip if models absent).
- `audit_without_embed_returns_empty_relevant_decisions` — `Auditor::with_index_and_root` (no embed), assert `relevant_decisions.is_empty()`.

**Deliverable.** Commit `feat(audit): semantic top-k via EmbedEngine for relevant_decisions surface`.

### Task BC — Read `[audit] semantic_top_k` and `semantic_threshold` from `illuminate.toml`

**Files:**
- `crates/illuminate-audit/src/policy.rs` (or wherever `illuminate.toml` parsing lives — the policy parser may have access)
- `crates/illuminate-cli/src/commands/audit.rs::load_policies` — if the existing toml parser ignores `[audit]`, extend it to ALSO return audit config.

**Goal.** When `illuminate.toml` has:
```toml
[audit]
semantic_top_k = 5
semantic_threshold = 0.6
```
the CLI/MCP picks up these values and passes them to `Auditor::with_index_root_and_embed`. Defaults: top_k=5, threshold=0.6.

**Tests.** In audit/tests:
- `audit_config_reads_semantic_top_k_from_toml` — parse a sample `[audit] semantic_top_k = 7` toml, assert the audit config has `top_k == 7`.

**Deliverable.** Commit `feat(audit): read semantic_top_k and semantic_threshold from illuminate.toml`.

### Task BD — MCP missing tools

**File:** `crates/illuminate-mcp/src/tools.rs` — handler implementations + schema additions.

**Tools to add (all read-only on the existing graph):**
1. `illuminate_decisions_for` — input: `{ path: string }`. Returns decisions whose `affected_files` (or `metadata.files`) contains the path. Implementation: `Graph::list_entities`-style query. Mirror the existing `find_precedents` shape for the response.
2. `illuminate_failures_for` — input: `{ path: string }` or `{ module: string }`. Returns failure episodes referencing the path/module. Filter by `episode.source == Some("reflexion")` or whatever convention `illuminate-reflect` uses.
3. `illuminate_get_wiki_page` — input: `{ id: string }`. Returns the markdown content of the wiki page with that id. Implementation: use `illuminate-wiki::walk` or read from `<repo>/.illuminate/wiki/<dir>/<id>.md` directly.

Add each to the `tools/list` schema declaration (around line 945+) and dispatch in `tools/call`.

**Tests.** Add to `crates/illuminate-mcp/tests/` (mirror existing test pattern, e.g., `audit_impact_test.rs`):
- `decisions_for_returns_matching_decisions`
- `failures_for_returns_matching_failures`
- `get_wiki_page_returns_markdown_content`

**Deliverable.** Commit `feat(mcp): add decisions_for / failures_for / get_wiki_page tools per MCP.md`.

### Task BE — CLI `audit-diff` and `decisions for <path>`

**Files:**
- `crates/illuminate-cli/src/main.rs` — clap variants.
- `crates/illuminate-cli/src/commands/audit_diff.rs` (new).
- `crates/illuminate-cli/src/commands/decisions.rs` (new — or extend existing `decisions` if present).

**`illuminate audit-diff [base]`:**
- Default base: `HEAD~1`.
- Run `git diff --name-only <base>...HEAD` to get changed files.
- For each file, run `audit_with_files(plan_text="changes since <base>", &[file])`.
- Aggregate impact + violations across files. Print summary.
- Exit code: 0 if no violations across all files, 2 if any violation.

**`illuminate decisions for <path>`:**
- Resolve graph.
- List decisions whose metadata or affected_files contains the path (or fuzzy match on path components).
- Print human-readable list.

**Tests.** In `crates/illuminate-cli/tests/`:
- `audit_diff_runs_audit_for_each_changed_file` — set up tempdir git repo with two files, modify one, run `illuminate audit-diff`, assert output mentions only the modified file.
- `decisions_for_path_lists_relevant_episodes` — populate graph with a decision affecting `src/payments/`, run `illuminate decisions for src/payments/cache.rs`, assert decision appears.

**Deliverable.** Commit `feat(cli): add audit-diff and decisions-for-path commands per CLI.md`.

### Task BF — Final coordinator + tag v0.6.0

Coordinator reads docs vs code one more time. Confirms the HIGH gap (extraction wiring) is closed and the MEDIUM gaps are addressed. Lists remaining drift for v0.7+. Tag v0.6.0 with CHANGELOG section.

## Conventions

- Rust 2024. `cargo fmt --all` clean. `cargo clippy --workspace --all-targets -- -D warnings` clean. CI uses `--all-targets` so always run that locally.
- TDD strict.
- Single-line lowercase commit messages, no Co-Authored-By, push to `origin/master` after each task.
- Pre-write step: `./target/release/illuminate audit "<plan>"` before any source modification.

## Order

BA → BB → BC → BD → BE → BF. Each task self-contained; BA unlocks the value of BB (graph has entities to match against).
