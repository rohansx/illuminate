use illuminate_wiki::scaffold::write_scaffold;

#[test]
fn writes_full_scaffold() {
    let tmp = tempfile::tempdir().unwrap();
    write_scaffold(tmp.path()).unwrap();
    let wiki = tmp.path().join(".illuminate").join("wiki");
    assert!(wiki.join("schema.md").is_file());
    assert!(wiki.join("index.md").is_file());
    assert!(wiki.join("log.md").is_file());
    for sub in ["decisions", "patterns", "failures", "modules"] {
        assert!(wiki.join(sub).is_dir());
        assert!(wiki.join(sub).join(".gitkeep").is_file());
    }
}

#[test]
fn idempotent_does_not_overwrite_existing() {
    let tmp = tempfile::tempdir().unwrap();
    write_scaffold(tmp.path()).unwrap();
    let index = tmp.path().join(".illuminate/wiki/index.md");
    std::fs::write(&index, "custom content").unwrap();
    write_scaffold(tmp.path()).unwrap();
    assert_eq!(std::fs::read_to_string(&index).unwrap(), "custom content");
}
