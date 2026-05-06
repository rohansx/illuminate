# Illuminate — Roadmap

> **Status: v0.1 closed loop shipped** (May 2026)

This roadmap describes what ships in each release and what is deliberately deferred. The goal is to ship the closed loop in v0.1 and let evidence guide the rest.

For the loop itself, see `PRODUCT_OVERVIEW.md`. For component detail, see `ARCHITECTURE.md`.

---

## Release philosophy

- **Ship the loop, then expand it.** The first release shows capture → graph → audit → wiki → reflect → graph. Anything that doesn't tighten that loop is deferred.
- **Each release should be valuable on its own.** No "you need v0.2 to use v0.1" gates. v0.1 is a complete product, just narrower than the eventual product.
- **No commercial layer until OSS adoption is proven.** v0.4+ commercial features are off the table until 50+ teams use the OSS version organically.
- **Evidence before scope.** Decisions about what to build in v0.2 are deferred until v0.1 is in users' hands.

---

## v0.1 — Closed loop, narrow scope — Shipped (May 2026)

**Target window:** 8–10 weeks from kickoff. Shipped on schedule.

**Goal:** Demonstrate the full loop on a single agent (Claude Code) with a single repo. Useful enough that a dev who installs it would still use it a month later.

### In scope

| Component | What ships |
|-----------|-----------|
| `illuminate-core` | Graph API on top of `ctxgraph`. Entity types, relations, query helpers. |
| `illuminate-trail` | Claude Code session capture only. Watches `~/.claude/projects/`. Writes normalized jsonl to `.illuminate/trail/`. |
| `illuminate-extract` | Local NER pipeline (GLiNER + GLiREL + embeddings). LLM fallback wired but optional. |
| `illuminate-embed` | all-MiniLM-L6-v2 ONNX embeddings. |
| `illuminate-index` | Tree-sitter file/symbol indexer. Rust + TypeScript + Python at minimum. |
| `illuminate-audit` | Policy engine. Reads `illuminate.toml` + queries graph. Returns structured findings. |
| `illuminate-mcp` | JSON-RPC server, exposes `illuminate_audit`, `illuminate_explain`, `illuminate_search`. |
| `illuminate-cli` | `init`, `audit`, `explain`, `wiki rebuild`, `wiki serve`, `bootstrap`, `failure log`, `status`, `stats`. |
| Wiki rendering | Graph → markdown wiki (auto-generated index.md, log.md). Markdown is source-of-truth. |
| Bootstrap | Sources 1–4 in `BOOTSTRAP.md`: agent files, ADRs, git history, README. Source 5 (interview) optional flag. |
| `CLAUDE.md` directive | `illuminate init --claude` writes the audit-pre-write directive. |

### Out of scope (v0.1)

- Cursor and Codex hooks. Claude Code is the only agent in v0.1.
- Pre-write OS-level hooks. v0.1 relies on the `CLAUDE.md` directive + MCP. Hook integration is v0.3.
- Reflect ingester from CI logs / Sentry. Manual `illuminate failure log` only.
- Wiki PR-review flow for low-confidence pages. v0.1 logs them in a queue and surfaces via CLI; no GitHub integration.
- Cross-repo sharing of any kind.
- Web UI. Markdown is plenty.
- Analytics, dashboards, telemetry.
- Auth / RBAC / cloud sync.
- LLM-classified distill (auto-classification of trail content into decisions). v0.1 is dev-triggered only.

### Exit criteria — Met

- `illuminate bootstrap` reads CLAUDE.md and agent files, writes wiki pages, and reports `pages written` within seconds on a fresh repo. Verified by end-to-end integration test.
- `illuminate wiki rebuild` walks the wiki, registers episodes in the graph, and regenerates `index.md`. Verified by integration test.
- `illuminate audit "<plan>"` exits 2 on policy violations (e.g. rejected Redis when policy disallows it) and exits 0 on clean plans. Verified by integration test (`end_to_end_audit`).
- `illuminate wiki serve` renders the wiki as HTML on localhost.
- Single binary (`cargo install illuminate-cli`) installs all of the above. No extra services, no Docker, no Python.
- Audit and wiki work fully offline; no LLM provider required for the core loop.

### Distribution

- `cargo install --git github.com/rohansx/illuminate illuminate-cli`
- Homebrew tap: `brew install rohansx/tap/illuminate`
- Prebuilt binaries for x86_64-linux, aarch64-linux, x86_64-darwin, aarch64-darwin via GitHub releases.

### Single launch artifact

A short HN post at v0.1 ship-time. One repo, one binary, one demo video.

### Shipped commits (v0.1)

- Trail capture: claude-code session jsonl → `.illuminate/trail/` (commits `8b9e4e8` through `7ac031e`)
- Wiki layer: page parser, linter, walker, scaffold, render, episode mapping (`0d5df95` through `c0b0148`)
- Bootstrap pipeline: agent files + ADRs → wiki pages + graph episodes (`2598d8a` through `c238b35`)
- Audit integration: ancestor-walk policy loader, trail register, CLAUDE.md directive (`51a07e3`, `3740eaf`, `9238683`)
- Wiki HTTP serve: `illuminate wiki serve` on localhost (`9c78df0`)

---

## v0.2 — Broaden capture

**Target window:** 4–6 weeks after v0.1 ships.

**Goal:** Make the loop work for teams using Cursor or Codex. Add automation around failures.

### In scope

| Component | What ships |
|-----------|-----------|
| `illuminate-trail` (extended) | Cursor session capture (file watch + format normalizer). Codex session capture. |
| `illuminate-reflect` (extended) | Optional CI log parsers. Sentry/PagerDuty webhook receivers. Failure auto-promotion from incident systems. |
| Auto-distill | LLM-classified distillation of trail content into decision candidates. Dev reviews via `illuminate wiki review`. |
| Per-repo policies | Richer `illuminate.toml` policy DSL. Path-scoped policies, time-bounded policies, cross-policy exclusions. |
| Wiki review queue | CLI flow to walk through low-confidence candidates and accept/reject. |

### Exit criteria

- A team using Cursor as primary agent can install Illuminate and capture sessions to the same `.illuminate/trail/` format.
- A failure logged in Sentry produces a wiki page (with optional human review) and surfaces as a warning in subsequent audits.
- The auto-distill pass produces ≥ 70% precision on captured trails (measured manually on a test set).

### Out of scope (still)

- Pre-write hooks. Still v0.3.
- Cloud anything.
- Multi-repo views.

---

## v0.3 — Polish + adoption

**Target window:** ongoing after v0.2.

**Goal:** Reduce friction for adoption. Make the audit unmissable.

### In scope

| Component | What ships |
|-----------|-----------|
| Pre-write hook | `PreToolUse` hook for Claude Code. Synchronous audit on Write/Edit. Bypassable but logged. |
| GitHub Action | `illuminate audit-pr` action. Comments findings on PRs, fails the check on `error`-severity violations. |
| Wiki search | Semantic + grep search over the wiki. CLI + (optional) local web UI. |
| Bootstrap helpers v2 | Better ADR parser (more formats). Slack history import (optional). Linear/Jira import (optional). |
| Onboarding flow | First-run wizard. `illuminate init` in interactive mode walks through agent setup, runs bootstrap, opens the wiki. |

### Exit criteria

- 50+ teams using Illuminate organically (via GitHub stars + telemetry-free signals like Homebrew downloads, cargo installs).
- One large open-source project running Illuminate with public wiki/.
- Documented case study: "Team X reduced AI-coauthored PR rework by N%" (or whatever the measured metric is).

---

## v0.4+ — Commercial layer

**Target window:** only after v0.3 hits its exit criteria.

**Goal:** Make money without breaking the local-first promise.

### In scope (when it arrives)

- **Hosted graph mirror.** Optional opt-in. Team's graph synced to Illuminate Cloud for cross-machine access. Same graph, different storage. Encryption client-side.
- **Team dashboards.** Audit metrics over time, decision-velocity, drift trends. Aggregate-only — no plaintext episodes leave the team's environment.
- **Cross-repo decision sharing.** "Apply our company-wide auth policy to all repos." Hosted or self-hosted.
- **Enterprise auth + RBAC.** SSO, audit logs of audit calls (meta), GDPR/SOC2 paperwork.
- **Managed wiki.** Team-shared wiki rendered as a hosted website. (Optional. The markdown stays in git.)

### Hard constraints

- The OSS version must remain fully functional offline. Commercial features are additions, never replacements.
- No commercial feature gates a security-relevant feature. PII stripping, opt-in capture, gitignored trails — all stay free.
- Commercial revenue must come from convenience, scale, and integrations — never from "we hold your data hostage."

---

## Things deliberately not on the roadmap

- **Cloud-only / "AI-native" rewrite.** Local-first is the value proposition. A cloud-only version would lose the buyers in regulated verticals.
- **VS Code extension as primary surface.** The CLI + MCP server is the primary surface. An extension is a thin wrapper, not a roadmap item.
- **General-purpose AI assistant.** Illuminate guards generation. It does not generate.
- **Replacement for git or GitHub.** Illuminate uses git and runs alongside GitHub. No competition.
- **Multi-tenancy in the OSS binary.** Each repo's graph is independent. Multi-tenant aggregation is a v0.4+ commercial feature.
- **Real-time streaming extraction.** v0.1 extracts on session end. If real-time turns out to be useful, evidence will say so. Until then, end-of-session is fine.

---

## Risk register

The most likely things that could derail the roadmap:

| Risk | Mitigation |
|------|-----------|
| Cold-start UX is bad enough that devs uninstall before bootstrap completes. | Bootstrap targets < 5 minutes. Smoke-test in `illuminate init` reports first-finding visibility. |
| Agents don't reliably call `illuminate_audit`. | v0.3 ships pre-write hook. CI gate as backup. |
| Local NER quality is too low; LLM fallback dominates. | Tune signal scorer aggressively. Fall back to dev-triggered distill if auto isn't reliable. |
| Karpathy LLM Wiki copycats commoditize the wiki layer. | Lead with the linter, not the wiki. Wiki is differentiation by integration, not by features. |
| Solo founder bandwidth. | Scope ruthlessly. v0.1 is the bar. Defer everything else. |

Each risk is tracked in `wiki/decisions/` once the project is dogfooding itself.

---

## Versioning policy

- `0.1.0` ships when v0.1 exit criteria are met.
- Patch bumps for bug fixes (`0.1.1`, `0.1.2`, ...).
- Minor bumps for additive features within a release line (`0.1.0` → `0.1.1` → ... → `0.2.0`).
- Major bump only if the public CLI or MCP contract changes incompatibly. Avoid this for as long as possible.
- The wiki schema (`SCHEMA.md`) follows the same versioning. Schema changes get migration scripts in `illuminate rebuild`.

---

## Success metrics

What we're optimizing for, in order:

1. **Daily-active devs running `illuminate audit`.** This is the top-line proxy for "the loop is working." Counted via CLI usage on the dev's machine; never reported home.
2. **Wiki page count growth per repo.** Indicates the graph is fed and the wiki is populated.
3. **Audit hit rate.** Of all `illuminate_audit` calls, what fraction return at least one finding? Target: > 30% on a populated repo.
4. **Time-to-first-finding for new repos.** From `illuminate init` to the first audit call that returns a non-trivial finding. Target: < 1 hour of normal coding.
5. **Self-reported drift reduction.** Teams report whether they feel agents introduce less drift after using Illuminate. Qualitative; tracked via case studies, not metrics.

The first four are observable from the CLI. The fifth is interview-driven. None of them require server-side telemetry; users opt in to share metrics by sharing case-study writeups on their own terms.
