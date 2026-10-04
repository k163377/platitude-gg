//! What a landing clears of its own session's making (CLAUDE.md §Git 運用):
//! the branches the session's git created, as the reference-transaction
//! hook wrote down, the tree the app made for the session to start in, as
//! the session wrote down when it started there, and the trees that have
//! them out — each only when main, or the branch as it stood before its
//! rebase, holds all it holds. What stays of the tree the session started
//! in is left to a later landing; anything else another session, the app
//! or the user made is neither judged nor named.

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

/// A tree as the app makes one for a session to start in: `claude/<name>`
/// at main's tip by a git with no session's mark, and the app's own mark
/// in the tree's admin directory.
fn app_tree(sb: &Sandbox, name: &str) -> PathBuf {
    let tree = tree_by(sb, name, None);
    let admin = sb.repo.join(".git/worktrees").join(name);
    std::fs::write(admin.join("claude-desktop-worktree"), "").expect("the app's mark");
    tree
}

/// A transcript where Claude Code keeps one for `session` started at
/// `tree`: under a directory named after the place — by its long name, as
/// the app hands it over — every character but a letter or digit spelled
/// `-`.
fn transcript_in(sb: &Sandbox, tree: &Path, session: &str) {
    let place: String = std::fs::canonicalize(tree)
        .expect("the tree's long name")
        .display()
        .to_string()
        .trim_start_matches(r"\\?\")
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let dir = sb.claude_config().join("projects").join(place);
    std::fs::create_dir_all(&dir).expect("the project directory");
    std::fs::write(dir.join(format!("{session}.jsonl")), "{}\n").expect("the transcript");
}

/// What SessionStart says to `session` as it starts in `tree`.
fn start_in(sb: &Sandbox, tree: &Path, session: &str) -> String {
    sb.hook(
        "session-start",
        &format!(
            "{{\"session_id\":\"{session}\",\"cwd\":\"{}\",\"hook_event_name\":\"SessionStart\",\
             \"source\":\"startup\"}}",
            tree.display().to_string().replace('\\', "/")
        ),
    )
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

/// The tree the app made for a session to start in is written down as the
/// session's when it starts there, and its landing takes it away with its
/// branch. One something still works in is left behind — and stays so when
/// the session resumes there: a later landing, whoever's, says it still
/// stands, and takes it once nothing works in it. A tree the app made that
/// nobody started in, one another session started in and never landed
/// from, and one the app did not make stay unjudged.
#[test]
fn a_landing_clears_the_tree_its_session_started_in() {
    let sb = Sandbox::new("leftover-start");
    let started = app_tree(&sb, "started-tree");
    let busy = app_tree(&sb, "busy-start");
    let theirs = app_tree(&sb, "their-start");
    let pooled = app_tree(&sb, "pooled-tree");
    let plain = tree_by(&sb, "plain-tree", None);
    for (tree, who) in [(&started, "mine"), (&busy, "mine"), (&theirs, "theirs")] {
        let said = start_in(&sb, tree, who);
        assert!(said.contains("written down as this session's"), "{said}");
    }
    let said = start_in(&sb, &plain, "mine");
    assert!(
        said.contains("outside the seat roster") && !said.contains("written down"),
        "a tree the app did not make is nobody's start: {said}"
    );
    let unjudged = ["their-start", "pooled-tree", "plain-tree"];
    let stay = |text: &str| {
        let left = branches(&sb);
        for name in unjudged {
            let branch = format!("claude/{name}");
            assert!(left.contains(&branch), "{branch}: {text}");
            assert!(
                swept(text).iter().all(|line| !line.contains(name)),
                "{name} is not this session's to judge: {text}"
            );
        }
        assert!(
            theirs.exists() && pooled.exists() && plain.exists(),
            "{text}"
        );
    };

    sb.write_refs(&sb.seat, 9);
    sb.commit_all(&sb.seat, "feat(core): nine", &[]);
    let held = InUse::hold(&busy);
    let (ok, text) = sb.land_as("worktree-a", "mine");
    assert!(ok, "{text}");
    assert!(
        text.contains("took away worktree started-tree (claude/started-tree)") && !started.exists(),
        "{text}"
    );
    assert!(
        !branches(&sb).contains(&"claude/started-tree".to_string())
            && !recorded(&sb, "claude/started-tree"),
        "the branch and its record go with the tree: {text}"
    );
    assert!(
        text.contains("kept worktree busy-start (claude/busy-start) — ")
            && text.contains("works in it"),
        "{text}"
    );
    assert!(
        text.contains("left claude/busy-start to whichever landing comes next"),
        "{text}"
    );
    assert!(
        busy.exists() && recorded(&sb, "claude/busy-start"),
        "{text}"
    );
    stay(&text);
    let said = start_in(&sb, &busy, "mine");
    assert!(
        said.contains("written down as this session's"),
        "a resume: {said}"
    );

    sb.write_refs(&sb.seat, 10);
    sb.commit_all(&sb.seat, "feat(core): ten", &[]);
    let (ok, text) = sb.land_as("worktree-a", "later");
    drop(held);
    assert!(ok, "{text}");
    assert!(
        text.contains(
            "still standing, left behind by earlier landings — their sessions' work, not this \
             one's to delete: worktree busy-start ("
        ),
        "{text}"
    );
    assert!(busy.exists(), "{text}");
    stay(&text);

    sb.write_refs(&sb.seat, 11);
    sb.commit_all(&sb.seat, "feat(core): eleven", &[]);
    let (ok, text) = sb.land_as("worktree-a", "later");
    assert!(ok, "{text}");
    assert!(
        text.contains(
            "took away worktree busy-start (claude/busy-start), which session mine left behind"
        ) && !busy.exists(),
        "{text}"
    );
    assert!(
        !branches(&sb).contains(&"claude/busy-start".to_string())
            && !recorded(&sb, "claude/busy-start"),
        "{text}"
    );
    assert!(recorded(&sb, "claude/their-start"), "{text}");
    stay(&text);
}

/// A session whose start wrote nothing down — a chip's first hooks run the
/// code of the tree the app cut from origin/main — is found by its
/// transcript at its landing, which takes the tree away. Another session's
/// transcript is not this landing's to read a tree from.
#[test]
fn a_landing_finds_the_tree_its_session_started_in_by_its_transcript() {
    let sb = Sandbox::new("leftover-transcript");
    let chip = app_tree(&sb, "chip-start");
    let theirs = app_tree(&sb, "their-chip");
    transcript_in(&sb, &chip, "mine");
    transcript_in(&sb, &theirs, "theirs");
    sb.write_refs(&sb.seat, 14);
    sb.commit_all(&sb.seat, "feat(core): fourteen", &[]);
    let (ok, text) = sb.land_as("worktree-a", "mine");
    assert!(ok, "{text}");
    assert!(
        text.contains("took away worktree chip-start (claude/chip-start)") && !chip.exists(),
        "{text}"
    );
    assert!(
        !branches(&sb).contains(&"claude/chip-start".to_string()),
        "{text}"
    );
    assert!(
        theirs.exists() && swept(&text).iter().all(|line| !line.contains("their-chip")),
        "{text}"
    );
    assert!(!recorded(&sb, "claude/their-chip"), "{text}");
}

/// The app hands a tree on to the next session it starts there: a tree an
/// earlier landing left behind is then that session's, and nobody else's
/// landing judges it.
#[test]
fn a_tree_the_app_hands_on_is_the_next_sessions() {
    let sb = Sandbox::new("leftover-handed-on");
    let handed = app_tree(&sb, "handed-on");
    start_in(&sb, &handed, "mine");
    sb.write_refs(&sb.seat, 12);
    sb.commit_all(&sb.seat, "feat(core): twelve", &[]);
    let held = InUse::hold(&handed);
    let (ok, text) = sb.land_as("worktree-a", "mine");
    drop(held);
    assert!(
        ok && text.contains("left claude/handed-on to whichever landing comes next"),
        "{text}"
    );

    let said = start_in(&sb, &handed, "next");
    assert!(said.contains("written down as this session's"), "{said}");
    sb.write_refs(&sb.seat, 13);
    sb.commit_all(&sb.seat, "feat(core): thirteen", &[]);
    let (ok, text) = sb.land_as("worktree-a", "later");
    assert!(ok, "{text}");
    assert!(
        swept(&text).iter().all(|line| !line.contains("handed-on")),
        "the next session's start is not this landing's to judge: {text}"
    );
    assert!(
        handed.exists() && branches(&sb).contains(&"claude/handed-on".to_string()),
        "{text}"
    );
}

/// What keeps a tree the session started in other than something working
/// in it — its own files, its own commits — keeps it the session's own:
/// nothing is left to a later landing, and no later landing judges it.
#[test]
fn a_start_tree_kept_for_its_own_work_is_not_left_behind() {
    let sb = Sandbox::new("leftover-start-work");
    let dirty = app_tree(&sb, "dirty-start");
    start_in(&sb, &dirty, "mine");
    sb.write(&dirty, "scratch.txt", "not committed\n");
    sb.write_refs(&sb.seat, 15);
    sb.commit_all(&sb.seat, "feat(core): fifteen", &[]);
    let (ok, text) = sb.land_as("worktree-a", "mine");
    assert!(ok, "{text}");
    assert!(
        text.contains("kept worktree dirty-start (claude/dirty-start) — 1 uncommitted file(s)"),
        "{text}"
    );
    assert!(!text.contains("left claude/dirty-start"), "{text}");

    sb.write_refs(&sb.seat, 16);
    sb.commit_all(&sb.seat, "feat(core): sixteen", &[]);
    let (ok, text) = sb.land_as("worktree-a", "later");
    assert!(ok, "{text}");
    assert!(
        swept(&text)
            .iter()
            .all(|line| !line.contains("dirty-start")),
        "{text}"
    );
    assert!(dirty.exists(), "{text}");
}

/// A start writes down only the branch the app cut for its tree: not one
/// checked out there since, nor one a session's git made.
#[test]
fn a_start_writes_down_only_the_branch_the_app_cut_for_its_tree() {
    let sb = Sandbox::new("leftover-start-branch");
    let switched = app_tree(&sb, "switched");
    branch_by(&sb, "users-own", "main", None);
    sb.git_ok(&switched, &["switch", "-q", "users-own"]);
    let said = start_in(&sb, &switched, "mine");
    assert!(!said.contains("written down"), "{said}");
    assert!(!recorded(&sb, "users-own"), "{said}");

    let theirs = tree_by(&sb, "their-cut", Some("theirs"));
    std::fs::write(
        sb.repo
            .join(".git/worktrees/their-cut")
            .join("claude-desktop-worktree"),
        "",
    )
    .expect("the app's mark");
    let said = start_in(&sb, &theirs, "mine");
    assert!(!said.contains("written down"), "{said}");
    let record = std::fs::read_to_string(
        sb.repo
            .join(".git/pgg-gate/made/refs/heads/claude/their-cut"),
    )
    .expect("the record their git wrote");
    assert_eq!(record.trim(), "theirs", "{said}");
}
