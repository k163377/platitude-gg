//! What QML loads off the tree by relative import: `import "dir"`,
//! `import "file.js" as X`, and a JS file's `.import "file.js" as X`, each
//! resolved against the importing file's own directory as the engine
//! resolves it. A module import (`import QtQuick`, `import platitude.ui`)
//! resolves on the import path instead — what a runner stages there is its
//! own to name (`crate::qmltest`).

use std::collections::BTreeSet;
use std::path::{Component, Path};

use super::under;

/// What the QML and JS files under `from` (workspace paths) load off the
/// tree at `dir` by relative import, and what that loads in turn: a
/// directory as itself — an import of one reads its qmldir and every type
/// in it —, a file as itself. Only what the tree holds, and nothing under
/// `from`, which is read already.
pub(super) fn loaded_from(dir: &Path, from: &[String]) -> BTreeSet<String> {
    let mut loaded = BTreeSet::new();
    let mut queue: Vec<String> = from.iter().flat_map(|f| sources(dir, f)).collect();
    let mut read = BTreeSet::new();
    while let Some(file) = queue.pop() {
        if !read.insert(file.clone()) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(dir.join(&file)) else {
            continue;
        };
        let base = Path::new(&file).parent().unwrap_or(Path::new(""));
        for named in relative_imports(&text) {
            let Some(target) = folded(&base.join(named)) else {
                continue;
            };
            if loaded.contains(&target)
                || from.iter().any(|f| under(&target, f))
                || !dir.join(&target).exists()
            {
                continue;
            }
            queue.extend(sources(dir, &target));
            loaded.insert(target);
        }
    }
    loaded
}

/// The QML and JS files at `path`: the file itself, or every one under the
/// directory (deeper than an import reads, which costs an input too many
/// at most).
fn sources(dir: &Path, path: &str) -> Vec<String> {
    let full = dir.join(path);
    if !full.is_dir() {
        return vec![path.to_string()];
    }
    let mut found = Vec::new();
    for extension in ["qml", "js"] {
        // A directory that cannot be listed loads nothing this can name.
        let _ = crate::gate::graph::collect(dir, &full, extension, &mut found);
    }
    found
}

/// The paths a QML or JS text imports by string, as written: a line's
/// `import "…"` or `.import "…"`, a path a URL scheme or a leading `/`
/// marks as no file of the tree left out.
fn relative_imports(text: &str) -> Vec<&str> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim_start();
            let rest = line
                .strip_prefix(".import")
                .or_else(|| line.strip_prefix("import"))?;
            let rest = rest.trim_start().strip_prefix('"')?;
            let named = &rest[..rest.find('"')?];
            (!(named.is_empty() || named.contains(':') || named.starts_with('/'))).then_some(named)
        })
        .collect()
}

/// `path` as a workspace path, its `.` and `..` folded away; `None` for
/// one that climbs out of the workspace.
fn folded(path: &Path) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop()?;
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::{folded, loaded_from, relative_imports};
    use std::path::Path;

    #[test]
    fn a_relative_import_is_read_as_written_and_a_module_import_is_not() {
        let text = "import QtQuick\nimport platitude.ui\nimport \"../../src/auto\" as Automation\n\
                    import \"painted.js\" as Painted\n  .import \"inner.js\" as Inner\n\
                    // import \"commented.js\" as Gone\nimport \"qrc:/x.qml\"\nimport \"/abs\"\n";
        assert_eq!(
            relative_imports(text),
            ["../../src/auto", "painted.js", "inner.js"]
        );
        assert_eq!(
            folded(&Path::new("crates/a/tests/qml").join("../../src/auto")).as_deref(),
            Some("crates/a/src/auto")
        );
        assert_eq!(folded(Path::new("a/../../b")), None);
    }

    /// The tests' imports, followed as far as they lead: a directory, a
    /// helper beside them, and what the directory imports in turn — none
    /// of it under the tests' own directory, which is read already, and
    /// nothing the tree does not hold.
    #[test]
    fn what_the_tests_load_is_followed_through_what_it_loads() {
        let root = crate::yard::Yard::new("qml-imports");
        let write = |path: &str, text: &str| {
            let full = root.join(path);
            std::fs::create_dir_all(full.parent().expect("a parent")).expect("a directory");
            std::fs::write(full, text).expect("a file");
        };
        write(
            "app/tests/qml/tst_a.qml",
            "import \"../../src/auto\" as A\nimport \"painted.js\" as P\nimport \"../gone\"\n",
        );
        write(
            "app/tests/qml/painted.js",
            ".import \"../../src/shared/ink.js\" as Ink\n",
        );
        write("app/src/auto/Driver.qml", "import \"../lib\"\nItem {}\n");
        write("app/src/lib/Lib.qml", "Item {}\n");
        write("app/src/shared/ink.js", "function ink() {}\n");
        write("app/src/ui/Untouched.qml", "Item {}\n");
        let loaded = loaded_from(&root, &["app/tests/qml".to_string()]);
        let expected: Vec<&str> = vec!["app/src/auto", "app/src/lib", "app/src/shared/ink.js"];
        assert_eq!(
            loaded.iter().map(String::as_str).collect::<Vec<_>>(),
            expected
        );
    }
}
