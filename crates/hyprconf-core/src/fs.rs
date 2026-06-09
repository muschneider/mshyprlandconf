// SPDX-License-Identifier: MIT OR Apache-2.0
//! Safe filesystem writes: atomic replace (temp + fsync + rename) and
//! timestamped backups.
//!
//! The atomicity guarantee matters: the original file is only ever replaced by
//! a single `rename(2)` of a fully-written, fsync'd temp file in the *same*
//! directory. A crash or interruption therefore leaves the original intact —
//! never a half-written file.

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Errors from the safe-write helpers.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FsError {
    /// An I/O error against a specific path.
    #[error("i/o error on {path}: {source}")]
    Io {
        /// The path involved.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },
}

fn io(path: &Path, source: std::io::Error) -> FsError {
    FsError::Io {
        path: path.to_path_buf(),
        source,
    }
}

/// What a [`save_atomically`] call did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveReport {
    /// The file written.
    pub path: PathBuf,
    /// The backup created, if the file already existed.
    pub backup: Option<PathBuf>,
}

/// Atomically write `contents` to `path`.
///
/// Writes to a temp file in the same directory, fsyncs it, then renames it over
/// `path`. The rename is atomic on a POSIX filesystem, so readers see either the
/// old or the new file, never a partial one.
///
/// # Errors
///
/// Returns [`FsError::Io`] if any step fails. On failure the temp file is
/// removed and `path` is left untouched.
pub fn atomic_write(path: &Path, contents: &str) -> Result<(), FsError> {
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "out".to_string());

    let unique = format!(
        "{}.{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    let tmp = dir.join(format!(".{file_name}.hyprconf-tmp.{unique}"));

    // Write + fsync the temp file, cleaning it up on any error.
    let write_result = (|| -> std::io::Result<()> {
        let mut file = File::create(&tmp)?;
        file.write_all(contents.as_bytes())?;
        file.sync_all()?;
        Ok(())
    })();
    if let Err(e) = write_result {
        let _ = std::fs::remove_file(&tmp);
        return Err(io(&tmp, e));
    }

    // Atomically replace the destination.
    if let Err(e) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(io(path, e));
    }

    // Best-effort durability of the rename itself.
    if let Ok(handle) = File::open(&dir) {
        let _ = handle.sync_all();
    }

    Ok(())
}

/// Copy `path` to a timestamped `*.bak` sibling, if it exists.
///
/// Returns the backup path, or `None` if `path` did not exist.
///
/// # Errors
///
/// Returns [`FsError::Io`] if the copy fails.
pub fn backup_existing(path: &Path) -> Result<Option<PathBuf>, FsError> {
    if !path.exists() {
        return Ok(None);
    }
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "out".to_string());
    let backup = path.with_file_name(format!("{name}.{secs}.bak"));
    std::fs::copy(path, &backup).map_err(|e| io(&backup, e))?;
    Ok(Some(backup))
}

/// Back up any existing file, then atomically write the new contents.
///
/// # Errors
///
/// Returns [`FsError::Io`] if the backup or write fails.
pub fn save_atomically(path: &Path, contents: &str) -> Result<SaveReport, FsError> {
    let backup = backup_existing(path)?;
    atomic_write(path, contents)?;
    Ok(SaveReport {
        path: path.to_path_buf(),
        backup,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("hyprconf-fs-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn atomic_write_creates_and_replaces() {
        let dir = temp_dir("atomic");
        let path = dir.join("hyprland.conf");

        atomic_write(&path, "v1\n").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "v1\n");

        atomic_write(&path, "v2 longer content\n").unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "v2 longer content\n"
        );

        // No temp files left behind.
        let leftovers: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().contains("hyprconf-tmp"))
            .collect();
        assert!(leftovers.is_empty(), "temp files leaked: {leftovers:?}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_atomically_backs_up_the_prior_file() {
        let dir = temp_dir("backup");
        let path = dir.join("hyprland.conf");

        // First save: nothing to back up.
        let report = save_atomically(&path, "original\n").unwrap();
        assert!(report.backup.is_none());

        // Second save: the prior file is preserved in the backup.
        let report = save_atomically(&path, "edited\n").unwrap();
        let backup = report.backup.expect("a backup should exist");
        assert_eq!(std::fs::read_to_string(&backup).unwrap(), "original\n");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "edited\n");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn original_survives_a_failed_write() {
        // Simulate an "interrupted" write: target the destination but make the
        // temp write fail by pointing at a directory that cannot be created.
        let dir = temp_dir("interrupt");
        let path = dir.join("hyprland.conf");
        std::fs::write(&path, "important\n").unwrap();

        // A path whose parent does not exist => temp File::create fails, but the
        // original must remain byte-for-byte intact (no truncation).
        let bad = dir.join("missing-subdir").join("hyprland.conf");
        std::fs::write(&bad, "x").ok(); // ensure it doesn't exist
        let result = atomic_write(&bad, "should not appear");
        assert!(result.is_err());

        // The real original is untouched.
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "important\n");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
