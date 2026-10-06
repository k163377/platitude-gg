//! Line-ending context: rulings, sampled baselines, and the marks the
//! status read carries for pending files.

use super::*;
use crate::eol;

impl RepoSession {
    /// What is known about a path's line endings before the patch is read.
    ///
    /// A history diff gets no baseline: it would have to sample the
    /// neighbours as they stood at that commit, not the files on disk.
    ///
    /// `cancel` is the read's own (stopped by the read that passes it) and
    /// stops only this path's ruling. The answers kept for every read —
    /// whether git normalises, a (directory, extension)'s baseline — are
    /// read on the session's token: stopped half-way, the baseline would be
    /// kept as "unknown" until a write or a ref move drops it.
    pub(super) async fn ending_context(
        &self,
        workdir: &Path,
        target: &DiffTarget,
        cancel: &CancellationToken,
    ) -> EndingContext {
        let (path, historical) = match target {
            DiffTarget::Commit { path, .. }
            | DiffTarget::Range { path, .. }
            | DiffTarget::Choice { path, .. } => (path, true),
            DiffTarget::Staged { path, .. }
            | DiffTarget::Unstaged { path }
            | DiffTarget::Untracked { path } => (path, false),
        };
        let one = [path.clone()];
        let ruling = match self.normalising(workdir, &self.root_cancel).await {
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
                EndingContext::Open(self.eol_baseline(workdir, path, &self.root_cancel).await)
            }
            // Not knowing is silence: the diff beside it is the thing
            // that was asked for.
            Err(e) => {
                if !e.is_cancelled() {
                    tracing::debug!(error = %e, "line-ending ruling failed");
                }
                EndingContext::Excluded
            }
        }
    }

    /// Whether git normalises line endings here, read once rather than a
    /// process per diff opened. Dropped with the sampled baselines: a
    /// write or a moved ref can bring a different `.gitattributes`.
    async fn normalising(
        &self,
        workdir: &Path,
        cancel: &CancellationToken,
    ) -> Result<bool, GitError> {
        self.eol_normalises
            .get_or_try_init(|| eol::normalises(&self.executor, workdir, cancel))
            .await
    }

    /// The remotes this repository has and where a push goes, read once
    /// rather than a `git config` per poll tick. A config file written
    /// since is looked for first, so a remote added in a terminal is seen
    /// (below).
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

    /// Drops the answers read out of the repository's own config — the
    /// remote list and `core.autocrlf` — when that file has been written
    /// since. `git remote add` in a terminal and the settings screen's
    /// line-ending write (`models::line_endings` spawns against a path)
    /// move no ref and land no write, so nothing else would notice.
    ///
    /// A stat per poll tick, not the process the cache saves. It watches
    /// the repository's own config (`--git-path config`, the common one
    /// for a linked worktree); a global write is seen on the next write that
    /// drops what is derived (one reaching the tree or the history —
    /// `session::write::settle_after`) or ref move, except the push mark
    /// ([`RepoSession::note_push_default`]).
    pub(super) fn forget_what_the_config_decides(&self) {
        let Some(path) = self.config_path() else {
            return;
        };
        let stamp = ConfigStamp::of(&path);
        let mut seen = relock(&self.config_stamp);
        if seen.as_ref() == Some(&stamp) {
            return;
        }
        // Not on the first look: nothing is kept yet.
        if seen.is_some() {
            self.remotes.forget();
            // The eol marks too: a repository just told to convert must
            // stop warning about the files it now converts.
            self.forget_eol_derived();
        }
        *seen = Some(stamp);
    }

    /// Takes the config file as it stands now for read, after a write of
    /// this session's that rewrote it and changed nothing kept from it:
    /// `git branch --delete` rewrites the file to drop `branch.<name>.*`,
    /// section or no section. Without this, every delete reads the remotes
    /// again behind itself.
    ///
    /// An edit that landed beside the write is still seen, one listing
    /// late: the ref the delete moved drops the remotes for the next
    /// listing (`publish_refs`).
    pub(super) fn own_config_rewrite(&self) {
        let Some(path) = self.config_path() else {
            return;
        };
        let stamp = ConfigStamp::of(&path);
        let mut seen = relock(&self.config_stamp);
        // Nothing kept yet is nothing to keep.
        if seen.is_some() {
            *seen = Some(stamp);
        }
    }

    /// Drops the remote answer when the push mark moved under it, and
    /// re-reads refs to say the new one. `seen` is the status tick's read
    /// of the effective config ([`crate::remote::push_marks`]): the stat
    /// above misses a `--global remote.pushDefault`, and without this the
    /// toolbar names the old destination while the send
    /// (`plan_current_push`) goes to the new.
    ///
    /// Only a held answer is compared (none means the next listing reads
    /// the current state anyway); the re-read caches the new mark, so the
    /// next tick finds them equal. The refs pass hashes the mark into
    /// `join_key`, so the snapshot goes out fresh though no ref moved.
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
        {
            let mut baselines = relock(&self.eol_baselines);
            if self.keeps_what_it_reads() {
                baselines.insert(key, fresh.clone());
            }
        }
        fresh
    }

    /// Drops everything read once and kept, before a write's refresh: a
    /// write can bring a new `.gitattributes`, change the neighbours or add
    /// a remote.
    pub(super) fn forget_derived(&self) {
        self.forget_eol_derived();
        self.remotes.forget();
    }

    /// Drops only the line-ending context, as after HEAD moves: remotes are
    /// dropped apart — by a write before its command, by an external move
    /// for the next read.
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
    /// Three reads for the whole tree: the two pending diffs (sized by the
    /// change) and one `ls-files --eol` for untracked files, whose column
    /// answers without a patch — except for a mixed new file.
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

        // One reading per path (one row), but the side is kept: only the
        // index goes into a commit.
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
        // Only paths status reports: one the patch header C-quotes has no
        // row, and goes unmarked.
        let pending: std::collections::HashSet<&str> =
            status.items.iter().map(|i| i.path()).collect();
        readings.retain(|path, _| pending.contains(path.as_str()));

        for (path, shape) in untracked.unwrap_or_default() {
            if !pending.contains(path.as_str()) {
                continue;
            }
            // Untracked: nothing in the index yet, so it speaks of the next
            // `git add`.
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

        // Which are text at all, in one spawn.
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
                // Only the two estimated cases pay for a sample.
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
