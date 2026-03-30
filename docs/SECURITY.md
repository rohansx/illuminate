# illuminate — Security Model

**Last updated:** 2026-03-30

---

## Privacy Architecture

### Principle: Nothing leaves your machine unless absolutely necessary

illuminate is designed so that the vast majority of operations are fully local. The only external communication happens during the LLM fallback path of extraction (~30% of episodes), and even then, PII is stripped first.

```
┌────────────────────────────────────────────────────┐
│                 LOCAL MACHINE                       │
│                                                    │
│  ┌──────────────────────────────────────────────┐  │
│  │ ALWAYS LOCAL (no network)                    │  │
│  │                                              │  │
│  │  - All queries (search, audit, explain, etc) │  │
│  │  - Graph reads and writes                    │  │
│  │  - Code indexing                             │  │
│  │  - Policy evaluation                         │  │
│  │  - ~70% of extraction (local ONNX)          │  │
│  │  - Embedding generation                      │  │
│  │  - Reflexion recording                       │  │
│  └──────────────────────────────────────────────┘  │
│                                                    │
│  ┌──────────────────────────────────────────────┐  │
│  │ SOMETIMES EXTERNAL (~30% of extraction)      │  │
│  │                                              │  │
│  │  Text → CloakPipe PII strip → LLM API       │  │
│  │  (pseudonymized only)                        │  │
│  └──────────────────────────────────────────────┘  │
│                                                    │
│  ┌──────────────────────────────────────────────┐  │
│  │ OPTIONAL EXTERNAL                            │  │
│  │                                              │  │
│  │  - GitHub API (PR ingestion, read-only)      │  │
│  │  - Model downloads (one-time, verified)      │  │
│  └──────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────┘
```

---

## CloakPipe PII Protection

### What gets stripped

Before any text reaches an LLM API, CloakPipe pseudonymizes:

| PII Type | Example | Pseudonym |
|----------|---------|-----------|
| Person names | "Priya Sharma" | PERSON_1 |
| Email addresses | "priya@company.com" | EMAIL_1 |
| Phone numbers | "+91-98765-43210" | PHONE_1 |
| Social Security Numbers | "123-45-6789" | SSN_1 |
| Aadhaar numbers | "1234 5678 9012" | AADHAAR_1 |
| PAN numbers | "ABCDE1234F" | PAN_1 |
| Credit card numbers | "4111-1111-1111-1111" | CC_1 |
| IP addresses | "192.168.1.100" | IP_1 |
| Custom patterns | (configurable regex) | CUSTOM_N |

### How it works

```
1. Input:  "Priya chose Postgres over MongoDB for billing"
2. Strip:  "PERSON_1 chose Postgres over MongoDB for billing"
3. Send:   LLM receives pseudonymized text
4. Receive: LLM returns entities referencing PERSON_1
5. Rehydrate: Map PERSON_1 back to "Priya"
6. Output:  Entity(name="Priya", type=Person)
```

The LLM never sees real names, emails, or identifiers.

### Configuration

```toml
# illuminate.toml
[extraction.cloakpipe]
enabled = true
patterns = ["names", "emails", "phone", "ssn", "aadhaar", "pan"]
custom_patterns = [
    "INTERNAL-\\d{6}",          # Internal ticket IDs
    "[A-Z]{3}-\\d{4}-[A-Z]{2}"  # Custom reference codes
]
```

### Fully offline mode

Set `ILLUMINATE_OFFLINE=1` to disable all LLM calls. Extraction uses only local ONNX models. Quality may be lower for complex episodes, but no data ever leaves the machine.

---

## Data Storage Security

### SQLite encryption (planned)

For enterprise deployments, illuminate will support SQLite encryption via SQLCipher:

```toml
# illuminate.toml (enterprise)
[storage]
encryption = true
key_env = "ILLUMINATE_DB_KEY"
```

### File permissions

illuminate creates files with restrictive permissions:

| File | Permissions | Rationale |
|------|------------|-----------|
| `.illuminate/graph.db` | `0644` | Readable by team (committed to git) |
| `.illuminate/config.toml` | `0600` | May contain local overrides |
| `~/.cache/illuminate/models/` | `0755` | Shared models, read-only after download |

### No telemetry

illuminate does not:
- Phone home
- Collect usage analytics
- Send crash reports
- Track installations
- Require account creation (free tier)

---

## API Key Management

### Principles

1. **Never store API keys in config files** — use environment variables only
2. **Keys are referenced by env var name**, not by value
3. **illuminate.toml stores `key_env`**, not `key`

```toml
# CORRECT
[extraction.llm]
api_key_env = "OPENAI_API_KEY"

# WRONG — illuminate.toml is committed to git
# api_key = "sk-abc123..."
```

### Required keys

| Key | Required? | Scope | Purpose |
|-----|-----------|-------|---------|
| `OPENAI_API_KEY` | Optional | LLM fallback | Extraction of low-confidence episodes |
| `ILLUMINATE_GITHUB_TOKEN` | Optional | GitHub | PR description ingestion (read-only, repo scope) |
| `ILLUMINATE_DB_KEY` | Optional | SQLCipher | Database encryption (enterprise) |

### GitHub token scope

illuminate requires minimal GitHub permissions:
- `repo:read` — read PR descriptions and review comments
- No write access needed
- No admin access needed
- Read-only tokens (fine-grained PATs) recommended

---

## Threat Model

### In scope

| Threat | Mitigation |
|--------|-----------|
| PII leakage via LLM API | CloakPipe strips PII before transmission |
| API key exposure in config | Keys stored in env vars, not files |
| Malicious webhook payloads | Input validation, rate limiting |
| SQLite injection via search | Parameterized queries throughout |
| Model supply chain attack | SHA-256 verification on download |
| Unauthorized graph access | File permissions, SQLCipher (enterprise) |
| Decision tampering | Append-only graph (no deletes or updates) |

### Out of scope (v0.1)

| Threat | Rationale |
|--------|-----------|
| Physical machine access | Local tool — if attacker has machine access, illuminate is not the primary concern |
| Network MitM on LLM calls | HTTPS/TLS is handled by the OS and reqwest |
| Adversarial NER inputs | Low risk for developer tool; text comes from trusted sources (git, PRs) |
| Side-channel timing attacks | Not relevant for local developer tool |

---

## Compliance Readiness

### SOC2

illuminate's architecture supports SOC2 compliance:
- **Confidentiality**: PII stripped before external calls, optional encryption at rest
- **Integrity**: Append-only graph with temporal audit trail
- **Availability**: Works offline, no external dependencies for queries
- **Privacy**: Local-first, CloakPipe protection, no telemetry

### HIPAA

For healthcare teams:
- PHI never reaches LLM APIs (CloakPipe strips all identifiers)
- Offline mode eliminates all external data transmission
- SQLCipher encryption at rest (enterprise)
- Audit trail via bi-temporal history

### GDPR

- No personal data collection by illuminate itself
- CloakPipe pseudonymization before any external transmission
- Right to erasure: while the graph is append-only, entity names can be pseudonymized in place
- No cross-border data transfer when using offline mode

---

## Security Checklist for Deployment

- [ ] Set `ILLUMINATE_OFFLINE=1` in high-security environments
- [ ] Use fine-grained GitHub PATs with minimal scope
- [ ] Store API keys in environment variables, never in files
- [ ] Review `illuminate.toml` before committing (no secrets)
- [ ] Configure CloakPipe custom patterns for org-specific identifiers
- [ ] Set appropriate file permissions on `.illuminate/`
- [ ] Enable SQLCipher for enterprise deployments (when available)
- [ ] Review webhook endpoint access controls if exposed to network
