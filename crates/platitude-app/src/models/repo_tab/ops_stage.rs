//! Staging and discarding a chosen set of rows, and parts of the open diff.
//!
//! Every one of them moves at the press, ahead of git (デザイン規約 §バケツ
//! ごとの一覧と、ペインの分け方, §diff の中のステージ, §消す操作は先に画面か
//! ら消す): whole files, a part of the open diff, chosen rows discarded. The
//! press writes its id down with the hub's `ops::StageAhead`, git's answer
//! is handed to it by id (`RepoTab::write_answered`, beside the rest of a
//! write's bookkeeping), and the page has the lists, the counts and the open
//! diff draw again whenever what it holds moves (`staging_seq`). When the
//! moved rows go back or are let go of is the owner's (tested in
//! `ops::stage_ahead_tests` / `ops::stage_parts_tests`); this file is the
//! wiring.

use platitude_core::details::DiffTarget;

use platitude_core::stage::{DiscardSide, PressedOn};

use crate::ops::{DiffSpot, Part, PartVerb, Reach, Side};

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
    /// Stages or unstages whole files, moving their rows at once. Answers
    /// whether anything QML reads moved — the caller says so.
    pub(super) fn move_paths(&mut self, side: Side, paths: Vec<String>) -> bool {
        if paths.is_empty() {
            return false;
        }
        let reach = Reach::Paths(paths.iter().cloned().collect());
        let asked = self.ask_session(move |s| match side {
            Side::Stage => s.stage_paths(paths),
            Side::Unstage => s.unstage_paths(paths),
        });
        self.pressed_ahead(side, reach, asked)
    }

    /// The same for a bucket's `Stage all` / `Unstage all`.
    pub(super) fn move_all(&mut self, side: Side) -> bool {
        let asked = self.ask_session(|s| match side {
            Side::Stage => s.stage_all(),
            Side::Unstage => s.unstage_all(),
        });
        self.pressed_ahead(side, Reach::All, asked)
    }

    /// A stage press made from a slot (`press` asks the door and writes the
    /// press down), told the way the slot's readers hear it: the rows on
    /// `changed`, the Commit button's wait on its own notify
    /// (`commit_waits_changed`) — which the door may have said already in
    /// the same turn, for the write it counted out (`ask_session`).
    pub(super) fn press_told(&mut self, press: impl FnOnce(&mut Self) -> bool) {
        let waiting = self.commit_waits;
        if press(self) {
            self.changed();
        }
        if self.commit_waits != waiting {
            self.commit_waits_changed();
        }
    }

    /// The press, written down with the hub under the id the door gave it;
    /// answers whether anything QML reads moved — a press taken always does.
    /// No notify of its own: the slot that pressed tells what moved, the
    /// rows on its `changed` and the Commit button's wait on its own
    /// (`commit_waits_changed`).
    pub(super) fn pressed_ahead(&mut self, side: Side, reach: Reach, asked: Option<u64>) -> bool {
        let taken = crate::hub::stage_ahead(self.tab_id, |ahead| ahead.asked(side, reach, asked));
        self.read_writes_out(taken)
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
    /// commands (`RepoSession::discard_chosen`). The rows go at the press
    /// (`StageAhead::asked_discard`); answers whether anything QML reads
    /// moved, as [`Self::pressed_ahead`] does.
    pub(super) fn discard_chosen_rows(&mut self) -> bool {
        let keys = std::mem::take(&mut self.pending_paths);
        let chosen: Vec<(String, DiscardSide)> =
            keys.iter().filter_map(|key| chosen_row(key)).collect();
        // Conflicted rows only: nothing goes to git, said to a watch armed
        // for it as the door says a write it turns down (`ask_session`) — a
        // waiting reader must not expect an answer.
        if chosen.is_empty() {
            self.write_watch.asked(None);
            return false;
        }
        let pressed = chosen.clone();
        let asked = self.ask_session(move |s| s.discard_chosen(chosen));
        let taken =
            crate::hub::stage_ahead(self.tab_id, |ahead| ahead.asked_discard(&pressed, asked));
        self.read_writes_out(taken)
    }

    /// Stages or unstages part of one file's diff. Answers whether a write
    /// went out — the pane arms a wait only the write's answer puts down, so
    /// a request turned away here must say so or the marks are held for good
    /// — and whether anything QML reads moved (the part, written down under
    /// the id the door gave it: `StageAhead::asked_part`).
    pub(super) fn stage_chosen(&mut self, spot: Spot) -> (bool, bool) {
        let verb = if spot.kind == "staged" {
            PartVerb::Unstage
        } else {
            PartVerb::Stage
        };
        self.press_part(verb, spot)
    }

    /// The same for throwing part of the unstaged diff away.
    pub(super) fn discard_chosen(&mut self, spot: Spot) -> (bool, bool) {
        self.press_part(PartVerb::Discard, spot)
    }

    /// The part's write through the door, and the part written down with the
    /// hub — not for an untracked file, which a part marks intent-to-add
    /// first: its row and its diff are the status's and the read's to say.
    ///
    /// Pressed on the diff the pane draws, the part is placed in that
    /// drawing: the bytes read and the parts pressed on them still standing
    /// go with it (`StageAhead::pressed_on`, `stage::PressedOn`), so a hunk
    /// pressed while the one before it is still queued lands where it was
    /// drawn, and joins that one's burst (`session::write`).
    fn press_part(&mut self, verb: PartVerb, spot: Spot) -> (bool, bool) {
        let Some(target) = crate::encode::worktree_target(&spot.kind, &spot.path, &spot.orig_path)
        else {
            tracing::warn!(kind = spot.kind, "a part asked of a non-worktree diff");
            return (false, false);
        };
        let selects = crate::encode::hunk_selection(spot.hunk, spot.line);
        let Some(hunk) = selects.first().map(|select| select.hunk) else {
            return (false, false);
        };
        // The fingerprint of the diff the indices were made on (hex, from
        // DiffModel). Without one the selection addresses nothing.
        let Ok(seen) = u64::from_str_radix(&spot.fingerprint, 16) else {
            tracing::warn!(
                fingerprint = spot.fingerprint,
                "a part asked without a diff fingerprint"
            );
            return (false, false);
        };
        let key = crate::encode::diff_key(&target);
        let untracked = matches!(target, DiffTarget::Untracked { .. });
        let on = (!untracked)
            .then(|| crate::hub::stage_ahead(self.tab_id, |ahead| ahead.pressed_on(&key, seen)))
            .flatten()
            .map(|(read, ahead)| PressedOn { seen, read, ahead });
        let asked = self.ask_session(move |s| match (verb, on) {
            (PartVerb::Discard, Some(on)) => s.discard_partial_on(target, selects, on),
            (PartVerb::Discard, None) => s.discard_partial(target, selects, seen),
            (_, Some(on)) => s.apply_partial_on(target, selects, on),
            (_, None) => s.apply_partial(target, selects, seen),
        });
        if untracked {
            return (asked.is_some(), self.read_writes_out(false));
        }
        let part = Part {
            verb,
            path: spot.path,
            whole: spot.whole,
            spot: DiffSpot {
                key,
                fingerprint: seen,
                hunk,
                line: usize::try_from(spot.line).ok(),
            },
        };
        let taken = crate::hub::stage_ahead(self.tab_id, |ahead| ahead.asked_part(part, asked));
        (asked.is_some(), self.read_writes_out(taken))
    }
}

/// A part of the open diff as the pane addresses it: the diff-key prefix
/// (`unstaged` / `staged` / `untracked`), the file, the hunk and its line
/// (negative for the whole hunk), the fingerprint of the bytes on screen
/// (hex), and whether the press takes every change shown
/// (`DiffModel.takesAll`).
pub(super) struct Spot {
    pub kind: String,
    pub path: String,
    pub orig_path: String,
    pub hunk: i32,
    pub line: i32,
    pub fingerprint: String,
    pub whole: bool,
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
