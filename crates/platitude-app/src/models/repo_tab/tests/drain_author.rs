//! Whom an amend's commit button names: HEAD's author against the identity
//! git would record (`state::compare_head_author`) — offered to take over
//! as recorded, and stacked as a second face only for another person as
//! `.mailmap` spells both.

use super::super::*;

/// The identity's read, as configured.
fn configured(name: &str, email: &str) -> TabMsg {
    TabMsg::Author {
        name: name.into(),
        email: email.into(),
        complete: true,
        sign_commits: false,
        signing_format: String::new(),
    }
}

/// The identity as configured, then as `.mailmap` spells it — two reads.
fn identity(name: &str, email: &str, shown_name: &str, shown_email: &str) -> Vec<TabMsg> {
    vec![
        configured(name, email),
        TabMsg::AuthorShown {
            name: shown_name.into(),
            email: shown_email.into(),
        },
    ]
}

fn head(name: &str, email: &str, shown_name: &str, shown_email: &str) -> Vec<TabMsg> {
    vec![TabMsg::HeadAuthor {
        author: platitude_core::commit::HeadAuthor {
            oid: None,
            name: name.into(),
            email: email.into(),
            shown_name: shown_name.into(),
            shown_email: shown_email.into(),
        },
    }]
}

/// Someone else's commit: offered, and two faces.
#[test]
fn another_persons_head_is_offered_and_wears_both_faces() {
    let mut tab = RepoTab::default();
    tab.absorb(identity(
        "Demo User",
        "demo@example.com",
        "Demo User",
        "demo@example.com",
    ));
    tab.absorb(head(
        "Yuki Tanaka",
        "yuki@example.com",
        "Yuki Tanaka",
        "yuki@example.com",
    ));
    assert!(tab.head_author_differs);
    assert!(tab.head_author_another);
}

/// The identity's own old address, which `.mailmap` folds into the one
/// configured: git records the old one, so taking it over is offered, under
/// one face.
#[test]
fn an_old_address_mailmap_folds_into_the_identitys_is_one_face() {
    let mut tab = RepoTab::default();
    tab.absorb(identity(
        "Demo User",
        "demo@example.com",
        "Demo User",
        "demo@example.com",
    ));
    tab.absorb(head(
        "Demo User",
        "demo@old.example.com",
        "Demo User",
        "demo@example.com",
    ));
    assert!(tab.head_author_differs);
    assert!(!tab.head_author_another);
}

/// The identity configured under the old address `.mailmap` maps away: its
/// own commit is one face, told apart by the spelling the graph shows.
#[test]
fn an_identity_under_an_address_mailmap_maps_away_is_one_face_with_its_commit() {
    let mut tab = RepoTab::default();
    tab.absorb(identity(
        "Demo User",
        "demo@old.example.com",
        "Demo User",
        "demo@example.com",
    ));
    tab.absorb(head(
        "Demo User",
        "demo@new.example.com",
        "Demo User",
        "demo@example.com",
    ));
    assert!(tab.head_author_differs, "recorded under another address");
    assert!(!tab.head_author_another);
}

/// No second face before the identity it would stand beside is read.
#[test]
fn no_second_face_before_the_identity_is_read() {
    let mut tab = RepoTab::default();
    tab.absorb(head(
        "Yuki Tanaka",
        "yuki@example.com",
        "Yuki Tanaka",
        "yuki@example.com",
    ));
    assert!(!tab.head_author_another);
    tab.absorb(identity(
        "Demo User",
        "demo@example.com",
        "Demo User",
        "demo@example.com",
    ));
    assert!(tab.head_author_another, "the read that landed asks again");
}

/// The same identity read again keeps the spelling `.mailmap` gave it; another
/// one is shown as configured until its own spelling lands.
#[test]
fn only_another_identity_waits_for_its_spelling() {
    let mut tab = RepoTab::default();
    tab.absorb(identity(
        "Demo User",
        "demo@old.example.com",
        "Demo User",
        "demo@example.com",
    ));
    tab.absorb(vec![configured("Demo User", "demo@old.example.com")]);
    assert_eq!(tab.author_shown_email, "demo@example.com", "read again");
    tab.absorb(vec![configured("Ada", "ada@example.com")]);
    assert_eq!(
        tab.author_shown_email, "ada@example.com",
        "another identity"
    );
}

/// The empty answer core gives where the last author's HEAD is gone
/// (`RepoSession::watch_amend`) lets the offer and the second face go.
#[test]
fn an_empty_answer_lets_heads_author_go() {
    let mut tab = RepoTab::default();
    tab.absorb(identity(
        "Demo User",
        "demo@example.com",
        "Demo User",
        "demo@example.com",
    ));
    tab.absorb(head(
        "Yuki Tanaka",
        "yuki@example.com",
        "Yuki Tanaka",
        "yuki@example.com",
    ));
    tab.absorb(vec![TabMsg::HeadAuthor {
        author: platitude_core::commit::HeadAuthor::default(),
    }]);
    assert!(!tab.head_author_differs);
    assert!(!tab.head_author_another);
}
