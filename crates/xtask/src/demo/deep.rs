//! Histories longer than the graph's window (its cut, the stand-in for a
//! scrolled-off HEAD, a branch off the window altogether), and one long
//! enough that rewriting it takes seconds.

use std::time::{SystemTime, UNIX_EPOCH};

use super::repo::DemoRepo;

/// Commits `deep` puts down: past `session::DEFAULT_LOG_LIMIT` (2000) by
/// enough that the cut lands well inside the history.
const DEEP_COMMITS: u64 = 2100;
/// How far apart they sit. Not the shared half-hour tick, which would put
/// the newest commit months in the future.
const DEEP_STEP_SECS: u64 = 60;

/// One commit in the stream. `from: None` chains onto whatever the ref
/// already holds.
fn deep_commit(stream: &mut String, on: &str, mark: u64, from: Option<u64>, when: u64, msg: &str) {
    stream.push_str(&format!("commit refs/heads/{on}\n"));
    stream.push_str(&format!("mark :{mark}\n"));
    stream.push_str(&format!(
        "author Demo User <demo@example.com> {when} +0000\n"
    ));
    stream.push_str(&format!(
        "committer Demo User <demo@example.com> {when} +0000\n"
    ));
    stream.push_str(&format!("data {}\n{msg}\n", msg.len()));
    if let Some(parent) = from {
        stream.push_str(&format!("from :{parent}\n"));
    }
    stream.push_str("M 100644 :1 f.txt\n\n");
}

/// When the first of the `DEEP_COMMITS` was written.
fn deep_first(now: u64) -> u64 {
    now - 3600 - DEEP_COMMITS * DEEP_STEP_SECS
}

/// More commits than the graph loads at once, so the pane has to say
/// where it stopped.
///
/// One `fast-import` stream, not a process per commit
/// (ci/baseline/code-costs-windows-x64.md §コーパス生成). One blob serves
/// every commit: what this preset is for is the count.
pub(super) fn deep(repo: &mut DemoRepo) -> Result<(), String> {
    deep_history(repo, SideLine::None)
}

/// The second line a deep history may carry beside `main` — all that
/// separates the three presets built from it.
enum SideLine {
    /// One lane, the whole way down (`deep`).
    None,
    /// Forked below the window's cut with its tip up among the newest
    /// commits, so both lanes are live at both ends (`deep_detached`).
    Live,
    /// Forked below the cut and left down there, so the branch's own tip
    /// is on no row the graph draws (`deep_parked`).
    Parked,
}

/// The stream all three deep presets are built from.
fn deep_history(repo: &mut DemoRepo, side: SideLine) -> Result<(), String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    let first = deep_first(now);
    let mut stream = String::from("blob\nmark :1\ndata 3\nv1\n\n");
    for n in 1..=DEEP_COMMITS {
        let from = (n > 1).then_some(n);
        let message = format!("chore: filler commit {n}");
        deep_commit(
            &mut stream,
            "main",
            n + 1,
            from,
            first + n * DEEP_STEP_SECS,
            &message,
        );
    }
    match side {
        SideLine::None => {}
        SideLine::Live => {
            // Dated against main's march so the tip lands a little under
            // the newest commits.
            let fork = DEEP_COMMITS - DEEP_FORK_BACK;
            for n in 1..=DEEP_SIDE_COMMITS {
                let from = if n == 1 {
                    fork + 1
                } else {
                    DEEP_SIDE_MARK + n - 1
                };
                let message = format!("feat: side work {n}");
                deep_commit(
                    &mut stream,
                    "side",
                    DEEP_SIDE_MARK + n,
                    Some(from),
                    first + (DEEP_COMMITS - DEEP_SIDE_COMMITS - 4 + n) * DEEP_STEP_SECS,
                    &message,
                );
            }
        }
        SideLine::Parked => {
            // Half a step past its parent, so it takes a place of its own
            // in the date order.
            let fork = DEEP_COMMITS - DEEP_FORK_BACK;
            deep_commit(
                &mut stream,
                DEEP_PARKED_BRANCH,
                DEEP_PARKED_MARK,
                Some(fork + 1),
                first + fork * DEEP_STEP_SECS + DEEP_STEP_SECS / 2,
                "feat: work left below the window",
            );
        }
    }
    repo.git_stdin(&["fast-import", "--quiet"], &stream)?;
    // fast-import writes the ref and nothing else: the working tree is still
    // the empty one `init` left.
    repo.git(&["reset", "--hard", "main"])?;
    Ok(())
}

/// How far down the same history the detached variant stands: too far to
/// share the screen with the newest row at any window height.
const DEEP_DETACH_BACK: u64 = 800;

/// Where a second line forks off, counted back from the newest commit —
/// below the window's cut (`DEEP_COMMITS` less the limit). The live line
/// is loaded without its fork, so both lanes still run into the footer's
/// fade; the parked one is dated down there too, so none of it loads.
const DEEP_FORK_BACK: u64 = 2080;

/// Commits on that second line. Enough to be a line, few enough that
/// its tip stays well below the newest commits.
const DEEP_SIDE_COMMITS: u64 = 6;

/// Where the second line's fast-import marks start — clear of main's,
/// which run to `DEEP_COMMITS + 1`.
const DEEP_SIDE_MARK: u64 = 100_000;

/// What the parked line is called — the argument a verb aiming at this
/// shape is given.
pub(super) const DEEP_PARKED_BRANCH: &str = "parked";

/// The parked line's one mark — clear of main's and of the side line's.
const DEEP_PARKED_MARK: u64 = 200_000;

/// `deep`, with one branch parked below the window's cut: its tip is on no
/// row the graph draws and is not an ancestor of HEAD.
///
/// The one shape where the delete row's early answer can only come from
/// git: a drawn branch is answered off the rows
/// (`GraphModel::branch_delete_merged`), one off the window by
/// `merge-base` (`RepoSession::check_branch_delete`). Unmerged, so the
/// answer is the refusal that turns the row into the held `-D`
/// (`delete-branch-early-far`).
pub(super) fn deep_parked(repo: &mut DemoRepo) -> Result<(), String> {
    deep_history(repo, SideLine::Parked)
}

/// `deep` with a changed file on top: the performance scenario's diff.
pub(super) fn perf(repo: &mut DemoRepo) -> Result<(), String> {
    deep(repo)?;
    repo.commit(
        "f.txt",
        "A changed file for the visible diff.\n",
        "test: performance scenario",
    )?;
    Ok(())
}

/// `deep` with two revisions of a coloured file on top: the performance
/// sequence's diffs.
pub(super) fn perf_sequence(repo: &mut DemoRepo) -> Result<(), String> {
    deep(repo)?;
    for revision in 0..2 {
        let mut source = String::from("class Benchmark {\n");
        for line in 0..180 {
            source.push_str(&format!(
                "    fun item{line}(): Int = {}\n",
                line + revision
            ));
        }
        source.push_str("}\n");
        repo.commit("bench.kt", &source, "test: coloured performance sequence")?;
    }
    Ok(())
}

/// The same history with HEAD detached a long way down it and a second
/// lane live at both ends — the one shape where the stand-in
/// (`GraphHeadPin`) rides the bottom edge, wears the detached marker, and
/// has a neighbouring lane running past under it.
pub(super) fn deep_detached(repo: &mut DemoRepo) -> Result<(), String> {
    deep_history(repo, SideLine::Live)?;
    repo.git(&["switch", "--detach", &format!("main~{DEEP_DETACH_BACK}")])?;
    Ok(())
}

/// Commits on the line `replay` rewrites. A replay pays a process per
/// commit (ci/baseline/code-costs-windows-x64.md §git のプロセス代), so
/// three hundred stand for seconds: long enough to catch the running
/// state, short enough to pay for on both sides of every `check`.
const REPLAY_COMMITS: u64 = 300;

/// Where `replay`'s blob marks start — clear of its commit marks, which
/// run to `REPLAY_COMMITS + 1` and then to [`REPLAY_BASE_MARK`].
const REPLAY_BLOB_MARK: u64 = 100_000;

/// The mark of the one commit the fork never saw.
const REPLAY_BASE_MARK: u64 = 1_000;

/// One commit with its own blob at its own path: a replay drops a commit
/// whose diff is empty, and distinct paths collide over nothing.
fn replay_commit(
    stream: &mut String,
    on: &str,
    mark: u64,
    from: Option<u64>,
    when: u64,
    path: &str,
    msg: &str,
) {
    let blob = REPLAY_BLOB_MARK + mark;
    let body = format!("{path}\n");
    stream.push_str(&format!(
        "blob\nmark :{blob}\ndata {}\n{body}\n",
        body.len()
    ));
    stream.push_str(&format!("commit refs/heads/{on}\n"));
    stream.push_str(&format!("mark :{mark}\n"));
    stream.push_str(&format!(
        "author Demo User <demo@example.com> {when} +0000\n"
    ));
    stream.push_str(&format!(
        "committer Demo User <demo@example.com> {when} +0000\n"
    ));
    stream.push_str(&format!("data {}\n{msg}\n", msg.len()));
    if let Some(parent) = from {
        stream.push_str(&format!("from :{parent}\n"));
    }
    stream.push_str(&format!("M 100644 :{blob} {path}\n\n"));
}

/// A rewrite that can be watched happening: `main` carries
/// [`REPLAY_COMMITS`] of its own and `base` one commit the fork never
/// saw, so `rebase base` replays every one of them.
///
/// The only preset whose rebase is still running when photographed
/// (`replay-running`); every other one is already stopped or over.
pub(super) fn replay(repo: &mut DemoRepo) -> Result<(), String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    let first = now - 3600 - (REPLAY_COMMITS + 1) * DEEP_STEP_SECS;
    let mut stream = String::new();
    replay_commit(
        &mut stream,
        "main",
        1,
        None,
        first,
        "notes.txt",
        "docs: the note both lines start from",
    );
    for n in 1..=REPLAY_COMMITS {
        replay_commit(
            &mut stream,
            "main",
            n + 1,
            Some(n),
            first + n * DEEP_STEP_SECS,
            &format!("n/{n:04}.txt"),
            &format!("feat: step {n}"),
        );
    }
    // Off the root, so the whole of the other line is what a rebase has
    // to replay.
    replay_commit(
        &mut stream,
        "base",
        REPLAY_BASE_MARK,
        Some(1),
        first + (REPLAY_COMMITS + 1) * DEEP_STEP_SECS,
        "base.txt",
        "chore: the base moves on without them",
    );
    repo.git_stdin(&["fast-import", "--quiet"], &stream)?;
    // fast-import writes refs and nothing else (`deep_history`).
    repo.git(&["reset", "--hard", "main"])?;
    Ok(())
}
