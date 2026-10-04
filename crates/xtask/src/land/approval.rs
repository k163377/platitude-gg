//! A landing waits for the user's look at what its seat put on the board
//! after they asked for it (CLAUDE.md §Git 運用): the landing takes the
//! seat's runs off the board, and a picture put up after the ask shows UI
//! the user has not approved.

use crate::seats::{Identity, seat_entries};

/// Refuses the landing while `branch`'s seat has runs on the board put up
/// after the user's latest message, or once a landing found such runs —
/// the permit then waits for the user's next message, so that taking them
/// off the board approves nothing (`hook::permit`). Nothing is asked
/// without a session (a person landing by hand is the one who looks), a
/// roster seat (the board takes runs from no other tree) or a permit on
/// record.
pub(super) fn awaited(here: &str, listing: &str, branch: &str) -> Result<(), String> {
    let Some(seat) = seat_entries(listing)
        .into_iter()
        .find(|entry| entry.tree.branch == branch)
        .map(|entry| entry.seat)
    else {
        return Ok(());
    };
    let me = Identity::current(None);
    if me.session.trim().is_empty() {
        return Ok(());
    }
    let Some(asked) = crate::hook::permit::asked(here, &me.session) else {
        return Ok(());
    };
    let unseen = crate::shots::put_up_since(here, seat, asked.millis());
    if unseen.is_empty() && !asked.waits() {
        return Ok(());
    }
    crate::hook::permit::wait_for_approval(here, &me.session);
    Err(refusal(seat, &asked.said(), &unseen))
}

fn refusal(seat: &str, asked: &str, unseen: &[String]) -> String {
    let found = if unseen.is_empty() {
        format!(
            "a landing already found runs seat {seat} put on the board after the user's message \
             {asked}, and taking them off the board approves nothing"
        )
    } else {
        let names: Vec<String> = unseen.iter().map(|label| format!("「{label}」")).collect();
        format!(
            "seat {seat} put {} run(s) on the board after the user's message {asked}: {}",
            unseen.len(),
            names.join(", ")
        )
    };
    format!(
        "{found}. A landing takes the seat's runs off the board, so they would go unseen — and a \
         picture put up after the ask shows UI the user has not approved. Nothing has moved: \
         tell the user the board holds them (F5) and ask for their approval; the user's next \
         反映 lands the branch, and putting them up again after a landing is no substitute \
         (CLAUDE.md §Git 運用)."
    )
}

#[cfg(test)]
mod tests {
    use super::refusal;

    #[test]
    fn the_refusal_names_the_pictures_the_message_and_the_way_on() {
        let text = refusal(
            "c",
            "「main反映して」 (5m ago)",
            &["行メニューの影".to_string(), "帯の高さ".to_string()],
        );
        for part in [
            "seat c put 2 run(s)",
            "「main反映して」 (5m ago)",
            "「行メニューの影」, 「帯の高さ」",
            "Nothing has moved",
            "approval",
            "next 反映",
            "CLAUDE.md §Git 運用",
        ] {
            assert!(text.contains(part), "{part:?} missing from: {text}");
        }
        let taken_down = refusal("c", "「main反映して」 (5m ago)", &[]);
        for part in [
            "already found runs seat c put",
            "approves nothing",
            "next 反映",
        ] {
            assert!(
                taken_down.contains(part),
                "{part:?} missing from: {taken_down}"
            );
        }
    }
}
