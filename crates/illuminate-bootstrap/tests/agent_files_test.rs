use illuminate_bootstrap::agent_files::parse_agent_file;

const SAMPLE: &str = r#"# Project rules

## Caching

We use Memcached. Do not introduce Redis.

## Style

Use 2-space indents. Prefer explicit imports.

## Vague heading

Just some text without decision signals.
"#;

#[test]
fn extracts_sections_with_decision_signals() {
    let cands = parse_agent_file("CLAUDE.md", SAMPLE);
    let titles: Vec<_> = cands.iter().map(|c| c.title.as_str()).collect();
    assert!(titles.contains(&"Caching"), "caching section must be captured");
    assert!(titles.contains(&"Style"), "style section has 'use' / 'prefer' signals");
}

#[test]
fn skips_signal_free_sections() {
    let cands = parse_agent_file("CLAUDE.md", SAMPLE);
    let titles: Vec<_> = cands.iter().map(|c| c.title.as_str()).collect();
    assert!(!titles.contains(&"Vague heading"));
}

#[test]
fn id_slug_combines_filename_and_heading() {
    let cands = parse_agent_file("CLAUDE.md", "## Caching\n\nUse Memcached.\n");
    assert_eq!(cands.len(), 1);
    assert_eq!(cands[0].id_slug, "agent-claude-md-caching");
}
