//! The open diff with parts pressed on it: the rows laid out of
//! `parts::after` (whose layout is held against git in core), when a read
//! after the parts in front is kept under the drawing, and when a press
//! empties the file's side.

use platitude_core::parse::diff::parse_patch;
use platitude_core::parts::{Moves, Part};

use super::ahead::{laid, takes_all};
use crate::encode::flatten_after;
use crate::ops::DiffPart;

const HEAD: &str =
    "diff --git a/a.txt b/a.txt\nindex 1111111..2222222 100644\n--- a/a.txt\n+++ b/a.txt\n";

/// A line added near the top, and one rewritten further down.
const TWO_HUNKS: &str = "@@ -1,4 +1,5 @@\n one\n+new1\n two\n three\n four\n\
@@ -10,4 +11,4 @@ heading\n ten\n-eleven\n+ELEVEN\n twelve\n thirteen\n";

fn pressed(hunk: usize, line: Option<usize>, landed: bool) -> DiffPart {
    DiffPart {
        id: 1,
        part: Part {
            hunk,
            line,
            moves: Moves::Old,
        },
        landed,
    }
}

#[test]
fn the_rows_are_the_laid_out_diffs_and_a_line_turned_to_context_is_not_emphasised() {
    let read = parse_patch(format!("{HEAD}{TWO_HUNKS}").as_bytes());
    let marks = platitude_core::intraline::marks(&read);
    let drawn = laid(&read, &[pressed(1, Some(2), false)]).expect("a part stands");
    let rows = flatten_after(&drawn, true, &Default::default(), Some(&marks));
    let turned = rows
        .iter()
        .find(|row| row.text.contains("ELEVEN"))
        .expect("the staged line");
    assert_eq!(turned.kind, "ctx");
    assert!(turned.emph.is_empty());
    let removed = rows
        .iter()
        .find(|row| row.kind == "del")
        .expect("the line it replaced");
    assert_eq!(
        (removed.hunk, removed.line, removed.patch),
        (1, 1, 0),
        "a row's place is its place in the laid-out diff, where a press on it lands"
    );
    assert!(laid(&read, &[]).is_none());
}

#[test]
fn only_a_press_on_the_last_change_of_one_files_edit_takes_all() {
    let one = parse_patch(format!("{HEAD}@@ -1,3 +1,3 @@\n a\n-b\n+B\n c\n").as_bytes());
    assert!(takes_all(&one, 0, -1), "the one hunk");
    assert!(!takes_all(&one, 0, 1), "one of two changed lines");
    let two = parse_patch(format!("{HEAD}{TWO_HUNKS}").as_bytes());
    assert!(!takes_all(&two, 0, -1));
    let last = parse_patch(format!("{HEAD}@@ -1,2 +1,3 @@\n a\n+b\n c\n").as_bytes());
    assert!(takes_all(&last, 0, 1), "the only changed line");
    let made = parse_patch(
        b"diff --git a/n.txt b/n.txt\nnew file mode 100644\nindex 0000000..1111111\n--- /dev/null\n\
          +++ b/n.txt\n@@ -0,0 +1 @@\n+n\n",
    );
    assert!(
        !takes_all(&made, 0, -1),
        "a file made is the entry's to say"
    );
    let mode = parse_patch(
        b"diff --git a/a.txt b/a.txt\nold mode 100644\nnew mode 100755\nindex 1111111..2222222\n\
          --- a/a.txt\n+++ b/a.txt\n@@ -1,3 +1,3 @@\n a\n-b\n+B\n c\n",
    );
    assert!(!takes_all(&mode, 0, -1), "no part moves the mode");
}
