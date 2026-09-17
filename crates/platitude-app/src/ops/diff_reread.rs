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
/// **Which status is that one is a stamp.** The answer names the
/// smallest number the first report of HEAD after its write can carry
/// (`WriteAnswer::head_seq`), and counts standing beside a report at
/// or above it were read after the write. A count of answers
/// cannot say this: another write answering in between moves every
/// counter without saying anything about which tree the status in hand
/// describes, and a read already in flight when the write ended arrives
/// with a number from before it and would be taken for the write's own.
///
/// **The status can arrive first.** The answer and the status travel
/// feeds of their own and are drained apart, so the one the re-read
/// would have answered for may be already applied by the time the page
/// reads the file. The read is spent at once ([`Self::read_after`]
/// is handed the newest status for exactly this): what the wait was for
/// has been and gone, and the next status to move the tree is somebody
/// else's change — suppressed, it would leave their edit unread on
/// screen.
///
/// **Spent by the status it answers for**, for the same reason.
#[derive(Debug, Default)]
pub struct DiffReread {
    /// The first report of HEAD whose counts can speak for the write
    /// this file was read for; zero while no read is standing.
    after: u64,
}

impl DiffReread {
    /// The file was read again for a write whose answer named `after` as
    /// the first report whose counts can speak for it; `seen` is the
    /// report the counts on screen last stood beside.
    ///
    /// **The read is spent where that status has already
    /// been applied** — it read the file once itself, and
    /// the next status is news. Spent too where the
    /// write named no report at all: there is nothing to
    /// measure a status against, so the one behind it reads
    /// the file once more.
    pub fn read_after(&mut self, after: u64, seen: u64) {
        self.after = if after == 0 || seen >= after {
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
    pub fn taken(&mut self, seen: u64) -> bool {
        if self.after == 0 || seen < self.after {
            return false;
        }
        self.after = 0;
        true
    }
}
