//! A lossless model of a single `.lua` file.
//!
//! `full_moon` is itself a lossless (concrete) parser: its `Ast` preserves every
//! token, comment and whitespace, and its `Display` reproduces the source
//! exactly. So [`LuaDocument`] simply owns the parsed `Ast`, and `to_text()` is
//! a faithful round-trip — comments, ordering and any dynamic Lua included.

use std::path::PathBuf;

use full_moon::ast::Ast;

/// A parsed `.lua` file: its origin, original source, and lossless AST.
#[derive(Debug, Clone)]
pub struct LuaDocument {
    /// The file this document was read from, if any.
    pub path: Option<PathBuf>,
    source: String,
    ast: Ast,
}

impl LuaDocument {
    /// Construct from an already-parsed AST and its original source.
    #[must_use]
    pub fn new(path: Option<PathBuf>, source: String, ast: Ast) -> Self {
        Self { path, source, ast }
    }

    /// The original source text.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The parsed AST.
    #[must_use]
    pub fn ast(&self) -> &Ast {
        &self.ast
    }

    /// Serialize back to Lua. For an unedited document this reproduces the
    /// original input (comments, ordering and all) byte-for-byte.
    #[must_use]
    pub fn to_text(&self) -> String {
        self.ast.to_string()
    }
}
