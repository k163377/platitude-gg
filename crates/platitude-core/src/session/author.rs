//! Who a commit is attributed to: the identity git will record, the
//! signing configuration behind it, and HEAD's own message and author —
//! what an amend starts from.

use super::*;

impl RepoSession {
    /// Re-reads the author identity and signing configuration. The task
    /// answers once the configuration has published — what the write
    /// queue waits on after an identity write (`session::write`) — with
    /// the read that did not land, empty where it did. `None` where no
    /// repository is open and nothing was read.
    pub fn refresh_author(self: &Arc<Self>) -> Option<tokio::task::JoinHandle<Vec<FollowUp>>> {
        let workdir = self.workdir()?;
        let s = Arc::clone(self);
        Some(self.runtime.spawn(async move {
            let cancel = s.root_cancel.clone();
            match identity::load(&s.executor, &workdir, &cancel).await {
                Ok(config) => {
                    s.sink.event(SessionEvent::AuthorLoaded { config });
                    Vec::new()
                }
                Err(e) => {
                    s.fail(FollowUp::Author.label(), e);
                    vec![FollowUp::Author]
                }
            }
        }))
    }

    /// Records `user.name` / `user.email`.
    pub fn set_identity(
        self: &Arc<Self>,
        name: String,
        email: String,
        scope: identity::ConfigScope,
    ) -> Option<OperationId> {
        self.write(
            OperationKind::Identity,
            AfterWrite::Author,
            move |exec, repo, cancel| async move {
                let written =
                    identity::set_identity(&exec, &repo.workdir, &name, &email, scope, &cancel)
                        .await?;
                if written.is_saved() {
                    return Ok(());
                }
                // Half of an identity reads as a whole one everywhere it
                // is used, so a write that did not take is reported as a
                // failure even when git raised nothing against it.
                Err(GitError::Rejected {
                    message: if written.message.is_empty() {
                        "git still reports a different identity".to_string()
                    } else {
                        written.message
                    },
                })
            },
        )
    }

    /// Reads HEAD's message and author so an amend can start from them.
    ///
    /// On demand: only the amend path wants
    /// them, and a repository refresh already runs several commands.
    pub fn load_head_commit(self: &Arc<Self>) {
        self.spawn_read("head-commit", |s, workdir, cancel| async move {
            // An unborn branch has no HEAD to amend; the empty prefill is
            // that state's answer. A read that failed outright goes to
            // the error surface — an amend started from a blank it
            // trusts would commit the blank.
            let head = commit::head_commit(&s.executor, &workdir, &cancel)
                .await?
                .unwrap_or_default();
            Ok(SessionEvent::HeadCommitLoaded { head })
        });
    }
}
