# Prime Agent — evaluation for illuminate

**Date:** 2026-08-06 · against `PrimeIntellect-ai/prime-agent` @ `master` (shallow clone)
· illuminate workspace v0.31.0

Question asked: *can we use prime-agent's RLM part and its continual-harness part, or build
something self-improving out of them?*

Short answer: **the RLM half is a non-starter for illuminate; the harness half contributes
three mechanics worth porting, and those three close a real hole — illuminate today has no
feedback on whether its own guidance worked.** The concrete proposal is in §5.

---

## 1. What prime-agent actually is

A Bun/TypeScript monorepo (a fork of [`pi`](https://github.com/earendil-works/pi), MIT) plus a
small Python runtime package. Four workspace packages — `ai` (provider layer), `tui`, `agent`,
`coding-agent` — and `prime-agent-runtime/src/rlm/` (~1,500 lines of Python).

It is built on two abstractions, and they are much less coupled than the blog post implies.

### 1.1 RLM — the runtime

`packages/coding-agent/docs/rlm.md`. The claim is "context as variables, tools as function
calls." Mechanically:

- The model gets **one built-in tool**: `ipython`. Files, shell, skills, delegation all happen
  as Python inside a persistent kernel. Kernel state survives tool calls *and* compaction.
- `rlm("task", name=...)` spawns a real child `AgentSession`. It returns an **admission
  handle** immediately (`rlm_child_id`, `name`, `session_dir`, `model`) — never the child's
  answer. Results come back only via `agent_message.send(..., receiver_role="parent")` or
  files. The child registry survives compaction and kernel restart.
- Skills are importable Python packages with a `SKILL.md` for routing; only metadata goes in
  the startup prompt.
- Provider calls, credentials, transcripts, scheduling, and policy stay in the TypeScript host.
  Python reaches them through `rlm.host_request(...)`.

This is a **coding-agent host**. It is the layer illuminate deliberately sits *underneath*.

### 1.2 Continual harness — the durable state

`packages/coding-agent/src/core/refinement/refinement.ts` (1,017 lines) plus
`prime-agent-runtime/src/rlm/harness.py` (819 lines, the kernel-side mirror).

State is four kinds of entry — `prompt` (supplemental notes only; the base system prompt is
immutable), `memory` (facts/decisions/failures/preferences), `skill` (Python-backed, must carry
a `reference` + `arguments` contract), `subagent` (reusable delegation specs) — stored as JSON
under `harness/harness_state.json`, in two scopes: **local** (session dir, the default) and
**global** (`~/.prime/agent`, cross-session).

`/refine` is the improvement pass:

1. Feed the trajectory + current harness overview + prior refinement history to a model.
2. It emits JSON only: `{summary, rationale, expectedOutcome, edits[]}` where each edit is
   `{action: create|update|delete, kind, id, title, content, path, reference, arguments,
   metadata, reason}`.
3. `applyRefinementProposal()` validates each edit, snapshots `before`, writes `after`, bumps
   `version`, and appends a `RefinementEvent {trigger, changes[], evidence, outcome}`.
4. `rollbackProposal()` mechanically inverts an applied result — every create becomes a delete,
   every update restores its `before`. Rollback is derived, not re-planned.

Two details that matter more than the headline feature:

- **A cheap review gate guards the expensive pass.** `AUTO_REFINE_REVIEW_SYSTEM_PROMPT` runs a
  4k-token model call at turn intervals and at compaction, answering only
  `{shouldRefine, rationale, instructions}`. Auto-refine is rejected for "one-off noise,
  unsupported hypotheses, and transient tool outputs."
- **Promotion local→global is a written policy, not a vibe.** Project-specific lessons may go
  global "only when the title, path, or content explicitly names the project/workspace and the
  lesson is likely to be reused."

---

## 2. Verdict on the RLM part: do not adopt

Not a close call, for four independent reasons.

1. **It is a competing product category.** Illuminate is substrate under Claude Code, Cursor,
   and Codex. Prime Agent *is* one of those. Adopting its runtime would mean illuminate becomes
   an agent host and loses the "works with whatever your team already runs" position.
2. **It breaks the deployment promise exactly the way Redis would.** RLM requires a Bun host, a
   Python 3 environment, an IPython kernel, a daemon, and worker processes. `CLAUDE.md`'s
   never-use-Redis decision is grounded in "single binary, single SQLite file"; an IPython
   kernel is a *larger* violation than Redis, not a smaller one.
3. **Determinism.** Illuminate's moat per `STATUS_AND_PLAN.md` §1 is that no LLM sits in the
   audit or query path — you cannot build a CI merge gate on a system that answers differently
   on Tuesday. The RLM loop is model-driven end to end.
4. **The value doesn't transfer.** Prompt-as-a-variable and programmatic subagents solve
   *context economics inside one long agent run*. Illuminate's problem is *durable team
   knowledge across runs, agents, and people*. Different axis.

One narrow idea is worth keeping in the back pocket, and it is not RLM: `illuminate-compress`
already does statistical JSON compaction. If audit/enrich responses ever get large enough to
matter, the RLM insight is "hand the agent a *handle* to the big result, not the result." That
is a `illuminate-compress` follow-up (the CCR retrieve-by-hash layer explicitly not ported),
not a reason to import a runtime.

---

## 3. Verdict on the harness part: port the mechanics, not the code

The code is ~1,800 lines of filesystem-JSON plumbing and LLM-call orchestration in two
languages. There is nothing to vendor. Three *mechanics* are genuinely worth having, and each
maps onto something illuminate already half-has.

### 3.1 The versioned, reversible, evidence-attributed edit record

Illuminate's `ReflexionEpisode` (`crates/illuminate-reflect/src/lib.rs`) records
`{failure, root_cause, corrective_action, files_affected, severity, recorded_at}`. That is the
*lesson*. It records nothing about whether the lesson was ever right.

The harness record adds exactly the missing fields: `evidence` (what in the trajectory
justifies this), `expectedOutcome` (what should improve and how to validate it), `before`/`after`
snapshots, a monotonic `version`, and `rollbackOf`. `rollbackProposal()` is ~30 lines and ports
1:1 to Rust.

Illuminate's wiki decisions and `.rhai` policies have no change log at all today — they are
edited as files. A decision that turned out wrong is indistinguishable from one that was always
right.

### 3.2 The scope ladder with an explicit promotion rule

Prime Agent: **local (session)** → **global (cross-session)**, promote only on a written test.

Illuminate already has a *three*-tier ladder — `.illuminate/trail/` (gitignored, per-session) →
`.illuminate/wiki/` (committed, per-repo) → the team repo via `illuminate publish` / `sync` /
`verify`. What it does not have is a stated rule for **when a session observation earns
promotion to a repo decision, or a repo decision to a team decision**. Prime Agent's rule
("names the project explicitly *and* is likely to recur") is the right shape and drops straight
into `docs/trust-model.md`.

### 3.3 The cheap gate in front of the expensive pass

`AUTO_REFINE_REVIEW_SYSTEM_PROMPT` exists so the 32k-token refinement doesn't run on every
turn. Illuminate's analogue is the ~30% LLM fallback in extraction: gate it on a deterministic
signal (did this session's trail contain a novel entity, a failed command, or an audit finding
that was overridden?) rather than a fixed rate.

---

## 4. The actual hole this exposes

Working through the harness makes illuminate's real gap obvious, and it is not "we need
memory" — illuminate has more memory than prime-agent does.

**Illuminate never learns whether its own guidance worked.**

Trace the current loop:

- `Auditor::audit()` emits findings carrying `trace_id`, `confidence`, `decision_ref`,
  `evidence`, `wiki_url` (`crates/illuminate-audit/src/response.rs`).
- The `trace_id` is minted and **thrown away**. No `audit_runs` table exists in `graph.db`.
- `ReflexionStore::find_relevant()` ranks by FTS5 match + file-path match only. A rule that has
  fired 40 times and been dismissed 39 times ranks identically to one that saved the team
  twice.
- `audit_with_reflexions()` upgrades `Pass → Warning` whenever *any* relevant reflexion exists —
  regardless of whether that reflexion has ever predicted a real failure.
- `illuminate failure log` records a failure, but nothing links it back to the audit that ran
  before it and did (or did not) warn about it.

So: illuminate accumulates guidance monotonically, and every piece of it keeps full authority
forever. That is precisely the failure mode that trains teams to ignore a linter.

Prime Agent's harness has the opposite problem — it self-edits freely but has no deterministic
notion of whether an edit helped. Neither system closes the loop. Illuminate is the one that
*can* close it deterministically, because it already has commits, CI output, the code graph,
and `failure log` on the same substrate.

---

## 5. Proposal — `illuminate-refine`: the outcome loop

A self-improving **ranker and demoter**, not a self-improving rule author. Deterministic, no
LLM in the path, two new tables, no new infrastructure.

### 5.1 Persist the audit (prerequisite, small)

New `audit_runs` table in `graph.db`, keyed by the `trace_id` that is already minted:
plan hash, files in scope, findings emitted, `decision_ref`s cited, severity, timestamp, git
HEAD at audit time. This single change is what makes everything below possible.

### 5.2 Resolve the outcome deterministically

For each persisted audit run, walk forward from its HEAD and classify — no model involved:

| Signal | Deterministic test |
|---|---|
| **complied** | the following commit touched a flagged file and the rejected pattern is gone |
| **ignored** | the following commit touched a flagged file and the pattern is still present |
| **missed** | a `failure log` / CI failure landed on files the audit saw and did not flag |
| **false positive** | flagged, pattern removed by an override marker or the finding dismissed via `illuminate verify` |

These are exactly the `EvalSignal` / `Severity` / `Outcome` / `GateStatus` model already
designed for **`illuminate-eval`** in `docs/plans/2026-06-16-phase3.md` (slices 5–7, ported from
`rohansx/reflect`, already attributed in `NOTICE`). That crate is designed and unbuilt. This
proposal is its first real consumer — the outcome resolver is `illuminate-eval` plus a git walk.

### 5.3 Score the guidance

Per decision / pattern / policy / reflexion, accumulate `{fired, complied, ignored, missed,
false_positive}` and fold to a Laplace-smoothed earned-precision score (also already designed in
the `illuminate-eval` slice). Three consumers:

1. **Audit ranking.** `find_relevant()` orders by earned precision × similarity, not similarity
   alone.
2. **Enrich budget.** `illuminate-enrich` injects the guidance that has historically changed
   behavior, instead of the top-k most similar.
3. **Automatic severity demotion.** A rule dismissed 20 times drops `error → warning →
   advisory` on its own. This is the single highest-value output: it stops illuminate from
   training its users to ignore it.

### 5.4 Propose edits as wiki PRs — never auto-write

Here the harness edit shape ports directly:

```
{ action: create|update|delete,
  kind:   decision|pattern|policy|reflexion,
  id, content,
  evidence:        [trace_id, ...],     // audit runs that justify this
  expectedOutcome: "...",               // and how to validate it
  before, after, version, rollback_of }
```

Applied as a front-matter/markdown edit under `.illuminate/wiki/`, with `before`/`after`
recorded so rollback is derived mechanically. The human gate already exists — `illuminate
verify` records a sign-off via a surgical front-matter edit (shipped, commit `91ef3e8`).

### 5.5 The line we should not cross

**Do not build auto-refine that authors new decisions from an LLM read of a trajectory.**
Illuminate's differentiator is that its guidance is human-authored, deterministic, and
CI-gateable. A self-improving *ranker* strengthens that. A self-improving *rule author*
destroys it — and it is the one part of the harness that is genuinely cheap to copy, which is
why it is worth naming as explicitly out of scope.

---

## 6. Sizing and sequencing

| Slice | Work | Depends on |
|---|---|---|
| 1 | `audit_runs` persistence + `trace_id` retained end-to-end | — |
| 2 | `illuminate-eval` leaf crate (already fully specced, phase3 slices 5–7) | — |
| 3 | Outcome resolver (git walk + eval signals) → `guidance_outcomes` table | 1, 2 |
| 4 | Scoring fold → `find_relevant` ranking + enrich budget + severity demotion | 3 |
| 5 | Edit proposals as wiki PRs with derived rollback | 4 |

Slices 1–2 are independent and each stands alone. Slice 4 is where the user-visible behavior
change lands.

## 7. Licensing

Prime Agent is MIT; illuminate is MIT. The established pattern in `NOTICE` — re-implement in
Rust, attribute in good faith, vendor nothing — applies directly. If the refinement record shape
and the derived-rollback construction port (§3.1, §5.4), add a `prime-agent` entry to `NOTICE`
alongside the existing `headroom` and `codebase-memory-mcp` entries. Nothing in this proposal
copies source.
