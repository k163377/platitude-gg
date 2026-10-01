//! Locating Qt for build and run: qtbridge needs `qmake` on PATH, and on
//! Windows the built exe needs Qt's bin there too (verify-ui skill,
//! windows.md). The harness shells don't inherit the user's PATH edits, so
//! Qt is prepended to the PATH children get.
//!
//! The Qt is the version CI pins (`QT_VERSION:` in ci.yml) wherever this
//! machine has it: a PATH naming another Qt — the user's own, from before an
//! update — would build half a tree against one version's headers and link
//! it against the other's libraries, and cargo does not notice the switch.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[cfg(windows)]
const QMAKE_NAMES: [&str; 1] = ["qmake.exe"];
#[cfg(not(windows))]
const QMAKE_NAMES: [&str; 2] = ["qmake6", "qmake"];

/// The PATH child processes (cargo, the app) should run with: `QT_BIN`,
/// then the pinned version's install (aqtinstall's layout under `C:\Qt`),
/// then whatever qmake PATH already names (CI and Linux, where PATH is set
/// to the pinned Qt by whoever installed it).
pub fn path_with_qt() -> Result<OsString, String> {
    let current = std::env::var_os("PATH").unwrap_or_default();
    let pinned = pinned_here();
    let bin = explicit_qt_bin().or_else(|| pinned.as_deref().and_then(installed_bin));
    if let Some(bin) = bin {
        let mut parts = vec![bin];
        parts.extend(std::env::split_paths(&current));
        return std::env::join_paths(parts).map_err(|e| format!("rebuilding PATH failed: {e}"));
    }
    if let Some(dir) = qmake_dir_on(&current) {
        // An install that names its version (aqtinstall's layout, also CI's)
        // has to be the pinned one.
        if let (Some(pinned), Some(found)) = (pinned.as_deref(), version_of_bin(&dir))
            && pinned != found
        {
            return Err(format!(
                "the qmake on PATH is Qt {found} ({}), and ci.yml pins {pinned}: install \
                 {pinned} or set QT_BIN to its bin directory",
                dir.display()
            ));
        }
        return Ok(current);
    }
    Err(match (cfg!(windows), pinned) {
        (true, Some(version)) => format!(
            "Qt {version} (ci.yml's QT_VERSION) is not under C:\\Qt and qmake is not on \
             PATH. Install it with aqtinstall (`aqt install-qt --base \
             https://download.qt.io/ windows desktop {version} win64_msvc2022_64 -O \
             C:\\Qt`), or set QT_BIN to its bin directory."
        ),
        _ => "qmake not found on PATH. Add the Qt bin directory to PATH or set QT_BIN \
              to it."
            .to_string(),
    })
}

/// The Qt version CI's workflow pins (`QT_VERSION:`) — the one place it
/// is pinned, for CI and the container alike.
pub(crate) fn pinned_in(workflow: &str) -> Option<String> {
    workflow
        .lines()
        .filter_map(|line| line.trim().strip_prefix("QT_VERSION:"))
        .map(|value| value.trim().trim_matches('"').trim_matches('\''))
        .find(|value| !value.is_empty())
        .map(str::to_string)
}

/// The version this tree pins; `None` where the workflow cannot be read.
fn pinned_here() -> Option<String> {
    let workflow = crate::tree::workspace_root()
        .join(".github")
        .join("workflows")
        .join("ci.yml");
    pinned_in(&std::fs::read_to_string(workflow).ok()?)
}

fn qmake_dir_on(path: &OsString) -> Option<PathBuf> {
    std::env::split_paths(path).find(|dir| has_qmake(dir))
}

/// `<version>/<toolchain>/bin` → `<version>`, for a directory name made of
/// numbers and dots; `None` for a layout that does not say.
fn version_of_bin(bin: &Path) -> Option<String> {
    let name = bin.parent()?.parent()?.file_name()?.to_str()?;
    let numeric = !name.is_empty() && name.split('.').all(|part| part.parse::<u32>().is_ok());
    numeric.then(|| name.to_string())
}

fn has_qmake(dir: &Path) -> bool {
    QMAKE_NAMES.iter().any(|name| dir.join(name).is_file())
}

fn explicit_qt_bin() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os("QT_BIN")?);
    has_qmake(&dir).then_some(dir)
}

/// `version`'s bin directory in aqtinstall's layout, `C:\Qt\<version>\<toolchain>\bin`
/// (the one toolchain this project installs), on Windows; nowhere else has
/// a well-known location.
fn installed_bin(version: &str) -> Option<PathBuf> {
    if !cfg!(windows) {
        return None;
    }
    bin_under(&Path::new("C:\\Qt").join(version))
}

fn bin_under(version_dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(version_dir).ok()?;
    entries
        .flatten()
        .map(|e| e.path().join("bin"))
        .find(|bin| has_qmake(bin))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pin_is_the_first_qt_version_line() {
        let workflow = "env:\n  QT_VERSION: \"6.12.0\"\n  AQT_SOURCE: \"x\"\njobs:\n";
        assert_eq!(pinned_in(workflow).as_deref(), Some("6.12.0"));
        assert_eq!(pinned_in("env:\n  QT_VERSION: ''\n"), None);
        assert_eq!(pinned_in("jobs:\n"), None);
    }

    #[test]
    fn a_bin_in_the_versioned_layout_names_its_version() {
        let bin = Path::new("C:\\Qt").join("6.10.3").join("msvc2022_64").join("bin");
        assert_eq!(version_of_bin(&bin).as_deref(), Some("6.10.3"));
        let ci = Path::new("/tmp/qt/6.12.0/gcc_64/bin");
        assert_eq!(version_of_bin(ci).as_deref(), Some("6.12.0"));
        assert_eq!(version_of_bin(Path::new("/usr/lib/qt6/bin")), None);
        assert_eq!(version_of_bin(Path::new("bin")), None);
    }

    #[test]
    fn a_version_directory_answers_with_the_toolchain_that_has_qmake() {
        let root = std::env::temp_dir().join(format!("pgg-qt-bin-{}", std::process::id()));
        let empty = root.join("6.12.0").join("Src");
        let toolchain = root.join("6.12.0").join("msvc2022_64").join("bin");
        std::fs::create_dir_all(&empty).expect("dir");
        std::fs::create_dir_all(&toolchain).expect("dir");
        std::fs::write(toolchain.join(QMAKE_NAMES[0]), b"").expect("qmake");
        assert_eq!(bin_under(&root.join("6.12.0")), Some(toolchain));
        assert_eq!(bin_under(&root.join("6.11.3")), None, "not installed");
        std::fs::remove_dir_all(&root).expect("clean up");
    }
}
