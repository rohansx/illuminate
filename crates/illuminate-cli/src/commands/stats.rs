use super::open_graph;
use illuminate_trail::TokenTotals;

pub fn run() -> illuminate::Result<()> {
    let graph = open_graph()?;
    let stats = graph.stats()?;

    println!("illuminate stats");
    println!("{}", "-".repeat(30));
    println!("Episodes:  {}", stats.episode_count);
    println!("Entities:  {}", stats.entity_count);
    println!("Edges:     {}", stats.edge_count);

    if !stats.sources.is_empty() {
        let sources: Vec<String> = stats
            .sources
            .iter()
            .map(|(name, count)| format!("{name} ({count})"))
            .collect();
        println!("Sources:   {}", sources.join(", "));
    }

    println!("DB size:   {}", format_bytes(stats.db_size_bytes));

    print_token_panel();

    Ok(())
}

/// Print the token-savings panel computed from the repo's captured trails.
///
/// Folds every parseable `.illuminate/trail/*.jsonl` via
/// [`illuminate_trail::aggregate_tokens`]. When no trails are captured (no
/// opted-in repo, no `trail/` directory, or zero sessions) it prints a single
/// "no token data captured yet" line — the command still succeeds.
fn print_token_panel() {
    let records = super::trail_tokens::load_records();
    let totals = illuminate_trail::aggregate_tokens(&records);

    println!();
    println!("tokens");
    println!("{}", "-".repeat(30));
    if totals.sessions == 0 {
        println!("no token data captured yet — run `illuminate trail watch` to capture sessions");
        return;
    }
    println!("Sessions:     {}", totals.sessions);
    println!("Input:        {}", totals.input_tokens);
    println!("Output:       {}", totals.output_tokens);
    println!("Cache-read:   {}", totals.cache_read_input_tokens);
    println!("Cache-create: {}", totals.cache_creation_input_tokens);
    println!("Cache-saved%: {:.2}%", totals.cache_saved_pct);
}

/// Render a [`TokenTotals`] as the dashboard `tokens` envelope object.
///
/// Shared with the `illuminate wiki serve` dashboard so the CLI panel and the
/// `/api/dashboard` tile derive their numbers from one place. The key names
/// are a stable JSON contract (see `serve_dashboard_api_test.rs`).
pub fn tokens_json(totals: &TokenTotals) -> serde_json::Value {
    serde_json::json!({
        "sessions": totals.sessions,
        "input": totals.input_tokens,
        "output": totals.output_tokens,
        "cache_read": totals.cache_read_input_tokens,
        "cache_creation": totals.cache_creation_input_tokens,
        "cache_saved_pct": totals.cache_saved_pct,
    })
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn totals(sessions: u64, input: u64, output: u64, cache_read: u64) -> TokenTotals {
        TokenTotals {
            sessions,
            input_tokens: input,
            output_tokens: output,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: cache_read,
            cache_saved_pct: if cache_read + input == 0 {
                0.0
            } else {
                (cache_read as f64) / ((cache_read + input) as f64) * 100.0
            },
        }
    }

    #[test]
    fn tokens_json_has_stable_numeric_keys() {
        let v = tokens_json(&totals(3, 1000, 250, 500));
        assert_eq!(v["sessions"], 3);
        assert_eq!(v["input"], 1000);
        assert_eq!(v["output"], 250);
        assert_eq!(v["cache_read"], 500);
        assert_eq!(v["cache_creation"], 0);
        assert!(v["cache_saved_pct"].is_number());
        // every field numeric, never null
        for k in [
            "sessions",
            "input",
            "output",
            "cache_read",
            "cache_creation",
            "cache_saved_pct",
        ] {
            assert!(v[k].is_number(), "{k} must be numeric");
        }
    }

    #[test]
    fn tokens_json_zero_totals_are_numeric_zeros() {
        let v = tokens_json(&totals(0, 0, 0, 0));
        assert_eq!(v["sessions"], 0);
        assert_eq!(v["input"], 0);
        assert_eq!(v["cache_saved_pct"].as_f64(), Some(0.0));
        assert!(!v["cache_saved_pct"].is_null());
    }
}
