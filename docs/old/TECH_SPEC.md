# illuminate — Technical Specification

**Version:** 0.1.0
**Last updated:** 2026-03-30

---

## 1. Technology Stack

| Component | Technology | Version | Rationale |
|-----------|-----------|---------|-----------|
| Language | Rust | Edition 2024 (1.85+) | Single binary, cross-platform, no runtime deps, memory safety |
| NER (local) | GLiNER v2.1 | INT8 ONNX | Span-based NER, 10 entity types, ~5ms inference |
| REL (local) | GLiREL | ONNX | Relation extraction from entity pairs, ~5ms inference |
| ONNX Runtime | ort crate | 2.x | Cross-platform, CPU-only, INT8 support |
| PII protection | CloakPipe | Latest | Strips PII before LLM calls, HIPAA/SOC2 ready |
| Storage | SQLite | 3.45+ | Embedded, WAL mode, FTS5, recursive CTEs |
| Embeddings | all-MiniLM-L6-v2 | ONNX | 384-dim vectors, ~80MB, local inference |
| AST parsing | tree-sitter | 0.24+ | Incremental, multi-language, mature |
| Protocol | MCP | JSON-RPC 2.0 | Claude Code, Cursor, Windsurf compatibility |
| Async runtime | Tokio | 1.x | De facto Rust async runtime |
| CLI framework | clap | 4.x | Derive macros, shell completions |
| HTTP client | reqwest | Latest | GitHub API, LLM API calls |
| Serialization | serde + serde_json | Latest | JSON and TOML parsing |
| Error handling | thiserror + anyhow | Latest | Typed errors in libraries, anyhow in CLI |
| Logging | tracing | Latest | Structured logging with span context |
| Connection pool | r2d2 | Latest | SQLite connection pooling |
| Hashing | xxhash-rust | Latest | Fast content hashing for incremental indexing |
| SHA-256 | sha2 | Latest | Symbol hash stability |

---

## 2. Crate Specifications

### 2.1 illuminate-core

**Purpose:** Decision graph engine — types, storage, temporal queries, entity linking, intent policies.

**Public API:**

```rust
// Graph operations
pub struct Graph { /* SQLite connection pool */ }

impl Graph {
    pub fn open(path: &Path) -> Result<Self>;
    pub fn insert_episode(&self, episode: &Episode) -> Result<EpisodeId>;
    pub fn insert_entity(&self, entity: &Entity) -> Result<EntityId>;
    pub fn insert_edge(&self, edge: &Edge) -> Result<EdgeId>;
    pub fn insert_anchor(&self, anchor: &Anchor) -> Result<AnchorId>;

    pub fn get_episode(&self, id: &EpisodeId) -> Result<Option<Episode>>;
    pub fn get_entity(&self, id: &EntityId) -> Result<Option<Entity>>;
    pub fn get_entity_by_name(&self, name: &str, entity_type: &str) -> Result<Option<Entity>>;

    pub fn search_fts(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>>;
    pub fn search_semantic(&self, embedding: &[f32], limit: usize) -> Result<Vec<SearchResult>>;
    pub fn traverse(&self, entity_id: &EntityId, depth: u32) -> Result<TraversalResult>;

    pub fn query_as_of(&self, query: &str, as_of: DateTime) -> Result<Vec<SearchResult>>;
    pub fn supersede_edge(&self, edge_id: &EdgeId, valid_until: DateTime) -> Result<()>;

    pub fn stats(&self) -> Result<GraphStats>;
}

// Types
pub struct Episode {
    pub id: EpisodeId,
    pub text: String,
    pub source: Source,
    pub source_ref: Option<String>,
    pub author: Option<String>,
    pub recorded_at: DateTime<Utc>,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_until: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
    pub embedding: Option<Vec<f32>>,
}

pub struct Entity {
    pub id: EntityId,
    pub name: String,
    pub entity_type: EntityType,
    pub created_at: DateTime<Utc>,
    pub summary: Option<String>,
}

pub struct Edge {
    pub id: EdgeId,
    pub source_entity: EntityId,
    pub target_entity: EntityId,
    pub relation_type: RelationType,
    pub episode_id: EpisodeId,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_until: Option<DateTime<Utc>>,
    pub recorded_at: DateTime<Utc>,
}

pub struct Anchor {
    pub id: AnchorId,
    pub episode_id: EpisodeId,
    pub file_path: String,
    pub symbol_name: Option<String>,
    pub symbol_hash: Option<String>,
    pub line_start: Option<u32>,
    pub line_end: Option<u32>,
}

// Policy evaluation
pub struct PolicyEngine { /* parsed TOML policies */ }

impl PolicyEngine {
    pub fn load(config: &Config) -> Result<Self>;
    pub fn check(&self, entities: &[Entity], paths: &[String]) -> Vec<PolicyViolation>;
}
```

**Dependencies:** `rusqlite`, `serde`, `chrono`, `toml`, `thiserror`

### 2.2 illuminate-extract

**Purpose:** Tiered NER pipeline — GLiNER + GLiREL (ONNX) with confidence gate and LLM fallback.

**Public API:**

```rust
pub struct ExtractionPipeline { /* ONNX sessions, config */ }

impl ExtractionPipeline {
    pub fn new(config: &ExtractionConfig) -> Result<Self>;
    pub fn extract(&self, text: &str) -> Result<ExtractionResult>;
    pub fn extract_batch(&self, texts: &[&str]) -> Result<Vec<ExtractionResult>>;
}

pub struct ExtractionResult {
    pub entities: Vec<ExtractedEntity>,
    pub relations: Vec<ExtractedRelation>,
    pub temporal: Vec<TemporalExpr>,
    pub confidence: f32,
    pub used_llm: bool,
}

pub struct ExtractedEntity {
    pub text: String,
    pub entity_type: EntityType,
    pub span: (usize, usize),
    pub confidence: f32,
}

pub struct ExtractedRelation {
    pub head: String,
    pub tail: String,
    pub relation_type: RelationType,
    pub confidence: f32,
}
```

**Dependencies:** `ort`, `tokenizers`, `reqwest`, `serde_json`, `illuminate-core`

### 2.3 illuminate-index

**Purpose:** Minimal code indexer using tree-sitter for decision-to-code anchoring.

**Public API:**

```rust
pub struct CodeIndex { /* SQLite connection, parsers */ }

impl CodeIndex {
    pub fn open(path: &Path) -> Result<Self>;
    pub fn index_project(&mut self, root: &Path, languages: &[Language]) -> Result<IndexStats>;
    pub fn index_file(&mut self, path: &Path) -> Result<Vec<Symbol>>;
    pub fn lookup_symbol(&self, name: &str) -> Result<Vec<Symbol>>;
    pub fn lookup_file(&self, path: &str) -> Result<Vec<Symbol>>;
    pub fn find_anchors(&self, entity_name: &str, changed_files: &[String]) -> Result<Vec<Anchor>>;
}

pub struct Symbol {
    pub id: SymbolId,
    pub file_path: String,
    pub name: String,
    pub symbol_type: SymbolType,
    pub signature: Option<String>,
    pub visibility: Visibility,
    pub line_start: u32,
    pub line_end: u32,
    pub hash: String,
    pub language: Language,
}

pub enum SymbolType {
    Function,
    Struct,
    Class,
    Interface,
    Enum,
    Trait,
    Import,
}
```

**Dependencies:** `tree-sitter`, `tree-sitter-{rust,go,typescript,python,java,c,cpp}`, `rusqlite`, `xxhash-rust`, `sha2`

### 2.4 illuminate-audit

**Purpose:** Contextual linter — cross-reference agent plans against the decision graph.

**Public API:**

```rust
pub struct Auditor {
    graph: Graph,
    index: CodeIndex,
    policies: PolicyEngine,
    reflections: ReflexionStore,
}

impl Auditor {
    pub fn new(graph: Graph, index: CodeIndex, policies: PolicyEngine, reflections: ReflexionStore) -> Self;
    pub fn audit(&self, plan_text: &str) -> Result<AuditResult>;
}

pub struct AuditResult {
    pub status: AuditStatus,          // pass, warning, violation
    pub violations: Vec<Violation>,
    pub policy_violations: Vec<PolicyViolation>,
    pub reflexions: Vec<ReflexionMatch>,
}

pub struct Violation {
    pub violation_type: ViolationType,
    pub plan_entity: String,
    pub conflicting_decision: Episode,
    pub code_anchors: Vec<Anchor>,
    pub severity: Severity,
}

pub enum Severity { Error, Warning, Info }
pub enum AuditStatus { Pass, Warning, Violation }
```

**Dependencies:** `illuminate-core`, `illuminate-index`, `illuminate-reflect`

### 2.5 illuminate-route

**Purpose:** Tri-modal search and subject-to-file routing.

**Public API:**

```rust
pub struct Router {
    graph: Graph,
    index: CodeIndex,
}

impl Router {
    pub fn search(&self, query: &str, opts: &SearchOptions) -> Result<Vec<SearchResult>>;
    pub fn explain(&self, file_path: &str) -> Result<Vec<Episode>>;
    pub fn evolution(&self, file_path: &str, symbol: &str) -> Result<Timeline>;
    pub fn precedents(&self, query: &str, limit: usize) -> Result<Vec<Episode>>;
    pub fn route(&self, subject: &str, limit: usize) -> Result<ReadingPlan>;
}

pub struct ReadingPlan {
    pub decisions: Vec<Episode>,
    pub code_files: Vec<FileEntry>,
    pub reflexions: Vec<ReflexionEpisode>,
    pub estimated_tokens: usize,
}

pub struct FileEntry {
    pub path: String,
    pub symbols: Vec<String>,
    pub priority: u8,
    pub estimated_tokens: usize,
}
```

**Dependencies:** `illuminate-core`, `illuminate-index`

### 2.6 illuminate-watch

**Purpose:** Auto-ingestion daemon — monitor git, GitHub PRs, and webhooks.

**Public API:**

```rust
pub struct Watcher {
    extract: ExtractionPipeline,
    graph: Graph,
    index: CodeIndex,
    config: WatchConfig,
}

impl Watcher {
    pub async fn watch_git(&self, opts: &GitWatchOptions) -> Result<()>;
    pub async fn watch_github(&self, opts: &GitHubWatchOptions) -> Result<()>;
    pub async fn start_webhook(&self, port: u16) -> Result<()>;
    pub async fn run_daemon(&self) -> Result<()>;
    pub async fn backfill_git(&self, count: usize) -> Result<BackfillStats>;
}
```

**Dependencies:** `illuminate-extract`, `illuminate-core`, `illuminate-index`, `reqwest`, `tokio`, `axum` (for webhook server)

### 2.7 illuminate-reflect

**Purpose:** Reflexion loop — capture agent failures as lessons.

**Public API:**

```rust
pub struct ReflexionStore {
    graph: Graph,
}

impl ReflexionStore {
    pub fn record(&self, reflexion: &ReflexionInput) -> Result<EpisodeId>;
    pub fn find_relevant(&self, entities: &[String], files: &[String], limit: usize) -> Result<Vec<ReflexionEpisode>>;
}

pub struct ReflexionInput {
    pub failure: String,
    pub root_cause: String,
    pub corrective_action: String,
    pub files_affected: Vec<String>,
    pub severity: Severity,
}

pub struct ReflexionEpisode {
    pub episode: Episode,
    pub failure: String,
    pub root_cause: String,
    pub corrective_action: String,
    pub severity: Severity,
}
```

**Dependencies:** `illuminate-core`

### 2.8 illuminate-mcp

**Purpose:** MCP server exposing 12 tools via JSON-RPC 2.0.

**Public API:**

```rust
pub struct McpServer {
    context: AppContext,
}

impl McpServer {
    pub fn new(context: AppContext) -> Self;
    pub async fn serve_stdio(&self) -> Result<()>;
    pub async fn serve_http(&self, host: &str, port: u16) -> Result<()>;
}

pub struct AppContext {
    pub graph: Graph,
    pub index: CodeIndex,
    pub auditor: Auditor,
    pub router: Router,
    pub reflections: ReflexionStore,
    pub pipeline: ExtractionPipeline,
    pub config: Config,
}
```

**Dependencies:** `illuminate-core`, `illuminate-index`, `illuminate-audit`, `illuminate-route`, `illuminate-reflect`, `illuminate-extract`, `tokio`, `serde_json`

### 2.9 illuminate-cli

**Purpose:** CLI binary — clap-based command dispatch.

**Dependencies:** All other illuminate crates, `clap`, `tokio`, `colored`

---

## 3. SQLite Schema

```sql
-- Schema version tracking
CREATE TABLE schema_version (
    version INTEGER NOT NULL
);

-- Episodes
CREATE TABLE episodes (
    id          TEXT PRIMARY KEY,
    text        TEXT NOT NULL,
    source      TEXT NOT NULL CHECK(source IN ('git','github-pr','manual','webhook','reflexion')),
    source_ref  TEXT,
    author      TEXT,
    recorded_at TEXT NOT NULL,
    valid_from  TEXT,
    valid_until TEXT,
    tags        TEXT,        -- JSON array: ["tag1", "tag2"]
    embedding   BLOB         -- 384 × f32 = 1,536 bytes
);

CREATE INDEX idx_episodes_source ON episodes(source);
CREATE INDEX idx_episodes_recorded ON episodes(recorded_at);
CREATE INDEX idx_episodes_valid ON episodes(valid_from, valid_until);

-- Full-text search on episodes
CREATE VIRTUAL TABLE episodes_fts USING fts5(
    text,
    content=episodes,
    content_rowid=rowid,
    tokenize='porter unicode61'
);

-- Entities
CREATE TABLE entities (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    normalized  TEXT NOT NULL,  -- lowercase, stripped suffixes
    entity_type TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    summary     TEXT
);

CREATE UNIQUE INDEX idx_entities_norm ON entities(normalized, entity_type);

CREATE VIRTUAL TABLE entities_fts USING fts5(
    name, summary,
    content=entities,
    content_rowid=rowid,
    tokenize='porter unicode61'
);

-- Edges (relations between entities)
CREATE TABLE edges (
    id             TEXT PRIMARY KEY,
    source_entity  TEXT NOT NULL REFERENCES entities(id),
    target_entity  TEXT NOT NULL REFERENCES entities(id),
    relation_type  TEXT NOT NULL,
    episode_id     TEXT NOT NULL REFERENCES episodes(id),
    valid_from     TEXT,
    valid_until    TEXT,
    recorded_at    TEXT NOT NULL
);

CREATE INDEX idx_edges_source ON edges(source_entity);
CREATE INDEX idx_edges_target ON edges(target_entity);
CREATE INDEX idx_edges_episode ON edges(episode_id);
CREATE INDEX idx_edges_valid ON edges(valid_from, valid_until);

-- Code anchors
CREATE TABLE anchors (
    id          TEXT PRIMARY KEY,
    episode_id  TEXT NOT NULL REFERENCES episodes(id),
    file_path   TEXT NOT NULL,
    symbol_name TEXT,
    symbol_hash TEXT,
    line_start  INTEGER,
    line_end    INTEGER,
    created_at  TEXT NOT NULL
);

CREATE INDEX idx_anchors_episode ON anchors(episode_id);
CREATE INDEX idx_anchors_file ON anchors(file_path);
CREATE INDEX idx_anchors_symbol ON anchors(symbol_name);

-- Reflexion metadata (extends episodes with reflexion-specific fields)
CREATE TABLE reflexions (
    episode_id        TEXT PRIMARY KEY REFERENCES episodes(id),
    failure           TEXT NOT NULL,
    root_cause        TEXT NOT NULL,
    corrective_action TEXT NOT NULL,
    severity          TEXT NOT NULL CHECK(severity IN ('low','medium','high','critical')),
    files_affected    TEXT  -- JSON array
);

-- Code symbols (in index.db, separate file)
CREATE TABLE symbols (
    id          TEXT PRIMARY KEY,
    file_path   TEXT NOT NULL,
    name        TEXT NOT NULL,
    symbol_type TEXT NOT NULL,
    signature   TEXT,
    visibility  TEXT,
    line_start  INTEGER NOT NULL,
    line_end    INTEGER NOT NULL,
    hash        TEXT NOT NULL,
    language    TEXT NOT NULL,
    content_hash TEXT NOT NULL,  -- xxh3 of file content (for incremental)
    updated_at  TEXT NOT NULL
);

CREATE INDEX idx_symbols_name ON symbols(name);
CREATE INDEX idx_symbols_file ON symbols(file_path);
CREATE INDEX idx_symbols_hash ON symbols(hash);
```

---

## 4. MCP Tool Schemas

### illuminate_audit

```json
{
    "name": "illuminate_audit",
    "description": "Cross-reference an agent's proposed plan against the decision graph and intent policies. Returns structured warnings with source attribution and code anchors.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "plan": {
                "type": "string",
                "description": "The agent's proposed plan or action in natural language"
            }
        },
        "required": ["plan"]
    }
}
```

### illuminate_search

```json
{
    "name": "illuminate_search",
    "description": "Tri-modal search (FTS5 + semantic + graph walk) across decisions, code symbols, and reflexion episodes.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "query": { "type": "string", "description": "Search query in natural language" },
            "limit": { "type": "integer", "default": 10 },
            "as_of": { "type": "string", "description": "ISO 8601 date for point-in-time query" }
        },
        "required": ["query"]
    }
}
```

### illuminate_explain

```json
{
    "name": "illuminate_explain",
    "description": "Given a file path, return all linked decisions and their full traces.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "path": { "type": "string", "description": "File path to explain" }
        },
        "required": ["path"]
    }
}
```

### illuminate_impact

```json
{
    "name": "illuminate_impact",
    "description": "Given a decision ID, show every file and symbol anchored to that decision.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "decision_id": { "type": "string" }
        },
        "required": ["decision_id"]
    }
}
```

### illuminate_evolution

```json
{
    "name": "illuminate_evolution",
    "description": "Show how a function changed over time with linked decisions and reflexion lessons.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "path": { "type": "string", "description": "File path" },
            "symbol": { "type": "string", "description": "Symbol name (function, struct, etc.)" }
        },
        "required": ["path"]
    }
}
```

### illuminate_traverse

```json
{
    "name": "illuminate_traverse",
    "description": "Walk the decision graph from an entity. Show relationships, temporal history, superseded decisions.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "entity": { "type": "string", "description": "Entity name to start traversal from" },
            "depth": { "type": "integer", "default": 2, "description": "Maximum traversal depth" }
        },
        "required": ["entity"]
    }
}
```

### illuminate_precedents

```json
{
    "name": "illuminate_precedents",
    "description": "Find similar past decisions via embedding similarity.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "query": { "type": "string", "description": "Decision or situation to find precedents for" },
            "limit": { "type": "integer", "default": 5 }
        },
        "required": ["query"]
    }
}
```

### illuminate_route

```json
{
    "name": "illuminate_route",
    "description": "Given a subject, return a ranked reading plan of files and decisions.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "subject": { "type": "string", "description": "Topic or subject to explore" },
            "limit": { "type": "integer", "default": 10 }
        },
        "required": ["subject"]
    }
}
```

### illuminate_log

```json
{
    "name": "illuminate_log",
    "description": "Manually record a decision or event into the graph.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "text": { "type": "string", "description": "Decision description" },
            "source": { "type": "string", "default": "manual" },
            "tags": { "type": "array", "items": { "type": "string" } }
        },
        "required": ["text"]
    }
}
```

### illuminate_reflect

```json
{
    "name": "illuminate_reflect",
    "description": "Record a failure/lesson from the current session as a reflexion episode.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "failure": { "type": "string", "description": "What went wrong" },
            "root_cause": { "type": "string", "description": "Why it went wrong" },
            "corrective_action": { "type": "string", "description": "What to do instead" },
            "files_affected": { "type": "array", "items": { "type": "string" } },
            "severity": { "type": "string", "enum": ["low", "medium", "high", "critical"], "default": "medium" }
        },
        "required": ["failure", "root_cause", "corrective_action"]
    }
}
```

### illuminate_symbols

```json
{
    "name": "illuminate_symbols",
    "description": "Look up code symbols by name. Returns file path, line number, and linked decisions.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "name": { "type": "string", "description": "Symbol name to search for" }
        },
        "required": ["name"]
    }
}
```

### illuminate_stats

```json
{
    "name": "illuminate_stats",
    "description": "Graph statistics: episodes, entities, edges, intent coverage percentage.",
    "inputSchema": {
        "type": "object",
        "properties": {}
    }
}
```

---

## 5. Extraction Pipeline Detail

### Stage-by-stage specification

| # | Stage | Input | Output | Latency |
|---|-------|-------|--------|---------|
| 1 | GLiNER NER | Raw text | Entity spans with types and confidence | ~5ms |
| 2 | GLiREL REL | Text + entity spans | Relation triples with types | ~5ms |
| 3 | Confidence gate | Entity + relation scores | Decision: local-only or LLM fallback | <1ms |
| 4 | CloakPipe pseudonymize | Text + entities | Pseudonymized text + mapping | <1ms |
| 5 | LLM fallback | Pseudonymized text | Refined entities + relations | ~200ms |
| 6 | CloakPipe rehydrate | LLM output + mapping | Real entity names restored | <1ms |
| 7 | Coreference resolution | Text + entities | Pronouns resolved to entities | <1ms |
| 8 | Entity supplement | Text + entities | Dictionary-based missed entity detection | <1ms |
| 9 | Type remapping | Entities | Misclassified types corrected | <1ms |
| 10 | Conflict resolution | Relations | Contradictory relations resolved | <1ms |
| 11 | Temporal parsing | Text | Date/time expressions extracted | <1ms |
| 12 | Code anchoring | Entities + git context | File:line anchors created | ~2ms |

**Total local path:** ~12ms
**Total LLM path:** ~215ms

### Confidence gate formula

```
score = 0.3 × entity_score + 0.3 × relation_score + 0.2 × type_diversity + 0.2 × known_entity_bonus

entity_score = min(1.0, entity_count / expected_entities)
relation_score = min(1.0, relation_count / max(1, entity_count - 1))
type_diversity = unique_types / total_entities
known_entity_bonus = known_entities / total_entities  (entities already in graph)

threshold = 0.7 (configurable)
```

---

## 6. Performance Budgets

### Per-operation latency budget

| Operation | Budget | Breakdown |
|-----------|--------|-----------|
| `illuminate_audit` | 20ms | Policy: 1ms, Entity extraction: 2ms, Graph query: 5ms, Anchor enrichment: 5ms, Reflexion check: 5ms, Serialization: 2ms |
| `illuminate_search` | 15ms | FTS5: 3ms, Semantic: 5ms, Graph walk: 5ms, RRF fusion: 1ms, Serialization: 1ms |
| `illuminate_route` | 20ms | Search: 15ms, Token estimation: 2ms, Plan assembly: 2ms, Serialization: 1ms |
| Local extraction | 15ms | GLiNER: 5ms, GLiREL: 5ms, Post-processing: 5ms |
| Incremental index | 20ms | File check: 2ms, Parse changed: 10ms, DB write: 8ms |

### Memory budget

| Component | Budget |
|-----------|--------|
| SQLite (graph.db, typical) | 10 MB |
| ONNX Runtime (GLiNER + GLiREL) | 150 MB |
| Embedding model | 80 MB |
| Tree-sitter parsers | 5 MB |
| Application overhead | 10 MB |
| **Peak total** | **255 MB** |
| **Idle (MCP server)** | **20 MB** |

### Disk budget

| Component | Size |
|-----------|------|
| illuminate binary | ~15 MB |
| GLiNER ONNX model | ~650 MB |
| GLiREL ONNX model | ~50 MB |
| Embedding model | ~80 MB |
| graph.db (typical project) | 1-10 MB |
| index.db (typical project) | <1 MB |
| **Total (with models)** | **~800 MB** |
| **Total (without models)** | **~25 MB** |
