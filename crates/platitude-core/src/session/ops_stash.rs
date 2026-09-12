//! The four writes that move work onto and off the stash, and the
//! rename git has no command for.
//!
//! Beside [`super::stash_round`], which is the round trip a switch makes
//! *through* the stash — this is the stash asked for on its own.
//!
//! Every one answers under the same kind ([`OperationKind::Stash`]), so
//! which press an answer belongs to is the id's to say: an apply pressed
//! just before a pop answers first, and a consumer that counted answers
//! by turn would take it for the pop's ([`crate::operation`]).

use super::stash_round::conflicts_now;
use super::*;

impl RepoSession {
    /// Renames a stash entry — stored again under the new label, old entry
    /// dropped (git has no rename for one — see [`crate::stash::rename`]).
    pub fn rename_stash(
        self: &Arc<Self>,
        selector: String,
        message: String,
    ) -> Option<OperationId> {
        self.write(
            OperationKind::Stash,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                stash::rename(&exec, &repo.workdir, &selector, &message, &cancel).await
            },
        )
    }

    /// `git stash push`, over the whole working tree or only `paths`.
    pub fn stash_push(
        self: &Arc<Self>,
        message: String,
        options: stash::PushOptions,
        paths: Vec<String>,
    ) -> Option<OperationId> {
        self.write(
            OperationKind::Stash,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                stash::push(&exec, &repo.workdir, &message, options, &paths, &cancel).await
            },
        )
    }

    /// `git stash pop <selector>` (drops the stash on success).
    ///
    /// A restore that conflicts is not a failure: the work is across,
    /// waiting to be settled, and git keeps the entry in that case — so a
    /// conflicting pop lands exactly where an apply would have, and the
    /// way back is still in the list (デザイン規約 §変更を退避する).
    /// The exit code cannot tell that apart from a refusal that did
    /// nothing, so the working tree decides (`conflicts_now`).
    ///
    /// Only when the tree was settled to begin with, though: git will not
    /// restore onto an index that already has unmerged paths — it refuses
    /// outright and changes nothing (measured) — and the conflicts still
    /// standing there afterwards are the old ones, not proof of anything.
    pub fn stash_pop(self: &Arc<Self>, selector: String) -> Option<OperationId> {
        self.write(
            OperationKind::Stash,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let settled_first = !conflicts_now(&exec, &repo, &cancel).await?;
                match stash::pop(&exec, &repo.workdir, &selector, &cancel).await {
                    Ok(()) => Ok(()),
                    Err(error) => {
                        if settled_first && conflicts_now(&exec, &repo, &cancel).await? {
                            Ok(())
                        } else {
                            Err(error)
                        }
                    }
                }
            },
        )
    }

    /// `git stash apply <selector>` (keeps the stash).
    ///
    /// A restore that conflicts is read the way [`Self::stash_pop`] reads
    /// one: the work is across and waiting to be settled, so the working
    /// tree decides whether the non-zero exit was that or a refusal that
    /// did nothing.
    pub fn stash_apply(self: &Arc<Self>, selector: String) -> Option<OperationId> {
        self.write(
            OperationKind::Stash,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let settled_first = !conflicts_now(&exec, &repo, &cancel).await?;
                match stash::apply(&exec, &repo.workdir, &selector, &cancel).await {
                    Ok(()) => Ok(()),
                    Err(error) => {
                        if settled_first && conflicts_now(&exec, &repo, &cancel).await? {
                            Ok(())
                        } else {
                            Err(error)
                        }
                    }
                }
            },
        )
    }

    /// `git stash drop <selector>`.
    pub fn stash_drop(self: &Arc<Self>, selector: String) -> Option<OperationId> {
        self.write(
            OperationKind::Stash,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                stash::drop(&exec, &repo.workdir, &selector, &cancel).await
            },
        )
    }
}
