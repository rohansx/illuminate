# Changelog

All notable changes to Illuminate are tracked here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

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
