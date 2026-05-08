//! illuminate-bootstrap: cold-start ingestion.
//!
//! Each source produces [`BootstrapCandidate`]s; the orchestrator dedupes,
//! writes wiki markdown pages, and registers graph episodes.

pub mod adr;
pub mod agent_files;
pub mod candidate;
pub mod git_history;
pub mod orchestrate;
pub mod readme;
pub mod signals;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum BootstrapError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error: {0}")]
    Parse(String),
}

pub type Result<T> = std::result::Result<T, BootstrapError>;
