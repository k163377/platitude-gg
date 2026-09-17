//! The line each verb has to be caught saying, for the verbs whose
//! failure the camera cannot see (`Outcome::must_say`).
//!
//! A table: a verb that fails invisibly is one row here, and the run
//! stays about running.
//!
//! The rows are data, so the tables can be walked, and the two walks in
//! `tests` are what let them be split by subject at all.
//! `no_verb_is_claimed_twice` catches a verb written
//! onto two of the files, which a chain of `match`es would hand to
//! whichever was asked first without a word. And
//! `every_verb_is_one_the_drivers_dispatch` catches the failure this
//! table has in the field — a verb renamed on the QML side leaves a key
//! nothing reaches, `must_say` goes quiet, and the run is judged on a
//! picture that reads the same either way (rules-refs/app-ui.md).
//!
//! Which file a verb sits in carries nothing of its own: the tables are
//! walked in the order [`TABLES`] lists them and each verb stands on
//! one, so a verb that moves house changes no answer. Grep the verb.

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
}

impl Arg {
    fn answers(&self, arg: &str) -> bool {
        match self {
            Self::Is(want) => arg == *want,
            Self::OneOf(any) => any.contains(&arg),
            Self::Starts(head) => arg.starts_with(head),
            Self::Ends(tail) => arg.ends_with(tail),
            Self::Has(part) => arg.contains(part),
        }
    }
}

/// One verb's line, and the arguments that want a different one.
///
/// `plain` is a field, and the compiler asks for it — a
/// verb that answered no argument at all would put
/// `must_say` back to `None`, which is the one failure that
/// passes.
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
/// when the picture is the whole of it. `arg` is the verb's own argument:
/// one verb serves two panes and wants a different line for each.
pub(super) fn must_say(verb: &str, arg: &str) -> Option<&'static str> {
    let found = TABLES.iter().copied().flatten().find(|v| v.name == verb)?;
    Some(
        found
            .when
            .iter()
            .find(|(when, _)| when.answers(arg))
            .map_or(found.plain, |(_, line)| *line),
    )
}

#[cfg(test)]
mod tests {
    use super::{TABLES, must_say};

    fn every_verb() -> impl Iterator<Item = &'static super::Verb> {
        TABLES.iter().copied().flatten()
    }

    /// The tables are split by subject, and a verb written onto two of
    /// them would be answered by whichever is walked first while the
    /// other row sat there looking right.
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

    /// The key is a verb's spelling, so a rename on the QML side leaves a
    /// row nothing reaches: `must_say` goes quiet and the run is judged
    /// on a picture that reads the same whether the verb worked or not
    /// (rules-refs/app-ui.md).
    ///
    /// Only this direction can be asked. The drivers dispatch far more
    /// verbs than this table judges, because most verbs are judged on
    /// their picture and belong in no table at all.
    #[test]
    fn every_verb_is_one_the_drivers_dispatch() {
        // The harness is a QML module of its own, so every file in it is
        // one of the drivers — no name filter to keep in step with what
        // the verbs are grouped by (.claude/rules/app-ui.md §QML モジュール).
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

    /// The narrow rows come first, and what no row claims falls to
    /// `plain`.
    #[test]
    fn the_narrow_row_answers_before_the_plain_one() {
        assert_eq!(
            must_say("stash", "named"),
            Some("graph_settled gone=true top=stash named=true")
        );
        assert_eq!(
            must_say("stash", "anything else"),
            Some("graph_settled gone=true top=stash named=false")
        );
        assert_eq!(must_say("stash", ""), must_say("stash", "anything else"));
    }

    /// A verb no table claims is one whose picture is the whole of it.
    #[test]
    fn a_verb_no_table_claims_says_nothing() {
        assert_eq!(must_say("no-such-verb", ""), None);
    }
}
