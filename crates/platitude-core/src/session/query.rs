//! On-demand reads answered as events: publish state, history reach,
//! signatures, commit details and file diffs.

use super::*;
use crate::eol;

impl RepoSession {
    /// Asks how much of `range` a remote already has, so the UI can warn
    /// before rewriting published history. A read, not a write.
    pub fn check_publish(self: &Arc<Self>, range: String) {
        let Some(workdir) = self.workdir() else {
            return;
        };
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let cancel = s.root_cancel.clone();
            match publish::state_of(&s.executor, &workdir, &range, &cancel).await {
                Ok(state) => s.sink.event(SessionEvent::PublishChecked { range, state }),
                Err(e) => s.fail("publish", e),
            }
        });
    }

    /// Asks whether HEAD can reach `oid`, so the UI can tell which commits
    /// a rewrite may start from. A read, not a write.
    pub fn check_in_history(self: &Arc<Self>, oid: Oid) {
        let Some(workdir) = self.workdir() else {
            return;
        };
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let cancel = s.root_cancel.clone();
            match commit::is_in_head_history(&s.executor, &workdir, &oid, &cancel).await {
                Ok(in_history) => s.sink.event(SessionEvent::InHistoryChecked {
                    oid: oid.to_hex(),
                    in_history,
                }),
                Err(e) => s.fail("history", e),
            }
        });
    }

    /// Asks whether `oid` carries a signature and what git makes of it.
    ///
    /// Kept out of [`Self::load_details`] on purpose: verifying runs gpg or
    /// ssh-keygen, and the details pane has a 100ms budget. The answer
    /// arrives on its own, after the commit is already on screen.
    pub fn check_signature(self: &Arc<Self>, oid: Oid) {
        let Some(workdir) = self.workdir() else {
            return;
        };
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let cancel = s.root_cancel.clone();
            match identity::verify_commit(&s.executor, &workdir, &oid.to_hex(), &cancel).await {
                Ok(signature) => s.sink.event(SessionEvent::SignatureChecked {
                    oid: oid.to_hex(),
                    signature,
                }),
                Err(e) => s.fail("signature", e),
            }
        });
    }

    /// Loads full details of one commit (metadata + changed files).
    pub fn load_details(self: &Arc<Self>, oid: Oid) {
        let Some(workdir) = self.workdir() else {
            return;
        };
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let cancel = s.root_cancel.clone();
            match details::commit_details(&s.executor, &workdir, &oid, &cancel).await {
                Ok(details) => s.sink.event(SessionEvent::DetailsLoaded { details }),
                Err(e) => s.fail("details", e),
            }
        });
    }

    /// Loads a unified diff for one file.
    pub fn load_diff(self: &Arc<Self>, target: DiffTarget) {
        let Some(workdir) = self.workdir() else {
            return;
        };
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let cancel = s.root_cancel.clone();
            // What git's settings say, and the neighbours if they are the
            // only answer, are read **beside** the diff rather than after
            // it: a notice that turns up a moment later is one the reader
            // has already scrolled past.
            let (diff, endings) = tokio::join!(
                details::file_diff_raw(&s.executor, &workdir, &target, &cancel),
                s.ending_context(&workdir, &target, &cancel),
            );
            match diff {
                Ok(raw) => {
                    let patches = crate::parse::diff::parse_patch(&raw);
                    let fingerprint = details::fingerprint(&raw);
                    let endings = match endings {
                        EndingContext::Excluded => None,
                        EndingContext::Open(baseline) => {
                            eol::settle(eol::read_one(&raw), baseline.as_ref())
                        }
                    };
                    // Colouring is the one thing in this task that computes
                    // rather than waits, so it goes to a blocking thread
                    // instead of holding a runtime worker for as long as a
                    // large diff takes. It is also somebody else's code:
                    // if it goes down the diff must not go with it, and
                    // the bytes are still here to be read again.
                    let (patches, colors) = match tokio::task::spawn_blocking(move || {
                        let colors = crate::highlight::colors(&patches);
                        (patches, colors)
                    })
                    .await
                    {
                        Ok(pair) => pair,
                        Err(e) => {
                            tracing::warn!(error = %e, "syntax colours failed; showing the diff plain");
                            (
                                crate::parse::diff::parse_patch(&raw),
                                crate::highlight::DiffColors::default(),
                            )
                        }
                    };
                    let is_binary = patches.iter().any(|p| p.is_binary);
                    let preview =
                        preview::file_preview(&s.executor, &workdir, &target, is_binary, &cancel)
                            .await;
                    s.sink.event(SessionEvent::DiffLoaded {
                        target,
                        patches,
                        preview,
                        fingerprint,
                        endings,
                        colors,
                    });
                }
                Err(e) => s.fail("diff", e),
            }
        });
    }
}
