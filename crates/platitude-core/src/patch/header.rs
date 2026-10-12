//! The lines that open one file's patch, fitted to what the selection left.
//!
//! What a header says about the file itself — renamed, created, deleted —
//! holds for the whole diff only. Applied to part of one, git either refuses
//! (`deleted file … still has contents`, `new file … depends on old contents`,
//! exit 128) or carries the file-level change along: a rename unstaged in
//! part comes back whole, the old name in the index and the new one
//! untracked. A selection changes lines of one file, so a header that would
//! say more is rewritten as a plain change to the file the patch applies to.
//! A mode change is the same kind of thing: `apply` sets the mode a patch
//! names, so the executable bit would ride along with any hunk of its file
//! — staged with it, unstaged with it, or discarded from the working tree
//! with it. Without the pair, `apply` keeps the mode the file has where it
//! lands, and the change stays on its side until the whole file moves.

use std::borrow::Cow;

use super::PatchSide;
use super::render::Tally;

/// Lines that name a rename, which a selection never carries.
const RENAME: [&[u8]; 4] = [
    b"similarity index ",
    b"dissimilarity index ",
    b"rename from ",
    b"rename to ",
];

/// Lines that change the file's mode, which a selection never carries.
const MODE: [&[u8]; 2] = [b"old mode ", b"new mode "];

fn changes_mode(line: &[u8]) -> bool {
    MODE.iter().any(|m| line.starts_with(m))
}

/// `header` as the selection's patch has to open. A whole selection of a
/// creation or a deletion leaves the emptied side empty, and that header
/// stands.
pub(super) fn fitted<'a>(
    header: &[&'a [u8]],
    side: PatchSide,
    tally: &Tally,
) -> Vec<Cow<'a, [u8]>> {
    // The side the patch is applied to stays whole; the other is what the
    // selection makes of it, and a creation or deletion says it is empty.
    let (applied, emptied, left) = match side {
        PatchSide::Forward => (&b"--- "[..], &b"deleted file mode "[..], tally.new),
        PatchSide::Reverse => (&b"+++ "[..], &b"new file mode "[..], tally.old),
    };
    let renamed = header.iter().any(|l| l.starts_with(b"rename from "));
    let not_emptied = left > 0 && header.iter().any(|l| l.starts_with(emptied));
    let Some(name) = header
        .iter()
        .find_map(|l| l.strip_prefix(applied))
        .filter(|_| renamed || not_emptied)
    else {
        return header
            .iter()
            .copied()
            .filter(|l| !changes_mode(l))
            .map(Cow::Borrowed)
            .collect();
    };

    // git appends a tab to a name with a space in it, on these lines only.
    let bare = name.strip_suffix(b"\t").unwrap_or(name);
    let mut out = Vec::with_capacity(header.len());
    for &line in header {
        if line.starts_with(emptied)
            || RENAME.iter().any(|r| line.starts_with(r))
            || changes_mode(line)
        {
            continue;
        }
        if renamed && line.starts_with(b"diff --git ") {
            out.push(Cow::Owned([&b"diff --git "[..], bare, b" ", bare].concat()));
        } else if line.starts_with(b"--- ") || line.starts_with(b"+++ ") {
            // Both sides name the applied file. `apply` takes the name after
            // the first `/`, so the prefix it keeps does not matter.
            out.push(Cow::Owned([&line[..4], name].concat()));
        } else {
            out.push(Cow::Borrowed(line));
        }
    }
    out
}
