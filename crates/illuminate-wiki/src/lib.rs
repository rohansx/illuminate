//! illuminate-wiki: markdown wiki layer for illuminate.
//!
//! Pages are markdown files with YAML front-matter, organized under
//! `.illuminate/wiki/{decisions,patterns,failures,modules}/`. This crate
//! parses, validates, and (later) renders them, plus offers helpers to
//! register pages into the [`illuminate::Graph`] as episodes.

pub mod dashboard;
pub mod episode;
pub mod lint;
pub mod page;
pub mod render;
pub mod scaffold;
pub mod serve;
pub mod walk;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum WikiError {
    #[error("parse error: {0}")]
    Parse(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("yaml error: {0}")]
    Yaml(String),
}

pub type Result<T> = std::result::Result<T, WikiError>;
