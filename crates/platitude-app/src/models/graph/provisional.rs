//! The discard log's picked entry on the graph (破棄記録仕様.md): the
//! session walks its parts' tips and draws what only those tips reach
//! dashed (`RepoSession::show_discard`); this side answers which rows those
//! are and where they stand, for the rows to stay lit while every other row
//! dims as a search's misses do, and for the view to keep to their span.

use std::collections::{HashMap, HashSet};

use platitude_core::session::ShownDiscard;

use super::*;

impl GraphModel {
    /// Puts the entry on the graph: `tips` are its parts' tips, the first
    /// the one it lands on; `looks` say how each draws — `uncommitted` for a
    /// copy of thrown-away work, `stash` for a dropped stash, both
    /// stash-shaped; `commit` for a commit (`encode::look_word`); `lost` are
    /// the commits only the tips reach (full hex). No tip that is an id
    /// takes the entry off.
    pub(super) fn show_provisional(
        &mut self,
        tips: Vec<String>,
        looks: Vec<String>,
        lost: Vec<String>,
        stands: &[String],
    ) {
        let parsed: Vec<(Oid, String, String)> = tips
            .into_iter()
            .zip(looks.into_iter().chain(std::iter::repeat(String::new())))
            .filter_map(|(hex, look)| Oid::from_hex_str(&hex).ok().map(|oid| (oid, hex, look)))
            .collect();
        let Some((_, first, _)) = parsed.first() else {
            self.hide_provisional();
            return;
        };
        self.provisional_tip = first.clone();
        let shown = ShownDiscard {
            tips: parsed.iter().map(|(oid, _, _)| *oid).collect(),
            lost: lost
                .iter()
                .filter_map(|hex| Oid::from_hex_str(hex).ok())
                .collect(),
            stashlike: parsed
                .iter()
                .filter(|(_, _, look)| stash_shaped(look))
                .map(|(oid, _, _)| *oid)
                .collect(),
            stands: stands.iter().filter_map(|line| stands_of(line)).collect(),
        };
        self.provisional = lost
            .into_iter()
            .chain(parsed.iter().map(|(_, hex, _)| hex.clone()))
            .collect::<HashSet<String>>();
        self.provisional_looks = parsed
            .into_iter()
            .filter(|(_, _, look)| stash_shaped(look))
            .map(|(_, hex, look)| (hex, look))
            .collect::<HashMap<String, String>>();
        self.provisional_on = true;
        self.provisional_walked = false;
        self.settle_provisional();
        self.stats_changed();
        crate::hub::with_session(self.tab_id, |s| s.show_discard(Some(shown)));
    }

    /// The walk that took the entry whose first tip is `tip` has landed:
    /// a tip not drawn by now lies past the window. An answer for an entry
    /// picked over is not this one's.
    pub(super) fn note_provisional_walk(&mut self, tip: &str) {
        if self.provisional_on && !tip.is_empty() && tip == self.provisional_tip {
            self.provisional_walked = true;
        }
    }

    /// Takes the entry off, and the session walks again without it.
    pub(super) fn hide_provisional(&mut self) {
        if self.drop_provisional() {
            crate::hub::with_session(self.tab_id, |s| s.show_discard(None));
        }
    }

    /// Takes the entry off this side alone — for a session that never had
    /// it (`restand`). Answers whether one was on.
    pub(super) fn drop_provisional(&mut self) -> bool {
        if !self.provisional_on {
            return false;
        }
        self.provisional.clear();
        self.provisional_tip.clear();
        self.provisional_looks.clear();
        self.provisional_on = false;
        self.provisional_walked = false;
        self.settle_provisional();
        self.stats_changed();
        true
    }

    /// Whether the row is one the entry would bring back: a part's tip, or
    /// a commit only the tips reach.
    pub(super) fn provisional_row(&self, row: usize) -> bool {
        self.provisional_on
            && self
                .rows
                .get(row)
                .is_some_and(|item| self.provisional.contains(&item.oid_hex))
    }

    /// Whether the row is one of the entry's tips drawn as `look`: a copy of
    /// thrown-away work as an uncommitted row is (破棄記録仕様.md: the picked
    /// entry's head with its uncommitted row), a dropped stash as a stash's.
    pub(super) fn provisional_tip_drawn_as(&self, row: usize, look: &str) -> bool {
        self.provisional_on
            && self
                .rows
                .get(row)
                .and_then(|item| self.provisional_looks.get(&item.oid_hex))
                .is_some_and(|drawn| drawn == look)
    }

    /// Re-reads where the entry's rows stand, after the rows moved or the
    /// entry did. Answers whether anything the bindings read moved; the
    /// revision moves with every call while an entry is shown, since a walk
    /// writes other rows under the same indices.
    pub(super) fn settle_provisional(&mut self) -> bool {
        let (mut first, mut last, mut tip) = (-1, -1, -1);
        if self.provisional_on {
            for i in 0..self.rows.len() {
                if !self.provisional_row(i) {
                    continue;
                }
                let at = i32::try_from(i).unwrap_or(i32::MAX);
                if first < 0 {
                    first = at;
                }
                last = at;
                if self.rows[i].oid_hex == self.provisional_tip {
                    tip = at;
                }
            }
        }
        let moved = (first, last, tip)
            != (
                self.provisional_first,
                self.provisional_last,
                self.provisional_tip_row,
            );
        self.provisional_first = first;
        self.provisional_last = last;
        self.provisional_tip_row = tip;
        self.provisional_revision += 1;
        moved || self.provisional_on
    }
}

/// `<tip> <base> <commit>` read back: the copy `tip`, drawn on `commit`
/// where its base was made.
fn stands_of(line: &str) -> Option<(Oid, platitude_core::discards::Stands)> {
    let mut ids = line.split(' ').map(|hex| Oid::from_hex_str(hex).ok());
    let (Some(Some(tip)), Some(Some(made)), Some(Some(on))) = (ids.next(), ids.next(), ids.next())
    else {
        return None;
    };
    Some((tip, platitude_core::discards::Stands { made, on }))
}

/// Whether a tip that draws as `look` is stash-shaped: its index and
/// untracked commits are the row's own, not rows of their own.
fn stash_shaped(look: &str) -> bool {
    matches!(look, "uncommitted" | "stash")
}
