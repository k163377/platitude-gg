//! The per-line state machine [`super::read`] drives, and what it
//! tallies on the way through one file's patch.

use super::{Eol, Reading};

/// Which column of the patch a line came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Side {
    Minus,
    Plus,
    Context,
}

/// How many lines of each ending one side holds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Tally {
    lf: u32,
    crlf: u32,
}

impl Tally {
    pub(super) fn add(&mut self, eol: Eol) {
        match eol {
            Eol::Lf => self.lf += 1,
            Eol::Crlf => self.crlf += 1,
        }
    }

    fn take(&mut self, eol: Eol) {
        match eol {
            Eol::Lf => self.lf = self.lf.saturating_sub(1),
            Eol::Crlf => self.crlf = self.crlf.saturating_sub(1),
        }
    }

    fn count(self, eol: Eol) -> u32 {
        match eol {
            Eol::Lf => self.lf,
            Eol::Crlf => self.crlf,
        }
    }

    fn is_empty(self) -> bool {
        self.lf == 0 && self.crlf == 0
    }

    /// The one ending present, when the other is absent.
    fn sole(self) -> Option<Eol> {
        match (self.lf, self.crlf) {
            (0, 0) => None,
            (_, 0) => Some(Eol::Lf),
            (0, _) => Some(Eol::Crlf),
            _ => None,
        }
    }

    /// The ending that outnumbers the other. A tie has no answer.
    pub(super) fn majority(self) -> Option<Eol> {
        match self.lf.cmp(&self.crlf) {
            std::cmp::Ordering::Greater => Some(Eol::Lf),
            std::cmp::Ordering::Less => Some(Eol::Crlf),
            std::cmp::Ordering::Equal => None,
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct Scan {
    pub(super) path: Option<String>,
    pub(super) minus: Tally,
    pub(super) plus: Tally,
    pub(super) context: Tally,
    /// The old side's last line carried no terminator at all.
    pub(super) minus_bare: bool,
    pub(super) plus_bare: bool,
    /// Binary, combined or unmerged: no line endings to talk about.
    pub(super) quiet: bool,
    /// The old side is `/dev/null`.
    pub(super) added: bool,
    /// The new side is `/dev/null`.
    pub(super) removed: bool,
    pub(super) old_left: u32,
    pub(super) new_left: u32,
    pub(super) last: Option<(Side, Eol)>,
}

impl Scan {
    pub(super) fn in_hunk(&self) -> bool {
        self.old_left > 0 || self.new_left > 0
    }

    pub(super) fn header(&mut self, line: &[u8]) {
        if line.starts_with(b"Binary files ") || line.starts_with(b"GIT binary patch") {
            self.quiet = true;
            return;
        }
        if line.starts_with(b"new file mode ") {
            self.added = true;
            return;
        }
        if line.starts_with(b"deleted file mode ") {
            self.removed = true;
            return;
        }
        if let Some(rest) = text_after(line, b"--- ") {
            if rest == "/dev/null" {
                self.added = true;
            }
            return;
        }
        if let Some(rest) = text_after(line, b"+++ ") {
            if rest == "/dev/null" {
                self.removed = true;
            } else {
                self.path = Some(strip_side_prefix(&rest, "b/"));
            }
            return;
        }
        if line.starts_with(b"@@@") {
            // A combined hunk, reached without its `diff --cc` header.
            self.quiet = true;
            return;
        }
        if let Some((old, new)) = hunk_counts(line) {
            self.old_left = old;
            self.new_left = new;
            self.last = None;
        }
    }

    pub(super) fn content(&mut self, line: &[u8], terminated: bool) {
        // The marker belongs to the line above it and is outside both
        // counts, so it is handled before anything is charged to a side.
        if line.starts_with(b"\\") {
            if let Some((side, eol)) = self.last.take() {
                match side {
                    Side::Minus => {
                        self.minus.take(eol);
                        self.minus_bare = true;
                    }
                    Side::Plus => {
                        self.plus.take(eol);
                        self.plus_bare = true;
                    }
                    Side::Context => {
                        self.context.take(eol);
                        self.minus_bare = true;
                        self.plus_bare = true;
                    }
                }
            }
            return;
        }

        let (side, charge_old, charge_new) = match line.first() {
            Some(b'-') => (Side::Minus, true, false),
            Some(b'+') => (Side::Plus, false, true),
            Some(b' ') => (Side::Context, true, true),
            // An empty line inside a hunk is git's context line for an empty
            // source line with the marker column eaten by a tool in the
            // middle; count it as context so the budgets still land.
            None => (Side::Context, true, true),
            // Anything else means the counts were wrong; leave the hunk and
            // let the header path have the line.
            _ => {
                self.old_left = 0;
                self.new_left = 0;
                self.header(line);
                return;
            }
        };

        if charge_old {
            self.old_left = self.old_left.saturating_sub(1);
        }
        if charge_new {
            self.new_left = self.new_left.saturating_sub(1);
        }

        // Only a terminated line has an ending to classify; an unterminated
        // final line is about to be corrected by `\ No newline` anyway.
        if !terminated {
            self.last = None;
            return;
        }
        let eol = if line.last() == Some(&b'\r') {
            Eol::Crlf
        } else {
            Eol::Lf
        };
        match side {
            Side::Minus => self.minus.add(eol),
            Side::Plus => self.plus.add(eol),
            Side::Context => self.context.add(eol),
        }
        self.last = Some((side, eol));
    }

    pub(super) fn settle(&self) -> Reading {
        if self.quiet || self.removed {
            return Reading::Quiet;
        }
        // Nothing was added, so nothing this change did can have made the
        // endings worse. Deleting the odd lines out of a mixed file is a
        // repair.
        if self.plus.is_empty() {
            return Reading::Quiet;
        }

        if self.added {
            return match (self.plus.sole(), self.plus.majority()) {
                (Some(eol), _) => Reading::NewFile { eol },
                // A new file that arrives already mixed is case (b): the
                // lines that disagree with the rest are all its own.
                (None, Some(file)) => mixed(self.plus, file),
                (None, None) => Reading::Quiet,
            };
        }

        // (d) — the old side had no terminator anywhere. Its only line was
        // the bare one, so `minus_bare` with nothing tallied says it all.
        if self.minus_bare && self.minus.is_empty() && self.context.is_empty() {
            return match self.plus.majority().or_else(|| self.plus.sole()) {
                Some(eol) => Reading::FirstEnding { eol },
                None => Reading::Quiet,
            };
        }

        // (a) — no context lines and each side is uniform. A one-line file
        // whose only line changed its ending lands here too, deliberately:
        // it is the same statement about a smaller file.
        if self.context.is_empty()
            && let (Some(from), Some(to)) = (self.minus.sole(), self.plus.sole())
            && from != to
        {
            return Reading::Flipped { from, to };
        }

        // (b) — what the file uses is what the untouched lines use. Falling
        // back to the old side and then to the new keeps a fully rewritten
        // file answerable; a tie means the file is already so mixed that
        // there is no "this file uses" to name, and nothing is said.
        let Some(file) = self
            .context
            .majority()
            .or_else(|| self.minus.majority())
            .or_else(|| self.plus.majority())
        else {
            return Reading::Quiet;
        };
        mixed(self.plus, file)
    }
}

/// (b) from a side's tally: the added lines that disagree with the file.
fn mixed(plus: Tally, file: Eol) -> Reading {
    let added = match file {
        Eol::Lf => Eol::Crlf,
        Eol::Crlf => Eol::Lf,
    };
    let lines = plus.count(added);
    if lines == 0 {
        return Reading::Quiet;
    }
    Reading::Mixed { lines, added, file }
}

/// `@@ -a,b +c,d @@` → the old and new line counts. A count left out is 1.
fn hunk_counts(line: &[u8]) -> Option<(u32, u32)> {
    let rest = text_after(line, b"@@ ")?;
    let (ranges, _) = rest.split_once(" @@")?;
    let mut old = None;
    let mut new = None;
    for part in ranges.split(' ') {
        let (slot, digits) = match part.as_bytes().first() {
            Some(b'-') => (&mut old, &part[1..]),
            Some(b'+') => (&mut new, &part[1..]),
            _ => continue,
        };
        let count = match digits.split_once(',') {
            Some((_, n)) => n.parse::<u32>().ok()?,
            None => 1,
        };
        *slot = Some(count);
    }
    Some((old?, new?))
}

pub(super) fn text_after(line: &[u8], prefix: &[u8]) -> Option<String> {
    let rest = line.strip_prefix(prefix)?;
    Some(String::from_utf8_lossy(rest).trim_end().to_string())
}

fn strip_side_prefix(path: &str, prefix: &str) -> String {
    let cleaned = path.trim_end_matches('\t');
    cleaned.strip_prefix(prefix).unwrap_or(cleaned).to_string()
}
