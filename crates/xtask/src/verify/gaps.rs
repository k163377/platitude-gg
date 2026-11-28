//! A menu row its menu has no part for, as the app says it — read off a
//! verify-ui run's lines (`super::outcome`) and a `qmltestrunner`'s log
//! and stderr (`crate::qmltest`). Either is a red whatever else the run or
//! the case came to.
//!
//! The drawer (`MenuLines.draw`) stands such a row nowhere and says so in
//! a warning: nothing else shows it — the exit is clean and the picture is
//! of a menu one row short. `cargo xtask structure` sees only that every
//! row of core's type has a part in some menu; which rows a menu needs is
//! its classes', known once an answer comes. A test that stands a gap on
//! purpose asks for the warning with `ignoreWarning`, which takes it out
//! of the log.

/// What `MenuLines` says ahead of the rows it found no part for — held to
/// the drawer's own `console.warn` by `structure::menu_words`. ASCII,
/// so a Windows console's code page leaves it readable.
pub(crate) const MARK: &str = "MenuLines: a row with no part: ";

/// The rows `line` says have no part, if it says a gap.
pub(crate) fn in_line(line: &str) -> Option<String> {
    let at = line.find(MARK)?;
    Some(line[at + MARK.len()..].trim().to_string())
}

/// The gaps `lines` say, each once, in the order they first came.
pub(crate) fn met<'a>(lines: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut met: Vec<String> = Vec::new();
    for rows in lines.into_iter().filter_map(in_line) {
        if !met.contains(&rows) {
            met.push(rows);
        }
    }
    met
}

/// What a red for a gap means, said after the gaps it names.
pub(crate) const MEANING: &str = "a menu was handed rows it has no part for (verify::gaps): the \
     drawer stood them nowhere, so the menu came up short and neither the exit nor the picture \
     shows it — a red until the menu declares a part for every row its classes have, in the card \
     its seat names (menu::Row::place).";

#[cfg(test)]
mod tests {
    use super::{in_line, met};

    /// The words the drawer says, as the app and a QtTest log carry them:
    /// whatever precedes them, the rows follow the mark.
    #[test]
    fn a_gap_is_read_wherever_it_is_said() {
        let app = "qrc:/qt/qml/platitude/ui/MenuLines.qml:128: MenuLines: a row with no part: \
                   pull, tag/delete-tag-both";
        let log = "QWARN  : qmltestrunner::CommitRowMenu::test_x() MenuLines: a row with no \
                   part: pull, tag/delete-tag-both";
        assert_eq!(in_line(app).as_deref(), Some("pull, tag/delete-tag-both"));
        assert_eq!(in_line(log), in_line(app));
        assert_eq!(in_line("qml: MenuLines drew"), None);
    }

    /// One gap said by every stand of a menu is one red, named once.
    #[test]
    fn a_gap_said_again_is_one_gap() {
        let said = [
            "MenuLines: a row with no part: pull",
            "MenuLines: a row with no part: card reset",
            "MenuLines: a row with no part: pull",
        ];
        assert_eq!(met(said), ["pull", "card reset"]);
    }
}
