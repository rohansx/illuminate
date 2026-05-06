# Illuminate — Ingestion

This document specifies the three input mechanisms that feed the knowledge graph: prompt-trail capture, decision extraction (git + manual), and failure capture (reflect).

For consumers of the graph, see `AUDIT.md` (linter) and `SCHEMA.md` (wiki).

---

## Pipeline overview

```
   ┌──────────────┐  ┌──────────────┐  ┌──────────────┐
   │   prompts    │  │     git      │  │  failures    │
   │ trail crate  │  │ extract crate│  │ reflect crate│
   └──────┬───────┘  └──────┬───────┘  └──────┬───────┘
          │                 │                 │
          ▼                 ▼                 ▼
       ┌──────────────────────────────────────────┐
       │          NORMALIZATION LAYER             │
       │   episode {sources, text, files, ts}     │
       └──────────────────────┬───────────────────┘
                              │
                              ▼
       ┌──────────────────────────────────────────┐
       │           EXTRACTION PIPELINE            │
       │  GLiNER ─► GLiREL ─► embed ─► confidence │
       │     │                          │         │
       │     └─ low confidence? ─► route crate    │
       │        (LLM fallback w/ PII strip)       │
       └──────────────────────┬───────────────────┘
                              │
                              ▼
       ┌──────────────────────────────────────────┐
       │             DEDUP + LINK                 │
       │   semantic match against existing graph  │
       │   merge or create new entity/edge        │
       └──────────────────────┬───────────────────┘
                              │
                              ▼
       ┌──────────────────────────────────────────┐
       │           WRITE TO GRAPH                 │
       │   ctxgraph episode + entity + edges      │
       │   bi-temporal: valid_at, recorded_at     │
       └──────────────────────┬───────────────────┘
                              │
                              ▼
       ┌──────────────────────────────────────────┐
       │           WIKI MAINTENANCE               │
       │   confidence >= auto_merge_threshold     │
       │     ─► write/update markdown page        │
       │   else                                   │
       │     ─► open wiki PR for review           │
       │   append entry to wiki/log.md            │
       └──────────────────────────────────────────┘
```

Every input path produces an **episode** — a normalized chunk with source metadata, text, file references, and timestamps. The extraction pipeline is shared across all sources; what differs is the producer.

---

## Episode shape

```rust
pub struct Episode {
    pub id: EpisodeId,                   // uuid v7 (time-sortable)
    pub source: EpisodeSource,           // Trail | Git | Failure | Manual
    pub source_ref: String,              // e.g., "github.com/.../pull/847" or "trail/...jsonl"
    pub text: String,                    // raw text to extract from
    pub files_touched: Vec<PathBuf>,     // for code anchoring
    pub authors: Vec<Author>,            // who produced this episode
    pub valid_at: DateTime<Utc>,         // when the underlying event happened
    pub recorded_at: DateTime<Utc>,      // when illuminate ingested it
    pub repo: RepoId,                    // which repo this belongs to
}

pub enum EpisodeSource {
    Trail { agent: AgentKind, session_id: String },
    Git { commit_sha: String, kind: GitEpisodeKind }, // commit | pr_body | review_comment
    Failure { incident_ref: Option<String> },
    Manual { tag: Option<String> },
}
```

Episodes are append-only. The same conversation captured twice produces two episodes (with different `recorded_at` but the same `source_ref`); dedup happens at the graph layer, not at the episode layer.

---

## 1. Prompt-trail capture (`illuminate-trail`)

The trail crate watches agent session storage and produces `Trail` episodes.

### What it watches

| Agent | Path | Format | Watch method |
|-------|------|--------|--------------|
| Claude Code | `~/.claude/projects/<hash>/<session-id>.jsonl` | JSONL | inotify |
| Cursor | `~/.cursor/conversations/...` (varies by version) | JSON | poll @ 5s |
| Codex | `~/.codex/sessions/...` | JSON | inotify |

The on-disk format for each agent is treated as an external dependency that may shift between versions; the trail crate includes a per-agent normalizer.

### Normalization

Raw session jsonl gets normalized to:

```json
{
  "session_id": "...",
  "agent": "claude-code",
  "model": "claude-sonnet-4-6",
  "started_at": "2026-05-06T09:00:00Z",
  "ended_at": "2026-05-06T09:45:00Z",
  "repo_path": "/home/rsx/Desktop/projx/payments-service",
  "messages": [
    { "role": "user", "ts": "...", "text": "..." },
    { "role": "assistant", "ts": "...", "text": "...", "tool_calls": [...] },
    ...
  ],
  "files_touched": ["src/cache.rs", "tests/cache_test.rs"],
  "tool_invocations": [
    { "name": "illuminate_audit", "params": {...}, "result": {...} },
    ...
  ]
}
```

### Persistence

Normalized output is written to `.illuminate/trail/<date>-<topic-slug>-<agent>.jsonl`. The topic slug is derived from the first user message via cheap keyword extraction.

`trail/` is gitignored. It contains the dev's raw prompts, which are sensitive (may include credentials, paths, secrets the dev pasted in). The graph is what gets shared via wiki/.

### Opt-in scope

The watcher only captures sessions whose `repo_path` corresponds to a repo with an `illuminate.toml`. A dev's personal projects, dotfiles, or work outside opted-in repos are **not** captured.

### v0.1 scope

- Claude Code only
- File watching via inotify (Linux/macOS)
- No Cursor/Codex (deferred to v0.2)
- No real-time extraction — extraction runs on session end (jsonl finalized)

---

## 2. Decision extraction (`illuminate-extract`)

Extracts decisions from text-bearing episodes — primarily Git and Trail, but the same pipeline runs on any `Episode`.

### Sources

| Source | Detection method |
|--------|------------------|
| Git commits | post-commit hook + periodic backfill |
| PR bodies | GitHub/GitLab API poll, configured per-repo |
| PR review comments | same |
| Trail jsonl files | filesystem watch (after session ends) |
| Manual `illuminate log` | CLI write |
| ADRs / `docs/decisions/` | bootstrap one-shot import |

### Pipeline stages

```
text input
   │
   ▼
┌───────────────────────────────────────────┐
│  STAGE 1: signal scoring                  │
│   regex + keyword heuristics              │
│   "we chose", "rejected", "instead of",   │
│   "decision:", "ADR", PR labels, etc.     │
│   ─► score 0.0–1.0                        │
└────────────────┬──────────────────────────┘
                 │ score < 0.3? skip
                 ▼
┌───────────────────────────────────────────┐
│  STAGE 2: GLiNER (entity extraction)      │
│   detect: Component, Service, Database,   │
│   Pattern, Person, Constraint, Decision   │
└────────────────┬──────────────────────────┘
                 │
                 ▼
┌───────────────────────────────────────────┐
│  STAGE 3: GLiREL (relation extraction)    │
│   between entities: chose, rejected,      │
│   replaced, depends_on, caused, etc.      │
└────────────────┬──────────────────────────┘
                 │
                 ▼
┌───────────────────────────────────────────┐
│  STAGE 4: confidence aggregation          │
│   per-entity confidence, per-relation     │
│   confidence, joint score                 │
└────────────────┬──────────────────────────┘
                 │ joint < threshold?
                 │       │
                 │       ▼
                 │   ┌───────────────────────┐
                 │   │  STAGE 4b: LLM        │
                 │   │  fallback (~30%)      │
                 │   │  PII strip via        │
                 │   │  cloakpipe first      │
                 │   │  then LLM classifier  │
                 │   └───────────┬───────────┘
                 │               │
                 ▼               ▼
┌───────────────────────────────────────────┐
│  STAGE 5: embed for retrieval             │
│   all-MiniLM-L6-v2 → 384-dim vector       │
│   stored alongside entity                 │
└────────────────┬──────────────────────────┘
                 │
                 ▼
┌───────────────────────────────────────────┐
│  STAGE 6: dedup + link                    │
│   semantic match against graph            │
│   if match: attach as additional source   │
│   if not:   create new entity/edge        │
└────────────────┬──────────────────────────┘
                 │
                 ▼
            write to graph
```

### Signal scoring

Stage 1 cheap filter that rejects ~90% of input. Heuristics include:

- Conventional-commit prefixes: `feat:`, `fix:`, `refactor:`, `chore:` (each weighted differently)
- Decision-language keywords: "chose", "rejected", "decided", "instead of", "we will not", "use X not Y"
- Length floor: < 30 characters skipped (commits like "fix typo" never carry decisions)
- PR labels: `architecture`, `breaking-change`, `decision`, `adr`
- File-set heuristic: changes touching `docs/decisions/` or `architecture/` boost score

The threshold is tunable in `illuminate.toml`:

```toml
[extraction]
signal_threshold = 0.3   # below this, skip ENTIRELY (no NER, no LLM)
confidence_threshold = 0.5  # below this, LLM fallback
```

### LLM fallback

When local NER joint confidence is below `confidence_threshold`, the route crate optionally calls a configured LLM:

```toml
[extraction.llm]
provider = "anthropic"          # anthropic | openai | ollama | none
model = "claude-haiku-4-5"
api_key_env = "ANTHROPIC_API_KEY"
pii_strip = true                # ALWAYS true — never override
max_calls_per_day = 1000        # safety cap
cache = true                    # idempotent; same input never re-asks
```

If `provider = "none"`, the extractor skips low-confidence episodes silently. They appear in the graph as `confidence: <local>` and surface in `illuminate wiki review` for the dev to manually classify.

PII stripping uses cloakpipe (or a built-in fallback). Substituted before send, restored on response. If the LLM provider fails (network, rate limit, auth), the episode is queued for retry; ingestion does not block.

### Output

For each accepted episode, the extractor writes:

- **Episode** — the normalized record (always written, even if no entities found)
- **Entities** — one or more typed entities (`Component`, `Decision`, etc.)
- **Edges** — typed relations between entities
- **Embeddings** — one per entity, for semantic retrieval
- **Wiki page (maybe)** — if confidence meets `auto_merge_threshold`, the agent writes/updates a wiki page

---

## 3. Failure capture (`illuminate-reflect`)

Failures are decisions in retrospect. The reflect crate captures them and feeds the graph the same way decisions are.

### Sources

| Source | How |
|--------|-----|
| Manual CLI | `illuminate failure log "..." --root-cause "..." --files ...` |
| Wiki page (markdown) | dev writes `wiki/failures/<date>-<slug>.md`; ingester picks it up |
| CI failure logs | optional integration; parses test failures and links to commit |
| Sentry / incident systems | optional webhook receiver |

### Manual CLI form

```bash
illuminate failure log \
    --title "Race condition in payments queue draining" \
    --root-cause "cursor read+advance not atomic under concurrent calls" \
    --fix "compare_and_swap on cursor" \
    --files "payments-service/src/queue.rs" \
    --severity high \
    --affected-modules payments-service
```

This writes a wiki page (`failures/2026-02-21-race-condition-payments.md`) and a graph entity (`Failure { id: ..., affected_files: [...] }`).

### Wiki-first form

If a dev manually writes `wiki/failures/<date>-<slug>.md` with the schema in `SCHEMA.md`, the wiki linter validates it and the next `illuminate wiki rebuild` adds the corresponding graph entry.

### What ends up in the graph

Each failure entity has:

- `affected_files: Vec<PathBuf>` — used for code-anchored audit
- `affected_modules: Vec<ModuleId>`
- `lesson: String` — the `## Lesson for future agents` section verbatim
- `severity: Low | Medium | High | Critical`

When an agent calls `illuminate_audit` with files that intersect any failure's `affected_files`, the audit response includes the failure summary and lesson.

---

## 4. Manual decision entries (`illuminate.toml` / wiki)

Two paths:

### Via `illuminate.toml`

Static, machine-enforceable rules — these are *policies*, not decisions per se, but the audit engine treats them similarly:

```toml
[policies.no_redis_payments]
rule = "must_use"
entity = "Memcached"
reject = ["Redis", "Dragonfly"]
paths = ["services/payments-service/**"]
reason = "deployment target disallows stateful sidecars"
severity = "error"
```

These are loaded at audit time, not ingested into the graph as episodes. They're the "hard constraints"; decisions are the "history."

### Via wiki

A dev creates a markdown file in `wiki/decisions/` directly (no CLI needed). On the next `illuminate wiki rebuild` (or git hook), the page is parsed and the graph entry created with `confidence: 1.0`.

---

## Bootstrapping

When `illuminate init` runs against an existing repo, it produces a one-shot ingestion of historical context. See `BOOTSTRAP.md` for the full pipeline. Summary of sources:

1. `CLAUDE.md` / `AGENTS.md` / `.cursorrules`
2. `docs/adr/`, `docs/decisions/`
3. Last 6 months of `git log`
4. README + CONTRIBUTING.md
5. Optional onboarding interview prompts

All run through the same extraction pipeline.

---

## Cost model

For a typical mid-size repo (1,000 episodes/month):

| Stage | Cost | Notes |
|-------|------|-------|
| Signal scoring | $0 | regex |
| GLiNER + GLiREL | $0 | local ONNX, ~50ms/episode |
| Embeddings | $0 | local ONNX, ~10ms/episode |
| LLM fallback (~30%) | ~$0.0003/episode | claude-haiku-4-5 with PII strip |
| Total per 1k episodes | ~$0.30 | vs Graphiti's ~$1.80 |

Queries cost $0 (entirely local).

---

## Failure modes & retry

- **Trail jsonl truncated mid-write** — extractor waits for inotify "close-write" event; partial files are deferred.
- **LLM provider unreachable** — episode requeued with exponential backoff; ingestion not blocked.
- **NER model load failure** — daemon refuses to start (loud failure); CLI commands work in degraded mode (wiki/audit only).
- **SQLite write contention** — single-writer model; trail watcher and CLI commands serialize via a tokio mutex inside the daemon.

All failures appear in `illuminate status` and are logged to `.illuminate/log/illuminate.log`.

---

## What's not in the ingestion path

- Telemetry, analytics, "phone home" — never.
- Outbound network calls beyond the optionally-configured LLM provider — never.
- Streaming/real-time extraction during a session — only at session end (deferred for v0.1, considered for v0.2 if useful).
- Cross-repo linking — each repo's graph is independent. A multi-repo dashboard is a v0.4+ commercial feature.
