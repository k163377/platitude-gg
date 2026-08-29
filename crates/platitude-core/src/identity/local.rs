//! The identity one repository sets for itself.
//!
//! git already keeps this per repository, so nothing here is a second
//! copy of it (実装計画.md §リポジトリ毎の設定は持たない): a name or an
//! address written through this module goes into that repository's own
//! configuration file and is read back out of the same one.
//!
//! **Empty means "not written here"**, and the two keys are independent —
//! which is the whole errand. The common case is one address for work and
//! the name unchanged, so a repository overrides one key and inherits the
//! other, and the screen has to be able to say both.

use super::*;

/// Keys this level is asked about.
///
/// The signing keys are not among them: what a repository overrides is
/// whom its commits are from, and a key that cannot be edited here has no
/// business arriving as though it could.
const LOCAL_PATTERN: &str = r"^user\.(name|email)$";

/// What this repository's own configuration file sets, and nothing else.
///
/// Not [`load`], which answers with what git would use here: a value that
/// is only inherited from the user's own file arrives there spelled
/// exactly like one this repository wrote down, and telling those two
/// apart is the question this module exists to answer.
///
/// A key written with an empty value reads as unset, the same as
/// everywhere else the identity is parsed. git does not treat the two
/// alike — an empty `user.email` puts `<>` on the author line rather than
/// falling back — but nothing in this app can write that, and a box that
/// showed it would be a box whose empty state meant two different things.
pub async fn load_local(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Identity, GitError> {
    let out = config::get_regexp_at(
        executor,
        workdir,
        ConfigScope::Local,
        LOCAL_PATTERN,
        "git config --local --get-regexp user",
        cancel,
    )
    .await?;
    Ok(parse_config(&out).identity)
}

/// Records `user.name` / `user.email` in this repository's own file,
/// where **an empty value asks for the key to be taken out** rather than
/// set to nothing.
///
/// What comes back is what that file holds afterwards ([`load_local`]),
/// not what git would use — the question the screen asked was which of
/// the two keys this repository sets for itself, and the answer to that
/// cannot be read at the effective level.
///
/// Contention is settled the way [`set_identity`] settles it, and for the
/// same reason: two keys are two invocations, the lock is taken and
/// released per invocation, so the write reads itself back and goes again
/// once when what came out is not what was asked for.
pub async fn set_local_identity(
    executor: &GitExecutor,
    workdir: &Path,
    name: &str,
    email: &str,
    cancel: &CancellationToken,
) -> Result<IdentityWrite, GitError> {
    let name = name.trim();
    let email = email.trim();
    let written = write_local_pair(executor, workdir, name, email, cancel).await?;
    if written.is_saved() {
        return Ok(written);
    }
    write_local_pair(executor, workdir, name, email, cancel).await
}

/// One pass: whichever of the two keys is not already what was asked for,
/// then what the file holds after.
///
/// **What the file already says is read first**, and a key that already
/// says it is left alone. Not an optimisation: `git config --unset` fails
/// when there was nothing to unset, and it fails with the *same* exit code
/// as its refusal to touch a key written more than once (実測 git 2.55:
/// both are 5). Asking first is what keeps those two apart — after it, a
/// failed unset is a real one and is reported as such.
async fn write_local_pair(
    executor: &GitExecutor,
    workdir: &Path,
    name: &str,
    email: &str,
    cancel: &CancellationToken,
) -> Result<IdentityWrite, GitError> {
    let held = load_local(executor, workdir, cancel).await?;
    let mut message = String::new();
    for (key, wanted, held_value) in [
        ("user.name", name, held.name.as_deref()),
        ("user.email", email, held.email.as_deref()),
    ] {
        if held_value == non_empty(wanted).as_deref() {
            continue;
        }
        // Stops at the first that fails, the way the global write does:
        // the second would be writing into a file this pass has just been
        // told it cannot change.
        if let Err(e) = write_local_key(executor, workdir, key, wanted, cancel).await {
            // Shutting down is not a write that failed; it is no write.
            if e.is_cancelled() {
                return Err(e);
            }
            message = e.to_string();
            break;
        }
    }
    let identity = load_local(executor, workdir, cancel).await?;
    Ok(IdentityWrite {
        name_saved: identity.name.as_deref() == non_empty(name).as_deref(),
        email_saved: identity.email.as_deref() == non_empty(email).as_deref(),
        identity,
        message,
    })
}

/// One key, written or taken out.
///
/// `--local` is spelled out on both, though git already writes there by
/// default (実測 git 2.55: a bare `--unset` of a key held only in the
/// user's own file exits 5 and leaves that file alone). The level a value
/// is read back from and the level it is written at are then named by the
/// same word, which is what stops the two from drifting apart later.
async fn write_local_key(
    executor: &GitExecutor,
    workdir: &Path,
    key: &str,
    value: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = GitCommand::new().cwd(workdir);
    let cmd = if value.is_empty() {
        cmd.args(["config", "--local", "--unset", key])
    } else {
        // No `--` separator: `git config <key> -- <value>` stores "--" as
        // the value (the trap `set_identity` documents).
        cmd.args(["config", "--local", key, value])
    };
    executor.run(cmd, cancel).await.map(drop)
}
