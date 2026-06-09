//! The `ilm` binary — the shorthand alias for `illuminate` (v3.2 roadmap).
//!
//! This is a one-line shim over the crate's shared dispatch in
//! [`illuminate_cli::run`], the same entry point the `illuminate` binary uses.
//! Because both binaries call the identical `run()`, they expose the same clap
//! command tree and behave identically — `ilm ask` / `ilm onboard` /
//! `ilm ingest` resolve exactly as their `illuminate` counterparts — with no
//! duplicated command logic to drift out of sync.

fn main() {
    illuminate_cli::run();
}
