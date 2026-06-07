//! Persistent application settings under the XDG config dir
//! (`$XDG_CONFIG_HOME/hyprconf/settings.toml`): theme, last output format,
//! window size and recent files. UI-free and serde-(de)serializable.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

const MAX_RECENTS: usize = 8;

/// Persisted app settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The selected theme's display name (e.g. `Catppuccin Mocha`).
    pub theme: String,
    /// The last chosen output format (`conf` or `lua`).
    pub last_format: String,
    /// Last window width.
    pub window_width: f32,
    /// Last window height.
    pub window_height: f32,
    /// Most-recently-opened config paths (newest first).
    pub recent_files: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "Catppuccin Mocha".to_string(),
            last_format: "conf".to_string(),
            window_width: 1120.0,
            window_height: 800.0,
            recent_files: Vec::new(),
        }
    }
}

impl Settings {
    /// Load settings, falling back to defaults on any error.
    #[must_use]
    pub fn load() -> Self {
        config_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|text| toml::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Persist settings (best effort; non-critical, so a plain write — no fsync —
    /// keeps frequent saves like window-resize cheap).
    pub fn save(&self) {
        let Some(path) = config_path() else {
            return;
        };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(text) = toml::to_string_pretty(self) {
            let _ = std::fs::write(&path, text);
        }
    }

    /// Record a recently-opened file (deduped, newest first, capped).
    pub fn add_recent(&mut self, path: &str) {
        self.recent_files.retain(|p| p != path);
        self.recent_files.insert(0, path.to_string());
        self.recent_files.truncate(MAX_RECENTS);
    }
}

/// `$XDG_CONFIG_HOME/hyprconf/settings.toml`.
fn config_path() -> Option<PathBuf> {
    directories::BaseDirs::new()
        .map(|dirs| dirs.config_dir().join("hyprconf").join("settings.toml"))
}

/// `$XDG_DATA_HOME/hyprconf` (for profiles).
#[must_use]
pub fn data_dir() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|dirs| dirs.data_dir().join("hyprconf"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_toml() {
        let mut settings = Settings {
            theme: "Tokyo Night".to_string(),
            last_format: "lua".to_string(),
            window_width: 1300.0,
            ..Settings::default()
        };
        settings.add_recent("/a/hyprland.conf");

        let text = toml::to_string_pretty(&settings).unwrap();
        let back: Settings = toml::from_str(&text).unwrap();
        assert_eq!(settings, back);
    }

    #[test]
    fn recent_files_dedupe_and_cap() {
        let mut s = Settings::default();
        for i in 0..12 {
            s.add_recent(&format!("/f{i}"));
        }
        assert_eq!(s.recent_files.len(), MAX_RECENTS);
        assert_eq!(s.recent_files[0], "/f11"); // newest first

        s.add_recent("/f11"); // already present -> moves to front, no duplicate
        assert_eq!(s.recent_files[0], "/f11");
        assert_eq!(s.recent_files.iter().filter(|p| *p == "/f11").count(), 1);
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        // `#[serde(default)]` lets a partial file load.
        let partial: Settings = toml::from_str("theme = \"Nord\"\n").unwrap();
        assert_eq!(partial.theme, "Nord");
        assert_eq!(partial.last_format, "conf"); // default
    }
}
