# Illuminate — Crate Reference

Crate-by-crate breakdown: responsibility, public API surface, dependencies, and timeline. For the high-level architecture, see `ARCHITECTURE.md`. For specific components, see `INGESTION.md`, `AUDIT.md`, and `SCHEMA.md`.

---

## Workspace layout

```
crates/
├── illuminate-core      # graph API on top of ctxgraph
├── illuminate-trail     # session capture (claude/cursor/codex)
├── illuminate-extract   # NER pipeline (GLiNER, GLiREL, embeddings)
├── illuminate-embed     # embeddings service
├── illuminate-index     # tree-sitter code indexer
├── illuminate-audit     # policy engine + graph queries
├── illuminate-watch     # daemon harness (hosts trail watcher + workers)
├── illuminate-reflect   # failure capture as Reflexion episodes
├── illuminate-route     # subject-to-reading-plan generator (FTS5 + semantic RRF)
├── illuminate-mcp       # MCP server (JSON-RPC)
└── illuminate-cli       # binary
```

The `illuminate-cli` crate is the only binary. Everything else is a library.

---

## Dependency graph (compile-time)

```
                   illuminate-cli
                          │
        ┌────────┬────────┼────────┬────────┬────────┐
        ▼        ▼        ▼        ▼        ▼        ▼
   illuminate-trail │  illuminate-watch │  illuminate-mcp
                   ▼                   ▼              │
            illuminate-extract  illuminate-reflect    │
                  │                   │               │
                  └────────┬──────────┘               │
                           ▼                          ▼
                  illuminate-route          illuminate-audit
                           │                          │
                           └──────────┬───────────────┘
                                      ▼
                              illuminate-index
                                      │
                                      ▼
                              illuminate-embed
                                      │
                                      ▼
                              illuminate-core
                                      │
                                      ▼
                                  ctxgraph
```

Lower crates have no upward dependencies. `illuminate-core` is the lowest layer and depends only on `ctxgraph` + workspace deps. `illuminate-cli` depends on everything; it's the integration point.

---

## `illuminate-core`

**Responsibility.** The graph API layered on top of `ctxgraph`. Defines Illuminate's domain entities and relationships, provides query helpers, manages `graph.db`.

**Key types.**

```rust
pub enum EntityKind {
    Person, Component, Service, Language, Database,
    Infrastructure, Decision, Constraint, Metric, Pattern,
    Module, Failure,
}

pub struct Entity {
    pub id: EntityId,
    pub kind: EntityKind,
    pub name: String,
    pub canonical_aliases: Vec<String>,
    pub embedding: Option<Embedding>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct Episode { /* see INGESTION.md */ }

pub struct Edge {
    pub head: EntityId,
    pub relation: String, // "chose", "rejected", "depends_on", ...
    pub tail: EntityId,
    pub episode: EpisodeId,
    pub valid_at: DateTime<Utc>,
    pub recorded_at: DateTime<Utc>,
}
```

**Public API (sketch).**

```rust
pub trait GraphStore {
    fn open(path: &Path) -> Result<Self>;
    fn add_episode(&mut self, ep: Episode) -> Result<EpisodeId>;
    fn add_entity(&mut self, e: Entity) -> Result<EntityId>;
    fn add_edge(&mut self, e: Edge) -> Result<()>;

    fn entities_for_file(&self, path: &Path) -> Result<Vec<Entity>>;
    fn decisions_referencing(&self, entity: EntityId) -> Result<Vec<Entity>>;
    fn semantic_search(&self, embedding: &Embedding, k: usize) -> Result<Vec<(Entity, f32)>>;
    fn rebuild(&mut self, sources: &[Source]) -> Result<RebuildReport>;
}
```

**Dependencies.** `ctxgraph`, `rusqlite`, `chrono`, `serde`, `uuid`.

**Timeline.** v0.1.

---

## `illuminate-trail`

**Responsibility.** Watch agent session storage and produce normalized `Trail` episodes. Writes to `.illuminate/trail/`.

**Key types.**

```rust
pub enum AgentKind {
    ClaudeCode,
    Cursor,
    Codex,
}

pub struct TrailRecord {
    pub session_id: String,
    pub agent: AgentKind,
    pub model: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub repo_path: PathBuf,
    pub messages: Vec<Message>,
    pub files_touched: Vec<PathBuf>,
    pub tool_invocations: Vec<ToolInvocation>,
}

pub trait SessionWatcher {
    fn watch(&mut self, on_session_end: Box<dyn Fn(TrailRecord)>) -> Result<()>;
}
```

**Public API.**

```rust
pub fn start_claude_watcher(opts: WatcherOpts) -> Result<JoinHandle>;
pub fn start_cursor_watcher(opts: WatcherOpts) -> Result<JoinHandle>;
pub fn start_codex_watcher(opts: WatcherOpts) -> Result<JoinHandle>;

pub fn normalize_session(raw: RawSession) -> TrailRecord;
pub fn write_trail(path: &Path, record: &TrailRecord) -> Result<()>;
```

**Dependencies.** `tokio`, `notify` (filesystem watching), `serde`, `chrono`.

**Timeline.** v0.1 ships Claude Code watcher only. Cursor + Codex in v0.2.

---

## `illuminate-extract`

**Responsibility.** Run the NER pipeline on episodes. GLiNER for entities, GLiREL for relations, signal scoring, dedup, write to graph.

**Public API.**

```rust
pub struct Extractor {
    /* gliner model, glirel model, embedder */
}

impl Extractor {
    pub fn new(opts: ExtractorOpts) -> Result<Self>;

    pub fn signal_score(&self, text: &str) -> f32;
    pub fn extract_entities(&self, text: &str) -> Result<Vec<EntityCandidate>>;
    pub fn extract_relations(&self, text: &str, entities: &[EntityCandidate]) -> Result<Vec<RelationCandidate>>;

    pub fn process_episode(&mut self, ep: Episode, store: &mut impl GraphStore) -> Result<ExtractionReport>;
}

pub struct ExtractionReport {
    pub entities_added: usize,
    pub edges_added: usize,
    pub used_llm_fallback: bool,
    pub confidence: f32,
}
```

**Dependencies.** `gline-rs`, `ort`, `ndarray`, `illuminate-embed`, `illuminate-route`, `illuminate-core`.

**Timeline.** v0.1.

---

## `illuminate-embed`

**Responsibility.** Compute embeddings via all-MiniLM-L6-v2 (ONNX). Used by `illuminate-extract` (for new entities) and `illuminate-audit` (for query-time semantic search).

**Public API.**

```rust
pub type Embedding = [f32; 384];

pub struct Embedder {
    /* model handle */
}

impl Embedder {
    pub fn load(model_path: &Path) -> Result<Self>;
    pub fn embed(&self, text: &str) -> Result<Embedding>;
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>>;
    pub fn cosine_sim(a: &Embedding, b: &Embedding) -> f32;
}
```

**Dependencies.** `fastembed`, `ort`, `ndarray`.

**Timeline.** v0.1.

---

## `illuminate-index`

**Responsibility.** Tree-sitter–based source-code indexer. Maps files → modules, files → symbols, symbols → declarations.

**Public API.**

```rust
pub struct CodeIndex {
    pub by_file: HashMap<PathBuf, ModuleId>,
    pub by_symbol: HashMap<String, Vec<SymbolLocation>>,
}

impl CodeIndex {
    pub fn build(repo_root: &Path) -> Result<Self>;
    pub fn module_for_path(&self, path: &Path) -> Option<ModuleId>;
    pub fn symbols_in_file(&self, path: &Path) -> Vec<Symbol>;
    pub fn rebuild_incremental(&mut self, changed: &[PathBuf]) -> Result<()>;
}
```

**Languages supported in v0.1.** Rust, TypeScript, Python. JavaScript via TS parser. v0.2 adds Go, Java, Kotlin.

**Dependencies.** `tree-sitter` + per-language parsers.

**Timeline.** v0.1.

---

## `illuminate-audit`

**Responsibility.** The linter. Reads `illuminate.toml` policies, queries the graph, scores severity, returns structured findings. See `AUDIT.md` for the full contract.

**Public API.**

```rust
pub struct Auditor {
    pub policies: PolicySet,
    pub graph: Box<dyn GraphStore>,
    pub embedder: Embedder,
    pub index: CodeIndex,
}

impl Auditor {
    pub fn audit(&self, req: AuditRequest) -> Result<AuditResponse>;
    pub fn explain(&self, file: &Path) -> Result<ExplainResponse>;
    pub fn audit_diff(&self, diff: &Diff) -> Result<AuditResponse>;
}
```

**Dependencies.** `illuminate-core`, `illuminate-embed`, `illuminate-index`, `serde`, `toml`.

**Timeline.** v0.1.

---

## `illuminate-watch`

**Responsibility.** The daemon harness. Long-running process that hosts the trail watcher, the extraction worker, and the (optional) webhook receivers. Keeps SQLite writers serialized.

**Public API.**

```rust
pub struct Daemon {
    pub config: DaemonConfig,
    pub store: Arc<Mutex<Box<dyn GraphStore>>>,
}

impl Daemon {
    pub fn new(repo_root: &Path) -> Result<Self>;
    pub fn run(&mut self) -> Result<()>;
    pub fn shutdown(&mut self) -> Result<()>;
}
```

**Dependencies.** `tokio`, `illuminate-trail`, `illuminate-extract`, `illuminate-reflect`.

**Timeline.** v0.1 ships a foreground daemon (run via `illuminate serve --daemon`). v0.2 adds systemd / launchd integration.

---

## `illuminate-reflect`

**Responsibility.** Failure capture as Reflexion-pattern episodes in the decision graph. Implements the Reflexion loop from agent research: when an agent fails, record the failure + root cause + corrective action so future audits surface the lesson and prevent the same mistake. Backed by the same `Graph` that decisions live in — failures are first-class graph episodes with `source = "reflexion"`.

> **Naming note.** Earlier docs described this crate's public API as `FailureRecord` with `FailureSink::record`. The implementation has always exposed `ReflexionInput` / `ReflexionEpisode` / `ReflexionStore` because the design follows the Reflexion paper (Shinn et al., 2023) — store failures as searchable episodes in the same graph as decisions, surface them via the same query path. The MCP `illuminate_reflect` tool and `illuminate failures register` CLI both consume this API.

**Public API.**

```rust
pub struct ReflexionInput {
    pub failure: String,
    pub root_cause: String,
    pub corrective_action: String,
    pub files_affected: Vec<String>,
    pub severity: Severity,
}

pub enum Severity { Low, Medium, High, Critical }

pub struct ReflexionEpisode {
    pub episode_id: String,
    pub failure: String,
    pub root_cause: String,
    pub corrective_action: String,
    pub files_affected: Vec<String>,
    pub severity: Severity,
    pub recorded_at: DateTime<Utc>,
}

pub struct ReflexionStore {
    /* graph handle */
}

impl ReflexionStore {
    pub fn new(graph: Graph) -> Self;
    pub fn record(&mut self, input: &ReflexionInput) -> Result<String>;
    pub fn find_relevant(&self, entity_names: &[String], file_paths: &[&str], limit: usize) -> Result<Vec<ReflexionEpisode>>;
}
```

`ReflexionStore::record` writes a graph episode with `source = "reflexion"` and metadata containing the structured fields. `Auditor::audit_with_reflexions` consumes `ReflexionStore::find_relevant` to surface lessons during audits.

**Dependencies.** `illuminate-core`, `serde`, `chrono`.

**Timeline.** v0.1: manual CLI (`illuminate reflect ...`) + MCP tool. CI/Sentry/PagerDuty webhook receivers remain deferred.

---

## `illuminate-route`

**Responsibility.** Subject-to-reading-plan generator. Given a natural-language subject ("how does our payment retry logic work?"), runs RRF (Reciprocal Rank Fusion) across FTS5 and semantic search over the decision graph and returns a ranked reading plan: relevant decisions, code files, and an estimated-token budget. Used by the MCP `illuminate_route` tool and the CLI's onboarding helpers.

> **Naming note.** Earlier docs (pre-v0.10) described `illuminate-route` as the LLM fallback router for ingestion. That responsibility actually lives in `illuminate-extract::llm_extract` (PII stripping via the optional `cloakpipe` Cargo feature, OpenRouter/OpenAI providers). The `illuminate-route` crate has consistently been the reading-plan generator; this section now documents that.

**Public API.**

```rust
pub struct ReadingPlan {
    pub decisions: Vec<DecisionEntry>,
    pub code_files: Vec<FileEntry>,
    pub estimated_tokens: usize,
}

pub struct DecisionEntry {
    pub id: String,
    pub content: String,
    pub source: Option<String>,
    pub score: f64,
}

pub struct FileEntry {
    pub path: String,
    pub symbols: Vec<String>,
    pub priority: u8,
    pub estimated_tokens: usize,
}

pub fn route(
    graph: &Graph,
    embed: Option<&EmbedEngine>,
    subject: &str,
    limit: usize,
) -> Result<ReadingPlan>;
```

When `embed: Some(_)` is supplied, `route` runs `Graph::search_fused` (RRF over FTS5 + semantic) and `embed: None` falls back to FTS5-only. The function never panics on missing data — empty graph yields an empty `ReadingPlan`.

**Dependencies.** `illuminate-core`, `illuminate-embed`, `serde`.

**Timeline.** v0.1 shipped FTS5+semantic. v0.7 added test coverage for the embed-disabled fallback path.

---

## `illuminate-mcp`

**Responsibility.** JSON-RPC server speaking the MCP protocol. Exposes audit, explain, search, decisions, failures.

**Tools exposed.**

| Tool | Calls |
|------|-------|
| `illuminate_audit` | `Auditor::audit` |
| `illuminate_explain` | `Auditor::explain` |
| `illuminate_search` | `GraphStore::semantic_search` + FTS5 |
| `illuminate_decisions_for` | path → list of decisions |
| `illuminate_failures_for` | path → list of failures |
| `illuminate_get_wiki_page` | id → markdown content |

See `MCP.md` for the full protocol surface.

**Public API.**

```rust
pub struct McpServer {
    auditor: Arc<Auditor>,
    store: Arc<dyn GraphStore>,
}

impl McpServer {
    pub fn run_stdio(self) -> Result<()>;
    pub fn run_streamable_http(self, addr: SocketAddr) -> Result<()>;
}
```

**Dependencies.** `illuminate-audit`, `illuminate-core`, `serde_json`, `tokio`.

**Timeline.** v0.1 ships stdio. v0.2 adds streamable HTTP for remote integrations.

---

## `illuminate-cli`

**Responsibility.** The single binary. Subcommands wire everything together.

**Subcommand surface (v0.1).**

```
illuminate init [--claude] [--cursor] [--codex] [--no-bootstrap] [--interactive]
illuminate audit "<plan>" [--files PATH...] [--rationale TEXT]
illuminate audit-pr <pr-number> [--repo OWNER/REPO]
illuminate audit-diff
illuminate explain <path>
illuminate decisions list [--module SLUG] [--tag TAG] [--include-superseded]
illuminate decisions show <id>
illuminate failures list
illuminate failure log [--title T] [--root-cause R] [--fix F] [--files F1,F2] [--severity S]
illuminate wiki rebuild
illuminate wiki serve [--port 8765]
illuminate wiki review            # walk the low-confidence candidate queue
illuminate wiki lint
illuminate index [--enrich]
illuminate bootstrap [--source git|adr|readme|interview] [--since DATE]
illuminate models download
illuminate serve                  # start MCP server (stdio)
illuminate serve --daemon         # start watcher daemon
illuminate stats [audit|llm|graph]
illuminate status
illuminate rebuild                # rebuild graph.db from wiki + trail
illuminate forget <id>
illuminate purge --decision <id>  # confirms, then deletes
illuminate trail purge --older-than DAYS
```

See `CLI.md` for full per-command documentation.

**Dependencies.** All other crates.

**Timeline.** v0.1.

---

## What's not its own crate (and why)

A few things that *could* be crates but aren't:

- **`illuminate-policy`** — policy DSL parsing. Lives inside `illuminate-audit`. If the DSL grows beyond ~500 LoC, extract.
- **`illuminate-bootstrap`** — extracted in v0.4 to host agent-files / ADR / git-history / README sources cleanly. Now its own crate at `crates/illuminate-bootstrap/`.
- **`illuminate-pii`** — PII stripping for LLM fallback. Lives inside `illuminate-extract::llm_extract` (gated on the optional `cloakpipe` Cargo feature) because it's tightly coupled to the LLM call path there, not in `illuminate-route`. Extract if multiple consumers need it.

Resist the urge to over-decompose. The current 11 crates already exceeds many production Rust workspaces. Adding more crates per concern slows compile times and obscures the architecture.

---

## Compile-time targets

- All crates compile to `cdylib` + `rlib` where useful (none currently need cdylib).
- `illuminate-cli` is the only `bin = true` crate.
- `cargo build --release` target time on a modern laptop: < 90 seconds for a clean build.
- Final stripped binary size target: < 80 MB (including ONNX dependencies; models are external).

---

## Testing strategy per crate

| Crate | Test types |
|-------|-----------|
| `illuminate-core` | Unit tests for graph operations; round-trip tests for SQLite serialization. |
| `illuminate-trail` | Fixture-based tests with sample Claude/Cursor/Codex jsonl files. |
| `illuminate-extract` | Integration tests against a small fixed corpus; assert entity/relation counts. |
| `illuminate-embed` | Smoke tests; assert vector dim = 384, deterministic for same input. |
| `illuminate-index` | Tree-sitter fixture tests per language. |
| `illuminate-audit` | Property-style tests: same input → same output. Policy DSL parser tests. |
| `illuminate-watch` | Daemon lifecycle tests with mock workers. |
| `illuminate-reflect` | Reflexion store round-trip tests; `find_relevant` ranking tests. |
| `illuminate-route` | Reading-plan ranking tests across FTS5-only, semantic-only, and fused paths. |
| `illuminate-mcp` | JSON-RPC contract tests. |
| `illuminate-cli` | End-to-end smoke tests via `assert_cmd`. |

Coverage target: 80%+ overall (per `rules/common/testing.md`).
