//! The stash that took the working tree away, and the tree turning out
//! to be empty behind it.

/// One stash press that empties the working tree, from the press to the
/// reading that shows the tree gone.
///
/// **Two moments, and the second is not the write's answer.** git
/// answering says the entry was made; what the page is waiting for is
/// the status behind it, where the tree turns out to have nothing left
/// in it — and only then does whose doing that was matter at all.
/// Somebody committing the same changes in a terminal empties the tree
/// exactly the same way, and must not take a reader off the message they
/// are still writing.
///
/// So this **stands** from the answer until the page takes it
/// ([`Self::taken`]), rather than being read off whichever answer is in
/// hand. The answer and the status travel feeds of their own, and
/// between the two a fetch running behind the press comes back: read off
/// a property every answer rewrites, the landing is taken down by that
/// fetch before the tree it emptied has even been read, and the reader
/// is left standing over a pane that no longer describes anything.
///
/// **By id, never by turn**, because every stash operation answers under
/// the same word — a pop pressed from the details band answers exactly
/// like the push this is about, and a drop answers like both.
///
/// **Which status can answer at all is a stamp.** The counts say which
/// report of HEAD they were read beside (`WorkTreeModel.statusSeq`), and
/// the write's answer names the smallest one that can speak for what it
/// left. A status from before that fence describes the tree as it was —
/// read as the answer, the dirty tree it still shows would settle the
/// press before the empty one ever arrived, and the pane would sit over
/// a tree it no longer describes. Reports of HEAD arrive on their own as
/// well, carrying no counts at all, which is the same trap one step in.
///
/// **Only the presses that can empty a tree write here.** A pop or an
/// apply puts changes back, and a drop or a rename leaves the tree
/// alone, so none of them can be the landing this describes; armed with
/// them, it would say "ours" about a tree somebody else emptied.
#[derive(Debug, Default)]
pub struct StashOut {
    /// The write the tree is waiting on, as the bridge carries an
    /// `OperationId` — zero while none is out.
    waiting: u64,
    /// That write landed and nothing has read the tree it left yet.
    ours: bool,
    /// The smallest report the counts can stand beside and still speak
    /// for that write; zero while no answer is standing.
    after: u64,
}

impl StashOut {
    /// The press: the stash went to the queue and came back with this
    /// id.
    ///
    /// Whatever an earlier press left standing goes with it — the tree
    /// emptying next is the work of the stash asked for last. Where the
    /// queue accepted nothing (the session is closed) nothing is waited
    /// for and nothing is claimed: no answer is coming, and a tree that
    /// empties afterwards was emptied by somebody else.
    pub fn asked(&mut self, accepted: Option<u64>) {
        self.ours = false;
        self.after = 0;
        let Some(id) = accepted.filter(|id| *id != 0) else {
            tracing::debug!("stash the queue took nothing for empties no tree of ours");
            self.waiting = 0;
            return;
        };
        self.waiting = id;
    }

    /// git answered a write, naming `after` as the smallest report the
    /// counts that can speak for it will stand beside. Only this one's
    /// own answer claims the tree, and only where git made the entry: a
    /// refusal took nothing away, so a tree that empties after it is not
    /// this press's doing.
    ///
    /// Says whether it was this one's, so the caller can tell a stash
    /// this window pressed from one it only watched.
    pub fn answered(&mut self, id: u64, failed: bool, after: u64) -> bool {
        if self.waiting == 0 || self.waiting != id {
            return false;
        }
        self.waiting = 0;
        self.ours = !failed;
        self.after = after;
        true
    }

    /// The page is reading a status whose counts stand beside the report
    /// numbered `seen`, and `emptied` is whether they leave the tree with
    /// nothing in it.
    ///
    /// Says whether this window's own stash is what emptied it. **The
    /// first status that can speak for the press settles it either way**
    /// — a tree that still has something in it settles it too, since the
    /// stash plainly did not take that away, and nothing is left standing
    /// for a later emptying somebody else made.
    pub fn read(&mut self, seen: u64, emptied: bool) -> bool {
        if !self.ours || self.after == 0 || seen < self.after {
            return false;
        }
        self.ours = false;
        self.after = 0;
        emptied
    }

    /// Whether a landing is still standing, for the tests that ask what
    /// a press left behind.
    #[cfg(test)]
    pub fn ours(&self) -> bool {
        self.ours
    }
}
