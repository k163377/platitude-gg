//! The presets the discard log is drawn from (破棄記録仕様.md §3): one of
//! every move git's own reflogs keep that takes commits off a tip, and one
//! of every line Platitude GG's own record holds — written here in the
//! record's own shape (§2), since xtask runs none of the application.

use std::time::{SystemTime, UNIX_EPOCH};

use super::deep::{DEEP_PARKED_BRANCH, deep_parked};
use super::repo::{DemoRepo, output_of};

/// The ref whose reflog holds the record (`platitude_core::discards::RECORD_REF`).
const RECORD_REF: &str = "refs/pgg/discards";

/// Every kind of entry, oldest first, so the newest is thrown-away work: a
/// tag deleted here and on a remote at once, then the moves git keeps (a
/// rebase, a reset that threw work away too, an
/// amend, a branch set back by hand, a detached HEAD left behind, two
/// branches one rebase moved together), then the record's notes — a branch
/// deleted here and on a remote holding another tip, a remote's branch
/// deleted and one pushed over, a tag deleted here, on a remote, and pushed
/// over — then forty commits of later work, which put what the older
/// entries took down the graph, and last a worktree removed, a stash
/// dropped and one popped, and the copies: untracked files deleted, and
/// files discarded.
///
/// Each entry is dated where the list writes its time a different way
/// (`RecoverEntries.ago`): minutes, hours, yesterday, days, the date.
pub(super) fn discards(repo: &mut DemoRepo) -> Result<(), String> {
    let clock = Clock::now()?;
    repo.pin_clock(Some(clock.ago(DAY * 430)));
    repo.commit("README.md", "# demo\n", "docs: start the readme")?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    repo.commit("src/lib.txt", "lib v1\n", "feat: add the library")?;
    repo.add_origin()?;
    repo.git(&["push", "--quiet", "origin", "main"])?;

    tag_deleted_both(repo, &clock)?;
    moves(repo, &clock)?;
    remote_notes(repo, &clock)?;
    tag_notes(repo, &clock)?;
    for step in 1..=LATER_COMMITS {
        let hours_back = u64::from(LATER_COMMITS - step) + 31;
        repo.pin_clock(Some(clock.ago(hours_back * HOUR)));
        repo.commit(
            "log.txt",
            &format!("entry {step}\n"),
            &format!("chore: log entry {step}"),
        )?;
    }
    the_rest(repo, &clock)?;
    repo.pin_clock(None);
    Ok(())
}

const MINUTE: u64 = 60;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;

/// The moment the preset is built, and the ones it dates its entries at.
struct Clock {
    now: u64,
}

impl Clock {
    fn now() -> Result<Self, String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs();
        Ok(Self { now })
    }

    fn ago(&self, secs: u64) -> u64 {
        self.now.saturating_sub(secs)
    }
}

/// `deep_parked` with its parked branch deleted on the record: the entry's
/// tip lies past the graph's window, so it cannot come back until the graph
/// walks that far (破棄記録仕様.md §4).
pub(super) fn discards_deep(repo: &mut DemoRepo) -> Result<(), String> {
    deep_parked(repo)?;
    let tip = repo.git(&["rev-parse", DEEP_PARKED_BRANCH])?;
    repo.git(&["branch", "--delete", "--force", DEEP_PARKED_BRANCH])?;
    note(
        repo,
        &tip,
        &format!("delete branch {DEEP_PARKED_BRANCH}"),
        &[
            ("Operation", "delete branch"),
            ("Branch", DEEP_PARKED_BRANCH),
        ],
    )
}

/// The oldest entry, last in the list: a tag deleted here and on origin at
/// once, the same object on both sides — one part, its title `here and on`.
fn tag_deleted_both(repo: &mut DemoRepo, clock: &Clock) -> Result<(), String> {
    repo.pin_clock(Some(clock.ago(DAY * 420)));
    repo.git(&["tag", "v0.1"])?;
    let commit = repo.git(&["rev-parse", "HEAD"])?;
    repo.git(&["push", "--quiet", "origin", "v0.1"])?;
    repo.git(&["tag", "--delete", "v0.1"])?;
    repo.git(&["push", "--quiet", "origin", "--delete", "refs/tags/v0.1"])?;
    note(
        repo,
        &commit,
        "delete tag v0.1",
        &[
            ("Operation", "delete tag"),
            ("Tag", "v0.1"),
            ("Object", &commit),
            ("Remote", "origin"),
        ],
    )
}

/// The moves git's own reflogs keep.
fn moves(repo: &mut DemoRepo, clock: &Clock) -> Result<(), String> {
    // A topic rebased onto main: its first two commits are left behind.
    repo.pin_clock(Some(clock.ago(DAY * 400)));
    repo.git(&["switch", "--create", "feature/api"])?;
    repo.commit("src/api.txt", "api v1\n", "feat: draft the api")?;
    repo.commit("src/api.txt", "api v2\n", "feat: finish the api")?;
    repo.git(&["switch", "main"])?;
    repo.commit("src/app.txt", "app v2\n", "feat: rework the app")?;
    repo.git(&["switch", "feature/api"])?;
    repo.git(&["rebase", "--quiet", "main"])?;
    repo.git(&["switch", "main"])?;

    // Two commits a reset takes off main, with work in the tree it throws
    // away too: copied first, the move it goes with on the copy (§2).
    repo.pin_clock(Some(clock.ago(DAY * 300)));
    repo.commit("src/session.txt", "session v1\n", "feat: store sessions")?;
    repo.commit(
        "src/session.txt",
        "session v2\n",
        "feat: add the OAuth callback",
    )?;
    repo.write("src/session.txt", "session v3, half done\n")?;
    repo.write("src/app.txt", "app v2, tidied\n")?;
    repo.git(&["add", "src/app.txt"])?;
    let old = repo.git(&["rev-parse", "HEAD"])?;
    let new = repo.git(&["rev-parse", "HEAD~2"])?;
    let moved = format!("main {old} {new}");
    copy(
        repo,
        &["src/app.txt", "src/session.txt"],
        &[],
        "reset --hard",
        Some(&moved),
    )?;
    repo.git(&["reset", "--quiet", "--hard", "HEAD~2"])?;

    // An amend: the first wording is left behind.
    repo.pin_clock(Some(clock.ago(DAY * 200)));
    repo.commit("docs/guide.md", "guide v1\n", "docs: add a gide")?;
    repo.git(&["commit", "--amend", "--message", "docs: add a guide"])?;

    // A branch set back by hand.
    repo.pin_clock(Some(clock.ago(DAY * 120)));
    repo.git(&["switch", "--create", "spike"])?;
    repo.commit("spike.txt", "try\n", "chore: try a spike")?;
    repo.git(&["switch", "main"])?;
    repo.git(&["branch", "--force", "spike", "main"])?;

    // A detached HEAD with work on it, left for main.
    repo.pin_clock(Some(clock.ago(DAY * 90)));
    repo.git(&["switch", "--detach", "HEAD~1"])?;
    repo.commit("probe.txt", "probe\n", "test: probe the parser")?;
    repo.git(&["switch", "main"])?;

    // Two stacked branches one rebase moves together, on the record as one.
    repo.pin_clock(Some(clock.ago(DAY * 60)));
    repo.git(&["switch", "--create", "stack/base"])?;
    repo.commit("stack/base.txt", "base\n", "feat: lay the base")?;
    repo.git(&["switch", "--create", "stack/top"])?;
    repo.commit("stack/top.txt", "top\n", "feat: build on the base")?;
    repo.git(&["switch", "main"])?;
    repo.commit("src/lib.txt", "lib v2\n", "feat: grow the library")?;
    let before = [
        repo.git(&["rev-parse", "stack/base"])?,
        repo.git(&["rev-parse", "stack/top"])?,
    ];
    repo.git(&["switch", "stack/top"])?;
    repo.git(&["rebase", "--quiet", "--update-refs", "main"])?;
    repo.git(&["switch", "main"])?;
    let after = [
        repo.git(&["rev-parse", "stack/base"])?,
        repo.git(&["rev-parse", "stack/top"])?,
    ];
    let base = format!("stack/base {} {}", before[0], after[0]);
    let top = format!("stack/top {} {}", before[1], after[1]);
    note(
        repo,
        &before[0],
        "rebase 2 branches",
        &[("Operation", "rebase"), ("Moved", &base), ("Moved", &top)],
    )
}

/// What went from the remote: a branch deleted here and over there, the
/// remote holding a commit the branch here did not; a remote's branch
/// deleted; one pushed over.
fn remote_notes(repo: &mut DemoRepo, clock: &Clock) -> Result<(), String> {
    repo.pin_clock(Some(clock.ago(DAY * 33)));
    repo.git(&["switch", "--create", "feature/login"])?;
    repo.commit("src/login.txt", "login v1\n", "feat: add the login form")?;
    repo.git(&[
        "push",
        "--quiet",
        "--set-upstream",
        "origin",
        "feature/login",
    ])?;
    let theirs = repo.git(&["rev-parse", "HEAD"])?;
    repo.commit("src/login.txt", "login v2\n", "feat: check the password")?;
    let ours = repo.git(&["rev-parse", "HEAD"])?;
    repo.git(&["switch", "main"])?;
    repo.git(&["branch", "--delete", "--force", "feature/login"])?;
    repo.git(&["push", "--quiet", "origin", "--delete", "feature/login"])?;
    note(
        repo,
        &ours,
        "delete branch feature/login",
        &[
            ("Operation", "delete branch"),
            ("Branch", "feature/login"),
            ("Upstream", "origin feature/login"),
            ("Remote", "origin"),
            ("Remote-branch", "feature/login"),
            ("Remote-tip", &theirs),
        ],
    )?;

    repo.pin_clock(Some(clock.ago(DAY * 20)));
    repo.git(&["switch", "--create", "old-api"])?;
    repo.commit("src/old.txt", "old\n", "feat: keep the old api")?;
    repo.git(&["push", "--quiet", "origin", "old-api"])?;
    let gone = repo.git(&["rev-parse", "HEAD"])?;
    repo.git(&["switch", "main"])?;
    repo.git(&["branch", "--delete", "--force", "old-api"])?;
    repo.git(&["push", "--quiet", "origin", "--delete", "old-api"])?;
    repo.git(&["fetch", "--quiet", "--prune", "origin"])?;
    note(
        repo,
        &gone,
        "delete origin/old-api",
        &[
            ("Operation", "delete remote branch"),
            ("Remote", "origin"),
            ("Branch", "old-api"),
        ],
    )?;

    repo.pin_clock(Some(clock.ago(DAY * 12)));
    repo.git(&["switch", "--create", "experiment"])?;
    repo.commit("src/try.txt", "try v1\n", "feat: try a cache")?;
    repo.git(&["push", "--quiet", "--set-upstream", "origin", "experiment"])?;
    let pushed_over = repo.git(&["rev-parse", "HEAD"])?;
    repo.git(&["reset", "--quiet", "--hard", "HEAD~1"])?;
    repo.commit("src/try.txt", "try v2\n", "feat: try a queue instead")?;
    repo.git(&["push", "--quiet", "--force", "origin", "experiment"])?;
    repo.git(&["switch", "main"])?;
    note(
        repo,
        &pushed_over,
        "force-push origin/experiment",
        &[
            ("Operation", "force push"),
            ("Remote", "origin"),
            ("Branch", "experiment"),
        ],
    )
}

/// A tag deleted here, one deleted on the remote, one pushed over there.
fn tag_notes(repo: &mut DemoRepo, clock: &Clock) -> Result<(), String> {
    repo.pin_clock(Some(clock.ago(DAY * 9)));
    repo.git(&[
        "tag",
        "--annotate",
        "--message",
        "first candidate",
        "v1.2.0-rc1",
    ])?;
    let object = repo.git(&["rev-parse", "refs/tags/v1.2.0-rc1"])?;
    let commit = repo.git(&["rev-parse", "HEAD"])?;
    repo.git(&["tag", "--delete", "v1.2.0-rc1"])?;
    note(
        repo,
        &commit,
        "delete tag v1.2.0-rc1",
        &[
            ("Operation", "delete tag"),
            ("Tag", "v1.2.0-rc1"),
            ("Object", &object),
        ],
    )?;

    repo.pin_clock(Some(clock.ago(DAY * 5)));
    repo.git(&["tag", "v0.9", "HEAD~1"])?;
    let old = repo.git(&["rev-parse", "HEAD~1"])?;
    repo.git(&["push", "--quiet", "origin", "v0.9"])?;
    repo.git(&["push", "--quiet", "origin", "--delete", "refs/tags/v0.9"])?;
    note(
        repo,
        &old,
        "delete tag v0.9 on origin",
        &[
            ("Operation", "delete remote tag"),
            ("Tag", "v0.9"),
            ("Object", &old),
            ("Remote", "origin"),
        ],
    )?;

    repo.pin_clock(Some(clock.ago(DAY * 3)));
    repo.git(&["tag", "v1.0", "HEAD~2"])?;
    let first = repo.git(&["rev-parse", "HEAD~2"])?;
    repo.git(&["push", "--quiet", "origin", "v1.0"])?;
    repo.git(&["tag", "--force", "v1.0", "HEAD"])?;
    repo.git(&["push", "--quiet", "--force", "origin", "v1.0"])?;
    note(
        repo,
        &first,
        "force-push tag v1.0 to origin",
        &[
            ("Operation", "force push tag"),
            ("Tag", "v1.0"),
            ("Object", &first),
            ("Remote", "origin"),
        ],
    )
}

/// A worktree removed, a stash dropped and one popped, untracked files
/// deleted, and — newest — files discarded.
fn the_rest(repo: &mut DemoRepo, clock: &Clock) -> Result<(), String> {
    repo.pin_clock(Some(clock.ago(HOUR * 30)));
    let side = repo.root.join("side");
    let side_path = side.to_string_lossy().replace('\\', "/");
    repo.git(&["worktree", "add", "--quiet", "-b", "side/notes", &side_path])?;
    let head = repo.git(&["rev-parse", "HEAD"])?;
    repo.git(&["worktree", "remove", &side_path])?;
    note(
        repo,
        &head,
        &format!("remove worktree {side_path}"),
        &[
            ("Operation", "remove worktree"),
            ("Worktree-path", &side_path),
            ("Branch", "side/notes"),
        ],
    )?;

    for (message, operation, back) in [
        ("try the OAuth callback again", "stash drop", HOUR * 20),
        ("half a refactor", "stash pop", HOUR * 3),
    ] {
        repo.pin_clock(Some(clock.ago(back)));
        repo.write("src/app.txt", &format!("app, {message}\n"))?;
        repo.git(&["stash", "push", "--quiet", "--message", message])?;
        let entry = repo.git(&["rev-parse", "stash@{0}"])?;
        let said = format!("On main: {message}");
        if operation == "stash pop" {
            repo.git(&["stash", "pop", "--quiet"])?;
            repo.git(&["checkout", "--", "src/app.txt"])?;
        } else {
            repo.git(&["stash", "drop", "--quiet"])?;
        }
        note(
            repo,
            &entry,
            &format!("{operation}: {said}"),
            &[("Operation", operation), ("Message", &said)],
        )?;
    }

    repo.pin_clock(Some(clock.ago(MINUTE * 50)));
    repo.write("notes/todo.txt", "todo\n")?;
    repo.write("notes/ideas.txt", "ideas\n")?;
    copy(
        repo,
        &[],
        &["notes/todo.txt", "notes/ideas.txt"],
        "untracked",
        None,
    )?;
    repo.git(&["clean", "--quiet", "--force", "--", "notes"])?;

    repo.pin_clock(Some(clock.ago(MINUTE * 2)));
    repo.write("src/app.txt", "app v3, half done\n")?;
    repo.write("src/lib.txt", "lib v3, half done\n")?;
    repo.git(&["add", "src/app.txt"])?;
    copy(repo, &["src/app.txt", "src/lib.txt"], &[], "discard", None)?;
    repo.git(&["reset", "--quiet", "--hard"])?;
    Ok(())
}

/// A copy of the work at `tracked` and `untracked` (§2.1): the stash shape
/// `stash create` writes for the tracked side — the index and the working
/// tree — and the untracked files' own commit, made again under the
/// record's message and identity.
fn copy(
    repo: &mut DemoRepo,
    tracked: &[&str],
    untracked: &[&str],
    operation: &str,
    moved: Option<&str>,
) -> Result<(), String> {
    let head = repo.git(&["rev-parse", "HEAD"])?;
    let (working_tree, index) = if tracked.is_empty() {
        let tree = repo.git(&["rev-parse", "HEAD^{tree}"])?;
        let index = recorder(
            repo,
            &["commit-tree", &tree, "-p", &head, "-m", "index on main"],
        )?;
        (tree, index)
    } else {
        let stash = repo.git(&["stash", "create"])?;
        (
            repo.git(&["rev-parse", &format!("{stash}^{{tree}}")])?,
            repo.git(&["rev-parse", &format!("{stash}^2")])?,
        )
    };
    let mut parents = vec![head, index];
    if !untracked.is_empty() {
        parents.push(untracked_commit(repo, untracked)?);
    }
    let count = tracked.len() + untracked.len();
    let summary = format!("{operation} {count} paths on main");
    let work = repo.work.to_string_lossy().replace('\\', "/");
    // `Worktree:` names the worktree as git does, so a run's copy of the
    // template reads the work as its own (`template` moves the repository).
    let mut trailers = vec![
        ("Operation", operation),
        ("Worktree-path", work.as_str()),
        ("Worktree", "."),
        ("Branch", "main"),
    ];
    if let Some(moved) = moved {
        trailers.push(("Moved", moved));
    }
    let mut args = vec!["commit-tree", working_tree.as_str()];
    for parent in &parents {
        args.extend(["-p", parent.as_str()]);
    }
    let message = message(&summary, &trailers);
    args.extend(["-m", message.as_str()]);
    let copy = recorder(repo, &args)?;
    append(repo, &copy, &summary)
}

/// The untracked files' own commit: a root commit of them alone, written
/// through an index of its own so the real one is left as it is.
fn untracked_commit(repo: &mut DemoRepo, paths: &[&str]) -> Result<String, String> {
    let index = repo.root.join("untracked-index");
    let work = repo.work.clone();
    let mut add = repo.command(&work, &["add", "--force", "--"]);
    add.args(paths).env("GIT_INDEX_FILE", &index);
    ran(&mut add)?;
    let mut write = repo.command(&work, &["write-tree"]);
    write.env("GIT_INDEX_FILE", &index);
    let tree = ran(&mut write)?;
    std::fs::remove_file(&index).map_err(|e| format!("removing {}: {e}", index.display()))?;
    recorder(
        repo,
        &["commit-tree", &tree, "-m", "untracked files on main"],
    )
}

/// A note (§2): a commit on what was taken, its tree, carrying what to put
/// back in its trailers.
fn note(
    repo: &mut DemoRepo,
    taken: &str,
    summary: &str,
    trailers: &[(&str, &str)],
) -> Result<(), String> {
    let tree = repo.git(&["rev-parse", &format!("{taken}^{{tree}}")])?;
    let message = message(summary, trailers);
    let value = recorder(repo, &["commit-tree", &tree, "-p", taken, "-m", &message])?;
    append(repo, &value, summary)
}

/// The record's next line.
fn append(repo: &mut DemoRepo, value: &str, summary: &str) -> Result<(), String> {
    recorder(
        repo,
        &[
            "update-ref",
            "--create-reflog",
            "-m",
            summary,
            RECORD_REF,
            value,
        ],
    )
    .map(drop)
}

fn message(summary: &str, trailers: &[(&str, &str)]) -> String {
    let lines: Vec<String> = trailers
        .iter()
        .map(|(key, value)| format!("{key}: {value}"))
        .collect();
    format!("{summary}\n\n{}", lines.join("\n"))
}

/// git as the record's writer: its own identity, with no email (§2).
fn recorder(repo: &mut DemoRepo, args: &[&str]) -> Result<String, String> {
    let work = repo.work.clone();
    let mut cmd = repo.command(&work, args);
    cmd.env("GIT_AUTHOR_NAME", "Platitude GG")
        .env("GIT_AUTHOR_EMAIL", "")
        .env("GIT_COMMITTER_NAME", "Platitude GG")
        .env("GIT_COMMITTER_EMAIL", "");
    ran(&mut cmd)
}

fn ran(cmd: &mut std::process::Command) -> Result<String, String> {
    let out = output_of(cmd)?;
    if !out.status.success() {
        return Err(format!(
            "{cmd:?} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// The commits on main after the last discard.
const LATER_COMMITS: u32 = 40;
