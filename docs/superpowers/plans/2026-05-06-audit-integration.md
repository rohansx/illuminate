# Audit Integration Implementation Plan

**Goal:** Wire all the upstream pieces (trail, wiki, bootstrap) into the audit so `illuminate audit "<plan>"` returns real findings on a populated repo. The audit engine and MCP tool already exist; this plan is integration plumbing.

**Architecture:** (1) Add `trail register` CLI: turn captured trails into graph episodes (raw content, no NER — sufficient for FTS5 audit search). (2) Make audit's policy loader find `.illuminate/illuminate.toml` (current code only looks at `cwd/illuminate.toml`). (3) Make `illuminate init` write a `CLAUDE.md` directive instructing the agent to call `illuminate_audit` before writes. (4) End-to-end smoke.

**Scope cut:** Automatic NER on trails is dev-triggered per ROADMAP. We register trails as raw episodes; the existing `illuminate watch --git` flow can do NER-flavored ingestion when desired. MCP server already wires `illuminate_audit`; no changes needed to it.

---

## Tasks

### C1: `illuminate trail register` — register trail content as episodes

Add a subcommand `Register { ident: Option<String> }` to `commands/trail.rs`. Without `ident`, registers all trails. With one, registers just that trail. Each trail's messages get concatenated into one episode body (one episode per session — keeps the FTS5 index focused on the session, not per-message).

Episode shape: `content = "\n\n".join([f"[{role}] {text}" for msg in messages])`, `source = "trail:claude-code"`, metadata = `{ session_id, started_at, ended_at }`.

Tests aren't strictly necessary for CLI plumbing — exercise via smoke test in C4.

Commit: `feat(cli): trail register turns captured trails into graph episodes`

### C2: Audit policy loader walks ancestors for `.illuminate/illuminate.toml`

Modify `crates/illuminate-cli/src/commands/audit.rs::load_policies`. Replace the `cwd.join("illuminate.toml")` lookup with the same ancestor-walk pattern used elsewhere (`.illuminate/illuminate.toml` first, fall back to legacy `illuminate.toml` at cwd).

Commit: `fix(audit): load policies from .illuminate/illuminate.toml`

### C3: `illuminate init --claude` writes the CLAUDE.md directive

Read `crates/illuminate-cli/src/commands/init.rs`. Find where `--claude` is handled. Write or append to `<repo>/CLAUDE.md`:

```markdown
## illuminate: required pre-write step

Before writing or modifying any source file in this repository, invoke
the `illuminate_audit` MCP tool with your plan, the files you intend to
modify, and your rationale. Surface the response to the user before
proceeding.

If `status` is `block`, do not proceed without explicit user approval.
If `status` is `warn`, surface the warnings to the user before writing.
```

Idempotent: if the section already exists in CLAUDE.md, skip.

Commit: `feat(cli): init --claude appends pre-write directive to claude.md`

### C4: end-to-end smoke

Ad-hoc smoke test:

1. Set up a tempdir with `.illuminate/illuminate.toml` containing one policy:
```toml
[policies.no_redis]
rule = "rejected_pattern"
pattern = "Redis"
reason = "deployment target disallows stateful sidecars"
severity = "error"
```
2. Add a CLAUDE.md with a "## Caching\n\nUse Memcached. Never use Redis." section.
3. Run `illuminate bootstrap` → pages written.
4. Run `illuminate wiki rebuild` → episodes registered.
5. Run `illuminate audit "add Redis caching to billing service"` → expect a violation.
6. Run `illuminate audit "add Memcached caching"` → expect pass.

Commit a small dogfood file documenting the result on this repo if anything stands out.
