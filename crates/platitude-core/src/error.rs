//! Error types for git execution and repository access.

use std::path::PathBuf;
use std::time::Duration;

use crate::report::{ReportKind, WriteReport};

/// Errors produced while locating, spawning or running the git CLI.
///
/// **These sentences are read as they are written.** Every one of them
/// reaches a screen — the error line, the notice bar, the command log,
/// the band's state card — and nothing on the way renders markup, so a
/// command wrapped in backticks arrives with the backticks in it
/// (デザイン規約 §git 用語のコード表記: the code chip is a menu row's form,
/// and it is a chip). Punctuate them the way a sentence is
/// punctuated, and leave the marking up to whoever draws them.
#[derive(Debug, thiserror::Error)]
pub enum GitError {
    /// The git executable could not be found on PATH.
    #[error("git executable not found on PATH")]
    GitNotFound {
        #[source]
        source: std::io::Error,
    },

    /// Spawning the subprocess failed for a reason other than a missing binary.
    #[error("failed to spawn {command}: {source}")]
    Spawn {
        command: String,
        #[source]
        source: std::io::Error,
    },

    /// I/O failure while talking to a running subprocess.
    #[error("i/o error while running {command}: {source}")]
    Io {
        command: String,
        #[source]
        source: std::io::Error,
    },

    /// The command ran but exited with a non-zero status. `stderr` is passed
    /// through untouched: the app presents git's own message to the user.
    #[error("{command} exited with code {code}: {stderr}")]
    Failed {
        command: String,
        code: i32,
        stderr: String,
    },

    /// A write that did not happen and has something to say for itself:
    /// the far side turned it down under a rule of its own, a hook here
    /// did, or git worked out from what this end holds that it could not
    /// stand. Nothing was half done and there is nothing here to put
    /// right, so the screen reports it
    /// (デザイン規約 §答えの要らない報せ).
    ///
    /// Reads the same as [`GitError::Failed`] wherever it is only
    /// being logged: `report` is the part the screen is made out
    /// of, and which of them this is lives in
    /// [`ReportKind`].
    ///
    /// Boxed because every `Result<_, GitError>` in the crate carries the
    /// widest variant, and four more strings here would put that cost on
    /// reads that can never be refused by anybody (`result_large_err`).
    #[error("{command} exited with code {code}: {stderr}")]
    Reported {
        command: String,
        code: i32,
        stderr: String,
        report: Box<WriteReport>,
    },

    /// The command exceeded its time budget and was killed.
    #[error("{command} timed out after {timeout:?}")]
    TimedOut { command: String, timeout: Duration },

    /// The command was cancelled (repository switch, app shutdown, ...).
    #[error("{command} was cancelled")]
    Cancelled { command: String },

    /// The given path is not inside a git repository (or does not exist).
    ///
    /// `bare` separates the one folder that *is* a repository and still
    /// cannot be opened: a bare one has no work tree to show. The screen
    /// says something different for it, and this is how it knows — git's
    /// wording is for people.
    #[error("not a git repository: {}", path.display())]
    NotARepository {
        path: PathBuf,
        stderr: String,
        bare: bool,
    },

    /// The command succeeded but printed something we cannot interpret.
    #[error("unexpected output from {command}: {message}")]
    UnexpectedOutput { command: String, message: String },

    /// A message written here: a value the application refused to hand
    /// over, or a write git raised nothing against that did not take
    /// effect all the same. Shown as is.
    #[error("{message}")]
    Rejected { message: String },

    /// A write this end worked out could not stand, and stopped before
    /// git — carrying the [`WriteReport`] the screen states it from.
    ///
    /// The same pair as [`GitError::Failed`] and [`GitError::Reported`],
    /// one step earlier: those two are git's answer with and without
    /// something to report, these two are this end's own. **Nothing ran**,
    /// which is the whole of why this variant names no command and no exit
    /// code — putting one in front of the reader would name a command they
    /// never ran (デザイン規約 §git が言ったことを読む場所).
    #[error("{message}")]
    Withheld {
        message: String,
        report: Box<WriteReport>,
    },
}

impl GitError {
    /// True when the error is a cooperative cancellation.
    pub fn is_cancelled(&self) -> bool {
        matches!(self, GitError::Cancelled { .. })
    }

    /// What this failure has to say for itself, where it is one of the
    /// ones that does.
    pub fn report(&self) -> Option<&WriteReport> {
        match self {
            GitError::Reported { report, .. } | GitError::Withheld { report, .. } => Some(report),
            _ => None,
        }
    }

    /// Whether git would not send because what this end holds about the
    /// remote is older than the remote itself — the one report with a
    /// move behind it (`RepoSession::push` fetches on it).
    pub fn is_outdated(&self) -> bool {
        self.report()
            .is_some_and(|report| report.kind == ReportKind::Outdated)
    }
}
