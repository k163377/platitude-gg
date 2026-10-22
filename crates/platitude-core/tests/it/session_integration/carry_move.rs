//! Moving off a dirty tree: what the move carries across, and what the
//! stash keeps hold of when it cannot. git's own refusal of a plain move
//! while an operation stands is in [`periodic`].

use crate::support::TestRepo;
use crate::support::session::{opened, write_result};
use platitude_core::OperationKind;
use platitude_core::session::SessionEvent;

/// A move still refused once the tree has been emptied puts the work back,
/// and git's first refusal is what comes out.
#[tokio::test(flavor = "multi_thread")]
async fn a_move_nothing_can_unblock_puts_the_stashed_work_back() {
    let mut repo = TestRepo::init();
    repo.commit_file("x.txt", "base\n", "root");
    repo.git(&["switch", "-c", "other"]);
    repo.commit_file("x.txt", "theirs\n", "other");
    repo.git(&["switch", "main"]);
    // Hidden from status and the stash, yet git will not write over it.
    repo.write_file("x.txt", "mine\n");
    repo.git(&["update-index", "--skip-worktree", "--", "x.txt"]);
    repo.write_file("left.txt", "mine too\n");

    let (sink, session) = opened(&repo).await;
    session.checkout(platitude_core::branch::CheckoutTarget::Branch {
        name: "other".into(),
    });
    let error = write_result(&sink, OperationKind::Checkout).await;
    assert!(
        error.is_some_and(|e| e.contains("would be overwritten")),
        "git's first refusal is the one worth reporting"
    );

    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "main");
    assert!(
        repo.git(&["stash", "list"]).is_empty(),
        "the entry went back where it came from"
    );
    assert_eq!(
        std::fs::read_to_string(repo.path.join("left.txt")).unwrap(),
        "mine too\n",
        "the work is where it was before the refused move"
    );
    session.close_and_wait().await;
}

/// Two branches that disagree about `both.txt`, HEAD on `main`.
fn colliding_branches() -> TestRepo {
    let mut repo = TestRepo::init();
    repo.commit_file("both.txt", "base\n", "root");
    repo.git(&["switch", "-c", "other"]);
    repo.commit_file("both.txt", "theirs\n", "other");
    repo.git(&["switch", "main"]);
    repo
}

/// A move git refuses is made the long way round, with nothing asked:
/// stash, switch, put back (デザイン規約 §未コミット変更がある状態での移動).
#[tokio::test(flavor = "multi_thread")]
async fn a_move_git_refuses_goes_round_through_a_stash() {
    let mut repo = TestRepo::init();
    repo.commit_file("both.txt", "l1\nl2\nl3\nl4\nl5\n", "root");
    repo.git(&["switch", "-c", "other"]);
    repo.commit_file("both.txt", "l1-THEIRS\nl2\nl3\nl4\nl5\n", "other");
    repo.git(&["switch", "main"]);
    // Collides with `other`, so the plain switch is refused — but on
    // another line, so the restore merges cleanly.
    repo.write_file("both.txt", "l1\nl2\nl3\nl4\nl5-MINE\n");

    let (sink, session) = opened(&repo).await;
    session.checkout(platitude_core::branch::CheckoutTarget::Branch {
        name: "other".into(),
    });
    assert_eq!(write_result(&sink, OperationKind::Checkout).await, None);

    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "other");
    assert_eq!(
        std::fs::read_to_string(repo.path.join("both.txt")).unwrap(),
        "l1-THEIRS\nl2\nl3\nl4\nl5-MINE\n",
        "both sides of the file survived"
    );
    assert!(
        repo.git(&["stash", "list"]).is_empty(),
        "a clean restore takes the stash with it"
    );
    session.close_and_wait().await;
}

/// When the sides cannot be combined, git leaves the markers and keeps the
/// stash, and both are the answer asked for: the work is across, waiting
/// to be settled, and still recoverable from the stash.
#[tokio::test(flavor = "multi_thread")]
async fn a_conflicting_carry_leaves_the_stash_as_the_way_back() {
    let mut repo = colliding_branches();
    repo.write_file("both.txt", "mine\n");
    repo.git(&["add", "--", "both.txt"]);

    let (sink, session) = opened(&repo).await;
    session.checkout(platitude_core::branch::CheckoutTarget::Branch {
        name: "other".into(),
    });
    assert_eq!(
        write_result(&sink, OperationKind::Checkout).await,
        None,
        "a conflict is the outcome that was asked for, not an error"
    );

    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "other");
    let both = std::fs::read_to_string(repo.path.join("both.txt")).unwrap();
    assert!(both.contains("<<<<<<<") && both.contains("mine"), "{both}");
    assert!(
        repo.git(&["status", "--porcelain=v2"]).contains("u UU"),
        "left unmerged for the merge tool"
    );
    assert_eq!(
        repo.git(&["stash", "list"]).lines().count(),
        1,
        "the entry stays, so the work exists outside the marked-up file"
    );
    session.close_and_wait().await;
}

/// Somebody else's entry sits at `stash@{0}` when the move begins; the
/// carry may put back only its own, or unasked-for work lands in the tree.
#[tokio::test(flavor = "multi_thread")]
async fn a_carry_leaves_other_stashes_alone() {
    let mut repo = colliding_branches();
    repo.write_file("both.txt", "parked work\n");
    repo.git(&["stash", "push", "-m", "parked"]);
    repo.write_file("both.txt", "mine\n");

    let (sink, session) = opened(&repo).await;
    session.checkout(platitude_core::branch::CheckoutTarget::Branch {
        name: "other".into(),
    });
    assert_eq!(write_result(&sink, OperationKind::Checkout).await, None);

    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "other");
    let list = repo.git(&["stash", "list"]);
    assert!(
        list.contains("parked"),
        "the parked entry was not ours to pop: {list}"
    );
    session.close_and_wait().await;
}

/// The untracked file the target tracks has nowhere to go, but every
/// tracked change travels anyway, and the entry stays as the way back to
/// the one that did not.
#[tokio::test(flavor = "multi_thread")]
async fn a_carry_that_cannot_restore_reports_gits_message() {
    let mut repo = TestRepo::init();
    repo.commit_file("seed.txt", "seed\n", "root");
    repo.git(&["switch", "-c", "other"]);
    repo.commit_file("theirs.txt", "only over there\n", "other");
    repo.git(&["switch", "main"]);
    repo.write_file("theirs.txt", "mine, uncommitted\n");
    repo.write_file("seed.txt", "seed\nand a tracked edit\n");

    let (sink, session) = opened(&repo).await;
    session.checkout(platitude_core::branch::CheckoutTarget::Branch {
        name: "other".into(),
    });
    let error = write_result(&sink, OperationKind::Checkout).await;
    assert!(
        error.is_some_and(|e| e.contains("untracked")),
        "git's own wording goes through"
    );
    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "other");
    assert_eq!(
        std::fs::read_to_string(repo.path.join("seed.txt")).unwrap(),
        "seed\nand a tracked edit\n",
        "the tracked edit came across regardless"
    );
    assert_eq!(
        repo.git(&["stash", "list"]).lines().count(),
        1,
        "the entry is the way back to the file that stayed behind"
    );
    session.close_and_wait().await;
}

/// A conflicting pop lands where an apply would have: git keeps the entry
/// (デザイン規約 §変更を退避する).
#[tokio::test(flavor = "multi_thread")]
async fn a_conflicting_pop_keeps_the_entry_and_is_not_a_failure() {
    let mut repo = colliding_branches();
    repo.write_file("both.txt", "mine\n");
    repo.git(&["stash", "push", "-u"]);
    repo.git(&["switch", "other"]);

    let (sink, session) = opened(&repo).await;
    session.stash_pop("stash@{0}".into());
    assert_eq!(
        write_result(&sink, OperationKind::Stash).await,
        None,
        "the restore landed; it just needs settling"
    );

    let both = std::fs::read_to_string(repo.path.join("both.txt")).unwrap();
    assert!(both.contains("<<<<<<<") && both.contains("mine"), "{both}");
    assert!(
        repo.git(&["status", "--porcelain=v2"]).contains("u UU"),
        "left unmerged to be settled"
    );
    assert_eq!(
        repo.git(&["stash", "list"]).lines().count(),
        1,
        "kept, the way an apply would have kept it"
    );
    session.close_and_wait().await;
}

/// git refuses to restore onto an index that already has unmerged paths,
/// and the conflicts left standing are the old ones — so the tree after
/// cannot judge the pop; the tree before decides.
#[tokio::test(flavor = "multi_thread")]
async fn a_pop_refused_by_a_conflicted_tree_is_still_a_failure() {
    let mut repo = colliding_branches();
    repo.git(&["switch", "-c", "mine", "main"]);
    repo.commit_file("both.txt", "ours\n", "mine");
    // An entry that has nothing to do with the conflict below.
    repo.write_file("spare.txt", "parked\n");
    repo.git(&["stash", "push", "-u"]);
    repo.git_expect_failure(&["merge", "other"]);
    assert!(
        repo.git(&["status", "--porcelain=v2"]).contains("u UU"),
        "the merge stopped on a conflict"
    );

    let (sink, session) = opened(&repo).await;
    session.stash_pop("stash@{0}".into());
    let error = write_result(&sink, OperationKind::Stash).await;
    assert!(
        error.is_some(),
        "the refusal is not read as a landed conflict"
    );
    assert_eq!(
        repo.git(&["stash", "list"]).lines().count(),
        1,
        "nothing was restored, so nothing was dropped"
    );
    assert!(!repo.path.join("spare.txt").exists(), "still in the entry");
    session.close_and_wait().await;
}

/// Read the way a conflicting pop is; the entry was staying either way.
#[tokio::test(flavor = "multi_thread")]
async fn a_conflicting_apply_is_not_a_failure_either() {
    let mut repo = colliding_branches();
    repo.write_file("both.txt", "mine\n");
    repo.git(&["stash", "push", "-u"]);
    repo.git(&["switch", "other"]);

    let (sink, session) = opened(&repo).await;
    session.stash_apply("stash@{0}".into());
    assert_eq!(
        write_result(&sink, OperationKind::Stash).await,
        None,
        "the restore landed; it just needs settling"
    );

    let both = std::fs::read_to_string(repo.path.join("both.txt")).unwrap();
    assert!(both.contains("<<<<<<<") && both.contains("mine"), "{both}");
    assert!(
        repo.git(&["status", "--porcelain=v2"]).contains("u UU"),
        "left unmerged to be settled"
    );
    assert_eq!(repo.git(&["stash", "list"]).lines().count(), 1);
    session.close_and_wait().await;
}

/// Moving out of a stopped cherry-pick is one write: `--quit` puts the
/// operation down keeping every commit an earlier step made, the tree
/// goes into a stash, and the move lands. Nothing is undone — that is the
/// `--abort` still on the exit card (デザイン規約 §進行中の操作から出る).
#[tokio::test(flavor = "multi_thread")]
async fn a_move_out_of_a_stopped_pick_puts_it_in_a_stash() {
    let mut repo = colliding_branches();
    repo.commit_file("both.txt", "ours\n", "main");
    let before = repo.git(&["rev-parse", "HEAD"]);
    repo.git_expect_failure(&["cherry-pick", "other"]);
    assert!(
        repo.git(&["status", "--porcelain=v2"]).contains("u UU"),
        "the pick stopped on a conflict"
    );

    let (sink, session) = opened(&repo).await;
    session.checkout_leaving_operation(platitude_core::branch::CheckoutTarget::Branch {
        name: "other".into(),
    });
    assert_eq!(write_result(&sink, OperationKind::Checkout).await, None);
    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "other");
    assert_eq!(
        repo.git(&["status", "--porcelain=v2"]),
        "",
        "the tree went into the stash, so nothing is in the move's way"
    );
    assert_eq!(
        repo.git(&["stash", "list"]).lines().count(),
        1,
        "and it is still somewhere: the entry is not popped at the far end"
    );
    assert_eq!(
        repo.git(&["rev-parse", "main"]),
        before,
        "the branch it was on is where it was — nothing was rewound"
    );
    assert!(
        !repo.path.join(".git/sequencer").exists(),
        "nothing of the pick is left for the next command to trip over"
    );
    session.close_and_wait().await;
}

/// The same move out of a stopped rebase aborts instead: `rebase --quit`
/// leaves HEAD detached on the half-rewritten line with every copy made so
/// far unreferenced, while `--abort` puts the branch back, and a replay is
/// not work in hand.
#[tokio::test(flavor = "multi_thread")]
async fn a_move_out_of_a_stopped_rebase_puts_the_branch_back() {
    let mut repo = colliding_branches();
    repo.commit_file("both.txt", "ours\n", "mine");
    let before = repo.git(&["rev-parse", "HEAD"]);
    repo.git_expect_failure(&["rebase", "other"]);

    let (sink, session) = opened(&repo).await;
    session.checkout_leaving_operation(platitude_core::branch::CheckoutTarget::Branch {
        name: "other".into(),
    });
    assert_eq!(write_result(&sink, OperationKind::Checkout).await, None);
    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "other");
    assert_eq!(
        repo.git(&["rev-parse", "main"]),
        before,
        "the branch the rebase was rewriting is back where it started"
    );
    assert_eq!(
        repo.git(&["stash", "list"]),
        "",
        "an abort leaves nothing to carry, so no entry is made"
    );
    session.close_and_wait().await;
}

/// With no operation standing (a reader ran `cherry-pick --quit`), the
/// unmerged index alone is in the way, and the same two steps clear it.
#[tokio::test(flavor = "multi_thread")]
async fn a_move_out_of_unmerged_files_stashes_them_with_no_operation_to_put_down() {
    let mut repo = colliding_branches();
    repo.commit_file("both.txt", "ours\n", "main");
    repo.git_expect_failure(&["cherry-pick", "other"]);
    repo.git(&["cherry-pick", "--quit"]);
    assert!(
        repo.git(&["status", "--porcelain=v2"]).contains("u UU"),
        "the operation is gone and the conflict is not"
    );

    let (sink, session) = opened(&repo).await;
    session.checkout_leaving_operation(platitude_core::branch::CheckoutTarget::Branch {
        name: "other".into(),
    });
    assert_eq!(write_result(&sink, OperationKind::Checkout).await, None);
    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "other");
    assert_eq!(repo.git(&["status", "--porcelain=v2"]), "");
    assert_eq!(repo.git(&["stash", "list"]).lines().count(), 1);
    session.close_and_wait().await;
}

/// With nothing standing and nothing colliding, git carries the work
/// across itself.
#[tokio::test(flavor = "multi_thread")]
async fn nothing_in_the_way_means_nothing_is_put_aside() {
    let mut repo = colliding_branches();
    // On neither branch, so nothing it could collide with.
    repo.write_file("spare.txt", "uncommitted\n");

    let (sink, session) = opened(&repo).await;
    session.checkout_leaving_operation(platitude_core::branch::CheckoutTarget::Branch {
        name: "other".into(),
    });
    assert_eq!(write_result(&sink, OperationKind::Checkout).await, None);
    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "other");
    assert_eq!(
        repo.git(&["stash", "list"]),
        "",
        "nothing was in the way, so nothing was put aside"
    );
    assert_eq!(
        std::fs::read_to_string(repo.path.join("spare.txt")).unwrap(),
        "uncommitted\n",
        "and the work came with it"
    );
    session.close_and_wait().await;
}

/// Walking away from `Move here?` costs the cherry-pick nothing.
#[tokio::test(flavor = "multi_thread")]
async fn a_move_that_has_to_ask_leaves_the_operation_standing() {
    let mut repo = colliding_branches();
    // `other` holds a commit `main` does not, so moving it onto `main`
    // would put that one out of reach: the case the question is for.
    repo.commit_file("both.txt", "ours\n", "main");
    repo.git_expect_failure(&["cherry-pick", "other"]);

    let (sink, session) = opened(&repo).await;
    session.checkout_moving_branch("other".into(), "main".into(), true);
    assert_eq!(write_result(&sink, OperationKind::Checkout).await, None);
    assert!(
        sink.count(|e| matches!(
            e,
            platitude_core::session::SessionEvent::MoveNeedsAsk { .. }
        )) > 0,
        "the move came back as a question"
    );
    assert!(
        repo.git(&["status", "--porcelain=v2"]).contains("u UU"),
        "and the pick it would have undone is still standing"
    );
    session.close_and_wait().await;
}

/// The question says how many commits the move takes off the branch — the
/// branch's own, whatever a tag of the same name points at.
#[tokio::test(flavor = "multi_thread")]
async fn a_move_that_has_to_ask_counts_what_the_branch_loses() {
    let mut repo = colliding_branches();
    repo.git(&["switch", "other"]);
    repo.commit_file("more.txt", "more\n", "other again");
    repo.git(&["switch", "main"]);
    repo.git(&["tag", "other", "main"]);

    let (sink, session) = opened(&repo).await;
    session.checkout_moving_branch("other".into(), "main".into(), false);
    assert_eq!(write_result(&sink, OperationKind::Checkout).await, None);
    assert_eq!(
        sink.count(|e| matches!(
            e,
            platitude_core::session::SessionEvent::MoveNeedsAsk { lost: 2, .. }
        )),
        1,
        "the move came back as a question naming the two commits it would take off"
    );
    session.close_and_wait().await;
}

/// A branch that fell behind the start takes nothing off it: moved and
/// landed on, nothing asked.
#[tokio::test(flavor = "multi_thread")]
async fn a_move_that_takes_nothing_off_the_branch_lands_on_it() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "root");
    repo.git(&["branch", "behind"]);
    let ahead = repo.commit_file_id("a.txt", "2\n", "ahead");

    let (sink, session) = opened(&repo).await;
    session.checkout_moving_branch("behind".into(), "main".into(), false);
    assert_eq!(write_result(&sink, OperationKind::Checkout).await, None);
    assert_eq!(
        sink.count(|e| matches!(e, SessionEvent::MoveNeedsAsk { .. })),
        0,
        "nothing to ask about"
    );
    assert_eq!(
        (
            repo.git(&["rev-parse", "behind"]),
            repo.git(&["rev-parse", "--abbrev-ref", "HEAD"])
        ),
        (ahead, "behind".to_string())
    );
    session.close_and_wait().await;
}

/// A carry whose session closes half-way finishes anyway, so the work it
/// emptied the tree of is put back — read from the repository, since
/// nobody is watching events (rules-refs/core.md「ローカル書き込みは走り切る」).
///
/// Held at the stash: between it and the `switch` is the one stretch where
/// a stopped write would leave the work somewhere other than the tree.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_carry_whose_session_closes_half_way_still_puts_the_work_back() {
    let mut repo = TestRepo::init();
    repo.commit_file("both.txt", "l1\nl2\nl3\nl4\nl5\n", "root");
    repo.git(&["switch", "-c", "other"]);
    repo.commit_file("both.txt", "l1-THEIRS\nl2\nl3\nl4\nl5\n", "other");
    repo.git(&["switch", "main"]);
    repo.write_file("both.txt", "l1\nl2\nl3\nl4\nl5-MINE\n");

    let (sink, session) = opened(&repo).await;
    let (release, held) = std::sync::mpsc::channel::<()>();
    sink.hook_once(reached_the_stash, move || {
        held.recv().expect("the test releases the carry");
    });
    session.checkout(platitude_core::branch::CheckoutTarget::Branch {
        name: "other".into(),
    });
    sink.wait_for("the carry reaching its stash", |events| {
        events.iter().any(reached_the_stash).then_some(())
    })
    .await;
    session.close();
    release.send(()).expect("the carry is released");

    assert_eq!(write_result(&sink, OperationKind::Checkout).await, None);
    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "other");
    assert!(
        repo.git(&["stash", "list"]).is_empty(),
        "nothing was left in the stash for anybody to find later"
    );
    assert_eq!(
        std::fs::read_to_string(repo.path.join("both.txt")).unwrap(),
        "l1-THEIRS\nl2\nl3\nl4\nl5-MINE\n",
        "and the work is back in the tree the move landed in"
    );
    session.close_and_wait().await;
}

/// The stash a carry makes on its way round, as the sink announces it.
fn reached_the_stash(event: &SessionEvent) -> bool {
    matches!(event, SessionEvent::CommandStarted { display, .. }
        if display.starts_with("git stash push"))
}

/// A move that fails for a reason a stash cannot help with (a name git
/// rejects) stops there: nothing is stashed.
#[tokio::test(flavor = "multi_thread")]
async fn a_move_that_fails_outright_stashes_nothing() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    repo.write_file("f.txt", "uncommitted\n");

    let (sink, session) = opened(&repo).await;
    session.checkout(platitude_core::branch::CheckoutTarget::Branch {
        name: "no-such-branch".into(),
    });
    assert!(
        write_result(&sink, OperationKind::Checkout).await.is_some(),
        "git rejected the branch name"
    );
    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "main");
    assert_eq!(
        std::fs::read_to_string(repo.path.join("f.txt")).unwrap(),
        "uncommitted\n",
        "the work never left the tree"
    );
    assert_eq!(repo.git(&["stash", "list"]), "", "no entry left behind");
    session.close_and_wait().await;
}

/// Left out of the pre-merge run: the refusal and the untouched pick are
/// git's, and the one reading of ours in it (this wording is not one a
/// stash gets past) is held by `branch::tests::every_other_failure_stays_an_error`.
mod periodic {
    use super::*;

    /// A plain `checkout` (not `checkout_leaving_operation`) is still git's
    /// refusal: core does not abort anything nobody asked it to.
    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "git's own refusal of a move while a pick stands: not worth the pre-merge run"]
    async fn an_ordinary_move_still_refuses_while_a_pick_is_standing() {
        let mut repo = colliding_branches();
        repo.commit_file("both.txt", "ours\n", "main");
        repo.git_expect_failure(&["cherry-pick", "other"]);

        let (sink, session) = opened(&repo).await;
        session.checkout(platitude_core::branch::CheckoutTarget::Branch {
            name: "other".into(),
        });
        let error = write_result(&sink, OperationKind::Checkout).await;
        assert!(
            error.is_some_and(|e| e.contains("cherry-picking")),
            "git's own refusal comes through"
        );
        assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "main");
        assert!(
            repo.git(&["status", "--porcelain=v2"]).contains("u UU"),
            "the stopped pick is untouched"
        );
        session.close_and_wait().await;
    }
}
