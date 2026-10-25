//! A branch on a remote travels as one pair (`remote::RemoteBranch`): the
//! remote and the name over there, each handed to git as given — where
//! both names hold a slash, the branch here goes by another name, and a
//! tag over there shares the branch's.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use crate::support::TestRepo;
use crate::support::exec::{Log, logged};
use crate::support::remote::{far, origin_and_clone};
use platitude_core::branch;
use platitude_core::remote;

/// A `file://` remote answers instantly; the budget just has to exist.
const NET: std::time::Duration = remote::DEFAULT_NETWORK_TIMEOUT;

/// `origin`'s bare repository added again as `team/fork`, a tag
/// `feature/x` over there on `main`, and `topic` here with a commit of its
/// own, checked out. The bare repository and the clone.
fn slashed_remote() -> (TestRepo, TestRepo) {
    let (mut bare, mut work) = origin_and_clone();
    bare.git(&["tag", "feature/x", "main"]);
    let url = bare.file_url();
    work.git(&["remote", "add", "team/fork", &url]);
    work.git(&["switch", "-c", "topic"]);
    work.commit_file("topic.txt", "ours\n", "our work");
    (bare, work)
}

/// The rows of `log` that start with one of `heads`, in the order they ran.
fn rows(log: &Log, heads: &[&str]) -> Vec<String> {
    log.displays()
        .into_iter()
        .filter(|row| heads.iter().any(|head| row.starts_with(head)))
        .collect()
}

/// The ask, the first push, a leased one, a refused and a granted delete,
/// and the upstream taken off after it: the remote goes as `team/fork`,
/// the name over there as `refs/heads/feature/x` (the lease of a send by
/// its bare name), and the tag of that name is never touched.
#[tokio::test]
async fn a_pair_reaches_git_as_its_two_names() {
    let (mut bare, mut work) = slashed_remote();
    let target = far("team/fork", "feature/x");
    let tag = bare.git(&["rev-parse", "refs/tags/feature/x"]);
    let (exec, log, cancel) = logged();

    let free = remote::branch_tip(&exec, &work.path, &target, NET, &cancel)
        .await
        .expect("asked");
    assert_eq!(free, None, "the tag of that name is no branch");

    let first = remote::publish_spec("topic", &target, "");
    remote::push(&exec, &work.path, &first, NET, &cancel)
        .await
        .expect("published");
    let ours = work.git(&["rev-parse", "topic"]);
    assert_eq!(bare.git(&["rev-parse", "refs/heads/feature/x"]), ours);
    assert_eq!(work.git(&["config", "branch.topic.remote"]), "team/fork");
    assert_eq!(
        work.git(&["config", "branch.topic.merge"]),
        "refs/heads/feature/x"
    );

    let there = remote::branch_tip(&exec, &work.path, &target, NET, &cancel)
        .await
        .expect("asked")
        .expect("the branch is there now");
    assert_eq!(there.to_hex(), ours);

    work.commit_file("topic.txt", "more\n", "more of ours");
    let leased = remote::publish_spec("topic", &target, &ours);
    remote::push(&exec, &work.path, &leased, NET, &cancel)
        .await
        .expect("the lease holds");
    let moved = work.git(&["rev-parse", "topic"]);
    assert_eq!(bare.git(&["rev-parse", "refs/heads/feature/x"]), moved);

    let stale = remote::delete_remote_branch(&exec, &work.path, &target, &ours, NET, &cancel)
        .await
        .expect_err("the branch moved on from the commit the lease names");
    let report = stale.report().expect("the refusal is a report");
    assert_eq!(
        (report.remote.as_str(), report.name.as_str()),
        ("team/fork", "feature/x")
    );
    assert!(stale.is_outdated(), "{stale}");

    remote::delete_remote_branch(&exec, &work.path, &target, &moved, NET, &cancel)
        .await
        .expect("deleted");
    let untracked = remote::untrack_deleted(&exec, &work.path, &target, &cancel)
        .await
        .expect("untracked");
    assert_eq!(untracked, ["topic"]);
    let config = work.git(&["config", "--local", "--list"]);
    assert!(
        !config.contains("branch.topic."),
        "no upstream is left naming the deleted branch: {config}"
    );
    let heads = bare.git(&["for-each-ref", "--format=%(refname)", "refs/heads"]);
    assert_eq!(heads, "refs/heads/main", "only the branch went");
    assert_eq!(bare.git(&["rev-parse", "refs/tags/feature/x"]), tag);

    let ask = "git ls-remote --heads --end-of-options team/fork refs/heads/feature/x";
    let send = "git push --porcelain --no-follow-tags --set-upstream";
    let delete = "git push --porcelain --no-follow-tags --force-with-lease=refs/heads/feature/x";
    assert_eq!(
        rows(&log, &["git ls-remote", "git push"]),
        [
            ask.to_string(),
            format!("{send} -- team/fork refs/heads/topic:refs/heads/feature/x"),
            ask.to_string(),
            format!(
                "{send} --force-with-lease=feature/x:{ours} \
                 -- team/fork refs/heads/topic:refs/heads/feature/x"
            ),
            format!("{delete}:{ours} --delete -- team/fork refs/heads/feature/x"),
            format!("{delete}:{moved} --delete -- team/fork refs/heads/feature/x"),
        ]
    );
}

/// Both of git's ways of writing an upstream take the pair as given: two
/// keys for a name not fetched, `--set-upstream-to` on the remote-tracking
/// ref for one that is.
#[tokio::test]
async fn an_upstream_is_written_either_way_from_the_two_names() {
    let (mut bare, mut work) = slashed_remote();
    bare.git(&["branch", "feature/x", "main"]);
    work.git(&["fetch", "team/fork"]);
    let (exec, log, cancel) = logged();

    branch::set_upstream(
        &exec,
        &work.path,
        "topic",
        &far("team/fork", "brand/new"),
        &cancel,
    )
    .await
    .expect("written as two keys");
    branch::set_upstream(
        &exec,
        &work.path,
        "main",
        &far("team/fork", "feature/x"),
        &cancel,
    )
    .await
    .expect("written by git's own flag");

    for (name, merge) in [
        ("topic", "refs/heads/brand/new"),
        ("main", "refs/heads/feature/x"),
    ] {
        assert_eq!(
            work.git(&["config", &format!("branch.{name}.remote")]),
            "team/fork"
        );
        assert_eq!(
            work.git(&["config", &format!("branch.{name}.merge")]),
            merge
        );
    }
    assert_eq!(
        rows(&log, &["git config --local branch.", "git branch"]),
        [
            "git config --local branch.topic.remote team/fork",
            "git config --local branch.topic.merge refs/heads/brand/new",
            "git branch --set-upstream-to=refs/remotes/team/fork/feature/x --end-of-options main",
        ]
    );
}

/// A replace sends the new name from the old pair's remote-tracking ref,
/// deletes the old name leased to the commit shown, and repoints what
/// tracked it — every name spelled from the pair, the tag of the old
/// name left where it was.
#[tokio::test]
async fn a_replace_reaches_git_from_the_old_pair_and_the_new_name() {
    let (mut bare, mut work) = slashed_remote();
    let tag = bare.git(&["rev-parse", "refs/tags/feature/x"]);
    work.git(&["push", "-u", "team/fork", "topic:refs/heads/feature/x"]);
    let shown = work.git(&["rev-parse", "refs/remotes/team/fork/feature/x"]);
    let (exec, log, cancel) = logged();

    remote::replace_remote_branch(
        &exec,
        &work.path,
        &far("team/fork", "feature/x"),
        "feature/y",
        &shown,
        NET,
        &cancel,
    )
    .await
    .expect("replaced");

    let heads = bare.git(&["for-each-ref", "--format=%(refname)", "refs/heads"]);
    assert_eq!(heads, "refs/heads/feature/y\nrefs/heads/main");
    assert_eq!(bare.git(&["rev-parse", "refs/heads/feature/y"]), shown);
    assert_eq!(bare.git(&["rev-parse", "refs/tags/feature/x"]), tag);
    assert_eq!(work.git(&["config", "branch.topic.remote"]), "team/fork");
    assert_eq!(
        work.git(&["config", "branch.topic.merge"]),
        "refs/heads/feature/y"
    );
    let send = "git push --porcelain --no-follow-tags";
    assert_eq!(
        rows(&log, &["git push", "git branch"]),
        [
            format!("{send} -- team/fork refs/remotes/team/fork/feature/x:refs/heads/feature/y"),
            format!(
                "{send} --force-with-lease=refs/heads/feature/x:{shown} \
                 --delete -- team/fork refs/heads/feature/x"
            ),
            "git branch --set-upstream-to=refs/remotes/team/fork/feature/y --end-of-options topic"
                .to_string(),
        ]
    );
}
