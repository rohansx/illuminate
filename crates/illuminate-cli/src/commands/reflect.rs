use super::open_graph;
use illuminate_reflect::{ReflexionInput, ReflexionStore, Severity};

/// Record a reflexion episode.
pub fn run(
    failure: String,
    root_cause: Option<String>,
    fix: Option<String>,
    files: Option<String>,
    severity: Option<String>,
) -> illuminate::Result<()> {
    let graph = open_graph()?;
    let store = ReflexionStore::new(graph);

    let input = ReflexionInput {
        failure,
        root_cause: root_cause.unwrap_or_default(),
        corrective_action: fix.unwrap_or_default(),
        files_affected: files
            .map(|f| f.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default(),
        severity: severity
            .as_deref()
            .map(parse_severity)
            .unwrap_or(Severity::Medium),
    };

    let episode_id = store.record(&input)?;
    println!("reflexion recorded: {episode_id}");

    Ok(())
}

fn parse_severity(s: &str) -> Severity {
    match s.to_lowercase().as_str() {
        "low" => Severity::Low,
        "high" => Severity::High,
        "critical" => Severity::Critical,
        _ => Severity::Medium,
    }
}
