//! A diff with a part pressed on it, against the diff git prints after the
//! write (worked by hand from what each side holds; the reads pin three
//! lines of context). `stage_integration::pressed_on` holds the same rule
//! against git itself.

use crate::parse::diff::{DiffLineKind, DiffLineLocation, FilePatch, parse_patch};
use crate::parts::{Moves, Part, after, same_lines};

const HEAD: &str =
    "diff --git a/a.txt b/a.txt\nindex 1111111..2222222 100644\n--- a/a.txt\n+++ b/a.txt\n";

/// A line added near the top, and one rewritten further down.
const TWO_HUNKS: &str = "@@ -1,4 +1,5 @@\n one\n+new1\n two\n three\n four\n\
@@ -10,4 +11,4 @@ heading\n ten\n-eleven\n+ELEVEN\n twelve\n thirteen\n";

fn read(body: &str) -> Vec<FilePatch> {
    parse_patch(format!("{HEAD}{body}").as_bytes())
}

/// Each hunk as its numbers, each line as its kind and its two numbers.
fn said(patches: &[FilePatch]) -> Vec<String> {
    let no = |n: Option<u32>| n.map_or(-1, i64::from);
    patches
        .iter()
        .flat_map(|p| &p.hunks)
        .flat_map(|h| {
            std::iter::once(format!(
                "-{},{} +{},{}",
                h.old_start, h.old_count, h.new_start, h.new_count
            ))
            .chain(h.lines.iter().map(move |l| {
                let kind = match l.kind {
                    DiffLineKind::Context => "ctx",
                    DiffLineKind::Addition => "add",
                    DiffLineKind::Deletion => "del",
                    DiffLineKind::NoNewline => "note",
                };
                format!("{kind} {} {}", no(l.old_no), no(l.new_no))
            }))
        })
        .collect()
}

fn part(hunk: usize, line: Option<usize>, moves: Moves) -> Part {
    Part { hunk, line, moves }
}

#[test]
fn a_hunk_staged_leaves_and_the_old_side_below_it_moves_down() {
    let laid = after(&read(TWO_HUNKS), &[part(0, None, Moves::Old)]);
    assert!(laid.exact);
    assert_eq!(
        said(&laid.patches),
        [
            "-11,4 +11,4",
            "ctx 11 11",
            "del 12 -1",
            "add -1 12",
            "ctx 13 13",
            "ctx 14 14"
        ]
    );
    assert_eq!(laid.patches[0].hunks[0].heading, "heading");
    let first = DiffLineLocation {
        patch: 0,
        hunk: 0,
        line: 0,
    };
    assert_eq!(
        laid.origin(first).map(|o| (o.at.hunk, o.at.line, o.turned)),
        Some((1, 0, false)),
        "a line keeps its place in the read: its colours are its own"
    );
}

#[test]
fn a_hunk_discarded_moves_the_new_side_below_it_up() {
    let laid = after(&read(TWO_HUNKS), &[part(0, None, Moves::New)]);
    assert_eq!(said(&laid.patches)[0], "-10,4 +10,4");
}

#[test]
fn a_line_staged_turns_into_context_and_the_old_side_counts_it() {
    let laid = after(&read(TWO_HUNKS), &[part(1, Some(2), Moves::Old)]);
    assert_eq!(
        said(&laid.patches)[6..],
        [
            "-10,5 +11,4",
            "ctx 10 11",
            "del 11 -1",
            "ctx 12 12",
            "ctx 13 13",
            "ctx 14 14"
        ]
    );
    let turned = DiffLineLocation {
        patch: 0,
        hunk: 1,
        line: 2,
    };
    assert!(laid.origin(turned).is_some_and(|o| o.turned));
}

#[test]
fn a_hunks_only_change_staged_by_its_line_takes_the_hunk() {
    let by_line = after(&read(TWO_HUNKS), &[part(0, Some(1), Moves::Old)]);
    let by_hunk = after(&read(TWO_HUNKS), &[part(0, None, Moves::Old)]);
    assert_eq!(said(&by_line.patches), said(&by_hunk.patches));
}

#[test]
fn context_a_part_grows_at_an_end_is_cut_to_three_lines() {
    let body = "@@ -1,6 +1,8 @@\n a\n b\n c\n+X\n d\n+Y\n e\n f\n";
    let laid = after(&read(body), &[part(0, Some(3), Moves::Old)]);
    assert_eq!(
        said(&laid.patches),
        [
            "-3,5 +3,6",
            "ctx 3 3",
            "ctx 4 4",
            "ctx 5 5",
            "add -1 6",
            "ctx 6 7",
            "ctx 7 8"
        ]
    );
}

#[test]
fn a_hunk_the_part_could_split_is_left_as_read() {
    let body = "@@ -1,12 +1,15 @@\n a\n b\n c\n+X\n d\n e\n f\n+Y\n g\n h\n i\n+Z\n j\n k\n l\n";
    let laid = after(&read(body), &[part(0, Some(7), Moves::Old)]);
    assert!(!laid.exact, "seven lines of context between two changes");
    assert_eq!(said(&laid.patches), said(&read(body)));
}

#[test]
fn a_line_unstaged_leaves_the_staged_side() {
    let body = "@@ -1,3 +1,3 @@\n a\n-b\n+B\n c\n";
    let laid = after(&read(body), &[part(0, Some(2), Moves::New)]);
    assert_eq!(
        said(&laid.patches),
        ["-1,3 +1,2", "ctx 1 1", "del 2 -1", "ctx 3 2"]
    );
    let laid = after(&read(body), &[part(0, Some(1), Moves::New)]);
    assert_eq!(
        said(&laid.patches),
        ["-1,3 +1,4", "ctx 1 1", "ctx 2 2", "add -1 3", "ctx 3 4"],
        "a removal unstaged is on both sides again"
    );
}

#[test]
fn a_change_left_beside_a_missing_newline_is_left_as_read() {
    let body = "@@ -1,2 +1,3 @@\n a\n-b\n\\ No newline at end of file\n+b\n+c\n\\ No newline at end of file\n";
    let laid = after(&read(body), &[part(0, Some(4), Moves::Old)]);
    assert!(!laid.exact);
    assert_eq!(said(&laid.patches), said(&read(body)));
}

#[test]
fn a_diff_of_two_patches_is_left_as_read() {
    let body = format!(
        "{TWO_HUNKS}diff --git a/b.txt b/b.txt\nindex 3333333..4444444 100644\n--- a/b.txt\n\
         +++ b/b.txt\n@@ -1 +1 @@\n-x\n+y\n"
    );
    let laid = after(&read(&body), &[part(0, None, Moves::Old)]);
    assert!(!laid.exact);
    assert_eq!(said(&laid.patches), said(&read(&body)));
}

/// The second part was pressed on the diff with the first taken out: what
/// was hunk 1 there is hunk 0.
#[test]
fn parts_compose_in_the_order_they_were_pressed() {
    let both = after(
        &read(TWO_HUNKS),
        &[part(0, None, Moves::Old), part(0, Some(2), Moves::Old)],
    );
    assert!(both.exact);
    assert_eq!(
        said(&both.patches),
        [
            "-11,5 +11,4",
            "ctx 11 11",
            "del 12 -1",
            "ctx 13 12",
            "ctx 14 13",
            "ctx 15 14"
        ]
    );
}

#[test]
fn the_same_lines_leave_the_headings_out_and_a_patch_with_no_hunk() {
    let laid = after(&read(TWO_HUNKS), &[part(0, None, Moves::Old)]);
    let git = read("@@ -11,4 +11,4 @@ other heading\n ten\n-eleven\n+ELEVEN\n twelve\n thirteen\n");
    assert!(same_lines(&laid.patches, &git));
    let all = after(
        &read(TWO_HUNKS),
        &[part(0, None, Moves::Old), part(0, None, Moves::Old)],
    );
    assert!(same_lines(&all.patches, &[]), "git prints nothing");
    let moved = read("@@ -11,4 +11,4 @@\n ten\n-eleven\n+ELEVEN!\n twelve\n thirteen\n");
    assert!(!same_lines(&laid.patches, &moved));
}
