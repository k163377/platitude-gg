//! Where the task runner's throwaway trees go — demo repositories,
//! verify-ui runs, a container run's host folder, the claims on them,
//! test trees: the primary checkout's `target/tmp`, one folder every seat
//! shares. It is inside the project the virus scanner is told to leave
//! alone, and `target/` is ignored, so the primary's status stays clean.
//! A `cargo clean` in the primary takes all of it — the runs standing, the
//! claims, the templates and the trees a person kept (`verify::keep`).
//!
//! In the container it is the container's own `/tmp`: `/work` is the
//! host's checkout across a mount, where a rename is not atomic and what a
//! run leaves would land on the host.
//!
//! The folder sits inside the primary checkout, so git that looks for a
//! repository from a folder there that holds none finds the primary —
//! `main`. What starts such a git carries [`stop_git_above`].

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// The folder the trees go in, made once per process.
pub(crate) fn root() -> &'static Path {
    static MADE: OnceLock<()> = OnceLock::new();
    let root = place();
    MADE.get_or_init(|| {
        // A folder that cannot be made fails the first write into it, which
        // names the path.
        let _ = std::fs::create_dir_all(root);
    });
    root
}

/// Where [`root`] is, without making it — for the readers that only ask
/// whether a path is in there (`hook::seat`).
pub(crate) fn place() -> &'static Path {
    static PLACE: OnceLock<PathBuf> = OnceLock::new();
    PLACE.get_or_init(|| {
        if std::env::var_os(crate::linux::IN_CONTAINER).is_some() {
            std::env::temp_dir()
        } else {
            in_the_primary()
        }
    })
}

/// The primary checkout's `target/tmp`, found from the tree this runner
/// was built in — not the working directory, which a gate test points at
/// a sandbox repository. A tree in no repository is its own primary.
fn in_the_primary() -> PathBuf {
    let workspace = crate::tree::workspace_root();
    let primary = crate::tree::primary_root(&workspace.to_string_lossy()).unwrap_or(workspace);
    let root = primary.join("target").join("tmp");
    // One spelling, with the platform's separators: the `.git` file a seat
    // reads its primary from writes forward slashes, and the path is handed
    // to git as a ceiling.
    std::path::absolute(&root).unwrap_or(root)
}

/// Keeps git from looking above [`root`] for a repository, so a folder
/// there that holds none is not one. Children inherit it, so it is set on
/// what starts the git: the app a verify-ui run drives, whose working
/// directory and the folders the open-refused verbs hand it hold no
/// repository.
pub(crate) fn stop_git_above(command: &mut Command) -> &mut Command {
    command.env("GIT_CEILING_DIRECTORIES", root())
}

#[cfg(test)]
mod tests {
    /// From a seat or from the primary itself, the one folder is the
    /// primary's: a seat's own `target` would split the claims and the
    /// templates by seat.
    #[test]
    fn the_folder_is_the_primary_checkouts_target_tmp() {
        if std::env::var_os(crate::linux::IN_CONTAINER).is_some() {
            return;
        }
        let place = super::place();
        assert!(place.ends_with("target/tmp"), "{}", place.display());
        let primary = place
            .parent()
            .and_then(std::path::Path::parent)
            .expect("two folders above");
        assert!(
            primary.join(".git").is_dir(),
            "{} is no primary checkout",
            primary.display()
        );
    }
}
