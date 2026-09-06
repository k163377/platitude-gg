//! Which binary `git` is: the one PATH names, unless PATH names the
//! launcher Git for Windows puts there, in which case the git it launches.
//!
//! `C:\Program Files\Git\cmd\git.exe` — the one entry the installer adds
//! to PATH — is a 46KB launcher: it sets up an environment and spawns
//! `mingw64\bin\git.exe`, which does the work. Two processes per command,
//! and the first is pure overhead on a machine where a process is most
//! of what a command costs (measured on a warm 24-thread desktop: `git
//! --version` 22.7ms through the launcher, 10.3ms without; the details
//! pane's `git show` 19.0ms against 10.7ms). The real binary needs none
//! of the launcher's setup: it puts its own `libexec/git-core`,
//! `mingw64/bin` and `usr/bin` on the PATH of what it spawns, so hooks,
//! `!` aliases, ssh and gpg are found, and it derives `HOME` and finds
//! the system config the same way (measured, with a PATH stripped of
//! every Git directory and no `HOME`).
//!
//! The search walks the directories `Command::new("git")` would, in the
//! order it would: an earlier git that is not the launcher wins the way
//! it always did, and only PATH's own answer is looked behind.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The program [`super::GitExecutor::new`] spawns, as a path with
/// somewhere in it — what a screen showing "the git this app runs" has to
/// put in front of a reader.
///
/// **Read out, never run.** [`default_program`] is what is spawned; this
/// answers the same question one step further along, resolving the bare
/// name it may return against the same directories in the same order. A
/// reading of what the OS would do rather than a promise: either platform
/// can have the file replaced between this and the next spawn, and it
/// costs a wrong path on screen and nothing else. Falls back to the bare
/// name where nothing along the search answers.
pub fn default_program_path() -> OsString {
    let named = default_program();
    if Path::new(&named).components().count() > 1 {
        return named;
    }
    // The file name, not the program name: the bare `git` above is what is
    // *spawned*, and what a spawn looks for on Windows carries the
    // extension the same way `behind_launcher` writes it.
    let file = if cfg!(windows) { "git.exe" } else { "git" };
    search_dirs()
        .into_iter()
        .find_map(|dir| {
            let candidate = dir.join(file);
            candidate.is_file().then_some(candidate)
        })
        .map_or(named, PathBuf::into_os_string)
}

/// Whether two paths name the same program — the same binary, not the
/// same spelling. One of these comes from a settings file and the other
/// from a chooser, and a reader who picked the git already running must
/// not be told they picked a different one.
///
/// Each side is read as what would be spawned for it ([`spawnable`]: the
/// `.exe` a spawn adds, and the git behind the launcher) and then as the
/// file itself where there is one to resolve — a link and its target, two
/// spellings of one directory — and as its levelled text where there is
/// not: a path that names nothing can only be compared as written.
pub fn same_program(one: &Path, two: &Path) -> bool {
    identity(&spawnable(one)) == identity(&spawnable(two))
}

/// One spelling for one program: the resolved path of the file, or the
/// levelled text of a path that names no file.
fn identity(path: &Path) -> String {
    match std::fs::canonicalize(path) {
        Ok(real) => levelled(&real),
        Err(_) => levelled(path),
    }
}

/// Separators forward and, on Windows, case folded: the two things a
/// spelling can differ in without naming another file.
fn levelled(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        text.to_lowercase()
    } else {
        text
    }
}

/// The file a spawn opens for `path`: on Windows a name with no extension
/// runs the `.exe` beside it, so the one with the extension is the file.
/// `exists` answers whether a path is a file.
fn as_spawned(path: &Path, exists: impl Fn(&Path) -> bool) -> PathBuf {
    if cfg!(windows) && path.extension().is_none() {
        let exe = path.with_extension("exe");
        if exists(&exe) {
            return exe;
        }
    }
    path.to_path_buf()
}

/// The program to spawn for a git a reader named: the path itself, unless
/// it is the launcher Git for Windows puts on PATH and the git it launches
/// stands beside it — then that one, for the reason [`default_program`]
/// looks behind the launcher on PATH. A chooser opened on a Git for
/// Windows install lands on `cmd\git.exe` as naturally as PATH does, and
/// a path taken as given would put the launcher's second process back on
/// every command.
///
/// **The launcher is looked for under the name a spawn opens**, not the
/// one typed: `cmd\git` with no extension is the launcher as much as
/// `cmd\git.exe` is, and taken as typed it would be spawned as itself.
pub fn spawnable(named: &Path) -> PathBuf {
    spawnable_among(named, Path::is_file)
}

/// [`spawnable`] against `exists`, so the launcher rule can be asked
/// without a Git for Windows install to ask it of.
fn spawnable_among(named: &Path, exists: impl Fn(&Path) -> bool) -> PathBuf {
    let named = as_spawned(named, &exists);
    let is_git_exe = named
        .file_name()
        .is_some_and(|file| file.eq_ignore_ascii_case("git.exe"));
    match named.parent() {
        Some(dir) if is_git_exe => behind_launcher([dir.to_path_buf()], exists).unwrap_or(named),
        _ => named,
    }
}

/// The program [`super::GitExecutor::new`] spawns.
pub(super) fn default_program() -> OsString {
    let behind = cfg!(windows)
        .then(search_dirs)
        .and_then(|dirs| behind_launcher(dirs, Path::is_file));
    match behind {
        Some(real) => {
            tracing::info!(program = %real.display(), "git launcher on PATH bypassed");
            real.into_os_string()
        }
        None => OsString::from("git"),
    }
}

/// The directories a bare `git` is resolved against, in the order std's
/// `Command` walks them on Windows: the application's own directory, the
/// system and Windows directories, then PATH.
fn search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(mut app) = std::env::current_exe() {
        app.pop();
        dirs.push(app);
    }
    if let Some(root) = std::env::var_os("SystemRoot") {
        dirs.push(Path::new(&root).join("System32"));
        dirs.push(PathBuf::from(root));
    }
    if let Some(path) = std::env::var_os("PATH") {
        dirs.extend(std::env::split_paths(&path).filter(|p| !p.as_os_str().is_empty()));
    }
    dirs
}

/// The bin directories a Git for Windows install keeps its real git in,
/// by toolchain: x64, arm64, x86.
const REAL_GIT_DIRS: [&str; 3] = ["mingw64", "clangarm64", "mingw32"];

/// The git behind the launcher, when the first `git.exe` along `dirs` is
/// the launcher — `None` when it is anything else, or when nothing stands
/// behind it. `exists` answers whether a path is a file.
fn behind_launcher(
    dirs: impl IntoIterator<Item = PathBuf>,
    exists: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    let found = dirs
        .into_iter()
        .map(|dir| dir.join("git.exe"))
        .find(|candidate| exists(candidate.as_path()))?;
    let launcher_dir = found.parent()?;
    let is_launcher = launcher_dir
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("cmd"));
    if !is_launcher {
        return None;
    }
    let install = launcher_dir.parent()?;
    REAL_GIT_DIRS
        .iter()
        .map(|arch| install.join(arch).join("bin").join("git.exe"))
        .find(|candidate| exists(candidate.as_path()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn among<const N: usize>(files: [&str; N]) -> impl Fn(&Path) -> bool {
        let files: Vec<PathBuf> = files.iter().map(PathBuf::from).collect();
        move |p| files.iter().any(|f| f == p)
    }

    fn dirs<const N: usize>(dirs: [&str; N]) -> Vec<PathBuf> {
        dirs.iter().map(PathBuf::from).collect()
    }

    /// Two spellings of one program are one program: the settings file and
    /// a chooser write paths differently, and a reader who picked the git
    /// already running must not be told they picked another one. Paths
    /// that name no file — none of these exist — are compared as written.
    #[test]
    fn separators_and_windows_case_do_not_make_two_programs() {
        let slashed = Path::new("C:/Nowhere/Git/cmd/git.exe");
        assert!(same_program(
            slashed,
            Path::new(r"C:\Nowhere\Git\cmd\git.exe")
        ));
        assert_eq!(
            same_program(slashed, Path::new(r"c:\nowhere\git\cmd\GIT.EXE")),
            cfg!(windows),
            "case is Windows's to ignore and nobody else's"
        );
        // Two files that are not there to look behind stay two spellings.
        assert!(!same_program(
            slashed,
            Path::new("C:/Nowhere/Git/mingw64/bin/git.exe")
        ));
    }

    /// A chooser opened on a Git for Windows install lands on the launcher
    /// as naturally as PATH does, and the launcher is not the program: what
    /// is spawned for it is the git behind it, and the two spellings name
    /// one program.
    #[test]
    fn the_launcher_a_reader_names_spawns_the_git_behind_it() {
        let launcher = Path::new("C:/Program Files/Git/cmd/git.exe");
        let real = Path::new("C:/Program Files/Git/mingw64/bin/git.exe");
        let install = among([
            "C:/Program Files/Git/cmd/git.exe",
            "C:/Program Files/Git/mingw64/bin/git.exe",
        ]);
        assert_eq!(spawnable_among(launcher, &install), real);
        // Named outright, the real git is taken as it is; so is a git in a
        // `cmd` directory with nothing behind it, and a program that is not
        // spelled `git.exe` at all.
        assert_eq!(spawnable_among(real, &install), real);
        let lone = Path::new("D:/tools/cmd/git.exe");
        assert_eq!(spawnable_among(lone, among(["D:/tools/cmd/git.exe"])), lone);
        let other = Path::new("C:/Program Files/Git/cmd/git-bash.exe");
        assert_eq!(spawnable_among(other, &install), other);
    }

    /// With the files there to resolve, the launcher and the git behind it
    /// are one program however either is spelled.
    #[test]
    fn a_launcher_on_disk_and_the_git_behind_it_are_one_program() {
        let dir = tempfile::tempdir().expect("tempdir");
        let launcher = dir.path().join("cmd").join("git.exe");
        let real = dir.path().join("mingw64").join("bin").join("git.exe");
        for file in [&launcher, &real] {
            std::fs::create_dir_all(file.parent().expect("a directory")).expect("mkdir");
            std::fs::write(file, b"").expect("a file");
        }
        assert!(same_program(&launcher, &real));
        assert!(same_program(&real, &launcher));
        let elsewhere = dir.path().join("other").join("git.exe");
        std::fs::create_dir_all(elsewhere.parent().expect("a directory")).expect("mkdir");
        std::fs::write(&elsewhere, b"").expect("a file");
        assert!(!same_program(&launcher, &elsewhere));
    }

    /// A link and its target run one binary, so a reader who picked either
    /// picked the git already running.
    #[cfg(unix)]
    #[test]
    fn a_link_and_its_target_are_one_program() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("git-2.55");
        std::fs::write(&target, b"").expect("a file");
        let link = dir.path().join("git");
        std::os::unix::fs::symlink(&target, &link).expect("symlink");
        assert!(same_program(&link, &target));
    }

    /// A name typed without its extension runs the `.exe` beside it, so
    /// the two spellings are one program.
    #[cfg(windows)]
    #[test]
    fn the_extension_a_spawn_adds_does_not_make_a_second_program() {
        let dir = tempfile::tempdir().expect("tempdir");
        let exe = dir.path().join("git.exe");
        std::fs::write(&exe, b"").expect("a file");
        assert!(same_program(&dir.path().join("git"), &exe));
    }

    /// The launcher typed without its extension is the launcher: what is
    /// spawned for it is the git behind it, the same as for `cmd\git.exe`,
    /// and the two are one program with the git already running.
    #[cfg(windows)]
    #[test]
    fn the_launcher_typed_without_its_extension_is_looked_behind_too() {
        let real = Path::new("C:/Program Files/Git/mingw64/bin/git.exe");
        let install = among([
            "C:/Program Files/Git/cmd/git.exe",
            "C:/Program Files/Git/mingw64/bin/git.exe",
        ]);
        assert_eq!(
            spawnable_among(Path::new("C:/Program Files/Git/cmd/git"), &install),
            real
        );
        let dir = tempfile::tempdir().expect("tempdir");
        let launcher = dir.path().join("cmd").join("git.exe");
        let behind = dir.path().join("mingw64").join("bin").join("git.exe");
        for file in [&launcher, &behind] {
            std::fs::create_dir_all(file.parent().expect("a directory")).expect("mkdir");
            std::fs::write(file, b"").expect("a file");
        }
        assert!(same_program(&dir.path().join("cmd").join("git"), &behind));
    }

    /// The one the settings screen shows behind an empty box: wherever the
    /// name resolves, it resolves to a file with somewhere in it.
    #[test]
    fn the_default_program_resolves_to_a_file() {
        let found = PathBuf::from(default_program_path());
        assert!(found.is_file(), "dev/CI machines must have git: {found:?}");
        assert!(
            found.components().count() > 1,
            "resolved to somewhere, not back to the bare name: {found:?}"
        );
    }

    #[test]
    fn the_launcher_on_path_gives_way_to_the_git_behind_it() {
        let real = behind_launcher(
            dirs(["C:/Windows/System32", "C:/Program Files/Git/cmd"]),
            among([
                "C:/Program Files/Git/cmd/git.exe",
                "C:/Program Files/Git/mingw64/bin/git.exe",
            ]),
        );
        assert_eq!(
            real,
            Some(PathBuf::from("C:/Program Files/Git/mingw64/bin/git.exe"))
        );
    }

    #[test]
    fn the_arm64_install_keeps_its_git_under_another_name() {
        let real = behind_launcher(
            dirs(["C:/Program Files/Git/cmd"]),
            among([
                "C:/Program Files/Git/cmd/git.exe",
                "C:/Program Files/Git/clangarm64/bin/git.exe",
            ]),
        );
        assert_eq!(
            real,
            Some(PathBuf::from("C:/Program Files/Git/clangarm64/bin/git.exe"))
        );
    }

    #[test]
    fn a_git_that_is_not_the_launcher_is_left_to_the_path() {
        // Git Bash puts mingw64/bin ahead of cmd: the first git found is
        // the real one already, and nothing is swapped.
        let real = behind_launcher(
            dirs([
                "C:/Program Files/Git/mingw64/bin",
                "C:/Program Files/Git/cmd",
            ]),
            among([
                "C:/Program Files/Git/mingw64/bin/git.exe",
                "C:/Program Files/Git/cmd/git.exe",
            ]),
        );
        assert_eq!(real, None);
    }

    #[test]
    fn an_earlier_git_wins_over_a_later_launcher() {
        // A git beside the application, or anywhere ahead of the launcher
        // on PATH, is the one `Command` would have run: it still is.
        let real = behind_launcher(
            dirs(["D:/app", "C:/Program Files/Git/cmd"]),
            among([
                "D:/app/git.exe",
                "C:/Program Files/Git/cmd/git.exe",
                "C:/Program Files/Git/mingw64/bin/git.exe",
            ]),
        );
        assert_eq!(real, None);
    }

    #[test]
    fn a_launcher_with_nothing_behind_it_is_left_alone() {
        let real = behind_launcher(
            dirs(["C:/Program Files/Git/cmd"]),
            among(["C:/Program Files/Git/cmd/git.exe"]),
        );
        assert_eq!(real, None);
    }

    #[test]
    fn no_git_anywhere_is_left_to_the_path_too() {
        let real = behind_launcher(dirs(["C:/Windows"]), among([]));
        assert_eq!(real, None);
    }

    /// On this host, whatever it is: the walk over the real directories
    /// answers what the search says it should, and never a file that is
    /// not there.
    #[test]
    fn the_default_is_git_itself_or_the_binary_the_walk_found() {
        let expected = cfg!(windows)
            .then(search_dirs)
            .and_then(|dirs| behind_launcher(dirs, Path::is_file));
        let program = default_program();
        match expected {
            Some(real) => {
                assert!(real.is_file(), "{}", real.display());
                assert_eq!(program, real.into_os_string());
            }
            None => assert_eq!(program, "git"),
        }
    }

    #[test]
    fn the_launcher_directory_is_matched_without_regard_to_case() {
        let real = behind_launcher(
            dirs(["C:/Program Files/Git/CMD"]),
            among([
                "C:/Program Files/Git/CMD/git.exe",
                "C:/Program Files/Git/mingw64/bin/git.exe",
            ]),
        );
        assert_eq!(
            real,
            Some(PathBuf::from("C:/Program Files/Git/mingw64/bin/git.exe"))
        );
    }
}
