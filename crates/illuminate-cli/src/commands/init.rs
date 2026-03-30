use std::env;
use std::fs;
use std::path::Path;

use illuminate::Graph;

pub fn run(name: Option<String>, claude: bool, cursor: bool, windsurf: bool) -> illuminate::Result<()> {
    let dir = env::current_dir().map_err(illuminate::IlluminateError::Io)?;
    let _graph = Graph::init(&dir)?;

    let project_name = name.unwrap_or_else(|| {
        dir.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unnamed")
            .to_string()
    });

    // Create default illuminate.toml if not present
    let toml_path = dir.join("illuminate.toml");
    if !toml_path.exists() {
        fs::write(&toml_path, default_config(&project_name))
            .map_err(illuminate::IlluminateError::Io)?;
        println!("  Created: illuminate.toml");
    }

    // Configure agent integrations
    if claude {
        configure_claude(&dir)?;
    }
    if cursor {
        configure_cursor(&dir)?;
    }
    if windsurf {
        configure_windsurf(&dir)?;
    }

    println!("Initialized illuminate for '{project_name}'");
    println!("  Database: .illuminate/graph.db");
    println!();
    println!("Get started:");
    println!("  illuminate models download              Download ONNX models");
    println!("  illuminate watch --git --backfill 100   Ingest git history");
    println!("  illuminate serve                        Start MCP server");

    if claude || cursor || windsurf {
        println!();
        println!("Agent configuration:");
        if claude { println!("  Claude Code: .claude.json updated"); }
        if cursor { println!("  Cursor: .cursor/mcp.json updated"); }
        if windsurf { println!("  Windsurf: .windsurf/mcp.json updated"); }
    }

    Ok(())
}

fn default_config(project_name: &str) -> String {
    format!(r#"# illuminate.toml — configuration for {project_name}

[project]
name = "{project_name}"

[extraction]
confidence_threshold = 0.5

[watch]
git = true
git_backfill = 100

# Intent policies — uncomment and customize for your project
#
# [policies.example_caching]
# rule = "must_use"
# entity = "Memcached"
# reject = ["Redis"]
# reason = "VPC overhead — see ADR #42"
# severity = "error"
#
# [policies.example_frozen]
# rule = "frozen"
# paths = ["src/auth/**"]
# reason = "Security audit in progress"
# severity = "error"
# expires = "2026-06-01"
"#)
}

fn configure_claude(dir: &Path) -> illuminate::Result<()> {
    let config_path = dir.join(".claude.json");

    let config = if config_path.exists() {
        let content = fs::read_to_string(&config_path).map_err(illuminate::IlluminateError::Io)?;
        let mut value: serde_json::Value = serde_json::from_str(&content)
            .unwrap_or_else(|_| serde_json::json!({}));
        // Add illuminate MCP server
        value["mcpServers"]["illuminate"] = serde_json::json!({
            "command": "illuminate",
            "args": ["serve"]
        });
        value
    } else {
        serde_json::json!({
            "mcpServers": {
                "illuminate": {
                    "command": "illuminate",
                    "args": ["serve"]
                }
            }
        })
    };

    let json_str = serde_json::to_string_pretty(&config)
        .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;
    fs::write(&config_path, json_str).map_err(illuminate::IlluminateError::Io)?;
    Ok(())
}

fn configure_cursor(dir: &Path) -> illuminate::Result<()> {
    let cursor_dir = dir.join(".cursor");
    fs::create_dir_all(&cursor_dir).map_err(illuminate::IlluminateError::Io)?;

    let config = serde_json::json!({
        "mcpServers": {
            "illuminate": {
                "command": "illuminate",
                "args": ["serve"]
            }
        }
    });

    let json_str = serde_json::to_string_pretty(&config)
        .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;
    fs::write(cursor_dir.join("mcp.json"), json_str).map_err(illuminate::IlluminateError::Io)?;
    Ok(())
}

fn configure_windsurf(dir: &Path) -> illuminate::Result<()> {
    let ws_dir = dir.join(".windsurf");
    fs::create_dir_all(&ws_dir).map_err(illuminate::IlluminateError::Io)?;

    let config = serde_json::json!({
        "mcpServers": {
            "illuminate": {
                "command": "illuminate",
                "args": ["serve"]
            }
        }
    });

    let json_str = serde_json::to_string_pretty(&config)
        .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;
    fs::write(ws_dir.join("mcp.json"), json_str).map_err(illuminate::IlluminateError::Io)?;
    Ok(())
}
