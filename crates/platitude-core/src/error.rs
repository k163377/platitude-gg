//! Error types for git execution and repository access.

use std::path::PathBuf;
use std::time::Duration;

/// Errors produced while locating, spawning or running the git CLI.
#[derive(Debug, thiserror::Error)]
pub enum GitError {
    /// The git executable could not be found on PATH.
    #[error("git executable not found on PATH")]
    GitNotFound {
        #[source]
        source: std::io::Error,
    },

    /// Spawning the subprocess failed for a reason other than a missing binary.
    #[error("failed to spawn `{command}`: {source}")]
    Spawn {
        command: String,
        #[source]
        source: std::io::Error,
    },

    /// I/O failure while talking to a running subprocess.
    #[error("i/o error while running `{command}`: {source}")]
    Io {
        command: String,
        #[source]
        source: std::io::Error,
    },

    /// The command ran but exited with a non-zero status. `stderr` is passed
    /// through untouched: the app presents git's own message to the user.
    #[error("`{command}` exited with code {code}: {stderr}")]
    Failed {
        command: String,
        code: i32,
        stderr: String,
    },

    /// A push git refused because what this window knows about the remote is
    /// older than the remote itself (`fetch first` on a plain push, `stale
    /// info` on a lease pinned to a commit the remote has left). Fetching is
    /// what unblocks it, so it is told apart from a refusal nothing can be
    /// done about — a hook, a protected branch, an unreachable host.
    ///
    /// Reads the same as [`GitError::Failed`]: it is the same failure to
    /// whoever is looking at it, only actionable.
    #[error("`{command}` exited with code {code}: {stderr}")]
    PushOutdated {
        command: String,
        code: i32,
        stderr: String,
    },

    /// The command exceeded its time budget and was killed.
    #[error("`{command}` timed out after {timeout:?}")]
    TimedOut { command: String, timeout: Duration },

    /// The command was cancelled (repository switch, app shutdown, ...).
    #[error("`{command}` was cancelled")]
    Cancelled { command: String },

    /// Installed git is older than the supported minimum.
    #[error("unsupported git version {found} (minimum supported is {minimum})")]
    UnsupportedVersion { found: String, minimum: String },

    /// The given path is not inside a git repository (or does not exist).
    ///
    /// `bare` separates the one folder that *is* a repository and still
    /// cannot be opened: a bare one has no work tree to show. The screen
    /// says something different for it, and this is how it knows — git's
    /// wording is for people, not for branching on.
    #[error("not a git repository: {}", path.display())]
    NotARepository {
        path: PathBuf,
        stderr: String,
        bare: bool,
    },

    /// The command succeeded but printed something we cannot interpret.
    #[error("unexpected output from `{command}`: {message}")]
    UnexpectedOutput { command: String, message: String },

    /// A value the application refused to hand to git. git never ran, so
    /// there is no message of its own to pass through — this one is
    /// written for the person who typed the value, and is shown as is.
    #[error("{message}")]
    Rejected { message: String },
}

impl GitError {
    /// True when the error is a cooperative cancellation, not a failure.
    pub fn is_cancelled(&self) -> bool {
        matches!(self, GitError::Cancelled { .. })
    }
}
