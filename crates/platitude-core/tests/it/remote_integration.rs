//! Remote traffic, exercised entirely offline: the remote is a `file://`
//! URL of a second local repository (実装計画 §10).
//!
//! What git alone decides about that traffic is in [`periodic`].

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use crate::support::TestRepo;
use crate::support::exec::env;
use crate::support::remote::{a_branch_over_there, moved_over_there, origin_and_clone};
use crate::support::session::{opened, write_answer, write_result};
use platitude_core::GitError;
use platitude_core::OperationKind;
use platitude_core::commit;
use platitude_core::remote::{self, PushForce, PushSpec};
use platitude_core::report::ReportKind;
use platitude_core::session::SessionEvent;

/// A `file://` remote answers instantly; the budget just has to exist.
const NET: std::time::Duration = remote::DEFAULT_NETWORK_TIMEOUT;

/// git has no command for this: the name moves, its commit stays, and the
/// local branch that tracked it follows.
#[tokio::test]
async fn replacing_a_remote_branch_moves_the_name_and_the_tracking() {
    let (mut bare, mut work) = origin_and_clone();
    let tip = a_branch_over_there(&mut work, "billing");
    work.git(&["switch", "billing"]);
    // Replacing publishes nothing: this commit is only here.
    work.commit_file("b.txt", "b2\n", "not published");

    replace_billing(&work, &tip)
        .await
        .expect("replace remote branch");

    assert!(
        !bare.git_ok(&["rev-parse", "--verify", "refs/heads/billing"]),
        "the old name is gone"
    );
    assert_eq!(
        bare.git(&["rev-parse", "billing-v2"]),
        tip,
        "the new name points where the old one did, not at what is only here"
    );
    assert_eq!(
        work.git(&["config", "branch.billing.merge"]),
        "refs/heads/billing-v2",
        "the branch that tracked it follows the name"
    );
    assert!(
        !work.git_ok(&["rev-parse", "--verify", "refs/remotes/origin/billing"]),
        "the tracking ref for the old name went with the delete"
    );
}

#[tokio::test]
async fn a_replace_whose_push_fails_deletes_nothing() {
    let (mut bare, mut work) = origin_and_clone();
    let tip = a_branch_over_there(&mut work, "billing");
    // Unrelated work already under the new name: the push cannot fast-forward.
    work.git(&["switch", "-c", "someone-else"]);
    work.commit_file("c.txt", "c\n", "not ours");
    work.git(&["push", "origin", "someone-else:billing-v2"]);
    work.git(&["switch", "billing"]);

    let error = replace_billing(&work, &tip)
        .await
        .expect_err("the push is refused");
    assert!(
        error.is_outdated() || matches!(error, GitError::Failed { .. }),
        "{error:?}"
    );
    assert_eq!(
        bare.git(&["rev-parse", "billing"]),
        tip,
        "the old name is still there"
    );
    assert_eq!(
        work.git(&["config", "branch.billing.merge"]),
        "refs/heads/billing",
        "and nothing was re-pointed"
    );
}

/// [`remote::replace_remote_branch`] of `billing` with `billing-v2` on
/// `origin`, leased to the commit the screen showed.
async fn replace_billing(work: &TestRepo, shown: &str) -> Result<(), GitError> {
    let (exec, cancel) = env();
    remote::replace_remote_branch(
        &exec,
        &work.path,
        "origin",
        "billing",
        "billing-v2",
        shown,
        NET,
        &cancel,
    )
    .await
}

/// The lease is on the commit the screen showed: a push since the last
/// fetch turns the delete into a refusal a fetch answers, and their
/// commit — never here — stays over there. Unmoved, the same delete goes.
#[tokio::test]
async fn a_remote_branch_that_moved_since_the_last_fetch_is_not_deleted() {
    let (mut bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();
    let shown = a_branch_over_there(&mut work, "topic");
    let theirs = moved_over_there(&bare, "topic", "theirs.txt", "pushed since our fetch");

    let err =
        remote::delete_remote_branch(&exec, &work.path, "origin", "topic", &shown, NET, &cancel)
            .await
            .expect_err("the remote is not where the screen showed it");
    let Some(report) = err.report() else {
        panic!("a stale lease reads as any outdated push: {err}");
    };
    assert_eq!(report.kind, ReportKind::MovedDelete);
    assert_eq!(
        (report.remote.as_str(), report.name.as_str()),
        ("origin", "topic")
    );
    assert_eq!(
        bare.git(&["rev-parse", "topic"]),
        theirs,
        "their commit is still over there"
    );

    work.git(&["fetch", "origin"]);
    let seen = work.git(&["rev-parse", "origin/topic"]);
    remote::delete_remote_branch(&exec, &work.path, "origin", "topic", &seen, NET, &cancel)
        .await
        .expect("leased to what is there, the delete goes");
    assert!(!bare.git_ok(&["rev-parse", "--verify", "refs/heads/topic"]));
}

/// The name goes qualified, so a branch the remote dropped since the last
/// fetch is the lease's refusal like any other move — a bare name fails
/// before the lease is weighed (`remote ref does not exist`) — and a tag
/// of the same name over there neither blocks the delete nor goes with it.
#[tokio::test]
async fn a_remote_branch_dropped_since_the_last_fetch_is_the_leases_refusal() {
    let (mut bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();
    let shown = a_branch_over_there(&mut work, "gone");
    bare.git(&["update-ref", "-d", "refs/heads/gone"]);

    let err =
        remote::delete_remote_branch(&exec, &work.path, "origin", "gone", &shown, NET, &cancel)
            .await
            .expect_err("nothing is there to hold the lease");
    assert_eq!(
        err.report().map(|report| report.kind),
        Some(ReportKind::MovedDelete),
        "{err}"
    );

    let tip = a_branch_over_there(&mut work, "twin");
    work.git(&["push", "origin", &format!("{tip}:refs/tags/twin")]);
    remote::delete_remote_branch(&exec, &work.path, "origin", "twin", &tip, NET, &cancel)
        .await
        .expect("the qualified name picks the branch");
    assert!(!bare.git_ok(&["rev-parse", "--verify", "refs/heads/twin"]));
    assert_eq!(
        bare.git(&["rev-parse", "refs/tags/twin"]),
        tip,
        "and the tag of the same name stays"
    );
}

/// Replace is a push and then the leased delete: refused, it stops with
/// both names over there and the tracking left on the old one.
#[tokio::test]
async fn a_replace_whose_delete_is_refused_leaves_both_names_and_the_tracking() {
    let (mut bare, mut work) = origin_and_clone();
    let shown = a_branch_over_there(&mut work, "billing");
    let theirs = moved_over_there(&bare, "billing", "theirs.txt", "pushed since our fetch");

    let err = replace_billing(&work, &shown)
        .await
        .expect_err("the old name moved since the screen showed it");
    assert!(err.is_outdated(), "{err}");
    assert_eq!(
        bare.git(&["rev-parse", "billing"]),
        theirs,
        "the old name stays"
    );
    assert_eq!(
        bare.git(&["rev-parse", "billing-v2"]),
        shown,
        "beside the new one, which went up first"
    );
    assert_eq!(
        work.git(&["config", "branch.billing.merge"]),
        "refs/heads/billing",
        "and nothing was re-pointed"
    );
}

/// `Delete both` runs the local half first (`session::delete_branch_everywhere`
/// says why), so a refused lease stops it half-way: the branch is gone here
/// and the remote keeps theirs. The refusal is the outdated one a push
/// gives, and so is what follows it: a fetch, after which the commit the
/// screen showed is reached from the remote-tracking ref.
#[tokio::test(flavor = "multi_thread")]
async fn delete_both_refused_by_the_lease_keeps_the_commit_under_the_remote_tracking_ref() {
    let (mut bare, mut work) = origin_and_clone();
    let shown = a_branch_over_there(&mut work, "topic");
    let theirs = moved_over_there(&bare, "topic", "theirs.txt", "pushed since our fetch");
    let (sink, session) = opened(&work).await;

    let id = session
        .delete_branch_everywhere(
            "topic".into(),
            "origin".into(),
            "topic".into(),
            false,
            shown.clone(),
        )
        .expect("accepted");
    let refused = write_answer(&sink, id).await;

    assert!(refused.is_some(), "the remote half is refused");
    assert!(
        !work.git_ok(&["rev-parse", "--verify", "refs/heads/topic"]),
        "the local half had already run"
    );
    assert_eq!(bare.git(&["rev-parse", "topic"]), theirs);

    assert_eq!(
        write_result(&sink, OperationKind::Fetch).await,
        None,
        "the refusal is caught up with, as a push's is"
    );
    assert_eq!(work.git(&["rev-parse", "origin/topic"]), theirs);
    assert!(
        work.git_ok(&["merge-base", "--is-ancestor", &shown, "origin/topic"]),
        "and the commit the screen showed is still reached from it"
    );
    session.close_and_wait().await;
}

/// A branch row's menu publishes the branch it names, wherever HEAD is,
/// and the remote's name is weighed against that branch's history.
#[tokio::test(flavor = "multi_thread")]
async fn a_branch_the_worktree_is_not_on_is_published_and_weighed_as_itself() {
    let (mut bare, mut work) = origin_and_clone();
    // `shared` is over there one commit behind `topic`; `main` has neither.
    work.git(&["switch", "-c", "topic"]);
    work.commit_file("b.txt", "b\n", "theirs");
    work.git(&["push", "origin", "topic:shared"]);
    work.commit_file("b.txt", "b2\n", "ours");
    work.git(&["switch", "main"]);
    let (sink, session) = opened(&work).await;

    session.check_remote_branch("origin".into(), "shared".into(), "topic".into());
    let (state, local) = sink
        .wait_for("the remote's answer", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::RemoteBranchChecked { state, local, .. } => {
                    Some((*state, local.clone()))
                }
                _ => None,
            })
        })
        .await;
    assert_eq!(
        state,
        remote::RemoteBranchState::FastForward,
        "their commit is one of topic's, though not one of HEAD's"
    );
    assert_eq!(
        local, "topic",
        "the answer says whose history it weighed, so a question about another branch cannot take it"
    );

    let id = session
        .publish(
            "topic".into(),
            "origin".into(),
            "shared".into(),
            String::new(),
        )
        .expect("the write is accepted");
    assert_eq!(write_answer(&sink, id).await, None);
    assert_eq!(
        bare.git(&["rev-parse", "shared"]),
        work.git(&["rev-parse", "topic"])
    );
    assert_eq!(
        work.git(&["config", "branch.topic.merge"]),
        "refs/heads/shared"
    );
    assert_eq!(
        work.git(&["symbolic-ref", "--short", "HEAD"]),
        "main",
        "the worktree stays where it was"
    );
    session.close_and_wait().await;
}

/// `ls-remote` matches a bare name against the tail of a ref, so the check
/// only means anything when it asks for the whole path.
#[tokio::test]
async fn the_remote_branch_check_answers_for_the_exact_name_only() {
    let (_bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();

    work.git(&["switch", "-c", "feature/topic"]);
    work.commit_file("b.txt", "b\n", "topic work");
    work.git(&["push", "origin", "feature/topic"]);

    assert!(
        remote::branch_tip(&exec, &work.path, "origin", "feature/topic", NET, &cancel)
            .await
            .expect("ask")
            .is_some()
    );
    assert!(
        remote::branch_tip(&exec, &work.path, "origin", "topic", NET, &cancel)
            .await
            .expect("ask")
            .is_none(),
        "`topic` is not taken just because `feature/topic` is"
    );
    assert!(
        remote::branch_tip(&exec, &work.path, "origin", "feature", NET, &cancel)
            .await
            .expect("ask")
            .is_none()
    );
}

/// A name taken by commits this history never had is refused outright, so
/// "taken" alone cannot decide the UI's move: the commit `branch_tip` hands
/// back tells the two cases apart before anything is sent.
#[tokio::test]
async fn a_taken_name_holding_commits_of_its_own_is_refused() {
    let (mut bare, mut work) = origin_and_clone();
    let (exec, cancel) = env();

    let theirs = a_branch_over_there(&mut work, "theirs");
    work.git(&["switch", "-c", "ours"]);
    work.commit_file("c.txt", "ours\n", "ours");

    let tip = remote::branch_tip(&exec, &work.path, "origin", "theirs", NET, &cancel)
        .await
        .expect("ask")
        .expect("the name is taken");
    assert!(
        !commit::is_in_history(&exec, &work.path, &tip, "HEAD", &cancel)
            .await
            .expect("compare"),
        "their commit is not one of ours, which is what makes this refusable"
    );

    let spec = remote::publish_spec("ours", "origin", "theirs", "");
    let sent = remote::push(&exec, &work.path, &spec, NET, &cancel).await;
    assert!(
        sent.as_ref().is_err_and(GitError::is_outdated),
        "git turns a first push that is not a fast-forward down: {sent:?}"
    );
    assert_eq!(
        bare.git(&["rev-parse", "theirs"]),
        theirs,
        "and their branch is where it was"
    );

    // Only a leased overwrite can land here; a lease on a commit that is no
    // longer there is refused.
    let stale = remote::publish_spec("ours", "origin", "theirs", &"0".repeat(40));
    assert!(
        remote::push(&exec, &work.path, &stale, NET, &cancel)
            .await
            .is_err(),
        "a lease against a commit the remote does not hold is refused"
    );
    assert_eq!(bare.git(&["rev-parse", "theirs"]), theirs);

    let leased = remote::publish_spec("ours", "origin", "theirs", &theirs);
    remote::push(&exec, &work.path, &leased, NET, &cancel)
        .await
        .expect("the overwrite the question offers, pinned to what it showed");
    assert_ne!(
        bare.git(&["rev-parse", "theirs"]),
        theirs,
        "their branch now carries ours instead"
    );
}

/// What git itself does behind the fixed command lines these functions hand
/// it (the lines are unit-tested beside `remote::list` and `remote::fetch`).
/// What each reads of git's answer is held above and by `remote::refusal`'s
/// own tests, so only the full gate runs these.
mod periodic {
    use super::*;

    #[tokio::test]
    #[ignore = "git's own tracking-ref cleanup: not worth the pre-merge run"]
    async fn deleting_a_remote_branch_prunes_on_the_next_fetch() {
        let (mut bare, mut work) = origin_and_clone();
        let (exec, cancel) = env();

        work.git(&["checkout", "-b", "temp"]);
        work.commit_file("t.txt", "t\n", "temp work");
        remote::push(
            &exec,
            &work.path,
            &PushSpec {
                remote: "origin".into(),
                local: "temp".into(),
                remote_branch: "temp".into(),
                set_upstream: false,
                force: PushForce::None,
            },
            NET,
            &cancel,
        )
        .await
        .expect("push temp");
        assert!(bare.git(&["branch", "--list"]).contains("temp"));
        let tip = work.git(&["rev-parse", "temp"]);

        remote::delete_remote_branch(&exec, &work.path, "origin", "temp", &tip, NET, &cancel)
            .await
            .expect("delete remote branch");
        assert!(!bare.git(&["branch", "--list"]).contains("temp"));

        remote::fetch(&exec, &work.path, Some("origin"), NET, &cancel)
            .await
            .expect("fetch --prune");
        assert!(
            !work
                .git(&["branch", "-r", "--list"])
                .contains("origin/temp"),
            "--prune dropped the stale remote-tracking ref"
        );
    }

    /// Why `Delete both` cannot run the remote half first: `push --delete`
    /// takes the remote-tracking ref with it, and a branch merged only into
    /// that upstream is then refused by the plain `branch --delete` —
    /// leaving the remote gone and the branch here.
    #[tokio::test]
    #[ignore = "git's own merged check behind a fixed order: not worth the pre-merge run"]
    async fn a_branch_whose_upstream_went_first_is_refused_as_not_merged() {
        let (_bare, mut work) = origin_and_clone();
        let (exec, cancel) = env();
        let shown = a_branch_over_there(&mut work, "topic");

        remote::delete_remote_branch(&exec, &work.path, "origin", "topic", &shown, NET, &cancel)
            .await
            .expect("the remote half goes");
        assert!(!work.git_ok(&["rev-parse", "--verify", "refs/remotes/origin/topic"]));

        let err = platitude_core::branch::delete(&exec, &work.path, "topic", false, &cancel)
            .await
            .expect_err("merged into an upstream that is no longer here");
        assert!(err.to_string().contains("not fully merged"), "{err}");
    }

    /// Adding a remote is bookkeeping: a URL that goes nowhere is
    /// accepted, which is why a failed push leaves the remote in place and
    /// `set-url` is the way back.
    #[tokio::test]
    #[ignore = "what git records behind a fixed command line: not worth the pre-merge run"]
    async fn adding_a_remote_records_the_url_without_reaching_it() {
        let (bare, mut work) = origin_and_clone();
        let (exec, cancel) = env();

        let nowhere = "file:///nowhere/there-is-no-such-repository.git";
        remote::add(&exec, &work.path, "fork", nowhere, &cancel)
            .await
            .expect("add a remote nothing answers for");
        assert_eq!(work.git(&["remote", "get-url", "fork"]), nowhere);

        let again = remote::add(&exec, &work.path, "fork", nowhere, &cancel)
            .await
            .expect_err("git keeps its own names unique");
        assert!(matches!(again, GitError::Failed { .. }), "{again:?}");

        remote::set_url(&exec, &work.path, "fork", &bare.file_url(), &cancel)
            .await
            .expect("correct the URL");
        assert_eq!(work.git(&["remote", "get-url", "fork"]), bare.file_url());

        let spec = remote::publish_spec("main", "fork", "main", "");
        remote::push(&exec, &work.path, &spec, NET, &cancel)
            .await
            .expect("push to the corrected remote");
    }

    /// The remote stays after its first push fails: undoing the add would
    /// throw away the only part of the answer worth keeping.
    #[tokio::test]
    #[ignore = "what git keeps after a failed push: not worth the pre-merge run"]
    async fn a_push_to_a_remote_that_goes_nowhere_leaves_the_remote_behind() {
        let (_bare, mut work) = origin_and_clone();
        let (exec, cancel) = env();

        let nowhere = "file:///nowhere/there-is-no-such-repository.git";
        remote::add(&exec, &work.path, "fork", nowhere, &cancel)
            .await
            .expect("add");
        let spec = remote::publish_spec("main", "fork", "main", "");
        let error = remote::push(&exec, &work.path, &spec, NET, &cancel)
            .await
            .expect_err("nothing answers there");
        assert!(matches!(error, GitError::Failed { .. }), "{error:?}");

        assert_eq!(
            work.git(&["remote", "get-url", "fork"]),
            nowhere,
            "the remote is still here to be corrected"
        );
        assert_eq!(
            // `--default` so an unset key is an empty answer; the exit
            // code would read to the harness as a broken command.
            work.git(&["config", "--default", "", "--get", "branch.main.remote"]),
            "",
            "and a push that never landed recorded no upstream"
        );
    }

    #[tokio::test]
    #[ignore = "what git lists behind a fixed --get-regexp, the pushurl key included: not worth the pre-merge run"]
    async fn lists_remotes_from_config() {
        let mut repo = TestRepo::init();
        repo.commit_file("a.txt", "one\n", "root");
        let (exec, cancel) = env();

        assert!(
            remote::list(&exec, &repo.path, &cancel)
                .await
                .expect("list")
                .is_empty(),
            "a fresh repository has no remotes"
        );

        repo.git(&["remote", "add", "origin", "file:///tmp/a"]);
        repo.git(&["remote", "add", "upstream", "file:///tmp/b"]);
        repo.git(&["remote", "set-url", "--push", "origin", "file:///tmp/push"]);

        let remotes = remote::list(&exec, &repo.path, &cancel)
            .await
            .expect("list");
        assert_eq!(remotes.len(), 2);
        assert_eq!(remotes[0].name, "origin");
        assert_eq!(remotes[0].fetch_url, "file:///tmp/a");
        assert_eq!(remotes[0].push_url, "file:///tmp/push");
        assert_eq!(remotes[1].name, "upstream");
    }

    #[tokio::test]
    #[ignore = "git writing both upstream keys behind --set-upstream: not worth the pre-merge run"]
    async fn publishing_sends_the_branch_and_records_the_upstream() {
        let (mut bare, mut work) = origin_and_clone();
        let (exec, cancel) = env();

        work.git(&["switch", "-c", "topic"]);
        work.commit_file("b.txt", "b\n", "topic work");

        let spec = remote::publish_spec("topic", "origin", "topic", "");
        assert!(spec.set_upstream, "the answer is recorded, not re-asked");
        remote::push(&exec, &work.path, &spec, NET, &cancel)
            .await
            .expect("publish");

        assert_eq!(
            bare.git(&["rev-parse", "topic"]),
            work.git(&["rev-parse", "topic"]),
            "the branch exists over there, at our tip"
        );
        assert_eq!(
            work.git(&["config", "branch.topic.merge"]),
            "refs/heads/topic"
        );
        assert_eq!(work.git(&["config", "branch.topic.remote"]), "origin");
    }

    /// Why the UI asks before a first push onto a taken name: git does not
    /// refuse a fast-forward, so somebody else's branch quietly moves and we
    /// end up tracking it.
    #[tokio::test]
    #[ignore = "git taking a first push that fast-forwards a taken name: not worth the pre-merge run"]
    async fn a_first_push_onto_a_taken_name_is_not_refused_when_it_fast_forwards() {
        let (mut bare, mut work) = origin_and_clone();
        let (exec, cancel) = env();

        // `shared` exists on the remote already, one commit behind us, and
        // nothing here records that it does.
        work.git(&["switch", "-c", "shared"]);
        work.commit_file("b.txt", "b\n", "theirs");
        work.git(&["push", "origin", "shared"]);
        let theirs = bare.git(&["rev-parse", "shared"]);
        work.commit_file("b.txt", "b2\n", "ours");

        let tip = remote::branch_tip(&exec, &work.path, "origin", "shared", NET, &cancel)
            .await
            .expect("ask")
            .expect("the question the UI puts to the remote before it pushes");
        assert!(
            commit::is_in_history(&exec, &work.path, &tip, "HEAD", &cancel)
                .await
                .expect("compare"),
            "what makes this the fast-forward case: their commit is one of ours"
        );

        let spec = remote::publish_spec("shared", "origin", "shared", "");
        remote::push(&exec, &work.path, &spec, NET, &cancel)
            .await
            .expect("git takes it — this is the hole the question covers");

        assert_ne!(
            bare.git(&["rev-parse", "shared"]),
            theirs,
            "their branch moved to our commit without git objecting"
        );
    }

    /// A user's `push.followTags` would send every annotated tag on the pushed
    /// commits beside the branch, and a far side turning those away fails the
    /// push with the branch landed (`[remote rejected]` on the tag's line,
    /// exit 1). Every push sends the one ref asked for.
    #[tokio::test]
    #[ignore = "git's push.followTags behind the fixed --no-follow-tags: not worth the pre-merge run"]
    async fn a_push_sends_the_branch_alone_whatever_follow_tags_says() {
        let (mut bare, mut work) = origin_and_clone();
        let (exec, cancel) = env();
        bare.write_hook(
            "update",
            "case \"$1\" in refs/tags/*) echo \"error: no tags here\" >&2; exit 1;; esac\n",
        );
        work.git(&["config", "push.followTags", "true"]);
        work.commit_file("b.txt", "ours\n", "work of our own");
        work.git(&["tag", "-a", "rel2", "-m", "release two"]);

        remote::push(
            &exec,
            &work.path,
            &PushSpec {
                remote: "origin".into(),
                local: "main".into(),
                remote_branch: "main".into(),
                set_upstream: false,
                force: PushForce::None,
            },
            NET,
            &cancel,
        )
        .await
        .expect("the branch is all that goes, and it lands");
        assert_eq!(
            bare.git(&["rev-parse", "refs/heads/main"]),
            work.git(&["rev-parse", "HEAD"])
        );
        assert!(
            !bare.git_ok(&["rev-parse", "--verify", "--quiet", "refs/tags/rel2"]),
            "the tag stays here"
        );
    }
}
