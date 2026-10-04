//! SessionStart: where the session sits and where the seats stand.

use super::payload::string_field;
use crate::seats::{
    self, Held, Identity, SEATS, Standing, commits_in, how_claims_move, take_seat, whose,
    worktree_root,
};
use crate::subprocess::git_query;

/// SessionStart: plain stdout becomes session context for this event.
pub(super) fn session_start(input: &str) -> Result<(), String> {
    let cwd = string_field(input, "cwd").unwrap_or_default();
    // Installed on every start so no clone or seat is without it. A
    // failure is printed, not returned: the greeting still has to be given.
    match crate::gate::hooks::install(std::path::Path::new(&cwd)) {
        Ok(_) => {}
        Err(why) => println!("The gate's git hook could not be installed: {why}"),
    }
    // A session's environment must carry the mark; without it the gate
    // silently holds nothing (gate::hooks).
    if std::env::var_os(crate::gate::hooks::SESSION).is_none_or(|mark| mark.is_empty()) {
        println!(
            "The gate cannot tell this session's git from the user's: {} is not in this \
             environment, so refs/heads/main is open to any git this session runs. Say so \
             and leave main alone — the name the gate reads is in \
             crates/xtask/src/gate/hooks.rs, and it is the whole of what holds main to the \
             pre-merge tests.",
            crate::gate::hooks::SESSION
        );
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
             The claim is the only thing that decides who gets a seat; the \
             letter comes from it. \
             Make the edit in the seat you are given, and report the branch \
             as ready to merge. {seats}"
        ),
        Some(root) => {
            let name = root.rsplit('/').next().unwrap_or_default();
            if SEATS.contains(&name) {
                let session = string_field(input, "session_id").unwrap_or_default();
                // A conversation carried on — compacted, resumed — takes no
                // claim: its landing handed the seat back, and its next edit
                // claims it again if the work goes on (`seats::how_claims_move`).
                if begins(input)
                    && let Some(note) = claim_at_start(&cwd, &session)
                {
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

/// Whether this start begins a conversation (`startup`, `/clear`) rather
/// than carries one on (`resume`, `compact`).
fn begins(input: &str) -> bool {
    !matches!(
        string_field(input, "source").as_deref(),
        Some("resume" | "compact")
    )
}

/// A session started inside an unclaimed seat claims it (the `claude
/// --worktree <letter>` road). A seat somebody else holds stays theirs.
fn claim_at_start(cwd: &str, session: &str) -> Option<String> {
    // git resolves the lock's argument by exact top-level path; from a
    // subdirectory the claim would fail silently.
    let root = crate::seats::worktree_root(cwd).unwrap_or_else(|| cwd.to_string());
    let me = Identity::current(Some(session));
    // A claim that could not be written is left to the survey: the
    // session sits here either way.
    match take_seat(&root, &root, &me, Held::BySession) {
        Standing::Ours | Standing::Free => None,
        Standing::Foreign(reason) | Standing::Stale(reason) => Some(format!(
            "This seat is {}. This session is {}. {} Two sessions in one seat commit on \
             top of one another, so move to a seat of your own (`cargo xtask seat`).",
            whose(&reason),
            me.mark(),
            how_claims_move(),
        )),
    }
}

/// One line about the seat this session sits in.
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

/// The seat roster in one line, from the survey `cargo xtask seats` prints.
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

struct SeatBuckets {
    free: Vec<String>,
    pending: Vec<String>,
    in_use: Vec<String>,
    missing: Vec<&'static str>,
}

/// Sorts a survey into the greeting's buckets. Pure so the tests can hand
/// it surveys git never produced.
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
        // A claim is a session in the seat whatever became of its
        // process (`seats::claim`).
        if state.locked {
            let tree = if state.on_disk { "" } else { ", no tree" };
            buckets.in_use.push(format!("{name} (locked{tree})"));
            continue;
        }
        // A letter whose tree went away is not unused: its work is
        // unreachable until a takeover grows the tree back.
        if !state.on_disk {
            let ahead = state.ahead.unwrap_or(0);
            buckets
                .pending
                .push(format!("{name} ({} +{ahead}, no tree)", state.branch));
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
                // For the reader only: the assignment itself puts a
                // handed-out seat at main's tip.
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
            on_disk: true,
            locked,
            lock_reason: String::new(),
            ahead: Some(ahead),
            behind: Some(behind),
            dirty: Some(dirty),
            index_age: None,
        }
    }

    fn seat(name: &'static str, state: SeatState) -> Seat {
        Seat {
            name,
            state: Some(state),
        }
    }

    /// A letter git lists and the disk does not have, carrying `ahead`
    /// commits on its own branch.
    fn treeless(name: &'static str, locked: bool, ahead: u32) -> Seat {
        seat(
            name,
            SeatState {
                on_disk: false,
                behind: None,
                dirty: None,
                ..surveyed(&format!("worktree-{name}"), locked, ahead, 0, 0)
            },
        )
    }

    fn never_used(name: &'static str) -> Seat {
        Seat { name, state: None }
    }

    /// The claim is the session; nothing here asks after a process.
    #[test]
    fn a_claimed_seat_is_in_use_even_at_main_with_nothing_in_it() {
        let buckets = seat_buckets(&[seat("a", surveyed("worktree-a", true, 0, 0, 0))]);
        assert_eq!(buckets.in_use, vec!["a (locked)"]);
        assert!(buckets.free.is_empty(), "{:?}", buckets.free);
    }

    #[test]
    fn a_dirty_seat_is_in_use_not_free() {
        let buckets = seat_buckets(&[seat("a", surveyed("worktree-a", false, 0, 0, 13))]);
        assert_eq!(buckets.in_use, vec!["a (13 uncommitted change(s))"]);
        assert!(buckets.free.is_empty(), "{:?}", buckets.free);
    }

    #[test]
    fn sorts_a_survey_into_the_greeting_buckets() {
        let survey = vec![
            seat("a", surveyed("worktree-a", false, 1, 0, 13)),
            seat("b", surveyed("worktree-b", false, 0, 0, 0)),
            seat("c", surveyed("", false, 0, 3, 0)),
            never_used("d"),
            seat("e", surveyed("worktree-e", false, 0, 2, 0)),
            seat("f", surveyed("worktree-f", true, 0, 0, 0)),
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

    /// Claimed, a treeless letter reads as somebody's, not as waiting for
    /// merge.
    #[test]
    fn a_letter_whose_tree_went_away_is_waiting_for_merge_not_uncreated() {
        let buckets = seat_buckets(&[treeless("c", false, 17)]);
        assert_eq!(buckets.pending, vec!["c (worktree-c +17, no tree)"]);
        assert!(buckets.missing.is_empty(), "{:?}", buckets.missing);

        let buckets = seat_buckets(&[treeless("c", true, 17)]);
        assert_eq!(buckets.in_use, vec!["c (locked, no tree)"]);
        assert!(buckets.pending.is_empty(), "{:?}", buckets.pending);
    }

    #[test]
    fn a_seat_git_cannot_answer_for_is_not_offered() {
        let buckets = seat_buckets(&[seat(
            "e",
            SeatState {
                ahead: None,
                behind: None,
                dirty: None,
                ..surveyed("worktree-e", false, 0, 0, 0)
            },
        )]);
        assert_eq!(buckets.in_use, vec!["e (state unreadable)"]);
        assert!(buckets.free.is_empty(), "{:?}", buckets.free);
    }
}
