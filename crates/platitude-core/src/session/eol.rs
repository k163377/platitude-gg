//! Line-ending context: rulings, sampled baselines, and the marks the
//! status read carries for pending files.

use super::*;
use crate::eol;

impl RepoSession {
    /// What is known about a path's line endings before the patch is read.
    ///
    /// A history diff is deliberately given no baseline: the two estimated
    /// cases would have to sample the files around this one **as they stood
    /// at that commit**, and the files on disk are a different tree.
    pub(super) async fn ending_context(
        &self,
        workdir: &Path,
        target: &DiffTarget,
        cancel: &CancellationToken,
    ) -> EndingContext {
        let (path, historical) = match target {
            DiffTarget::Commit { path, .. } => (path, true),
            DiffTarget::Staged { path, .. }
            | DiffTarget::Unstaged { path }
            | DiffTarget::Untracked { path } => (path, false),
        };
        match eol::ruling(&self.executor, workdir, path, cancel).await {
            Ok(eol::Ruling::NotText) => EndingContext::Excluded,
            Ok(eol::Ruling::Normalised) => EndingContext::Open(None),
            Ok(eol::Ruling::Open) if historical => EndingContext::Open(None),
            Ok(eol::Ruling::Open) => {
                EndingContext::Open(self.eol_baseline(workdir, path, cancel).await)
            }
            // Not knowing is silence, not a failure worth a pane of its own:
            // the diff beside it is the thing that was asked for.
            Err(e) => {
                if !e.is_cancelled() {
                    tracing::debug!(error = %e, "line-ending ruling failed");
                }
                EndingContext::Excluded
            }
        }
    }

    /// The cached baseline for a path's (directory, extension), sampling it
    /// the first time anything in that pair is looked at.
    async fn eol_baseline(
        &self,
        workdir: &Path,
        path: &str,
        cancel: &CancellationToken,
    ) -> Option<eol::Baseline> {
        let key = eol::cache_key(path);
        if let Ok(cache) = self.eol_baselines.lock()
            && let Some(hit) = cache.get(&key)
        {
            return hit.clone();
        }
        let fresh = eol::baseline(&self.executor, workdir, path, cancel)
            .await
            .unwrap_or_default();
        if let Ok(mut cache) = self.eol_baselines.lock() {
            cache.insert(key, fresh.clone());
        }
        fresh
    }

    /// Drops every sampled baseline. A write or a moved ref can bring a new
    /// `.gitattributes` or change what the neighbours look like, and there
    /// is no cheaper way to find out than to ask again when next asked.
    pub(super) fn forget_eol_baselines(&self) {
        if let Ok(mut cache) = self.eol_baselines.lock() {
            cache.clear();
        }
        self.eol_marks_stale.store(true, Ordering::SeqCst);
    }

    pub(super) fn eol_marks(&self) -> Arc<Vec<EolMark>> {
        match self.eol_marks.lock() {
            Ok(marks) => Arc::clone(&marks),
            Err(_) => Arc::new(Vec::new()),
        }
    }

    /// Which pending files have something to say about their line endings.
    ///
    /// Three reads for the whole tree rather than one per file: the two
    /// pending diffs, whose size is the size of the change, and one
    /// `ls-files --eol` for the files git has never seen — a new file's
    /// whole question is "what endings does it have", which that column
    /// answers without a patch. Only a new file that arrives already mixed
    /// needs its patch, because the count in that sentence is in the lines.
    pub(super) async fn settle_eol_marks(
        &self,
        workdir: &Path,
        status: &WorkTreeStatus,
        cancel: &CancellationToken,
    ) -> Arc<Vec<EolMark>> {
        let marks = Arc::new(if status.is_dirty() {
            self.read_eol_marks(workdir, status, cancel).await
        } else {
            Vec::new()
        });
        if let Ok(mut slot) = self.eol_marks.lock() {
            *slot = Arc::clone(&marks);
        }
        marks
    }

    async fn read_eol_marks(
        &self,
        workdir: &Path,
        status: &WorkTreeStatus,
        cancel: &CancellationToken,
    ) -> Vec<EolMark> {
        let run = |args: &[&'static str]| {
            let cmd = GitCommand::new().cwd(workdir).args(args.to_vec());
            self.executor.run(cmd, cancel)
        };
        let (unstaged, staged, untracked) = tokio::join!(
            run(&["diff", "--no-ext-diff"]),
            run(&["diff", "--cached", "--no-ext-diff"]),
            eol::untracked_shapes(&self.executor, workdir, cancel),
        );

        // A path with something to say from either side says it once — the
        // row is one row whichever bucket it is in — but **which** side it
        // was is kept, because only the index goes into a commit.
        let mut readings: BTreeMap<String, (eol::Reading, bool)> = BTreeMap::new();
        if let Ok(out) = unstaged {
            for seen in eol::read(&out.stdout) {
                readings.insert(seen.path, (seen.reading, false));
            }
        }
        if let Ok(out) = staged {
            for seen in eol::read(&out.stdout) {
                readings.insert(seen.path, (seen.reading, true));
            }
        }
        // Only files status actually reports: a path spelled differently by
        // the patch header (git C-quotes the awkward ones) has no row to
        // put a mark on, and guessing which row it meant is worse than
        // leaving it unmarked.
        let pending: std::collections::HashSet<&str> =
            status.items.iter().map(|i| i.path()).collect();
        readings.retain(|path, _| pending.contains(path.as_str()));

        for (path, shape) in untracked.unwrap_or_default() {
            if !pending.contains(path.as_str()) {
                continue;
            }
            // Untracked means nothing of it is in the index yet, so
            // whatever it says is about the next `git add`, not this
            // commit.
            match shape {
                eol::Shape::Uniform(eol) => {
                    readings.insert(path, (eol::Reading::NewFile { eol }, false));
                }
                // The one shape the column cannot finish: the sentence
                // counts lines, and only the patch has them.
                eol::Shape::Mixed => {
                    let target = DiffTarget::Untracked { path: path.clone() };
                    if let Ok(raw) =
                        details::file_diff_raw(&self.executor, workdir, &target, cancel).await
                    {
                        readings.insert(path, (eol::read_one(&raw), false));
                    }
                }
                eol::Shape::Nothing => {}
            }
        }
        readings.retain(|_, (reading, _)| *reading != eol::Reading::Quiet);
        if readings.is_empty() {
            return Vec::new();
        }

        // git's word on which of these are text at all, in one spawn for
        // the lot rather than one per file.
        let paths: Vec<String> = readings.keys().cloned().collect();
        let rulings = eol::rulings(&self.executor, workdir, &paths, cancel)
            .await
            .unwrap_or_else(|e| {
                if !e.is_cancelled() {
                    tracing::debug!(error = %e, "line-ending rulings failed");
                }
                Vec::new()
            });

        let mut marks = Vec::new();
        for (path, ruling) in paths.into_iter().zip(rulings) {
            let Some((reading, staged)) = readings.get(&path).copied() else {
                continue;
            };
            if ruling == eol::Ruling::NotText {
                continue;
            }
            let baseline = match ruling {
                eol::Ruling::NotText | eol::Ruling::Normalised => None,
                // Only the two estimated cases pay for a sample, and the
                // cache means a directory pays once.
                eol::Ruling::Open if reading.is_exact() => None,
                eol::Ruling::Open => self.eol_baseline(workdir, &path, cancel).await,
            };
            if let Some(notice) = eol::settle(reading, baseline.as_ref()) {
                marks.push(EolMark {
                    path,
                    notice,
                    staged,
                });
            }
        }
        marks
    }
}
