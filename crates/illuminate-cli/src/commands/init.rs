use std::env;
use std::fs;
use std::path::Path;

use illuminate::Graph;

pub fn run(name: Option<String>, claude: bool, cursor: bool, windsurf: bool, hooks: bool) -> illuminate::Result<()> {
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

    if hooks {
        configure_hooks(&dir)?;
    }

    if claude || cursor || windsurf || hooks {
        println!();
        println!("Agent configuration:");
        if claude { println!("  Claude Code: .claude.json + CLAUDE.md"); }
        if cursor { println!("  Cursor: .cursor/mcp.json"); }
        if windsurf { println!("  Windsurf: .windsurf/mcp.json"); }
        if hooks { println!("  Hooks: .claude/settings.json (auto-audit on Write/Edit)"); }
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

    // generate CLAUDE.md with illuminate instructions
    let claude_md_path = dir.join("CLAUDE.md");
    if !claude_md_path.exists() {
        fs::write(&claude_md_path, claude_md_content())
            .map_err(illuminate::IlluminateError::Io)?;
    }

    Ok(())
}

fn claude_md_content() -> &'static str {
    r#"# illuminate integration

This project uses [illuminate](https://github.com/rohansx/illuminate) for architectural decision tracking.

## before writing code

Call `illuminate_audit` with your proposed plan to check for architectural conflicts:

```
illuminate_audit({ "plan": "description of what you intend to do" })
```

If the audit returns violations, adjust your plan before proceeding.

## after a failure

If something breaks or doesn't work as expected, record it:

```
illuminate_reflect({
  "failure": "what went wrong",
  "root_cause": "why it happened",
  "corrective_action": "what to do instead"
})
```

## exploring the codebase

- `illuminate_search` - find decisions about a topic
- `illuminate_explain` - understand why a file was built this way
- `illuminate_route` - get a reading plan for a subject
- `illuminate_symbols` - look up code symbols and linked decisions
- `illuminate_impact` - see what code depends on a decision

## intent policies

Check `illuminate.toml` for active policies. These are machine-enforced architectural rules.
"#
}

fn configure_hooks(dir: &Path) -> illuminate::Result<()> {
    let claude_dir = dir.join(".claude");
    fs::create_dir_all(&claude_dir).map_err(illuminate::IlluminateError::Io)?;

    let settings_path = claude_dir.join("settings.json");

    let config = if settings_path.exists() {
        let content = fs::read_to_string(&settings_path).map_err(illuminate::IlluminateError::Io)?;
        let mut value: serde_json::Value = serde_json::from_str(&content)
            .unwrap_or_else(|_| serde_json::json!({}));
        // add illuminate hook
        let hooks = value
            .as_object_mut()
            .unwrap()
            .entry("hooks")
            .or_insert_with(|| serde_json::json!({}));
        let pre_hooks = hooks
            .as_object_mut()
            .unwrap()
            .entry("PreToolUse")
            .or_insert_with(|| serde_json::json!([]));
        if let Some(arr) = pre_hooks.as_array_mut() {
            let already = arr.iter().any(|h| {
                h.get("command")
                    .and_then(|c| c.as_str())
                    .is_some_and(|c| c.contains("illuminate"))
            });
            if !already {
                arr.push(serde_json::json!({
                    "matcher": "Write|Edit|MultiEdit",
                    "command": "illuminate audit-hook --stdin"
                }));
            }
        }
        value
    } else {
        serde_json::json!({
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Write|Edit|MultiEdit",
                        "command": "illuminate audit-hook --stdin"
                    }
                ]
            }
        })
    };

    let json_str = serde_json::to_string_pretty(&config)
        .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;
    fs::write(&settings_path, json_str).map_err(illuminate::IlluminateError::Io)?;
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
