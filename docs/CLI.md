# Illuminate — CLI Reference

The `illuminate` CLI is the single binary that ships with the project. All other components are libraries it composes.

For agent-side tooling, see `MCP.md`. For configuration, see `illuminate.toml.example` at the repo root.

---

## Global flags

```
illuminate [--config PATH] [--repo PATH] [--quiet|--verbose] <subcommand>
```

| Flag | Default | Meaning |
|------|---------|---------|
| `--config` | `.illuminate/illuminate.toml` | Path to the config file. |
| `--repo` | current directory (walks upward to find `.illuminate/`) | Repo root override. |
| `--quiet` | off | Suppress info logs. Errors still print. |
| `--verbose` | off | Print debug-level logs. Useful for troubleshooting. |

Exit codes:

- `0` — success
- `1` — generic error (config invalid, file not found, etc.)
- `2` — audit returned `block` status (used by CI integrations)
- `3` — audit returned `warn` status (informational; CI does not fail by default)

---

## Lifecycle commands

### `illuminate init`

Set up Illuminate in the current repo.

```
illuminate init [--claude] [--cursor] [--codex]
                [--no-bootstrap] [--interactive]
                [--skip-models]
```

Effects:

1. Creates `.illuminate/` directory with `illuminate.toml`, `wiki/` skeleton, and `.gitignore` updates.
2. With `--claude`: appends the audit-pre-write directive to `CLAUDE.md` (creates if missing) and sets up MCP server registration in `.claude.json`.
3. With `--cursor`: writes a block to `.cursor/rules/illuminate.md`.
4. With `--codex`: prints integration instructions (Codex doesn't have a standard config file).
5. Runs the bootstrap pipeline (see `BOOTSTRAP.md`) unless `--no-bootstrap` is set.
6. With `--interactive`: prompts the dev with onboarding questions during bootstrap.
7. Downloads ONNX models if not already present (~700 MB) unless `--skip-models`.

Idempotent. Re-running `illuminate init` updates the config without re-bootstrapping (run `illuminate bootstrap` for that).

### `illuminate models download`

```
illuminate models download [--force]
```

Downloads GLiNER, GLiREL, and all-MiniLM-L6-v2 ONNX models to `~/.cache/illuminate/models/`. Verifies SHA256. Skips if already present unless `--force`.

### `illuminate serve`

Start the MCP server (foreground, stdio) so agents can call audit tools.

```
illuminate serve [--daemon] [--http ADDR]
```

- Default: stdio. Agents launch this as a child process.
- `--daemon`: background daemon. Hosts the trail watcher and ingestion workers in addition to the MCP server.
- `--http ADDR`: bind HTTP server (Streamable HTTP per MCP spec). For remote / containerized deployments.

### `illuminate status`

Print the current state of the local Illuminate installation.

```
illuminate status
```

Shows:

- Daemon running? Last heartbeat.
- Graph entity count, last update time.
- Wiki page count.
- Trail file count + total size.
- Pending review queue size.
- Configured LLM provider (or "none").

---

## Audit commands

### `illuminate audit`

Run the audit engine on a free-form plan.

```
illuminate audit "<plan>"
                 [--files PATH ...]
                 [--rationale TEXT]
                 [--format json|text|markdown]
```

Returns the audit response on stdout. Exit code reflects severity:

- `0` — pass
- `2` — block
- `3` — warn

Default format is text (human-friendly). `--format json` returns the raw response (see `AUDIT.md` for shape).

### `illuminate audit-diff`

Audit the working tree's uncommitted changes.

```
illuminate audit-diff [--staged-only] [--format json|text|markdown]
```

Computes the diff vs `HEAD`, runs the audit on each chunk, returns aggregated findings.

### `illuminate audit-pr`

Audit a GitHub PR. Used by the CI gate.

```
illuminate audit-pr <pr-number> [--repo OWNER/REPO]
                                 [--token-env GITHUB_TOKEN]
                                 [--comment]
                                 [--format json|markdown]
```

With `--comment`, posts the audit findings as a PR comment via the GitHub API.

### `illuminate explain`

Explain why a file matters: which decisions, patterns, and failures touch it.

```
illuminate explain <path>
```

No "plan" required. Useful for orientation when reading unfamiliar code.

### `illuminate enrich`

Deterministic pre-LLM prompt enrichment. Takes a developer prompt and the local graph, returns an enriched prompt with relevant decisions/patterns/failures and code paths injected. Shipped in v0.19 as the first half of the v3 two-product positioning (see `PRODUCT_OVERVIEW.md` → Illuminate Enrich).

```
illuminate enrich "<prompt>"
                  [--files PATH ...]
                  [--max-bytes N]
                  [--format human|prompt|json]
```

| Flag | Default | Effect |
|------|---------|--------|
| `<prompt>` | (required) | The developer's raw prompt. |
| `-f PATH`, `--files PATH ...` | `[]` | Hint which files the prompt is about (narrows code-graph queries). |
| `--max-bytes N` | `4096` | Soft cap on injected context length. Trailing injections drop deterministically when the budget overflows. |
| `--format human \| prompt \| json` | `human` | `human` prints the enriched prompt plus a footer summarizing injection count and the determinism receipt prefix. `prompt` emits the enriched text verbatim (pipe into an agent). `json` emits the full `EnrichResponse` envelope. |

**Determinism guarantee.** Same `(prompt, graph state)` → byte-identical output. Every response includes a hex-encoded `graph_state_hash` (SHA-256 over the canonical view of injections) as the receipt. Verified by the property test `determinism_property_same_input_yields_identical_output` in `crates/illuminate-enrich/src/lib.rs`.

**Examples.**

```bash
# Default: enriched prompt + a footer line with the hash prefix.
illuminate enrich "add Redis caching to the txn endpoint"

# Pipe the enriched prompt straight into Claude Code:
illuminate enrich "add caching to txn lookup" --format prompt | claude code

# Inspect what was injected and the determinism receipt:
illuminate enrich "refactor process_payment" --format json | jq '.injections, .graph_state_hash'

# Narrow the code-graph query by passing file hints:
illuminate enrich "fix the race condition" --files src/payments/txn.rs --files src/payments/cache.rs
```

**Exit codes.** `0` always (enrichment is informational, never blocking). For a hard gate, use `illuminate audit`.

**MCP equivalent.** Planned for v0.20: `illuminate_enrich` MCP tool so agents can call enrich inline without shelling out. For now, the CLI verb is the only entry point.

### `illuminate publish`

Explicit publish gesture for one captured trail session. Reads a trail jsonl from `.illuminate/trail/`, redacts per the chosen level, writes a structured markdown page under `<team-repo>/sessions/<YYYY-MM-DD>-<slug>.md`, and registers a graph episode so future `illuminate enrich` calls can surface the published session. Shipped in v0.21 as the second half of the v3 two-product positioning (see `PRODUCT_OVERVIEW.md` → Illuminate Repo).

```
illuminate publish --trail PATH
                   --redaction full|summary|decision|discard
                   --team-repo PATH
                   [--commit-sha SHA]
                   [--json]

illuminate publish --install-hook --team-repo PATH
```

| Flag | Default | Effect |
|------|---------|--------|
| `--trail PATH` | (required for publish) | Path to the trail jsonl to publish (e.g. `.illuminate/trail/<file>.jsonl`). |
| `--redaction X` | `summary` | One of `full` / `summary` / `decision` / `discard`. See `docs/SCHEMA.md` → Session page for what each level emits. |
| `--team-repo PATH` | (required) | Target directory. The crate writes `<team-repo>/sessions/<file>.md`. **The only place outside `.illuminate/` that any illuminate crate writes to.** |
| `--commit-sha SHA` | (none) | Git commit this session produced — recorded in front-matter so the resulting page links the prompt to the code. |
| `--json` | off | Emit the `PublishResponse` envelope (session_id, written_paths, graph_episode_id) instead of a human-readable summary. |
| `--install-hook` | off | Skip the publish; write a `.git/hooks/pre-commit` script that runs `illuminate publish` on every commit. Requires `--team-repo`. The hook defaults to `--redaction summary`; override per commit via `ILLUMINATE_PUBLISH_REDACTION=<level>` or skip with `git commit --no-verify`. |

**Trust-model invariants.** This is the only `illuminate` verb that writes outside `.illuminate/`, and only to the explicit `--team-repo` path. No defaults, no implicit network. The planned `TeamRepoTarget::GitRemote` variant is deliberately gated for v3.1 behind `illuminate trust check` to enforce a config-linter pass before any cross-machine write. See `docs/trust-model.md`.

**Examples.**

```bash
# Publish the most recent trail session as a summary into a sibling team repo:
LATEST=$(ls -1t .illuminate/trail/*.jsonl | head -1)
illuminate publish --trail "$LATEST" --redaction summary --team-repo ../team-illuminate

# Decision-only — front-matter and `commit_sha` only, body intentionally empty:
illuminate publish --trail "$LATEST" --redaction decision \
    --team-repo ../team-illuminate --commit-sha "$(git rev-parse HEAD)"

# Install the pre-commit hook so every git commit auto-publishes a summary:
illuminate publish --install-hook --team-repo ../team-illuminate

# Decide per-commit at the hook level:
ILLUMINATE_PUBLISH_REDACTION=full git commit -m "ship the LRU cache"
```

**Exit codes.** `0` on success or `discard`. Non-zero on missing flags or filesystem errors.

---

## Decision / pattern / failure commands

### `illuminate decisions`

```
illuminate decisions list [--module SLUG] [--tag TAG]
                          [--include-superseded] [--include-retired]
                          [--format json|text]
illuminate decisions show <id> [--format json|markdown]
illuminate decisions for <path>
```

`for` returns all decisions whose `modules` or `paths` intersect the given path.

### `illuminate patterns`

```
illuminate patterns list [--module SLUG] [--tag TAG]
illuminate patterns show <id>
```

### `illuminate failures`

```
illuminate failures list [--module SLUG] [--severity S]
illuminate failures show <id>
```

### `illuminate failure log`

Record a new failure.

```
illuminate failure log [--title TITLE]
                        [--root-cause TEXT]
                        [--fix TEXT]
                        [--lesson TEXT]
                        [--files PATH,PATH]
                        [--modules SLUG,SLUG]
                        [--severity low|medium|high|critical]
                        [--from-incident URL]
```

Without flags, opens an editor with a template. With flags, runs non-interactively.

Output: writes `wiki/failures/<date>-<slug>.md` and a graph entity.

---

## Wiki commands

### `illuminate wiki rebuild`

Regenerate `wiki/index.md` and reconcile the graph index against on-disk markdown.

```
illuminate wiki rebuild [--strict]
```

`--strict` fails on any wiki-lint error.

### `illuminate wiki serve`

Serve the wiki as static HTML (locally). Not deployed; opens in a browser.

```
illuminate wiki serve [--port 8765]
```

Markdown is rendered with a minimal stylesheet. Anchors and inter-page links work. No JavaScript.

### `illuminate wiki lint`

Validate every wiki page against `SCHEMA.md`.

```
illuminate wiki lint [--strict]
```

Used by CI. Fails on any error in `--strict` mode.

### `illuminate wiki review`

Walk the queue of low-confidence candidates produced by extraction.

```
illuminate wiki review [--threshold 0.5]
```

For each candidate, the dev sees the proposed decision, the source episode, and the confidence. Choices: accept (write wiki page), reject (mark episode as not a decision), edit (open in editor), defer (move to back of queue).

### `illuminate wiki redact`

Bulk-redact a pattern across the wiki (and the corresponding graph entities).

```
illuminate wiki redact <regex> [--dry-run]
```

Useful when a sensitive term slipped through. Always run with `--dry-run` first.

---

## Bootstrap & rebuild commands

### `illuminate bootstrap`

Run (or re-run) the bootstrap pipeline.

```
illuminate bootstrap [--source git|adr|agent_files|readme|interview|all]
                     [--since DATE]
                     [--include-diffs]
                     [--max-commits N]
                     [--interview]
```

See `BOOTSTRAP.md` for full source semantics.

### `illuminate rebuild`

Rebuild `graph.db` from `wiki/` and `trail/`. Idempotent.

```
illuminate rebuild [--from wiki|trail|both] [--clean]
```

`--clean` deletes the existing `graph.db` first. Otherwise, dedup is preserved.

---

## Index commands

### `illuminate index`

Rebuild the code index (file → module map, symbol → location map).

```
illuminate index [--enrich] [--lang rust,ts,py]
```

`--enrich` runs deeper analysis (slower, more accurate). Use after large refactors or when cross-references seem stale.

---

## Search commands

### `illuminate search`

Full-text + semantic search over the graph.

```
illuminate search "<query>" [--limit N] [--type entity|decision|pattern|failure]
                             [--format json|text]
```

Combines SQLite FTS5 with embedding similarity. Results are ranked by combined score.

---

## Trail commands

### `illuminate trail list`

List captured trail files.

```
illuminate trail list [--last N] [--agent claude|cursor|codex]
```

### `illuminate trail show`

Print a trail file's normalized content.

```
illuminate trail show <path-or-session-id> [--messages-only]
```

### `illuminate trail purge`

Delete trail files older than N days.

```
illuminate trail purge --older-than DAYS [--dry-run]
```

### `illuminate trail install-service`

Generate a systemd user unit at `~/.config/systemd/user/illuminate-trail.service` that runs `illuminate trail watch` at login. Linux only.

```bash
illuminate trail install-service           # write the unit
systemctl --user daemon-reload
systemctl --user enable --now illuminate-trail
journalctl --user -u illuminate-trail -f   # tail the watcher
```

Pass `--dry-run` to inspect the unit content without writing. Pass `--force` to overwrite an existing unit.

---

## Reflect commands

(Same as failure commands; `failure log` is the primary entry. Future: `reflect ingest-ci-log`.)

---

## Stats / observability

### `illuminate stats`

```
illuminate stats               # everything
illuminate stats audit         # audit calls per day, hit rate, latency
illuminate stats llm           # llm fallback calls + cost
illuminate stats graph         # entity counts by type, edge counts
illuminate stats trail         # trails captured, by agent
```

All stats are local. Nothing reported home.

---

## Maintenance

### `illuminate forget`

Mark a decision/pattern as retired. The page is kept in the wiki; the graph stops surfacing it in audit responses.

```
illuminate forget <id> [--reason TEXT]
```

### `illuminate purge`

Hard-delete from the graph (and optionally the wiki).

```
illuminate purge --decision <id> [--also-wiki]
```

Asks for confirmation. Use sparingly; supersession via a new decision is usually preferable.

---

## Migration & versioning

### `illuminate migrate`

Run pending schema migrations on `graph.db`.

```
illuminate migrate [--dry-run]
```

Run automatically by `illuminate serve` and `illuminate audit`. Manual invocation is for advanced users.

---

## Examples

### Day-zero setup

```bash
cd payments-service
illuminate init --claude --interactive
# answers a few questions, runs bootstrap, downloads models
illuminate audit "add Redis caching to txn lookup"
# returns a warning about the no-Redis decision
```

### Daily use (mostly invisible)

The daemon (`illuminate serve --daemon`) runs in the background. The dev opens Claude Code, types prompts, accepts/rejects suggestions. Behind the scenes:

- Trails are captured.
- The MCP server fields `illuminate_audit` calls.
- After session end, the extractor runs.
- New decisions appear in `wiki/decisions/` (auto-merged or queued for review).

### Onboarding a new dev

```bash
git clone acme/payments-service
cd payments-service
illuminate init --no-bootstrap          # wiki already exists in git
illuminate wiki serve
# browser opens at localhost:8765
```

### CI gate (GitHub Actions)

```yaml
- name: Illuminate audit
  run: illuminate audit-pr ${{ github.event.pull_request.number }} --repo ${{ github.repository }} --comment
```

Fails the build if any `error`-severity finding triggers.

---

## Shell completion

```bash
illuminate completions bash > /etc/bash_completion.d/illuminate
illuminate completions zsh > "${fpath[1]}/_illuminate"
illuminate completions fish > ~/.config/fish/completions/illuminate.fish
```

---

## What the CLI does NOT do

- Modify source code. The CLI is read-only with respect to the codebase. Wiki writes are the only output.
- Make outbound network calls (except for `models download` and configured LLM provider).
- Auto-update. Use `cargo install --git` or `brew upgrade` to update.
- Send telemetry. None.
