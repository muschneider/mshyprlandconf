// SPDX-License-Identifier: MIT OR Apache-2.0
//! Parsing of `.lua` text into a lossless [`LuaDocument`], plus `require()`
//! include resolution with cycle/missing-file detection.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::lua::document::LuaDocument;
use crate::lua::extract::require_modules;

/// A typed error from reading or following Lua `require()` includes.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LuaError {
    /// A file could not be read for a non-"not found" reason.
    #[error("failed to read {path}: {source}")]
    Io {
        /// The file involved.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// A file (root or `require`d) does not exist.
    #[error("file not found: {path}")]
    NotFound {
        /// The missing path.
        path: PathBuf,
    },

    /// The Lua source failed to parse.
    #[error("lua parse error{}: {message}", path_suffix(.path))]
    Parse {
        /// The file that failed to parse, if known.
        path: Option<PathBuf>,
        /// A human-readable description of the parse failure(s).
        message: String,
    },

    /// A `require()` include forms a cycle.
    #[error("require cycle detected at {path} (chain: {})", format_chain(.chain))]
    IncludeCycle {
        /// The path that closed the cycle.
        path: PathBuf,
        /// The include chain leading up to it (root first).
        chain: Vec<PathBuf>,
    },
}

fn path_suffix(path: &Option<PathBuf>) -> String {
    path.as_ref()
        .map(|p| format!(" in {}", p.display()))
        .unwrap_or_default()
}

fn format_chain(chain: &[PathBuf]) -> String {
    chain
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(" -> ")
}

/// All `.lua` documents loaded while following a root file's `require`s.
#[derive(Debug, Clone)]
pub struct LuaBundle {
    /// Every loaded file, root first.
    pub documents: Vec<LuaDocument>,
    /// The index of the root document.
    pub root: usize,
    /// Canonical path -> index, for `require` resolution by the mapper.
    pub by_path: HashMap<PathBuf, usize>,
}

impl LuaBundle {
    /// The root document.
    #[must_use]
    pub fn root(&self) -> &LuaDocument {
        &self.documents[self.root]
    }
}

/// Parses `.lua` text and follows `require()` includes.
#[derive(Debug, Default, Clone, Copy)]
pub struct LuaParser;

impl LuaParser {
    /// Parse a single Lua file's text into a [`LuaDocument`] (no I/O, no
    /// `require` following).
    ///
    /// # Errors
    ///
    /// Returns [`LuaError::Parse`] if the text is not valid Lua.
    pub fn parse_str(text: &str, path: Option<PathBuf>) -> Result<LuaDocument, LuaError> {
        let ast = full_moon::parse(text).map_err(|errors| LuaError::Parse {
            path: path.clone(),
            message: errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; "),
        })?;
        Ok(LuaDocument::new(path, text.to_string(), ast))
    }

    /// Read and parse a file, following every top-level `require("module")`.
    ///
    /// Modules resolve to `<dir>/<module>.lua` (with `.` treated as a path
    /// separator) relative to the requiring file. Cycles and missing files are
    /// reported as [`LuaError`]s rather than panicking.
    ///
    /// # Errors
    ///
    /// Returns [`LuaError`] on I/O failure, a missing file, a parse error, or a
    /// `require` cycle.
    pub fn parse_file(path: impl AsRef<Path>) -> Result<LuaBundle, LuaError> {
        let mut bundle = LuaBundle {
            documents: Vec::new(),
            root: 0,
            by_path: HashMap::new(),
        };
        let mut chain: Vec<PathBuf> = Vec::new();
        let root = load(path.as_ref(), &mut bundle, &mut chain)?;
        bundle.root = root;
        Ok(bundle)
    }
}

fn load(path: &Path, bundle: &mut LuaBundle, chain: &mut Vec<PathBuf>) -> Result<usize, LuaError> {
    let canonical = canonicalize(path)?;

    if chain.contains(&canonical) {
        return Err(LuaError::IncludeCycle {
            path: canonical,
            chain: chain.clone(),
        });
    }
    if let Some(&existing) = bundle.by_path.get(&canonical) {
        return Ok(existing); // already loaded (diamond include)
    }

    let text = std::fs::read_to_string(&canonical).map_err(|source| map_io(&canonical, source))?;
    let document = LuaParser::parse_str(&text, Some(canonical.clone()))?;

    let my_index = bundle.documents.len();
    bundle.documents.push(document);
    bundle.by_path.insert(canonical.clone(), my_index);
    chain.push(canonical.clone());

    let dir = canonical
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let stmts: Vec<_> = bundle.documents[my_index].ast().nodes().stmts().collect();
    let modules = require_modules(&stmts);
    drop(stmts);

    for module in modules {
        let candidate = resolve_require(&dir, &module);
        load(&candidate, bundle, chain)?;
    }

    chain.pop();
    Ok(my_index)
}

fn canonicalize(path: &Path) -> Result<PathBuf, LuaError> {
    std::fs::canonicalize(path).map_err(|source| map_io(path, source))
}

fn map_io(path: &Path, source: std::io::Error) -> LuaError {
    if source.kind() == std::io::ErrorKind::NotFound {
        LuaError::NotFound {
            path: path.to_path_buf(),
        }
    } else {
        LuaError::Io {
            path: path.to_path_buf(),
            source,
        }
    }
}

/// Resolve a `require` module string to a file path: `.` becomes `/`, and a
/// `.lua` extension is appended, relative to the requiring file's directory.
pub(crate) fn resolve_require(dir: &Path, module: &str) -> PathBuf {
    let relative = module.replace('.', "/");
    dir.join(format!("{relative}.lua"))
}
