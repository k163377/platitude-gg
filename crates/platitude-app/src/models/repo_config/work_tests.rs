//! Which answer the repository screen shows, with the answers fed in the
//! orders they can arrive — the numbering and the drain alone, without Qt
//! or a repository (`work`).

use super::work::ReadTicket;
use super::*;

fn read(generation: u64, path: &str, name: &str) -> ConfigMsg {
    ConfigMsg::Read {
        generation,
        path: path.into(),
        local_name: name.into(),
        local_email: format!("{name}@example.com"),
        effective_name: name.into(),
        effective_email: format!("{name}@example.com"),
    }
}

fn written(save: u64, name_saved: bool, email_saved: bool, error: &str) -> ConfigMsg {
    ConfigMsg::Written {
        save,
        path: "A".into(),
        error: error.into(),
        name_saved,
        email_saved,
    }
}

/// The screen turned to `path` and got the read's ticket.
fn looking_at(model: &mut RepoConfigModel, path: &str) -> ReadTicket {
    model.look_at(path.into()).expect("a path was named")
}

#[test]
fn a_read_answers_the_ask_it_came_from_when_the_screen_went_a_b_a() {
    let mut model = RepoConfigModel::default();
    let first = looking_at(&mut model, "A");
    let second = looking_at(&mut model, "B");
    let third = looking_at(&mut model, "A");
    assert_eq!(
        (first.generation, second.generation, third.generation),
        (1, 2, 3)
    );

    assert!(!model.absorb(vec![read(first.generation, "A", "stale")]));
    assert_eq!(
        model.state, "reading",
        "the first ask's answer is not the third's"
    );
    assert_eq!(model.local_name, "");

    assert!(!model.absorb(vec![read(second.generation, "B", "other")]));
    assert_eq!(model.state, "reading", "nor is B's");

    assert!(!model.absorb(vec![read(third.generation, "A", "current")]));
    assert_eq!(model.state, "ready");
    assert_eq!(model.local_name, "current");
    assert_eq!(model.repo_path, "A");
}

#[test]
fn the_later_ask_about_one_repository_stands_whichever_answers_last() {
    let mut model = RepoConfigModel::default();
    let first = looking_at(&mut model, "A");
    let again = model.ask_again().expect("a repository is named");
    assert!(again.generation > first.generation);

    model.absorb(vec![read(again.generation, "A", "new")]);
    assert_eq!(model.local_name, "new");
    model.absorb(vec![read(first.generation, "A", "old")]);
    assert_eq!(
        model.local_name, "new",
        "the older read cannot overwrite the newer"
    );
    assert_eq!(model.state, "ready");
}

#[test]
fn the_ask_after_a_read_cancels_that_read_and_only_that_one() {
    let mut model = RepoConfigModel::default();
    let first = looking_at(&mut model, "A");
    let second = looking_at(&mut model, "B");
    assert!(first.cancel.is_cancelled(), "the read of A is nobody's now");
    assert!(
        !second.cancel.is_cancelled(),
        "the read of B is the screen's"
    );
    let third = model.ask_again().expect("a repository is named");
    assert!(second.cancel.is_cancelled());
    assert!(!third.cancel.is_cancelled());
}

#[test]
fn a_save_is_answered_by_number_and_a_half_landing_is_shown_as_one() {
    let mut model = RepoConfigModel::default();
    let opening = looking_at(&mut model, "A");
    model.absorb(vec![read(opening.generation, "A", "before")]);
    let save = model.begin_save().expect("nothing is out");
    assert!(model.write_busy);
    assert_eq!(model.begin_save(), None, "one save at a time");

    assert!(
        !model.absorb(vec![written(save + 40, true, true, "")]),
        "an answer to a save nobody asked for is dropped"
    );
    assert!(model.write_busy, "and the save out is still out");

    assert!(
        model.absorb(vec![written(
            save,
            true,
            false,
            "error: cannot overwrite multiple values"
        )]),
        "the save's own answer asks for the repository to be read again"
    );
    assert!(!model.write_busy);
    assert!(model.write_unsaved);
    assert!(model.write_name_saved);
    assert!(!model.write_email_saved);
    assert_eq!(model.error, "error: cannot overwrite multiple values");
}

#[test]
fn a_read_from_before_a_save_cannot_land_on_the_boxes_the_save_changed() {
    let mut model = RepoConfigModel::default();
    let opening = looking_at(&mut model, "A");
    model.absorb(vec![read(opening.generation, "A", "before")]);
    let stale = model.ask_again().expect("a poll of the same repository");
    let save = model.begin_save().expect("nothing is out");
    assert!(model.absorb(vec![written(
        save,
        false,
        false,
        "error: could not lock config file"
    )]));
    let after = model
        .ask_again()
        .expect("the re-read a landed save asks for");
    assert!(
        stale.cancel.is_cancelled(),
        "the older read is ended by the newer ask"
    );

    assert!(!model.absorb(vec![read(stale.generation, "A", "before")]));
    assert_eq!(
        model.local_name, "before",
        "unchanged: the stale read was refused"
    );
    assert_eq!(
        model.error, "error: could not lock config file",
        "and git's words stand while the re-read is out"
    );
    model.absorb(vec![read(after.generation, "A", "after")]);
    assert_eq!(model.local_name, "after");
    assert_eq!(
        model.error, "error: could not lock config file",
        "the read that follows a save keeps the words the save left"
    );
}

// Behind the save's answer, before any re-read is numbered: neither the
// stale read's failure nor its reading may land.
#[test]
fn a_read_from_before_a_save_answering_in_the_saves_own_drain_is_refused() {
    let mut model = RepoConfigModel::default();
    let opening = looking_at(&mut model, "A");
    model.absorb(vec![read(opening.generation, "A", "before")]);
    let stale = model.ask_again().expect("a poll of the same repository");
    let save = model.begin_save().expect("nothing is out");
    assert!(model.absorb(vec![
        written(save, false, false, "error: could not lock config file"),
        ConfigMsg::ReadFailed {
            generation: stale.generation,
            path: "A".into(),
            message: "fatal: not a git repository".into(),
        },
    ]));
    assert!(
        stale.cancel.is_cancelled(),
        "the save's answer ends the read from before it"
    );
    assert_eq!(
        model.error, "error: could not lock config file",
        "git's words about the save stand"
    );
    assert_eq!(
        model.state, "ready",
        "and the screen is not in error for a read it has moved past"
    );

    let save = model.begin_save().expect("the last one is answered");
    let stale = model
        .ask_again()
        .expect("another poll, out when the save lands");
    model.absorb(vec![
        written(save, true, true, ""),
        read(stale.generation, "A", "older"),
    ]);
    assert_eq!(
        model.local_name, "before",
        "a reading from before the save cannot put the boxes back"
    );
}

#[test]
fn a_save_out_when_the_screen_turns_away_is_nobodys() {
    let mut model = RepoConfigModel::default();
    let _a = looking_at(&mut model, "A");
    let save = model.begin_save().expect("nothing is out");
    let _b = looking_at(&mut model, "B");
    assert!(!model.write_busy, "the new screen has no save out");
    assert!(
        !model.absorb(vec![written(save, false, false, "error: lost")]),
        "the old save's answer asks for nothing here"
    );
    assert!(!model.write_busy);
    assert!(!model.write_unsaved, "and puts no mark on B's boxes");
    assert_eq!(model.error, "");
}
