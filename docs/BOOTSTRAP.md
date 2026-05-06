# Illuminate — Bootstrap

The cold-start problem: on the day a team installs Illuminate, the graph is empty. The linter has nothing to enforce. The wiki has nothing to surface. If the dev's first three audit calls return "no findings," the dev concludes the tool doesn't work and uninstalls.

Bootstrap is the system that makes day-one valuable. This document specifies what gets ingested, in what order, with what fallbacks.

For ongoing ingestion (after bootstrap is done), see `INGESTION.md`.

---

## Goals

- **Day-one usefulness.** First audit call after `illuminate init` should return at least one substantive finding on a non-trivial repo.
- **Idempotent.** Running bootstrap twice doesn't double-ingest. Re-running it after a year picks up only what changed.
- **Inspectable.** Dev can see what was ingested, where it came from, and override anything that's wrong.
- **Bounded cost.** Bootstrap on a 6-month-old repo runs in < 5 minutes on a laptop with no LLM calls. With LLM fallback enabled, < $1.

---

## What `illuminate init` runs

```
illuminate init [--claude] [--cursor] [--codex] [--no-bootstrap]
   │
   ▼
1. write .illuminate/ skeleton
   ├─ illuminate.toml (from template)
   ├─ wiki/ scaffold
   └─ .gitignore additions

2. configure agent integrations
   ├─ --claude: append CLAUDE.md directive
   ├─ --cursor: write .cursorrules block
   └─ --codex:  emit instructions

3. run bootstrap pipeline (unless --no-bootstrap)
   ├─ source 1: existing agent files
   ├─ source 2: ADRs / decision docs
   ├─ source 3: git history
   ├─ source 4: README + CONTRIBUTING
   └─ source 5: optional interview

4. write a summary to wiki/log.md
   "bootstrap complete: N decisions, M patterns, K modules"
```

`--no-bootstrap` is for users who want to start from a clean slate (or who plan to run the bootstrap selectively later via `illuminate bootstrap <source>`).

---

## Source 1: existing agent files

Agents in the wild already document team norms in:

- `CLAUDE.md`, `CLAUDE.local.md`
- `AGENTS.md`
- `.cursorrules`, `.cursor/rules/*.md`
- `.windsurfrules`
- Various `.<agent>.md` conventions

These files contain the team's existing answer to "what should the agent never do." Bootstrapping treats them as a high-confidence source of decisions.

### Pipeline

1. Read each candidate file.
2. Strip code fences and front-matter.
3. Run extraction (the same pipeline used for git ingestion — see `INGESTION.md`).
4. Mark resulting decisions with `confidence: 0.95` and `source.kind = "agent_file"`.
5. Write each as a wiki page in `wiki/decisions/` with `created` set to the file's last-modified time.

### Caveats

- These files often mix decisions ("don't use Redis") with style ("use 2-space indent"). The signal scorer separates them — style rules are filtered out.
- If an existing rule contradicts a later git history finding, bootstrap surfaces the conflict and asks the dev to resolve (or auto-marks the older one as superseded if dates make that obvious).

---

## Source 2: existing ADRs / decision docs

Many teams already have an ADR ("Architecture Decision Record") practice. Common locations:

- `docs/adr/`, `docs/decisions/`, `architecture/decisions/`
- `*.adr.md`, `ADR-*.md`
- `docs/architecture/*.md`

ADR formats vary, but they share structure: title, status, context, decision, consequences. The ADR parser is a thin pre-processor that:

1. Detects the format (Nygard / MADR / custom).
2. Maps fields to the wiki schema (see `SCHEMA.md`).
3. Preserves the original `id` if present (e.g., ADR-0042 → `dec-adr-0042-<slug>`).
4. Sets `status` based on the ADR's status field (proposed/accepted/deprecated → active/active/superseded).

### Pipeline

1. Discover ADR files via path pattern match.
2. Parse each one (with a permissive parser; unknown sections preserved as raw text).
3. Convert to wiki page format, with `confidence: 1.0` (human-written).
4. Write to `wiki/decisions/`.
5. Don't re-extract via NER — the ADR is already structured.

The original ADR file is left in place. The wiki version is a parallel representation; the audit engine uses the wiki version. Teams who want to keep the ADR practice alongside the wiki can — the bootstrap doesn't enforce migration.

---

## Source 3: git history

The most volume-rich source, and the noisiest. Default scope: last 6 months of commits + PRs.

### Pipeline

```
git log --since="6 months ago" --pretty=...
   │
   ▼
for each commit:
   text = commit_message + (optional: diff_summary)
   files = changed paths
   author, date = from commit metadata
   ─► run signal scorer
       │
       ▼ score >= threshold
   ─► run extractor pipeline
       │
       ▼ entities + relations
   ─► dedup against current graph
       │
       ▼
   ─► write episode + entities + edges

for each PR (if github/gitlab token provided):
   text = PR title + body + accepted review comments
   ─► same pipeline
```

### What gets filtered

Most commits don't carry decisions. The signal scorer (see `INGESTION.md` Stage 1) filters out:

- Conventional commits with `chore:`, `style:`, `revert:`, `merge:` types
- Commits shorter than 30 chars
- Auto-generated commits ("dependency update", "Renovate Bot", etc.)
- Squash-merge commits whose body just lists individual commit subjects

Result on a typical 6-month-old repo with ~1000 commits: ~50–150 episodes survive scoring; ~15–40 produce structured decisions.

### Cost

- Local NER: free, ~50ms/commit. 1000 commits → ~50s of CPU.
- LLM fallback (~30%): ~$0.0003/episode × ~150 episodes → ~$0.05.
- Total bootstrap on a 6-month-old repo: < 1 minute, < $0.10.

### Configuration

```toml
[bootstrap.git]
since = "6 months ago"             # or absolute date "2025-01-01"
include_diffs = false              # diffs help signal but bloat episodes
include_prs = true                 # requires github_token if private
max_commits = 5000                 # safety cap
```

---

## Source 4: README + CONTRIBUTING.md

These often contain implicit architectural notes:

- "We use PostgreSQL because..." → decision
- "Modules live in `src/` organized by domain" → pattern
- "Don't add new dependencies without team review" → policy

The README parser:

1. Strips badges, install instructions, and license blocks.
2. Extracts paragraphs that match decision-language signals.
3. Runs them through the extractor with `confidence: 0.7` (mid; README text is opinion-shaped).
4. Surfaces low-confidence candidates for dev review rather than auto-merging.

If a CONTRIBUTING.md exists, it's parsed similarly but with a focus on policy-shaped statements ("PRs must include tests", "use conventional commits").

---

## Source 5: optional interview

The best source of unwritten team knowledge is the senior engineer. Bootstrap optionally prompts the dev:

```
illuminate init --interactive

> What should the agent never do in this repo?
  (e.g., "never add new microservices", "never modify src/auth/")

> 1. ____________________________________________
> 2. ____________________________________________
> 3. ____________________________________________

> What did your team try and reject in the last year?
  (e.g., "Redis caching - VPC limits", "GraphQL - team velocity")

> 1. ____________________________________________
> 2. ____________________________________________

> What is one architectural decision you wish was written down?

> 1. ____________________________________________
```

Each entry produces a `wiki/decisions/` page with `confidence: 1.0` and `source.kind = "interview"`. The dev can edit the resulting markdown immediately.

This is optional; `illuminate init --no-interactive` skips it entirely.

---

## Order of operations

Sources are run in priority order (highest-confidence first). When a later source contradicts an earlier source, the earlier one wins, but the conflict is logged for review:

```
1. agent files (CLAUDE.md, AGENTS.md, .cursorrules)  — confidence 0.95
2. ADRs                                               — confidence 1.0
3. interview answers                                  — confidence 1.0
4. README / CONTRIBUTING                              — confidence 0.7
5. git history                                        — confidence 0.5–0.85
```

ADRs and interviews tie at 1.0 because they're both human-written and intentional. Agent files are slightly lower because they sometimes mix style and decisions.

---

## What gets written

After bootstrap completes:

```
.illuminate/
├── illuminate.toml          (from template, lightly edited)
├── graph.db                 (populated)
├── wiki/
│   ├── index.md             (auto-generated catalog)
│   ├── log.md               (bootstrap entry: "imported N decisions...")
│   ├── schema.md            (copy of canonical schema)
│   ├── decisions/
│   │   ├── adr-0042-postgres-over-mongo.md     (from ADR)
│   │   ├── 2025-12-no-redis-payments.md        (from git)
│   │   └── ...
│   ├── patterns/...
│   ├── failures/...
│   └── modules/...
└── trail/                   (empty until first session)
```

The `wiki/log.md` entry from bootstrap looks like:

```
2026-05-06T10:14:33Z  BOOTSTRAP-START
2026-05-06T10:14:35Z  ADD     dec-claude-md-no-microservices  (agent_file, conf=0.95)
2026-05-06T10:14:36Z  ADD     dec-adr-0042-postgres           (adr, conf=1.0)
2026-05-06T10:15:02Z  ADD     dec-2025-12-no-redis-payments   (git, conf=0.81)
...
2026-05-06T10:18:11Z  BOOTSTRAP-COMPLETE  decisions=37 patterns=8 modules=5
```

---

## Verification

After bootstrap, dev runs:

```bash
illuminate stats          # counts: 37 decisions, 8 patterns, ...
illuminate decisions list # all decisions, by date
illuminate audit "test plan: add Redis caching"  # smoke-test the linter
```

The smoke-test should return at least one non-trivial finding on a typical repo. If it returns nothing, that's a signal bootstrap didn't ingest enough — the dev can re-run with `--include-diffs` or extend the git history window.

---

## Re-running bootstrap

`illuminate bootstrap` (without `init`) re-runs the bootstrap pipeline against current sources:

```bash
illuminate bootstrap                      # full re-run; idempotent
illuminate bootstrap --source git         # only re-scan git history
illuminate bootstrap --since "2026-01-01" # extend window
illuminate bootstrap --interview          # additional interview round
```

Idempotent because the dedup layer in the extractor catches re-ingested content via `(source_ref, content_hash)`. The same commit doesn't produce two decisions.

---

## Failure modes

- **No git history.** Empty repo. Bootstrap skips git silently and proceeds with other sources. If all sources are empty, the wiki shows a "Bootstrap found no content. Try `illuminate bootstrap --interview`." message.
- **ADR parser fails on non-standard format.** Logged, file skipped, error surfaced in `wiki/log.md` with the path. Dev can either fix the file or write a custom parser.
- **LLM fallback unreachable.** Falls back to local-only extraction with no degradation other than slightly lower coverage. Logged.
- **README is autogenerated boilerplate.** Signal scorer rejects it; nothing is added. No false positives.

---

## What's deferred

- **Slack/Discord chat history ingestion.** Many decisions live in chat. Useful but expensive to build well. Deferred to v0.3+.
- **Linear/Jira ticket ingestion.** Same. Deferred.
- **Codebase semantic scan** (read every source file, extract design patterns). Possible but expensive. Modules are summarized in `wiki/modules/`; full source-code semantic ingestion is deferred until there's evidence it pays off.

The minimum viable bootstrap is: agent files + ADRs + git history. Everything else is incremental.
