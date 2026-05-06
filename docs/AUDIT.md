# Illuminate — Audit (the Linter)

This document specifies how the audit engine works: the inputs it accepts, the queries it runs, the response shape it returns, and the integration points that get agents to actually call it.

For ingestion (how the graph gets populated), see `INGESTION.md`. For wiki schema, see `SCHEMA.md`.

---

## Goals

- **Course-correct agents before code is written.** The audit fires on plan/intent, not on diff. Drift prevented, not detected after the fact.
- **Deterministic.** Same input → same output. No LLM in the path. Same hash of `{plan, files, repo_state}` always returns the same audit.
- **Fast.** < 200ms target round-trip for typical queries on a local repo with ≤ 10k entities.
- **Useful when the graph is empty.** Absence of evidence is not absence of warning. Bootstrap from `illuminate.toml` + `CLAUDE.md` so day-one is non-trivial.

---

## The audit contract

The audit engine takes a proposed change and returns structured findings.

### Request

```json
{
  "method": "illuminate_audit",
  "params": {
    "plan": "Add Redis caching layer to transaction lookup",
    "files": [
      "services/payments-service/src/cache.rs",
      "services/payments-service/src/txn.rs"
    ],
    "rationale": "p99 latency on txn lookup exceeded SLO",
    "agent": "claude-code",
    "session_id": "abc123"
  }
}
```

Field semantics:

- `plan` (required) — natural-language description of what the agent intends to do.
- `files` (required) — list of paths the agent expects to modify or create. Glob-allowed.
- `rationale` (optional) — why the change is being made. Used to weight the response.
- `agent`, `session_id` (optional) — for telemetry; never required.

### Response

```json
{
  "status": "warn",
  "violations": [],
  "warnings": [
    {
      "id": "dec-2025-12-no-redis-payments",
      "kind": "decision",
      "title": "No Redis caching in payments service",
      "summary": "Team rejected Redis 3 months ago. Use in-memory LRU instead.",
      "wiki_url": "wiki/decisions/2025-12-no-redis-payments.md",
      "confidence": 0.92,
      "evidence": ["github.com/acme/payments/pull/847"]
    }
  ],
  "relevant_patterns": [
    {
      "id": "pat-lru-cache-with-ttl",
      "title": "LRU cache with TTL",
      "wiki_url": "wiki/patterns/lru-cache-with-ttl.md",
      "implementation_hint": "see services/payments-service/src/cache.rs"
    }
  ],
  "prior_failures": [],
  "policies_applied": ["no_redis_payments"],
  "trace_id": "audit-..."
}
```

Top-level `status`:

- `pass` — no findings. Agent may proceed.
- `warn` — past decisions or patterns are relevant. Agent should surface them and continue.
- `block` — a hard policy violation. Agent should not proceed without explicit override.

The agent is expected to treat `block` as authoritative and `warn` as advisory. The `policies_applied` field tells callers which `illuminate.toml` rules fired.

### Severity levels

| Severity | Where it comes from | Status mapping |
|----------|---------------------|----------------|
| `error` | `illuminate.toml` policy with `severity = "error"` | `block` |
| `warning` | `illuminate.toml` policy with `severity = "warning"`, OR an active superseding decision | `warn` |
| `info` | Related decisions/patterns/failures | `warn` (folded into warnings/relevant) |

A single audit response can contain multiple severity levels; the top-level `status` is the highest among them.

---

## Engine internals

```
audit request
    │
    ▼
┌────────────────────────────────────┐
│  1. parse + validate               │
│     load illuminate.toml           │
│     resolve files → modules        │
│     (via illuminate-index)         │
└──────────────┬─────────────────────┘
               │
               ▼
┌────────────────────────────────────┐
│  2. file-anchored queries          │
│   for each file:                   │
│     entities touching this file?   │
│     decisions/patterns/failures    │
│     anchored to this module?       │
└──────────────┬─────────────────────┘
               │
               ▼
┌────────────────────────────────────┐
│  3. semantic queries on plan text  │
│   embed(plan) →                    │
│     top-k similar decisions,       │
│     top-k similar patterns,        │
│     top-k similar failures         │
│   (k tunable, default 5)           │
└──────────────┬─────────────────────┘
               │
               ▼
┌────────────────────────────────────┐
│  4. policy evaluation              │
│   for each policy in toml:         │
│     does it apply?                 │
│       (path glob match,            │
│        rejected entity match,      │
│        pattern match)              │
│     produce violation/warning      │
└──────────────┬─────────────────────┘
               │
               ▼
┌────────────────────────────────────┐
│  5. dedup + rank                   │
│   collapse multiple findings on    │
│   the same entity                  │
│   rank by confidence + recency     │
└──────────────┬─────────────────────┘
               │
               ▼
┌────────────────────────────────────┐
│  6. respond                        │
│   serialize findings, set status   │
│   write trace to log               │
└────────────────────────────────────┘
```

### File-to-module resolution

`illuminate-index` maintains a map from path glob → module id. The mapping is rebuilt on `illuminate index` and on each commit (incremental). When a file doesn't map to any module, the audit still runs on semantic similarity alone — the result will be lower-precision but never absent.

### Semantic similarity

Default top-k = 5, threshold = cosine similarity ≥ 0.55. Tunable per-team:

```toml
[audit]
semantic_top_k = 5
semantic_threshold = 0.55
```

Higher threshold → fewer false positives, more missed warnings. Lower threshold → more noise, less missed.

### Policy types

Policies in `illuminate.toml`:

```toml
[policies.no_redis_payments]
rule = "must_use"               # required tech for a path/scope
entity = "Memcached"
reject = ["Redis", "Dragonfly"]
paths = ["services/payments-service/**"]
reason = "deployment target disallows stateful sidecars"
severity = "error"

[policies.auth_module]
rule = "frozen"                 # no edits allowed
paths = ["src/auth/**"]
reason = "security audit in progress"
severity = "error"
expires = "2026-04-15"

[policies.no_microservices]
rule = "rejected_pattern"       # NL pattern matched against plan text
pattern = "microservice split"
reason = "tried in 2024, reverted due to latency"
severity = "error"

[policies.use_tree_sitter]
rule = "must_use"
entity = "tree-sitter"
reject = ["regex-based parser"]
severity = "warning"
```

Policy rule kinds:

- `must_use` — for a given `paths` / scope, a particular entity is required; alternatives are rejected.
- `frozen` — files matching `paths` cannot be modified at all (e.g., during audit).
- `rejected_pattern` — natural-language pattern matched semantically against `plan` text. Useful for "we rejected microservices" type rules.
- `requires_review` — non-blocking; surfaces a warning that flags the change for human review.

Policies expire automatically after `expires`. After expiration, the rule is logged but not enforced.

---

## How agents are induced to call it

The audit is only as useful as the rate at which agents call it. Three integration mechanisms, used in concert:

### 1. MCP tool (every agent)

`illuminate-mcp` exposes `illuminate_audit` as a JSON-RPC tool. Claude Code, Cursor, and Codex (with their MCP integrations enabled) can all call it. Adoption depends on the agent choosing to call the tool, which is unreliable on its own.

### 2. `CLAUDE.md` directive

Append to the repo's `CLAUDE.md`:

```markdown
## Required pre-write step

Before writing or modifying any source file in this repository, invoke
`illuminate_audit` with your plan, the files you intend to modify, and
your rationale. Surface the response to the user before proceeding.

If `status` is `block`, do not proceed without explicit user approval.
If `status` is `warn`, surface the warnings to the user before writing.
```

This works for Claude Code and any agent that reads `CLAUDE.md` / `AGENTS.md`. `illuminate init --claude` writes this directive automatically.

### 3. Pre-write hook (Claude Code only, deterministic)

For Claude Code specifically, `illuminate init --claude --hook` installs a `PreToolUse` hook that intercepts `Write`/`Edit` tool calls. The hook runs `illuminate audit` synchronously and:

- If `status = "block"`, the hook denies the tool call and surfaces the warning.
- If `status = "warn"`, the hook lets the call through but injects the warning into the agent's context.

This is deterministic — agents cannot skip it. Cost: a small latency hit on every Write/Edit.

### 4. PR-time CI gate (any agent, post-hoc)

A GitHub Action runs `illuminate audit-pr <pr-number>` against every PR. The action:

1. Computes the diff.
2. Runs the audit against each chunk of changed files.
3. Comments findings on the PR.
4. Fails the check if any finding is `severity = "error"`.

This catches what the in-session hook missed. It's also useful for repos where the agent isn't directly integrated.

---

## CLI usage

```bash
# audit a free-form plan
illuminate audit "Add Redis caching to txn lookup" \
    --files services/payments-service/src/cache.rs

# audit a PR
illuminate audit-pr 847 --repo acme/payments

# audit the working tree (uncommitted changes)
illuminate audit-diff

# explain why a file matters (no plan)
illuminate explain services/payments-service/src/cache.rs

# show all decisions affecting a path
illuminate decisions for services/payments-service/**
```

`illuminate audit` is the human entry point; `illuminate_audit` (MCP) is the agent entry point. Both call the same engine.

---

## Performance budget

Target round-trip for a typical query:

| Stage | Budget |
|-------|--------|
| parse + validate | < 5ms |
| file → module resolution | < 10ms |
| graph queries (file-anchored) | < 50ms |
| embedding + semantic search | < 80ms |
| policy evaluation | < 5ms |
| serialization | < 5ms |
| **total** | **< 200ms** |

For `policies` evaluation alone (no graph queries), target < 20ms. This is the floor — `illuminate audit` always returns at least the policy result, even on a cold/empty graph.

---

## When the graph is empty

A freshly installed repo has nothing in the graph yet. The audit engine still runs:

- Policies from `illuminate.toml` apply immediately (these are static rules).
- Bootstrap-ingested decisions (from `CLAUDE.md`, ADRs, git log) are populated by `illuminate init` and are queryable from minute one.
- Semantic queries return empty top-k; the audit response shows no warnings but doesn't fail.

This matters because the demo on day one cannot be "the audit returns nothing." The bootstrap pipeline ensures it returns *something* useful from the first run.

---

## Confidence and explainability

Every finding carries:

- `confidence` — 0.0–1.0, where 1.0 is human-written.
- `evidence` — list of source refs (PR url, commit sha, trail file path).
- `wiki_url` — link to the human-readable page that explains it.

The agent's job is to surface this verbatim to the dev. Don't paraphrase the warning — the wiki page is the canonical explanation. The audit response is just a pointer.

---

## What the audit does NOT do

- It does not generate code.
- It does not call the LLM provider.
- It does not modify the graph (read-only path).
- It does not block PRs by itself — that's the CI gate's job.
- It does not enforce style/lint rules — that's the existing toolchain (ESLint, Clippy, etc.). Illuminate handles intent; existing tools handle syntax.

---

## Trace + observability

Every audit call writes a row to `.illuminate/audits.log` (gitignored):

```
2026-05-06T12:14:33Z  trace_id=...  status=warn  files=2  warnings=1  policies=[no_redis_payments]  duration_ms=87
```

`illuminate stats audit` summarizes recent activity (calls per day, average duration, hit rate). For teams that want to evaluate whether the audit is actually changing agent behavior, this is the input data.

For privacy: the log captures `trace_id`, status, counts, and matched policy names. It does **not** capture the `plan` text, `rationale`, or file paths beyond aggregated counts. Devs who want richer traces can opt in via `[audit] verbose_logging = true`.
