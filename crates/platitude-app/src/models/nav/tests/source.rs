//! What a section's source answers about the refs it holds.

use super::super::testkit::*;
use super::super::*;

/// A remote named `my` is not `my/fork`'s: its branches and the commits
/// they stand on are cut by the configured names, not by the first slash.
#[test]
fn a_remotes_branches_and_their_tips_are_cut_by_its_configured_name() {
    let mut fork = remote("my/fork/main");
    fork.oid = oid("d");
    let names = ["my", "my/fork"];
    let source = Source::Remotes(named(vec![fork, remote("my/main")], &names));

    assert_eq!(source.branches_on("my", &names), vec!["main"]);
    assert_eq!(source.tips_on("my", &names), vec![oid("b")]);
    assert_eq!(source.branches_on("my/fork", &names), vec!["main"]);
    assert_eq!(source.tips_on("my/fork", &names), vec![oid("d")]);
    assert!(source.tips_on("other", &names).is_empty());
}

/// A remote row's PR is looked up by its branch half, cut where the writes
/// cut the row: `my/fork/main` is `main`, not `fork/main`; a remote no
/// configured name owns still loses its first segment.
#[test]
fn a_remote_rows_pr_key_is_its_branch_half_at_the_owning_remote() {
    let source = Source::Remotes(named(Vec::new(), &["my", "my/fork"]));
    let names = source.remote_names();
    assert_eq!(pr_key("my/fork/main", names), "main");
    assert_eq!(pr_key("my/main", names), "main");
    assert_eq!(pr_key("gone/feature/x", names), "feature/x");
    assert_eq!(pr_key("main", names), "main");
}
