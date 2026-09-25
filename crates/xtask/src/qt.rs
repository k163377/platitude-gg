//! Locating Qt for build and run: qtbridge needs `qmake` on PATH, and on
//! Windows the built exe needs Qt's bin there too (verify-ui skill,
//! windows.md). The harness shells don't inherit the user's PATH edits, so
//! Qt is prepended when qmake isn't reachable.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[cfg(windows)]
const QMAKE_NAMES: [&str; 1] = ["qmake.exe"];
#[cfg(not(windows))]
const QMAKE_NAMES: [&str; 2] = ["qmake6", "qmake"];

/// The PATH child processes (cargo, the app) should run with.
pub fn path_with_qt() -> Result<OsString, String> {
    let current = std::env::var_os("PATH").unwrap_or_default();
    if qmake_on(&current) {
        return Ok(current);
    }
    let bin = find_qt_bin().ok_or_else(|| {
        format!(
            "qmake not found on PATH{}. Add the Qt bin directory to PATH \
             or set QT_BIN to it.",
            if cfg!(windows) {
                " and no Qt installation under C:\\Qt"
            } else {
                ""
            }
        )
    })?;
    let mut parts = vec![bin];
    parts.extend(std::env::split_paths(&current));
    std::env::join_paths(parts).map_err(|e| format!("rebuilding PATH failed: {e}"))
}

fn qmake_on(path: &OsString) -> bool {
    std::env::split_paths(path).any(|dir| has_qmake(&dir))
}

fn has_qmake(dir: &Path) -> bool {
    QMAKE_NAMES.iter().any(|name| dir.join(name).is_file())
}

/// A Qt bin directory: `QT_BIN`, then the well-known install locations.
fn find_qt_bin() -> Option<PathBuf> {
    if let Some(explicit) = std::env::var_os("QT_BIN") {
        let dir = PathBuf::from(explicit);
        if has_qmake(&dir) {
            return Some(dir);
        }
    }
    if cfg!(windows) {
        // aqtinstall layout: C:\Qt\<version>\<toolchain>\bin. Prefer the
        // newest version (numeric compare — "6.10" sorts after "6.9").
        let mut candidates: Vec<(Vec<u32>, PathBuf)> = Vec::new();
        for version_dir in read_dirs(Path::new("C:\\Qt")) {
            let version = numeric_version(&version_dir);
            if version.is_empty() {
                continue;
            }
            for toolchain in read_dirs(&version_dir) {
                let bin = toolchain.join("bin");
                if has_qmake(&bin) {
                    candidates.push((version.clone(), bin));
                }
            }
        }
        candidates.sort();
        return candidates.pop().map(|(_, bin)| bin);
    }
    None
}

fn read_dirs(parent: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(parent) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect()
}

/// `"6.10.3"` → `[6, 10, 3]`; non-numeric names (e.g. "Tools") → empty.
fn numeric_version(dir: &Path) -> Vec<u32> {
    let Some(name) = dir.file_name().and_then(|n| n.to_str()) else {
        return Vec::new();
    };
    let parts: Vec<u32> = name.split('.').filter_map(|p| p.parse().ok()).collect();
    if parts.len() == name.split('.').count() {
        parts
    } else {
        Vec::new()
    }
}
