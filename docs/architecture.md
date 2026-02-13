# Illuminate — System Architecture

> Technical architecture for illuminate.sh — the open source contribution copilot.

---

## Tech Stack

| Layer | Technology | Rationale |
|---|---|---|
| **Frontend (Landing)** | SvelteKit + Vite | Compiles to vanilla JS, tiny bundles, SSG for static pages, fast DX |
| **Frontend (App)** | SvelteKit | Full-stack framework with SSR, routing, and form handling |
| **Backend API** | Go | High concurrency for background workers, low memory footprint, fast cold starts |
| **Database** | PostgreSQL | Relational data (users, contributions, issues), JSON columns for flexible schemas |
| **Cache** | Redis | Session storage, rate limit counters, hot issue data, feed caching |
| **AI** | Claude API (Anthropic) | Deep dive generation, skill extraction, issue analysis |
| **Search** | PostgreSQL full-text + pg_trgm | Issue search and filtering (Meilisearch as future upgrade path) |
| **Auth** | GitHub OAuth 2.0 | Single sign-on via GitHub, access to user repos and contribution data |
| **Hosting** | Fly.io / Railway | Go API + PostgreSQL + Redis on a single platform, global edge deployment |
| **CDN** | Cloudflare | Static asset caching, DDoS protection, edge caching for portfolio pages |
| **CI/CD** | GitHub Actions | Test, lint, build, deploy on push to main |

---

## High-Level Architecture

```
                    ┌─────────────────────────────────┐
                    │          CLIENTS                 │
                    │                                  │
                    │  Web App    Portfolio   Widget   │
                    │  (SvelteKit) Pages     (SVG)    │
                    └──────────────┬──────────────────┘
                                   │ HTTPS
                                   ▼
                    ┌──────────────────────────────────┐
                    │         API GATEWAY (Go)          │
                    │                                   │
                    │  Auth Middleware → Rate Limiter    │
                    │  → Router → Request Validation    │
                    └──────────────┬───────────────────┘
                                   │
          ┌────────────┬───────────┼───────────┬────────────┐
          ▼            ▼           ▼           ▼            ▼
    ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐
    │   User   │ │  Issue   │ │ Guidance │ │Portfolio │ │  Growth  │
    │ Service  │ │ Service  │ │ Service  │ │ Service  │ │ Service  │
    └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘
         └─────────────┴──────┬─────┴─────────────┴────────────┘
                              │
               ┌──────────────┼──────────────┐
               ▼              ▼              ▼
         ┌──────────┐  ┌──────────┐   ┌──────────┐
         │PostgreSQL│  │  Redis   │   │ Claude   │
         │          │  │          │   │ API      │
         └──────────┘  └──────────┘   └──────────┘
               ▲                            ▲
               │                            │
         ┌──────────┐                 ┌──────────┐
         │ GitHub   │                 │Background│
         │ API      │◄────────────────│ Workers  │
         └──────────┘                 └──────────┘
```

---

## Service Breakdown

### User Service
Handles authentication and profile management.

- **GitHub OAuth flow** — login, token exchange, token refresh
- **Skill profile engine** — analyzes repos via GitHub API, extracts languages/frameworks/domains
- **Onboarding state machine** — tracks completion of profile setup steps
- **Preferences storage** — comfort level, goals, time commitment, notification settings

**Key endpoints:**
```
POST   /auth/github/callback
GET    /api/users/me
PATCH  /api/users/me/profile
PATCH  /api/users/me/preferences
POST   /api/users/me/refresh-profile
```

### Issue Service
The matching engine. Crawls, indexes, and ranks issues.

- **Issue crawler** — background worker fetching issues from tracked repos (every 6h)
- **Repo health scorer** — computes health from commit frequency, PR merge rate, response time
- **Matching algorithm** — multi-factor ranking: skill match, growth match, repo health, freshness, competition
- **Feed generator** — produces personalized, paginated issue feeds per user

**Key endpoints:**
```
GET    /api/issues/feed          (personalized, ranked)
GET    /api/issues/:id
GET    /api/issues/search        (filtered search)
POST   /api/issues/:id/thumbs    (feedback for ranking)
```

### Guidance Service
AI-powered deep dives for individual issues.

- **Context assembler** — fetches issue body, repo README, CONTRIBUTING.md, referenced files from GitHub
- **Prompt builder** — constructs structured prompts for Claude API
- **Deep dive generator** — produces 5-section analysis (project overview, issue context, approach, questions, red flags)
- **Cache layer** — caches generated deep dives (invalidated when issue updates)

**Key endpoints:**
```
GET    /api/issues/:id/deep-dive
POST   /api/issues/:id/deep-dive/regenerate
POST   /api/issues/:id/deep-dive/report   (flag inaccuracy)
```

### Portfolio Service
Tracks contributions and generates shareable profiles.

- **Contribution detector** — background worker polling GitHub Search API for merged PRs
- **Enrichment pipeline** — adds repo metadata, line counts, skill tags to raw contributions
- **Stats aggregator** — computes totals, streaks, language breakdowns
- **Profile renderer** — generates public portfolio pages and embeddable widgets

**Key endpoints:**
```
GET    /api/portfolio/me
GET    /api/portfolio/:username          (public)
GET    /api/portfolio/:username/widget   (SVG embed)
POST   /api/portfolio/contributions      (manual add)
GET    /api/portfolio/me/export           (JSON/CSV)
```

### Growth Service
Progression tracking and recommendations.

- **Level calculator** — determines user level from contribution history
- **Skill radar** — multi-dimensional skill assessment updated per contribution
- **Reflection store** — stores post-contribution reflections
- **Next-step engine** — suggests issues/projects based on current level and goals

**Key endpoints:**
```
GET    /api/growth/me
POST   /api/growth/reflections
GET    /api/growth/suggestions
```

---

## Background Workers

All workers are Go goroutines managed by a job scheduler.

| Worker | Schedule | Description |
|---|---|---|
| **Issue Crawler** | Every 6 hours | Fetches new/updated issues from tracked repos via GitHub GraphQL API |
| **Contribution Detector** | Daily per user | Queries GitHub Search API for merged PRs to external repos |
| **Profile Refresher** | Weekly per user | Re-analyzes GitHub repos to update skill profiles |
| **Notification Dispatcher** | Real-time + batched | Processes event queue and delivers via in-app, email, or push |
| **Repo Health Scorer** | Daily | Recomputes health scores for all tracked repositories |

---

## Data Flow: Issue Matching

```
User opens feed
       │
       ▼
┌─────────────────┐     ┌──────────────────┐
│  User Profile   │────▶│  Matching Engine  │
│  (skills, goals,│     │                  │
│   preferences)  │     │  Score each issue │
└─────────────────┘     │  against profile  │
                        └────────┬─────────┘
                                 │
┌─────────────────┐              │
│  Issue Index    │──────────────┘
│  (cached issues │
│   with metadata)│     Result: ranked feed
└─────────────────┘     sorted by composite score
```

**Scoring formula (v1):**
```
score = (skill_match * 0.35)
      + (growth_match * 0.20)
      + (repo_health * 0.20)
      + (freshness * 0.15)
      + (low_competition * 0.10)
```

---

## Data Flow: AI Deep Dive

```
User clicks issue
       │
       ▼
┌──────────────┐    ┌───────────────┐    ┌────────────┐
│ Check cache  │───▶│ Cache hit?    │─Y─▶│ Return     │
└──────────────┘    └───────┬───────┘    │ cached     │
                            │ N          └────────────┘
                            ▼
                   ┌────────────────┐
                   │ Fetch context  │
                   │ from GitHub:   │
                   │ - Issue body   │
                   │ - README       │
                   │ - CONTRIBUTING │
                   │ - Referenced   │
                   │   files        │
                   └───────┬────────┘
                           ▼
                   ┌────────────────┐
                   │ Build prompt   │
                   │ + send to      │
                   │ Claude API     │
                   └───────┬────────┘
                           ▼
                   ┌────────────────┐
                   │ Parse response │
                   │ into 5 sections│
                   │ + cache result │
                   └───────┬────────┘
                           ▼
                   ┌────────────────┐
                   │ Return deep    │
                   │ dive to client │
                   └────────────────┘
```

---

## Database Schema (PostgreSQL)

Key tables — see [product-overview.md](product-overview.md) for full data models.

```sql
-- Core tables
users
user_skills
user_preferences

-- Issue engine
repositories
issues
issue_skills

-- Contributions
contributions
reflections

-- Growth
growth_records
skill_radar_scores

-- System
watchlist_items
notifications
```

---

## Security

- **OAuth tokens** encrypted at rest (AES-256-GCM)
- **API authentication** via JWT (short-lived) + refresh tokens
- **Rate limiting** per user and per IP (Redis-backed)
- **CORS** restricted to illuminate.sh domains
- **CSP headers** on all pages
- **No secrets in client** — all GitHub API calls proxied through backend
- **Minimal OAuth scopes** — `read:user` + `public_repo` only

---

## Infrastructure

```
┌────────────────────────────────────────────┐
│              Cloudflare CDN                 │
│  (DNS, DDoS protection, static caching)    │
└─────────────────────┬──────────────────────┘
                      │
         ┌────────────┴────────────┐
         ▼                         ▼
┌─────────────────┐      ┌─────────────────┐
│  Static Site    │      │   Go API        │
│  (SvelteKit SSG)│      │  (Fly.io)       │
│  (Cloudflare    │      │                 │
│   Pages)        │      │  PostgreSQL     │
└─────────────────┘      │  Redis          │
                         └─────────────────┘
```

---

*Architecture version 1.0 — February 2026*
