//! illuminate-index: Minimal code indexer for decision-to-code anchoring.
//!
//! Uses tree-sitter to extract function signatures, struct/class declarations,
//! and import statements. Creates stable symbol hashes for anchoring decisions
//! to code locations.

pub mod indexer;
pub mod storage;
pub mod symbols;

use std::path::Path;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum IndexError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("parser error for {language}: {message}")]
    Parser { language: String, message: String },
}

pub type Result<T> = std::result::Result<T, IndexError>;

/// Supported languages for code indexing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
    Go,
    TypeScript,
    Python,
    Java,
    C,
}

impl Language {
    /// Detect language from file extension.
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "rs" => Some(Language::Rust),
            "go" => Some(Language::Go),
            "ts" | "tsx" => Some(Language::TypeScript),
            "js" | "jsx" => Some(Language::TypeScript), // reuse TS parser
            "py" => Some(Language::Python),
            "java" => Some(Language::Java),
            "c" | "h" => Some(Language::C),
            "cpp" | "cc" | "cxx" | "hpp" => Some(Language::C), // reuse C parser for now
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::Go => "go",
            Language::TypeScript => "typescript",
            Language::Python => "python",
            Language::Java => "java",
            Language::C => "c",
        }
    }

    /// Get the tree-sitter language for this language.
    pub fn tree_sitter_language(&self) -> tree_sitter::Language {
        match self {
            Language::Rust => tree_sitter_rust::LANGUAGE.into(),
            Language::Go => tree_sitter_go::LANGUAGE.into(),
            Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Language::Python => tree_sitter_python::LANGUAGE.into(),
            Language::Java => tree_sitter_java::LANGUAGE.into(),
            Language::C => tree_sitter_c::LANGUAGE.into(),
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A code anchor linking a decision episode to a code location.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CodeAnchor {
    pub episode_id: String,
    pub file_path: String,
    pub symbol_name: Option<String>,
    pub symbol_hash: Option<String>,
    pub line_start: Option<u32>,
    pub line_end: Option<u32>,
}

/// Index a single file and return extracted symbols.
pub fn index_file(path: &Path, source: &[u8], lang: Language) -> Result<Vec<symbols::Symbol>> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&lang.tree_sitter_language())
        .map_err(|e| IndexError::Parser {
            language: lang.to_string(),
            message: e.to_string(),
        })?;

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| IndexError::Parser {
            language: lang.to_string(),
            message: "failed to parse file".to_string(),
        })?;

    let file_path = path.to_string_lossy().to_string();
    let mut extracted = Vec::new();

    symbols::extract_symbols(tree.root_node(), source, &file_path, lang, &mut extracted);

    Ok(extracted)
}
