//! Staging and discarding a chosen set of rows.

use platitude_core::stage::DiscardSide;

use super::*;

/// One chosen row as `<bucket>:<path>` (`WipPane.chosenKeys`) — the
/// bucket says which row, since a file changed on both sides has one in
/// each. A conflicted row is skipped: git refuses to restore it until told
/// how it was resolved. An unknown bucket is a caller's bug.
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
    /// `send` returns the id its ask was given, which the door keeps for
    /// whoever waits on that write ([`RepoTab::ask_session`]).
    pub(super) fn drain_paths(
        &mut self,
        send: impl FnOnce(
            &std::sync::Arc<platitude_core::session::RepoSession>,
            Vec<String>,
        ) -> Option<platitude_core::OperationId>,
    ) {
        let paths = std::mem::take(&mut self.pending_paths);
        if paths.is_empty() {
            return;
        }
        self.ask_session(move |s| send(s, paths));
    }

    /// Fills `discard_count` / `discard_only` from the gathered rows,
    /// consuming them like every write does. The caller raises `changed`
    /// (a signal needs an attached object, which the tests here lack).
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

    /// Discards the gathered rows (destructive); core sorts them into
    /// commands (`RepoSession::discard_chosen`).
    pub(super) fn discard_chosen_rows(&mut self) {
        self.drain_paths(|s, keys| {
            let chosen: Vec<(String, DiscardSide)> =
                keys.iter().filter_map(|key| chosen_row(key)).collect();
            // Conflicted rows only: no ask, so no id — a waiting reader
            // must not expect an answer.
            (!chosen.is_empty())
                .then(|| s.discard_chosen(chosen))
                .flatten()
        });
    }

    /// Answers whether a write went out: the pane arms a wait only the
    /// write's answer puts down, so a request turned away here must say so
    /// or the marks are held for good.
    pub(super) fn stage_chosen(
        &mut self,
        kind: String,
        path: String,
        orig_path: String,
        hunk: i32,
        line: i32,
        fingerprint: String,
    ) -> bool {
        let Some(target) = crate::encode::working_tree_target(&kind, &path, &orig_path) else {
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
        self.ask_session(|s| s.apply_partial(target.clone(), selects.clone(), seen))
            .is_some()
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
        let Some(target) = crate::encode::working_tree_target(&kind, &path, &orig_path) else {
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
        self.ask_session(|s| s.discard_partial(target.clone(), selects.clone(), seen))
            .is_some()
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
        let mut tab = RepoTab {
            pending_paths: vec![
                "unstaged:a.txt".into(),
                "conflicts:both.txt".into(),
                "staged:b.txt".into(),
            ],
            ..RepoTab::default()
        };
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
            let mut tab = RepoTab {
                pending_paths: vec![key.into()],
                ..RepoTab::default()
            };
            tab.plan_discard_rows();
            assert_eq!(tab.discard_count, 1);
            assert_eq!(tab.discard_only, only);
        }
    }

    #[test]
    fn a_choice_of_conflicted_rows_alone_plans_nothing() {
        let mut tab = RepoTab {
            pending_paths: vec!["conflicts:both.txt".into()],
            ..RepoTab::default()
        };
        tab.plan_discard_rows();
        assert_eq!(tab.discard_count, 0);
        assert_eq!(tab.discard_only, "");
    }
}
