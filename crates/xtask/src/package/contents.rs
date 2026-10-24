//! What the bundle carries of Qt, by name (P5-確認事項 §3): the plugins and
//! QML modules package puts in by hand, the frameworks that come with them,
//! and why each thing Qt would also bring stays out. Then the readings that
//! hold a built bundle, and the app's imports, to those lists — the
//! bundle's directories, a module's qmldir, qmlimportscanner's answer.
//! Nothing here runs a tool, so all of it is checked on any desk.
//!
//! macdeployqt takes every plugin of a kind it deploys, and a module's
//! directory with every module under it: `QtQuick`'s holds each style and
//! LocalStorage, which brings QtSql and the SQL drivers. Here nothing comes
//! in that a list does not name, and what comes along with a named thing —
//! a framework a plugin loads, a module an import reaches — is named too:
//! a Qt that moves either turns package red, by name.

use std::path::Path;

/// The Qt plugins, as they stand under the pinned Qt's `plugins`.
pub(super) const QT_PLUGINS: [&str; 6] = [
    // The platform on a Mac.
    "platforms/libqcocoa.dylib",
    // The one the stand runs on: a snapshot starts headless the way every
    // run here does.
    "platforms/libqoffscreen.dylib",
    // The image formats as they are until P5-確認事項 §1 settles the range.
    "imageformats/libqgif.dylib",
    "imageformats/libqico.dylib",
    "imageformats/libqjpeg.dylib",
    "imageformats/libqsvg.dylib",
];

/// Why the network plugins stay out. QtNetwork itself comes, as a library
/// QtQml loads.
const NETWORK: &str = "a network plugin: none goes in (実装計画 §11.2)";

/// The plugin directories macdeployqt fills that stay out, and why — each
/// reason what the thing is, then why the bundle does without it.
const QT_PLUGINS_LEFT_OUT: [(&str, &str); 5] = [
    (
        "iconengines",
        "QIcon's SVG engine: no QIcon is made from an SVG — the marks are QML images, which \
         imageformats reads, and the icon is the bundle's icns",
    ),
    ("networkinformation", NETWORK),
    (
        "sqldrivers",
        "the SQL drivers: they come with QtSql, which only QtQuick.LocalStorage would bring",
    ),
    (
        "styles",
        "QtWidgets' styles: a QGuiApplication makes no QStyle, and libqmacstyle alone would \
         bring QtWidgets",
    ),
    ("tls", NETWORK),
];

/// The QML modules, each the files of its own directory under the pinned
/// Qt's `qml`: what the engine looks up on disk as the app runs. Their QML
/// comes from their libraries' resources (each qmldir says `prefer`), so a
/// directory's files — qmldir, plugin, type information — are the whole of
/// its module; the directories under it are other modules, or Qt Design
/// Studio's.
pub(super) const QML_MODULES: [&str; 13] = [
    // QtQuick's qmldir imports it, and it imports the two after it.
    "QtQml",
    "QtQml.Models",
    "QtQml.WorkerScript",
    "QtQuick",
    // `SplitHandleBar` imports it bare, and Fusion answers it (see
    // `OTHER_STYLE`).
    "QtQuick.Controls",
    // Fusion's fallback: its qmldir imports it.
    "QtQuick.Controls.Basic",
    // The app's style.
    "QtQuick.Controls.Fusion",
    // Fusion's QML imports these two, and Basic's the second.
    "QtQuick.Controls.Fusion.impl",
    "QtQuick.Controls.impl",
    // What every style's controls are made from.
    "QtQuick.Templates",
    // The folder and file dialogs — Cocoa's, on a Mac.
    "QtQuick.Dialogs",
    "QtQuick.Layouts",
    "QtQuick.Window",
];

/// Why a style but Fusion stays out.
const OTHER_STYLE: &str = "a style QtQuick.Controls may choose: the app's is Fusion — Main.qml \
     imports QtQuick.Controls.Fusion before anything imports QtQuick.Controls, and Qt then takes \
     Fusion as the style (qquickstyleplugin.cpp)";

/// Why the dialogs Qt draws itself stay out.
const DRAWN_DIALOGS: &str = "the dialogs Qt draws itself, for where the platform has none: a \
     Mac's are Cocoa's (their library still comes, as QtQuickDialogs2's)";

/// What qmlimportscanner reaches from `ui/` beside [`QML_MODULES`], and why
/// each stays out, as [`QT_PLUGINS_LEFT_OUT`] says it.
const QML_LEFT_OUT: [(&str, &str); 19] = [
    ("QML", "the engine's own types: no qmldir is read for them"),
    (
        "Qt.labs.folderlistmodel",
        "a module only the dialogs Qt draws itself import (QtQuick.Dialogs.quickimpl)",
    ),
    (
        "QtQuick.Controls.Basic.impl",
        "a module only Basic's controls import, each of which Fusion replaces",
    ),
    ("QtQuick.Controls.FluentWinUI3", OTHER_STYLE),
    ("QtQuick.Controls.FluentWinUI3.impl", OTHER_STYLE),
    ("QtQuick.Controls.Imagine", OTHER_STYLE),
    ("QtQuick.Controls.Imagine.impl", OTHER_STYLE),
    ("QtQuick.Controls.Material", OTHER_STYLE),
    ("QtQuick.Controls.Material.impl", OTHER_STYLE),
    ("QtQuick.Controls.Universal", OTHER_STYLE),
    ("QtQuick.Controls.Universal.impl", OTHER_STYLE),
    ("QtQuick.Controls.iOS", OTHER_STYLE),
    ("QtQuick.Controls.iOS.impl", OTHER_STYLE),
    ("QtQuick.Controls.macOS", OTHER_STYLE),
    ("QtQuick.Controls.macOS.impl", OTHER_STYLE),
    ("QtQuick.Dialogs.quickimpl", DRAWN_DIALOGS),
    (
        "QtQuick.Effects",
        "a module only styles the app does not use import",
    ),
    ("QtQuick.NativeStyle", "what the macOS style is made from"),
    (
        "QtQuick.Shapes",
        "a module only Basic's SelectionRectangle imports, which Fusion replaces",
    ),
];

/// The modules under the directories of carried ones that nothing in `ui/`
/// reaches — what a copy of the whole directory brings — and why each
/// stays out, as [`QT_PLUGINS_LEFT_OUT`] says it.
const QML_UNREACHED: [(&str, &str); 9] = [
    (
        "QtQml.DesignSupport",
        "Qt Design Studio's support: nothing in ui/ reaches it",
    ),
    (
        "QtQml.XmlListModel",
        "XML list models: nothing in ui/ reaches it",
    ),
    (
        "QtQuick.Controls.Native",
        "the native styles' controls: nothing in ui/ reaches them",
    ),
    (
        "QtQuick.LocalStorage",
        "a database for QML: nothing in ui/ reaches it, and its plugin alone would bring QtSql \
         and with it the SQL drivers",
    ),
    (
        "QtQuick.Particles",
        "particle effects: nothing in ui/ reaches them",
    ),
    (
        "QtQuick.Shapes.DesignHelpers",
        "Qt Design Studio's shape helpers: nothing in ui/ reaches them",
    ),
    (
        "QtQuick.VectorImage",
        "vector images as shapes: nothing in ui/ reaches it",
    ),
    (
        "QtQuick.VectorImage.Helpers",
        "the vector images' helpers: nothing in ui/ reaches them",
    ),
    (
        "QtQuick.tooling",
        "the type descriptions tools read: nothing in ui/ reaches it",
    ),
];

/// The frameworks: what the app's binary and the libraries above load, as
/// macdeployqt follows them. The list is a result, not a choice: every
/// other framework stays out with the plugin or module that would load it
/// — QtWidgets with libqmacstyle, QtSql with QtQuick.LocalStorage.
pub(super) const FRAMEWORKS: [&str; 24] = [
    // The app's binary and what it loads. It links QtQuickTest, and so
    // QtTest, though nothing uses them: qtbridge-runtime links it, and the
    // Mac's linker keeps a library nothing uses.
    "QtCore",
    "QtDBus",
    "QtGui",
    "QtNetwork",
    "QtOpenGL",
    "QtQml",
    "QtQmlMeta",
    "QtQmlModels",
    "QtQmlWorkerScript",
    "QtQuick",
    "QtQuickTest",
    "QtTest",
    // QtQuick.Controls and its two styles.
    "QtQuickControls2",
    "QtQuickControls2Basic",
    "QtQuickControls2BasicStyleImpl",
    "QtQuickControls2Fusion",
    "QtQuickControls2FusionStyleImpl",
    "QtQuickControls2Impl",
    "QtQuickTemplates2",
    // QtQuick.Dialogs.
    "QtQuickDialogs2",
    "QtQuickDialogs2QuickImpl",
    "QtQuickDialogs2Utils",
    // QtQuick.Layouts.
    "QtQuickLayouts",
    // imageformats/libqsvg.
    "QtSvg",
];

/// The app's own modules, which no directory of Qt's holds: `platitude.ui`
/// is compiled into it, `platitude` registered by the bridge.
const APP_OWN: [&str; 2] = ["platitude", "platitude.ui"];

/// Where Qt finds the bundle's plugins and QML, relative to `Contents`, as
/// `Contents/Resources/qt.conf`. Qt 6.12 finds both there without one, and
/// macdeployqt writes none; the bundle says its layout rather than leave it
/// to Qt's guess.
pub(super) const QT_CONF: &str = "[Paths]\nPlugins = PlugIns\nQmlImports = Resources/qml\n";

/// A module's directory under `qml`.
pub(super) fn module_dir(module: &str) -> String {
    module.replace('.', "/")
}

/// The link a module's directory holds to its plugin, which sits in
/// `Contents/PlugIns/quick`: relative, as macdeployqt writes it — up out of
/// the module's directories, `qml` and `Resources`.
pub(super) fn plugin_link(module: &str, file: &str) -> String {
    let up = module.split('.').count() + 2;
    format!("{}PlugIns/quick/{file}", "../".repeat(up))
}

/// Where a file of a module's directory goes in the bundle.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Goes {
    /// Beside the module's qmldir, in `Contents/Resources/qml`.
    WithModule,
    /// In `Contents/PlugIns/quick`, linked from the module's directory: the
    /// module's plugin.
    Quick,
    /// Nowhere: a debug build of the plugin, which macdeployqt leaves too.
    Nowhere,
}

/// Where the file `name` of a module's directory goes.
pub(super) fn goes(name: &str) -> Goes {
    if name.ends_with("_debug.dylib") {
        Goes::Nowhere
    } else if name.ends_with(".dylib") {
        Goes::Quick
    } else {
        Goes::WithModule
    }
}

/// The plugin a qmldir names: `[optional] plugin <name> [<path>]`.
fn plugin_of(qmldir: &str) -> Option<&str> {
    qmldir.lines().find_map(|line| {
        let line = line.trim();
        let rest = line.strip_prefix("optional ").unwrap_or(line);
        rest.strip_prefix("plugin ")?.split_whitespace().next()
    })
}

/// The file a module's plugin is on a Mac.
fn plugin_file(plugin: &str) -> String {
    format!("lib{plugin}.dylib")
}

/// One import qmlimportscanner names.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Import {
    name: String,
    /// `module`; a path import is a `directory` or a `javascript`.
    kind: String,
    /// Where it found the module: none for one no import path holds.
    path: Option<String>,
}

/// What the app's QML imports as it is written (qmlimportscanner with no
/// import path): each module of Qt's it names must be one the bundle
/// carries.
pub(super) fn written(imports: &[Import]) -> Vec<String> {
    qt_modules(imports)
        .filter(|import| !QML_MODULES.contains(&import.name.as_str()))
        .map(|import| match left_out(&QML_LEFT_OUT, &import.name) {
            Some(why) => format!(
                "ui/ imports {}, which the bundle leaves out as {why}",
                import.name
            ),
            None => format!(
                "ui/ imports {}, which the bundle does not carry",
                import.name
            ),
        })
        .collect()
}

/// What the scanner reaches once it follows Qt's modules: each module is
/// carried or left out on purpose, and each one listed is reached.
pub(super) fn reached(imports: &[Import]) -> Vec<String> {
    let mut parted: Vec<String> = qt_modules(imports)
        .filter_map(|import| {
            let name = import.name.as_str();
            if import.path.is_none() {
                Some(format!(
                    "{name} is reached and no directory of Qt's QML holds it"
                ))
            } else if QML_MODULES.contains(&name) || left_out(&QML_LEFT_OUT, name).is_some() {
                None
            } else {
                Some(format!(
                    "{name} is reached and on neither list: carry it or leave it out, with why"
                ))
            }
        })
        .collect();
    let listed = QML_MODULES
        .iter()
        .copied()
        .chain(QML_LEFT_OUT.iter().map(|(name, _)| *name));
    for name in listed {
        if !imports.iter().any(|import| import.name == name) {
            parted.push(format!("{name} is listed and nothing reaches it"));
        }
    }
    parted
}

/// The modules among `imports` that are not the app's own.
fn qt_modules(imports: &[Import]) -> impl Iterator<Item = &Import> {
    imports
        .iter()
        .filter(|import| import.kind == "module" && !APP_OWN.contains(&import.name.as_str()))
}

/// Why `name` stays out, where a list says it does.
fn left_out<'a>(list: &[(&str, &'a str)], name: &str) -> Option<&'a str> {
    list.iter()
        .find(|(left, _)| *left == name)
        .map(|(_, why)| *why)
}

/// What a built bundle carries of Qt, read off its directories.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Carried {
    /// The entries of `Contents/Frameworks`.
    frameworks: Vec<String>,
    /// The files under `Contents/PlugIns`, as `<directory>/<file>`, but
    /// for the modules' plugins in `quick`.
    plugins: Vec<String>,
    /// The files of `Contents/PlugIns/quick`.
    quick: Vec<String>,
    /// Each directory under `Contents/Resources/qml` that holds a qmldir,
    /// as its module's name, with the plugin the qmldir names.
    modules: Vec<(String, Option<String>)>,
}

/// Reads what `app` carries of Qt off its directories.
pub(super) fn carried(app: &Path) -> Result<Carried, String> {
    let contents = app.join("Contents");
    let plugins_dir = contents.join("PlugIns");
    let mut plugins = Vec::new();
    for kind in names(&plugins_dir)? {
        if kind == "quick" {
            continue;
        }
        let dir = plugins_dir.join(&kind);
        if dir.is_dir() {
            plugins.extend(
                names(&dir)?
                    .into_iter()
                    .map(|file| format!("{kind}/{file}")),
            );
        } else {
            plugins.push(kind);
        }
    }
    let mut modules = Vec::new();
    modules_under(&contents.join("Resources/qml"), "", &mut modules)?;
    Ok(Carried {
        frameworks: names(&contents.join("Frameworks"))?,
        plugins,
        quick: names(&plugins_dir.join("quick"))?,
        modules,
    })
}

/// The module `dir` is, named `name`, where it holds a qmldir, then each
/// one under it; a link is not followed.
fn modules_under(
    dir: &Path,
    name: &str,
    found: &mut Vec<(String, Option<String>)>,
) -> Result<(), String> {
    let qmldir = dir.join("qmldir");
    if qmldir.is_file() {
        let text = std::fs::read_to_string(&qmldir)
            .map_err(|e| format!("could not read {}: {e}", qmldir.display()))?;
        found.push((name.to_string(), plugin_of(&text).map(str::to_string)));
    }
    for entry in names(dir)? {
        let below = dir.join(&entry);
        if below.is_dir() && !below.is_symlink() {
            let name = if name.is_empty() {
                entry
            } else {
                format!("{name}.{entry}")
            };
            modules_under(&below, &name, found)?;
        }
    }
    Ok(())
}

/// The entries of `dir`, sorted; none where there is no `dir`.
fn names(dir: &Path) -> Result<Vec<String>, String> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut names = std::fs::read_dir(dir)
        .map_err(|e| format!("could not read {}: {e}", dir.display()))?
        .map(|entry| {
            entry
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .map_err(|e| format!("could not read {}: {e}", dir.display()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    names.sort();
    Ok(names)
}

/// Where a bundle and the lists part: what it carries that no list names,
/// and what a list names that it lacks.
pub(super) fn parted(carried: &Carried) -> Vec<String> {
    let mut parted = Vec::new();
    let mut frameworks = Vec::new();
    for entry in &carried.frameworks {
        match entry.strip_suffix(".framework") {
            Some(name) => frameworks.push(name.to_string()),
            None => parted.push(format!("Frameworks holds {entry}, which is no framework")),
        }
    }
    parted.extend(compare(
        (
            "a framework no list names",
            "a framework listed and not carried",
        ),
        &frameworks,
        &FRAMEWORKS,
        |_| None,
    ));
    parted.extend(compare(
        ("a plugin no list names", "a plugin listed and not carried"),
        &carried.plugins,
        &QT_PLUGINS,
        |plugin| {
            left_out(
                &QT_PLUGINS_LEFT_OUT,
                plugin.split('/').next().unwrap_or(plugin),
            )
        },
    ));
    let modules: Vec<String> = carried
        .modules
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    parted.extend(compare(
        (
            "a QML module no list names",
            "a QML module listed and not carried",
        ),
        &modules,
        &QML_MODULES,
        |module| left_out(&QML_LEFT_OUT, module).or_else(|| left_out(&QML_UNREACHED, module)),
    ));
    // A module's plugin goes in with its module, so what PlugIns/quick
    // holds is held to the qmldirs carried, not to a list.
    let named: Vec<String> = carried
        .modules
        .iter()
        .filter_map(|(_, plugin)| plugin.as_deref().map(plugin_file))
        .collect();
    let named: Vec<&str> = named.iter().map(String::as_str).collect();
    parted.extend(compare(
        (
            "a QML plugin no carried qmldir names",
            "a QML plugin a carried qmldir names and PlugIns/quick lacks",
        ),
        &carried.quick,
        &named,
        |_| None,
    ));
    parted
}

/// One line, under `said.0`, for each of `carried` that `listed` does not
/// name — with why it stays out where `why_out` says — and one, under
/// `said.1`, for each of `listed` not carried.
fn compare<'a>(
    said: (&str, &str),
    carried: &[String],
    listed: &[&str],
    why_out: impl Fn(&str) -> Option<&'a str>,
) -> Vec<String> {
    let (extra, missing) = said;
    let extras = carried
        .iter()
        .filter(|name| !listed.contains(&name.as_str()))
        .map(|name| match why_out(name) {
            Some(why) => format!("{extra}: {name} — left out as {why}"),
            None => format!("{extra}: {name}"),
        });
    let lacking = listed
        .iter()
        .filter(|name| !carried.iter().any(|carried| carried == *name))
        .map(|name| format!("{missing}: {name}"));
    extras.chain(lacking).collect()
}

/// qmlimportscanner's answer: a JSON array of one object per import. The
/// three strings a check reads are kept; every other value — a flag, the
/// array of a module's files — is stepped over.
pub(super) fn scanned(json: &str) -> Result<Vec<Import>, String> {
    let mut read = Reader { text: json, at: 0 };
    let mut found = Vec::new();
    read.eat('[')?;
    if !read.closes(']') {
        loop {
            found.push(read.import()?);
            if read.closes(']') {
                break;
            }
            read.eat(',')?;
        }
    }
    read.space();
    if read.at < read.text.len() {
        return Err(format!(
            "qmlimportscanner's answer goes on past its array, at byte {}",
            read.at
        ));
    }
    Ok(found)
}

/// The small JSON reading [`scanned`] needs (xtask is std alone).
struct Reader<'a> {
    text: &'a str,
    at: usize,
}

impl Reader<'_> {
    fn peek(&self) -> Option<char> {
        self.text[self.at..].chars().next()
    }

    fn space(&mut self) {
        while let Some(c) = self.peek().filter(|c| c.is_whitespace()) {
            self.at += c.len_utf8();
        }
    }

    fn eat(&mut self, want: char) -> Result<(), String> {
        if self.closes(want) {
            return Ok(());
        }
        Err(format!(
            "qmlimportscanner's answer has {:?} at byte {} where {want:?} goes",
            self.peek(),
            self.at
        ))
    }

    /// Whether `close` comes next, which is then read.
    fn closes(&mut self, close: char) -> bool {
        self.space();
        let closes = self.peek() == Some(close);
        if closes {
            self.at += close.len_utf8();
        }
        closes
    }

    fn import(&mut self) -> Result<Import, String> {
        let at = self.at;
        let (mut name, mut kind, mut path) = (None, None, None);
        self.eat('{')?;
        if !self.closes('}') {
            loop {
                let key = self.string()?;
                self.eat(':')?;
                let value = self.value()?;
                match key.as_str() {
                    "name" => name = value,
                    "type" => kind = value,
                    "path" => path = value,
                    _ => {}
                }
                if self.closes('}') {
                    break;
                }
                self.eat(',')?;
            }
        }
        match (name, kind) {
            (Some(name), Some(kind)) => Ok(Import { name, kind, path }),
            // A script a script imports (`.import "x.js"` in a `.js`) has
            // its path and no name.
            (None, Some(kind)) if kind != "module" => Ok(Import {
                name: String::new(),
                kind,
                path,
            }),
            _ => Err(format!(
                "qmlimportscanner names an import without its name or type, at byte {at}"
            )),
        }
    }

    /// A value: the string, if it is one.
    fn value(&mut self) -> Result<Option<String>, String> {
        self.space();
        match self.peek() {
            Some('"') => self.string().map(Some),
            Some(open @ ('[' | '{')) => {
                let close = if open == '[' { ']' } else { '}' };
                self.at += 1;
                if !self.closes(close) {
                    loop {
                        if open == '{' {
                            self.string()?;
                            self.eat(':')?;
                        }
                        self.value()?;
                        if self.closes(close) {
                            break;
                        }
                        self.eat(',')?;
                    }
                }
                Ok(None)
            }
            Some(_) => {
                let start = self.at;
                while let Some(c) = self.peek().filter(|c| !matches!(c, ',' | ']' | '}')) {
                    if c.is_whitespace() {
                        break;
                    }
                    self.at += c.len_utf8();
                }
                if self.at == start {
                    return Err(format!(
                        "qmlimportscanner's answer has no value at byte {start}"
                    ));
                }
                Ok(None)
            }
            None => Err("qmlimportscanner's answer ends inside an import".to_string()),
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.eat('"')?;
        let mut out = String::new();
        let mut chars = self.text[self.at..].char_indices();
        while let Some((offset, c)) = chars.next() {
            match c {
                '"' => {
                    self.at += offset + 1;
                    return Ok(out);
                }
                '\\' => match chars.next() {
                    Some((_, 'n')) => out.push('\n'),
                    Some((_, 't')) => out.push('\t'),
                    Some((_, 'r')) => out.push('\r'),
                    Some((_, other)) => out.push(other),
                    None => break,
                },
                other => out.push(other),
            }
        }
        Err("qmlimportscanner's answer ends inside a string".to_string())
    }
}

#[cfg(test)]
mod tests;
