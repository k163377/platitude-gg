//! Which parts of a changed line actually changed — the emphasis the
//! diff pane lays under them, worked out from the rows alone.
//!
//! Deletions and the additions that replace them arrive as adjacent
//! runs. The k-th line of one run is read against the k-th of the
//! other: the parts the two do not share are what the eye should land
//! on, the parts they do share are the quiet ground around them. A line
//! with no counterpart — the tail of the longer run — changed as a
//! whole; a run with no counterpart at all (pure insertion, pure
//! removal) gets no emphasis, because a wash with nothing quiet in it
//! says nothing.
//!
//! This is presentation: `git diff --word-diff` answers a similar
//! question, but its output replaces the line-based form the pane is
//! built on, so the ranges are worked out here, over text the diff
//! already carries.

use crate::parse::diff::{DiffHunk, DiffLineKind, FilePatch};

/// Byte ranges into one row's text — `(start, len)`, ascending and
/// disjoint.
type Ranges = Vec<(usize, usize)>;

/// What changed inside every row of a diff, addressed the way its rows
/// already are: which patch, which hunk, which line of it. Empty where
/// there is nothing to emphasise.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IntraMarks {
    patches: Vec<Vec<Vec<Ranges>>>,
}

impl IntraMarks {
    /// The emphasised ranges of one row — none, where none are.
    pub fn line(&self, patch: usize, hunk: usize, line: usize) -> &[(usize, usize)] {
        static NOTHING: Ranges = Vec::new();
        self.patches
            .get(patch)
            .and_then(|p| p.get(hunk))
            .and_then(|h| h.get(line))
            .unwrap_or(&NOTHING)
    }
}

/// Below this much sharing, two lines are not versions of each other —
/// emphasising their scattered coincidences would light most of both
/// rows and say less than the wash already does. Whole-line emphasis
/// instead. Per mille of the longer line's characters.
const KINSHIP_FLOOR_PER_MILLE: usize = 500;

/// A shared run shorter than this, between two emphasised stretches, is
/// swallowed by them: `1` surviving inside `let x = 1;` → `let y = 12;`
/// reads as noise.
const KEEP_RUN_CHARS: usize = 3;

/// The refinement pass ([`refined`]) is quadratic in the unshared
/// middle, so it only runs where the middle is small…
const REFINE_CELL_CAP: usize = 4_096;

/// …and only until a whole diff has spent this many cells on it. Past
/// the budget every remaining pair keeps its coarse one-range answer —
/// the same shape, less finely cut.
const REFINE_CELL_BUDGET: usize = 2_000_000;

/// Reads every hunk of `patches` and answers what to emphasise on each
/// row. Linear in the text, plus a bounded refinement
/// ([`REFINE_CELL_BUDGET`]); cheap enough to ride with the rows
/// themselves.
pub fn marks(patches: &[FilePatch]) -> IntraMarks {
    let mut budget = REFINE_CELL_BUDGET;
    IntraMarks {
        patches: patches
            .iter()
            .map(|patch| {
                if patch.is_binary || patch.unmerged || patch.is_combined {
                    return patch
                        .hunks
                        .iter()
                        .map(|h| vec![Vec::new(); h.lines.len()])
                        .collect();
                }
                patch
                    .hunks
                    .iter()
                    .map(|hunk| hunk_marks(hunk, &mut budget))
                    .collect()
            })
            .collect(),
    }
}

/// One hunk: pair each run of deletions with the run of additions that
/// follows it, and read the pairs against each other.
fn hunk_marks(hunk: &DiffHunk, budget: &mut usize) -> Vec<Ranges> {
    let lines = &hunk.lines;
    let mut out = vec![Vec::new(); lines.len()];
    let mut i = 0usize;
    while i < lines.len() {
        if lines[i].kind != DiffLineKind::Deletion {
            i += 1;
            continue;
        }
        let dels = i;
        while i < lines.len() && lines[i].kind == DiffLineKind::Deletion {
            i += 1;
        }
        let dels_end = i;
        // `\ No newline at end of file` can stand between the two runs;
        // it is git talking.
        let mut j = i;
        while j < lines.len() && lines[j].kind == DiffLineKind::NoNewline {
            j += 1;
        }
        let adds = j;
        while j < lines.len() && lines[j].kind == DiffLineKind::Addition {
            j += 1;
        }
        if j == adds {
            // Pure removal: nothing replaced it, nothing to weigh it
            // against.
            continue;
        }
        i = j;
        let paired = (dels_end - dels).min(j - adds);
        for k in 0..paired {
            let (del_ranges, add_ranges) =
                line_marks(&lines[dels + k].text, &lines[adds + k].text, budget);
            out[dels + k] = del_ranges;
            out[adds + k] = add_ranges;
        }
        // The tail of the longer run changed as a whole.
        for line in dels + paired..dels_end {
            out[line] = whole(&lines[line].text);
        }
        for line in adds + paired..j {
            out[line] = whole(&lines[line].text);
        }
    }
    out
}

fn whole(text: &str) -> Ranges {
    if text.is_empty() {
        Vec::new()
    } else {
        vec![(0, text.len())]
    }
}

/// What one pair of lines does not share: the common prefix and suffix
/// are quiet, the middles are emphasised — refined into several ranges
/// where the budget allows, one range each otherwise.
fn line_marks(old: &str, new: &str, budget: &mut usize) -> (Ranges, Ranges) {
    let prefix = common_prefix(old, new);
    // The suffix may not reach into the prefix.
    let suffix = common_suffix(&old[prefix..], &new[prefix..]);
    let old_mid = (prefix, old.len() - suffix);
    let new_mid = (prefix, new.len() - suffix);
    if old_mid.0 == old_mid.1 && new_mid.0 == new_mid.1 {
        return (Vec::new(), Vec::new());
    }
    // Two lines that mostly disagree are not versions of each other.
    let chars = |s: &str| s.chars().count();
    let shared = chars(&old[..prefix]) + chars(&old[old_mid.1..]);
    let longer = chars(old).max(chars(new));
    if longer > 0 && shared * 1_000 < KINSHIP_FLOOR_PER_MILLE * longer {
        return (whole(old), whole(new));
    }
    let om = &old[old_mid.0..old_mid.1];
    let nm = &new[new_mid.0..new_mid.1];
    let cells = chars(om).saturating_mul(chars(nm));
    if cells > 0 && cells <= REFINE_CELL_CAP && cells <= *budget {
        *budget -= cells;
        return refined(om, nm, old_mid.0, new_mid.0);
    }
    (span_of(old_mid), span_of(new_mid))
}

fn span_of((start, end): (usize, usize)) -> Ranges {
    if start == end {
        Vec::new()
    } else {
        vec![(start, end - start)]
    }
}

fn common_prefix(a: &str, b: &str) -> usize {
    let mut at = 0usize;
    for (ca, cb) in a.chars().zip(b.chars()) {
        if ca != cb {
            break;
        }
        at += ca.len_utf8();
    }
    at
}

fn common_suffix(a: &str, b: &str) -> usize {
    let mut at = 0usize;
    for (ca, cb) in a.chars().rev().zip(b.chars().rev()) {
        if ca != cb {
            break;
        }
        at += ca.len_utf8();
    }
    at
}

/// The unshared middles cut finely: a character-level longest common
/// subsequence, with shared runs shorter than [`KEEP_RUN_CHARS`]
/// swallowed by the emphasis around them.
fn refined(om: &str, nm: &str, old_off: usize, new_off: usize) -> (Ranges, Ranges) {
    let a: Vec<(usize, char)> = om.char_indices().collect();
    let b: Vec<(usize, char)> = nm.char_indices().collect();
    // lcs[i][j]: length of the longest common subsequence of a[i..] and
    // b[j..], one row flattened.
    let (n, m) = (a.len(), b.len());
    let mut lcs = vec![0u16; (n + 1) * (m + 1)];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[i * (m + 1) + j] = if a[i].1 == b[j].1 {
                lcs[(i + 1) * (m + 1) + j + 1] + 1
            } else {
                lcs[(i + 1) * (m + 1) + j].max(lcs[i * (m + 1) + j + 1])
            };
        }
    }
    // Walk the table, keeping runs: shared stretches long enough to
    // stand, unshared stretches as emphasis on their own side.
    let mut old_ranges = Vec::new();
    let mut new_ranges = Vec::new();
    let (mut i, mut j) = (0usize, 0usize);
    let mut shared_run = 0usize;
    let (mut oi, mut nj) = (0usize, 0usize); // starts of the pending shared run
    let flush_shared = |old_ranges: &mut Ranges,
                        new_ranges: &mut Ranges,
                        a: &[(usize, char)],
                        b: &[(usize, char)],
                        run: usize,
                        oi: usize,
                        nj: usize,
                        i: usize,
                        j: usize| {
        // A shared run too short to stand becomes emphasis on both
        // sides instead.
        if run > 0 && run < KEEP_RUN_CHARS {
            push_range(old_ranges, byte_at(a, oi, om), byte_at(a, i, om));
            push_range(new_ranges, byte_at(b, nj, nm), byte_at(b, j, nm));
        }
    };
    while i < n || j < m {
        if i < n && j < m && a[i].1 == b[j].1 {
            if shared_run == 0 {
                oi = i;
                nj = j;
            }
            shared_run += 1;
            i += 1;
            j += 1;
            continue;
        }
        flush_shared(
            &mut old_ranges,
            &mut new_ranges,
            &a,
            &b,
            shared_run,
            oi,
            nj,
            i,
            j,
        );
        shared_run = 0;
        // Step the side whose skip keeps the subsequence longest.
        if j == m || (i < n && lcs[(i + 1) * (m + 1) + j] >= lcs[i * (m + 1) + j + 1]) {
            push_range(&mut old_ranges, byte_at(&a, i, om), byte_at(&a, i + 1, om));
            i += 1;
        } else {
            push_range(&mut new_ranges, byte_at(&b, j, nm), byte_at(&b, j + 1, nm));
            j += 1;
        }
    }
    flush_shared(
        &mut old_ranges,
        &mut new_ranges,
        &a,
        &b,
        shared_run,
        oi,
        nj,
        i,
        j,
    );
    let lift = |ranges: Ranges, off: usize| -> Ranges {
        ranges.into_iter().map(|(s, l)| (s + off, l)).collect()
    };
    (lift(old_ranges, old_off), lift(new_ranges, new_off))
}

/// The byte offset of the `i`-th character, or the text's end.
fn byte_at(chars: &[(usize, char)], i: usize, text: &str) -> usize {
    chars.get(i).map_or(text.len(), |(at, _)| *at)
}

/// Appends `[start, end)` to `ranges`, merging into the last range when
/// they touch.
fn push_range(ranges: &mut Ranges, start: usize, end: usize) {
    if end <= start {
        return;
    }
    if let Some(last) = ranges.last_mut()
        && last.0 + last.1 >= start
    {
        last.1 = end - last.0;
        return;
    }
    ranges.push((start, end - start));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::diff::parse_patch;

    fn one_hunk(body: &str) -> Vec<FilePatch> {
        parse_patch(format!("--- a/f.rs\n+++ b/f.rs\n{body}").as_bytes())
    }

    #[test]
    fn a_tail_grown_on_a_line_is_the_only_emphasis() {
        let patches = one_hunk("@@ -1,1 +1,1 @@\n-let x = 1;\n+let x = 1; // c\n");
        let marks = marks(&patches);
        assert_eq!(marks.line(0, 0, 0), &[], "the deletion lost nothing");
        assert_eq!(marks.line(0, 0, 1), &[(10, 5)], "only the tail is new");
    }

    #[test]
    fn a_renamed_identifier_is_cut_out_alone() {
        let patches = one_hunk("@@ -1,1 +1,1 @@\n-let x = 1;\n+let y = 1;\n");
        let marks = marks(&patches);
        assert_eq!(marks.line(0, 0, 0), &[(4, 1)]);
        assert_eq!(marks.line(0, 0, 1), &[(4, 1)]);
    }

    #[test]
    fn several_changes_come_out_as_several_ranges() {
        let patches = one_hunk(
            "@@ -1,1 +1,1 @@\n-fn take(strip: f64, bottom: f64)\n+fn take(strips: f64, bottoms: f64)\n",
        );
        let added = marks(&patches).line(0, 0, 1).to_vec();
        assert!(
            added.len() >= 2,
            "an `s` grew in two places, not one: {added:?}"
        );
    }

    #[test]
    fn lines_that_mostly_disagree_are_emphasised_whole() {
        let patches = one_hunk("@@ -1,1 +1,1 @@\n-abc\n+wxyz nothing alike\n");
        let marks = marks(&patches);
        assert_eq!(marks.line(0, 0, 0), &[(0, 3)]);
        assert_eq!(marks.line(0, 0, 1), &[(0, "wxyz nothing alike".len())]);
    }

    #[test]
    fn the_tail_of_the_longer_run_changed_as_a_whole() {
        let patches = one_hunk("@@ -1,1 +1,2 @@\n-let x = 1;\n+let x = 2;\n+let y = 3;\n");
        let marks = marks(&patches);
        assert_eq!(marks.line(0, 0, 1), &[(8, 1)], "the pair reads finely");
        assert_eq!(
            marks.line(0, 0, 2),
            &[(0, "let y = 3;".len())],
            "the unpaired line changed as a whole"
        );
    }

    #[test]
    fn runs_with_no_counterpart_get_no_emphasis() {
        // Pure insertion: a wash with nothing quiet in it says nothing.
        let patches = one_hunk("@@ -1,1 +1,3 @@\n context\n+alpha\n+beta\n");
        let added = marks(&patches);
        for line in 0..3 {
            assert_eq!(added.line(0, 0, line), &[], "line {line}");
        }
        // Pure removal, likewise.
        let gone = one_hunk("@@ -1,2 +1,1 @@\n context\n-doomed\n");
        assert_eq!(marks(&gone).line(0, 0, 1), &[]);
    }

    #[test]
    fn short_shared_runs_are_swallowed_by_the_emphasis() {
        // Between the changes only `b` and `d` survive, each shorter
        // than KEEP_RUN_CHARS — kept, they would read as noise, so the
        // emphasis swallows them into one stretch.
        let mut budget = usize::MAX;
        let (old_marks, new_marks) =
            line_marks("Qlong ab1de tailW", "Qlong xb2dy tailW", &mut budget);
        assert_eq!(old_marks, vec![(6, 5)]);
        assert_eq!(new_marks, vec![(6, 5)]);
    }

    #[test]
    fn a_no_newline_note_does_not_part_the_runs() {
        let patches =
            one_hunk("@@ -1,1 +1,1 @@\n-old tail\n\\ No newline at end of file\n+old tails\n");
        let marks = marks(&patches);
        assert_eq!(marks.line(0, 0, 0), &[], "still read as a pair");
        assert_eq!(marks.line(0, 0, 2), &[(8, 1)]);
    }

    #[test]
    fn combined_diffs_are_left_alone() {
        let patch = "\
diff --cc s.txt
--- a/s.txt
+++ b/s.txt
@@@ -1,1 -1,1 +1,2 @@@
 +ours
+ theirs
";
        let patches = parse_patch(patch.as_bytes());
        assert!(patches[0].is_combined);
        let marks = marks(&patches);
        assert_eq!(marks.line(0, 0, 0), &[]);
        assert_eq!(marks.line(0, 0, 1), &[]);
    }
}
