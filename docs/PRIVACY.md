# Illuminate — Privacy & Security

This document specifies what data Illuminate captures, where it lives, what (if anything) leaves the dev's machine, and how the system behaves under failure. It's the document the security review at a regulated buyer (Harvey, Abridge, Hippocratic AI, financial institutions, healthcare) will read first.

For the architecture that this model rests on, see [`ARCHITECTURE.md`](ARCHITECTURE.md). For the **canonical reference on what is stored at each of the three layers (raw trail / session summary / graph entities), where it lives, how long it's retained, and what the redaction pipeline filters out**, see [`data-model.md`](data-model.md) — that's the artifact compliance reviewers should read alongside this one.

---

## TL;DR

- All capture is local. The graph is local. The wiki is local (in your git repo).
- No telemetry, no analytics, no "anonymous usage stats," no auto-update phone-home. Ever.
- Optional LLM fallback during ingestion (~30% of episodes). Off by default. PII stripped before send if enabled.
- The audit/query path never calls an LLM. Fully deterministic, fully local.
- You can run Illuminate fully air-gapped after first install (download models once).

---

## Data residency

| Artifact | Location | Visibility |
|----------|----------|------------|
| `.illuminate/trail/` (raw session jsonl) | dev's laptop | gitignored, local-only |
| `.illuminate/graph.db` (SQLite) | dev's laptop | gitignored, local-only |
| `.illuminate/wiki/` (markdown pages) | git repo | shared with whoever has repo access |
| `illuminate.toml` | git repo | shared with whoever has repo access |
| `.illuminate/log/illuminate.log` | dev's laptop | gitignored, local-only |
| `.illuminate/audits.log` | dev's laptop | gitignored, local-only |
| ONNX model files | `~/.cache/illuminate/models/` | dev's laptop, downloaded once |

The `.gitignore` written by `illuminate init` ensures the laptop-local artifacts are not accidentally committed.

### What's in the trail (and why it's gitignored)

`trail/<date>-<topic>-<agent>.jsonl` contains the full normalized session: every prompt, every assistant response, every tool call, every file the agent touched. This is sensitive because:

- Devs sometimes paste credentials, internal URLs, or customer data into prompts.
- The model's responses may include speculative or wrong-headed analysis that wasn't merged.
- The diff between failed approaches and the final implementation is informative but exposes the dev's reasoning process in a way they might not want shared.

The trail is the input to extraction. Once extraction has run, the trail isn't needed for the audit/query path. Teams that want extra paranoia can run `illuminate trail purge --older-than 30d` to delete trails after they're processed.

### What's in the wiki (and why it's git-shared)

The wiki contains *distilled* knowledge: structured decisions, patterns, failures, module overviews. It's the output of extraction, not the raw input. A wiki page mentions the *kind* of decision ("rejected Redis for caching"), not the original prompt that surfaced it.

When a wiki page is auto-generated from a trail, the trail file is referenced via `sources` in the front-matter — but the prompt text itself is not copied into the page. Devs can review the wiki before pushing the repo and remove anything sensitive that slipped through.

---

## Network boundaries

```
┌────────────────────────────────────────────────────────┐
│                  illuminate (default)                  │
├────────────────────────────────────────────────────────┤
│                                                        │
│  ┌──── on first install ────┐                          │
│  │ download ONNX models     │  ◄── HF Hub / GitHub     │
│  │ (one-time, ~700 MB)      │      Releases CDN        │
│  └──────────────────────────┘                          │
│                                                        │
│  AFTER FIRST INSTALL: NO NETWORK CALLS BY DEFAULT      │
│                                                        │
│  ┌── optional, opt-in ──────────────┐                  │
│  │ LLM fallback during ingestion    │  ◄── Anthropic / │
│  │ ONLY when local NER confidence   │      OpenAI /    │
│  │ < threshold (~30% of episodes)   │      Ollama /    │
│  │ PII stripped via cloakpipe first │      none        │
│  └──────────────────────────────────┘                  │
│                                                        │
│  NEVER:                                                │
│  - telemetry, analytics, "anonymous stats"             │
│  - auto-update checks against a server                 │
│  - error reporting (Sentry, Bugsnag, etc.)             │
│  - cross-repo synchronization                          │
│  - audit/query-path LLM calls                          │
│                                                        │
└────────────────────────────────────────────────────────┘
```

### Air-gapped operation

After running `illuminate models download` once on a machine with internet, Illuminate can run fully offline. Disable LLM fallback in `illuminate.toml`:

```toml
[extraction.llm]
provider = "none"
```

In that mode, low-confidence episodes are queued for dev review (`illuminate wiki review`) rather than sent to any LLM. Coverage drops slightly; correctness is unaffected.

### Verifying no network calls

Devs can verify with:

```bash
illuminate audit "..." 2>&1 | grep -i "http\|connect"   # should be empty
strace -f -e trace=connect illuminate audit "..."       # only local sockets
```

The query path makes only file system + SQLite calls. The ingestion path makes file system + SQLite calls plus, if `provider != "none"`, one HTTPS call per low-confidence episode.

---

## PII handling

### What PII can appear in inputs

| Input | Likely PII |
|-------|-----------|
| Trail jsonl | dev's prompts may contain emails, credentials, URLs, paths, customer names |
| Git commit messages | author email, sometimes incident IDs |
| PR bodies | reviewers' usernames, ticket links |
| README | contributor emails |

### How PII is handled

**Local stages (NER, embedding):** No PII protection needed because data never leaves the laptop. Entities like `Person { name: "priya" }` are added to the graph as-is — same way git stores the author already.

**LLM fallback (~30% of episodes):** Before any text is sent to a configured LLM provider, it's run through cloakpipe (or a built-in fallback regex) to strip:

- Email addresses → `<email>`
- Phone numbers → `<phone>`
- API keys / tokens (regex-detected) → `<secret>`
- File paths containing usernames → `<path>`
- Custom team-specified patterns from `illuminate.toml`

Custom patterns let teams ban specific terms (project codenames, customer names, internal hostnames):

```toml
[extraction.llm.pii_patterns]
codenames = ["Project Phoenix", "Operation Eagle"]
hostnames = ["*.internal.acme.com"]
```

The substitution table is stored in memory only for the duration of the call; the LLM response is post-processed to undo the substitutions where safe.

If PII stripping fails (regex panic, unexpected input), the episode is skipped and logged rather than sent through unstripped.

### What's never sent to an LLM

- The dev's name or email (always stripped before send).
- The repo path (replaced with `<repo>`).
- File contents (only commit messages / PR bodies / trail text are sent — never source code itself).
- Any string matching `[email/secret/path]` patterns above.

---

## Threat model

### In scope

| Threat | Mitigation |
|--------|-----------|
| Trail file exfiltration via accidental commit | gitignored by `illuminate init`; warned about by `illuminate status` |
| Sensitive data in wiki pages pushed to public git | wiki review before push; team can run `illuminate wiki redact <pattern>` |
| LLM provider logs prompts | PII stripped before send; provider list is dev-configured; can be set to `none` |
| Malicious agent calls `illuminate_audit` with crafted input to read graph contents | Audit responses contain only metadata about decisions/patterns; no source-code content. Read-only path. |
| Compromised dependency injects code | Workspace pinned, `cargo deny` configured, `cargo audit` in CI |
| ONNX model tampering | Models downloaded over HTTPS with SHA256 verification; signed releases planned |

### Out of scope (and why)

- **Server-side data breach.** There is no server. There is no centralized data store. A breach would require breaking into each dev's laptop individually.
- **Adversarial dev intentionally exfiltrating.** The local-first model trusts the dev with their own machine. If the dev is the attacker, no tooling on their machine helps.
- **Side-channel via embeddings.** Embeddings are 384-dim float vectors of trail content. They could in theory be inverted by a sophisticated attacker. Mitigation: embeddings are gitignored (in `graph.db`) and never sent to a remote service.

---

## Failure modes

What happens when things go wrong:

| Failure | Behavior |
|---------|----------|
| Disk fills | SQLite writes fail loudly; trail watcher pauses; CLI commands return clear error. No silent data loss. |
| `graph.db` corrupted | `illuminate rebuild` regenerates from `wiki/` + `trail/`. No data loss. |
| `wiki/` accidentally deleted | If committed to git, recover from git. If never committed, gone (expected — this is the team-shared source-of-truth). |
| Trail watcher crashes | Daemon supervisor restarts it. Missed events are picked up on next session-end inotify event. |
| LLM provider down | Episode requeued with backoff; ingestion not blocked; audit unaffected (audit doesn't use LLM). |
| Network unreachable | If LLM provider is set, low-confidence episodes are queued. Audit, wiki, and graph all keep working. |
| ONNX model file missing | `illuminate audit` runs in degraded mode (no semantic search; only path-anchored + policy queries). Loud warning. |
| Compromised LLM API key | Bound to ingestion only; rotating the key blocks future low-confidence episodes from being sent. Past graph contents are unaffected. |

---

## Compliance posture

What Illuminate does and doesn't claim:

### What it supports

- **No data leaves the customer environment by default.** Suitable for HIPAA-aware workflows, GDPR data residency requirements, and regulated industry deployment.
- **Audit log of LLM-fallback calls.** When `[extraction.llm.provider]` is set, every call is logged locally with timestamp, episode id, and PII-stripped payload size. Available via `illuminate stats llm`.
- **Right-to-deletion.** `illuminate forget <decision-id>` marks an entity as retired; `illuminate purge --decision <id>` removes it from the graph. The corresponding wiki page can be deleted via git.
- **Local backup.** Everything that matters is in `wiki/` (in git) and reproducible from `trail/` (rebuildable). Standard git backup practices cover the team-shared layer.

### What it does not claim

- **No SOC2 / ISO27001 certification.** OSS project; certification is the deploying team's responsibility for their own environment.
- **No formal threat model audit by a third party.** Reviewed internally; external review is welcome but not yet commissioned.
- **No legal liability for misuse.** MIT license; no warranty.

For commercial deployments (v0.4+) where Illuminate Cloud may be involved, formal compliance docs will accompany that offering. The OSS binary stays as described above.

---

## Configuration knobs

Privacy-relevant settings in `illuminate.toml`:

```toml
[extraction.llm]
provider = "none"              # none | anthropic | openai | ollama
pii_strip = true               # ALWAYS true; cannot be disabled
max_calls_per_day = 1000       # safety cap
audit_log = true               # log every LLM call locally

[trail]
enabled = true                 # set false to fully disable session capture
purge_after_days = 90          # auto-delete trail jsonl after N days
exclude_patterns = ["*secret*", "**/credentials/**"]

[wiki]
auto_merge_threshold = 0.7
require_review_below = 0.5

[audit]
verbose_logging = false        # if true, log full audit input/output (devs only)
```

Defaults are paranoid. Devs opt in to broader behavior (e.g., LLM fallback, longer trail retention) explicitly.

---

## Reporting a vulnerability

Email security@illuminate.sh. Coordinated disclosure preferred. Critical vulnerabilities patched within 48 hours; details published in CHANGELOG with credit (if requested).
