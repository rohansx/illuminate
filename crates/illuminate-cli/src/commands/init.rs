use std::env;

use illuminate::Graph;

pub fn run(name: Option<String>) -> illuminate::Result<()> {
    let dir = env::current_dir().map_err(illuminate::CtxGraphError::Io)?;
    let _graph = Graph::init(&dir)?;

    let project_name = name.unwrap_or_else(|| {
        dir.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unnamed")
            .to_string()
    });

    println!("Initialized illuminate for '{project_name}'");
    println!("  Database: .illuminate/graph.db");
    println!();
    println!("Get started:");
    println!("  illuminate models download    Download ONNX models for extraction");
    println!("  illuminate log \"Your first decision or event\"");
    println!("  illuminate query \"search for something\"");

    Ok(())
}
