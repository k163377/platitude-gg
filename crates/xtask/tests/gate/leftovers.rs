//! What a landing clears of its own session's making (CLAUDE.md §Git 運用):
//! the branches the session's git created, as the reference-transaction
//! hook wrote down, and the trees that have them out — each only when main,
//! or the branch as it stood before its rebase, holds all it holds. What
//! another session, the app or the user made is neither judged nor named.

use std::path::{Path, PathBuf};

use crate::support::Sandbox;

fn listed(sb: &Sandbox) -> String {
    sb.git_ok(&sb.repo, &["worktree", "list", "--porcelain"])
}

fn branches(sb: &Sandbox) -> Vec<String> {
    sb.git_ok(
        &sb.repo,
        &["for-each-ref", "--format=%(refname:short)", "refs/heads/"],
    )
    .lines()
    .map(str::to_string)
    .collect()
}

/// The environment of a git the session `who` runs; none for the app's
/// git or the user's own.
fn by(who: Option<&str>) -> Vec<(&str, &str)> {
    who.map(|who| ("CLAUDE_CODE_SESSION_ID", who))
        .into_iter()
        .collect()
}

/// A tree beside the seats on a new branch `claude/<name>` at main's tip,
/// made by `who`'s git.
fn tree_by(sb: &Sandbox, name: &str, who: Option<&str>) -> PathBuf {
    let path = sb.repo.join(".claude/worktrees").join(name);
    let spelled = path.display().to_string().replace('\\', "/");
    let branch = format!("claude/{name}");
    sb.git(
        &sb.repo,
        &["worktree", "add", "-q", "-b", &branch, &spelled, "main"],
        &by(who),
    )
    .unwrap_or_else(|e| panic!("{name}: {e}"));
    path
}

/// A branch at `at`, made by `who`'s git.
fn branch_by(sb: &Sandbox, name: &str, at: &str, who: Option<&str>) {
    sb.git(&sb.repo, &["branch", name, at], &by(who))
        .unwrap_or_else(|e| panic!("{name}: {e}"));
}

/// Whether the hook's record names a creator for `branch`.
fn recorded(sb: &Sandbox, branch: &str) -> bool {
    sb.repo
        .join(".git/pgg-gate/made/refs/heads")
        .join(branch)
        .exists()
}

/// Holds the tree in use the way the system sees a session in it, for as
/// long as the value lives: on Windows a file open in it (the directory
/// will not be renamed), elsewhere a process standing in it.
#[cfg(windows)]
struct InUse(#[expect(dead_code, reason = "held open, never read")] std::fs::File);

#[cfg(windows)]
impl InUse {
    fn hold(tree: &Path) -> Self {
        Self(std::fs::File::open(tree.join("Cargo.toml")).expect("a file of the tree"))
    }
}

#[cfg(unix)]
struct InUse(std::process::Child);

#[cfg(unix)]
impl InUse {
    fn hold(tree: &Path) -> Self {
        Self(
            std::process::Command::new("sleep")
                .arg("600")
                .current_dir(tree)
                .spawn()
                .expect("a process standing in the tree"),
        )
    }
}

#[cfg(unix)]
impl Drop for InUse {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// A tree this session made whose directory went while git still lists
/// it — carrying a commit of its own when `work` says so.
fn vanished(sb: &Sandbox, name: &str, work: bool) -> PathBuf {
    let tree = tree_by(sb, name, Some("mine"));
    if work {
        sb.write(&tree, "internal-docs/notes.md", "# notes\n\nleft behind\n");
        sb.commit_all(&tree, "docs: work its directory took along", &[]);
    }
    std::fs::remove_dir_all(&tree).expect("its directory goes");
    tree
}

/// The `leftovers:` lines of a landing's output.
fn swept(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|line| line.starts_with("leftovers:"))
        .collect()
}

/// The trees this session made go when main holds all they hold and
/// nothing stands in the way — one whose directory went, by the commit
/// git's record names. Another session's, the app's and a seat stay, and
/// are not named.
#[test]
fn a_landing_clears_the_trees_its_session_made_and_no_others() {
    let sb = Sandbox::new("leftover-trees");
    let mine = Some("mine");
    let spent = tree_by(&sb, "spent-tree", mine);
    let dirty = tree_by(&sb, "dirty-tree", mine);
    sb.write(&dirty, "scratch.txt", "not committed\n");
    let ahead = tree_by(&sb, "ahead-tree", mine);
    sb.write(&ahead, "internal-docs/notes.md", "# notes\n\nits own\n");
    sb.commit_all(&ahead, "docs: work of its own", &[]);
    let locked = tree_by(&sb, "locked-tree", mine);
    let spelled = locked.display().to_string().replace('\\', "/");
    sb.git_ok(
        &sb.repo,
        &["worktree", "lock", "--reason", "parked by hand", &spelled],
    );
    let busy = tree_by(&sb, "busy-tree", mine);
    let went_away = vanished(&sb, "vanished-tree", false);
    vanished(&sb, "vanished-work", true);
    let theirs = tree_by(&sb, "theirs-tree", Some("theirs"));
    let apps = tree_by(&sb, "app-tree", None);
    let seat = sb.repo.join(".claude/worktrees/c");
    let spelled = seat.display().to_string().replace('\\', "/");
    sb.git(
        &sb.repo,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "worktree-c",
            &spelled,
            "main",
        ],
        &by(mine),
    )
    .expect("a seat this session made");
    sb.write_refs(&sb.seat, 5);
    sb.commit_all(&sb.seat, "feat(core): five", &[]);

    let held = InUse::hold(&busy);
    let (ok, text) = sb.land_as("worktree-a", "mine");
    drop(held);
    assert!(ok, "{text}");

    let trees = listed(&sb);
    for (gone, line) in [
        (&spent, "took away worktree spent-tree (claude/spent-tree)"),
        (
            &went_away,
            "took away worktree vanished-tree (claude/vanished-tree)",
        ),
    ] {
        let name = gone.file_name().expect("a name").to_string_lossy();
        assert!(!gone.exists() && !trees.contains(name.as_ref()), "{text}");
        assert!(text.contains(line), "{line}: {text}");
    }
    for (tree, why) in [
        ("dirty-tree", "1 uncommitted file(s)"),
        ("ahead-tree", "1 commit(s) main does not hold"),
        ("locked-tree", "it is locked (parked by hand)"),
        ("busy-tree", "works in it"),
        ("vanished-work", "1 commit(s) main does not hold"),
    ] {
        assert!(trees.contains(tree), "{tree}: {trees}");
        let line = swept(&text)
            .into_iter()
            .find(|line| line.contains(&format!("kept worktree {tree} ")))
            .unwrap_or_else(|| panic!("{tree} named as kept: {text}"));
        assert!(line.contains(why), "{why}: {line}");
    }
    for (other, named) in [
        (&theirs, "theirs-tree"),
        (&apps, "app-tree"),
        (&seat, "worktree c "),
    ] {
        assert!(other.exists(), "{}: {text}", other.display());
        assert!(
            swept(&text).iter().all(|line| !line.contains(named)),
            "{named} is not this session's to judge: {text}"
        );
    }
    assert!(
        trees.contains("theirs-tree") && trees.contains("app-tree"),
        "{trees}"
    );
    let left = branches(&sb);
    for gone in ["claude/spent-tree", "claude/vanished-tree"] {
        assert!(!left.iter().any(|branch| branch == gone), "{gone}: {text}");
        assert!(!recorded(&sb, gone), "the record goes with {gone}");
    }
    for stays in [
        "claude/ahead-tree",
        "claude/theirs-tree",
        "claude/app-tree",
        "worktree-c",
    ] {
        assert!(left.iter().any(|branch| branch == stays), "{stays}: {text}");
    }
}

/// The branches this session made go when main holds all they hold —
/// main's own past, a copy of the landing branch from before its history
/// was rewritten — or the landing branch did before its rebase: a copy
/// taken half way, whose lines the branch went on to change. One with work
/// of its own stays, named. Another session's and the user's stay
/// unjudged.
#[test]
fn a_landing_clears_the_branches_its_session_made_and_no_others() {
    let sb = Sandbox::new("leftover-branches");
    let mine = Some("mine");
    branch_by(&sb, "claude/gone-tree", "main", mine);
    let ahead = tree_by(&sb, "ahead-tree", mine);
    sb.write(&ahead, "internal-docs/notes.md", "# notes\n\nits own\n");
    sb.commit_all(&ahead, "docs: work of its own", &[]);
    branch_by(&sb, "keep/own-work", "claude/ahead-tree", mine);
    sb.write_refs(&sb.seat, 7);
    sb.commit_all(&sb.seat, "feat(core): seven", &[]);
    branch_by(&sb, "keep/mid", "worktree-a", mine);
    sb.write_refs(&sb.seat, 8);
    sb.commit_all(&sb.seat, "feat(core): eight", &[]);
    branch_by(&sb, "keep/pre-reorg", "worktree-a", mine);
    sb.git(
        &sb.seat,
        &[
            "commit",
            "-q",
            "--amend",
            "-m",
            "feat(core): eight, reworded",
        ],
        &[],
    )
    .expect("the rewrite");
    branch_by(&sb, "their-mark", "main", Some("theirs"));
    branch_by(&sb, "users-own", "main", None);
    // Main moves on, so the landing rebases what it lands.
    sb.write(&sb.repo, "internal-docs/notes.md", "# notes\n\nmoved\n");
    sb.commit_all(&sb.repo, "docs: main moved", &[("PGG_GATE_SKIP", "1")]);

    let (ok, text) = sb.land_as("worktree-a", "mine");
    assert!(ok && text.contains("rebasing"), "{text}");

    let left = branches(&sb);
    for gone in ["claude/gone-tree", "keep/mid", "keep/pre-reorg"] {
        assert!(!left.iter().any(|branch| branch == gone), "{gone}: {text}");
        assert!(!recorded(&sb, gone), "the record goes with {gone}");
    }
    for stays in [
        "main",
        "worktree-a",
        "keep/own-work",
        "claude/ahead-tree",
        "their-mark",
        "users-own",
    ] {
        assert!(left.iter().any(|branch| branch == stays), "{stays}: {text}");
    }
    let went = swept(&text)
        .into_iter()
        .find(|line| line.contains("took away branch(es)"))
        .unwrap_or_else(|| panic!("branches taken away: {text}"));
    for gone in [
        "claude/gone-tree (was ",
        "keep/mid (was ",
        "keep/pre-reorg (was ",
    ] {
        assert!(went.contains(gone), "{gone}: {went}");
    }
    assert!(
        text.contains("keep/own-work (1 commit(s) main does not hold)"),
        "{text}"
    );
    for other in ["their-mark", "users-own"] {
        assert!(
            swept(&text).iter().all(|line| !line.contains(other)),
            "{other} is not this session's to judge: {text}"
        );
    }
}
