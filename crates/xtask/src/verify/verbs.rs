//! The line each verb has to be caught saying, for the verbs whose
//! failure the camera cannot see (`Outcome::must_say`). A verb that fails
//! invisibly is one row here.
//!
//! The rows are data so `tests` can walk them: `no_verb_is_claimed_twice`
//! is what lets the tables be split by subject (a chain of `match`es
//! would answer a doubled verb silently), and
//! `every_verb_is_one_the_drivers_dispatch` catches a verb renamed on the
//! QML side. Which file a verb sits in carries no meaning — grep the verb.

mod diff;
mod fetch;
mod graph;
mod graph_walk;
mod identity;
mod nav;
mod panes;
mod remote;
mod sequencer;
mod stash;
mod switch;
mod window;

/// Which argument a row answers to.
///
/// A verb's rows are read in order and the first that answers wins, so
/// the narrow forms are written before the wide ones.
pub(super) enum Arg {
    /// The whole argument — `Is("")` is the run that passes none.
    Is(&'static str),
    OneOf(&'static [&'static str]),
    Starts(&'static str),
    Ends(&'static str),
    Has(&'static str),
    /// Not the argument: which preset the run was given. Two runs that
    /// differ only in preset are two states, and one line for both would
    /// go on passing after they stop differing (`corner`).
    WithPreset(&'static str),
}

impl Arg {
    fn answers(&self, arg: &str, presets: &[String]) -> bool {
        match self {
            Self::Is(want) => arg == *want,
            Self::OneOf(any) => any.contains(&arg),
            Self::Starts(head) => arg.starts_with(head),
            Self::Ends(tail) => arg.ends_with(tail),
            Self::Has(part) => arg.contains(part),
            Self::WithPreset(want) => presets.iter().any(|preset| preset == want),
        }
    }
}

/// One verb's line, and the arguments that want a different one.
/// `plain` is required so every argument gets a line: `None` from
/// `must_say` is the one failure that passes.
pub(super) struct Verb {
    pub(super) name: &'static str,
    /// The arguments with a line of their own, narrowest first.
    pub(super) when: &'static [(Arg, &'static str)],
    /// What every other argument gets.
    pub(super) plain: &'static str,
}

/// Every table, asked in no particular order — `no_verb_is_claimed_twice`
/// is what makes the order not matter.
const TABLES: &[&[Verb]] = &[
    diff::TABLE,
    fetch::TABLE,
    graph::TABLE,
    graph_walk::TABLE,
    identity::TABLE,
    nav::TABLE,
    panes::TABLE,
    remote::TABLE,
    sequencer::TABLE,
    stash::TABLE,
    switch::TABLE,
    window::TABLE,
];

/// What `verb` has to say for its picture to be worth anything, or `None`
/// when the picture is the whole of it. A run's state is `arg` and
/// `presets` together, and a row may key on either.
pub(super) fn must_say(verb: &str, arg: &str, presets: &[String]) -> Option<&'static str> {
    let found = TABLES.iter().copied().flatten().find(|v| v.name == verb)?;
    Some(
        found
            .when
            .iter()
            .find(|(when, _)| when.answers(arg, presets))
            .map_or(found.plain, |(_, line)| *line),
    )
}

#[cfg(test)]
mod tests {
    use super::{TABLES, must_say};

    fn every_verb() -> impl Iterator<Item = &'static super::Verb> {
        TABLES.iter().copied().flatten()
    }

    #[test]
    fn no_verb_is_claimed_twice() {
        let mut seen: Vec<&str> = Vec::new();
        for verb in every_verb() {
            assert!(
                !seen.contains(&verb.name),
                "`{}` is on two tables — one of the two rows is never read",
                verb.name
            );
            seen.push(verb.name);
        }
    }

    /// A rename on the QML side leaves a row nothing reaches: `must_say`
    /// goes quiet and the run is judged on the picture alone
    /// (rules-refs/app-ui.md「xtask verify/verbs.rs」). Only this direction
    /// can be asked — most verbs are judged on their picture and have no
    /// row.
    #[test]
    fn every_verb_is_one_the_drivers_dispatch() {
        // Every file of the harness module is a driver, so no name filter
        // (.claude/rules/app-ui.md「QML モジュールは 2 つ」).
        let auto = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/platitude-app/src/auto");
        let mut drivers = String::new();
        let mut found = 0;
        for entry in std::fs::read_dir(&auto).unwrap_or_else(|e| panic!("{}: {e}", auto.display()))
        {
            let entry = entry.unwrap_or_else(|e| panic!("{}: {e}", auto.display()));
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.ends_with(".qml") {
                continue;
            }
            drivers.push_str(
                &std::fs::read_to_string(entry.path()).unwrap_or_else(|e| panic!("{name}: {e}")),
            );
            found += 1;
        }
        assert!(
            found > 2,
            "found {found} harness file(s) under {} — the search stopped matching, and a test that \
             reads nothing passes every row",
            auto.display()
        );
        for verb in every_verb() {
            assert!(
                drivers.contains(&format!("\"{}\"", verb.name)),
                "no driver spells `{}` — the row is unreachable and its runs pass on the picture",
                verb.name
            );
        }
    }

    fn presets(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_string()).collect()
    }

    #[test]
    fn the_narrow_row_answers_before_the_plain_one() {
        assert_eq!(
            must_say("stash", "named", &[]),
            Some("graph_settled gone=true top=stash named=true")
        );
        assert_eq!(
            must_say("stash", "anything else", &[]),
            Some("graph_settled gone=true top=stash named=false")
        );
        assert_eq!(
            must_say("stash", "", &[]),
            must_say("stash", "anything else", &[])
        );
    }

    #[test]
    fn a_row_may_answer_to_the_preset_rather_than_the_argument() {
        assert_eq!(
            must_say("corner", "1", &presets(&["basic"])),
            Some("git_corner pane=details shown=true")
        );
        assert_eq!(
            must_say("corner", "1", &presets(&["long"])),
            Some("git_corner pane=details shown=false room=0")
        );
        assert_ne!(
            must_say("corner", "1", &presets(&["basic"])),
            must_say("corner", "1", &presets(&["long"])),
            "the pair is two states or it is one run twice"
        );
        // The argument outranks the preset where the pane decides alone:
        // the working tree's foot is the commit button's.
        for preset in [&["basic"][..], &["long"][..]] {
            assert_eq!(
                must_say("corner", "wip", &presets(preset)),
                Some("git_corner pane=wip shown=false room=0")
            );
        }
    }

    #[test]
    fn a_verb_no_table_claims_says_nothing() {
        assert_eq!(must_say("no-such-verb", "", &[]), None);
    }
}
