//! `illuminate browse` — list and read published sessions from a team repo.
//!
//! Reads `<team-repo>/sessions/*.md` (the path `illuminate-publish` writes to),
//! parses the YAML front-matter, and renders a sorted table (default) or the
//! full body of a single page (`browse show <id>`).
//!
//! Closes a v3.0 GA gap — the publish gesture exists in v0.21, but until now
//! there was no first-class way to read what was published back.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use super::open_graph;

#[derive(Debug, Clone)]
struct SessionPage {
    file_name: String,
    id: Option<String>,
    title: Option<String>,
    page_type: Option<String>,
    session_id: Option<String>,
    agent: Option<String>,
    model: Option<String>,
    redaction: Option<String>,
    commit_sha: Option<String>,
    created: Option<String>,
    body: String,
}

pub fn run(
    team_repo: Option<PathBuf>,
    show_id: Option<String>,
    limit: usize,
    json_output: bool,
) -> illuminate::Result<()> {
    let repo = resolve_team_repo(team_repo)?;
    let sessions_dir = repo.join("sessions");
    if !sessions_dir.exists() {
        return Err(illuminate::IlluminateError::InvalidInput(format!(
            "no sessions directory at {} — has anything been published yet?",
            sessions_dir.display()
        )));
    }

    let mut pages = read_pages(&sessions_dir)?;
    // Sort newest-first by `created` (RFC 3339-ish). Fallback: filename.
    pages.sort_by(|a, b| {
        b.created
            .as_deref()
            .cmp(&a.created.as_deref())
            .then(b.file_name.cmp(&a.file_name))
    });

    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    if let Some(query) = show_id {
        // Show a single page in full.
        let Some(page) = find_page(&pages, &query) else {
            return Err(illuminate::IlluminateError::NotFound(format!(
                "no session matching '{query}' in {}",
                sessions_dir.display()
            )));
        };
        render_show(&mut out, page, json_output).map_err(illuminate::IlluminateError::Io)?;
    } else {
        // List view.
        if json_output {
            let summaries: Vec<serde_json::Value> = pages
                .iter()
                .take(limit)
                .map(|p| {
                    serde_json::json!({
                        "id": p.id,
                        "title": p.title,
                        "page_type": p.page_type,
                        "session_id": p.session_id,
                        "agent": p.agent,
                        "model": p.model,
                        "redaction": p.redaction,
                        "commit_sha": p.commit_sha,
                        "created": p.created,
                        "file": p.file_name,
                    })
                })
                .collect();
            writeln!(out, "{}", serde_json::to_string_pretty(&summaries).unwrap())
                .map_err(illuminate::IlluminateError::Io)?;
        } else {
            render_list(&mut out, &pages, limit, &sessions_dir)
                .map_err(illuminate::IlluminateError::Io)?;
        }
    }
    Ok(())
}

fn resolve_team_repo(explicit: Option<PathBuf>) -> illuminate::Result<PathBuf> {
    if let Some(p) = explicit {
        if !p.exists() {
            return Err(illuminate::IlluminateError::InvalidInput(format!(
                "--team-repo {} does not exist",
                p.display()
            )));
        }
        return Ok(p);
    }
    // Fallback 1: sibling `team-illuminate` directory next to the repo root.
    let _graph = open_graph(); // touches illuminate.toml ancestor walk
    if let Some(parent) = std::env::current_dir()
        .ok()
        .and_then(|c| c.parent().map(Path::to_path_buf))
    {
        let candidate = parent.join("team-illuminate");
        if candidate.join("sessions").exists() {
            return Ok(candidate);
        }
    }
    // Fallback 2: `<cwd>/team-illuminate`.
    let cwd_candidate = std::env::current_dir()
        .map_err(illuminate::IlluminateError::Io)?
        .join("team-illuminate");
    if cwd_candidate.join("sessions").exists() {
        return Ok(cwd_candidate);
    }
    Err(illuminate::IlluminateError::InvalidInput(
        "no --team-repo given and no team-illuminate/ directory found nearby".to_string(),
    ))
}

fn read_pages(dir: &Path) -> illuminate::Result<Vec<SessionPage>> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).map_err(illuminate::IlluminateError::Io)? {
        let entry = entry.map_err(illuminate::IlluminateError::Io)?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase() != "md")
            .unwrap_or(true)
        {
            continue;
        }
        let raw = fs::read_to_string(&path).map_err(illuminate::IlluminateError::Io)?;
        out.push(parse_page(&path, &raw));
    }
    Ok(out)
}

fn parse_page(path: &Path, raw: &str) -> SessionPage {
    let file_name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let (front, body) = if let Some(rest) = raw.strip_prefix("---\n") {
        if let Some((fm, b)) = rest.split_once("\n---\n") {
            (fm.to_string(), b.to_string())
        } else {
            (String::new(), raw.to_string())
        }
    } else {
        (String::new(), raw.to_string())
    };

    let mut page = SessionPage {
        file_name,
        id: None,
        title: None,
        page_type: None,
        session_id: None,
        agent: None,
        model: None,
        redaction: None,
        commit_sha: None,
        created: None,
        body,
    };

    for line in front.lines() {
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        let key = k.trim();
        let val = v.trim().trim_matches('"').trim().to_string();
        if val.is_empty() {
            continue;
        }
        match key {
            "id" => page.id = Some(val),
            "title" => page.title = Some(val),
            "page_type" => page.page_type = Some(val),
            "session_id" => page.session_id = Some(val),
            "agent" => page.agent = Some(val),
            "model" => page.model = Some(val),
            "redaction" => page.redaction = Some(val),
            "commit_sha" => page.commit_sha = Some(val),
            "created" => page.created = Some(val),
            _ => {}
        }
    }
    // Derive a fallback title from the first H1 in the body if front-matter
    // lacked one. Matches what the rest of the CLI does for wiki pages.
    if page.title.is_none() {
        for line in page.body.lines() {
            let l = line.trim();
            if let Some(rest) = l.strip_prefix("# ") {
                page.title = Some(rest.trim().to_string());
                break;
            }
        }
    }
    page
}

fn find_page<'a>(pages: &'a [SessionPage], query: &str) -> Option<&'a SessionPage> {
    pages.iter().find(|p| {
        p.id.as_deref() == Some(query)
            || p.session_id.as_deref() == Some(query)
            || p.file_name == query
            || p.file_name.trim_end_matches(".md") == query
    })
}

fn render_list<W: Write>(
    out: &mut W,
    pages: &[SessionPage],
    limit: usize,
    sessions_dir: &Path,
) -> std::io::Result<()> {
    writeln!(out, "─── illuminate browse — published sessions ───")?;
    writeln!(
        out,
        "  team repo: {}\n",
        sessions_dir.parent().unwrap_or(sessions_dir).display()
    )?;
    if pages.is_empty() {
        writeln!(
            out,
            "  (no sessions in {} — try `illuminate publish` first)",
            sessions_dir.display()
        )?;
        return Ok(());
    }
    writeln!(
        out,
        "  {:<12}  {:<14}  {:<10}  TITLE / ID",
        "DATE", "AGENT", "REDACTION"
    )?;
    writeln!(out, "  {}", "─".repeat(72))?;
    for p in pages.iter().take(limit) {
        let date = p
            .created
            .as_deref()
            .map(|c| c.split('T').next().unwrap_or(c).to_string())
            .unwrap_or_else(|| "?".to_string());
        let agent = p.agent.clone().unwrap_or_else(|| "?".to_string());
        let red = p.redaction.clone().unwrap_or_else(|| "?".to_string());
        let title = p
            .title
            .clone()
            .or_else(|| p.id.clone())
            .unwrap_or_else(|| p.file_name.clone());
        writeln!(out, "  {date:<12}  {agent:<14}  {red:<10}  {title}")?;
        if let Some(id) = &p.id {
            writeln!(out, "  {:<38}  id: {id}", "")?;
        }
    }
    if pages.len() > limit {
        writeln!(
            out,
            "\n  … {} more (use --limit to show more, or `illuminate browse show <id>` to read one)",
            pages.len() - limit
        )?;
    }
    Ok(())
}

fn render_show<W: Write>(
    out: &mut W,
    page: &SessionPage,
    json_output: bool,
) -> std::io::Result<()> {
    if json_output {
        let v = serde_json::json!({
            "id": page.id,
            "title": page.title,
            "page_type": page.page_type,
            "session_id": page.session_id,
            "agent": page.agent,
            "model": page.model,
            "redaction": page.redaction,
            "commit_sha": page.commit_sha,
            "created": page.created,
            "file": page.file_name,
            "body": page.body,
        });
        writeln!(out, "{}", serde_json::to_string_pretty(&v).unwrap())?;
        return Ok(());
    }
    writeln!(
        out,
        "─── {} ───",
        page.title.as_deref().unwrap_or("(untitled)")
    )?;
    if let Some(id) = &page.id {
        writeln!(out, "  id:         {id}")?;
    }
    if let Some(sid) = &page.session_id {
        writeln!(out, "  session_id: {sid}")?;
    }
    if let Some(c) = &page.created {
        writeln!(out, "  created:    {c}")?;
    }
    if let Some(a) = &page.agent {
        writeln!(out, "  agent:      {a}")?;
    }
    if let Some(m) = &page.model {
        writeln!(out, "  model:      {m}")?;
    }
    if let Some(r) = &page.redaction {
        writeln!(out, "  redaction:  {r}")?;
    }
    if let Some(c) = &page.commit_sha {
        writeln!(out, "  commit_sha: {c}")?;
    }
    writeln!(out, "  file:       {}", page.file_name)?;
    writeln!(out)?;
    writeln!(out, "{}", page.body.trim_end())?;
    Ok(())
}
