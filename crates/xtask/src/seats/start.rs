//! The tree the desktop app made for a session to start in: a chip's
//! session opens in one — `claude/<name>`, cut from origin/main — and moves
//! to a seat to work. The app's git creates it with no session's mark, so
//! the reference-transaction hook writes no maker down; the session is
//! written down as its maker here instead (`gate::hooks::Made::StartedIn`),
//! for its landing to take the tree and the branch away (`land::leftovers`).

use std::path::{Path, PathBuf};

use super::worktree_blocks;
use crate::gate::hooks::{self, Made};
use crate::subprocess::git_query;

/// The file the app leaves in the admin directory of each worktree it
/// makes (`.git/worktrees/<name>/`); its own clean-up reads it the same way.
const APP_MADE: &str = "claude-desktop-worktree";

/// Writes the branch of `tree` down as `session`'s start, when the app made
/// `tree` and cut that branch for it: over another session's start — the
/// app hands a tree on to the next session it starts there — but over
/// neither this session's own record, so one its landing left behind stays
/// so when it resumes there, nor one a session's git made, which no app
/// cut. None for a tree the app did not make, or a branch it did not cut
/// there; else the branch's name, or why it was not written down.
pub(crate) fn write_down(tree: &str, session: &str) -> Option<Result<String, String>> {
    let (common, reference) = app_made(tree)?;
    let branch = reference.strip_prefix("refs/heads/")?.to_string();
    let Some(common) = common else {
        return Some(Err("git could not name the repository".to_string()));
    };
    match hooks::maker(&common, &reference) {
        Some(maker) if maker.session == session => return Some(Ok(branch)),
        Some(maker) if maker.how == Made::Created => return None,
        _ => {}
    }
    Some(hooks::write_down(&common, &reference, session, Made::StartedIn).map(|()| branch))
}

/// Writes down the tree the app made for `session` to start in, when
/// nothing names its branch's maker yet — for a landing, whose session's
/// start may not have: a chip's first hooks run the code of the tree the
/// app cut from origin/main, which lags main until main is pushed. Found
/// from where Claude Code keeps the session's transcript ([`started_at`]).
pub(crate) fn adopt(listing: &str, session: &str) {
    let Some(started) = started_at(session) else {
        return;
    };
    // The first tree listed is the primary checkout, which no app made.
    let Some(tree) = worktree_blocks(listing)
        .into_iter()
        .skip(1)
        .find(|tree| spelled_like(&tree.path, &started))
    else {
        return;
    };
    if let Some((Some(common), reference)) = app_made(&tree.path)
        && hooks::maker(&common, &reference).is_none()
    {
        // Unwritten, the tree stays as it would have: nothing to say.
        let _ = hooks::write_down(&common, &reference, session, Made::StartedIn);
    }
}

/// The repository's common directory and the branch `tree` has out, when
/// the app made `tree` and cut that branch for it. The app names the
/// branch after the tree — `<prefix>/<name>`, the bare name with no prefix
/// set — so a branch checked out there since, the user's or a session's,
/// is none of it.
fn app_made(tree: &str) -> Option<(Option<PathBuf>, String)> {
    let admin = git_query(tree, &["rev-parse", "--absolute-git-dir"])?;
    if !Path::new(&admin).join(APP_MADE).is_file() {
        return None;
    }
    let reference = git_query(tree, &["symbolic-ref", "-q", "HEAD"])?;
    let name = Path::new(tree).file_name()?.to_str()?;
    if reference.rsplit('/').next() != Some(name) {
        return None;
    }
    let common = crate::subprocess::common_git_dir(tree).map(PathBuf::from);
    Some((common, reference))
}

/// The directory Claude Code keeps `session`'s transcript in, under its
/// config directory's `projects/`: one per place a session started, named
/// after that place.
fn started_at(session: &str) -> Option<String> {
    // The id becomes a file name.
    if session.is_empty()
        || !session
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return None;
    }
    let config = std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
                .map(|home| PathBuf::from(home).join(".claude"))
        })?;
    let transcript = format!("{session}.jsonl");
    std::fs::read_dir(config.join("projects"))
        .ok()?
        .flatten()
        .find(|place| place.path().join(&transcript).is_file())
        .map(|place| place.file_name().to_string_lossy().into_owned())
}

/// Whether the tree at `path` is the place a project directory is named
/// after: every character but a letter or digit spelled `-`. Tried on the
/// path as git lists it and as the filesystem resolves it — the app hands
/// Claude Code the long name, git keeps what it was given.
fn spelled_like(path: &str, place: &str) -> bool {
    let resolved = std::fs::canonicalize(path).ok().map(|path| {
        path.to_string_lossy()
            .trim_start_matches(r"\\?\")
            .to_string()
    });
    std::iter::once(path.to_string())
        .chain(resolved)
        .any(|path| {
            let spelled: String = path
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
                .collect();
            if cfg!(windows) {
                spelled.eq_ignore_ascii_case(place)
            } else {
                spelled == place
            }
        })
}

#[cfg(test)]
mod tests {
    use super::spelled_like;

    /// As Claude Code named the directories of sessions that started in a
    /// tree the app made, the Windows spelling and the POSIX one.
    #[test]
    fn a_tree_is_spelled_the_way_its_project_directory_is_named() {
        let chip = "C:/Users/u/IdeaProjects/platitude-gg/.claude/worktrees/hungry-easley-3957c9";
        assert!(spelled_like(
            chip,
            "C--Users-u-IdeaProjects-platitude-gg--claude-worktrees-hungry-easley-3957c9"
        ));
        assert!(!spelled_like(
            chip,
            "C--Users-u-IdeaProjects-platitude-gg--claude-worktrees-a"
        ));
        assert!(spelled_like(
            "/home/u/platitude-gg/.claude/worktrees/keen-williams-6ac56a",
            "-home-u-platitude-gg--claude-worktrees-keen-williams-6ac56a"
        ));
    }
}
