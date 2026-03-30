//! illuminate-watch: Auto-ingestion daemon for git commits, GitHub PRs, and webhooks.
//!
//! Monitors development workflow and feeds decision-relevant text into the extraction pipeline.

pub mod git;
pub mod signal;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum WatchError {
    #[error("git error: {0}")]
    Git(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("graph error: {0}")]
    Graph(#[from] illuminate::IlluminateError),

    #[error("parse error: {0}")]
    Parse(String),
}

pub type Result<T> = std::result::Result<T, WatchError>;
