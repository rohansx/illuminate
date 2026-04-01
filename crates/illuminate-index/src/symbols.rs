//! Symbol extraction from tree-sitter AST nodes.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::Language;

/// A code symbol extracted from source code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub file_path: String,
    pub name: String,
    pub symbol_type: SymbolType,
    pub signature: Option<String>,
    pub visibility: Visibility,
    pub line_start: u32,
    pub line_end: u32,
    pub hash: String,
    pub language: String,
}

/// Type of code symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SymbolType {
    Function,
    Struct,
    Class,
    Interface,
    Enum,
    Trait,
    Import,
}

impl std::fmt::Display for SymbolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolType::Function => write!(f, "function"),
            SymbolType::Struct => write!(f, "struct"),
            SymbolType::Class => write!(f, "class"),
            SymbolType::Interface => write!(f, "interface"),
            SymbolType::Enum => write!(f, "enum"),
            SymbolType::Trait => write!(f, "trait"),
            SymbolType::Import => write!(f, "import"),
        }
    }
}

/// Visibility of a symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    Public,
    Private,
}

/// Compute a stable hash for a symbol signature.
///
/// The hash is based on language + symbol_type + name + signature,
/// normalized to be stable across formatting changes.
pub fn symbol_hash(
    lang: &str,
    symbol_type: &SymbolType,
    name: &str,
    signature: Option<&str>,
) -> String {
    let normalized = format!(
        "{}:{}:{}:{}",
        lang,
        symbol_type,
        name.trim(),
        signature.unwrap_or("").trim()
    );
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Extract symbols from a tree-sitter AST node.
pub fn extract_symbols(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    file_path: &str,
    lang: Language,
    out: &mut Vec<Symbol>,
) {
    let kind = node.kind();

    match lang {
        Language::Rust => extract_rust_symbol(node, source, file_path, kind, out),
        Language::Go => extract_go_symbol(node, source, file_path, kind, out),
        Language::TypeScript => extract_ts_symbol(node, source, file_path, kind, out),
        Language::Python => extract_python_symbol(node, source, file_path, kind, out),
        Language::Java => extract_java_symbol(node, source, file_path, kind, out),
        Language::C => extract_c_symbol(node, source, file_path, kind, out),
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_symbols(child, source, file_path, lang, out);
    }
}

fn node_text<'a>(node: tree_sitter::Node<'_>, source: &'a [u8]) -> &'a str {
    node.utf8_text(source).unwrap_or("")
}

fn child_by_field<'a>(node: tree_sitter::Node<'a>, field: &str) -> Option<tree_sitter::Node<'a>> {
    node.child_by_field_name(field)
}

fn extract_rust_symbol(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    file_path: &str,
    kind: &str,
    out: &mut Vec<Symbol>,
) {
    match kind {
        "function_item" => {
            if let Some(name_node) = child_by_field(node, "name") {
                let name = node_text(name_node, source).to_string();
                let sig = node_text(node, source)
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_string();
                let vis = if node_text(node, source).starts_with("pub") {
                    Visibility::Public
                } else {
                    Visibility::Private
                };
                let hash = symbol_hash("rust", &SymbolType::Function, &name, Some(&sig));
                out.push(Symbol {
                    file_path: file_path.to_string(),
                    name,
                    symbol_type: SymbolType::Function,
                    signature: Some(sig),
                    visibility: vis,
                    line_start: node.start_position().row as u32 + 1,
                    line_end: node.end_position().row as u32 + 1,
                    hash,
                    language: "rust".to_string(),
                });
            }
        }
        "struct_item" => {
            if let Some(name_node) = child_by_field(node, "name") {
                let name = node_text(name_node, source).to_string();
                let vis = if node_text(node, source).starts_with("pub") {
                    Visibility::Public
                } else {
                    Visibility::Private
                };
                let hash = symbol_hash("rust", &SymbolType::Struct, &name, None);
                out.push(Symbol {
                    file_path: file_path.to_string(),
                    name,
                    symbol_type: SymbolType::Struct,
                    signature: None,
                    visibility: vis,
                    line_start: node.start_position().row as u32 + 1,
                    line_end: node.end_position().row as u32 + 1,
                    hash,
                    language: "rust".to_string(),
                });
            }
        }
        "enum_item" => {
            if let Some(name_node) = child_by_field(node, "name") {
                let name = node_text(name_node, source).to_string();
                let hash = symbol_hash("rust", &SymbolType::Enum, &name, None);
                out.push(Symbol {
                    file_path: file_path.to_string(),
                    name,
                    symbol_type: SymbolType::Enum,
                    signature: None,
                    visibility: Visibility::Public,
                    line_start: node.start_position().row as u32 + 1,
                    line_end: node.end_position().row as u32 + 1,
                    hash,
                    language: "rust".to_string(),
                });
            }
        }
        "trait_item" => {
            if let Some(name_node) = child_by_field(node, "name") {
                let name = node_text(name_node, source).to_string();
                let hash = symbol_hash("rust", &SymbolType::Trait, &name, None);
                out.push(Symbol {
                    file_path: file_path.to_string(),
                    name,
                    symbol_type: SymbolType::Trait,
                    signature: None,
                    visibility: Visibility::Public,
                    line_start: node.start_position().row as u32 + 1,
                    line_end: node.end_position().row as u32 + 1,
                    hash,
                    language: "rust".to_string(),
                });
            }
        }
        "use_declaration" => {
            let text = node_text(node, source).to_string();
            let hash = symbol_hash("rust", &SymbolType::Import, &text, None);
            out.push(Symbol {
                file_path: file_path.to_string(),
                name: text,
                symbol_type: SymbolType::Import,
                signature: None,
                visibility: Visibility::Private,
                line_start: node.start_position().row as u32 + 1,
                line_end: node.end_position().row as u32 + 1,
                hash,
                language: "rust".to_string(),
            });
        }
        _ => {}
    }
}

fn extract_go_symbol(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    file_path: &str,
    kind: &str,
    out: &mut Vec<Symbol>,
) {
    match kind {
        "function_declaration" => {
            if let Some(name_node) = child_by_field(node, "name") {
                let name = node_text(name_node, source).to_string();
                let sig = node_text(node, source)
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_string();
                let vis = if name.chars().next().is_some_and(|c| c.is_uppercase()) {
                    Visibility::Public
                } else {
                    Visibility::Private
                };
                let hash = symbol_hash("go", &SymbolType::Function, &name, Some(&sig));
                out.push(Symbol {
                    file_path: file_path.to_string(),
                    name,
                    symbol_type: SymbolType::Function,
                    signature: Some(sig),
                    visibility: vis,
                    line_start: node.start_position().row as u32 + 1,
                    line_end: node.end_position().row as u32 + 1,
                    hash,
                    language: "go".to_string(),
                });
            }
        }
        "type_declaration" => {
            // Walk children to find type_spec
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "type_spec"
                    && let Some(name_node) = child_by_field(child, "name")
                {
                    let name = node_text(name_node, source).to_string();
                    let hash = symbol_hash("go", &SymbolType::Struct, &name, None);
                    out.push(Symbol {
                        file_path: file_path.to_string(),
                        name,
                        symbol_type: SymbolType::Struct,
                        signature: None,
                        visibility: Visibility::Public,
                        line_start: child.start_position().row as u32 + 1,
                        line_end: child.end_position().row as u32 + 1,
                        hash,
                        language: "go".to_string(),
                    });
                }
            }
        }
        _ => {}
    }
}

fn extract_ts_symbol(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    file_path: &str,
    kind: &str,
    out: &mut Vec<Symbol>,
) {
    match kind {
        "function_declaration" | "method_definition" => {
            if let Some(name_node) = child_by_field(node, "name") {
                let name = node_text(name_node, source).to_string();
                let sig = node_text(node, source)
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_string();
                let hash = symbol_hash("typescript", &SymbolType::Function, &name, Some(&sig));
                out.push(Symbol {
                    file_path: file_path.to_string(),
                    name,
                    symbol_type: SymbolType::Function,
                    signature: Some(sig),
                    visibility: Visibility::Public,
                    line_start: node.start_position().row as u32 + 1,
                    line_end: node.end_position().row as u32 + 1,
                    hash,
                    language: "typescript".to_string(),
                });
            }
        }
        "class_declaration" => {
            if let Some(name_node) = child_by_field(node, "name") {
                let name = node_text(name_node, source).to_string();
                let hash = symbol_hash("typescript", &SymbolType::Class, &name, None);
                out.push(Symbol {
                    file_path: file_path.to_string(),
                    name,
                    symbol_type: SymbolType::Class,
                    signature: None,
                    visibility: Visibility::Public,
                    line_start: node.start_position().row as u32 + 1,
                    line_end: node.end_position().row as u32 + 1,
                    hash,
                    language: "typescript".to_string(),
                });
            }
        }
        "interface_declaration" => {
            if let Some(name_node) = child_by_field(node, "name") {
                let name = node_text(name_node, source).to_string();
                let hash = symbol_hash("typescript", &SymbolType::Interface, &name, None);
                out.push(Symbol {
                    file_path: file_path.to_string(),
                    name,
                    symbol_type: SymbolType::Interface,
                    signature: None,
                    visibility: Visibility::Public,
                    line_start: node.start_position().row as u32 + 1,
                    line_end: node.end_position().row as u32 + 1,
                    hash,
                    language: "typescript".to_string(),
                });
            }
        }
        "import_statement" => {
            let text = node_text(node, source).to_string();
            let hash = symbol_hash("typescript", &SymbolType::Import, &text, None);
            out.push(Symbol {
                file_path: file_path.to_string(),
                name: text,
                symbol_type: SymbolType::Import,
                signature: None,
                visibility: Visibility::Private,
                line_start: node.start_position().row as u32 + 1,
                line_end: node.end_position().row as u32 + 1,
                hash,
                language: "typescript".to_string(),
            });
        }
        _ => {}
    }
}

fn extract_python_symbol(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    file_path: &str,
    kind: &str,
    out: &mut Vec<Symbol>,
) {
    match kind {
        "function_definition" => {
            if let Some(name_node) = child_by_field(node, "name") {
                let name = node_text(name_node, source).to_string();
                let sig = node_text(node, source)
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_string();
                let vis = if name.starts_with('_') {
                    Visibility::Private
                } else {
                    Visibility::Public
                };
                let hash = symbol_hash("python", &SymbolType::Function, &name, Some(&sig));
                out.push(Symbol {
                    file_path: file_path.to_string(),
                    name,
                    symbol_type: SymbolType::Function,
                    signature: Some(sig),
                    visibility: vis,
                    line_start: node.start_position().row as u32 + 1,
                    line_end: node.end_position().row as u32 + 1,
                    hash,
                    language: "python".to_string(),
                });
            }
        }
        "class_definition" => {
            if let Some(name_node) = child_by_field(node, "name") {
                let name = node_text(name_node, source).to_string();
                let hash = symbol_hash("python", &SymbolType::Class, &name, None);
                out.push(Symbol {
                    file_path: file_path.to_string(),
                    name,
                    symbol_type: SymbolType::Class,
                    signature: None,
                    visibility: Visibility::Public,
                    line_start: node.start_position().row as u32 + 1,
                    line_end: node.end_position().row as u32 + 1,
                    hash,
                    language: "python".to_string(),
                });
            }
        }
        "import_statement" | "import_from_statement" => {
            let text = node_text(node, source).to_string();
            let hash = symbol_hash("python", &SymbolType::Import, &text, None);
            out.push(Symbol {
                file_path: file_path.to_string(),
                name: text,
                symbol_type: SymbolType::Import,
                signature: None,
                visibility: Visibility::Private,
                line_start: node.start_position().row as u32 + 1,
                line_end: node.end_position().row as u32 + 1,
                hash,
                language: "python".to_string(),
            });
        }
        _ => {}
    }
}

fn extract_java_symbol(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    file_path: &str,
    kind: &str,
    out: &mut Vec<Symbol>,
) {
    match kind {
        "method_declaration" => {
            if let Some(name_node) = child_by_field(node, "name") {
                let name = node_text(name_node, source).to_string();
                let sig = node_text(node, source)
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_string();
                let vis = if sig.contains("public") {
                    Visibility::Public
                } else {
                    Visibility::Private
                };
                let hash = symbol_hash("java", &SymbolType::Function, &name, Some(&sig));
                out.push(Symbol {
                    file_path: file_path.to_string(),
                    name,
                    symbol_type: SymbolType::Function,
                    signature: Some(sig),
                    visibility: vis,
                    line_start: node.start_position().row as u32 + 1,
                    line_end: node.end_position().row as u32 + 1,
                    hash,
                    language: "java".to_string(),
                });
            }
        }
        "class_declaration" => {
            if let Some(name_node) = child_by_field(node, "name") {
                let name = node_text(name_node, source).to_string();
                let hash = symbol_hash("java", &SymbolType::Class, &name, None);
                out.push(Symbol {
                    file_path: file_path.to_string(),
                    name,
                    symbol_type: SymbolType::Class,
                    signature: None,
                    visibility: Visibility::Public,
                    line_start: node.start_position().row as u32 + 1,
                    line_end: node.end_position().row as u32 + 1,
                    hash,
                    language: "java".to_string(),
                });
            }
        }
        "interface_declaration" => {
            if let Some(name_node) = child_by_field(node, "name") {
                let name = node_text(name_node, source).to_string();
                let hash = symbol_hash("java", &SymbolType::Interface, &name, None);
                out.push(Symbol {
                    file_path: file_path.to_string(),
                    name,
                    symbol_type: SymbolType::Interface,
                    signature: None,
                    visibility: Visibility::Public,
                    line_start: node.start_position().row as u32 + 1,
                    line_end: node.end_position().row as u32 + 1,
                    hash,
                    language: "java".to_string(),
                });
            }
        }
        _ => {}
    }
}

fn extract_c_symbol(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    file_path: &str,
    kind: &str,
    out: &mut Vec<Symbol>,
) {
    match kind {
        "function_definition" => {
            if let Some(declarator) = child_by_field(node, "declarator") {
                // For C, the function name is nested inside the declarator
                if let Some(name_node) = child_by_field(declarator, "declarator") {
                    let name = node_text(name_node, source).to_string();
                    let sig = node_text(node, source)
                        .lines()
                        .next()
                        .unwrap_or("")
                        .to_string();
                    let hash = symbol_hash("c", &SymbolType::Function, &name, Some(&sig));
                    out.push(Symbol {
                        file_path: file_path.to_string(),
                        name,
                        symbol_type: SymbolType::Function,
                        signature: Some(sig),
                        visibility: Visibility::Public,
                        line_start: node.start_position().row as u32 + 1,
                        line_end: node.end_position().row as u32 + 1,
                        hash,
                        language: "c".to_string(),
                    });
                }
            }
        }
        "struct_specifier" => {
            if let Some(name_node) = child_by_field(node, "name") {
                let name = node_text(name_node, source).to_string();
                let hash = symbol_hash("c", &SymbolType::Struct, &name, None);
                out.push(Symbol {
                    file_path: file_path.to_string(),
                    name,
                    symbol_type: SymbolType::Struct,
                    signature: None,
                    visibility: Visibility::Public,
                    line_start: node.start_position().row as u32 + 1,
                    line_end: node.end_position().row as u32 + 1,
                    hash,
                    language: "c".to_string(),
                });
            }
        }
        "preproc_include" => {
            let text = node_text(node, source).to_string();
            let hash = symbol_hash("c", &SymbolType::Import, &text, None);
            out.push(Symbol {
                file_path: file_path.to_string(),
                name: text,
                symbol_type: SymbolType::Import,
                signature: None,
                visibility: Visibility::Private,
                line_start: node.start_position().row as u32 + 1,
                line_end: node.end_position().row as u32 + 1,
                hash,
                language: "c".to_string(),
            });
        }
        _ => {}
    }
}
