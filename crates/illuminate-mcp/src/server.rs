use std::path::PathBuf;
use std::sync::Arc;

use illuminate::Graph;
use illuminate_audit::policy::{AuditConfig, IntentPolicy};
use illuminate_embed::EmbedEngine;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::protocol::{Request, Response, codes};
use crate::resources::{list_wiki_resources, read_wiki_resource};
use crate::tools::{ToolContext, tool_result, tools_list};

pub struct McpServer {
    ctx: Arc<ToolContext>,
}

impl McpServer {
    pub fn new(graph: Graph, embed: Option<EmbedEngine>) -> Self {
        Self {
            ctx: Arc::new(ToolContext::new(graph, embed)),
        }
    }

    pub fn with_policies(
        graph: Graph,
        embed: Option<EmbedEngine>,
        policies: Vec<IntentPolicy>,
    ) -> Self {
        Self {
            ctx: Arc::new(ToolContext::with_policies(graph, embed, policies)),
        }
    }

    /// Build an `McpServer` with both intent policies and a code-graph
    /// `index.db`. The connection to `index.db` is opened lazily on the
    /// first audit that supplies files — a missing path is silently ignored.
    pub fn with_index(
        graph: Graph,
        embed: Option<EmbedEngine>,
        policies: Vec<IntentPolicy>,
        index_db_path: Option<PathBuf>,
    ) -> Self {
        Self::with_index_and_root(graph, embed, policies, index_db_path, None)
    }

    /// Build an `McpServer` with index, policies, and a repo root used to
    /// normalize ABSOLUTE file paths agents pass via `illuminate_audit` into
    /// the repo-relative form the indexer stored. Without `repo_root`,
    /// absolute paths silently miss the index — see Task R / `Auditor::with_index_and_root`.
    pub fn with_index_and_root(
        graph: Graph,
        embed: Option<EmbedEngine>,
        policies: Vec<IntentPolicy>,
        index_db_path: Option<PathBuf>,
        repo_root: Option<PathBuf>,
    ) -> Self {
        Self::with_index_root_and_audit_config(
            graph,
            embed,
            policies,
            index_db_path,
            repo_root,
            AuditConfig::default(),
        )
    }

    /// Build an `McpServer` plus the audit-pipeline config loaded from
    /// `[audit]` in illuminate.toml.
    ///
    /// Mirrors [`Self::with_index_and_root`] but threads `audit_config` into
    /// the `ToolContext` so `semantic_top_k` and `semantic_threshold` flow
    /// from config rather than from hardcoded constants in `tools.rs`.
    pub fn with_index_root_and_audit_config(
        graph: Graph,
        embed: Option<EmbedEngine>,
        policies: Vec<IntentPolicy>,
        index_db_path: Option<PathBuf>,
        repo_root: Option<PathBuf>,
        audit_config: AuditConfig,
    ) -> Self {
        let ctx =
            ToolContext::with_index_and_root(graph, embed, policies, index_db_path, repo_root)
                .with_audit_config(audit_config);
        Self { ctx: Arc::new(ctx) }
    }

    /// Run the MCP server: read JSON-RPC lines from stdin, write responses to stdout.
    pub async fn run(&self) {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        let mut reader = BufReader::new(stdin);
        let mut stdout = stdout;
        let mut line = String::new();

        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break, // EOF
                Ok(_) => {}
                Err(e) => {
                    eprintln!("illuminate-mcp: read error: {e}");
                    break;
                }
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Parse JSON-RPC request
            let request: Request = match serde_json::from_str(trimmed) {
                Ok(r) => r,
                Err(e) => {
                    let resp = Response::error(
                        Value::Null,
                        codes::PARSE_ERROR,
                        &format!("parse error: {e}"),
                    );
                    Self::write_response(&mut stdout, &resp).await;
                    continue;
                }
            };

            // Notifications have no id — do not send a response
            if request.is_notification() {
                eprintln!("illuminate-mcp: notification: {}", request.method);
                continue;
            }

            let id = request.id.clone().unwrap_or(Value::Null);
            let response = self.dispatch(id.clone(), &request).await;
            Self::write_response(&mut stdout, &response).await;
        }
    }

    async fn dispatch(&self, id: Value, request: &Request) -> Response {
        match request.method.as_str() {
            "initialize" => {
                let result = json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {"tools": {}, "resources": {}},
                    "serverInfo": {"name": "illuminate", "version": "0.3.0"}
                });
                Response::ok(id, result)
            }

            "tools/list" => Response::ok(id, tools_list()),

            "resources/list" => {
                let repo_root = self
                    .ctx
                    .repo_root()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
                let resources = list_wiki_resources(&repo_root);
                Response::ok(id, json!({ "resources": resources }))
            }

            "resources/read" => {
                let params = request.params.clone().unwrap_or(Value::Null);
                let uri = match params["uri"].as_str() {
                    Some(u) => u.to_string(),
                    None => {
                        return Response::error(id, codes::INVALID_PARAMS, "missing uri");
                    }
                };
                let repo_root = self
                    .ctx
                    .repo_root()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
                match read_wiki_resource(&uri, &repo_root) {
                    Ok(result) => Response::ok(id, result),
                    Err(e) => Response::error(id, codes::INVALID_PARAMS, &e),
                }
            }

            "tools/call" => {
                let params = request.params.clone().unwrap_or(Value::Null);
                let tool_name = match params["name"].as_str() {
                    Some(n) => n.to_string(),
                    None => {
                        return Response::error(id, codes::INVALID_PARAMS, "missing tool name");
                    }
                };
                let args = params["arguments"].clone();

                let ctx = Arc::clone(&self.ctx);
                let result = match tool_name.as_str() {
                    "add_episode" => ctx.add_episode(args).await,
                    "search" => ctx.search(args).await,
                    "get_decision" => ctx.get_decision(args).await,
                    "traverse" => ctx.traverse(args).await,
                    "traverse_batch" => ctx.traverse_batch(args).await,
                    "find_precedents" => ctx.find_precedents(args).await,
                    "list_entities" => ctx.list_entities(args).await,
                    "export_graph" => ctx.export_graph(args).await,
                    "illuminate_audit" => ctx.illuminate_audit(args).await,
                    "illuminate_reflect" => ctx.illuminate_reflect(args).await,
                    "illuminate_route" => ctx.illuminate_route(args).await,
                    "illuminate_stats" => ctx.illuminate_stats(args).await,
                    "illuminate_impact" => ctx.illuminate_impact(args).await,
                    "illuminate_explain" => ctx.illuminate_explain(args).await,
                    "illuminate_symbols" => ctx.illuminate_symbols(args).await,
                    "illuminate_decisions_for" => ctx.illuminate_decisions_for(args).await,
                    "illuminate_failures_for" => ctx.illuminate_failures_for(args).await,
                    "illuminate_get_wiki_page" => ctx.illuminate_get_wiki_page(args).await,
                    other => Err(format!("unknown tool: {other}")),
                };

                Response::ok(id, tool_result(result))
            }

            "notifications/initialized" => {
                // Notification — should not reach here (filtered above), but handle gracefully
                Response::ok(id, Value::Null)
            }

            other => {
                eprintln!("illuminate-mcp: unknown method: {other}");
                Response::error(
                    id,
                    codes::METHOD_NOT_FOUND,
                    &format!("method not found: {other}"),
                )
            }
        }
    }

    async fn write_response(stdout: &mut tokio::io::Stdout, resp: &Response) {
        let mut line = match serde_json::to_string(resp) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("illuminate-mcp: failed to serialize response: {e}");
                return;
            }
        };
        line.push('\n');
        if let Err(e) = stdout.write_all(line.as_bytes()).await {
            eprintln!("illuminate-mcp: write error: {e}");
        }
        let _ = stdout.flush().await;
    }
}
