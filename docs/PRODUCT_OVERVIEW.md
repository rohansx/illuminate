# Illuminate — Product Overview (v2)

**Domain:** illuminate.sh
**Tagline:** Compounding context for AI-coding teams.
**Secondary line:** ESLint for intent — the linter, the wiki, and the memory your agents are missing.

> **Status:** v0.1 closed loop shipped May 2026. Trail capture (Claude Code), wiki layer, bootstrap, audit, MCP, GitHub Action all live. v0.2 (Cursor + auto-distill) and v0.3 (pre-write hook + adoption polish) on the roadmap. See `ROADMAP.md`.

---

## The Problem

AI coding agents now write a substantial fraction of production code. The tools to *generate* code have raced ahead. The tools to *remember* the reasoning behind that code, the failed attempts that shaped it, and the architectural decisions it has to respect — those have not.

Three losses compound silently in every team using Claude Code, Cursor, or Codex:

**1. Reasoning is lost the moment a session ends.**
A developer iterates with an agent for an hour — exploring approaches, hitting dead ends, refining the spec, finally landing on an implementation. They commit the code. The session goes to `~/.claude/projects/<hash>.jsonl` on their laptop. The next reviewer sees the diff and not a word of why.

**2. Decisions made today are forgotten by next week.**
"We rejected Redis for caching because the deployment target doesn't allow stateful sidecars." Two weeks later, a different developer asks an agent to add caching. Agent suggests Redis. The decision is invisible to the agent and probably to the developer too.

**3. Failures don't generalize.**
An agent writes code that breaks production. The fix gets shipped. The lesson — *agents touching this module should know about race condition X* — exists only in the post-mortem doc nobody reads.

Each loss alone is annoying. Together they mean the team's collective knowledge stays flat while code volume grows exponentially. Onboarding new hires gets harder. PR reviews get shallower. Agents drift.

This is the problem Illuminate solves.

---

## The Insight

The answer isn't "ship better prompts" or "write more docs." Both have been tried for decades and don't work — humans don't maintain artifacts that don't directly produce code.

The answer is a **flywheel** where every coding session, every architectural decision, and every failure becomes a source that feeds a structured graph, which then guards future agents from drift, which then surfaces decisions to humans when they need them.

```
prompts captured → graph fed → agents guarded → failures captured → graph fed → ...
```

The loop tightens with every cycle. After three months, the graph holds enough context that new hires query it instead of asking seniors. After six months, agents working on the codebase produce noticeably less drift than agents working on uninstrumented codebases. After a year, switching off Illuminate means losing the team's accumulated engineering memory — which is what makes it a real product, not a feature.

This is what "compounding context" means. The graph gets stronger with use. That's the moat.

---

## What It Does

Illuminate has three input mechanisms, one knowledge graph, and two output surfaces.

### Three input mechanisms

**1. Prompt-trail capture.** A daemon watches Claude Code, Cursor, and Codex sessions. Every coding session is automatically captured: the prompt, the iteration, the failed approaches, the final plan. Stored locally as raw "trail" files, never sent to a third party. From these, the system extracts decisions and patterns into the graph.

**2. Decision ingestion.** Git commits, PR descriptions, and merge messages are scanned by local ONNX models (GLiNER for entities, GLiREL for relations, all-MiniLM-L6-v2 for embeddings) to surface high-signal architectural decisions. LLM fallback ~30% of the time for ambiguous cases, with PII stripped first. The team can also write decisions explicitly in `illuminate.toml`.

**3. Failure capture (Reflect).** When an agent's code fails — tests break, production incident, runtime error — the failure context (root cause, fix, affected files) is recorded. Future agents touching the same area see warnings derived from the failure.

### One knowledge graph

A bi-temporal, append-only knowledge graph stored as a single SQLite file in `.illuminate/graph.db`. Entities are deduplicated; relationships are typed; everything is timestamped. Built on top of `ctxgraph`. Local-first, deterministic queries, no LLM in the query path.

### Two output surfaces

**1. The Linter (machine-readable).** When an agent proposes a change via MCP, Illuminate audits the proposal against the graph and `illuminate.toml` policies. Returns: violations, warnings, relevant past decisions, prior failures in the same area. The agent course-corrects *before* writing code.

**2. The Wiki (human-readable).** Markdown pages, browsable in any editor or via Obsidian. Decisions, patterns, anti-patterns, failures — all linked, all searchable. Auto-maintained by the system; humans review distilled pages via PR. New hires read the wiki to understand "how this team thinks." Senior engineers query it when they forget why a past decision was made.

---

## What It's Not

- **Not a replacement for git, GitHub, or GitLab.** It runs alongside them. It uses git notes for some metadata, but it doesn't replace any existing version control.
- **Not a prompt management SaaS.** Tools like Langfuse, PromptLayer, and Braintrust manage *production API prompts*. Illuminate captures *development-time coding sessions*, which is a different artifact entirely.
- **Not a generic AI-powered wiki.** Notion AI and similar products bolt LLMs onto existing knowledge bases. Illuminate is the other direction: knowledge accrues as a side effect of normal coding work.
- **Not a code review tool.** CodeRabbit, Qodo, and Graphite Agent review PRs after the fact. Illuminate prevents drift *before* code is written, and captures context the reviewer would otherwise lack.
- **Not a hosted SaaS.** Local-first, self-hosted, your data never leaves your infrastructure. The graph and wiki sit in your repo or on your machine.

---

## How It Works in Practice

A walkthrough of a single dev's day with Illuminate installed:

**9:00 AM — Dev opens Claude Code in `payments-service` repo.**
The daemon notices a new session start. It silently begins capturing.

**9:15 AM — Dev asks: "Add Redis caching to the transaction lookup endpoint."**
Before Claude Code writes any code, it calls `illuminate_audit` (configured as a required pre-write step in `CLAUDE.md`). The audit returns: *"Past decision (3 months ago): Redis rejected for this service due to stateful sidecar restrictions on the deployment target. Alternative used: in-memory LRU with 30s TTL. See `wiki/decisions/2025-12-no-redis-payments.md`."* Claude Code surfaces this to the dev, suggests the LRU approach instead.

**9:45 AM — Implementation lands. Dev commits.**
The session is finalized in the trail. The decision-extractor runs, finds nothing materially new (an existing pattern was reused), so the graph isn't updated.

**11:00 AM — Different dev, different repo, opens Cursor.**
Same daemon, separate per-repo graph. Same workflow.

**3:00 PM — A flaky test reveals an old race condition.**
Dev fixes it, writes a brief post-mortem in `wiki/failures/`. The reflect ingester picks it up and adds it to the graph. From now on, any agent touching the affected module will see the warning.

**Next week — New hire joins.**
They run `illuminate wiki serve` locally, browse the team's accumulated decisions, patterns, and failures. They ask: "Why does the payments service use in-memory caching instead of Redis?" The wiki answers in one click. They didn't have to ping anyone on Slack.

This is what the loop looks like in motion.

---

## Architecture (high-level)

Ten Rust crates, one binary, one SQLite file per repo. See `ARCHITECTURE.md` for the full diagrams.

- `illuminate-core` — graph + SQLite (built on `ctxgraph`)
- `illuminate-trail` — prompt session capture (Claude Code, Cursor, Codex hooks)
- `illuminate-extract` — local NER (GLiNER + GLiREL via ONNX)
- `illuminate-embed` — local embeddings (all-MiniLM-L6-v2)
- `illuminate-index` — code indexer (tree-sitter)
- `illuminate-audit` — policy engine + graph queries
- `illuminate-reflect` — failure capture and ingestion
- `illuminate-route` — LLM fallback router (used sparingly, ~30% of extraction)
- `illuminate-mcp` — JSON-RPC server exposing audit tools to agents
- `illuminate-cli` — `illuminate init`, `illuminate wiki`, `illuminate audit`, etc.

Compiled to a single binary. No Docker, no Python, no Neo4j, no cloud.

---

## Positioning

**Privacy-first.** All capture is local. The graph is local. Wiki is local. PII is stripped before any LLM fallback. Self-host or run entirely offline.

**Deterministic.** Queries against the graph are deterministic — no LLM in the audit path means the same code change gets the same audit result every time. Reproducible, debuggable, gradable.

**Cheap.** ~$0.30 per 1,000 episodes ingested vs Graphiti's $1.80 (because most extraction runs locally on ONNX). Free for the query path entirely.

**Compounding.** The graph gets stronger with use. A team three months in produces materially better-guided agents than a team that just installed it. Switching cost grows with usage.

**Built on `ctxgraph`.** The bi-temporal knowledge graph engine is its own open-source project (2.4× F1 vs Graphiti, ~250× faster). Illuminate is the product layer on top.

---

## Existing Landscape

Verified competitors and adjacent tools as of May 2026:

**Closest competitors (prompt-to-code lineage):**
- `usegitai.com` (Git AI) — closest existing product. Tracks AI-authored lines with git notes, has an `/ask` skill for future agents to query original intent. Strong execution. Narrower scope (no graph, no audit, no wiki). Cloud-first.
- `Ekaanth/blameprompt` — open-source git-blame-for-prompts. Local-first. Solid capture mechanism, no graph or audit layer.

**Karpathy LLM Wiki implementations:**
- The original gist (`karpathy/442a6bf555914893e9891c11519de94f`)
- `rohitg00`'s LLM Wiki v2 — extends with quality scoring, supersession, mesh sync
- `jgoldfed/keppi` — graph traversal CLI for markdown wikis

**Knowledge graph engines (substrate):**
- `getzep/graphiti` — primary baseline. Higher cost, slower, requires Python and Neo4j.
- `ctxgraph` — Illuminate's underlying engine. Already public.

**Prompt management (different problem, included for completeness):**
- Langfuse, PromptLayer, Braintrust, PromptHub, Agenta, LangSmith — production prompt management for API-served LLM applications. Adjacent space, different artifact.

No existing tool ships the full capture → distill → guard → reflect loop. The components exist independently; the integration is the product.

---

## Why It Could Work

- **The pain is real and growing.** Every team using AI coding agents at scale hits drift. The HN thread "Do we need a new GitHub for AI coding era?" hit the front page in late 2025 because the complaint is universal.
- **The artifact is new.** The prompt + reasoning + plan trail is a new kind of engineering artifact. No existing tool owns it. Format and tooling are still up for grabs.
- **Local-first matches the buyer.** AI companies in regulated verticals (Harvey, Abridge, Hippocratic AI) cannot ship dev prompts to a third-party SaaS. Local-first is table stakes for that segment.
- **Onboarding is the killer use case.** "New hires query the team wiki instead of pinging seniors" is concrete, measurable, and procurement-ready. It's the user story to lead with.
- **Compounding moat.** The graph gets more valuable with every session. Three months of accumulated context can't be replicated by switching to a competitor.

## Why It Might Not Work

Honest list:

- **Cold start.** Day-one of installation, the graph is empty and the linter has nothing to enforce. Bootstrapping needs explicit attention. (Mitigation: ingest existing git history, parse `CLAUDE.md` and `AGENTS.md`, optional onboarding interviews. See `BOOTSTRAP.md`.)
- **Distillation quality.** When a session captures something, deciding *what's worth saving* is a judgment call. Wrong calls pollute the wiki and erode trust. (Mitigation: ship dev-triggered distill in v0.1, layer on automatic classification only when accuracy is proven.)
- **Agent-calls-audit reliability.** MCP tools alone aren't enough — agents skip them. Need pre-write hooks, `CLAUDE.md` directives, *and* PR-time CI gates working in concert. (Mitigation: ship all three; pick the strongest for the demo.)
- **Karpathy LLM Wiki hype is at peak.** Many builders will ship "wiki for X" products in the next quarter. Differentiation has to be the loop, not the wiki itself. (Mitigation: lead with the linter and the audit demo; the wiki is a side benefit.)
- **Solo founder bandwidth.** This is a substantial product. v0.1 alone is ~3 months of focused work. Day job is a real constraint. (Mitigation: scope ruthlessly. Ship the loop in its smallest form. Defer everything else.)

---

## Build Plan

See `ROADMAP.md` for full milestone detail. Summary:

### v0.1 — Closed loop, narrow scope (target: 8-10 weeks)

The absolute minimum that demonstrates the flywheel:

1. **`illuminate-core`** (already done in `ctxgraph`) — the substrate
2. **`illuminate-trail`** — Claude Code session capture daemon, writes to `.illuminate/trail/`
3. **`illuminate-extract`** — local NER pipeline (GLiNER + embeddings) extracting decisions from trail + git history
4. **`illuminate-audit`** — MCP tool exposing audit-before-write to agents
5. **`illuminate-cli`** — `illuminate init`, `illuminate audit`, `illuminate wiki serve`
6. **Markdown wiki rendering** — graph → human-browsable pages

Skip in v0.1: Cursor/Codex hooks (Claude Code only), automatic distill (dev-triggered only), reflect integration (manual log only), dashboards, analytics, auth, anything cloud.

Ship as open-source rust crates + binary. Single HN launch post.

### v0.2 — Broaden capture (target: 4-6 weeks after v0.1 ships)

- Cursor + Codex hooks
- Reflect ingester (failures from CI logs, sentry, manual reports)
- LLM-classified distill (with human-in-the-loop review)
- Per-repo `illuminate.toml` policies

### v0.3 — Polish + adoption (target: ongoing)

- Pre-write hook for Claude Code (deterministic audit trigger, not MCP-dependent)
- PR-time CI integration (GitHub Action)
- Wiki search (semantic + grep)
- Bootstrap helpers (parse existing ADRs, CLAUDE.md, etc.)

### v0.4+ — Commercial layer (only after meaningful OSS adoption)

- Hosted graph mirror (optional, opt-in)
- Team dashboards
- Cross-repo decision sharing
- Enterprise auth + RBAC

Don't build any of v0.4 until v0.1-v0.3 has 50+ teams using it organically.

---

## Open Questions

1. **First user.** Self-dogfood on internal repos? Design partners? Solo dogfooding on personal projects?
2. **Distill default.** Dev-triggered, rule-based, or LLM-classified? (Recommendation: dev-triggered for v0.1.)
3. **Wiki source-of-truth.** Markdown pages → graph index, or graph → markdown export? (Recommendation: markdown is source-of-truth, graph indexes it. Easier to reason about, easier to git-version. See `SCHEMA.md`.)
4. **Distribution.** Single rust binary via cargo + curl install? Homebrew? VS Code extension wrapper? (Recommendation: cargo install + curl bash one-liner for v0.1.)
5. **License.** MIT for the crates, or AGPL to discourage closed-source forks? (Recommendation: MIT for adoption, build commercial moat through hosted features later.)

---

## Next Steps

- Validate end-to-end extraction accuracy on a real codebase before shipping v0.1
- Decide on distillation default and wiki source-of-truth question
- Land `ARCHITECTURE.md` and `SCHEMA.md` (done)
- Pivot illuminate.sh landing page to the new positioning
- Archive the old illuminate-as-OSS-copilot framing (done — see `docs/old/`)
