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

    /// Loads a unified diff for one file, and its colours behind it.
    pub fn load_diff(self: &Arc<Self>, target: DiffTarget) {
        let Some(workdir) = self.workdir() else {
            return;
        };
        let s = Arc::clone(self);
        // Claimed before anything is read, so the colouring below can ask
        // whether this is still the file being read (see `diff_epoch`).
        let epoch = s
            .diff_epoch
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            + 1;
        self.runtime.spawn(async move {
            let cancel = s.root_cancel.clone();
            // What git's settings say, and the neighbours if they are the
            // only answer, are read **beside** the diff rather than after
            // it: a notice that turns up a moment later is one the reader
            // has already scrolled past.
            // The file the diff is of, fetched beside it rather than
            // after: the colours are read against it (`highlight`), and a
            // second round trip would land after the rows are on screen.
            // Only for a language something can be said about — otherwise
            // it is a process spent on a file nobody will colour.
            let wants_source = crate::highlight::knows(preview::target_path(&target));
            let (diff, endings, source) = tokio::join!(
                details::file_diff_raw(&s.executor, &workdir, &target, &cancel),
                s.ending_context(&workdir, &target, &cancel),
                async {
                    match wants_source {
                        true => preview::source_text(&s.executor, &workdir, &target, &cancel).await,
                        false => None,
                    }
                },
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
                    let patches = Arc::new(patches);
                    let is_binary = patches.iter().any(|p| p.is_binary);
                    let preview =
                        preview::file_preview(&s.executor, &workdir, &target, is_binary, &cancel)
                            .await;
                    s.sink.event(SessionEvent::DiffLoaded {
                        target: target.clone(),
                        patches: Arc::clone(&patches),
                        preview,
                        fingerprint,
                        endings,
                    });
                    s.paint_diff(target, patches, source, epoch);
                }
                Err(e) => s.fail("diff", e),
            }
        });
    }

    /// Works out the colours for a diff already on its way to the pane and
    /// sends them after it ([`SessionEvent::DiffColoured`]).
    ///
    /// Two things keep this off the reader's path. It is the one part of
    /// reading a diff that computes rather than waits, so it goes to a
    /// blocking thread rather than holding a runtime worker for as long as
    /// a large file takes; and it is skipped outright once the reader has
    /// moved on, which is what stops a walk down a commit's file list from
    /// leaving a colouring per row running behind it.
    ///
    /// It is also somebody else's code. If it goes down, the diff does not
    /// go with it — the rows are already gone out, and a panic here costs
    /// the file its colours and nothing else.
    fn paint_diff(
        self: &Arc<Self>,
        target: DiffTarget,
        patches: Arc<Vec<FilePatch>>,
        source: Option<String>,
        epoch: u64,
    ) {
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            if !s.diff_is_current(epoch) {
                return;
            }
            let colors = match tokio::task::spawn_blocking(move || {
                crate::highlight::colors(&patches, source.as_deref())
            })
            .await
            {
                Ok(colors) => colors,
                Err(e) => {
                    tracing::warn!(error = %e, "syntax colours failed; the diff stays plain");
                    return;
                }
            };
            // Asked again: a long colouring can be overtaken while it runs,
            // and the pane would drop the answer anyway.
            if !s.diff_is_current(epoch) {
                return;
            }
            s.sink.event(SessionEvent::DiffColoured { target, colors });
        });
    }

    /// Whether the diff read that claimed `epoch` is still the latest one.
    fn diff_is_current(&self, epoch: u64) -> bool {
        self.diff_epoch.load(std::sync::atomic::Ordering::SeqCst) == epoch
    }
}
