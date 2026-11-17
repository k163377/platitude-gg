//! Tests of [`crate::status`]'s parser and tally (structure.md §分割).

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

/// Another worktree's pane lists one row per path, and its tally has to agree
/// with it (`Kinds`).
#[test]
fn a_file_changed_on_both_sides_is_one_row_folded() {
    let bytes = z(&[
        &format!("# branch.oid {SHA}"),
        &format!("1 MM N... 100644 100644 100644 {H1} {H2} both.txt"),
        // Added then edited: the index's letter (an addition) wins.
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
    // This window's pane still counts sides.
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

fn tracked(staged: char, unstaged: char, path: &str, orig: Option<&str>) -> StatusItem {
    StatusItem::Tracked {
        staged,
        unstaged,
        path: path.to_string(),
        orig_path: orig.map(str::to_string),
    }
}

fn holding(items: Vec<StatusItem>) -> WorktreeStatus {
    WorktreeStatus {
        branch_head: Some("main".to_string()),
        upstream: Some("origin/main".to_string()),
        upstream_tracked: true,
        ahead: 2,
        behind: 1,
        items,
        // Any number that is not the one nothing staged hashes to.
        staged_print: 7,
        ..WorktreeStatus::default()
    }
}

/// A row with nothing on the working tree's side goes with the commit,
/// whatever its index letter was.
#[test]
fn a_commit_takes_the_rows_staged_alone() {
    let status = holding(vec![
        tracked('M', '.', "edited.txt", None),
        tracked('A', '.', "added.txt", None),
        tracked('D', '.', "deleted.txt", None),
        tracked('R', '.', "renamed.txt", Some("old.txt")),
        tracked('C', '.', "copied.txt", Some("source.txt")),
        tracked('T', '.', "retyped.txt", None),
    ]);
    let left = status.left_by_commit().expect("no conflict stands in it");
    assert_eq!(left.items, []);
}

/// A row on both sides keeps the working tree's half, under the name the
/// commit recorded: the old name was the index side's.
#[test]
fn a_commit_leaves_the_working_tree_half_of_a_row_on_both_sides() {
    let status = holding(vec![
        tracked('M', 'M', "edited-twice.txt", None),
        tracked('A', 'M', "added-then-edited.txt", None),
        tracked('A', 'D', "added-then-deleted.txt", None),
        tracked('R', 'M', "renamed-then-edited.txt", Some("old.txt")),
        tracked('R', 'D', "renamed-then-deleted.txt", Some("older.txt")),
        tracked('M', 'T', "edited-then-retyped.txt", None),
    ]);
    let left = status.left_by_commit().expect("no conflict stands in it");
    assert_eq!(
        left.items,
        [
            tracked('.', 'M', "edited-twice.txt", None),
            tracked('.', 'M', "added-then-edited.txt", None),
            tracked('.', 'D', "added-then-deleted.txt", None),
            tracked('.', 'M', "renamed-then-edited.txt", None),
            tracked('.', 'D', "renamed-then-deleted.txt", None),
            tracked('.', 'T', "edited-then-retyped.txt", None),
        ]
    );
}

/// What the index never held is as it was: the unstaged rows, the untracked
/// files, and an intent-to-add entry's rename, whose old name is the
/// working tree side's.
#[test]
fn a_commit_leaves_what_was_not_staged_as_it_was() {
    let unstaged = vec![
        tracked('.', 'M', "edited.txt", None),
        tracked('.', 'D', "deleted.txt", None),
        tracked('.', 'A', "meant-to-add.txt", None),
        tracked('.', 'R', "moved-on-disk.txt", Some("where-it-was.txt")),
        StatusItem::Untracked {
            path: "new.txt".to_string(),
        },
    ];
    let mut items = vec![tracked('M', '.', "staged.txt", None)];
    items.extend(unstaged.iter().cloned());
    let left = holding(items)
        .left_by_commit()
        .expect("no conflict stands in it");
    assert_eq!(left.items, unstaged);
}

/// git takes no commit over a conflict, so no status follows from one.
#[test]
fn no_status_follows_a_commit_from_one_holding_a_conflict() {
    let status = holding(vec![
        tracked('M', '.', "staged.txt", None),
        StatusItem::Unmerged {
            ours: 'U',
            theirs: 'U',
            path: "clash.txt".to_string(),
        },
    ]);
    assert_eq!(status.left_by_commit(), None);
}

/// The index holds what HEAD does behind a commit — the print the parser
/// ends on where nothing is staged — and the branch lines are the ones the
/// commit was made over: no row says where HEAD went.
#[test]
fn a_commit_leaves_nothing_staged_and_the_branch_lines_as_they_were() {
    let status = holding(vec![
        tracked('M', 'M', "both.txt", None),
        tracked('A', '.', "added.txt", None),
    ]);
    let left = status.left_by_commit().expect("no conflict stands in it");
    assert_eq!(left.staged().count(), 0);
    let nothing_staged = parse_status(&z(&[
        &format!("1 .M N... 100644 100644 100644 {H1} {H1} both.txt"),
        "? new.txt",
    ]))
    .unwrap()
    .staged_print;
    assert_eq!(left.staged_print, nothing_staged);
    assert_ne!(left.staged_print, status.staged_print);
    assert_eq!(
        (
            left.branch_oid,
            left.branch_head,
            left.upstream,
            left.upstream_tracked,
            left.ahead,
            left.behind
        ),
        (
            status.branch_oid,
            status.branch_head.clone(),
            status.upstream.clone(),
            true,
            2,
            1
        )
    );
}

#[test]
fn garbage_entry_is_fatal() {
    let bytes = z(&["Z whatever"]);
    assert!(parse_status(&bytes).is_err());
}

/// Real git's `--porcelain=v2 -z` bytes under tests/fixtures/, regenerated
/// only by `worktree_state::capture_fixtures` (command in the panic below).
#[test]
fn committed_status_fixture_parses() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/status_v2.bin");
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
