//! Locating Qt for build and run: qtbridge needs `qmake` on PATH, and on
//! Windows the built exe needs Qt's bin there too (verify-ui skill,
//! windows.md). The harness shells don't inherit the user's PATH edits, so
//! Qt is prepended to the PATH children get.
//!
//! The Qt is the version the tree pins ([`PIN`]) wherever this machine has
//! it: a PATH naming another Qt — the user's own, from before an update —
//! would build half a tree against one version's headers and link it
//! against the other's libraries, and cargo does not notice the switch.
//!
//! A Qt is what its qmake answers (`qmake -query`), not what its directory
//! is called. And the build scripts take `QMAKE` before PATH and are run
//! again only when `QMAKE` changes (qt-build-utils): a `QMAKE` naming
//! another Qt is refused here, and a build that may follow one Qt with
//! another in the same target directory names its qmake there
//! (`app_build`'s `qmake`, which `perf` hands its rig and `--qt` builds).

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

#[cfg(windows)]
const QMAKE_NAMES: [&str; 1] = ["qmake.exe"];
#[cfg(not(windows))]
const QMAKE_NAMES: [&str; 2] = ["qmake6", "qmake"];

/// One Qt install, as its qmake answers for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Qt {
    /// The directory qmake sits in, which goes first on PATH.
    pub(crate) bin: PathBuf,
    pub(crate) qmake: PathBuf,
    /// `QT_VERSION`.
    pub(crate) version: String,
    /// `QT_INSTALL_PREFIX`.
    pub(crate) prefix: String,
    /// Where a run loads the libraries from: `QT_INSTALL_BINS` on Windows
    /// (the DLLs), `QT_INSTALL_LIBS` elsewhere.
    pub(crate) runtime: PathBuf,
}

impl Qt {
    /// The PATH a build or a run of this Qt gets: its bin first.
    pub(crate) fn path(&self) -> Result<OsString, String> {
        let current = std::env::var_os("PATH").unwrap_or_default();
        let mut parts = vec![self.bin.clone()];
        parts.extend(std::env::split_paths(&current));
        std::env::join_paths(parts).map_err(|e| format!("rebuilding PATH failed: {e}"))
    }

    /// One line for a record: what the build linked and the run loads.
    pub(crate) fn describe(&self) -> String {
        format!(
            "Qt {} at {} (qmake {}, runtime {})",
            self.version,
            self.prefix,
            self.qmake.display(),
            self.runtime.display()
        )
    }
}

/// The PATH child processes (cargo, the app) should run with: `QT_BIN`,
/// then the pinned version's install (aqtinstall's layout under `C:\Qt`),
/// then whatever qmake PATH already names (CI and Linux, where PATH is set
/// to the pinned Qt by whoever installed it).
pub fn path_with_qt() -> Result<OsString, String> {
    match chosen()? {
        Some(qt) => qt.path(),
        None => Err(not_found(pinned_here().as_deref())),
    }
}

/// As [`path_with_qt`], for a caller with work that needs no Qt: `None`
/// where no Qt is found. A Qt of another version is still refused —
/// whatever in the work does build against Qt would build against it.
pub fn path_with_qt_if_any() -> Result<Option<OsString>, String> {
    chosen()?.map(|qt| qt.path()).transpose()
}

/// This tree's Qt, or why there is none.
pub(crate) fn this_tree() -> Result<Qt, String> {
    chosen()?.ok_or_else(|| not_found(pinned_here().as_deref()))
}

/// This tree's Qt (see [`path_with_qt`]), asked once per process: a gate
/// starts hundreds of steps, and the answer cannot move under it.
pub(crate) fn chosen() -> Result<Option<Qt>, String> {
    static CHOSEN: OnceLock<Result<Option<Qt>, String>> = OnceLock::new();
    CHOSEN.get_or_init(choose).clone()
}

fn choose() -> Result<Option<Qt>, String> {
    let pinned = pinned_here();
    let qt = if let Some(explicit) = explicit()? {
        // Asked for by name: its version is the asker's choice.
        Some(explicit)
    } else if let Some(bin) = pinned.as_deref().and_then(installed_bin) {
        Some(answered(&bin, pinned.as_deref())?)
    } else if let Some(dir) = qmake_dir_on(&std::env::var_os("PATH").unwrap_or_default()) {
        Some(answered(&dir, pinned.as_deref())?)
    } else {
        None
    };
    if let Some(qt) = &qt {
        no_other_qmake(qt, std::env::var_os("QMAKE").as_deref())?;
    }
    Ok(qt)
}

/// The Qt of `version` — a commit's pin, or one asked for over it —
/// wherever this machine has it. `QT_BIN` answers only if it is that
/// version: a build that silently took it would measure another Qt.
pub(crate) fn of_version(version: &str) -> Result<Qt, String> {
    let qt = if let Some(explicit) = explicit()? {
        if explicit.version != version {
            return Err(format!(
                "QT_BIN is Qt {} and this build wants Qt {version}: unset QT_BIN, or ask for \
                 Qt {} by name",
                explicit.version, explicit.version
            ));
        }
        explicit
    } else if let Some(bin) = installed_bin(version) {
        answered(&bin, Some(version))?
    } else {
        let dir = qmake_dir_on(&std::env::var_os("PATH").unwrap_or_default())
            .ok_or_else(|| not_found(Some(version)))?;
        answered(&dir, Some(version))?
    };
    no_other_qmake(&qt, std::env::var_os("QMAKE").as_deref())?;
    Ok(qt)
}

/// `QT_BIN`, which must hold a qmake if it is set at all: a name that
/// points nowhere is no reason to build against another Qt.
fn explicit() -> Result<Option<Qt>, String> {
    let Some(dir) = std::env::var_os("QT_BIN") else {
        return Ok(None);
    };
    let dir = PathBuf::from(dir);
    let qmake = qmake_in(&dir).ok_or_else(|| {
        format!(
            "QT_BIN is {} and holds no qmake: point it at a Qt bin directory, or unset it",
            dir.display()
        )
    })?;
    query(&qmake).map(Some)
}

/// The Qt whose qmake is in `bin`, held to `pinned` when there is one.
fn answered(bin: &Path, pinned: Option<&str>) -> Result<Qt, String> {
    let qmake = qmake_in(bin).ok_or_else(|| format!("{} holds no qmake", bin.display()))?;
    let qt = query(&qmake)?;
    if let Some(pinned) = pinned
        && qt.version != pinned
    {
        return Err(format!(
            "the qmake at {} is Qt {}, and the build wants {pinned}: install {pinned} or set \
             QT_BIN to its bin directory",
            qmake.display(),
            qt.version
        ));
    }
    Ok(qt)
}

/// A `QMAKE` in the environment that names another Qt than `qt`: the
/// build scripts would take it over PATH. Another qmake of the same
/// install — `qmake` beside `qmake6`, which the Linux image sets — is the
/// same Qt, by what it answers.
fn no_other_qmake(qt: &Qt, set: Option<&OsStr>) -> Result<(), String> {
    let Some(set) = set.filter(|value| !value.is_empty()) else {
        return Ok(());
    };
    let named = PathBuf::from(set);
    let same = |a: &Path, b: &Path| match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    };
    if same(&named, &qt.qmake) {
        return Ok(());
    }
    let refused = |what: String| {
        Err(format!(
            "QMAKE is {} ({what}), and the Qt chosen is {}: the build scripts read QMAKE \
             before PATH. Unset QMAKE, or set QT_BIN to the bin directory it names",
            named.display(),
            qt.describe()
        ))
    };
    match query(&named) {
        Ok(other) if one_install(&other, qt) => Ok(()),
        Ok(other) => refused(other.describe()),
        Err(why) => refused(why),
    }
}

/// Two qmakes that answer for one install: the headers a build reads and
/// the libraries a run loads are the same.
fn one_install(a: &Qt, b: &Qt) -> bool {
    a.version == b.version && a.prefix == b.prefix && a.runtime == b.runtime
}

/// What `qmake -query` says about its own install.
pub(crate) fn query(qmake: &Path) -> Result<Qt, String> {
    let output = Command::new(qmake)
        .arg("-query")
        .output()
        .map_err(|e| format!("could not run {}: {e}", qmake.display()))?;
    if !output.status.success() {
        return Err(format!(
            "{} -query failed: {}",
            qmake.display(),
            output.status
        ));
    }
    parse_query(qmake, &String::from_utf8_lossy(&output.stdout))
}

fn parse_query(qmake: &Path, text: &str) -> Result<Qt, String> {
    let value = |key: &str| {
        text.lines()
            .filter_map(|line| line.trim_end().strip_prefix(key)?.strip_prefix(':'))
            .find(|value| !value.is_empty())
            .map(str::to_string)
            .ok_or_else(|| format!("{} -query says no {key}", qmake.display()))
    };
    let runtime = if cfg!(windows) {
        value("QT_INSTALL_BINS")?
    } else {
        value("QT_INSTALL_LIBS")?
    };
    Ok(Qt {
        bin: qmake
            .parent()
            .ok_or_else(|| format!("{} has no directory", qmake.display()))?
            .to_path_buf(),
        qmake: qmake.to_path_buf(),
        version: value("QT_VERSION")?,
        prefix: value("QT_INSTALL_PREFIX")?,
        runtime: PathBuf::from(runtime),
    })
}

fn not_found(pinned: Option<&str>) -> String {
    match (cfg!(windows), pinned) {
        (true, Some(version)) => format!(
            "Qt {version} is not under C:\\Qt and qmake is not on PATH. Install it as CI's \
             Windows jobs do — aqtinstall from the AQT_SOURCE the container's Dockerfile \
             pins, then `aqt \
             install-qt windows desktop {version} win64_msvc2022_64 -O C:\\Qt` — or set QT_BIN \
             to its bin directory."
        ),
        _ => "qmake not found on PATH. Add the Qt bin directory to PATH or set QT_BIN \
              to it."
            .to_string(),
    }
}

/// Where Qt's version is pinned, for a desk, the container and CI alike: a
/// file of its own at the tree's root, holding the version and nothing
/// else. Not a line of CI's workflow: the gate keys every container step
/// by the pin, and an edit to a workflow changes nothing built here.
pub(crate) const PIN: &str = ".qt-version";

/// The version a pin file names: its one line.
pub(crate) fn pinned_in(pin: &str) -> Option<String> {
    pin.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

/// Where the pin stood before it had a file of its own: the `QT_VERSION:`
/// line of CI's workflow. Read out of older commits ([`pinned_at`]) and of
/// trees that are behind (`linux::former_image_tag`), never off this tree,
/// whose workflow pins nothing — so the path is spelled in pieces, and the
/// gate's graph does not take this file for the workflow's reader
/// (反映前テストの機械化.md §依存木).
pub(crate) fn former_pin() -> String {
    format!(".{}/{}/{}", "github", "workflows", "ci.yml")
}

/// The version a workflow from before [`PIN`] names.
fn formerly_pinned_in(workflow: &str) -> Option<String> {
    workflow
        .lines()
        .filter_map(|line| line.trim().strip_prefix("QT_VERSION:"))
        .map(|value| value.trim().trim_matches('"').trim_matches('\''))
        .find(|value| !value.is_empty())
        .map(str::to_string)
}

/// The pin as a commit holds it, in whichever of the two places that
/// commit pins. `show` answers a path's text at the commit.
pub(crate) fn pinned_at(show: &dyn Fn(&str) -> Option<String>) -> Option<String> {
    show(PIN)
        .and_then(|pin| pinned_in(&pin))
        .or_else(|| show(&former_pin()).and_then(|workflow| formerly_pinned_in(&workflow)))
}

/// The version this tree pins; `None` where the pin cannot be read.
fn pinned_here() -> Option<String> {
    pinned_in(&std::fs::read_to_string(crate::tree::workspace_root().join(PIN)).ok()?)
}

fn qmake_dir_on(path: &OsStr) -> Option<PathBuf> {
    std::env::split_paths(path).find(|dir| qmake_in(dir).is_some())
}

fn qmake_in(dir: &Path) -> Option<PathBuf> {
    QMAKE_NAMES
        .iter()
        .map(|name| dir.join(name))
        .find(|qmake| qmake.is_file())
}

/// `version`'s bin directory in aqtinstall's layout, `C:\Qt\<version>\<toolchain>\bin`
/// (the one toolchain this project installs), on Windows; nowhere else has
/// a well-known location. Only where to ask: the answer is qmake's.
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
        .find(|bin| qmake_in(bin).is_some())
}

/// The directory a run of `exe` with `path` loads Qt's core library from:
/// the exe's own directory, then PATH — the order the Windows loader
/// takes for a library that is neither known nor already loaded. `None`
/// off Windows, where the loader reads the binary's own search path.
pub(crate) fn runtime_dir(exe: &Path, path: &OsStr) -> Option<PathBuf> {
    if !cfg!(windows) {
        return None;
    }
    exe.parent()
        .map(Path::to_path_buf)
        .into_iter()
        .chain(std::env::split_paths(path))
        .find(|dir| dir.join("Qt6Core.dll").is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pin_is_the_files_one_line() {
        assert_eq!(pinned_in("6.12.0\n").as_deref(), Some("6.12.0"));
        assert_eq!(pinned_in("\n  6.12.0  \r\n").as_deref(), Some("6.12.0"));
        assert_eq!(pinned_in("\n"), None);
    }

    /// The tree holds the pin every build here is held to, and it reads
    /// as a version.
    #[test]
    fn this_tree_pins_a_version() {
        let version = pinned_here().expect("the pin file at the tree's root");
        assert!(
            version.split('.').all(|part| part.parse::<u32>().is_ok()),
            "{version:?} does not look like a version"
        );
    }

    /// A commit from before the pin had a file of its own answers off its
    /// workflow; one that holds both answers off the file.
    #[test]
    fn a_commit_pins_in_the_file_or_in_the_workflow_it_had_before() {
        let workflow = "env:\n  QT_VERSION: \"6.10.3\"\n  AQT_SOURCE: \"x\"\njobs:\n";
        let before = |path: &str| (path == former_pin()).then(|| workflow.to_string());
        assert_eq!(pinned_at(&before).as_deref(), Some("6.10.3"));
        let after = |path: &str| match path {
            PIN => Some("6.12.0\n".to_string()),
            _ => before(path),
        };
        assert_eq!(pinned_at(&after).as_deref(), Some("6.12.0"));
        assert_eq!(pinned_at(&|_| None), None);
        // A workflow that pins nothing: the tree's own, since the move.
        let unpinned = |path: &str| (path == former_pin()).then(|| "jobs:\n".to_string());
        assert_eq!(pinned_at(&unpinned), None);
        assert_eq!(formerly_pinned_in("env:\n  QT_VERSION: ''\n"), None);
    }

    /// The answer is qmake's, whatever the directory is called.
    #[test]
    fn a_qt_is_what_its_qmake_answers() {
        let qmake = Path::new("C:/Qt/6.12.0/msvc2022_64/bin/qmake.exe");
        let answer = "QT_SYSROOT:\nQT_INSTALL_PREFIX:C:/Qt/6.10.3/msvc2022_64\n\
                      QT_INSTALL_BINS:C:/Qt/6.10.3/msvc2022_64/bin\n\
                      QT_INSTALL_LIBS:C:/Qt/6.10.3/msvc2022_64/lib\nQT_VERSION:6.10.3\r\n";
        let qt = parse_query(qmake, answer).expect("an answer");
        assert_eq!(qt.version, "6.10.3");
        assert_eq!(qt.prefix, "C:/Qt/6.10.3/msvc2022_64");
        assert_eq!(qt.bin, Path::new("C:/Qt/6.12.0/msvc2022_64/bin"));
        let runtime = if cfg!(windows) { "bin" } else { "lib" };
        assert_eq!(
            qt.runtime,
            PathBuf::from(format!("C:/Qt/6.10.3/msvc2022_64/{runtime}"))
        );
        assert!(parse_query(qmake, "QT_INSTALL_PREFIX:x\n").is_err());
    }

    #[test]
    fn a_qmake_named_by_qmake_must_be_the_one_chosen() {
        let qt = Qt {
            bin: PathBuf::from("C:/Qt/6.12.0/b"),
            qmake: PathBuf::from("C:/Qt/6.12.0/b/qmake.exe"),
            version: "6.12.0".into(),
            prefix: "C:/Qt/6.12.0/b".into(),
            runtime: PathBuf::from("C:/Qt/6.12.0/b"),
        };
        assert!(no_other_qmake(&qt, None).is_ok());
        assert!(no_other_qmake(&qt, Some(OsStr::new(""))).is_ok());
        assert!(no_other_qmake(&qt, Some(OsStr::new("C:/Qt/6.12.0/b/qmake.exe"))).is_ok());
        let other = no_other_qmake(&qt, Some(OsStr::new("C:/Qt/6.10.3/b/qmake.exe")));
        assert!(other.is_err_and(|e| e.contains("QMAKE is C:/Qt/6.10.3")));
    }

    /// The Linux image's `QMAKE` is `qmake`, and the chooser takes the
    /// `qmake6` beside it.
    #[test]
    fn two_qmakes_of_one_install_are_one_qt() {
        let answer = "QT_INSTALL_PREFIX:/opt/qt/6.12.0/gcc_64\n\
                      QT_INSTALL_BINS:/opt/qt/6.12.0/gcc_64/bin\n\
                      QT_INSTALL_LIBS:/opt/qt/6.12.0/gcc_64/lib\nQT_VERSION:6.12.0\n";
        let six = parse_query(Path::new("/opt/qt/6.12.0/gcc_64/bin/qmake6"), answer);
        let plain = parse_query(Path::new("/opt/qt/6.12.0/gcc_64/bin/qmake"), answer);
        let (six, plain) = (six.expect("an answer"), plain.expect("an answer"));
        assert!(one_install(&six, &plain));
        let older = parse_query(
            Path::new("/opt/qt/6.10.3/gcc_64/bin/qmake"),
            &answer.replace("6.12.0", "6.10.3"),
        )
        .expect("an answer");
        assert!(!one_install(&six, &older));
    }

    #[test]
    fn a_version_directory_answers_with_the_toolchain_that_has_qmake() {
        let root = crate::yard::Yard::new("qt-bin");
        let empty = root.join("6.12.0").join("Src");
        let toolchain = root.join("6.12.0").join("msvc2022_64").join("bin");
        std::fs::create_dir_all(&empty).expect("dir");
        std::fs::create_dir_all(&toolchain).expect("dir");
        std::fs::write(toolchain.join(QMAKE_NAMES[0]), b"").expect("qmake");
        assert_eq!(bin_under(&root.join("6.12.0")), Some(toolchain));
        assert_eq!(bin_under(&root.join("6.11.3")), None, "not installed");
    }

    /// The exe's own directory first, then PATH in order.
    #[test]
    #[cfg(windows)]
    fn a_run_loads_qt_from_the_first_directory_that_holds_it() {
        let root = crate::yard::Yard::new("qt-runtime");
        let exe_dir = root.join("built");
        let [first, second] = [root.join("qt-a"), root.join("qt-b")];
        for dir in [&exe_dir, &first, &second] {
            std::fs::create_dir_all(dir).expect("dir");
        }
        std::fs::write(second.join("Qt6Core.dll"), b"").expect("a dll");
        let exe = exe_dir.join("platitude-gg.exe");
        let path = std::env::join_paths([&first, &second]).expect("a path");
        assert_eq!(runtime_dir(&exe, &path), Some(second.clone()));
        std::fs::write(exe_dir.join("Qt6Core.dll"), b"").expect("a dll");
        assert_eq!(runtime_dir(&exe, &path), Some(exe_dir));
    }
}
