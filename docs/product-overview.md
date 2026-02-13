# Illuminate

### Your Open Source Contribution Copilot

**Domain:** illuminate.sh
**Version:** 1.0 — Product Specification Document
**Date:** February 2026

---

## Table of Contents

1. Executive Summary
2. Problem Statement
3. Product Vision
4. Target Users
5. Competitive Landscape
6. Core Features
7. User Journeys
8. System Architecture
9. Data Models
10. AI Strategy
11. API & Integrations
12. Monetization
13. Roadmap
14. Success Metrics
15. Risks & Mitigations

---

## 1. Executive Summary

Illuminate is a platform that helps developers discover, contribute to, and track their open source journey — all in one place. It combines AI-powered issue matching with personalized guidance and a contribution portfolio, solving the three biggest pain points new and intermediate open source contributors face: finding the right issue, knowing how to start, and showcasing their work.

Unlike existing tools that simply list issues filtered by GitHub labels, Illuminate understands your skills, matches you to issues you can actually solve, coaches you through the contribution process, and builds a living portfolio of everything you ship.

**One-liner:** Illuminate turns "I want to contribute to open source" into a guided, trackable, rewarding experience.

---

## 2. Problem Statement

### The Contributor's Struggle

Open source contribution is one of the most effective ways to grow as a developer, build a public reputation, and land better opportunities. Yet most developers who want to contribute never make it past the starting line. The reasons are consistent and well-documented:

**Discovery Overload.** GitHub has over 420 million repositories. Tools like goodfirstissue.dev and up-for-grabs.net surface thousands of issues, but dump them as flat, unranked lists. A Python backend developer scrolling through C++ compiler issues and Ruby gem bugs learns nothing and wastes time. There is no skill-aware matching.

**The "Now What?" Wall.** A developer finds an interesting issue. They open the repository. It has 2,000 files, no architecture guide, and the issue description references three internal modules they've never seen. They close the tab. This happens constantly — the gap between "I found an issue" and "I know how to start" is where most contributors are lost.

**Fear of Saying the Wrong Thing.** Open source has social norms that aren't obvious. New contributors worry about asking dumb questions, duplicating work, or violating unwritten rules. They need help knowing what to ask and how to communicate in an issue thread before writing a single line of code.

**Invisible Contributions.** GitHub's contribution graph shows green squares — activity, not impact. There is no native way to see "I contributed to FastAPI, Next.js, and three other projects, merging 12 PRs across them." Developers who want to showcase their open source work have to manually curate it, and most don't bother.

**No Progression System.** Contributing to open source feels like a random walk. There's no sense of growth, no suggested next step, no path from "fix a typo" to "implement a feature" to "become a maintainer." Each contribution exists in isolation.

### Why Existing Tools Fall Short

| Tool | What It Does | What It Misses |
|---|---|---|
| goodfirstissue.dev | Lists issues with "good first issue" label | No skill matching, no guidance, no tracking |
| up-for-grabs.net | Aggregates beginner-friendly projects | Same as above — it's a directory, not a tool |
| CodeTriage | Emails you one issue/day from subscribed repos | Random selection, no personalization |
| Gitmatch | Recommends repos based on your GitHub profile | Recommends repos, not issues. No guidance |
| gh-oss-stats | CLI to query your contribution history | Developer-only, no UI, no portfolio, no discovery |
| GitHub Explore | Shows trending repos and topics | Built for browsing, not for structured contribution |

None of these tools connect the full loop: discover → understand → contribute → track → grow.

Illuminate does.

---

## 3. Product Vision

### The Illuminate Loop

Illuminate is built around a continuous cycle that turns casual interest into consistent contribution:

```
  ┌─────────────┐
  │   DISCOVER   │ ← AI matches issues to your skills & goals
  └──────┬───────┘
         │
         ▼
  ┌─────────────┐
  │  UNDERSTAND  │ ← AI breaks down the issue, codebase, and approach
  └──────┬───────┘
         │
         ▼
  ┌─────────────┐
  │  CONTRIBUTE  │ ← Guided questions, PR templates, etiquette coaching
  └──────┬───────┘
         │
         ▼
  ┌─────────────┐
  │    TRACK     │ ← Auto-detected contributions, portfolio, stats
  └──────┬───────┘
         │
         ▼
  ┌─────────────┐
  │    GROW      │ ← Reflections, skill progression, harder challenges
  └──────┴───────┘
         │
         └──────→ Back to DISCOVER (with updated profile)
```

### Design Principles

**Skill-first, not label-first.** The system understands what you know, not just what GitHub labels say. An issue labeled "good first issue" in a Rust async runtime is not beginner-friendly for a JavaScript developer. Illuminate knows the difference.

**Guidance over gatekeeping.** The goal is to make contributors feel prepared, not to automate their contributions. Illuminate teaches you how to fish — it explains the codebase, suggests an approach, and generates questions to ask. It never submits code or PRs on your behalf.

**Quality over quantity.** Illuminate favors well-maintained repos with responsive maintainers and clear contribution guides. It actively steers users away from dead projects, hostile communities, and issues that have been open for years with no movement.

**Your story, your data.** Contribution history belongs to the user. Export everything. The portfolio is shareable, embeddable, and never locked behind a paywall.

---

## 4. Target Users

### Primary: The Aspiring Contributor

- Has 6 months to 3 years of coding experience
- Knows Git basics, has a GitHub account, has built personal projects
- Wants to contribute to open source but doesn't know where to start
- Motivated by learning, resume building, and community belonging
- Typically a student, bootcamp grad, or early-career developer

### Secondary: The Occasional Contributor

- Has contributed to 1–5 projects sporadically
- Wants to be more consistent but loses momentum between contributions
- Needs discovery, reminders, and tracking more than guidance
- Motivated by building a public portfolio and deepening skills

### Tertiary: The Strategic Contributor

- Experienced developer who contributes to open source intentionally
- Wants to target specific ecosystems (e.g., AI/ML, DevOps, Web frameworks)
- Values the tracking and portfolio features more than the guidance
- May use Illuminate to prepare for job interviews or showcase expertise

---

## 5. Competitive Landscape

### Direct Competitors

No tool currently occupies the exact space Illuminate targets. The closest alternatives are fragmented across different parts of the contribution lifecycle:

**Issue Discovery:** goodfirstissue.dev, goodfirstissues.com, up-for-grabs.net, CodeTriage — all label-based aggregators with zero personalization.

**Repo Recommendation:** Gitmatch (Open-Source-Recommender) — uses embeddings to match your GitHub profile to repos, but stops at repo-level recommendations. No issue matching, no guidance.

**Contribution Tracking:** gh-oss-stats — a CLI tool that queries GitHub's Search API to compile your external contributions. No UI, no portfolio, no discovery.

**General GitHub Tooling:** GitHub's own Explore page, Copilot suggestions, and the new AI issue triage action. These are built for maintainers, not contributors.

### Illuminate's Differentiation

Illuminate is the only tool that combines personalized issue discovery, AI-powered contribution guidance, and a contribution portfolio into a single platform. The closest analog isn't a tool — it's having a mentor who knows your skills, follows open source projects, and walks you through your first few contributions.

---

## 6. Core Features

### 6.1 — Skill Profile & Onboarding

**What it does:** Builds a comprehensive understanding of who you are as a developer — your languages, frameworks, domains, comfort level, and goals.

**How it works:**

- Connect via GitHub OAuth. Illuminate reads your public repositories, starred repos, and contribution history.
- The system analyzes your repos to extract: primary languages (weighted by recency and LOC), frameworks and libraries used, domain signals (web, CLI, data, ML, DevOps, etc.), and code complexity patterns.
- You complete a short onboarding flow to fill gaps the profile analysis can't catch: your comfort level (beginner / intermediate / advanced), how much time you want to invest (1–2 hours/week, a weekend project, etc.), your goals (learn a new language, build portfolio, give back, get hired), and any technologies you're learning or want to explore.
- The resulting profile is a living document — it updates as you contribute and as your GitHub activity evolves.

**Why it matters:** Everything downstream (issue matching, difficulty calibration, guidance depth) depends on an accurate skill profile. This is the foundation of personalization.

---

### 6.2 — Smart Issue Matching

**What it does:** Surfaces a curated, ranked feed of open source issues tailored to your skills, goals, and available time.

**How it works:**

- Illuminate continuously indexes issues from a curated set of high-quality open source repositories (see Repository Quality Criteria below).
- For each issue, the system extracts: required languages and frameworks (from repo metadata + issue text), estimated difficulty (based on issue description complexity, referenced files, and label signals), domain tags, and time estimate.
- Issues are scored against your profile using a multi-factor ranking algorithm. Key factors include skill match (how well the issue aligns with what you know), growth match (does it push you slightly beyond your comfort zone, in directions you've expressed interest in), repo health score (maintainer responsiveness, PR merge rate, recent commit activity, community size), issue freshness (recently opened issues are preferred), and competition (issues with fewer existing comments or linked PRs rank higher).
- Results are presented as a personalized feed — not a flat list. Each issue card shows the repo name and description, the issue title and a one-line AI summary, match score and match reasons (e.g., "Uses Python + Flask, which you know well"), estimated difficulty and time, and repo health indicators (last commit, avg PR review time, contributor count).

**Repository Quality Criteria:**

Not all repos are worth contributing to. Illuminate filters for repos that have a clear CONTRIBUTING.md or contribution guide, show recent commit activity (at least one commit in the last 30 days), have a track record of merging external PRs (not just internal team commits), respond to issues within a reasonable timeframe (median < 7 days), and use standard labeling (good-first-issue, help-wanted, bug, enhancement, etc.).

**Filtering & Controls:**

Users can filter by language, framework or ecosystem, difficulty level, estimated time commitment, issue type (bug fix, feature, docs, testing, refactor), and repo size (small/medium/large).

---

### 6.3 — Issue Deep Dive & AI Guidance

**What it does:** When you select an issue, Illuminate becomes your mentor. It breaks down the project, explains the issue in context, suggests an approach, and generates smart questions you can ask before writing any code.

**This is the killer feature — it's what no other tool offers.**

**The Deep Dive Page contains five sections:**

**Section A — Project Overview:**
A concise summary of what the project does, who maintains it, what tech stack it uses, and how contributions are typically handled. This is generated from the repo's README, CONTRIBUTING.md, and recent activity. Think of it as a "briefing" before you enter the codebase.

**Section B — Issue Context:**
A plain-language explanation of the issue. What's broken or missing? Why does it matter? What part of the codebase is likely involved? The AI reads the issue description, any linked issues or PRs, and referenced files to produce this summary. If the issue references specific files or modules, Illuminate identifies and explains them.

**Section C — Suggested Approach:**
A step-by-step plan for tackling the issue. This isn't code — it's strategy. For example: "1. Set up the dev environment following the CONTRIBUTING.md guide. 2. Locate the authentication middleware in /src/middleware/auth.ts. 3. The bug is likely in how session tokens are validated — compare the current logic against the OAuth2 spec linked in issue #342. 4. Write a failing test first, then fix the logic. 5. Run the existing test suite to check for regressions."

**Section D — Questions to Ask:**
This is critical for new contributors. Illuminate generates 3–5 thoughtful questions you can post in the issue thread before starting work. Examples: "Is there a preferred approach for handling this edge case, or is it open to the contributor's judgment?" or "I see this module also touches the caching layer — should I keep the scope limited to the auth logic, or is updating the cache behavior in scope?" or "Are there any related issues or prior attempts at this that I should review first?"

These questions serve two purposes: they show the maintainer you've done your homework, and they prevent you from going down the wrong path.

**Section E — Red Flags & Warnings:**
Illuminate warns you if the issue has been open for over 6 months with no maintainer response, if there's already an open PR addressing it (potential duplicate), if the repo's last commit was months ago (possibly abandoned), or if the issue description is vague and may need clarification before starting.

---

### 6.4 — Contribution Tracker & Portfolio

**What it does:** Automatically detects and records your open source contributions, then presents them as a beautiful, shareable portfolio.

**How it works:**

- Illuminate periodically queries GitHub's API (using your OAuth token) to detect merged PRs to external repositories (repos you don't own), issues you've opened or commented on, and repos you've been added as a collaborator to.
- Each detected contribution is enriched with the repo's description and star count, the PR title and description, lines added/removed, files changed, and the date range of your involvement.
- Contributions are organized into a portfolio with multiple views.

**Portfolio Views:**

- **Timeline View:** A chronological feed of all your contributions, showing your OSS journey as it unfolds.
- **Project View:** Grouped by repository — see all your contributions to a single project in one place, with aggregate stats (total PRs, total lines, first/last contribution date).
- **Stats Dashboard:** Total projects contributed to, total PRs merged, total lines of code contributed, languages used (with proportions), contribution streak (longest consecutive weeks with at least one contribution), and most active ecosystems.
- **Shareable Profile Page:** A public URL at illuminate.sh/[username] that showcases your portfolio. This page is designed to be linked from your resume, GitHub README, LinkedIn, or personal website.
- **Embeddable Widget:** An SVG badge or HTML embed that displays your contribution summary, similar to GitHub's contribution graph but focused on external OSS work.

**Manual Additions:**

Not all contributions happen through PRs. Users can manually log contributions like participating in code reviews, reporting bugs, writing documentation externally, mentoring other contributors, and speaking about or writing about an OSS project.

---

### 6.5 — Growth Engine

**What it does:** Tracks your progression as a contributor and suggests a path from beginner to seasoned open source developer.

**How it works:**

- After each contribution, Illuminate prompts a quick (optional) reflection: What did you learn? What was challenging? Would you contribute to this project again?
- These reflections, combined with your contribution history, feed into a growth model that tracks your skill evolution over time.

**Progression Levels:**

- **Explorer** — You've set up your profile and are browsing issues.
- **First Light** — You've made your first contribution (PR merged or meaningful issue filed).
- **Contributor** — You've contributed to 3+ projects with merged PRs.
- **Regular** — You've maintained a contribution streak of 4+ weeks.
- **Specialist** — You've made 5+ contributions to a single project (showing depth, not just breadth).
- **Luminary** — You've been recognized by a project (added as collaborator, mentioned in changelog, etc.) or have contributed to a major project (10k+ stars).

**Skill Radar:**
A visual representation of your skill development across dimensions like frontend, backend, DevOps, testing, documentation, and specific language ecosystems. Updated as you contribute across different types of issues.

**Suggested Next Steps:**
Based on your current level and goals, Illuminate suggests what to try next. If you've only done bug fixes, it suggests a feature implementation. If you've only contributed to small projects, it suggests a larger, more impactful one. If you've been doing Python exclusively, and you expressed interest in learning Go, it surfaces Go issues at your level.

---

### 6.6 — Watchlist & Notifications

**What it does:** Lets you save issues you're interested in and stay informed about changes.

**Notifications include:**

- An issue you saved was claimed by someone else or a PR was opened for it.
- A maintainer responded to an issue you're watching.
- A new issue was posted in a project you follow that matches your skills.
- Your open PR received a review or comment.
- Weekly digest: "Here are 5 new issues matched to your profile this week."

**Delivery channels:** In-app notifications, email digest (configurable frequency), and optionally browser push notifications.

---

## 7. User Journeys

### Journey 1: First-Time Contributor (Priya)

Priya is a computer science student who has built a few Python projects. She wants to contribute to open source to strengthen her resume but has no idea where to start.

1. She visits illuminate.sh and signs in with GitHub.
2. Illuminate analyzes her repos and identifies Python, Flask, and basic SQL as her strengths.
3. During onboarding, she selects "Beginner" comfort level, "2–3 hours/week" time commitment, and "Build my portfolio" as her goal.
4. Her feed shows 15 matched issues, sorted by relevance. The top result is a bug in a Flask-based API framework with 2k stars, labeled "good first issue," with a match score of 94%.
5. She clicks into the issue. The Deep Dive explains the project, breaks down the bug, and suggests she start by reading the test file for the affected module. It generates 3 questions she can ask in the issue thread.
6. She posts the questions. The maintainer responds within a day with helpful context.
7. She submits a PR. It gets merged.
8. Illuminate auto-detects the merged PR and adds it to her portfolio. She's now at "First Light" level. A reflection prompt appears: "What did you learn?"
9. The next time she opens Illuminate, her feed has already adjusted — slightly harder issues, same ecosystem.

### Journey 2: Returning Contributor (Marcus)

Marcus has contributed to a few projects over the past year but does it inconsistently. He forgets which projects he's contributed to and has no way to showcase his work.

1. He signs in to Illuminate. His GitHub history is analyzed — contributions to 4 external repos detected.
2. His portfolio is auto-populated: 7 merged PRs across 4 projects, 340 lines added.
3. He shares his illuminate.sh/marcus profile on LinkedIn.
4. He enables weekly digest emails. Every Monday, he gets 5 new issue recommendations.
5. He sets a personal goal: contribute to 2 new projects this month. Illuminate tracks his progress.
6. After hitting his goal, the Growth Engine suggests he try a larger project or a different language.

---

## 8. System Architecture

### High-Level Overview

```
┌──────────────────────────────────────────────────────────────┐
│                        CLIENT LAYER                          │
│                                                              │
│   ┌─────────────────┐  ┌──────────────┐  ┌──────────────┐   │
│   │    Web App       │  │  Public       │  │  Embeddable  │   │
│   │  (illuminate.sh) │  │  Portfolio    │  │  Widget      │   │
│   │                  │  │  Pages        │  │  (SVG/HTML)  │   │
│   └────────┬─────────┘  └──────┬───────┘  └──────┬───────┘   │
│            └───────────────────┼──────────────────┘           │
└────────────────────────────────┼──────────────────────────────┘
                                 │
                                 ▼
┌──────────────────────────────────────────────────────────────┐
│                        API GATEWAY                           │
│              Authentication, Rate Limiting, Routing           │
└────────────────────────────────┬─────────────────────────────┘
                                 │
        ┌────────────┬───────────┼───────────┬─────────────┐
        ▼            ▼           ▼           ▼             ▼
┌──────────┐  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐
│  User    │  │  Issue   │ │ Guidance │ │ Portfolio│ │ Growth   │
│  Service │  │  Service │ │ Service  │ │ Service  │ │ Service  │
│          │  │          │ │          │ │          │ │          │
│ Profiles │  │ Matching │ │ AI Mentor│ │ Tracking │ │ Progress │
│ Auth     │  │ Indexing │ │ Deep Dive│ │ Stats    │ │ Levels   │
│ Prefs    │  │ Ranking  │ │ Q&A Gen  │ │ Export   │ │ Suggest  │
└────┬─────┘  └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘
     │             │            │             │            │
     └─────────────┴──────┬─────┴─────────────┴────────────┘
                          │
              ┌───────────┼───────────┐
              ▼           ▼           ▼
        ┌──────────┐ ┌──────────┐ ┌──────────┐
        │ Primary  │ │  Cache   │ │  AI      │
        │ Database │ │  Layer   │ │  Layer   │
        │(Postgres)│ │ (Redis)  │ │(Claude   │
        │          │ │          │ │  API)    │
        └──────────┘ └──────────┘ └──────────┘
              ▲                         ▲
              │                         │
        ┌──────────┐              ┌──────────┐
        │  GitHub  │              │  Issue   │
        │  API     │              │  Index   │
        │ (OAuth + │              │ (Search  │
        │  REST/   │              │  Engine) │
        │  GraphQL)│              │          │
        └──────────┘              └──────────┘
```

### Service Breakdown

**User Service** — Handles GitHub OAuth authentication, manages user skill profiles, stores preferences (comfort level, goals, time commitment, filters), and keeps profiles updated as GitHub activity changes.

**Issue Service** — The engine of Illuminate. Runs background jobs to crawl and index issues from qualifying repositories. Parses issue metadata (labels, assignees, linked PRs, timestamps). Computes repo health scores. Runs the matching algorithm against user profiles to produce personalized, ranked feeds.

**Guidance Service** — Orchestrates the AI layer for issue deep dives. When a user clicks into an issue, this service fetches the issue details, repo context (README, CONTRIBUTING.md, referenced files), and sends structured prompts to the AI layer. Returns the five-section deep dive (project overview, issue context, approach, questions, red flags).

**Portfolio Service** — Periodically polls GitHub for user contributions (merged PRs, issues, comments). Enriches raw contribution data with repo metadata. Computes aggregate stats. Generates shareable profile pages and embeddable widgets. Handles manual contribution logging.

**Growth Service** — Tracks user progression levels. Stores reflections. Computes the skill radar. Generates "next step" suggestions based on contribution history and goals.

### Background Workers

**Issue Crawler** — Runs on a schedule (e.g., every 6 hours). Fetches new and updated issues from tracked repositories via GitHub's API. Filters for qualifying issues (open, not assigned, matching label criteria). Stores enriched issue data in the index.

**Contribution Detector** — Runs per-user on a schedule (e.g., daily). Queries GitHub's Search API for merged PRs by the user to external repos. Compares against known contributions to detect new ones. Triggers portfolio updates.

**Profile Refresher** — Runs periodically to re-analyze a user's GitHub repos and update their skill profile. Ensures the matching algorithm reflects their current skills, not stale data.

**Notification Dispatcher** — Processes events (new matched issues, PR reviews, saved issue updates) and delivers notifications via in-app, email, or push based on user preferences.

---

## 9. Data Models

### User Profile

- User ID
- GitHub username, avatar, bio
- OAuth access token (encrypted)
- Skill profile: languages (with proficiency scores), frameworks, domains
- Comfort level: beginner / intermediate / advanced
- Time commitment preference
- Goals (multi-select)
- Technologies learning or exploring
- Notification preferences
- Created at, updated at

### Issue (Indexed)

- Issue ID (GitHub issue ID + repo)
- Repository: name, owner, stars, description, primary language, health score
- Issue title, body (raw + AI summary)
- Labels
- Difficulty estimate (computed)
- Time estimate (computed)
- Required skills (extracted)
- Domain tags
- Status: open / claimed / closed
- Linked PRs count
- Maintainer response time (for this issue)
- Freshness score
- Indexed at, last checked at

### Contribution

- Contribution ID
- User ID
- Type: merged PR / issue filed / code review / manual
- Repository: name, owner, stars
- PR/Issue title, URL, description
- Lines added, lines removed, files changed
- Date opened, date merged
- Reflection text (optional)
- Skills demonstrated (extracted)

### Watchlist Item

- User ID
- Issue ID
- Saved at
- Status: watching / claimed / completed / abandoned
- Notes (user-added)

### Growth Record

- User ID
- Current level
- Total contributions count
- Streak (current, longest)
- Skill radar scores (by dimension)
- Level-up history (timestamps)

---

## 10. AI Strategy

### Where AI Is Used

Illuminate uses AI in four specific, well-scoped areas. The AI is a tool within the system, not the system itself.

**1. Skill Extraction from Repos**
Analyze a user's GitHub repositories to identify languages, frameworks, and domains. This goes beyond GitHub's built-in language detection (which is purely LOC-based) by reading package manifests (package.json, requirements.txt, go.mod, etc.), import patterns, and project structure.

**2. Issue Analysis & Difficulty Estimation**
Parse issue descriptions to extract required skills, estimate complexity, and summarize in plain language. This involves reading the issue body, any linked code or files, related issues, and the repo's structure. Output: skill tags, difficulty score (1–5), time estimate, one-line summary.

**3. Deep Dive Generation**
The core AI feature. Given an issue and its repo context, generate the five-section deep dive: project overview, issue context, suggested approach, questions to ask, and red flags. This is a structured generation task with clear inputs and outputs.

**4. Growth Suggestions**
Based on a user's contribution history, skill radar, and stated goals, suggest what type of issue or project to try next. This is a lighter AI task — more recommendation logic than generation.

### AI Principles

**Transparency.** Every AI-generated section is clearly labeled. Users know what's human-written (issue descriptions, README content) and what's AI-generated (summaries, approach suggestions, questions).

**No code generation.** Illuminate never writes code for the user. The guidance is strategic — it explains the approach, not the implementation. This is a deliberate choice: the goal is to make the user a better contributor, not to automate contributions.

**Fallback gracefully.** If the AI can't generate a useful deep dive (e.g., the issue is too vague, the repo has no documentation), the system says so honestly and suggests the user ask for clarification in the issue thread.

**Human-in-the-loop.** AI suggestions are starting points. Users can edit, discard, or refine any generated content before posting it anywhere.

---

## 11. API & Integrations

### GitHub Integration (Primary)

Illuminate's GitHub integration is the backbone of the platform. It uses:

- **OAuth App** for authentication and user authorization.
- **REST API v3** for fetching repository metadata, issue details, and user contribution history.
- **GraphQL API v4** for efficient bulk queries (e.g., fetching all issues with specific labels across multiple repos in a single request).
- **Webhooks (future)** for real-time updates on watched issues and user PRs.

**Required OAuth Scopes:** read:user (profile info), public_repo (read access to public repos and issues), and optionally notifications (for PR review alerts).

### Illuminate Public API (Future — V2+)

Expose a public API so developers can programmatically access their portfolio data, embed contribution stats in their own sites or tools, and integrate with other developer platforms (e.g., pull Illuminate portfolio data into a personal website build pipeline).

### Potential Third-Party Integrations

- **GitLab / Codeberg** — Extend beyond GitHub (V2+).
- **Discord / Slack** — Deliver notifications and weekly digests to developer communities.
- **LinkedIn** — Allow users to publish contribution summaries as LinkedIn posts or add to their profile.
- **VS Code Extension** — Surface matched issues and deep dives directly in the editor sidebar.
- **Browser Extension** — Overlay Illuminate data (match score, deep dive link, repo health) on GitHub issue pages.

---

## 12. Monetization

### Free Tier (Core)

The free tier should be generous enough that most individual contributors never need to pay. It includes:

- Full skill profile and onboarding
- Issue matching feed (up to 20 results per refresh)
- 5 AI deep dives per month
- Contribution tracking and portfolio
- Public profile page at illuminate.sh/[username]
- Growth levels and progression

### Pro Tier (~$8/month)

For serious contributors who want unlimited access and deeper features:

- Unlimited AI deep dives
- Priority issue matching (faster refresh, larger pool)
- Advanced filters (repo size, maintainer response time, specific orgs)
- Custom portfolio themes and domain linking
- Email digest customization
- Export contribution data as JSON/CSV
- Skill radar and detailed growth analytics

### Team/Org Tier (Future)

For bootcamps, coding schools, developer communities, and companies running open source programs:

- Dashboard for tracking multiple contributors
- Curated issue lists for specific learning paths
- Group analytics and leaderboards
- Custom branding for portfolio pages
- API access for integration with internal tools

### Revenue Principles

- Never paywall the portfolio. Your contribution history is yours.
- Never paywall basic issue discovery. The free tier should be useful on its own.
- Monetize depth and convenience, not access.

---

## 13. Roadmap

### Phase 0 — Foundation (Weeks 1–4)

- GitHub OAuth integration
- Skill profile analysis engine
- Basic issue indexing pipeline (start with ~500 curated repos)
- Issue matching algorithm v1 (language + label-based)
- Minimal web UI: onboarding flow, issue feed, issue detail page

### Phase 1 — Core Loop (Weeks 5–10)

- AI deep dive generation (all 5 sections)
- Contribution auto-detection via GitHub API
- Portfolio page: timeline view, project view, stats dashboard
- Public profile pages at illuminate.sh/[username]
- Watchlist and basic notifications
- Repo health scoring system

### Phase 2 — Growth & Engagement (Weeks 11–16)

- Growth engine: progression levels, skill radar
- Reflection prompts after contributions
- "Next step" suggestions
- Weekly digest emails
- Embeddable SVG widget for GitHub README
- Advanced issue filters

### Phase 3 — Polish & Scale (Weeks 17–24)

- Expand repo index to 2,000+ repositories
- Pro tier launch
- Browser extension (GitHub overlay)
- Portfolio themes and customization
- Performance optimization and caching
- Mobile-responsive redesign

### Phase 4 — Expand (6+ months)

- VS Code extension
- GitLab support
- Team/Org tier
- Public API
- Community features (opt-in activity feed)
- Hacktoberfest / GSoC seasonal modes

---

## 14. Success Metrics

### North Star Metric

**Monthly Active Contributors (MAC):** Users who make at least one open source contribution (merged PR or meaningful issue) in a given month, discovered or tracked through Illuminate.

### Supporting Metrics

**Activation Rate:** Percentage of signups who complete onboarding and view at least 3 matched issues within their first session.

**Deep Dive → Contribution Rate:** Percentage of AI deep dives that lead to the user actually opening a PR or commenting on the issue within 7 days.

**Portfolio Adoption:** Percentage of active users who have shared their illuminate.sh profile link at least once (clicked "copy link" or embedded widget).

**Retention (Weekly):** Percentage of users who return to Illuminate at least once per week over a 4-week period.

**Issue Match Quality:** User satisfaction with matched issues, measured by thumbs up/down on issue cards and by whether matched issues are actually attempted.

**Contribution Streak Length:** Average and median streak length across all users — a proxy for whether Illuminate is helping people stay consistent.

### Anti-Metrics (Things to Watch For)

- Users contributing low-quality PRs just to "game" the progression system.
- Over-reliance on AI-generated questions (posting them verbatim without reading the issue).
- Users abandoning issues after claiming them (high watchlist-to-completion drop-off).

---

## 15. Risks & Mitigations

### GitHub API Rate Limits

**Risk:** GitHub's REST API allows 5,000 requests/hour for authenticated users, but the Search API is capped at 30 requests/minute. Heavy indexing and contribution detection could hit limits.

**Mitigation:** Use GraphQL for bulk queries (much more efficient). Cache aggressively. Stagger background jobs. Use conditional requests (If-Modified-Since headers) to avoid redundant fetches. For the issue index, prioritize incremental updates over full re-crawls.

### AI Quality and Hallucination

**Risk:** The AI deep dive could generate incorrect information about a codebase — suggesting the wrong file, misunderstanding the issue, or generating misleading approach suggestions.

**Mitigation:** Ground all AI generation in actual repo data (README, CONTRIBUTING.md, referenced files). Include confidence indicators. Always label AI content clearly. Provide a "Report inaccuracy" button. Never generate code — only strategy and questions.

### Stale Issue Data

**Risk:** Issues get closed, assigned, or resolved between index refreshes. A user could try to contribute to an issue that's already been addressed.

**Mitigation:** Check issue status in real-time when a user clicks into a deep dive (not just from the cache). Show warnings if the issue was recently updated. Include "last checked" timestamps.

### Low-Quality Contribution Incentives

**Risk:** Gamification elements (progression levels, streaks) could encourage users to submit low-quality PRs to level up — exactly the problem GitHub maintainers are currently fighting.

**Mitigation:** Progression is based on merged PRs, not submitted PRs. Only contributions that are accepted by maintainers count. The system tracks acceptance rate and flags users whose PRs are frequently rejected or closed without merge.

### GitHub Policy Changes

**Risk:** GitHub could restrict API access, change OAuth scopes, or launch a competing feature.

**Mitigation:** Minimize dependence on any single API endpoint. Store contribution data locally so portfolios survive API changes. Plan GitLab/Codeberg support as a hedge. Focus on the AI guidance layer, which GitHub is unlikely to replicate in the same depth.

### Cold Start Problem

**Risk:** New users with minimal GitHub history get poor skill profiles, leading to bad issue matches.

**Mitigation:** The onboarding flow explicitly asks about skills and goals to fill gaps. Allow manual skill additions. Use the first few contributions to rapidly calibrate the profile. Show a "calibrating" state for the first week, with broader recommendations that narrow over time.

---

## Appendix: Naming & Brand

**Name:** Illuminate

**Domain:** illuminate.sh

**Tagline Options:**
- "Light the path to open source."
- "Your open source contribution copilot."
- "Discover. Contribute. Grow."

**Brand Concept:** The name evokes clarity and guidance — shining a light on the right issues, illuminating the path through unfamiliar codebases, and highlighting your contributions for the world to see. The .sh domain reinforces developer identity (shell scripts, terminal culture) while being memorable and short.

---

*This is a living document. Last updated February 2026.*