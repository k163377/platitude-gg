//! What the band's `NO LFS` stands on: how many pending files need Git
//! LFS where git cannot run it (デザイン規約 §ウィンドウの縁).

use super::*;
use crate::lfs;

impl RepoSession {
    /// The count this status read carries. Counted again where the tree
    /// moved (`moved`: status or a write, the line-ending marks' beat) or
    /// the last count could not be had; otherwise the held count, asking
    /// again only whether Git LFS has arrived.
    ///
    /// An attribute edit that moves no status — a `.gitattributes` that is
    /// pending already, `info/attributes` — is counted on the next beat
    /// that moves one.
    pub(super) async fn lfs_needed(
        &self,
        workdir: &Path,
        status: &WorkTreeStatus,
        moved: bool,
        cancel: &CancellationToken,
    ) -> usize {
        let retry = self.lfs_unsettled.swap(false, Ordering::SeqCst);
        if moved || retry {
            self.settle_lfs_needed(workdir, status, cancel).await
        } else {
            self.recheck_lfs_needed(workdir, cancel).await
        }
    }

    /// Counts afresh. A count that could not be had leaves the held one
    /// standing and is tried again on the next tick: the badge is what
    /// stops the whole file going in, and a count read as nothing would
    /// wait for the tree to move before it said so.
    async fn settle_lfs_needed(
        &self,
        workdir: &Path,
        status: &WorkTreeStatus,
        cancel: &CancellationToken,
    ) -> usize {
        match self.read_lfs_needed(workdir, status, cancel).await {
            Some(needed) => {
                self.lfs_needed.store(needed, Ordering::SeqCst);
                needed
            }
            None => {
                self.lfs_unsettled.store(true, Ordering::SeqCst);
                self.lfs_needed.load(Ordering::SeqCst)
            }
        }
    }

    /// The held count, asking whether Git LFS has arrived only while the
    /// count stands — so an install is seen on the next tick with nothing
    /// else moving, at one process a tick in that state alone.
    async fn recheck_lfs_needed(&self, workdir: &Path, cancel: &CancellationToken) -> usize {
        let held = self.lfs_needed.load(Ordering::SeqCst);
        if held > 0 && self.lfs_answers(workdir, cancel).await == Some(true) {
            self.lfs_needed.store(0, Ordering::SeqCst);
            return 0;
        }
        held
    }

    /// LFS first: where git runs it (Git for Windows carries it) nothing is
    /// ever counted, and that is asked once a session — the attributes of
    /// every pending path are read only where it is missing. `None` where
    /// either could not be asked.
    async fn read_lfs_needed(
        &self,
        workdir: &Path,
        status: &WorkTreeStatus,
        cancel: &CancellationToken,
    ) -> Option<usize> {
        let paths: Vec<String> = status
            .items
            .iter()
            .filter(|item| lfs::stages_content(item))
            .map(|item| item.path().to_string())
            .collect();
        if paths.is_empty() || self.lfs_answers(workdir, cancel).await? {
            return Some(0);
        }
        match lfs::filtered(&self.executor, workdir, &paths, cancel).await {
            Ok(count) => Some(count),
            Err(e) => {
                if !e.is_cancelled() {
                    tracing::debug!(error = %e, "lfs attribute read failed");
                }
                None
            }
        }
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
