//! The file read again when a write answered, and the status coming
//! behind that write.

/// One re-read of the open diff, from the answer that asked for it to
/// the status it is already the answer to.
///
/// The file is read again at the write's answer (git has moved the index by
/// then); the status published behind that write arrives afterwards and,
/// read as news, would ask for the same file again.
///
/// Which status that is, is a stamp: the answer names the smallest number
/// the first report of HEAD after its write can carry
/// (`WriteAnswer::head_seq`), and counts beside a report at or above it
/// were read after the write. Counting answers cannot say this — another
/// write answering in between moves every counter, and a read in flight
/// when the write ended carries a number from before it.
///
/// The status can arrive first (the two feeds are drained apart), so the
/// read is spent at once where it was already applied, and otherwise by
/// the status it answers for. Left standing, it would swallow the next
/// status — somebody else's edit, left unread on screen.
#[derive(Debug, Default)]
pub struct DiffReread {
    /// The first report of HEAD whose counts can speak for the write
    /// this file was read for; zero while no read is standing.
    after: u64,
}

impl DiffReread {
    /// The file was read again for a write whose answer named `after`;
    /// `seen` is the report the counts on screen last stood beside.
    ///
    /// Spent at once where that status is already applied, or where the
    /// write named no report (nothing to measure against, so the status
    /// behind it reads the file once more).
    pub fn read_after(&mut self, after: u64, seen: u64) {
        self.after = if after == 0 || seen >= after {
            0
        } else {
            after
        };
    }

    /// A status arrived beside report `seen`: says whether the standing
    /// read already answers for it, and spends the read where it does.
    /// A report from before the fence leaves the read standing — the one
    /// describing what the write left is still coming.
    pub fn taken(&mut self, seen: u64) -> bool {
        if self.after == 0 || seen < self.after {
            return false;
        }
        self.after = 0;
        true
    }
}
