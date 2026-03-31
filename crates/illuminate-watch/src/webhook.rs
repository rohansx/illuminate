//! Webhook receiver - HTTP endpoint for external ingestion (slack, jira, etc).

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::signal::score_decision_signal;

/// Webhook request payload.
#[derive(Debug, Clone, Deserialize)]
pub struct WebhookPayload {
    pub text: String,
    pub source: Option<String>,
    pub source_ref: Option<String>,
    pub author: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// Webhook response.
#[derive(Debug, Serialize)]
pub struct WebhookResponse {
    pub status: String,
    pub episode_id: String,
    pub entities_extracted: usize,
    pub relations_extracted: usize,
}

/// Health check response.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub graph_episodes: usize,
}

/// Shared state for the webhook server.
pub struct WebhookState {
    pub graph: illuminate::Graph,
    pub signal_threshold: f64,
}

/// Start the webhook HTTP server.
///
/// Endpoints:
/// - POST /ingest - ingest a decision from an external source
/// - GET /health - health check
pub async fn serve(
    graph: illuminate::Graph,
    port: u16,
    signal_threshold: f64,
) -> crate::Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let state = Arc::new(Mutex::new(WebhookState {
        graph,
        signal_threshold,
    }));

    let addr = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&addr).await?;
    eprintln!("illuminate webhook: listening on http://{addr}");

    loop {
        let (mut stream, _) = listener.accept().await?;
        let state = Arc::clone(&state);

        tokio::spawn(async move {
            let mut buf = vec![0u8; 16384];
            let n = match stream.read(&mut buf).await {
                Ok(n) => n,
                Err(_) => return,
            };

            let request = String::from_utf8_lossy(&buf[..n]);
            let (status, body) = handle_request(&request, &state).await;

            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );

            let _ = stream.write_all(response.as_bytes()).await;
        });
    }
}

async fn handle_request(
    raw: &str,
    state: &Arc<Mutex<WebhookState>>,
) -> (String, String) {
    let lines: Vec<&str> = raw.lines().collect();
    let first_line = lines.first().unwrap_or(&"");

    if first_line.starts_with("GET /health") {
        return handle_health(state).await;
    }

    if first_line.starts_with("POST /ingest") {
        // find json body after double newline
        let body = raw
            .split("\r\n\r\n")
            .nth(1)
            .or_else(|| raw.split("\n\n").nth(1))
            .unwrap_or("");

        return handle_ingest(body, state).await;
    }

    (
        "404 Not Found".to_string(),
        r#"{"error":"not found"}"#.to_string(),
    )
}

async fn handle_health(state: &Arc<Mutex<WebhookState>>) -> (String, String) {
    let state = state.lock().await;
    let episodes = state
        .graph
        .stats()
        .map(|s| s.episode_count)
        .unwrap_or(0);

    let resp = HealthResponse {
        status: "healthy".to_string(),
        version: "0.8.0".to_string(),
        graph_episodes: episodes,
    };

    (
        "200 OK".to_string(),
        serde_json::to_string(&resp).unwrap_or_default(),
    )
}

async fn handle_ingest(
    body: &str,
    state: &Arc<Mutex<WebhookState>>,
) -> (String, String) {
    let payload: WebhookPayload = match serde_json::from_str(body) {
        Ok(p) => p,
        Err(e) => {
            return (
                "400 Bad Request".to_string(),
                format!(r#"{{"error":"invalid json: {e}"}}"#),
            );
        }
    };

    let state = state.lock().await;

    // check decision signal
    let score = score_decision_signal(&payload.text);
    if score < state.signal_threshold {
        return (
            "200 OK".to_string(),
            format!(r#"{{"status":"skipped","reason":"below signal threshold ({score:.2} < {})"}}"#, state.signal_threshold),
        );
    }

    let source = payload.source.unwrap_or_else(|| "webhook".to_string());

    let mut metadata = serde_json::Map::new();
    if let Some(ref source_ref) = payload.source_ref {
        metadata.insert("source_ref".to_string(), serde_json::json!(source_ref));
    }
    if let Some(ref author) = payload.author {
        metadata.insert("author".to_string(), serde_json::json!(author));
    }
    if let Some(ref tags) = payload.tags {
        metadata.insert("tags".to_string(), serde_json::json!(tags));
    }
    metadata.insert("signal_score".to_string(), serde_json::json!(score));

    let episode = illuminate::Episode {
        id: uuid::Uuid::now_v7().to_string(),
        content: payload.text,
        source: Some(source),
        recorded_at: chrono::Utc::now(),
        metadata: Some(serde_json::Value::Object(metadata)),
    };

    match state.graph.add_episode(episode) {
        Ok(result) => {
            let resp = WebhookResponse {
                status: "ok".to_string(),
                episode_id: result.episode_id,
                entities_extracted: result.entities_extracted,
                relations_extracted: result.edges_created,
            };
            (
                "200 OK".to_string(),
                serde_json::to_string(&resp).unwrap_or_default(),
            )
        }
        Err(e) => (
            "500 Internal Server Error".to_string(),
            format!(r#"{{"error":"{e}"}}"#),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_webhook_payload() {
        let json = r#"{
            "text": "team decided to freeze auth module for pci audit",
            "source": "slack",
            "source_ref": "thread:C04ABCD/1234567",
            "author": "priya",
            "tags": ["security", "auth"]
        }"#;

        let payload: WebhookPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.text, "team decided to freeze auth module for pci audit");
        assert_eq!(payload.source.as_deref(), Some("slack"));
        assert_eq!(payload.author.as_deref(), Some("priya"));
        assert_eq!(payload.tags.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn parse_minimal_payload() {
        let json = r#"{"text": "chose postgres"}"#;
        let payload: WebhookPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.text, "chose postgres");
        assert!(payload.source.is_none());
        assert!(payload.author.is_none());
        assert!(payload.tags.is_none());
    }

    #[test]
    fn health_response_serializes() {
        let resp = HealthResponse {
            status: "healthy".to_string(),
            version: "0.8.0".to_string(),
            graph_episodes: 42,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("healthy"));
        assert!(json.contains("42"));
    }
}
