//! Moving off a dirty tree: what the move carries across, and what the
//! stash keeps hold of when it cannot.

use crate::support::TestRepo;
use crate::support::session::{opened, write_result};

/// A move that fails for a reason a stash cannot help with — a name git
/// rejects — stops there: nothing is stashed, so the uncommitted work is
/// still in the tree where its owner left it.
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
        write_result(&sink, "checkout").await.is_some(),
        "git rejected the branch name"
    );
    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "main");
    assert_eq!(
        std::fs::read_to_string(repo.path.join("f.txt")).unwrap(),
        "uncommitted\n",
        "the work never left the tree"
    );
    assert_eq!(repo.git(&["stash", "list"]), "", "no entry left behind");
    session.close();
}

/// A move still refused once the tree has been emptied: whatever is
/// holding it is not something a stash gets past, so the work goes back
/// where it was and git's refusal is what comes out.
#[tokio::test(flavor = "multi_thread")]
async fn a_move_nothing_can_unblock_puts_the_stashed_work_back() {
    let mut repo = TestRepo::init();
    repo.commit_file("x.txt", "base\n", "root");
    repo.git(&["switch", "-c", "other"]);
    repo.commit_file("x.txt", "theirs\n", "other");
    repo.git(&["switch", "main"]);
    // Hidden from status and from the stash, but still in the move's way:
    // git will not write over what it was told to stop looking at.
    repo.write_file("x.txt", "mine\n");
    repo.git(&["update-index", "--skip-worktree", "--", "x.txt"]);
    repo.write_file("left.txt", "mine too\n");

    let (sink, session) = opened(&repo).await;
    session.checkout(platitude_core::branch::CheckoutTarget::Branch {
        name: "other".into(),
    });
    let error = write_result(&sink, "checkout").await;
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
    session.close();
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

/// The move git will not make itself is made the long way round instead,
/// with nothing asked: stash, switch, put back — the sequence a person
/// would type (デザイン規約 §未コミット変更がある状態での移動).
#[tokio::test(flavor = "multi_thread")]
async fn a_move_git_refuses_goes_round_through_a_stash() {
    let mut repo = TestRepo::init();
    repo.commit_file("both.txt", "l1\nl2\nl3\nl4\nl5\n", "root");
    repo.git(&["switch", "-c", "other"]);
    repo.commit_file("both.txt", "l1-THEIRS\nl2\nl3\nl4\nl5\n", "other");
    repo.git(&["switch", "main"]);
    // Collides with `other` (the file differs there), so the plain switch
    // is refused — but on another line, so the restore merges it cleanly.
    repo.write_file("both.txt", "l1\nl2\nl3\nl4\nl5-MINE\n");

    let (sink, session) = opened(&repo).await;
    session.checkout(platitude_core::branch::CheckoutTarget::Branch {
        name: "other".into(),
    });
    assert_eq!(write_result(&sink, "checkout").await, None);

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
    session.close();
}

/// Going round through the stash is what keeps the staged/unstaged split
/// — the reason the move is not `switch --merge`, which refuses outright
/// while anything is staged.
#[tokio::test(flavor = "multi_thread")]
async fn a_move_that_goes_round_keeps_what_was_staged_staged() {
    let mut repo = TestRepo::init();
    repo.commit_file("both.txt", "l1\nl2\nl3\nl4\nl5\n", "root");
    repo.git(&["switch", "-c", "other"]);
    repo.commit_file("both.txt", "l1-THEIRS\nl2\nl3\nl4\nl5\n", "other");
    repo.git(&["switch", "main"]);
    repo.write_file("both.txt", "l1\nl2\nl3\nl4\nl5-MINE\n");
    repo.write_file("staged.txt", "staged\n");
    repo.git(&["add", "--", "staged.txt"]);

    let (sink, session) = opened(&repo).await;
    session.checkout(platitude_core::branch::CheckoutTarget::Branch {
        name: "other".into(),
    });
    assert_eq!(write_result(&sink, "checkout").await, None);

    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "other");
    assert_eq!(
        std::fs::read_to_string(repo.path.join("both.txt")).unwrap(),
        "l1-THEIRS\nl2\nl3\nl4\nl5-MINE\n",
        "both sides of the file survived"
    );
    assert_eq!(
        repo.git(&["diff", "--cached", "--name-only"]),
        "staged.txt",
        "what was staged is staged still"
    );
    session.close();
}

/// The same move when the sides cannot be combined: git leaves the markers
/// and keeps the stash, and neither is a failure to report — the work is
/// across, waiting to be settled, and still recoverable from the stash.
/// This is the display a person typing the three commands would land on.
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
        write_result(&sink, "checkout").await,
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
    session.close();
}

/// Somebody else's entry sits at `stash@{0}` when the move begins. The one
/// this makes goes on top and is the only one it may put back — pop the
/// wrong one and work nobody asked about lands in the tree.
#[tokio::test(flavor = "multi_thread")]
async fn a_carry_leaves_other_stashes_alone() {
    let mut repo = colliding_branches();
    // Somebody's earlier work, parked before any of this.
    repo.write_file("both.txt", "parked work\n");
    repo.git(&["stash", "push", "-m", "parked"]);
    repo.write_file("both.txt", "mine\n");

    let (sink, session) = opened(&repo).await;
    session.checkout(platitude_core::branch::CheckoutTarget::Branch {
        name: "other".into(),
    });
    assert_eq!(write_result(&sink, "checkout").await, None);

    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "other");
    let list = repo.git(&["stash", "list"]);
    assert!(
        list.contains("parked"),
        "the parked entry was not ours to pop: {list}"
    );
    session.close();
}

/// A restore that really cannot land still reports. The untracked file the
/// target tracks has nowhere to go — but every tracked change travels
/// anyway, and the entry stays as the way back to the one that did not.
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
    let error = write_result(&sink, "checkout").await;
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
    session.close();
}

/// A pop whose restore conflicts lands exactly where an apply would have:
/// git keeps the entry, and the conflict is the outcome that was asked
/// for, not a failure to report (デザイン規約 §変更を退避する).
#[tokio::test(flavor = "multi_thread")]
async fn a_conflicting_pop_keeps_the_entry_and_is_not_a_failure() {
    let mut repo = colliding_branches();
    repo.write_file("both.txt", "mine\n");
    repo.git(&["stash", "push", "-u"]);
    repo.git(&["switch", "other"]);

    let (sink, session) = opened(&repo).await;
    session.stash_pop("stash@{0}".into());
    assert_eq!(
        write_result(&sink, "stash").await,
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
    session.close();
}

/// The same reading must not swallow a pop that did nothing. git refuses
/// to restore onto an index that already has unmerged paths, and the
/// conflicts standing there afterwards are the old ones — so the tree
/// alone cannot judge it, and what it was before decides.
#[tokio::test(flavor = "multi_thread")]
async fn a_pop_refused_by_a_conflicted_tree_is_still_a_failure() {
    let mut repo = colliding_branches();
    repo.git(&["switch", "-c", "mine", "main"]);
    repo.commit_file("both.txt", "ours\n", "mine");
    // An entry that has nothing to do with the conflict below.
    repo.write_file("spare.txt", "parked\n");
    repo.git(&["stash", "push", "-u"]);
    // A conflicting merge exits 1, which is the point of it.
    repo.git_expect_failure(&["merge", "other"]);
    assert!(
        repo.git(&["status", "--porcelain=v2"]).contains("u UU"),
        "the merge stopped on a conflict"
    );

    let (sink, session) = opened(&repo).await;
    session.stash_pop("stash@{0}".into());
    let error = write_result(&sink, "stash").await;
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
    session.close();
}
