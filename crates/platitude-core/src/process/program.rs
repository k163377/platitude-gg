//! Which binary `git` is: the one PATH names, unless PATH names the
//! launcher Git for Windows puts there, in which case the git it launches.
//!
//! `C:\Program Files\Git\cmd\git.exe` — the installer's PATH entry — is a
//! launcher that spawns `mingw64\bin\git.exe`: a second process on every
//! command, where a process is most of what a command costs
//! (ci/baseline/code-costs-windows-x64.md). The real binary needs none of
//! the launcher's setup: it puts its own `libexec/git-core`, `mingw64/bin`
//! and `usr/bin` on its children's PATH (hooks, `!` aliases, ssh, gpg) and
//! derives `HOME` and the system config itself.
//!
//! The search walks the directories `Command::new("git")` would, in its
//! order: an earlier git that is not the launcher still wins.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The program [`super::GitExecutor::new`] spawns, as a full path — for
/// display only: [`default_program`]'s bare name resolved against the same
/// directories in the same order. The file can change before the next
/// spawn; that costs a wrong path on screen, nothing else. Falls back to
/// the bare name when the search finds nothing.
pub fn default_program_path() -> OsString {
    let named = default_program();
    if Path::new(&named).components().count() > 1 {
        return named;
    }
    let file = if cfg!(windows) { "git.exe" } else { "git" };
    search_dirs()
        .into_iter()
        .find_map(|dir| {
            let candidate = dir.join(file);
            candidate.is_file().then_some(candidate)
        })
        .map_or(named, PathBuf::into_os_string)
}

/// Whether two paths name the same binary. Each side is read as what would
/// be spawned for it ([`spawnable`]), then canonicalized where the file
/// exists (a link and its target, two spellings of one directory); a path
/// that names nothing is compared as levelled text.
pub fn same_program(one: &Path, two: &Path) -> bool {
    identity(&spawnable(one)) == identity(&spawnable(two))
}

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
/// runs the `.exe` beside it.
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
/// it is the Git for Windows launcher with its git beside it — then that
/// one, for the module doc's reason: a chooser lands on `cmd\git.exe` as
/// easily as PATH does. The launcher is matched under the name a spawn
/// opens, so extensionless `cmd\git` is looked behind too.
pub fn spawnable(named: &Path) -> PathBuf {
    spawnable_among(named, Path::is_file)
}

/// [`spawnable`] against `exists`, so tests need no Git for Windows install.
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

/// The directories std's `Command` resolves a bare `git` against on
/// Windows, in its order.
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
/// behind it.
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

    /// None of these paths exist, so they are compared as levelled text.
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
        // Nothing on disk to look behind: still two programs.
        assert!(!same_program(
            slashed,
            Path::new("C:/Nowhere/Git/mingw64/bin/git.exe")
        ));
    }

    #[test]
    fn the_launcher_a_reader_names_spawns_the_git_behind_it() {
        let launcher = Path::new("C:/Program Files/Git/cmd/git.exe");
        let real = Path::new("C:/Program Files/Git/mingw64/bin/git.exe");
        let install = among([
            "C:/Program Files/Git/cmd/git.exe",
            "C:/Program Files/Git/mingw64/bin/git.exe",
        ]);
        assert_eq!(spawnable_among(launcher, &install), real);
        assert_eq!(spawnable_among(real, &install), real);
        let lone = Path::new("D:/tools/cmd/git.exe");
        assert_eq!(spawnable_among(lone, among(["D:/tools/cmd/git.exe"])), lone);
        let other = Path::new("C:/Program Files/Git/cmd/git-bash.exe");
        assert_eq!(spawnable_among(other, &install), other);
    }

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

    #[cfg(windows)]
    #[test]
    fn the_extension_a_spawn_adds_does_not_make_a_second_program() {
        let dir = tempfile::tempdir().expect("tempdir");
        let exe = dir.path().join("git.exe");
        std::fs::write(&exe, b"").expect("a file");
        assert!(same_program(&dir.path().join("git"), &exe));
    }

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

    /// What the settings screen shows behind an empty box.
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
        // Git Bash puts mingw64/bin ahead of cmd.
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
