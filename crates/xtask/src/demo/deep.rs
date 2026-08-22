//! The one history longer than the graph's own window, and the shapes
//! the window's cut has to be looked at against.

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

/// More commits than the graph loads at once, so the pane has to say
/// where it stopped (the window cut's lanes and its one line).
///
/// Written as a single `fast-import` stream rather than a commit at a
/// time: 2100 git processes cost about a minute of every run that asks
/// for this shape, and one costs a tenth of a second (2026-08-16 実測
/// Windows: 92ms for this history). One blob serves every commit — what
/// this preset is for is the *count*, and a tree that changed on every
/// step would only make the import bigger.
pub(super) fn deep(repo: &mut DemoRepo) -> Result<(), String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    // The newest commit lands an hour ago, and the rest march back from
    // it — the same "past → now" shape every other preset has.
    let first = now - 3600 - DEEP_COMMITS * DEEP_STEP_SECS;
    let mut stream = String::from("blob\nmark :1\ndata 3\nv1\n\n");
    for n in 1..=DEEP_COMMITS {
        let when = first + n * DEEP_STEP_SECS;
        let message = format!("chore: filler commit {n}");
        stream.push_str("commit refs/heads/main\n");
        stream.push_str(&format!("mark :{}\n", n + 1));
        stream.push_str(&format!(
            "author Demo User <demo@example.com> {when} +0000\n"
        ));
        stream.push_str(&format!(
            "committer Demo User <demo@example.com> {when} +0000\n"
        ));
        stream.push_str(&format!("data {}\n{message}\n", message.len()));
        if n > 1 {
            stream.push_str(&format!("from :{}\n", n));
        }
        stream.push_str("M 100644 :1 f.txt\n\n");
    }
    repo.git_stdin(&["fast-import", "--quiet"], &stream)?;
    // fast-import writes the ref and nothing else: the work tree is still
    // the empty one `init` left, and a repository whose HEAD disagrees
    // with its files is not the shape any pane is meant to read.
    repo.git(&["reset", "--hard", "main"])?;
    Ok(())
}
