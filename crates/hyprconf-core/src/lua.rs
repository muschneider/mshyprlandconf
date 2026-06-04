//! The Hyprland Lua format (the `hl` global API), the modern default since 0.55.
//!
//! ## Reading strategy
//!
//! The reader is a **lossless static parse** of the declarative subset via the
//! [`full_moon`] crate (a concrete-syntax Lua parser). This is the right choice
//! for an *editor*: comments, ordering and formatting survive round-trips, and
//! we never have to execute untrusted config code to read it.
//!
//! - [`LuaDocument`] owns the parsed `Ast`; `to_text()` reproduces the source
//!   exactly (full_moon is lossless), so unedited files round-trip byte-for-byte.
//! - [`document_to_config`] / [`bundle_to_config`] interpret the recognised
//!   `hl.*` calls (and top-level `require`) into the format-agnostic
//!   [`crate::Config`].
//! - Anything outside the declarative subset (functions, loops, conditionals,
//!   `hl.on`/`hl.timer`/`hl.define_submap` closures, variable-captured
//!   `require`s, ...) is **not interpreted**: it stays verbatim in the document
//!   and is surfaced as [`LuaWarning::DynamicRegion`] so a GUI can show it
//!   read-only and never silently flatten user logic.
//!
//! A sandboxed `mlua` evaluation path (for full fidelity on dynamic configs)
//! is intentionally **not** implemented in this step; it is optional per the
//! spec and would be gated behind a `lua-eval` feature. The static path is
//! complete and is what editing is built on.
//!
//! ## Writing strategy
//!
//! [`LuaSerializer::serialize`] emits fresh idiomatic Lua against the `hl` API
//! confirmed in `meta/hl.meta.lua` (Hyprland 0.55.2): `hl.config(table)`,
//! `hl.bind`, `hl.window_rule`/`hl.layer_rule`/`hl.monitor`/`hl.workspace_rule`,
//! `hl.env`, `hl.exec_cmd`, `hl.animation`, `hl.curve`. Re-serializing a parsed
//! document instead uses the lossless path. See the CHANGELOG for the handful of
//! encodings (e.g. bind dispatcher-as-string, raw `match` strings) that favour
//! round-trip fidelity over a fully-verified API shape.

mod document;
mod extract;
mod mapper;
mod parser;
mod serializer;

pub use document::LuaDocument;
pub use mapper::{bundle_to_config, document_to_config, LuaWarning};
pub use parser::{LuaBundle, LuaError, LuaParser};
pub use serializer::{value_to_lua, LuaSerializer};
