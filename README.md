# Illuminate

### Your Open Source Contribution Copilot

**[illuminate.sh](https://illuminate.sh)** — Discover the right issues. Get AI-powered guidance. Track your contributions. Grow as a developer.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](CONTRIBUTING.md)

---

## What is Illuminate?

Illuminate is an open source platform that helps developers find, contribute to, and track their open source journey. It combines **AI-powered issue matching** with **personalized contribution guidance** and a **shareable portfolio** — solving the three biggest pain points contributors face:

1. **Finding the right issue** — not just any issue, one that matches your skills
2. **Knowing how to start** — understanding the codebase and the approach before writing code
3. **Showcasing your work** — a living portfolio of everything you've shipped

> Illuminate turns "I want to contribute to open source" into a guided, trackable, rewarding experience.

---

## How It Works

```
  DISCOVER   →   UNDERSTAND   →   CONTRIBUTE   →   TRACK   →   GROW
     │               │                │              │            │
  AI matches     AI breaks down    Guided         Auto-detect   Progression
  issues to      the codebase,     questions,     merged PRs,   levels,
  your skills    suggests an       PR etiquette   build your    skill radar,
  and goals      approach          coaching       portfolio     next steps
```

### Smart Issue Matching

Connect your GitHub account. Illuminate analyzes your repos to understand your languages, frameworks, and domains — then surfaces a personalized feed of open source issues ranked by skill match, growth potential, and repo health. No more scrolling through thousands of irrelevant issues.

### AI-Powered Deep Dives

Click into any issue and get a 5-section mentor experience:
- **Project Overview** — what the project does, who maintains it, how contributions work
- **Issue Context** — plain-language explanation of what's broken or missing
- **Suggested Approach** — step-by-step strategy (not code — strategy)
- **Questions to Ask** — 3–5 smart questions to post before starting work
- **Red Flags** — warnings about stale issues, duplicate PRs, or abandoned repos

### Contribution Tracker & Portfolio

Every merged PR is auto-detected and added to your portfolio at `illuminate.sh/[username]`. Timeline view, project view, stats dashboard — shareable on your resume, LinkedIn, or GitHub README.

### Growth Engine

Progress from **Explorer** to **Luminary** across six levels. Track your skills with a visual radar. Get AI-suggested next steps that push you beyond your comfort zone in the right direction.

---

## Tech Stack

| Layer | Technology |
|---|---|
| Frontend | SvelteKit + Vite |
| Backend API | Go |
| Database | PostgreSQL |
| Cache | Redis |
| AI | Claude API (Anthropic) |
| Auth | GitHub OAuth 2.0 |

---

## Project Structure

```
illuminate/
├── web/                    # SvelteKit frontend (landing page + app)
├── api/                    # Go backend API (coming soon)
├── docs/                   # Documentation
│   ├── product-overview.md # Full product specification
│   ├── architecture.md     # System architecture
│   └── roadmap.md          # Roadmap and progress tracker
├── CONTRIBUTING.md         # How to contribute
├── LICENSE                 # MIT License
└── README.md               # You are here
```

---

## Documentation

- **[Product Overview](docs/product-overview.md)** — Full product specification with features, user journeys, data models, and strategy
- **[Architecture](docs/architecture.md)** — System design, service breakdown, data flows, and infrastructure
- **[Roadmap](docs/roadmap.md)** — Phase-by-phase plan with progress tracking
- **[Contributing Guide](CONTRIBUTING.md)** — How to set up the dev environment and contribute

---

## Getting Started

### Prerequisites

- [Bun](https://bun.sh) 1.0+ — fast JavaScript runtime and package manager
- Go 1.22+ (for backend — coming soon)
- A GitHub account

### Run the Landing Page

```bash
cd web
bun install
bun dev
```

Open [http://localhost:5173](http://localhost:5173).

---

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for setup instructions, code style, and the PR process.

Whether you're fixing a typo or building a new feature, every contribution matters.

---

## Roadmap

See the full [Roadmap](docs/roadmap.md) for detailed progress.

**Current phase: 0 — Foundation**

- [x] Product specification
- [x] Architecture design
- [x] Landing page
- [ ] Go API scaffold
- [ ] GitHub OAuth
- [ ] Issue matching engine
- [ ] Minimal web app

---

## Why "Illuminate"?

The name means to **light the path**. Open source contribution shouldn't feel like stumbling through a dark codebase. Illuminate shines a light on the right issues, illuminates the path through unfamiliar code, and highlights your contributions for the world to see.

The `.sh` domain reinforces developer identity — shell scripts, terminal culture, the tools we use every day.

> **Note:** This project is not affiliated with Laravel's Illuminate components. We are a standalone open source contribution platform at [illuminate.sh](https://illuminate.sh).

---

## License

MIT — see [LICENSE](LICENSE).

---

**Built by [Rohan Sharma](https://github.com/rohansharma) and contributors.**

*Light the path to open source.*
