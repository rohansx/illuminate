# Changelog

All notable changes to Illuminate are tracked here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.5.0] — 2026-05-07

### Added — function-call edges across 4 languages, path normalization, cache-bucket tokens

- **Path normalization in `audit_with_files`.** New `Auditor::with_index_and_root` constructor accepts an optional `repo_root: Option<PathBuf>`. When set, absolute file paths are normalized to repo-relative form before `lookup_file` and seed building, eliminating silent empty-result bugs when CLI/MCP callers pass absolute paths. Existing `Auditor::new` and `Auditor::with_index` signatures unchanged (back-compat). New `resolve_repo_root_from_cwd()` helper mirroring `resolve_index_db_from_cwd`. CLI and MCP both wire through. (`crates/illuminate-audit/src/lib.rs`, `crates/illuminate-cli/src/commands/audit.rs`, `crates/illuminate-mcp/src/main.rs`)
- **Cache-bucket token fields on `TrailRecord`.** `cache_creation_input_tokens` and `cache_read_input_tokens` are now optional fields on both `UsageBlock` (raw) and `TrailRecord` (normalized). Anthropic accurate cost math is now possible: spend = `input × input_rate + output × output_rate + cache_creation × cache_creation_rate + cache_read × cache_read_rate`. Cursor and Codex leave the cache fields as `None` (no Anthropic-style cache buckets in those formats). `#[serde(default)]` for back-compat. (`crates/illuminate-trail/src/{raw.rs,record.rs,claude.rs}`)
- **Rust function-call edges.** `extract_rust_call_edges()` walks `function_item` → `call_expression` and emits `Edge { kind: Calls }` per call site. Source qualifier `<file>::<fn_name>`; target is the literal text of the call's function-path child (`bar`, `module::bar`, `x.method`, `Type::associated`). `self`/`crate`/`super` resolution deferred. Macro invocations excluded (they're `macro_invocation` nodes, not `call_expression`). Two-stage walker prevents double-attribution from nested `function_item`. (`crates/illuminate-index/src/edge_extract.rs`)
- **Go function-call edges.** Same shape as Rust. Walks `function_declaration` and `method_declaration` → `call_expression`. Anonymous `func_literal` calls attribute to the enclosing named function (their lexical scope). Method receivers resolve via `child_by_field_name("name")` returning `field_identifier`. Selector calls (`r.m()`) emit target `r.m` literal text.
- **TypeScript function-call edges.** Single recursive walker threading `enclosing_fn_name: Option<&str>` through children. Arrow functions transparent (calls inside arrow attribute to enclosing named fn; module-level arrows use `file::<path>` pseudo-node source). Class methods use the bare method name (no `Class::` prefix; recoverable via `Symbol` lookup). `function_declaration` and `method_definition` introduce new enclosing-fn slots. Member expressions (`obj.method()`) and subscript expressions (`obj[key]()`) emit literal text targets.
- **Python function-call edges.** Same single-walker pattern as TS. tree-sitter-python uses bare `call` node kind (not `call_expression`). `lambda` is transparent. Module-level calls attribute to `file::<path>`. Class methods drop class prefix. Decorated functions (`decorated_definition` wrapping `function_definition`) work transparently because the recursive walk descends into the wrapper.
- **Symbol-level seeds in `impact_radius`.** `Auditor::compute_impact` now seeds the BFS with both file-level pseudo-nodes (`file::<path>`) AND symbol-level qualifiers (`<path>::<sym>`) for every symbol returned by `lookup_file`. This is what makes the new Calls edges reachable: a seed `src/foo.rs::process_payment` follows outgoing Calls edges (callees) and incoming Calls edges (callers) via the recursive-CTE BFS. Without this, the v0.5 Calls edges would be stored but never traversed by audits. Forward-chain test (`a → b → c` in one file) confirms the lift end-to-end. (`crates/illuminate-audit/src/lib.rs`)

### Fixed

- Workspace-wide `cargo clippy --all-targets -- -D warnings` now passes — fixed `clippy::unnecessary_sort_by` in `illuminate-wiki/src/render.rs` (replaced with `sort_by_key(std::cmp::Reverse(...))`).
- `edge_extract.rs` module doc said "v0.5"; corrected to "v0.4" before v0.4.0 tag.

### Deferred to v0.6+

- Java + C function-call edge extractors (currently Java/C are imports-only; calls extend the matrix).
- Symbol-resolution pass (`self`/`crate`/`super` and TS class-prefix qualifiers — currently all literal text).
- Anthropic ephemeral 5m / 1h cache TTL split (currently collapsed into single `cache_creation_input_tokens`).
- `crates/illuminate-cli/src/commands/hook.rs` audit-hook wires through `Auditor::with_index_and_root` for impact in the PreToolUse path.
- Cost-attribution analytics consumer for the new token fields.

## [0.4.0] — 2026-05-07

### Added — impact pipeline + multi-language edge coverage

- **Cursor session capture** via `state.vscdb` SQLite (`cursorDiskKV` table). Format knowledge ported from codeburn (MIT). New: `default_state_db_path()`, `parse_state_db()`. Handles bubble JSON, ROWID cutoff for >250k row DBs, lookback-days floor, token-count extraction. (`crates/illuminate-trail/src/cursor.rs`)
- **Codex session capture** via `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`. Format knowledge ported from codeburn (MIT). New: `default_codex_dir()`, `discover_sessions()`, `parse_session()`. Streams via `BufReader`, clamps `ended_at >= started_at` for clock-skew resilience. (`crates/illuminate-trail/src/codex.rs`)
- **Edges schema in `index.db`** — `edges(source_qualified, target_qualified, kind, file_path, line)` with indexes on source/target/kind/file. Bidirectional BFS via SQLite recursive CTE: `storage::impact_radius(seeds, max_depth, max_nodes)` returns blast-radius reachable from changed files in either direction. Pattern informed by code-review-graph (MIT).
- **Per-language import edge extraction** — Rust (`use_declaration`), Go (`import_spec` covering single/grouped/aliased/dot/blank), TypeScript (`import_statement` covering named/namespace/default/side-effect/`import type`), Python (`import_statement` + `import_from_statement` covering simple/dotted/aliased/multi/from/relative), Java (`import_declaration` covering simple/static/wildcard), C (`preproc_include` covering quoted and system forms; C++ `.cpp/.cc/.cxx/.hpp` also dispatched through this best-effort).
- **Indexer populates edges during rebuild.** `CodeIndex::index_project` calls `index_file_with_edges` and persists both symbols and edges. New `IndexStats.edges_extracted`. (`crates/illuminate-index/src/indexer.rs`)
- **`Auditor::audit_with_files`** with long-lived `OnceLock<Option<Mutex<Connection>>>` index connection. Returns `ImpactInfo { seed_symbols, defined_symbols, impacted_symbols, truncated }`. Caps: `DEFAULT_IMPACT_DEPTH = 2`, `DEFAULT_IMPACT_NODES = 50`. Missing/corrupt `index.db` degrades gracefully to empty impact. (`crates/illuminate-audit/src/lib.rs`)
- **`Auditor::with_index`** constructor accepting `impl Into<PathBuf>`. Existing `Auditor::new` and `Auditor::audit` signatures unchanged. Shared `resolve_index_db_from_cwd()` helper used by both CLI and MCP for ancestor-walk index path resolution.
- **CLI `audit` extended** with `--index-db <PATH>` flag and positional file args. Prints "Blast radius: N symbols across M files" and "Defined symbols in touched files: N" sections in human output (capped at 10 entries each). JSON output unchanged shape, now includes `impact` field. (`crates/illuminate-cli/src/commands/audit.rs`)
- **`illuminate impact <files...>`** new CLI subcommand. Read-only inspection of file blast-radius. Prints defined symbols, imports, and impact-radius per file. `--json` for scripting. `--depth` and `--max-nodes` overrides. (`crates/illuminate-cli/src/commands/impact.rs`)
- **MCP `illuminate_audit` accepts `files` arg** and surfaces `impact` in the JSON response. Handler delegates to `Auditor::audit_with_files` (single source of truth shared with CLI) rather than the previous inline policy/conflict reimplementation.
- **`TrailRecord` carries optional `input_tokens` / `output_tokens`** (sum across messages in a session). Plumbed from Cursor (bubble `tokenCount`), Claude Code (`message.usage`), and Codex (`payload.usage`, defensive). Cache buckets explicitly excluded from totals for cross-agent comparability. Foundation for future cost-attribution analytics. (`crates/illuminate-trail/src/record.rs`, `cursor.rs`, `claude.rs`, `codex.rs`, `raw.rs::UsageBlock`)
- **`watcher.rs` migrated to `tracing`** — replaced `eprintln!` calls with `tracing::warn!` for consistency with `illuminate-audit` and `illuminate-mcp`.
- **Two-graph architecture made explicit** in `docs/ARCHITECTURE.md` — code graph (`illuminate-index`) ↔ decision graph (`illuminate-core`) joined by `illuminate-audit`. Capture diagram corrected: Cursor uses `state.vscdb`, Codex uses `rollout-*.jsonl`. New "Related Projects" section credits codeburn (MIT) and code-review-graph (MIT) whose format knowledge informed this release.
- **Wiki review queue (v0.2 item).** Interactive `illuminate wiki review` walks low-confidence bootstrap candidates. Accept/reject/edit/skip/quit prompts. `--list` for scripting.
- **Trail systemd user unit.** `illuminate trail install-service` writes `~/.config/systemd/user/illuminate-trail.service` with resource caps (MemoryMax=512M, CPUQuota=20%).
- **Pre-write hook smoke tests.** `crates/illuminate-cli/tests/audit_hook_smoke.rs` exercises the `audit-hook` subcommand end-to-end with stdin JSON.

### Fixed

- `illuminate init --hooks` no longer registers a bogus `--stdin` flag on `audit-hook`.
- Pre-write hook policy loader walks ancestors for `.illuminate/illuminate.toml` (matches `illuminate audit` behavior).
- Bootstrap content-hash dedup collapses identical content across `CLAUDE.md` / `.cursorrules` / `.windsurfrules`.
- Audit FTS5 fallback surfaces related graph episodes on clean plans (helps when graph has bootstrapped pages but no NER-extracted entities).
- Workspace-wide `cargo fmt` drift fixed; CI green.

### Deferred to v0.5+

- Within-function call/inheritance edges across all six languages (currently only file-level imports).
- Symbol-scoped edges (currently `source_qualified` is always `file::<path>`).
- Path normalization in `audit_with_files` (currently pass-through; documented).
- Cost-attribution analytics consumer for the new token fields.
- MCP `:memory:` graph fallback edge case.
- Cache-bucket token fields (`cache_creation_input_tokens` etc.) on `TrailRecord` for accurate Anthropic cost math.
- Dedicated `Language::Cpp` (currently `.cpp/.cc/.cxx/.hpp` route through C extractor; `#include` works, body parsing best-effort).

## [0.1.0] — 2026-05-06 → 2026-05-07

### Added — v0.1 closed loop (2026-05-06 → 2026-05-07)

- **Prompt-trail capture (Claude Code).** Daemon reads `~/.claude/projects/<hash>/<session>.jsonl`, normalizes to `TrailRecord`, writes to `<repo>/.illuminate/trail/<date>-<topic>-claude.jsonl`. Opt-in only (requires `.illuminate/illuminate.toml`). New crate `illuminate-trail`.
- **Wiki layer.** Markdown front-matter parser, schema linter, directory walker, scaffold writer, index renderer, `tiny_http` HTML server. Pages register into the graph as episodes. New crate `illuminate-wiki`.
- **Bootstrap pipeline.** `illuminate bootstrap` parses `CLAUDE.md`, `AGENTS.md`, `.cursorrules`, `.windsurfrules`, `.clinerules`, plus Nygard ADRs under `docs/adr/` etc., and emits wiki pages with deduplication and idempotent writes. New crate `illuminate-bootstrap`.
- **Audit integration.** Policies load via ancestor walk for `.illuminate/illuminate.toml` (legacy fallback to `cwd/illuminate.toml`). `illuminate audit "<plan>"` exits 2 on policy violation, 1 on warning, 0 on pass. End-to-end integration test in `crates/illuminate-cli/tests/end_to_end_audit.rs`.
- **CLAUDE.md directive.** `illuminate init --claude` appends a "## illuminate: required pre-write step" section instructing the agent to call `illuminate_audit` before edits.
- **CLI subcommands.** `trail {import, list, show, watch, register}`, `wiki {init, lint, list, rebuild, serve, search}`, `bootstrap`, `failures {list, register}`, `status`.
- **GitHub Action.** Composite action at `.github/actions/audit-pr/` that installs `illuminate`, rebuilds the wiki, runs `illuminate audit` on the PR title, and comments findings on the PR. Example workflow at `.github/workflows/example-audit-pr.yml.example`. Docs at `docs/CI.md`.
- **Wiki search.** `illuminate wiki search "<query>"` runs case-insensitive substring grep over wiki pages plus FTS5 search over the graph.
- **Repo dogfood.** `.illuminate/illuminate.toml` carries a `no_stateful_sidecars` policy. `CLAUDE.md` at the repo root has the audit-pre-write directive. The illuminate repo audits its own intent.

### Fixed (2026-05-07)

- Bootstrap-emitted ids now use `dec-bs-...` prefix to satisfy the wiki linter regex.
- Bootstrap front-matter `title:` and `ref:` fields YAML-quoted when they contain `:` or other YAML-significant characters.
- Trail raw parser surfaces field-invalid known-type records as `TrailError::Parse` instead of silently demoting them to `Unknown`.
- `scan_dir` in trail watcher now logs read_dir + import errors instead of swallowing them.

### Deferred to v0.2

- Cursor and Codex session capture (stubs only — no usable session data was on the dev machine to test against).
- Reflect ingester from CI logs and Sentry/PagerDuty webhooks.
- LLM-classified auto-distill of trail content into decisions.
- Wiki review queue for low-confidence candidates.

### Deferred to v0.3

- `PreToolUse` hook integration polish (the `audit-hook` subcommand exists; v0.3 ties it into `illuminate init --hooks` for one-shot setup).
- Bootstrap helpers v2 (richer ADR formats, Slack/Linear/Jira import).
- Onboarding wizard for first-run.

---

## Pre-existing components (carried forward)

These crates predate the v0.1 closed loop and were already functional:

- `illuminate-core` — graph engine on top of `ctxgraph`. `Graph::open_or_create`, `Episode::builder`, FTS5 search.
- `illuminate-extract` — NER pipeline (GLiNER + GLiREL + embeddings via ONNX).
- `illuminate-embed` — all-MiniLM-L6-v2 embeddings.
- `illuminate-index` — tree-sitter symbol indexer (Rust/TS/Python/Go/Java/C).
- `illuminate-audit` — policy engine.
- `illuminate-mcp` — JSON-RPC MCP server (already exposed `illuminate_audit`).
- `illuminate-watch` — daemon harness + git ingestion + signal scoring.
- `illuminate-reflect` — failure-as-episode store.
- `illuminate-route` — reading-plan generator with RRF fusion.
