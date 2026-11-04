//! QML's two words for a layout that does not come to rest, read off
//! what a run said — a verify-ui run's lines (`super::outcome`) and a
//! `qmltestrunner`'s log and stderr (`crate::qmltest`). Either is a red
//! whatever else the run or the case came to, unless [`ALLOWED`] names it.
//!
//! - `Binding loop detected for property "<p>"`: a binding whose
//!   evaluation wrote what it read. Qt drops the write and carries on, so
//!   the value standing is whichever pass came last
//!   (`QQmlBinding::printBindingLoopError`,
//!   `QQmlPropertyBinding::createBindingLoopErrorDescription`, v6.12.0).
//! - `possible QQuickItem::polish() loop`: an item polished again inside
//!   its own `updatePolish()`, a thousand times running; Qt gives up at
//!   ten thousand and the layout stands where it was
//!   (`qquickwindow.cpp`'s `PolishLoopDetector`, v6.12.0).
//!
//! Neither moves an exit code or a case's verdict, and a picture is of
//! whichever pass came last: Qt says it as a warning and nothing else
//! does.
//!
//! Both are `qmlWarning(object)`: the line is the object's place and type
//! — `<url>:<line>:<column>: QML <type>: <words>` — or the type and the
//! words alone for an object with no QML context. A loop is known by its
//! file's name and what follows `QML ` ([`Loop`]): the url is `qrc:` in
//! the app and `file:` in the staged import tree, and the line moves with
//! any comment above it. ASCII, so a Windows Qt's code page leaves it
//! readable (verify-ui skill).

/// What Qt says of each loop, whatever precedes or follows it.
const MARKS: [&str; 2] = [
    "Binding loop detected",
    "possible QQuickItem::polish() loop",
];

/// One loop, as a line places it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Loop {
    /// The file the warning places it in, by name (`NavSections.qml`);
    /// empty for an object with no QML context.
    pub(crate) file: String,
    /// What Qt said past `QML `: the type and the words
    /// (`NavList: Binding loop detected for property "topMargin"`).
    pub(crate) said: String,
}

impl Loop {
    /// The loop `line` says, if it says one.
    pub(crate) fn in_line(line: &str) -> Option<Loop> {
        let mark = MARKS.iter().find_map(|mark| line.find(mark))?;
        let before = &line[..mark];
        // The type is what the last `QML ` ahead of the words names.
        let typed = before.rfind("QML ").map_or(mark, |at| at + "QML ".len());
        let said = line[typed..].trim_end().trim_end_matches(':').to_string();
        // The place is the last word ahead of `: QML `, whatever the line
        // carries before it (a QtTest log's tag and function).
        let file = before
            .find(": QML ")
            .and_then(|at| before[..at].split_whitespace().last())
            .map(file_of)
            .unwrap_or_default();
        Some(Loop { file, said })
    }
}

impl std::fmt::Display for Loop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.file.is_empty() {
            write!(f, "QML {}", self.said)
        } else {
            write!(f, "{}: QML {}", self.file, self.said)
        }
    }
}

/// A place's file name: `qrc:/qt/qml/platitude/ui/NavSections.qml:162:5`
/// is `NavSections.qml`.
fn file_of(place: &str) -> String {
    let mut name = place.rsplit(['/', '\\']).next().unwrap_or(place);
    // The column, then the line, where the place has them.
    for _ in 0..2 {
        if let Some((head, tail)) = name.rsplit_once(':')
            && !tail.is_empty()
            && tail.bytes().all(|b| b.is_ascii_digit())
        {
            name = head;
        }
    }
    name.to_string()
}

/// A loop that stands, and why: said wherever it is met, judged by
/// nothing.
pub(crate) struct Allowed {
    /// [`Loop::file`].
    pub(crate) file: &'static str,
    /// [`Loop::said`].
    pub(crate) said: &'static str,
    pub(crate) why: &'static str,
}

/// The loops that stand.
pub(crate) const ALLOWED: &[Allowed] = &[];

/// Why `found` stands, if [`ALLOWED`] names it.
fn allowance(found: &Loop) -> Option<&'static str> {
    ALLOWED
        .iter()
        .find(|allowed| allowed.file == found.file && allowed.said == found.said)
        .map(|allowed| allowed.why)
}

/// The loop `line` says that nothing allows, if it says one.
pub(crate) fn refused(line: &str) -> Option<Loop> {
    Loop::in_line(line).filter(|found| allowance(found).is_none())
}

/// One loop the lines said, however often.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Met {
    pub(crate) found: Loop,
    pub(crate) times: usize,
    /// The first line that said it, as it came.
    pub(crate) first: String,
    /// Why it stands ([`ALLOWED`]); `None` for a red.
    pub(crate) allowed: Option<&'static str>,
}

impl std::fmt::Display for Met {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.found)?;
        if self.times > 1 {
            write!(f, " ({} times)", self.times)?;
        }
        match self.allowed {
            Some(why) => write!(f, " — allowed: {why}"),
            None => Ok(()),
        }
    }
}

/// The loops `lines` say, each once, in the order they first came.
pub(crate) fn met<'a>(lines: impl IntoIterator<Item = &'a str>) -> Vec<Met> {
    let mut met: Vec<Met> = Vec::new();
    for line in lines {
        let Some(found) = Loop::in_line(line) else {
            continue;
        };
        match met.iter_mut().find(|had| had.found == found) {
            Some(had) => had.times += 1,
            None => met.push(Met {
                allowed: allowance(&found),
                found,
                times: 1,
                first: line.trim().to_string(),
            }),
        }
    }
    met
}

/// What a red for a loop means, said after the loops it names.
pub(crate) const MEANING: &str = "QML said a binding or a polish loop (verify::loops): a binding \
     that writes what it reads, or an item polished again inside its own polish. Qt drops the \
     binding's write or gives the polish up and carries on, so neither the exit nor the picture \
     shows it — a red until the loop is gone, or until verify::loops::ALLOWED names it with its \
     reason.";

#[cfg(test)]
mod tests {
    use super::{ALLOWED, Loop, met, refused};

    /// The words of a loop as the app and a QtTest log say them: the
    /// place's url and its line differ, the loop does not.
    #[test]
    fn a_loop_is_known_by_its_file_and_its_words_wherever_it_is_said() {
        let app = "qrc:/qt/qml/platitude/ui/NavSections.qml:162:5: QML NavList: Binding loop \
                   detected for property \"topMargin\":";
        let log = "QWARN  : qmltestrunner::NavRows::test_seat() \
                   file:///C:/w/target/qmltest/42/import/platitude/ui/NavSections.qml:156:5: QML \
                   NavList: Binding loop detected for property \"topMargin\":";
        let found = Loop::in_line(app).expect("a loop");
        assert_eq!(found.file, "NavSections.qml");
        assert_eq!(
            found.said,
            "NavList: Binding loop detected for property \"topMargin\""
        );
        assert_eq!(Loop::in_line(log), Some(found.clone()));
        assert_eq!(
            found.to_string(),
            "NavSections.qml: QML NavList: Binding loop detected for property \"topMargin\""
        );
    }

    /// A bindable property's loop says no expression after it, and a
    /// polish loop is said of the item.
    #[test]
    fn both_kinds_are_read_with_or_without_a_place() {
        let bindable = Loop::in_line(
            "qrc:/qt/qml/platitude/ui/CardText.qml:414:9: QML QQuickTextEdit: Binding loop \
             detected for property \"width\"",
        )
        .expect("a loop");
        assert_eq!(bindable.file, "CardText.qml");
        assert_eq!(
            bindable.said,
            "QQuickTextEdit: Binding loop detected for property \"width\""
        );
        let polish = Loop::in_line(
            "qrc:/qt/qml/platitude/ui/GraphPane.qml:40:5: QML ListView: possible \
             QQuickItem::polish() loop",
        )
        .expect("a loop");
        assert_eq!(polish.file, "GraphPane.qml");
        assert_eq!(polish.said, "ListView: possible QQuickItem::polish() loop");
        let nowhere = Loop::in_line("QML Item: Binding loop detected for property \"x\"")
            .expect("a loop with no context");
        assert_eq!(nowhere.file, "");
        assert_eq!(
            nowhere.to_string(),
            "QML Item: Binding loop detected for property \"x\""
        );
        for quiet in [
            "qrc:/qt/qml/platitude/ui/NavSections.qml:178:9",
            "qrc:/qt/qml/platitude/ui/X.qml:3: TypeError: Cannot read property 'a' of null",
            "QQuickItem(0x1) called polish() inside updatePolish() of ListView",
        ] {
            assert_eq!(Loop::in_line(quiet), None, "{quiet}");
        }
    }

    #[test]
    fn each_loop_is_met_once_with_how_often_it_was_said() {
        let lines = [
            "qrc:/a/A.qml:1:1: QML Item: Binding loop detected for property \"x\":",
            "qrc:/a/A.qml:2:1",
            "INFO bench: stage run=r1",
            "qrc:/a/B.qml:9:1: QML ListView: possible QQuickItem::polish() loop",
            "qrc:/a/A.qml:1:1: QML Item: Binding loop detected for property \"x\":",
        ];
        let met = met(lines);
        assert_eq!(met.len(), 2, "{met:?}");
        assert_eq!((met[0].found.file.as_str(), met[0].times), ("A.qml", 2));
        assert_eq!(met[0].first, lines[0]);
        assert_eq!(met[1].found.file, "B.qml");
        assert!(met.iter().all(|m| m.allowed.is_none()));
        assert_eq!(
            met[0].to_string(),
            "A.qml: QML Item: Binding loop detected for property \"x\" (2 times)"
        );
        assert!(refused(lines[0]).is_some());
        assert!(refused(lines[2]).is_none());
    }

    /// An allowed loop is one the product still has: a file that is gone
    /// takes its row with it.
    #[test]
    fn every_allowed_loop_names_a_file_the_product_has() {
        let root = crate::tree::workspace_root().join("crates/platitude-app/src");
        for allowed in ALLOWED {
            let found = ["ui", "auto"]
                .iter()
                .any(|dir| root.join(dir).join(allowed.file).is_file());
            assert!(found, "{} is no file of ui/ or auto/", allowed.file);
            assert!(
                !allowed.why.trim().is_empty(),
                "{} says no reason",
                allowed.file
            );
            let line = format!("qrc:/qt/qml/x/{}:1:1: QML {}", allowed.file, allowed.said);
            let met = met([line.as_str()]);
            assert_eq!(
                met.first().and_then(|m| m.allowed),
                Some(allowed.why),
                "{} is not read back as the loop it names",
                allowed.file
            );
        }
    }
}
