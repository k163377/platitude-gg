//! What the band's `NO LFS` stands on: how many pending files need Git
//! LFS where git cannot run it (デザイン規約 §ウィンドウの縁) — and which of
//! them a discard could not copy (破棄記録仕様.md §2.1), the files whose
//! discard the screen warns of as one that cannot be brought back.

use super::*;
use crate::lfs;

/// What a status read carries about Git LFS.
#[derive(Clone, Default)]
pub(super) struct LfsNeeds {
    /// Pending files that need Git LFS where git cannot run it.
    pub(super) needed: usize,
    /// Of those, the ones `git add` refuses — `filter.lfs.required` is set
    /// — so a discard leaves them out of its copy. Empty where the filter
    /// is not required: git takes such a file whole.
    pub(super) not_copied: Arc<Vec<String>>,
}

impl RepoSession {
    /// The reading this status carries. Read again where the tree moved
    /// (`moved`: status or a write, the line-ending marks' beat) or the
    /// last reading could not be had; otherwise the held one, asking again
    /// only whether Git LFS has arrived.
    ///
    /// An attribute or config edit that moves no status — a
    /// `.gitattributes` that is pending already, `info/attributes`,
    /// `filter.lfs.required` — is read on the next beat that moves one.
    pub(super) async fn lfs_needs(
        &self,
        workdir: &Path,
        status: &WorkTreeStatus,
        moved: bool,
        cancel: &CancellationToken,
    ) -> LfsNeeds {
        let retry = self.lfs_unsettled.swap(false, Ordering::SeqCst);
        if moved || retry {
            self.settle_lfs_needs(workdir, status, cancel).await
        } else {
            self.recheck_lfs_needs(workdir, cancel).await
        }
    }

    /// Reads afresh. A reading that could not be had leaves the held one
    /// standing and is tried again on the next tick: the badge is what
    /// stops the whole file going in, and a reading of nothing would wait
    /// for the tree to move before it said so.
    async fn settle_lfs_needs(
        &self,
        workdir: &Path,
        status: &WorkTreeStatus,
        cancel: &CancellationToken,
    ) -> LfsNeeds {
        match self.read_lfs_needs(workdir, status, cancel).await {
            Some(needs) => {
                *relock(&self.lfs_held) = needs.clone();
                needs
            }
            None => {
                self.lfs_unsettled.store(true, Ordering::SeqCst);
                relock(&self.lfs_held).clone()
            }
        }
    }

    /// The held reading, asking whether Git LFS has arrived only while it
    /// counts something — so an install is seen on the next tick with
    /// nothing else moving, at one process a tick in that state alone.
    async fn recheck_lfs_needs(&self, workdir: &Path, cancel: &CancellationToken) -> LfsNeeds {
        let held = relock(&self.lfs_held).clone();
        if held.needed > 0 && self.lfs_answers(workdir, cancel).await == Some(true) {
            let none = LfsNeeds::default();
            *relock(&self.lfs_held) = none.clone();
            return none;
        }
        held
    }

    /// LFS first: where git runs it (Git for Windows carries it) nothing is
    /// ever counted, and that is asked once a session — the attributes of
    /// every pending path are read only where it is missing, and the config
    /// only where some path needs it. `None` where any of them could not be
    /// asked.
    async fn read_lfs_needs(
        &self,
        workdir: &Path,
        status: &WorkTreeStatus,
        cancel: &CancellationToken,
    ) -> Option<LfsNeeds> {
        let paths: Vec<String> = status
            .items
            .iter()
            .filter(|item| lfs::stages_content(item))
            .map(|item| item.path().to_string())
            .collect();
        if paths.is_empty() || self.lfs_answers(workdir, cancel).await? {
            return Some(LfsNeeds::default());
        }
        let found = match lfs::filtered(&self.executor, workdir, &paths, cancel).await {
            Ok(found) => found,
            Err(e) => {
                if !e.is_cancelled() {
                    tracing::debug!(error = %e, "lfs attribute read failed");
                }
                return None;
            }
        };
        if found.is_empty() {
            return Some(LfsNeeds::default());
        }
        let required = match lfs::required(&self.executor, workdir, cancel).await {
            Ok(required) => required,
            Err(e) => {
                if !e.is_cancelled() {
                    tracing::debug!(error = %e, "lfs required read failed");
                }
                return None;
            }
        };
        Some(LfsNeeds {
            needed: found.len(),
            not_copied: Arc::new(if required { found } else { Vec::new() }),
        })
    }

    /// Whether git runs Git LFS here; `None` where the asking failed. One
    /// asking at a time (`Derived`); a yes is kept for the session's life —
    /// nothing in this app takes LFS away again — and a no is asked again.
    async fn lfs_answers(&self, workdir: &Path, cancel: &CancellationToken) -> Option<bool> {
        match self
            .lfs_runs
            .get_or_try_init(|| lfs::runs(&self.executor, workdir, cancel))
            .await
        {
            Ok(answers) => {
                if !answers {
                    self.lfs_runs.forget();
                }
                Some(answers)
            }
            Err(e) => {
                if !e.is_cancelled() {
                    tracing::debug!(error = %e, "lfs version read failed");
                }
                None
            }
        }
    }
}
