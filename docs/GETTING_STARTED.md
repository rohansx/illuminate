# Getting started with Illuminate

A step-by-step walkthrough. By the end you'll have a working `illuminate` setup that catches a Redis-vs-LRU decision conflict before any code is written.

Every command and output below was validated end-to-end in a tempdir before this doc shipped. If something diverges, file an issue.

---

## 1. Install

From source (recommended while OSS):

```bash
cargo install --git https://github.com/rohansx/illuminate illuminate-cli --locked
illuminate --version
# illuminate 0.X.Y
```

The binary is named `illuminate`. Everything else is library code linked into it.

---

## 2. Initialize a repo

```bash
cd your-project
git init        # if not already
illuminate init -n your-project
```

Output:

```
  Created: .illuminate/illuminate.toml
  Wiki scaffold: .illuminate/wiki/
Initialized illuminate for 'your-project'
  Database: .illuminate/graph.db

Get started:
  illuminate models download              Download ONNX models
  illuminate watch --git --backfill 100   Ingest git history
  illuminate serve                        Start MCP server
```

What `init` did:
- Created `.illuminate/illuminate.toml` — config + policies.
- Created `.illuminate/wiki/{decisions,patterns,failures,modules,_review}/` — markdown source-of-truth.
- Created an empty `.illuminate/graph.db` — the bi-temporal SQLite knowledge graph.
- Wiki scaffold includes a starter `index.md`, `log.md`, `schema.md`.

Optional flags:

| Flag | Effect |
|------|--------|
| `--claude` | Append the audit-pre-write directive to `CLAUDE.md`. |
| `--cursor` | Add MCP entry to `~/.cursor/mcp.json`. |
| `--windsurf` | Same for Windsurf. |
| `--hooks` | Wire `audit-hook` as a Claude Code PreToolUse hook (auto-blocks on `Write`/`Edit` violations). |

---

## 3. Capture your first decision

Edit `.illuminate/illuminate.toml` and add a policy:

```toml
[policies.no_redis]
rule = "rejected_pattern"
pattern = "Redis"
reason = "Use in-memory LRU with TTL instead — see dec-no-redis"
severity = "error"
decision_ref = "dec-no-redis"
```

Then create the matching wiki page at `.illuminate/wiki/decisions/dec-no-redis.md`:

```markdown
---
id: dec-no-redis
title: No Redis for caching
page_type: decision
status: active
tags: [caching, infrastructure]
created: 2026-05-09T09:00:00Z
updated: 2026-05-09T09:00:00Z
---

## Decision

We do not use Redis for caching in this project.

## Context

Redis adds a stateful sidecar that breaks our deployment story (single binary + SQLite). The performance gain over a tuned in-memory LRU is rarely worth the operational overhead.

## Consequences

- Caches are in-memory LRUs with TTL, scoped per-process.
- Cache stampedes are mitigated with ±10% jitter on TTLs.
- Trade-off: cache state is lost on restart.
```

Two layers of enforcement now exist:

| Layer | What it does |
|-------|--------------|
| `[policies.no_redis]` in TOML | Hard rule. Audit blocks (exit 2) on any plan containing "Redis". |
| `wiki/decisions/dec-no-redis.md` | Rich context. Surfaces in the audit response's `wiki_url` and in `illuminate explain` output. |

---

## 4. (Optional) Download extraction models

Without models, illuminate still works for policy-based audits (string match is deterministic, no model needed).

With models, the NER pipeline (GLiNER + GLiREL via ONNX) extracts entities and relations from your trail/wiki/git data, populating the graph for semantic search:

```bash
illuminate models download
# downloads ~150MB into ~/.cache/illuminate/models/
```

If you want to skip this entirely (CI, low-disk environments), set `ILLUMINATE_NO_EMBED=1` and any embed-using path will degrade gracefully.

---

## 5. Bootstrap from existing signals

You can capture decisions without writing every wiki page by hand. `illuminate bootstrap` walks five sources:

1. **Agent files**: `CLAUDE.md`, `AGENTS.md`, `.cursorrules`, `.windsurfrules`, `.clinerules`.
2. **ADRs**: any `docs/adr/`, `docs/decisions/`, `architecture/decisions/` directory in Nygard format.
3. **Git history**: last 6 months of commits matching decision-shaped subject lines or signal phrases (`switch`, `instead of`, `we decided`).
4. **README + CONTRIBUTING**: architecture-style sections (`## Architecture`, `## Tech Stack`, `## Decisions`, ...).
5. **Interview YAML**: structured Q&A in `.illuminate/interview.yaml`.

The interview YAML is the highest-confidence source (you wrote it explicitly), so its candidates land in the wiki directly. Lower-confidence sources route to `.illuminate/wiki/_review/` for human curation.

Example `.illuminate/interview.yaml`:

```yaml
language: "Rust 2024"
database: "PostgreSQL with sqlx"
architecture: "modular monolith, single binary"
deployment: "single binary + SQLite, no Docker, no Redis"
avoid:
  - "Redis for caching"
  - "global mutable state"
prefer:
  - "in-memory LRU with TTL for caching"
  - "structured tracing logs"
services:
  - name: "audit"
    description: "the contextual linter — checks plans against policies + the graph"
```

Run:

```bash
illuminate bootstrap
```

Output:

```
bootstrap complete:
  sources run:        ["interview"]
  candidates found:   8
  pages written:      8
  pages skipped:      0
  pages queued for review: 0

rebuilding wiki index + graph...
rebuilt index.md (8 pages); registered 8 episodes
```

Each scalar field becomes one decision page; each `avoid:` / `prefer:` list item becomes one; each `services:` entry becomes a module page.

> v0.15+ note: bootstrap auto-runs `wiki rebuild` so the graph is materialized before you exit. Use `bootstrap --no-rebuild` if you want to inspect candidates first.

---

## 6. Run your first audit

The audit takes a plan in plain language and returns structured findings.

**A violating plan** (the policy you added in step 3 fires):

```bash
illuminate audit "add Redis caching to txn lookup"
```

```
✗ Violations detected:

  Policy: no_redis
  Found: Redis
  Reason: Use in-memory LRU with TTL instead — see dec-no-redis
  Severity: Error (confidence: 1.00)
```

Exit code: `2`. Use this in CI to block changes.

**A benign plan**:

```bash
illuminate audit "refactor LRU cache for clarity"
```

```
✓ No violations detected
```

Exit code: `0`.

**JSON for tooling**:

```bash
illuminate audit "add Redis caching" --json
```

```json
{
  "status": "violation",
  "policy_violations": [
    {
      "policy_name": "no_redis",
      "found": "Redis",
      "reason": "Use in-memory LRU with TTL instead — see dec-no-redis",
      "severity": "error",
      "decision_ref": "dec-no-redis",
      "evidence": "plan contains 'Redis'",
      "confidence": 1.0
    }
  ],
  "impact": { "seed_symbols": [], "impacted_symbols": [], "truncated": false },
  "relevant_decisions": [],
  "trace_id": "5503b55e-bb16-461e-ba03-8fb40977d0b5",
  "policies_applied": ["no_redis"],
  "wiki_url": ".illuminate/wiki/decisions/dec-no-redis.md"
}
```

| Exit code | Meaning |
|-----------|---------|
| `0` | Pass |
| `2` | Violation (CI should block) |
| `3` | Warning (CI may surface but not block) |

---

## 7. Add code-graph blast-radius (optional but powerful)

```bash
illuminate index           # tree-sitter symbol + edge index for the source tree
illuminate audit "refactor process_payment" src/payments/cache.rs --index-db .illuminate/index.db
```

Now the audit response includes a `Blast radius` section listing impacted symbols (callers + callees within depth=2):

```
Blast radius: 4 symbols impacted across 2 files
  - file::src/orders/checkout.rs
  - src/orders/checkout.rs::create_order
  - file::src/payments/cache.rs
  - src/payments/cache.rs::process_payment
```

Six languages are supported for both imports + function-call edges: Rust, Go, TypeScript, Python, Java, C (C++ via the shared C grammar — `#include` extraction works; full body parsing is best-effort).

For a read-only inspection of any file:

```bash
illuminate impact src/payments/cache.rs        # symbols defined, imports out, blast radius
illuminate explain src/payments/cache.rs       # decisions / patterns / failures linked to this file
illuminate decisions for src/payments/         # decisions whose content mentions this path
```

---

## 8. Wire MCP for live agent integration

**Stdio (Claude Code, Cursor)**: add to `~/.claude.json` (or the equivalent for your agent):

```json
{
  "mcpServers": {
    "illuminate": {
      "command": "illuminate",
      "args": ["mcp", "start"]
    }
  }
}
```

**HTTP (network / CI / shared runner)**: launch the server with bearer-token auth:

```bash
ILLUMINATE_HTTP_TOKEN=$(openssl rand -hex 32)
illuminate mcp serve --http --bind 127.0.0.1:7800
```

Configure `[mcp.http]` in `.illuminate/illuminate.toml`:

```toml
[mcp.http]
bind = "127.0.0.1:7800"
bearer_token_env = "ILLUMINATE_HTTP_TOKEN"
```

Tools an agent can call:

| Tool | When to use |
|------|-------------|
| `illuminate_audit` | Before writing/editing code. Pass `plan` (required) and `files` (optional). |
| `illuminate_impact` | Inspect blast-radius for a file without running policy checks. |
| `illuminate_explain` | Find decisions/patterns/failures touching a file. |
| `illuminate_decisions_for` | List decisions affecting a path. |
| `illuminate_failures_for` | List failure episodes for a path. |
| `illuminate_get_wiki_page` | Fetch a wiki page by id. |
| `illuminate_search` | FTS5 + semantic search across the graph. |
| `illuminate_reflect` | Record a failure inline (the agent asks the team for context). |

Plus MCP-protocol surfaces:
- `resources/list` and `resources/read` expose wiki pages with `illuminate://wiki/{decisions,patterns,failures,modules}/<id>` URIs.
- `prompts/list` and `prompts/get` provide `illuminate_audit_check` (reminds the agent to call audit first) and `illuminate_summarize_failures`.

---

## 9. The flywheel

Once everything is wired:

1. **Run a Claude Code session.** Sessions land in `~/.claude/projects/<hash>/<session>.jsonl`.
2. **`illuminate trail register <jsonl>`.** Trail captured to `.illuminate/trail/`. NER runs (if models installed). Entities and relations land in the graph.
3. **Next audit is smarter.** `audit` semantic top-k surfaces decisions you forgot about. Blast-radius traverses real call edges.
4. **A bug ships → run `illuminate failure log`.** Future audits surface the lesson.

```bash
illuminate failure log \
  --title "Cache stampede on hot keys" \
  --root-cause "no jitter in TTL" \
  --fix "added ±10% TTL jitter" \
  --severity high \
  --files src/cache.rs
```

```
wrote .illuminate/wiki/failures/2026-05-09-cache-stampede-on-hot-keys.md
registered as graph episode 019e0bfe-837d-7f33-aa8f-732aac746eb4
```

---

## 10. CI integration

Use the prebuilt composite action:

```yaml
# .github/workflows/illuminate-audit.yml
on: pull_request
jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: rohansx/illuminate/.github/actions/audit-pr@master
```

The action runs `illuminate audit-pr ${{ github.event.pull_request.number }} --comment --format markdown`, which posts findings as a PR comment via `gh`. Block on exit 2; soft-warn on exit 3.

---

## 11. Common questions

**How do I see what's in the graph?**

```bash
illuminate stats              # episode/entity/edge counts
illuminate decisions list     # all decisions
illuminate patterns list      # all patterns
illuminate failures list      # all failures
illuminate search "caching"   # FTS5 + semantic
```

**How do I redact something sensitive that slipped through?**

```bash
illuminate wiki redact "secret-token-\\w+" --dry-run    # preview
illuminate wiki redact "secret-token-\\w+"              # apply (replaces in files + deletes graph episodes)
```

**How do I disagree with a decision?**

Edit the wiki page (`.illuminate/wiki/decisions/<id>.md`), set `status: superseded`, and write a new decision page with the new direction. Then `illuminate wiki rebuild` to re-register in the graph.

**My CI is too slow because of model loading.**

Set `ILLUMINATE_NO_EMBED=1` in CI. Audits still work (string match + FTS5); semantic top-k just stays empty.

**The audit response is too verbose.**

Use `--json` and pipe through `jq` to extract only what your tooling needs:

```bash
illuminate audit "..." --json | jq -r '.policy_violations[] | "\(.policy_name): \(.reason)"'
```

---

## 12. What's next

- **[ARCHITECTURE.md](ARCHITECTURE.md)** — the two-graph design (code graph ↔ decision graph) and the audit lifecycle.
- **[AUDIT.md](AUDIT.md)** — full audit response shape and policy DSL.
- **[MCP.md](MCP.md)** — MCP protocol surface, tool schemas, transport options.
- **[CLI.md](CLI.md)** — every command's flags.
- **[BOOTSTRAP.md](BOOTSTRAP.md)** — bootstrap source semantics.
- **[CHANGELOG.md](../CHANGELOG.md)** — per-version log.
