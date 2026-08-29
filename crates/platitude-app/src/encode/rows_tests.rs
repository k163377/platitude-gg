//! The cases for `rows`, which is at the line ceiling without them
//! (.claude/rules/structure.md §上限).

use platitude_core::highlight::DiffColors;

use super::*;
use platitude_core::parse::diff::parse_patch;

#[test]
fn rows_carry_the_markup_the_theme_gave_them() {
    let patch = "\
diff --git a/src/a.rs b/src/a.rs
--- a/src/a.rs
+++ b/src/a.rs
@@ -1,2 +1,2 @@
 fn main() {
-    let a = 1;
+    let a = 2;
";
    let patches = parse_patch(patch.as_bytes());
    let rows = flatten_patches(
        &patches,
        true,
        &platitude_core::highlight::colors(&patches, None),
        None,
    );
    assert!(
        rows.iter()
            .filter(|r| r.kind == "add" || r.kind == "del" || r.kind == "ctx")
            .all(|r| r.rich && r.text.starts_with("<font")),
        "every line of a language the set knows is marked up: {rows:?}"
    );
    assert!(rows.first().is_some_and(|r| r.kind == "hunk" && !r.rich));
}

#[test]
fn flatten_produces_hunk_headers_and_numbers() {
    let patch = "\
--- a/f
+++ b/f
@@ -1,2 +1,2 @@ heading
 same
-old
+new
";
    let rows = flatten_patches(
        &parse_patch(patch.as_bytes()),
        true,
        &DiffColors::default(),
        None,
    );
    assert_eq!(rows[0].kind, "hunk");
    assert!(rows[0].text.contains("@@ -1,2 +1,2 @@ heading"));
    assert_eq!(rows[1].kind, "ctx");
    assert_eq!((rows[1].old_no, rows[1].new_no), (1, 1));
    assert_eq!(rows[2].kind, "del");
    assert_eq!((rows[2].old_no, rows[2].new_no), (2, -1));
    assert_eq!(rows[3].kind, "add");
    assert_eq!((rows[3].old_no, rows[3].new_no), (-1, 2));
}

#[test]
fn flattened_rows_carry_the_indices_that_address_them() {
    let patch = "\
--- a/f
+++ b/f
@@ -1,2 +1,3 @@
 keep
+added
@@ -10,2 +11,1 @@
-removed
 tail
";
    let rows = flatten_patches(
        &parse_patch(patch.as_bytes()),
        true,
        &DiffColors::default(),
        None,
    );
    assert_eq!((rows[0].kind, rows[0].hunk, rows[0].line), ("hunk", 0, -1));
    assert_eq!((rows[1].kind, rows[1].hunk, rows[1].line), ("ctx", 0, 0));
    assert_eq!((rows[2].kind, rows[2].hunk, rows[2].line), ("add", 0, 1));
    assert_eq!((rows[3].kind, rows[3].hunk, rows[3].line), ("hunk", 1, -1));
    assert_eq!((rows[4].kind, rows[4].hunk, rows[4].line), ("del", 1, 0));
    assert_eq!((rows[5].kind, rows[5].hunk, rows[5].line), ("ctx", 1, 1));
}

// The shapes below are `git diff` output as it stands, taken off git
// 2.55 in a throwaway repository. An untracked file read the way
// `details::file_diff` reads one (`--no-index` against `/dev/null`)
// and the same file once added come out **byte for byte the same** —
// git labels the old side `a/fresh.txt` in the header either way and
// only `---` tells the truth about it — so one constant covers both.
const ADDED: &str = "\
diff --git a/fresh.txt b/fresh.txt
new file mode 100644
index 0000000..fbbee86
--- /dev/null
+++ b/fresh.txt
@@ -0,0 +1,2 @@
+alpha
+beta
";
const EDITED: &str = "\
diff --git a/kept.txt b/kept.txt
index 814f4a4..879de50 100644
--- a/kept.txt
+++ b/kept.txt
@@ -1,2 +1,2 @@
 one
-two
+TWO
";
const DELETED: &str = "\
diff --git a/gone.txt b/gone.txt
deleted file mode 100644
index bd43ee2..0000000
--- a/gone.txt
+++ /dev/null
@@ -1 +0,0 @@
-doomed
";

#[test]
fn a_file_with_only_a_new_side_is_a_new_file() {
    let patches = parse_patch(ADDED.as_bytes());
    assert!(is_new_file(&patches));
    // The header names the old side too; `---` is what takes it away.
    assert_eq!(patches[0].new_path.as_deref(), Some("fresh.txt"));
    assert_eq!(patches[0].old_path, None);
}

#[test]
fn a_file_that_existed_before_is_not_a_new_one() {
    assert!(!is_new_file(&parse_patch(EDITED.as_bytes())));
    assert!(!is_new_file(&parse_patch(DELETED.as_bytes())));
}

#[test]
fn a_diff_of_unknown_shape_is_not_read_as_new() {
    assert!(!is_new_file(&[]));
    let headerless = "\
@@ -1,1 +1,1 @@
-old
+new
";
    assert!(!is_new_file(&parse_patch(headerless.as_bytes())));
}

#[test]
fn binary_patch_flattens_to_meta_row() {
    let patch = "\
diff --git a/x.png b/x.png
Binary files a/x.png and b/x.png differ
";
    let rows = flatten_patches(
        &parse_patch(patch.as_bytes()),
        true,
        &DiffColors::default(),
        None,
    );
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kind, "meta");
    assert!(
        flatten_patches(
            &parse_patch(patch.as_bytes()),
            false,
            &DiffColors::default(),
            None,
        )
        .is_empty()
    );
}

/// `git diff` on a conflicted path, verbatim (git 2.55).
const CONFLICTED: &str = "\
diff --cc shared.txt
index 804ce7b,ba44bb1..0000000
--- a/shared.txt
+++ b/shared.txt
@@@ -1,3 -1,3 +1,7 @@@ heading
  one
++<<<<<<< HEAD
 +OURS
++=======
+ THEIRS
++>>>>>>> topic
  three
";

#[test]
fn a_combined_diff_flattens_with_its_marker_columns() {
    let rows = flatten_patches(
        &parse_patch(CONFLICTED.as_bytes()),
        true,
        &DiffColors::default(),
        None,
    );
    assert_eq!(rows[0].kind, "hunk");
    assert_eq!(rows[0].text, "@@@ -1,3 -1,3 +1,7 @@@ heading");
    assert_eq!(rows[0].markers, "", "a heading has no side of its own");

    let seen: Vec<(&str, &str, &str)> = rows[1..]
        .iter()
        .map(|r| (r.kind, r.markers.as_str(), r.text.as_str()))
        .collect();
    assert_eq!(
        seen,
        vec![
            ("ctx", "  ", "one"),
            ("add", "++", "<<<<<<< HEAD"),
            ("add", " +", "OURS"),
            ("add", "++", "======="),
            ("add", "+ ", "THEIRS"),
            ("add", "++", ">>>>>>> topic"),
            ("ctx", "  ", "three"),
        ]
    );
}

#[test]
fn a_conflicted_diff_is_combined_and_not_a_new_file() {
    let patches = parse_patch(CONFLICTED.as_bytes());
    assert!(is_combined(&patches));
    assert!(!is_unmerged_only(&patches));
    assert!(!is_new_file(&patches));
}

#[test]
fn a_conflict_both_sides_added_is_not_read_as_a_new_file() {
    let patch = "\
diff --cc added.txt
index 5b79a82,34a1fdf..0000000
--- a/added.txt
+++ b/added.txt
@@@ -1,1 -1,1 +1,5 @@@
++<<<<<<< HEAD
 +ours
++=======
+ theirs
++>>>>>>> topic
";
    let patches = parse_patch(patch.as_bytes());
    assert!(is_combined(&patches));
    assert!(!is_new_file(&patches));
}

#[test]
fn an_unmerged_path_contributes_no_rows() {
    let patches = parse_patch(b"* Unmerged path ours-del.txt\n");
    assert!(is_unmerged_only(&patches));
    assert!(!is_combined(&patches));
    assert!(!is_new_file(&patches), "it is not a new file either");
    assert!(flatten_patches(&patches, true, &DiffColors::default(), None).is_empty());
}

#[test]
fn a_unified_hunk_heading_is_unchanged_by_the_combined_form() {
    let rows = flatten_patches(
        &parse_patch(EDITED.as_bytes()),
        true,
        &DiffColors::default(),
        None,
    );
    assert_eq!(rows[0].text, "@@ -1,2 +1,2 @@");
    assert!(rows.iter().all(|r| r.markers.is_empty()));
}
