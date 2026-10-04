//! The git side: `refs/heads/main` moves only onto a stamped commit,
//! whichever git moves it, except the user's own.

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
    // A reset to where main already stands still runs the hook; it must
    // pass with no stamp.
    sb.git_ok(&sb.repo, &["reset", "-q", "--hard", "HEAD"]);

    // The manual test-skip control, which the Claude hook denies to sessions.
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

/// Without the session mark (`CLAUDECODE`) main moves with no stamp and
/// no skip flag: that git is the user's, and an IDE's has no cargo on
/// PATH to reach a verdict with.
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

/// The hook writes down whose git created each branch. A creation counts
/// only when the branch was not there before, and the record goes only once
/// the branch is gone: git also runs the hook when it packs refs, as a
/// creation in packed-refs and a deletion of the loose file. A creation by a
/// git with no session id leaves the name nobody's.
#[test]
fn the_hook_writes_down_which_session_made_a_branch() {
    let sb = Sandbox::new("hook-made");
    let creator = |branch: &str| {
        std::fs::read_to_string(sb.repo.join(".git/pgg-gate/made/refs/heads").join(branch))
            .ok()
            .map(|id| id.trim().to_string())
    };
    let make = |branch: &str, env: &[(&str, &str)]| {
        sb.git(&sb.repo, &["branch", branch, "main"], env)
            .unwrap_or_else(|e| panic!("{branch}: {e}"));
    };
    let mine = [("CLAUDE_CODE_SESSION_ID", "mine")];
    let theirs = [("CLAUDE_CODE_SESSION_ID", "theirs")];
    make("keep/copy", &mine);
    make("marker", &theirs);
    make("nobodys", &[]);
    make("from-codex", &[("CODEX_THREAD_ID", "thread-1")]);
    assert_eq!(creator("keep/copy").as_deref(), Some("mine"));
    assert_eq!(creator("marker").as_deref(), Some("theirs"));
    assert_eq!(creator("nobodys"), None);
    assert_eq!(creator("from-codex").as_deref(), Some("thread-1"));

    // Another session's git packs every ref: nothing changes hands.
    sb.git(
        &sb.repo,
        &["pack-refs", "--all"],
        &[("CLAUDE_CODE_SESSION_ID", "theirs"), ("PGG_GATE_SKIP", "1")],
    )
    .expect("the refs packed");
    assert!(
        !sb.repo.join(".git/refs/heads/keep/copy").exists(),
        "the branch was packed, not left loose"
    );
    assert_eq!(creator("keep/copy").as_deref(), Some("mine"));
    assert_eq!(creator("nobodys"), None);

    // Deleted, the record goes; made again, it is the new maker's.
    sb.git(&sb.repo, &["branch", "-D", "keep/copy"], &mine)
        .expect("given up");
    assert_eq!(creator("keep/copy"), None);
    make("keep/copy", &theirs);
    assert_eq!(creator("keep/copy").as_deref(), Some("theirs"));
    sb.git(&sb.repo, &["branch", "-D", "marker"], &[])
        .expect("deleted by the user");
    make("marker", &[]);
    assert_eq!(creator("marker"), None, "the user's own name is nobody's");
}
