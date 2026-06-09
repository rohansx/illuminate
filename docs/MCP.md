# Illuminate — MCP Server

The MCP (Model Context Protocol) server is the agent-facing surface. It exposes twelve `illuminate_*` tools — `ask`, `audit`, `decisions_for`, `enrich`, `explain`, `failures_for`, `get_wiki_page`, `impact`, `reflect`, `route`, `stats`, and `symbols` — as JSON-RPC endpoints that Claude Code, Cursor, Codex, and any MCP-aware client can call. (The graph-primitive tools `add_episode`, `search`, `get_decision`, `traverse`, `traverse_batch`, `find_precedents`, `list_entities`, and `export_graph` are also registered.)

For the audit engine itself, see `AUDIT.md`. For CLI usage, see `CLI.md`.

---

## Transports

### stdio (default)

Most agents launch the MCP server as a child process and speak JSON-RPC over stdin/stdout.

```bash
illuminate serve              # starts stdio server (foreground)
```

This is the form Claude Code and Cursor use. The agent's MCP client connects to the running process.

### Streamable HTTP (v0.2)

For remote / containerized deployments where the agent and the server are on different machines.

```bash
illuminate serve --http 127.0.0.1:8765
```

Implements the Streamable HTTP transport per the MCP spec.

---

## Registering with agents

### Claude Code

`illuminate init --claude` writes the registration to `.claude.json` (project-level) and updates `CLAUDE.md`:

```json
{
  "mcpServers": {
    "illuminate": {
      "command": "illuminate",
      "args": ["serve"]
    }
  }
}
```

After init, restart Claude Code. The tools appear in the tool list automatically.

### Cursor

Cursor uses `~/.cursor/mcp.json` (or per-workspace `.cursor/mcp.json`). `illuminate init --cursor` writes:

```json
{
  "servers": {
    "illuminate": {
      "command": "illuminate",
      "args": ["serve"]
    }
  }
}
```

### Codex / other MCP clients

Provide stdio configuration; the agent's docs vary. Generic instructions printed by `illuminate init --codex`.

---

## Tool surface

### `illuminate_audit`

Audit a proposed change.

**Request:**

```json
{
  "method": "illuminate_audit",
  "params": {
    "plan": "Add Redis caching layer to transaction lookup",
    "files": ["services/payments-service/src/cache.rs"],
    "rationale": "p99 latency exceeded SLO",
    "agent": "claude-code",
    "session_id": "abc123"
  }
}
```

**Response:** see `AUDIT.md` for full shape.

### `illuminate_explain`

Explain why a file matters.

**Request:**

```json
{
  "method": "illuminate_explain",
  "params": {
    "path": "services/payments-service/src/cache.rs"
  }
}
```

**Response:**

```json
{
  "module": "payments-service",
  "active_decisions": [
    { "id": "dec-2025-12-no-redis-payments", "title": "...", "wiki_url": "..." }
  ],
  "active_patterns": [
    { "id": "pat-lru-cache-with-ttl", "title": "...", "wiki_url": "..." }
  ],
  "known_failures": [],
  "related_modules": []
}
```

### `illuminate_ask`

Cross-corpus retrieval over decisions / patterns / failures / sessions / ingested docs / trail. Shipped v0.22 as the MCP companion to the `illuminate ask` CLI verb — the agent gets a structured envelope (no LLM synthesis on Illuminate's side) and can either summarize it itself or pass it through v3.3's planned synthesis layer.

**Request:**

```json
{
  "method": "illuminate_ask",
  "params": {
    "question": "why did we reject Redis for the payments service?",
    "limit": 20
  }
}
```

**Response:**

```json
{
  "question": "why did we reject Redis for the payments service?",
  "hits": [
    {
      "kind": "decision",
      "id": "019e1190-913b-7b32-a2d7-3fe57efca2f5",
      "title": "Caching — never use Redis",
      "snippet": "[dec-bs-claude-md-caching-never-use-redis] Caching — never use Redis ...",
      "source": "wiki",
      "score_bucket": "high"
    },
    {
      "kind": "ingested_doc",
      "id": "019e64ea-048a-7e02-ba2a-aca143ebe405",
      "title": "Getting started with Illuminate",
      "snippet": "...",
      "source": "ingested:local-docs",
      "score_bucket": "high"
    }
  ],
  "hit_count": 11,
  "empty_kinds": ["pattern", "failure", "module", "session"]
}
```

`kind` is one of `decision` / `pattern` / `failure` / `module` / `session` / `ingested_doc` / `trail_episode` / `other`. `score_bucket` is one of `high` / `med` / `low` / `min`. `empty_kinds` lists the kinds with zero hits — useful for the agent's "no published sessions on this topic yet" callouts and (in v3.3) for the LLM synthesis prompt.

### `illuminate_enrich`

Deterministic pre-LLM prompt enrichment. Surfaces relevant decisions, patterns, failures, and code paths so the next generation step starts from team context instead of a blank prompt. No LLM in the path — same `(prompt, graph state)` produces a byte-identical response (the `graph_state_hash` field is the SHA-256 receipt).

Shipped in v0.19 as the first half of the v3 two-product positioning (see `PRODUCT_OVERVIEW.md` → Illuminate Enrich).

**Request:**

```json
{
  "method": "illuminate_enrich",
  "params": {
    "prompt": "add Redis caching to the txn endpoint",
    "files": ["src/payments/txn.rs"],
    "max_bytes": 4096
  }
}
```

**Response:**

```json
{
  "enriched_prompt": "# Team context (from illuminate)\n\n## Relevant decisions\n- [dec-no-redis](.illuminate/wiki/decisions/dec-no-redis.md)\n  Team rejected Redis for this service 3 months ago...\n\n## Patterns\n- [pat-lru-30s] LRU with 30s TTL\n\n---\n# Original prompt\nadd Redis caching to the txn endpoint",
  "injections": [
    { "source": "decision", "id": "dec-no-redis", "wiki_url": ".illuminate/wiki/decisions/dec-no-redis.md", "content": "...", "score_bucket": "high" },
    { "source": "pattern", "id": "pat-lru-30s", "wiki_url": ".illuminate/wiki/patterns/pat-lru-30s.md", "content": "...", "score_bucket": "med" }
  ],
  "graph_state_hash": "c81828ca275fe0611a7721eada6232ec3e2e3dad99125bae87ef0686de9d83b2"
}
```

| Field | Type | Notes |
|-------|------|-------|
| `prompt` | string (required) | The developer's raw prompt. |
| `files` | string[] (optional) | File-path hints; narrows code-graph queries. |
| `max_bytes` | integer (optional) | Soft cap on injected content; trailing injections drop deterministically when over budget. Default 4096. |

**Determinism guarantee.** Same `(prompt, graph state)` → byte-identical `enriched_prompt`. Test coverage in `crates/illuminate-enrich/src/lib.rs::determinism_property_same_input_yields_identical_output`.

### `search`

Combined FTS5 + semantic search. Registered as the graph-primitive `search` tool (no `illuminate_` prefix).

**Request:**

```json
{
  "method": "search",
  "params": {
    "query": "caching strategy",
    "limit": 5,
    "type": "decision"
  }
}
```

**Response:**

```json
{
  "results": [
    {
      "id": "dec-2025-12-no-redis-payments",
      "kind": "decision",
      "title": "...",
      "snippet": "...",
      "wiki_url": "...",
      "score": 0.87
    }
  ]
}
```

### `illuminate_decisions_for`

List decisions affecting a path.

**Request:**

```json
{
  "method": "illuminate_decisions_for",
  "params": { "path": "services/payments-service/**" }
}
```

**Response:** list of decision summaries.

### `illuminate_failures_for`

List failures affecting a path.

**Request:**

```json
{
  "method": "illuminate_failures_for",
  "params": { "path": "services/payments-service/src/queue.rs" }
}
```

**Response:** list of failure summaries with `lesson` text included (this is the field the agent should surface).

### `illuminate_get_wiki_page`

Fetch the markdown content of a wiki page by id.

**Request:**

```json
{
  "method": "illuminate_get_wiki_page",
  "params": { "id": "dec-2025-12-no-redis-payments" }
}
```

**Response:**

```json
{
  "id": "...",
  "type": "decision",
  "title": "...",
  "front_matter": { /* parsed yaml */ },
  "body": "## Decision\n..."
}
```

---

## Resource exposure

In addition to tools, the MCP server exposes wiki pages as **resources** (per the MCP spec). Agents can subscribe to `wiki/decisions/*`, `wiki/patterns/*`, `wiki/failures/*` and read them on demand.

This complements the tool surface: tools are *queries*, resources are *fetches*. An agent can say "list resources matching `wiki/failures/*`" and walk the failure log without crafting a search query.

---

## Prompts

The server registers a small set of prompts that agents can invoke:

| Prompt | Arguments | Purpose |
|--------|-----------|---------|
| `illuminate_audit_check` | — | Templated prompt: "Before writing any code, call illuminate_audit with your plan and the files you intend to modify." |
| `illuminate_summarize_failures` | `topic` (optional) | Walk recent failures (optionally filtered by `topic`) and summarize lessons relevant to the current session. |
| `illuminate_session_start` | `task` (optional) | Warm-start prompt: at session open, call `illuminate_route` + `illuminate_enrich` + `illuminate_failures_for` to ground the first action in routed files, prior decisions, and past failures. The optional `task` argument is interpolated into the prompt to scope those queries. |

These are optional. The audit tool is the primary integration; prompts are convenience wrappers.

`prompts/get` returns the standard MCP envelope — `{ description, messages: [{ role: "user", content: { type: "text", text } }] }` — and an unknown prompt name returns an `INVALID_PARAMS` error.

---

## Performance + concurrency

- The server holds a read-only handle to `graph.db` per request. Multiple concurrent audits are safe (SQLite WAL mode).
- Writes happen only via the daemon worker (or CLI commands). The MCP server is read-only from the graph's perspective.
- Audit latency target: < 200ms p50, < 500ms p99 (see `AUDIT.md`).
- The server caches loaded ONNX models in memory; cold start ~1.5s, warm requests ~50ms for embedding alone.

---

## Authentication

For stdio: no auth needed; the agent and server run as the same user.

For HTTP: bearer token configured in `illuminate.toml`:

```toml
[mcp.http]
bind = "127.0.0.1:8765"
bearer_token_env = "ILLUMINATE_HTTP_TOKEN"
```

The CLI never starts an HTTP server bound to a public interface without an explicit token. Refuses to start if the token is missing.

---

## Logging

Every MCP request is logged to `.illuminate/log/mcp.log` (gitignored):

```
2026-05-06T12:14:33Z  trace_id=...  method=illuminate_audit  duration_ms=87  status=warn
```

Log entries contain:

- timestamp, trace id, method name, duration, return status
- argument shape (`{plan: <len 64>, files: 2}`) — never argument *content*
- nothing PII-ish

Verbose logging (`[mcp.verbose] = true` in toml) adds full request/response bodies. Off by default. Devs only.

---

## Versioning

The MCP tool contract follows semver:

- Tool names are stable across minor versions.
- Adding fields to a response is backwards-compatible.
- Removing fields or changing field types requires a major version bump.
- The `illuminate.protocol_version` field in every response indicates the contract version.

When a new version is released, the server falls back to old-version response shapes if the client requests them via standard MCP version negotiation.

---

## Errors

JSON-RPC errors follow the standard. Custom error codes:

| Code | Meaning |
|------|---------|
| `-32000` | Generic illuminate error |
| `-32001` | Graph not initialized (run `illuminate init`) |
| `-32002` | Models missing (run `illuminate models download`) |
| `-32003` | Audit threshold/policy violation (returned only when `--strict` mode is on) |

The agent should surface error messages to the dev verbatim; they're written to be human-readable.

---

## Developing against the MCP server

For testing without a real agent client, use `mcp-cli`:

```bash
mcp-cli illuminate_audit --plan "test" --files src/foo.rs
```

Or speak JSON-RPC manually:

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"illuminate_audit","params":{"plan":"test","files":["src/foo.rs"]}}' | illuminate serve
```

---

## What the MCP server does NOT do

- Modify source code (agent-side concern).
- Mutate the graph (writes go through the daemon, not the MCP server).
- Authenticate users (stdio assumes single-user; HTTP uses bearer tokens).
- Cache responses across sessions (each request hits the graph; SQLite is fast enough).
- Stream responses in chunks (responses are small; full JSON only).
