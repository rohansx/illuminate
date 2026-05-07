# Plan: symbol-level seeds, language coverage, token surfacing, impact CLI

**Date:** 2026-05-07 (third autonomous batch — 6-hour run)
**Workspace:** `/home/rsx/Desktop/projx/illuminate` master
**Predecessor:** plans `2026-05-07-cross-agent-coverage-and-edges.md` and `2026-05-07-impact-pipeline-end-to-end.md` shipped the impact pipeline end-to-end with Rust + Go + TypeScript + Python edge coverage. CI is now green after `bacb2f5` workspace-wide fmt fix.

## Goals (all four follow-ups identified by the closing coordinator)

1. **Symbol-level seeds** for `impact_radius` — current file-level seeds (`file::<path>`) miss within-file granularity. Audit response should surface the actual symbols inside touched files.
2. **Java + C + C++ extractors** — round out static-import coverage across languages already supported by `illuminate-index` symbol extraction (currently 6 languages: Rust, Go, TS, Python, Java, C; only first 4 have edges).
3. **Token counts on `TrailRecord`** — `cursor.rs` already extracts `tokenCount.inputTokens`/`outputTokens` into `BubbleRow` but they're stored as `#[allow(dead_code)]`. Surface them on the public record so future cost-attribution analytics has a path.
4. **`illuminate impact <file>` CLI** — surface the code graph directly. Given a file (or set of files), print the symbols defined in it, the imports out of it, and the blast-radius set. Read-only inspection tool for dev-debugging.

## Non-goals

- Within-function call resolution (genuinely hard per-language; calls/inheritance edges deferred to v0.5+).
- TypeScript-style `import type` distinction (already handled identically; no semantic split).
- Cost-attribution analytics on top of token counts (deferred — this is just the plumbing).
- Vector/visual graph render (the impact CLI prints text; visualizer is a v0.5+ separate feature).

## Conventions

- Rust 2024. `cargo fmt --all` (workspace-wide), `cargo clippy --workspace --tests -- -D warnings` clean before push.
- TDD strict.
- Single-line lowercase commit messages, no Co-Authored-By, push to `origin/master` after each task.
- Pre-write step: `./target/release/illuminate audit "<plan>"`.

## Tasks

### Task K — Java import edge extraction

**Files:**
- `crates/illuminate-index/src/edge_extract.rs` — add `extract_java_edges()`.
- `crates/illuminate-index/src/lib.rs` — extend dispatch for `Language::Java`.
- `crates/illuminate-index/tests/edge_extract_tests.rs`.

**Java import grammar (tree-sitter-java):**
- `import com.foo.Bar;` — `import_declaration` containing a `scoped_identifier` `com.foo.Bar`.
- `import static com.foo.Bar.method;` — `import_declaration` with `static` modifier; the path is still a `scoped_identifier`.
- `import com.foo.*;` — wildcard form, still `scoped_identifier` plus `asterisk`.

**Approach.** Walk for `import_declaration` nodes; find the `scoped_identifier` child; emit one edge per import with target = the identifier text (e.g., `"com.foo.Bar"`). For wildcard, target ends with `.*` — keep verbatim.

**Tests:** `extracts_simple_java_import`, `extracts_static_java_import`, `extracts_wildcard_java_import`, `extracts_multiple_java_imports`, plus `index_file_with_edges_returns_java_imports`.

**Update** existing `rebuild_populates_no_edges_for_java_file` test (in index_tests.rs) — point at C instead.

**Deliverable.** Commit `feat(index): java import edge extraction`.

### Task L — C import edge extraction

**Approach.** C uses `#include` preprocessor directives. tree-sitter-c emits `preproc_include` nodes containing either a `string_literal` (`"foo.h"`) or `system_lib_string` (`<stdio.h>`).

**Tests:** `extracts_quoted_c_include`, `extracts_system_c_include`, `extracts_multiple_c_includes`.

**Deliverable.** Commit `feat(index): c include edge extraction`.

### Task M — C++ import edge extraction

C++ shares the C tree-sitter-c grammar in this repo (`Language::C` covers both per `lib.rs:51`). C++ adds `using namespace`, `import` (C++20 modules), and `#include`. v0.4 scope: `#include` only (the path already taken by C). Verify the existing C extractor handles `.cpp/.cc/.cxx/.hpp` files. If yes, just extend the test suite to assert C++ imports work.

**Deliverable.** If C and C++ share extractor: commit `test(index): cover c++ headers in c include extractor`. Otherwise add a separate `extract_cpp_edges`.

### Task N — Token counts on TrailRecord

**Files:**
- `crates/illuminate-trail/src/record.rs` — add optional fields:
  ```rust
  pub struct TrailRecord {
      ...
      #[serde(default)]
      pub input_tokens: Option<u64>,
      #[serde(default)]
      pub output_tokens: Option<u64>,
  }
  ```
- `crates/illuminate-trail/src/cursor.rs` — populate from `BubbleRow.input_tokens` / `output_tokens` (already extracted; just plumb to the record). Sum across messages in a conversation.
- `crates/illuminate-trail/src/claude.rs` — Claude session JSONL has token usage per message (`usage.input_tokens`, `usage.output_tokens` on assistant records). Extract and sum.
- `crates/illuminate-trail/src/codex.rs` — Codex session_meta or per-message; extract if present, otherwise leave None.

**Tests:** Update existing parser tests to assert the new fields are populated when source has them; add `tokens_default_to_none_when_absent`.

**Deliverable.** Commit `feat(trail): surface input/output token counts on TrailRecord`.

### Task O — Symbol-level seed enrichment in audit response

**File:** `crates/illuminate-audit/src/lib.rs`.

**Goal.** Currently `compute_impact` builds `seed_symbols` as `["file::src/foo.rs"]`. Enrich it: for each file, also lookup the symbols in that file from `index.db` and add their qualified names. The qualified-name format follows `<file_path>::<symbol_name>` (e.g., `src/foo.rs::process_payment`) — this is a NEW convention but consistent with how `index_file_with_edges` could emit symbol-scoped edges later.

**Approach.**
1. After `build_seed_qualifiers` builds the file-level seeds, add a second pass: for each file, call `illuminate_index::storage::lookup_file(&conn, &path)` to get `Vec<Symbol>`. Append `format!("{}::{}", file_path, sym.name)` to the seeds.
2. The `impact_radius` BFS still runs over the same edges — but now it can match either file-level or symbol-level qualified names. For v0.4, no edges target symbol-level names (extractors still emit file-level), so the symbol seeds will only appear in `seed_symbols` (informational), not generate new `impacted_symbols`.
3. **Critical:** the response now distinguishes "files I touched" from "symbols I touched in those files." Add field `defined_symbols: Vec<String>` on `ImpactInfo` (back-compat via `#[serde(default)]`). Move symbol-level qualified names there. Keep `seed_symbols` as the file-level pseudo-nodes.

```rust
pub struct ImpactInfo {
    pub seed_symbols: Vec<String>,        // existing — file-level pseudo-nodes
    #[serde(default)]
    pub defined_symbols: Vec<String>,     // NEW — symbols inside touched files
    pub impacted_symbols: Vec<String>,    // existing — BFS reachable
    pub truncated: bool,
}
```

**Tests in `audit/tests/impact_tests.rs`:**
- `audit_with_files_includes_defined_symbols` — populate `index.db` with symbols (use `CodeIndex::index_project` against a tempdir Rust file with two functions), call `audit_with_files`, assert `impact.defined_symbols` contains both function-qualified names.
- `audit_with_files_defined_symbols_empty_when_index_missing` — no index, defined_symbols stays empty, no error.
- `audit_with_files_defined_symbols_uses_qualified_format` — assert format `<rel_path>::<name>`.

**Deliverable.** Commit `feat(audit): enrich impact response with defined_symbols from index`.

### Task P — `illuminate impact <file>` CLI command

**Files:**
- `crates/illuminate-cli/src/main.rs` — new clap subcommand variant `Impact { files: Vec<PathBuf>, json: bool, depth: Option<u32>, max_nodes: Option<usize> }`.
- `crates/illuminate-cli/src/commands/impact.rs` — new file. Implements the inspection.

**Behavior.** Given `illuminate impact src/foo.rs src/bar.rs`:
1. Resolve `index.db` via the shared `resolve_index_db_from_cwd(None)` helper.
2. For each file: print the symbols defined (via `lookup_file`), the outgoing import edges (via `lookup_outgoing` on `file::<path>`), and the impact-radius result.
3. Output format: human-readable by default; `--json` flag emits `{ files: [{path, defined: [...], imports: [...], impact: {...}}] }`.
4. Doesn't require a plan — pure read-only inspection. Doesn't apply policies. Doesn't change graph or index.

**Tests in `crates/illuminate-cli/tests/`:**
- `impact_prints_symbols_and_imports` — populate index.db, run `target/release/illuminate impact <file>`, assert output mentions defined symbols and imports.
- `impact_no_index_db_prints_helpful_message` — no index, print "no index.db found at ..., run `illuminate index` first" to stderr, exit 0.

**Deliverable.** Commit `feat(cli): impact command for inspecting file blast radius`.

### Task Q — Final coordinator pass

Architect agent reads PRODUCT_OVERVIEW, ARCHITECTURE, the new code from K-P, and reports whether v0.4 is coherent. Lists any remaining drift or gaps.

## Commit cadence

One commit per task. Push after each. Run continuously through K-Q without check-ins.

## Blocked / escalation triggers

- Any pre-write `illuminate audit` returns `block` → halt, surface to user.
- Workspace tests regress (was 394 before this batch starts) → halt, dispatch fix-implementer.
- Architectural drift signal from coordinator → halt.
