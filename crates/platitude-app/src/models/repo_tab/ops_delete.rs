//! The deletes, and the rows they take off the screen before git has
//! answered for them (デザイン規約 §消す操作は先に画面から消す).
//!
//! **Request, wait and put-down in one place.** Every press here asks the
//! session and, with the id it comes back with, writes down what the press
//! took away — so every press is written with the rows it moves, and
//! every row that moves has an answer coming for it. What the waiting
//! *means* is `ops::StandIn`'s, which knows nothing of Qt or git and is
//! where the transitions are tested (`ops::stand_in_tests`); this file
//! is the wiring: git on one side, the properties QML draws from on the
//! other.

use super::*;

use crate::ops::{Row, StandIn};

impl RepoTab {
    /// `git branch --delete` (`-D` under `force`). The plain form is the
    /// one press whose menu stays up for git's answer, so which branch it
    /// was about is kept until that answer comes (`settle_write`); the
    /// forced form was already the answer to a refusal and stands for
    /// nothing.
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

    /// Writes the plain delete down as the question the next answer to
    /// **that write** is about, and takes the last answer down with it —
    /// the check's beat (`look_up_branch_delete`). Says whether the
    /// picture QML draws from moved. Apart from the notify so a test can
    /// hold it against `settle_write`.
    pub(super) fn arm_branch_delete(&mut self, name: &str, accepted: Option<u64>) -> bool {
        self.branch_delete_out.asked(name, accepted);
        self.read_branch_delete_out()
    }

    /// Copies the picture the card is drawn from out of its owner, the
    /// way the four `gone_*` are copied out of the delete's
    /// ([`Self::stand_in`]). Says whether any of it moved.
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

    /// `git push <remote> --delete <branch>`: the reading over there goes
    /// and the local branch is untouched, so the row that leaves is the
    /// remote one. Named to the sidebar the way the row is — the remote's
    /// own name may hold a slash, and the two halves were cut apart with
    /// the configured names (`GitFacts.remoteOfRef`), so joining them back
    /// is what the row is keyed by.
    pub(super) fn remote_branch_delete(&mut self, remote: String, branch: String) {
        let row = format!("{remote}/{branch}");
        let asked = self.ask_session(|s| s.delete_remote_branch(remote.clone(), branch.clone()));
        self.ref_push_asked(&row, asked.map(platitude_core::OperationId::as_u64));
        self.took_away(&[(Row::Remote, &row)], asked);
    }

    /// Writes down the id a push a ref row sent was accepted under, with
    /// the row it was about, so git's answer to **that** press is what
    /// the page reports (`ops::PushOut`).
    ///
    /// **The answer carries more than the rows coming back.** The
    /// delete's own owner puts those back either way; what only the
    /// refusal carries is why, and a refusal folded into the group is one
    /// a fetch answering in the same drain takes over.
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

    /// `git push <remote> --delete refs/tags/<tag>`. Fully qualified,
    /// because a bare name a branch shares over there is refused and
    /// neither is deleted (measured — `remote::delete_remote_tag`).
    ///
    /// `row_goes` is whether the sidebar's row leaves with the remote
    /// copy: a name only the remote had has nothing left here to keep
    /// a row, while one this repository holds too keeps its row and
    /// loses only the badge (`RefTagMenu`). Which of the two it is,
    /// is the row's own reading, which is why it travels with
    /// the press.
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

    /// `git stash drop` (destructive). A dropped entry has no chip: a
    /// chip is a name on a row, and a stash is the row, leaving
    /// with the walk.
    pub(super) fn stash_drop(&mut self, selector: String) {
        let asked = self.ask_session(|s| s.stash_drop(selector.clone()));
        self.took_away(&[(Row::Stash, &selector)], asked);
    }

    /// git's answer to a write, whichever one it was. Refused, and the
    /// rows come back **before the drain's notify goes out**, so the row a
    /// refusal is about is on screen at the moment the reader is told
    /// about it (`RepoPage.absorbWriteResult`); landed, and they stay away
    /// until a reading below proves them gone. An answer that is not this
    /// delete's own moves nothing (`ops::StandIn`).
    pub(super) fn delete_answered(&mut self, id: u64, failed: bool, reads_from: u64) {
        // The drain notifies once the whole batch is absorbed
        // (`take_feed`), and this runs inside it. One raised here would
        // let the page read a half-drained tab.
        self.stand_in(|gone| gone.answered(id, failed, reads_from));
    }

    /// A list has drawn a reading, so the rows are asked again whether
    /// the lists that draw them have caught up — and the ones that can go
    /// are drawn back in. **Every row answers off its own list**, so
    /// naming the one that said so could only name it wrongly
    /// (`ops::StandIn::look_again`).
    pub(super) fn note_listing_drawn(&mut self) {
        if self.stand_in(StandIn::look_again) {
            self.changed();
        }
    }

    /// Takes the picture of whatever this tab is already standing in for,
    /// without moving anything (`attach_feed`).
    ///
    /// **A page joins the operation already out.** The machine is the
    /// hub's and outlives whichever component draws it, so a page built
    /// over a tab with a delete still out has to be told what that is;
    /// without this the four properties would say nothing while the
    /// owner said otherwise, and the first reading to arrive would hide
    /// rows on their way back.
    pub(super) fn read_stand_in(&mut self) {
        self.stand_in(|_| {});
    }

    /// Takes a press's rows off the screen under the id its write was
    /// accepted with, and raises the notify that draws them gone — the
    /// row leaves at the press, which is the whole of what this is for.
    fn took_away(&mut self, rows: &[(Row, &str)], accepted: Option<platitude_core::OperationId>) {
        let accepted = accepted.map(platitude_core::OperationId::as_u64);
        if self.stand_in(|gone| gone.asked(rows, accepted)) {
            self.changed();
        }
    }

    /// Moves this tab's stand-in on and copies the picture it leaves into
    /// the four properties QML draws from, saying whether they moved.
    ///
    /// The copy is what keeps a binding out of the hub: a property read
    /// while the hub is borrowed is the re-entrant borrow the bridge
    /// panics on (規約 §Qt Bridges の要点), and the borrow here is over
    /// before the values are written.
    fn stand_in(&mut self, f: impl FnOnce(&mut StandIn)) -> bool {
        let rows = crate::hub::stand_in(self.tab_id, f);
        if rows.branch == self.gone_branch
            && rows.remote == self.gone_remote
            && rows.tag == self.gone_tag
            && rows.stash == self.gone_stash
        {
            return false;
        }
        self.gone_branch = rows.branch;
        self.gone_remote = rows.remote;
        self.gone_tag = rows.tag;
        self.gone_stash = rows.stash;
        true
    }
}
