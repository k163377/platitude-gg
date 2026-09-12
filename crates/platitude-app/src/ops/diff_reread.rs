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
/// can carry (`WriteAnswer::head_seq`), and a report at or above it
/// looked after the write. A count of answers cannot say this: another
/// write answering in between moves every counter without saying
/// anything about which tree the status in hand describes, and a read
/// already in flight when the write ended arrives with a number from
/// before it and would be taken for the write's own.
///
/// **Spent by the report it answers for**, so the next status to move
/// the tree — somebody editing the file in another window — is news
/// again and the file is read for it.
#[derive(Debug, Default)]
pub struct DiffReread {
    /// The first report of HEAD that can speak for the write this file
    /// was read for; zero while no read is standing.
    after: u64,
}

impl DiffReread {
    /// The file was read again for a write whose answer named `after` as
    /// the first report of HEAD that can speak for it.
    ///
    /// Zero is a write that named no report — nothing to measure a
    /// status against, so nothing is claimed and the status behind it
    /// reads the file once more.
    pub fn read_after(&mut self, after: u64) {
        self.after = after;
    }

    /// A status has arrived carrying the report of HEAD numbered
    /// `head_seq`: says whether the read standing here already answers
    /// for it, and spends the read where it does.
    ///
    /// A report from before the fence answers for nothing and leaves the
    /// read standing — it describes the repository as it was, and the
    /// one that describes what the write left is still coming.
    pub fn taken(&mut self, head_seq: u64) -> bool {
        if self.after == 0 || head_seq < self.after {
            return false;
        }
        self.after = 0;
        true
    }
}
