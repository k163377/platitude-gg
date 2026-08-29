//! The one history longer than the graph's own window, and the shapes
//! the window's cut and the stand-in for a scrolled-off HEAD have to be
//! looked at against.

use std::time::{SystemTime, UNIX_EPOCH};

use super::repo::DemoRepo;

/// Commits `deep` puts down. Past `session::DEFAULT_LOG_LIMIT` (2000) by
/// enough that the cut lands well inside the history rather than on its
/// oldest row — a window that stopped one commit short of the end says
/// the same thing as one that stopped in the middle, and only the second
/// is what the footer is for.
const DEEP_COMMITS: u64 = 2100;
/// How far apart they sit. Its own march rather than the shared
/// half-hour tick: 2100 of those would land the newest commit two months
/// from now.
const DEEP_STEP_SECS: u64 = 60;

/// One commit in the stream: the ref it lands on, its own mark, the
/// parent it takes (`None` chains onto whatever the ref already holds),
/// when it was written, and what it says.
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

/// When the first of the `DEEP_COMMITS` was written, so a caller adding a
/// second line can date it against the same march.
fn deep_first(now: u64) -> u64 {
    // The newest commit lands an hour ago, and the rest march back from
    // it — the same "past → now" shape every other preset has.
    now - 3600 - DEEP_COMMITS * DEEP_STEP_SECS
}

/// More commits than the graph loads at once, so the pane has to say
/// where it stopped (the window cut's lanes and its one line).
///
/// Written as a single `fast-import` stream rather than a commit at a
/// time: 2100 git processes cost about a minute of every run that asks
/// for this shape, and one costs a tenth of a second (measured,
/// Windows: 92ms for this history). One blob serves every commit — what
/// this preset is for is the *count*, and a tree that changed on every
/// step would only make the import bigger.
pub(super) fn deep(repo: &mut DemoRepo) -> Result<(), String> {
    deep_history(repo, false)
}

/// The stream both deep presets are built from. `forked` adds the second
/// line described on [`deep_detached`].
fn deep_history(repo: &mut DemoRepo, forked: bool) -> Result<(), String> {
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
    if forked {
        // Marks well clear of main's (2..=DEEP_COMMITS + 1), and dated
        // against main's own march so the tip lands a little under the
        // newest commits rather than at the far end of the window.
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
    repo.git_stdin(&["fast-import", "--quiet"], &stream)?;
    // fast-import writes the ref and nothing else: the work tree is still
    // the empty one `init` left, and a repository whose HEAD disagrees
    // with its files is not the shape any pane is meant to read.
    repo.git(&["reset", "--hard", "main"])?;
    Ok(())
}

/// How far down the same history the detached variant stands. Far enough
/// that the row cannot be brought on screen at the same time as the
/// newest one, whatever the window's height.
const DEEP_DETACH_BACK: u64 = 800;

/// Where the second line forks off, counted back from the newest commit.
/// **Older than the window's own cut** (`DEEP_COMMITS` less the limit),
/// so the fork itself is never loaded: the two lanes then run the whole
/// height of the graph and both are still going at the bottom, which is
/// what puts more than one lane into the footer's fade.
const DEEP_FORK_BACK: u64 = 2080;

/// Commits on that second line. Enough to be a line rather than a spur,
/// few enough that its tip stays well below the newest commits.
const DEEP_SIDE_COMMITS: u64 = 6;

/// Where the second line's fast-import marks start — clear of main's,
/// which run to `DEEP_COMMITS + 1`.
const DEEP_SIDE_MARK: u64 = 100_000;

/// The same history in every shape the stand-in and the fades below it
/// have to survive at once: the working tree standing a long way
/// **down** it, standing on no branch, and a graph **wider than one
/// lane**.
///
/// - it is the one shape in which the stand-in rides the **bottom** edge
///   (`GraphHeadPin`) — scrolled to the newest commits, the row it stands
///   for is eight hundred rows below the viewport
/// - and the only preset where that stand-in wears the detached marker
/// - the second line forks below the window's cut and its tip sits above
///   HEAD, so **both lanes are live at both ends**: the stand-in has a
///   neighbouring lane running past under it, which is what shows that
///   it draws its own lanes rather than the ones it is covering
pub(super) fn deep_detached(repo: &mut DemoRepo) -> Result<(), String> {
    deep_history(repo, true)?;
    repo.git(&["switch", "--detach", &format!("main~{DEEP_DETACH_BACK}")])?;
    Ok(())
}
