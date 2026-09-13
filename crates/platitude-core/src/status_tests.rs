//! Tests of [`crate::status`]'s parser and tally, in a file of their own
//! (structure.md §分割: テストだけ巨大なら同ディレクトリの専用ファイルへ).

use crate::status::*;

fn z(tokens: &[&str]) -> Vec<u8> {
    let mut v = Vec::new();
    for t in tokens {
        v.extend_from_slice(t.as_bytes());
        v.push(0);
    }
    v
}

const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const H1: &str = "1111111111111111111111111111111111111111";
const H2: &str = "2222222222222222222222222222222222222222";

#[test]
fn a_file_changed_on_both_sides_is_two_rows() {
    let bytes = z(&[
        &format!("# branch.oid {SHA}"),
        &format!("1 MM N... 100644 100644 100644 {H1} {H2} both.txt"),
    ]);
    let status = parse_status(&bytes).unwrap();
    assert_eq!(status.staged().count(), 1);
    assert_eq!(status.unstaged().count(), 1);
    let kinds = Kinds::of(&status);
    assert_eq!(kinds.modified, 2);
    assert_eq!(kinds.total(), 2);
}

/// The same file counted for the pane that does not split it in two —
/// another working copy's, where the index is not this window's to move.
///
/// **The row and the list have to agree.** The row is the way into that
/// pane, so a tally counting sides over a list showing paths would be the
/// graph saying one number and the pane another about one tree.
#[test]
fn a_file_changed_on_both_sides_is_one_row_folded() {
    let bytes = z(&[
        &format!("# branch.oid {SHA}"),
        &format!("1 MM N... 100644 100644 100644 {H1} {H2} both.txt"),
        // Added to the index and then written again: the index's letter
        // is the one that says what this copy has that HEAD has not.
        &format!("1 AM N... 000000 100644 100644 {H1} {H2} new.txt"),
        "? fresh.txt",
        &format!("u UU N... 100644 100644 100644 100644 {H1} {H2} {H2} clash.txt"),
    ]);
    let status = parse_status(&bytes).unwrap();
    assert_eq!(
        Kinds::folded(&status),
        Kinds {
            added: 2,
            modified: 1,
            conflicted: 1,
            ..Kinds::default()
        }
    );
    // And this window's own pane goes on counting the sides it lists.
    assert_eq!(Kinds::of(&status).total(), 6);
}

#[test]
fn every_kind_lands_where_its_letter_says() {
    let bytes = z(&[
        &format!("# branch.oid {SHA}"),
        &format!("1 A. N... 000000 100644 100644 {H1} {H2} added.txt"),
        &format!("1 .D N... 100644 100644 000000 {H1} {H1} gone.txt"),
        &format!("1 .T N... 120000 120000 100644 {H1} {H1} was-a-link.txt"),
        &format!("2 R. N... 100644 100644 100644 {H1} {H1} R100 new-name.txt"),
        "old-name.txt",
        &format!("2 C. N... 100644 100644 100644 {H1} {H1} C75 copy.txt"),
        "source.txt",
        &format!("u UU N... 100644 100644 100644 100644 {H1} {H2} {H2} clash.txt"),
        "? untracked.txt",
    ]);
    let kinds = Kinds::of(&parse_status(&bytes).unwrap());
    assert_eq!(
        kinds,
        Kinds {
            // The staged `A`, and the untracked file.
            added: 2,
            modified: 1,
            deleted: 1,
            renamed: 1,
            copied: 1,
            conflicted: 1,
        }
    );
    assert_eq!(kinds.total(), 7);
}

#[test]
fn a_conflict_is_one_row_whatever_its_letters_say() {
    let bytes = z(&[
        &format!("# branch.oid {SHA}"),
        &format!("u AA N... 100644 100644 100644 100644 {H1} {H2} {H2} both-added.txt"),
        &format!("u DU N... 100644 100644 100644 100644 {H1} {H2} {H2} we-deleted.txt"),
    ]);
    let kinds = Kinds::of(&parse_status(&bytes).unwrap());
    assert_eq!(kinds.conflicted, 2);
    assert_eq!(kinds.total(), 2);
    assert_eq!(kinds.added, 0);
    assert_eq!(kinds.deleted, 0);
}

#[test]
fn a_hard_reset_takes_every_tracked_file_once_and_leaves_the_untracked() {
    let bytes = z(&[
        &format!("# branch.oid {SHA}"),
        // Changed on both sides: one file, one loss.
        &format!("1 MM N... 100644 100644 100644 {H1} {H2} both.txt"),
        &format!("1 .M N... 100644 100644 100644 {H1} {H2} unstaged.txt"),
        // In the index, so the reset writes over it.
        &format!("1 A. N... 000000 100644 100644 {H1} {H2} staged-add.txt"),
        &format!("u UU N... 100644 100644 100644 100644 {H1} {H2} {H2} clash.txt"),
        // Never told to git, so it survives.
        "? scratch.txt",
    ]);
    let counts = Counts::of(&parse_status(&bytes).unwrap());
    assert_eq!(counts.staged, 2);
    assert_eq!(counts.unstaged, 2);
    assert_eq!(counts.partially_staged, 1);
    assert_eq!(counts.hard_reset_takes(), 4);
}

#[test]
fn a_clean_tree_takes_nothing_and_a_clean_one_with_scratch_files_takes_nothing_either() {
    let bytes = z(&[&format!("# branch.oid {SHA}"), "? scratch.txt"]);
    let counts = Counts::of(&parse_status(&bytes).unwrap());
    assert_eq!(counts.untracked, 1);
    assert_eq!(counts.hard_reset_takes(), 0);
}

#[test]
fn a_clean_tree_has_nothing_to_tally() {
    let bytes = z(&[&format!("# branch.oid {SHA}"), "# branch.head main"]);
    let kinds = Kinds::of(&parse_status(&bytes).unwrap());
    assert_eq!(kinds, Kinds::default());
    assert_eq!(kinds.total(), 0);
}

#[test]
fn parses_headers_and_ordinary_entries() {
    let bytes = z(&[
        &format!("# branch.oid {SHA}"),
        "# branch.head main",
        "# branch.upstream origin/main",
        "# branch.ab +2 -1",
        "1 M. N... 100644 100644 100644 1111111111111111111111111111111111111111 2222222222222222222222222222222222222222 staged.txt",
        "1 .M N... 100644 100644 100644 1111111111111111111111111111111111111111 1111111111111111111111111111111111111111 un staged with spaces.txt",
        "? new file.txt",
    ]);
    let s = parse_status(&bytes).unwrap();
    assert_eq!(s.branch_oid.unwrap().to_hex(), SHA);
    assert_eq!(s.branch_head.as_deref(), Some("main"));
    assert_eq!(s.upstream.as_deref(), Some("origin/main"));
    assert_eq!((s.ahead, s.behind), (2, 1));
    assert_eq!(s.items.len(), 3);
    assert_eq!(s.staged().count(), 1);
    assert_eq!(s.unstaged().count(), 1);
    assert_eq!(s.untracked().count(), 1);
    assert_eq!(
        s.unstaged().next().unwrap().path(),
        "un staged with spaces.txt"
    );
    assert!(!s.has_conflicts());
    assert!(s.upstream_tracked);
}

/// A branch can name an upstream that has no remote-tracking ref: git
/// then leaves out `branch.ab` entirely.
#[test]
fn upstream_without_a_tracking_ref_reports_no_counts() {
    let bytes = z(&[
        &format!("# branch.oid {SHA}"),
        "# branch.head main",
        "# branch.upstream origin/main",
    ]);
    let s = parse_status(&bytes).unwrap();
    assert_eq!(s.upstream.as_deref(), Some("origin/main"));
    assert!(!s.upstream_tracked);
    assert_eq!((s.ahead, s.behind), (0, 0));
}

#[test]
fn parses_rename_with_following_orig_path() {
    let bytes = z(&[
        "2 R. N... 100644 100644 100644 1111111111111111111111111111111111111111 1111111111111111111111111111111111111111 R100 new name.txt",
        "old name.txt",
        "? other.txt",
    ]);
    let s = parse_status(&bytes).unwrap();
    assert_eq!(s.items.len(), 2);
    match &s.items[0] {
        StatusItem::Tracked {
            staged,
            path,
            orig_path,
            ..
        } => {
            assert_eq!(*staged, 'R');
            assert_eq!(path, "new name.txt");
            assert_eq!(orig_path.as_deref(), Some("old name.txt"));
        }
        other => panic!("expected rename entry, got {other:?}"),
    }
}

#[test]
fn parses_unmerged_and_detached() {
    let bytes = z(&[
        &format!("# branch.oid {SHA}"),
        "# branch.head (detached)",
        "u UU N... 100644 100644 100644 100644 1111111111111111111111111111111111111111 2222222222222222222222222222222222222222 3333333333333333333333333333333333333333 conflicted.txt",
    ]);
    let s = parse_status(&bytes).unwrap();
    assert_eq!(s.branch_head, None, "(detached) maps to None");
    assert!(s.has_conflicts());
    match &s.items[0] {
        StatusItem::Unmerged { ours, theirs, path } => {
            assert_eq!((*ours, *theirs), ('U', 'U'));
            assert_eq!(path, "conflicted.txt");
        }
        other => panic!("expected unmerged entry, got {other:?}"),
    }
}

#[test]
fn unborn_branch_has_no_oid() {
    let bytes = z(&["# branch.oid (initial)", "# branch.head main", "? x.txt"]);
    let s = parse_status(&bytes).unwrap();
    assert_eq!(s.branch_oid, None);
    assert_eq!(s.branch_head.as_deref(), Some("main"));
}

#[test]
fn empty_output_is_a_clean_tree() {
    let s = parse_status(b"").unwrap();
    assert!(s.items.is_empty());
    assert!(!s.has_conflicts());
}

#[test]
fn garbage_entry_is_fatal() {
    let bytes = z(&["Z whatever"]);
    assert!(parse_status(&bytes).is_err());
}

/// Raw `--porcelain=v2 -z` bytes out of real git, committed under
/// tests/fixtures/ and regenerated (never hand-edited) with:
/// `cargo test -p platitude-core --test it -- --ignored capture`
#[test]
fn committed_status_fixture_parses() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("status_v2.bin");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "missing fixture {} ({e}); regenerate with: cargo test -p platitude-core \
             --test it -- --ignored capture",
            path.display()
        )
    });
    let s = parse_status(&bytes).unwrap();
    assert_eq!(s.branch_head.as_deref(), Some("main"));
    assert!(s.staged().any(|i| i.path() == "staged.txt"));
    assert!(s.unstaged().any(|i| i.path() == "a.txt"));
    assert!(s.untracked().any(|i| i.path() == "untracked dir/inner.txt"));
    assert!(
        s.items.iter().any(|i| matches!(
            i,
            StatusItem::Tracked { staged: 'R', orig_path: Some(o), .. } if o == "old name.txt"
        )),
        "rename with unicode target present"
    );
}
