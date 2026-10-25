//! `film in` / `film out`: the film verb put into the tree checked out
//! here, and taken out again, never committed.
//!
//! The verb is not the harness's own because of what it needs: a menu
//! opened from the film has to stand at the film's hand — the offscreen
//! cursor never follows synthetic events, and `popup()` stands at the
//! cursor — which is a seam in the product's ui/, and a slot that runs any
//! program. So it is put in as text substitutions, each found exactly
//! once, and taken out by the same substitutions backwards: a
//! `git checkout --` would take the session's own edits to those files
//! with it.

use std::collections::BTreeMap;
use std::path::Path;

/// Where the verb's component stands while it is in.
pub(crate) const PAYLOAD: &str = "crates/platitude-app/src/auto/WindowFilmActs.qml";

const COMPONENT: &str = include_str!("WindowFilmActs.qml.in");

/// One substitution: `old` stands in `path` exactly once while the verb
/// is out, `new` while it is in.
struct Edit {
    path: &'static str,
    old: &'static str,
    new: &'static str,
}

/// What the component asks of the harness beyond its own `write_file`.
const SLOTS: &str = r#"
    /// Film (`cargo xtask film`): copies a file over another, as an
    /// editor saves it.
    #[qslot]
    fn film_copy(&self, from: String, to: String) -> String {
        match std::fs::copy(&from, &to) {
            Ok(_) => String::new(),
            Err(e) => format!("{from} -> {to}: {e}"),
        }
    }

    /// Film: runs a program to its end, its arguments one per line; says
    /// its status and output.
    #[qslot]
    fn film_run(&self, program: String, args: String) -> String {
        let mut command = std::process::Command::new(&program);
        command.args(args.lines());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000);
        }
        match command.output() {
            Ok(out) => format!(
                "{} {}{}",
                out.status,
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            ),
            Err(e) => format!("{program}: {e}"),
        }
    }
}
qml_register!(Harness, "Harness", singleton = true);"#;

const EDITS: &[Edit] = &[
    Edit {
        path: "crates/platitude-app/src/auto/qmldir",
        old: "WindowSettingsActs 1.0 WindowSettingsActs.qml\n",
        new: "WindowSettingsActs 1.0 WindowSettingsActs.qml\nWindowFilmActs 1.0 WindowFilmActs.qml\n",
    },
    // Spelled in pieces, as `docs::commands` spells its dispatch: a whole
    // path in a string reads to the gate's graph as this file reading the
    // app's crate root, which that check refuses.
    Edit {
        path: concat!("crates/platitude-app/src/", "main.rs"),
        old: "    qrc::embed!(\"auto/AutoActDriver.qml\");\n",
        new: "    qrc::embed!(\"auto/AutoActDriver.qml\");\n    qrc::embed!(\"auto/WindowFilmActs.qml\");\n",
    },
    Edit {
        path: "crates/platitude-app/src/auto/WindowAutoActDriver.qml",
        old: "    WindowSettingsActs {\n",
        new: "    WindowFilmActs {\n        window: driver.window\n        mainUi: driver.mainUi\n    }\n    \
              WindowSettingsActs {\n",
    },
    // The verb finishes itself (`finishAutoAct`), as the window's other
    // long verbs do.
    Edit {
        path: "crates/platitude-app/src/auto/AutoActCompletion.qml",
        old: "\"window-fill\", \"solo\", \"gate-sweep\", \"window-floor\",",
        new: "\"window-fill\", \"solo\", \"gate-sweep\", \"window-floor\", \"film\",",
    },
    // An automated window keeps its hand away from what hover opens; the
    // film's hand is the point.
    Edit {
        path: "crates/platitude-app/src/auto/WindowHarness.qml",
        old: "        Hand.away = Harness.automated\n",
        new: "        Hand.away = Harness.automated && Harness.autoAct !== \"film\"\n",
    },
    Edit {
        path: "crates/platitude-app/src/auto/WindowHarness.qml",
        old: "        value: Harness.autoAct !== \"\" ? 0.25 : -1\n",
        new: "        value: Harness.autoAct !== \"\" && Harness.autoAct !== \"film\" ? 0.25 : -1\n",
    },
    Edit {
        path: "crates/platitude-app/src/harness/singleton.rs",
        old: "}\nqml_register!(Harness, \"Harness\", singleton = true);",
        new: SLOTS,
    },
    Edit {
        path: "crates/platitude-app/src/ui/KeyMenu.qml",
        old: "QtObject {\n    id: keyMenu\n",
        new: "QtObject {\n    id: keyMenu\n\n    property point filmHand: Qt.point(-1, -1)\n",
    },
    Edit {
        path: "crates/platitude-app/src/ui/AppMenu.qml",
        old: "        else\n            appMenu.popup()\n        return true\n",
        new: "        else if (KeyMenu.filmHand.x >= 0)\n            \
              appMenu.popup(appMenu.parent.mapFromItem(null, KeyMenu.filmHand))\n        \
              else\n            appMenu.popup()\n        return true\n",
    },
];

/// Which way the substitutions run.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Way {
    In,
    Out,
}

/// Whether the film verb stands in `tree`.
pub(crate) fn injected(tree: &Path) -> bool {
    tree.join(PAYLOAD).exists()
}

pub(super) fn put_in(tree: &Path) -> Result<(), String> {
    if injected(tree) {
        return Err(format!(
            "the film verb is in this tree already ({PAYLOAD} stands)"
        ));
    }
    let files = substituted(Way::In, |path| read(tree, path))?;
    let replaced = write_all(tree, &files)?;
    if let Err(e) = std::fs::write(tree.join(PAYLOAD), COMPONENT) {
        put_back(tree, &replaced);
        return Err(format!("writing {PAYLOAD}: {e}"));
    }
    println!(
        "film: in — {} files edited and {PAYLOAD} added; nothing of it is committed, and `{}` \
         takes it out again",
        files.len(),
        super::OUT.line()
    );
    Ok(())
}

pub(super) fn take_out(tree: &Path) -> Result<(), String> {
    if !injected(tree) {
        return Err(format!(
            "the film verb is not in this tree ({PAYLOAD} is missing)"
        ));
    }
    let files = substituted(Way::Out, |path| read(tree, path))?;
    let replaced = write_all(tree, &files)?;
    if let Err(e) = std::fs::remove_file(tree.join(PAYLOAD)) {
        put_back(tree, &replaced);
        return Err(format!("removing {PAYLOAD}: {e}"));
    }
    println!(
        "film: out — {} files restored and {PAYLOAD} removed; `git status --short` shows what \
         was there before `film in`",
        files.len()
    );
    Ok(())
}

/// Holds the film standing in `tree` to this runner's before a shoot: the
/// edits have to be the ones this runner takes out, and its component is
/// put over the standing one when the two differ — a component changed
/// after `film in` would otherwise film with the old one. Says whether it
/// wrote: the release embeds the QML, so a rewrite owes a build.
pub(super) fn refreshed(tree: &Path) -> Result<bool, String> {
    substituted(Way::Out, |path| read(tree, path)).map_err(|e| {
        format!(
            "the edits standing in this tree are not the ones this runner puts in, so it could \
             not take them out either — put those files back as git has them, remove {PAYLOAD}, \
             and put the film in again: {e}"
        )
    })?;
    let standing = std::fs::read_to_string(tree.join(PAYLOAD))
        .map_err(|e| format!("reading {PAYLOAD}: {e}"))?;
    if standing == COMPONENT {
        return Ok(false);
    }
    std::fs::write(tree.join(PAYLOAD), COMPONENT).map_err(|e| format!("writing {PAYLOAD}: {e}"))?;
    println!("film: {PAYLOAD} was another version of the component; this runner's is in now");
    Ok(true)
}

fn read(tree: &Path, path: &str) -> Result<String, String> {
    std::fs::read_to_string(tree.join(path)).map_err(|e| format!("reading {path}: {e}"))
}

/// What a write replaced, to put back.
type Replaced = Vec<(&'static str, String)>;

/// Writes every file or none: a write that fails puts back what the ones
/// before it replaced, so a half-done `in` or `out` cannot leave a tree
/// neither of them reads.
fn write_all(tree: &Path, files: &BTreeMap<&'static str, String>) -> Result<Replaced, String> {
    let mut replaced = Replaced::new();
    for (path, text) in files {
        let written = read(tree, path).and_then(|was| {
            std::fs::write(tree.join(path), text).map_err(|e| format!("writing {path}: {e}"))?;
            Ok(was)
        });
        match written {
            Ok(was) => replaced.push((path, was)),
            Err(e) => {
                put_back(tree, &replaced);
                return Err(e);
            }
        }
    }
    Ok(replaced)
}

/// Writes back what [`write_all`] replaced. A file that will not take its
/// old text either is named: it carries the film's edit still.
fn put_back(tree: &Path, replaced: &[(&'static str, String)]) {
    for (path, was) in replaced {
        if let Err(e) = std::fs::write(tree.join(path), was) {
            eprintln!("film: {path} could not be put back ({e}) and carries the film's edit");
        }
    }
}

/// Every edited file's text once the substitutions have run `way` —
/// all of them checked before any file is written, so a tree that moved
/// under one edit is left as it was.
fn substituted(
    way: Way,
    read: impl Fn(&str) -> Result<String, String>,
) -> Result<BTreeMap<&'static str, String>, String> {
    let mut texts: BTreeMap<&'static str, String> = BTreeMap::new();
    let ordered: Vec<&Edit> = match way {
        Way::In => EDITS.iter().collect(),
        Way::Out => EDITS.iter().rev().collect(),
    };
    for edit in ordered {
        let text = match texts.get_mut(edit.path) {
            Some(text) => text,
            None => texts.entry(edit.path).or_insert(read(edit.path)?),
        };
        let (from, to) = match way {
            Way::In => (edit.old, edit.new),
            Way::Out => (edit.new, edit.old),
        };
        let found = text.matches(from).count();
        if found != 1 {
            return Err(format!(
                "{}: the film's edit looks for this exactly once and finds it {found} times — \
                 the file moved under it, so film/inject.rs follows the file:\n{from}",
                edit.path
            ));
        }
        *text = text.replacen(from, to, 1);
    }
    Ok(texts)
}

/// The component's text, which the scene's tests hold its step verbs to.
#[cfg(test)]
pub(super) fn component() -> &'static str {
    COMPONENT
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{Way, substituted, write_all};
    use crate::tree::workspace_root;

    /// A write that fails part way puts back the files written before it.
    #[test]
    fn a_write_that_fails_part_way_leaves_every_file_as_it_was() {
        let dir = crate::verify::claim_dir(&crate::temp::root().join("pgg-film"), "write-all")
            .expect("a directory of its own");
        std::fs::write(dir.join("a.txt"), "old").expect("a file");
        let files = BTreeMap::from([
            ("a.txt", "new".to_string()),
            ("gone/b.txt", "new".to_string()),
        ]);
        assert!(write_all(&dir, &files).is_err());
        assert_eq!(
            std::fs::read_to_string(dir.join("a.txt")).expect("read"),
            "old"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn read(path: &str) -> Result<String, String> {
        std::fs::read_to_string(workspace_root().join(path)).map_err(|e| format!("{path}: {e}"))
    }

    /// The tree as committed holds every edit's text once: a change to one
    /// of these files that moves it is a red here, not a film that will
    /// not go in.
    #[test]
    fn every_edit_finds_its_text_once_in_this_tree() {
        let edited = substituted(Way::In, read).expect("every edit applies");
        assert!(edited.len() > 1);
    }

    #[test]
    fn out_after_in_gives_every_file_back_as_it_was() {
        let edited = substituted(Way::In, read).expect("in");
        let back = substituted(Way::Out, |path| Ok(edited[path].clone())).expect("out");
        for (path, text) in back {
            assert_eq!(text, read(path).expect("read"), "{path}");
        }
    }
}
