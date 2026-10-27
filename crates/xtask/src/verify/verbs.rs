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
mod github;
mod graph;
mod graph_walk;
mod identity;
mod nav;
mod panes;
mod recover;
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
    /// The whole argument on one preset: a form whose landing the fixture
    /// decides (`op-exit-lands quit` on the tree the quit leaves staged;
    /// `switch-lands feature/topic-a:2` on the tree whose carry conflicts).
    IsOn(&'static str, &'static str),
    /// The whole argument, in a run on Windows — where the app keeps to
    /// rules the other systems do not (a folder's name: `worktrees::Disk`).
    /// The same argument's row for the others follows it. Judged where the
    /// run is, so the container's run reads the next row.
    IsOnWindows(&'static str),
}

impl Arg {
    fn answers(&self, arg: &str, presets: &[String]) -> bool {
        match self {
            Self::IsOnWindows(want) => cfg!(windows) && arg == *want,
            _ => self.names(arg, presets),
        }
    }

    /// Whether the row is for this run on some system: [`Arg::answers`]
    /// without the system's say, for the question of whether any
    /// registered line reaches the row at all.
    fn names(&self, arg: &str, presets: &[String]) -> bool {
        match self {
            Self::Is(want) | Self::IsOnWindows(want) => arg == *want,
            Self::OneOf(any) => any.contains(&arg),
            Self::Starts(head) => arg.starts_with(head),
            Self::Ends(tail) => arg.ends_with(tail),
            Self::Has(part) => arg.contains(part),
            Self::WithPreset(want) => presets.iter().any(|preset| preset == want),
            Self::IsOn(want, on) => arg == *want && presets.iter().any(|preset| preset == on),
        }
    }

    fn describe(&self) -> String {
        match self {
            Self::Is(want) => format!("the argument {want:?}"),
            Self::IsOnWindows(want) => format!("the argument {want:?} on Windows"),
            Self::OneOf(any) => format!("an argument among {any:?}"),
            Self::Starts(head) => format!("an argument starting {head:?}"),
            Self::Ends(tail) => format!("an argument ending {tail:?}"),
            Self::Has(part) => format!("an argument holding {part:?}"),
            Self::WithPreset(want) => format!("a run on --preset {want}"),
            Self::IsOn(want, on) => format!("the argument {want:?} on --preset {on}"),
        }
    }
}

/// A registered line as the tables read it: its verb, argument and
/// presets.
pub(crate) struct Typed {
    pub verb: String,
    pub arg: String,
    pub presets: Vec<String>,
}

/// The rows no line in `lines` reaches, each as `<verb>: <what the row is
/// for>` — a row a rename on the QML side left behind, or a state no
/// registered line puts the app in, which its runs would then pass on the
/// picture alone. The plain row of a verb is reached by any line of it.
pub(crate) fn unreached_rows(lines: &[Typed]) -> Vec<String> {
    let mut out = Vec::new();
    for verb in TABLES.iter().copied().flatten() {
        let of_verb: Vec<&Typed> = lines.iter().filter(|line| line.verb == verb.name).collect();
        if of_verb.is_empty() {
            out.push(format!(
                "{}: the table's rows (no line of the verb)",
                verb.name
            ));
            continue;
        }
        for (when, _) in verb.when {
            if !of_verb
                .iter()
                .any(|line| when.names(&line.arg, &line.presets))
            {
                out.push(format!("{}: the row for {}", verb.name, when.describe()));
            }
        }
    }
    out
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
    github::TABLE,
    graph::TABLE,
    graph_walk::TABLE,
    identity::TABLE,
    nav::TABLE,
    panes::TABLE,
    recover::TABLE,
    remote::TABLE,
    sequencer::TABLE,
    stash::TABLE,
    switch::TABLE,
    window::TABLE,
];

/// The claims a verb's run has to have read true at its app picture
/// (`Awaited.claim` / `pictured`): the state it completed on, read again
/// at the picture. Keyed like the judgment rows; a verb absent here owes
/// only the window's own claim, which every run makes.
const CLAIMS: &[(&str, Arg, &[&str])] = &[
    // The card back after the dismissal and the tremor (`back`, the plain
    // form and the `row-late` diagnostic).
    (
        "card-latch",
        Arg::OneOf(&["", "back", "back:row-late"]),
        &["card_latch"],
    ),
    // The worktree row gone with the status the write asked for.
    ("rebase-plan-run", Arg::Is(""), &["rebase_plan_ran"]),
    // The commit's landing: the tree emptied, the WIP face left and HEAD's
    // row — as the write left it — selected, whichever of the answer and
    // the status drained first; files left keep the face.
    ("commit", Arg::Starts(""), &["landed_on_head"]),
    ("amend", Arg::Starts(""), &["landed_on_head"]),
    // The panel drawn for the width and page it was measured over, and the
    // four buttons' shape over the fetch's answer: the opening fetch lands
    // after the panel does, and would redraw it under the picture.
    ("ops-panel", Arg::Starts(""), &["ops_panel"]),
    ("band-actions", Arg::Starts(""), &["band_actions"]),
    ("band-actions-none", Arg::Starts(""), &["band_actions"]),
    ("band-actions-fold", Arg::Starts(""), &["band_actions"]),
    ("band-actions-alert", Arg::Starts(""), &["band_actions"]),
    ("band-actions-stopped", Arg::Starts(""), &["band_actions"]),
];

/// What a registered line's run is judged on besides its pictures, as
/// text for the gate's cache key (`gate::plan::Step::contract`): the
/// must-say row and the claims its verb owes. A line that cannot be parsed
/// is its own error text — a key of its own, run again until it reads.
pub(crate) fn judgment_of(line: &str) -> String {
    let words: Vec<String> = line.split_whitespace().map(str::to_string).collect();
    match super::options::parse(&words) {
        Ok(opts) => {
            let presets = super::repos::presets_as_built(&opts);
            format!(
                "must_say={}\nclaims={}",
                must_say(&opts.verb, &opts.arg, &presets).unwrap_or("-"),
                claims_for(&opts.verb, &opts.arg, &presets).join(" ")
            )
        }
        Err(why) => format!("unreadable: {why}"),
    }
}

/// The claims `verb` run as `arg` on `presets` owes at its app picture.
pub(super) fn claims_for(verb: &str, arg: &str, presets: &[String]) -> &'static [&'static str] {
    CLAIMS
        .iter()
        .find(|(name, when, _)| *name == verb && when.answers(arg, presets))
        .map_or(&[], |(_, _, names)| *names)
}

/// Every verb the claims table names, for the check that each is one the
/// harness answers (`verify::coverage`).
pub(crate) fn verbs_with_claims() -> impl Iterator<Item = &'static str> {
    CLAIMS.iter().map(|(name, _, _)| *name)
}

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

    /// A Windows row answers on Windows alone, and the same argument's next
    /// row answers everywhere else.
    #[test]
    fn a_windows_row_answers_only_on_windows() {
        let said = must_say("worktree-rename-box", "detached=CON", &[]);
        let row = |os: &str| {
            format!(
                "worktree_rename_box where=nav open=true withheld=false kind=worktreeFolder \
                 placeholder=Worktree name text=CON refused={} tip={} mark= why={}",
                os == "windows",
                os == "windows",
                if os == "windows" {
                    "Not a folder name on Windows — CON is kept for a device"
                } else {
                    ""
                }
            )
        };
        let here = if cfg!(windows) { "windows" } else { "posix" };
        assert_eq!(said.map(str::to_string), Some(row(here)));
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
