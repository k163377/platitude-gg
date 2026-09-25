//! Handing resolution over to `git mergetool`: which tool git would
//! launch, what is installed to choose from, and the launch itself. A
//! built-in conflict editor is out of scope (実装計画.md §1 スコープ外「内蔵conflictエディタ」).

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::config;
use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor, literal_pathspec};

/// Keeps the tool's temporary files (`<file>_LOCAL_<pid>` …) out of the
/// working tree, where they would show as untracked until the tool closes.
/// `mergetool.keepBackup` is left alone: the `.orig` is the person's safety
/// net.
const MERGETOOL_ARGS: [&str; 2] = ["-c", "mergetool.writeToTemp=true"];

/// Launches the configured merge tool for `paths`, one at a time; git
/// stages each file the tool resolves.
///
/// The tool is resolved at launch, so a name shown earlier cannot be stale;
/// with none configured this refuses. Empty `paths` does nothing — bare
/// `git mergetool` walks every conflict while holding the write queue.
///
/// Runs until the person is done; cancelling the session stops it. The
/// tool must not need a console (no terminal tools such as vimdiff); a
/// windowed tool or a script that writes `$MERGED` both work.
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
        // `--gui` makes git resolve the tool as [`configured_tool`] does
        // (without it a lone `merge.guitool` is ignored). `--tool` too:
        // `--no-prompt` does not cover git's guessing path, which asks on
        // the closed stdin.
        .args(["mergetool", "--gui", "--no-prompt"])
        .arg(format!("--tool={tool}"))
        .arg("--")
        .args(paths.iter().map(|p| literal_pathspec(p)))
        .no_timeout()
        // Paced by the person in the tool (`process::Pace`).
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

/// [`available_tools`]' answer, read once per process: what is installed
/// does not change while the app is open, and the read takes seconds.
/// Errors are not stored, so a timed-out read can be tried again.
static INSTALLED: tokio::sync::OnceCell<Vec<String>> = tokio::sync::OnceCell::const_new();

/// Merge tools git found installed and this app can launch. User-defined
/// tools are [`user_defined_tools`]'s.
///
/// Seconds on Windows (ci/baseline/code-costs-windows-x64.md): keep it off
/// the write queue, with nothing waiting on it.
///
/// Terminal tools are dropped (there is no console), by git's own prose
/// marker; if that wording changes this returns nothing, which falls back
/// to the caller's text field. `emerge` is lost too; typing it still works.
///
/// The pre-merge tests mock this (`mock_available_tools`); mry copies what
/// a mock matches on, so the borrowed arguments are skipped.
#[mry::mry(skip_args(GitExecutor, Path, CancellationToken))]
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
        // Entries are indented by two tabs; anything shallower and not
        // blank (the user-defined heading, the next group) ends the list.
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

/// Merge tools defined in config (`mergetool.<name>.cmd`), all offered: a
/// script that writes `$MERGED` needs neither window nor console.
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
    // Only the key is read: a `cmd` with no value still names a tool.
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

/// Records which merge tool to launch (global `merge.guitool`), or clears
/// the choice when `tool` is empty.
///
/// `guitool`, not `tool`: launches pass `--gui`, under which `guitool`
/// wins, so writing `tool` would do nothing for anyone who set `guitool`.
/// Clearing can leave `merge.tool` for [`configured_tool`] to report — it
/// is what git would launch.
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
            // Exit 5 is "nothing was set" — the same outcome as clearing.
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
    // No `--`: `git config <key> -- <value>` stores "--" as the value.
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["config", "--global", "merge.guitool", tool]);
    executor.run(cmd, cancel).await.map(drop)
}
