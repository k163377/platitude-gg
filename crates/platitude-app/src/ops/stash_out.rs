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
/// **Neither half is allowed to arrive first.** The answer and the
/// status travel feeds of their own and are drained apart, so either can
/// be the one already in hand. Both are written down as they come
/// ([`Self::answered`], [`Self::applied`]) and [`Self::taken`] answers
/// off the pair — a press settled only from the status side loses the
/// tree that emptied before its answer was read, and one settled only
/// from the answer side loses it the other way round. The reader is left
/// over a pane describing a tree that is gone either way.
///
/// **By id, never by turn**, because every stash operation answers under
/// the same word — a pop pressed from the details band answers exactly
/// like the push this is about, and a drop answers like both.
///
/// **Which status can answer at all is a stamp.** The counts say which
/// report of HEAD they were read beside (`WorkTreeModel.statusSeq`), and
/// the write's answer names the smallest one that can speak for what it
/// left. A status from before that fence describes the tree as it was;
/// read as the answer, its dirty tree would settle the press before the
/// empty one ever arrived. Reports of HEAD arrive on their own as well,
/// carrying no counts at all, which is the same trap one step in.
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
    /// The newest report the counts on screen have stood beside, and
    /// whether they left the tree with nothing in it — written down by
    /// every status, so an answer arriving behind one can settle on it.
    seen: u64,
    seen_emptied: bool,
    /// Where this press's own answer stands in the answers a notify
    /// carried, or `None` where that notify carried none of it. **A
    /// different wait from the tree's**: this one is the page's cue to
    /// re-read the open file or to say what git refused, and it is put
    /// down at the top of every drain.
    at: Option<usize>,
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

    /// Puts down where the last notify's answer stood: this drain
    /// answers for itself. What the press is waiting for from the tree
    /// is left alone — that wait outlives any number of notifies.
    pub fn new_notify(&mut self) {
        self.at = None;
    }

    /// git answered a write, standing at `at` in this notify's answers
    /// and naming `after` as the smallest report the counts that can
    /// speak for it will stand beside.
    ///
    /// Only this one's own answer claims the tree, and only where git
    /// made the entry: a refusal took nothing away, so a tree that
    /// empties after it is not this press's doing. Says whether it was
    /// this one's — the answer is the press's to make sense of either
    /// way, since the file it left stale and the words it was refused
    /// with are nobody else's.
    pub fn answered(&mut self, id: u64, at: usize, failed: bool, after: u64) -> bool {
        if self.waiting == 0 || self.waiting != id {
            return false;
        }
        self.waiting = 0;
        self.ours = !failed;
        self.after = after;
        self.at = Some(at);
        true
    }

    /// A status has been applied, whose counts stood beside the report
    /// numbered `seen` and left the tree empty or not. Written down
    /// whether or not an answer is standing: the answer may still be
    /// coming, and this is the only record of what it would settle on.
    pub fn applied(&mut self, seen: u64, emptied: bool) {
        if seen < self.seen {
            return;
        }
        self.seen = seen;
        self.seen_emptied = emptied;
    }

    /// The page is acting on the tree it has: says whether this window's
    /// own stash is what emptied it, and settles the press.
    ///
    /// **Answered off whichever half arrived second**, so the page asks
    /// this both where a status lands and where a write answers. The
    /// first status that can speak for the press settles it either way —
    /// a tree that still has something in it settles it too, since the
    /// stash plainly did not take that away, and nothing is left
    /// standing for a later emptying somebody else made.
    pub fn taken(&mut self) -> bool {
        if !self.ours || self.after == 0 || self.seen < self.after {
            return false;
        }
        self.ours = false;
        self.after = 0;
        self.seen_emptied
    }

    /// Where this press's own answer stands in this notify, or `None`
    /// where this notify carried none.
    #[must_use]
    pub fn answer(&self) -> Option<usize> {
        self.at
    }

    /// Whether a landing is still standing, for the tests that ask what
    /// a press left behind.
    #[cfg(test)]
    pub fn ours(&self) -> bool {
        self.ours
    }
}
