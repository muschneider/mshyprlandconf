//! Locating and loading the user's Hyprland configuration.
//!
//! Detection order (when no explicit path is given): `hyprland.lua` then
//! `hyprland.conf` under `$XDG_CONFIG_HOME/hypr` (falling back to
//! `~/.config/hypr`). Parsing goes through `hyprconf-core` and follows includes.
//! This runs off the UI thread inside an `iced::Task`.

use std::path::{Path, PathBuf};

use hyprconf_core::{conf, lua, Config, ConfigFormat, Schema};

/// The result of attempting to load a configuration.
#[derive(Debug, Clone)]
pub enum LoadState {
    /// A load is in progress.
    Loading,
    /// A configuration was loaded successfully.
    Loaded(Box<Loaded>),
    /// No configuration file could be found.
    NotFound {
        /// The paths that were probed.
        searched: Vec<PathBuf>,
    },
    /// A configuration file was found but failed to load.
    Error {
        /// The path that failed.
        path: PathBuf,
        /// The error message.
        message: String,
    },
}

impl LoadState {
    /// The loaded configuration, if the load succeeded.
    #[must_use]
    pub fn loaded(&self) -> Option<&Loaded> {
        match self {
            LoadState::Loaded(loaded) => Some(loaded),
            _ => None,
        }
    }
}

/// A successfully loaded configuration plus its provenance.
#[derive(Debug, Clone)]
pub struct Loaded {
    /// The detected on-disk format.
    pub format: ConfigFormat,
    /// The (canonical) root file that was loaded.
    pub source: PathBuf,
    /// How many additional files were pulled in via includes.
    pub included_files: usize,
    /// The parsed, format-agnostic configuration.
    pub config: Config,
    /// The number of warnings emitted while mapping the file onto the schema.
    pub warnings: usize,
}

/// Load a configuration, optionally from an explicit path.
///
/// This is synchronous; the GUI runs it inside a [`iced::Task`] so the window
/// stays responsive.
#[must_use]
pub fn load_config(explicit: Option<PathBuf>) -> LoadState {
    let schema = Schema::shared();

    if let Some(path) = explicit {
        if !path.exists() {
            return LoadState::Error {
                path: path.clone(),
                message: "file does not exist".to_string(),
            };
        }
        return load_path(&path, format_from_ext(&path), schema);
    }

    let dir = hypr_config_dir();
    let lua = dir.join("hyprland.lua");
    let conf = dir.join("hyprland.conf");

    if lua.exists() {
        load_path(&lua, ConfigFormat::Lua, schema)
    } else if conf.exists() {
        load_path(&conf, ConfigFormat::Conf, schema)
    } else {
        LoadState::NotFound {
            searched: vec![lua, conf],
        }
    }
}

fn load_path(path: &Path, format: ConfigFormat, schema: &Schema) -> LoadState {
    match format {
        ConfigFormat::Lua => match lua::LuaParser::parse_file(path) {
            Ok(bundle) => {
                let (config, warnings) = lua::bundle_to_config(&bundle, schema);
                let source = bundle
                    .root()
                    .path
                    .clone()
                    .unwrap_or_else(|| path.to_path_buf());
                LoadState::Loaded(Box::new(Loaded {
                    format,
                    source,
                    included_files: bundle.documents.len().saturating_sub(1),
                    config,
                    warnings: warnings.len(),
                }))
            }
            Err(e) => LoadState::Error {
                path: path.to_path_buf(),
                message: e.to_string(),
            },
        },
        ConfigFormat::Conf => match conf::ConfParser::parse_file(path) {
            Ok(bundle) => {
                let (config, warnings) = conf::bundle_to_config(&bundle, schema);
                let source = bundle
                    .root()
                    .path
                    .clone()
                    .unwrap_or_else(|| path.to_path_buf());
                LoadState::Loaded(Box::new(Loaded {
                    format,
                    source,
                    included_files: bundle.documents.len().saturating_sub(1),
                    config,
                    warnings: warnings.len(),
                }))
            }
            Err(e) => LoadState::Error {
                path: path.to_path_buf(),
                message: e.to_string(),
            },
        },
    }
}

/// Guess the format from a path's extension, defaulting to `conf`.
fn format_from_ext(path: &Path) -> ConfigFormat {
    match path.extension().and_then(|e| e.to_str()) {
        Some("lua") => ConfigFormat::Lua,
        _ => ConfigFormat::Conf,
    }
}

/// `$XDG_CONFIG_HOME/hypr` (or `~/.config/hypr`).
fn hypr_config_dir() -> PathBuf {
    directories::BaseDirs::new()
        .map(|dirs| dirs.config_dir().join("hypr"))
        .unwrap_or_else(|| PathBuf::from("~/.config/hypr"))
}

/// A short human label for a format, for the status bar.
#[must_use]
pub fn format_label(format: ConfigFormat) -> &'static str {
    match format {
        ConfigFormat::Lua => "Lua",
        ConfigFormat::Conf => "conf",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyprconf_core::Value;

    /// Create a unique temp directory for a test, returning its path.
    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("hyprconf-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn loads_explicit_conf() {
        let dir = temp_dir("conf");
        let path = dir.join("hyprland.conf");
        std::fs::write(&path, "general {\n    gaps_in = 7\n}\n").unwrap();

        match load_config(Some(path)) {
            LoadState::Loaded(loaded) => {
                assert_eq!(loaded.format, ConfigFormat::Conf);
                assert_eq!(loaded.config.get("general:gaps_in"), Some(&Value::Int(7)));
            }
            other => panic!("expected Loaded, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn loads_explicit_lua() {
        let dir = temp_dir("lua");
        let path = dir.join("hyprland.lua");
        std::fs::write(&path, "hl.config({ general = { gaps_in = 9 } })\n").unwrap();

        match load_config(Some(path)) {
            LoadState::Loaded(loaded) => {
                assert_eq!(loaded.format, ConfigFormat::Lua);
                assert_eq!(loaded.config.get("general:gaps_in"), Some(&Value::Int(9)));
            }
            other => panic!("expected Loaded, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_explicit_path_is_error() {
        match load_config(Some(PathBuf::from("/nonexistent/hyprconf-xyz.conf"))) {
            LoadState::Error { .. } => {}
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn format_from_ext_defaults_to_conf() {
        assert_eq!(format_from_ext(Path::new("a.lua")), ConfigFormat::Lua);
        assert_eq!(format_from_ext(Path::new("a.conf")), ConfigFormat::Conf);
        assert_eq!(format_from_ext(Path::new("a")), ConfigFormat::Conf);
    }
}
