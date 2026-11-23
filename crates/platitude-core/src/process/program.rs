//! Which binary `git` is: the first one along the directories a spawn of
//! bare `git` searches, spawned as found — whatever stands there.
//!
//! Where that is Git for Windows' launcher (`cmd\git.exe`, the installer's
//! PATH entry), the launcher is what runs, and it starts the real git as a
//! second process: a cost on every command
//! (ci/baseline/code-costs-windows-x64.md). It is not looked behind (user
//! decision — ask before changing): the launcher sets the environment up
//! from the user's variables and the install's `etc\git-bash.config`, and
//! the git behind it, started directly, does not set it up the same way
//! (`%HOME%\bin` lands ahead of the install's own bin directories, and
//! that file's `MSYS` is never read). Skipping it runs a git unlike the
//! `git` a terminal runs, and so does skipping any other wrapper first on
//! PATH.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The program [`super::GitExecutor::new`] spawns, as a full path: the bare
/// `git` resolved against the directories a spawn searches, in their
/// order, as they stand now. The file can change before the next spawn; on
/// screen that costs a wrong path, nothing else. Falls back to the bare
/// name when the search finds nothing.
pub fn default_program_path() -> OsString {
    let file = if cfg!(windows) { "git.exe" } else { "git" };
    search_dirs()
        .into_iter()
        .find_map(|dir| {
            let candidate = dir.join(file);
            candidate.is_file().then_some(candidate)
        })
        .map_or_else(|| OsString::from("git"), PathBuf::into_os_string)
}

/// Whether two paths name the same binary. Each side is read as the file a
/// spawn opens for it ([`as_spawned`]), then canonicalized where the file
/// exists (a link and its target, two spellings of one directory); a path
/// that names nothing is compared as levelled text. A launcher and the git
/// it starts are two binaries.
pub fn same_program(one: &Path, two: &Path) -> bool {
    identity(&as_spawned(one, Path::is_file)) == identity(&as_spawned(two, Path::is_file))
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

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(!same_program(
            slashed,
            Path::new("C:/Nowhere/Git/mingw64/bin/git.exe")
        ));
    }

    /// The launcher a reader picks is the program the run spawns, not the
    /// git it starts: a box naming one while the run is on the other names
    /// another git.
    #[test]
    fn a_launcher_on_disk_and_the_git_behind_it_are_two_programs() {
        let dir = crate::temp::dir();
        let launcher = dir.path().join("cmd").join("git.exe");
        let real = dir.path().join("mingw64").join("bin").join("git.exe");
        for file in [&launcher, &real] {
            std::fs::create_dir_all(file.parent().expect("a directory")).expect("mkdir");
            std::fs::write(file, b"").expect("a file");
        }
        assert!(!same_program(&launcher, &real));
        assert!(same_program(&launcher, &launcher));
    }

    #[cfg(unix)]
    #[test]
    fn a_link_and_its_target_are_one_program() {
        let dir = crate::temp::dir();
        let target = dir.path().join("git-2.55");
        std::fs::write(&target, b"").expect("a file");
        let link = dir.path().join("git");
        std::os::unix::fs::symlink(&target, &link).expect("symlink");
        assert!(same_program(&link, &target));
    }

    #[cfg(windows)]
    #[test]
    fn the_extension_a_spawn_adds_does_not_make_a_second_program() {
        let dir = crate::temp::dir();
        let exe = dir.path().join("git.exe");
        std::fs::write(&exe, b"").expect("a file");
        assert!(same_program(&dir.path().join("git"), &exe));
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
}
