//! The four writes that move work onto and off the stash, and the
//! rename git has no command for. The round trip a switch makes
//! *through* the stash is [`super::stash_round`].
//!
//! All answer under [`OperationKind::Stash`], so only the id says which
//! press an answer belongs to — an apply pressed just before a pop
//! answers first ([`crate::operation`]).

use super::stash_round::conflicts_now;
use super::*;
use crate::discards;

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
    /// A restore that conflicts is a success: git keeps the entry, so it
    /// lands where an apply would (デザイン規約 §変更を退避する). The exit
    /// code cannot tell that from a refusal that did nothing, so the
    /// working tree decides (`conflicts_now`) — only if it was settled
    /// beforehand: git refuses outright onto unmerged paths, and the
    /// conflicts standing afterwards are the old ones.
    ///
    /// An entry the pop took from the list goes on the discard record
    /// (破棄記録仕様.md §2); one a conflict left there did not go.
    pub fn stash_pop(self: &Arc<Self>, selector: String) -> Option<OperationId> {
        let s = Arc::clone(self);
        self.write(
            OperationKind::Stash,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let settled_first = !conflicts_now(&exec, &repo, &cancel).await?;
                let before =
                    discards::stash_before(&exec, &repo.workdir, &selector, &cancel).await?;
                match stash::pop(&exec, &repo.workdir, &selector, &cancel).await {
                    Ok(()) => {
                        if let Some(before) = before {
                            let written = discards::record_stash(
                                &exec,
                                &repo.workdir,
                                &repo.git_dir,
                                &before,
                                true,
                                &cancel,
                            )
                            .await;
                            s.recorded(written);
                        }
                        Ok(())
                    }
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
    /// one.
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

    /// `git stash drop <selector>`; the entry goes on the discard record
    /// (破棄記録仕様.md §2).
    pub fn stash_drop(self: &Arc<Self>, selector: String) -> Option<OperationId> {
        let s = Arc::clone(self);
        self.write(
            OperationKind::Stash,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let before =
                    discards::stash_before(&exec, &repo.workdir, &selector, &cancel).await?;
                stash::drop(&exec, &repo.workdir, &selector, &cancel).await?;
                if let Some(before) = before {
                    let written = discards::record_stash(
                        &exec,
                        &repo.workdir,
                        &repo.git_dir,
                        &before,
                        false,
                        &cancel,
                    )
                    .await;
                    s.recorded(written);
                }
                Ok(())
            },
        )
    }
}
