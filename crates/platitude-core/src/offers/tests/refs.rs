//! The words a ref's row is read in, where its upstream stands, and what a
//! worktree's spaced second click does.

use super::super::*;

#[test]
fn a_kind_word_nothing_answers_to_is_no_kind() {
    assert_eq!(RefKind::from_word("HEAD"), None);
    assert_eq!(RefKind::from_word(""), None);
    assert_eq!(RefKind::from_word("branch"), Some(RefKind::Branch));
    assert_eq!(
        RefKind::from_word("worktree"),
        Some(RefKind::Worktree),
        "the chip's word"
    );
}

#[test]
fn every_lead_reads_back_as_its_word() {
    for leads in [Leads::Nowhere, Leads::Holder, Leads::Itself] {
        assert_eq!(Leads::from_word(leads.word()), Some(leads));
    }
    assert_eq!(Leads::from_word("elsewhere"), None);
}

/// The colour the upstream's name wears: a remote reading, a branch here,
/// or the one HEAD is on — told by the tree the menu stands on, a gone
/// one by where it was configured.
#[test]
fn an_upstream_stands_where_its_configuration_and_the_tree_say() {
    let main = TreeStanding::on("main");
    assert_eq!(upstream_place("", false, &main), UpstreamPlace::None);
    assert_eq!(
        upstream_place("origin/feat", false, &main),
        UpstreamPlace::Remote
    );
    assert_eq!(upstream_place("main", true, &main), UpstreamPlace::Current);
    assert_eq!(upstream_place("release", true, &main), UpstreamPlace::Local);
    assert_eq!(
        upstream_place("main", true, &TreeStanding::detached()),
        UpstreamPlace::Local,
        "detached, no branch is the one HEAD is on"
    );
    assert_eq!(
        upstream_place("origin/main", true, &main),
        UpstreamPlace::Local,
        "a branch here may be called like a remote one"
    );
}

/// A worktree on no branch: the box opens on its folder, but on the
/// repository's own worktree nothing happens, and a lock or a gone folder
/// says why instead — the lock first.
#[test]
fn a_folder_is_renamed_unless_git_would_keep_it() {
    assert_eq!(worktree_rename(false, false, false), WorktreeRename::Box);
    assert_eq!(worktree_rename(true, false, false), WorktreeRename::Never);
    assert_eq!(worktree_rename(false, true, false), WorktreeRename::Locked);
    assert_eq!(worktree_rename(false, false, true), WorktreeRename::Gone);
    assert_eq!(worktree_rename(false, true, true), WorktreeRename::Locked);
    assert_eq!(WorktreeRename::Never.word(), "");
}
