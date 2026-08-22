//! Who a commit is attributed to: the identity git will record, the
//! signing configuration behind it, and HEAD's own message and author —
//! what an amend starts from.

use super::*;

impl RepoSession {
    /// Re-reads the author identity and signing configuration.
    pub fn refresh_author(self: &Arc<Self>) {
        self.spawn_read("identity", |s, workdir, cancel| async move {
            let config = identity::load(&s.executor, &workdir, &cancel).await?;
            Ok(SessionEvent::AuthorLoaded { config })
        });
    }

    /// Records `user.name` / `user.email`.
    pub fn set_identity(
        self: &Arc<Self>,
        name: String,
        email: String,
        scope: identity::ConfigScope,
    ) {
        self.write(
            "identity",
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
        );
    }

    /// Reads HEAD's message and author so an amend can start from them.
    ///
    /// On demand rather than with every refresh: only the amend path wants
    /// them, and a repository refresh already runs several commands.
    pub fn load_head_commit(self: &Arc<Self>) {
        let Some(workdir) = self.workdir() else {
            return;
        };
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let cancel = s.root_cancel.clone();
            // An unborn branch has no HEAD to amend; that is a state, not a
            // failure worth an error banner.
            let head = commit::head_commit(&s.executor, &workdir, &cancel)
                .await
                .unwrap_or_default();
            s.sink.event(SessionEvent::HeadCommitLoaded { head });
        });
    }
}
