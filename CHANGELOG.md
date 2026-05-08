# Changelog

All notable changes to Illuminate are tracked here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.13.0] — 2026-05-08

### Added — illuminate-config crate + Graph::delete_episode

Both items in this release close v0.7-v0.12 deferred-items.

- **New `illuminate-config` crate.** `AuditConfig`, `TrailConfig`, `ExtractionConfig`, `McpHttpConfig` and their parsers (`parse_audit_config`, `parse_trail_config`, `parse_extraction_config`, `parse_mcp_http_config`) plus all `DEFAULT_*` constants moved out of `illuminate-audit::policy` into a new workspace member at `crates/illuminate-config/`. `illuminate-audit::policy` re-exports the entire surface so existing imports (`illuminate_audit::policy::AuditConfig`, etc.) continue to work without changes — the CLI and MCP crates compile unmodified. The motivating consumer is `Graph::load_extraction_pipeline_from_config` in `illuminate-core`: it now calls `illuminate_config::parse_extraction_config` instead of an inline `toml::Value` walk, closing the dep-cycle that has been blocking this refactor since v0.7. Behavior is identical (same TOML path, same default `0.5`), with a small bonus that wrong-type values now log via `tracing::warn!` instead of falling through silently. Existing 14 config tests moved to `crates/illuminate-config/tests/config_tests.rs`; 2 new re-export sanity tests in `crates/illuminate-audit/tests/`. (`crates/illuminate-config/`, `crates/illuminate-core/src/graph.rs`, `crates/illuminate-audit/src/policy.rs`)
- **`Graph::delete_episode(id) -> Result<bool>`.** New transactional delete API on `Graph` that cascades to dependent rows: `anchors` (foreign key `episode_id`), `episode_entities` (junction), and `edges` (extraction-spawned). The `episodes_fts` virtual table is handled automatically via the existing `episodes_ad` AFTER DELETE trigger; the embedding lives inline on the `episodes` row so it's removed with the row. `entities` is intentionally NOT cascaded — extracted entities can be referenced across unrelated episodes, so over-eager removal would lose referential integrity for the rest of the graph. Returns `Ok(false)` (not `Err`) for unknown ids. Used by the v0.12 `wiki redact` command to actually delete graph episodes matching the redacted regex (file-side replacement was already shipped; this closes the v0.14-deferred graph deletion). 4 new core tests (`delete_episode_returns_true_for_existing_id`, `_returns_false_for_unknown_id`, `_removes_anchors`, `_with_embedding`), 1 CLI smoke test (`wiki_redact_deletes_matching_episodes`). (`crates/illuminate-core/src/{graph.rs,storage/sqlite.rs}`, `crates/illuminate-cli/src/commands/wiki.rs`)

### Deferred to v0.14+

- AuditResult `confidence` per-finding field (overall + per-finding; design space — what makes a meaningful audit confidence is fuzzy).
- Bootstrap interactive TTY interview (writes the same v0.11 YAML schema; needs prompt-library choice).
- `failure log` editor mode (`$EDITOR` template).
- MCP HTTP Server-Sent Events streaming.
- mTLS / OAuth for MCP HTTP.

## [0.12.0] — 2026-05-08

### Added — three CLI commands closing remaining `docs/CLI.md` gaps

- **`illuminate audit-pr <pr-number>`** per `docs/CLI.md:125-136`. Uses `gh` CLI to fetch PR metadata (title, base, head, url) and changed-file list, runs `Auditor::audit_with_files` with full impact + relevant-decisions surface, and formats the result as either PR-comment-friendly Markdown (default) or JSON. `--repo OWNER/REPO` flag with auto-detection from `git remote get-url origin` (handles ssh + https github URLs). `--comment` posts the result via `gh pr comment --body-file`. `--token-env` forwards a token env var to gh. Exit codes match `audit` (0/2/3). The `.github/actions/audit-pr/action.yml` composite action was upgraded to call `illuminate audit-pr ${{ github.event.pull_request.number }} --repo ${{ github.repository }} --comment`, replacing the previous "audit only the PR title" behavior with real per-file analysis. (`crates/illuminate-cli/src/commands/audit_pr.rs`, `.github/actions/audit-pr/action.yml`)
- **`illuminate explain <path> [--json]`** per `docs/CLI.md:138-143`. Reads `Graph::get_anchors_for_file` for the path, fetches each linked episode, classifies by source heuristic (`wiki:dec/`, `wiki:pat/`, `wiki:fail/`, `reflexion:`) into Decisions / Patterns / Failures / Other, and prints a human-readable per-section breakdown with anchor line ranges and symbol names. JSON mode emits a structured envelope. No plan required — pure orientation aid. Mirrors the `illuminate_explain` MCP tool. (`crates/illuminate-cli/src/commands/explain.rs`)
- **`illuminate patterns list/show`** per `docs/CLI.md:164-169`. `list [--module SLUG] [--tag TAG]` walks `<repo>/.illuminate/wiki/patterns/*.md`, filters by either the dedicated `modules:` front-matter field OR a `module:<slug>` tag (best-effort union), and by tag substring match. `show <id>` finds the page with matching front-matter `id` and prints the raw markdown. Mirrors the existing `decisions` / `failures` subcommand structure. Empty wiki and unknown ids handled gracefully (clear messages, non-zero exit only on `show`-not-found). (`crates/illuminate-cli/src/commands/patterns.rs`)

### Deferred to v0.13+

- `illuminate wiki redact` (per `docs/CLI.md:241-253`).
- `illuminate rebuild` (top-level — current `wiki rebuild` is a subcommand).
- `illuminate search` (top-level — currently only `wiki search` and `query`).
- Bootstrap interactive TTY interview (writes the same v0.11 YAML schema).
- `failure log` editor mode (`$EDITOR` template prompt).
- MCP HTTP Server-Sent Events streaming.
- Refactor `Graph::load_extraction_pipeline_from_config` via shared `parse_extraction_config` (still blocked by potential dep cycle).

## [0.11.0] — 2026-05-08

### Added — bootstrap interview source (5/5) + `failure log` CLI per docs

- **Bootstrap interview source.** 5th of 5 documented bootstrap sources. `crates/illuminate-bootstrap/src/interview.rs` reads `<repo>/.illuminate/interview.yaml` and emits high-confidence (`0.7`) candidates per scalar field (`language`, `database`, `architecture`, `deployment`) plus list entries (`avoid:`, `prefer:`) and structured `services:` objects. Confidence sits above the default `auto_merge_threshold` so interview answers route directly into the canonical wiki rather than `_review/` (the user explicitly stated these). YAML parse failures fall through to `Ok(vec![])` with a `tracing::warn!` so missing/malformed files never break bootstrap. Interactive TTY mode that writes the YAML from stdin prompts is deferred to v0.12 — the YAML schema is the stable contract. (`crates/illuminate-bootstrap/src/{interview.rs,orchestrate.rs}`)
- **`illuminate failure log` CLI subcommand per `docs/CLI.md`.** New singular `Failure { cmd: FailureCmd }` clap variant alongside the existing plural `Failures::Register/List`. `illuminate failure log --title T --root-cause R --fix F --severity high [--lesson L] [--files A,B,C] [--modules X,Y] [--from-incident URL]` writes a fully-formed `<repo>/.illuminate/wiki/failures/<YYYY-MM-DD>-<slug>.md` with valid front-matter (id, title, page_type, status, tags, created, updated) and structured sections (`## Root Cause`, `## Fix`, `## Lesson for future agents`, `## Affected Files`, `## Affected Modules`, `## Severity`), then registers the page as a graph episode (source `failure:fail-<slug>`) via the shared `try_attach_extraction` helper so NER runs and entities populate. Required fields (`title`, `root_cause`, `fix`, `severity`) are validated; invalid severity values rejected with a clear error. Editor mode (open `$EDITOR` with template) is deferred to v0.12 — current behavior fails fast with a "v0.12 task" message when fields are missing, keeping CI/agent integration deterministic. (`crates/illuminate-cli/src/commands/failure.rs`, `crates/illuminate-cli/src/main.rs`)

### Deferred to v0.12+

- Bootstrap interactive TTY interview mode (writes the same YAML schema as the v0.11 file-driven path).
- `illuminate failure log` editor mode (`$EDITOR` template prompt when fields absent).
- MCP HTTP Server-Sent Events streaming for long-running tools.
- Refactor `Graph::load_extraction_pipeline_from_config` via shared `parse_extraction_config` (still blocked by potential dep cycle; needs an `illuminate-config` crate).

## [0.10.0] — 2026-05-08

### Added — MCP HTTP transport + docs realignment

- **MCP Streamable HTTP transport.** New `crates/illuminate-mcp/src/http.rs` exposes the same `dispatch()` logic over HTTP via `axum 0.8`. POST `/mcp` accepts a JSON-RPC request body and returns a JSON-RPC response. Optional bearer-token auth: when `[mcp.http].bearer_token_env` is configured AND the named env var is set, requests must carry `Authorization: Bearer <token>` (returns 401 otherwise). When absent, auth is disabled with a startup `WARNING` log. Bind address from `[mcp.http].bind` (default `127.0.0.1:7800`). New `parse_mcp_http_config()` in `illuminate-audit::policy` mirrors the tolerance pattern from `parse_audit_config`. CLI `illuminate mcp serve --http [--bind <addr>]` enables HTTP; `illuminate mcp start` and the legacy `illuminate serve` continue to use stdio. Dispatch routing is shared between transports via a `build_router()` constructor used by both the live server and in-process tests (`tower::ServiceExt::oneshot`). 5 HTTP integration tests + 2 config-parser tests. (`crates/illuminate-mcp/{src/http.rs,Cargo.toml}`, `crates/illuminate-audit/src/policy.rs`, `crates/illuminate-cli/src/{commands/mcp.rs,main.rs}`)
- **Docs realigned to actual `illuminate-route` and `illuminate-reflect` shapes.** Earlier docs described `illuminate-route` as the LLM fallback router and `illuminate-reflect` as `FailureRecord`-based failure capture. The implementations have always been the subject-to-reading-plan generator (`ReadingPlan { decisions, code_files, estimated_tokens }`) and the Reflexion-pattern episode store (`ReflexionInput`/`ReflexionEpisode`/`ReflexionStore`) respectively. `docs/CRATES.md` and `docs/ARCHITECTURE.md` now document the real APIs with naming notes explaining the historical drift. The actual LLM-fallback-with-PII-stripping logic lives in `illuminate-extract::llm_extract` behind the optional `cloakpipe` Cargo feature — also documented. Test-fixtures table updated: `illuminate-route` tests cover FTS5-only / semantic-only / fused ranking; `illuminate-reflect` tests cover Reflexion store round-trip + `find_relevant` ranking. (`docs/CRATES.md`, `docs/ARCHITECTURE.md`)

### Deferred to v0.11+

- MCP HTTP Server-Sent Events streaming (current transport is request/response only).
- mTLS / OAuth for MCP HTTP (bearer token only today).
- Bootstrap interactive interview (5th of 5 sources; needs UX design for stdin / file modes).
- Refactor `Graph::load_extraction_pipeline_from_config` to use canonical `parse_extraction_config` (still blocked by potential dependency cycle; needs an `illuminate-config` crate or moving the parser into core).

## [0.9.0] — 2026-05-08

### Added — audit evidence + decision_ref, MCP resources + prompts protocols

- **Audit evidence + policy decision_ref plumbing.** `PolicyViolation` and `Violation` both gained an `evidence: Option<String>` field carrying the snippet that triggered the match (a literal pattern phrase for `RejectedPattern`, the failed condition description for `MustUse`/`Frozen`, the first 200 chars of the conflicting episode for graph conflicts). `PolicyViolation` also gained `decision_ref: Option<String>` propagated from `RejectedPattern.decision_ref` in the policy TOML. `derive_wiki_url` now resolves with priority `policy_violations[0].decision_ref → decision_violations[0].conflicting_decision.id → relevant_decisions[0].episode_id`, closing the v0.7 limitation. (`crates/illuminate-audit/src/{lib.rs,policy.rs,response.rs}`)
- **MCP `resources/list` and `resources/read`.** Per `docs/MCP.md`, the server now exposes wiki pages as discoverable resources with URIs of the form `illuminate://wiki/{decisions,patterns,failures,modules}/<id>`. `list` walks `<repo>/.illuminate/wiki/` and returns `{uri, name, description, mimeType}` per page. `read` parses the URI, finds the matching page (with dir/page_type cross-check to prevent serving from wrong directory), and returns the literal markdown (front-matter included) per the MCP spec contract. New module `crates/illuminate-mcp/src/resources.rs`. `initialize` capability advertises `"resources": {}`. (`crates/illuminate-mcp/src/{resources.rs,server.rs,tools.rs}`)
- **MCP `prompts/list` and `prompts/get`.** Two named prompts per `docs/MCP.md`: `illuminate_audit_check` (reminds agent to call `illuminate_audit` before writing code, honor violations/warnings) and `illuminate_summarize_failures` (asks agent to call `illuminate_failures_for` with optional topic and produce a 2-3 paragraph lesson summary). New module `crates/illuminate-mcp/src/prompts.rs`. `initialize` capability advertises `"prompts": {}`. Unknown prompt names return `INVALID_PARAMS`. (`crates/illuminate-mcp/src/{prompts.rs,server.rs}`)

### Deferred to v0.10+

- MCP Streamable HTTP transport (currently stdio-only; `[mcp.http]` config still parsed-but-not-read).
- `illuminate-route` schema realignment per docs (currently `Plan`; docs spec `ReadingPlan`).
- `illuminate-reflect` schema realignment per docs (currently `Reflexion`; docs spec `FailureRecord`).
- Bootstrap interactive interview (5th of 5 sources).
- Refactor `Graph::load_extraction_pipeline_from_config` to use canonical `parse_extraction_config` (blocked by potential dependency cycle; needs `illuminate-config` crate or moving parser into core).

## [0.8.1] — 2026-05-08

### Added — config consumer wiring + watch parser fix + 4th bootstrap source

> Note: the `v0.8.0` git tag predates this branch (older release-ci commit). This release uses `v0.8.1` as the next available patch tag.

- **`illuminate-watch` git parser fix.** The previous `parse_git_log` mis-attributed file lists across commit boundaries when `--name-only` was set with multiple commits. New format `--format=%x1e%H%x00%an%x00%aI%x00%n%B%x1f` puts the record separator at the START of each chunk (so split chunks own their own data), uses NUL between fields, RS between commits, US (`%x1f`) as a body/file-list boundary inside the chunk. Bonus: also fixes a latent terminal-width truncation bug where git was silently truncating long subjects to ~80 chars (the leading `%n` defeats this). The bootstrap workaround in `git_history.rs` from v0.7 was updated to use the same format. (`crates/illuminate-watch/src/git.rs`, `crates/illuminate-bootstrap/src/git_history.rs`)
- **Trail watcher honors `[trail]` config.** `WatcherOpts` gained `enabled: bool` and `exclude_patterns: Vec<String>` fields. When `enabled = false`, `run_watcher` returns immediately with a `tracing::info!` line. When `exclude_patterns` is non-empty, sessions whose resolved `repo_path` matches any glob are skipped post-parse (so the agent-reported cwd is honored). Manual `illuminate trail import <path>` bypasses exclusions (explicit user action). CLI `cmd_watch` loads `TrailConfig` via a new `load_trail_config_from_cwd` ancestor walker mirroring `load_audit_config`. (`crates/illuminate-trail/src/{watcher.rs,import.rs}`, `crates/illuminate-cli/src/commands/trail.rs`)
- **`illuminate watch` reads `[extraction].signal_threshold` from `illuminate.toml`.** CLI flag changed from `signal_threshold: f64` (default 0.7) to `signal_threshold: Option<f64>`. New `resolve_signal_threshold` in `commands/watch.rs` resolves with priority: CLI flag > `parse_extraction_config(toml).signal_threshold` > `DEFAULT_EXTRACTION_SIGNAL_THRESHOLD` (0.7). All five `run_*` entry points (`run_git`, `run_git_since`, `run_github`, `run_webhook`, `run_daemon`) consume it as their first line and surface the resolved value in the existing "processing N commits (signal threshold: X)" log so the source is visible. The `confidence_threshold` was already consumed by `Graph::load_extraction_pipeline_from_config`. (`crates/illuminate-cli/src/commands/watch.rs`, `crates/illuminate-cli/src/main.rs`)
- **Bootstrap: README + CONTRIBUTING source.** 4th of 5 documented bootstrap sources. `readme::collect` walks `<repo>/{README,CONTRIBUTING}.md` (case-insensitive filename match), splits on `## ` headings, and emits architecture-style sections (`## Architecture`, `## Tech Stack`, `## Stack`, `## Tools`, `## Decisions`, `## Design`, `## Rationale`) as candidates unconditionally. Other sections only match when their body contains signal phrases (`instead of`, `we chose`, `rather than`, etc.). Boilerplate sections (`## Installation`, `## Usage`, `## License`, etc.) are skipped via exact match. Confidence 0.5 → routes to `wiki/_review/` for curation. SIGNAL_PHRASES extracted to a new `crates/illuminate-bootstrap/src/signals.rs` shared by `git_history` and `readme`. (`crates/illuminate-bootstrap/src/{readme.rs,signals.rs,orchestrate.rs}`)

### Deferred to v0.9+

- Bootstrap interactive interview source (5th of 5).
- AuditResult `confidence` and `evidence` per-finding fields.
- Policy-derived `wiki_url` (needs `RejectedPattern.decision_ref` plumbed through `PolicyViolation`).
- MCP HTTP transport, resources (`wiki/decisions/*` etc.), prompts.
- `illuminate-route` `ReadingPlan` and `illuminate-reflect` `FailureRecord` schema alignment per docs.
- Refactor `Graph::load_extraction_pipeline_from_config` to use `parse_extraction_config` from `illuminate-audit::policy` (currently blocked by potential dependency cycle; `illuminate-audit` already depends on `illuminate-core`).

## [0.7.0] — 2026-05-08

### Added — doc-alignment batch: bootstrap git-history, audit response surface, MCP page shape, config sections

**Breaking change:** `illuminate audit` and `illuminate audit-diff` now exit `3` on warn (was `1`). Violation remains `2`, pass remains `0`. CI integrators that branched on exit `1` for warnings must update to `3`. The `illuminate hook` command is unchanged (`block=2`, `allow=0`).

- **Bootstrap: git-history source.** `illuminate-bootstrap::git_history::collect` walks the last 6 months of `git log` (configurable `DEFAULT_HISTORY_MONTHS`) and emits decision-shaped commits as low-confidence (`0.6`) candidates routed to `wiki/_review/` for human curation. Decision-shape detection uses subject keywords (`decision`, `adopt`, `switch`, `migrate`, `chose`, ...) and signal phrases (`instead of`, `we decided`, `in favor of`, ...); conventional non-decision prefixes (`chore:`, `docs:`, `style:`, `test:`, `ci:`, `build:`) are filtered up front. Wired into `orchestrate::run_bootstrap` between ADRs and the existing dedup/write pipeline; collection failures degrade gracefully so other sources keep running. Shells out directly with `%H%x00%an%x00%aI%x00%B%x1e` to sidestep the multi-commit `--name-only` interleaving in `illuminate-watch`. (`crates/illuminate-bootstrap/src/{git_history.rs,orchestrate.rs}`)
- **Audit response: `trace_id`, `policies_applied`, `wiki_url`.** Per `docs/AUDIT.md`. `trace_id` is a fresh UUID v4 per `Auditor::audit` call for log/CI/MCP correlation. `policies_applied` lists every loaded policy name (regardless of whether it fired) so callers can debug "why didn't my policy match?" without chasing other issues. `wiki_url` is derived via priority order: first decision-violation's conflicting episode, then top `relevant_decisions` entry, returning a relative path under `.illuminate/wiki/decisions/<id>.md`. Policy violations are intentionally excluded for v0.7 (policy types do not yet carry a wiki id; `RejectedPattern.decision_ref` plumbing tracked separately). `confidence` and `evidence` per-finding fields remain deferred. (`crates/illuminate-audit/src/{lib.rs,response.rs}`)
- **CLI exit codes aligned with `docs/CLI.md`.** `audit` and `audit-diff` now exit `0` on pass, `2` on violation, `3` on warning. `hook` keeps `2` for block / `0` for allow per the PreToolUse contract — unchanged. **Breaking** for CI wrappers that branched on exit `1` for warnings. (`crates/illuminate-cli/src/commands/{audit.rs,audit_diff.rs}`)
- **MCP `illuminate_get_wiki_page` returns documented shape.** Now returns `{id, type, title, front_matter, body, path}` per `docs/MCP.md` (was `{id, content, path}`). `type` mirrors the lowercase `PageType` (`"decision"`, `"pattern"`, `"failure"`, `"module"`); `front_matter` is the parsed YAML; `body` is the markdown body. `path` is retained as a non-spec debugging extension. Parse errors return `{error, id, path}` to keep `tools/call` always succeeding. (`crates/illuminate-mcp/src/tools.rs`)
- **`[trail]` and `[extraction]` config parsers.** `parse_trail_config` yields `TrailConfig { enabled, purge_after_days, exclude_patterns }`; `parse_extraction_config` yields `ExtractionConfig { signal_threshold, confidence_threshold }`. Tolerant by design: parse errors, missing sections, wrong section types, and wrong field types all yield defaults with `tracing::warn!` so misconfiguration is visible without breaking pipelines. Consumer wiring (trail watcher honoring `enabled`/`exclude_patterns`, extractor honoring thresholds) deferred to follow-up tasks. (`crates/illuminate-audit/src/policy.rs`)

### Deferred to v0.8+

- `[trail]` / `[extraction]` config consumer wiring (parsed-but-not-read today).
- `illuminate-watch::git::parse_git_log` multi-commit `--name-only` interleaving bug — bootstrap shells out directly as a workaround; unifying needs a parser fix in watch.
- AuditResult `confidence` and `evidence` per-finding fields.
- Policy-derived `wiki_url` (needs `RejectedPattern.decision_ref` plumbed through `PolicyViolation`).
- Bootstrap README and interactive interview sources (still 3 of 5).
- MCP HTTP transport, resources (`wiki/decisions/*` etc.), prompts.
- `illuminate-route` `ReadingPlan` and `illuminate-reflect` `FailureRecord` schema alignment.

## [0.6.0] — 2026-05-08

### Added — extraction-on-register, semantic top-k, MCP tool surface, audit-diff

- **Trail / failures register now wires the extraction pipeline.** New shared `try_attach_extraction(&mut Graph, &db_path)` helper in `illuminate-cli::commands` resolves models via `find_models_dir` (env `ILLUMINATE_MODELS_DIR` > `~/.cache/illuminate/models` > `.illuminate/models`), pre-checks for `.onnx` files to keep first-install stderr quiet, and calls `Graph::load_extraction_pipeline_from_config` when an `illuminate.toml` is present (`load_extraction_pipeline` otherwise). Wired into both `trail::cmd_register` and `failures::cmd_register`, which previously opened the graph via `Graph::open_or_create` (yielding `pipeline: None`) and stored episodes raw — entities never reached the graph, so audits couldn't match them. Closes the highest-severity v0.5 doc-vs-code drift. (`crates/illuminate-cli/src/commands/{mod.rs,trail.rs,failures.rs}`)
- **Audit semantic top-k via `Graph::search_fused`.** New `Auditor::with_index_root_and_embed` constructor accepts `Option<Arc<EmbedEngine>>`, `semantic_top_k: usize`, `semantic_threshold: f64`. When the embed engine is wired and `top_k > 0`, `Auditor::audit` runs a final pass that embeds the plan, calls `Graph::search_fused` (RRF-fused FTS5 + cosine), filters by threshold, and surfaces results as `AuditResult.relevant_decisions: Vec<RelevantDecision>`. Pass is purely informational — never affects `status`. All failure paths (top-k disabled, no embed, embed error, search error) yield empty vec and log at `WARN`. CLI and MCP both wire through. Defaults: `top_k=5`, `threshold=0.0`. (`crates/illuminate-audit/src/lib.rs`, `crates/illuminate-audit/src/response.rs`)
- **`[audit]` config keys honored from `illuminate.toml`.** New `policy::parse_audit_config(toml_content) -> AuditConfig` sibling to `parse_policies`, plus `AuditConfig { semantic_top_k, semantic_threshold }` with `Default` returning `(5, 0.0)`. Tolerant by design: parse errors, missing `[audit]` section, wrong section type, missing fields, and wrong field types all yield defaults; wrong-type fields log `tracing::warn!` so misconfiguration is visible without breaking the audit run. CLI (`audit`, `audit-diff`) and MCP (`ToolContext::with_audit_config`) both load and apply. (`crates/illuminate-audit/src/policy.rs`)
- **MCP tool surface: `illuminate_decisions_for`, `illuminate_failures_for`, `illuminate_get_wiki_page`.** Per `docs/MCP.md`. `decisions_for` and `failures_for` are FTS5-phrase-quoted thin pass-throughs over `Graph::search` (path separators no longer trigger FTS5 syntax errors); `failures_for` filters to episodes whose `source` contains `"failure"` or `"reflexion"`. `get_wiki_page` walks `<repo_root>/.illuminate/wiki/` via `illuminate_wiki::walk::walk_wiki` and matches on either front-matter `id` or filename stem; returns `{error: "not found"}` on miss to preserve the `tools/call`-always-succeeds wire convention. All three appear in `tools_list()` and have schema-validated request shapes. (`crates/illuminate-mcp/src/tools.rs`)
- **CLI: `audit-diff [BASE]` and `decisions for <path>`.** Per `docs/CLI.md` and `docs/AUDIT.md`. `audit-diff` resolves the changed-file set via `git diff --name-only <BASE>...HEAD` (default `HEAD~1`), filters deletions, and reuses the same env-config / index resolution / embed loader as `audit::run`; `--json` and human formats parallel `audit`. `decisions for <PATH>` extends the existing `decisions` subcommand with the same FTS5-phrase-quoted query the MCP `illuminate_decisions_for` tool uses, so CLI and agent surfaces yield identical result sets. (`crates/illuminate-cli/src/commands/{audit_diff.rs,decisions.rs}`, `crates/illuminate-cli/src/main.rs`)

### Deferred to v0.7+

- Bootstrap source coverage: only ADRs and CLAUDE.md / AGENTS.md / `.cursorrules` are wired today; docs require git-history extraction, README parse, and the optional onboarding interview prompt.
- CLI exit-code alignment: `audit` and `audit-diff` currently return `0/1/2`; `docs/CLI.md` specifies `0/2/3` (warn → 3). One-line change deferred so existing CI integrations don't break mid-cycle.
- Audit response surface: `AuditResult` is missing `wiki_url`, `confidence`, `evidence`, `policies_applied`, and `trace_id` per `docs/AUDIT.md`. Tracker item.
- MCP transports beyond stdio: Streamable HTTP, resources (`wiki/decisions/*` etc.), and prompts (`illuminate_audit_check`, `illuminate_summarize_failures`) remain unimplemented.
- `illuminate-route` and `illuminate-reflect` schema alignment: docs spec `ReadingPlan` and `FailureRecord` shapes; current crates expose `Plan` and `Reflexion`.
- `[trail]`, `[mcp.http]`, `[extraction]` config sections still parsed-but-ignored.
- MCP `illuminate_get_wiki_page` response shape: returns `{id, content, path}` rather than the documented `{id, type, title, front_matter, body}`. Functional but drifted.

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
