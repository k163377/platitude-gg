//! Handing git an identity, and reading back what it holds afterwards.

use super::*;

/// What a [`set_identity`] left behind, read back from git: the second of
/// the two `git config` calls can fail alone and leave half an identity,
/// and a repository-local setting can sit over a global write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityWrite {
    /// What git reports now at the level written: effective for
    /// [`set_identity`], the repository's own file alone for
    /// [`super::set_local_identity`] (the only level that shows an override).
    pub identity: Identity,
    /// git reports the name that was asked for.
    pub name_saved: bool,
    /// git reports the address that was asked for.
    pub email_saved: bool,
    /// git's message from the call that failed; empty when all succeeded.
    /// Unsaved with no message = a local setting sits over the global one.
    pub message: String,
}

impl IdentityWrite {
    /// Both halves are what was asked for; anything less shows as
    /// half-written.
    pub fn is_saved(&self) -> bool {
        self.name_saved && self.email_saved
    }
}

/// Records `user.name` and `user.email`, and answers what git holds after.
///
/// The pair cannot be made atomic
/// (rules-refs/core.md「identity の 2 連書きは原子化できない」), so the write
/// reads itself back and goes again once when it differs: the writes are
/// idempotent and contention clears by the second pass, while any other
/// failure answers the same however often asked.
///
/// Only an empty name is refused — the only thing git refuses, and only at
/// commit time ("Author identity unknown"); the rest is git's to handle
/// (rules-refs/core.md「検証は git に任せる」). Values are trimmed, as git
/// would, so the stored configuration equals what commits show.
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

/// One pass: both keys, then a read-back. Stops at the first failing call
/// but reads back either way — a failed call says nothing about what the
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
        // No `--`: `git config <key> -- <value>` stores "--" as the value;
        // a leading dash in the value is accepted as-is.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::refusing;

    /// The executor runs nothing and the path is no repository, so a write
    /// that got past the refusal fails too: the message and the count tell
    /// the two apart.
    #[tokio::test]
    async fn an_empty_name_is_refused_before_git_is_asked() {
        let (exec, asked) = refusing::git();
        let cancel = CancellationToken::new();

        let err = set_identity(
            &exec,
            &refusing::nowhere(),
            "   ",
            "e@example.com",
            ConfigScope::Local,
            &cancel,
        )
        .await
        .expect_err("an empty name is refused");

        assert!(err.to_string().contains("must not be empty"), "{err}");
        assert_eq!(asked.count(), 0, "nothing was asked of git");
    }
}
