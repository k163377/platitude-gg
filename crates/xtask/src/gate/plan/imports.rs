//! What QML loads off the tree by a relative path, and what that loads in
//! turn: the URL paths each text's loads name (`super::loads`), the file
//! each names on disk as Qt works it out (`super::urls`: its resolution,
//! the file), a QML file's own directory, and the files a directory's
//! `qmldir` names. A module import (`import QtQuick`, `import platitude.ui`)
//! resolves on the import path instead — what a runner stages there is its
//! own to name (`crate::qmltest`).
//!
//! What cannot be followed is not dropped: a path the tree does not hold is
//! kept as it is named, so that its appearing moves a key; a load whose
//! value or file nothing here works out, and a text the lexer could not
//! read to its end, leave what is loaded unnamed (`Loaded::unbounded`), and
//! no key answers for a run that reads them.

use std::collections::BTreeSet;
use std::path::Path;

use super::loads::{Named, loads};
use super::under;
use super::urls::{file_of, url_path};

/// What the QML and JS under a directory load off the tree.
#[derive(Debug, Default)]
pub(super) struct Loaded {
    /// The files and directories they load, as workspace paths.
    pub paths: BTreeSet<String>,
    /// Why what they load cannot all be named, where it cannot: the first
    /// load nothing here follows, and the file it stands in.
    pub unbounded: Option<String>,
}

/// What the QML and JS files under `from` (workspace paths) load off the
/// tree at `dir` by relative path, and what that loads in turn: a
/// directory as itself — an import of one reads its qmldir and every type
/// in it —, a file as itself. Nothing under `from`, which is read already.
pub(super) fn loaded_from(dir: &Path, from: &[String]) -> Loaded {
    let mut loaded = Loaded::default();
    let mut queue = Vec::new();
    for path in from {
        enqueue(dir, path, &mut queue, &mut loaded);
    }
    let mut read = BTreeSet::new();
    while let Some(file) = queue.pop() {
        if !read.insert(file.clone()) {
            continue;
        }
        let base = Path::new(&file).parent().unwrap_or(Path::new(""));
        // A QML file implicitly imports its own directory. Key the
        // directory too: a new sibling type or qmldir can change resolution.
        if file.ends_with(".qml") {
            let directory = base.to_string_lossy().replace('\\', "/");
            if !directory.is_empty()
                && !from.iter().any(|f| under(&directory, f))
                && loaded.paths.insert(directory.clone())
            {
                enqueue(dir, &directory, &mut queue, &mut loaded);
            }
        }
        let named = match std::fs::read_to_string(dir.join(&file)) {
            Ok(text) if file.ends_with("qmldir") => qmldir_files(&text),
            Ok(text) => loads(&text),
            Err(_) => Err("a file that is no text".to_string()),
        };
        let named = match named {
            Ok(named) => named,
            Err(why) => {
                unbounded(&mut loaded, &file, why);
                continue;
            }
        };
        for Named { path, .. } in named {
            let target = match file_of(dir, base, &path) {
                Ok(target) => target,
                Err(why) => {
                    unbounded(&mut loaded, &file, why);
                    continue;
                }
            };
            if loaded.paths.contains(&target) || from.iter().any(|f| under(&target, f)) {
                continue;
            }
            // Kept as named where the tree does not hold it: its appearing moves the key.
            enqueue(dir, &target, &mut queue, &mut loaded);
            loaded.paths.insert(target);
        }
    }
    loaded
}

/// Notes the first reason what is loaded cannot all be named.
fn unbounded(loaded: &mut Loaded, file: &str, why: String) {
    if loaded.unbounded.is_none() {
        loaded.unbounded = Some(format!("{file}: {why}"));
    }
}

/// Queues the files a load of `path` reads on ([`sources`]), or notes why
/// they cannot be listed.
fn enqueue(dir: &Path, path: &str, queue: &mut Vec<String>, loaded: &mut Loaded) {
    match sources(dir, path) {
        Ok(files) => queue.extend(files),
        Err(why) => unbounded(loaded, path, why),
    }
}

/// The files at `path` a load reads on: the file itself where it is QML,
/// JS or a `qmldir`, or every such file under the directory, nested
/// modules' `qmldir`s included (deeper than an import reads, which costs an
/// input too many at most). None where the tree holds nothing at `path`: a
/// path is keyed as absent by whoever names it. `Err` for a symbolic link
/// or a tree that cannot be read: an unreadable tree is not an empty tree.
fn sources(dir: &Path, path: &str) -> Result<Vec<String>, String> {
    let full = dir.join(path);
    let metadata = match std::fs::symlink_metadata(&full) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("cannot inspect {}: {error}", full.display())),
    };
    if metadata.file_type().is_symlink() {
        return Err(format!("a symbolic link is not a source closure: {path}"));
    }
    if !metadata.is_dir() {
        return if path.ends_with("qmldir")
            || [".qml", ".js", ".mjs"]
                .iter()
                .any(|end| path.ends_with(end))
        {
            Ok(vec![path.to_string()])
        } else {
            Ok(Vec::new())
        };
    }
    let mut found = Vec::new();
    let entries = std::fs::read_dir(&full)
        .map_err(|error| format!("cannot list {}: {error}", full.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("cannot list {}: {error}", full.display()))?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| format!("a source name is not UTF-8 under {}", full.display()))?;
        found.extend(sources(
            dir,
            &format!("{}/{name}", path.trim_end_matches('/')),
        )?);
    }
    Ok(found)
}

/// The files a `qmldir` names, as URL paths relative to it: the last word
/// of each type, singleton, internal or script line that ends in a QML or
/// JS file; `Err` for one off the tree.
fn qmldir_files(text: &str) -> Result<Vec<Named>, String> {
    let mut out = Vec::new();
    let words = text
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .filter_map(|line| line.split_whitespace().last())
        .filter(|word| {
            [".qml", ".js", ".mjs"]
                .iter()
                .any(|end| word.ends_with(end))
        });
    for word in words {
        if let Some(path) = url_path(word)? {
            out.push(Named { path, load: true });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{loaded_from, loads, qmldir_files};
    use std::path::Path;

    #[test]
    fn nested_module_definitions_and_failed_walks_cannot_drop_inputs() {
        let root = crate::yard::Yard::new("qml-nested-modules");
        std::fs::create_dir_all(root.join("tests/sub")).unwrap();
        std::fs::create_dir_all(root.join("shared")).unwrap();
        std::fs::write(
            root.join("tests/sub/qmldir"),
            "Card 1.0 ../../shared/Card.qml\n",
        )
        .unwrap();
        std::fs::write(root.join("shared/Card.qml"), "import QtQuick\nItem {}\n").unwrap();
        let loaded = loaded_from(&root, &["tests".into()]);
        assert!(loaded.unbounded.is_none(), "{loaded:?}");
        assert!(loaded.paths.contains("shared/Card.qml"), "{loaded:?}");

        // A platform-independent metadata error: an invalid path must not be mistaken for an absent file.
        let failed = loaded_from(&root, &["tests/\0".into()]);
        assert!(
            failed.unbounded.is_some(),
            "a failed walk cannot keep a cache key: {failed:?}"
        );
    }

    #[test]
    fn implicit_siblings_and_their_module_are_inputs() {
        let root = crate::yard::Yard::new("qml-neighbours");
        for (path, text) in [
            (
                "tests/tst_a.qml",
                "import QtQuick\nLoader { source: '../cards/Card.qml' }",
            ),
            ("cards/Card.qml", "import QtQuick\nNeighbour {}"),
            ("cards/Neighbour.qml", "import QtQuick\nItem {}"),
            ("cards/qmldir", "Neighbour 1.0 Neighbour.qml"),
        ] {
            let file = root.join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, text).unwrap();
        }
        let got = loaded_from(&root, &["tests".into()]);
        assert!(got.unbounded.is_none(), "{got:?}");
        assert!(
            got.paths
                .iter()
                .any(|p| super::under("cards/Neighbour.qml", p)),
            "{got:?}"
        );
        assert!(
            got.paths.iter().any(|p| super::under("cards/qmldir", p)),
            "{got:?}"
        );
    }

    #[test]
    fn a_qmldir_names_the_files_of_its_types() {
        let text = "module platitude.ui\n# a comment.qml\nsingleton Theme 1.0 Theme.qml\n\
                    internal Seat ../seat/Seat.qml\nCard 1.0 Card%20View.qml\nscript 1.0 tools.js\nplugin x\n";
        let paths: Vec<String> = qmldir_files(text)
            .expect("on the tree")
            .into_iter()
            .map(|n| n.path)
            .collect();
        assert_eq!(
            paths,
            [
                "Theme.qml",
                "../seat/Seat.qml",
                "Card%20View.qml",
                "tools.js"
            ]
        );
    }

    /// The tests' loads, followed as far as they lead: a directory, a
    /// helper beside them, what the directory imports and its qmldir names
    /// in turn — none of it under the tests' own directory, which is read
    /// already —, the file a URL names on disk, a helper the tree does not
    /// hold kept as named; a load nothing follows leaves what is loaded
    /// unbounded, and a directory the tree does not hold loads nothing.
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
            "import \"../../src/auto\" as A\nimport 'painted.js' as P\nimport \"../gone\"\n\
             Loader { source: '../../src/' + 'cards/Card%20View.qml' }\n",
        );
        write(
            "app/tests/qml/painted.js",
            ".import '../../src/shared/ink.js' as Ink\n",
        );
        write("app/src/auto/Driver.qml", "import \"../lib\"\nItem {}\n");
        write("app/src/auto/qmldir", "Seat 1.0 ../seat/Seat.qml\n");
        write("app/src/seat/Seat.qml", "Item {}\n");
        write("app/src/lib/Lib.qml", "Item {}\n");
        write("app/src/cards/Card View.qml", "Item {}\n");
        write("app/src/shared/ink.js", "function ink() {}\n");
        write("app/src/ui/Untouched.qml", "Item {}\n");
        let loaded = loaded_from(&root, &["app/tests/qml".to_string()]);
        let expected: Vec<&str> = vec![
            "app/src/auto",
            "app/src/cards",
            "app/src/cards/Card View.qml",
            "app/src/lib",
            "app/src/seat",
            "app/src/seat/Seat.qml",
            "app/src/shared/ink.js",
            "app/tests/gone",
        ];
        assert_eq!(
            loaded.paths.iter().map(String::as_str).collect::<Vec<_>>(),
            expected
        );
        assert_eq!(loaded.unbounded, None);
        for (file, text) in [
            (
                "app/tests/out/tst_b.qml",
                "import '../../../../outside.js' as Out\n",
            ),
            (
                "app/tests/named/tst_c.qml",
                "Loader { source: '../' + card }\n",
            ),
            (
                "app/tests/escaped/tst_d.qml",
                "Loader { source: '../shared%2FCard.qml' }\n",
            ),
        ] {
            write(file, text);
            let dir = Path::new(file).parent().expect("dir").display().to_string();
            let loaded = loaded_from(&root, &[dir.replace('\\', "/")]);
            assert!(
                loaded
                    .unbounded
                    .as_deref()
                    .is_some_and(|why| why.starts_with(file)),
                "{file}: {loaded:?}"
            );
        }
        let loaded = loaded_from(&root, &["app/tests/none".to_string()]);
        assert!(
            loaded.paths.is_empty() && loaded.unbounded.is_none(),
            "{loaded:?}"
        );
        let odd = root.join("app/tests/odd/tst_odd.qml");
        std::fs::create_dir_all(odd.parent().expect("parent")).expect("dir");
        std::fs::write(&odd, [0xff, 0xfe, 0x00]).expect("odd");
        let loaded = loaded_from(&root, &["app/tests/odd".to_string()]);
        assert!(loaded.unbounded.is_some(), "{loaded:?}");
    }

    /// Every URL path `tst_loadspellings` spells, read off its own text,
    /// names one of the two files Qt reads there: the file each spelling's
    /// run compares against is the one the reader keys it on.
    #[test]
    fn each_spelling_of_the_spelling_test_names_the_file_qt_reads() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let file = "crates/platitude-app/tests/qml/tst_loadspellings.qml";
        let text = std::fs::read_to_string(root.join(file)).expect("the spelling test");
        let base = Path::new(file).parent().expect("its directory");
        let plain = "crates/platitude-app/tests/loads/Plain.qml";
        let words = "crates/platitude-app/tests/loads/Two Words.qml";
        let named: Vec<(String, String)> = loads(&text)
            .expect("read to its end")
            .into_iter()
            .filter(|n| n.load)
            .map(|n| {
                let target = super::file_of(&root, base, &n.path).expect("a file of the tree");
                (n.path, target)
            })
            .collect();
        assert_eq!(
            named,
            [
                ("../loads/Plain.qml", plain),
                ("../loads/./Plain.qml", plain),
                ("../loads/Two%20Words.qml", words),
                ("../loads/Two Words.qml", words),
            ]
            .map(|(path, target)| (path.to_string(), target.to_string()))
        );
    }

    /// The tree's own QML tests load what the reader follows: the harness's
    /// module and the files `tst_loadspellings` loads, every load named, so
    /// their stamps keep.
    #[test]
    fn the_trees_qml_tests_load_what_is_followed() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let loaded = loaded_from(&root, &["crates/platitude-app/tests/qml".to_string()]);
        for path in [
            "crates/platitude-app/src/auto",
            "crates/platitude-app/tests/loads/Plain.qml",
            "crates/platitude-app/tests/loads/Two Words.qml",
        ] {
            assert!(loaded.paths.contains(path), "{path}: {loaded:?}");
        }
        assert!(
            loaded
                .paths
                .iter()
                .all(|path| path.starts_with("crates/platitude-app/")),
            "{loaded:?}"
        );
        assert_eq!(loaded.unbounded, None);
    }
}
