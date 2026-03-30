# illuminate — System Design

**Version:** 0.1.0
**Last updated:** 2026-03-30

---

## 1. Design Goals

| Goal | Constraint |
|------|-----------|
| Zero infrastructure | Single binary, single SQLite file, no Docker/Python/Neo4j |
| Privacy by default | PII stripped before any external call, queries fully local |
| Sub-20ms audit latency | No LLM in query/audit path, all local computation |
| Offline-capable | Local ONNX models handle ~70% of extraction at $0 |
| Append-only correctness | Decisions never deleted, only superseded with temporal metadata |
| MCP-native distribution | Works with any MCP client without custom integration |

---

## 2. Component Design

### 2.1 Decision Graph Engine (illuminate-core)

**Responsibility:** Store and query episodes, entities, edges, and anchors with bi-temporal semantics.

**Key design decisions:**

**SQLite over embedded graph databases:**
- SQLite is battle-tested, zero-config, and supports recursive CTEs for graph traversal
- FTS5 provides full-text search without additional dependencies
- WAL mode enables concurrent reads during writes
- Single file simplifies backup, sync, and version control
- No need for a dedicated graph database — our graph is small enough (typically <100K edges) that recursive CTEs perform well

**Bi-temporal model:**
Every edge tracks two time dimensions. This is implemented as four nullable columns:

```sql
valid_from  TEXT,  -- When this became true in reality
valid_until TEXT,  -- When this stopped being true (NULL = still active)
recorded_at TEXT,  -- When illuminate learned this fact
```

Point-in-time query:
```sql
SELECT * FROM edges
WHERE valid_from <= ?as_of
  AND (valid_until IS NULL OR valid_until > ?as_of)
```

**Entity deduplication:**
Entities are deduplicated by normalized name + type. Normalization:
1. Lowercase
2. Strip common suffixes ("Service", "Client", "Manager")
3. Collapse whitespace
4. Alias resolution from a dictionary (e.g., "postgres" = "postgresql" = "pg")

**Connection pooling:**
Single `r2d2` pool with:
- 1 writer connection (serialized writes)
- N reader connections (parallel reads)
- WAL mode enabled at initialization

### 2.2 Extraction Engine (illuminate-extract)

**Responsibility:** Convert raw text into structured episodes with entities, relations, and temporal metadata.

**Pipeline architecture:**

The pipeline is a chain of 12 stages, each receiving and returning a mutable `ExtractionContext`:

```rust
struct ExtractionContext {
    input_text: String,
    entities: Vec<Entity>,
    relations: Vec<Relation>,
    confidence: f32,
    temporal: Vec<TemporalExpr>,
    anchors: Vec<CodeAnchor>,
    metadata: HashMap<String, String>,
}
```

Each stage is a trait:
```rust
trait ExtractionStage {
    fn process(&self, ctx: &mut ExtractionContext) -> Result<()>;
}
```

The pipeline is assembled at startup based on configuration. Stages can be disabled (e.g., `no_embed = true` disables embedding generation).

**ONNX inference:**
- GLiNER and GLiREL share a single ONNX Runtime session
- Models loaded lazily on first extraction call
- INT8 quantization reduces memory from ~600MB to ~150MB
- Batch inference for backfill operations (process 10 episodes at once)

**LLM fallback protocol:**
```
1. CloakPipe.pseudonymize(text) → (pseudonymized_text, mapping)
2. LLM.extract(pseudonymized_text) → pseudonymized_entities
3. CloakPipe.rehydrate(pseudonymized_entities, mapping) → real_entities
4. Merge with local extraction (union, prefer LLM for conflicts)
```

The LLM receives a single structured prompt:
```
Extract entities and relations from this text.
Entity types: Person, Service, Database, Library, Language, Pattern, Component, API, Infrastructure, Decision
Relation types: chose, rejected, depends_on, replaced, caused, approved, blocked, migrated_to, owns, learned

Text: {pseudonymized_text}

Return JSON: { "entities": [...], "relations": [...] }
```

One call. No multi-step chain. No agent loop. This is why illuminate uses 1 LLM call vs Graphiti's 6.

### 2.3 Code Indexer (illuminate-index)

**Responsibility:** Minimal symbol extraction for decision anchoring. Not a full code intelligence engine.

**Design constraint:** illuminate-index extracts only what's needed for anchoring — function signatures, struct declarations, and import statements. It does not build a full dependency graph, resolve types, or provide go-to-definition.

**Tree-sitter queries:**
Each language has a set of tree-sitter queries that extract symbols:

```scheme
;; Rust function extraction
(function_item
  name: (identifier) @name
  parameters: (parameters) @params
  return_type: (type_identifier)? @return_type) @func
```

**Symbol hashing:**
Each symbol gets a stable hash for tracking across renames:
```
hash = SHA-256(normalize(language + ":" + symbol_type + ":" + name + ":" + signature))
```

This hash is used by code anchors. If a function is renamed, the hash changes, and stale anchors are flagged during the next `illuminate index` run.

**Incremental strategy:**
```
For each file in project:
  if mtime(file) > last_indexed(file):
    content_hash = xxh3(read(file))
    if content_hash != stored_hash(file):
      reparse(file)
      update_symbols(file)
      update_hash(file, content_hash)
```

### 2.4 Contextual Linter (illuminate-audit)

**Responsibility:** Cross-reference an agent's plan against the decision graph and intent policies.

**Audit algorithm:**

```
audit(plan_text):
  1. entities = lightweight_ner(plan_text)       # Extract entities from plan
  2. policy_violations = check_policies(entities) # Check TOML policies
  3. For each entity in entities:
     a. graph_matches = search_graph(entity)      # FTS5 + semantic search
     b. For each match:
        - Check if match conflicts with plan intent
        - Enrich with code anchors from illuminate-index
  4. reflexion_matches = search_reflexions(entities)
  5. Combine all violations, sort by severity
  6. Return structured JSON response
```

**Lightweight NER for plan text:**
The audit path does NOT use the full 12-stage extraction pipeline. Instead, it uses a simplified entity extractor:
1. Dictionary lookup against known entities in the graph
2. Simple regex patterns for common technology names
3. No LLM call — audit path is always local and fast

This keeps audit latency under 20ms.

**Policy evaluation order:**
1. `frozen` policies checked first (path-based, fast rejection)
2. `must_use` and `rejected_pattern` checked against extracted entities
3. `convention` checked against plan structure
4. Graph conflict detection (most expensive, last)

### 2.5 Subject-to-File Router (illuminate-route)

**Responsibility:** Given a natural language subject, return a ranked reading plan of decisions and files.

**RRF (Reciprocal Rank Fusion) scoring:**

Three search modes run in parallel:
1. FTS5 full-text search → ranked results
2. Semantic embedding search → ranked results
3. Graph walk (entities matching subject → linked episodes → anchored files) → ranked results

Results are fused:
```
score(item) = Σ_mode 1 / (60 + rank_mode(item))
```

Items appearing in multiple modes are ranked highest. The constant 60 is the standard RRF value that balances early and late ranks.

**Token estimation:**
Each file in the reading plan includes an estimated token count:
```
tokens ≈ bytes / 4  (rough estimate for code)
```

This helps agents budget their context window.

### 2.6 MCP Server (illuminate-mcp)

**Responsibility:** Expose illuminate's capabilities via the Model Context Protocol.

**Transport handling:**

```
stdio mode:
  Read JSON-RPC from stdin → parse → dispatch → serialize → write to stdout

http mode:
  Bind to port → accept connections → read JSON-RPC from request body
  → dispatch → serialize → write to response body
  Supports Streamable HTTP (SSE for long-running operations)
```

**Tool dispatch:**
Each MCP tool maps to a handler function:

```rust
match tool_name {
    "illuminate_audit"      => audit::handle(params, &ctx),
    "illuminate_search"     => route::search(params, &ctx),
    "illuminate_explain"    => route::explain(params, &ctx),
    "illuminate_evolution"  => evolution::handle(params, &ctx),
    "illuminate_impact"     => audit::impact(params, &ctx),
    "illuminate_traverse"   => graph::traverse(params, &ctx),
    "illuminate_precedents" => route::precedents(params, &ctx),
    "illuminate_route"      => route::plan(params, &ctx),
    "illuminate_log"        => graph::log_episode(params, &ctx),
    "illuminate_reflect"    => reflect::handle(params, &ctx),
    "illuminate_symbols"    => index::symbols(params, &ctx),
    "illuminate_stats"      => graph::stats(params, &ctx),
    _ => Err(MethodNotFound),
}
```

**Shared context:**
All handlers share an `AppContext` with:
- Graph database connection pool (read + write)
- Index database connection pool
- ONNX runtime session (lazy loaded)
- Configuration (from illuminate.toml)

### 2.7 Reflexion Loop (illuminate-reflect)

**Responsibility:** Capture agent failures as lesson episodes for future sessions.

**Episode format:**
```rust
struct ReflexionEpisode {
    failure: String,           // What went wrong
    root_cause: String,        // Why it went wrong
    corrective_action: String, // What fixed it / what to do instead
    files_affected: Vec<String>,
    severity: Severity,        // How bad was the failure
    session_id: String,        // Which agent session
}
```

**Retrieval during audit:**
When `illuminate-audit` processes a plan, it queries reflexion episodes:
1. Match by file paths in the plan
2. Match by entities in the plan (e.g., "Redis" matches a past Redis failure)
3. Rank by recency and severity
4. Attach top-3 relevant reflexions to the audit response

---

## 3. Data Flow Diagrams

### 3.1 Ingestion Flow

```
Git commit "Switch from REST to gRPC for billing — latency improvement"
    │
    ▼
illuminate-watch/git.rs
    │ Score decision signal: 0.85 (high — "switch from", "for")
    │
    ▼
illuminate-extract/pipeline.rs
    │
    ├─ Stage 1: GLiNER → entities: [REST(API), gRPC(API), billing(Service)]
    ├─ Stage 2: GLiREL → relations: [replaced(billing, REST→gRPC)]
    ├─ Stage 3: Confidence gate → 0.82 (above threshold, skip LLM)
    ├─ Stage 7: Coref → (no pronouns)
    ├─ Stage 11: Temporal → recorded_at: commit timestamp
    └─ Stage 12: Anchoring → files changed in commit → code anchors
    │
    ▼
illuminate-core/graph.rs
    │ Insert episode, entities, edges, anchors into graph.db
    │
    ▼
graph.db updated
```

### 3.2 Audit Flow

```
Agent plan: "Add Redis caching layer to billing service"
    │
    ▼
illuminate-audit/auditor.rs
    │
    ├─ Extract entities from plan:
    │    Redis(Database), billing(Service), caching(Pattern)
    │
    ├─ Check policies:
    │    policies.caching.must_use = "Memcached"
    │    policies.caching.reject = ["Redis"]
    │    → VIOLATION: Redis rejected by policy
    │
    ├─ Query graph for "Redis" + "billing" + "caching":
    │    → Episode a1b2c3d4: "Use Memcached, not Redis" (PR #847)
    │    → Entity: Memcached (chose), Redis (rejected)
    │    → CONFLICT: Plan proposes rejected entity
    │
    ├─ Enrich with code anchors:
    │    → src/cache/provider.rs:42-89 (MemcachedClient)
    │    → src/billing/checkout.rs:15 (cache import)
    │
    └─ Check reflexions:
         → Session 2026-03-20: Redis migration failed (connection pool exhaustion)
    │
    ▼
Structured JSON response with violations, anchors, reflexions
```

### 3.3 Search Flow

```
Query: "why Postgres?"
    │
    ├─────────────┬─────────────┐
    ▼             ▼             ▼
  FTS5         Semantic      Graph Walk
  "Postgres"   embed(query)   entity:Postgres
    │             │              │
    ▼             ▼              ▼
  Rank: [       Rank: [       Rank: [
   ep_1: 1,     ep_3: 1,      ep_1: 1,
   ep_2: 2,     ep_1: 2,      ep_4: 2,
   ep_5: 3      ep_7: 3       ep_2: 3
  ]             ]              ]
    │             │              │
    └─────────────┼──────────────┘
                  ▼
            RRF Fusion
            ep_1: 1/(60+1) + 1/(60+2) + 1/(60+1) = 0.0491  ← top result
            ep_2: 1/(60+2) + 1/(60+3) = 0.0321
            ep_3: 1/(60+1) = 0.0164
            ...
                  │
                  ▼
            Sorted results with source attribution
```

---

## 4. Concurrency Model

### Single-writer, multi-reader

illuminate uses SQLite's WAL mode with a strict concurrency model:

```
┌─────────────────────────────┐
│  Write path (serialized)    │
│                             │
│  illuminate-watch daemon    │──→  Single writer connection
│  illuminate_log MCP tool    │     (mutex-protected)
│  illuminate_reflect MCP tool│
└─────────────────────────────┘

┌─────────────────────────────┐
│  Read path (parallel)       │
│                             │
│  illuminate_audit           │──→  Pool of N reader connections
│  illuminate_search          │     (concurrent, no locking)
│  illuminate_explain         │
│  illuminate_route           │
│  illuminate_traverse        │
│  illuminate_stats           │
│  illuminate_symbols         │
└─────────────────────────────┘
```

The write path is serialized via a Rust `Mutex<Connection>`. The read path uses an `r2d2` connection pool. WAL mode allows reads to proceed concurrently with a write without blocking.

### Async architecture

illuminate uses Tokio for:
- MCP server I/O (stdin/stdout and HTTP)
- GitHub API polling
- File watching
- LLM API calls

CPU-bound work (ONNX inference, tree-sitter parsing) runs on `spawn_blocking` to avoid starving the async runtime.

---

## 5. Error Handling Strategy

### Error hierarchy

```rust
#[derive(thiserror::Error)]
enum IlluminateError {
    // Storage errors
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    // Extraction errors
    #[error("model not found: {0}")]
    ModelNotFound(String),
    #[error("ONNX inference failed: {0}")]
    OnnxError(#[from] ort::Error),
    #[error("LLM API error: {0}")]
    LlmError(String),

    // Index errors
    #[error("parser error for {language}: {message}")]
    ParserError { language: String, message: String },

    // Config errors
    #[error("invalid policy: {0}")]
    PolicyError(String),
    #[error("config error: {0}")]
    ConfigError(String),

    // MCP errors
    #[error("invalid tool parameters: {0}")]
    InvalidParams(String),
}
```

### Graceful degradation

| Failure | Behavior |
|---------|----------|
| ONNX models not downloaded | Prompt user to run `illuminate models download` |
| LLM API unreachable | Use local extraction only (lower quality, still functional) |
| LLM API returns error | Log warning, accept local extraction |
| Tree-sitter parser missing for language | Skip that language, index others |
| SQLite corruption | Detect via integrity check, prompt for `illuminate repair` |
| GitHub API rate limit | Back off with exponential retry, log warning |

---

## 6. Configuration Layering

```
Priority (highest first):
  1. Environment variables (ILLUMINATE_*)
  2. .illuminate/config.toml (local overrides, not committed)
  3. illuminate.toml (project config, committed)
  4. ~/.config/illuminate/config.toml (user defaults)
  5. Compiled defaults
```

Each layer can override any setting from a lower layer. This enables:
- Developers overriding team settings locally
- CI/CD setting `ILLUMINATE_OFFLINE=1`
- Teams committing shared policies in `illuminate.toml`

---

## 7. Testing Strategy

### Unit tests

Each crate has `#[cfg(test)]` modules testing internal functions:
- `illuminate-core`: Graph operations, temporal queries, policy evaluation
- `illuminate-extract`: Individual pipeline stages, confidence scoring
- `illuminate-index`: Symbol extraction per language, hash stability
- `illuminate-audit`: Violation detection, severity assignment
- `illuminate-route`: Search ranking, RRF fusion

### Integration tests

`tests/` directory at workspace root:
- End-to-end extraction from fixture text → graph → audit
- MCP tool call → response validation
- Backfill from test git repository
- Policy violation detection with real graph data

### Benchmark tests

`benches/` directory:
- Extraction F1 on 20 unseen episodes (validates against Graphiti baseline)
- Query latency under varying graph sizes
- Indexing speed for projects of different sizes
- Audit latency with and without reflexion lookup

---

## 8. Deployment Model

### Binary distribution

```
GitHub Releases:
  illuminate-{version}-x86_64-linux.tar.gz
  illuminate-{version}-aarch64-linux.tar.gz
  illuminate-{version}-x86_64-darwin.tar.gz
  illuminate-{version}-aarch64-darwin.tar.gz

Homebrew:
  brew install rohansx/tap/illuminate

Cargo:
  cargo install illuminate-cli
```

### Model distribution

ONNX models (~700 MB total) are downloaded on first use:
```
~/.cache/illuminate/models/
├── gliner-v2.1-int8.onnx     (~650 MB)
├── glirel.onnx                (~50 MB)
└── all-MiniLM-L6-v2.onnx     (~80 MB)
```

Models are downloaded from GitHub Releases with SHA-256 verification. No Hugging Face dependency at runtime.

### Upgrade path

illuminate uses semver. Graph schema migrations run automatically:
```rust
fn migrate(conn: &Connection) -> Result<()> {
    let version = get_schema_version(conn)?;
    match version {
        0 => migrate_v0_to_v1(conn)?,
        1 => migrate_v1_to_v2(conn)?,
        // ...
        CURRENT_VERSION => {} // up to date
        _ => return Err(Error::UnknownSchema(version)),
    }
    Ok(())
}
```

Migrations are forward-only and non-destructive (append-only philosophy extends to the schema).
