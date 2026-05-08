//! `illuminate failure` — record a new failure inline (singular form).
//!
//! Sister to `illuminate failures` (plural) which manages already-existing
//! wiki pages. This module's `log` subcommand creates a brand-new
//! `wiki/failures/<date>-<slug>.md` and registers it as a graph episode in
//! one shot, suitable for non-interactive agent use.

use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum FailureCmd {
    /// Record a new failure (writes wiki page + graph episode).
    ///
    /// Required flags: --title, --root-cause, --fix, --severity. v0.11 has no
    /// interactive editor; if any required flag is missing the command exits
    /// with an error pointing at the missing field. Editor mode is deferred to
    /// v0.12.
    Log {
        /// Short failure title (becomes part of slug + page id).
        #[arg(long)]
        title: Option<String>,

        /// Why it went wrong.
        #[arg(long)]
        root_cause: Option<String>,

        /// What we did to remediate.
        #[arg(long)]
        fix: Option<String>,

        /// Lesson agents should remember next time (optional).
        #[arg(long)]
        lesson: Option<String>,

        /// Comma-separated list of repo-relative file paths that were affected.
        #[arg(long, value_delimiter = ',')]
        files: Vec<PathBuf>,

        /// Comma-separated list of module slugs (e.g. `payments,orders`).
        #[arg(long, value_delimiter = ',')]
        modules: Vec<String>,

        /// Severity: low | medium | high | critical.
        #[arg(long)]
        severity: Option<String>,

        /// URL of the originating incident report (optional).
        #[arg(long)]
        from_incident: Option<String>,

        /// Skip interactive editor (default true in v0.11; editor is v0.12).
        ///
        /// v0.11 always runs non-interactively — this flag exists so the docs
        /// example surface doesn't break and so v0.12 has a place to flip the
        /// default. Setting `--no-editor=false` today errors with a clear
        /// "editor mode is v0.12" hint rather than silently doing nothing.
        #[arg(long, default_value_t = true)]
        no_editor: bool,
    },
}

pub fn run(cmd: FailureCmd) -> illuminate::Result<()> {
    match cmd {
        FailureCmd::Log {
            title,
            root_cause,
            fix,
            lesson,
            files,
            modules,
            severity,
            from_incident,
            no_editor,
        } => cmd_log(
            title,
            root_cause,
            fix,
            lesson,
            files,
            modules,
            severity,
            from_incident,
            no_editor,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn cmd_log(
    title: Option<String>,
    root_cause: Option<String>,
    fix: Option<String>,
    lesson: Option<String>,
    files: Vec<PathBuf>,
    modules: Vec<String>,
    severity: Option<String>,
    from_incident: Option<String>,
    no_editor: bool,
) -> illuminate::Result<()> {
    if !no_editor {
        return Err(illuminate::IlluminateError::InvalidInput(
            "interactive editor mode is v0.12; pass all required flags or run with --no-editor"
                .to_string(),
        ));
    }

    let title = title.ok_or_else(|| {
        illuminate::IlluminateError::InvalidInput(
            "--title is required (interactive editor mode is v0.12)".to_string(),
        )
    })?;
    let root_cause = root_cause.ok_or_else(|| {
        illuminate::IlluminateError::InvalidInput("--root-cause is required".to_string())
    })?;
    let fix = fix.ok_or_else(|| {
        illuminate::IlluminateError::InvalidInput("--fix is required".to_string())
    })?;
    let severity = severity.ok_or_else(|| {
        illuminate::IlluminateError::InvalidInput("--severity is required".to_string())
    })?;

    let severity_lower = severity.to_lowercase();
    if !["low", "medium", "high", "critical"].contains(&severity_lower.as_str()) {
        return Err(illuminate::IlluminateError::InvalidInput(format!(
            "severity must be one of: low, medium, high, critical (got: {severity})"
        )));
    }

    let slug = slugify(&title);
    if slug.is_empty() {
        return Err(illuminate::IlluminateError::InvalidInput(
            "--title must contain at least one alphanumeric character".to_string(),
        ));
    }
    let now = chrono::Utc::now();
    let date = now.format("%Y-%m-%d").to_string();
    let filename = format!("{date}-{slug}.md");
    let id = format!("fail-{slug}");
    let now_rfc = now.to_rfc3339();

    let mut body = String::new();
    body.push_str("---\n");
    body.push_str(&format!("id: {id}\n"));
    body.push_str(&format!("title: {title}\n"));
    body.push_str("page_type: failure\n");
    body.push_str("status: active\n");
    body.push_str("tags: [logged]\n");
    body.push_str(&format!("created: {now_rfc}\n"));
    body.push_str(&format!("updated: {now_rfc}\n"));
    body.push_str("---\n\n");
    body.push_str(&format!("## Root Cause\n\n{root_cause}\n\n"));
    body.push_str(&format!("## Fix\n\n{fix}\n\n"));
    body.push_str(&format!(
        "## Lesson for future agents\n\n{}\n\n",
        lesson.as_deref().unwrap_or("(none provided)")
    ));
    if !files.is_empty() {
        body.push_str("## Affected Files\n\n");
        for f in &files {
            body.push_str(&format!("- {}\n", f.display()));
        }
        body.push('\n');
    }
    if !modules.is_empty() {
        body.push_str("## Affected Modules\n\n");
        for m in &modules {
            body.push_str(&format!("- {m}\n"));
        }
        body.push('\n');
    }
    body.push_str(&format!("## Severity\n\n{severity_lower}\n\n"));
    if let Some(url) = from_incident {
        body.push_str(&format!("<!-- from-incident: {url} -->\n"));
    }

    let repo_root = find_repo_root()?;

    let wiki_dir = repo_root.join(".illuminate").join("wiki").join("failures");
    std::fs::create_dir_all(&wiki_dir)?;
    let wiki_path = wiki_dir.join(&filename);
    if wiki_path.exists() {
        return Err(illuminate::IlluminateError::AlreadyExists(format!(
            "{}",
            wiki_path.display()
        )));
    }
    std::fs::write(&wiki_path, &body)?;
    let display_path = wiki_path
        .strip_prefix(&repo_root)
        .unwrap_or(&wiki_path)
        .display();
    println!("wrote {display_path}");

    let db_path = repo_root.join(".illuminate").join("graph.db");
    let mut graph = illuminate::Graph::open_or_create(&db_path)?;
    super::try_attach_extraction(&mut graph, &db_path);

    let source = format!("failure:{id}");
    let episode = illuminate::Episode::builder(&body).source(&source).build();
    let result = graph.add_episode(episode)?;
    println!("registered as graph episode {}", result.episode_id);

    Ok(())
}

fn find_repo_root() -> illuminate::Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    let mut cur = Some(cwd.as_path());
    while let Some(d) = cur {
        if d.join(".illuminate").join("illuminate.toml").is_file() {
            return Ok(d.to_path_buf());
        }
        cur = d.parent();
    }
    Err(illuminate::IlluminateError::NotFound(
        "no .illuminate/illuminate.toml found in current or parent directories. \
         Run `illuminate init` first."
            .to_string(),
    ))
}

/// Lowercase, replace non-alphanumeric runs with `-`, trim leading/trailing dashes.
fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_dash = true;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::slugify;

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Cache stampede"), "cache-stampede");
    }

    #[test]
    fn slugify_collapses_runs() {
        assert_eq!(slugify("Foo  --  bar!!!"), "foo-bar");
    }

    #[test]
    fn slugify_trims_edges() {
        assert_eq!(slugify("---hello---"), "hello");
    }

    #[test]
    fn slugify_unicode_dropped() {
        // Non-ASCII drops out; we don't try to transliterate.
        assert_eq!(slugify("Über cache"), "ber-cache");
    }
}
