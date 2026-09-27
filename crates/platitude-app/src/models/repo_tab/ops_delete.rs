//! The deletes, and the rows they take off the screen before git has
//! answered for them (デザイン規約 §消す操作は先に画面から消す).
//!
//! Every press asks the session and writes down the rows it took away
//! under the id it got back, so every row that moves has an answer
//! coming. What the waiting means is `ops::StandIn`'s (tested in
//! `ops::stand_in_tests`); this file is the wiring.

use super::*;

use crate::ops::{Row, StandIn};

impl RepoTab {
    /// `git branch --delete` (`-D` under `force`). Only the plain form is
    /// kept for its answer (`branch_delete_out`).
    pub(super) fn branch_delete(&mut self, name: String, force: bool) {
        let asked = self.ask_session(|s| s.delete_branch(name.clone(), force));
        let accepted = asked.map(platitude_core::OperationId::as_u64);
        if !force && self.arm_branch_delete(&name, accepted) {
            // QML is told the last answer is gone, so a delete of a
            // re-made branch of the same name reads its own answer as
            // a change.
            self.changed();
        }
        self.took_away(&[(Row::Branch, &name)], asked);
    }

    /// Writes the plain delete down against its write's id and takes the
    /// last answer down. Says whether the picture QML draws from moved;
    /// apart from the notify so a test can hold it against `settle_write`.
    pub(super) fn arm_branch_delete(&mut self, name: &str, accepted: Option<u64>) -> bool {
        self.branch_delete_out.asked(name, accepted);
        self.read_branch_delete_out()
    }

    /// Copies the card's picture out of its owner, as [`Self::stand_in`]
    /// does for `gone_*`. Says whether any of it moved.
    pub(super) fn read_branch_delete_out(&mut self) -> bool {
        let landed = self.branch_delete_out.landed().to_string();
        let refused = self.branch_delete_out.refused().to_string();
        let at = self
            .branch_delete_out
            .answer()
            .and_then(|at| i32::try_from(at).ok())
            .unwrap_or(-1);
        if landed == self.branch_delete_landed
            && refused == self.branch_delete_refused
            && at == self.branch_delete_answer
        {
            return false;
        }
        self.branch_delete_landed = landed;
        self.branch_delete_refused = refused;
        self.branch_delete_answer = at;
        true
    }

    /// `git push <remote> --delete <branch>`: only the remote row leaves,
    /// keyed `<remote>/<branch>` as the row is — the halves were cut with
    /// the configured names (`GitFacts.remoteOfRef`), since a remote's
    /// name may hold a slash.
    pub(super) fn remote_branch_delete(&mut self, remote: String, branch: String) {
        let row = format!("{remote}/{branch}");
        let asked = self.ask_session(|s| s.delete_remote_branch(remote.clone(), branch.clone()));
        self.ref_push_asked(&row, asked.map(platitude_core::OperationId::as_u64));
        self.took_away(&[(Row::Remote, &row)], asked);
    }

    /// Writes down the id a ref row's push was accepted under, with its
    /// row, so the page reports git's answer to that press
    /// (`ops::PushOut`). The rows come back through the delete's own
    /// owner; this carries the refusal's reason, which a fetch in the same
    /// drain would take over in the group.
    pub(super) fn ref_push_asked(&mut self, row: &str, accepted: Option<u64>) {
        self.ref_push_out.asked(accepted, row.to_string());
    }

    /// Both halves as one queued write, so it is one thing to put back.
    pub(super) fn branch_delete_everywhere(
        &mut self,
        branch: String,
        remote: String,
        remote_branch: String,
        force: bool,
    ) {
        let row = format!("{remote}/{remote_branch}");
        let asked = self.ask_session(|s| {
            s.delete_branch_everywhere(branch.clone(), remote.clone(), remote_branch.clone(), force)
        });
        self.ref_push_asked(&row, asked.map(platitude_core::OperationId::as_u64));
        self.took_away(&[(Row::Branch, &branch), (Row::Remote, &row)], asked);
    }

    /// `git tag --delete`: only the name goes.
    pub(super) fn tag_delete(&mut self, name: String) {
        let asked = self.ask_session(|s| s.delete_tag(name.clone()));
        self.took_away(&[(Row::Tag, &name)], asked);
    }

    /// `git push <remote> --delete refs/tags/<tag>` — fully qualified: a
    /// bare name a branch shares over there is refused and neither is
    /// deleted (`remote::delete_remote_tag`).
    ///
    /// `row_goes`: the sidebar's row leaves only where the remote alone
    /// had the name; one held here too keeps its row and loses the badge
    /// (`RefTagMenu`). The row knows which, so it travels with the press.
    pub(super) fn remote_tag_delete(&mut self, remote: String, tag: String, row_goes: bool) {
        let asked = self.ask_session(|s| s.delete_remote_tag(remote.clone(), tag.clone()));
        self.ref_push_asked(
            &format!("{remote}/{tag}"),
            asked.map(platitude_core::OperationId::as_u64),
        );
        let rows: &[(Row, &str)] = if row_goes { &[(Row::Tag, &tag)] } else { &[] };
        self.took_away(rows, asked);
    }

    /// `git tag --delete` and then the remote's copy, as one queued
    /// write: the local half first, so a pair that stops part-way
    /// leaves the name only on the remote.
    pub(super) fn tag_delete_everywhere(&mut self, tag: String, remote: String) {
        let asked = self.ask_session(|s| s.delete_tag_everywhere(tag.clone(), remote.clone()));
        self.ref_push_asked(
            &format!("{remote}/{tag}"),
            asked.map(platitude_core::OperationId::as_u64),
        );
        self.took_away(&[(Row::Tag, &tag)], asked);
    }

    /// `git stash drop` (destructive). No chip goes: a stash is a row of
    /// its own, leaving with the walk.
    pub(super) fn stash_drop(&mut self, selector: String) {
        let asked = self.ask_session(|s| s.stash_drop(selector.clone()));
        self.took_away(&[(Row::Stash, &selector)], asked);
    }

    /// `git worktree remove`. The WORKTREES row goes, keyed by the path
    /// the row carries; the branch it had out keeps its row.
    pub(super) fn worktree_remove(&mut self, path: String, name: String) {
        let asked = self.ask_session(|s| s.remove_worktree(path.clone(), name.clone()));
        self.took_away(&[(Row::Worktree, &path)], asked);
    }

    /// git's answer to any write. Refused, the rows come back before the
    /// drain's notify, so the row is on screen when the reader is told
    /// (`RepoPage.absorbWriteResult`); landed, they stay away until a
    /// reading proves them gone. Another write's answer moves nothing
    /// (`ops::StandIn`).
    pub(super) fn delete_answered(&mut self, id: u64, failed: bool, reads_from: u64) {
        // No notify: this runs inside the drain, and one raised mid-batch
        // would show a half-drained tab (`take_feed`).
        self.stand_in(|gone| gone.answered(id, failed, reads_from));
    }

    /// A list has drawn a reading, so each row is asked again whether its
    /// list has caught up. Which list is not passed: every row answers off
    /// its own, so naming one could only name it wrongly
    /// (`ops::StandIn::look_again`).
    pub(super) fn note_listing_drawn(&mut self) {
        if self.stand_in(StandIn::look_again) {
            self.changed();
        }
    }

    /// Takes the picture of whatever this tab is already standing in for,
    /// without moving anything (`attach_feed`): the stand-in is the hub's
    /// and outlives the page, so a page built over a delete still out has
    /// to be told.
    pub(super) fn read_stand_in(&mut self) {
        self.stand_in(|_| {});
    }

    /// Takes a press's rows off the screen under its write's id, and
    /// notifies so they leave at the press.
    fn took_away(&mut self, rows: &[(Row, &str)], accepted: Option<platitude_core::OperationId>) {
        let accepted = accepted.map(platitude_core::OperationId::as_u64);
        if self.stand_in(|gone| gone.asked(rows, accepted)) {
            self.changed();
        }
    }

    /// Moves this tab's stand-in on and copies its picture into the five
    /// `gone_*`, saying whether they moved. The copy keeps bindings out of
    /// the hub's borrow (rules/app-ui.md §Qt Bridges・QML の不変条件).
    fn stand_in(&mut self, f: impl FnOnce(&mut StandIn)) -> bool {
        let rows = crate::hub::stand_in(self.tab_id, f);
        if rows.branch == self.gone_branch
            && rows.remote == self.gone_remote
            && rows.tag == self.gone_tag
            && rows.stash == self.gone_stash
            && rows.worktree == self.gone_worktree
        {
            return false;
        }
        self.gone_branch = rows.branch;
        self.gone_remote = rows.remote;
        self.gone_tag = rows.tag;
        self.gone_stash = rows.stash;
        self.gone_worktree = rows.worktree;
        true
    }
}
