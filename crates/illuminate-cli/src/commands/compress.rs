//! `illuminate compress` — shrink a large JSON payload (tool output, logs,
//! query rows) before it's fed to an agent, using illuminate's SmartCrusher
//! port. Reads from a file or stdin, writes the compacted JSON to stdout, and
//! reports the token-savings band on stderr (so it composes in a pipe).

use std::io::Read;
use std::path::PathBuf;

use illuminate_compress::{CrusherConfig, crush};

pub fn run(
    file: Option<PathBuf>,
    max_items: Option<usize>,
    no_marker: bool,
    stats: bool,
) -> std::io::Result<()> {
    let input = match file {
        Some(p) => std::fs::read_to_string(p)?,
        None => {
            let mut s = String::new();
            std::io::stdin().read_to_string(&mut s)?;
            s
        }
    };

    let value: serde_json::Value = serde_json::from_str(&input).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("input is not valid JSON: {e}"),
        )
    })?;

    let mut cfg = CrusherConfig::default();
    if let Some(m) = max_items {
        cfg.max_items = m;
    }
    if no_marker {
        cfg.marker = false;
    }

    let out = crush(&value, &cfg);

    println!(
        "{}",
        serde_json::to_string_pretty(&out.value).map_err(std::io::Error::other)?
    );

    // Savings band on stderr so stdout stays a clean JSON document.
    if stats {
        let summary = serde_json::json!({
            "orig_tokens": out.orig_tokens,
            "crushed_tokens": out.crushed_tokens,
            "savings_ratio": (out.savings_ratio() * 1000.0).round() / 1000.0,
            "dropped": out.dropped,
        });
        eprintln!("{summary}");
    } else {
        eprintln!(
            "compressed: {} → {} tokens (~{:.0}% saved, {} rows dropped)",
            out.orig_tokens,
            out.crushed_tokens,
            out.savings_ratio() * 100.0,
            out.dropped
        );
    }
    Ok(())
}
