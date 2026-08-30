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
        let one = [path.clone()];
        let ruling = match self.normalising(workdir, cancel).await {
            Ok(converting) => eol::rulings_given(&self.executor, workdir, &one, converting, cancel)
                .await
                .map(|mut r| r.pop().unwrap_or(eol::Ruling::Open)),
            Err(e) => Err(e),
        };
        match ruling {
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

    /// Whether git normalises line endings on its own here, read once.
    ///
    /// A property of the repository's configuration rather than of any
    /// path, and it was being read again for every diff opened — a whole
    /// process on the way to showing a file. Dropped by the same event
    /// that drops the sampled baselines, because a write or a moved ref is
    /// also what can bring a different `.gitattributes` along with it.
    async fn normalising(
        &self,
        workdir: &Path,
        cancel: &CancellationToken,
    ) -> Result<bool, GitError> {
        self.eol_normalises
            .get_or_try_init(|| eol::normalises(&self.executor, workdir, cancel))
            .await
    }

    /// The remotes this repository has and where a push goes, read once.
    ///
    /// Every refs listing wanted them and every refs listing spawned a
    /// `git config` to ask — a process per poll tick for a list that only
    /// a write moves. A config file that has been written since is looked
    /// for first, so a remote added in a terminal is seen by the poll
    /// (below) rather than waiting for a write or a ref move.
    pub(super) async fn remotes(
        &self,
        workdir: &Path,
        cancel: &CancellationToken,
    ) -> Result<remote::Remotes, GitError> {
        self.forget_what_the_config_decides();
        self.remotes
            .get_or_try_init(|| remote::read(&self.executor, workdir, cancel))
            .await
    }

    /// Drops the answers read out of the repository's own configuration
    /// when that file has been written since — the remote list and
    /// `core.autocrlf`. `git remote add` in a terminal moves no ref and
    /// lands no write, and neither does the settings screen writing the
    /// line-ending setting for this repository (`models::line_endings`
    /// spawns against a path rather than through a session), so nothing
    /// else here would notice either of them.
    ///
    /// A stat rather than a `git config`: this runs on every poll tick,
    /// and the reason those answers are kept at all is that a process per
    /// tick was too much to pay for them. What it watches is the
    /// repository's own config (a linked worktree shares the common one,
    /// which is what `--git-path config` answers) — a value written into
    /// the user's global config is still only seen on the next write or
    /// ref move. The push mark is the exception: the status tick reads it
    /// in every scope and hands it to [`RepoSession::note_push_default`].
    pub(super) fn forget_what_the_config_decides(&self) {
        let Some(path) = self.config_path() else {
            return;
        };
        let stamp = ConfigStamp::of(&path);
        let mut seen = relock(&self.config_stamp);
        if seen.as_ref() == Some(&stamp) {
            return;
        }
        // Not on the first look: there is no answer being kept yet, and
        // the read below is about to take the current one anyway.
        if seen.is_some() {
            self.remotes.forget();
            // The marks go with it: whether git decides the stored
            // endings is what turns a notice on and off, so a repository
            // that has just been told to convert has to stop warning
            // about the files it now converts.
            self.forget_eol_derived();
        }
        *seen = Some(stamp);
    }

    /// Drops the remote answer when the repository's push mark moved under
    /// it, and sends the refs listing out to say the new one.
    ///
    /// `seen` is what the status tick just read out of the effective
    /// configuration ([`crate::remote::push_marks`]). The stat above never
    /// sees a `git config --global remote.pushDefault` — it writes a file
    /// the stat does not watch — so without this the toolbar keeps naming
    /// the old destination while the send (`plan_current_push`, which
    /// reads the effective configuration) already goes to the new one.
    ///
    /// Only a held answer is compared: no answer yet means the next refs
    /// listing is already going to read the current state. The re-read
    /// converges — it puts the fresh mark in the cache, and the following
    /// tick finds the two equal. The refs pass it asks for re-hashes the
    /// mark into its inputs (`join_key`), so the snapshot goes out fresh
    /// even though no ref moved.
    pub(super) fn note_push_default(self: &Arc<Self>, seen: &Option<remote::PushDefault>) {
        let held = self.remotes.peek(|remotes| remotes.push_default.clone());
        if held.is_some_and(|held| held != *seen) {
            self.remotes.forget();
            self.refresh_refs();
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
        if let Some(hit) = relock(&self.eol_baselines).get(&key).cloned() {
            return hit;
        }
        let fresh = eol::baseline(&self.executor, workdir, path, cancel)
            .await
            .unwrap_or_default();
        relock(&self.eol_baselines).insert(key, fresh.clone());
        fresh
    }

    /// Drops everything read once and kept before a write's refresh. A write
    /// can bring a new `.gitattributes`, change what the neighbours look like
    /// or add a remote, and its refresh must read the post-write answers.
    pub(super) fn forget_derived(&self) {
        self.forget_eol_derived();
        self.remotes.forget();
    }

    /// Drops only the line-ending context after HEAD moves. The refs read
    /// has already asked for remotes by then; write-driven reads invalidated
    /// that answer before the command, while external moves invalidate it
    /// separately for the next read.
    pub(super) fn forget_eol_derived(&self) {
        relock(&self.eol_baselines).clear();
        self.eol_normalises.forget();
        self.eol_marks_stale.store(true, Ordering::SeqCst);
    }

    pub(super) fn eol_marks(&self) -> Arc<Vec<EolMark>> {
        Arc::clone(&relock(&self.eol_marks))
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
        *relock(&self.eol_marks) = Arc::clone(&marks);
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
        let converting = self.normalising(workdir, cancel).await.unwrap_or(false);
        let rulings = eol::rulings_given(&self.executor, workdir, &paths, converting, cancel)
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
