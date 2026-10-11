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
    /// The repository's own worktree. git has no attribute for it:
    /// the listing opens on it wherever it is run from (git-worktree(1)).
    /// A bare repository's bare entry takes that place and is still the
    /// main one; the caller drops it for having no working tree.
    pub main: bool,
    /// The branch a rebase or bisect standing in this worktree is about,
    /// and which of the two ([`held_by_operation`]): listed `detached`, yet
    /// git keeps the branch to this worktree as if it were checked out —
    /// and turns a rename of it down from any worktree (builtin/branch.c
    /// `reject_rebase_or_bisect_branch`). `None` elsewhere.
    pub held: Option<Held>,
}

/// A branch an operation standing in a worktree keeps to it
/// ([`WorktreeEntry::held`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Held {
    pub branch: String,
    pub by: HeldBy,
}

/// What keeps a branch to a worktree listed `detached` ([`Held`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeldBy {
    Rebase,
    Bisect,
}

impl HeldBy {
    /// The word the screen branches on.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rebase => "rebase",
            Self::Bisect => "bisect",
        }
    }
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
                held: None,
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
    let mut entries = parse_worktrees(&out.stdout).map_err(|e| GitError::UnexpectedOutput {
        command: "git worktree list".to_string(),
        message: e.to_string(),
    })?;
    for entry in entries
        .iter_mut()
        .filter(|entry| entry.detached && !entry.bare && !entry.prunable)
    {
        entry.held = held_by_operation(Path::new(&entry.path));
    }
    Ok(entries)
}

/// The branch a rebase or bisect standing in the worktree at `path` is
/// about, read off the files git keeps for it: `rebase-merge/head-name` /
/// `rebase-apply/head-name` (a full ref), and `BISECT_START` beside
/// `BISECT_LOG` (a short name, or a commit where the bisect started
/// detached). What git's own `is_shared_symref` reads before refusing the
/// branch to any other worktree — the listing only says `detached`.
///
/// Plain file reads, a few per detached worktree: no git process. `None`
/// where nothing stands, or the worktree's git directory cannot be found.
fn held_by_operation(path: &Path) -> Option<Held> {
    let git_dir = crate::repo::git_dir_of(path)?;
    let read = |name: &str| {
        std::fs::read_to_string(git_dir.join(name))
            .ok()
            .map(|text| text.trim().to_string())
    };
    let held = |branch: &str, by| Held {
        branch: branch.to_string(),
        by,
    };
    for head_name in ["rebase-merge/head-name", "rebase-apply/head-name"] {
        if let Some(branch) = read(head_name)
            .as_deref()
            .and_then(|full| full.strip_prefix("refs/heads/"))
        {
            return Some(held(branch, HeldBy::Rebase));
        }
    }
    if !git_dir.join("BISECT_LOG").exists() {
        return None;
    }
    let start = read("BISECT_START")?;
    let branch = start.strip_prefix("refs/heads/").unwrap_or(&start);
    let detached = branch.len() >= 40 && branch.bytes().all(|b| b.is_ascii_hexdigit());
    (!branch.is_empty() && !detached && !branch.starts_with("refs/"))
        .then(|| held(branch, HeldBy::Bisect))
}

/// What a new worktree is made to stand on. Always a branch: every
/// road into one from the screen lands on a branch, as a move does
/// (デザイン規約 §ブランチ・コミットへの移動「この GUI の行き先は必ずブランチ」).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorktreeOn {
    /// A branch made for it at `start` (`-b`) — the commit, tag or ref the
    /// menu was opened on.
    NewBranch { name: String, start: String },
    /// A local branch no worktree has out.
    Branch(String),
    /// A local branch made off a remote one and set to follow it (`--track
    /// -b`), as `switch` does for a remote branch with no local one.
    /// `remote_ref` is the remote branch as the screen names it
    /// (`origin/feature`).
    Tracking { local: String, remote_ref: String },
}

/// `git worktree add`: a new folder at `path` with `on` checked out in it.
/// `name` is the worktree as the screen names it (its folder), which a
/// refusal's heading is written from.
///
/// Unforced, so git keeps everything it refuses for — a folder already
/// there, a branch another worktree has out, a name taken or malformed. The
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
///
/// `--quiet` reaches the checkout's own `reset`, whose progress git
/// writes to the pipe too once it runs past two seconds — ahead of any
/// refusal after it.
///
/// A non-zero answer is read against what git recorded and left: a
/// `post-checkout` hook that ended non-zero in its trace failed over a
/// worktree git made and kept ([`crate::report::worktree_hook_failed`]).
/// Otherwise a `-b` branch there now was git's — made before it turned the
/// rest down; the branch's own refusal (its name taken since) ends the
/// command with 255 before anything else.
pub async fn add(
    executor: &GitExecutor,
    workdir: &Path,
    path: &str,
    on: &WorktreeOn,
    name: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let made = match on {
        WorktreeOn::NewBranch { name, .. } | WorktreeOn::Tracking { local: name, .. } => Some(name),
        WorktreeOn::Branch(_) => None,
    };
    if let Some(made) = made.filter(|made| crate::branch::reads_as_option(made)) {
        return Err(GitError::Rejected {
            message: format!("'{made}' opens with '-', which git would read as an option"),
        });
    }
    if !folder_free(Path::new(path)) {
        return Err(crate::report::worktree_folder_taken(name, path));
    }
    let cmd = GitCommand::new()
        .cwd(workdir)
        .traced()
        .args(["worktree", "add", "--quiet"]);
    let cmd = match on {
        WorktreeOn::NewBranch { name, start } => cmd.args(["-b", name.as_str(), "--", path, start]),
        WorktreeOn::Branch(branch) => cmd.args(["--", path, branch.as_str()]),
        WorktreeOn::Tracking { local, remote_ref } => cmd.args([
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
    if out.code == 0 {
        return Ok(());
    }
    // git runs the hook only once the worktree is whole, and keeps it over
    // the hook's no (builtin/worktree.c, the minimum to 2.55); its trace has
    // the hook's end — a worktree left behind by a take-back that failed is
    // no hook's.
    if out.trace.hook_failed(&["post-checkout"]) {
        return Err(crate::report::worktree_hook_failed(name, command, &out));
    }
    // A read that fails here answers "none left": git's refusal is the
    // answer to report, not this read's failure.
    let left = match made {
        Some(made) if out.code != BRANCH_REFUSED => {
            crate::branch::has_branch(executor, workdir, made, cancel)
                .await
                .is_ok_and(|there| there)
                .then_some(made.as_str())
        }
        _ => None,
    };
    Err(crate::report::worktree_not_added(name, command, &out, left))
}

/// The code `worktree add` ends with where its own `git branch` for `-b`
/// failed: the child's failure returns -1 from the command
/// (builtin/worktree.c `add`, the minimum to 2.55).
const BRANCH_REFUSED: i32 = 255;

/// The folder every worktree made here lives in: beside the
/// repository's own worktree `main`, of the repository's name with `.worktrees`
/// after it (git's paths, `/` throughout). `None` for a `main` with no
/// parent.
#[must_use]
pub fn worktrees_folder(main: &str) -> Option<String> {
    match main.trim_end_matches('/').rsplit_once('/') {
        Some((parent, repo)) if !repo.is_empty() => Some(format!("{parent}/{repo}.worktrees")),
        _ => None,
    }
}

/// Where a new worktree for `branch` goes: in `worktrees_folder`, as the
/// branch's name with each `/` a `-` — so the folder a worktree is named by on
/// screen spells its whole branch. Empty for a `main` with no parent or a
/// branch with no name.
#[must_use]
pub fn new_worktree_path(main: &str, branch: &str) -> String {
    let folder = branch.replace('/', "-");
    match worktrees_folder(main) {
        Some(root) if !folder.is_empty() => format!("{root}/{folder}"),
        _ => String::new(),
    }
}

/// Whether git would make a worktree at `path`: nothing there, or a folder
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
/// branch it had out stays. `name` is the worktree as the screen names it,
/// which a refusal's heading is written from.
///
/// Unforced, so git keeps what it refuses for (uncommitted changes, a
/// lock, submodules); ignored files go with the folder
/// (git-worktree(1)). Every refusal is a report
/// ([`crate::report::worktree_not_removed`]) but one: a worktree git does
/// not know — removed elsewhere since the menu opened — is gone as asked.
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
    if out.code == 0 {
        return Ok(());
    }
    let said = out.stderr_utf8();
    if out.code == 128 && said.contains(NOT_A_WORKTREE) {
        return Ok(());
    }
    Err(crate::report::worktree_not_removed(
        name,
        command,
        &out,
        lock_reason(&said),
    ))
}

/// What git says for a path it lists no worktree at (`LC_ALL=C`,
/// builtin/worktree.c, the minimum to 2.55).
const NOT_A_WORKTREE: &str = "' is not a working tree";

/// The reason a worktree was locked with, out of git's refusal to remove
/// it — the owner's own words, which git sets between its two halves
/// (builtin/worktree.c `remove_worktree`, the minimum to 2.55): `Some("")`
/// for a lock without one, `None` for no lock. A full stop the owner ended
/// it with is left off: the screen's sentence closes it (`It is locked: …`).
fn lock_reason(said: &str) -> Option<&str> {
    const LOCKED: &str = "cannot remove a locked working tree";
    const REASON: &str = ", lock reason: ";
    const ADVICE: &str = "\nuse 'remove -f -f'";
    let (_, rest) = said.split_once(LOCKED)?;
    let Some(reason) = rest.strip_prefix(REASON) else {
        return Some("");
    };
    let reason = reason
        .rsplit_once(ADVICE)
        .map_or(reason, |(reason, _)| reason);
    Some(reason.strip_suffix('.').unwrap_or(reason))
}

/// Where the worktree at `from` goes when its folder is renamed `name`:
/// beside where it stands, in the same parent (git's paths, `/` throughout).
/// Empty for a `from` with no parent.
#[must_use]
pub fn renamed_worktree_path(from: &str, name: &str) -> String {
    match from.trim_end_matches('/').rsplit_once('/') {
        Some((parent, _)) => format!("{parent}/{name}"),
        None => String::new(),
    }
}

/// Whether `name` is one folder's name: not empty, not `.` or `..`, and no
/// separator that would make it a path to somewhere else — `\` too, on
/// every disk, since the screen spells every path with `/`. What one system
/// keeps out of a name beyond that is [`Disk::refuses`].
#[must_use]
pub fn is_folder_name(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\'])
}

/// Whose rules a folder's name answers to beyond [`is_folder_name`]: Windows
/// keeps characters, device names and a name's last letter for itself; Linux
/// and macOS take anything else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disk {
    Windows,
    Posix,
}

/// What Windows keeps out of a folder's name, as `git worktree move` meets it
/// (`tests/it/worktree_move.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiskRefusal {
    /// One of `< > : " | ? *`: git answers `Invalid argument` and nothing moves.
    Character(char),
    /// A device's name (`CON`, `PRN`, `AUX`, `NUL`, `COM1`–`COM9`, `LPT1`–`LPT9`),
    /// with an extension or without: git moves the folder to a name only a
    /// `\\?\` path reaches, cannot write the worktree's `.git` there, and
    /// leaves the worktree broken (`NUL` it turns down outright).
    Device,
    /// A dot or a space at the end, which Windows drops: the folder would land
    /// under another name than the one asked for.
    Trailing(char),
}

impl Disk {
    /// The rules of the system this runs on.
    pub const HERE: Self = if cfg!(windows) {
        Self::Windows
    } else {
        Self::Posix
    };

    /// What this system keeps out of `name`, a name [`is_folder_name`]
    /// answered; `None` where it takes it.
    #[must_use]
    pub fn refuses(self, name: &str) -> Option<DiskRefusal> {
        if self == Self::Posix {
            return None;
        }
        if let Some(c) = name.chars().find(|c| "<>:\"|?*".contains(*c)) {
            return Some(DiskRefusal::Character(c));
        }
        let stem = name.split('.').next().unwrap_or_default();
        let numbered = |prefix: &str| {
            stem.len() == 4
                && stem
                    .get(..3)
                    .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
                && matches!(stem.as_bytes().get(3), Some(b'1'..=b'9'))
        };
        if ["CON", "PRN", "AUX", "NUL"]
            .iter()
            .any(|device| stem.eq_ignore_ascii_case(device))
            || numbered("COM")
            || numbered("LPT")
        {
            return Some(DiskRefusal::Device);
        }
        name.chars()
            .last()
            .filter(|c| matches!(c, '.' | ' '))
            .map(DiskRefusal::Trailing)
    }
}

/// Whether nothing at all is at `path` — a file, a folder, an empty one, a
/// link. `git worktree move` moves into a folder that is there instead of
/// refusing it, so anything there is in the way. A path that cannot be
/// looked at is taken.
#[must_use]
pub fn path_free(path: &Path) -> bool {
    matches!(std::fs::symlink_metadata(path), Err(err) if err.kind() == std::io::ErrorKind::NotFound)
}

/// What [`move_to`] turns down before git runs.
fn check_move(from: &str, to: &str, name: &str) -> Result<(), GitError> {
    let leaf = to
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or_default();
    // Never sent by the name box, which turns such a name down as it is typed —
    // one Windows keeps for itself too, which would leave the worktree broken.
    if !is_folder_name(leaf)
        || Disk::HERE.refuses(leaf).is_some()
        || renamed_worktree_path(from, leaf) != to
    {
        return Err(GitError::Rejected {
            message: format!("'{to}' is not a folder beside '{from}'"),
        });
    }
    if !path_free(Path::new(to)) {
        return Err(crate::report::worktree_rename_refused(
            name,
            format!("'{to}' already exists"),
        ));
    }
    Ok(())
}

/// `git worktree move -- <from> <to>`: the worktree's folder renamed, its
/// branch, HEAD and changes going with it. `name` is the worktree as the
/// screen names it (its folder now), which a refusal's heading is written
/// from. `cwd` is where git runs: anywhere in the repository but the
/// worktree being moved, whose folder Windows will not rename while a
/// process stands in it.
///
/// Turned down here, before git: a `to` that is not one folder in `from`'s
/// parent, and anything already at `to` — git moves into a folder that is
/// there, and would put the worktree inside it
/// ([`crate::report::worktree_rename_refused`]).
///
/// Unforced, so git keeps everything else it refuses for: the repository's
/// own worktree, a lock, submodules, a folder that is gone, and a path still
/// listed for another worktree ([`crate::report::worktree_not_renamed`]).
pub async fn move_to(
    executor: &GitExecutor,
    cwd: &Path,
    from: &str,
    to: &str,
    name: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    check_move(from, to, name)?;
    let cmd = GitCommand::new()
        .cwd(cwd)
        .args(["worktree", "move", "--", from, to]);
    let command = cmd.describe();
    let out = executor.run_unchecked(cmd, cancel).await?;
    match out.code {
        0 => Ok(()),
        _ => Err(crate::report::worktree_not_renamed(name, command, &out)),
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
            "worktree C:/repo/.claude/worktrees/worktree-1",
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
    fn the_entry_git_opens_the_listing_with_is_the_main_worktree() {
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

        let bare = z(&[
            "worktree /srv/repo.git",
            "bare",
            "",
            "worktree /srv/worktree",
            "",
        ]);
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
            "worktree /srv/worktree",
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
            "worktree C:/tmp/worktree-probe/main",
            "HEAD 8cd5289c16c24ae01dd41d6a1a4afec20f6ba99d",
            "branch refs/heads/master",
            "",
            "worktree C:/tmp/worktree-probe/worktree-det",
            "HEAD 8cd5289c16c24ae01dd41d6a1a4afec20f6ba99d",
            "detached",
            "",
            "worktree C:/tmp/worktree-probe/worktree-gone",
            "HEAD 8cd5289c16c24ae01dd41d6a1a4afec20f6ba99d",
            "branch refs/heads/gone",
            "prunable gitdir file points to non-existent location",
            "",
            "worktree C:/tmp/worktree-probe/worktree-lock",
            "HEAD 8cd5289c16c24ae01dd41d6a1a4afec20f6ba99d",
            "branch refs/heads/locked",
            "locked seat held by claude",
            "",
            "worktree C:/tmp/worktree-probe/worktree-lock2",
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
            "worktree /srv/worktree",
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
    fn a_new_worktree_goes_under_the_repositorys_worktrees_folder_by_its_branch() {
        assert_eq!(
            new_worktree_path("C:/work/platitude-gg", "feature/login"),
            "C:/work/platitude-gg.worktrees/feature-login"
        );
        assert_eq!(
            new_worktree_path("/srv/repo/", "fix"),
            "/srv/repo.worktrees/fix"
        );
        assert_eq!(new_worktree_path("C:/repo", "x"), "C:/repo.worktrees/x");
    }

    #[test]
    fn no_place_without_a_parent_or_a_name() {
        assert_eq!(new_worktree_path("repo", "x"), "");
        assert_eq!(worktrees_folder("repo"), None);
        assert_eq!(new_worktree_path("C:/work/repo", ""), "");
        assert_eq!(
            worktrees_folder("C:/work/repo").as_deref(),
            Some("C:/work/repo.worktrees")
        );
    }

    /// git's two refusals of a locked worktree (the minimum and 2.55), a
    /// reason over two lines, and a refusal for something else.
    #[test]
    fn a_locks_reason_is_read_out_of_gits_refusal() {
        assert_eq!(
            lock_reason(
                "fatal: cannot remove a locked working tree, lock reason: on a stick\n\
                 use 'remove -f -f' to override or unlock first"
            ),
            Some("on a stick")
        );
        assert_eq!(
            lock_reason(
                "fatal: cannot remove a locked working tree, lock reason: one\ntwo\n\
                 use 'remove -f -f' to override or unlock first"
            ),
            Some("one\ntwo")
        );
        assert_eq!(
            lock_reason(
                "fatal: cannot remove a locked working tree, lock reason: on a stick.\n\
                 use 'remove -f -f' to override or unlock first"
            ),
            Some("on a stick"),
            "the screen's sentence closes it, so its own full stop goes"
        );
        assert_eq!(
            lock_reason(
                "fatal: cannot remove a locked working tree;\n\
                 use 'remove -f -f' to override or unlock first"
            ),
            Some("")
        );
        assert_eq!(
            lock_reason("fatal: 'C:/t/wt' contains modified or untracked files"),
            None
        );
    }

    #[test]
    fn a_renamed_worktree_stays_in_its_parent() {
        assert_eq!(
            renamed_worktree_path("C:/work/repo.worktrees/old", "feature-x"),
            "C:/work/repo.worktrees/feature-x"
        );
        assert_eq!(
            renamed_worktree_path("/srv/repo/.claude/worktrees/a/", "b"),
            "/srv/repo/.claude/worktrees/b"
        );
        assert_eq!(renamed_worktree_path("loose", "x"), "");
    }

    #[test]
    fn a_folder_name_is_one_folder() {
        assert!(is_folder_name("feature-x"));
        assert!(is_folder_name(".hidden"));
        assert!(is_folder_name("-dash"));
        for not in ["", ".", "..", "a/b", "a\\b", "../out"] {
            assert!(!is_folder_name(not), "{not:?}");
        }
    }

    /// Windows keeps its characters, its devices' names — an extension does not
    /// free one, a longer name is not one — and a name's trailing dot or space;
    /// Linux and macOS take every one of them.
    #[test]
    fn windows_keeps_what_the_others_take() {
        use DiskRefusal::{Character, Device, Trailing};
        let kept = [
            ("a<b", Character('<')),
            ("a>b", Character('>')),
            ("a:b", Character(':')),
            ("a\"b", Character('"')),
            ("a|b", Character('|')),
            ("why?", Character('?')),
            ("a*b", Character('*')),
            ("CON", Device),
            ("con.txt", Device),
            ("Nul", Device),
            ("aux", Device),
            ("prn.tar.gz", Device),
            ("COM1", Device),
            ("lpt9.log", Device),
            ("dot.", Trailing('.')),
            ("space ", Trailing(' ')),
        ];
        for (name, why) in kept {
            assert_eq!(Disk::Windows.refuses(name), Some(why), "{name:?}");
            assert_eq!(Disk::Posix.refuses(name), None, "{name:?}");
        }
        for taken in [
            "conx",
            "COM0",
            "com10",
            "lpt",
            "a.con",
            "ノート",
            "éé",
            "x.y",
        ] {
            assert_eq!(Disk::Windows.refuses(taken), None, "{taken:?}");
        }
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
