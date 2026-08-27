//! Parsed patches flattened into the rows the diff pane draws.

use platitude_core::highlight::DiffColors;
use platitude_core::intraline::IntraMarks;
use platitude_core::parse::diff::{DiffLineKind, FilePatch};

use super::markup::{display_ranges, styled};

/// One flattened row of the diff pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffRow {
    /// `hunk` / `ctx` / `add` / `del` / `meta`.
    pub kind: &'static str,
    /// -1 when the side has no line number.
    pub old_no: i32,
    pub new_no: i32,
    /// What the row draws — plain text, or the same line marked up for
    /// `Text.StyledText` when [`DiffRow::rich`]. Only one of the two is
    /// ever held: keeping both doubles what a long diff costs.
    pub text: String,
    /// Whether [`DiffRow::text`] is markup. False wherever the theme had
    /// nothing to say — a language the set has never heard of, a hunk
    /// heading, git's own `\ No newline` note, a conflict marker.
    pub rich: bool,
    /// Display columns of what changed inside this row —
    /// `"col:wides:width:wides,…"`: where each run starts and how far it
    /// runs, said in the mono font's columns and in the wide glyphs
    /// standing in them. Empty where nothing is emphasised
    /// (`platitude_core::intraline`, laid out by [`display_ranges`]). The
    /// pane draws these as the stronger wash under the text
    /// (デザイン規約 §シンタックスハイライト), and needs the second number
    /// to place the first — a wide glyph is drawn from a fallback font
    /// whose advance need not be two of the mono one's.
    pub emph: String,
    /// One of git's conflict fences (`<<<<<<<` / `|||||||` / `=======` /
    /// `>>>>>>>`); the pane drops its voice for these
    /// (デザイン規約 §シンタックスハイライト).
    pub fence: bool,
    /// Which hunk of the file this row belongs to, and which line of that
    /// hunk it is (-1 on the hunk header). These are the same indices
    /// [`platitude_core::patch::HunkSelect`] addresses, so a row can be
    /// staged straight from what the pane is showing.
    pub hunk: i32,
    pub line: i32,
    /// Which of the read's patches the two above are counted within (-1
    /// where they name nothing). Hunks are numbered from zero inside each
    /// patch, so this is what makes the pair above an address: the
    /// selection reads a row's own source line back off
    /// `patches[patch].hunks[hunk].lines[line]` rather than keeping a
    /// second copy of the text (`DiffModel::source_line`).
    pub patch: i32,
    /// One marker column per side of a combined diff (`" +"`, `"++"`,
    /// `"- "`), empty on every ordinary row. Which side a line came from
    /// is in here and nowhere else: the colour cannot carry it, since git
    /// paints our side and theirs the same green.
    pub markers: String,
}

/// Whether the patch has a new side and no old one — a file the repository
/// is seeing for the first time (untracked, or newly added to the index).
///
/// The pane reads this to drop the piecemeal staging it would otherwise
/// offer (デザイン規約 §diff の中のステージ). A deleted file is not one
/// of these — its lines still exist on the old side, and a part of them
/// can still be staged.
///
/// The new side has to be named, not merely inferred from a missing old
/// one: a patch that carries no `diff --git` header at all parses with
/// both sides empty ([`platitude_core::parse::diff::parse_patch`]
/// synthesizes the file entry), and that is a diff whose shape is unknown
/// rather than one that is known to be new.
pub fn is_new_file(patches: &[FilePatch]) -> bool {
    !patches.is_empty()
        && patches.iter().all(|p| {
            // A conflicted path is never one of these: `AA` — both
            // branches invented the file — has no old side by
            // construction, and an unmerged entry has neither side
            // because git printed no patch at all.
            !p.is_combined && !p.unmerged && p.old_path.is_none() && p.new_path.is_some()
        })
}

/// Whether the diff compares its file against **more than one** side —
/// the form git prints for a conflicted path. Nothing in it can be staged
/// or thrown away piecemeal (`platitude_core::patch::is_combined` is the
/// floor under that), and its rows carry [`DiffRow::markers`].
pub fn is_combined(patches: &[FilePatch]) -> bool {
    patches.iter().any(|p| p.is_combined)
}

/// Whether git named the path as unmerged and printed no patch for it:
/// one of the two sides does not exist, so there is nothing to compare
/// (`DU` / `UD` / `AU` / `UA`). The pane has no rows to show and says what
/// the two sides did instead.
pub fn is_unmerged_only(patches: &[FilePatch]) -> bool {
    !patches.is_empty() && patches.iter().all(|p| p.unmerged)
}

/// Flattens parsed patches into displayable rows (hunk headers inline).
/// `binary_note` inserts the "(binary file)" meta row; the caller turns it
/// off when a preview (image / size summary) already covers that file.
/// `marks` is `None` on the repaint that lays colours over rows already
/// on screen — that pass keeps every row's `emph` as it is, so working
/// the columns out again would be thrown away.
pub fn flatten_patches(
    patches: &[FilePatch],
    binary_note: bool,
    colors: &DiffColors,
    marks: Option<&IntraMarks>,
) -> Vec<DiffRow> {
    let mut rows = Vec::new();
    for (patch_index, patch) in patches.iter().enumerate() {
        if patch.unmerged {
            // No rows: the pane builds the unmerged sentence from the
            // stage letters (`Words.conflict`).
            continue;
        }
        if patch.is_binary {
            if binary_note {
                rows.push(DiffRow {
                    kind: "meta",
                    old_no: -1,
                    new_no: -1,
                    text: String::from("(binary file)"),
                    rich: false,
                    emph: String::new(),
                    fence: false,
                    hunk: -1,
                    line: -1,
                    patch: -1,
                    markers: String::new(),
                });
            }
            continue;
        }
        for (hunk_index, hunk) in patch.hunks.iter().enumerate() {
            let heading = if hunk.heading.is_empty() {
                String::new()
            } else {
                format!(" {}", hunk.heading)
            };
            let hunk_no = i32::try_from(hunk_index).unwrap_or(-1);
            let patch_no = i32::try_from(patch_index).unwrap_or(-1);
            rows.push(DiffRow {
                kind: "hunk",
                old_no: -1,
                new_no: -1,
                text: hunk_header(hunk, &heading),
                rich: false,
                emph: String::new(),
                fence: false,
                hunk: hunk_no,
                line: -1,
                patch: patch_no,
                markers: String::new(),
            });
            for (line_index, line) in hunk.lines.iter().enumerate() {
                let kind = match line.kind {
                    DiffLineKind::Context => "ctx",
                    DiffLineKind::Addition => "add",
                    DiffLineKind::Deletion => "del",
                    DiffLineKind::NoNewline => "meta",
                };
                let read = colors.line(patch_index, hunk_index, line_index);
                let markup = styled(&line.text, &read.spans);
                let rich = !markup.is_empty();
                rows.push(DiffRow {
                    kind,
                    old_no: line.old_no.map_or(-1, |n| n as i32),
                    new_no: line.new_no.map_or(-1, |n| n as i32),
                    text: if rich { markup } else { line.text.clone() },
                    rich,
                    emph: marks.map_or_else(String::new, |marks| {
                        display_ranges(&line.text, marks.line(patch_index, hunk_index, line_index))
                    }),
                    fence: read.fence,
                    hunk: hunk_no,
                    line: i32::try_from(line_index).unwrap_or(-1),
                    patch: patch_no,
                    markers: line.markers.clone(),
                });
            }
        }
    }
    rows
}

/// The `@@` line as git writes it: one range per old side, and a run of
/// `@` one longer than that count on both ends.
fn hunk_header(hunk: &platitude_core::parse::diff::DiffHunk, heading: &str) -> String {
    let ats = "@".repeat(hunk.extra_old.len() + 2);
    let mut out = format!("{ats} -{},{}", hunk.old_start, hunk.old_count);
    for (start, count) in &hunk.extra_old {
        out.push_str(&format!(" -{start},{count}"));
    }
    out.push_str(&format!(
        " +{},{} {ats}{heading}",
        hunk.new_start, hunk.new_count
    ));
    out
}

#[cfg(test)]
mod tests {
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
}
