# Illuminate — Roadmap

> **Status:** v0.18 shipped (May 2026). The substrate is real and end-to-end. This document is a **v3 reset** — the work that already shipped is collapsed into a single Shipped section; the next-cycle plan (v3.0 → v3.2 → v3-cloud) targets the two-product positioning described in [`PRODUCT_OVERVIEW.md`](PRODUCT_OVERVIEW.md). See [`CHANGELOG.md`](../CHANGELOG.md) for the per-version log.

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
- **No commercial layer until OSS adoption is proven.** v3-cloud is off the table until v3.0 + v3.1 have 50+ teams using them organically.
- **Evidence before scope.** Decisions about v3.1 scope are deferred until v3.0 is in users' hands.
- **Local-first is non-negotiable.** Every release must work fully offline. Cloud features are additions, never replacements.

---

## v3.0 — The Enrich Wedge

**Target window:** 6–8 weeks from kickoff.

**Goal:** Turn the v0.18 substrate into the two-product positioning. Ship `illuminate-enrich` (Product 1's wedge) and `illuminate-publish` (Product 2's curate gesture). Both products demonstrably in motion in a single 60-second demo video.

### In scope

| Component | What ships |
|-----------|-----------|
| `illuminate-enrich` (new) | Pre-LLM prompt enrichment crate. CLI wrapper mode (`illuminate enrich -- claude code`). Queries `illuminate-route` for a reading plan; rewrites the prompt deterministically with relevant decisions/patterns/failures and code-graph paths. No LLM in the enrich path. |
| `illuminate-publish` (new) | Explicit publish gesture. `illuminate publish` CLI verb + pre-commit hook installer. Redaction-level chooser (full / summary / decision-only / discard). Writes structured markdown + json sidecar to `team-illuminate/sessions/<date>-<slug>.md`. |
| `illuminate browse` | Terminal UI over published sessions. Search, blame ("who prompted this code?"), open original session jsonl in `$EDITOR`. |
| `illuminate trust check` | Lints `illuminate.toml` for any config that would route data off-host without consent. Surfaces the trust-model invariants. |
| Trust-model docs | `docs/trust-model.md` lands with the v3 reset (see [`trust-model.md`](trust-model.md)). |
| Schema extension | `page_type: session` added to `SCHEMA.md`. Existing decision/pattern/failure/module schemas unchanged. |
| Enrichment demo artifact | One 60-second video: raw prompt → enriched prompt → noticeably-better generation. Single most important launch artifact. |

### Out of scope (v3.0)

- Cursor / Codex *enrich* hooks. Claude Code only for enrichment in v3.0 (capture already works for all three).
- LLM-assisted summary at publish time (use a deterministic template).
- Hosted cloud / illuminate.sh mirror.
- Cross-repo decision sharing.
- VS Code / Zed editor extensions.
- Pre-write enrich hook (CLI wrapper only in v3.0).

### Exit criteria

- `illuminate enrich -- claude code "<prompt>"` returns an enriched prompt that includes at least one decision, pattern, or failure reference on a populated repo. Verified end-to-end against the Utkrushta dogfood graph.
- `illuminate publish` invoked from a pre-commit hook writes a valid markdown + json pair to the configured team-repo path. Round-trip readable by `illuminate browse`.
- `illuminate trust check` returns 0 on a default `illuminate.toml` and non-zero when a misconfigured `TeamRepoTarget::GitRemote` is present without a paired consent flag.
- Determinism guarantee for enrich: same `(prompt, graph_state_hash)` → byte-identical output. Verified by property test.
- Single-binary install still works (`cargo install illuminate-cli`); no new system dependencies.

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

## v3.2 — Polish + Adoption

**Target window:** ongoing after v3.1.

**Goal:** Reduce friction for adoption. Make enrichment unmissable.

### In scope

| Component | What ships |
|-----------|-----------|
| Self-coaching dashboard | Local-only, dev-owned, never shared upward. "Your last 10 prompts, what enrichment added, what the agent did with it." Pure dev value; aggregation forbidden. |
| Wiki search v2 | Semantic + grep with per-page-type ranking weights. CLI + dashboard. |
| Bootstrap helpers v2 | spec-kit constitutions, AGENTS.md variants, additional ADR formats. Slack / Linear / Jira import (optional, opt-in). |
| Editor extensions | Thin wrappers for VS Code / Cursor / Zed that surface `illuminate enrich` + `illuminate publish` inline. Not new surfaces — just convenience. |
| Onboarding wizard | `illuminate init --interactive` walks through agent setup, runs bootstrap, opens the wiki, and runs the enrichment demo. |

### Exit criteria

- 50+ teams using Illuminate organically (GitHub stars + Homebrew downloads + cargo installs).
- One large open-source project running Illuminate with a public team-illuminate/ repo.
- Documented case study: *"Team X reduced AI-coauthored PR rework by N%"* — measured by counting iterations per merged AI-assisted commit before vs after Illuminate.

---

## v3-cloud — Optional Hosted Layer

**Target window:** only after v3.2 hits its exit criteria.

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
