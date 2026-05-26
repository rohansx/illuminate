# Illuminate — Roadmap

> **Status:** v0.21 shipped (May 2026). The v3.0 wedges (`illuminate-enrich` + `illuminate-publish`) are live; both products of the v3 positioning are end-to-end functional. This document tracks the **next-cycle plan** — v3.1 (broaden capture), v3.2 (docs as first-class content per [`knowledge-layer.md`](knowledge-layer.md)), v3.3 (high-value doc features), v3.4 (workflow features), v3.5 (polish + adoption), v3-cloud (optional hosted). See [`CHANGELOG.md`](../CHANGELOG.md) for the per-version log.

---

## What's shipped (v0.1 → v0.18)

The full closed loop is live:

| Stage | What ships | Reference |
|---|---|---|
| **Capture** | Claude Code, Cursor, Codex sessions → `.illuminate/trail/` (jsonl, gitignored, opt-in) | `illuminate-trail` |
| **Extract** | Local NER pipeline (GLiNER + GLiREL via ONNX), embeddings (all-MiniLM-L6-v2), optional LLM fallback with PII strip | `illuminate-extract`, `illuminate-embed` |
| **Graph** | Bi-temporal SQLite knowledge graph on top of `ctxgraph`. FTS5 + semantic RRF search. | `illuminate-core`, `illuminate-route` |
| **Code graph** | tree-sitter symbol + edge index across Rust/Go/TS/Python/Java/C. Recursive-CTE BFS for blast-radius. | `illuminate-index` |
| **Bootstrap** | 5 sources (agent files, ADRs, git history, README/CONTRIBUTING, interview YAML). Idempotent. < 5 min on a 6-month-old repo. | `illuminate-bootstrap` |
| **Audit (linter)** | Policy DSL (`rejected_pattern`, `must_use`, `frozen`), `decision_ref` plumbing, `evidence`/`confidence`/`trace_id`/`wiki_url` on every finding. Code-graph blast-radius joined with decision graph. | `illuminate-audit` |
| **Reflect (failure capture)** | Reflexion episodes via `illuminate failure log` CLI and `illuminate_reflect` MCP tool. Surfaced in future audits via `find_relevant`. | `illuminate-reflect` |
| **MCP** | JSON-RPC server: `stdio` (default) + Streamable HTTP via axum 0.8 with bearer auth. Tools (`illuminate_audit/explain/search/decisions_for/failures_for/get_wiki_page/route/reflect/impact`). `resources/list` + `resources/read` for wiki pages. `prompts/list` + `prompts/get` (audit_check, summarize_failures). | `illuminate-mcp` |
| **Wiki dashboard** | `illuminate wiki serve` — home / browse / search / **audit playground** / JSON API / quick-add form for non-CLI page creation. Dark mode, mobile responsive, no JS framework. | `illuminate-wiki` |
| **CI** | GitHub Action `audit-pr@master` — comments findings on PRs via `gh`, fails check on `error`-severity violations. Exit codes 0/2/3 per `docs/CLI.md`. | `.github/actions/audit-pr/` |
| **CLI surface** | `init`, `bootstrap`, `audit`, `audit-diff`, `audit-pr`, `impact`, `explain`, `failure log`, `decisions list/show/for`, `patterns list/show`, `failures list/show`, `index`, `search`, `rebuild`, `wiki serve/redact`, `trail import/list/register/watch/install-service`, `mcp serve`, `models download`, `status`, `stats`. | `illuminate-cli` |

**14 crates. 650+ tests passing. `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all` clean.**

Per-version detail in [`CHANGELOG.md`](../CHANGELOG.md).

### Still deferred from v0.18

These remain on the punch list but are not in the v3.0 critical path:

- Bootstrap interactive TTY interview (YAML file-driven works today).
- `failure log` editor mode (`$EDITOR` template).
- MCP HTTP Server-Sent Events streaming.
- mTLS / OAuth for MCP HTTP (bearer-token-only today).
- `wiki redact` graph-side deletion (file-side replacement only).
- `evidence` field shape change from `Option<String>` to `Vec<String>` for full `docs/AUDIT.md` parity.
- Audit history view (`/audits`) showing recent audit runs over time.
- Capturing actual PNG screenshots into `docs/screenshots/`.

---

## Release philosophy (carried into v3)

- **Ship the loop, then expand it.** Every release demonstrates the full pipeline end-to-end; new releases tighten the existing loop rather than introducing parallel half-loops.
- **Each release valuable on its own.** No "you need v3.1 to use v3.0" gates. v3.0 stands alone as the enrich-wedge demo + publish gesture.
- **No commercial layer until OSS adoption is proven.** v3-cloud is off the table until v3.5 hits its adoption exit criteria (50+ teams).
- **Evidence before scope.** Decisions about v3.1 scope are deferred until v3.0 is in users' hands.
- **Local-first is non-negotiable.** Every release must work fully offline. Cloud features are additions, never replacements.

---

## v3.0 — The Enrich Wedge — **mostly shipped**

**Goal:** Turn the v0.18 substrate into the two-product positioning. Ship `illuminate-enrich` (Product 1's wedge) and `illuminate-publish` (Product 2's curate gesture). Both products demonstrably in motion in a single 60-second demo video.

### Status

| Component | Status |
|-----------|--------|
| `illuminate-enrich` (CLI wrapper + property test + MCP tool) | ✅ **Shipped v0.19** |
| `illuminate-publish` (CLI verb + pre-commit hook + redaction-level chooser) | ✅ **Shipped v0.21** |
| Trust-model docs (`docs/trust-model.md`) | ✅ Shipped (v3 docs reset commit `021913f`) |
| Schema extension (`page_type: session` in `SCHEMA.md`) | ✅ Shipped v0.21 |
| FTS5 sanitizer at the `Graph` boundary (unblocks audit MCP) | ✅ Shipped v0.20 (bonus) |
| `illuminate browse` (terminal UI over published sessions) | 🔜 Still pending |
| `illuminate trust check` (config linter for off-host writes) | 🔜 Still pending |
| Enrichment demo artifact (60-second video) | 🔜 Still pending — primary launch artifact |

### Out of scope (v3.0)

- Cursor / Codex *enrich* hooks. Claude Code only for enrichment in v3.0 (capture already works for all three).
- LLM-assisted summary at publish time (use a deterministic template).
- Hosted cloud / illuminate.sh mirror.
- Cross-repo decision sharing.
- VS Code / Zed editor extensions.
- Pre-write enrich hook (CLI wrapper only in v3.0).

### Exit criteria

- `illuminate enrich "<prompt>"` returns an enriched prompt that includes at least one decision, pattern, or failure reference on a populated repo — **verified end-to-end on this repo at v0.19**.
- `illuminate publish` invoked from a pre-commit hook writes a valid markdown page to the configured team-repo path — **verified end-to-end on this repo at v0.21** (produced `/tmp/team-illuminate-smoke/sessions/2026-05-10-...md` + graph episode `019e604f-...`).
- `illuminate trust check` returns 0 on a default `illuminate.toml` and non-zero when a misconfigured `TeamRepoTarget::GitRemote` is present without a paired consent flag — **still pending**.
- Determinism guarantee for enrich: same `(prompt, graph_state_hash)` → byte-identical output — **verified by property test in v0.19**.
- Single-binary install still works (`cargo install illuminate-cli`); no new system dependencies — **verified, no new deps in v0.19 / v0.20 / v0.21**.

### Distribution

- `cargo install --git github.com/rohansx/illuminate illuminate-cli`
- Homebrew tap continues
- One launch artifact: the 60-second enrichment demo + a short post titled along the lines of *"Prompts are the new source code. We need a GitHub for them."*

---

## v3.1 — Broaden Capture & Curation

**Target window:** 6–8 weeks after v3.0 ships.

**Goal:** Make the enrich + publish loop work across Cursor and Codex, and automate failure capture so the team graph keeps compounding without manual `failure log` calls.

### In scope

| Component | What ships |
|-----------|-----------|
| `illuminate-enrich` (extended) | Cursor + Codex enrich hooks. Pre-write hook mode for Claude Code (deterministic, not MCP-dependent). |
| `illuminate-publish` (extended) | LLM-assisted summary at publish time with human-in-the-loop preview. Templates per `page_type`. |
| Reflect ingester | CI log parsers (GitHub Actions, CircleCI) + Sentry/PagerDuty webhook receivers. Failure auto-promotion from incident systems. |
| `spec-kit-illuminate` | Extension that ingests spec-kit's constitution and specs as decision sources. Bidirectional: Illuminate's published sessions can also surface as spec-kit context. |
| Enrichment policies | Per-repo `[enrich]` block in `illuminate.toml`: which decisions to surface for which paths, max-token budget per enrich call, dampening for low-confidence decisions. |

### Exit criteria

- A team using Cursor as primary agent can install Illuminate and get enriched prompts via the Cursor hook.
- A failure logged in Sentry produces a wiki page (with optional human review) and surfaces as a warning in subsequent enrich + audit calls.
- spec-kit users can run Illuminate alongside spec-kit without manual config — the extension auto-detects spec-kit artifacts and ingests them.
- LLM-assisted summary precision ≥ 70% on a manually-graded test set of 100 sessions.

---

## v3.2 — Docs as First-Class Content

**Target window:** 6–8 weeks after v3.1.

**Goal:** Make the team repo's third content category — author-written and ingested docs — first-class alongside auto-captured sessions/decisions/patterns/failures. The headline feature is **`illuminate ask`**: cross-corpus Q&A over everything the graph knows. Companion design doc: [`knowledge-layer.md`](knowledge-layer.md).

### In scope

| Component | What ships |
|-----------|-----------|
| `illuminate-ingest` (new crate) | Read-only adapters for external knowledge homes — confluence, notion, github wiki, google docs, spec-kit artifacts, additional local `docs/*.md` trees. Each adapter implements `IngestAdapter` (`fetch_all` / `fetch_since`). **Strictly read-only:** no write-back. Configured per-team via `[ingest]` block in `illuminate.toml`. |
| `Doc` entity type in `illuminate-core` | New entity with sub-types: `Adr`, `Runbook`, `Design`, `OnboardingGuide`, `Convention`, `PromptCookbook`, `Integration`, `Oncall`, `Generic`. Schema additions only — no breaking changes to existing entity types. |
| `illuminate ask` CLI verb | Cross-corpus Q&A. Pipeline: parse question → graph retrieval (decisions + patterns + failures + sessions + docs) → final synthesis LLM call. Single source of truth for "what does this team think about X?". |
| `illuminate browse` CLI verb | Already in v3.0 scope; landing here in v3.2 as a docs-aware renderer (links between docs/decisions/patterns auto-resolved). |
| `illuminate ingest [--watch]` | Run all configured adapters once or on a schedule. Watch mode keeps the graph in sync incrementally via `fetch_since(watermark)`. |
| `illuminate_ask` MCP tool | Exposes cross-corpus Q&A to Claude Code, Cursor, and Codex. The MCP-side companion to the CLI verb. |
| `ilm` shorthand alias | Symlink / `[bin]` alias `ilm` → `illuminate` so the `ilm ask` / `ilm browse` / `ilm ingest` UX in [`knowledge-layer.md`](knowledge-layer.md) works without typing the long form. |
| `docs/` schema entry | `SCHEMA.md` gains a `docs/` directory section with per-subdir conventions (adr, designs, runbooks, onboarding, prompts, integrations, conventions, oncall). |

### Exit criteria

- `illuminate ingest` against a repo with `[ingest.local_docs]` configured ingests every `docs/*.md` into the graph as `source: ingested:local-docs` episodes. Round-trip via `illuminate ask` returns the right doc.
- At least three external adapters land (confluence + notion + github-wiki recommended; spec-kit + google-docs nice-to-have).
- `illuminate ask "why did we choose X?"` answers with citations across decision pages, session pages, and doc pages on a populated team repo.
- Trust-model invariant verified: `grep -r 'fn push\|fn write\|fn commit_back' crates/illuminate-ingest/` returns nothing. Read-only by construction.
- `ilm ask` / `ilm browse` / `ilm ingest` all resolve via the shorthand alias.

### Out of scope (v3.2)

- LLM-assisted summary at publish (v3.1 carry-over if not done).
- Doc decay detection — v3.3 (depends on `Doc` entity type landing first).
- Auto-drafted docs from sessions (`--as-doc`) — v3.3.
- Prompt cookbook auto-suggest during enrich — v3.3.
- Cross-repo team-of-teams federation — v3.4.

---

## v3.3 — High-Value Doc Features

**Target window:** 6–8 weeks after v3.2.

**Goal:** Turn the docs corpus into something that compounds — drift detection against the code graph, auto-drafted docs from sessions, prompt cookbook with auto-suggest, and agent-aware doc review.

### In scope

| Component | What ships |
|-----------|-----------|
| Doc decay detection (PR bot) | Watches doc files vs the tree-sitter code graph. When a doc references a symbol that has materially changed or been removed, opens a PR against the team repo flagging stale sections. Wikis can't do this because they don't see the code; code-review tools can't because they don't see the docs. Illuminate sees both. |
| `illuminate publish --as-doc` | Auto-draft a design doc from a session's reasoning trail. Dev reviews + edits + merges. Collapses the cost of writing design docs so they actually get written. |
| Prompt cookbook auto-suggest | Match the dev's prompt against patterns in `docs/prompts/` during `illuminate enrich`. Inject matched cookbook entries as additional context. First team to nail this becomes 2-3× more effective at prompting. |
| Agent-aware doc review | When a doc change is PR'd, audit it against existing decisions/patterns. Same logic as `illuminate audit` on code, applied to doc drift. |
| `docs/prompts/` as a first-class content type | Schema additions, examples, and `illuminate prompts list/show` CLI for curating the team's prompt cookbook. |

### Exit criteria

- Doc decay PR bot catches a stale doc reference on a real repo and opens a PR with the right line ranges flagged.
- `illuminate publish --as-doc docs/designs/<slug>.md` produces a coherent design-doc draft from a real session.
- Prompt-cookbook injection visible in `illuminate enrich --format json` output when the prompt matches a cookbook entry.

---

## v3.4 — Workflow Features

**Target window:** ongoing after v3.3.

**Goal:** Turn the corpus into workflows — onboarding journeys, on-call context bundles, agent skill packs, federation, living diagrams.

### In scope

| Component | What ships |
|-----------|-----------|
| `illuminate onboard` | Personalized onboarding journey for new hires. Walks foundational decisions → patterns → service-specific docs in graph-suggested order. Mark sections read, suggest next. |
| `illuminate oncall <service>` | Context bundle for incidents: relevant docs, recent decisions, past failures, the prompts that produced the failing code. Focused reading list instead of 3 AM wiki grep. |
| `illuminate skill build` | Auto-generate a Claude Code skill pack from the team's docs + decisions + patterns. Drop into `~/.claude/skills/` — agent now knows what your team knows. Updates as the repo evolves. |
| Cross-repo team-of-teams federation | Optional layer: share certain doc categories (security policies, deployment patterns, on-call runbooks) across N team-illuminate repos. Opt-in per repo. Federated, not centralized. |
| Living architecture diagrams | mermaid diagrams in `docs/architecture/*.md` auto-updated against the tree-sitter parse. Diagrams + code stay in sync without manual upkeep. |

### Exit criteria

- A new hire on a real team can run `illuminate onboard` and reach productive contributions in days, not months (measured via case-study).
- `illuminate skill build` produces a skill pack that, when installed, demonstrably improves Claude Code's first-pass accuracy on team-specific tasks.

---

## v3.5 — Polish + Adoption

**Target window:** ongoing alongside v3.3 / v3.4.

**Goal:** Reduce friction for adoption. Make the loop unmissable.

### In scope

| Component | What ships |
|-----------|-----------|
| Self-coaching dashboard | Local-only, dev-owned, never shared upward. "Your last 10 prompts, what enrichment added, what the agent did with it." Pure dev value; aggregation forbidden. |
| Wiki search v2 | Semantic + grep with per-page-type ranking weights. CLI + dashboard. |
| Bootstrap helpers v2 | Slack / Linear / Jira import (optional, opt-in). Additional ADR formats. |
| Editor extensions | Thin wrappers for VS Code / Cursor / Zed that surface `illuminate enrich` + `illuminate publish` + `illuminate ask` inline. Not new surfaces — just convenience. |
| Onboarding wizard | `illuminate init --interactive` walks through agent setup, runs bootstrap, opens the wiki, and runs the enrichment + ask demo. |

### Exit criteria

- 50+ teams using Illuminate organically (GitHub stars + Homebrew downloads + cargo installs).
- One large open-source project running Illuminate with a public team-illuminate/ repo.
- Documented case study: *"Team X reduced AI-coauthored PR rework by N%"* — measured by counting iterations per merged AI-assisted commit before vs after Illuminate.

---

## v3-cloud — Optional Hosted Layer

**Target window:** only after v3.5 hits its exit criteria.

**Goal:** Make money without breaking the local-first promise.

### In scope (when it arrives)

- **Hosted illuminate.sh** — team-repo mirror with faster search and queries. Same graph, different storage. Client-side encryption.
- **Cross-repo decision sharing.** "Apply our company-wide auth policy to all repos." Hosted or self-hosted. Consent flow per repo.
- **Enterprise SSO + audit logs.** SOC2 / GDPR paperwork.
- **Multi-team graph federation.** Team repos can subscribe to a parent org repo for shared decisions.
- **Managed wiki.** Team-shared wiki rendered as a hosted website. (Optional. The markdown stays in git.)

### Hard constraints

- The OSS version must remain fully functional offline. Commercial features are additions, never replacements.
- No commercial feature gates a security-relevant feature. PII stripping, opt-in capture, gitignored trails, the trust-model invariants — all stay free.
- Commercial revenue must come from convenience, scale, and integrations — never from *"we hold your data hostage."*
- The trust model (`docs/trust-model.md`) applies to cloud features unchanged: no individual scoring, no surveillance views, no leaderboards.

---

## Things deliberately not on the roadmap

- **Cloud-only / "AI-native" rewrite.** Local-first is the value proposition. A cloud-only version would lose the buyers in regulated verticals.
- **VS Code extension as primary surface.** The CLI + MCP server stays the primary surface. Extensions are thin wrappers.
- **General-purpose AI assistant.** Illuminate enriches generation. It does not generate.
- **Replacement for git or GitHub.** Illuminate uses git and runs alongside GitHub. No competition.
- **Multi-tenancy in the OSS binary.** Each repo's graph is independent. Multi-tenant aggregation is a v3-cloud feature.
- **Real-time streaming extraction.** v0.x extracts on session end. If real-time turns out to be useful, evidence will say so.
- **Individual surveillance / prompt leaderboards.** Structurally forbidden by the trust model.

---

## Risk register

The most likely things that could derail the v3 roadmap:

| Risk | Mitigation |
|------|-----------|
| Enrichment quality is not visibly better than raw prompts; the wedge fails. | v3.0 validation step: manually construct 10 enriched prompts on the Utkrushta dogfood graph and grade vs raw. If the lift is invisible, fix the pipeline before public launch. |
| Publish-gesture friction makes devs skip curation. | Default to "publish summary on git commit, one-keystroke to opt out." Tune in v3.1 based on dogfood data. |
| Prompt-as-source framing is novel; market dismisses it. | Lead public communication with the enrichment demo (visible quality win) before the philosophy.md manifesto. |
| Cursor + Codex enrich hooks are blocked by host-agent constraints. | v3.0 ships Claude Code only. v3.1 explores hooks; falls back to CLI wrapper across agents if hooks are unavailable. |
| Solo founder bandwidth. | Scope ruthlessly. v3.0 is enrich + publish on top of the existing substrate. Cursor/Codex enrich, hosted cloud, etc., are v3.1+. |
| Spec-kit ecosystem captures the AI-coding-workflow category before Illuminate ships v3.1. | v3.1 explicitly ships `spec-kit-illuminate` extension. Illuminate composes with spec-kit; doesn't compete. |

---

## Versioning policy

- **Semantic versioning** continues. The v3 reset is a positioning marker, not a SemVer reset — releases will continue as `0.19.0`, `0.20.0`, ... up to `1.0.0` when the v3.0 exit criteria are met.
- **Patch bumps** for bug fixes.
- **Minor bumps** for additive features within a release line.
- **Major bump (1.0)** when v3.0 ships and the public CLI + MCP contract stabilizes. After that, breaking changes require a clear migration story.
- The wiki schema (`SCHEMA.md`) follows the same versioning. Schema additions (e.g., `page_type: session` in v3.0) are backwards-compatible; removals require a migration in `illuminate rebuild`.

---

## Success metrics

What we're optimizing for, in order:

1. **Daily-active devs running `illuminate enrich`.** Top-line proxy for "the wedge is working." Counted via CLI usage on the dev's machine; never reported home.
2. **Published sessions per repo per week.** Indicates Stage 4 (curate) is being exercised; the team repo is compounding.
3. **Enrichment hit rate.** Of all `illuminate enrich` calls, what fraction injected at least one relevant decision/pattern/failure? Target: > 50% on a populated repo.
4. **Time-to-first-enrichment for new repos.** From `illuminate init` to the first enrich call that demonstrably changes the prompt. Target: < 1 hour of normal coding.
5. **Self-reported drift reduction + onboarding speed.** Teams report whether agents introduce less drift after Illuminate, and whether new hires reach productivity faster. Qualitative; tracked via case studies.

The first four are observable from the CLI. The fifth is interview-driven. None require server-side telemetry; users opt in by sharing case-study writeups on their own terms.
