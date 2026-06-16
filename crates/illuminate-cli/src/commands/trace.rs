//! `illuminate trace <symbol>` — directional process-flow trace via the code
//! graph (`index.db`), read-only.
//!
//! Resolves the symbol to qualified-name seeds (best-effort; the code graph
//! stores edge endpoints as free text), then walks `calls`/`imports`/
//! `inherits`/`references` edges in the requested direction. Mirrors the
//! read-only, `index.db`-resolving shape of `illuminate impact`.

use std::path::PathBuf;

use illuminate_audit::resolve_index_db_from_cwd;
use illuminate_index::edges::{EdgeKind, FlowDir, FlowResult};
use illuminate_index::storage::{resolve_qn_to_symbols, trace_flow};
use rusqlite::Connection;
use serde::Serialize;

/// Default BFS depth for a trace.
const DEFAULT_DEPTH: u32 = 3;

/// Default cap on traversed edges.
const DEFAULT_MAX_STEPS: usize = 200;

/// Cap on the number of steps rendered in human output.
const HUMAN_STEP_LIMIT: usize = 50;

/// Run the `trace` subcommand.
#[allow(clippy::too_many_arguments)]
pub fn run(
    symbol: String,
    index_db: Option<PathBuf>,
    dir: Option<String>,
    kinds: Option<String>,
    depth: Option<u32>,
    max_steps: Option<usize>,
    json: bool,
) -> illuminate::Result<()> {
    let resolved = resolve_index_db_from_cwd(index_db.as_deref());
    let Some(index_path) = resolved else {
        let hint = match index_db {
            Some(p) => format!("no index.db found at {}", p.display()),
            None => "no index.db found in .illuminate/".to_string(),
        };
        eprintln!("{hint}, run `illuminate index` first");
        return Ok(());
    };

    let dir = match dir.as_deref() {
        None => FlowDir::Downstream,
        Some(s) => match FlowDir::from_str(&s.to_lowercase()) {
            Some(d) => d,
            None => {
                eprintln!("invalid --dir '{s}' (expected: downstream, upstream, both)");
                return Ok(());
            }
        },
    };

    let kinds = match parse_kinds(kinds.as_deref()) {
        Ok(k) => k,
        Err(bad) => {
            eprintln!(
                "invalid --kinds entry '{bad}' (expected: calls, imports, inherits, references)"
            );
            return Ok(());
        }
    };

    let depth = depth.unwrap_or(DEFAULT_DEPTH);
    let max_steps = max_steps.unwrap_or(DEFAULT_MAX_STEPS);

    let conn = Connection::open(&index_path)
        .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;

    let seeds = resolve_qn_to_symbols(&conn, &symbol)
        .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;

    if seeds.is_empty() {
        if json {
            let payload = JsonOutput {
                symbol: &symbol,
                dir: dir.as_str(),
                seeds: &[],
                steps: Vec::new(),
                truncated: false,
            };
            let s = serde_json::to_string_pretty(&payload)
                .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;
            println!("{s}");
        } else {
            println!("no symbol matched '{symbol}' in the code graph");
        }
        return Ok(());
    }

    let result = trace_flow(&conn, &seeds, dir, &kinds, depth, max_steps)
        .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;

    if json {
        let payload = JsonOutput {
            symbol: &symbol,
            dir: dir.as_str(),
            seeds: &result.seeds,
            steps: result
                .steps
                .iter()
                .map(|s| JsonStep {
                    from: &s.from,
                    to: &s.to,
                    kind: s.kind.as_str(),
                    depth: s.depth,
                })
                .collect(),
            truncated: result.truncated,
        };
        let s = serde_json::to_string_pretty(&payload)
            .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;
        println!("{s}");
    } else {
        print_human(&symbol, dir, &result, depth);
    }

    Ok(())
}

/// Parse a comma-separated `--kinds` list. `None` / empty defaults to `calls`.
/// Returns the offending token on an unknown kind.
fn parse_kinds(s: Option<&str>) -> Result<Vec<EdgeKind>, String> {
    let Some(raw) = s else {
        return Ok(vec![EdgeKind::Calls]);
    };
    let mut out = Vec::new();
    for part in raw.split(',') {
        let p = part.trim();
        if p.is_empty() {
            continue;
        }
        match EdgeKind::from_str(&p.to_lowercase()) {
            Some(k) => out.push(k),
            None => return Err(p.to_string()),
        }
    }
    if out.is_empty() {
        Ok(vec![EdgeKind::Calls])
    } else {
        Ok(out)
    }
}

#[derive(Serialize)]
struct JsonStep<'a> {
    from: &'a str,
    to: &'a str,
    kind: &'a str,
    depth: u32,
}

#[derive(Serialize)]
struct JsonOutput<'a> {
    symbol: &'a str,
    dir: &'a str,
    seeds: &'a [String],
    steps: Vec<JsonStep<'a>>,
    truncated: bool,
}

fn print_human(symbol: &str, dir: FlowDir, result: &FlowResult, depth: u32) {
    println!("trace {symbol} ({dir}, depth={depth})");
    println!("  seeds: {}", result.seeds.join(", "));
    let n = result.steps.len();
    println!("  {n} edges:");
    if n == 0 {
        println!("    (none)");
    } else {
        for st in result.steps.iter().take(HUMAN_STEP_LIMIT) {
            println!("    [{}] {} --{}--> {}", st.depth, st.from, st.kind, st.to);
        }
        if n > HUMAN_STEP_LIMIT {
            println!("    ... ({} more)", n - HUMAN_STEP_LIMIT);
        }
    }
    if result.truncated {
        println!("    (results truncated at step cap)");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_kinds_defaults_to_calls() {
        assert_eq!(parse_kinds(None).unwrap(), vec![EdgeKind::Calls]);
        assert_eq!(parse_kinds(Some("  ")).unwrap(), vec![EdgeKind::Calls]);
    }

    #[test]
    fn parse_kinds_accepts_all_four() {
        let got = parse_kinds(Some("calls, imports,inherits , references")).unwrap();
        assert_eq!(
            got,
            vec![
                EdgeKind::Calls,
                EdgeKind::Imports,
                EdgeKind::Inherits,
                EdgeKind::References
            ]
        );
    }

    #[test]
    fn parse_kinds_rejects_unknown() {
        assert_eq!(parse_kinds(Some("calls,bogus")).unwrap_err(), "bogus");
    }

    #[test]
    fn dir_parsing_is_case_insensitive() {
        assert_eq!(FlowDir::from_str("both"), Some(FlowDir::Both));
        assert_eq!(
            FlowDir::from_str("UPSTREAM".to_lowercase().as_str()),
            Some(FlowDir::Upstream)
        );
        assert_eq!(FlowDir::from_str("sideways"), None);
    }
}
