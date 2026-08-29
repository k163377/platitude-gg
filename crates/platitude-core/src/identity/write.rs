//! Handing git an identity, and reading back what it holds afterwards.

use super::*;

/// What a [`set_identity`] left behind.
///
/// Read back from git rather than echoed, for two reasons. An identity is
/// two `git config` calls and the lock on the configuration file is taken
/// and released per call, so the second one can fail on its own and leave
/// half of an identity — which reads as a whole one to everything
/// downstream, because both halves are set. And a repository-local
/// setting can sit over a global write, so even two calls that both
/// succeeded do not say what a commit will carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityWrite {
    /// What git reports now, at the level the write was aimed at: the
    /// effective identity for [`set_identity`], and the repository's own
    /// file alone for [`super::set_local_identity`] — which is the only
    /// level that can say whether an override is there at all.
    pub identity: Identity,
    /// git reports the name that was asked for.
    pub name_saved: bool,
    /// git reports the address that was asked for.
    pub email_saved: bool,
    /// git's own message from the last call that failed; empty when every
    /// one of them succeeded. A write can end unsaved with nothing here —
    /// that is the local setting sitting over the global one.
    pub message: String,
}

impl IdentityWrite {
    /// Both halves are what was asked for. Anything less is a half-written
    /// identity and must not be shown as a finished one.
    pub fn is_saved(&self) -> bool {
        self.name_saved && self.email_saved
    }
}

/// Records `user.name` and `user.email`, and answers what git holds after.
///
/// The pair is not atomic and cannot be made so: `git config` writes one
/// key per invocation and has no batch form, `--edit` holds no lock at all
/// (measured: a concurrent write lands while the editor is open), and
/// taking the lock directly would mean reimplementing git's own. So the
/// write reads itself back and, when what came out is not what was asked
/// for, goes again — once. Every one of these writes is idempotent, and
/// losing the lock is somebody else holding it for the moment it took to
/// spawn the second call, so a second pass settles contention. A failure
/// that is not contention answers the same way however often it is asked,
/// and the screen is a better place to wait than a loop.
///
/// Only an empty name is refused, because that is the only thing git
/// itself refuses — and it refuses it at commit time ("Author identity
/// unknown"), long after the value was entered. Everything else git
/// handles on its own, so nothing here second-guesses it (measured
/// against git 2.51):
///
/// - `<`, `>` and newlines are dropped when git builds an author line, and
///   leading/trailing spaces and punctuation are stripped
/// - a newline in a value is escaped as `\n` when git writes the config
///   file, so it cannot smuggle in another setting
/// - an empty *email* is accepted; the author line simply carries `<>`
///
/// Values are trimmed, which only anticipates what git does to them
/// anyway, and keeps the stored configuration equal to what commits show.
pub async fn set_identity(
    executor: &GitExecutor,
    workdir: &Path,
    name: &str,
    email: &str,
    scope: ConfigScope,
    cancel: &CancellationToken,
) -> Result<IdentityWrite, GitError> {
    let name = name.trim();
    let email = email.trim();
    if name.is_empty() {
        return Err(GitError::Rejected {
            message: "the name must not be empty".to_string(),
        });
    }

    let written = write_pair(executor, workdir, name, email, scope, cancel).await?;
    if written.is_saved() {
        return Ok(written);
    }
    write_pair(executor, workdir, name, email, scope, cancel).await
}

/// One pass: both keys, then what git makes of them.
///
/// Stops at the first call that fails — the second would be writing into a
/// configuration this pass has just been told it cannot change — but reads
/// back either way, because a call that failed says nothing about what the
/// file now holds.
async fn write_pair(
    executor: &GitExecutor,
    workdir: &Path,
    name: &str,
    email: &str,
    scope: ConfigScope,
    cancel: &CancellationToken,
) -> Result<IdentityWrite, GitError> {
    let mut message = String::new();
    for (key, value) in [("user.name", name), ("user.email", email)] {
        let mut cmd = GitCommand::new().cwd(workdir).arg("config");
        if scope == ConfigScope::Global {
            cmd = cmd.arg("--global");
        }
        // No `--` separator: `git config <key> -- <value>` stores "--" as
        // the value. A leading dash in the value is accepted as-is.
        cmd = cmd.args([key, value]);
        if let Err(e) = executor.run(cmd, cancel).await {
            // Shutting down is not a write that failed; it is no write.
            if e.is_cancelled() {
                return Err(e);
            }
            message = e.to_string();
            break;
        }
    }
    let config = load(executor, workdir, cancel).await?;
    Ok(IdentityWrite {
        name_saved: config.identity.name.as_deref() == Some(name),
        email_saved: config.identity.email.as_deref() == non_empty(email).as_deref(),
        identity: config.identity,
        message,
    })
}
