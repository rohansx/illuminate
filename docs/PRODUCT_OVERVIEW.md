# Illuminate — Product Overview (v3)

**Domain:** illuminate.sh
**Tagline:** GitHub for agents. Prompts are the new source code — version, share, and enrich them like you do code.
**Secondary line:** Pre-prompt enrichment + prompt versioning + agent guarding, all local-first.

> **Status (May 2026):** v0.1 → v0.18 shipped — capture, extract, audit, wiki dashboard, MCP stdio+HTTP, GitHub Action all live. This document is a **positioning reset**: the work that already shipped is the substrate; the two products framed below (Enrich, Repo) are the next-cycle build on top of it. See [`ROADMAP.md`](ROADMAP.md) for what shipped vs what's planned and [`CHANGELOG.md`](../CHANGELOG.md) for per-version detail.

---

## The Core Insight

For 30 years, software engineering versioned the wrong thing — and got lucky.

In the pre-AI era, humans wrote source code and compilers produced binaries. We versioned the **source** (human intent) because binaries could be regenerated cheaply. Git, GitHub, and the entire SCM industry exist because *source is the artifact worth preserving*.

In the AI era, this assumption has quietly broken.

Humans now write prompts. AI produces source code. The source code is the new *binary equivalent* — it's the machine output. The **prompt + reasoning + iteration** is the new source — it's the human intent.

But we're still versioning the binaries. We commit code to git and throw away the prompts that produced it. The reasoning evaporates. The team's accumulated thinking exists only as scattered jsonl files on individual laptops.

**Illuminate exists to fix this.** It's GitHub for agents — the version control, collaboration layer, and intelligence engine for the thing humans now actually write: prompts.

---

## Two Products, One Substrate

Illuminate ships as one coherent system with two user-facing products, both powered by the same underlying decision graph and prompt index.

### Product 1: Illuminate Enrich — The Pre-Prompt Optimizer

Before a developer's prompt reaches Claude Code, Cursor, or Codex, Illuminate intercepts it, queries the team's accumulated context, and rewrites it to be more specific, grounded, and informed by relevant team decisions.

The dev types: *"add caching to the txn endpoint"*

Illuminate rewrites this (or surfaces additions) into: *"add caching to the txn endpoint at `src/payments/txn.rs`. Relevant past decisions: team rejected Redis 3mo ago due to deployment constraints (`wiki/decisions/2025-12-no-redis-payments.md`); team pattern for caching is LRU with 30s TTL (`wiki/patterns/lru-cache-with-ttl.md`). This module has had race-condition history — don't introduce mutex on the txn lock; lock-free atomics chosen (`wiki/failures/2026-02-race-condition-payments.md`)."*

The agent receives the enriched prompt and produces materially better code on the first try. Fewer iterations. Less drift. Faster ship.

**Why this is the wedge:**

- **Visible value on every prompt.** The dev sees better generations immediately, not just when something goes wrong.
- **Doesn't depend on agent cooperation.** Enrichment happens before the prompt reaches the agent, so it's deterministic — unlike post-hoc tools that hope the agent calls them.
- **Free quality lift.** No model upgrade, no fine-tuning, no new infra. Just better context routing.
- **Bottom-up adoption.** Devs install it because it makes *their* day better, not because management mandates it.

> **Implementation status:** **shipped in v0.19** — `illuminate-enrich` is live with the CLI verb, the `illuminate_enrich` MCP tool, and a property-tested determinism guarantee. The substrate (decision graph, semantic search, code-graph blast-radius, reading-plan generator) underneath shipped through v0.18; v0.20 pushed the FTS5 query sanitizer one layer deeper into `Graph::search` so every caller benefits transparently. See [`ROADMAP.md`](ROADMAP.md#whats-shipped-v01--v018).

### Product 2: Illuminate Repo — GitHub for Agents

A versioned, browsable, searchable record of every prompt the team has chosen to publish, the reasoning behind it, the code that resulted, and the decisions that emerged.

Think `git log` for prompts. `git blame` for "why does this code exist?" GitHub's web UI for browsing your team's accumulated thinking.

**Concretely:**

- Every prompt → response → code change is captured locally as a session.
- Devs explicitly publish sessions they want to share with the team (the `git commit` equivalent for prompts).
- Published sessions live in a team-shared git repo of markdown + structured data.
- Anyone on the team can browse, search, blame, and link to any past prompt.
- Future agents query the repo as context when generating new code.

**Why this matters:**

- **Onboarding stops being broken.** New hires query the repo to understand "why does this team use LRU instead of Redis?" — they get the original prompt and reasoning, not a stale ADR.
- **PR review gets its missing layer.** Reviewers see not just the diff but the prompt that produced it.
- **Knowledge compounds.** Every published prompt makes future enrichment smarter, future agents better-guided, and future humans better-informed.
- **Switching cost grows with usage.** A team six months in has a prompt history that can't be replicated by switching tools.

> **Implementation status:** **shipped in v0.21** — `illuminate-publish` is live with `illuminate publish --trail PATH --redaction <level> --team-repo PATH`, a `.git/hooks/pre-commit` installer, and 9 unit tests covering each redaction level. Capture, wiki rendering, and the dashboard substrate shipped through v0.18. The `docs/` content type (auto-draft from sessions, `illuminate ask`, `illuminate browse`) is the v3.2+ scope — see [`knowledge-layer.md`](knowledge-layer.md).

---

## Three Content Categories in the Team Repo

The team repo holds three kinds of content. All three are markdown + structured metadata in git. All three feed the same graph and the same enrichment pipeline. All three are queryable by humans and agents.

1. **Auto-captured.** Created by Illuminate as a byproduct of normal coding work — `sessions/` (via `illuminate-publish`), `decisions/`, `patterns/`, `failures/`, `modules/` (extracted by `illuminate-extract`).
2. **Author-written.** Created by humans in their editor of choice, committed to the team repo like any other code — `docs/architecture/`, `docs/adr/`, `docs/designs/`, `docs/runbooks/`, `docs/onboarding/`, `docs/integrations/`, `docs/conventions/`, **`docs/prompts/`** (the prompt cookbook), `docs/oncall/`.
3. **Ingested.** Pulled read-only from existing knowledge homes — `CLAUDE.md`, `AGENTS.md`, `.cursorrules`, repo ADRs, spec-kit artifacts, confluence pages, notion pages, github wiki, google docs, PDFs.

The first category ships today (v0.21). The second category is a schema convention — humans can write `docs/*.md` into their team repo and `illuminate-extract` already discovers them via the bootstrap pipeline. The third category needs `illuminate-ingest` (planned for v3.2 — see [`knowledge-layer.md`](knowledge-layer.md)).

What this unlocks once all three categories are live: cross-corpus Q&A via `illuminate ask`, doc decay detection against the tree-sitter code graph, auto-drafted design docs from sessions, prompt cookbooks that auto-suggest during enrichment, personalized onboarding journeys, on-call context bundles, and agent skill packs. The full landscape is in [`knowledge-layer.md`](knowledge-layer.md).

> **Secondary positioning frame.** Beyond "GitHub for agents," teams adopting the third content category can pitch Illuminate to engineering managers as: *the engineering team's AI-aware knowledge home — docs, decisions, prompts, and failures, all in git, all indexed, all queryable by humans and agents.* The primary developer-facing pitch is unchanged.

---

## The Four-Stage Pipeline

Illuminate's runtime is a four-stage pipeline. Every prompt flows through it:

```
   ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐
   │  ENRICH  │ → │ GENERATE │ → │  CAPTURE │ → │  CURATE  │
   │          │   │ (agent)  │   │          │   │          │
   │ pre-LLM  │   │  Claude  │   │  local   │   │  dev     │
   │ context  │   │  Cursor  │   │  trail   │   │  publishes│
   │ injection│   │  Codex   │   │  jsonl   │   │  to team │
   └──────────┘   └──────────┘   └──────────┘   └──────────┘
        ↑                                            │
        │                                            ▼
        │                                       ┌──────────┐
        │                                       │   TEAM   │
        └───────────────────────────────────────│   REPO   │
                  (graph indexes published       │ (github  │
                   prompts, decisions, patterns) │  for     │
                                                 │  agents) │
                                                 └──────────┘
                                                      │
                                                      ▼
                                                ┌──────────┐
                                                │   GUARD  │
                                                │  + AUDIT │
                                                │ (linter, │
                                                │  reflect)│
                                                └──────────┘
```

**Stage 1: Enrich (Product 1).** Before the prompt reaches the agent, the team's graph is queried for relevant context, which is injected into the prompt.

**Stage 2: Generate.** The enriched prompt goes to Claude Code / Cursor / Codex. The agent produces code.

**Stage 3: Capture.** The entire session — prompt, response, iteration, code change — is captured locally. This is automatic and silent. Nothing leaves the dev's machine.

**Stage 4: Curate.** When the dev is ready (usually at commit time), Illuminate asks: *publish this session to the team repo?* The dev chooses: publish in full, publish summary, publish only the decision, or discard. Nothing is shared without consent.

The team repo (Product 2) is the substrate. It feeds back into Stage 1 — every published prompt makes future enrichment smarter. The loop tightens.

Alongside the pipeline, Illuminate runs two guard rails (both shipped through v0.18):

- **Audit (linter).** Before the agent writes code, it cross-references the proposed change against the graph. Catches drift the enrichment didn't prevent. See [`AUDIT.md`](AUDIT.md).
- **Reflect (failure capture).** When generated code fails (test breaks, incident, runtime error), the failure context is captured and added to the graph. Future agents touching the same area see warnings.

| Stage | Shipped? | Crate |
|---|---|---|
| Enrich | Planned (v3.0) | `illuminate-enrich` (planned) |
| Generate | n/a — runs in the host agent (Claude Code, Cursor, Codex) | — |
| Capture | ✅ v0.1+ | `illuminate-trail` |
| Curate | Partial — manual `illuminate failure log` and dashboard quick-add ship today; the per-session publish gesture is v3.0 | `illuminate-publish` (planned) |
| Guard / Audit | ✅ v0.1+ | `illuminate-audit`, `illuminate-mcp` |
| Reflect | ✅ v0.1+ | `illuminate-reflect` |

---

## The Trust Model

This is the part that makes Illuminate viable. Get it wrong and developers won't install it. The full trust spec lives in [`trust-model.md`](trust-model.md); the canonical "what is stored at each layer, with what retention, and the redaction pipeline" reference is in [`data-model.md`](data-model.md). A summary follows.

### What stays local, always

- **Every session, captured automatically.** Stored in `.illuminate/trail/` on the dev's machine. Gitignored. Never auto-uploaded. Never seen by management.
- **Personal scratch space.** Half-formed thoughts, dead ends, embarrassing prompts, debugging spirals — all stay local.
- **Enrichment queries.** When the team graph is queried to enrich a prompt, the query happens locally against a local mirror.

### What gets published, only with explicit consent

- **Curated sessions.** The dev chooses what to publish, with what redaction level (full session / summary / decision only).
- **Decisions, patterns, and failures.** Surfaced from published sessions, written to the team repo as markdown.
- **No silent uploads.** Ever. No "anonymized telemetry" by default. No "team analytics opt-out checkbox you missed."

### What's never built

- **No individual scoring.** Illuminate will not produce dashboards rating individual developers' prompt quality. Period.
- **No management surveillance views.** No "see what your team is prompting" admin dashboard.
- **No prompt leaderboards.** Aggregate team-level trends are possible only if a team explicitly opts in *and* the data is genuinely anonymized at the source.

This isn't a marketing constraint. It's a structural commitment. The moment Illuminate enables individual surveillance, devs game it, sabotage it, or refuse to install it — and the whole compounding-knowledge thesis collapses. The local-first architecture *enforces* the trust model, it doesn't just promise it.

---

## What It's Not

- **Not surveillance software.** No individual scoring. No management dashboards. No prompt rating for HR. If a competitor builds that, it's a different product.
- **Not a replacement for git or GitHub.** Runs alongside them. The team repo can literally be a GitHub repo with structured markdown.
- **Not a hosted SaaS by default.** Local-first. Self-hostable. Hosted illuminate.sh is an optional convenience layer added later, not the product itself.
- **Not a prompt management tool for production APIs.** Tools like Langfuse, PromptLayer, and Braintrust manage *production prompts shipped to end users*. Illuminate handles *development-time coding sessions*, a different artifact entirely.
- **Not a generic AI-powered wiki.** The wiki is a byproduct of the loop, not the product. The product is the prompt-versioning + enrichment system.
- **Not confluence or notion.** Illuminate is git-based and markdown-native. Teams write docs in their editor of choice (VS Code, Obsidian, neovim, whatever). Illuminate makes those docs first-class context for AI agents — not the other way around. External-source ingestion (confluence, notion, github wiki, google docs) is **always read-only**: never writes back. See [`knowledge-layer.md`](knowledge-layer.md) for the boundary in detail.
- **Not a code review tool.** Adjacent space, different artifact (Illuminate owns the prompt + reasoning trail, not the review verdict).
- **Not a code intelligence engine.** `illuminate-index` is a narrow internal code graph (file → symbol mapping, import graph, module boundaries, 1-hop call edges, capped blast-radius) sized exactly to what `illuminate-audit` and `illuminate-enrich` need. Full call graphs, inheritance chains, cluster analysis, type inference, and cross-repo references stay out of scope — that's GitNexus / code-review-graph / Sourcegraph territory. Illuminate composes with them via MCP rather than replacing them. See [`code-graph-strategy.md`](code-graph-strategy.md) for the decision matrix.
- **Not competing with spec-kit.** spec-kit handles work *before* code is written (intent). Illuminate handles work *during and after* code is written (reality). They compose — Illuminate can ingest spec-kit's constitution and specs as decision sources.

---

## A Day in the Life

A walkthrough of a single dev's day with Illuminate installed, showing both products in motion:

**9:00 AM — Dev opens Claude Code in `payments-service`.**
The `illuminate-trail` daemon notices a new session start. Capture begins silently.

**9:15 AM — Dev types: "Add caching to the transaction lookup endpoint."**

Before Claude Code sees this prompt, `illuminate-enrich` intercepts it. It:

1. Identifies "caching" as a concept the team has decided on.
2. Queries the graph: finds `Decision::NoRedis` (3 months old) and `Pattern::LRU30s`.
3. Identifies "transaction lookup endpoint" → `src/payments/txn.rs` via the code graph.
4. Queries the graph: finds `Failure::RaceConditionPayments` (2 months old).
5. Rewrites the prompt with this context inline.

Claude Code receives the enriched prompt and produces code that uses the team's LRU pattern, in `src/payments/txn.rs`, with the race-condition warning addressed. **Right on the first try.** No iteration needed.

**9:30 AM — Dev runs `git commit`.**

A pre-commit hook asks: *"Publish this session's reasoning to the team repo?"* The dev clicks "publish summary." Illuminate writes a structured markdown page to `team-illuminate/sessions/2026-05-25-add-caching.md` linking to the commit, summarizing the reasoning, and updating the graph.

**11:00 AM — Different dev, different repo, opens Cursor.**

Same daemon, separate per-repo trail. Same enrichment pipeline. The team graph is shared via the team repo; each repo's graph is local but indexes the team repo's published content.

**3:00 PM — A flaky test reveals an old race condition.**

Dev fixes it, writes a brief post-mortem. The `illuminate-reflect` ingester picks it up and adds it to the graph. From now on, any agent touching the affected module will see the warning *during the enrich stage*, before the prompt is even sent.

**Next week — New hire joins.**

They `cd` into the team repo, run `illuminate browse`, and read the team's published sessions in order: decisions made, patterns adopted, failures avoided. They ask: "Why does the payments service use in-memory caching instead of Redis?" The repo answers in one click — original prompt, reasoning, alternatives considered, the LRU pattern that emerged. They didn't have to ping anyone on Slack. They're productive on day two.

---

## Architecture (high-level)

Sixteen Rust crates planned (fourteen shipped through v0.18, two planned for v3.0), one binary, one SQLite file per repo. See [`ARCHITECTURE.md`](ARCHITECTURE.md) for the full diagrams and [`CRATES.md`](CRATES.md) for per-crate detail.

**Shipped (v0.1 → v0.18):**

- `illuminate-core` — graph + SQLite (built on `ctxgraph`)
- `illuminate-config` — shared `illuminate.toml` parsers
- `illuminate-trail` — automatic session capture (Claude Code, Cursor, Codex)
- `illuminate-extract` — local NER (GLiNER + GLiREL via ONNX)
- `illuminate-embed` — local embeddings (all-MiniLM-L6-v2)
- `illuminate-index` — code indexer (tree-sitter, six languages)
- `illuminate-audit` — policy engine + drift detection (the linter)
- `illuminate-bootstrap` — five cold-start sources
- `illuminate-watch` — daemon harness + git/GitHub ingestion
- `illuminate-reflect` — failure capture (Reflexion episodes)
- `illuminate-route` — reading-plan generator (FTS5 + semantic RRF)
- `illuminate-wiki` — markdown layer + dashboard
- `illuminate-mcp` — JSON-RPC server exposing tools to agents (stdio + HTTP)
- `illuminate-cli` — user-facing binary

**Planned (v3.0):**

- `illuminate-enrich` — pre-LLM prompt enrichment (Product 1's wedge crate)
- `illuminate-publish` — explicit publish gesture, redaction-level chooser, pre-commit hook

Compiled to a single binary. No Docker, no Python, no Neo4j, no cloud required.

---

## Positioning

**Prompts are the new source code.** Illuminate is the version control, collaboration layer, and intelligence engine for them. GitHub for agents.

**Local-first.** All capture is local. All enrichment is local. All audits are local. Publishing to the team repo is explicit and consensual.

**Deterministic enrichment.** No LLM in the enrich path means the same prompt + graph state produces the same enriched output. Reproducible, debuggable, fast.

**Cheap.** ~$0.30 per 1,000 sessions ingested (mostly local ONNX). Free at the enrichment and query paths. No per-seat SaaS pricing until the optional hosted layer.

**Compounding.** The team repo gets stronger with use. A team six months in has materially better enrichment than a team that just installed it. The switching cost grows with usage.

**Built on ctxgraph.** The bi-temporal knowledge graph engine is its own open-source project (2.4× F1 vs Graphiti, ~250× faster). Illuminate is the product layer.

---

## Existing Landscape

**Prompt-to-code lineage (closest):**
- `usegitai.com` — tracks AI-authored lines with git notes. Narrower (no graph, no enrich, no publish flow). Cloud-first.
- `Ekaanth/blameprompt` — open-source git-blame-for-prompts. Capture only, no graph.

**Spec-driven development (complementary):**
- `github/spec-kit` — 95k stars, structured pre-code workflow. Illuminate complements: spec-kit captures planned intent, Illuminate captures actual reality. Likely ships as a spec-kit extension in addition to standalone.

**Karpathy LLM Wiki implementations:**
- Original gist and derivatives. Personal-knowledge focused. Illuminate is the team-coding-workflow specialization.

**Code knowledge graphs (substrate-adjacent):**
- `tirth8205/code-review-graph` — structural code graph for token-efficient review. Complementary to Illuminate's decision graph. `illuminate-index` does the rust-native subset needed for audit queries.

**Session-capture utilities:**
- `getagentseal/codeburn` — multi-tool session parser for cost observability. Different output (cost dashboards). Useful as a reference for session-format reverse-engineering.

**Prompt management (different problem):**
- Langfuse, PromptLayer, Braintrust, PromptHub — production API prompt management. Illuminate handles development-time coding sessions, a different artifact entirely.

No existing tool ships the full enrich → capture → curate → publish → guard loop for coding sessions. Components exist independently; the integration is the product.

---

## Why It Could Work

- **The framing is true and unexploited.** Prompts genuinely are the new source code. No one has built proper version control for them yet. The category is open.
- **Enrichment is a visible win.** Better generations on every prompt is a tangible value prop that doesn't require behavior change from devs.
- **GitHub-comparison is sticky.** "GitHub for agents" lands in five seconds. Every developer instantly knows what it means.
- **Local-first matches the buyer.** AI companies in regulated verticals (Harvey, Abridge, Hippocratic AI) cannot ship dev prompts to a third-party SaaS. Local-first is non-negotiable, and it's the architecture from day one.
- **Onboarding is the killer use case.** "New hires read the team repo to understand how this team thinks" is concrete and procurement-ready.
- **Compounding moat.** Six months of published team prompts can't be replicated by switching tools. Switching cost grows with usage.
- **Spec-kit ecosystem is a tailwind, not a competitor.** 95k stars of validated demand for AI-coding workflow tooling. Illuminate fits as both standalone and as a spec-kit extension.

## Why It Might Not Work

Honest list:

- **Enrichment quality is the technical bet.** If enriched prompts aren't visibly better than raw prompts, the product has no wedge. Requires real engineering on the query-and-context-injection pipeline.
- **Cold start is hard.** Day one of installation, the team repo is empty. Enrichment has no context. Bootstrapping (parsing existing CLAUDE.md, ADRs, git history, spec-kit artifacts) needs real effort. *Mitigated by the shipped [`BOOTSTRAP.md`](BOOTSTRAP.md) pipeline — 5 sources, idempotent, sub-5-minute runs on a 6-month-old repo.*
- **Curation friction.** If devs find "publish this session?" prompts annoying, they'll skip them. Need to find the right default (auto-publish on commit? prompt every time? batched weekly?) and let teams tune.
- **Prompt-as-source framing is novel and needs evangelism.** People will dismiss it at first ("why would I version a prompt?"). Need landing-page copy, demos, talks that make the framing land. *See [`philosophy.md`](philosophy.md) for the manifesto.*
- **Solo founder bandwidth.** Substantial product. v3.0 alone is 6–8 weeks of focused work on top of the v0.18 substrate. Day job is a real constraint.

Mitigations:

- Ship enrichment quality as the v3.0 demo. If enrichment doesn't visibly improve generations, kill the wedge.
- Bootstrap from every available source on `illuminate init` (CLAUDE.md, ADRs, git history, spec-kit, existing wikis). *Already shipped — see [`BOOTSTRAP.md`](BOOTSTRAP.md).*
- Default to "publish summary on git commit" with one-click override. Friction minimized.
- Lead public communication with the enrichment demo (visible quality win) before the prompt-as-source manifesto (philosophical pitch).
- Scope ruthlessly. v3.0 is enrich + publish on top of the existing capture/audit/wiki substrate. Cursor/Codex enrich hooks, hosted cloud, etc., are v3.1+.

---

## Build Plan

> **What already shipped (v0.1 → v0.18).** The substrate is real and end-to-end: trail capture (Claude Code, Cursor, Codex), local NER pipeline, audit + policy engine (with `decision_ref`, `evidence`, `wiki_url`, `trace_id`), MCP server (stdio + Streamable HTTP), GitHub Action, bootstrap (5 sources), wiki dashboard at `illuminate wiki serve` (home / browse / search / audit playground / JSON API / quick-add form). 14 crates, 650+ tests passing. See [`CHANGELOG.md`](../CHANGELOG.md) and [`ROADMAP.md`](ROADMAP.md#whats-shipped-v01--v018).

The v3 build plan picks up from v0.18 and targets the two wedge crates that turn the substrate into the two-product story above.

### v3.0 — The Enrich Wedge (target: 6–8 weeks)

The minimum that demonstrates **both products** in motion on top of the existing substrate:

1. **`illuminate-enrich`** — pre-LLM prompt enrichment hook. Wraps the agent invocation (CLI wrapper first; pre-write hook second). Queries `illuminate-route` for a reading plan + relevant decisions/patterns/failures; injects them into the prompt deterministically. *The wedge crate.*
2. **`illuminate-publish`** — explicit publish gesture. New CLI verb `illuminate publish` and pre-commit hook. Redaction-level chooser (full session / summary / decision-only / discard). Writes to a configurable team repo path (defaults to a sibling `team-illuminate/` directory or a configured git remote).
3. **`illuminate browse`** — terminal UI over published sessions. Search, blame ("who prompted this code?"), open original session jsonl in `$EDITOR`.
4. **Trust-model enforcement.** Default-deny on uploads. No `.illuminate/trail/` ever crosses a network boundary. `illuminate trust check` lints the config.
5. **Enrichment demo artifact.** One 60-second video: raw prompt → enriched prompt → noticeably-better generation. Single most important launch artifact.

Skip in v3.0:

- Cursor / Codex *enrich* hooks (Claude Code only — capture already works for all three).
- LLM-assisted summary at publish time (use a deterministic template).
- Hosted cloud.
- Cross-repo decision sharing.

### v3.1 — Broaden Capture & Curation (target: 6–8 weeks after v3.0)

- Cursor + Codex enrich hooks.
- LLM-assisted distill at publish time (with human-in-the-loop preview).
- `spec-kit-illuminate` — extension that ingests spec-kit's constitution and specs as decision sources.
- Reflect ingester for CI logs / Sentry / PagerDuty (the manual `failure log` flow shipped in v0.11; this automates it).
- Per-repo enrichment policies in `illuminate.toml`: which decisions to surface for which paths, max-token budget per enrich call, dampening for low-confidence decisions.

### v3.2 — Polish + Adoption (target: ongoing)

- Self-coaching dashboard (local-only, dev-owned, never shared upward) — "your last 10 prompts, what enrichment added, what the agent did with it." Pure dev value, never aggregated.
- Wiki search v2 (semantic + grep with ranking weights per page type).
- Bootstrap helpers for spec-kit constitutions, AGENTS.md variants, additional ADR formats.
- VS Code / Cursor / Zed editor extensions wrapping `illuminate enrich` and `illuminate publish` (thin wrappers, not new surfaces).

### v3-cloud — Optional Hosted Layer

Only after meaningful OSS adoption (50+ teams):

- Hosted illuminate.sh — team repo mirror with faster search and queries.
- Cross-repo decision sharing (with consent — see [`trust-model.md`](trust-model.md)).
- Enterprise SSO + audit logs.
- Multi-team graph federation.

Strictly opt-in. Self-hosted is always a first-class option.

---

## Open Questions

1. **First user.** Self-dogfood on Utkrushta repos? CloakPipe-adjacent design partners? Solo dogfooding on personal projects? *Recommendation: solo dogfood, then 3–5 design partners.*

2. **Curation default.** Auto-publish summary on git commit (one-click override) vs. always-ask vs. never-publish-by-default? *Recommendation: auto-publish summary on commit, with one keystroke to opt out. Friction has to be near-zero or devs will skip.*

3. **Team repo format.** Standard git repo with structured markdown + json, or proprietary format with conversion tools? *Recommendation: standard git + markdown. No lock-in. Editor-agnostic. Greppable.* (Already shipped: markdown is the source-of-truth in `wiki/` — extending the same convention to published sessions.)

4. **Enrichment trigger.** MCP tool (agent has to call it) vs. CLI wrapper (intercepts the call) vs. pre-write hook (deterministic)? *Recommendation: CLI wrapper for v3.0 (most reliable, no agent cooperation needed), pre-write hook for v3.1.*

5. **License.** MIT for the crates, or AGPL to discourage closed-source forks? *Recommendation: MIT for the core crates (maximize adoption). Commercial moat through hosted features later, not through licensing.* (Already shipped: MIT.)

6. **Hosted timeline.** When to start building the cloud product? *Recommendation: not until v3.0 + v3.1 ship and have 50+ active teams. Premature cloud kills startups; the OSS adoption story funds the cloud product.*

---

## Next Steps

In order:

1. **Validate enrichment end-to-end on a real codebase.** Take a 6+ month repo (one of your own works — Utkrushta is a candidate). Manually construct what enriched prompts would look like for 10 real prompts from your history. Compare generation quality with/without enrichment. If the difference is visible, ship. If not, fix the pipeline before going public.

2. **Add `illuminate-enrich` and `illuminate-publish` to [`ARCHITECTURE.md`](ARCHITECTURE.md) and [`CRATES.md`](CRATES.md)** with their own data-flow diagrams. *Done as part of this v3 reset.*

3. **Land [`trust-model.md`](trust-model.md)** — the explicit local-vs-shared boundary document. Critical for adoption; devs will read this before installing. *Done as part of this v3 reset.*

4. **Land [`philosophy.md`](philosophy.md)** — the "prompts are the new source code" manifesto. Public-facing essay. Could be the launch post. *Done as part of this v3 reset.*

5. **Rebrand illuminate.sh landing page** to match the new positioning. Archive the old OSS-copilot codebase.

6. **Ship the v3.0 enrichment demo.** One 60-second video showing a prompt → enriched prompt → better generation. This is the single most important artifact for launch.
