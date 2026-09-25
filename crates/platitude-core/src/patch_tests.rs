//! Tests of [`crate::patch`], in a file of their own (structure.md §分割).

use crate::patch::*;

const TWO_HUNKS: &str = "\
diff --git a/f.txt b/f.txt
index 1111111..2222222 100644
--- a/f.txt
+++ b/f.txt
@@ -1,3 +1,4 @@ fn head
 one
-two
+two changed
+two and a half
 three
@@ -10,3 +11,3 @@
 ten
-eleven
+eleven changed
 twelve
";

fn text(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).expect("utf-8 patch")
}

#[test]
fn counts_hunks() {
    assert_eq!(hunk_count(TWO_HUNKS.as_bytes()), 2);
    assert_eq!(hunk_count(b""), 0);
}

#[test]
fn whole_hunk_selection_keeps_the_hunk_verbatim() {
    let out = build_partial(
        TWO_HUNKS.as_bytes(),
        &[HunkSelect::whole(0)],
        PatchSide::Forward,
    )
    .expect("patch");
    let out = text(out);
    assert!(out.contains("--- a/f.txt"), "file header kept");
    assert!(out.contains("@@ -1,3 +1,4 @@ fn head"), "got: {out}");
    assert!(out.contains("+two and a half"));
    assert!(!out.contains("eleven"), "second hunk dropped");
}

#[test]
fn second_hunk_alone_is_renumbered_against_the_old_side() {
    let out = text(
        build_partial(
            TWO_HUNKS.as_bytes(),
            &[HunkSelect::whole(1)],
            PatchSide::Forward,
        )
        .expect("patch"),
    );
    // Applied on its own the new side starts where the old side does.
    assert!(out.contains("@@ -10,3 +10,3 @@"), "got: {out}");
}

#[test]
fn forward_line_selection_demotes_unselected_deletions_to_context() {
    // Body indices: 0 " one", 1 "-two", 2 "+two changed",
    // 3 "+two and a half", 4 " three". Stage only the second addition.
    let out = text(
        build_partial(
            TWO_HUNKS.as_bytes(),
            &[HunkSelect::lines(0, [3])],
            PatchSide::Forward,
        )
        .expect("patch"),
    );
    assert!(out.contains("@@ -1,3 +1,4 @@"), "got: {out}");
    assert!(out.contains(" two\n"), "unselected deletion became context");
    assert!(!out.contains("+two changed"));
    assert!(out.contains("+two and a half"));
}

#[test]
fn forward_deletion_only_selection_shrinks_the_new_side() {
    let out = text(
        build_partial(
            TWO_HUNKS.as_bytes(),
            &[HunkSelect::lines(0, [1])],
            PatchSide::Forward,
        )
        .expect("patch"),
    );
    // The old side is invariant (3); the new side loses the deletion.
    assert!(out.contains("@@ -1,3 +1,2 @@"), "got: {out}");
    assert!(out.contains("-two\n"));
    assert!(!out.contains("two changed"));
}

#[test]
fn reverse_line_selection_demotes_unselected_additions_to_context() {
    let out = text(
        build_partial(
            TWO_HUNKS.as_bytes(),
            &[HunkSelect::lines(0, [1])],
            PatchSide::Reverse,
        )
        .expect("patch"),
    );
    // With -R the new side is the pre-image and keeps its count (4); the
    // old side grows by the additions that stay staged, now context.
    assert!(out.contains("@@ -1,5 +1,4 @@"), "got: {out}");
    assert!(out.contains("-two\n"), "selected deletion kept");
    assert!(
        out.contains(" two changed\n") && out.contains(" two and a half\n"),
        "unselected additions became context: {out}"
    );
}

#[test]
fn reverse_second_hunk_alone_is_renumbered_against_the_new_side() {
    let out = text(
        build_partial(
            TWO_HUNKS.as_bytes(),
            &[HunkSelect::whole(1)],
            PatchSide::Reverse,
        )
        .expect("patch"),
    );
    assert!(out.contains("@@ -11,3 +11,3 @@"), "got: {out}");
}

#[test]
fn all_context_selection_yields_nothing() {
    assert!(
        build_partial(
            TWO_HUNKS.as_bytes(),
            &[HunkSelect::lines(0, [])],
            PatchSide::Forward,
        )
        .is_none()
    );
    assert!(build_partial(TWO_HUNKS.as_bytes(), &[], PatchSide::Forward).is_none());
}

#[test]
fn crlf_content_survives_byte_for_byte() {
    let raw = "--- a/f\n+++ b/f\n@@ -1,2 +1,2 @@\n x\r\n-y\r\n+z\r\n";
    let out =
        build_partial(raw.as_bytes(), &[HunkSelect::whole(0)], PatchSide::Forward).expect("patch");
    assert!(
        out.windows(4).any(|w| w == b"+z\r\n"),
        "carriage return kept: {:?}",
        String::from_utf8_lossy(&out)
    );
}

#[test]
fn no_newline_marker_follows_its_line() {
    let raw = "\
--- a/f
+++ b/f
@@ -1,2 +1,2 @@
 keep
-old
\\ No newline at end of file
+new
\\ No newline at end of file
";
    // Selecting only the addition drops the deletion's marker with it.
    let out = text(
        build_partial(
            raw.as_bytes(),
            &[HunkSelect::lines(0, [3])],
            PatchSide::Forward,
        )
        .expect("patch"),
    );
    assert_eq!(out.matches("\\ No newline").count(), 1, "got: {out}");
    assert!(out.contains(" old\n"), "deletion demoted to context: {out}");
}

#[test]
fn a_kept_last_line_keeps_its_no_newline_marker() {
    let raw = "\
--- a/f
+++ b/f
@@ -1,2 +1,2 @@
-one
-two
\\ No newline at end of file
+ONE
+TWO
\\ No newline at end of file
";
    // Body indices: 0 "-one", 1 "-two", 2 marker, 3 "+ONE", 4 "+TWO".
    // Staging one -> ONE leaves "two" as the last line of both sides,
    // so the file still ends where it did.
    let out = text(
        build_partial(
            raw.as_bytes(),
            &[HunkSelect::lines(0, [0, 3])],
            PatchSide::Forward,
        )
        .expect("patch"),
    );
    assert_eq!(
        out, "@@ -1,2 +1,2 @@\n-one\n+ONE\n two\n\\ No newline at end of file\n",
        "got: {out}"
    );
}

/// A replacement lists all of its deletions before its additions, so a
/// line left unselected has to move past the additions that replace the
/// lines above it.
const REPLACED_PAIR: &str = "\
--- a/f
+++ b/f
@@ -1,3 +1,3 @@
-one
-two
+ONE
+TWO
 tail
";

#[test]
fn forward_line_selection_keeps_an_unselected_line_in_place() {
    // Body indices: 0 "-one", 1 "-two", 2 "+ONE", 3 "+TWO".
    // Stage one -> ONE only: the result has to read ONE, two, tail.
    let out = text(
        build_partial(
            REPLACED_PAIR.as_bytes(),
            &[HunkSelect::lines(0, [0, 2])],
            PatchSide::Forward,
        )
        .expect("patch"),
    );
    assert_eq!(
        out, "@@ -1,3 +1,3 @@\n-one\n+ONE\n two\n tail\n",
        "got: {out}"
    );
}

#[test]
fn reverse_line_selection_keeps_an_unselected_line_in_place() {
    // Unstaging two -> TWO: applied with -R the new side is the
    // pre-image, so ONE has to stay above the line being restored.
    let out = text(
        build_partial(
            REPLACED_PAIR.as_bytes(),
            &[HunkSelect::lines(0, [1, 3])],
            PatchSide::Reverse,
        )
        .expect("patch"),
    );
    assert_eq!(
        out, "@@ -1,3 +1,3 @@\n ONE\n-two\n+TWO\n tail\n",
        "got: {out}"
    );
}

#[test]
fn multi_file_patch_keeps_only_files_with_selected_hunks() {
    let raw = format!(
        "{TWO_HUNKS}diff --git a/g.txt b/g.txt\n--- a/g.txt\n+++ b/g.txt\n@@ -1 +1 @@\n-a\n+b\n"
    );
    let out = text(
        build_partial(raw.as_bytes(), &[HunkSelect::whole(2)], PatchSide::Forward).expect("patch"),
    );
    assert!(out.contains("g.txt"));
    assert!(!out.contains("f.txt"), "unselected file dropped: {out}");
}
