# Bootstrap Pipeline Implementation Plan

**Goal:** Cold-start ingestion. `illuminate bootstrap` walks existing repo signals (CLAUDE.md / AGENTS.md / .cursorrules + ADRs) and emits wiki pages + graph episodes so day-one audits return meaningful findings.

**Architecture:** New crate `illuminate-bootstrap`. Each source has a parser that produces `Vec<BootstrapCandidate>`. The orchestrator dedupes (by id + content hash), writes wiki markdown, registers episodes via `illuminate::Graph::add_episode`. CLI `bootstrap` subcommand. Existing `illuminate-watch` git ingester already covers git history via `illuminate watch --git --backfill N` — we don't reinvent that here.

**Scope cut:** README parser, signal-scored git history, and interactive interview prompts are deferred to v0.2. The minimum viable bootstrap is agent files + ADRs.

---

## File structure

**Create:**
- `crates/illuminate-bootstrap/Cargo.toml`
- `crates/illuminate-bootstrap/src/lib.rs`
- `crates/illuminate-bootstrap/src/candidate.rs` — `BootstrapCandidate` struct
- `crates/illuminate-bootstrap/src/agent_files.rs` — parse CLAUDE.md / AGENTS.md / .cursorrules
- `crates/illuminate-bootstrap/src/adr.rs` — parse Nygard-style ADRs
- `crates/illuminate-bootstrap/src/orchestrate.rs` — run all sources, write outputs
- `crates/illuminate-bootstrap/tests/agent_files_test.rs`
- `crates/illuminate-bootstrap/tests/adr_test.rs`
- `crates/illuminate-bootstrap/tests/orchestrate_test.rs`
- `crates/illuminate-cli/src/commands/bootstrap.rs`

**Modify:**
- `crates/illuminate-cli/Cargo.toml` — add bootstrap dep
- `crates/illuminate-cli/src/main.rs` — add `Bootstrap` subcommand
- `crates/illuminate-cli/src/commands/mod.rs` — add `pub mod bootstrap;`
- `crates/illuminate-cli/src/commands/init.rs` — call bootstrap orchestrate after scaffold (optional, with `--skip-bootstrap` opt-out)

---

## Tasks

### B1: candidate type + agent_files parser

`BootstrapCandidate { id_slug, title, page_type: PageType, status, body, tags, source_kind, source_ref, confidence }`. Helpers to convert into a `WikiPage`-shaped markdown string.

Agent files (CLAUDE.md, AGENTS.md, .cursorrules, .windsurfrules, etc.) parser:
- Find any of those files at the repo root.
- Split content by markdown headings (`#`, `##`).
- Each heading + paragraph block becomes a candidate IF the body contains decision signals (keywords: "use", "do not", "never", "always", "we chose", "we use", "we reject", "instead of"). Filter out simple style/formatting guidance.
- `id_slug` = `agent-<filename>-<heading-slug>`. `confidence: 0.85`.

Tests: parse a fake CLAUDE.md fixture, assert correct number of candidates with expected ids + titles.

Commit: `feat(bootstrap): parse claude.md / agents.md / cursor rules as candidates`

### B2: ADR parser

Detect Nygard-style ADRs (heading: `# 0042: Title`, sections: `Status`, `Context`, `Decision`, `Consequences`).

Walk these directories: `docs/adr/`, `docs/decisions/`, `architecture/decisions/` (any that exist).
For each `.md` file, parse heading number+title and the four sections.
`id_slug = "adr-{number}-{slug}"`, `confidence: 1.0`, source_kind: `adr`, source_ref: file path.

Tests: parse a Nygard fixture, parse a malformed file (no heading number) which should be skipped or downgraded.

Commit: `feat(bootstrap): parse nygard-style adrs as decisions`

### B3: orchestrator + CLI

`run_bootstrap(repo_root: &Path) -> Result<BootstrapReport>`:
- Collect candidates from all sources.
- Dedup by `id_slug` (later sources don't overwrite earlier higher-confidence ones).
- For each candidate, write a `.illuminate/wiki/<dir>/<id_slug>.md` page (idempotent — skip existing files).
- Register each as a graph episode (best-effort).
- Append entries to `wiki/log.md`.
- Return `BootstrapReport { sources_run, candidates_found, pages_written, episodes_registered }`.

CLI: `illuminate bootstrap` runs the orchestrator on the current repo (must be opted-in). Prints the report.

Commit: `feat(bootstrap): orchestrator and bootstrap cli subcommand`

### B4: integrate into init

When `illuminate init` runs in a non-empty repo (i.e., agent files / ADRs exist), automatically run bootstrap. `--no-bootstrap` flag opts out.

Commit: `feat(cli): run bootstrap automatically during init`
