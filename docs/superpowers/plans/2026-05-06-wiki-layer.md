# Wiki Layer Implementation Plan

**Goal:** SCHEMA.md-aligned markdown wiki: parse front-matter, validate against schema, scaffold wiki/ in `init`, render an index, expose via CLI (`wiki rebuild`, `wiki lint`, `wiki serve`, `wiki review`). Pages get registered into the graph as episodes (so audit can surface them).

**Architecture:** New crate `illuminate-wiki` (workspace member). Lib API: `WikiPage` struct, `parse_page(&str) -> Result<WikiPage>`, `lint_page(&WikiPage) -> Vec<LintError>`, `walk_wiki(&Path) -> impl Iterator<Item=WikiPage>`, `render_index(&[WikiPage]) -> String`. CLI wires `wiki` subcommand similar to `trail`. `init` writes `wiki/{schema.md,index.md,log.md}` plus empty `decisions/`, `patterns/`, `failures/`, `modules/` dirs.

**Tech stack:** Rust 2024, `serde`, `serde_yaml` (new dep — for YAML front-matter), `pulldown-cmark` (new dep — for `wiki serve` rendering), `chrono`, `tokio`, `clap`. Re-uses `illuminate-core::Graph` for episode registration.

---

## File structure

**Create:**
- `crates/illuminate-wiki/Cargo.toml`
- `crates/illuminate-wiki/src/lib.rs` — re-exports
- `crates/illuminate-wiki/src/page.rs` — `WikiPage`, `FrontMatter`, parsing
- `crates/illuminate-wiki/src/lint.rs` — `lint_page`, `LintError`
- `crates/illuminate-wiki/src/walk.rs` — `walk_wiki` directory traversal
- `crates/illuminate-wiki/src/render.rs` — `render_index`, simple HTML render
- `crates/illuminate-wiki/src/scaffold.rs` — `write_scaffold(&Path)` creates initial wiki layout
- `crates/illuminate-wiki/src/episode.rs` — `page_to_episode(&WikiPage) -> illuminate::Episode`
- `crates/illuminate-wiki/tests/page_test.rs` + `lint_test.rs` + `walk_test.rs` + `scaffold_test.rs`
- `crates/illuminate-cli/src/commands/wiki.rs`

**Modify:**
- `Cargo.toml` (workspace) — add `serde_yaml` and `pulldown-cmark` to workspace deps
- `crates/illuminate-cli/Cargo.toml` — depend on `illuminate-wiki`
- `crates/illuminate-cli/src/main.rs` — add `Wiki { cmd }` variant
- `crates/illuminate-cli/src/commands/mod.rs` — add `pub mod wiki;`
- `crates/illuminate-cli/src/commands/init.rs` — call `scaffold::write_scaffold` on init

---

## Tasks

### W1: Create crate skeleton + WikiPage + parse_page

Files: `crates/illuminate-wiki/{Cargo.toml,src/lib.rs,src/page.rs,tests/page_test.rs}`. Workspace `Cargo.toml`: add `serde_yaml = "0.9"` and `pulldown-cmark = "0.12"`.

- `WikiPage { front: FrontMatter, body: String }`
- `FrontMatter { id, title, page_type, status, created, updated, tags, modules, related, supersedes, superseded_by, confidence, sources, authors }` — all serializable
- `parse_page(&str) -> Result<WikiPage>` — parses `---\nyaml\n---\nbody`
- 4 tests: minimal valid page, missing front-matter, invalid YAML, full schema page

Commit: `feat(wiki): wikipage parser with yaml front-matter`

### W2: Lint wiki pages against SCHEMA.md

- `LintError { code: LintCode, message: String }`, `LintCode` enum (MissingId, IdSlugMismatch, InvalidStatus, BadDates, MissingRequiredSection, UnknownReference, etc.)
- `lint_page(&WikiPage) -> Vec<LintError>` — checks ID format, status whitelist, created ≤ updated, type-specific required sections (Decision pages need `## Decision/Context/Consequences`; Failure pages need `## What broke/Root cause/Fix/Lesson for future agents`)
- 5 tests: clean page, missing decision section, bad status, mismatched date, mismatched id format

Commit: `feat(wiki): lint pages against schema rules`

### W3: Walk a wiki directory + tests

- `walk_wiki(root: &Path) -> impl Iterator<Item=WalkedPage>` (or `Vec<WalkedPage>` — simpler)
- `WalkedPage { path: PathBuf, page: Result<WikiPage, ParseError> }`
- 2 tests: empty dir, fixture dir with one decision

Commit: `feat(wiki): walk wiki directory and parse all pages`

### W4: Scaffold writer

- `write_scaffold(repo_root: &Path) -> Result<()>` writes `.illuminate/wiki/schema.md` (copy of `docs/SCHEMA.md` from this repo or a baked-in template), `index.md`, `log.md`, plus empty `decisions/`, `patterns/`, `failures/`, `modules/` dirs (with `.gitkeep`).
- Idempotent: skips existing files.
- 1 test: scaffold a tempdir, assert files present.

Commit: `feat(wiki): scaffold writer for fresh repos`

### W5: Render index

- `render_index(pages: &[WikiPage]) -> String` — markdown index grouped by type, with link to each page.
- Stub HTML render via `pulldown-cmark` for `wiki serve`.
- 2 tests: empty index, mixed-type index ordering.

Commit: `feat(wiki): render index from page list`

### W6: page → graph episode mapping

- `page_to_episode(page: &WikiPage) -> illuminate::Episode` — wraps the wiki page into an `Episode` with `source = "wiki"`, content = page body, metadata = serialized front-matter.
- 1 test: round-trip a page through `page_to_episode`.

Commit: `feat(wiki): convert wiki pages to graph episodes`

### W7: CLI `wiki` subcommand

- `commands/wiki.rs` with `WikiCmd::{Lint, Rebuild, Serve, List}`.
- `lint`: walk wiki, run lint_page on each, print errors, exit nonzero if any.
- `rebuild`: walk wiki, write each as episode to graph (create graph if missing; reuse `commands::open_graph` pattern).
- `serve`: render index + each page as HTML via `pulldown-cmark`, serve over `tiny_http` on `--port` (or just print links if we want to avoid a new dep — let's print for now and defer the HTTP server).
- `list`: print pages by type with ID + title.
- Wire into `commands/mod.rs` + `main.rs`.

Commit: `feat(cli): wiki subcommand (lint/rebuild/serve/list)`

### W8: integrate scaffold into `init`

- Read existing `commands/init.rs`. Add a call to `illuminate_wiki::scaffold::write_scaffold(repo_root)` after the existing init writes its files. Idempotent so re-init is safe.

Commit: `feat(cli): scaffold wiki on illuminate init`

### W9: dogfood

- Add a sample decision page to this repo's `.illuminate/wiki/decisions/` based on the actual no-redis-payments fictional example, run `illuminate wiki lint` then `illuminate wiki list`. Verify output.

Commit: `chore: dogfood wiki with sample decision`
