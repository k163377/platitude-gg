//! SessionStart: where the session sits, where the seats stand, and which
//! free letter it should take.

use super::payload::string_field;
use crate::seats::{
    self, Identity, SEATS, Standing, claim_liveness, commits_in, take_seat, worktree_root,
};
use crate::subprocess::git_query;

/// SessionStart: sessions opened in the primary checkout get the worktree
/// rule injected, and every session gets told where the seats stand, so
/// taking a free one needs no survey. Plain stdout becomes session
/// context for this event.
pub(super) fn session_start(input: &str) -> Result<(), String> {
    let cwd = string_field(input, "cwd").unwrap_or_default();
    // The hook that holds main to the gate's stamp goes in on every
    // session start, so no clone and no seat is ever without it. A
    // failure is said, not fatal: the greeting still has to be given.
    match crate::gate::install(std::path::Path::new(&cwd)) {
        Ok(_) => {}
        Err(why) => println!("The gate's git hook could not be installed: {why}"),
    }
    let seats = seat_report(&cwd).unwrap_or_default();
    match worktree_root(&cwd) {
        None => println!(
            "This session runs in the primary checkout, and the primary \
             checkout is only ever read. Every write belongs in a worktree \
             seat (`claude --worktree <letter>`, or EnterWorktree by path) \
             — implementation, documents, settings and the shared session \
             rules alike: parallel sessions fight over target/ and the \
             release exe, and they keep reaching for the same files, so a \
             direct commit to main collides with theirs (CLAUDE.md ビルド・\
             テスト / Git 運用). Take a seat, make the edit there, and \
             report the branch as ready to merge. {seats}"
        ),
        Some(root) => {
            let name = root.rsplit('/').next().unwrap_or_default();
            if SEATS.contains(&name) {
                let session = string_field(input, "session_id").unwrap_or_default();
                if let Some(note) = claim_at_start(&cwd, &session) {
                    println!("{note}");
                }
                if let Some(stand) = seat_stand(&cwd, &seats) {
                    println!("{stand}");
                }
            } else {
                println!(
                    "This session runs in worktree '{name}', outside the \
                     seat roster a-f. Continue this branch's pending work if \
                     that is what the session is for; otherwise take a seat \
                     with EnterWorktree by path (CLAUDE.md ビルド・テスト). \
                     {seats}"
                );
            }
        }
    }
    Ok(())
}

/// A session that starts inside an unclaimed seat claims it, so the
/// `claude --worktree <letter>` road is covered the same way EnterWorktree
/// is. A seat somebody else holds gets told so, not fought over.
fn claim_at_start(cwd: &str, session: &str) -> Option<String> {
    // The lock names the worktree by its top-level path (git resolves the
    // argument by exact real path); a session started in a subdirectory
    // would otherwise fail the claim silently.
    let root = crate::seats::worktree_root(cwd).unwrap_or_else(|| cwd.to_string());
    let me = Identity::current(Some(session));
    // A claim that could not be written is a survey concern: the session
    // is already sitting here either way, and the greeting still says
    // where the seat stands.
    match take_seat(&root, &root, &me) {
        Standing::Ours | Standing::Free => None,
        Standing::Foreign(reason) | Standing::Stale(reason) => Some(format!(
            "This seat is held by another claim. The lock says: {reason}. This \
             session is {}. {} Two sessions in one seat commit on top of one \
             another, so move to a seat of your own rather than working here.",
            me.mark(),
            claim_liveness(&reason),
        )),
    }
}

/// One line about the seat this session sits in. A merged seat starts
/// over from main's tip; a seat carrying unmerged commits is a merge
/// waiting to happen, and only its own continuation should build on it.
fn seat_stand(cwd: &str, seats: &str) -> Option<String> {
    let branch = git_query(cwd, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    let ahead = commits_in(cwd, &format!("main..{branch}"))?;
    if ahead > 0 {
        return Some(format!(
            "This seat's branch {branch} carries {ahead} unmerged commit(s) \
             — continue that work, or take another seat and leave this one \
             for its merge. {seats}"
        ));
    }
    let behind = commits_in(cwd, &format!("{branch}..main"))?;
    (behind > 0).then(|| {
        format!(
            "This seat's branch {branch} is merged and {behind} behind main \
             — start with `git reset --hard main` so the work begins at the \
             tip (CLAUDE.md ビルド・テスト)."
        )
    })
}

/// The seat roster in one line, read from the same survey `cargo xtask
/// seats` prints, so the answer is the repository's and not a guess.
fn seat_report(cwd: &str) -> Option<String> {
    let survey = seats::survey(cwd)?;
    let buckets = seat_buckets(&survey);
    let mut parts = Vec::new();
    if !buckets.free.is_empty() {
        parts.push(format!("free: {}", buckets.free.join(", ")));
    }
    if !buckets.pending.is_empty() {
        parts.push(format!("waiting for merge: {}", buckets.pending.join(", ")));
    }
    if !buckets.in_use.is_empty() {
        parts.push(format!("in use: {}", buckets.in_use.join(", ")));
    }
    if !buckets.missing.is_empty() {
        parts.push(format!("not created yet: {}", buckets.missing.join(", ")));
    }
    let mut report = format!("Worktree seats — {}.", parts.join("; "));
    if let Some(pick) = spread_pick(&buckets.takeable) {
        report.push_str(&format!(
            " Take seat {pick} this session — the recommendation is \
             randomized so sessions started in one burst spread out. \
             Sitting down *is* the check: enter {pick} straight away and \
             let the claim answer, because the hook takes `git worktree \
             lock` on the way in and denies the call when somebody already \
             holds it. A denial is that answer, not a failure — move to \
             another free letter and enter it the same way. Do not survey \
             first: this listing is from the session's start and `cargo \
             xtask seats` is only ever a snapshot, so a seat either one \
             calls free can be gone by the time you act on it. `seats` is \
             for reading how the seats stand, never for deciding whether \
             to sit."
        ));
    }
    Some(report)
}

/// The greeting's buckets, and the seats a new session may take.
struct SeatBuckets {
    free: Vec<String>,
    pending: Vec<String>,
    in_use: Vec<String>,
    missing: Vec<&'static str>,
    takeable: Vec<&'static str>,
}

/// Sorts a survey into the greeting's buckets. A lock is a session's own
/// claim, uncommitted changes are a session's work in progress, and
/// commits ahead of main are a merge waiting to happen. Only a seat with
/// none of those is takeable. Pure so the tests can hand it surveys git
/// never produced.
fn seat_buckets(survey: &[seats::Seat]) -> SeatBuckets {
    let mut buckets = SeatBuckets {
        free: Vec::new(),
        pending: Vec::new(),
        in_use: Vec::new(),
        missing: Vec::new(),
        takeable: Vec::new(),
    };
    for seat in survey {
        let name = seat.name;
        let Some(state) = &seat.state else {
            buckets.missing.push(name);
            buckets.takeable.push(name);
            continue;
        };
        if state.locked {
            // A claim outlives a session that died with it: index-age is
            // the tell, and hours of stillness under a lock reads as a
            // leftover, not a session.
            let idle = state
                .index_age
                .filter(|age| age.as_secs() >= 3600)
                .map(|age| {
                    format!(
                        ", idle {} — stale? `git worktree unlock \
                         .claude/worktrees/{name}` if its session is gone",
                        seats::format_age(Some(age))
                    )
                })
                .unwrap_or_default();
            buckets.in_use.push(format!("{name} (locked{idle})"));
            continue;
        }
        match (state.ahead, state.behind, state.dirty) {
            (Some(ahead), _, dirty) if ahead > 0 => {
                let branch = if state.branch.is_empty() {
                    "detached"
                } else {
                    state.branch.as_str()
                };
                let uncommitted = match dirty {
                    Some(dirty) if dirty > 0 => format!(", {dirty} uncommitted"),
                    _ => String::new(),
                };
                buckets
                    .pending
                    .push(format!("{name} ({branch} +{ahead}{uncommitted})"));
            }
            (Some(0), _, Some(dirty)) if dirty > 0 => {
                buckets
                    .in_use
                    .push(format!("{name} ({dirty} uncommitted change(s))"));
            }
            (Some(0), Some(behind), Some(0)) => {
                if state.branch.is_empty() {
                    // Detached HEAD: nothing pre-git guards stands on it, so
                    // treat it like a merged seat that wants resetting to
                    // the tip.
                    buckets
                        .free
                        .push(format!("{name} (detached — reset --hard main first)"));
                } else if behind > 0 {
                    buckets
                        .free
                        .push(format!("{name} (reset --hard main first)"));
                } else {
                    buckets.free.push(format!("{name} (at main)"));
                }
                buckets.takeable.push(name);
            }
            _ => buckets.in_use.push(format!("{name} (state unreadable)")),
        }
    }
    buckets
}

/// One takeable seat, chosen off the clock's nanoseconds. Sessions started
/// in one burst all read the same inventory, and a deterministic "first
/// free letter" would send every one of them to the same seat. A spread
/// recommendation lets a burst self-assign; the losers of any remaining
/// race are told above to move on rather than retry.
fn spread_pick(takeable: &[&'static str]) -> Option<&'static str> {
    if takeable.is_empty() {
        return None;
    }
    let entropy = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.subsec_nanos() as usize)
        .unwrap_or(0);
    takeable.get(entropy % takeable.len()).copied()
}

#[cfg(test)]
mod tests {
    use super::seat_buckets;
    use crate::seats::{Seat, SeatState};

    fn surveyed(branch: &str, locked: bool, ahead: u32, behind: u32, dirty: usize) -> SeatState {
        SeatState {
            branch: branch.to_string(),
            locked,
            lock_reason: String::new(),
            ahead: Some(ahead),
            behind: Some(behind),
            dirty: Some(dirty),
            index_age: None,
        }
    }

    #[test]
    fn an_hour_idle_lock_is_flagged_as_maybe_stale() {
        let mut state = surveyed("worktree-a", true, 0, 0, 0);
        state.index_age = Some(std::time::Duration::from_secs(6 * 3600));
        let survey = vec![Seat {
            name: "a",
            state: Some(state),
        }];
        let buckets = seat_buckets(&survey);
        assert_eq!(buckets.in_use.len(), 1, "{:?}", buckets.in_use);
        assert!(buckets.in_use[0].contains("stale?"), "{:?}", buckets.in_use);
        assert!(buckets.takeable.is_empty(), "{:?}", buckets.takeable);
    }

    #[test]
    fn a_dirty_seat_is_in_use_not_free() {
        let survey = vec![Seat {
            name: "a",
            state: Some(surveyed("worktree-a", false, 0, 0, 13)),
        }];
        let buckets = seat_buckets(&survey);
        assert_eq!(buckets.in_use, vec!["a (13 uncommitted change(s))"]);
        assert!(buckets.free.is_empty(), "{:?}", buckets.free);
        assert!(buckets.takeable.is_empty(), "{:?}", buckets.takeable);
    }

    #[test]
    fn sorts_a_survey_into_the_greeting_buckets() {
        let survey = vec![
            Seat {
                name: "a",
                state: Some(surveyed("worktree-a", false, 1, 0, 13)),
            },
            Seat {
                name: "b",
                state: Some(surveyed("worktree-b", false, 0, 0, 0)),
            },
            Seat {
                name: "c",
                state: Some(surveyed("", false, 0, 3, 0)),
            },
            Seat {
                name: "d",
                state: None,
            },
            Seat {
                name: "e",
                state: Some(surveyed("worktree-e", false, 0, 2, 0)),
            },
            Seat {
                name: "f",
                state: Some(surveyed("worktree-f", true, 0, 0, 0)),
            },
        ];
        let buckets = seat_buckets(&survey);
        assert_eq!(buckets.pending, vec!["a (worktree-a +1, 13 uncommitted)"]);
        assert_eq!(
            buckets.free,
            vec![
                "b (at main)",
                "c (detached — reset --hard main first)",
                "e (reset --hard main first)",
            ]
        );
        assert_eq!(buckets.in_use, vec!["f (locked)"]);
        assert_eq!(buckets.missing, vec!["d"]);
        assert_eq!(buckets.takeable, vec!["b", "c", "d", "e"]);
    }

    #[test]
    fn a_seat_git_cannot_answer_for_is_not_offered() {
        let survey = vec![Seat {
            name: "e",
            state: Some(SeatState {
                branch: "worktree-e".to_string(),
                locked: false,
                lock_reason: String::new(),
                ahead: None,
                behind: None,
                dirty: None,
                index_age: None,
            }),
        }];
        let buckets = seat_buckets(&survey);
        assert_eq!(buckets.in_use, vec!["e (state unreadable)"]);
        assert!(buckets.takeable.is_empty(), "{:?}", buckets.takeable);
    }
}
