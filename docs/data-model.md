# Illuminate — Data Model

**Companion doc to** [`PRODUCT_OVERVIEW.md`](PRODUCT_OVERVIEW.md) and [`ARCHITECTURE.md`](ARCHITECTURE.md).
Captures what Illuminate stores, where it stores it, how long it keeps it, and what gets filtered out.

This is the document security buyers, compliance reviewers, and curious developers will read before deciding whether to install. Get it right and trust follows. Get it wrong and adoption stalls.

> **Note on commands:** `ilm` in this document is the planned shorthand alias for the `illuminate` CLI (landing in v3.2 alongside `illuminate ask` / `illuminate browse`). Every `ilm <subcommand>` example here is equivalent to `illuminate <subcommand>`. See [`ROADMAP.md`](ROADMAP.md#v32--docs-as-first-class-content).

---

## The Core Decision

When a coding session ends, Illuminate doesn't store one thing. It stores *three* things at three different layers, with three different lifecycles, serving three different purposes:

| Layer | What | Where | Lifecycle | Purpose |
|---|---|---|---|---|
| **Raw Trail** | Full session jsonl | Local only | Days to weeks | Fidelity, recovery, extraction source |
| **Session Summary** | Structured markdown | Local → team repo on consent | Permanent if published | Human reading, onboarding, review |
| **Graph Entities** | Structured nodes/edges | Local + team repo | Permanent | Fast deterministic queries by agents |

Raw is fidelity. Summaries are durability. Graph entities are query speed. All three serve different purposes; none is redundant.

---

## Layer 1: Raw Trail

The complete raw session as captured from Claude Code, Cursor, or Codex. Every prompt, every model response, every tool call, every file edit, every timestamp.

### What gets stored

```jsonl
{"type": "session_start", "session_id": "abc123", "model": "claude-sonnet-4-5", "repo": "payments-service", "timestamp": "2026-05-25T09:15:00Z"}
{"type": "user_message", "content": "Add caching to the transaction lookup endpoint", "timestamp": "2026-05-25T09:15:03Z"}
{"type": "assistant_message", "content": "I'll help you add caching...", "timestamp": "2026-05-25T09:15:08Z"}
{"type": "tool_call", "tool": "read_file", "args": {"path": "src/payments/txn.rs"}, "timestamp": "2026-05-25T09:15:10Z"}
{"type": "tool_result", "content_ref": "git:a1b2c3d:src/payments/txn.rs", "timestamp": "2026-05-25T09:15:10Z"}
{"type": "assistant_message", "content": "Looking at the txn.rs file, I see...", "timestamp": "2026-05-25T09:15:15Z"}
...
{"type": "session_end", "session_id": "abc123", "duration_seconds": 2832, "turn_count": 23, "timestamp": "2026-05-25T10:02:15Z"}
```

### Where it lives

`~/.illuminate/trail/<repo>/<session-id>.jsonl` on the developer's local machine.

For repo-scoped storage: `<repo>/.illuminate/trail/<session-id>.jsonl` — gitignored by default.

### Lifecycle

- **Default retention:** 30 days local. Configurable per team via `illuminate.toml` (`trail.retention_days = 90` etc.).
- **Pinning:** developers can pin specific sessions to keep longer (`ilm pin <session-id>`).
- **Archival:** after retention period, raw trails are deleted by default. Optionally compressed and moved to `~/.illuminate/archive/`.
- **Never auto-uploaded.** The raw trail never leaves the developer's machine without an explicit publish gesture.

### What we filter out before writing

Even at the raw layer, some content is filtered to keep the trail useful and safe:

- **System heartbeats and keepalives.** Pure noise. Stripped.
- **Failed model retries.** If Claude Code retries a turn due to rate limit or transient error, only the successful turn is kept. Retries collapsed.
- **Thinking blocks beyond the most recent N turns.** Claude's `<thinking>` blocks are useful in the moment but mostly redundant once the response is final. Default: keep thinking blocks for the last 3 turns only.
- **Tool call results that duplicate git state.** When a tool reads a file, we store a reference (`git:<commit>:<path>`) rather than the full file contents. The file at that commit is recoverable from git.
- **Detected secrets and PII** (see [Redaction Policy](#redaction-policy) below). Secrets are *never* written to disk in plaintext, even at the raw layer.

### Storage characteristics

- **Average session size:** ~500KB (after filtering, before compression)
- **Per-developer per-year:** ~25MB rotating (200 sessions × 90 day retention)
- **Per-team per-year:** zero — raw trail never goes to the team layer

---

## Layer 2: Session Summary

A structured markdown page summarizing the session. This is what humans actually read for onboarding, code review, and "why does this exist" questions.

### Schema

```markdown
---
session_id: 2026-05-25-add-caching-claude
date: 2026-05-25T09:15:00Z
duration_minutes: 47
turn_count: 23
model: claude-sonnet-4-5
agent: claude-code
repo: payments-service
author: rohan
files_touched:
  - src/payments/txn.rs
  - src/payments/cache.rs
  - tests/integration/cache_test.rs
commit: a1b2c3d
prompt_intent: "Add caching to the transaction lookup endpoint"
decisions_made:
  - id: decision-2026-05-25-lru-caching-payments
    summary: "Used LRU with 30s TTL instead of Redis"
  - id: decision-2026-05-25-lockfree-atomics-txn
    summary: "Lock-free atomics over mutex for the txn lock path"
alternatives_considered:
  - alternative: "Redis"
    reason_rejected: "Deployment constraints — service requires stateless sidecars"
  - alternative: "Mutex on txn lock"
    reason_rejected: "Known race condition history in this module"
patterns_applied:
  - patterns/lru-cache-with-ttl.md
warnings_surfaced:
  - failures/2026-02-race-condition-payments.md
new_patterns_proposed: []
new_failures_recorded: []
redaction_applied: ["secrets-detection", "pii-scrub"]
publish_status: published-summary
---

## What happened

The developer asked for caching on the transaction lookup endpoint. Illuminate surfaced two pieces of relevant context during enrichment: the team's prior decision to reject Redis for this service, and a known race condition history in the txn lock path.

The agent proposed an LRU cache with a 30-second TTL based on the team's established pattern, with lock-free atomics to avoid the race condition area. The developer approved the approach.

Implementation took three iterations. The first version had a subtle bug in concurrent reads; the second version fixed it but introduced a memory growth issue under high load; the third version landed cleanly with bounded memory and concurrent-read safety.

## Key reasoning

When choosing between LRU and a time-windowed cache, the agent considered the access pattern of transaction lookups — heavily skewed toward recent transactions, with a long tail of older ones. LRU was a clear fit. A time-windowed cache would have evicted hot recent transactions during quiet periods.

For the concurrency strategy, the agent initially suggested a `parking_lot::Mutex`. The developer pushed back, citing the failure history. The agent then proposed `crossbeam::AtomicCell` for the txn lock, which fit the existing pattern in the service.

## What didn't work

Iteration 1: Used `HashMap` directly. Concurrent reads racing with evictions caused panics under load. Test coverage caught it.

Iteration 2: Switched to `DashMap`. Solved concurrency but unbounded growth in memory. Required adding an LRU bound.

Iteration 3: Used `lru::LruCache` wrapped in atomic operations. Landed.

## Links

- Commit: [a1b2c3d](https://github.com/team/payments-service/commit/a1b2c3d)
- Raw trail: local only (`trail/2026-05-25-add-caching-claude.jsonl`)
- Related decision: [decisions/2025-12-no-redis-payments.md](decisions/2025-12-no-redis-payments.md)
- Related pattern: [patterns/lru-cache-with-ttl.md](patterns/lru-cache-with-ttl.md)
- Related failure: [failures/2026-02-race-condition-payments.md](failures/2026-02-race-condition-payments.md)
```

### Where it lives

- **Local (always):** `<repo>/.illuminate/sessions/<session-id>.md`
- **Team repo (if published):** `team-illuminate/sessions/<session-id>.md`

### Lifecycle

- **Generated automatically** at session end. Uses a local LLM call (or cheap cloud call with PII stripped) to generate the natural-language sections. Structured metadata extracted via NER.
- **Reviewed by the developer** before publication. The dev can edit any section, choose what to redact, and decide what to publish.
- **Published only via explicit gesture** (`ilm publish <session>` or the commit-time prompt).
- **Permanent once published.** Lives in the team git repo. Can be amended via standard git commits, never silently rewritten.

### What the developer controls

At publish time, the developer chooses:

```
📦 Session: 2026-05-25-add-caching
   Duration: 47 minutes, 23 turns
   Files: src/payments/txn.rs, src/payments/cache.rs, tests/...

   Extracted:
   ✓ 1 decision: "Use LRU with 30s TTL for transaction caching"
   ✓ 0 new patterns (applied existing patterns/lru-cache-with-ttl.md)
   ✓ 1 reference to existing failure (race-condition-payments.md)

   What to publish?
   [x] Session summary (recommended)
   [x] Extracted decisions
   [ ] Full raw transcript  ← unchecked by default
   [ ] Files changed (already in git)

   Redaction:
   [x] Strip detected secrets and PII
   [ ] Anonymize file paths
   [ ] Anonymize commit author

   [Publish]  [Skip this session]  [Edit summary first]
```

Sensible defaults. Friction near zero. Every choice is explicit.

### Storage characteristics

- **Average summary size:** ~5KB
- **Per-developer per-year:** ~1MB (200 sessions, all published)
- **Per-team per-year:** ~10MB for a 10-engineer team

---

## Layer 3: Graph Entities

The structured nodes and edges that flow into the knowledge graph. These are what `illuminate_enrich` and `illuminate_audit` actually query — fast, deterministic, no LLM in the path.

### Entity types

**Decision**
```yaml
type: Decision
id: decision-2026-05-25-lru-caching-payments
title: "Use LRU with 30s TTL for transaction caching"
created: 2026-05-25T10:02:15Z
status: active
supersedes: []
contradicts: []
confidence: high
references:
  files: ["src/payments/txn.rs", "src/payments/cache.rs"]
  modules: ["payments-service"]
  patterns: ["patterns/lru-cache-with-ttl.md"]
  failures: ["failures/2026-02-race-condition-payments.md"]
  rejected_alternatives: ["Redis", "Memcached", "Mutex on txn lock"]
source:
  session: sessions/2026-05-25-add-caching-claude.md
  commit: a1b2c3d
```

**Pattern**
```yaml
type: Pattern
id: pattern-lru-cache-with-ttl
title: "LRU cache with 30s TTL"
created: 2025-12-08T14:22:00Z
status: established
references:
  used_by: ["src/payments/", "src/inventory/", "src/notifications/"]
  related_decisions: ["decision-2025-12-no-redis-payments"]
usage_count: 14
last_used: 2026-05-25T10:02:15Z
```

**Failure**
```yaml
type: Failure
id: failure-2026-02-race-condition-payments
title: "Race condition in txn lock under concurrent writes"
created: 2026-02-14T03:47:00Z
severity: high
root_cause: "Mutex contention causing deadlock under load > 1000 req/s"
fix_commit: e4f5g6h
references:
  files: ["src/payments/txn.rs"]
  modules: ["payments-service"]
  related_decisions: ["decision-2026-02-lockfree-txn"]
recurrence_check:
  triggers_on_files: ["src/payments/txn.rs"]
  warning_message: "Race condition history in this file. Avoid mutex on txn lock."
```

**Module**
```yaml
type: Module
id: module-payments-service
title: "Payments service"
created: 2025-08-01T00:00:00Z
references:
  paths: ["src/payments/"]
  related_decisions: [...]
  related_patterns: [...]
  related_failures: [...]
  active_docs: ["docs/architecture/payments.md", "docs/runbooks/payments-rollback.md"]
```

**Doc** (when docs are first-class — see [`knowledge-layer.md`](knowledge-layer.md))
```yaml
type: Doc
id: doc-architecture-payments
title: "Payments service architecture"
path: docs/architecture/payments.md
created: 2025-08-15T00:00:00Z
last_modified: 2026-04-22T00:00:00Z
doc_type: architecture
references:
  modules: ["payments-service"]
  related_decisions: [...]
freshness_status: current  # or "may-be-stale" / "stale"
```

### Where it lives

- **Local query store:** `.illuminate/graph.db` (SQLite via ctxgraph)
- **Team-shared mirror:** `team-illuminate/graph-export/` as structured JSON files, one per entity. This is the portable, reviewable, git-versioned form of the graph.

The SQLite database is regeneratable from the JSON export. If `graph.db` is deleted, `ilm rebuild` reconstructs it. This makes the team repo the source of truth and the local SQLite a query cache.

### Lifecycle

- **Generated automatically** by the extraction pipeline at session end.
- **Append-only.** Supersession is a new node with a `supersedes:` relation, not a destructive edit.
- **Permanent.** Graph entities are never deleted, only superseded.
- **Versioned via git** at the team layer (JSON files diff cleanly).

### What gets extracted vs. what doesn't

The extraction pipeline pulls structured facts from session summaries and commit messages. It extracts:

- **Decisions:** "We chose X over Y because Z"
- **Patterns:** Recurring approaches across sessions ("the team uses pattern P in 14 files")
- **Failures:** Post-mortems, incident reports, "this broke when..."
- **Entity references:** file paths, module names, library names, technology choices

It does *not* extract:

- Verbatim model responses (kept only in the raw layer)
- Conversation flow (the back-and-forth structure)
- Specific implementation details (those live in the code itself)
- Subjective tone or preferences

The graph stores *what is true about the team's choices*, not *what was said in conversation*.

### Storage characteristics

- **Average entity size:** ~2KB JSON + ~0.5KB embedding
- **Per-developer per-year:** ~400KB (assuming 200 sessions, ~1 entity per session)
- **Per-team per-year:** ~5MB for a 10-engineer team

---

## Redaction Policy

Some content cannot be stored, even at the raw layer. Redaction runs *before* any content is written to disk.

### What gets redacted automatically

**Detected secrets:**
- API keys (OpenAI, Anthropic, AWS, GCP, Stripe, GitHub, etc. — pattern detection)
- Tokens (bearer tokens, JWT tokens, OAuth tokens)
- Passwords (when prefixed by identifiable patterns)
- Private keys (RSA, ECDSA, SSH)
- Connection strings with embedded credentials

When detected:
- The secret is replaced with `[REDACTED-SECRET-<type>]`
- The session is marked `contains_redacted: true`
- The dev is notified at session end

**Detected PII (configurable per team):**
- Email addresses (optional; some teams keep, some scrub)
- Phone numbers
- SSN / national ID patterns
- Credit card numbers (PCI requirement)
- Custom regex patterns defined in `illuminate.toml`

**File path scrubbing (optional):**
- Some teams scrub absolute paths to relative (e.g., `/Users/rohan/work/secret-project/` → `/`)
- Configurable per team

### What the developer controls

At publish time, additional redaction options are available:

- Anonymize file paths
- Anonymize commit author
- Strip specific sections of the summary
- Strip specific tool calls from the session

### What never crosses the local boundary

Even with all defaults, the following never reach the team layer:

- Raw transcripts (unless dev explicitly checks "publish raw transcript")
- Failed/aborted sessions
- Sessions in repos without an `illuminate.toml`
- Sessions explicitly discarded by the dev

### Integration with cloakpipe

For teams using cloakpipe (the separate Anthropic-tools-proxy), redaction can be performed in the proxy layer, ensuring even the local raw trail never sees the unredacted content. This is the highest-assurance posture.

---

## Storage Layout Summary

```
~/.illuminate/                          # local user state
├── archive/                            # compressed old raw trails (optional)
│   └── 2026-Q1/
└── config.toml                         # global config

<repo>/                                 # any repo with illuminate
├── illuminate.toml                     # config + ingest sources
├── .illuminate/                        # local-only state
│   ├── trail/                          # raw trail (gitignored)
│   │   └── <session-id>.jsonl
│   ├── sessions/                       # local session summaries
│   │   └── <session-id>.md
│   ├── graph.db                        # SQLite query cache (gitignored)
│   └── pins                            # pinned sessions list
└── .gitignore                          # adds .illuminate/{trail,graph.db}

team-illuminate/                        # shared team git repo
├── illuminate.toml                     # team-level config
├── schema.md                           # how the agent maintains the repo
├── sessions/                           # published session summaries
│   └── <session-id>.md
├── decisions/                          # extracted decisions as markdown
├── patterns/                           # extracted patterns
├── failures/                           # extracted failures
├── modules/                            # auto-generated module pages
├── docs/                               # author-written docs (see knowledge-layer.md)
└── graph-export/                       # structured entities (JSON)
    ├── decisions/
    │   └── decision-<id>.json
    ├── patterns/
    │   └── pattern-<id>.json
    └── failures/
        └── failure-<id>.json
```

---

## Lifecycle Operations

### At session start

- Daemon detects new session from agent
- Verifies repo has `illuminate.toml` (opt-in only)
- Starts capture
- Begins streaming filter pass (secrets, PII) — writes only filtered content to disk

### During session

- Raw trail written incrementally to `.illuminate/trail/<session-id>.jsonl`
- No graph or summary work yet — keep capture cheap

### At session end (detected by agent timeout, explicit close, or commit)

- Final filter pass on the complete raw trail
- Extraction pipeline runs:
  - NER on the session
  - Identifies decisions, patterns, failures, entity references
  - Generates session summary (local LLM or cheap cloud)
  - Writes local session summary to `.illuminate/sessions/`
  - Updates local graph in `.illuminate/graph.db`
- Developer is notified that the session is captured

### At git commit time

- Pre-commit hook offers the publish prompt
- Dev chooses what to publish + redaction options
- Published content is written to `team-illuminate/`
- Local raw trail is unchanged (still local-only)

### At rotation time (daily cron or daemon timer)

- Trails older than retention window are archived or deleted
- Graph cache is compacted
- Index statistics are refreshed

### On explicit operations

- `ilm pin <session-id>` — exempt from rotation
- `ilm forget <session-id>` — delete from local raw layer
- `ilm publish <session-id>` — publish a previously-skipped session
- `ilm unpublish <session-id>` — remove from team repo (audit-logged)
- `ilm rebuild` — regenerate local SQLite graph from team repo JSON exports

---

## What This Means for Trust

This data model exists because trust requires structure, not just promises.

- **Raw fidelity** without **mandatory sharing** — devs get a personal scratch space that never leaks.
- **Durable summaries** with **explicit publication** — the team benefits from what devs choose to share.
- **Graph structure** without **PII** — the queryable layer is safe to share because it's been through extraction.
- **Three layers** with **three lifecycles** — short-lived raw, durable summaries, permanent graph. Each layer holds the right amount.

The architecture *enforces* the trust model. There is no "we won't look at it" promise that depends on Illuminate the company behaving well. The raw layer is local; the team layer requires a gesture; the graph is structured. Each constraint is physical, not policy.

---

## Mapping to the Shipped State (v0.21)

Where this data-model spec is already enforced in code today, vs. where it remains v3.x scope:

| Layer / Behaviour | Shipped today | Pending |
|---|---|---|
| Raw Trail capture (`.illuminate/trail/<session>.jsonl`) | ✅ v0.1+ — Claude Code / Cursor / Codex via `illuminate-trail` | Retention rotation (currently manual); `ilm pin` |
| Tool-result git-state references (`git:<commit>:<path>`) | Partial — trail records `files_touched` paths | Full content-ref dereferencing on read |
| Streaming secret / PII filter pass | Partial — `cloakpipe` integration exists for LLM-extraction fallback | Pre-disk redaction pass on the trail itself |
| Session Summary (`.illuminate/sessions/<id>.md`) | Partial — emitted on publish only, not at every session-end | Auto-summary at session end with NER + local-LLM template |
| Publish gesture (`illuminate publish`) | ✅ v0.21 — full / summary / decision / discard redactions, pre-commit hook installer | Interactive TUI prompt, raw-transcript checkbox |
| Graph Entities (Decision / Pattern / Failure / Module) | ✅ v0.6+ — extracted via `illuminate-extract` + NER + LLM fallback | `Doc` entity (v3.2 — see [`knowledge-layer.md`](knowledge-layer.md)) |
| `team-illuminate/graph-export/` JSON mirror | 🔜 Not yet — graph currently lives only as SQLite + the markdown source-of-truth | v3.2 — emit JSON mirror alongside markdown so the team repo is portable |
| `ilm rebuild` (regenerate SQLite from team repo) | ✅ v0.7+ — `illuminate rebuild` works against the wiki markdown today; v3.2 will extend it to consume the JSON mirror too | |
| `ilm forget` / `ilm unpublish` | 🔜 Not yet — `illuminate wiki redact` covers the file-side; graph-side deletion is the v0.19+ punch list | |
| Trust model invariants (no auto-upload, opt-in per repo, etc.) | ✅ Shipped — see [`trust-model.md`](trust-model.md) | |

The "Shipped today" column is what a v0.21 install delivers. The "Pending" column is the punch list for v3.2+ — see [`ROADMAP.md`](ROADMAP.md#v32--docs-as-first-class-content) for the per-version plan.

---

## What This Doc Doesn't Cover

For specific topics, see the companion docs:

- **Architecture and crate layout:** [`ARCHITECTURE.md`](ARCHITECTURE.md)
- **Product positioning and value:** [`PRODUCT_OVERVIEW.md`](PRODUCT_OVERVIEW.md)
- **Docs as first-class content:** [`knowledge-layer.md`](knowledge-layer.md)
- **Trust model in full:** [`trust-model.md`](trust-model.md)
- **Wiki page schema (`page_type` front-matter):** [`SCHEMA.md`](SCHEMA.md)
- **MCP tools and protocol:** [`MCP.md`](MCP.md)
- **Bootstrap (cold-start ingestion):** [`BOOTSTRAP.md`](BOOTSTRAP.md)
- **Privacy and threat model:** [`PRIVACY.md`](PRIVACY.md)
