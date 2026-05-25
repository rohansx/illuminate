# Prompts are the new source code.

## A short manifesto for why Illuminate exists.

---

For thirty years, software engineering versioned the wrong thing — and got lucky.

In the pre-AI era, humans wrote source code and compilers produced binaries. The source was the artifact worth keeping. Binaries could be regenerated cheaply, so binaries were disposable. Git, GitHub, the entire SCM industry — they all exist because the source code is where human intent lives.

The model is so well-internalized that we no longer remember it's a model. *Of course* you version the source. *Of course* the binary is a build artifact. *Of course* `dist/` is in your `.gitignore`. We treat this as a law of nature.

It isn't a law. It's an assumption. And in the AI era, the assumption has quietly broken.

---

## What changed

Humans now write prompts. AI produces source code.

This is not a stylistic shift. It is a structural change in the artifact pipeline:

```
   pre-AI era:    human ──► source code ──► compiler ──► binary
                          (intent)          (artifact)
                          ▲ versioned        ▲ gitignored

   AI era:        human ──► prompt ──► AI model ──► source code
                          (intent)              (artifact)
                          ▲ ???                 ▲ versioned in git
```

The source code is the new binary equivalent. It's the machine output. It's reproducible from the inputs — given the same prompt, the same model, and the same context, you get materially similar code.

The **prompt + the reasoning + the iteration** is the new source. It is where human intent lives. It is what the next person needs to understand the code.

But we're still versioning the binaries.

---

## What we lose by versioning the wrong artifact

Every team that uses Claude Code, Cursor, or Codex at scale watches three losses compound silently:

**The reasoning evaporates.** A developer iterates with an agent for an hour — explores three approaches, hits two dead ends, refines the spec, lands on an implementation. They commit the code. The session ends. The jsonl file lives on their laptop until disk pressure deletes it. The next reviewer sees a diff and not a single word of why.

**Decisions made today are forgotten by next week.** *"We rejected Redis because our deployment target doesn't allow stateful sidecars."* Two weeks later a different developer asks an agent to add caching. The agent suggests Redis. The decision is invisible to the agent, probably invisible to the developer too, and definitely invisible to anyone reviewing the resulting PR. The team accidentally re-litigates an old conversation, often without realizing.

**Failures don't generalize.** Generated code breaks production. The fix gets shipped. The lesson — *agents touching this module should know about race condition X* — exists only in a Slack thread or a post-mortem doc nobody re-reads. Three months later a new dev runs into the same race condition with a different agent.

Each loss alone is annoying. Together they mean **the team's collective knowledge stays flat while code volume grows exponentially.** Onboarding gets harder. Reviews get shallower. Agents drift.

This is the predictable consequence of versioning the wrong artifact. We are recording the outputs of a generative process and discarding the inputs that produced them. Then we're surprised when the team's understanding can't keep up.

---

## Why "just write more docs" doesn't work

There is a tempting answer: *the team should document its decisions better.*

This has been tried, by every team, for decades. It doesn't work, for a structural reason: **humans don't maintain artifacts that don't directly produce code.** Documentation rots because nobody is forced to keep it current. ADRs go stale because the system that consumes ADRs is the next dev's optional reading habit.

The prompt is different. The prompt is the input to the next code generation. If the prompt is wrong or context-poor, the agent produces worse code immediately and visibly. There is an immediate negative feedback loop that documentation never had.

A versioned, queryable, enriched prompt isn't documentation. It's the source artifact. Maintaining it has a direct payoff on the next thing you build. That's why it can compound where docs can't.

---

## What "GitHub for agents" means

GitHub didn't succeed because it was a good UI on top of git. It succeeded because it made code **social**. Pull requests, code review, blame, issues — these are all about humans coordinating around an artifact.

The artifact has shifted. Code is downstream of prompts now. The social layer should follow.

GitHub for agents is what the social layer around prompts looks like:

- **`git log` for prompts.** Every prompt the team has chosen to publish, in order, browsable.
- **`git blame` for *"why does this code exist?"*** Click any line; see the prompt that produced it, the reasoning that shaped it, the decision it depends on.
- **Code review, one layer deeper.** Reviewers see not just the diff but the prompt that produced the diff. The conversation moves from *"is this code good?"* to *"was this prompt asking the right question?"*
- **Onboarding stops being broken.** New hires query the team's published prompts in order. They reach productivity because they can see how the team thinks, not just what the team wrote.

This isn't a feature request for GitHub. GitHub is for code. The new artifact needs its own tool, built around the new artifact's grain.

That tool is Illuminate.

---

## Two products, one substrate

Illuminate ships as one coherent system with two user-facing products:

**Illuminate Enrich** is the pre-prompt optimizer. Before your prompt reaches Claude Code, Cursor, or Codex, Illuminate queries the team's accumulated context and rewrites the prompt to be more specific, more grounded, more informed by relevant team decisions. The agent receives a better prompt and produces better code on the first try. No iteration loop. No drift.

**Illuminate Repo** is the versioned record. Sessions you choose to publish land in a team-shared git repo of structured markdown. Anyone can browse, search, blame, link. Future agents query the repo as context when generating new code.

Both ride on the same substrate: local trail capture, a bi-temporal decision graph, a code-graph blast-radius index, a deterministic policy engine.

The wedge is enrichment. The compounding moat is the repo. Enrich + Repo together close a loop the industry has never had — every prompt makes the next prompt smarter; every published session makes future onboarding faster.

---

## The four stages

Every prompt flows through four stages:

```
   ENRICH ── ► GENERATE ── ► CAPTURE ── ► CURATE
   (Illuminate)  (host       (Illuminate   (Illuminate +
                  agent)      trail)         dev choice)
                                                │
                                                ▼
                                          team repo
                                                │
                                                ▼
                                     feeds back into Enrich
```

**Stage 1 — Enrich.** The prompt is rewritten with relevant team context before the agent sees it. Deterministic, local, no LLM in the path. Same prompt + same graph → same enriched output.

**Stage 2 — Generate.** The host agent (Claude Code, Cursor, Codex) generates code from the enriched prompt. Illuminate sits out of this stage entirely. Use whatever agent your team likes.

**Stage 3 — Capture.** The session — prompt, response, iteration, code change — is captured locally. Automatic. Silent. Nothing leaves the machine.

**Stage 4 — Curate.** When the dev is ready, Illuminate asks: *publish this session?* The dev chooses: full, summary, decision-only, discard. Nothing is shared without consent.

The published session feeds Stage 1 for everyone else on the team. The loop tightens with use.

This is the *minimum complete pipeline.* Anything less misses one of the four. Tools that capture without enriching produce dashboards but don't change behavior. Tools that enrich without capturing have no graph to query against. Tools that publish without curating are surveillance.

---

## Why local-first is non-negotiable

You cannot ship a tool that captures every developer prompt as a SaaS. It would not be possible to install in any regulated vertical. It would not be acceptable to most senior developers. It would be one breach away from being a class-action lawsuit.

Illuminate's local-first architecture isn't a marketing point. It's the design constraint that makes the product *possible to ship at all*:

- **Capture is local.** `.illuminate/trail/` never leaves the dev's machine.
- **Enrichment is local.** The graph queries that build the enriched prompt run against `.illuminate/graph.db`.
- **Audit is local.** Policy checks happen against the local graph.
- **Publishing is explicit.** Movement from local to team-shared is a per-session decision, made by the dev, with a redaction-level chosen at publish time.

The trust model is enforced by the architecture, not by promises. See [`trust-model.md`](trust-model.md) for the specifics. The short version: **everything stays local until you explicitly publish it; some things are never built no matter what a customer asks for.**

---

## What we are not building

A long list, because the framing invites misunderstanding.

- **Not surveillance software.** No individual scoring. No management dashboards. No prompt rating for HR. If a competitor builds that, it's a different product.
- **Not a replacement for GitHub.** Illuminate uses git and runs alongside GitHub. The team repo can literally be a GitHub repo with structured markdown.
- **Not a prompt-management tool for production APIs.** Langfuse, PromptLayer, Braintrust manage prompts shipped to end users. Illuminate captures development-time coding sessions — a different artifact entirely.
- **Not a generic AI-powered wiki.** The wiki is a byproduct of the loop, not the product.
- **Not a code review tool.** Adjacent space, different artifact.
- **Not competing with spec-kit.** spec-kit captures planned intent (before code). Illuminate captures actual reality (during and after). They compose.

The discipline of saying *"not that"* is what keeps the product from sprawling into a vague "AI productivity suite." The prompt is the artifact. Everything else is downstream.

---

## What this implies for software teams

If prompts really are the new source code, then a few things follow:

**The team's prompts are an asset.** Six months of curated, published, enriched team prompts are something a competitor cannot replicate by hiring your engineers. The asset compounds in a way that headcount doesn't.

**Onboarding shifts.** New hires don't ask seniors *"how does this team think?"* They query the team repo. The dev who can read the published prompts gets up to speed in days, not months.

**PR review shifts.** Reviewers stop asking *"is this code good?"* and start asking *"is this prompt asking the right question?"* The review conversation moves one layer up the stack.

**Hiring shifts.** *Prompt curation* and *prompt blame* become resume entries. Engineers who can keep a team's prompt repo healthy are as valuable as engineers who can keep a codebase healthy.

**Documentation shifts.** ADRs and design docs become trivially generated from curated prompt history. The maintenance burden disappears because the artifact is already there.

**Failure analysis shifts.** Post-mortems link to the prompt that produced the broken code, not just the diff. The lesson generalizes because the next agent touching the same module sees the warning in its enriched prompt.

Not all of these will play out the way I just described. The framing is right; the specifics will surprise everyone. That's what new categories do.

---

## Why this is worth building

Many tools improve at the margin. They make existing workflows slightly better. They are useful and unmemorable.

A few tools change which artifact a team versions. Git did. GitHub did. Tools that move the artifact stay for decades because they redefine the substrate on top of which everything else works.

Prompts are the new source code. Versioning them, enriching them, sharing them — that is the next substrate. The team that owns that substrate owns a layer of the developer toolchain that doesn't have an incumbent yet.

This is the bet. Illuminate is the tool.

If the framing is right, *every* coding team will run something like this within five years. If the framing is wrong, Illuminate is a footnote — a clever tool for a niche use case. The product is structured to survive either outcome: the substrate is useful today (audit, wiki, blast-radius) regardless of whether the prompt-as-source framing wins the broader argument.

But I think the framing will win, because the losses are real, the compounding is real, and the architecture that respects developer trust is finally possible to build.

---

## Where to go next

- **Use it.** [Install Illuminate](../README.md#install) and run it against your real repos. The substrate (v0.18) is already useful. The enrich + publish wedge ships in v3.0.
- **Read the trust model.** [`trust-model.md`](trust-model.md) is the contract.
- **Read the product overview.** [`PRODUCT_OVERVIEW.md`](PRODUCT_OVERVIEW.md) is the framing in commercial language.
- **Read the roadmap.** [`ROADMAP.md`](ROADMAP.md) is the build plan.
- **Argue with me.** File an issue. The framing is in beta. The product is shipping.
