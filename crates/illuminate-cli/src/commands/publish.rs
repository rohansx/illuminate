//! `illuminate publish` — explicit publish gesture for curated trail sessions.
//!
//! Stage 4 of the v3 pipeline. Reads a trail jsonl, redacts per the chosen
//! level, writes a markdown page under `<team_repo>/sessions/`, and registers
//! a graph episode so future `illuminate enrich` calls can surface it.
//!
//! Run with `--install-hook` to write a `.git/hooks/pre-commit` script that
//! invokes this verb on every commit.

use std::path::{Path, PathBuf};

use illuminate_publish::{
    PublishRequest, RedactionLevel, TeamRepoTarget, install_pre_commit_hook, publish,
    write_design_doc,
};

use super::open_graph;

/// Run the `publish` subcommand.
#[allow(clippy::too_many_arguments)]
pub fn run(
    trail: Option<PathBuf>,
    redaction: String,
    team_repo: Option<PathBuf>,
    commit_sha: Option<String>,
    install_hook: bool,
    as_doc: Option<PathBuf>,
    json_output: bool,
) -> illuminate::Result<()> {
    if let Some(target) = as_doc {
        return run_as_doc(trail, target, json_output);
    }

    if install_hook {
        let team = team_repo.ok_or_else(|| {
            illuminate::IlluminateError::InvalidInput(
                "--install-hook requires --team-repo".to_string(),
            )
        })?;
        let repo_root = find_repo_root()?;
        let hook = install_pre_commit_hook(&repo_root, &team).map_err(map_publish_err)?;
        println!("installed pre-commit hook → {}", hook.display());
        return Ok(());
    }

    let trail_path = trail.ok_or_else(|| {
        illuminate::IlluminateError::InvalidInput(
            "--trail is required (path to a .illuminate/trail/*.jsonl file)".to_string(),
        )
    })?;
    let team = team_repo.ok_or_else(|| {
        illuminate::IlluminateError::InvalidInput("--team-repo is required".to_string())
    })?;
    let level = RedactionLevel::parse(&redaction).ok_or_else(|| {
        illuminate::IlluminateError::InvalidInput(format!(
            "unknown --redaction value: {redaction} (expected: full | summary | decision | discard)"
        ))
    })?;

    let req = PublishRequest {
        trail_path,
        redaction: level,
        commit_sha,
        team_repo: TeamRepoTarget::LocalPath(team),
    };

    let mut graph = open_graph()?;
    let resp = publish(&mut graph, &req).map_err(map_publish_err)?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&resp).unwrap());
    } else {
        match resp.redaction {
            RedactionLevel::Discard => {
                println!("session {} discarded (nothing written)", resp.session_id);
            }
            _ => {
                for p in &resp.written_paths {
                    println!("wrote {}", p.display());
                }
                if let Some(ep) = &resp.graph_episode_id {
                    println!("registered graph episode {ep}");
                }
                println!("redaction: {}", resp.redaction.as_str());
            }
        }
    }
    Ok(())
}

/// `--as-doc <path>` path: deterministically draft a design-doc markdown from
/// the trail and write it to the caller-named `target` (template-based, no LLM).
/// Does not touch the graph and does not require `--team-repo`.
fn run_as_doc(
    trail: Option<PathBuf>,
    target: PathBuf,
    json_output: bool,
) -> illuminate::Result<()> {
    let trail_path = trail.ok_or_else(|| {
        illuminate::IlluminateError::InvalidInput(
            "--as-doc requires --trail (path to a .illuminate/trail/*.jsonl file)".to_string(),
        )
    })?;

    let record = illuminate_publish::read_trail_file(&trail_path).map_err(map_publish_err)?;
    let written = write_design_doc(&record, &target).map_err(map_publish_err)?;

    if json_output {
        let payload = serde_json::json!({
            "session_id": record.session_id,
            "as_doc": true,
            "written_path": written.display().to_string(),
        });
        println!("{}", serde_json::to_string_pretty(&payload).unwrap());
    } else {
        println!("wrote design doc → {}", written.display());
    }
    Ok(())
}

fn find_repo_root() -> illuminate::Result<PathBuf> {
    let cwd = std::env::current_dir().map_err(illuminate::IlluminateError::Io)?;
    let mut dir: &Path = cwd.as_path();
    loop {
        if dir.join(".git").exists() {
            return Ok(dir.to_path_buf());
        }
        match dir.parent() {
            Some(p) => dir = p,
            None => {
                return Err(illuminate::IlluminateError::InvalidInput(
                    "not inside a git repository".to_string(),
                ));
            }
        }
    }
}

fn map_publish_err(e: illuminate_publish::PublishError) -> illuminate::IlluminateError {
    match e {
        illuminate_publish::PublishError::Io(io) => illuminate::IlluminateError::Io(io),
        illuminate_publish::PublishError::Graph(g) => g,
        illuminate_publish::PublishError::Parse(s) => {
            illuminate::IlluminateError::InvalidInput(format!("trail parse: {s}"))
        }
    }
}
