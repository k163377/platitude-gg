//! Another worktree's uncommitted work applied here, landing as `git stash
//! apply` lands (デザイン規約 §別の worktree を読む): read off that worktree
//! into a stash nothing names ([`discards::stash_of_work`]) and applied by
//! git's own apply, which this end never imitates. That worktree is only
//! read.
//!
//! The landing is what is decided, not the means (user decision — ask
//! before changing): applied cleanly, the files join this tree's
//! uncommitted work, what was staged there unstaged here except files newly
//! added, which stay added, and untracked files untracked; a different HEAD
//! still applies, three-way. Where both changed the same lines the file
//! lands conflicted, which is no failure. Where this tree's own work is in
//! the way (a file changed here that they change, an untracked file where
//! they add one), nothing is written. Nothing is written there either and
//! no STASHES entry is made: that worktree still holding its work is what
//! the entry `stash apply` keeps would be, so trying again is a discard
//! here and another press.
//!
//! What git turns down, and what this end finds in the way before git
//! runs, is a report for the notice bar (`report::carried_refused` /
//! `withheld_about`), not a failure for the command log to raise — the
//! command log does not open (user decision — ask before changing).

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
/// tracked changes before it finds that, and leaves them (`could not restore
/// untracked files from stash`, measured on 2.55;
/// `git_leaves_the_tracked_half_applied`) — then `git stash apply` of the
/// rest, whose conflicts are a landing as an apply's are
/// ([`RepoSession::stash_apply`]), and the untracked files only once that
/// has landed: git's own apply puts them in after turning the rest down too
/// (`do_apply_stash` jumps to `restore_untracked` when its merge fails).
/// So a refusal over work here in the way, or over an untracked file where
/// an added one goes, writes no tracked change (exit 1 up to 2.55, as a
/// conflicted landing, told apart by `conflicts_now`) yet would write the
/// untracked files. Only the failures before the merge return without them:
/// the index refresh (an unmerged index, or another git's lock, which
/// 2.43.0 and 2.43.1 report with exit 1 and no words) and a merge standing
/// here. A change that overlaps only what is staged here goes through as a
/// three-way merge and leaves the index as it is.
///
/// Three rare, hand-made shapes are left as git's own apply leaves them
/// (user decision — ask before changing). Where a file added or changed
/// there stands as a folder in that working tree and is missing here, the
/// tracked part makes the file and `checkout-index` (128) then has nowhere
/// to put the untracked file under the folder; and from a case-sensitive
/// filesystem to a case-insensitive one. Both leave the tracked part
/// applied, with git's words as the failure. Where a tracked `d/f` was
/// removed there for an untracked file `d`, the folder `d/` here is refused
/// before git, which would empty it in the merge and go through.
async fn apply_work(
    exec: &GitExecutor,
    repo: &RepoInfo,
    there: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let name = joins::shown_name(there);
    let unread =
        |message: String| report::withheld_about(report::ReportKind::CarriedUnread, &name, message);
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
            // where it is, as a discard does (`discard_record::copy_of_choices`).
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
        return Err(report::withheld_about(
            report::ReportKind::CarriedFileInTheWay,
            &name,
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
    // Exit 1 is a conflicted apply, and up to 2.55 a refusal too (from 2.56
    // a refusal exits 128), as for `stash apply` of an entry (`stash::apply`);
    // the working tree tells them apart, and a refusal's words keep an exit
    // 1 row a failure. `--no-index`: what was staged there lands unstaged
    // here whatever `stash.index` says.
    let cmd = GitCommand::new()
        .cwd(&repo.workdir)
        .answers_by_code(1)
        .except_when_it_says(crate::relied_messages::CARRIED_TURNED_DOWN)
        .args(["stash", "apply", "--no-index"])
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
