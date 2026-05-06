//! illuminate-wiki: markdown wiki layer for illuminate.
//!
//! Pages are markdown files with YAML front-matter, organized under
//! `.illuminate/wiki/{decisions,patterns,failures,modules}/`. This crate
//! parses, validates, and (later) renders them, plus offers helpers to
//! register pages into the [`illuminate::Graph`] as episodes.

pub mod lint;
pub mod page;

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
