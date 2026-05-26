# Illuminate — Wiki Schema

This document defines the markdown schema for the team-shared wiki. The wiki is the human-readable source-of-truth; the graph is its index. If `graph.db` is deleted, the wiki regenerates it (`illuminate rebuild`).

For the graph schema (entities, relationships, episodes), see `illuminate-core` and the entity definitions in `illuminate.toml`. This file covers the markdown layer only.

---

## Why markdown is source-of-truth

- Git already does versioning, diffing, blame, and review. Don't reinvent.
- Markdown renders in GitHub, GitLab, Obsidian, any editor — no proprietary viewer.
- Text editors and LLM agents both read markdown natively.
- A corrupted graph is a cache miss, not a data loss.

The graph holds the queryable index. The wiki holds the truth.

---

## Directory layout

```
.illuminate/wiki/
├── index.md            # auto-generated catalog (top-level entry point)
├── log.md              # append-only audit log of wiki changes
├── schema.md           # copy of this document; how the agent maintains the wiki
├── decisions/
│   └── YYYY-MM-<slug>.md
├── patterns/
│   └── <slug>.md
├── failures/
│   └── YYYY-MM-<slug>.md
└── modules/
    └── <slug>.md
```

Four content categories:

| Folder | Holds | Lifecycle |
|--------|-------|-----------|
| `decisions/` | Architectural choices made by the team | Append-only; supersession via new doc |
| `patterns/` | Reusable approaches the team has converged on | Append-only; supersession via new doc |
| `failures/` | Things that broke and why | Append-only; never edited |
| `modules/` | Per-module overviews (1 page per top-level module) | Mutable; latest state |

`decisions/`, `patterns/`, and `failures/` are append-only. `modules/` are living documents that summarize the *current* state of a module.

---

## Common front-matter

Every wiki page begins with YAML front-matter. Required fields are bold:

```yaml
---
id: dec-2025-12-no-redis-payments       # **stable id, generated**
title: No Redis caching in payments     # **human-readable title**
type: decision                          # **decision | pattern | failure | module**
status: active                          # active | superseded | retired
created: 2025-12-14T11:42:00Z           # **ISO-8601 UTC**
updated: 2025-12-14T11:42:00Z
authors:
  - name: priya
    source: github
sources:
  - kind: pr
    ref: github.com/acme/payments/pull/847
  - kind: trail
    ref: .illuminate/trail/2025-12-14-redis-eval-claude.jsonl
tags:
  - caching
  - infrastructure
  - payments
modules:
  - payments-service
related:
  - dec-2025-11-eks-no-statefulsets
supersedes: []                          # ids this doc supersedes
superseded_by: []                       # ids that supersede this doc (when retired)
confidence: 0.92                        # 0.0–1.0; how sure we are this is canonical
---
```

Field meaning:

- `id` — stable identifier. Format: `<type-prefix>-<YYYY-MM>-<slug>`. The graph uses this as the primary key. Never change after creation.
- `status`:
  - `active` — current canon
  - `superseded` — replaced by a newer decision (link via `superseded_by`)
  - `retired` — no longer relevant, kept for history
- `sources` — where the decision came from. Keep raw refs so audits can re-verify.
- `confidence` — extractor confidence (1.0 = manually written, ~0.7 = high-confidence NER, ~0.5 = LLM-classified).

---

## Decision page (`decisions/`)

Format:

```markdown
---
id: dec-2025-12-no-redis-payments
title: No Redis caching in payments service
type: decision
status: active
created: 2025-12-14T11:42:00Z
updated: 2025-12-14T11:42:00Z
authors: [{ name: priya, source: github }]
sources:
  - { kind: pr, ref: github.com/acme/payments/pull/847 }
tags: [caching, infrastructure, payments]
modules: [payments-service]
related: [dec-2025-11-eks-no-statefulsets]
confidence: 0.92
---

## Decision

Do not introduce Redis or any stateful sidecar to the payments service.
Use an in-memory LRU cache with 30s TTL where caching is needed.

## Context

The payments service runs on a deployment target that does not allow
stateful sidecars. (See `dec-2025-11-eks-no-statefulsets`.) Redis would
require either a managed external instance — increasing latency budgets
beyond our SLO — or a sidecar pattern that is explicitly disallowed.

## Alternatives considered

- **External Redis (managed):** rejected; +12ms p50 latency
- **Memcached sidecar:** same restriction as Redis
- **In-memory LRU (chosen):** zero deployment overhead, sufficient for
  the access pattern of `txnLookup`

## Consequences

- ✅ No deploy-time complexity
- ✅ Cache size bounded by process memory (acceptable for this service)
- ❌ No cross-pod cache sharing — each pod warms independently
- ❌ Cache lost on restart; first-request latency spike

## See also

- `patterns/lru-cache-with-ttl.md` — implementation pattern
- `failures/2026-02-race-condition-payments.md` — unrelated, but in same module
```

Mandatory sections: `## Decision`, `## Context`, `## Consequences`. The agent rejects pages missing these.

`## Alternatives considered` is recommended; the linter cites it back to agents who propose rejected alternatives.

---

## Pattern page (`patterns/`)

Format:

```markdown
---
id: pat-lru-cache-with-ttl
title: LRU cache with TTL
type: pattern
status: active
created: 2025-12-14T11:50:00Z
updated: 2025-12-14T11:50:00Z
tags: [caching, in-memory]
modules: [payments-service, billing-service]
related: [dec-2025-12-no-redis-payments]
confidence: 1.0
---

## Pattern

For request-path caching where:
- cross-pod sharing is not required, and
- cache size fits in process memory, and
- staleness up to 30s is acceptable,

use an in-memory LRU cache with TTL.

## Implementation

```rust
use lru::LruCache;
use std::time::{Duration, Instant};

pub struct TtlLru<K, V> {
    inner: LruCache<K, (V, Instant)>,
    ttl: Duration,
}

// ... implementation
```

## When to use

- High-frequency lookup where backend is slow but tolerant to stale reads
- Per-pod caches where cross-pod consistency isn't required

## When NOT to use

- When freshness < 1s is required → use direct backend
- When cache must survive restart → use external store (subject to deployment constraints)

## Code anchors

- `payments-service/src/cache.rs` — canonical implementation
- `billing-service/src/checkout/cache.rs` — secondary use
```

Patterns are linked from decisions. Multiple decisions can cite the same pattern.

---

## Failure page (`failures/`)

Format:

```markdown
---
id: fail-2026-02-race-condition-payments
title: Race condition in payments queue draining
type: failure
status: active
created: 2026-02-21T18:14:00Z
updated: 2026-02-21T18:14:00Z
authors: [{ name: alex, source: github }]
sources:
  - { kind: incident, ref: pagerduty/incidents/inc-44291 }
  - { kind: pr, ref: github.com/acme/payments/pull/903 }
tags: [race-condition, payments, queue]
modules: [payments-service]
severity: high
confidence: 1.0
---

## What broke

Concurrent calls to `drainQueue()` from two goroutines could double-process
a transaction when the queue cursor was checked but not yet advanced.

## Root cause

The cursor read and advance were not atomic. Under load, scheduler interleaving
exposed the gap.

## Fix

Wrapped cursor read+advance in a single `compare_and_swap` operation.
See `payments-service/src/queue.rs` line 142.

## Lesson for future agents

When proposing changes to `payments-service/src/queue.rs`:
- Cursor advance MUST be atomic with cursor read.
- Tests must include concurrent invocation (`tokio::join!` or `rayon`).
- The audit response surfaces this failure when files in `src/queue.rs` are touched.

## Affected files

- `payments-service/src/queue.rs`
- `payments-service/tests/queue_concurrent_test.rs`
```

Mandatory sections: `## What broke`, `## Root cause`, `## Fix`, `## Lesson for future agents`.

The `## Lesson for future agents` section is what the audit engine surfaces. Write it as instructions to a future agent who is about to touch the affected code.

Failures are **never edited**. If new information surfaces, write a new failure page that references the old one.

---

## Module page (`modules/`)

Unlike decisions/patterns/failures, module pages are **mutable** — they summarize current state.

```markdown
---
id: mod-payments-service
title: Payments service
type: module
status: active
created: 2025-09-03T10:00:00Z
updated: 2026-05-01T14:22:00Z
tags: [service, payments]
paths: [services/payments-service/**]
confidence: 1.0
---

## Purpose

Handles transaction lookup, balance updates, and refund processing for the
acme platform.

## Active decisions

- `dec-2025-12-no-redis-payments` — no stateful sidecars
- `dec-2025-11-eks-no-statefulsets` — deployment constraint
- `dec-2026-01-tree-sitter-over-treesitter-rs` — parser choice (cross-cutting)

## Active patterns

- `pat-lru-cache-with-ttl` — caching strategy

## Known failures

- `fail-2026-02-race-condition-payments` — atomic cursor advance required

## Recent activity

(Auto-generated section, last 10 commits affecting this module.)

- 2026-05-01 — alex — fix typo in docstring (commit abc1234)
- 2026-04-29 — priya — add idempotency key to refund flow (commit def5678)
```

Module pages are kept up to date by the maintenance pass (`illuminate wiki distill --module <slug>`). The "active decisions/patterns/failures" sections are computed from the graph; the agent shouldn't hand-edit them. The `## Purpose` section is human-written.

---

## Session page (`sessions/`) — published, lives in the team repo

> **New in v0.21.** Sessions are *published* trail captures: a dev explicitly
> chose to share part of a Claude Code / Cursor / Codex session with the team
> via `illuminate publish` (Stage 4 of the v3 pipeline). Unlike decisions /
> patterns / failures — which live under `.illuminate/wiki/` inside the source
> repo — sessions live in a **separate `team-illuminate/` repo** configured per
> install. The schema below is what `illuminate-publish` writes; readers see
> the same shape regardless of which agent produced the original session.

```markdown
---
id: ses-2026-05-25-add-redis-caching-to-the-txn-endpoint
page_type: session
session_id: a8387a3e-21ad-4053-877f-0660f2fefb07
agent: claude-code
model: claude-opus-4-7
redaction: summary
commit_sha: "443b517"
files_touched: ["src/payments/txn.rs"]
created: 2026-05-25T10:00:00+00:00
source: published:claude-code
---

## Prompt

add caching to the txn endpoint

## Final response

Use LRU with 30s TTL — Redis is rejected per dec-no-redis.

## Files touched

- `src/payments/txn.rs`
```

### Front-matter fields

| Field | Type | Notes |
|-------|------|-------|
| `id` | string | `ses-<filename-stem>`, derived from the filename. Filename is `<YYYY-MM-DD>-<slug>.md` where the slug is generated from the first user prompt. |
| `page_type` | `session` | Distinguishes from `decision` / `pattern` / `failure` / `module`. |
| `session_id` | string | The capture-time session id from the underlying agent (Claude Code, Cursor, Codex). |
| `agent` | string | `claude-code` / `cursor` / `codex`. |
| `model` | string | Model identifier reported by the agent at capture time. Empty when the agent did not surface one. |
| `redaction` | string | One of `full` / `summary` / `decision` / `discard`. Records the dev's choice at publish time. (`discard` pages do not exist on disk — included here only for completeness.) |
| `commit_sha` | string | Git commit this session produced, if any. `""` when not supplied. |
| `files_touched` | string[] | Repo-relative paths the session edited (mirrored from the trail record). |
| `created` | RFC 3339 | Session start time. |
| `source` | string | Always `published:<agent>` (e.g. `published:claude-code`). Used by the audit + enrich query path to distinguish published sessions from raw trail episodes. |

### Body shape by redaction level

| Level | Body |
|-------|------|
| `full` | `## Full transcript` heading followed by every message in order (`### N. User` / `### N. Assistant` / etc.), then `## Files touched` if any. |
| `summary` | `## Prompt` (the first user prompt), `## Final response` (the last assistant response), `## Files touched` if any. The default for the pre-commit hook flow. |
| `decision` | A single line: `(decision-only publish — see front-matter for context)`. The front-matter is the payload; the body is empty by design. |
| `discard` | No file is written. |

### Trust-model invariants enforced

- Movement from trail (Ring 1, local-only) → session page (Ring 2, team-shared) happens **only** through `illuminate publish` with an explicit `--redaction` flag.
- The crate writes only to the path the caller named in `--team-repo`. No defaults that auto-write outside `.illuminate/`.
- v3.0 supports `TeamRepoTarget::LocalPath` only. The planned `GitRemote` variant is gated for v3.1 behind `illuminate trust check`.
- See `trust-model.md` for the full three-rings model.

---

## `docs/` — author-written, first-class content (v3.2+)

> **Planned for v3.2.** Once `illuminate-ingest` lands and the `Doc` entity type is in `illuminate-core`, the team repo gains a `docs/` subtree alongside `decisions/` / `patterns/` / `failures/` / `modules/` / `sessions/`. Humans write the content in their editor of choice; `illuminate` indexes it. See [`knowledge-layer.md`](knowledge-layer.md) for the full design and feature landscape.

The full team-repo layout once docs are first-class:

```
team-illuminate/
├── illuminate.toml             # config + ingest sources
├── schema.md                   # how the agent maintains the repo
├── index.md                    # auto-generated catalog
├── log.md                      # append-only audit trail
│
├── sessions/                   # auto-captured (published with consent — v0.21)
├── decisions/                  # extracted + author-written
├── patterns/                   # extracted + author-written
├── failures/                   # extracted + author-written
├── modules/                    # auto-generated module pages
│
└── docs/                       # author-written, first-class content (v3.2+)
    ├── architecture/           # overview, auth-flow, payment-flow, etc.
    ├── adr/                    # ADR-style numbered records (0001-... / 0002-...)
    ├── designs/                # design docs (often auto-drafted via `--as-doc`)
    ├── runbooks/               # operational procedures, rollback steps
    ├── onboarding/             # curated reading order for new hires
    ├── prompts/                # THE prompt cookbook — first-class content type
    ├── integrations/           # external-service integration references (stripe, postgres, …)
    ├── conventions/            # team conventions (naming, error-handling, …)
    └── oncall/                 # on-call playbooks per service
```

### Doc front-matter (minimum)

Doc files don't need to follow a single rigid schema — humans wrote them, in their editor of choice, often before Illuminate existed. The minimum the indexer expects:

```markdown
---
id: arch-payment-flow                # filename stem, prefix-stable
page_type: doc                       # always `doc`
doc_kind: architecture               # adr | architecture | runbook | design | onboarding | prompt | integration | convention | oncall | generic
title: Payment flow                  # human-readable title
status: active                       # active | superseded | archived
tags: [payments, architecture]       # optional, freeform
created: 2025-09-03T10:00:00Z
updated: 2026-05-25T14:22:00Z
source: author                       # `author` (human-written) or `ingested:<adapter>` for read-only mirrors of external sources
---

# (markdown body — humans write this)
```

For ingested content (confluence pages, notion blocks, etc.), `illuminate-ingest` populates `source: ingested:<adapter>`, adds an `external_id`, and an `external_url` field linking back to the canonical source. **Ingested docs are read-only mirrors** — never written back to the external source.

### Per-subdir conventions

| Subdir | Naming convention | Typical fields |
|--------|-------------------|----------------|
| `docs/architecture/` | `<topic>.md` (overview, auth-flow, payment-flow, ...) | mermaid diagrams welcome; living-diagram auto-update is v3.4 |
| `docs/adr/` | `NNNN-<slug>.md` (Nygard / MADR style — see [`BOOTSTRAP.md`](BOOTSTRAP.md)) | `decision_status: proposed \| accepted \| deprecated` |
| `docs/designs/` | `<slug>.md` (often the same slug as the originating session, e.g. `cache-layer.md`) | `source: published-as-doc:<session-id>` when auto-drafted via `illuminate publish --as-doc` |
| `docs/runbooks/` | `<service>-<scenario>.md` | `service`, `severity` |
| `docs/onboarding/` | `NN-<topic>.md` (numbered for reading order) + `README.md` index | `prereqs: [other-onboarding-doc-ids]` |
| `docs/prompts/` | `<task-shape>.md` (`adding-api-endpoint`, `database-migration`, `security-review`, …) | `agents: [claude-code, cursor]`, `auto_suggest_triggers: [...]` — used by `illuminate enrich` cookbook injection in v3.3 |
| `docs/integrations/` | `<vendor>.md` (stripe, postgres, sendgrid, …) | `vendor`, `version` |
| `docs/conventions/` | `<area>.md` (naming, error-handling, logging, …) | none required |
| `docs/oncall/` | `<service>.md` | `service`, `pager_link` |

### Trust-model invariants for docs

- All write operations to `docs/*.md` use the same path the dev named — no implicit network. Same boundary as `illuminate-publish`.
- External-source ingestion (`docs/` mirrors of confluence pages, etc.) is **always read-only**: `illuminate-ingest` pulls, never pushes back.
- Authentication tokens for external adapters (`CONFLUENCE_TOKEN`, `NOTION_TOKEN`, …) read from env vars only. Never stored on disk.
- Each external adapter in `illuminate.toml` requires an explicit `enabled = true` field; zero defaults that auto-fetch.

Full design + the new features this unlocks (`illuminate ask`, doc decay detection, auto-draft from sessions, prompt cookbook auto-suggest, onboarding journeys, on-call bundles, agent skill packs) is in [`knowledge-layer.md`](knowledge-layer.md).

---

## `index.md` (auto-generated)

The top-level index. Generated by `illuminate wiki rebuild`. Contains:

- Counts: N decisions (M active, K superseded), N patterns, N failures, N modules
- Most recent decisions (10)
- Most recently superseded decisions (5)
- Modules with most active warnings
- Tag cloud / facet links

Never hand-edited.

---

## `log.md` (append-only)

The audit log. Every wiki write goes here. Append-only, one line per change:

```
2026-05-06T12:14:33Z  ADD     dec-2026-05-06-rate-limit-on-public-api  (extractor, conf=0.81)
2026-05-06T12:14:33Z  UPDATE  mod-payments-service                     (auto, related-link added)
2026-05-06T13:02:11Z  SUPERSEDE dec-2024-08-redis-payments BY dec-2025-12-no-redis-payments  (manual)
```

Format: `<timestamp>  <verb>  <id>  (<actor>, [<note>])`. Verbs: `ADD`, `UPDATE`, `SUPERSEDE`, `RETIRE`. Actor is `manual`, `extractor`, `bootstrap`, or `agent:<name>`.

The log lets the team review automated changes without scanning git diffs of every individual page.

---

## Supersession rules

Decisions and patterns can be superseded but not deleted:

```
                ┌─────────────────────────────────────────┐
                │  dec-2024-08-redis-payments             │
                │  status: superseded                     │
                │  superseded_by: [dec-2025-12-no-redis-  │
                │                  payments]              │
                └────────────────┬────────────────────────┘
                                 │
                                 ▼
                ┌─────────────────────────────────────────┐
                │  dec-2025-12-no-redis-payments          │
                │  status: active                         │
                │  supersedes: [dec-2024-08-redis-        │
                │               payments]                 │
                └─────────────────────────────────────────┘
```

The audit engine queries only `status: active` decisions by default. The `--include-history` flag surfaces superseded ones for context. Superseded pages remain in the wiki and the graph.

When the extractor detects a contradiction with an existing active decision, it doesn't auto-supersede — it opens a wiki PR (or emits a warning) for human review. Auto-supersession would be a footgun.

---

## How the agent maintains the wiki

When `illuminate wiki distill` runs, the agent:

1. **Scans recent trail files** (last N days, configurable).
2. **Extracts candidate decisions** via NER + LLM fallback.
3. **For each candidate:**
   - Look up by semantic similarity in the graph.
   - If similar to an existing decision, attach as additional `sources` (don't create a duplicate).
   - If new, create a draft wiki page in `decisions/` with `status: active`, `confidence: <extractor confidence>`.
4. **For pages with `confidence < 0.7`:** open as a wiki PR for human review (don't merge automatically).
5. **For pages with `confidence >= 0.7`:** merge directly, log to `log.md`.

The threshold is configurable per-team in `illuminate.toml`:

```toml
[wiki]
auto_merge_threshold = 0.7   # auto-merge if confidence >= this
require_review_below = 0.5   # never auto-merge below this; always PR
```

Manually written pages (no NER involved) get `confidence: 1.0`.

---

## Validating wiki pages

`illuminate wiki lint` runs on every `illuminate wiki rebuild` and on every PR. It checks:

- All required front-matter fields present
- `id` matches filename pattern
- `status` is one of the allowed values
- `created` ≤ `updated`
- `supersedes` and `superseded_by` reference real ids
- Decision pages have `## Decision`, `## Context`, `## Consequences`
- Failure pages have `## What broke`, `## Root cause`, `## Fix`, `## Lesson for future agents`
- No two pages share the same `id`
- All `related` ids exist
- All `modules` references exist (or are explicitly creatable)

The CI gate fails on any error. The agent uses the same lint pass before submitting auto-generated pages.

---

## Migration / rebuild

If `graph.db` is deleted or corrupt, run:

```bash
illuminate rebuild
```

This:

1. Walks `wiki/` and parses every page.
2. Walks `trail/` and re-runs extraction.
3. Walks `git log` and re-runs decision extraction.
4. Rebuilds `graph.db` from scratch.

Idempotent. Safe to run any time. Source-of-truth is preserved because everything that mattered lived in `wiki/` and `trail/`.
