//! Which parts of a changed line actually changed — the emphasis the
//! diff pane lays under them.
//!
//! The k-th line of a deletion run is read against the k-th of the
//! addition run after it. The tail of the longer run changed as a whole;
//! a run with no counterpart at all (pure insertion or removal) gets no
//! emphasis, as there is nothing quiet around it.
//!
//! Worked out here rather than by `git diff --word-diff`, whose output
//! replaces the line-based form the pane is built on.

use std::ops::Range;

use crate::parse::diff::{DiffHunk, DiffLineKind, DiffLineLocation, FilePatch};

/// Byte ranges into one row's text, ascending, disjoint and none empty.
type Ranges = Vec<Range<usize>>;

/// What changed inside every row of a diff, addressed by patch, hunk and
/// line. Empty where there is nothing to emphasise.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IntraMarks {
    patches: Vec<Vec<Vec<Ranges>>>,
}

impl IntraMarks {
    /// The emphasised ranges of one row — none, where none are.
    pub fn line(&self, location: DiffLineLocation) -> &[Range<usize>] {
        static NOTHING: Ranges = Vec::new();
        location.entry_in(&self.patches).unwrap_or(&NOTHING)
    }
}

/// Below this much sharing (per mille of the longer line's characters) two
/// lines are not versions of each other: whole-line emphasis instead of
/// lighting their scattered coincidences.
const KINSHIP_FLOOR_PER_MILLE: usize = 500;

/// A shared run shorter than this, between two emphasised stretches, is
/// swallowed by them: `1` surviving inside `let x = 1;` → `let y = 12;`
/// reads as noise.
const KEEP_RUN_CHARS: usize = 3;

/// The refinement pass ([`refined`]) is quadratic in the unshared
/// middle, so it only runs where the middle is small…
const REFINE_CELL_CAP: usize = 4_096;

/// …and only until a whole diff has spent this many cells on it; past it,
/// remaining pairs keep their coarse one-range answer.
const REFINE_CELL_BUDGET: usize = 2_000_000;

/// What to emphasise on each row of `patches`. Linear in the text plus a
/// bounded refinement ([`REFINE_CELL_BUDGET`]), so it can ride with the
/// rows.
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
        // `\ No newline at end of file` can stand between the two runs.
        let mut j = i;
        while j < lines.len() && lines[j].kind == DiffLineKind::NoNewline {
            j += 1;
        }
        let adds = j;
        while j < lines.len() && lines[j].kind == DiffLineKind::Addition {
            j += 1;
        }
        if j == adds {
            // Pure removal.
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
    span_of(0..text.len())
}

/// What one pair of lines does not share: the common prefix and suffix
/// are quiet, the middles are emphasised — refined into several ranges
/// where the budget allows, one range each otherwise.
fn line_marks(old: &str, new: &str, budget: &mut usize) -> (Ranges, Ranges) {
    let prefix = common_prefix(old, new);
    // The suffix may not reach into the prefix.
    let suffix = common_suffix(&old[prefix..], &new[prefix..]);
    let old_mid = prefix..old.len() - suffix;
    let new_mid = prefix..new.len() - suffix;
    if old_mid.is_empty() && new_mid.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let chars = |s: &str| s.chars().count();
    let shared = chars(&old[..prefix]) + chars(&old[old_mid.end..]);
    let longer = chars(old).max(chars(new));
    if longer > 0 && shared * 1_000 < KINSHIP_FLOOR_PER_MILLE * longer {
        return (whole(old), whole(new));
    }
    let om = &old[old_mid.clone()];
    let nm = &new[new_mid.clone()];
    let cells = chars(om).saturating_mul(chars(nm));
    if cells > 0 && cells <= REFINE_CELL_CAP && cells <= *budget {
        *budget -= cells;
        return refined(om, nm, old_mid.start, new_mid.start);
    }
    (span_of(old_mid), span_of(new_mid))
}

fn span_of(range: Range<usize>) -> Ranges {
    if range.is_empty() {
        Vec::new()
    } else {
        vec![range]
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
        if run > 0 && run < KEEP_RUN_CHARS {
            push_range(old_ranges, byte_at(a, oi, om)..byte_at(a, i, om));
            push_range(new_ranges, byte_at(b, nj, nm)..byte_at(b, j, nm));
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
            push_range(&mut old_ranges, byte_at(&a, i, om)..byte_at(&a, i + 1, om));
            i += 1;
        } else {
            push_range(&mut new_ranges, byte_at(&b, j, nm)..byte_at(&b, j + 1, nm));
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
    // Both ends lie inside the middle, which starts `off` into a `&str`:
    // the sums stay within the line's length.
    let lift = |ranges: Ranges, off: usize| -> Ranges {
        ranges
            .into_iter()
            .map(|range| range.start + off..range.end + off)
            .collect()
    };
    (lift(old_ranges, old_off), lift(new_ranges, new_off))
}

/// The byte offset of the `i`-th character, or the text's end.
fn byte_at(chars: &[(usize, char)], i: usize, text: &str) -> usize {
    chars.get(i).map_or(text.len(), |(at, _)| *at)
}

/// Appends `range` to `ranges`, whose last range it does not start
/// before, merging into that one where the two touch or overlap. An empty
/// one adds nothing.
fn push_range(ranges: &mut Ranges, range: Range<usize>) {
    if range.is_empty() {
        return;
    }
    if let Some(last) = ranges.last_mut()
        && last.end >= range.start
    {
        last.end = last.end.max(range.end);
        return;
    }
    ranges.push(range);
}

#[cfg(test)]
#[expect(
    clippy::single_range_in_vec_init,
    reason = "a row's emphasis is a slice of ranges, and a row of one range is a slice of one"
)]
mod tests {
    use super::*;
    use crate::parse::diff::parse_patch;

    fn one_hunk(body: &str) -> Vec<FilePatch> {
        parse_patch(format!("--- a/f.rs\n+++ b/f.rs\n{body}").as_bytes())
    }

    fn at(patch: usize, hunk: usize, line: usize) -> DiffLineLocation {
        DiffLineLocation { patch, hunk, line }
    }

    #[test]
    fn a_tail_grown_on_a_line_is_the_only_emphasis() {
        let patches = one_hunk("@@ -1,1 +1,1 @@\n-let x = 1;\n+let x = 1; // c\n");
        let marks = marks(&patches);
        assert!(
            marks.line(at(0, 0, 0)).is_empty(),
            "the deletion lost nothing"
        );
        assert_eq!(marks.line(at(0, 0, 1)), &[10..15], "only the tail is new");
    }

    #[test]
    fn a_renamed_identifier_is_cut_out_alone() {
        let patches = one_hunk("@@ -1,1 +1,1 @@\n-let x = 1;\n+let y = 1;\n");
        let marks = marks(&patches);
        assert_eq!(marks.line(at(0, 0, 0)), &[4..5]);
        assert_eq!(marks.line(at(0, 0, 1)), &[4..5]);
    }

    /// Two files, the second with two hunks, each pair changed somewhere
    /// else: a renamed `x` at 4, a digit at 8, a tail grown at 10.
    const TWO_FILES: &str = "\
diff --git a/a.rs b/a.rs
--- a/a.rs
+++ b/a.rs
@@ -1,1 +1,1 @@
-let x = 1;
+let y = 1;
diff --git a/b.rs b/b.rs
--- a/b.rs
+++ b/b.rs
@@ -1,1 +1,1 @@
-let a = 1;
+let a = 2;
@@ -9,1 +9,1 @@
-let z = 1;
+let z = 1; // c
";

    #[test]
    fn each_part_of_a_location_picks_its_own_line() {
        let marks = marks(&parse_patch(TWO_FILES.as_bytes()));
        assert_eq!(marks.line(at(0, 0, 1)), &[4..5], "the first file");
        assert_eq!(marks.line(at(1, 0, 1)), &[8..9], "another patch");
        assert_eq!(marks.line(at(1, 1, 1)), &[10..15], "another hunk");
        assert!(marks.line(at(1, 1, 0)).is_empty(), "another line");
    }

    #[test]
    fn a_location_past_any_part_answers_nothing() {
        let marks = marks(&parse_patch(TWO_FILES.as_bytes()));
        assert!(marks.line(at(2, 0, 1)).is_empty(), "no third patch");
        assert!(marks.line(at(1, 2, 1)).is_empty(), "no third hunk");
        assert!(marks.line(at(1, 1, 2)).is_empty(), "no third line");
    }

    #[test]
    fn several_changes_come_out_as_several_ranges() {
        let patches = one_hunk(
            "@@ -1,1 +1,1 @@\n-fn take(strip: f64, bottom: f64)\n+fn take(strips: f64, bottoms: f64)\n",
        );
        let added = marks(&patches).line(at(0, 0, 1)).to_vec();
        assert!(
            added.len() >= 2,
            "an `s` grew in two places, not one: {added:?}"
        );
    }

    #[test]
    fn lines_that_mostly_disagree_are_emphasised_whole() {
        let patches = one_hunk("@@ -1,1 +1,1 @@\n-abc\n+wxyz nothing alike\n");
        let marks = marks(&patches);
        assert_eq!(marks.line(at(0, 0, 0)), &[0..3]);
        assert_eq!(marks.line(at(0, 0, 1)), &[0.."wxyz nothing alike".len()]);
    }

    #[test]
    fn the_tail_of_the_longer_run_changed_as_a_whole() {
        let patches = one_hunk("@@ -1,1 +1,2 @@\n-let x = 1;\n+let x = 2;\n+let y = 3;\n");
        let marks = marks(&patches);
        assert_eq!(marks.line(at(0, 0, 1)), &[8..9], "the pair reads finely");
        assert_eq!(
            marks.line(at(0, 0, 2)),
            &[0.."let y = 3;".len()],
            "the unpaired line changed as a whole"
        );
    }

    #[test]
    fn runs_with_no_counterpart_get_no_emphasis() {
        // Pure insertion.
        let patches = one_hunk("@@ -1,1 +1,3 @@\n context\n+alpha\n+beta\n");
        let added = marks(&patches);
        for line in 0..3 {
            assert!(added.line(at(0, 0, line)).is_empty(), "line {line}");
        }
        // Pure removal, likewise.
        let gone = one_hunk("@@ -1,2 +1,1 @@\n context\n-doomed\n");
        assert!(marks(&gone).line(at(0, 0, 1)).is_empty());
    }

    #[test]
    fn short_shared_runs_are_swallowed_by_the_emphasis() {
        // Between the changes only `b` and `d` survive, each shorter
        // than KEEP_RUN_CHARS.
        let mut budget = usize::MAX;
        let (old_marks, new_marks) =
            line_marks("Qlong ab1de tailW", "Qlong xb2dy tailW", &mut budget);
        assert_eq!(old_marks, vec![6..11]);
        assert_eq!(new_marks, vec![6..11]);
    }

    /// Touching, overlapping and held ranges run into the last one; one
    /// that stands apart, or an empty one between them, does not join them.
    #[test]
    fn a_range_runs_into_the_last_only_where_the_two_meet() {
        let pushed = |ranges: &[Range<usize>]| {
            let mut out = Ranges::new();
            for range in ranges {
                push_range(&mut out, range.clone());
            }
            out
        };
        assert_eq!(pushed(&[0..2, 2..4]), [0..4], "touching");
        assert_eq!(pushed(&[0..3, 2..5]), [0..5], "overlapping");
        assert_eq!(pushed(&[0..5, 1..3]), [0..5], "held inside");
        assert_eq!(pushed(&[0..2, 3..5]), [0..2, 3..5], "apart");
        assert_eq!(pushed(&[0..2, 2..2, 3..5]), [0..2, 3..5], "nothing between");
        assert!(pushed(&[4..4]).is_empty(), "nothing at all");
    }

    #[test]
    fn a_no_newline_note_does_not_part_the_runs() {
        let patches =
            one_hunk("@@ -1,1 +1,1 @@\n-old tail\n\\ No newline at end of file\n+old tails\n");
        let marks = marks(&patches);
        assert!(marks.line(at(0, 0, 0)).is_empty(), "still read as a pair");
        assert_eq!(marks.line(at(0, 0, 2)), &[8..9]);
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
        assert!(marks.line(at(0, 0, 0)).is_empty());
        assert!(marks.line(at(0, 0, 1)).is_empty());
    }
}
