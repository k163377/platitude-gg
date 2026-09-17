//! Every case here drives `read`, so they sit together in
//! one file.

use super::*;

/// Builds patch bytes from lines, so a `\r` in a test is visible
/// where it matters.
fn patch(lines: &[&str]) -> Vec<u8> {
    let mut out = Vec::new();
    for l in lines {
        out.extend_from_slice(l.as_bytes());
        out.push(b'\n');
    }
    out
}

const HEAD: [&str; 4] = [
    "diff --git a/f.txt b/f.txt",
    "index 4cb29ea..e1587ff 100644",
    "--- a/f.txt",
    "+++ b/f.txt",
];

fn with_head(body: &[&str]) -> Vec<u8> {
    let mut lines = HEAD.to_vec();
    lines.extend_from_slice(body);
    patch(&lines)
}

#[test]
fn a_whole_file_flip_is_read_as_a_flip() {
    // The exact shape git printed for a three-line LF file rewritten
    // with CRLF (measured).
    let raw = with_head(&[
        "@@ -1,3 +1,3 @@",
        "-one",
        "-two",
        "-three",
        "+one\r",
        "+two\r",
        "+three\r",
    ]);
    assert_eq!(
        read_one(&raw),
        Reading::Flipped {
            from: Eol::Lf,
            to: Eol::Crlf
        }
    );
}

#[test]
fn one_crlf_line_in_an_lf_file_is_read_as_mixed() {
    let raw = with_head(&[
        "@@ -1,7 +1,7 @@",
        " a",
        " b",
        " c",
        "-d",
        "+d\r",
        " e",
        " f",
        " g",
    ]);
    assert_eq!(
        read_one(&raw),
        Reading::Mixed {
            lines: 1,
            added: Eol::Crlf,
            file: Eol::Lf
        }
    );
}

#[test]
fn the_file_a_notice_names_is_the_one_the_untouched_lines_use() {
    // Most of the file is rewritten with CRLF but some lines are left
    // alone: what the file "uses" has to come from those, or a change
    // big enough to outvote the file would report itself as normal.
    let mut body = vec!["@@ -1,6 +1,6 @@", " keep", " keep"];
    body.extend(std::iter::repeat_n("-line", 4));
    body.extend(std::iter::repeat_n("+line\r", 4));
    assert_eq!(
        read_one(&with_head(&body)),
        Reading::Mixed {
            lines: 4,
            added: Eol::Crlf,
            file: Eol::Lf
        }
    );
}

#[test]
fn a_file_that_was_already_mixed_says_nothing_until_the_change_adds_to_it() {
    let settled = with_head(&["@@ -1,4 +1,5 @@", " a", " b", " c\r", "-d", "+d", "+e"]);
    assert_eq!(read_one(&settled), Reading::Quiet);

    let worsened = with_head(&["@@ -1,4 +1,5 @@", " a", " b", " c\r", "-d", "+d", "+e\r"]);
    assert_eq!(
        read_one(&worsened),
        Reading::Mixed {
            lines: 1,
            added: Eol::Crlf,
            file: Eol::Lf
        }
    );
}

#[test]
fn a_new_file_reports_the_ending_it_arrived_with() {
    // `--no-index` against /dev/null is how an untracked file is
    // rendered; `new file mode` is what a staged add prints.
    let raw = patch(&[
        "diff --git a/fresh.txt b/fresh.txt",
        "new file mode 100644",
        "index 0000000..e3c39d1",
        "--- /dev/null",
        "+++ b/fresh.txt",
        "@@ -0,0 +1,2 @@",
        "+new\r",
        "+file\r",
    ]);
    assert_eq!(read_one(&raw), Reading::NewFile { eol: Eol::Crlf });
}

#[test]
fn a_new_file_that_arrives_mixed_is_reported_as_mixed() {
    let raw = patch(&[
        "diff --git a/fresh.txt b/fresh.txt",
        "new file mode 100644",
        "index 0000000..e3c39d1",
        "--- /dev/null",
        "+++ b/fresh.txt",
        "@@ -0,0 +1,3 @@",
        "+new",
        "+file\r",
        "+here",
    ]);
    assert_eq!(
        read_one(&raw),
        Reading::Mixed {
            lines: 1,
            added: Eol::Crlf,
            file: Eol::Lf
        }
    );
}

#[test]
fn a_file_with_no_ending_at_all_reports_its_first() {
    // Measured shape: the marker follows the old side's only line.
    let raw = with_head(&[
        "@@ -1 +1 @@",
        "-no-newline-here",
        "\\ No newline at end of file",
        "+no-newline-here",
    ]);
    assert_eq!(read_one(&raw), Reading::FirstEnding { eol: Eol::Lf });

    let crlf = with_head(&[
        "@@ -1 +2 @@",
        "-no-newline-here",
        "\\ No newline at end of file",
        "+no-newline-here\r",
    ]);
    assert_eq!(read_one(&crlf), Reading::FirstEnding { eol: Eol::Crlf });
}

#[test]
fn losing_the_final_newline_is_not_an_ending_change() {
    // The new side's last line has no terminator, so the tally
    // passes it over: an LF tally would read as a flip.
    let raw = with_head(&[
        "@@ -1,2 +1,2 @@",
        " one",
        "-two",
        "+two",
        "\\ No newline at end of file",
    ]);
    assert_eq!(read_one(&raw), Reading::Quiet);
}

#[test]
fn a_deletion_only_change_says_nothing() {
    let raw = with_head(&["@@ -1,3 +1,2 @@", " a", "-b\r", " c"]);
    assert_eq!(read_one(&raw), Reading::Quiet);
}

#[test]
fn a_removed_file_says_nothing() {
    let raw = patch(&[
        "diff --git a/gone.txt b/gone.txt",
        "deleted file mode 100644",
        "index e3c39d1..0000000",
        "--- a/gone.txt",
        "+++ /dev/null",
        "@@ -1,2 +0,0 @@",
        "-new\r",
        "-file\r",
    ]);
    assert_eq!(read_one(&raw), Reading::Quiet);
}

#[test]
fn binary_and_combined_patches_say_nothing() {
    let binary = patch(&[
        "diff --git a/x.png b/x.png",
        "index 1111111..2222222 100644",
        "Binary files a/x.png and b/x.png differ",
    ]);
    assert_eq!(read_one(&binary), Reading::Quiet);

    let combined = patch(&[
        "diff --cc c.txt",
        "index 1111111,2222222..3333333",
        "--- a/c.txt",
        "+++ b/c.txt",
        "@@@ -1,2 -1,2 +1,3 @@@",
        "++mine\r",
        " +theirs",
    ]);
    assert_eq!(read_one(&combined), Reading::Quiet);

    let unmerged = patch(&["* Unmerged path c.txt"]);
    assert_eq!(read_one(&unmerged), Reading::Quiet);
}

#[test]
fn a_content_line_that_looks_like_a_header_stays_content() {
    // The hunk counts say how many lines belong to it, so a context
    // line reading `diff --git …` cannot start a second file.
    let raw = with_head(&[
        "@@ -1,3 +1,3 @@",
        " diff --git a/x b/x",
        "-plain",
        "+plain\r",
        " tail",
    ]);
    let got = read(&raw);
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].path, "f.txt");
    assert_eq!(
        got[0].reading,
        Reading::Mixed {
            lines: 1,
            added: Eol::Crlf,
            file: Eol::Lf
        }
    );
}

#[test]
fn a_multi_file_patch_reports_only_the_files_with_something_to_say() {
    let mut raw = with_head(&["@@ -1,2 +1,2 @@", " a", "-b", "+b\r"]);
    raw.extend_from_slice(&patch(&[
        "diff --git a/quiet.txt b/quiet.txt",
        "index 1111111..2222222 100644",
        "--- a/quiet.txt",
        "+++ b/quiet.txt",
        "@@ -1,2 +1,2 @@",
        " a",
        "-b",
        "+c",
    ]));
    raw.extend_from_slice(&patch(&[
        "diff --git a/deep/dir/second.txt b/deep/dir/second.txt",
        "index 3333333..4444444 100644",
        "--- a/deep/dir/second.txt",
        "+++ b/deep/dir/second.txt",
        "@@ -1,3 +1,3 @@",
        "-x",
        "-y",
        "-z",
        "+x\r",
        "+y\r",
        "+z\r",
    ]));
    let got = read(&raw);
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].path, "f.txt");
    assert_eq!(got[1].path, "deep/dir/second.txt");
    assert_eq!(
        got[1].reading,
        Reading::Flipped {
            from: Eol::Lf,
            to: Eol::Crlf
        }
    );
}

#[test]
fn only_the_two_exact_cases_are_offered_to_history() {
    assert!(
        Reading::Flipped {
            from: Eol::Lf,
            to: Eol::Crlf
        }
        .is_exact()
    );
    assert!(
        Reading::Mixed {
            lines: 1,
            added: Eol::Crlf,
            file: Eol::Lf
        }
        .is_exact()
    );
    assert!(!Reading::NewFile { eol: Eol::Crlf }.is_exact());
    assert!(!Reading::FirstEnding { eol: Eol::Crlf }.is_exact());
    assert!(!Reading::Quiet.is_exact());
}

#[test]
fn empty_and_broken_input_reads_as_nothing() {
    assert!(read(b"").is_empty());
    assert!(read(b"not a patch at all\n").is_empty());
    assert_eq!(read_one(b""), Reading::Quiet);
}
