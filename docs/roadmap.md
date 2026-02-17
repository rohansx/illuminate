# Illuminate — Roadmap & Progress

> Living document tracking what's been built, what's in progress, and what's coming next.

---

## Status Legend

| Icon | Meaning |
|------|---------|
| `[x]` | Shipped |
| `[~]` | In progress |
| `[ ]` | Not started |

---

## Phase 0 — Foundation (Weeks 1–4)

**Goal:** Core infrastructure, auth, and basic issue discovery.

- [x] Project specification document
- [x] Repository setup + MIT license
- [x] Architecture documentation
- [x] Roadmap and progress tracking
- [x] SEO-optimized README
- [x] Contributing guide
- [x] Landing page (illuminate.sh)
- [x] Go API scaffold (`/api` directory)
- [x] PostgreSQL schema + migrations
- [ ] Redis integration
- [x] GitHub OAuth integration
- [x] Skill profile analysis engine
- [x] Basic issue indexing pipeline (~500 curated repos)
- [x] Issue matching algorithm v1 (language + label-based)
- [x] Minimal web app UI: onboarding flow, issue feed, issue detail page

**Milestone:** A user can sign in with GitHub, see a personalized issue feed, and click into an issue.

---

## Phase 1 — Core Loop (Weeks 5–10)

**Goal:** AI guidance, contribution tracking, and public portfolios.

- [~] AI deep dive generation (all 5 sections via Zhipu GLM API)
- [~] Deep dive caching layer
- [ ] Contribution auto-detection via GitHub Search API
- [ ] Portfolio page: timeline view
- [ ] Portfolio page: project view
- [ ] Portfolio page: stats dashboard
- [ ] Public profile pages at illuminate.sh/[username]
- [ ] Watchlist (save issues)
- [ ] Basic notifications (in-app)
- [ ] Repo health scoring system
- [ ] Import repo — user submits a repo URL, system indexes its issues and applies all existing features (matching, deep dive, etc.)
- [ ] Open source hiring tab — discover open source projects that are also hiring contributors

**Milestone:** A user can get AI guidance on an issue, contribute, and see it on their public portfolio.

---

## Phase 2 — Growth & Engagement (Weeks 11–16)

**Goal:** Progression system, email notifications, and embeddable widgets.

- [ ] Growth engine: progression levels (Explorer → Luminary)
- [ ] Skill radar visualization
- [ ] Reflection prompts after contributions
- [ ] "Next step" suggestions
- [ ] Weekly digest emails
- [ ] Embeddable SVG widget for GitHub README
- [ ] Advanced issue filters (repo size, response time, org)
- [ ] Notification preferences UI

**Milestone:** Users have a clear growth path and can embed their contribution stats anywhere.

---

## Phase 3 — Polish & Scale (Weeks 17–24)

**Goal:** Pro tier, performance, and expanded repository coverage.

- [ ] Expand repo index to 2,000+ repositories
- [ ] Pro tier launch ($8/month)
- [ ] Payment integration (Stripe)
- [ ] Browser extension (GitHub overlay)
- [ ] Portfolio themes and customization
- [ ] Performance optimization and aggressive caching
- [ ] Mobile-responsive redesign
- [ ] Rate limit handling and graceful degradation
- [ ] Load testing and scaling strategy

**Milestone:** Illuminate is production-ready with a sustainable revenue model.

---

## Phase 4 — Expand (6+ months post-launch)

**Goal:** Platform expansion and ecosystem integrations.

- [ ] VS Code extension
- [ ] GitLab support
- [ ] Team/Org tier
- [ ] Public API
- [ ] Community features (opt-in activity feed)
- [ ] Hacktoberfest / GSoC seasonal modes
- [ ] Discord / Slack integration
- [ ] LinkedIn portfolio publishing
- [ ] Codeberg support

**Milestone:** Illuminate is a multi-platform open source contribution ecosystem.

---

## Key Decisions Log

| Date | Decision | Rationale |
|------|----------|-----------|
| Feb 2026 | SvelteKit for frontend | Smallest bundle size, fastest compilation, Vite-powered |
| Feb 2026 | Go for backend API | High concurrency for background workers, low memory, fast cold starts |
| Feb 2026 | PostgreSQL over MongoDB | Relational data with strong consistency; JSON columns for flexible fields |
| Feb 2026 | Zhipu GLM API for AI | Free tier with glm-4-flash, OpenAI-compatible format, good structured generation |
| Feb 2026 | Fly.io for hosting | Simple Go deployment, built-in PostgreSQL and Redis, global edge |
| Feb 2026 | Static landing page first | Validate positioning and collect interest before building the full app |

---

## Metrics We'll Track From Day 1

- Landing page visits and GitHub stars
- Waitlist signups (if applicable)
- Time to first contribution after signup
- Deep dive → contribution conversion rate
- Weekly active users returning to feed

---

*Last updated: February 2026*
