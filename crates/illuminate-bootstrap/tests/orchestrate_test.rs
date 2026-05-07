use illuminate_bootstrap::orchestrate::run_bootstrap;
use std::fs;

fn make_opted_in_repo(repo: &std::path::Path) {
    fs::create_dir_all(repo.join(".illuminate")).unwrap();
    fs::write(repo.join(".illuminate/illuminate.toml"), "name='x'\n").unwrap();
    fs::create_dir_all(repo.join(".illuminate/wiki/decisions")).unwrap();
    fs::create_dir_all(repo.join(".illuminate/wiki/patterns")).unwrap();
    fs::create_dir_all(repo.join(".illuminate/wiki/failures")).unwrap();
    fs::create_dir_all(repo.join(".illuminate/wiki/modules")).unwrap();
}

#[test]
fn writes_pages_for_agent_file_decisions() {
    let tmp = tempfile::tempdir().unwrap();
    make_opted_in_repo(tmp.path());
    fs::write(
        tmp.path().join("CLAUDE.md"),
        "## Caching\n\nWe use Memcached. Do not introduce Redis.\n",
    )
    .unwrap();
    let report = run_bootstrap(tmp.path()).unwrap();
    assert!(report.candidates_found >= 1);
    assert!(report.pages_written >= 1);
    let dir = tmp.path().join(".illuminate/wiki/decisions");
    let entries: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("md"))
        .collect();
    assert!(!entries.is_empty());
}

#[test]
fn collapses_same_content_across_agent_files() {
    let tmp = tempfile::tempdir().unwrap();
    make_opted_in_repo(tmp.path());
    let body = "## Caching\n\nWe use Memcached. Do not introduce Redis.\n";
    fs::write(tmp.path().join("CLAUDE.md"), body).unwrap();
    fs::write(tmp.path().join(".cursorrules"), body).unwrap();
    fs::write(tmp.path().join(".windsurfrules"), body).unwrap();
    let report = run_bootstrap(tmp.path()).unwrap();
    // 3 candidates found, but only 1 unique body → 1 page
    assert!(report.candidates_found >= 1);
    assert_eq!(report.pages_written, 1, "duplicates across agent files must collapse to one page");
}

#[test]
fn skips_existing_pages_idempotently() {
    let tmp = tempfile::tempdir().unwrap();
    make_opted_in_repo(tmp.path());
    fs::write(
        tmp.path().join("CLAUDE.md"),
        "## Caching\n\nWe use Memcached.\n",
    )
    .unwrap();
    let r1 = run_bootstrap(tmp.path()).unwrap();
    assert!(r1.pages_written >= 1);
    let r2 = run_bootstrap(tmp.path()).unwrap();
    assert_eq!(r2.pages_written, 0);
    assert!(r2.pages_skipped_existing >= 1);
}
