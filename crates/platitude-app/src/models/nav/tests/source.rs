//! What a section's source answers about the refs it holds.

use super::testkit::*;
use super::*;

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
