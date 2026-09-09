//! The git side: `refs/heads/main` moves only onto a stamped commit,
//! whichever git moves it — and not at all for the user's own.

use crate::support::Sandbox;

#[test]
fn main_moves_only_onto_a_gated_commit_whatever_moves_it() {
    let sb = Sandbox::new("hook");
    sb.write_refs(&sb.seat, 6);
    let tip = sb.commit_all(&sb.seat, "feat(core): six", &[]);
    let main_before = sb.main_sha();

    let merge = sb.git(&sb.repo, &["merge", "--ff-only", "worktree-a"], &[]);
    assert!(
        merge.as_ref().is_err_and(|e| e.contains("no gate stamp")),
        "{merge:?}"
    );
    let update = sb.git(&sb.seat, &["update-ref", "refs/heads/main", &tip], &[]);
    assert!(
        update.as_ref().is_err_and(|e| e.contains("no gate stamp")),
        "{update:?}"
    );
    sb.write(&sb.repo, "internal-docs/notes.md", "# notes\n\ndirect\n");
    sb.git_ok(&sb.repo, &["add", "-A"]);
    let direct = sb.git(&sb.repo, &["commit", "-q", "-m", "docs: direct"], &[]);
    assert!(
        direct.as_ref().is_err_and(|e| e.contains("no gate stamp")),
        "{direct:?}"
    );
    assert_eq!(sb.main_sha(), main_before, "main did not move");
    // A reset to where main already stands writes the ref to its own
    // value; git runs the hook for it, and it must pass with no stamp.
    sb.git_ok(&sb.repo, &["reset", "-q", "--hard", "HEAD"]);

    // The user's own way past, which the Claude hook denies to sessions.
    sb.write(&sb.repo, "internal-docs/notes.md", "# notes\n\nskipped\n");
    sb.git_ok(&sb.repo, &["add", "-A"]);
    sb.git(
        &sb.repo,
        &["commit", "-q", "-m", "docs: skipped"],
        &[("PGG_GATE_SKIP", "1")],
    )
    .expect("PGG_GATE_SKIP lets the user through");
    let rewind = sb.git(&sb.repo, &["reset", "-q", "--hard", &main_before], &[]);
    assert!(
        rewind.as_ref().is_err_and(|e| e.contains("no gate stamp")),
        "{rewind:?}"
    );
    sb.git(
        &sb.repo,
        &["reset", "-q", "--hard", &main_before],
        &[("PGG_GATE_SKIP", "1")],
    )
    .expect("PGG_GATE_SKIP rewinds");

    sb.gate_ok(&sb.seat, &[]);
    sb.git_ok(&sb.repo, &["merge", "--ff-only", "worktree-a"]);
    assert_eq!(sb.main_sha(), tip);
}

/// The gate is held over sessions and over nobody else: without the mark
/// in the environment a direct commit on main and a rewind both go
/// through, with no stamp anywhere and no escape spelled. That git is
/// the user's — a terminal of their own, an IDE, a window they are
/// clicking in — and an IDE's has no cargo on PATH to reach a verdict
/// with either.
#[test]
fn the_users_own_git_moves_main_with_no_stamp() {
    let sb = Sandbox::new("user");
    let unmarked = [("CLAUDECODE", "")];
    let main_before = sb.main_sha();
    sb.write(&sb.repo, "internal-docs/notes.md", "# notes\n\nthe user\n");
    sb.git_ok(&sb.repo, &["add", "-A"]);
    let session = sb.git(&sb.repo, &["commit", "-q", "-m", "docs: a session"], &[]);
    assert!(
        session.as_ref().is_err_and(|e| e.contains("no gate stamp")),
        "{session:?}"
    );
    sb.git(
        &sb.repo,
        &["commit", "-q", "-m", "docs: the user"],
        &unmarked,
    )
    .expect("the user commits on their own main");
    assert_ne!(sb.main_sha(), main_before, "main moved for the user");
    sb.git(
        &sb.repo,
        &["reset", "-q", "--hard", &main_before],
        &unmarked,
    )
    .expect("the user rewinds their own main");
    assert_eq!(sb.main_sha(), main_before);
}

#[test]
fn install_leaves_somebody_elses_hooks_directory_alone() {
    let sb = Sandbox::new("foreign-hooks");
    sb.git_ok(&sb.repo, &["config", "core.hooksPath", "/somewhere/else"]);
    let (ok, text) = sb.gate(&sb.repo, &["install"], &[]);
    assert!(!ok && text.contains("not the gate's"), "{text}");
    assert_eq!(
        sb.git_ok(&sb.repo, &["config", "--get", "core.hooksPath"]),
        "/somewhere/else"
    );
}
