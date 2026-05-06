use illuminate_trail::repo::resolve_repo;
use std::fs;
use std::path::Path;

fn make_opted_in(root: &Path) {
    fs::create_dir_all(root.join(".illuminate")).unwrap();
    fs::write(root.join(".illuminate/illuminate.toml"), "name = 'test'\n").unwrap();
}

#[test]
fn detects_repo_at_cwd_directly() {
    let tmp = tempfile::tempdir().unwrap();
    make_opted_in(tmp.path());
    let resolved = resolve_repo(tmp.path()).unwrap();
    assert_eq!(resolved, tmp.path());
}

#[test]
fn walks_ancestors_to_find_opt_in_marker() {
    let tmp = tempfile::tempdir().unwrap();
    make_opted_in(tmp.path());
    let nested = tmp.path().join("a/b/c");
    fs::create_dir_all(&nested).unwrap();
    let resolved = resolve_repo(&nested).unwrap();
    assert_eq!(resolved, tmp.path());
}

#[test]
fn returns_none_when_no_marker_found() {
    let tmp = tempfile::tempdir().unwrap();
    // no .illuminate
    assert!(resolve_repo(tmp.path()).is_none());
}
