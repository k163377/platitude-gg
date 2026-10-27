//! What a run shows, read off the QML sources rather than off a run: the
//! window's core (what `Main` and the parts every line stands up compose
//! unconditionally) and what a registered line brings up beyond it
//! (`gate::lines`), closed over what those parts compose in turn.
//!
//! Composition is lexical. `Type {` declares an object of `Type`; the
//! declaring file composes it, unless the declaration stands behind one of
//! the boundaries nothing static can see through:
//!
//! - a popup (a type whose root chain ends in `Popup` / `Menu` / `Dialog`
//!   / `ToolTip` / `Drawer`, or one of those Qt types declared outright)
//!   stands only when opened, and so does everything declared inside one
//!   — the file *opens* these, and a line that names the file as a
//!   subject shows them;
//! - a `Component {}` block is a template, a `Loader` that binds `active:`
//!   builds its `sourceComponent` on a condition (except the harness's own
//!   switches, `keepBuilt` and `Harness.autoAct`, on in every automated
//!   run), and a view's `delegate:` or a `Repeater`'s child is built for
//!   the rows its model has — the file *builds* these, and only a line
//!   that names the built type itself shows it.
//!
//! A file's root declaration is always composition (a wearer shows what it
//! wears — the item tree answers with the outermost type only), and a
//! `"Name.qml"` string is a part loaded by URL (`HarnessSeat`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::graph::{collect, stem_of};

/// The Qt types a component's root chain ends in when it stands only
/// while open.
const POPUP_BASES: [&str; 5] = ["Popup", "Menu", "Dialog", "ToolTip", "Drawer"];

/// The `active:` conditions a Loader binds that are on in every automated
/// run: the harness's own switches, not the product's state.
const ALWAYS_ON: [&str; 2] = ["keepBuilt", "Harness.autoAct"];

/// The file the engine loads, which every run shows.
pub(crate) const MAIN: &str = "Main";

/// Which directory of the app a QML file stands in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Module {
    /// `ui/`, the product.
    Product,
    /// `auto/`, the harness.
    Harness,
}

struct Component {
    module: Module,
    /// What this file's own object is, after the `.` of a qualified type.
    root: Option<String>,
    singleton: bool,
}

/// Every QML component of the app and what each composes, opens or builds.
pub(crate) struct Composition {
    components: BTreeMap<String, Component>,
    composes: BTreeMap<String, BTreeSet<String>>,
    /// What stands once the file's popup is open (module doc).
    opens: BTreeMap<String, BTreeSet<String>>,
    /// The components another one wears as its root: never met by their
    /// own name in the item tree, which answers with the wearer's.
    worn: BTreeSet<String>,
}

impl Composition {
    /// Read off the tree's QML, product and harness alike.
    pub(crate) fn read(root: &Path) -> Result<Composition, String> {
        let mut files = Vec::new();
        collect(
            root,
            &root.join("crates/platitude-app/src"),
            "qml",
            &mut files,
        )?;
        let mut sources = Vec::new();
        for file in files {
            let text =
                std::fs::read_to_string(root.join(&file)).map_err(|e| format!("{file}: {e}"))?;
            sources.push((file, text));
        }
        Ok(Composition::from_sources(&sources))
    }

    /// From `(workspace path, text)` pairs; a path under an `auto/`
    /// directory is the harness's, the rest the product's.
    pub(crate) fn from_sources(sources: &[(String, String)]) -> Composition {
        let mut components = BTreeMap::new();
        let mut stripped = BTreeMap::new();
        for (file, text) in sources {
            // Comments and literals blanked, line for line; a QML string can hold a brace that would throw the
            // frames off (`crate::lex`).
            let code = crate::lex::blanked(text, crate::lex::Lang::Qml, false).0;
            let module = if file.contains("/auto/") {
                Module::Harness
            } else {
                Module::Product
            };
            components.insert(
                stem_of(file),
                Component {
                    module,
                    root: root_type(&code),
                    singleton: text.contains("pragma Singleton"),
                },
            );
            stripped.insert(stem_of(file), (code, text.clone()));
        }
        let worn = components
            .values()
            .filter_map(|c| c.root.clone())
            .filter(|root| components.contains_key(root))
            .collect();
        let mut composition = Composition {
            components,
            composes: BTreeMap::new(),
            opens: BTreeMap::new(),
            worn,
        };
        let mut popups: BTreeSet<String> = composition
            .components
            .keys()
            .filter(|stem| POPUP_BASES.contains(&composition.base(stem).as_str()))
            .cloned()
            .collect();
        popups.extend(POPUP_BASES.iter().map(|base| (*base).to_string()));
        for (stem, (code, raw)) in &stripped {
            for declared in declarations(code, &popups) {
                if declared.kind == *stem || !composition.components.contains_key(&declared.kind) {
                    continue;
                }
                let into = match declared.held {
                    Held::No => &mut composition.composes,
                    Held::Opened => &mut composition.opens,
                    // Built on a condition of its own: shown by a line
                    // that names the built type itself.
                    Held::Built => continue,
                };
                into.entry(stem.clone()).or_default().insert(declared.kind);
            }
            for part in parts_by_url(raw) {
                if part != *stem && composition.components.contains_key(&part) {
                    composition
                        .composes
                        .entry(stem.clone())
                        .or_default()
                        .insert(part);
                }
            }
        }
        composition
    }

    /// The type a component's root chain ends in: a Qt type, or the stem
    /// itself for a file with no readable root.
    fn base(&self, stem: &str) -> String {
        let mut at = stem.to_string();
        let mut seen = BTreeSet::new();
        while let Some(root) = self
            .components
            .get(&at)
            .and_then(|c| c.root.clone())
            .filter(|root| seen.insert(root.clone()))
        {
            at = root;
        }
        at
    }

    pub(crate) fn is_component(&self, stem: &str) -> bool {
        self.components.contains_key(stem)
    }

    pub(crate) fn module(&self, stem: &str) -> Option<Module> {
        self.components.get(stem).map(|c| c.module)
    }

    /// Whether a run can meet a component in the window at all: a
    /// singleton never stands in the tree, so no line shows it and its
    /// readers carry it. A `QtObject`-rooted part is met — the walk visits
    /// what an item holds by name (`WindowCensus.heldBy`).
    pub(crate) fn instantiable(&self, stem: &str) -> bool {
        self.components.get(stem).is_some_and(|c| !c.singleton)
    }

    /// Whether `stem`'s popup opens onto anything: what naming it as a
    /// subject brings up beyond `stem` itself.
    #[cfg(test)]
    pub(crate) fn opens_anything(&self, stem: &str) -> bool {
        self.opens
            .get(stem)
            .is_some_and(|opened| !opened.is_empty())
    }

    /// Whether another component wears `stem` as its root, so no run
    /// meets `stem` by that name.
    pub(crate) fn is_worn(&self, stem: &str) -> bool {
        self.worn.contains(stem)
    }

    /// Who composes `stem` outright, and who opens onto it — for a person
    /// asking why a component is or is not in the core (`verbs --why`).
    pub(crate) fn declarers_of(&self, stem: &str) -> (Vec<String>, Vec<String>) {
        let by = |edges: &BTreeMap<String, BTreeSet<String>>| {
            edges
                .iter()
                .filter(|(_, to)| to.contains(stem))
                .map(|(from, _)| from.clone())
                .collect::<Vec<_>>()
        };
        (by(&self.composes), by(&self.opens))
    }

    /// Whether `by` composes or opens onto `stem` — the declared edge.
    #[cfg(test)]
    fn declares(&self, by: &str, stem: &str) -> bool {
        self.composes.get(by).is_some_and(|c| c.contains(stem))
            || self.opens.get(by).is_some_and(|o| o.contains(stem))
    }

    /// `starts` and everything they compose, transitively; nothing held.
    pub(crate) fn standing<'a>(
        &self,
        starts: impl IntoIterator<Item = &'a str>,
    ) -> BTreeSet<String> {
        let mut seen = BTreeSet::new();
        let mut stack: Vec<String> = starts.into_iter().map(str::to_string).collect();
        while let Some(at) = stack.pop() {
            if !seen.insert(at.clone()) {
                continue;
            }
            stack.extend(self.composes.get(&at).into_iter().flatten().cloned());
        }
        seen
    }

    /// What `subjects` bring up: each subject, what its popup opens onto
    /// (one level — a popup inside that one is its own subject), and all
    /// they compose.
    pub(crate) fn brought<'a>(
        &self,
        subjects: impl IntoIterator<Item = &'a str>,
    ) -> BTreeSet<String> {
        let mut starts: Vec<String> = Vec::new();
        for subject in subjects {
            starts.push(subject.to_string());
            starts.extend(self.opens.get(subject).into_iter().flatten().cloned());
        }
        self.standing(starts.iter().map(String::as_str))
    }
}

/// What a component's own object is: the type of the first object
/// declared — `Item {`, `QtObject {`, `AppCard {` — past any qualifier
/// (`Controls.Menu {` is a `Menu`). `pragma` and `import` lines open
/// nothing.
pub(crate) fn root_type(code: &str) -> Option<String> {
    let bytes = code.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_uppercase() && (i == 0 || !is_ident(bytes[i - 1])) {
            let start = i;
            while i < bytes.len() && (is_ident(bytes[i]) || bytes[i] == b'.') {
                i += 1;
            }
            let mut after = i;
            while after < bytes.len() && (bytes[after] == b' ' || bytes[after] == b'\t') {
                after += 1;
            }
            if after < bytes.len() && bytes[after] == b'{' {
                let word = &code[start..i];
                return Some(word.rsplit('.').next().unwrap_or(word).to_string());
            }
            continue;
        }
        i += 1;
    }
    None
}

/// What stands between a declaration and the window.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Held {
    /// Nothing: declared, it stands.
    No,
    /// A popup — the type's own, or one it is declared inside.
    Opened,
    /// A template, a conditional Loader, a view's delegate: built on a
    /// condition of the data or the state.
    Built,
}

struct Declared {
    kind: String,
    held: Held,
}

struct Frame {
    kind: String,
    /// What whatever is declared inside stands behind.
    held: Held,
    /// A Loader frame whose `sourceComponent` is built on a condition.
    builds: bool,
}

fn is_ident(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

/// Every `Type {` in `code` (comments and string bodies already taken
/// out), with what it stands behind. A template or a conditional Loader
/// outranks a popup: what a Loader inside a dialog builds is not shown by
/// opening the dialog.
fn declarations(code: &str, popups: &BTreeSet<String>) -> Vec<Declared> {
    let bytes = code.as_bytes();
    let mut out = Vec::new();
    let mut frames: Vec<Frame> = Vec::new();
    let mut root_seen = false;
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_uppercase() && (i == 0 || !(is_ident(bytes[i - 1]) || bytes[i - 1] == b'.')) {
            let start = i;
            while i < bytes.len() && is_ident(bytes[i]) {
                i += 1;
            }
            let kind = &code[start..i];
            let mut after = i;
            while after < bytes.len() && (bytes[after] == b' ' || bytes[after] == b'\t') {
                after += 1;
            }
            if after >= bytes.len() || bytes[after] != b'{' {
                continue;
            }
            let property = property_before(bytes, start);
            let enclosing = frames.iter().map(|f| f.held).max().unwrap_or(Held::No);
            // A Repeater's or an Instantiator's child object is its
            // delegate (their default property), built per row; an inline
            // component (`component Row: Item {`) is a template this file
            // instantiates by a name no file is called.
            let under_a_repeater = frames
                .last()
                .is_some_and(|f| f.kind == "Repeater" || f.kind == "Instantiator");
            let built = under_a_repeater
                || property.as_ref().is_some_and(|p| p.inline_component)
                || match property.as_ref().map(|p| p.name.as_str()) {
                    Some("sourceComponent") => {
                        frames.last().is_none_or(|f| f.kind != "Loader" || f.builds)
                    }
                    Some("delegate") => true,
                    _ => false,
                };
            let is_root = !root_seen && frames.is_empty();
            let is_popup = popups.contains(kind);
            let held = if built {
                Held::Built
            } else if is_popup && !is_root {
                enclosing.max(Held::Opened)
            } else {
                enclosing
            };
            out.push(Declared {
                kind: kind.to_string(),
                held,
            });
            // What is declared inside a built object (a delegate, a
            // template, a Loader's component) is built with it; inside a
            // popup, opened with it.
            frames.push(Frame {
                kind: kind.to_string(),
                held: if kind == "Component" || built {
                    Held::Built
                } else if is_popup {
                    enclosing.max(Held::Opened)
                } else {
                    enclosing
                },
                builds: kind == "Loader" && loader_binds_active(bytes, after + 1),
            });
            root_seen = true;
            i = after + 1;
            continue;
        }
        if c == b'{' {
            frames.push(Frame {
                kind: String::new(),
                held: frames.iter().map(|f| f.held).max().unwrap_or(Held::No),
                builds: false,
            });
        } else if c == b'}' {
            frames.pop();
        }
        i += 1;
    }
    out
}

/// What a declaration is the value of.
struct Property {
    name: String,
    /// `component Name: Type {` — a template declared under `Name`.
    inline_component: bool,
}

/// The property a declaration at `at` is the value of (`delegate: X {`),
/// if any: the identifier before a `:` that nothing but whitespace
/// separates from the declaration.
fn property_before(bytes: &[u8], at: usize) -> Option<Property> {
    let mut j = at;
    while j > 0 && (bytes[j - 1] == b' ' || bytes[j - 1] == b'\t' || bytes[j - 1] == b'\n') {
        j -= 1;
    }
    if j == 0 || bytes[j - 1] != b':' {
        return None;
    }
    let mut k = j - 1;
    while k > 0 && (bytes[k - 1] == b' ' || bytes[k - 1] == b'\t') {
        k -= 1;
    }
    let end = k;
    while k > 0 && (is_ident(bytes[k - 1]) || bytes[k - 1] == b'.') {
        k -= 1;
    }
    let name = String::from_utf8_lossy(&bytes[k..end]).into_owned();
    let mut w = k;
    while w > 0 && (bytes[w - 1] == b' ' || bytes[w - 1] == b'\t') {
        w -= 1;
    }
    let inline_component = bytes[..w].ends_with(b"component")
        && (w == b"component".len() || !is_ident(bytes[w - b"component".len() - 1]));
    Some(Property {
        name,
        inline_component,
    })
}

/// Whether the Loader block opening at `from` binds `active:` to anything
/// but the harness's own switches ([`ALWAYS_ON`]).
fn loader_binds_active(bytes: &[u8], from: usize) -> bool {
    let mut depth = 0;
    let mut k = from;
    while k < bytes.len() {
        match bytes[k] {
            b'{' => depth += 1,
            b'}' => {
                if depth == 0 {
                    return false;
                }
                depth -= 1;
            }
            _ if depth == 0
                && bytes[k..].starts_with(b"active:")
                && (k == 0 || !is_ident(bytes[k - 1])) =>
            {
                let end = bytes[k..]
                    .iter()
                    .position(|b| *b == b'\n')
                    .map_or(bytes.len(), |n| k + n);
                let expr = String::from_utf8_lossy(&bytes[k + 7..end]);
                return !ALWAYS_ON.iter().any(|on| expr.contains(on));
            }
            _ => {}
        }
        k += 1;
    }
    false
}

/// The components a file loads by URL: every `"Name.qml"` in it.
fn parts_by_url(raw: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut rest = raw;
    while let Some(open) = rest.find('"') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('"') else {
            break;
        };
        let word = &after[..close];
        if let Some(name) = word.strip_suffix(".qml")
            && name.starts_with(|c: char| c.is_ascii_uppercase())
            && name.bytes().all(is_ident)
        {
            parts.push(name.to_string());
        }
        rest = &after[close + 1..];
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::{Composition, Module, root_type};
    use std::collections::BTreeSet;

    fn sources(files: &[(&str, &str)]) -> Vec<(String, String)> {
        files
            .iter()
            .map(|(name, text)| {
                let dir = if name.starts_with("Auto") || name.starts_with("Window") {
                    "auto"
                } else {
                    "ui"
                };
                (
                    format!("crates/platitude-app/src/{dir}/{name}.qml"),
                    (*text).to_string(),
                )
            })
            .collect()
    }

    fn set(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|n| (*n).to_string()).collect()
    }

    #[test]
    fn the_root_type_is_read_past_pragma_imports_and_qualifiers() {
        assert_eq!(
            root_type("pragma ComponentBehavior: Bound\nimport QtQuick\n\nAppDialog {\n}\n"),
            Some("AppDialog".into())
        );
        assert_eq!(root_type("Controls.Menu {\n}\n"), Some("Menu".into()));
        assert_eq!(root_type("QtObject {}\n"), Some("QtObject".into()));
        assert_eq!(root_type("import QtQuick\n"), None);
    }

    /// A popup declared as a child stands only when opened, and so does
    /// what is declared inside it; a file whose root is a popup type is
    /// shown by its wearers through that root.
    #[test]
    fn a_popup_is_held_by_its_declarer_and_holds_its_contents() {
        let comp = Composition::from_sources(&sources(&[
            ("Main", "Item {\n    Pane {}\n    RowMenu {}\n}\n"),
            ("Pane", "Item {\n    Label {}\n}\n"),
            ("Label", "Text {}\n"),
            (
                "RowMenu",
                "Item {\n    Menu {\n        Row {}\n    }\n    Badge {}\n}\n",
            ),
            ("Row", "MenuItem {}\n"),
            ("Badge", "Item {}\n"),
        ]));
        // `RowMenu` is Item-rooted: composed. Its `Menu {}` is Qt's, and
        // the `Row {}` inside stands only when it is open.
        assert_eq!(
            comp.standing(["Main"]),
            set(&["Main", "Pane", "Label", "RowMenu", "Badge"])
        );
        assert_eq!(
            comp.brought(["RowMenu"]),
            set(&["RowMenu", "Badge", "Row"]),
            "naming the holder opens its menu onto the rows"
        );
        assert!(comp.opens_anything("RowMenu"));
        assert!(!comp.opens_anything("Pane"));
    }

    #[test]
    fn a_popup_rooted_file_is_held_where_declared_and_shows_its_wearer_root() {
        let comp = Composition::from_sources(&sources(&[
            ("Main", "Item {\n    SettingsDialog {}\n}\n"),
            ("AppDialog", "Dialog {}\n"),
            (
                "SettingsDialog",
                "AppDialog {\n    Section {}\n    Loader {\n        active: open\n        sourceComponent: Deep {}\n    }\n}\n",
            ),
            ("Section", "Item {}\n"),
            ("Deep", "Item {}\n"),
        ]));
        assert_eq!(comp.standing(["Main"]), set(&["Main"]));
        // Opened: the dialog, its root type, its contents — but not what a
        // conditional Loader inside it holds.
        assert_eq!(
            comp.brought(["SettingsDialog"]),
            set(&["SettingsDialog", "AppDialog", "Section"])
        );
        assert_eq!(comp.brought(["Deep"]), set(&["Deep"]));
    }

    /// A Loader on the harness's switches builds in every run; one on the
    /// product's state does not, and a template is built by nobody.
    #[test]
    fn loaders_and_templates_hold_unless_the_switch_is_the_harnesss() {
        let comp = Composition::from_sources(&sources(&[
            (
                "Main",
                "Item {\n    Loader {\n        active: page.keepBuilt\n        sourceComponent: Host {}\n    }\n    \
                 Loader {\n        active: page.planShown\n        sourceComponent: Plan {}\n    }\n    \
                 Loader {\n        sourceComponent: Always {}\n    }\n    \
                 Component {\n        id: picker\n        Picker {}\n    }\n    \
                 ListView {\n        delegate: Row {}\n    }\n    \
                 Repeater {\n        model: 3\n        Mark {}\n    }\n    \
                 component Entry: Rectangle {\n        Badge {}\n    }\n    Entry {}\n}\n",
            ),
            ("Host", "Item {}\n"),
            ("Plan", "Item {}\n"),
            ("Always", "Item {}\n"),
            ("Picker", "Item {}\n"),
            ("Row", "Item {}\n"),
            ("Mark", "Item {}\n"),
            ("Badge", "Item {}\n"),
        ]));
        assert_eq!(comp.standing(["Main"]), set(&["Main", "Host", "Always"]));
        // Built parts are not opened onto: each is named itself.
        assert_eq!(comp.brought(["Main"]), set(&["Main", "Host", "Always"]));
        assert_eq!(comp.brought(["Plan"]), set(&["Plan"]));
        assert!(!comp.opens_anything("Main"));
    }

    #[test]
    fn a_part_loaded_by_url_is_composed_and_the_harness_is_told_apart() {
        let comp = Composition::from_sources(&sources(&[
            (
                "Main",
                "Item {\n    HarnessSeat {\n        part: \"WindowHarness.qml\"\n    }\n}\n",
            ),
            ("HarnessSeat", "Item {}\n"),
            ("WindowHarness", "Item {\n    AutoActDriver {}\n}\n"),
            ("AutoActDriver", "Item {}\n"),
        ]));
        assert_eq!(
            comp.standing(["Main"]),
            set(&["Main", "HarnessSeat", "WindowHarness", "AutoActDriver"])
        );
        assert_eq!(comp.module("WindowHarness"), Some(Module::Harness));
        assert_eq!(comp.module("HarnessSeat"), Some(Module::Product));
    }

    #[test]
    fn a_singleton_is_never_met_and_a_qtobject_part_is() {
        let comp = Composition::from_sources(&sources(&[
            ("Theme", "pragma Singleton\nQtObject {}\n"),
            ("State", "QtObject {}\n"),
            ("DeepState", "State {}\n"),
            ("Pane", "Item {}\n"),
        ]));
        assert!(!comp.instantiable("Theme"));
        assert!(
            comp.instantiable("State"),
            "held by name, the walk visits it"
        );
        assert!(comp.instantiable("DeepState"));
        assert!(comp.instantiable("Pane"));
        assert!(!comp.instantiable("Nothing"));
    }

    /// A brace in a string or a comment is no object, and a qualified or
    /// attached name (`Qt.point`, `Component.onCompleted`) declares none.
    /// A single-quoted or template string holding a brace does not throw
    /// the frames off either: the popup after it still holds its rows.
    #[test]
    fn text_that_is_no_declaration_declares_nothing() {
        let comp = Composition::from_sources(&sources(&[
            (
                "Main",
                "Item {\n    // Pane { in a comment\n    text: \"Pane {\"\n    Component.onCompleted: {\n        \
                 const p = Qt.point(1, 2)\n    }\n    other: Pane.width\n    tick: '}'\n    tpl: `{`\n    \
                 Menu {\n        Row {}\n    }\n}\n",
            ),
            ("Pane", "Item {}\n"),
            ("Row", "Item {}\n"),
        ]));
        assert_eq!(comp.standing(["Main"]), set(&["Main"]));
        assert!(!comp.declares("Main", "Pane"));
        assert_eq!(comp.brought(["Main"]), set(&["Main", "Row"]));
        assert!(!comp.is_worn("Row") && !comp.is_worn("Main"));
    }

    #[test]
    fn a_root_another_file_wears_is_worn_and_its_declarers_are_named() {
        let comp = Composition::from_sources(&sources(&[
            ("Main", "Item {\n    GraphFind {}\n    RowMenu {}\n}\n"),
            ("FindBar", "Item {}\n"),
            ("GraphFind", "FindBar {}\n"),
            ("RowMenu", "Item {\n    Menu {\n        Row {}\n    }\n}\n"),
            ("Row", "Item {}\n"),
        ]));
        assert!(comp.is_worn("FindBar") && !comp.is_worn("GraphFind"));
        assert_eq!(
            comp.declarers_of("Row"),
            (Vec::new(), vec!["RowMenu".to_string()])
        );
        assert_eq!(
            comp.declarers_of("FindBar"),
            (vec!["GraphFind".to_string()], Vec::new())
        );
    }
}
