# Plan: v0.8.1 — wire parsed configs to consumers + fix watch parser bug

**Date:** 2026-05-08 (continuation after v0.7.0)
**Workspace:** master, 532 tests passing.

## Context

v0.7 added `[trail]` and `[extraction]` config parsing but the structs are never read by consumers. v0.7 also surfaced an `illuminate-watch::git::parse_git_log` bug (mis-attributes file lists across commit boundaries when `--name-only` is enabled with multiple commits). This batch fixes the bug and wires the parsed configs to actual consumers.

## Tasks

### Task DA — Fix `illuminate-watch::git::parse_git_log` multi-commit bug

**File:** `crates/illuminate-watch/src/git.rs`.

**Bug.** With `git log --format=... --name-only` and multiple commits, output looks like:
```
<hash1>
<author1>
<date1>
<message1>
---END---
file1
file2
<hash2>
<author2>
...
```

The current parser splits on `---END---` then takes the first line of each chunk as the next commit's hash, but the file list of commit N becomes the leading lines of chunk N+1. Result: `<hash2>` ends up with `file1` / `file2` as its hash/author/date.

**Fix.** Adjust the format to put `---END---` AFTER the file list, not before. Or use a different sentinel:
```
git log --format=%H%x00%an%x00%aI%x00%B%x1e --name-only
```
Where `%x00` is NUL (field separator) and `%x1e` is RS (record separator). The bootstrap workaround in `git_history.rs` already uses this — port the same approach.

Or, keep `---END---` but ensure file list comes BEFORE it: `--format=...%n---END---` with `--name-only` causes git to put files between message and END for that commit, then next commit starts. But empirically that's where the interleaving comes from.

**Recommend:** drop `---END---` entirely, use NUL+RS like bootstrap does.

**Tests:** Add to `crates/illuminate-watch/tests/` (or wherever):
- `parse_git_log_handles_multiple_commits_with_files` — verify 3 commits with `--name-only` parse correctly.
- Existing tests should continue to pass.

After the fix, optionally simplify `crates/illuminate-bootstrap/src/git_history.rs` to call `illuminate_watch::git::get_commits_since` instead of shelling out itself. (Decide based on whether the dep makes sense.)

**Deliverable.** Commit `fix(watch): correct git log parser interleaving for multi-commit --name-only output`.

### Task DB — Wire `TrailConfig` to consumers

**Files:**
- `crates/illuminate-trail/src/watcher.rs` — `enabled` and `exclude_patterns`.
- `crates/illuminate-cli/src/commands/trail.rs` — pass config through.

**Goal.** When `[trail].enabled = false`, the trail watcher exits cleanly (or doesn't start). When `[trail].exclude_patterns = ["secrets/**"]`, sessions whose `repo_path` matches any pattern are skipped.

**Implementation:**
1. The CLI's `trail watch` (or `trail register`) command loads `illuminate.toml` somewhere. Add `parse_trail_config` alongside `parse_policies` / `parse_audit_config`. Pass `TrailConfig` through to `WatcherOpts` (or whichever entry point exists).
2. In the watcher loop, before importing each session, check `config.enabled` (skip everything if false) and `config.exclude_patterns` (use a glob match — add `glob = "0.3"` to deps if not present).
3. Print a one-line `tracing::info!` on startup showing the effective config.

**Tests:**
- `watcher_skips_when_enabled_false` — set up a trail dir, run watcher with `enabled=false`, assert no imports happened.
- `watcher_skips_excluded_repo_paths` — exclude_patterns has `["secrets/**"]`, register a session in `<tempdir>/secrets/foo`, assert it's skipped.

**Deliverable.** Commit `feat(trail): trail watcher honors enabled and exclude_patterns from config`.

### Task DC — Wire `ExtractionConfig` to consumers

**Files:**
- `crates/illuminate-cli/src/commands/mod.rs::try_attach_extraction` — read config, pass `confidence_threshold`.
- `crates/illuminate-core/src/graph.rs::load_extraction_pipeline` — accept config or threshold.

**Goal.** Currently `ExtractionPipeline::with_defaults` uses hardcoded `0.5` as threshold. With `[extraction].confidence_threshold = 0.7` in `illuminate.toml`, the pipeline should use 0.7.

**Implementation:**
1. `try_attach_extraction` already reads `illuminate.toml` (or has access to it). Parse `ExtractionConfig` and pass `config.confidence_threshold` to `ExtractionPipeline::new(schema, models_dir, threshold)` (the existing constructor takes a threshold parameter).
2. `signal_threshold` consumer: `crates/illuminate-watch::signal::score_decision_signal` already returns a score. Where is this threshold checked? Find it and wire `config.signal_threshold` through.

**Tests:**
- `extraction_pipeline_uses_configured_threshold` — set `[extraction].confidence_threshold=0.9`, build pipeline, verify confidence cutoff (likely needs accessor on Pipeline or test the behavior).

**Deliverable.** Commit `feat(extract): pipeline uses confidence_threshold from [extraction] config`.

### Task DD — Bootstrap README source

**File:** `crates/illuminate-bootstrap/src/readme.rs` (new), `orchestrate.rs` (extend).

**Goal.** Parse `<repo>/README.md` (and `CONTRIBUTING.md`) for decision-shaped content. Decisions are typically in sections like `## Architecture`, `## Tech Stack`, `## Why X over Y`.

**Implementation:**
1. Walk `<repo>/{README,CONTRIBUTING}.md` (case-insensitive).
2. Split into sections by `^##` headings.
3. Filter to sections whose heading or body contains decision signal phrases (reuse `git_history`'s SIGNAL_PHRASES).
4. Emit each as a `BootstrapCandidate` with `confidence=0.5` (lower than git-history; README content is often outdated).

**Tests:**
- `extracts_architecture_section_from_readme`
- `skips_unrelated_sections`
- `readme_candidates_default_to_low_confidence`

**Deliverable.** Commit `feat(bootstrap): readme and contributing source for architecture sections`.

### Task DE — Final coordinator + tag v0.8.1

Architect verifies. CHANGELOG section. Tag `v0.8.1` (since `v0.8.0` is taken by an old release-ci commit).

## Conventions

Standard. Pre-write audit, TDD, single-line lowercase commits, push origin/master, fmt/clippy all-targets clean.

## Order

DA → DB → DC → DD → DE.
