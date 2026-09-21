//! An index of owners — nothing is defined here. Each module declares
//! the operations it runs beside the code that runs them
//! (`crate::command`); the rule is .claude/rules-refs/structure.md
//! §コマンドの正本.

use crate::command::Command;

/// Every declaration, in the order the usage page reads.
pub(crate) fn all() -> Vec<&'static Command> {
    [
        crate::check::COMMANDS,
        crate::gate::COMMANDS,
        crate::sweep::COMMANDS,
        crate::structure::COMMANDS,
        crate::waits::COMMANDS,
        crate::docs::COMMANDS,
        crate::qmltest::COMMANDS,
        crate::deny::COMMANDS,
        crate::demo::COMMANDS,
        crate::verify::COMMANDS,
        crate::corpus::COMMANDS,
        crate::perf::COMMANDS,
        crate::shipped::COMMANDS,
        crate::linux::COMMANDS,
        crate::seats::COMMANDS,
        crate::shots::COMMANDS,
        crate::still::COMMANDS,
        crate::footprint::COMMANDS,
        crate::budget::COMMANDS,
        crate::land::COMMANDS,
        crate::gui::COMMANDS,
        crate::hook::COMMANDS,
    ]
    .concat()
}

/// The ids that are declared twice, which would make a reference
/// ambiguous and a rename silently partial.
pub(crate) fn repeated() -> Vec<&'static str> {
    let mut ids: Vec<&'static str> = all().iter().map(|command| command.id).collect();
    ids.sort_unstable();
    let mut twice: Vec<&'static str> = ids
        .windows(2)
        .filter(|p| p[0] == p[1])
        .map(|p| p[0])
        .collect();
    twice.dedup();
    twice
}

#[cfg(test)]
mod tests {
    use super::{all, repeated};

    /// An id is what every other place refers an operation by, so two
    /// entries sharing one would make a reference mean either of them.
    #[test]
    fn no_id_is_declared_twice() {
        assert_eq!(repeated(), Vec::<&str>::new());
    }

    /// Every entry carries a line a reader can type and a purpose that
    /// says what typing it does: those two are what a reference to the
    /// id is worth, and an entry short of either is a reference to
    /// nothing.
    #[test]
    fn every_entry_carries_a_line_and_a_purpose() {
        assert!(!all().is_empty());
        for command in all() {
            assert!(
                command
                    .line()
                    .contains(&format!("cargo xtask {}", command.verb())),
                "{} spells its own verb: {}",
                command.id,
                command.line()
            );
            assert!(
                !command.purpose.is_empty(),
                "{} says what it is for",
                command.id
            );
            assert!(
                command.instruction().contains(command.purpose),
                "{} hands a reader its purpose",
                command.id
            );
        }
    }

    /// Ids name the operation: a verb rename leaves every
    /// one as it is, so nothing derives an id from a call.
    #[test]
    fn an_id_is_not_derived_from_the_call() {
        for command in all() {
            assert!(
                !command.id.contains(' '),
                "{} is one word per segment",
                command.id
            );
            assert!(
                command
                    .id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-'),
                "{} is a plain id",
                command.id
            );
        }
    }

    /// Several operations share a verb, and that is the shape the
    /// catalogue is for: `seat`, `seat release` and `seat takeover` are
    /// one verb and three things a session may be told to do.
    #[test]
    fn one_verb_carries_as_many_operations_as_it_has() {
        let seat: Vec<&str> = all()
            .iter()
            .filter(|command| command.verb() == "seat")
            .map(|command| command.id)
            .collect();
        assert_eq!(seat, ["seat.take", "seat.release", "seat.takeover"]);
    }
}
