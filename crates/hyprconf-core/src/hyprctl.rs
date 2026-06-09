// SPDX-License-Identifier: MIT OR Apache-2.0
//! Thin, UI-free wrappers around the `hyprctl` CLI.
//!
//! These run the external binary with [`std::process::Command`]; the GUI invokes
//! them off the UI thread inside an `iced::Task`. Everything degrades gracefully:
//! if `hyprctl` is missing or Hyprland isn't running, calls return an error
//! instead of panicking, and [`detect`] returns `None`.

use std::process::Command;

/// A typed error from a `hyprctl` invocation.
#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum HyprctlError {
    /// `hyprctl` could not be executed (not installed / not in `PATH`).
    #[error("hyprctl is not available: {0}")]
    Unavailable(String),
    /// `hyprctl` ran but reported failure.
    #[error("hyprctl {command} failed: {message}")]
    Failed {
        /// The sub-command attempted.
        command: String,
        /// `hyprctl`'s error output.
        message: String,
    },
}

/// Information about a running Hyprland instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HyprlandInfo {
    /// The semantic version string, e.g. `0.55.2`.
    pub version: String,
    /// The git tag, if reported (e.g. `v0.55.2`).
    pub tag: Option<String>,
}

/// Detect a running Hyprland by parsing `hyprctl version`.
///
/// Returns `None` if `hyprctl` cannot be run or its output is unrecognised.
#[must_use]
pub fn detect() -> Option<HyprlandInfo> {
    let output = Command::new("hyprctl").arg("version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    parse_version(&String::from_utf8_lossy(&output.stdout))
}

/// Parse the first line of `hyprctl version` output.
fn parse_version(text: &str) -> Option<HyprlandInfo> {
    let first = text.lines().next()?;
    // "Hyprland 0.55.2 built from branch ..."
    let version = first.split_whitespace().nth(1)?.trim().to_string();
    if version.is_empty() {
        return None;
    }
    let tag = text.lines().find_map(|line| {
        line.trim()
            .strip_prefix("Tag:")
            .map(|rest| rest.split(',').next().unwrap_or("").trim().to_string())
            .filter(|t| !t.is_empty())
    });
    Some(HyprlandInfo { version, tag })
}

/// Apply a single keyword live: `hyprctl keyword <name> <value>`.
///
/// # Errors
///
/// Returns [`HyprctlError`] if `hyprctl` cannot run or reports failure.
pub fn apply_keyword(name: &str, value: &str) -> Result<String, HyprctlError> {
    run(&["keyword", name, value])
}

/// Trigger a config reload: `hyprctl reload`.
///
/// # Errors
///
/// Returns [`HyprctlError`] if `hyprctl` cannot run or reports failure.
pub fn reload() -> Result<String, HyprctlError> {
    run(&["reload"])
}

fn run(args: &[&str]) -> Result<String, HyprctlError> {
    let output = Command::new("hyprctl")
        .args(args)
        .output()
        .map_err(|e| HyprctlError::Unavailable(e.to_string()))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(HyprctlError::Failed {
            command: args.join(" "),
            message: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        })
    }
}

/// Parse a Hyprland version string `[v]MAJOR.MINOR.PATCH` into a tuple.
#[must_use]
pub fn parse_semver(version: &str) -> Option<(u32, u32, u32)> {
    let core = version.trim().trim_start_matches('v');
    // Stop at the first non-version character (e.g. `-dirty`).
    let core: String = core
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    Some((major, minor, patch))
}

/// Whether `required` is strictly newer than `running` (both `MAJOR.MINOR.PATCH`).
///
/// Returns `None` if either string is unparseable.
#[must_use]
pub fn is_newer(required: &str, running: &str) -> Option<bool> {
    Some(parse_semver(required)? > parse_semver(running)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_real_version_output() {
        let sample = "Hyprland 0.55.2 built from branch v0.55.2 at commit 39d7e2 clean (version: bump).\nDate: ...\nTag: v0.55.2, commits: 7319\n";
        let info = parse_version(sample).unwrap();
        assert_eq!(info.version, "0.55.2");
        assert_eq!(info.tag.as_deref(), Some("v0.55.2"));
    }

    #[test]
    fn unrecognised_output_is_none() {
        assert!(parse_version("").is_none());
        assert!(parse_version("garbage").is_none());
    }

    #[test]
    fn semver_parse_and_compare() {
        assert_eq!(parse_semver("0.55.2"), Some((0, 55, 2)));
        assert_eq!(parse_semver("v0.42.0"), Some((0, 42, 0)));
        assert_eq!(parse_semver("0.50.0-dirty"), Some((0, 50, 0)));

        assert_eq!(is_newer("0.56.0", "0.55.2"), Some(true));
        assert_eq!(is_newer("0.42.0", "0.55.2"), Some(false));
        assert_eq!(is_newer("0.55.2", "0.55.2"), Some(false));
    }
}
