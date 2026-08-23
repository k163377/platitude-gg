//! Staging and discarding a chosen set of rows.

use platitude_core::stage::DiscardSide;

use super::*;

/// One chosen row, as the bridge hands it over: `<bucket>:<path>` — the
/// key the pane's choice already speaks (`WipPane.chosenKeys`). The
/// bucket names which row was chosen, which status alone cannot answer:
/// a file changed on both sides has a row in each bucket. A conflicted
/// row rides along untouched and unparsed — git refuses to restore a
/// conflicted path until told how it was resolved — and a key without a
/// known bucket is a caller's bug.
fn chosen_row(key: &str) -> Option<(String, DiscardSide)> {
    let Some((bucket, path)) = key.split_once(':') else {
        tracing::warn!(key, "a chosen row without a bucket");
        return None;
    };
    let side = match bucket {
        "unstaged" => DiscardSide::Unstaged,
        "untracked" => DiscardSide::Untracked,
        "staged" => DiscardSide::Staged,
        "conflicts" => return None,
        other => {
            tracing::warn!(bucket = other, "a chosen row from an unknown bucket");
            return None;
        }
    };
    Some((path.to_string(), side))
}

impl RepoTab {
    /// The gathered-path writes share one shape: take the set, skip an
    /// empty ask, hand the batch to the session.
    pub(super) fn drain_paths(
        &mut self,
        send: impl FnOnce(&std::sync::Arc<platitude_core::session::RepoSession>, Vec<String>),
    ) {
        let paths = std::mem::take(&mut self.pending_paths);
        if paths.is_empty() {
            return;
        }
        self.with_session(move |s| send(s, paths));
    }

    /// Works out what a discard of the gathered rows would take, before
    /// anything is written: `discardCount` — how many rows it would
    /// touch — and `discardOnly`, the bucket of the one chosen row when
    /// there is exactly one, which picks the tag the menu row wears
    /// (デザイン規約 §その他の操作). Consumes the gathered set the way
    /// every write does, so an abandoned plan cannot be spent later.
    /// The caller raises `changed` (a signal wants an attached object,
    /// and the tests here have none).
    pub(super) fn plan_discard_rows(&mut self) {
        let keys = std::mem::take(&mut self.pending_paths);
        let mut count = 0;
        let mut only = "";
        for (_, side) in keys.iter().filter_map(|key| chosen_row(key)) {
            count += 1;
            only = match side {
                DiscardSide::Unstaged => "unstaged",
                DiscardSide::Untracked => "untracked",
                DiscardSide::Staged => "staged",
            };
        }
        self.discard_count = count;
        self.discard_only = if count == 1 {
            only.to_string()
        } else {
            String::new()
        };
    }

    /// Discards the gathered rows (destructive). The keys are parsed
    /// here; the sorting into commands is core's
    /// (`RepoSession::discard_chosen`), and a set that boils down to
    /// nothing — conflicted rows only — sends nothing.
    pub(super) fn discard_chosen_rows(&mut self) {
        self.drain_paths(|s, keys| {
            let chosen: Vec<(String, DiscardSide)> =
                keys.iter().filter_map(|key| chosen_row(key)).collect();
            if chosen.is_empty() {
                return;
            }
            s.discard_chosen(chosen);
        });
    }

    /// Answers whether a write went out. The pane arms a wait the write's
    /// own answer puts down, so a request turned away here — no worktree
    /// target, nothing selected, no fingerprint yet — must say so: armed
    /// with no answer coming, the wait would hold the marks for good.
    pub(super) fn stage_chosen(
        &mut self,
        kind: String,
        path: String,
        orig_path: String,
        hunk: i32,
        line: i32,
        fingerprint: String,
    ) -> bool {
        let Some(target) = crate::encode::worktree_target(&kind, &path, &orig_path) else {
            tracing::warn!(kind, "selection staging asked for a non-worktree diff");
            return false;
        };
        let selects = crate::encode::hunk_selection(hunk, line);
        if selects.is_empty() {
            return false;
        }
        // The fingerprint of the diff the indices were made on (hex, from
        // DiffModel). Without one the selection addresses nothing.
        let Ok(seen) = u64::from_str_radix(&fingerprint, 16) else {
            tracing::warn!(fingerprint, "selection staging without a diff fingerprint");
            return false;
        };
        let mut queued = false;
        self.with_session(|s| {
            s.apply_partial(target.clone(), selects.clone(), seen);
            queued = true;
        });
        queued
    }

    /// Answers whether a write went out, for the same reason as
    /// [`Self::stage_chosen`].
    pub(super) fn discard_chosen(
        &mut self,
        kind: String,
        path: String,
        orig_path: String,
        hunk: i32,
        line: i32,
        fingerprint: String,
    ) -> bool {
        let Some(target) = crate::encode::worktree_target(&kind, &path, &orig_path) else {
            tracing::warn!(kind, "selection discard asked for a non-worktree diff");
            return false;
        };
        let selects = crate::encode::hunk_selection(hunk, line);
        if selects.is_empty() {
            return false;
        }
        let Ok(seen) = u64::from_str_radix(&fingerprint, 16) else {
            tracing::warn!(fingerprint, "selection discard without a diff fingerprint");
            return false;
        };
        let mut queued = false;
        self.with_session(|s| {
            s.discard_partial(target.clone(), selects.clone(), seen);
            queued = true;
        });
        queued
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chosen_row_splits_at_the_first_colon_only() {
        assert_eq!(
            chosen_row("unstaged:dir/a:b.txt"),
            Some(("dir/a:b.txt".to_string(), DiscardSide::Unstaged)),
            "a path may hold a colon of its own"
        );
        assert_eq!(
            chosen_row("untracked:new.txt"),
            Some(("new.txt".to_string(), DiscardSide::Untracked))
        );
        assert_eq!(
            chosen_row("staged:a.txt"),
            Some(("a.txt".to_string(), DiscardSide::Staged))
        );
    }

    #[test]
    fn a_conflicted_or_unreadable_row_drops_out() {
        assert_eq!(chosen_row("conflicts:both.txt"), None);
        assert_eq!(chosen_row("no-bucket-here"), None);
        assert_eq!(chosen_row("folder:docs"), None, "an unknown bucket");
    }

    #[test]
    fn the_plan_counts_the_rows_a_discard_would_touch() {
        let mut tab = RepoTab::default();
        tab.pending_paths = vec![
            "unstaged:a.txt".into(),
            "conflicts:both.txt".into(),
            "staged:b.txt".into(),
        ];
        tab.plan_discard_rows();
        assert_eq!(tab.discard_count, 2, "the conflicted row is not counted");
        assert_eq!(
            tab.discard_only, "",
            "two rows wear the count, not a bucket"
        );
        assert!(tab.pending_paths.is_empty(), "the plan consumes the set");
    }

    #[test]
    fn a_single_row_names_its_bucket_for_the_tag() {
        for (key, only) in [
            ("unstaged:a.txt", "unstaged"),
            ("untracked:new.txt", "untracked"),
            ("staged:b.txt", "staged"),
        ] {
            let mut tab = RepoTab::default();
            tab.pending_paths = vec![key.into()];
            tab.plan_discard_rows();
            assert_eq!(tab.discard_count, 1);
            assert_eq!(tab.discard_only, only);
        }
    }

    #[test]
    fn a_choice_of_conflicted_rows_alone_plans_nothing() {
        let mut tab = RepoTab::default();
        tab.pending_paths = vec!["conflicts:both.txt".into()];
        tab.plan_discard_rows();
        assert_eq!(tab.discard_count, 0);
        assert_eq!(tab.discard_only, "");
    }
}
