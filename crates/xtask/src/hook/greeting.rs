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
             seat — implementation, documents, settings and the shared \
             session rules alike: parallel sessions fight over target/ and \
             the release exe, and they keep reaching for the same files, so \
             a direct commit to main collides with theirs (CLAUDE.md \
             ビルド・テスト / Git 運用). Nothing has to be decided about \
             that now: the first edit that would land here is held, and \
             `cargo xtask seat` is what answers it — the roster claims a \
             free letter for this session and prints the path to enter. \
             Never name a letter yourself, and do not survey for one; the \
             claim is the only thing that ever decided who gets a seat. \
             Make the edit in the seat you are given, and report the branch \
             as ready to merge. {seats}"
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
                     with `cargo xtask seat` and enter the path it prints \
                     (CLAUDE.md ビルド・テスト). {seats}"
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
    Some(format!(
        "Worktree seats, as they stood when this session began — {}. That \
         is a snapshot for the reader, not a menu to pick from: `cargo \
         xtask seat` is what hands this session a seat, and it decides \
         behind the claim, where the decision is real.",
        parts.join("; ")
    ))
}

/// The greeting's buckets.
struct SeatBuckets {
    free: Vec<String>,
    pending: Vec<String>,
    in_use: Vec<String>,
    missing: Vec<&'static str>,
}

/// Sorts a survey into the greeting's buckets. A live claim is a session
/// sitting there, uncommitted changes are work in progress, and commits
/// ahead of main are a merge waiting to happen. Pure so the tests can
/// hand it surveys git never produced.
fn seat_buckets(survey: &[seats::Seat]) -> SeatBuckets {
    let mut buckets = SeatBuckets {
        free: Vec::new(),
        pending: Vec::new(),
        in_use: Vec::new(),
        missing: Vec::new(),
    };
    for seat in survey {
        let name = seat.name;
        let Some(state) = &seat.state else {
            buckets.missing.push(name);
            continue;
        };
        // A claim whose process is gone is not a session in the seat, and
        // saying so is what keeps anyone from unlocking one by hand and
        // landing on top of a session that was only quiet.
        if state.locked && !state.claim_dead {
            buckets.in_use.push(format!("{name} (locked)"));
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
                // Where a free seat stands is the reader's business only:
                // whoever is handed one is put on its branch at main's tip
                // by the assignment itself.
                let at = match (state.branch.is_empty(), behind) {
                    (true, _) => "detached",
                    (false, 0) => "at main",
                    (false, behind) => &format!("{behind} behind main"),
                };
                buckets.free.push(format!("{name} ({at})"));
            }
            _ => buckets.in_use.push(format!("{name} (state unreadable)")),
        }
    }
    buckets
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
            claim_dead: false,
            ahead: Some(ahead),
            behind: Some(behind),
            dirty: Some(dirty),
            index_age: None,
        }
    }

    #[test]
    fn a_claim_whose_session_ended_is_not_a_seat_in_use() {
        let mut state = surveyed("worktree-a", true, 0, 0, 0);
        state.claim_dead = true;
        let survey = vec![Seat {
            name: "a",
            state: Some(state),
        }];
        let buckets = seat_buckets(&survey);
        assert!(buckets.in_use.is_empty(), "{:?}", buckets.in_use);
        assert_eq!(buckets.free, vec!["a (at main)"]);
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
            vec!["b (at main)", "c (detached)", "e (2 behind main)"]
        );
        assert_eq!(buckets.in_use, vec!["f (locked)"]);
        assert_eq!(buckets.missing, vec!["d"]);
    }

    #[test]
    fn a_seat_git_cannot_answer_for_is_not_offered() {
        let survey = vec![Seat {
            name: "e",
            state: Some(SeatState {
                branch: "worktree-e".to_string(),
                locked: false,
                lock_reason: String::new(),
                claim_dead: false,
                ahead: None,
                behind: None,
                dirty: None,
                index_age: None,
            }),
        }];
        let buckets = seat_buckets(&survey);
        assert_eq!(buckets.in_use, vec!["e (state unreadable)"]);
        assert!(buckets.free.is_empty(), "{:?}", buckets.free);
    }
}
