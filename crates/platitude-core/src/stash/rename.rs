//! The stash rename git does not have: `commit-tree`, `stash store`, then
//! drop the old entry. The order decides which step is safe to have run
//! alone if the next one fails.

use super::*;

/// Renames a stash entry: the label the list shows. git has none, so the
/// entry's commit is rewritten with the new message and nothing else
/// (same tree, parents, identities and dates), which keeps the list and
/// the commit saying the same thing. The entry moves to the top of the
/// list (a reflog only grows at the front) and its commit id changes
/// (nothing but the reflog points at a stash).
///
/// The old entry is dropped last, only after its oid is confirmed at the
/// shifted position: keeping both entries is a mess, losing the work is
/// worse.
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
    // No separator: `stash store` takes one commit and no pathspec.
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
        // Half done: the list moved, so which entry to drop cannot be
        // worked out (デザイン規約 §答えの要らない報せ / §状態).
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

/// `stash@{n}` → `stash@{n+1}`.
fn shift_selector(selector: &str) -> Option<String> {
    let (head, index) = selector.rsplit_once("@{")?;
    let index = index.strip_suffix('}')?.parse::<u32>().ok()?;
    Some(format!("{head}@{{{}}}", index + 1))
}

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
