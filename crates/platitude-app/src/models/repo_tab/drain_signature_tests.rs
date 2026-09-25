//! Which answer about a commit's signature the pane is left reading
//! (`drain::settle_signature`).

use super::*;

/// Two commits out of the `co-authors` demo repository, named here
/// because the wedge below was measured on them: row 3 is the one the
/// reader is on, row 0 the selection the page made on its way there.
const ON_SCREEN: &str = "f7892e60a51c14c36a159b33169be38823edc07f";
const LEFT_BEHIND: &str = "1db47e8896c214d7ef5fa1aa2c34d010ab9f7481";

/// Both rows asked about before either is answered — what two
/// selections in quick succession leave in flight.
fn asked_about_both() -> RepoTab {
    let mut tab = RepoTab::default();
    tab.look_up_signature(LEFT_BEHIND.into());
    tab.look_up_signature(ON_SCREEN.into());
    tab
}

// Two reads race, and the one about the selection already left behind
// can win — a coin flip decided by how long two `ssh-keygen` runs take,
// which under concurrent runs land close enough together for either
// order (observed). The answer the pane is waiting for had already
// landed, so letting the late one overwrite it puts the pane back to
// "nothing has answered" — and nothing asks again, so it stays there.
#[test]
fn a_signature_answer_about_a_selection_left_behind_is_dropped() {
    let mut tab = asked_about_both();
    tab.settle_signature(
        ON_SCREEN.into(),
        "verified".into(),
        "G".into(),
        "demo@example.com".into(),
    );
    tab.settle_signature(
        LEFT_BEHIND.into(),
        "signed".into(),
        "U".into(),
        "stranger@example.com".into(),
    );
    assert_eq!(tab.signature_oid, ON_SCREEN);
    assert_eq!(tab.signature_kind, "verified");
    assert_eq!(tab.signature_code, "G");
    assert_eq!(tab.signature_signer, "demo@example.com");
}

#[test]
fn the_answer_the_pane_is_waiting_for_lands_however_late() {
    let mut tab = asked_about_both();
    tab.settle_signature(
        LEFT_BEHIND.into(),
        "signed".into(),
        "U".into(),
        "stranger@example.com".into(),
    );
    tab.settle_signature(
        ON_SCREEN.into(),
        "verified".into(),
        "G".into(),
        "demo@example.com".into(),
    );
    assert_eq!(tab.signature_oid, ON_SCREEN);
    assert_eq!(tab.signature_code, "G");
}

// Coming back to a row asks again (デザイン規約 §署名の表示 「同じ行へ
// 戻ってくれば投げ直す」), and the answer to that second asking is
// about the commit on screen — so it is the one that is kept.
#[test]
fn coming_back_to_a_row_takes_the_answer_to_the_asking_that_brought_it() {
    let mut tab = asked_about_both();
    tab.look_up_signature(LEFT_BEHIND.into());
    tab.settle_signature(
        ON_SCREEN.into(),
        "verified".into(),
        "G".into(),
        "demo@example.com".into(),
    );
    tab.settle_signature(
        LEFT_BEHIND.into(),
        "signed".into(),
        "U".into(),
        "stranger@example.com".into(),
    );
    assert_eq!(tab.signature_oid, LEFT_BEHIND);
    assert_eq!(tab.signature_code, "U");
}

// A read that answers a session nobody is asking any more — the shape
// `Feed::clear_queued` exists for, arriving one drain too late.
#[test]
fn an_answer_nobody_asked_for_is_dropped() {
    let mut tab = RepoTab::default();
    tab.settle_signature(
        ON_SCREEN.into(),
        "verified".into(),
        "G".into(),
        "demo@example.com".into(),
    );
    assert_eq!(tab.signature_oid, "");
}
