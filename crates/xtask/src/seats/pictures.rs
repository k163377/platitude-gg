//! Publishing authority reads the claim without changing the seat.

use super::claim::{Held, Identity, Standing, lock_reason, standing, whose};
use super::commands;
use super::worktrees::{slashed, worktree_root};

/// Whether a run may go on the board from a roster seat: only from one
/// this session holds. A picture claims nothing — after a landing hands
/// the seat back (`land::release_claim`), a picture of the landed work
/// would hold the letter for a session whose work is done, and the roster
/// could hand it to nobody (CLAUDE.md §Git 運用). A run with no session
/// behind it (CI's, a person's own shell) holds no claim and goes through.
pub(crate) fn may_picture(seat: &str) -> Result<(), String> {
    let Ok(cwd) = std::env::current_dir() else {
        return Ok(());
    };
    let Some(root) = worktree_root(&slashed(&cwd)) else {
        return Ok(());
    };
    let me = Identity::current(None);
    // Read from the tree: this run stands in it.
    let standing = standing(lock_reason(&root), &me, Held::BySession);
    picture_verdict(seat, &me, standing)
}

/// [`may_picture`]'s rule, apart from the git that read the claim.
fn picture_verdict(seat: &str, me: &Identity, standing: Standing) -> Result<(), String> {
    if me.session.trim().is_empty() {
        return Ok(());
    }
    match standing {
        Standing::Ours => Ok(()),
        Standing::Free => Err(format!(
            "seat {seat} carries no claim of this session's — its work landed or was handed \
             back, and the letter is the roster's to hand out. A picture claims nothing: \
             pictures the user has to approve go up before the landing, which waits for the \
             approval, and none of the landed work's goes up again after it (CLAUDE.md §Git \
             運用). New work in this tree, or a picture the user asks for now, is claimed by \
             its first edit or by `{}`.",
            commands::TAKE.line()
        )),
        Standing::Foreign(reason) | Standing::Stale(reason) => Err(format!(
            "seat {seat} is {} — a picture taken here would go up under somebody else's work.",
            whose(&reason)
        )),
    }
}

#[cfg(test)]
mod tests {
    /// A landing hands the seat back; a picture taken after it is refused
    /// rather than taking the letter back for a session whose work is done.
    #[test]
    fn a_picture_goes_up_only_from_a_seat_this_session_holds() {
        use super::{Identity, Standing, picture_verdict};
        let me = Identity {
            session: "mine".to_string(),
            pid: None,
            host: None,
        };
        assert!(picture_verdict("a", &me, Standing::Ours).is_ok());
        let after_landing =
            picture_verdict("a", &me, Standing::Free).expect_err("an unclaimed seat");
        assert!(
            after_landing.contains("claims nothing") && after_landing.contains("cargo xtask seat"),
            "{after_landing}"
        );
        let theirs = picture_verdict("b", &me, Standing::Foreign("claude-seat theirs".into()))
            .expect_err("somebody else's seat");
        assert!(theirs.contains("held by session theirs"), "{theirs}");
        // CI's runs and a person's own shell carry no session, and hold no
        // claim to keep.
        let nobody = Identity {
            session: String::new(),
            pid: None,
            host: None,
        };
        assert!(picture_verdict("a", &nobody, Standing::Free).is_ok());
    }
}
