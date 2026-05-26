# Illuminate — Docs & Team Knowledge Layer

**Companion doc to** [`PRODUCT_OVERVIEW.md`](PRODUCT_OVERVIEW.md). Captures the docs-as-first-class-content decision and what it unlocks. Lands as the v3.2+ scope on the v3 [`ROADMAP.md`](ROADMAP.md).

> **Note on commands:** `ilm` in this document is the planned **shorthand alias** for the `illuminate` CLI (landing alongside `illuminate ask` / `illuminate browse` in v3.2). Until that ships, every `ilm <subcommand>` example is equivalent to `illuminate <subcommand>`.

---

## The Insight

In the AI era, engineering teams are producing *more* docs than ever — design docs, ADRs, runbooks, post-mortems, integration guides, agent specs, prompt cookbooks. The volume keeps growing because:

- Architectural decisions multiply with every new agent integration
- Prompt patterns need to be documented so the team can share what works
- Post-mortems are richer because agents leave fuller failure trails
- New AI services (vector DBs, embedding models, agent frameworks) each need documentation
- Spec-driven workflows (spec-kit) produce structured spec artifacts as a normal part of development

But these docs live in a fragmented mess:

- Some in the repo as markdown (`docs/`, `ARCHITECTURE.md`, `ADR/`)
- Some in confluence
- Some in notion
- Some in google docs
- Some in slack pins and threads
- Some in github wiki
- Some in obsidian vaults
- Some as pdfs in shared drives

**Nobody knows where the latest version of anything is.** New hires ask "where are the docs?" and get five different answers. Agents can't find them because they're scattered. Decisions made in slack threads don't reach the wiki. Wiki pages go stale because they're disconnected from code.

This is the docs problem in the AI coding era. Illuminate is uniquely positioned to solve it because:

1. The team illuminate repo is already shipping as a git-based, markdown-native, structured knowledge home.
2. The illuminate graph already indexes prompts, decisions, and failures — adding docs is the same schema discipline applied to one more content type.
3. The enrichment pipeline already queries the graph for context — docs become another input automatically.
4. Local-first architecture means docs stay in the team's own infrastructure, not a third-party SaaS.

---

## What Was Wrong With My Earlier Pushback

In an earlier iteration of this conversation, I pushed back hard on the framing of "central knowledge base for the org." I was over-correcting. Specifically:

- I argued the idea was "scope creep toward confluence/notion."
- I painted it as "building a second product inside the first one."
- I claimed it would "shift the buyer" and "dilute positioning."

I was wrong. The proposal was never "build a confluence competitor." It was: **let the team illuminate repo accept docs as a first-class content type alongside prompts, decisions, and failures.**

That's not scope creep. It's the same architectural pattern applied to a content type we should have included from the start. The team repo's schema was always going to be:

```
team-illuminate/
├── decisions/
├── patterns/
├── failures/
├── modules/
├── sessions/
└── docs/        ← this should have been there from day one
```

Adding `docs/` is one line in the schema. The ingestion, indexing, search, and enrichment all use infrastructure that already exists. It's not a new product. It's the right scope for the existing one.

---

## The Correct Boundary

Illuminate accepts docs. Illuminate does not host docs in the SaaS sense, edit docs in the WYSIWYG sense, or compete with confluence/notion as company-wide knowledge platforms.

The line is clear:

| Illuminate does | Illuminate does not |
|---|---|
| Accept markdown docs into the team repo | Provide a WYSIWYG editor |
| Index docs for enrichment and search | Replace confluence/notion |
| Detect doc decay against the codebase | Become the company-wide knowledge platform |
| Ingest from external sources (read-only) | Sync edits back to external sources |
| Auto-draft docs from coding sessions | Generate docs from thin air |
| Render markdown for human browsing | Build a feature-complete wiki UI |
| Be the engineering team's AI-aware knowledge home | Be every team's knowledge platform |

The buyer stays the same: engineering teams using AI agents. The architecture stays the same: local-first, git-based, single binary. The product stays the same: team-aware context for AI coding agents. The schema just includes docs.

---

## Three Content Categories in the Team Repo

The team illuminate repo accepts three categories of content. All three are markdown + structured metadata in git. All three feed the graph. All three are queryable by agents through MCP. All three are browsable by humans.

### 1. Auto-captured content (Illuminate generates)

Created by Illuminate as a byproduct of normal coding work:

- **Sessions** — curated prompt + reasoning trails, published by devs at commit time
- **Decisions** — extracted from sessions, commits, PR descriptions
- **Patterns** — extracted from repeated successful approaches
- **Failures** — extracted from post-mortems and reflect captures

### 2. Author-written content (humans write)

Created by humans, committed to the team repo like any other code:

- Architecture docs (`docs/architecture/`)
- ADRs (`docs/adr/`)
- Design docs (`docs/designs/`)
- Runbooks (`docs/runbooks/`)
- Onboarding guides (`docs/onboarding/`)
- Integration references (`docs/integrations/`)
- Team conventions (`docs/conventions/`)
- **Prompt cookbooks** (`docs/prompts/`) — first-class content type
- On-call playbooks (`docs/oncall/`)

Humans write these in their editor of choice (VS Code, Obsidian, neovim, whatever supports markdown). Illuminate is not the editor; git is the storage.

### 3. Ingested content (from external sources)

Pulled from existing knowledge homes, read-only, indexed into the graph:

- Existing CLAUDE.md, AGENTS.md, .cursorrules
- Existing ADRs from `docs/adr/`
- Spec-kit artifacts (`.specify/memory/constitution.md`, `specs/*/spec.md`)
- Confluence pages (by URL list or space ID)
- Notion pages (by workspace or page IDs)
- GitHub wiki
- Google Docs (via connector)
- PDFs and slides (extracted via OCR if needed)
- Slack threads marked with a specific emoji or pinned (optional)

**External ingestion is read-only.** Confluence stays the source of truth for confluence pages; Illuminate is a consumer that makes them queryable in the AI workflow. Never writes back to external systems.

---

## How Devs Interact With Docs

### Reading docs

```bash
ilm browse                    # opens local markdown render in browser
ilm browse docs/architecture  # navigates to a section
ilm search "auth flow"        # full-text + semantic search across all content
ilm graph "authentication"    # shows entity graph around a concept
```

Or just open the markdown files in any editor. The team repo is a normal git repo; standard tooling works.

### Writing docs

```bash
cd team-illuminate
vim docs/architecture/payment-flow.md
git add docs/architecture/payment-flow.md
git commit -m "Document payment flow architecture"
git push
```

That's it. No special UI, no upload step. Illuminate's daemon detects the change, re-indexes the doc, updates the graph, makes it available for enrichment.

### Asking questions across the corpus

```bash
ilm ask "why did we choose postgres over mongo?"
ilm ask "what's the deployment pipeline for the payments service?"
ilm ask "how do we usually prompt for input validation?"
ilm ask "what failures have we had with the cache layer?"
```

This is the **`ilm ask`** feature — chat with the team's brain. Not chat with one doc; cross-document, cross-decision, cross-failure synthesis. Uses the graph for entity resolution + retrieval, then a final synthesis call.

The same capability is exposed as an MCP tool (`illuminate_ask`) so agents in Claude Code, Cursor, and Codex can query it during normal work — giving generations access to the entire team knowledge corpus, not just the active file.

### Ingesting external sources

```bash
# in illuminate.toml
[ingest]
local_docs = ["docs/", "ARCHITECTURE.md", "AGENTS.md"]
confluence = ["https://company.atlassian.net/wiki/spaces/ENG/"]
notion = ["workspace-id-123"]
github_wiki = true
spec_kit = true
```

Then:

```bash
ilm ingest          # one-time pull, extract, index
ilm ingest --watch  # keep in sync on a schedule
```

---

## What This Unlocks: New Features

Once docs are first-class content in the team repo, a set of features becomes possible that wasn't on the table before. These are not all v0.1 work; they're the strategic landscape that opens up.

### 1. Chat with the team's brain (`ilm ask`)

The headline feature. Natural-language questions across the entire team knowledge corpus — decisions, patterns, failures, sessions, and docs. Cross-document synthesis, not single-doc Q&A.

**Why this is differentiated:** notion AI and confluence AI answer questions about a single page or workspace. `ilm ask` answers questions across the team's full engineering memory, with the graph providing structured retrieval that beats pure embedding search.

### 2. Doc decay detection

Illuminate sees both the docs and the code changes flowing through it. When code that a doc describes has materially changed since the doc was written, Illuminate opens a PR against the team repo flagging stale sections.

```
PR: docs/auth-flow.md may be stale
The auth flow described references AuthService.validateToken(),
but that method was removed 3 months ago in commit a1b2c3d.
Sections lines 45-67 may need updating.
```

**Why this is differentiated:** wikis can't detect code drift because they don't see the code. Code-review tools can't detect doc drift because they don't see the docs. Illuminate sees both.

### 3. Auto-drafted docs from coding sessions

A session captures a complete reasoning trail. If a developer just designed a new caching layer through a 2-hour session, Illuminate has enough context to auto-draft a design doc as a PR against the team repo.

```bash
ilm publish --as-doc docs/designs/cache-layer.md
```

Dev reviews, edits, merges. A doc exists where there would have been none. The cost of writing design docs collapses, so they actually get written.

### 4. Prompt cookbook as first-class content

`docs/prompts/` becomes a first-class directory in the team repo. Teams curate proven prompt patterns: "how we prompt for postgres migrations," "our standard pattern for adding API endpoints," "how to prompt for security review."

These get auto-suggested during enrichment when the dev's prompt matches a known pattern. The first team to nail this becomes 2-3x more effective at prompting than peers, and the gap compounds.

**Why this matters:** developers share code patterns constantly. Almost nobody shares prompt patterns systematically. This is an underserved area in AI tooling that Illuminate is positioned to own.

### 5. Spec-driven workflows with team context

When a dev runs spec-kit's `/speckit.specify`, the spec generation now happens with full team-doc context as input. Specs come out grounded in the team's architectural constraints, naming conventions, and past decisions — not generic.

`spec-kit-illuminate` extension passes ingested team docs as additional context to spec-kit's generation pipeline. Specs go from generic to team-shaped.

### 6. Cross-repo team knowledge

A company with 10 microservice repos has its institutional knowledge fragmented across 10 illuminate installations. An optional "team-of-teams" layer lets them share certain doc categories across repos (security policies, deployment patterns, on-call runbooks).

Opt-in, repo by repo. Federated, not centralized. The shared repo is just another git repo that all team illuminate installations index.

### 7. Agent-aware doc review

When someone PRs a doc change, Illuminate checks it against existing decisions and patterns for contradiction.

```
PR: docs/caching.md
WARNING: This doc says "use Redis for caching."
A decision exists at decisions/2025-12-no-redis-payments.md
rejecting Redis for the payments service. Resolve before merging.
```

Same audit logic that catches code drift, applied to doc drift.

### 8. Personalized onboarding journeys

`docs/onboarding/` is a curated reading order for new hires. Illuminate auto-suggests the right reading sequence based on the graph: foundational decisions first, then patterns, then service-specific docs.

A new hire's first week becomes:

```bash
ilm onboard           # start guided journey
ilm onboard --next    # mark current section read, suggest next
ilm onboard --status  # see progress
```

New hires productive in days instead of months. Ramp time is the most expensive thing in engineering hiring; collapsing it is procurement-grade value.

### 9. Doc-aware code review

A reviewer is reading a PR. Illuminate sidebar (or CLI output, or PR comment) shows:

```
This PR touches src/auth/.
Relevant docs:
  - docs/architecture/auth-flow.md (last updated 2 weeks ago)
  - docs/adr/2025-08-auth-provider.md
Recent failures in this area:
  - failures/2026-02-token-refresh-race.md
Open decisions:
  - decisions/2026-04-deprecate-session-cookies.md (in progress)
```

No more "wait, was there a doc about this?" The context comes to the reviewer automatically.

### 10. Living architecture diagrams

`architecture.md` files with mermaid diagrams can be auto-updated as the code structure changes. Illuminate's tree-sitter parse of the code keeps the diagrams accurate. Doc + code stay in sync without manual upkeep.

### 11. On-call context bundle

When an incident fires, `ilm oncall <service>` generates a context bundle: relevant docs, recent decisions affecting the failing service, past failures in the same area, the prompts that produced the code that's now failing.

The on-call engineer gets a focused reading list instead of grepping through wikis at 3 AM.

### 12. Agent skill packs

Claude Code and similar agents support "skills" — reusable instruction bundles. Illuminate auto-generates a skill pack from the team's docs + decisions + patterns. Install once; the agent now knows everything your team knows. Updates automatically as the repo evolves.

```bash
ilm skill build > team-skill.md  # ready to drop into ~/.claude/skills/
```

---

## How This Changes the Architecture

The good news: **almost nothing changes.** Docs slot into existing infrastructure.

| Component | Change required |
|---|---|
| `illuminate-core` | Add `Doc` entity type (and sub-types: ADR, Runbook, Design, Onboarding, etc.) |
| `illuminate-trail` | No change |
| `illuminate-enrich` | Query the `Doc` entity type during context retrieval. Returns relevant snippets. |
| `illuminate-publish` | No change for sessions. Adds `--as-doc` mode for auto-drafting docs. |
| `illuminate-extract` | Add doc-specific extraction (extracts decisions and patterns from authored docs) |
| `illuminate-embed` | No change (embeddings work on any markdown) |
| `illuminate-index` | No change |
| `illuminate-audit` | Audit docs against decisions when docs are PR'd |
| `illuminate-reflect` | No change |
| `illuminate-mcp` | Add `illuminate_ask` tool (cross-corpus Q&A) |
| `illuminate-cli` | Add `ilm ask`, `ilm browse`, `ilm ingest`, `ilm onboard`, `ilm oncall`, `ilm skill` |
| **New: `illuminate-ingest`** | Crate for external-source ingestion (confluence, notion, github wiki, etc.) |

**One new crate** (`illuminate-ingest`), one new MCP tool (`illuminate_ask`), a handful of new CLI commands. That's the entire architectural impact. Everything else is schema additions.

---

## What Goes in the Team Repo

Updated schema:

```
team-illuminate/
├── illuminate.toml             # config + ingest sources
├── schema.md                   # how the agent maintains the repo
├── index.md                    # auto-generated catalog
├── log.md                      # append-only audit trail
│
├── sessions/                   # auto-captured (published with consent)
│   ├── 2026-05-25-add-caching.md
│   └── ...
│
├── decisions/                  # extracted + author-written
│   ├── 2025-12-no-redis-payments.md
│   └── ...
│
├── patterns/                   # extracted + author-written
│   ├── lru-cache-with-ttl.md
│   └── ...
│
├── failures/                   # extracted + author-written
│   ├── 2026-02-race-condition-payments.md
│   └── ...
│
├── modules/                    # auto-generated module pages
│   ├── payments-service.md
│   └── ...
│
└── docs/                       # author-written, first-class content
    ├── architecture/
    │   ├── overview.md
    │   └── auth-flow.md
    ├── adr/
    │   ├── 0001-database-choice.md
    │   └── 0002-monorepo-structure.md
    ├── designs/
    │   └── cache-layer.md
    ├── runbooks/
    │   └── payment-service-rollback.md
    ├── onboarding/
    │   ├── README.md
    │   ├── 01-codebase-tour.md
    │   └── 02-deployment.md
    ├── prompts/                # the prompt cookbook
    │   ├── README.md
    │   ├── adding-api-endpoint.md
    │   └── database-migration.md
    ├── integrations/
    │   └── stripe.md
    ├── conventions/
    │   ├── naming.md
    │   └── error-handling.md
    └── oncall/
        └── payment-service.md
```

Everything is markdown. Everything is in git. Everything is indexed by the graph. Everything is queryable by humans and agents.

---

## Build Plan Update

Mapping the four phases below onto Illuminate's actual versioning (v3.0 = `illuminate-enrich` + `illuminate-publish`, both shipped in v0.19 + v0.21):

### v3.0 — already shipped

Stays the same. Docs ingestion of local markdown (`docs/`, `CLAUDE.md`, `AGENTS.md`) is part of the bootstrap pipeline (5 sources, shipped through v0.11). No new crates needed for the v3.0 release. See [`ROADMAP.md`](ROADMAP.md#whats-shipped-v01--v018).

### v3.2 — adds `ilm ask` (the docs phase begins)

- New crate: `illuminate-ingest` (read-only adapters for external sources — confluence, notion, github wiki, google docs, spec-kit)
- New MCP tool: `illuminate_ask` (cross-corpus Q&A over decisions/patterns/failures/sessions/docs)
- New CLI: `illuminate ask`, `illuminate browse`, `illuminate ingest` (plus `ilm` shorthand alias)
- Schema additions: `Doc` entity type and sub-types in `illuminate-core::types`

### v3.3 — adds the high-value doc features

- Doc decay detection (PR bot that flags stale sections against tree-sitter code drift)
- Auto-drafted docs from sessions (`illuminate publish --as-doc`)
- Prompt cookbook tooling (auto-suggest during enrichment when prompt matches a `docs/prompts/` pattern)
- Agent-aware doc review (audit docs against decisions when PR'd)

### v3.4 — adds the workflow features

- Personalized onboarding (`illuminate onboard`)
- On-call context bundle (`illuminate oncall <service>`)
- Agent skill packs (`illuminate skill build`)
- Cross-repo team-of-teams federation
- Living architecture diagrams (mermaid auto-updates against the tree-sitter parse)

---

## What This Means for Positioning

The primary positioning doesn't change:

> **Team-aware context for your AI coding agents. Locally captured. Privately published. Compounding with every prompt.**

But a secondary frame becomes available, specifically for the docs angle:

> **Illuminate is the engineering team's AI-aware knowledge home. Docs, decisions, prompts, and failures — all in git, all indexed, all queryable by humans and agents.**

This is a stronger pitch in some contexts (engineering managers, eng leads concerned about onboarding and knowledge management) without contradicting the primary developer-facing pitch.

The "what it's not" section gets one new line:

> Not confluence or notion. Illuminate is git-based and markdown-native. Teams write docs in their editor of choice. Illuminate makes those docs first-class context for AI agents — not the other way around.

---

## What Stays Out (Reaffirmed Boundaries)

To prevent scope creep from creeping back in:

- **No WYSIWYG editor.** Ever. Teams write markdown in their editor of choice.
- **No proprietary cloud doc hosting.** The team repo is git. Self-hosted by default.
- **No write-back to external sources.** Confluence/notion ingestion is read-only. Always.
- **No general-purpose knowledge management.** Buyer is engineering teams using AI agents, not "every team in the company."
- **No replacing confluence/notion.** Co-exists with them. Makes them more useful by extracting their content for AI workflows.

These boundaries are what keep illuminate from drifting into a category it can't win.

---

## Summary

Docs are first-class content in the team illuminate repo, alongside sessions, decisions, patterns, and failures. They flow through the same pipeline: written or ingested → indexed into the graph → queryable by humans (`illuminate ask`, `illuminate browse`) and agents (`illuminate_ask` MCP tool) → feed enrichment for future prompts.

This is not a new product. It's the team repo's schema correctly scoped to what engineering teams actually need.

The new features it unlocks — `illuminate ask`, doc decay detection, auto-drafted docs, prompt cookbooks, onboarding journeys, skill packs — are genuinely valuable and uniquely possible because Illuminate already has the graph, the indexer, the enrichment pipeline, and the capture loop.

**This is the right scope. Ship it.**
