//! One content line of a combined hunk: one marker column per
//! parent, then the text (column semantics: the module note).

use super::{DiffHunk, DiffLine, DiffLineKind};

/// One content line of a combined hunk: `parent_no.len()` marker columns
/// followed by the text (column semantics: the module note).
pub(super) fn read_combined_line(
    h: &mut DiffHunk,
    line: &str,
    parent_no: &mut [u32],
    new_no: &mut u32,
) {
    // git never prints the no-newline note in this form (measured), but
    // reading one costs nothing and losing the line would cost a row.
    if line.starts_with('\\') {
        h.lines.push(DiffLine {
            kind: DiffLineKind::NoNewline,
            old_no: None,
            new_no: None,
            text: line.to_string(),
            markers: String::new(),
        });
        return;
    }
    let n = parent_no.len();
    // Short lines are tolerated the way the unified reader tolerates an
    // empty context line: the missing columns are the spaces some tool
    // trimmed off the end.
    let bytes = line.as_bytes();
    let markers: String = (0..n)
        .map(|i| bytes.get(i).map_or(' ', |b| *b as char))
        .collect();
    if let Some(bad) = markers.chars().find(|c| !matches!(c, ' ' | '+' | '-')) {
        tracing::trace!(line, marker = %bad, "unexpected line inside combined hunk");
        return;
    }
    let text = line
        .get(n.min(line.len())..)
        .unwrap_or_default()
        .to_string();

    let removed = markers.contains('-');
    // On a removed line a parent that has it is the one marked `-`; on a
    // line that survived it is the one that is *not* marked `+`.
    let has = |c: char| if removed { c == '-' } else { c == ' ' };
    let first = markers.chars().next().is_some_and(has);

    let kind = if removed {
        DiffLineKind::Deletion
    } else if markers.contains('+') {
        DiffLineKind::Addition
    } else {
        DiffLineKind::Context
    };
    let old_no = first.then(|| parent_no[0]);
    let at = (!removed).then_some(*new_no);
    for (i, c) in markers.chars().enumerate() {
        if has(c) {
            parent_no[i] += 1;
        }
    }
    if !removed {
        *new_no += 1;
    }
    h.lines.push(DiffLine {
        kind,
        old_no,
        new_no: at,
        text,
        markers,
    });
}

/// Which side of a conflicted (two-parent) diff a row's marker columns
/// name: `"ours"` / `"theirs"`, or `""` for a line both sides have, a
/// line neither has (a fence git wrote, or one typed while resolving),
/// and markers too short to say — a single-parent diff has none at all.
///
/// The same reading [`read_combined_line`] gives the columns: on a
/// removed line the side that has it is the one marked `-`; on a line
/// that survived it is the one that is *not* marked `+`.
pub fn side_of_markers(markers: &str) -> &'static str {
    let bytes = markers.as_bytes();
    let (Some(&ours), Some(&theirs)) = (bytes.first(), bytes.get(1)) else {
        return "";
    };
    let held = if markers.contains('-') { b'-' } else { b' ' };
    match (ours == held, theirs == held) {
        (true, false) => "ours",
        (false, true) => "theirs",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::super::parse_patch;
    use super::super::testkit::PATCH;
    use super::*;

    // The constants below are real git output, taken off git 2.55 in
    // throwaway repositories (probe scripts, observed) — from `git
    // diff` unless a constant says otherwise, and trimmed where only the
    // shape matters.

    /// A `UU` conflict left as git wrote it: markers in the file, two
    /// parents, and one hunk that also carries a change neither side
    /// disputed (`five` → `FIVE-theirs`).
    const CONFLICTED: &str = "\
diff --cc f.txt
index 804ce7b,ba44bb1..0000000
--- a/f.txt
+++ b/f.txt
@@@ -1,5 -1,5 +1,9 @@@
  one
++<<<<<<< HEAD
 +TWO-ours
++=======
+ TWO-theirs
++>>>>>>> topic
  three
  four
- five
+ FIVE-theirs
";

    /// The four marker shapes a resolved and an unresolved conflict put
    /// on screen (measured, git 2.55): `- ` ours / ` -` theirs on a
    /// resolved file, `+ ` / ` +` while the markers are still in the
    /// tree, `--` both, `++` a line typed while resolving.
    #[test]
    fn a_side_is_read_off_the_marker_columns() {
        assert_eq!(side_of_markers("- "), "ours");
        assert_eq!(side_of_markers(" -"), "theirs");
        assert_eq!(side_of_markers(" +"), "ours");
        assert_eq!(side_of_markers("+ "), "theirs");
        assert_eq!(side_of_markers("--"), "");
        assert_eq!(side_of_markers("++"), "");
        assert_eq!(side_of_markers("  "), "");
        // A single-parent diff has no marker columns at all.
        assert_eq!(side_of_markers(""), "");
        assert_eq!(side_of_markers("-"), "");
    }

    #[test]
    fn reads_a_conflicted_file_as_one_combined_patch() {
        let files = parse_patch(CONFLICTED.as_bytes());
        assert_eq!(files.len(), 1);
        let f = &files[0];
        assert!(f.is_combined);
        assert!(!f.unmerged);
        assert_eq!(f.path(), "f.txt");
        assert_eq!(f.hunks.len(), 1);
        let h = &f.hunks[0];
        assert_eq!((h.old_start, h.old_count), (1, 5), "first parent");
        assert_eq!(h.extra_old, vec![(1, 5)], "second parent");
        assert_eq!((h.new_start, h.new_count), (1, 9), "the result");
        assert_eq!(h.lines.len(), 10);
    }

    #[test]
    fn combined_lines_are_coloured_the_way_git_colours_them() {
        // Measured with `color.ui=always`: any `+` column makes the line
        // an addition, any `-` a deletion, all-spaces context.
        let h = &parse_patch(CONFLICTED.as_bytes())[0].hunks[0];
        let seen: Vec<(&str, DiffLineKind)> = h
            .lines
            .iter()
            .map(|l| (l.markers.as_str(), l.kind))
            .collect();
        assert_eq!(
            seen,
            vec![
                ("  ", DiffLineKind::Context),
                ("++", DiffLineKind::Addition),
                (" +", DiffLineKind::Addition),
                ("++", DiffLineKind::Addition),
                ("+ ", DiffLineKind::Addition),
                ("++", DiffLineKind::Addition),
                ("  ", DiffLineKind::Context),
                ("  ", DiffLineKind::Context),
                ("- ", DiffLineKind::Deletion),
                ("+ ", DiffLineKind::Addition),
            ]
        );
        assert_eq!(h.lines[1].text, "<<<<<<< HEAD", "markers are content");
        assert_eq!(h.lines[2].text, "TWO-ours");
    }

    #[test]
    fn combined_line_numbers_count_the_result_and_the_first_parent() {
        let h = &parse_patch(CONFLICTED.as_bytes())[0].hunks[0];
        let seen: Vec<(Option<u32>, Option<u32>)> =
            h.lines.iter().map(|l| (l.old_no, l.new_no)).collect();
        assert_eq!(
            seen,
            vec![
                (Some(1), Some(1)), // `  one`   — in both
                (None, Some(2)),    // `++<<<<<<<` — in neither parent
                (Some(2), Some(3)), // ` +TWO-ours` — ours has it
                (None, Some(4)),    // `++=======`
                (None, Some(5)),    // `+ TWO-theirs` — theirs has it, not ours
                (None, Some(6)),    // `++>>>>>>>`
                (Some(3), Some(7)),
                (Some(4), Some(8)),
                (Some(5), None), // `- five` — ours had it, the result does not
                (None, Some(9)), // `+ FIVE-theirs`
            ]
        );
    }

    #[test]
    fn a_line_both_parents_lost_is_still_one_deletion() {
        // `--one`: both sides had it and the result does not. Emptying a
        // conflicted file is the shortest way to produce one.
        let patch = "\
diff --cc n.txt
index 3e0f775,b81406d..0000000
--- a/n.txt
+++ b/n.txt
@@@ -1,2 -1,2 +1,0 @@@
--one
- OURS
 -THEIRS
";
        let h = &parse_patch(patch.as_bytes())[0].hunks[0];
        assert_eq!(h.lines[0].kind, DiffLineKind::Deletion);
        assert_eq!(h.lines[0].old_no, Some(1), "the first parent had it");
        assert_eq!(h.lines[0].new_no, None);
        // The second parent's line only counts against the second parent,
        // so the first parent's numbering does not move for it.
        assert_eq!(h.lines[1].old_no, Some(2), "` OURS` is ours' line 2");
        assert_eq!(h.lines[2].old_no, None, "`THEIRS` is not ours' at all");
    }

    #[test]
    fn a_combined_heading_survives_the_extra_at_signs() {
        // The heading git prints keeps its trailing space, and the closing
        // run is three `@` rather than two.
        let patch = "\
diff --cc code.rs
--- a/code.rs
+++ b/code.rs
@@@ -5,5 -5,5 +5,9 @@@ fn main()
      let f = 6;
++<<<<<<< HEAD
 +    let OURS = 7;
";
        let h = &parse_patch(patch.as_bytes())[0].hunks[0];
        assert_eq!(h.heading, "fn main()");
        assert_eq!(h.lines[0].text, "    let f = 6;", "content keeps indent");
    }

    #[test]
    fn a_binary_conflict_names_neither_side() {
        // Unlike the unified form, this one does not say `a/x and b/x`.
        let patch = "\
diff --cc bin.dat
index 9a16380,9e992b9..0000000
Binary files differ
";
        let files = parse_patch(patch.as_bytes());
        assert!(files[0].is_binary);
        assert!(files[0].is_combined);
        assert!(files[0].hunks.is_empty());
        assert_eq!(files[0].path(), "bin.dat");
    }

    #[test]
    fn an_unmerged_path_with_no_patch_is_still_a_file() {
        // Deleted on one side: there is no second blob to compare, so git
        // says only this. It arrives among ordinary patches and must not
        // swallow the one that follows.
        let patch = "\
diff --cc add.txt
--- a/add.txt
+++ b/add.txt
@@@ -1,1 -1,1 +1,3 @@@
++<<<<<<< HEAD
 +ours
+ theirs
* Unmerged path del.txt
diff --git a/plain.txt b/plain.txt
--- a/plain.txt
+++ b/plain.txt
@@ -1 +1 @@
-x
+y
";
        let files = parse_patch(patch.as_bytes());
        assert_eq!(files.len(), 3);
        assert!(files[0].is_combined);
        assert_eq!(files[1].path(), "del.txt");
        assert!(files[1].unmerged);
        assert!(files[1].hunks.is_empty());
        assert!(!files[1].is_combined, "there was nothing to combine");
        assert_eq!(files[2].path(), "plain.txt");
        assert!(!files[2].is_combined);
        assert_eq!(files[2].hunks[0].lines[0].kind, DiffLineKind::Deletion);
    }

    #[test]
    fn a_conflicted_file_that_matches_one_side_has_a_header_and_no_hunks() {
        // `--cc` prints only the hunks that differ from *every* parent, so
        // resolving by taking one side wholesale empties the patch while
        // the path stays unmerged. The file entry still has to exist —
        // dropping it would read as "no such file" rather than "nothing
        // left to decide".
        let patch = "\
diff --cc code.rs
index 211b973,f7fe72f..0000000
--- a/code.rs
+++ b/code.rs
";
        let files = parse_patch(patch.as_bytes());
        assert_eq!(files.len(), 1);
        assert!(files[0].is_combined);
        assert!(files[0].hunks.is_empty());
    }

    #[test]
    fn a_unified_patch_carries_no_markers() {
        // The field is what tells the two forms apart downstream, so the
        // ordinary case has to leave it empty rather than fill it in.
        let files = parse_patch(PATCH.as_bytes());
        assert!(!files[0].is_combined);
        assert!(files[0].hunks[0].extra_old.is_empty());
        assert!(files[0].hunks[0].lines.iter().all(|l| l.markers.is_empty()));
    }

    #[test]
    fn three_parents_are_read_as_three_columns() {
        // Not reachable from a conflicted working tree — git refuses to
        // stop an octopus merge in one (measured) — but `diff-tree -c` on
        // an octopus commit prints this, and the shape is the same one.
        let patch = "\
diff --combined f.txt
@@@@ -1,1 -1,1 -1,1 +1,2 @@@@
++ a
  +b
";
        let h = &parse_patch(patch.as_bytes())[0].hunks[0];
        assert_eq!(h.extra_old, vec![(1, 1), (1, 1)]);
        assert_eq!(h.lines[0].markers, "++ ");
        assert_eq!(h.lines[0].text, "a");
        assert_eq!(h.lines[0].old_no, None, "the first parent lacks it");
        assert_eq!(h.lines[1].markers, "  +");
        assert_eq!(h.lines[1].old_no, Some(1));
    }
}
