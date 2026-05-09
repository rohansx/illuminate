//! `illuminate bootstrap` — populate the wiki from existing repo signals.

use std::path::PathBuf;

pub fn run(no_rebuild: bool) -> std::io::Result<()> {
    let cwd = std::env::current_dir()?;
    let root = find_repo_root(&cwd)?;
    let report = illuminate_bootstrap::orchestrate::run_bootstrap(&root)
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    println!("bootstrap complete:");
    println!("  sources run:        {:?}", report.sources_run);
    println!("  candidates found:   {}", report.candidates_found);
    println!("  pages written:      {}", report.pages_written);
    println!("  pages skipped:      {}", report.pages_skipped_existing);
    println!(
        "  pages queued for review: {}",
        report.pages_queued_for_review
    );

    if !no_rebuild && report.pages_written > 0 {
        println!();
        println!("rebuilding wiki index + graph...");
        super::wiki::cmd_rebuild()?;
    }

    Ok(())
}

fn find_repo_root(cwd: &std::path::Path) -> std::io::Result<PathBuf> {
    let mut cur = Some(cwd);
    while let Some(d) = cur {
        if d.join(".illuminate").join("illuminate.toml").is_file() {
            return Ok(d.to_path_buf());
        }
        cur = d.parent();
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "no .illuminate/illuminate.toml found in cwd or ancestors — run `illuminate init` first",
    ))
}
