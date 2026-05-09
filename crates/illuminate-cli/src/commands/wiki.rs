//! `illuminate wiki` — manage the markdown wiki.

use clap::Subcommand;
use illuminate_wiki::episode::page_to_episode_parts;
use illuminate_wiki::lint::lint_page;
use illuminate_wiki::page::PageType;
use illuminate_wiki::render::render_index;
use illuminate_wiki::scaffold::write_scaffold;
use illuminate_wiki::walk::walk_wiki;
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub enum WikiCmd {
    /// Validate every wiki page against the schema
    Lint,
    /// Walk wiki/, register each page as a graph episode, regenerate index.md
    Rebuild,
    /// Print pages by type (id and title)
    List,
    /// Initialize an empty wiki/ scaffold in the current repo
    Init,
    /// Serve the wiki as HTML at http://127.0.0.1:<port>
    Serve {
        /// Port to bind (default 8765)
        #[arg(long, default_value = "8765")]
        port: u16,
    },
    /// Search the wiki by keyword (grep + FTS5)
    Search {
        /// Search query
        query: String,
        /// Max results
        #[arg(long, default_value = "10")]
        limit: usize,
    },
    /// Walk the review queue and accept/reject candidate pages
    Review {
        /// Non-interactive mode: print candidates with details and exit (no prompts)
        #[arg(long)]
        list: bool,
    },
    /// Bulk-redact a regex pattern across wiki files
    Redact {
        /// Regex pattern to match (Rust `regex` crate syntax)
        pattern: String,
        /// Show what would change without modifying files
        #[arg(long)]
        dry_run: bool,
    },
}

pub fn run(cmd: WikiCmd) -> std::io::Result<()> {
    match cmd {
        WikiCmd::Lint => cmd_lint(),
        WikiCmd::Rebuild => cmd_rebuild(),
        WikiCmd::List => cmd_list(),
        WikiCmd::Init => cmd_init(),
        WikiCmd::Serve { port } => cmd_serve(port),
        WikiCmd::Search { query, limit } => cmd_search(&query, limit),
        WikiCmd::Review { list } => cmd_review(list),
        WikiCmd::Redact { pattern, dry_run } => cmd_redact(&pattern, dry_run),
    }
}

fn repo_root() -> std::io::Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    let mut cur = Some(cwd.as_path());
    while let Some(d) = cur {
        if d.join(".illuminate").join("illuminate.toml").is_file() {
            return Ok(d.to_path_buf());
        }
        cur = d.parent();
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "no .illuminate/illuminate.toml found in cwd or ancestors — run `illuminate init`",
    ))
}

fn wiki_dir() -> std::io::Result<PathBuf> {
    Ok(repo_root()?.join(".illuminate").join("wiki"))
}

fn cmd_init() -> std::io::Result<()> {
    let root = repo_root()?;
    write_scaffold(&root).map_err(|e| std::io::Error::other(e.to_string()))?;
    println!("scaffolded {}", root.join(".illuminate/wiki").display());
    Ok(())
}

fn cmd_lint() -> std::io::Result<()> {
    let dir = wiki_dir()?;
    let walked = walk_wiki(&dir).map_err(|e| std::io::Error::other(e.to_string()))?;
    let mut errors = 0;
    for w in &walked {
        let path = w.path.display();
        match &w.page {
            Err(e) => {
                eprintln!("{path}: parse error: {e}");
                errors += 1;
            }
            Ok(page) => {
                let lints = lint_page(page);
                for l in &lints {
                    eprintln!("{path}: {:?}: {}", l.code, l.message);
                    errors += 1;
                }
            }
        }
    }
    if errors == 0 {
        println!("lint: ok ({} pages)", walked.len());
        Ok(())
    } else {
        Err(std::io::Error::other(format!("{errors} lint error(s)")))
    }
}

fn cmd_list() -> std::io::Result<()> {
    let dir = wiki_dir()?;
    let walked = walk_wiki(&dir).map_err(|e| std::io::Error::other(e.to_string()))?;
    let pages: Vec<_> = walked.into_iter().filter_map(|w| w.page.ok()).collect();
    if pages.is_empty() {
        println!("(no wiki pages yet)");
        return Ok(());
    }
    for (label, kind) in &[
        ("decisions", PageType::Decision),
        ("patterns", PageType::Pattern),
        ("failures", PageType::Failure),
        ("modules", PageType::Module),
    ] {
        let subset: Vec<_> = pages
            .iter()
            .filter(|p| p.front.page_type == *kind)
            .collect();
        if subset.is_empty() {
            continue;
        }
        println!("[{label}]");
        for p in subset {
            println!("  {} — {} ({})", p.front.id, p.front.title, p.front.status);
        }
    }
    Ok(())
}

pub(crate) fn cmd_rebuild() -> std::io::Result<()> {
    let root = repo_root()?;
    let dir = root.join(".illuminate").join("wiki");
    let walked = walk_wiki(&dir).map_err(|e| std::io::Error::other(e.to_string()))?;
    let pages: Vec<_> = walked.into_iter().filter_map(|w| w.page.ok()).collect();

    // 1. Regenerate index.md
    let index = render_index(&pages);
    let index_path = dir.join("index.md");
    std::fs::write(&index_path, index)?;

    // 2. Register each page as an episode in the graph (best-effort).
    //    If the graph hasn't been initialized yet, skip with a warning.
    let registered = match register_pages(&root, &pages) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("warning: graph not updated: {e}");
            0
        }
    };

    println!(
        "rebuilt index.md ({} pages); registered {} episodes",
        pages.len(),
        registered
    );
    Ok(())
}

fn cmd_serve(port: u16) -> std::io::Result<()> {
    let dir = wiki_dir()?;
    illuminate_wiki::serve::serve(&dir, port)
}

fn cmd_search(query: &str, limit: usize) -> std::io::Result<()> {
    let dir = wiki_dir()?;
    let walked =
        illuminate_wiki::walk::walk_wiki(&dir).map_err(|e| std::io::Error::other(e.to_string()))?;
    let pages: Vec<illuminate_wiki::page::WikiPage> =
        walked.into_iter().filter_map(|w| w.page.ok()).collect();

    // 1. wiki grep
    let lower_q = query.to_lowercase();
    let mut scored: Vec<(f32, &illuminate_wiki::page::WikiPage)> = pages
        .iter()
        .filter_map(|p| {
            let title_hits = p.front.title.to_lowercase().matches(&lower_q[..]).count() as f32;
            let body_hits = p.body.to_lowercase().matches(&lower_q[..]).count() as f32;
            let score = title_hits * 3.0 + body_hits;
            if score > 0.0 { Some((score, p)) } else { None }
        })
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    println!("== wiki grep ({} matches) ==", scored.len());
    for (score, page) in scored.iter().take(limit) {
        let snippet = first_match_snippet(&page.body, &lower_q, 100);
        println!(
            "  [{}] {} ({:.0})\n    {}",
            page.front.id, page.front.title, score, snippet
        );
    }

    // 2. graph FTS5
    println!();
    println!("== graph FTS5 ==");
    let repo = repo_root().ok();
    if let Some(root) = repo {
        let db = root.join(".illuminate").join("graph.db");
        if db.is_file() {
            match illuminate::Graph::open(&db) {
                Ok(graph) => match graph.search(query, limit) {
                    Ok(results) => {
                        if results.is_empty() {
                            println!("  (no matches)");
                        } else {
                            for (episode, _score) in results.iter().take(limit) {
                                let snippet: String = episode.content.chars().take(120).collect();
                                println!(
                                    "  [{}] {} — {}",
                                    episode.id,
                                    episode.source.as_deref().unwrap_or("?"),
                                    snippet
                                );
                            }
                        }
                    }
                    Err(e) => println!("  search error: {e}"),
                },
                Err(e) => println!("  graph open error: {e}"),
            }
        } else {
            println!("  (no graph.db; run `illuminate wiki rebuild` to populate)");
        }
    } else {
        println!("  (no .illuminate found)");
    }

    Ok(())
}

fn first_match_snippet(text: &str, query: &str, window: usize) -> String {
    let lower = text.to_lowercase();
    if let Some(pos) = lower.find(query) {
        let start = pos.saturating_sub(window / 2);
        let end = (pos + query.len() + window / 2).min(text.len());
        // Clamp to char boundaries
        let mut s = start;
        while !text.is_char_boundary(s) && s < text.len() {
            s += 1;
        }
        let mut e = end;
        while !text.is_char_boundary(e) && e > s {
            e -= 1;
        }
        let prefix = if s > 0 { "..." } else { "" };
        let suffix = if e < text.len() { "..." } else { "" };
        format!("{prefix}{}{suffix}", text[s..e].replace('\n', " "))
    } else {
        String::new()
    }
}

fn cmd_review(list_only: bool) -> std::io::Result<()> {
    use illuminate_wiki::page::{PageType, parse_page};

    let root = repo_root()?;
    let review_dir = root.join(".illuminate/wiki/_review");
    if !review_dir.is_dir() {
        println!("(no review queue at {})", review_dir.display());
        return Ok(());
    }

    let mut entries: Vec<_> = std::fs::read_dir(&review_dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("md"))
        .collect();
    entries.sort();

    if entries.is_empty() {
        println!("(review queue empty)");
        return Ok(());
    }

    if list_only {
        for path in &entries {
            let content = std::fs::read_to_string(path).unwrap_or_default();
            match parse_page(&content) {
                Ok(p) => println!(
                    "{}  {}  conf={}  type={:?}",
                    p.front.id,
                    p.front.title,
                    p.front
                        .confidence
                        .map(|c| format!("{:.2}", c))
                        .unwrap_or_else(|| "?".into()),
                    p.front.page_type,
                ),
                Err(_) => println!("{}  (unparseable)", path.display()),
            }
        }
        return Ok(());
    }

    println!(
        "review queue: {} candidates in {}",
        entries.len(),
        review_dir.display()
    );
    println!();

    let mut idx = 0;
    while idx < entries.len() {
        let path = entries[idx].clone();
        let content = std::fs::read_to_string(&path)?;
        let page = match parse_page(&content) {
            Ok(p) => p,
            Err(e) => {
                println!(
                    "[{}/{}] {} — UNPARSEABLE: {e}",
                    idx + 1,
                    entries.len(),
                    path.display()
                );
                println!("  Choose: [r]eject  [s]kip  [q]uit");
                match prompt_char()? {
                    'r' => {
                        let _ = std::fs::remove_file(&path);
                        idx += 1;
                    }
                    'q' => return Ok(()),
                    _ => idx += 1,
                }
                continue;
            }
        };

        println!("─── [{}/{}] {} ───", idx + 1, entries.len(), page.front.id);
        println!("title:      {}", page.front.title);
        println!("type:       {:?}", page.front.page_type);
        println!("status:     {}", page.front.status);
        println!(
            "confidence: {}",
            page.front
                .confidence
                .map(|c| format!("{:.2}", c))
                .unwrap_or_else(|| "?".into())
        );
        if let Some(s) = page.front.sources.first() {
            println!("source:     {} ({})", s.r#ref, s.kind);
        }
        println!();
        for line in page.body.lines().take(30) {
            println!("  {line}");
        }
        if page.body.lines().count() > 30 {
            println!("  ... ({} more lines)", page.body.lines().count() - 30);
        }
        println!();

        print!("[a]ccept  [r]eject  [e]dit  [s]kip  [q]uit > ");
        std::io::Write::flush(&mut std::io::stdout())?;
        match prompt_char()? {
            'a' => {
                let dest_dir = match page.front.page_type {
                    PageType::Decision => root.join(".illuminate/wiki/decisions"),
                    PageType::Pattern => root.join(".illuminate/wiki/patterns"),
                    PageType::Failure => root.join(".illuminate/wiki/failures"),
                    PageType::Module => root.join(".illuminate/wiki/modules"),
                };
                std::fs::create_dir_all(&dest_dir)?;
                let dest = dest_dir.join(path.file_name().unwrap());
                std::fs::rename(&path, &dest)?;
                append_log(&root, &page.front.id, "ACCEPT")?;
                println!("accepted -> {}", dest.display());
                idx += 1;
            }
            'r' => {
                std::fs::remove_file(&path)?;
                append_log(&root, &page.front.id, "REJECT")?;
                println!("rejected and deleted");
                idx += 1;
            }
            'e' => {
                let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".into());
                let status = std::process::Command::new(&editor).arg(&path).status();
                match status {
                    Ok(s) if s.success() => {
                        // re-prompt by NOT incrementing idx
                        continue;
                    }
                    Ok(_) | Err(_) => {
                        eprintln!("editor failed; skipping");
                        idx += 1;
                    }
                }
            }
            's' => {
                println!("skipped (still in queue)");
                idx += 1;
            }
            'q' => return Ok(()),
            other => {
                println!("unknown choice '{other}'; skipping");
                idx += 1;
            }
        }
        println!();
    }

    println!("review complete");
    Ok(())
}

fn prompt_char() -> std::io::Result<char> {
    let mut buf = String::new();
    std::io::stdin().read_line(&mut buf)?;
    Ok(buf
        .trim()
        .chars()
        .next()
        .unwrap_or(' ')
        .to_ascii_lowercase())
}

fn append_log(root: &std::path::Path, id: &str, verb: &str) -> std::io::Result<()> {
    let log_path = root.join(".illuminate/wiki/log.md");
    let entry = format!(
        "{}  {verb}  {id}  (review)\n",
        chrono::Utc::now().to_rfc3339()
    );
    let mut existing = std::fs::read_to_string(&log_path).unwrap_or_default();
    if !existing.is_empty() && !existing.ends_with('\n') {
        existing.push('\n');
    }
    existing.push_str(&entry);
    std::fs::write(&log_path, existing)?;
    Ok(())
}

/// Bulk-redact a regex pattern across all wiki markdown files AND graph
/// episodes.
///
/// `--dry-run` prints `<file>: <N> matches` for each affected page and leaves
/// the filesystem and graph untouched. Without `--dry-run`:
/// 1. each match in a wiki page is replaced with the literal `[REDACTED]`;
/// 2. every graph episode whose `content` matches the regex is deleted via
///    [`illuminate::Graph::delete_episode`], cascading to anchors,
///    episode_entities, edges, and the FTS5 mirror.
///
/// The graph pass enumerates episodes via `list_episodes` with a generous
/// limit and matches client-side — pragmatic, O(n) in episode count, but
/// avoids FTS5 quirks for arbitrary regex patterns. If `graph.db` does not
/// exist yet, that's not an error: graph deletion is reported as zero.
fn cmd_redact(pattern: &str, dry_run: bool) -> std::io::Result<()> {
    let re = regex::Regex::new(pattern).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("invalid regex pattern: {e}"),
        )
    })?;

    let dir = wiki_dir()?;
    if !dir.is_dir() {
        println!("(no wiki/ directory)");
        return Ok(());
    }

    let mut total_matches = 0usize;
    let mut affected_files = 0usize;
    visit_markdown_files(&dir, &mut |path: &std::path::Path| -> std::io::Result<()> {
        let content = std::fs::read_to_string(path)?;
        let count = re.find_iter(&content).count();
        if count == 0 {
            return Ok(());
        }
        affected_files += 1;
        total_matches += count;
        let display = path
            .strip_prefix(&dir)
            .unwrap_or(path)
            .display()
            .to_string();
        if dry_run {
            println!("{display}: {count} matches");
        } else {
            let redacted = re.replace_all(&content, "[REDACTED]").into_owned();
            std::fs::write(path, redacted)?;
            println!("redacted {display}: {count} matches");
        }
        Ok(())
    })?;

    if dry_run {
        println!(
            "dry-run summary: {total_matches} matches across {affected_files} files (no changes written)"
        );
        return Ok(());
    }

    println!("redact summary: {total_matches} matches across {affected_files} files");

    // Graph pass: delete every episode whose content matches the regex.
    let graph_deleted = redact_graph_episodes(&re)?;
    println!("redacted graph: {graph_deleted} episode(s) deleted");
    Ok(())
}

/// Enumerate graph episodes and delete those whose `content` matches `re`.
///
/// Returns the number of episodes successfully deleted. If no `graph.db`
/// exists yet, returns `Ok(0)` — wiki redact runs against the wiki even when
/// the graph hasn't been bootstrapped.
fn redact_graph_episodes(re: &regex::Regex) -> std::io::Result<usize> {
    let mut graph = match super::open_graph() {
        Ok(g) => g,
        Err(illuminate::IlluminateError::NotFound(_)) => return Ok(0),
        Err(e) => return Err(std::io::Error::other(e.to_string())),
    };

    // Generous limit; v0.13 chooses simplicity (list-and-filter) over an FTS5
    // round-trip that wouldn't translate arbitrary regex anyway.
    let episodes = graph
        .list_episodes(10_000, 0)
        .map_err(|e| std::io::Error::other(e.to_string()))?;

    let mut deleted = 0usize;
    for ep in episodes {
        if re.is_match(&ep.content)
            && graph
                .delete_episode(&ep.id)
                .map_err(|e| std::io::Error::other(e.to_string()))?
        {
            deleted += 1;
        }
    }
    Ok(deleted)
}

/// Recursively visit every `*.md` file under `dir`. Used by `cmd_redact` to
/// reach pages in nested type-directories (`decisions/`, `patterns/`, etc.)
/// while skipping hidden / non-markdown entries.
fn visit_markdown_files(
    dir: &std::path::Path,
    visit: &mut dyn FnMut(&std::path::Path) -> std::io::Result<()>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)?.flatten() {
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        if file_type.is_dir() {
            // Skip _review/ — that's a queue, not durable wiki content.
            if path.file_name().and_then(|n| n.to_str()) == Some("_review") {
                continue;
            }
            visit_markdown_files(&path, visit)?;
        } else if file_type.is_file() && path.extension().and_then(|e| e.to_str()) == Some("md") {
            visit(&path)?;
        }
    }
    Ok(())
}

fn register_pages(
    repo_root: &Path,
    pages: &[illuminate_wiki::page::WikiPage],
) -> std::io::Result<usize> {
    let db_path = repo_root.join(".illuminate").join("graph.db");
    let graph = illuminate::Graph::open_or_create(&db_path)
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    let mut count = 0;
    for page in pages {
        let (content, metadata) = page_to_episode_parts(page);
        let episode = illuminate::Episode::builder(&content)
            .source("wiki")
            .meta("wiki_metadata", metadata)
            .build();
        graph
            .add_episode(episode)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        count += 1;
    }
    Ok(count)
}
