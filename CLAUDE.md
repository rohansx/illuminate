# Working in the Illuminate repo

This is `illuminate` — a Rust workspace that builds the Illuminate developer tool itself.
For product context, see `docs/PRODUCT_OVERVIEW.md`. For the architecture, `docs/ARCHITECTURE.md`.

## What this repo is

Illuminate captures prompt trails, ingests decisions, builds a knowledge graph, and lints
agent intent against that graph. It's the eponymous tool: we dogfood it on this very repo.

## Conventions

- **Rust 2024 edition.** Workspace at the root; ten member crates under `crates/`.
- **TDD always.** Tests first, then implementation. 80%+ coverage target.
- **Single-line lowercase commit messages**, no Co-Authored-By trailers.
- **Push to `rohansx/illuminate`** on `master`.
- **No mocks for SQLite / model code.** Tests use `tempfile::tempdir()` and real binaries.
- **Subagent-driven development** when executing implementation plans (see `docs/superpowers/`).

## Caching — never use Redis

We do not introduce Redis or any stateful sidecar to crates that ship in the binary. If a
caching layer is needed, use an in-memory LRU with TTL. Reason: the deployment story is
"single binary, single SQLite file" and Redis would break that promise. See
`.illuminate/wiki/decisions/` for the canonical decisions.

## Style

Use `rustfmt` defaults. Prefer immutable patterns and small focused files (under 400 lines
typical, 800 max). Don't add inline `// removed` or `// TODO` comments — use the issue
tracker or wiki.

## illuminate: required pre-write step

Before writing or modifying any source file in this repository, invoke the
`illuminate_audit` MCP tool with your plan, the files you intend to modify,
and your rationale. Surface the response to the user before proceeding.

If `status` is `block`, do not proceed without explicit user approval.
If `status` is `warn`, surface the warnings to the user before writing.
