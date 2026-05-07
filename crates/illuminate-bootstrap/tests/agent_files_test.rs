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
    assert!(
        titles.contains(&"Caching"),
        "caching section must be captured"
    );
    assert!(
        titles.contains(&"Style"),
        "style section has 'use' / 'prefer' signals"
    );
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
    assert_eq!(cands[0].id_slug, "dec-bs-claude-md-caching");
}

#[test]
fn skips_noise_headings_like_always_do() {
    let s = "## Always do\n\nUse 2-space indents. Prefer explicit imports.\n";
    let cands = parse_agent_file("CLAUDE.md", s);
    assert!(
        cands.is_empty(),
        "always do is a list-section marker; should be skipped"
    );
}

#[test]
fn skips_noise_headings_resources() {
    let s = "## Resources\n\n- Use the docs at https://x. Always link them.\n";
    let cands = parse_agent_file("CLAUDE.md", s);
    assert!(cands.is_empty());
}

#[test]
fn skips_bullet_dominated_sections() {
    let s = "## Tools we use\n\n- We use Postgres\n- We use Redis\n- We use Memcached\n- We always check types\n";
    let cands = parse_agent_file("CLAUDE.md", s);
    assert!(
        cands.is_empty(),
        "bullet-dominated lists are reference, not decisions"
    );
}

#[test]
fn skips_code_example_blocks() {
    let s = "## Write: ALTER TABLE users DROP COLUMN preferences;\n\nExample of bad SQL we never want:\n\n```sql\nALTER TABLE users DROP COLUMN preferences;\n```\n\nUse migrations instead.\n";
    let cands = parse_agent_file("CLAUDE.md", s);
    // The "Write:" prefix or excessive code content should disqualify
    assert!(
        cands.is_empty(),
        "code example sections should be skipped, got: {:?}",
        cands.iter().map(|c| &c.title).collect::<Vec<_>>()
    );
}

#[test]
fn keeps_real_decisions() {
    let s = "## Database migrations\n\nAll schema changes go through Supabase CLI migrations. No manual SQL in the dashboard.\n";
    let cands = parse_agent_file("CLAUDE.md", s);
    assert_eq!(cands.len(), 1);
    assert_eq!(cands[0].title, "Database migrations");
}
