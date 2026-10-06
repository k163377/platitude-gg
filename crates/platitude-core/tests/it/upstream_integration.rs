//! What a branch follows: the tracking branch a checkout of a remote ref
//! creates, the upstream a move onto a remote ref leaves alone, and the
//! upstream written for a branch, push or no push. git's own rules around
//! an upstream are in [`periodic`].

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use crate::support::TestRepo;
use crate::support::exec::env;
use platitude_core::branch::{self, CheckoutTarget};
use platitude_core::remote::{self, PushForce};

/// A `file://` remote answers instantly; the budget just has to exist.
const NET: std::time::Duration = remote::DEFAULT_NETWORK_TIMEOUT;

/// What `%(upstream)` prints for a local branch — the spelling the refs
/// listing carries (`RefEntry::upstream`) and joins the delete's
/// reference point on; `None` where nothing is configured.
fn upstream_of(repo: &mut TestRepo, branch: &str) -> Option<String> {
    let printed = repo.git(&[
        "for-each-ref",
        "--format=%(upstream)",
        &format!("refs/heads/{branch}"),
    ]);
    (!printed.is_empty()).then_some(printed)
}

/// A local branch created from a remote-tracking ref must track it, so the
/// sidebar badge and push defaults are right from the first checkout.
#[tokio::test]
async fn checkout_of_a_remote_branch_creates_a_tracking_branch() {
    let mut origin = TestRepo::init();
    origin.commit_file("a.txt", "one\n", "root");
    origin.git(&["checkout", "-b", "published"]);
    origin.commit_file("b.txt", "two\n", "published work");
    origin.git(&["checkout", "main"]);

    let mut clone = TestRepo::init();
    let url = origin.file_url();
    clone.git(&["remote", "add", "origin", &url]);
    clone.git(&["fetch", "origin"]);
    let (exec, cancel) = env();

    branch::checkout(
        &exec,
        &clone.path,
        &CheckoutTarget::Track {
            remote_ref: "origin/published".into(),
            local: "published".into(),
        },
        &cancel,
    )
    .await
    .expect("track");

    assert_eq!(
        clone.git(&["rev-parse", "--abbrev-ref", "HEAD"]),
        "published"
    );
    assert_eq!(
        clone.git(&["rev-parse", "--abbrev-ref", "HEAD@{upstream}"]),
        "origin/published"
    );
}

/// Moving a branch says nothing about what it reads, so the move must not
/// re-point it — without `--no-track` it silently would (rules-refs/core.md
/// 「`switch --force-create` は start が remote-tracking だと upstream を書き換える」).
#[tokio::test]
async fn moving_a_branch_onto_a_remote_ref_leaves_its_upstream_alone() {
    let mut origin = TestRepo::init();
    origin.commit_file("a.txt", "one\n", "root");
    origin.git(&["checkout", "-b", "topic"]);
    origin.commit_file("b.txt", "two\n", "what the remote has");
    origin.git(&["checkout", "main"]);
    origin.commit_file("c.txt", "three\n", "where the branch is moved to");

    let mut clone = TestRepo::init();
    let url = origin.file_url();
    clone.git(&["remote", "add", "origin", &url]);
    clone.git(&["fetch", "origin"]);
    clone.git(&["checkout", "-b", "topic", "origin/topic"]);
    let (exec, cancel) = env();

    assert_eq!(
        upstream_of(&mut clone, "topic").as_deref(),
        Some("refs/remotes/origin/topic"),
        "the branch reads its own remote before the move"
    );

    branch::checkout(
        &exec,
        &clone.path,
        &CheckoutTarget::ForceCreate {
            local: "topic".into(),
            start: "origin/main".into(),
        },
        &cancel,
    )
    .await
    .expect("force-create");

    assert_eq!(
        clone.git(&["rev-parse", "HEAD"]),
        clone.git(&["rev-parse", "origin/main"]),
        "the branch stands where it was moved"
    );
    assert_eq!(
        upstream_of(&mut clone, "topic").as_deref(),
        Some("refs/remotes/origin/topic"),
        "and still reads what it read before"
    );
}

/// `--set-upstream-to` must get the full remote-tracking refname: a local
/// branch named like the shorthand makes it ambiguous, and git refuses the
/// whole command. Pinned with that collision in place — the only shape the
/// two spellings disagree on.
#[tokio::test]
async fn the_upstream_is_named_by_the_one_spelling_that_reads_one_way() {
    let mut origin = TestRepo::init();
    origin.commit_file("a.txt", "one\n", "root");
    origin.git(&["branch", "feature/x"]);

    let mut clone = TestRepo::init();
    let url = origin.file_url();
    clone.git(&["remote", "add", "origin", &url]);
    clone.git(&["fetch", "origin"]);
    clone.git(&["checkout", "-b", "topic", "origin/main"]);
    clone.git(&["branch", "origin/feature/x", "origin/main"]);
    let (exec, cancel) = env();

    branch::set_upstream(&exec, &clone.path, "topic", "origin", "feature/x", &cancel)
        .await
        .expect("set upstream");
    assert_eq!(
        upstream_of(&mut clone, "topic").as_deref(),
        Some("refs/remotes/origin/feature/x"),
        "the remote branch, not the local one wearing its name"
    );
    clone.git_expect_failure(&["branch", "--set-upstream-to=origin/feature/x", "topic"]);
}

/// A name with no tracking ref here is recorded all the same — git's own
/// command refuses it, so the two keys are written directly (デザイン規約
/// §ブランチが測られる相手を決める; rules-refs/core.md
/// 「upstream の書き込みは 2 通りで、分かれ目は追跡 ref が手元に在るか」).
///
/// End to end, because the halves prove nothing apart: the keys, `[gone]`
/// until the far side exists, the push planned off them, and the counts
/// starting once it lands.
#[tokio::test]
async fn an_upstream_the_next_push_has_to_make_is_recorded_all_the_same() {
    let mut origin = TestRepo::init();
    origin.commit_file("a.txt", "one\n", "root");

    let mut clone = TestRepo::init();
    let url = origin.file_url();
    clone.git(&["remote", "add", "origin", &url]);
    clone.git(&["fetch", "origin"]);
    clone.git(&["checkout", "-b", "topic", "origin/main"]);
    clone.commit_file("b.txt", "two\n", "work of our own");
    let (exec, cancel) = env();
    // The refusal this stands in for, from git's own command.
    clone.git_expect_failure(&[
        "branch",
        "--set-upstream-to=refs/remotes/origin/brand-new",
        "topic",
    ]);

    branch::set_upstream(&exec, &clone.path, "topic", "origin", "brand-new", &cancel)
        .await
        .expect("a name not here is still an answer");

    assert_eq!(
        upstream_of(&mut clone, "topic").as_deref(),
        Some("refs/remotes/origin/brand-new"),
        "the pair git writes for a fetched name, written for one that is not"
    );
    assert_eq!(
        track_of(&mut clone, "topic"),
        "[gone]",
        "nothing here answers to the name until the push makes it"
    );

    let plan = remote::plan_current_push(&exec, &clone.path, "origin", PushForce::None, &cancel)
        .await
        .expect("a plan off the keys just written");
    assert_eq!(
        (plan.remote.as_str(), plan.remote_branch.as_str()),
        ("origin", "brand-new"),
        "the push goes where the branch says it belongs"
    );
    remote::push(&exec, &clone.path, &plan, NET, &cancel)
        .await
        .expect("the push makes the far side of it");

    assert_eq!(
        origin.git(&["rev-parse", "--abbrev-ref", "brand-new"]),
        "brand-new",
        "made over there under the name that was answered with"
    );
    assert_eq!(
        track_of(&mut clone, "topic"),
        "",
        "and the counts start speaking, level"
    );
}

/// A refused push still leaves the upstream written — why
/// `RepoSession::point_upstream_and_push` writes the pair first rather
/// than use `push --set-upstream`, which records it only on a push that
/// landed (デザイン規約 §手元の改名の後のリモート). The remote is a path to
/// nothing: no network (実装計画 §10).
#[tokio::test(flavor = "multi_thread")]
async fn a_push_the_far_side_turns_down_still_leaves_the_upstream_written() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let nowhere = repo.path.join("nowhere");
    repo.git(&["remote", "add", "origin", &nowhere.display().to_string()]);

    // git's own pair, for contrast.
    repo.git_expect_failure(&["push", "--set-upstream", "origin", "main"]);
    assert_eq!(
        upstream_of(&mut repo, "main"),
        None,
        "git records the pair only on a push that landed"
    );

    let (sink, session) = crate::support::session::opened(&repo).await;
    session.point_upstream_and_push("main".into(), "origin".into(), "main-renamed".into());
    let error = sink
        .wait_for("WriteFinished", |evs| {
            evs.iter().find_map(|e| match e {
                platitude_core::session::SessionEvent::WriteFinished { error, .. } => {
                    Some(error.clone())
                }
                _ => None,
            })
        })
        .await;
    assert!(error.is_some(), "the push had nowhere to go");

    assert_eq!(
        upstream_of(&mut repo, "main").as_deref(),
        Some("refs/remotes/origin/main-renamed"),
        "and the branch is already measured against the name the push was to make"
    );
    session.close();
}

/// What `%(upstream:track)` prints for a local branch: `[gone]` where
/// the upstream it names is not here, and empty where the two are level
/// (`refs::parse_track`).
fn track_of(repo: &mut TestRepo, branch: &str) -> String {
    repo.git(&[
        "for-each-ref",
        "--format=%(upstream:track)",
        &format!("refs/heads/{branch}"),
    ])
}

/// What the pre-merge run leaves out: git's own rules around an upstream —
/// what `branch --delete` measures "merged" against, and that a branch
/// another worktree holds still takes an upstream. The reads of ours
/// they meet are held pre-merge (`is_merged_into`'s two answers in
/// `branch_integration::unmerged_branch_needs_the_forced_delete` and
/// `session_integration::query`, `set_upstream`'s command line above).
mod periodic {
    use super::*;

    /// `branch --delete` measures "merged" against the configured upstream
    /// where there is one, HEAD otherwise: a branch merged into HEAD but
    /// ahead of its upstream is refused (the delete row's early `-D` rides
    /// on this).
    #[tokio::test]
    #[ignore = "what git measures a delete against: not worth the pre-merge run"]
    async fn the_upstream_is_the_delete_reference_point() {
        let mut origin = TestRepo::init();
        origin.commit_file("a.txt", "one\n", "root");

        let mut clone = TestRepo::init();
        let url = origin.file_url();
        clone.git(&["remote", "add", "origin", &url]);
        clone.git(&["fetch", "origin"]);
        clone.git(&["checkout", "-b", "topic", "origin/main"]);
        clone.commit_file("b.txt", "two\n", "ahead of the upstream");
        clone.git(&["checkout", "-b", "keeper"]);
        let (exec, cancel) = env();

        assert_eq!(
            upstream_of(&mut clone, "topic").as_deref(),
            Some("refs/remotes/origin/main"),
            "the configured upstream comes back as the full refname"
        );
        assert_eq!(
            upstream_of(&mut clone, "keeper"),
            None,
            "a branch started from a local commit has none"
        );

        assert!(
            branch::is_merged_into(&exec, &clone.path, "refs/heads/topic", "HEAD", &cancel)
                .await
                .expect("merge check"),
            "HEAD stands on the same commit"
        );
        assert!(
            !branch::is_merged_into(
                &exec,
                &clone.path,
                "refs/heads/topic",
                "refs/remotes/origin/main",
                &cancel
            )
            .await
            .expect("merge check"),
            "the upstream does not reach the new commit, which is the measure git refuses over"
        );
        let err = branch::delete(&exec, &clone.path, "topic", false, &cancel)
            .await
            .expect_err("refused over the upstream while HEAD stands on the tip");
        assert!(err.to_string().contains("not fully merged"), "{err}");
    }

    /// An upstream that is a local branch (`remote = .`): `%(upstream)`
    /// spells it `refs/heads/…`, the key the listing joins on
    /// (`BranchItem::upstream_oid`), and git measures the delete against it
    /// just the same (what it prints about HEAD is only a warning).
    #[tokio::test]
    #[ignore = "what git measures a delete against: not worth the pre-merge run"]
    async fn a_local_upstream_is_the_reference_point_too() {
        let mut repo = TestRepo::init();
        let root = repo.commit_file_id("a.txt", "one\n", "root");
        repo.git(&["checkout", "-b", "topic"]);
        repo.commit_file("b.txt", "two\n", "topic work");
        repo.git(&["checkout", "main"]);
        repo.git(&["merge", "--ff-only", "topic"]);
        repo.git(&["branch", "--set-upstream-to=main", "topic"]);
        repo.git(&["checkout", "-b", "elsewhere", &root]);
        let (exec, cancel) = env();

        assert_eq!(
            upstream_of(&mut repo, "topic").as_deref(),
            Some("refs/heads/main"),
            "a local upstream comes back under refs/heads/, the key the listing holds it by"
        );
        assert!(
            !branch::is_merged_into(&exec, &repo.path, "refs/heads/topic", "HEAD", &cancel)
                .await
                .expect("merge check"),
            "HEAD stands on the root and does not reach the tip"
        );
        branch::delete(&exec, &repo.path, "topic", false, &cancel)
            .await
            .expect("the tip is on its upstream, which is the measure");
        assert!(!repo.git(&["branch", "--list", "topic"]).contains("topic"));
    }

    /// An upstream that names nothing is still configured (`%(upstream)`
    /// prints the name) but is not the measure: git falls back to HEAD, and
    /// so does the listing join, which looks the name up.
    #[tokio::test]
    #[ignore = "what git measures a delete against: not worth the pre-merge run"]
    async fn an_upstream_that_names_nothing_leaves_head_as_the_reference_point() {
        let mut repo = TestRepo::init();
        let root = repo.commit_file_id("a.txt", "one\n", "root");
        repo.commit_file("b.txt", "two\n", "later work");
        repo.git(&["branch", "topic", "main"]);
        repo.git(&["branch", "helper", "main"]);
        repo.git(&["branch", "--set-upstream-to=helper", "topic"]);
        repo.git(&["branch", "-D", "helper"]);
        repo.git(&["checkout", "-b", "elsewhere", &root]);
        let (exec, cancel) = env();

        assert_eq!(
            upstream_of(&mut repo, "topic").as_deref(),
            Some("refs/heads/helper"),
            "the configuration outlives the branch it named"
        );
        let err = branch::delete(&exec, &repo.path, "topic", false, &cancel)
            .await
            .expect_err("HEAD is the measure once the upstream resolves to nothing");
        assert!(err.to_string().contains("not fully merged"), "{err}");

        repo.git(&["checkout", "main"]);
        branch::delete(&exec, &repo.path, "topic", false, &cancel)
            .await
            .expect("HEAD reaches the tip now");
        assert!(!repo.git(&["branch", "--list", "topic"]).contains("topic"));
    }

    /// A branch another worktree has checked out still takes an
    /// upstream, though its delete is refused; which rows that leaves is
    /// `offers::ref_menu`'s answer.
    #[tokio::test]
    #[ignore = "git letting a held branch take an upstream: not worth the pre-merge run"]
    async fn a_branch_another_worktree_holds_still_takes_an_upstream() {
        let mut origin = TestRepo::init();
        origin.commit_file("a.txt", "one\n", "root");

        let mut clone = TestRepo::init();
        let url = origin.file_url();
        clone.git(&["remote", "add", "origin", &url]);
        clone.git(&["fetch", "origin"]);
        clone.git(&["branch", "topic", "origin/main"]);
        let held = clone.path.join("held");
        let held_arg = held.to_string_lossy().to_string();
        clone.git(&["worktree", "add", &held_arg, "topic"]);
        let (exec, cancel) = env();

        branch::set_upstream(&exec, &clone.path, "topic", "origin", "main", &cancel)
            .await
            .expect("the other worktree is no refusal here");
        assert_eq!(
            upstream_of(&mut clone, "topic").as_deref(),
            Some("refs/remotes/origin/main")
        );
    }
}
