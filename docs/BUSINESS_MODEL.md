# illuminate — Business Model

**Last updated:** 2026-03-30

---

## Pricing Tiers

### Local (Free, MIT Licensed)

**Target:** Individual developers using AI coding agents.

**Includes:**
- Full CLI + MCP server (12 tools)
- Decision graph (auto-ingestion from git)
- Tiered extraction (GLiNER local + LLM fallback)
- Code indexer (6 languages)
- Contextual linter (illuminate_audit)
- Reflexion loop
- Intent policies (TOML)
- Offline mode
- Single binary, single SQLite file

**Rationale:** The full product works for free on a single machine. This drives bottom-up adoption — same playbook as Docker, Tailscale, Terraform. Individual devs become advocates within their teams.

---

### Team ($20/dev/month)

**Target:** Engineering teams (5-50 developers) who need shared decision awareness.

**Includes everything in Local, plus:**
- **Shared decision graph sync** — merge graphs across team members via illuminate.sh
- **Cross-repo decision awareness** — decisions from repo A surface in repo B
- **ADR extraction from Slack** — keyword-triggered ingestion from configured channels
- **ADR extraction from Jira/Linear** — auto-ingest decision context from tickets
- **Team intent dashboard** — web UI showing intent coverage, active policies, decision timeline
- **Shadow PR reviews** — GitHub Action comments on PRs that contradict decisions
- **Priority support** — email support with 24h response

**Monetization trigger:** When a team has >3 devs using illuminate locally, they hit the pain point: "My graph knows things your graph doesn't." Shared sync solves this.

---

### Enterprise (Custom Pricing)

**Target:** Engineering orgs (50+ developers) with compliance and governance requirements.

**Includes everything in Team, plus:**
- **Cross-org governance** — policy enforcement across multiple teams and repositories
- **Compliance audit logs** — immutable record of all AI-generated code decisions for SOC2/HIPAA
- **Encrypted context sync** — end-to-end encrypted graph sync
- **Architectural drift alerting** — automated alerts when agent activity deviates from policies
- **SSO/SAML** — enterprise identity integration
- **Dedicated support** — SLA-backed support with dedicated account manager
- **Custom entity/relation schemas** — domain-specific extraction models
- **On-premise deployment option** — illuminate.sh hosted in customer's infrastructure

**Monetization trigger:** When an enterprise has AI agents in production across multiple teams, they need governance: "Can we prove our AI agents respected our architecture?"

---

### GitHub Action (Free)

**Target:** Any team using GitHub, regardless of illuminate adoption.

**What it does:**
- Runs as a GitHub Action on PRs
- Comments on architectural drift (not code style)
- "This PR contradicts decision X from PR #847"
- Powered by the same decision graph and audit engine

**Rationale:** Free GitHub Action drives awareness and adoption. Teams discover illuminate through PR comments, then adopt the CLI for real-time agent guarding.

---

## Revenue Model

```
Year 1 target: $120K ARR

Assumptions:
  - 10,000 free tier users (individual devs)
  - 500 team tier users across ~50 teams
  - Average team size: 10 devs
  - Conversion rate: free → team: 5%
  - Churn: 5%/month (settling to 3% by month 6)

Revenue:
  500 team users × $20/mo × 12 = $120,000 ARR

Year 2 target: $500K ARR
  - 2,000 team tier users
  - 5 enterprise customers at ~$50K/year
```

---

## Cost Structure

### Infrastructure costs (Year 1)

| Item | Monthly Cost | Notes |
|------|-------------|-------|
| illuminate.sh hosting | $50 | Static site + API for graph sync |
| GitHub Actions CI/CD | $0 | Free for open source |
| Model hosting (GitHub Releases) | $0 | Binary releases are free |
| Domain (illuminate.sh) | $2 | Annual, amortized |
| **Total** | **~$52/month** | Near-zero infrastructure |

### Why costs are low

illuminate's local-first architecture means:
- No server-side compute for extraction (runs on user's machine)
- No database hosting (SQLite on user's machine)
- No model serving infrastructure (ONNX on user's machine)
- Team sync is lightweight (delta-based graph merge, not real-time streaming)

The only server-side component is the team sync API, which is a thin coordination layer.

---

## Go-to-Market Strategy

### Phase 1: Developer-first adoption (Months 1-3)

1. **Show HN launch** — demonstrate the "aha moment" (agent warned about Redis)
2. **MCP registry listing** — discoverable by Claude Code, Cursor, Windsurf users
3. **Homebrew tap** — `brew install rohansx/tap/illuminate`
4. **Twitter/X technical threads** — extraction benchmarks, architecture deep dives
5. **Dev community talks** — Mumbai meetup, virtual talks

### Phase 2: Team conversion (Months 3-6)

1. **Free GitHub Action** — shadow PR reviews drive team awareness
2. **"Team pain point" content** — blog posts about shared decision graphs
3. **Team tier launch** — illuminate.sh self-serve signup
4. **Integration guides** — Claude Code + Cursor + Windsurf setup

### Phase 3: Enterprise pipeline (Months 6-12)

1. **Compliance case studies** — SOC2/HIPAA audit trail documentation
2. **Enterprise pilots** — free 3-month trial for qualifying orgs
3. **Partner channel** — consulting firms that implement AI governance
4. **Conference presence** — present at AI/DevOps conferences

---

## Key Metrics

| Metric | What it measures | Target (Month 3) |
|--------|-----------------|-------------------|
| GitHub stars | Developer interest | 2,000 |
| CLI installs (Homebrew + binary) | Adoption | 5,000 |
| Weekly active MCP connections | Active usage | 500 |
| Decisions extracted (aggregate) | Product value delivered | 100K |
| Team tier signups | Revenue pipeline | 50 teams |
| GitHub Action installs | Awareness channel | 200 repos |
| NPS (from in-CLI survey) | User satisfaction | >50 |

---

## Expansion Revenue

### Within-team expansion

As illuminate proves value:
1. Individual dev installs (free) → 2. Team adopts shared sync ($20/dev) → 3. More devs join team → 4. Revenue grows with headcount

### Cross-team expansion

Within an enterprise:
1. One team adopts Team tier → 2. Adjacent teams see PR comments → 3. Cross-team decisions surface → 4. Enterprise tier discussion begins

### Feature expansion

Future paid features (Team/Enterprise):
- **Custom extraction models** — fine-tuned NER for domain-specific entities
- **Decision analytics** — trend analysis, drift velocity metrics
- **Agent audit logs** — detailed record of every agent plan and illuminate's response
- **Multi-repo policy inheritance** — org-level policies cascading to repos
- **Slack/Teams bot** — query decisions from chat
