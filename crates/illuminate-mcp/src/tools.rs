use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use illuminate::{Episode, Graph};
use illuminate_embed::EmbedEngine;
use illuminate_audit::policy::IntentPolicy;
use illuminate_reflect::Severity;
use serde_json::{Value, json};

pub struct ToolContext {
    pub graph: Arc<Mutex<Graph>>,
    pub embed: Option<Arc<EmbedEngine>>,
    /// In-memory embedding cache: episode_id → 384-dim vector.
    embedding_cache: Mutex<Option<HashMap<String, Vec<f32>>>>,
    /// Intent policies loaded from illuminate.toml.
    policies: Vec<IntentPolicy>,
}

impl ToolContext {
    pub fn new(graph: Graph, embed: Option<EmbedEngine>) -> Self {
        Self {
            graph: Arc::new(Mutex::new(graph)),
            embed: embed.map(Arc::new),
            embedding_cache: Mutex::new(None),
            policies: Vec::new(),
        }
    }

    pub fn with_policies(graph: Graph, embed: Option<EmbedEngine>, policies: Vec<IntentPolicy>) -> Self {
        Self {
            graph: Arc::new(Mutex::new(graph)),
            embed: embed.map(Arc::new),
            embedding_cache: Mutex::new(None),
            policies,
        }
    }

    /// Populate the in-memory embedding cache from SQLite if it hasn't been loaded yet.
    /// Subsequent calls return immediately — the Option acts as a once-flag.
    fn warm_cache(&self) -> Result<(), String> {
        let mut cache = self.embedding_cache.lock().map_err(|e| e.to_string())?;
        if cache.is_none() {
            let graph = self.graph.lock().map_err(|e| e.to_string())?;
            let rows = graph.get_embeddings().map_err(|e| e.to_string())?;
            *cache = Some(rows.into_iter().collect());
        }
        Ok(())
    }

    /// Tool: add_episode
    /// Adds a new episode to the graph, computes and stores its embedding.
    pub async fn add_episode(&self, args: Value) -> Result<Value, String> {
        let text = args["text"]
            .as_str()
            .ok_or("missing required field: text")?
            .to_string();

        let source = args["source"].as_str().map(|s| s.to_string());

        let mut builder = Episode::builder(&text);
        if let Some(ref src) = source {
            builder = builder.source(src);
        }
        if let Some(tags) = args["tags"].as_array() {
            for tag in tags {
                if let Some(t) = tag.as_str() {
                    builder = builder.tag(t);
                }
            }
        }
        let episode = builder.build();
        let episode_id = episode.id.clone();

        // Store episode
        let result = {
            let graph = self.graph.lock().map_err(|e| e.to_string())?;
            graph.add_episode(episode).map_err(|e| e.to_string())?
        };

        // Compute embedding and persist to SQLite (skipped if embed engine is unavailable)
        if let Some(ref embed) = self.embed {
            let embedding = embed.embed(&text).map_err(|e| e.to_string())?;
            {
                let graph = self.graph.lock().map_err(|e| e.to_string())?;
                graph
                    .store_embedding(&episode_id, &embedding)
                    .map_err(|e| e.to_string())?;
            }

            // Insert into in-memory cache so find_precedents sees it immediately
            // without another SQLite round-trip.
            if let Ok(mut cache) = self.embedding_cache.lock()
                && let Some(ref mut map) = *cache
            {
                map.insert(episode_id.clone(), embedding);
                // If cache is None (never warmed), leave it — it will be populated
                // from SQLite (including this episode) on the first find_precedents call.
            }
        }

        Ok(json!({
            "episode_id": result.episode_id,
            "entities_found": result.entities_extracted,
            "edges_created": result.edges_created,
        }))
    }

    /// Tool: search
    /// Fused FTS5 + semantic search via RRF. Falls back to FTS5-only if embed is unavailable.
    pub async fn search(&self, args: Value) -> Result<Value, String> {
        let query = args["query"]
            .as_str()
            .ok_or("missing required field: query")?
            .to_string();
        let limit = args["limit"].as_u64().unwrap_or(10) as usize;

        let items: Vec<Value> = if let Some(ref embed) = self.embed {
            // Fused FTS5 + semantic search via RRF
            let query_embedding = embed.embed(&query).map_err(|e| e.to_string())?;
            let results = {
                let graph = self.graph.lock().map_err(|e| e.to_string())?;
                graph
                    .search_fused(&query, &query_embedding, limit)
                    .map_err(|e| e.to_string())?
            };
            results
                .into_iter()
                .map(|r| {
                    json!({
                        "id": r.episode.id,
                        "content": r.episode.content,
                        "score": r.score,
                        "source": r.episode.source,
                        "recorded_at": r.episode.recorded_at.to_rfc3339(),
                    })
                })
                .collect()
        } else {
            // FTS5-only search (embedding not available)
            let results = {
                let graph = self.graph.lock().map_err(|e| e.to_string())?;
                graph.search(&query, limit).map_err(|e| e.to_string())?
            };
            results
                .into_iter()
                .map(|(episode, score)| {
                    json!({
                        "id": episode.id,
                        "content": episode.content,
                        "score": score,
                        "source": episode.source,
                        "recorded_at": episode.recorded_at.to_rfc3339(),
                    })
                })
                .collect()
        };

        Ok(json!(items))
    }

    /// Tool: get_decision
    /// Retrieve a specific episode by ID with full context.
    pub async fn get_decision(&self, args: Value) -> Result<Value, String> {
        let id = args["id"]
            .as_str()
            .ok_or("missing required field: id")?
            .to_string();

        let graph = self.graph.lock().map_err(|e| e.to_string())?;

        let episode = graph
            .get_episode(&id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("episode not found: {id}"))?;

        // Serialize episode to JSON
        let episode_json = serde_json::to_value(&episode).map_err(|e| e.to_string())?;

        Ok(json!({
            "episode": episode_json,
        }))
    }

    /// Tool: traverse
    /// Traverse the knowledge graph from an entity.
    pub async fn traverse(&self, args: Value) -> Result<Value, String> {
        let entity_name = args["entity_name"]
            .as_str()
            .ok_or("missing required field: entity_name")?
            .to_string();
        let max_depth = (args["max_depth"].as_u64().unwrap_or(2) as usize).min(5);

        let graph = self.graph.lock().map_err(|e| e.to_string())?;

        let entity = graph
            .get_entity_by_name(&entity_name)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("entity not found: {entity_name}"))?;

        let (entities, edges) = graph
            .traverse(&entity.id, max_depth)
            .map_err(|e| e.to_string())?;

        let entities_json: Vec<Value> = entities
            .into_iter()
            .map(|e| serde_json::to_value(e).unwrap_or(Value::Null))
            .collect();

        let edges_json: Vec<Value> = edges
            .into_iter()
            .map(|e| serde_json::to_value(e).unwrap_or(Value::Null))
            .collect();

        Ok(json!({
            "entities": entities_json,
            "edges": edges_json,
        }))
    }

    /// Tool: find_precedents
    /// Find past episodes most semantically similar to a given context.
    ///
    /// Uses an in-memory embedding cache so the full SQLite embedding table is
    /// only read once per process lifetime. Subsequent calls score entirely in
    /// RAM — O(n) dot-products, no I/O.
    pub async fn find_precedents(&self, args: Value) -> Result<Value, String> {
        let context = args["context"]
            .as_str()
            .ok_or("missing required field: context")?
            .to_string();
        let limit = args["limit"].as_u64().unwrap_or(5) as usize;

        let embed = match self.embed {
            Some(ref e) => e,
            None => {
                return Ok(
                    json!({"error": "embedding not available, start with ILLUMINATE_NO_EMBED unset"}),
                );
            }
        };

        // Embed the query (always ~20-50ms CPU inference, unavoidable)
        let context_embedding = embed.embed(&context).map_err(|e| e.to_string())?;

        // Ensure the cache is warm (no-op after first call)
        self.warm_cache()?;

        // Score entirely in RAM — no SQLite I/O
        let mut scored: Vec<(String, f32)> = {
            let cache = self.embedding_cache.lock().map_err(|e| e.to_string())?;
            cache
                .as_ref()
                .expect("cache must be Some after warm_cache")
                .iter()
                .map(|(id, vec)| {
                    let sim = EmbedEngine::cosine_similarity(&context_embedding, vec);
                    (id.clone(), sim)
                })
                .collect()
        };

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let top: Vec<(String, f32)> = scored.into_iter().take(limit).collect();

        let mut results = Vec::new();
        let graph = self.graph.lock().map_err(|e| e.to_string())?;
        for (id, sim) in top {
            if let Ok(Some(ep)) = graph.get_episode(&id) {
                results.push(json!({
                    "id": ep.id,
                    "content": ep.content,
                    "source": ep.source,
                    "recorded_at": ep.recorded_at.to_rfc3339(),
                    "similarity": sim,
                }));
            }
        }

        Ok(json!(results))
    }

    /// Tool: list_entities
    /// List entities in the graph, optionally filtered by type.
    pub async fn list_entities(&self, args: Value) -> Result<Value, String> {
        let entity_type = args["entity_type"].as_str().map(|s| s.to_string());
        let limit = args["limit"].as_u64().unwrap_or(100) as usize;
        let offset = args["offset"].as_u64().unwrap_or(0) as usize;

        let graph = self.graph.lock().map_err(|e| e.to_string())?;

        let entities = graph
            .list_entities(entity_type.as_deref(), limit)
            .map_err(|e| e.to_string())?;

        // Skip `offset` entities (list_entities doesn't support offset natively)
        let entities: Vec<Value> = entities
            .into_iter()
            .skip(offset)
            .map(|e| serde_json::to_value(e).unwrap_or(Value::Null))
            .collect();

        Ok(json!({
            "entities": entities,
            "count": entities.len(),
        }))
    }

    /// Tool: export_graph
    /// Export all entities and edges from the graph.
    pub async fn export_graph(&self, args: Value) -> Result<Value, String> {
        let entity_type = args["entity_type"].as_str().map(|s| s.to_string());
        let include_episodes = args["include_episodes"].as_bool().unwrap_or(false);
        let limit = args["limit"].as_u64().unwrap_or(10000) as usize;

        let graph = self.graph.lock().map_err(|e| e.to_string())?;

        let entities = graph
            .list_entities(entity_type.as_deref(), limit)
            .map_err(|e| e.to_string())?;

        // Collect all edges for all entities
        let mut all_edges = Vec::new();
        let mut seen_edge_ids = std::collections::HashSet::new();
        for entity in &entities {
            let edges = graph
                .get_edges_for_entity(&entity.id)
                .map_err(|e| e.to_string())?;
            for edge in edges {
                if seen_edge_ids.insert(edge.id.clone()) {
                    all_edges.push(edge);
                }
            }
        }

        let entities_json: Vec<Value> = entities
            .into_iter()
            .map(|e| serde_json::to_value(e).unwrap_or(Value::Null))
            .collect();

        let edges_json: Vec<Value> = all_edges
            .into_iter()
            .map(|e| serde_json::to_value(e).unwrap_or(Value::Null))
            .collect();

        let mut result = json!({
            "entities": entities_json,
            "edges": edges_json,
            "entity_count": entities_json.len(),
            "edge_count": edges_json.len(),
        });

        if include_episodes {
            let episodes = graph.list_episodes(limit, 0).map_err(|e| e.to_string())?;
            let episodes_json: Vec<Value> = episodes
                .into_iter()
                .map(|e| serde_json::to_value(e).unwrap_or(Value::Null))
                .collect();
            result["episodes"] = json!(episodes_json);
            result["episode_count"] = json!(episodes_json.len());
        }

        Ok(result)
    }

    /// Tool: traverse_batch
    /// Traverse multiple entities in one call, returning a merged result.
    ///
    /// Replaces N sequential `traverse` calls with a single round-trip.
    /// Entities not found in the graph are silently skipped (same behaviour as
    /// individual `traverse` returning an error for unknown entities).
    pub async fn traverse_batch(&self, args: Value) -> Result<Value, String> {
        let entity_names: Vec<String> = args["entity_names"]
            .as_array()
            .ok_or("missing required field: entity_names")?
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        if entity_names.is_empty() {
            return Ok(json!({"entities": [], "edges": []}));
        }

        let max_depth = (args["max_depth"].as_u64().unwrap_or(2) as usize).min(5);

        let graph = self.graph.lock().map_err(|e| e.to_string())?;

        let mut all_entities: Vec<Value> = Vec::new();
        let mut all_edges: Vec<Value> = Vec::new();
        // Dedup by ID across multiple traversals
        let mut seen_entities = std::collections::HashSet::new();
        let mut seen_edges = std::collections::HashSet::new();

        for name in &entity_names {
            let entity = match graph.get_entity_by_name(name) {
                Ok(Some(e)) => e,
                Ok(None) | Err(_) => continue, // skip unknown entities
            };

            let (entities, edges) = match graph.traverse(&entity.id, max_depth) {
                Ok(r) => r,
                Err(_) => continue,
            };

            for e in entities {
                if seen_entities.insert(e.id.clone()) {
                    all_entities.push(serde_json::to_value(e).unwrap_or(Value::Null));
                }
            }
            for e in edges {
                if seen_edges.insert(e.id.clone()) {
                    all_edges.push(serde_json::to_value(e).unwrap_or(Value::Null));
                }
            }
        }

        Ok(json!({
            "entities": all_entities,
            "edges": all_edges,
        }))
    }

    // ── illuminate-specific tools ──

    /// Tool: illuminate_audit
    /// Cross-reference an agent's proposed plan against the decision graph and intent policies.
    pub async fn illuminate_audit(&self, args: Value) -> Result<Value, String> {
        let plan = args["plan"]
            .as_str()
            .ok_or("missing required field: plan")?
            .to_string();

        let graph = self.graph.lock().map_err(|e| e.to_string())?;

        // Build auditor with a clone-free approach: we need the graph for Auditor
        // but it's behind a Mutex. Create a temporary in-memory graph for audit,
        // or use the search API directly.
        // For graph conflict detection, we search directly
        let mut policy_violations = Vec::new();
        let plan_lower = plan.to_lowercase();

        // Check intent policies
        for policy in &self.policies {
            match policy {
                IntentPolicy::MustUse { name, entity, reject, reason, severity } => {
                    for rejected in reject {
                        if plan_lower.contains(&rejected.to_lowercase()) {
                            policy_violations.push(json!({
                                "policy": name,
                                "expected": entity,
                                "found": rejected,
                                "reason": reason,
                                "severity": format!("{severity:?}").to_lowercase(),
                            }));
                        }
                    }
                }
                IntentPolicy::Frozen { name, paths, reason, severity, expires } => {
                    if let Some(exp) = expires {
                        if chrono::Utc::now() > *exp { continue; }
                    }
                    for path in paths {
                        let base = path.to_lowercase().replace("/**", "").replace("/*", "");
                        if plan_lower.contains(&base) {
                            policy_violations.push(json!({
                                "policy": name,
                                "frozen_path": path,
                                "reason": reason,
                                "severity": format!("{severity:?}").to_lowercase(),
                            }));
                        }
                    }
                }
                IntentPolicy::RejectedPattern { name, pattern, reason, severity, .. } => {
                    if plan_lower.contains(&pattern.to_lowercase()) {
                        policy_violations.push(json!({
                            "policy": name,
                            "pattern": pattern,
                            "reason": reason,
                            "severity": format!("{severity:?}").to_lowercase(),
                        }));
                    }
                }
                _ => {}
            }
        }

        // Search graph for decision conflicts
        let mut decision_conflicts = Vec::new();
        let search_results = graph.search(&plan, 20).map_err(|e| e.to_string())?;
        for (episode, score) in &search_results {
            let content_lower = episode.content.to_lowercase();
            let has_rejection = ["rejected", "not ", "instead of", "over", "rather than", "avoid", "dropped"]
                .iter()
                .any(|kw| content_lower.contains(kw));

            if has_rejection && *score > 0.0 {
                decision_conflicts.push(json!({
                    "id": episode.id,
                    "content": episode.content,
                    "source": episode.source,
                    "recorded_at": episode.recorded_at.to_rfc3339(),
                    "relevance_score": score,
                }));
            }
        }

        // Search reflexion episodes
        let mut reflexions = Vec::new();
        for (episode, _) in &search_results {
            if episode.source.as_deref() == Some("reflexion") {
                if let Some(meta) = &episode.metadata {
                    if let Some(refl) = meta.get("reflexion") {
                        reflexions.push(json!({
                            "failure": refl.get("failure"),
                            "root_cause": refl.get("root_cause"),
                            "corrective_action": refl.get("corrective_action"),
                            "severity": refl.get("severity"),
                        }));
                    }
                }
            }
        }

        let has_errors = !policy_violations.is_empty() || !decision_conflicts.is_empty();
        let has_warnings = !reflexions.is_empty();
        let status = if has_errors { "violation" } else if has_warnings { "warning" } else { "pass" };

        Ok(json!({
            "status": status,
            "policy_violations": policy_violations,
            "decision_conflicts": decision_conflicts,
            "reflexions": reflexions,
        }))
    }

    /// Tool: illuminate_reflect
    /// Record a failure/lesson from the current agent session.
    pub async fn illuminate_reflect(&self, args: Value) -> Result<Value, String> {
        let failure = args["failure"]
            .as_str()
            .ok_or("missing required field: failure")?
            .to_string();
        let root_cause = args["root_cause"]
            .as_str()
            .ok_or("missing required field: root_cause")?
            .to_string();
        let corrective_action = args["corrective_action"]
            .as_str()
            .ok_or("missing required field: corrective_action")?
            .to_string();

        let files_affected: Vec<String> = args["files_affected"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        let severity = match args["severity"].as_str().unwrap_or("medium") {
            "low" => Severity::Low,
            "high" => Severity::High,
            "critical" => Severity::Critical,
            _ => Severity::Medium,
        };

        // Build the reflexion episode content and metadata
        let content = format!(
            "FAILURE: {failure}\nROOT CAUSE: {root_cause}\nCORRECTIVE ACTION: {corrective_action}"
        );

        let mut metadata = serde_json::Map::new();
        metadata.insert("reflexion".to_string(), json!({
            "failure": failure,
            "root_cause": root_cause,
            "corrective_action": corrective_action,
            "severity": severity,
            "files_affected": files_affected,
        }));

        let episode = Episode {
            id: uuid::Uuid::now_v7().to_string(),
            content,
            source: Some("reflexion".to_string()),
            recorded_at: chrono::Utc::now(),
            metadata: Some(Value::Object(metadata)),
        };
        let episode_id = episode.id.clone();

        let graph = self.graph.lock().map_err(|e| e.to_string())?;
        let result = graph.add_episode(episode).map_err(|e| e.to_string())?;

        Ok(json!({
            "episode_id": episode_id,
            "entities_extracted": result.entities_extracted,
            "edges_created": result.edges_created,
            "message": "Reflexion recorded. Future audits will surface this lesson.",
        }))
    }

    /// Tool: illuminate_route
    /// Given a subject, return a ranked reading plan of decisions and files.
    pub async fn illuminate_route(&self, args: Value) -> Result<Value, String> {
        let subject = args["subject"]
            .as_str()
            .ok_or("missing required field: subject")?
            .to_string();
        let limit = args["limit"].as_u64().unwrap_or(10) as usize;

        let graph = self.graph.lock().map_err(|e| e.to_string())?;
        let embed_ref = self.embed.as_deref();

        let plan = illuminate_route::route(&graph, embed_ref, &subject, limit)
            .map_err(|e| e.to_string())?;

        Ok(serde_json::to_value(plan).map_err(|e| e.to_string())?)
    }

    /// Tool: illuminate_stats
    /// Graph statistics: episodes, entities, edges, sources, DB size.
    pub async fn illuminate_stats(&self, _args: Value) -> Result<Value, String> {
        let graph = self.graph.lock().map_err(|e| e.to_string())?;
        let stats = graph.stats().map_err(|e| e.to_string())?;

        Ok(json!({
            "episodes": stats.episode_count,
            "entities": stats.entity_count,
            "edges": stats.edge_count,
            "anchors": stats.anchor_count,
            "sources": stats.sources,
            "db_size_bytes": stats.db_size_bytes,
        }))
    }

    /// Tool: illuminate_impact
    /// Given a decision ID, show every file and symbol anchored to that decision.
    pub async fn illuminate_impact(&self, args: Value) -> Result<Value, String> {
        let decision_id = args["decision_id"]
            .as_str()
            .ok_or("missing required field: decision_id")?
            .to_string();

        let graph = self.graph.lock().map_err(|e| e.to_string())?;

        let episode = graph
            .get_episode(&decision_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("decision not found: {decision_id}"))?;

        let anchors = graph
            .get_anchors_for_episode(&decision_id)
            .map_err(|e| e.to_string())?;

        let anchors_json: Vec<Value> = anchors
            .iter()
            .map(|a| {
                json!({
                    "file": a.file_path,
                    "symbol": a.symbol_name,
                    "lines": format!(
                        "{}-{}",
                        a.line_start.unwrap_or(0),
                        a.line_end.unwrap_or(0)
                    ),
                })
            })
            .collect();

        Ok(json!({
            "decision": {
                "id": episode.id,
                "content": episode.content,
                "source": episode.source,
                "recorded_at": episode.recorded_at.to_rfc3339(),
            },
            "anchors": anchors_json,
            "total_files_affected": anchors.len(),
        }))
    }

    /// Tool: illuminate_explain
    /// Given a file path, return all linked decisions via anchors.
    pub async fn illuminate_explain(&self, args: Value) -> Result<Value, String> {
        let path = args["path"]
            .as_str()
            .ok_or("missing required field: path")?
            .to_string();

        let graph = self.graph.lock().map_err(|e| e.to_string())?;

        let anchors = graph
            .get_anchors_for_file(&path)
            .map_err(|e| e.to_string())?;

        let mut decisions = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        for anchor in &anchors {
            if seen_ids.insert(anchor.episode_id.clone()) {
                if let Ok(Some(episode)) = graph.get_episode(&anchor.episode_id) {
                    decisions.push(json!({
                        "id": episode.id,
                        "content": episode.content,
                        "source": episode.source,
                        "recorded_at": episode.recorded_at.to_rfc3339(),
                        "anchor": {
                            "symbol": anchor.symbol_name,
                            "lines": format!(
                                "{}-{}",
                                anchor.line_start.unwrap_or(0),
                                anchor.line_end.unwrap_or(0)
                            ),
                        }
                    }));
                }
            }
        }

        Ok(json!({
            "path": path,
            "decisions": decisions,
            "total_decisions": decisions.len(),
        }))
    }
}

/// Wrap a tool result into an MCP content array.
pub fn tool_result(result: Result<Value, String>) -> Value {
    let text = match result {
        Ok(val) => serde_json::to_string_pretty(&val).unwrap_or_else(|_| val.to_string()),
        Err(msg) => serde_json::to_string_pretty(&json!({"error": msg}))
            .unwrap_or_else(|_| format!(r#"{{"error": "{msg}"}}"#)),
    };
    json!({
        "content": [{"type": "text", "text": text}]
    })
}

/// Returns the static tools list payload.
pub fn tools_list() -> Value {
    json!({
        "tools": [
            {
                "name": "add_episode",
                "description": "Add a new episode (decision, observation, or event) to the context graph. Extracts entities and relations automatically.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "text": {"type": "string", "description": "The episode content"},
                        "source": {"type": "string", "description": "Source label (e.g. 'slack', 'meeting', 'code-review')"},
                        "tags": {"type": "array", "items": {"type": "string"}, "description": "Optional tags"}
                    },
                    "required": ["text"]
                }
            },
            {
                "name": "search",
                "description": "Search the context graph using semantic + keyword fusion (RRF). Returns ranked episodes.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": {"type": "string", "description": "Search query"},
                        "limit": {"type": "integer", "description": "Max results (default 10)"},
                        "source": {"type": "string", "description": "Filter by source"}
                    },
                    "required": ["query"]
                }
            },
            {
                "name": "get_decision",
                "description": "Retrieve a specific episode by ID with full entity and relation context.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "string", "description": "Episode UUID"}
                    },
                    "required": ["id"]
                }
            },
            {
                "name": "traverse",
                "description": "Traverse the knowledge graph from an entity, returning connected entities and edges.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "entity_name": {"type": "string", "description": "Entity name to start from"},
                        "max_depth": {"type": "integer", "description": "Traversal depth (default 2, max 5)"}
                    },
                    "required": ["entity_name"]
                }
            },
            {
                "name": "traverse_batch",
                "description": "Traverse multiple entities in one call. Equivalent to N sequential traverse calls but with a single round-trip. Deduplicates entities and edges across all traversals.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "entity_names": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "List of entity names to traverse from"
                        },
                        "max_depth": {"type": "integer", "description": "Traversal depth per entity (default 2, max 5)"}
                    },
                    "required": ["entity_names"]
                }
            },
            {
                "name": "find_precedents",
                "description": "Find past decisions or context most semantically similar to a given situation.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "context": {"type": "string", "description": "Description of current situation"},
                        "limit": {"type": "integer", "description": "Max results (default 5)"}
                    },
                    "required": ["context"]
                }
            },
            {
                "name": "list_entities",
                "description": "List entities in the graph with optional type filter and pagination. Useful for graph visualization and exploration.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "entity_type": {"type": "string", "description": "Filter by entity type (e.g. 'person', 'technology', 'component')"},
                        "limit": {"type": "integer", "description": "Max results (default 100)"},
                        "offset": {"type": "integer", "description": "Skip first N results for pagination (default 0)"}
                    }
                }
            },
            {
                "name": "export_graph",
                "description": "Export all entities and edges from the graph. Optionally include episodes and filter by entity type.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "entity_type": {"type": "string", "description": "Filter entities by type"},
                        "include_episodes": {"type": "boolean", "description": "Include episodes in export (default false)"},
                        "limit": {"type": "integer", "description": "Max entities to export (default 10000)"}
                    }
                }
            },
            {
                "name": "illuminate_audit",
                "description": "Cross-reference an agent's proposed plan against the decision graph and intent policies. Returns structured warnings with source attribution. Use this BEFORE writing code to check for architectural conflicts.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "plan": {"type": "string", "description": "The agent's proposed plan or action in natural language"}
                    },
                    "required": ["plan"]
                }
            },
            {
                "name": "illuminate_reflect",
                "description": "Record a failure or lesson from the current session. Creates a reflexion episode so future agents don't repeat the same mistake.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "failure": {"type": "string", "description": "What went wrong"},
                        "root_cause": {"type": "string", "description": "Why it went wrong"},
                        "corrective_action": {"type": "string", "description": "What to do instead"},
                        "files_affected": {"type": "array", "items": {"type": "string"}, "description": "Affected file paths"},
                        "severity": {"type": "string", "enum": ["low", "medium", "high", "critical"], "description": "Failure severity (default: medium)"}
                    },
                    "required": ["failure", "root_cause", "corrective_action"]
                }
            },
            {
                "name": "illuminate_route",
                "description": "Given a subject, return a ranked reading plan of decisions and files. Helps agents understand unfamiliar parts of the codebase.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "subject": {"type": "string", "description": "Topic or subject to explore"},
                        "limit": {"type": "integer", "description": "Max entries (default 10)"}
                    },
                    "required": ["subject"]
                }
            },
            {
                "name": "illuminate_stats",
                "description": "Graph statistics: episode count, entity count, edge count, anchor count, source breakdown, database size.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "illuminate_impact",
                "description": "Given a decision ID, show every file and symbol anchored to that decision. Answers: what code breaks if we reverse this decision?",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "decision_id": {"type": "string", "description": "Episode/decision UUID"}
                    },
                    "required": ["decision_id"]
                }
            },
            {
                "name": "illuminate_explain",
                "description": "Given a file path, return all linked decisions and their context. Answers: why was this file built this way?",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "File path to explain"}
                    },
                    "required": ["path"]
                }
            }
        ]
    })
}
