//! Where the product's singletons stand and who imports them, counted by
//! machine (.claude/rules/app-ui.md「QML モジュール」).
//!
//! A qmldir's singleton is resolved for every document that sees the
//! qmldir, read or not (ci/baseline/qml-weight-windows-x64.md §import の
//! 代金), and every file of `ui/` sees the product's own qmldir. So that
//! qmldir declares only the tokens ([`TOKENS`]), and every other singleton
//! stands in a module of its own directory under `ui/` — `ui/<name>/` is
//! `platitude.ui.<name>`, singletons only, each file beside the qmldir
//! that declares it: a file a qmldir names from another directory imports
//! its own module from there and is instantiated recursively.
//!
//! A file reads one of those singletons only if it imports the module, and
//! imports the module only if it reads one: a name the file does not see
//! is no load failure but a ReferenceError each time the line is
//! evaluated, and an import nothing reads pays for every singleton of the
//! module in each load. A file of the module's own directory sees it
//! already and never imports it. The tokens are held to the first half
//! outside `ui/` itself: their module holds the parts too, and is imported
//! for those.
//!
//! A read is the singleton's name standing as a word of the code, with the
//! comments and the literals blanked (`crate::lex`): `Hand.away` and
//! `target: Hand` alike, and neither `onHandChanged` nor `card.Hand`.

use std::path::{Path, PathBuf};

use super::modules::{Exported, exported};

/// The product's QML, its qmldir the tokens' and each directory under it a
/// module of singletons.
const UI: &str = "crates/platitude-app/src/ui";
const PRODUCT: &str = "platitude.ui";
/// The singletons nearly every part reads, which the product's qmldir keeps.
const TOKENS: [&str; 2] = ["Theme", "Metrics"];
/// Where the app's QML stands: the product and the harness, and the QtTest
/// files with their stubs and rigs.
const READERS: [&str; 2] = ["crates/platitude-app/src", "crates/platitude-app/tests"];
const RULE: &str = ".claude/rules/app-ui.md「QML モジュール」";

/// A module whose singletons a file sees only by importing it.
#[derive(Debug)]
struct Module {
    uri: String,
    /// Its directory, as a workspace path: where its files see it unasked.
    dir: String,
    singletons: Vec<String>,
    /// Holds nothing but its singletons, so an import is owed to a read.
    singletons_only: bool,
}

/// One failure per misplaced singleton and per file that reads one it
/// does not import or imports one it does not read, and how many QML
/// files were read.
pub(super) fn check(root: &Path) -> Result<(Vec<String>, usize), String> {
    let mut failures = Vec::new();
    let modules = modules(root, &mut failures)?;
    let mut files = Vec::new();
    for dir in READERS {
        super::collect(&root.join(dir), &mut files)?;
    }
    files.retain(|path| path.extension().is_some_and(|e| e == "qml"));
    files.sort();
    for path in &files {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        failures.extend(findings(&super::relative(root, path), &text, &modules));
    }
    Ok((failures, files.len()))
}

/// What one QML file at `shown` (a workspace path) does against the rule,
/// read against the modules `root` declares — the write hook's, while the
/// edit is still in hand: its own place in a module, and its imports. A
/// tree whose modules cannot be read says nothing here: `check` names it.
pub(crate) fn notes(root: &Path, shown: &str, text: &str) -> Vec<String> {
    let mut out_of_shape = Vec::new();
    let Ok(modules) = modules(root, &mut out_of_shape) else {
        return Vec::new();
    };
    out_of_shape
        .into_iter()
        .filter(|failure| failure.starts_with(&format!("{shown} ")))
        .chain(findings(shown, text, &modules))
        .collect()
}

/// The product's module with its tokens, and the module of each directory
/// under `ui/`, with a failure for each line out of the shape the module
/// doc gives.
fn modules(root: &Path, failures: &mut Vec<String>) -> Result<Vec<Module>, String> {
    let ui = root.join(UI);
    let text = std::fs::read_to_string(ui.join("qmldir"))
        .map_err(|e| format!("could not read {UI}/qmldir: {e}"))?;
    let mut tokens = Vec::new();
    for (number, line) in text.lines().enumerate() {
        match exported(line) {
            Some(Exported {
                name,
                singleton: true,
            }) if TOKENS.contains(&name.as_str()) => {
                tokens.push(name);
            }
            Some(Exported {
                name,
                singleton: true,
            }) => failures.push(format!(
                "{UI}/qmldir:{}: declares the singleton {name}, which every file of ui/ resolves \
                 in each load whether it reads it or not — give it a module of its own under ui/ \
                 ({RULE})",
                number + 1
            )),
            _ => {}
        }
    }
    let mut modules = vec![Module {
        uri: PRODUCT.to_string(),
        dir: UI.to_string(),
        singletons: tokens,
        singletons_only: false,
    }];
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&ui)
        .map_err(|e| format!("could not read {UI}: {e}"))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_dir())
        .collect();
    dirs.sort();
    for dir in dirs {
        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        modules.push(singleton_module(&dir, &name, failures)?);
    }
    Ok(modules)
}

/// The module of `ui/<name>/`, off its qmldir: one `module` line naming
/// it by its directory, and a singleton beside it for every type line and
/// every QML file.
fn singleton_module(dir: &Path, name: &str, failures: &mut Vec<String>) -> Result<Module, String> {
    let shown = format!("{UI}/{name}");
    let uri = format!("{PRODUCT}.{name}");
    let mut module = Module {
        uri: uri.clone(),
        dir: shown.clone(),
        singletons: Vec::new(),
        singletons_only: true,
    };
    let Ok(text) = std::fs::read_to_string(dir.join("qmldir")) else {
        failures.push(format!(
            "{shown} holds no qmldir — a directory under ui/ is a module of singletons ({RULE})"
        ));
        return Ok(module);
    };
    let mut declared = Vec::new();
    let mut named_as = None;
    for (number, line) in text.lines().enumerate() {
        let words: Vec<&str> = line
            .split_whitespace()
            .take_while(|word| !word.starts_with('#'))
            .collect();
        if let ["module", written] = words.as_slice() {
            named_as = Some((*written).to_string());
            continue;
        }
        let at = format!("{shown}/qmldir:{}", number + 1);
        match exported(line) {
            Some(Exported {
                name,
                singleton: false,
            }) => failures.push(format!(
                "{at}: declares {name}, which is no singleton — a part belongs to ui/'s own \
                 qmldir ({RULE})"
            )),
            Some(Exported {
                name,
                singleton: true,
            }) => {
                let file = words.last().copied().unwrap_or_default();
                if file.contains(['/', '\\']) || !dir.join(file).is_file() {
                    failures.push(format!(
                        "{at}: {name}'s file {file} does not stand beside the qmldir — a \
                         singleton named from another directory imports its own module from \
                         there and is instantiated recursively ({RULE})"
                    ));
                }
                declared.push(file.to_string());
                module.singletons.push(name);
            }
            None => {}
        }
    }
    if named_as.as_deref() != Some(uri.as_str()) {
        failures.push(format!(
            "{shown}/qmldir names its module {}, where its directory makes it {uri} ({RULE})",
            named_as.as_deref().unwrap_or("nowhere")
        ));
    }
    for entry in std::fs::read_dir(dir).map_err(|e| format!("could not read {shown}: {e}"))? {
        let file = entry
            .map_err(|e| format!("could not read {shown}: {e}"))?
            .file_name();
        let file = file.to_string_lossy();
        if file.ends_with(".qml") && !declared.iter().any(|d| *d == file) {
            failures.push(format!(
                "{shown}/{file} is not declared by the module's qmldir — a singleton is \
                 declared there (`singleton <Type> 1.0 <file>`), a part belongs to ui/ itself \
                 ({RULE})"
            ));
        }
    }
    Ok(module)
}

/// Everything one QML file at `shown` (a workspace path) does against the
/// rule, by module.
fn findings(shown: &str, text: &str, modules: &[Module]) -> Vec<String> {
    let (code, unread) = crate::lex::blanked(text, crate::lex::Lang::Qml, false);
    let mut out = Vec::new();
    if let Some(unread) = unread {
        let line = text.chars().take(unread.at).filter(|c| *c == '\n').count() + 1;
        out.push(format!(
            "{shown}:{line}: {} — what stands past it is not read for a singleton",
            unread.what
        ));
    }
    let imported = imports(&code);
    let home = shown.rsplit_once('/').map(|(dir, _)| dir);
    for module in modules {
        let imports = imported.contains(&module.uri.as_str());
        if home == Some(module.dir.as_str()) {
            if imports && module.singletons_only {
                out.push(format!(
                    "{shown}: imports {}, its own directory's module — the file sees every \
                     singleton of it already ({RULE})",
                    module.uri
                ));
            }
            continue;
        }
        let read = module
            .singletons
            .iter()
            .find_map(|name| line_reading(&code, name).map(|line| (name, line)));
        match read {
            Some((name, line)) if !imports => out.push(format!(
                "{shown}:{line}: reads {name} without importing {} — the file does not see it, \
                 and the line fails each time it is evaluated ({RULE})",
                module.uri
            )),
            None if imports && module.singletons_only => out.push(format!(
                "{shown}: imports {} and reads none of its singletons — every one of them is \
                 resolved in each load for nothing ({RULE})",
                module.uri
            )),
            _ => {}
        }
    }
    out
}

/// The modules a file's code imports: the word after each `import`.
fn imports(code: &str) -> Vec<&str> {
    code.lines()
        .filter_map(|line| line.trim_start().strip_prefix("import "))
        .filter_map(|rest| rest.split_whitespace().next())
        .collect()
}

/// The line of the first word of `code` that is `name`: neither end runs on
/// into a longer name, and no `.` puts it on something else.
fn line_reading(code: &str, name: &str) -> Option<usize> {
    let word = |c: char| c.is_alphanumeric() || c == '_' || c == '$';
    let mut at = 0;
    while let Some(found) = code[at..].find(name) {
        let start = at + found;
        let end = start + name.len();
        let before = code[..start].chars().next_back();
        let after = code[end..].chars().next();
        if before.is_none_or(|c| !word(c) && c != '.') && after.is_none_or(|c| !word(c)) {
            return Some(code[..start].matches('\n').count() + 1);
        }
        at = end;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_read_is_the_name_as_a_word_of_its_own() {
        assert_eq!(line_reading("a\n    Hand.away = true\n", "Hand"), Some(2));
        assert_eq!(line_reading("    target: Hand\n", "Hand"), Some(1));
        for code in [
            "    onHandChanged: f()",
            "    MiddleHand.release()",
            "    card.Hand = 1",
            "    Handy.go()",
        ] {
            assert_eq!(line_reading(code, "Hand"), None, "{code}");
        }
    }

    #[test]
    fn a_name_in_a_comment_or_a_string_is_no_read_and_a_template_s_code_is() {
        let text = "import QtQuick\n// Hand.away\nItem {\n    \
                    property string a: \"Hand.away\"\n    \
                    property string b: `${Words.lead}`\n}\n";
        let (code, unread) = crate::lex::blanked(text, crate::lex::Lang::Qml, false);
        assert!(unread.is_none());
        assert_eq!(line_reading(&code, "Hand"), None);
        assert_eq!(line_reading(&code, "Words"), Some(5));
        assert_eq!(imports(&code), ["QtQuick"]);
    }

    /// Every shape the module doc forbids, one file or line each.
    #[test]
    fn a_singleton_out_of_place_and_an_import_out_of_step_are_caught() {
        let root = crate::yard::Yard::new("structure-singletons");
        let write = |path: &str, text: &str| {
            let file = root.join(path);
            std::fs::create_dir_all(file.parent().expect("a parent")).expect("the fixture's dirs");
            std::fs::write(file, text).expect("the fixture");
        };
        write(
            &format!("{UI}/qmldir"),
            "module platitude.ui\nsingleton Theme 1.0 Theme.qml\n\
             singleton Stray 1.0 Stray.qml\nPart 1.0 Part.qml\n",
        );
        write(
            &format!("{UI}/words/qmldir"),
            "module platitude.ui.words\nsingleton Words 1.0 Words.qml\n",
        );
        write(
            &format!("{UI}/words/Words.qml"),
            "pragma Singleton\nimport QtQuick\nimport platitude.ui.words\nQtObject {}\n",
        );
        write(
            &format!("{UI}/far/qmldir"),
            "module platitude.far\nsingleton Far 1.0 ../Far.qml\nPart 1.0 Part.qml\n",
        );
        write(&format!("{UI}/far/Loose.qml"), "Item {}\n");
        write(&format!("{UI}/bare/Bare.qml"), "Item {}\n");
        // Reads Words unseen; imports it for nothing; reads it as it should.
        write(
            &format!("{UI}/Unseen.qml"),
            "import platitude.ui\nItem {\n    width: Words.lead\n}\n",
        );
        write(
            &format!("{UI}/Idle.qml"),
            "import platitude.ui.words\nItem {\n    // Words.lead\n}\n",
        );
        write(
            &format!("{UI}/Kept.qml"),
            "import platitude.ui.words\nItem {\n    width: Words.lead + Theme.gap\n}\n",
        );
        // The tokens are seen unasked in ui/ alone.
        write(
            "crates/platitude-app/src/auto/Driver.qml",
            "import QtQuick\nItem {\n    color: Theme.ground\n}\n",
        );
        write(
            "crates/platitude-app/tests/qml/tst_kept.qml",
            "import platitude.ui\nimport platitude.ui.words\nItem {\n    \
             width: Words.lead + Theme.gap\n}\n",
        );
        let (failures, files) = check(&root).expect("scan the fixture");
        assert_eq!(files, 8);
        for wanted in [
            "ui/qmldir:3: declares the singleton Stray",
            "ui/words/Words.qml: imports platitude.ui.words, its own directory's module",
            "ui/far/qmldir:2: Far's file ../Far.qml does not stand beside the qmldir",
            "ui/far/qmldir:3: declares Part, which is no singleton",
            "ui/far/qmldir names its module platitude.far, where its directory makes it \
             platitude.ui.far",
            "ui/far/Loose.qml is not declared by the module's qmldir",
            "ui/bare holds no qmldir",
            "ui/Unseen.qml:3: reads Words without importing platitude.ui.words",
            "ui/Idle.qml: imports platitude.ui.words and reads none of its singletons",
            "auto/Driver.qml:3: reads Theme without importing platitude.ui",
        ] {
            assert!(
                failures.iter().any(|failure| failure.contains(wanted)),
                "{wanted}: {failures:#?}"
            );
        }
        assert_eq!(failures.len(), 10, "{failures:#?}");
    }

    /// The rule the count is for: the tree it runs on passes it.
    #[test]
    fn every_reader_of_a_singleton_imports_its_module_and_no_other() {
        let (failures, files) = check(&crate::tree::workspace_root()).expect("scan the app's QML");
        assert!(failures.is_empty(), "{failures:#?}");
        assert!(files > 1, "the app has QML to read");
    }
}
