//! An index of owners — nothing is defined here. Each module declares
//! its operations beside the code that runs them (`crate::command`).

use crate::command::Command;

/// Every declaration, in the order the usage page reads.
pub(crate) fn all() -> Vec<&'static Command> {
    [
        crate::check::COMMANDS,
        crate::gate::COMMANDS,
        crate::sweep::COMMANDS,
        crate::structure::COMMANDS,
        crate::waits::COMMANDS,
        crate::versions::COMMANDS,
        crate::docs::COMMANDS,
        crate::qmltest::COMMANDS,
        crate::deny::COMMANDS,
        crate::demo::COMMANDS,
        crate::verify::COMMANDS,
        crate::replay::COMMANDS,
        crate::corpus::COMMANDS,
        crate::perf::COMMANDS,
        crate::shipped::COMMANDS,
        crate::package::COMMANDS,
        crate::linux::COMMANDS,
        crate::seats::COMMANDS,
        crate::shots::COMMANDS,
        crate::still::COMMANDS,
        crate::footprint::COMMANDS,
        crate::budget::COMMANDS,
        crate::land::COMMANDS,
        crate::gui::COMMANDS,
        crate::awake::COMMANDS,
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

    #[test]
    fn no_id_is_declared_twice() {
        assert_eq!(repeated(), Vec::<&str>::new());
    }

    /// A typable line and a purpose are what a reference to the id is
    /// worth.
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

    /// Ids name the operation and survive a verb rename, so none is
    /// derived from a call.
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

    /// Several operations may share a verb: `seat`, `seat release` and
    /// `seat takeover` are three.
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
