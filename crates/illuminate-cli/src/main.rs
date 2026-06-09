//! The `illuminate` binary — a one-line shim over the crate's shared
//! dispatch in [`illuminate_cli::run`]. All command definitions and handling
//! live in the library so the `ilm` shorthand alias (`src/bin/ilm.rs`) shares
//! the exact same clap command tree with no duplicated logic.

fn main() {
    illuminate_cli::run();
}
