# Illuminate — Trust Model

This document is the explicit boundary between what Illuminate captures, what it shares, and what it refuses to build. It is the most important document in this repo for adoption. Developers will read it before installing.

> **The one-sentence version:** Everything stays local until you explicitly publish it; some things are never built no matter what a customer asks for.

For the product framing this trust model serves, see [`PRODUCT_OVERVIEW.md`](PRODUCT_OVERVIEW.md). For the manifesto, see [`philosophy.md`](philosophy.md). For the technical implementation of the local-first guarantees, see [`PRIVACY.md`](PRIVACY.md) and [`ARCHITECTURE.md`](ARCHITECTURE.md).

---

## Why this document exists

A tool that captures every prompt a developer types is, structurally, surveillance software *unless its trust model makes surveillance impossible.* That's the bar.

We've watched enough developer tools start with *"local-first, privacy-respecting"* and end with *"we collect anonymized telemetry to improve the product"* to know that promises in marketing copy are worth nothing. The trust model has to be **enforced by the architecture**, not by intentions.

This document specifies the enforcement: which data lives where, which network boundaries can ever be crossed, which features will never exist regardless of revenue impact.

If you find a behavior in Illuminate that contradicts this document, that is a bug, not a feature. File it.

---

## The three rings

Every piece of data Illuminate touches falls into one of three concentric rings. The ring determines what can happen to it.

```
        ┌─────────────────────────────────────────────────┐
        │  RING 1 — LOCAL                                 │
        │  (dev machine, gitignored, never auto-uploaded) │
        │                                                 │
        │   .illuminate/trail/        (raw sessions)      │
        │   .illuminate/graph.db      (local cache)       │
        │   .illuminate/index.db      (code graph)        │
        │   enrich-time queries       (local-only)        │
        │   audit-time queries        (local-only)        │
        │                                                 │
        │  ┌───────────────────────────────────────────┐  │
        │  │  RING 2 — TEAM (only with explicit       │  │
        │  │            publish gesture)              │  │
        │  │                                          │  │
        │  │   .illuminate/wiki/      (in git)         │  │
        │  │   illuminate.toml        (in git)         │  │
        │  │   team-illuminate/sessions/  (published   │  │
        │  │     prompts — opt-in per session)         │  │
        │  │                                          │  │
        │  │  ┌────────────────────────────────────┐  │  │
        │  │  │  RING 3 — INTERNET                 │  │  │
        │  │  │  (only if v3-cloud configured,    │  │  │
        │  │  │   only with team consent, only    │  │  │
        │  │  │   client-side-encrypted payloads) │  │  │
        │  │  │                                    │  │  │
        │  │  │   illuminate.sh hosted mirror      │  │  │
        │  │  │   cross-repo federation            │  │  │
        │  │  │   (v3-cloud, ships ≥ 12mo out)     │  │  │
        │  │  └────────────────────────────────────┘  │  │
        │  └───────────────────────────────────────────┘  │
        └─────────────────────────────────────────────────┘
```

**Ring 1 (Local)** is the default. Everything starts here. Data in Ring 1 never crosses a network boundary without an explicit user gesture.

**Ring 2 (Team)** is opt-in per session. The dev runs `illuminate publish` (or accepts the pre-commit hook prompt) to move a specific session from Ring 1 to Ring 2. There is no batch upload, no auto-publish, no "we'll move yesterday's sessions tomorrow."

**Ring 3 (Internet)** is opt-in per team and per feature. The OSS version of Illuminate never reaches Ring 3. v3-cloud features that touch Ring 3 require a paired `illuminate.toml` consent flag and a paired `illuminate trust check` pass.

---

## What stays local, always (Ring 1)

The following data **never** leaves the developer's machine without an explicit gesture, in any release of Illuminate, ever.

### Captured trail data

- `.illuminate/trail/*.jsonl` — raw normalized session transcripts. One file per Claude Code / Cursor / Codex session.
- Half-formed prompts, dead-end iterations, debugging spirals, embarrassing typos.
- Files-touched lists from sessions that the dev later discarded.
- Tool invocation logs (which MCP tools were called, when, with what arguments).
- Token counts (input/output) per session — these are accounting data; they stay local.

### Local indexes

- `.illuminate/graph.db` — the bi-temporal decision graph. Local cache, regeneratable.
- `.illuminate/index.db` — tree-sitter code graph. Local cache, regeneratable.
- Embedding vectors for any of the above. Embeddings are derived from local data and stay local.

### Enrich-time and audit-time queries

When `illuminate-enrich` (planned, v3.0) queries the graph to enrich a prompt, the query runs entirely against `.illuminate/graph.db`. No network call. Same for `illuminate-audit`, `illuminate explain`, `illuminate impact`, `illuminate search`, `illuminate decisions for <path>`.

### Personal scratch

- `~/.illuminate/personal/` (planned, v3.2) — dev-owned coaching dashboards, "your last 10 prompts" views. Never aggregated, never shared upward, never an admin-visible artifact.

### Failure transcripts

When `illuminate failure log` records a failure, the structured fields (title, root cause, fix) write to the wiki (Ring 2 candidate, but only if the dev commits the wiki page). The full unredacted incident transcript or post-mortem stays in Ring 1 unless the dev explicitly attaches it.

---

## What gets published, only with explicit consent (Ring 2)

Movement from Ring 1 to Ring 2 requires an explicit dev gesture every time. No batch, no schedule, no "we'll catch up tomorrow."

### The gesture

```bash
# Explicit:
illuminate publish

# Or via the pre-commit hook (asks before writing):
$ git commit -m "add LRU caching"
Publish this session's reasoning to the team repo?
  [F]ull session  [S]ummary  [D]ecision-only  [X] discard
```

The dev's choice is recorded in the resulting page's frontmatter (`source.redaction: full | summary | decision`). Subsequent readers can see what level was chosen.

### The four redaction levels

- **Full session.** Entire session transcript, prompts and responses, written as a `page_type: session` markdown page. Used when the reasoning is itself the point.
- **Summary.** A structured summary (prompt intent, alternatives considered, decision made, code change). Default for most publishes.
- **Decision-only.** Just the decision page (or pattern / failure page). Equivalent to today's `illuminate failure log` or wiki quick-add.
- **Discard.** Nothing is published. The session stays in Ring 1.

### Where it lands

- `.illuminate/wiki/` — already shipped, in git. Decisions, patterns, failures, modules.
- `team-illuminate/sessions/` (v3.0+) — published session pages. Same git repo or a sibling.
- `illuminate.toml` — config. In git, team-shared.

### What is *not* implicit

- No "publish everything from this week" command. Every gesture is per-session.
- No telemetry packaged with publishes. Just the markdown the dev approved.
- No anonymized aggregation. If a team wants aggregate metrics, they build their own queries against Ring 2 data they've chosen to publish.

---

## What's never built

These features will not exist. Period. No matter how much a customer is willing to pay.

### No individual scoring

Illuminate will never produce a dashboard, report, leaderboard, or aggregate that rates an individual developer's prompts. Not for managers, not for the dev themselves *aggregated upward*, not for HR, not for "self-coaching" that's actually visible to anyone else.

The dev's local `~/.illuminate/personal/` (v3.2) is the closest analog. It is **dev-owned**, **dev-private**, and **structurally cannot be aggregated** without the dev exporting and sharing it themselves.

### No management surveillance views

There will never be an "admin dashboard" view that shows what a team is prompting, how often, with what success rate, on which files, with which agents. No "team activity" view. No "engineering productivity" panel.

Aggregate team-level trends are possible *only* if a team explicitly opts in AND the data is anonymized at the source (no per-dev attribution at any layer of the pipeline) AND the dashboard does not surface anything that could deanonymize.

### No prompt leaderboards

No "top prompters this week." No "prompt-quality scores." No gamification of prompting. These corrode the trust model on contact.

### No silent telemetry

The Illuminate binary never phones home. Not for usage stats, not for crash reports, not for "anonymous performance metrics," not for "update notifications." If you want to know whether there's a new release, you run `illuminate update --check` (which is also opt-in).

If a future feature requires a remote check, it ships in v3-cloud, behind an explicit `illuminate.toml` consent flag, and is documented in this file before it lands.

### No auto-upload

Even in v3-cloud, no data crosses Ring 2 → Ring 3 without:

1. An explicit `[cloud.sync] enabled = true` flag in `illuminate.toml`, AND
2. A first-use prompt that requires interactive confirmation, AND
3. Client-side encryption of any payload before the network call, AND
4. A `illuminate trust check` pass that confirms the team understands what's being shared.

### No vendor lock-in via the graph

The team graph and team repo are standard git + markdown. If a team uninstalls Illuminate, they keep everything they've published. There is no proprietary format, no export tool needed, no "you can leave but you can't take your data with you." The trust model includes the freedom to walk away.

---

## How the architecture enforces this

Promises in marketing copy are worth nothing. These commitments are enforced by the architecture:

- **`illuminate-publish` is the only crate that can write outside `.illuminate/`.** Every other crate writes to local paths only. A `grep -r 'NetworkTarget' crates/` returns hits only in `illuminate-publish`. A grep for any HTTP client outside `illuminate-mcp` (HTTP transport for inbound MCP only — never outbound) returns nothing.
- **`.gitignore` ships with every `illuminate init`.** `.illuminate/trail/`, `.illuminate/graph.db`, `.illuminate/index.db`, `~/.illuminate/personal/` are gitignored by default. `illuminate trust check` lints for any project that has removed these ignores.
- **MCP transport is inbound-only.** The MCP HTTP server (`illuminate mcp serve --http`) listens for tool calls from local agents. It never originates outbound calls.
- **LLM fallback (ingestion) is opt-in and PII-stripped.** `[extraction.llm]` defaults to `provider = "none"`. When configured, payloads run through `cloakpipe` (optional Cargo feature) before leaving the machine. The set of providers (Anthropic, OpenAI, OpenRouter, local) is in code and auditable.
- **Determinism receipts.** `illuminate enrich` (planned, v3.0) returns a `graph_state_hash` in its response. Same input + same hash → same output. This makes the enrich behavior reproducible and inspectable.
- **No SDK or runtime injection.** Illuminate is a CLI + an MCP server. It does not modify the agent's process, install a kernel extension, or proxy network calls without consent.

If any of these change, this document must change first.

---

## Specifically for buyers in regulated verticals

Teams at Harvey, Abridge, Hippocratic AI, financial institutions, healthcare systems, and similar contexts have asked specifically:

| Question | Answer |
|---|---|
| Can we run Illuminate fully offline? | Yes. Set `[extraction.llm] provider = "none"`. No network calls in any path. |
| Does Illuminate phone home? | No. Ever. |
| Can our compliance team audit the binary's network behavior? | Yes. The binary is open source. `strace -f -e network illuminate <cmd>` shows you every syscall. |
| Can we self-host the team repo? | Yes. The team repo is just a git repo on your own infrastructure. No SaaS required. |
| Can we disable specific MCP tools? | Yes. The MCP server respects an `[mcp.tools.enabled]` allow-list. |
| Can devs opt out per-repo? | Yes. Repos without an `illuminate.toml` are skipped by the trail watcher (`[trail] enabled` defaults true only when the toml exists). |
| If we run v3-cloud, where does data sit? | Wherever you configure. Self-hosted Postgres + S3 for the mirror, or our hosted illuminate.sh. Client-side encryption either way. |
| What does a SOC2 audit look like? | The OSS binary's data residency story is already SOC2-friendly. v3-cloud will ship with SOC2 Type II before it goes GA. |

If your compliance team needs anything else documented, file an issue and we'll add it here.

---

## What you should do before installing

Spend ten minutes verifying the trust model is what you think it is:

```bash
# 1. Install
cargo install --git https://github.com/rohansx/illuminate illuminate-cli --locked

# 2. Initialize in a throwaway dir
mkdir /tmp/illuminate-trust-test && cd /tmp/illuminate-trust-test
git init && illuminate init -n trust-test

# 3. Verify the .gitignore is sensible
cat .gitignore | grep illuminate
# expect: .illuminate/graph.db
#         .illuminate/index.db
#         .illuminate/trail/

# 4. Watch the binary's network calls
strace -f -e network illuminate audit "test plan" 2>&1 | grep -E '(connect|sendto)'
# expect: nothing (no outbound calls)

# 5. Read the file system writes
strace -f -e openat illuminate audit "test plan" 2>&1 | grep -v '/proc' | grep -v '/sys'
# expect: only paths under the current dir, the binary's install dir, and ~/.cargo /.cache

# 6. Lint your real config
cd ~/your-real-project
illuminate trust check
# expect: pass (or specific warnings about anything you've configured)
```

If any of these checks fail or surprise you, the trust model has a bug. File it.

---

## Versioning this document

This document is **versioned with releases**. Any change to it requires:

- A CHANGELOG entry describing the change.
- A migration note if the change tightens trust (loosening trust requires a deprecation cycle).
- The change must be reviewed by at least one external contributor before merge.

The current version of this trust model applies to: `illuminate >= 0.18.0`. The v3.0 release will extend it (Ring 3 + v3-cloud) but cannot relax Ring 1 or Ring 2 commitments.

---

## Why this matters more than the product

The compounding-knowledge thesis (`philosophy.md`) only works if devs install Illuminate. Devs only install it if they trust it. They only trust it if the architecture makes betrayal impossible.

This document is the contract. If we ever break it, the product is over — not because of marketing damage, but because the structural commitment is what made the product possible in the first place.
