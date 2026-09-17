//! The rename git does not have.
//!
//! There is no `git stash rename`, so the label a list shows is changed
//! out of what git does have: `commit-tree` writes the entry's commit
//! again with new text, `stash store` puts that on the reflog, and the
//! entry it replaces is dropped. The order is what makes three commands
//! look like one operation from outside — and what decides which of them
//! is safe to have run alone if the next one does not.

use super::*;

/// Renames a stash entry: the label the list shows.
///
/// git has no rename for one, so this is built from what it does
/// have. The entry's commit is written again with the new message
/// and nothing else changed (same tree, same parents, same
/// identities and dates — `commit-tree` only replaces the text),
/// `stash store` puts that on the reflog, and the old entry is
/// dropped. Rewriting the commit is what keeps the two places a
/// stash's message is read — the list and the commit itself —
/// saying the same thing.
///
/// Two consequences to know about:
/// - **the entry moves to the top of the list.** The list is a reflog, and
///   a reflog only grows at the front; there is no writing into the middle
///   of one.
/// - **its commit id changes.** Nothing but the reflog points at a stash,
///   so nothing is left dangling.
///
/// The old entry is dropped last and only after its identity is confirmed
/// at the position the store pushed it to: a rename that ends up keeping
/// both entries is a mess, but losing the work is worse.
pub async fn rename(
    executor: &GitExecutor,
    workdir: &Path,
    selector: &str,
    message: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let entry = read_entry(executor, workdir, selector, cancel).await?;
    let mut cmd = GitCommand::new()
        .cwd(workdir)
        .args(["commit-tree", &entry.tree]);
    for parent in &entry.parents {
        cmd = cmd.args(["-p", parent]);
    }
    let out = executor
        .run(
            cmd.args(["-m", message])
                .env("GIT_AUTHOR_NAME", &entry.author_name)
                .env("GIT_AUTHOR_EMAIL", &entry.author_email)
                .env("GIT_AUTHOR_DATE", &entry.author_date)
                .env("GIT_COMMITTER_NAME", &entry.committer_name)
                .env("GIT_COMMITTER_EMAIL", &entry.committer_email)
                .env("GIT_COMMITTER_DATE", &entry.committer_date),
            cancel,
        )
        .await?;
    let stored = out.stdout_utf8().trim().to_string();
    if stored.is_empty() {
        return Err(GitError::Rejected {
            message: "git wrote no commit for the renamed stash".to_string(),
        });
    }
    // The commit stands alone: `stash store` takes one commit and no
    // pathspec; no separator in its usage.
    let store = GitCommand::new()
        .cwd(workdir)
        .args(["stash", "store", "-m", message, &stored]);
    executor.run(store, cancel).await?;

    // `store` prepends, so the old entry is one further down than it was.
    let Some(shifted) = shift_selector(selector) else {
        return Err(GitError::Rejected {
            message: format!("{selector} is not a stash selector this window can move"),
        });
    };
    let at = GitCommand::new()
        .cwd(workdir)
        // Exit 1 is the answer "no such entry" — the mismatch arm below.
        .answers_by_code(1)
        .args(["rev-parse", "--verify", "--quiet", &shifted]);
    let found = executor.run_unchecked(at, cancel).await?;
    if found.stdout_utf8().trim() != entry.oid {
        // A rename that got as far as the new entry and no further: the
        // list moved under it, so which entry to take away is exactly what
        // could not be worked out (デザイン規約 §答えの要らない報せ / §状態).
        return Err(crate::report::half_renamed(
            selector,
            GitError::UnexpectedOutput {
                command: "git rev-parse --verify".to_string(),
                message: "the stash list moved while renaming".to_string(),
            },
        ));
    }
    run_selector(executor, workdir, "drop", &shifted, cancel).await
}

/// `stash@{n}` → `stash@{n+1}`, which is where an entry stands once a new
/// one has been pushed in front of it.
fn shift_selector(selector: &str) -> Option<String> {
    let (head, index) = selector.rsplit_once("@{")?;
    let index = index.strip_suffix('}')?.parse::<u32>().ok()?;
    Some(format!("{head}@{{{}}}", index + 1))
}

/// One stash entry's commit, in the pieces a rewrite needs.
struct StashCommit {
    oid: String,
    tree: String,
    parents: Vec<String>,
    author_name: String,
    author_email: String,
    author_date: String,
    committer_name: String,
    committer_email: String,
    committer_date: String,
}

async fn read_entry(
    executor: &GitExecutor,
    workdir: &Path,
    selector: &str,
    cancel: &CancellationToken,
) -> Result<StashCommit, GitError> {
    const FORMAT: &str = "--format=%H%x00%T%x00%P%x00%an%x00%ae%x00%aI%x00%cn%x00%ce%x00%cI";
    let cmd =
        GitCommand::new()
            .cwd(workdir)
            .args(["log", "-1", FORMAT, "--end-of-options", selector]);
    let out = executor.run(cmd, cancel).await?;
    let text = out.stdout_utf8();
    let fields: Vec<&str> = text.trim_end_matches('\n').split('\0').collect();
    let [oid, tree, parents, an, ae, ad, cn, ce, cd] = fields.as_slice() else {
        return Err(GitError::Rejected {
            message: format!("could not read {selector}"),
        });
    };
    Ok(StashCommit {
        oid: (*oid).to_string(),
        tree: (*tree).to_string(),
        parents: parents.split_whitespace().map(str::to_string).collect(),
        author_name: (*an).to_string(),
        author_email: (*ae).to_string(),
        author_date: (*ad).to_string(),
        committer_name: (*cn).to_string(),
        committer_email: (*ce).to_string(),
        committer_date: (*cd).to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stored_entry_pushes_the_others_one_down() {
        assert_eq!(shift_selector("stash@{0}").as_deref(), Some("stash@{1}"));
        assert_eq!(shift_selector("stash@{9}").as_deref(), Some("stash@{10}"));
        assert_eq!(shift_selector("stash"), None);
        assert_eq!(shift_selector("stash@{tip}"), None);
    }
}
