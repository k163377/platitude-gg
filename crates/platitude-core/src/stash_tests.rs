//! Tests of [`crate::stash`]'s parsers and standing (structure.md §分割).

use crate::stash::*;

const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn z(tokens: &[&str]) -> Vec<u8> {
    let mut v = Vec::new();
    for t in tokens {
        v.extend_from_slice(t.as_bytes());
        v.push(0);
    }
    v
}

const BASE: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const HEAD: &str = "cccccccccccccccccccccccccccccccccccccccc";

#[test]
fn parses_entries() {
    let parents = format!("{BASE} {SHA}");
    let bytes = z(&[
        "stash@{0}",
        SHA,
        "1700000000",
        "WIP on main: 1234567 subject",
        &parents,
        "",
        "stash@{1}",
        SHA,
        "1699999999",
        "On feature: custom message",
        &parents,
        HEAD,
    ]);
    let stashes = parse_stashes(&bytes).unwrap();
    assert_eq!(stashes.len(), 2);
    assert_eq!(stashes[0].name, "stash@{0}");
    assert_eq!(stashes[0].time, 1_700_000_000);
    assert_eq!(stashes[0].stands, None, "a stash git made");
    assert_eq!(stashes[1].message, "On feature: custom message");
    let oid = |hex: &str| crate::oid::Oid::from_hex_str(hex).unwrap();
    assert_eq!(
        stashes[1].stands,
        Some(crate::discards::Stands {
            made: oid(BASE),
            on: oid(HEAD),
        }),
        "a discard's copy on a base made for it"
    );
}

#[test]
fn empty_output_means_no_stashes() {
    assert!(parse_stashes(b"").unwrap().is_empty());
    assert!(parse_stashes(b"\n").unwrap().is_empty());
}

#[test]
fn wrong_arity_is_an_error() {
    let bytes = z(&["stash@{0}", SHA]);
    assert!(parse_stashes(&bytes).is_err());
}

/// Real git's `stash list -z` bytes under tests/fixtures/, regenerated
/// only by `worktree_state::capture_fixtures` (command in the panic below).
#[test]
fn committed_stash_fixture_parses() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("stash_list.bin");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "missing fixture {} ({e}); regenerate with: cargo test -p platitude-core \
             --test it -- --ignored capture",
            path.display()
        )
    });
    let stashes = parse_stashes(&bytes).unwrap();
    assert_eq!(stashes.len(), 2);
    assert_eq!(stashes[0].name, "stash@{0}");
    assert!(stashes[1].message.contains("first stash メッセージ"));
}

#[test]
fn a_label_is_one_line_with_something_on_it() {
    assert!(is_valid_message("half of the refactor"));
    assert!(is_valid_message("日本語のラベル"));
    assert!(!is_valid_message(""));
    assert!(!is_valid_message("   "));
    assert!(!is_valid_message("two\nlines"));
    assert!(!is_valid_message("tab\there"));
}

#[test]
fn a_label_is_read_out_of_the_prefix_git_put_on_it() {
    assert_eq!(
        label_in("On main: half of the refactor"),
        "half of the refactor"
    );
    assert_eq!(
        label_in("On (no branch): on a detached head"),
        "on a detached head"
    );
    // Cut at the first `": "`, git's; a commit-style summary keeps its own.
    assert_eq!(
        label_in("On main: feat: write the summary"),
        "feat: write the summary"
    );
    // git's own, naming the commit rather than the work.
    assert_eq!(label_in("WIP on main: 1234567 subject"), "");
    assert_eq!(label_in("WIP on (no branch): 1234567 subject"), "");
    // `stash store` (a rename) writes no prefix.
    assert_eq!(label_in("a plain label"), "a plain label");
    assert_eq!(
        label_in("On its own with no colon"),
        "On its own with no colon"
    );
    // Accepted misreads: a label opening like git's prefix is read as one.
    assert_eq!(label_in("On second thought: revert it"), "revert it");
    assert_eq!(label_in("WIP on the parser"), "");
}

#[test]
fn the_standing_names_the_one_refusal_that_applies() {
    use crate::status::Counts;
    let counts = |staged, unstaged, untracked, conflicted| Counts {
        staged,
        unstaged,
        untracked,
        conflicted,
        partially_staged: 0,
    };
    // Unborn outranks everything: git refuses however dirty the tree is.
    assert_eq!(standing(true, &counts(1, 2, 3, 0)).as_str(), "unborn");
    assert_eq!(standing(false, &counts(1, 0, 0, 2)).as_str(), "conflicts");
    assert_eq!(standing(false, &counts(0, 0, 0, 0)).as_str(), "clean");
    assert_eq!(standing(false, &counts(0, 0, 1, 0)).as_str(), "ready");
    assert_eq!(standing(false, &counts(2, 1, 0, 0)).as_str(), "ready");
}
