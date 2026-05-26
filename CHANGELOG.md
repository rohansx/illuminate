# Changelog

All notable changes to Illuminate are tracked here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.22.0] — 2026-05-26

### Added — v3.2 begins: `illuminate-ingest`, `illuminate ask`, `illuminate browse`

This release opens the v3.2 docs-as-first-class phase (per [`docs/knowledge-layer.md`](docs/knowledge-layer.md)) and closes the last v3.0 GA gap (`illuminate browse`). Three new CLI verbs, one new MCP tool, one new crate.

- **`illuminate-ingest` crate.** New `crates/illuminate-ingest/` (~480 LoC, 8 unit tests). Public API: `IngestAdapter` trait (`name` / `fetch_all` / `fetch_since`), `IngestedDoc` struct, `DocKind` enum (Adr / Architecture / Runbook / Design / OnboardingGuide / Convention / PromptCookbook / Integration / Oncall / Spec / Generic), `LocalMarkdownAdapter`, `IngestReport`, `IngestError`, plus the `ingest_all(graph, adapter)` and `ingest_since(graph, adapter, watermark)` entry points. **Strictly read-only on the external side** — no `push()` / `write()` / `commit_back()` methods anywhere in the crate (the trust-model invariant from [`docs/trust-model.md`](docs/trust-model.md) and [`docs/knowledge-layer.md`](docs/knowledge-layer.md)). `LocalMarkdownAdapter` walks roots for `*.md`, skips `node_modules` / `target` / `dist` / `.git` / dotfiles (depth-0 root is always allowed), extracts the first H1 as title, infers `DocKind` from the directory name (`docs/adr/` → `Adr`, `docs/runbooks/` → `Runbook`, etc.), and stamps the resulting episodes with `source: ingested:local-docs` + metadata `(adapter, external_id, doc_kind, title, url?, author?, updated_at)`.
- **`illuminate ingest` CLI verb.** `illuminate ingest [--roots PATH...] [--json]`. Defaults to scanning `docs/`, `ARCHITECTURE.md`, `AGENTS.md`, `CLAUDE.md`, `README.md` if `--roots` is not supplied. Verified live on this repo: pulled 43 doc episodes into the graph from `docs/`.
- **`illuminate ask` CLI verb + `illuminate_ask` MCP tool.** Cross-corpus retrieval over decisions / patterns / failures / sessions / ingested docs / trail. `illuminate ask "<question>" [--limit N] [--format human|json]`. The MCP tool returns the same JSON envelope so Claude Code can call it inline. Pipeline: `graph.search` (already sanitized via the v0.20 FTS5 fix) → classify each hit by `source` prefix + `[id-prefix-...]` content token → group by kind → render as either a structured markdown report (`human`, default) or a JSON envelope with `{question, hits, hit_count, empty_kinds}`. **v0.22 ships retrieval-only — no LLM synthesis.** That's deferred to v3.3, which will add an optional final-rewrite step that consumes this exact envelope. Verified live: `illuminate ask "why no Redis caching"` returns the no-Redis decision at the top + 8 relevant ingested docs grouped under "Ingested docs"; same query via MCP `tools/call` returns the matching envelope with `hit_count: 11`.
- **`illuminate browse` CLI verb.** `illuminate browse [--team-repo PATH] [--limit N] [--json]` lists published sessions sorted newest-first; `illuminate browse <id-or-filename>` renders the full body. Parses front-matter (`id` / `session_id` / `agent` / `model` / `redaction` / `commit_sha` / `created` / `title`). Closes the v3.0 GA gap — `illuminate-publish` shipped in v0.21 but until now there was no first-class way to read the published sessions back. Verified live against `/tmp/team-illuminate-smoke` from the v0.21 smoke test.
- **`docs/CLI.md` + `docs/MCP.md`** updated with the three new CLI verbs and the new MCP tool. README CLI surface table gains three new lines.
- **Workspace version bump** `0.21.0` → `0.22.0` across `Cargo.toml` + all path-deps.

### Deferred to v0.23+

- **`illuminate-ingest` external adapters:** `ConfluenceAdapter`, `NotionAdapter`, `GithubWikiAdapter`, `GoogleDocsAdapter`, `SpecKitAdapter`. The `IngestAdapter` trait is stable; adding adapters is mechanical work + per-adapter API auth. v0.22 ships `LocalMarkdownAdapter` only.
- **LLM synthesis for `illuminate ask`** (v3.3) — adds an optional final-rewrite step consuming the v0.22 retrieval envelope.
- **`illuminate trust check`** config linter for off-host writes.
- **60-second enrichment + publish demo video** — primary v3.0 launch artifact.
- All carry-overs from v0.18 / v0.19 / v0.20 / v0.21.

## [0.21.0] — 2026-05-25

### Added — `illuminate-publish` crate + `illuminate publish` CLI verb (v3.0 wedge complete)

This release ships the **second half of the v3 two-product positioning** — both wedges (`illuminate-enrich` from v0.19 + `illuminate-publish` here) are now live. The full four-stage pipeline (enrich → generate → capture → curate) is end-to-end functional. Per `docs/ROADMAP.md`, this closes the core scope of v3.0.

- **`illuminate-publish` crate.** New `crates/illuminate-publish/` (~430 LoC). Public API: `RedactionLevel` (Full / Summary / Decision / Discard), `PublishRequest`, `PublishResponse`, `TeamRepoTarget` (LocalPath in MVP — GitRemote deferred to v3.1 per trust model), `PublishError`, and the entry points `publish(graph, req)` and `install_pre_commit_hook(repo_root, team_repo)`. The function reads a trail jsonl from `.illuminate/trail/`, renders a structured markdown page per the chosen redaction level (full = entire transcript, summary = first prompt + last response + files touched, decision = front-matter only, discard = no-op), writes to `<team_repo>/sessions/<YYYY-MM-DD>-<slug>.md`, and registers a graph episode with `source = "published:<agent>"` so future `illuminate enrich` calls can surface the published session. 9 unit tests cover each redaction level, filename / sessions-subdir layout, hook installation (with executable bit on Unix), graph-episode registration with correct source + metadata, and the slugify helper.
- **`illuminate publish` CLI verb.** `illuminate publish --trail PATH --redaction <full|summary|decision|discard> --team-repo PATH [--commit-sha SHA] [--json]` writes one session. `illuminate publish --install-hook --team-repo PATH` writes a `.git/hooks/pre-commit` script that calls publish on every commit (defaults to `summary`; override per-commit via `ILLUMINATE_PUBLISH_REDACTION=<level>` env var or skip with `git commit --no-verify`). Verified live on this repo: `illuminate publish --trail .illuminate/trail/<file>.jsonl --redaction summary --team-repo /tmp/team-illuminate-smoke` produced a valid markdown page with front-matter `page_type: session` + `source: published:claude-code` and registered a graph episode `019e604f-...`.
- **Wiki schema extension: `page_type: session`.** `docs/SCHEMA.md` gains a full section documenting the new page type, every front-matter field, body shape per redaction level, and the trust-model invariants enforced by the crate (only writes to the explicit `--team-repo` path, never network, `LocalPath` only in v3.0).
- **Trust-model enforcement.** `illuminate-publish` is the **only** crate in the workspace that writes outside `.illuminate/` — and even then only to a path the caller has explicitly named. No defaults, no implicit network. The GitRemote variant is deliberately gated for v3.1 once `illuminate trust check` ships to lint the config.
- **CHANGELOG, README, CLI docs.** New CLI surface line in `README.md`; new section in `docs/CLI.md` for `illuminate publish` with the flags table + examples + the `--install-hook` flow.
- **Workspace version bump.** `0.20.0` → `0.21.0` across `Cargo.toml` + all crate path-deps.

### Deferred to v0.22+

- **`illuminate browse`** — terminal UI over published sessions (search, blame, open original jsonl in `$EDITOR`).
- **`illuminate trust check`** — config linter that gates `TeamRepoTarget::GitRemote` and other network paths.
- **`TeamRepoTarget::GitRemote`** — push to a configured git remote with consent prompt.
- **LLM-assisted summary** at publish time (currently the `Summary` body is deterministic-template only).
- **60-second enrichment + publish demo video** — the primary v3.0 launch artifact.
- All v0.18 / v0.19 / v0.20 carry-overs.

## [0.20.0] — 2026-05-25

### Added — FTS5 sanitizer at the `Graph` boundary, audit MCP unblocked, repo hygiene

This release closes the technical debt from v0.19 in a single batch.

- **FTS5 sanitizer promoted into `illuminate-core`.** The helper that started life in `illuminate-enrich` (v0.19) and then moved up to `illuminate-route` (v0.19) now lives in `illuminate::query::sanitize_for_fts5` and is applied **inside `Graph::search`, `Graph::search_entities`, and `Graph::search_fused`**. Every caller of the graph search API (audit, route, MCP, dashboard, CLI search/query verbs, wiki dashboard `/search`) now benefits from the same fix transparently — no per-caller glue, no remembering to sanitize. Empty sanitized queries short-circuit before SQL execution. `illuminate-route::sanitize_for_fts5` is now a thin re-export of the core function for back-compat; `illuminate-enrich`'s re-export chain still works. 5 new tests in `illuminate-core::query::tests` cover operator-character stripping, stopword filtering, lowercasing + dedup, empty/garbage input, and snake_case identifier preservation. (`crates/illuminate-core/src/{query.rs,graph.rs}`)
- **Audit MCP tool unblocked.** Verified end-to-end: the exact plan that failed 4× earlier in this session with `fts5: syntax error near "/"` (a multi-line plan containing slashes, colons, and an HTML angle bracket from a prior session transcript) now returns cleanly via `tools/call` against the live MCP server. The audit still flags the expected decision conflict (Redis keyword match against the trail episode) but the underlying call no longer dies on FTS5 syntax. This was the bug that made the CLAUDE.md audit gate effectively unusable on any plan mentioning a file path.
- **Cleaner code paths in `illuminate-route` and `illuminate-enrich`.** Now that sanitization lives at the graph boundary, `route()` drops its pre-call sanitize + empty-check; the embedding still sees the raw subject (preserves meaning). `illuminate-enrich` drops its local `enrich_prompt` short-circuit. Net diff: 50+ LoC of duplicated sanitize logic removed; one single source of truth in `illuminate-core::query`.
- **Repo hygiene.** Removed `Illuminate Landing _standalone_.html` (1.9 MB design-reference file from the v0.17 wiki-dashboard theme adoption). The theme is in `crates/illuminate-wiki/src/dashboard.rs`; the standalone HTML is no longer load-bearing.
- **Workspace version bump.** `0.19.0` → `0.20.0` across `Cargo.toml` + all crate path-dep versions.

### Deferred to v0.21+

- All v0.19 deferrals carry forward (`illuminate-publish`, `illuminate browse`, `illuminate trust check`, `page_type: session` schema, enrichment demo video, plus the v0.18 carry-overs).

## [0.19.0] — 2026-05-25

### Added — v3 positioning reset + `illuminate-enrich` wedge crate

This release marks the **v3 positioning reset** — Illuminate is now framed as "GitHub for agents" with two user-facing products (Illuminate Enrich, Illuminate Repo) on one substrate, and ships the first half of that pivot: a fully working `illuminate-enrich` crate + CLI verb.

- **v3 docs reset across six files.** Rewrote `docs/PRODUCT_OVERVIEW.md` to adopt the GitHub-for-agents framing and the four-stage pipeline (enrich → generate → capture → curate); updated `docs/ARCHITECTURE.md` and `docs/CRATES.md` with shipped (14) + planned (2) crate sections and a `illuminate-enrich`/`illuminate-publish` data-flow diagram; reset `docs/ROADMAP.md` to v3.0/v3.1/v3.2/v3-cloud with the v0.1→v0.18 work collapsed into a single Shipped section; refreshed `README.md` hero, two-products section, four-stage pipeline diagram, and crate table with shipped/planned status. **New docs:** `docs/philosophy.md` (prompts-as-source-code manifesto) and `docs/trust-model.md` (three-rings local/team/internet boundary with verification scripts, no-individual-scoring commitment, regulated-verticals FAQ).
- **`illuminate-enrich` crate.** New `crates/illuminate-enrich/` (~550 LoC). Public API: `EnrichRequest`, `EnrichResponse`, `Injection`, `InjectionSource`, and the pure transformation function `enrich_prompt(graph, embed, req) -> Result<EnrichResponse>`. No LLM in the path. **Determinism guarantee:** same `(prompt, graph state)` produces a byte-identical enriched prompt — enforced by a SHA-256 receipt (`graph_state_hash`) returned in every response and a property test that runs the same input twice and asserts identical output. Sorts injections deterministically by `(source, id)`; applies a byte budget to drop trailing items; categorizes source via both the source field and the `[id-prefix-...]` content token wiki episodes carry. 10 unit tests cover determinism, populated-graph injection, empty-graph passthrough, byte budget, source inference, FTS5 sanitizer behavior, path extraction, and score bucketing.
- **`illuminate enrich` CLI verb.** New subcommand: `illuminate enrich "<prompt>" [--files PATH...] [--max-bytes N] [--format human|prompt|json]`. The `human` format prints the enriched prompt plus a footer summarizing injection count and the determinism hash prefix; `prompt` emits the enriched text verbatim (pipe-friendly); `json` emits the full `EnrichResponse` envelope. Verified live against the populated graph for this repo: `illuminate enrich "add Redis caching to the txn endpoint"` surfaces `dec-bs-claude-md-caching-never-use-redis` under "Relevant decisions" with a fresh `graph_state_hash`.
- **FTS5 query sanitizer.** A long-standing v0.8.0 bug (the audit MCP tool failed on any plan containing `/`, `:`, `<`, etc.) is now worked around at the enrich-call boundary. `sanitize_for_fts5` strips FTS5 operator characters, drops stopwords, lowercases, dedups, and OR-joins meaningful tokens before delegating to `route()` / `Graph::search`. Same fix wants to land in `illuminate-route` and `Graph::search` directly — tracked as a follow-up.
- **Workspace version bump.** `Cargo.toml` workspace.package.version bumped from `0.8.0` → `0.19.0` to match the actual release cadence in CHANGELOG. All hardcoded `version = "0.8.0"` path-deps across the workspace updated. `cargo build --workspace` passes cleanly at v0.19.0.

### Deferred to v0.20+

- **`illuminate-publish` crate** (the second v3.0 wedge, Stage 4 of the pipeline): explicit publish gesture, redaction-level chooser, pre-commit hook, structured `page_type: session` schema extension.
- **`illuminate browse`** terminal UI over published sessions.
- **`illuminate trust check`** config linter (default-deny on uploads).
- **FTS5 sanitizer in `illuminate-route::route()` and `Graph::search()`** so audit + search + MCP all benefit. Caller-side fix in `illuminate-enrich` already works; source-side fix avoids per-caller duplication.
- **`illuminate_enrich` MCP tool** so Claude Code can call enrich inline.
- **60-second enrichment demo video** — the primary launch artifact for v3.0.
- All v0.18 carry-overs (PNG screenshots in `docs/screenshots/`, `evidence` field shape, bootstrap interactive TTY, `failure log` editor mode, MCP HTTP SSE, mTLS/OAuth, `wiki redact` graph deletion, audit history view).

## [0.18.0] — 2026-05-09

### Added — dashboard quick-add form

- **`GET /new` and `POST /new` on the wiki dashboard.** Non-CLI teammates can now add decision / pattern / failure / module pages directly from the browser. Pick a type (decision pre-selected, or `?type=pattern` etc.), type a title, optional comma-separated tags, and a markdown body. Submission writes `<root>/.illuminate/wiki/<dir>/<id>.md` with valid YAML front-matter (`id = <prefix>-<slug>`, prefix is `dec`/`pat`/`fail`/`mod`), then 303-redirects to the new page view. The topnav gained a `+ new` link. Validation: title and body required (form re-renders with inputs preserved on error); duplicate id returns 409 with a "page already exists" banner. 10 new black-box tests on the pure `route()` function. Live-smoke against the release binary confirmed end-to-end: GET renders form with three radio options pre-checked correctly, POST writes a valid file with proper front-matter (e.g., `id: dec-smoke-test`, `page_type: decision`, `tags: ["qa"]`), returns 303 to `/page/decisions/dec-smoke-test`. (`crates/illuminate-wiki/src/{dashboard.rs,serve.rs}`)

### Deferred to v0.19+

- Capturing actual PNG screenshots from `scripts/capture-screenshots.sh` and committing them to `docs/screenshots/`.
- `evidence` field shape change from `Option<String>` to `Vec<String>` for full `docs/AUDIT.md` parity.
- Bootstrap interactive TTY interview.
- `failure log` editor mode.
- MCP HTTP Server-Sent Events streaming.
- mTLS / OAuth for MCP HTTP.
- Audit history view (`/audits`) showing recent audit runs over time.

## [0.17.0] — 2026-05-09

### Added — `wiki serve` dashboard, README polish, utkrushta dogfood

This release makes illuminate look and feel like a product, not just a CLI. The headline change: `illuminate wiki serve` is now a real dashboard with stats, browseable lists, search, an **audit playground** for non-CLI users, and a JSON API for ext integrations. Alongside, a polished README (badges, hero diagram, dashboard view-table, GitHub topics) and a working dogfood deployment in a real production repo.

- **`illuminate wiki serve` dashboard.** New `crates/illuminate-wiki/src/dashboard.rs` (~670 lines). Routes:
  - `/` — home with stats cards (decisions/patterns/failures/modules/episodes) + recent activity feed.
  - `/decisions`, `/patterns`, `/failures`, `/modules` — filterable list views (`?status=`, `?tag=`, `?severity=`).
  - `/page/<dir>/<id>` — single-page render with front-matter card + body + related panel; back-compat alias at `/<dir>/<id>`.
  - `/search?q=` — two-pane: wiki pages (substring) + graph episodes (FTS5 + semantic, when graph reachable).
  - `/audit` GET (form) and POST (HTML result page) — the killer non-CLI surface. Paste a plan, see the audit response visually with status banner, policy violations cards, decision conflicts, relevant decisions, blast radius. Confidence badges per finding.
  - `/api/{stats,pages,page/<id>,search,audit}` — JSON endpoints for ext integrations (any stack can hit these without MCP).
  - Sticky top nav with project name, search box, link to playground. Dark mode (`@media (prefers-color-scheme: dark)`). Mobile responsive (< 720 px column-stack). Type badges color-coded.
  - **Auditor injection via callback** (not dep-add) to keep `illuminate-wiki` dependency-light: the CLI's `wiki serve` builds an `Arc<AuditFn>` closure from the audit crate and passes it to the server. Each request constructs a fresh `Auditor` (Auditor isn't Send+Sync — extraction pipeline single-threaded; opening SQLite + WAL is < 10 ms).
  - Pure `route()` function extracted for testability — 9 black-box tests cover home/list/page/search/audit/api endpoints. (`crates/illuminate-wiki/src/{serve.rs,dashboard.rs}`, `crates/illuminate-cli/src/commands/wiki.rs`)
- **README visual polish.** Five badges (release, rust 2024, MCP stdio+HTTP, MIT, tests). Replaced prose flywheel with a labeled ASCII diagram. Added "Try it in 60 seconds" block right under the hero. New "What it looks like" section with a routes/views table linking to the dashboard. CLI surface aligned with `docs/CLI.md` (audit-pr, audit-diff, impact, explain, etc.). New "Built on" section credits codeburn (MIT) and code-review-graph (MIT). Suggested GitHub repository About text + topics in a `<details>` block at the bottom for one-click setup. Plus `scripts/capture-screenshots.sh` — populates a tempdir with sample data and starts the dashboard so screenshots can be captured against a known-good fixture.
- **Utkrushta dogfood (private — not pushed).** Wired illuminate into `/home/rsx/Desktop/utkrusht-ai/Utkrushta` with a project-specific `.illuminate/interview.yaml` (language, database, architecture, deployment, avoid/prefer lists, services), `.gitignore` entries for local caches, and a teammate-onboarding doc at `docs/illuminate-setup.md`. Bootstrap ran 5 sources → 19 wiki pages written + 23 in `_review/` + 37 graph episodes registered. Both `no_raw_sql` and `no_bare_exceptions` policies validated against realistic plan text. Local commit `04cba00` on the `feat/e2b-local-dev-hack` branch — **not pushed**, awaiting team review.

### Deferred to v0.18+

- Capturing actual PNG screenshots from `scripts/capture-screenshots.sh` and committing them to `docs/screenshots/`.
- `evidence` field shape change from `Option<String>` to `Vec<String>` for full `docs/AUDIT.md` parity.
- Bootstrap interactive TTY interview (the YAML schema is the stable contract).
- `failure log` editor mode.
- MCP HTTP Server-Sent Events streaming.
- mTLS / OAuth for MCP HTTP.

## [0.16.0] — 2026-05-09

### Added — getting-started walkthrough + `init` canonical location fix

- **`docs/GETTING_STARTED.md`** — step-by-step walkthrough: install, init, first decision, optional models, bootstrap from interview YAML, first audit (violation + benign + JSON), code-graph blast-radius, MCP wiring (stdio + HTTP), the flywheel (trail register, failure log), CI integration, common-questions FAQ. Every command and output validated end-to-end in a tempdir before this commit. Linked from `README.md`.
- **`illuminate init` now writes the canonical `.illuminate/illuminate.toml` location.** Validating the walkthrough surfaced a real UX bug: `init` previously wrote `illuminate.toml` to the repo root (the v0.1 legacy form), but `illuminate bootstrap`'s `find_repo_root` only accepted `.illuminate/illuminate.toml` — so the natural `init → bootstrap` flow failed with "no .illuminate/illuminate.toml found." Fixed: `init` now writes to `.illuminate/illuminate.toml` (the canonical location matching bootstrap, audit, and MCP path resolution). The audit policy loader's legacy fallback is preserved so existing v0.1-v0.15 projects continue to work without migration; new `init` writes only land in the canonical location. (`crates/illuminate-cli/src/commands/init.rs`)

### Deferred to v0.17+

- `evidence` field shape change from `Option<String>` to `Vec<String>` for full `docs/AUDIT.md` parity.
- Bootstrap interactive TTY interview.
- `failure log` editor mode.
- MCP HTTP Server-Sent Events streaming.
- mTLS / OAuth for MCP HTTP.

## [0.15.0] — 2026-05-09

### Added — end-to-end smoke test + bootstrap auto-rebuild

- **End-to-end golden-path smoke test** (`crates/illuminate-cli/tests/golden_path_e2e.rs`). Builds a tempdir repo with `git init`, a Rust source file, `.illuminate/illuminate.toml` with a `RejectedPattern` policy, `.illuminate/interview.yaml`, and a `CLAUDE.md`. Then exercises the full pipeline: `index → bootstrap → audit (violation) → audit --json (impact) → explain`. Uses `ILLUMINATE_NO_EMBED=1` so fastembed model files aren't required, runs in ~0.16s in isolation. Documents the user's golden path AND catches integration regressions across the v0.4-v0.14 work surface. Asserts loose where extraction depends on optional ONNX models (the policy/redis path is pure string match and always works).
- **`illuminate bootstrap` auto-rebuilds the wiki index + graph** by default. Previously bootstrap wrote wiki pages but left `graph.db` empty — users had to run `illuminate wiki rebuild` separately for the audit to see the new decisions. The smoke test surfaced this gap. Bootstrap now calls the same `cmd_rebuild` path automatically when at least one page was written. New `--no-rebuild` flag preserves the old write-pages-only behavior for users who want to inspect candidates before materializing the graph. (`crates/illuminate-cli/src/commands/{bootstrap.rs,wiki.rs}`, `crates/illuminate-cli/src/main.rs`)

### Deferred to v0.16+

- `evidence` field shape change from `Option<String>` to `Vec<String>` for full `docs/AUDIT.md` parity.
- Bootstrap interactive TTY interview (the YAML schema is the stable contract; v0.11 ships file-driven only).
- `failure log` editor mode.
- MCP HTTP Server-Sent Events streaming.
- mTLS / OAuth for MCP HTTP.

## [0.14.0] — 2026-05-08

### Added — per-finding `confidence` score on audit results

- **`confidence: f64` on `PolicyViolation`, `Violation`, and `RelevantDecision`.** Per `docs/AUDIT.md`. Single shared `default_confidence()` helper in `response.rs` returns `1.0` so back-compat deserialization treats older payloads as fully confident. Population matrix:
  - `RejectedPattern` policy match → `1.0` (deterministic string match)
  - `MustUse` / `Frozen` policy match → `0.9` (rule-based, slightly less specific)
  - Decision conflict via NER entity match → `0.8`
  - `RelevantDecision` (semantic top-k) → `(rrf_score * 2.0).min(1.0)` — RRF scores are typically 0.0–0.5, so doubling brings them into a 0.0–1.0 range comparable to the policy/conflict tiers.
  CI gates can now branch on `confidence >= 0.8` to fail only on high-confidence findings; agent UIs can render confidence visually. CLI `print_human` appends `(confidence: X.XX)` to each finding line. JSON output includes `"confidence"` automatically via `#[derive(Serialize)]`. **MCP wire layer updated**: `policy_violation_to_json`, `violation_to_json`, and the inline `relevant_decisions` mapper in `crates/illuminate-mcp/src/tools.rs` now emit the field too — they hand-build JSON rather than `serde_json::to_value`, so new struct fields don't surface automatically. 6 new tests pin the score matrix and the MCP wire layer. (`crates/illuminate-audit/src/{policy.rs,response.rs,lib.rs}`, `crates/illuminate-cli/src/commands/audit.rs`, `crates/illuminate-mcp/src/tools.rs`)

### Deferred to v0.15+

- `evidence` field shape change from `Option<String>` to `Vec<String>` for full `docs/AUDIT.md` parity (current single-string form still satisfies the field, just narrower).
- Bootstrap interactive TTY interview.
- `failure log` editor mode.
- MCP HTTP Server-Sent Events streaming.
- mTLS / OAuth for MCP HTTP.

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
