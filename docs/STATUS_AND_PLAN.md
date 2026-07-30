# Illuminate — Status and Plan

**As of:** 30 July 2026 · workspace **v0.31.0**

Companion to [`COMPANY_BRAIN.md`](COMPANY_BRAIN.md) (the v4 positioning) — that document says
*what illuminate is and why*; this one says *what is built, what is not, and in what order the
rest gets done*.

> **Why this doc exists.** `ROADMAP.md` claims v0.24 and `PRODUCT_OVERVIEW.md` claims
> v0.18/v0.21 against an actual v0.31.0. Rather than trust any of them, the state below was
> measured directly from the repository — crate sizes, test counts, CLI surface, MCP tools,
> and build behaviour were all verified by running them.

---

## 1. Where illuminate stands

### Measured, not claimed

| Dimension | Value |
|---|---|
| Workspace version | 0.31.0, Rust 2024 edition |
| Member crates | 20 |
| Rust source | ~65,500 lines |
| Tests | 1,031 passing on the offline path, 0 failing |
| CLI commands | ~52 modules under `crates/illuminate-cli/src/commands/` |
| MCP tools | 19 `illuminate_*` tools, stdio + HTTP |
| Storage | Two SQLite files per repo — `graph.db` (decisions) + `index.db` (code) |
| Lint gate | `cargo clippy --all-targets -- -D warnings`, clean |

### The competitive frame

"Company Brain" became a formal [YC Request for Startups](https://www.ycombinator.com/rfs)
category in Summer 2026, with a funded cohort — Supermemory, Hyperspell, Hyper, GBrain,
Memory Store. Hyper's thesis ("the bottleneck is persistent, graph-backed memory of a
company's decisions, taste, and which facts are stale") is illuminate's thesis, arrived at
independently and roughly 65,000 lines of working Rust later.

Illuminate is **not early to the insight** anymore. What it has that the category does not:

1. **Capture, not scrape.** Competitors infer knowledge from Slack/Notion/email exhaust.
   Illuminate captures at the moment of authorship — inside the agent loop.
2. **Write path, not read path.** Everyone else is retrieval; retrieval is advisory and
   fails exactly when the agent doesn't think to ask. Illuminate can *deny a write* and
   *fail a merge*.
3. **Determinism.** No LLM in the audit or query path. You cannot build a CI gate on a
   system that answers differently on Tuesday.
4. **Code-graph grounding.** Tree-sitter symbols and edges. No company brain reads the code.
5. **Portability** (earned by speaking OKF). Every competitor is a proprietary silo.

---

## 2. What was built in this cycle

Twelve commits on `claude/okf-research-teams-1xd6ji`. Every item below was verified by
running it, not just by passing tests.

### 2.1 Offline build — the local-first promise made true

**Problem found.** `illuminate` core defaulted `extract` on → `illuminate-extract` →
`fastembed` (feature `ort-download-binaries`) → `ort-sys`, whose build script **downloads an
ONNX Runtime binary from a third-party CDN at build time**. Only 5 of 20 crates built without
network access. For a project whose headline promise is *"single binary, local-first, no
infra,"* the build phoned out to `parcel.pyke.io` — and any team behind a corporate proxy hit
this on day one.

**What shipped.**

- A default-on `onnx` cargo feature threaded through all 20 crates.
- `illuminate-embed` gains a **stub engine with a byte-identical API surface**, so every
  downstream caller compiles untouched. This is safe because callers already handled
  `EmbedEngine::new()` returning `Err` — model download could always fail at runtime.
  `cosine_similarity` stays real in the slim build (it is pure arithmetic over two slices).
- `EmbedError::Unavailable` + `is_unavailable()` so callers distinguish "this build has no
  engine" from "the engine tried and failed".
- Two genuinely dead dependencies removed: `fastembed` in `illuminate-extract`,
  `illuminate-extract` in `illuminate-watch`.
- Runtime degrades exactly as the existing "models not installed" path already did.

**The released binary is unchanged** — default features are identical to before.

**CI guard.** A new `offline` job runs clippy + tests with `--no-default-features` and
asserts `ort-sys`/`fastembed` are absent from the slim dependency graph. `illuminate-extract`
is excluded on purpose: it *is* the ONNX crate, and a GLiNER extractor without a runtime is
nothing. The guarantee encoded is "every crate except the ONNX extractor builds and tests
with no network."

### 2.2 Trust — the layer nobody in the category built

OKF defines `verified` and `stale_after` and derives three trust tiers; no tool in the
ecosystem implements the *workflow*. The category's loudest published criticism is that
company brains "turn stale facts into confident answers instead of admitting uncertainty."

**What shipped.**

| Piece | Detail |
|---|---|
| `verified: [{by, at}]`, `stale_after` | Additive and skip-serialized on `FrontMatter`, so all 15 existing wiki pages round-trip **byte-identically**. Critical: `illuminate rebuild` round-trips the whole wiki, and gaining `verified: []` everywhere would be a diff across the entire team repo. |
| Trust tiers | `unverified` → `machine-confirmed` → `human-reviewed`, derived from OKF's actor convention (`human:<id>` / `process:<id>` / `<producer>/<version>`). A single human outranks any number of machines — the point is human attention, not vote count. |
| OKF bare-mapping form | A single verifier written as a bare mapping is read as a one-element list, per spec. |
| `illuminate verify <id> --as human:priya` | Records a sign-off. |
| 3 new lint codes | `StalePage`, `MalformedVerifier`, `VerificationPredatesUpdate` (page edited after sign-off, so the sign-off no longer covers it). |

**Two design decisions worth preserving.**

1. **Staleness never reads the clock.** `lint_page` stays pure and date-independent;
   staleness lives in `lint_page_on(page, today)` with the day as a *parameter*. This keeps
   the audit path deterministic — the standing architectural constraint.
2. **The verify edit is surgical.** The obvious implementation is parse → mutate →
   re-serialize, which would reorder keys and renormalize timestamps, turning a one-line
   sign-off into a whole-file diff. On a team's most consequential pages that destroys
   `git blame` — the exact history illuminate exists to preserve. Instead the entry is
   appended to the existing front-matter block. **Verified on a real page: a 3-line diff.**

### 2.3 OKF — portability in both directions

| Piece | Detail |
|---|---|
| `illuminate ingest --okf PATH` | `OkfBundleAdapter` on the existing `IngestAdapter` trait. Reads *anyone's* v0.2 bundle. |
| `illuminate export --format okf --out DIR` | Renders the wiki as a conformant bundle. |

**Tolerance is the hard part.** OKF §9 requires consumers *not* to reject a bundle for
unknown `type` values, unknown frontmatter keys, broken cross-links, or a missing
`index.md`. The natural implementation is stricter than the spec allows, so most of the
adapter's 16 tests exist to pin that tolerance. Reserved filenames (`index.md`, `log.md`) are
skipped at any depth; a malformed document is skipped, never fatal.

**The export is lossless.** OKF has no slot for illuminate's `id`, `confidence`, `modules`,
`authors`, or original `status` — those ride along as extension keys, which §9 requires
consumers to preserve. That keeps `export → ingest` a round trip instead of a one-way door.
Output is byte-stable, so re-exporting an unchanged wiki produces no diff. Relationships
become bundle-relative markdown links under `## Related`.

**Verified round trip on real data:** this repo's 15 wiki pages → bundle → re-ingested →
searchable, idempotent on re-run.

### 2.4 Team sync — the largest gap, closed

Before this, "team" meant *a shared folder on local disk*. `TeamRepoTarget::GitRemote` was
modeled but gated and unimplemented; there was no `sync`, `pull`, or `federate` verb. Nothing
carried a published page to a teammate and nothing brought their decisions back.

**`TeamRepoTarget::GitRemote`** — consent-gated. The design call worth preserving: it writes
to a **local working clone**, so `publish` still performs *zero* network I/O. Uploading is
`sync`'s job, behind its own explicit gesture. This keeps the crate's "no network calls"
invariant **literally** true rather than approximately — a publish can never surprise a
developer by sending something off-host.

**`illuminate sync`** — fetch → fast-forward → push, then re-index the team repo's wiki into
`graph.db` so teammates' decisions are immediately visible to `enrich` and `audit`.

Trust properties, each pinned by a test:

- `consent = false` refuses **before any step runs**.
- Pull is **fast-forward only** — sync never fabricates a merge in a repo it doesn't own.
- A failed fetch or fast-forward **aborts before the push**, so local commits are never
  uploaded on top of a stale view of the team's history.
- `--dry-run` executes nothing; `--no-push` consumes without publishing.
- "Nothing to push" is *reported*, not faked as a successful push.

Planning is a pure function and execution goes through an injectable `GitOps` trait, so the
whole orchestration is unit-tested without a network or a git binary.

**Verified end-to-end against real git repos:** a teammate's decision synced in and became
queryable locally; a local decision pushed to the remote; the consent gate refused;
`--no-push` consumed only.

### 2.5 A shipped bug, found and fixed

`register_docs` hardcoded `skipped_duplicates: 0` and never checked for existing episodes —
despite `ingest_all`'s doc comment promising dedup on `(adapter, external_id)`.

**Every `illuminate ingest` re-run duplicated the entire corpus.** Shipping since v0.22,
affecting `LocalMarkdownAdapter` too. A nightly ingest grew the graph without bound and
`illuminate ask` returned the same page N times.

Found by running the real binary twice rather than trusting the test suite. Fixed with a
5-test suite. Dedup skips unchanged docs and re-ingests when `updated_at` moves — keying on
the id alone would have made the corpus immutable.

---

## 3. What is not done

Ordered by how much it costs to be without.

| Gap | Status | Why it matters |
|---|---|---|
| **Trust-weighted ranking (C4)** | Not started | The schema and workflow ship, but `enrich`/`audit` don't yet *rank* by tier. A human-reviewed decision should outrank a machine-extracted guess when injecting context. **No tool in the category does this** — it is the differentiation, and it is currently inert. |
| **Abstention (C5)** | Not started | Answering the category's loudest criticism requires saying "I am unsure" below a confidence floor. `confidence` exists in front-matter and drives nothing. |
| **Connectors (Phase E)** | Not started | Two adapters exist (local markdown, OKF). Competitors ship Slack, Notion, Gmail, Drive, Linear. **Slack is where engineering decisions actually get made and illuminate is blind to it.** Largest remaining competitive gap. |
| **`cloud serve` on synced remotes (A3)** | Not started | Still scans local disk. `sync` unblocks it. |
| **Cross-machine identity (A4)** | Not started | No attribution model across machines. |
| **Benchmarks (Phase F)** | Not started | Supermemory publishes MemoryBench/LongMemEval numbers. Illuminate has none — in this category that reads as unserious regardless of engineering quality. |
| **Standalone `okf` crate (B3)** | Not started | Falls out of the exporter/adapter nearly free. Ecosystem presence only. |
| **Doc regeneration (D3)** | Not started | `ROADMAP.md` and `PRODUCT_OVERVIEW.md` remain 7+ versions stale. Best fixed by running `illuminate doc-decay` on our own docs — which is also the best available demo. |
| **Phase 3 remainder** | Not started | `illuminate-query` (openCypher subset) and `illuminate-eval` are designed in `docs/plans/2026-06-16-phase3.md`; neither crate exists. |

### Known limitations of this cycle's verification

- **The default-feature build was never run locally.** The sandbox blocks
  `parcel.pyke.io` — the very problem D1 solves for everyone else. Feature *resolution* was
  verified via `cargo tree`, and 1,031 tests pass on the slim path, but the default path is
  verified by CI only.
- **That gap caused a real miss.** Local clippy ran `--no-default-features`, so
  `#[cfg(feature = "onnx")]` code was compiled out of every check — and a doc-lint error in
  exactly that code reached CI. Fixed, and the diff swept for other instances. The lesson:
  **feature-gated code needs a verification pass under the feature that enables it.**
- **`illuminate_audit` never ran.** This repo's `CLAUDE.md` requires it before any source
  write. No `.mcp.json` exists and no MCP server was connected, and the CLI could not be
  built offline. D1 now makes the binary buildable in restricted environments, so a future
  session *can* satisfy the repo's own requirement.

---

## 4. The plan

Phases are ordered by **dependency, not appeal**. A and D were prerequisites for anything
else being credible; both are now done.

### ✅ Phase A — Cross the team gap

- **A1 ✅** `TeamRepoTarget::GitRemote`, consent-gated, no network on the publish path.
- **A2 ✅** `illuminate sync`.
- **A3** Point `illuminate cloud serve` at synced remotes rather than a local disk scan.
- **A4** Cross-machine identity and attribution for published knowledge.

### ✅ Phase D — Make it installable

- **D1 ✅** ONNX behind a default-on feature; offline build works.
- **D2 ✅** `offline` CI job + slim-graph assertion.
- **D3** Regenerate `ROADMAP.md` / `PRODUCT_OVERVIEW.md` against v0.31 using `doc-decay`.

### ◐ Phase C — Own trust and abstention

- **C1 ✅** `verified` / `stale_after` / trust tiers.
- **C2 ✅** `illuminate verify`.
- **C3 ✅** Trust lint rules.
- **C4** *Next.* Weight `enrich` and `audit` by trust tier. Ranking becomes a stable sort by
  `(tier, confidence, recency)` — still deterministic, so the audit path keeps its guarantee.
- **C5** Abstain below a confidence floor rather than asserting.

**C4 is the highest-value remaining work.** Everything needed for it now exists; without it
the trust ladder is schema without consequence.

### ✅ Phase B — Portability as a weapon

- **B1 ✅** `OkfBundleAdapter` + `--okf` ingest.
- **B2 ✅** `illuminate export --format okf`.
- **B3** Publish a standalone spec-complete `okf` crate.

### Phase E — Capture-grade connectors

Framed as illuminate's version, not the category's: **do not scrape a Slack channel into
embeddings.** Link a Slack thread to the decision it produced and let a human confirm at
publish time. Capture with human confirmation beats scraping with LLM inference, and it is
defensible in a way an embedding index is not.

- **E1** Slack adapter — thread → candidate decision, human-confirmed.
- **E2** Linear / GitHub PR adapters.
- **E3** Confluence / Notion / Google Docs (read-only, per existing trust invariants).

### Phase F — Get a number

- **F1** Run illuminate against MemoryBench / LongMemEval and publish results.

### Recommended order from here

**C4 → C5 → E1 → D3 → A3 → F1 → B3 → A4**

C4/C5 convert shipped schema into shipped differentiation. E1 (Slack) closes the biggest
competitive gap. D3 is cheap and stops the docs lying about the product.

---

## 5. Invariants that did not change

Everything above was built without relaxing any of these:

- **Local-first.** No required cloud.
- **Single binary, one SQLite file per repo.** No Docker, no sidecar, no Redis.
- **Deterministic audit and query paths.** No LLM, no clock reads. Same input, same output.
- **Append-only, bi-temporal graph.** Supersession is a new fact, never a mutation.
- **Markdown is source-of-truth.** The graph indexes the wiki, not the reverse.
- **Consent-gated publishing.** Nothing leaves the machine without an explicit human
  gesture — and `sync` is now the *only* network write path, explicitly opted into.

---

## 6. Related documents

- [`COMPANY_BRAIN.md`](COMPANY_BRAIN.md) — v4 positioning, competitive analysis, tech specs
- [`PRODUCT_OVERVIEW.md`](PRODUCT_OVERVIEW.md) — v3 positioning (stale; superseded)
- [`ARCHITECTURE.md`](ARCHITECTURE.md) — component structure and data flow
- [`CLI.md`](CLI.md) — full command reference, including `sync`, `verify`, and `export okf`
- [`SCHEMA.md`](SCHEMA.md) — wiki markdown schema
- [`AUDIT.md`](AUDIT.md) — audit semantics and exit-code matrix
- [`trust-model.md`](trust-model.md) — consent and redaction invariants
- [`plans/2026-06-16-phase3.md`](plans/2026-06-16-phase3.md) — the unbuilt Phase 3 design
