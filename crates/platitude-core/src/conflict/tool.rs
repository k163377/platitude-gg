//! Handing resolution over to `git mergetool`: which tool git would
//! launch, what is installed to choose from, and the launch itself.
//!
//! Resolving conflicts is explicitly out of scope for this application
//! (実装計画.md §1 スコープ外「内蔵conflictエディタ」): the built-in editor belongs to
//! whatever tool the user already configured, so everything here is about
//! reaching that tool and nothing about the merge.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::config;
use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor, literal_pathspec};

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
        .no_timeout()
        // Paced by the person in the tool, so the slot it sits in for
        // as long as they take is never the click's (`process::Pace`).
        .paced_elsewhere();
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
/// **Slow on Windows** — seconds, not milliseconds
/// (ci/baseline/code-costs-windows-x64.md). `--tool-help` sources every
/// one of git's tool definitions twice and probes each one's
/// availability, which on Windows means walking the registry and Program
/// Files. Never put this on the write queue, and never let anything wait
/// on it.
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
