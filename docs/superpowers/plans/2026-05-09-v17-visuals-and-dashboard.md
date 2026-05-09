# Plan: v0.17 — serve dashboard, README polish, utkrusht dogfood

**Date:** 2026-05-09
**Workspace:** `/home/rsx/Desktop/projx/illuminate` master, 650 tests passing.

## User intent

> "make it use of this in utkrusht repo and there should be proper visual as well in the repo readme and description and tags, and properly make the readme and fix it with what we are doing and also the serve version should have proper visuals and proper things to show, so think about what else we can show on serve to make it more useful for the company and lets not make it more cli native, it should be easier to connect and do stuffs"

Three streams. The codebase is at v0.16 — the closed loop works, but it's still very CLI-native. This release makes it look and feel like a real product: a browseable dashboard at the wiki serve URL, a polished README, and a real production repo (utkrusht) using it.

## Tasks

### Task JA — `illuminate wiki serve` dashboard

**Files:**
- `crates/illuminate-wiki/src/serve.rs` — extend.
- `crates/illuminate-wiki/src/dashboard.rs` (new) — HTML rendering for dashboard pages.
- `crates/illuminate-wiki/Cargo.toml` — verify `tiny_http`, `pulldown-cmark` already there.
- New tests in `crates/illuminate-wiki/tests/` for the new endpoints.

**Scope.** Today `illuminate wiki serve` renders each markdown page as HTML. Extend it into a small dashboard with these views, all served from the same `tiny_http` server (no client framework, no build step — this stays single-binary):

1. **`/` — Home/dashboard.**
   - Header with project name (from illuminate.toml) and version.
   - Stats cards: "N decisions", "M patterns", "K failures", "X modules", "T total episodes".
   - "Recent activity" — last 10 wiki page updates by `updated:` front-matter, mixed types, with type badges and one-line previews.
   - Quick links to Decisions / Patterns / Failures / Modules.
   - "Run an audit" link to `/audit` playground.
   - "Search" box (top-right) — submits to `/search`.

2. **`/decisions`, `/patterns`, `/failures`, `/modules`** — list views.
   - Filterable: query string `?status=active`, `?tag=caching`, `?module=payments`.
   - Each row: id badge, title, status, tags, updated date, one-line preview.
   - Click row → existing per-page view.

3. **`/page/<id>` — single page view.**
   - The existing per-page render (already exists), but with:
     - Front-matter rendered as a card at top (id, type, status, tags, dates).
     - Body as markdown → HTML.
     - "Related" panel showing other pages with overlapping tags.
     - Permalink/raw markdown link.

4. **`/search?q=...` — search view.**
   - Two columns: "Wiki pages" (substring match across all .md files) and "Graph episodes" (`Graph::search` if a graph is reachable).
   - Each result links to its source.

5. **`/audit` — audit playground (the killer feature).**
   - `<form method="POST" action="/audit">` with a `<textarea name="plan">` and a "Run audit" button.
   - On POST: run `Auditor::audit(plan)` against the same graph + policies, render the `AuditResult` as a styled response page (status badge, policy violations cards, decision conflicts, relevant decisions, blast radius if any).
   - This is the company-facing way to QA a plan without ever touching the CLI.

6. **`/api/*` — JSON endpoints** (for non-MCP integrations):
   - `GET /api/stats` — same JSON the home page uses.
   - `GET /api/pages?type=decision` — wiki page list.
   - `GET /api/page/<id>` — full page (front-matter + body + path).
   - `GET /api/search?q=...` — search results.
   - `POST /api/audit` — JSON in (`{"plan": "..."}`) → `AuditResult` JSON out.

**Styling.** One inline `<style>` block in a base layout:
- System font stack, max-width 980px, sensible spacing.
- Dark mode via `prefers-color-scheme: dark`.
- Mobile-friendly (stack columns on narrow screens).
- Type badges color-coded: decision = blue, pattern = green, failure = red, module = purple.
- Status pills: active = green, superseded = gray, deprecated = orange.

**Tests.**
- `home_page_renders_stats` — start server against tempdir wiki + graph; HTTP GET `/`; assert response contains stats words.
- `decisions_list_filters_by_status`
- `audit_playground_post_returns_violation_for_redis_plan`
- `api_stats_returns_json`
- `api_audit_post_returns_audit_result`

5 tests minimum.

**Deliverable.** Commit `feat(wiki): serve dashboard with stats, browse, search, audit playground, JSON API`.

### Task JB — README + repo metadata polish

**Files:**
- `README.md` — restructure for impact.
- (Optional) `docs/illuminate-architecture.svg` — referenced from README.

**Scope.** The README is dense prose. Make it scannable and visual:

1. **Top of README**: a one-line tagline + 4-5 badges (license, version, MCP supported, Rust 2024).
2. **Hero section**: a labeled ASCII flow diagram showing the loop (capture → graph → audit → wiki → reflect → graph). Replace the current paragraph-form description.
3. **"Try it in 60 seconds"**: condensed install + first-audit block right under the hero.
4. **Screenshots section**: placeholders for:
   - `docs/screenshots/dashboard-home.png` — the new home page from Task JA.
   - `docs/screenshots/audit-playground.png` — the playground showing a violation result.
   - `docs/screenshots/page-view.png` — a decision wiki page rendered.
   - For v0.17, write the README with the markdown image references; note in the task report that the actual PNGs need to be captured against the live serve. Provide a script (`scripts/capture-screenshots.sh`) that starts serve in a tempdir with sample data and outputs the URLs to capture.
5. **Topics/tags suggestion** at bottom of README (copy-paste-ready):
   ```
   knowledge-graph rust mcp ai-agents ci local-first
   linter wiki-as-code claude-code cursor codex
   ```
6. **GitHub repository About panel**: text in a section "Suggested GitHub About text" that the user can copy into the repo settings.

**Don't break existing content.** The current README has good philosophical framing about the three losses, the flywheel, the design decisions. Keep that — just reorganize so visual elements come first.

**Deliverable.** Commit `docs(readme): visual polish — badges, hero diagram, screenshots, topics`.

### Task JC — utkrusht dogfood

**External path:** `/home/rsx/Desktop/utkrusht-ai/Utkrushta` — user's production repo.

**Scope.**
1. `cd` to the utkrusht repo.
2. Run `illuminate init -n utkrusht`.
3. Inspect what bootstrap finds: agent files (CLAUDE.md), ADRs (if any), git history, README, etc.
4. Write a project-specific `.illuminate/illuminate.toml` policy set based on what utkrusht actually cares about (policies the user mentioned earlier: no-raw-sql, no-bare-exceptions, etc).
5. Write a small `.illuminate/interview.yaml` capturing utkrusht-specific decisions.
6. Run `illuminate bootstrap`.
7. Validate by running `illuminate audit` on a realistic plan that should fire (e.g., "add a raw SQL query to the orders endpoint" if no-raw-sql is a policy).
8. Document the setup in a `docs/illuminate-setup.md` IN THE UTKRUSHT REPO so the team knows what was done.
9. **Don't push** to the utkrusht remote — that's a real production repo. Commit locally and surface the diff for the user to review.

**Tests.** No automated tests (this is a setup task on a real repo). Manual verification: bootstrap output, audit response on a known-violation plan.

**Deliverable.** A local commit on utkrusht with the new `.illuminate/` directory + docs. Surface the commit SHA and key file paths in the task report.

## Execution

Order: JA → JB → JC. Each task uses the implementer / spec reviewer / code quality reviewer cycle per the skill. Sequential because JB references JA's screenshots.

## Conventions

- Rust 2024. `cargo fmt --all` clean. `cargo clippy --workspace --all-targets -- -D warnings` clean.
- TDD strict for JA (it's real code with new endpoints).
- Single-line lowercase commit messages.
- Push to `origin/master` for illuminate; **don't** push utkrusht.
- Pre-write `./target/release/illuminate audit "<plan>"` before each source modification.
