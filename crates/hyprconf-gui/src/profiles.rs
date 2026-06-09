// SPDX-License-Identifier: MIT OR Apache-2.0
//! Named configuration *profiles*: snapshots of a config the user can save
//! under a name and re-open later, plus "import from an arbitrary path".
//!
//! Profiles live under `$XDG_DATA_HOME/hyprconf/profiles/<name>.{conf,lua}`.
//! Saving serialises the in-memory [`Config`] with the chosen format's writer
//! and writes it atomically; loading is just [`crate::load::load_config`] over
//! the profile's path (format inferred from the extension), so the whole
//! editing/round-trip pipeline is reused unchanged.

use std::path::PathBuf;

use hyprconf_core::conf::config_to_conf;
use hyprconf_core::{Config, ConfigFormat, LuaSerializer};

use crate::settings::data_dir;

/// `$XDG_DATA_HOME/hyprconf/profiles`.
#[must_use]
pub fn profiles_dir() -> Option<PathBuf> {
    data_dir().map(|d| d.join("profiles"))
}

/// A saved profile: its display name (file stem) and full path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    /// The user-facing name (the file stem).
    pub name: String,
    /// The on-disk path.
    pub path: PathBuf,
}

/// List saved profiles, sorted by name. Empty if the directory is absent.
#[must_use]
pub fn list() -> Vec<Profile> {
    let Some(dir) = profiles_dir() else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut profiles: Vec<Profile> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| matches!(p.extension().and_then(|e| e.to_str()), Some("conf" | "lua")))
        .filter_map(|path| {
            let name = path.file_stem()?.to_string_lossy().into_owned();
            Some(Profile { name, path })
        })
        .collect();
    profiles.sort_by(|a, b| a.name.cmp(&b.name));
    profiles
}

/// Serialise `config` in `format` and save it as a profile named `name`.
///
/// Returns the path written. The name is sanitised to a safe file stem; an
/// empty/blank name falls back to `profile`.
///
/// # Errors
///
/// Returns a human-readable message if no data directory is available or the
/// write fails.
pub fn save(name: &str, format: ConfigFormat, config: &Config) -> Result<PathBuf, String> {
    let dir = profiles_dir().ok_or("no data directory available")?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("create {}: {e}", dir.display()))?;

    let stem = sanitize(name);
    let ext = match format {
        ConfigFormat::Lua => "lua",
        ConfigFormat::Conf => "conf",
    };
    let path = dir.join(format!("{stem}.{ext}"));

    let text = match format {
        ConfigFormat::Lua => LuaSerializer::serialize(config),
        ConfigFormat::Conf => config_to_conf(config),
    };
    hyprconf_core::fs::atomic_write(&path, &text).map_err(|e| format!("write failed: {e}"))?;
    Ok(path)
}

/// Reduce an arbitrary name to a safe file stem: alphanumerics, `-` and `_`
/// survive; every other character becomes `_`. Blank input becomes `profile`.
#[must_use]
pub fn sanitize(name: &str) -> String {
    let stem: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if stem.is_empty() {
        "profile".to_string()
    } else {
        stem
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_makes_safe_stems() {
        assert_eq!(sanitize("My Setup"), "My_Setup");
        assert_eq!(sanitize("work/laptop"), "work_laptop");
        assert_eq!(sanitize("a.b.c"), "a_b_c");
        assert_eq!(sanitize("keep-me_1"), "keep-me_1");
    }

    #[test]
    fn sanitize_blank_falls_back() {
        assert_eq!(sanitize(""), "profile");
        assert_eq!(sanitize("   "), "profile");
        assert_eq!(sanitize("///"), "___"); // non-blank, so kept (just replaced)
    }
}
