//! Conflict resolution support: which files are conflicted, how far a
//! stepped operation has got, and handing resolution to `git mergetool`.
//!
//! Resolving conflicts is explicitly out of scope for this application
//! (実装計画.md §1 スコープ外「内蔵conflictエディタ」): the built-in editor belongs to
//! whatever tool the user already configured.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::config;
use crate::error::GitError;
use crate::integrate::InProgress;
use crate::process::{GitCommand, GitExecutor, literal_pathspec};
use crate::status::{StatusItem, WorkTreeStatus};

/// How a conflicted file conflicts, derived from its two stage letters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictKind {
    /// Both sides changed the content (`UU`).
    BothModified,
    /// Both sides added the path (`AA`).
    BothAdded,
    /// Deleted here, modified there (`DU`).
    DeletedByUs,
    /// Modified here, deleted there (`UD`).
    DeletedByThem,
    /// Added here only (`AU`).
    AddedByUs,
    /// Added there only (`UA`).
    AddedByThem,
    /// Both sides deleted it (`DD`).
    BothDeleted,
    /// Any other stage combination git may report.
    Other,
}

impl ConflictKind {
    pub(crate) fn from_stages(ours: char, theirs: char) -> Self {
        match (ours, theirs) {
            ('U', 'U') => ConflictKind::BothModified,
            ('A', 'A') => ConflictKind::BothAdded,
            ('D', 'U') => ConflictKind::DeletedByUs,
            ('U', 'D') => ConflictKind::DeletedByThem,
            ('A', 'U') => ConflictKind::AddedByUs,
            ('U', 'A') => ConflictKind::AddedByThem,
            ('D', 'D') => ConflictKind::BothDeleted,
            _ => ConflictKind::Other,
        }
    }

    /// True when a merge tool has three usable sides to work with. The
    /// delete/delete and add/add-with-no-base cases are decided by picking
    /// a side, not by editing.
    pub fn is_content_conflict(self) -> bool {
        matches!(self, ConflictKind::BothModified | ConflictKind::BothAdded)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictedFile {
    pub path: String,
    pub kind: ConflictKind,
}

/// Conflicted entries of a status snapshot, in git's order.
pub fn conflicted(status: &WorkTreeStatus) -> Vec<ConflictedFile> {
    status
        .conflicted()
        .filter_map(|item| match item {
            StatusItem::Unmerged { ours, theirs, path } => Some(ConflictedFile {
                path: path.clone(),
                kind: ConflictKind::from_stages(*ours, *theirs),
            }),
            _ => None,
        })
        .collect()
}

/// How far a stepped operation has got ("commit 3 of 7").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub current: u32,
    pub total: u32,
}

/// Progress of an in-progress rebase, or `None` when none is running.
///
/// Both rebase backends are read: the merge backend (the default, and what
/// `--interactive` always uses) counts in `rebase-merge/msgnum`, while the
/// apply backend counts in `rebase-apply/next`.
pub async fn rebase_progress(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Option<Progress>, GitError> {
    const FILES: [(&str, &str); 2] = [
        ("rebase-merge/msgnum", "rebase-merge/end"),
        ("rebase-apply/next", "rebase-apply/last"),
    ];
    let mut cmd = GitCommand::new().cwd(workdir).arg("rev-parse");
    for (current, total) in FILES {
        cmd = cmd.args(["--git-path", current]);
        cmd = cmd.args(["--git-path", total]);
    }
    let out = executor.run(cmd, cancel).await?;
    let text = out.stdout_utf8();
    let paths: Vec<&str> = text.lines().map(str::trim_end).collect();

    for pair in paths.chunks_exact(2) {
        // `--git-path` prints paths relative to the cwd or absolute ones;
        // joining handles both.
        let current = read_count(&workdir.join(pair[0]));
        let total = read_count(&workdir.join(pair[1]));
        if let (Some(current), Some(total)) = (current, total)
            && total > 0
        {
            return Ok(Some(Progress { current, total }));
        }
    }
    Ok(None)
}

fn read_count(path: &Path) -> Option<u32> {
    std::fs::read_to_string(path).ok()?.trim().parse().ok()
}

/// What to call the two sides of a conflict, for whatever operation is
/// stopped. Either may be empty when git left nothing to name it by.
///
/// **The two swap over during a rebase**: the commits being replayed are
/// `theirs`, and `ours` is the upstream they are landing on. Reading the
/// names from the operation is what keeps a UI from having to explain
/// that — each side is called what it actually is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sides {
    /// The side already in place (`--ours`).
    pub ours: String,
    /// The side being brought in (`--theirs`).
    pub theirs: String,
}

/// Reads what the stopped operation's two sides are called.
///
/// Where each name comes from (measured —
/// `what_a_stopped_operation_says_about_its_two_sides`):
///
/// | operation | ours | theirs |
/// |---|---|---|
/// | rebase | `rebase-*/onto`, named | `rebase-*/head-name`, a full ref |
/// | merge | the current branch | `MERGE_HEAD`, named |
/// | cherry-pick | the current branch | `CHERRY_PICK_HEAD`, named |
/// | revert | the current branch | `REVERT_HEAD`, named |
pub async fn sides(
    executor: &GitExecutor,
    workdir: &Path,
    op: InProgress,
    cancel: &CancellationToken,
) -> Result<Sides, GitError> {
    if op == InProgress::Rebase {
        // Both backends keep the same two files under their own
        // directory; the merge backend is the default and the only one
        // this app starts, but a rebase begun at the command line may be
        // the other. One `rev-parse` resolves all four paths (the shape
        // `opstate::detect` uses) instead of a process per file.
        let mut cmd = GitCommand::new().cwd(workdir).arg("rev-parse");
        for rel in [
            "rebase-merge/head-name",
            "rebase-merge/onto",
            "rebase-apply/head-name",
            "rebase-apply/onto",
        ] {
            cmd = cmd.args(["--git-path", rel]);
        }
        let out = executor.run(cmd, cancel).await?;
        let mut values = [const { String::new() }; 4];
        for (slot, line) in values.iter_mut().zip(out.stdout_utf8().lines()) {
            *slot = std::fs::read_to_string(workdir.join(line.trim_end()))
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
        }
        let [merge_head, merge_onto, apply_head, apply_onto] = values;
        let head_name = if merge_head.is_empty() {
            apply_head
        } else {
            merge_head
        };
        let onto = if merge_onto.is_empty() {
            apply_onto
        } else {
            merge_onto
        };
        return Ok(Sides {
            ours: name_of(executor, workdir, &onto, cancel).await,
            theirs: short_ref(&head_name),
        });
    }

    let head = GitCommand::new()
        .cwd(workdir)
        .args(["rev-parse", "--abbrev-ref", "HEAD"]);
    let ours = executor
        .run(head, cancel)
        .await?
        .stdout_utf8()
        .trim()
        .into();
    // A detached HEAD names itself "HEAD", which says nothing about a
    // side; better to leave it to the caller's own wording.
    let ours = if ours == "HEAD" { String::new() } else { ours };

    let pseudo_ref = match op {
        InProgress::Merge => "MERGE_HEAD",
        InProgress::CherryPick => "CHERRY_PICK_HEAD",
        InProgress::Revert => "REVERT_HEAD",
        // Answered above. Named here rather than left to a panic: the
        // match stays exhaustive, and an empty rev names nothing.
        InProgress::Rebase => "",
    };
    Ok(Sides {
        ours,
        theirs: name_of(executor, workdir, pseudo_ref, cancel).await,
    })
}

/// Reads a file in the git directory, empty when it is not there. The
/// path is asked for rather than assumed: a worktree's git directory is
/// not `<workdir>/.git`.
pub(crate) async fn git_file(
    executor: &GitExecutor,
    workdir: &Path,
    rel: &str,
    cancel: &CancellationToken,
) -> String {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["rev-parse", "--git-path", rel]);
    let Ok(out) = executor.run(cmd, cancel).await else {
        return String::new();
    };
    let path = out.stdout_utf8().trim().to_string();
    std::fs::read_to_string(workdir.join(path))
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// The branch name a commit is reachable by, empty when git cannot name
/// one. An older commit comes back as `main~3`, which still says which
/// line of history it belongs to.
async fn name_of(
    executor: &GitExecutor,
    workdir: &Path,
    rev: &str,
    cancel: &CancellationToken,
) -> String {
    if rev.is_empty() {
        return String::new();
    }
    let cmd = GitCommand::new().cwd(workdir).args([
        "name-rev",
        "--name-only",
        "--refs=refs/heads/*",
        rev,
    ]);
    let Ok(out) = executor.run(cmd, cancel).await else {
        return String::new();
    };
    let name = out.stdout_utf8().trim().to_string();
    // git says "undefined" for a commit no branch reaches.
    if name == "undefined" {
        String::new()
    } else {
        name
    }
}

fn short_ref(full: &str) -> String {
    full.strip_prefix("refs/heads/").unwrap_or(full).to_string()
}

/// Keeps the temporary files git writes for the tool out of the working
/// tree. The default (false) puts `<file>_LOCAL_<pid>`, `_REMOTE_`,
/// `_BASE_` and `_BACKUP_` *beside* the conflicted file, so every launch
/// fills the pane with four to six untracked entries per path until the
/// tool is closed.
///
/// `mergetool.keepBackup` is deliberately left alone: the `<file>.orig`
/// its default leaves behind is not git's scratch space but the person's
/// safety net, and it is theirs to discard.
const MERGETOOL_ARGS: [&str; 2] = ["-c", "mergetool.writeToTemp=true"];

/// Launches the configured merge tool for `paths`, one at a time, and
/// stages each file the tool resolves (git does the `add` itself).
///
/// The tool is resolved here rather than passed in, so the name that is
/// launched is the one configured at this moment — a caller showing the
/// name in a menu cannot launch a stale one. With none configured this
/// refuses instead of running: git would otherwise guess a tool, and a
/// guessed tool makes it prompt on a stdin that is closed.
///
/// `paths` is never allowed to be empty. Bare `git mergetool` walks every
/// conflicted file in turn, and since the whole run holds the session's
/// write queue, that turns one launch into a queue blocked for as many
/// tool sessions as there are conflicts.
///
/// No time limit: the tool runs for as long as the person takes, and
/// cancelling the session is what stops it.
///
/// What a tool has to be is not "graphical" but "does not need the console
/// it was not given": on Windows the subprocess gets none
/// (CREATE_NO_WINDOW), which rules out anything that draws in a terminal
/// (vimdiff and its kind) and nothing else. A windowed tool works, and so
/// does a plain script that writes `$MERGED` — the merge tool contract is
/// the whole requirement.
pub async fn mergetool(
    executor: &GitExecutor,
    workdir: &Path,
    paths: &[String],
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    if paths.is_empty() {
        return Ok(());
    }
    let Some(tool) = configured_tool(executor, workdir, cancel).await? else {
        return Err(GitError::Rejected {
            message: "no merge tool is configured: set merge.guitool or merge.tool".to_string(),
        });
    };
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(MERGETOOL_ARGS)
        // `--gui` is what makes git resolve the tool the way
        // [`configured_tool`] reports it. Without it git reads `merge.tool`
        // alone, so someone who set only `merge.guitool` would be told a
        // tool is configured and then watch the launch fail.
        //
        // `--tool` is passed even so, because `--no-prompt` does not cover
        // the guessing path: with neither key set git picks a tool itself
        // and *then* asks on stdin to confirm, which closed stdin turns
        // into a failed file.
        .args(["mergetool", "--gui", "--no-prompt"])
        .arg(format!("--tool={tool}"))
        .arg("--")
        .args(paths.iter().map(|p| literal_pathspec(p)))
        .no_timeout();
    executor.run(cmd, cancel).await.map(drop)
}

/// The merge tool git would launch (`merge.guitool`, else `merge.tool`).
/// `None` means none is configured and `mergetool` would fail.
pub async fn configured_tool(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Option<String>, GitError> {
    for key in ["merge.guitool", "merge.tool"] {
        let cmd = GitCommand::new()
            .cwd(workdir)
            // Neither key being set answers with code 1, which is an
            // answer — and is what a machine that configured no merge tool
            // says to both of them.
            .answers_by_code(1)
            .args(["config", "--get", key]);
        let out = executor.run_unchecked(cmd, cancel).await?;
        if out.code == 0 {
            let value = out.stdout_utf8().trim().to_string();
            if !value.is_empty() {
                return Ok(Some(value));
            }
        }
    }
    Ok(None)
}

/// The answer, read once per process: what is installed on the machine
/// does not change while the app is open, and at eight seconds it is not
/// a read to repeat per tab or per dialog. Errors are not stored, so a
/// read that timed out can be tried again.
static INSTALLED: tokio::sync::OnceCell<Vec<String>> = tokio::sync::OnceCell::const_new();

/// Merge tools git found installed and this app can actually launch.
///
/// **Slow on Windows** — around 8 seconds, measured warm. `--tool-help`
/// sources every one of git's ~25 tool definitions twice and probes each
/// one's availability, which on Windows means walking the registry and
/// Program Files. Never put this on the write queue, and never let
/// anything wait on it.
///
/// Only the first group is read (what is installed); the second group
/// lists tools git knows of but cannot find, which is not an offer worth
/// making. User-defined tools are skipped here — [`user_defined_tools`]
/// names them from config without the eight seconds, and reads them from
/// a key rather than out of prose.
///
/// **Tools that draw in a terminal are dropped.** The subprocess gets no
/// console, so vimdiff and its kind cannot run, and offering them is
/// offering a dead end. git marks the rest itself, in the description it
/// prints. That marker is prose, and the price of it changing is this
/// returning nothing — which lands on the plain text field the caller
/// already has. It also loses `emerge`, which a graphical Emacs would run
/// fine; typing the name still works.
pub async fn available_tools(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Vec<String>, GitError> {
    let names = INSTALLED
        .get_or_try_init(|| async {
            let cmd = GitCommand::new()
                .cwd(workdir)
                .args(["mergetool", "--tool-help"]);
            let out = executor.run(cmd, cancel).await?;
            Ok::<_, GitError>(parse_tool_help(&out.stdout_utf8()))
        })
        .await?;
    Ok(names.clone())
}

/// Names from the installed group of `git mergetool --tool-help`, keeping
/// only the ones git marks as windowed.
pub(crate) fn parse_tool_help(text: &str) -> Vec<String> {
    const GROUP: &str = "may be set to one of the following:";
    const WINDOWED: &str = "(requires a graphical session)";

    let mut names = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        if !inside {
            inside = line.contains(GROUP);
            continue;
        }
        // Entries are indented by two tabs. A blank line separates the
        // user-defined block; anything else at a shallower indent is that
        // block's heading or the next group's preamble — either way the
        // installed list has ended.
        let Some(entry) = line.strip_prefix("\t\t") else {
            if line.trim().is_empty() {
                continue;
            }
            break;
        };
        if !entry.contains(WINDOWED) {
            continue;
        }
        if let Some(name) = entry.split_whitespace().next() {
            names.push(name.to_string());
        }
    }
    names
}

/// Merge tools defined in config (`mergetool.<name>.cmd`).
///
/// Cheap where [`available_tools`] is not, and read from a key rather than
/// from prose. **Not filtered**: someone who wrote a `cmd` chose it, and a
/// script that writes `$MERGED` satisfies the whole contract without
/// needing a window or a console.
pub async fn user_defined_tools(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Vec<String>, GitError> {
    let out = config::get_regexp(
        executor,
        workdir,
        r"^mergetool\..*\.cmd$",
        "git config --get-regexp",
        cancel,
    )
    .await?;
    // Only the key is read: the command itself is git's to run, and a
    // `cmd` written with no value still named a tool.
    let mut names = Vec::new();
    for record in config::parse_z_records(&out) {
        let name = record
            .key()
            .trim()
            .strip_prefix("mergetool.")
            .and_then(|rest| rest.strip_suffix(".cmd"))
            .unwrap_or_default();
        if !name.is_empty() {
            names.push(name.to_string());
        }
    }
    Ok(names)
}

/// Records which merge tool to launch, or clears the choice when `tool`
/// is empty.
///
/// Written to `merge.guitool`, not `merge.tool`, for two reasons that
/// point the same way. It is the key that takes effect: launches pass
/// `--gui`, under which git reads `guitool` first, so writing `tool`
/// would silently do nothing for anyone who already set `guitool`. And it
/// is the key that belongs to this app: `merge.tool` is what their
/// terminal `git mergetool` uses, and choosing a windowed tool here has
/// no business changing that — a terminal tool cannot run under this app
/// at all (no console), while `merge.tool` may well name one.
///
/// Always global. Which editor someone reaches for is a property of their
/// desk, not of one repository.
///
/// Clearing does not necessarily leave nothing configured: `merge.tool`
/// may still be set, and [`configured_tool`] will then report it. That is
/// the honest answer, since it is what git would launch.
pub async fn set_merge_tool(
    executor: &GitExecutor,
    workdir: &Path,
    tool: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let tool = tool.trim();
    if tool.is_empty() {
        let cmd = GitCommand::new()
            .cwd(workdir)
            // "nothing was set" comes back as code 5, which is the same
            // outcome as clearing rather than a failure to report.
            .answers_by_code(5)
            .args(["config", "--global", "--unset", "merge.guitool"]);
        let out = executor.run_unchecked(cmd, cancel).await?;
        return match out.code {
            0 | 5 => Ok(()),
            _ => Err(GitError::Rejected {
                message: out.failure_message(),
            }),
        };
    }
    // No `--` separator: `git config <key> -- <value>` stores "--" as the
    // value (the same trap `identity::set_identity` documents).
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["config", "--global", "merge.guitool", tool]);
    executor.run(cmd, cancel).await.map(drop)
}

/// Resolves a conflict by taking one side wholesale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The branch being merged into (`--ours`).
    Ours,
    /// The branch being merged in (`--theirs`).
    Theirs,
}

/// `git restore --ours|--theirs` followed by `git add`, which is what
/// "take this side" means to git (restore writes the chosen stage into
/// the working tree and leaves the path unmerged; the add resolves it —
/// measured, identical to the older `checkout --ours` spelling).
///
/// Note the reversal during a rebase: the commits being replayed are
/// "theirs", so `Ours` is the upstream side the caller is landing on.
pub async fn take_side(
    executor: &GitExecutor,
    workdir: &Path,
    paths: &[String],
    side: Side,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    if paths.is_empty() {
        return Ok(());
    }
    let flag = match side {
        Side::Ours => "--ours",
        Side::Theirs => "--theirs",
    };
    let specs: Vec<String> = paths.iter().map(|p| literal_pathspec(p)).collect();
    let restore = GitCommand::new()
        .cwd(workdir)
        .args(["restore", flag, "--"])
        .args(specs.iter().map(String::as_str));
    executor.run(restore, cancel).await?;
    let add = GitCommand::new()
        .cwd(workdir)
        .args(["add", "--"])
        .args(specs.iter().map(String::as_str));
    executor.run(add, cancel).await.map(drop)
}
