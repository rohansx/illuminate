//! illuminate-reflect: Reflexion loop for AI coding agents.
//!
//! Captures agent failures as "reflexion episodes" in the decision graph.
//! Future audit calls surface these lessons to prevent repeated mistakes.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Input for recording a reflexion episode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflexionInput {
    pub failure: String,
    pub root_cause: String,
    pub corrective_action: String,
    pub files_affected: Vec<String>,
    pub severity: Severity,
}

/// Severity of a reflexion episode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Low => write!(f, "low"),
            Severity::Medium => write!(f, "medium"),
            Severity::High => write!(f, "high"),
            Severity::Critical => write!(f, "critical"),
        }
    }
}

/// A stored reflexion episode with full metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflexionEpisode {
    pub episode_id: String,
    pub failure: String,
    pub root_cause: String,
    pub corrective_action: String,
    pub files_affected: Vec<String>,
    pub severity: Severity,
    pub recorded_at: DateTime<Utc>,
}

/// Store and retrieve reflexion episodes from the decision graph.
pub struct ReflexionStore {
    graph: illuminate::Graph,
}

impl ReflexionStore {
    pub fn new(graph: illuminate::Graph) -> Self {
        Self { graph }
    }

    /// Record a new reflexion episode.
    ///
    /// Creates an episode in the decision graph with source="reflexion" and
    /// stores the structured failure/lesson data in the episode metadata.
    pub fn record(&self, input: &ReflexionInput) -> illuminate::Result<String> {
        let content = format!(
            "FAILURE: {}\nROOT CAUSE: {}\nCORRECTIVE ACTION: {}",
            input.failure, input.root_cause, input.corrective_action
        );

        let mut metadata = serde_json::Map::new();
        metadata.insert(
            "reflexion".to_string(),
            serde_json::json!({
                "failure": input.failure,
                "root_cause": input.root_cause,
                "corrective_action": input.corrective_action,
                "severity": input.severity,
                "files_affected": input.files_affected,
            }),
        );

        let episode = illuminate::Episode {
            id: uuid::Uuid::now_v7().to_string(),
            content,
            source: Some("reflexion".to_string()),
            recorded_at: Utc::now(),
            metadata: Some(serde_json::Value::Object(metadata)),
        };

        let result = self.graph.add_episode(episode)?;
        Ok(result.episode_id)
    }

    /// Find reflexion episodes relevant to the given entities and file paths.
    ///
    /// Searches by:
    /// 1. FTS5 match on entity names
    /// 2. File path match in metadata
    /// Returns up to `limit` results, most recent first.
    pub fn find_relevant(
        &self,
        entities: &[String],
        files: &[String],
        limit: usize,
    ) -> illuminate::Result<Vec<ReflexionEpisode>> {
        let mut results = Vec::new();

        // Search by entity names
        for entity in entities {
            let search_results = self.graph.search(entity, limit * 2)?;
            for (episode, _score) in search_results {
                if episode.source.as_deref() == Some("reflexion") {
                    if let Some(refl) = parse_reflexion(&episode) {
                        if !results.iter().any(|r: &ReflexionEpisode| r.episode_id == refl.episode_id) {
                            results.push(refl);
                        }
                    }
                }
            }
        }

        // Search by file paths
        for file in files {
            let search_results = self.graph.search(file, limit * 2)?;
            for (episode, _score) in search_results {
                if episode.source.as_deref() == Some("reflexion") {
                    if let Some(refl) = parse_reflexion(&episode) {
                        // Check if any affected file matches
                        let file_matches = refl.files_affected.iter().any(|f| {
                            f == file || file.contains(f.as_str()) || f.contains(file.as_str())
                        });
                        if file_matches && !results.iter().any(|r: &ReflexionEpisode| r.episode_id == refl.episode_id) {
                            results.push(refl);
                        }
                    }
                }
            }
        }

        // Sort by recorded_at descending (most recent first) and limit
        results.sort_by(|a, b| b.recorded_at.cmp(&a.recorded_at));
        results.truncate(limit);

        Ok(results)
    }
}

/// Parse a reflexion episode from an Episode's metadata.
fn parse_reflexion(episode: &illuminate::Episode) -> Option<ReflexionEpisode> {
    let metadata = episode.metadata.as_ref()?;
    let refl = metadata.get("reflexion")?;

    Some(ReflexionEpisode {
        episode_id: episode.id.clone(),
        failure: refl.get("failure")?.as_str()?.to_string(),
        root_cause: refl.get("root_cause")?.as_str()?.to_string(),
        corrective_action: refl.get("corrective_action")?.as_str()?.to_string(),
        files_affected: refl
            .get("files_affected")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default(),
        severity: refl
            .get("severity")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or(Severity::Medium),
        recorded_at: episode.recorded_at,
    })
}
