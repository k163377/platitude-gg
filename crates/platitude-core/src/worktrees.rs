//! Worktree listing: `git worktree list --porcelain -z`.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeEntry {
    /// Absolute path of the worktree root (as git prints it).
    pub path: String,
    /// Checked-out branch (short name); `None` when detached or bare.
    pub branch: Option<String>,
    /// HEAD commit id (hex); `None` for a bare entry.
    pub head_hex: Option<String>,
    pub bare: bool,
    pub detached: bool,
    pub locked: bool,
    /// What `git worktree lock --reason` was given. Empty when unlocked
    /// and when locked without one; `locked` is the flag.
    pub lock_reason: String,
    /// `git worktree prune` would drop this entry: still listed, but its
    /// directory is gone.
    pub prunable: bool,
    /// Why git would drop it, in git's own words (`gitdir file points to
    /// non-existent location`). Empty when `prunable` is false.
    pub prune_reason: String,
    /// The repository's own working copy. git has no attribute for it:
    /// the listing opens on it wherever it is run from (git-worktree(1)).
    /// A bare repository's bare entry takes that place and is still the
    /// main one; the caller drops it for having no working copy.
    pub main: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("malformed worktree list output")]
pub struct WorktreeParseError;

/// Parses `--porcelain -z` output: NUL-terminated attribute lines, an
/// empty line between entries. Unknown attributes are ignored (forward
/// compatibility).
pub fn parse_worktrees(bytes: &[u8]) -> Result<Vec<WorktreeEntry>, WorktreeParseError> {
    let mut out = Vec::new();
    let mut cur: Option<WorktreeEntry> = None;
    for token in bytes.split(|b| *b == 0) {
        if token.is_empty() {
            if let Some(entry) = cur.take() {
                out.push(entry);
            }
            continue;
        }
        let line = String::from_utf8_lossy(token);
        if let Some(path) = line.strip_prefix("worktree ") {
            if let Some(entry) = cur.take() {
                out.push(entry);
            }
            cur = Some(WorktreeEntry {
                path: path.to_string(),
                // After the push above: no closed entry means this is the
                // first.
                main: out.is_empty(),
                branch: None,
                head_hex: None,
                bare: false,
                detached: false,
                locked: false,
                lock_reason: String::new(),
                prunable: false,
                prune_reason: String::new(),
            });
            continue;
        }
        let Some(entry) = cur.as_mut() else {
            return Err(WorktreeParseError);
        };
        if let Some(head) = line.strip_prefix("HEAD ") {
            entry.head_hex = Some(head.trim().to_string());
        } else if let Some(branch) = line.strip_prefix("branch ") {
            let branch = branch.trim();
            entry.branch = Some(
                branch
                    .strip_prefix("refs/heads/")
                    .unwrap_or(branch)
                    .to_string(),
            );
        } else if line.as_ref() == "bare" {
            entry.bare = true;
        } else if line.as_ref() == "detached" {
            entry.detached = true;
        } else if let Some(reason) = annotation(&line, "locked") {
            entry.locked = true;
            entry.lock_reason = reason.to_string();
        } else if let Some(reason) = annotation(&line, "prunable") {
            entry.prunable = true;
            entry.prune_reason = reason.to_string();
        }
    }
    if let Some(entry) = cur.take() {
        out.push(entry);
    }
    Ok(out)
}

/// A flag line that may carry a reason (`locked`, `prunable`): `Some("")`
/// for the bare word, `Some(reason)` with one, `None` otherwise. The word
/// has to end there: a future `lockedsomething` is another attribute.
fn annotation<'a>(line: &'a str, word: &str) -> Option<&'a str> {
    if line == word {
        return Some("");
    }
    line.strip_prefix(word)
        .and_then(|rest| rest.strip_prefix(' '))
        .map(str::trim)
}

pub async fn load(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Vec<WorktreeEntry>, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["worktree", "list", "--porcelain", "-z"]);
    let out = executor.run(cmd, cancel).await?;
    parse_worktrees(&out.stdout).map_err(|e| GitError::UnexpectedOutput {
        command: "git worktree list".to_string(),
        message: e.to_string(),
    })
}

/// What a new working copy is made to stand on. Always a branch: every
/// road into one from the screen lands on a branch, as a move does
/// (デザイン規約 §ブランチ・コミットへの移動「この GUI の行き先は必ずブランチ」).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CopyOn {
    /// A branch made for it at `start` (`-b`) — the commit, tag or ref the
    /// menu was opened on.
    NewBranch { name: String, start: String },
    /// A local branch no working copy has out.
    Branch(String),
    /// A local branch made off a remote one and set to follow it (`--track
    /// -b`), as `switch` does for a remote branch with no local one.
    /// `remote_ref` is the remote branch as the screen names it
    /// (`origin/feature`).
    Tracking { local: String, remote_ref: String },
}

/// `git worktree add`: a new folder at `path` with `on` checked out in it.
/// `name` is the copy as the screen names it (its folder), which a
/// refusal's heading is written from.
///
/// Unforced, so git keeps everything it refuses for — a folder already
/// there, a branch another copy has out, a name taken or malformed. The
/// menus answer those before the press; what reaches git anyway is a
/// report ([`crate::report::worktree_not_added`]).
///
/// A folder holding something is turned down here, before git: git makes
/// the `-b` branch first and only then looks at the folder, so its own
/// refusal leaves the branch behind.
///
/// So is a `-b` name opening with `-`, which the name box never sends:
/// git hands the name to its own `git branch` with nothing to end the
/// options before it, and it runs as one (`-m` renames the branch out
/// here).
///
/// The remote branch is spelled in full: `origin/feature` is ambiguous, and
/// refused, where a local branch has that name.
pub async fn add(
    executor: &GitExecutor,
    workdir: &Path,
    path: &str,
    on: &CopyOn,
    name: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let made = match on {
        CopyOn::NewBranch { name, .. } | CopyOn::Tracking { local: name, .. } => Some(name),
        CopyOn::Branch(_) => None,
    };
    if let Some(made) = made.filter(|made| crate::branch::reads_as_option(made)) {
        return Err(GitError::Rejected {
            message: format!("'{made}' opens with '-', which git would read as an option"),
        });
    }
    if !folder_free(Path::new(path)) {
        return Err(crate::report::worktree_folder_taken(name, path));
    }
    let cmd = GitCommand::new().cwd(workdir).args(["worktree", "add"]);
    let cmd = match on {
        CopyOn::NewBranch { name, start } => cmd.args(["-b", name.as_str(), "--", path, start]),
        CopyOn::Branch(branch) => cmd.args(["--", path, branch.as_str()]),
        CopyOn::Tracking { local, remote_ref } => cmd.args([
            "--track",
            "-b",
            local.as_str(),
            "--",
            path,
            &format!("refs/remotes/{remote_ref}"),
        ]),
    };
    let command = cmd.describe();
    let out = executor.run_unchecked(cmd, cancel).await?;
    match out.code {
        0 => Ok(()),
        _ => Err(crate::report::worktree_not_added(name, command, &out)),
    }
}

/// The folder every working copy made here lives in: beside the
/// repository's own copy `main`, of the repository's name with `.worktrees`
/// after it (git's paths, `/` throughout). `None` for a `main` with no
/// parent.
#[must_use]
pub fn copies_folder(main: &str) -> Option<String> {
    match main.trim_end_matches('/').rsplit_once('/') {
        Some((parent, repo)) if !repo.is_empty() => Some(format!("{parent}/{repo}.worktrees")),
        _ => None,
    }
}

/// Where a new working copy for `branch` goes: in `copies_folder`, as the
/// branch's name with each `/` a `-` — so the folder a copy is named by on
/// screen spells its whole branch. Empty for a `main` with no parent or a
/// branch with no name.
#[must_use]
pub fn new_copy_path(main: &str, branch: &str) -> String {
    let folder = branch.replace('/', "-");
    match copies_folder(main) {
        Some(root) if !folder.is_empty() => format!("{root}/{folder}"),
        _ => String::new(),
    }
}

/// Whether git would make a copy at `path`: nothing there, or a folder
/// with nothing in it (git-worktree(1) takes either). A file, or a folder
/// that cannot be read, is taken.
#[must_use]
pub fn folder_free(path: &Path) -> bool {
    match std::fs::read_dir(path) {
        Ok(mut entries) => entries.next().is_none(),
        Err(err) => err.kind() == std::io::ErrorKind::NotFound,
    }
}

/// `git worktree remove -- <path>`: the folder and git's record of it. The
/// branch it had out stays. `name` is the copy as the screen names it,
/// which a refusal's heading is written from.
///
/// Unforced, so git keeps what it refuses for (uncommitted changes, a
/// lock, submodules); ignored files go with the folder
/// (git-worktree(1)). Every refusal is a report
/// ([`crate::report::worktree_not_removed`]).
pub async fn remove(
    executor: &GitExecutor,
    workdir: &Path,
    path: &str,
    name: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["worktree", "remove", "--", path]);
    let command = cmd.describe();
    let out = executor.run_unchecked(cmd, cancel).await?;
    match out.code {
        0 => Ok(()),
        _ => Err(crate::report::worktree_not_removed(name, command, &out)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn z(lines: &[&str]) -> Vec<u8> {
        let mut v = Vec::new();
        for l in lines {
            v.extend_from_slice(l.as_bytes());
            v.push(0);
        }
        v
    }

    #[test]
    fn parses_main_linked_and_detached() {
        let bytes = z(&[
            "worktree C:/repo",
            "HEAD 1111111111111111111111111111111111111111",
            "branch refs/heads/main",
            "",
            "worktree C:/repo/.claude/worktrees/wt-1",
            "HEAD 2222222222222222222222222222222222222222",
            "detached",
            "",
        ]);
        let list = parse_worktrees(&bytes).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].path, "C:/repo");
        assert_eq!(list[0].branch.as_deref(), Some("main"));
        assert!(!list[0].detached);
        assert!(list[1].detached);
        assert_eq!(list[1].branch, None);
    }

    /// Every entry but the first answers no, the bare one included.
    #[test]
    fn the_entry_git_opens_the_listing_with_is_the_main_working_copy() {
        let bytes = z(&[
            "worktree C:/repo",
            "HEAD 1111111111111111111111111111111111111111",
            "branch refs/heads/main",
            "",
            "worktree C:/repo/../topic",
            "HEAD 2222222222222222222222222222222222222222",
            "branch refs/heads/feature/topic-a",
            "",
        ]);
        let list = parse_worktrees(&bytes).unwrap();
        assert!(list[0].main);
        assert!(!list[1].main);

        let bare = z(&["worktree /srv/repo.git", "bare", "", "worktree /srv/wt", ""]);
        let list = parse_worktrees(&bare).unwrap();
        assert!(list[0].main && list[0].bare);
        assert!(!list[1].main);
    }

    #[test]
    fn parses_bare_and_locked() {
        let bytes = z(&[
            "worktree /srv/repo.git",
            "bare",
            "",
            "worktree /srv/wt",
            "HEAD 3333333333333333333333333333333333333333",
            "branch refs/heads/dev",
            "locked reason text",
            "",
        ]);
        let list = parse_worktrees(&bytes).unwrap();
        assert!(list[0].bare);
        assert!(list[1].locked);
        assert_eq!(list[1].lock_reason, "reason text");
        assert_eq!(list[1].branch.as_deref(), Some("dev"));
    }

    /// Copied off a real listing (git 2.55.0.windows.3).
    #[test]
    fn parses_every_state_of_a_real_listing() {
        let bytes = z(&[
            "worktree C:/tmp/wtprobe/main",
            "HEAD 8cd5289c16c24ae01dd41d6a1a4afec20f6ba99d",
            "branch refs/heads/master",
            "",
            "worktree C:/tmp/wtprobe/wt-det",
            "HEAD 8cd5289c16c24ae01dd41d6a1a4afec20f6ba99d",
            "detached",
            "",
            "worktree C:/tmp/wtprobe/wt-gone",
            "HEAD 8cd5289c16c24ae01dd41d6a1a4afec20f6ba99d",
            "branch refs/heads/gone",
            "prunable gitdir file points to non-existent location",
            "",
            "worktree C:/tmp/wtprobe/wt-lock",
            "HEAD 8cd5289c16c24ae01dd41d6a1a4afec20f6ba99d",
            "branch refs/heads/locked",
            "locked seat held by claude",
            "",
            "worktree C:/tmp/wtprobe/wt-lock2",
            "HEAD 8cd5289c16c24ae01dd41d6a1a4afec20f6ba99d",
            "branch refs/heads/locked2",
            "locked",
            "",
        ]);
        let list = parse_worktrees(&bytes).unwrap();
        assert_eq!(list.len(), 5);
        assert!(!list[0].locked && !list[0].prunable && !list[0].detached);
        assert!(list[1].detached);
        assert!(list[2].prunable);
        assert_eq!(
            list[2].prune_reason,
            "gitdir file points to non-existent location"
        );
        assert!(!list[2].locked);
        assert!(list[3].locked);
        assert_eq!(list[3].lock_reason, "seat held by claude");
        // Locked without a reason: flag on, no words (a row would
        // otherwise draw `Locked —` with nothing after it).
        assert!(list[4].locked);
        assert_eq!(list[4].lock_reason, "");
    }

    #[test]
    fn an_attribute_that_only_starts_like_a_flag_is_ignored() {
        let bytes = z(&[
            "worktree /srv/wt",
            "HEAD 3333333333333333333333333333333333333333",
            "lockedness whatever",
            "prunableness whatever",
            "",
        ]);
        let list = parse_worktrees(&bytes).unwrap();
        assert!(!list[0].locked);
        assert!(!list[0].prunable);
    }

    #[test]
    fn a_new_copy_goes_under_the_repositorys_worktrees_folder_by_its_branch() {
        assert_eq!(
            new_copy_path("C:/work/platitude-gg", "feature/login"),
            "C:/work/platitude-gg.worktrees/feature-login"
        );
        assert_eq!(
            new_copy_path("/srv/repo/", "fix"),
            "/srv/repo.worktrees/fix"
        );
        assert_eq!(new_copy_path("C:/repo", "x"), "C:/repo.worktrees/x");
    }

    #[test]
    fn no_place_without_a_parent_or_a_name() {
        assert_eq!(new_copy_path("repo", "x"), "");
        assert_eq!(copies_folder("repo"), None);
        assert_eq!(new_copy_path("C:/work/repo", ""), "");
        assert_eq!(
            copies_folder("C:/work/repo").as_deref(),
            Some("C:/work/repo.worktrees")
        );
    }

    #[test]
    fn empty_output_is_empty() {
        assert!(parse_worktrees(b"").unwrap().is_empty());
    }

    #[test]
    fn attribute_before_worktree_is_an_error() {
        assert!(parse_worktrees(&z(&["branch refs/heads/x"])).is_err());
    }
}
