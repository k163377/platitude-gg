//! A diff with parts of it pressed — hunks or lines staged, unstaged or
//! discarded — laid out as git will print it once those writes are done.
//!
//! One rule for two readers: the application draws it the moment a part is
//! pressed (デザイン規約 §diff の中のステージ), and a part pressed on that
//! drawing while the ones before it are still queued is placed by it
//! (`stage::apply_partial`'s `Ahead`): the drawing is the diff the press was
//! made on, so its places are the places in what git prints after the parts
//! in front — once what git prints is checked to be exactly that
//! ([`same_lines`]).
//!
//! **Only what the diff alone decides is laid out** ([`After::exact`]). A
//! part leaves a hunk as git lays one — `CONTEXT` lines of context each side
//! (`details::DIFF_SHAPE_ARGS` pins them), cut where a part turned a change
//! at an end into context. Left as read:
//! - **a hunk the part could split**: a run of context longer than twice the
//!   context inside it is where git may cut it in two, as
//!   `diff.interHunkContext` — which the reads do not pin — says
//! - **a hunk ending without a newline** with a change left in it: the note
//!   is about the last line on one side, which a part moves between them
//! - **a diff of more than one patch with hunks**: a part's hunk counts
//!   across the whole diff (`patch::HunkSelect`)
//!
//! Not told apart, and so laid out otherwise than git until the next read:
//! the heading after a hunk's `@@` is the closest line above it that the
//! file's language calls a function (git's `funcname`), read on the old
//! side — a part that adds or takes one away above a later hunk changes that
//! hunk's heading ([`same_lines`] leaves headings out); and where lines
//! repeat, git's next diff may pair them differently ([`same_lines`] then
//! says no).

use crate::parse::diff::{DiffHunk, DiffLine, DiffLineKind, DiffLineLocation, FilePatch};

/// The lines of context git prints each side of a change.
const CONTEXT: usize = 3;

/// Which side of a diff a part moves, by the diff's own numbering: a stage
/// moves the old side (the index) to the new, an unstage or a discard the
/// new side (the index, the disk) back to the old.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Moves {
    Old,
    New,
}

/// One part pressed: a hunk of the diff, one line of it (its place in the
/// hunk's line list) or the whole of it, and the side it moves. Its places
/// are in the diff it was pressed on — the read with the parts pressed
/// before it already in it ([`after`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Part {
    pub hunk: usize,
    pub line: Option<usize>,
    pub moves: Moves,
}

/// Where a line of the laid-out diff came from: its place in the diff read,
/// and whether a part turned it from a change into context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Origin {
    pub at: DiffLineLocation,
    pub turned: bool,
}

/// The diff with the parts in it ([`after`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct After {
    pub patches: Vec<FilePatch>,
    /// Every line's [`Origin`], laid out as the patches are: one list per
    /// patch, one per hunk, one entry per line.
    origin: Vec<Vec<Vec<Origin>>>,
    /// Every part was laid out — none left a hunk as read.
    pub exact: bool,
}

impl After {
    /// Where the line at `at` in [`Self::patches`] came from.
    #[must_use]
    pub fn origin(&self, at: DiffLineLocation) -> Option<Origin> {
        at.entry_in(&self.origin).copied()
    }
}

/// `patches` with `parts` in them, one at a time in their order: each part's
/// places are in the diff the parts before it leave.
#[must_use]
pub fn after(patches: &[FilePatch], parts: &[Part]) -> After {
    let mut now = after_one(patches, None);
    for part in parts {
        let next = after_one(&now.patches, Some(*part));
        let origin = next
            .origin
            .iter()
            .map(|hunks| {
                hunks
                    .iter()
                    .map(|lines| {
                        lines
                            .iter()
                            .map(|o| match now.origin(o.at) {
                                Some(before) => Origin {
                                    at: before.at,
                                    turned: before.turned || o.turned,
                                },
                                None => *o,
                            })
                            .collect()
                    })
                    .collect()
            })
            .collect();
        now = After {
            patches: next.patches,
            origin,
            exact: now.exact && next.exact,
        };
    }
    now
}

/// `patches` with one part in them, or none.
fn after_one(patches: &[FilePatch], part: Option<Part>) -> After {
    let parts: Vec<Part> = part.into_iter().collect();
    let parts = parts.as_slice();
    let with_hunks = patches.iter().filter(|p| !p.hunks.is_empty()).count();
    let mut exact = with_hunks <= 1 || parts.is_empty();
    let mut out = Vec::with_capacity(patches.len());
    let mut origin = Vec::with_capacity(patches.len());
    for (at, patch) in patches.iter().enumerate() {
        let pressed = with_hunks == 1 && !patch.is_combined;
        if !pressed && !patch.hunks.is_empty() && !parts.is_empty() {
            exact = false;
        }
        let mut shift = Shift::default();
        let mut hunks = Vec::with_capacity(patch.hunks.len());
        let mut hunk_origin = Vec::with_capacity(patch.hunks.len());
        for (index, base) in patch.hunks.iter().enumerate() {
            let taken: Vec<Part> = if pressed {
                parts.iter().copied().filter(|p| p.hunk == index).collect()
            } else {
                Vec::new()
            };
            let place = |line: usize| DiffLineLocation {
                patch: at,
                hunk: index,
                line,
            };
            match laid_out(base, &taken, shift, &place) {
                Laid::Gone(grown) => shift.add(grown),
                Laid::Hunk(hunk, lines, grown) => {
                    shift.add(grown);
                    hunks.push(hunk);
                    hunk_origin.push(lines);
                }
                Laid::AsRead => {
                    exact = false;
                    let (hunk, lines) = shifted(base, shift, &place);
                    hunks.push(hunk);
                    hunk_origin.push(lines);
                }
            }
        }
        out.push(FilePatch {
            hunks,
            ..without_hunks(patch)
        });
        origin.push(hunk_origin);
    }
    After {
        patches: out,
        origin,
        exact,
    }
}

/// Whether `ours` and `git` hold the same hunks, line for line, numbers
/// and all — the headings after the `@@` aside (see the module note). A
/// patch left with no hunk is no part of it: git prints no patch for a
/// file the parts took every change of.
#[must_use]
pub fn same_lines(ours: &[FilePatch], git: &[FilePatch]) -> bool {
    let ours: Vec<&FilePatch> = ours.iter().filter(|p| !p.hunks.is_empty()).collect();
    let git: Vec<&FilePatch> = git.iter().filter(|p| !p.hunks.is_empty()).collect();
    ours.len() == git.len()
        && ours.iter().zip(&git).all(|(a, b)| {
            a.hunks.len() == b.hunks.len()
                && a.hunks.iter().zip(&b.hunks).all(|(x, y)| {
                    (x.old_start, x.old_count, x.new_start, x.new_count)
                        == (y.old_start, y.old_count, y.new_start, y.new_count)
                        && x.lines == y.lines
                })
        })
}

fn without_hunks(patch: &FilePatch) -> FilePatch {
    FilePatch {
        old_path: patch.old_path.clone(),
        new_path: patch.new_path.clone(),
        is_binary: patch.is_binary,
        is_combined: patch.is_combined,
        unmerged: patch.unmerged,
        gitlink: patch.gitlink,
        lfs: patch.lfs,
        mode_change: patch.mode_change.clone(),
        from_commit: patch.from_commit.clone(),
        hunks: Vec::new(),
    }
}

/// How far the numbers on each side have moved by the hunks above: the
/// lines a part gave the side it moves, or took from it.
#[derive(Debug, Default, Clone, Copy)]
struct Shift {
    old: i64,
    new: i64,
}

impl Shift {
    fn add(&mut self, grown: Shift) {
        self.old += grown.old;
        self.new += grown.new;
    }
}

enum Laid {
    /// No change is left: the hunk leaves, the side it moved having grown.
    Gone(Shift),
    Hunk(DiffHunk, Vec<Origin>, Shift),
    /// The parts in it are not laid out (see the module note).
    AsRead,
}

/// One hunk with the parts `taken` in it, under the hunks above that moved
/// its numbers by `above`.
fn laid_out(
    base: &DiffHunk,
    taken: &[Part],
    above: Shift,
    place: &dyn Fn(usize) -> DiffLineLocation,
) -> Laid {
    if taken.is_empty() {
        let (hunk, lines) = shifted(base, above, place);
        return Laid::Hunk(hunk, lines, Shift::default());
    }
    let mut grown = Shift::default();
    let mut kept: Vec<(DiffLine, Origin)> = Vec::with_capacity(base.lines.len());
    for (index, line) in base.lines.iter().enumerate() {
        let at = Origin {
            at: place(index),
            turned: false,
        };
        let part = taken
            .iter()
            .find(|p| p.line.is_none() || p.line == Some(index));
        let changed = matches!(line.kind, DiffLineKind::Addition | DiffLineKind::Deletion);
        match part {
            Some(part) if changed => {
                // The side it moves gains the line (an addition staged, a
                // removal unstaged or discarded: now on both sides) or gives
                // it up (now on neither).
                let gains = matches!(
                    (part.moves, line.kind),
                    (Moves::Old, DiffLineKind::Addition) | (Moves::New, DiffLineKind::Deletion)
                );
                let by = if gains { 1 } else { -1 };
                match part.moves {
                    Moves::Old => grown.old += by,
                    Moves::New => grown.new += by,
                }
                if gains {
                    let turned = DiffLine {
                        kind: DiffLineKind::Context,
                        ..line.clone()
                    };
                    kept.push((turned, Origin { turned: true, ..at }));
                }
            }
            _ => kept.push((line.clone(), at)),
        }
    }
    let is_change =
        |l: &DiffLine| matches!(l.kind, DiffLineKind::Addition | DiffLineKind::Deletion);
    if !kept.iter().any(|(l, _)| is_change(l)) {
        return Laid::Gone(grown);
    }
    if kept.iter().any(|(l, _)| l.kind == DiffLineKind::NoNewline) {
        return Laid::AsRead;
    }
    let lead = kept.iter().take_while(|(l, _)| !is_change(l)).count();
    let tail = kept.iter().rev().take_while(|(l, _)| !is_change(l)).count();
    let cut_lead = lead.saturating_sub(CONTEXT);
    let cut_tail = tail.saturating_sub(CONTEXT);
    let body = &kept[cut_lead..kept.len() - cut_tail];
    let mut run = 0;
    for (line, _) in body {
        run = if is_change(line) { 0 } else { run + 1 };
        if run > 2 * CONTEXT {
            return Laid::AsRead;
        }
    }
    let cut = i64::try_from(cut_lead).unwrap_or(0);
    let first_old = first(base.old_start, base.old_count) + above.old + cut;
    let first_new = first(base.new_start, base.new_count) + above.new + cut;
    let (mut old, mut new) = (first_old, first_new);
    let mut lines = Vec::with_capacity(body.len());
    let mut origin = Vec::with_capacity(body.len());
    for (line, from) in body {
        let (old_no, new_no) = match line.kind {
            DiffLineKind::Deletion => (number(old), None),
            DiffLineKind::Addition => (None, number(new)),
            _ => (number(old), number(new)),
        };
        old += i64::from(line.kind != DiffLineKind::Addition);
        new += i64::from(line.kind != DiffLineKind::Deletion);
        lines.push(DiffLine {
            old_no,
            new_no,
            ..line.clone()
        });
        origin.push(*from);
    }
    let count = |end: i64, first: i64| u32::try_from(end - first).unwrap_or(0);
    let (old_count, new_count) = (count(old, first_old), count(new, first_new));
    let hunk = DiffHunk {
        old_start: start(first_old, old_count),
        old_count,
        new_start: start(first_new, new_count),
        new_count,
        extra_old: base.extra_old.clone(),
        heading: base.heading.clone(),
        lines,
    };
    Laid::Hunk(hunk, origin, grown)
}

/// A hunk no part reached, under the hunks above that one did.
fn shifted(
    base: &DiffHunk,
    above: Shift,
    place: &dyn Fn(usize) -> DiffLineLocation,
) -> (DiffHunk, Vec<Origin>) {
    let moved = |no: Option<u32>, by: i64| no.and_then(|n| number(i64::from(n) + by));
    let lines = base
        .lines
        .iter()
        .map(|line| DiffLine {
            old_no: moved(line.old_no, above.old),
            new_no: moved(line.new_no, above.new),
            ..line.clone()
        })
        .collect();
    let origin = (0..base.lines.len())
        .map(|line| Origin {
            at: place(line),
            turned: false,
        })
        .collect();
    let at = |start: u32, by: i64| u32::try_from(i64::from(start) + by).unwrap_or(0);
    let hunk = DiffHunk {
        old_start: at(base.old_start, above.old),
        new_start: at(base.new_start, above.new),
        lines,
        ..base.clone()
    };
    (hunk, origin)
}

/// The first line a side of a hunk numbers: git names the line before an
/// empty side.
fn first(start: u32, count: u32) -> i64 {
    i64::from(start) + i64::from(count == 0)
}

fn start(first: i64, count: u32) -> u32 {
    u32::try_from(first - i64::from(count == 0)).unwrap_or(0)
}

fn number(no: i64) -> Option<u32> {
    u32::try_from(no).ok()
}
