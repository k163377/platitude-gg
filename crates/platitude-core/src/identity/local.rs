//! The identity one repository sets for itself, written to and read back
//! from that repository's own configuration file — git's copy, not a
//! second one (実装計画.md §7「設定はマシン単位で持つ」).
//!
//! Empty means "not written here", and the two keys are independent: the
//! common case overrides the address and inherits the name.

use super::*;

/// Keys this level is asked about — not the signing keys, which cannot be
/// edited here.
const LOCAL_PATTERN: &str = r"^user\.(name|email)$";

/// What this repository's own configuration file sets, and nothing else —
/// not [`load`], where an inherited value looks exactly like one written
/// here (rules-refs/core.md「`--local` でしか読めない」).
///
/// A key written with an empty value reads as unset. git would put `<>`
/// on the author line for an empty `user.email`, but this app never
/// writes one, and an empty box must mean one thing.
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

/// Records `user.name` / `user.email` in this repository's own file; an
/// empty value takes the key out. Answers what that file holds afterwards
/// ([`load_local`]), not what git would use.
///
/// Contention is settled as in [`set_identity`]: two keys are two
/// invocations, so the write reads itself back and goes again once when
/// it differs.
pub async fn set_local_identity(
    executor: &GitExecutor,
    workdir: &Path,
    values: &IdentityValues,
    cancel: &CancellationToken,
) -> Result<IdentityWrite, GitError> {
    let written = write_local_pair(executor, workdir, values, cancel).await?;
    if written.is_saved() {
        return Ok(written);
    }
    write_local_pair(executor, workdir, values, cancel).await
}

/// Which of the two keys a pass has to touch, and with what — the whole
/// of the decision, with no git in it, so its combinations are unit tests.
///
/// A key the file already holds as wanted is left alone. Not an
/// optimisation: `--unset` of an absent key exits 5 like a refused
/// multi-valued one, so only after this filter is a failed unset real
/// (rules-refs/core.md「今の値を先に読んで、違うキーだけ撃つ」).
fn keys_to_write<'a>(held: &Identity, wanted: &'a IdentityValues) -> Vec<(&'static str, &'a str)> {
    [
        ("user.name", wanted.name(), wanted.name_read(), &held.name),
        (
            "user.email",
            wanted.email(),
            wanted.email_read(),
            &held.email,
        ),
    ]
    .into_iter()
    .filter(|(_, _, read, held)| held.as_deref() != *read)
    .map(|(key, value, _, _)| (key, value))
    .collect()
}

/// One pass: the keys [`keys_to_write`] names, then what the file holds
/// after.
async fn write_local_pair(
    executor: &GitExecutor,
    workdir: &Path,
    values: &IdentityValues,
    cancel: &CancellationToken,
) -> Result<IdentityWrite, GitError> {
    let held = load_local(executor, workdir, cancel).await?;
    let mut message = String::new();
    for (key, wanted) in keys_to_write(&held, values) {
        // Stops at the first that fails: the second would write into a
        // file this pass was just told it cannot change.
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
        name_saved: identity.name.as_deref() == values.name_read(),
        email_saved: identity.email.as_deref() == values.email_read(),
        identity,
        message,
    })
}

/// One key, written or taken out. `--local` is spelled out though it is
/// git's default, so the level read back and the level written are named
/// by the same word.
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
        // No `--`: git would store it as the value (see `set_identity`).
        cmd.args(["config", "--local", key, value])
    };
    executor.run(cmd, cancel).await.map(drop)
}

#[cfg(test)]
mod tests {
    use super::{Identity, IdentityValues, keys_to_write};

    /// What one repository's own file holds; `None` is a key nobody wrote
    /// here.
    fn held(name: Option<&str>, email: Option<&str>) -> Identity {
        Identity {
            name: name.map(str::to_string),
            email: email.map(str::to_string),
        }
    }

    /// What a save asks for; an empty half is an emptied box.
    fn wanted(name: &str, email: &str) -> IdentityValues {
        IdentityValues::new(name, email)
    }

    /// A save that asks for what the file already says asks git for
    /// nothing — including a blank box over a key nobody wrote, which
    /// would otherwise reach a failing `--unset`.
    #[test]
    fn a_save_that_changes_nothing_writes_nothing() {
        assert!(
            keys_to_write(
                &held(Some("Ada"), Some("ada@example.com")),
                &wanted("Ada", "ada@example.com")
            )
            .is_empty()
        );
        assert!(keys_to_write(&Identity::default(), &wanted("", "")).is_empty());
        assert!(
            keys_to_write(&held(Some("Ada"), None), &wanted("Ada", "")).is_empty(),
            "one key written and one not, each asked for what it already is"
        );
    }

    /// One key at a time, the errand this level exists for; the other key
    /// is neither written nor taken out.
    #[test]
    fn one_key_moves_without_the_other() {
        assert_eq!(
            keys_to_write(
                &held(Some("Ada"), Some("ada@example.com")),
                &wanted("Ada", "work@example.com")
            ),
            vec![("user.email", "work@example.com")]
        );
        assert_eq!(
            keys_to_write(
                &held(Some("Ada"), Some("ada@example.com")),
                &wanted("Ada Lovelace", "ada@example.com")
            ),
            vec![("user.name", "Ada Lovelace")]
        );
    }

    /// Both, in the order the pass writes them — which is the order a
    /// half-landed write is reported in.
    #[test]
    fn both_keys_move_together_and_in_order() {
        assert_eq!(
            keys_to_write(&Identity::default(), &wanted("Ada", "ada@example.com")),
            vec![("user.name", "Ada"), ("user.email", "ada@example.com")]
        );
    }

    /// An emptied box takes its key out, and the two directions mix: a
    /// pass may be taking one key out while it writes the other.
    #[test]
    fn an_emptied_box_takes_its_key_out_beside_one_being_written() {
        assert_eq!(
            keys_to_write(&held(Some("Ada"), Some("ada@example.com")), &wanted("", "")),
            vec![("user.name", ""), ("user.email", "")]
        );
        assert_eq!(
            keys_to_write(
                &held(Some("Ada"), Some("ada@example.com")),
                &wanted("", "work@example.com")
            ),
            vec![("user.name", ""), ("user.email", "work@example.com")]
        );
        assert_eq!(
            keys_to_write(&held(None, Some("ada@example.com")), &wanted("Ada", "")),
            vec![("user.name", "Ada"), ("user.email", "")]
        );
    }

    /// What is asked arrives trimmed, so a box of blanks alone is an
    /// emptied one, and a name is written without the blanks around it.
    #[test]
    fn a_box_of_blanks_is_an_emptied_one() {
        assert_eq!(
            keys_to_write(&held(None, Some("ada@example.com")), &wanted(" Ada ", " ")),
            vec![("user.name", "Ada"), ("user.email", "")]
        );
    }
}
