//! The file read again when a write answered, and the status coming
//! behind that write.

/// One re-read of the open diff, from the answer that asked for it to
/// the status it is already the answer to.
///
/// **A write moves what the two sides hold, so the file on screen is a
/// picture of a file as it was.** It is read again where the answer is —
/// git has moved the index by the time it answers, so the fresh diff is
/// there to be had — and the status published behind that same write
/// arrives afterwards saying the tree moved. Read as news, it asks for
/// the very file that was just read.
///
/// **Which status is that one is a stamp, not a count.** The answer
/// names the smallest number the first report of HEAD after its write
/// can carry (`WriteAnswer::head_seq`), and counts standing beside a
/// report at or above it were read after the write. A count of answers
/// cannot say this: another write answering in between moves every
/// counter without saying anything about which tree the status in hand
/// describes, and a read already in flight when the write ended arrives
/// with a number from before it and would be taken for the write's own.
///
/// **The status can arrive first.** The answer and the status travel
/// feeds of their own and are drained apart, so the one the re-read
/// would have answered for may be already applied by the time the page
/// reads the file. Nothing is left standing then ([`Self::applied`] is
/// what makes that knowable): what the wait was for has been and gone,
/// and the next status to move the tree is somebody else's change —
/// suppressed, it would leave their edit unread on screen.
///
/// **Spent by the status it answers for**, for the same reason.
#[derive(Debug, Default)]
pub struct DiffReread {
    /// The first report of HEAD whose counts can speak for the write
    /// this file was read for; zero while no read is standing.
    after: u64,
    /// The newest report the counts on screen have stood beside — every
    /// status says which, whether or not a read was waiting for it.
    seen: u64,
}

impl DiffReread {
    /// A status has been applied, whose counts stood beside the report
    /// numbered `seen`. Written down whether or not anything is waiting
    /// for it: what has already been read is the one thing a read
    /// arriving afterwards has to measure itself against.
    pub fn applied(&mut self, seen: u64) {
        self.seen = self.seen.max(seen);
    }

    /// The file was read again for a write whose answer named `after` as
    /// the first report whose counts can speak for it.
    ///
    /// **Nothing is left standing where that status has already been
    /// applied** — it read the file once itself, and the next status is
    /// news. Nor where the write named no report at all: there is
    /// nothing to measure a status against, so the one behind it reads
    /// the file once more rather than skipping a read nobody can prove
    /// was already made.
    pub fn read_after(&mut self, after: u64) {
        self.after = if after == 0 || self.seen >= after {
            0
        } else {
            after
        };
    }

    /// A status has arrived whose counts stand beside the report
    /// numbered `seen`: says whether the read standing here already
    /// answers for it, and spends the read where it does.
    ///
    /// A report from before the fence answers for nothing and leaves the
    /// read standing — it describes the repository as it was, and the
    /// one that describes what the write left is still coming.
    ///
    /// **Writing the status down is [`Self::applied`]'s**, so that the
    /// one arriving with nothing to answer for is still remembered.
    pub fn taken(&mut self, seen: u64) -> bool {
        if self.after == 0 || seen < self.after {
            return false;
        }
        self.after = 0;
        true
    }
}
