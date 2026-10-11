//! Another worktree's uncommitted work applied here, landing as `git stash
//! apply` lands (デザイン規約 §別の worktree を読む): read off that worktree
//! into a stash nothing names ([`discards::stash_of_work`]) and applied by
//! git's own apply, which this end never imitates. That worktree is only
//! read.
//!
//! What git turns down, and what this end finds in the way before git
//! runs, is a report for the notice bar (`report::carried_refused` /
//! `carried_withheld`), not a failure for the command log to raise.

use super::stash_round::conflicts_now;
use super::*;
use crate::discards;

impl RepoSession {
    /// Applies the uncommitted work of the worktree at `there` — as the
    /// worktree listing spells it (`Carried::path`) — to this one. It
    /// waits for any write a session open there has accepted first
    /// ([`Self::write_reading`]), so it never reads that tree half-written by
    /// one; it reads that tree as it stands when its turn comes.
    pub fn apply_carried(self: &Arc<Self>, there: String) -> Option<OperationId> {
        let source = there.clone();
        self.write_reading(
            OperationKind::Stash,
            AfterWrite::Tree,
            &source,
            move |exec, repo, cancel| async move {
                apply_work(&exec, &repo, &there, &cancel).await
            },
        )
    }
}

/// The write: refused before git where the work cannot be read whole or a
/// file here stands where one of its untracked files goes — git applies the
/// tracked changes before it finds that, and leaves them (rules-refs/core.md
/// の `stash apply` の項) — then `git stash apply` of the rest, whose
/// conflicts are a landing as an apply's are ([`RepoSession::stash_apply`]),
/// and the untracked files only once that has landed: git's own apply puts
/// them in after turning the rest down too.
async fn apply_work(
    exec: &GitExecutor,
    repo: &RepoInfo,
    there: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let name = joins::shown_name(there);
    let unread = |message: String| {
        report::carried_withheld(&name, report::ReportKind::CarriedUnread, message)
    };
    let Some(git_dir) = discards::worktree_at(exec, there, cancel).await? else {
        return Err(unread(format!("no worktree stands at '{there}'")));
    };
    let at = Path::new(there);
    let status = status::load(exec, at, cancel).await?;
    let mut tracked = Vec::new();
    let mut untracked = Vec::new();
    for item in &status.items {
        match item {
            status::StatusItem::Tracked {
                path, orig_path, ..
            } => {
                tracked.push(path.clone());
                tracked.extend(orig_path.iter().cloned());
            }
            status::StatusItem::Unmerged { path, .. } => {
                return Err(unread(format!("'{path}' is unmerged in '{there}'")));
            }
            // A repository of its own inside the tree: a stash leaves it
            // where it is, as a discard does (破棄記録仕様.md §2.1).
            status::StatusItem::Untracked { path } if path.ends_with('/') => {}
            status::StatusItem::Untracked { path } => untracked.push(path.clone()),
        }
    }
    tracked.sort();
    tracked.dedup();
    if let Some(path) = untracked
        .iter()
        .find(|path| !discards::lies_free(&repo.workdir, path))
    {
        return Err(report::carried_withheld(
            &name,
            report::ReportKind::CarriedFileInTheWay,
            format!("'{path}' already stands here"),
        ));
    }
    let Some(stash) =
        discards::stash_of_work(exec, at, Path::new(&git_dir), &tracked, &untracked, cancel)
            .await?
    else {
        return Err(unread(format!("'{there}' has no commit yet")));
    };
    if !stash.not_copied.is_empty() {
        return Err(unread(format!(
            "git would not take {} in '{there}'",
            stash.not_copied.join(", ")
        )));
    }
    let settled_first = !conflicts_now(exec, repo, cancel).await?;
    // Exit 1 is a conflicted apply as well as a refusal, as for `stash apply`
    // of an entry (`stash::apply`); the working tree tells them apart.
    let cmd = GitCommand::new()
        .cwd(&repo.workdir)
        .answers_by_code(1)
        .args(["stash", "apply"])
        .arg(stash.commit.to_hex());
    let command = cmd.describe();
    let out = exec.run_unchecked(cmd, cancel).await?;
    if out.code != 0 && !(settled_first && conflicts_now(exec, repo, cancel).await?) {
        return Err(report::carried_refused(&name, command, &out));
    }
    if let Some(tree) = &stash.untracked {
        discards::put_untracked(exec, &repo.workdir, &repo.git_dir, tree, cancel).await?;
    }
    Ok(())
}
