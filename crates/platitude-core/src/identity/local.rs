//! The identity one repository sets for itself.
//!
//! git already keeps this per repository, so nothing here is a second
//! copy of it (実装計画.md §設定はマシン単位で持つ): a name or an
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
/// alike — an empty `user.email` puts `<>` on the author line — but
/// nothing in this app can write that, and a box that showed it would
/// be a box whose empty state meant two different things.
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
/// where **an empty value asks for the key to be taken
/// out**.
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

/// Which of the two keys a pass has to touch, and with what — the whole
/// of the decision, with no git in it.
///
/// **What the file already says is read first**, and a key that already
/// says it is left alone. Not an optimisation: `git config --unset` fails
/// when there was nothing to unset, and it fails with the *same* exit code
/// as its refusal to touch a key written more than once (measured git 2.55:
/// both are 5). Asking first is what keeps those two apart — after it, a
/// failed unset is a real one and is reported as such.
///
/// Separate from the pass that runs it because the combinations are
/// where the rule lives — two keys, each of them held or not and wanted
/// or not — and every one of them costs a repository and up to three git
/// processes to ask through. What the pass around it still owes a real
/// git is the other half: that this list is what actually goes out, and
/// what happens when one of them is refused.
fn keys_to_write<'a>(
    held: &Identity,
    name: &'a str,
    email: &'a str,
) -> Vec<(&'static str, &'a str)> {
    [
        ("user.name", name, held.name.as_deref()),
        ("user.email", email, held.email.as_deref()),
    ]
    .into_iter()
    .filter(|(_, wanted, held_value)| *held_value != non_empty(wanted).as_deref())
    .map(|(key, wanted, _)| (key, wanted))
    .collect()
}

/// One pass: the keys [`keys_to_write`] names, then what the file holds
/// after.
async fn write_local_pair(
    executor: &GitExecutor,
    workdir: &Path,
    name: &str,
    email: &str,
    cancel: &CancellationToken,
) -> Result<IdentityWrite, GitError> {
    let held = load_local(executor, workdir, cancel).await?;
    let mut message = String::new();
    for (key, wanted) in keys_to_write(&held, name, email) {
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
/// default (measured git 2.55: a bare `--unset` of a key held only in the
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

#[cfg(test)]
mod tests {
    use super::{Identity, keys_to_write};

    /// What one repository's own file holds, in the shape the read hands
    /// back: an absent key is a key nobody wrote here.
    fn held(name: Option<&str>, email: Option<&str>) -> Identity {
        Identity {
            name: name.map(str::to_string),
            email: email.map(str::to_string),
        }
    }

    /// A save that asks for what the file already says asks git for
    /// nothing. Both halves of it: a value that matches, and the blank
    /// box over a key nobody wrote — the second is the one that would
    /// otherwise reach a `--unset` git refuses, with the exit code it
    /// refuses a twice-written key with.
    #[test]
    fn a_save_that_changes_nothing_writes_nothing() {
        assert!(
            keys_to_write(
                &held(Some("Ada"), Some("ada@example.com")),
                "Ada",
                "ada@example.com"
            )
            .is_empty()
        );
        assert!(keys_to_write(&Identity::default(), "", "").is_empty());
        assert!(
            keys_to_write(&held(Some("Ada"), None), "Ada", "").is_empty(),
            "one key written and one not, each asked for what it already is"
        );
    }

    /// One key at a time, which is the errand the whole level exists for:
    /// another address for this project under the name the person already
    /// goes by. The other key is not touched — not written, and not taken
    /// out either.
    #[test]
    fn one_key_moves_without_the_other() {
        assert_eq!(
            keys_to_write(
                &held(Some("Ada"), Some("ada@example.com")),
                "Ada",
                "work@example.com"
            ),
            vec![("user.email", "work@example.com")]
        );
        assert_eq!(
            keys_to_write(
                &held(Some("Ada"), Some("ada@example.com")),
                "Ada Lovelace",
                "ada@example.com"
            ),
            vec![("user.name", "Ada Lovelace")]
        );
    }

    /// Both, in the order the pass writes them — which is the order a
    /// half-landed write is reported in.
    #[test]
    fn both_keys_move_together_and_in_order() {
        assert_eq!(
            keys_to_write(&Identity::default(), "Ada", "ada@example.com"),
            vec![("user.name", "Ada"), ("user.email", "ada@example.com")]
        );
    }

    /// An emptied box takes its key out, and the two directions mix: a
    /// pass may be taking one key out while it writes the other.
    #[test]
    fn an_emptied_box_takes_its_key_out_beside_one_being_written() {
        assert_eq!(
            keys_to_write(&held(Some("Ada"), Some("ada@example.com")), "", ""),
            vec![("user.name", ""), ("user.email", "")]
        );
        assert_eq!(
            keys_to_write(
                &held(Some("Ada"), Some("ada@example.com")),
                "",
                "work@example.com"
            ),
            vec![("user.name", ""), ("user.email", "work@example.com")]
        );
        assert_eq!(
            keys_to_write(&held(None, Some("ada@example.com")), "Ada", ""),
            vec![("user.name", "Ada"), ("user.email", "")]
        );
    }
}
