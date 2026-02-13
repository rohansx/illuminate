# Contributing to Illuminate

Thanks for your interest in contributing to Illuminate! This guide will help you get set up and make your first contribution.

---

## Table of Contents

1. [Code of Conduct](#code-of-conduct)
2. [Getting Started](#getting-started)
3. [Development Setup](#development-setup)
4. [Project Structure](#project-structure)
5. [Making Changes](#making-changes)
6. [Code Style](#code-style)
7. [Commit Messages](#commit-messages)
8. [Pull Request Process](#pull-request-process)
9. [Issue Labels](#issue-labels)
10. [Need Help?](#need-help)

---

## Code of Conduct

Be respectful, constructive, and inclusive. We're building a tool to make open source more welcoming — our community should reflect that.

- Be kind to newcomers — everyone starts somewhere
- Give constructive feedback on PRs
- Assume good intent
- No harassment, discrimination, or personal attacks

---

## Getting Started

### Prerequisites

- **[Bun](https://bun.sh) 1.0+** — JavaScript runtime and package manager
- **Go 1.22+** — for the backend API
- **PostgreSQL 16+** — database
- **Redis 7+** — caching layer
- **Git** — version control

### Fork and Clone

```bash
# Fork the repo on GitHub, then:
git clone https://github.com/YOUR_USERNAME/illuminate.git
cd illuminate
```

---

## Development Setup

### Frontend (Landing Page + Web App)

```bash
cd web
bun install
bun dev
```

The dev server runs at `http://localhost:5173` with hot module replacement.

**Useful commands:**

```bash
bun dev            # Start dev server
bun run build      # Production build
bun run preview    # Preview production build locally
bun run check      # Type checking
bun run lint       # Lint with ESLint
bun run format     # Format with Prettier
```

### Backend API (Go)

> Coming soon — the Go API is not yet scaffolded. Check the [roadmap](docs/roadmap.md) for progress.

```bash
cd api
go mod download
go run cmd/server/main.go
```

### Database

```bash
# Create the database
createdb illuminate_dev

# Run migrations (once the API is scaffolded)
cd api
go run cmd/migrate/main.go up
```

### Environment Variables

Copy the example env file and fill in your values:

```bash
cp .env.example .env
```

Required variables:

```
GITHUB_CLIENT_ID=        # GitHub OAuth app client ID
GITHUB_CLIENT_SECRET=    # GitHub OAuth app client secret
DATABASE_URL=            # PostgreSQL connection string
REDIS_URL=               # Redis connection string
CLAUDE_API_KEY=          # Anthropic API key
SESSION_SECRET=          # Random string for session encryption
```

---

## Project Structure

```
illuminate/
├── web/                    # SvelteKit frontend
│   ├── src/
│   │   ├── lib/
│   │   │   └── components/ # Reusable Svelte components
│   │   └── routes/         # SvelteKit file-based routing
│   ├── static/             # Static assets (fonts, images)
│   └── package.json
├── api/                    # Go backend (coming soon)
│   ├── cmd/                # Entry points
│   ├── internal/           # Private application code
│   └── go.mod
├── docs/                   # Documentation
├── CONTRIBUTING.md         # This file
├── LICENSE                 # MIT License
└── README.md
```

---

## Making Changes

### 1. Create a branch

```bash
git checkout -b feat/your-feature-name
```

**Branch naming convention:**

| Prefix | Use |
|--------|-----|
| `feat/` | New feature |
| `fix/` | Bug fix |
| `docs/` | Documentation only |
| `refactor/` | Code change that neither fixes a bug nor adds a feature |
| `test/` | Adding or updating tests |
| `chore/` | Tooling, CI, dependencies |

### 2. Make your changes

- Keep changes focused — one feature or fix per PR
- Write tests for new functionality
- Update documentation if your change affects it

### 3. Test your changes

```bash
# Frontend
cd web
bun run check     # Type check
bun run lint      # Lint
bun run build     # Ensure it builds

# Backend (once available)
cd api
go test ./...
go vet ./...
```

### 4. Push and open a PR

```bash
git push origin feat/your-feature-name
```

Then open a pull request on GitHub.

---

## Code Style

### Frontend (Svelte/TypeScript)

- **Formatting:** Prettier (config in `.prettierrc`)
- **Linting:** ESLint with Svelte plugin
- **TypeScript:** Strict mode enabled
- **CSS:** Plain CSS with custom properties — no CSS frameworks
- **Components:** One component per file, PascalCase naming
- **Imports:** Prefer `$lib/` aliases over relative paths

### Backend (Go)

- **Formatting:** `gofmt` — non-negotiable
- **Linting:** `golangci-lint`
- **Structure:** Follow standard Go project layout
- **Error handling:** Always handle errors explicitly, no silent swallows
- **Naming:** Follow Go conventions (exported = PascalCase, unexported = camelCase)

---

## Commit Messages

Use conventional commits:

```
type(scope): short description

Optional longer description explaining why.
```

**Types:** `feat`, `fix`, `docs`, `refactor`, `test`, `chore`

**Examples:**

```
feat(matching): add language-based issue scoring
fix(portfolio): handle repos with no description
docs: update architecture with caching strategy
chore: upgrade SvelteKit to 2.x
```

Keep the first line under 72 characters.

---

## Pull Request Process

1. **Fill out the PR template** — describe what changed and why
2. **Link related issues** — use "Closes #123" or "Fixes #123"
3. **Keep PRs small** — easier to review, faster to merge
4. **Respond to feedback** — we review PRs promptly and give constructive feedback
5. **Squash if noisy** — we may squash-merge if the commit history is messy

### What we look for in reviews:

- Does it work? Does it break anything?
- Is the code clear and maintainable?
- Are there tests for new behavior?
- Does it follow the existing patterns in the codebase?

---

## Issue Labels

| Label | Description |
|-------|-------------|
| `good-first-issue` | Great for newcomers to the project |
| `help-wanted` | We'd love community help on this |
| `bug` | Something isn't working |
| `enhancement` | New feature or improvement |
| `docs` | Documentation improvement |
| `frontend` | SvelteKit / UI related |
| `backend` | Go API related |
| `ai` | AI/Claude integration related |
| `infra` | Infrastructure, CI/CD, deployment |

---

## Need Help?

- **Open an issue** — describe what you're trying to do and where you're stuck
- **Check existing issues** — someone may have already asked
- **Read the docs** — [architecture](docs/architecture.md) and [product overview](docs/product-overview.md) explain the system

We're building Illuminate to make open source contribution easier. If you find anything confusing about contributing to *this* project, that's a bug — file an issue and we'll fix it.

---

*Thanks for contributing! Every PR matters.*
