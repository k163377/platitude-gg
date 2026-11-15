//! The boundary between the product's QML modules and the harness's,
//! counted by machine (.claude/rules/app-ui.md「QML モジュール」). Who
//! imports the product's own modules is `super::singletons`'.
//!
//! A shipped build carries no `platitude.auto`, and a static type
//! reference resolves at load time, so a product line naming a harness
//! type leaves the shipped build with no window; a read off a harness
//! singleton is no load failure but fails each time its line is
//! evaluated. Every xtask builds the app with the feature
//! (`crate::app_build::HARNESS_FEATURE`), so nothing else here notices.
//!
//! Naming a type is what QML resolves: an object declaration
//! (`WindowHarness {`) and a typed property (`property TabProbe probe`),
//! and for a singleton, which is never declared, a read off it
//! (`PageSettled.settled(page)`); a name in a comment or a string is a
//! reference.
//!
//! The Rust side's half: the singleton the feature registers
//! (`harness::singleton::Harness`) is not in the qmldir, and the product
//! may neither read it (a shipped build fails the line) nor ask
//! `AppBackend` for its members. Both are read out of the singleton's
//! source, so a property added there is out of reach at once.

use std::path::Path;

/// The product's QML, the directories under it included, and the harness
/// module whose names it may not use.
const PRODUCT: &str = "crates/platitude-app/src/ui";
const HARNESS_QMLDIR: &str = "crates/platitude-app/src/auto/qmldir";
/// The QML-facing harness singleton, whose members say what the product
/// may not ask `AppBackend` for.
const HARNESS_SINGLETON: &str = "crates/platitude-app/src/harness/singleton.rs";
/// What the product calls it, and what it calls the object it may ask.
const SINGLETON: &str = "Harness";
const BACKEND: &str = "AppBackend";

/// One failure per product line that reaches for the harness, and how many
/// names were looked for.
pub(super) fn check(root: &Path) -> Result<(Vec<String>, usize), String> {
    let types = harness_types(root)?;
    let members = singleton_members(root)?;
    let mut failures = Vec::new();
    let product = root.join(PRODUCT);
    let mut files = Vec::new();
    super::collect(&product, &mut files)?;
    files.retain(|path| path.extension().is_some_and(|e| e == "qml"));
    files.sort();
    for path in files {
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("could not read {}: {e}", path.display()))?;
        let shown = super::relative(&product, &path);
        for (number, line) in text.lines().enumerate() {
            if let Some(named) = types.iter().find(|ty| ty.named_by(line)) {
                failures.push(format!(
                    "{PRODUCT}/{shown}:{} names the harness type {} — a shipped build has no \
                     `platitude.auto`: the file would fail to load, or the line each time it \
                     runs. Reach it through `HarnessSeat` instead (.claude/rules/app-ui.md)",
                    number + 1,
                    named.name
                ));
            }
            if reads(line, SINGLETON, "") {
                failures.push(format!(
                    "{PRODUCT}/{shown}:{} reads the harness singleton — a shipped build registers \
                     no `{SINGLETON}`, and a read off a singleton the engine cannot resolve fails \
                     each time the line runs. The product asks its own properties and something \
                     else writes them (.claude/rules/app-ui.md)",
                    number + 1
                ));
            }
            if let Some(member) = members.iter().find(|member| reads(line, BACKEND, member)) {
                failures.push(format!(
                    "{PRODUCT}/{shown}:{} asks `{BACKEND}` for {member}, which is the harness's \
                     ({HARNESS_SINGLETON}). Nothing the product shows may turn on what a run was \
                     told to do (.claude/rules/app-ui.md)",
                    number + 1
                ));
            }
        }
    }
    Ok((failures, types.len() + members.len()))
}

/// Everything `Harness` puts in front of QML: the `qproperty!` names as
/// written, and the `#[qslot]` fns in camel case — only those, since the
/// file's own `Default` is a `fn` QML never sees.
fn singleton_members(root: &Path) -> Result<Vec<String>, String> {
    let path = root.join(HARNESS_SINGLETON);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("could not read {}: {e}", path.display()))?;
    let mut members = Vec::new();
    let mut is_slot = false;
    for line in text.lines() {
        let code = line.trim_start();
        if let Some(rest) = code.strip_prefix("qproperty!(\"")
            && let Some(name) = rest.split('"').next()
        {
            members.push(name.to_string());
        } else if is_slot
            && let Some(rest) = code.strip_prefix("fn ")
            && let Some(name) = rest.split('(').next()
        {
            members.push(camel(name));
        }
        // Assumes `#[qslot]` stands on the line right above its `fn`.
        is_slot = code == "#[qslot]";
    }
    if members.is_empty() {
        return Err(format!("{HARNESS_SINGLETON} puts nothing in front of QML"));
    }
    Ok(members)
}

/// `open_session_count` → `openSessionCount`, the name Qt registers the
/// slot under (`#[qobject(ConvertToCamelCase)]`).
fn camel(snake: &str) -> String {
    let mut out = String::with_capacity(snake.len());
    let mut rising = false;
    for ch in snake.chars() {
        if ch == '_' {
            rising = true;
        } else if rising {
            out.extend(ch.to_uppercase());
            rising = false;
        } else {
            out.push(ch);
        }
    }
    out
}

/// Whether the code of one line of QML ([`code_of`]) reads
/// `<object>.<member>` — or, with an empty member, anything off `object`.
/// Both ends of the name must end: `WindowHarness.qml` is not `Harness.`,
/// and `AppBackend.autoFetchMinutes` is not `AppBackend.autoAct`.
fn reads(line: &str, object: &str, member: &str) -> bool {
    let code = code_of(line);
    let looked_for = if member.is_empty() {
        format!("{object}.")
    } else {
        format!("{object}.{member}")
    };
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let mut at = 0;
    while let Some(found) = code[at..].find(&looked_for) {
        let start = at + found;
        let end = start + looked_for.len();
        let before_ends = code[..start].chars().next_back().is_none_or(|c| !word(c));
        let after_ends = member.is_empty() || !code[end..].starts_with(word);
        if before_ends && after_ends {
            return true;
        }
        at = end;
    }
    false
}

/// One line of QML with its string literals emptied and a `//` comment cut
/// off, so a name either holds (`"PageSettled.qml"`, `// as PageSettled.settled
/// says`) is a reference. The quotes stay, as the ends of the words beside them.
fn code_of(line: &str) -> String {
    let mut code = String::with_capacity(line.len());
    let mut quote = None;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match quote {
            Some(open) => {
                if c == '\\' {
                    chars.next();
                } else if c == open {
                    quote = None;
                    code.push(c);
                }
            }
            None if matches!(c, '"' | '\'' | '`') => {
                quote = Some(c);
                code.push(c);
            }
            None if c == '/' && chars.peek() == Some(&'/') => break,
            None => code.push(c),
        }
    }
    code
}

/// One type a qmldir exports.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Exported {
    pub(super) name: String,
    pub(super) singleton: bool,
}

impl Exported {
    /// Whether one line of QML names this type the way the engine resolves
    /// it: a singleton by a read off it, any other by [`names_type`].
    fn named_by(&self, line: &str) -> bool {
        if self.singleton {
            reads(line, &self.name, "")
        } else {
            names_type(line, &self.name)
        }
    }
}

/// The types `platitude.auto` exports, off its own qmldir.
fn harness_types(root: &Path) -> Result<Vec<Exported>, String> {
    let path = root.join(HARNESS_QMLDIR);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("could not read {}: {e}", path.display()))?;
    let types: Vec<Exported> = text.lines().filter_map(exported).collect();
    if types.is_empty() {
        return Err(format!("{HARNESS_QMLDIR} lists no types"));
    }
    Ok(types)
}

/// The type a qmldir line exports, by Qt's grammar (`QQmlDirParser::parse`):
/// `[singleton] <Type> [<version>] <file>`, where a `.js` / `.mjs` file is a
/// script and a word starting `#` starts a comment. A QML type's name starts
/// upper case, so a directive (`module`, `plugin` …) exports nothing, and
/// neither does an `internal` line: that type is the module's own.
pub(super) fn exported(line: &str) -> Option<Exported> {
    let words: Vec<&str> = line
        .split_whitespace()
        .take_while(|word| !word.starts_with('#'))
        .collect();
    let (singleton, rest) = match words.as_slice() {
        ["singleton", rest @ ..] => (true, rest),
        rest => (false, rest),
    };
    let (name, file) = match rest {
        [name, file] | [name, _, file] => (*name, *file),
        _ => return None,
    };
    let script = file.ends_with(".js") || file.ends_with(".mjs");
    (name.starts_with(|c: char| c.is_ascii_uppercase()) && !script).then(|| Exported {
        name: name.to_string(),
        singleton,
    })
}

/// Whether one line of QML names `type` the way the engine resolves —
/// an object declaration or a typed property.
fn names_type(line: &str, type_name: &str) -> bool {
    let code = line.trim_start();
    if code.starts_with("//") || code.starts_with("///") {
        return false;
    }
    let Some(rest) = code.strip_prefix(type_name) else {
        return declares_property(code, type_name);
    };
    // `Foo {` and `Foo{` are declarations; `Foobar {` is a different type.
    matches!(rest.trim_start().chars().next(), Some('{'))
        && rest.chars().next().is_none_or(|c| c == ' ' || c == '{')
}

/// `property Foo bar` / `required property Foo bar` / `readonly property
/// Foo bar` — the other way a QML file has to have the type resolved.
fn declares_property(code: &str, type_name: &str) -> bool {
    let mut words = code
        .split_whitespace()
        .skip_while(|word| matches!(*word, "required" | "readonly" | "default" | "final"));
    words.next() == Some("property") && words.next() == Some(type_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_declaration_and_a_typed_property_are_naming_the_type() {
        assert!(names_type("    WindowHarness {", "WindowHarness"));
        assert!(names_type("WindowHarness{", "WindowHarness"));
        assert!(names_type("    property TabProbe probe", "TabProbe"));
        assert!(names_type(
            "    required property TabProbe probe",
            "TabProbe"
        ));
        assert!(names_type(
            "    readonly property TabProbe probe: null",
            "TabProbe"
        ));
    }

    #[test]
    fn prose_and_neighbouring_names_are_not() {
        // The product's comments point at the harness constantly.
        assert!(!names_type(
            "    // reached by `WindowHarness`",
            "WindowHarness"
        ));
        assert!(!names_type(
            "    /// Automation: WindowHarness reads this",
            "WindowHarness"
        ));
        // A longer name that merely starts with one.
        assert!(!names_type("    TabProbeSeat {", "TabProbe"));
        // The property's own name only.
        assert!(!names_type("    property var tabProbe", "TabProbe"));
        // A call through a seat says nothing about the type.
        assert!(!names_type("    harness.ask().begin()", "WindowHarness"));
    }

    #[test]
    fn a_read_off_the_singleton_is_one_and_a_longer_name_is_not() {
        assert!(reads(
            "        active: Harness.autoAct !== \"\"",
            "Harness",
            ""
        ));
        // The product's own seat type, and the file it loads by URL.
        assert!(!reads("    HarnessSeat {", "Harness", ""));
        assert!(!reads("        part: \"WindowHarness.qml\"", "Harness", ""));
        assert!(reads(
            "        if (AppBackend.autoAct === \"delete-gone\")",
            "AppBackend",
            "autoAct"
        ));
        // The app's own auto-fetch setting.
        assert!(!reads(
            "        fetchField.text = AppBackend.autoFetchMinutes > 0",
            "AppBackend",
            "autoAct"
        ));
        assert!(!reads(
            "    /// written by the harness (`Harness.autoAct`)",
            "Harness",
            ""
        ));
    }

    #[test]
    fn a_name_in_a_string_or_a_trailing_comment_is_not_read() {
        assert!(!reads(
            "    shown: seat.ready // as Harness.autoAct says",
            "Harness",
            ""
        ));
        assert!(!reads("    part: 'Harness.qml'", "Harness", ""));
        // An escaped quote does not end the string, and the code after it counts.
        assert!(reads(
            "    text: \"say \\\"Harness.\\\"\" + Harness.autoAct",
            "Harness",
            ""
        ));
        assert_eq!(code_of("    a: \"x // y\" // z"), "    a: \"\" ");
    }

    #[test]
    fn a_slot_is_looked_for_under_the_name_qt_registers_it_by() {
        assert_eq!(camel("open_session_count"), "openSessionCount");
        assert_eq!(camel("report"), "report");
    }

    #[test]
    fn only_the_slots_and_the_properties_are_read_off_the_singleton() {
        let members = singleton_members(&crate::tree::workspace_root()).expect("read the harness");
        assert!(members.iter().any(|name| name == "autoAct"));
        assert!(members.iter().any(|name| name == "openSessionCount"));
        assert!(
            !members.iter().any(|name| name == "default"),
            "the file's own `Default` is a `fn` QML has never heard of"
        );
    }

    #[test]
    fn a_qmldir_line_exports_its_type_and_says_whether_it_is_a_singleton() {
        let singleton = |name: &str| Exported {
            name: name.to_string(),
            singleton: true,
        };
        let plain = |name: &str| Exported {
            name: name.to_string(),
            singleton: false,
        };
        for (line, wanted) in [
            (
                "singleton PageSettled 1.0 PageSettled.qml",
                singleton("PageSettled"),
            ),
            ("singleton Awaited Awaited.qml", singleton("Awaited")),
            ("TabProbe 1.0 TabProbe.qml", plain("TabProbe")),
            ("TabProbe TabProbe.qml", plain("TabProbe")),
            (
                "TabProbe 1.0 TabProbe.qml # the tab's probe",
                plain("TabProbe"),
            ),
        ] {
            assert_eq!(exported(line), Some(wanted), "{line}");
        }
        for line in [
            "module platitude.auto",
            "internal Seat Seat.qml",
            "internal Seat 1.0 Seat.qml",
            "# Note 1.0 Note.qml",
            "plugin auto",
            "Tools 1.0 tools.js",
            "Tools 1.0 tools.mjs",
            "",
        ] {
            assert_eq!(exported(line), None, "{line}");
        }
        let names = harness_types(&crate::tree::workspace_root()).expect("read the qmldir");
        for name in ["Awaited", "PageSettled"] {
            assert!(names.contains(&singleton(name)), "{names:#?}");
        }
    }

    /// A singleton is never declared: a product line reaches one by reading
    /// off it, and its qmldir line puts `singleton` in front of its name.
    #[test]
    fn a_read_off_a_harness_singleton_is_caught() {
        let root = crate::yard::Yard::new("structure-harness-singleton");
        let write = |path: &str, text: &str| {
            let file = root.join(path);
            std::fs::create_dir_all(file.parent().expect("a parent")).expect("the fixture's dirs");
            std::fs::write(file, text).expect("the fixture");
        };
        write(
            HARNESS_QMLDIR,
            "module platitude.auto\nTabProbe 1.0 TabProbe.qml\n\
             singleton PageSettled 1.0 PageSettled.qml\n",
        );
        write(HARNESS_SINGLETON, "    qproperty!(\"autoAct\", String),\n");
        // Only line 3 reads it: the others mention it in a comment or a string.
        write(
            &format!("{PRODUCT}/RepoPage.qml"),
            "Item {\n    /// PageSettled.settled(page) waits on it\n    \
             readonly property bool ready: PageSettled.settled(page)\n    \
             readonly property bool shown: seat.ready // as PageSettled.settled says\n    \
             readonly property string part: \"PageSettled.qml\"\n}\n",
        );
        // A module of singletons under the product's directory is the product too.
        write(
            &format!("{PRODUCT}/words/Words.qml"),
            "QtObject {\n    readonly property bool ready: PageSettled.settled(null)\n}\n",
        );
        let (failures, names) = check(&root).expect("scan the fixture");
        assert_eq!(names, 3, "two types and the singleton's one member");
        assert_eq!(failures.len(), 2, "{failures:#?}");
        assert!(
            failures[0].contains("RepoPage.qml:3 names the harness type PageSettled"),
            "{failures:#?}"
        );
        assert!(
            failures[1].contains("ui/words/Words.qml:2 names the harness type PageSettled"),
            "{failures:#?}"
        );
    }

    /// The rule the count is for: the tree it runs on passes it.
    #[test]
    fn the_product_reaches_for_nothing_a_shipped_build_lacks() {
        let (failures, names) = check(&crate::tree::workspace_root()).expect("scan the product");
        assert!(failures.is_empty(), "{failures:#?}");
        assert!(
            names > 1,
            "the harness puts more than one name out of reach"
        );
    }
}
