//! `contents`' own tests, in a file of their own (structure.md §分割).

use super::{
    Carried, FRAMEWORKS, Goes, Import, QML_LEFT_OUT, QML_MODULES, QML_UNREACHED, QT_PLUGINS,
    QT_PLUGINS_LEFT_OUT, carried, goes, module_dir, parted, plugin_link, plugin_of, reached,
    scanned, written,
};

/// What qmlimportscanner 6.12 answered for `ui/` with no import path.
const WRITTEN: &str = r#"[
    {
        "name": "QtQuick",
        "type": "module"
    },
    {
        "name": "QtQuick.Controls.Fusion",
        "type": "module"
    },
    {
        "name": "QtQuick.Layouts",
        "type": "module"
    },
    {
        "name": "platitude.ui",
        "type": "module"
    },
    {
        "name": "platitude",
        "type": "module"
    },
    {
        "name": "QtQuick.Dialogs",
        "type": "module"
    },
    {
        "name": "QtQuick.Window",
        "type": "module"
    },
    {
        "name": "QtQuick.Controls",
        "type": "module"
    }
]
"#;

/// Entries of its answers with an import path: a flag, a module's
/// files (cut to two), the app's own module, and the two path imports.
const FOLLOWED: &str = r#"[
    {
        "classname": "QtQuick2Plugin",
        "linkTarget": "Qt6::qtquick2plugin",
        "name": "QtQuick",
        "path": "C:/Qt/6.12.0/msvc2022_64/qml/QtQuick",
        "plugin": "qtquick2plugin",
        "pluginIsOptional": true,
        "prefer": ":/qt-project.org/imports/QtQuick/",
        "relativePath": "QtQuick",
        "type": "module"
    },
    {
        "classname": "QtQuickControls2FusionStylePlugin",
        "components": [
            "C:/Qt/6.12.0/msvc2022_64/qml/QtQuick/Controls/Fusion/ApplicationWindow.qml",
            "C:/Qt/6.12.0/msvc2022_64/qml/QtQuick/Controls/Fusion/BusyIndicator.qml"
        ],
        "linkTarget": "Qt6::qtquickcontrols2fusionstyleplugin",
        "name": "QtQuick.Controls.Fusion",
        "path": "C:/Qt/6.12.0/msvc2022_64/qml/QtQuick/Controls/Fusion",
        "plugin": "qtquickcontrols2fusionstyleplugin",
        "prefer": ":/qt-project.org/imports/QtQuick/Controls/Fusion/",
        "relativePath": "QtQuick/Controls/Fusion",
        "type": "module"
    },
    {
        "name": "platitude",
        "type": "module"
    },
    {
        "name": "sub",
        "path": "C:/Users/wrongwrong/IdeaProjects/platitude-gg/.claude/worktrees/b/target/scanner-sample/sub",
        "type": "directory"
    },
    {
        "name": "util.js",
        "path": "C:/Users/wrongwrong/IdeaProjects/platitude-gg/.claude/worktrees/b/target/scanner-sample/util.js",
        "type": "javascript"
    }
]

"#;

fn module(name: &str, path: Option<&str>) -> Import {
    Import {
        name: name.to_string(),
        kind: "module".to_string(),
        path: path.map(str::to_string),
    }
}

/// What the runner's qmlimportscanner reached from `ui/` through Qt
/// 6.12's modules, in the order macdeployqt's account of the Snapshot
/// run 37238446842 names them: the app's own two in no directory, then
/// by path.
const RUNNER_REACHED: [&str; 34] = [
    "platitude.ui",
    "platitude",
    "QML",
    "Qt.labs.folderlistmodel",
    "QtQml",
    "QtQml.Models",
    "QtQml.WorkerScript",
    "QtQuick",
    "QtQuick.Controls",
    "QtQuick.Controls.Basic",
    "QtQuick.Controls.Basic.impl",
    "QtQuick.Controls.FluentWinUI3",
    "QtQuick.Controls.FluentWinUI3.impl",
    "QtQuick.Controls.Fusion",
    "QtQuick.Controls.Fusion.impl",
    "QtQuick.Controls.Imagine",
    "QtQuick.Controls.Imagine.impl",
    "QtQuick.Controls.Material",
    "QtQuick.Controls.Material.impl",
    "QtQuick.Controls.Universal",
    "QtQuick.Controls.Universal.impl",
    "QtQuick.Controls.iOS",
    "QtQuick.Controls.iOS.impl",
    "QtQuick.Controls.impl",
    "QtQuick.Controls.macOS",
    "QtQuick.Controls.macOS.impl",
    "QtQuick.Dialogs",
    "QtQuick.Dialogs.quickimpl",
    "QtQuick.Effects",
    "QtQuick.Layouts",
    "QtQuick.NativeStyle",
    "QtQuick.Shapes",
    "QtQuick.Templates",
    "QtQuick.Window",
];

fn runner_reached() -> Vec<Import> {
    RUNNER_REACHED
        .iter()
        .map(|name| match *name {
            "platitude.ui" | "platitude" => module(name, None),
            _ => module(
                name,
                Some(&format!(
                    "/Users/runner/work/_temp/qt/6.12.0/macos/qml/{}",
                    module_dir(name)
                )),
            ),
        })
        .collect()
}

/// A bundle exactly as the lists say.
fn as_listed() -> Carried {
    Carried {
        frameworks: FRAMEWORKS
            .iter()
            .map(|name| format!("{name}.framework"))
            .collect(),
        plugins: QT_PLUGINS.iter().map(|plugin| plugin.to_string()).collect(),
        quick: vec![
            "libqquicklayoutsplugin.dylib".to_string(),
            "libqtquickcontrols2fusionstyleplugin.dylib".to_string(),
        ],
        modules: QML_MODULES
            .iter()
            .map(|name| {
                let plugin = match *name {
                    "QtQuick.Layouts" => Some("qquicklayoutsplugin".to_string()),
                    "QtQuick.Controls.Fusion" => {
                        Some("qtquickcontrols2fusionstyleplugin".to_string())
                    }
                    _ => None,
                };
                (name.to_string(), plugin)
            })
            .collect(),
    }
}

#[test]
fn each_thing_is_named_once_and_on_one_side() {
    let mut seen = std::collections::BTreeSet::new();
    for name in QML_MODULES.iter().chain(
        QML_LEFT_OUT
            .iter()
            .chain(QML_UNREACHED.iter())
            .map(|(name, _)| name),
    ) {
        assert!(seen.insert(*name), "{name} is named twice");
    }
    // The runner's own reach is the witness that these are unreached.
    for (name, _) in QML_UNREACHED {
        assert!(!RUNNER_REACHED.contains(&name), "{name} is reached");
    }
    let mut frameworks = std::collections::BTreeSet::new();
    for name in FRAMEWORKS {
        assert!(frameworks.insert(name), "{name} is named twice");
    }
    for plugin in QT_PLUGINS {
        let kind = plugin.split('/').next().expect("a directory");
        assert!(
            !QT_PLUGINS_LEFT_OUT.iter().any(|(left, _)| *left == kind),
            "{plugin} is in a directory left out"
        );
    }
}

/// macdeployqt's own example (shared.cpp, `recursiveCopyAndDeploy`) is
/// the witness to how far up a link climbs.
#[test]
fn a_modules_link_climbs_out_to_contents_as_macdeployqts_does() {
    assert_eq!(
        plugin_link("QtQuick.Controls", "libqtquickcontrolsplugin.dylib"),
        "../../../../PlugIns/quick/libqtquickcontrolsplugin.dylib"
    );
    assert_eq!(
        plugin_link("QtQuick", "libqtquick2plugin.dylib"),
        "../../../PlugIns/quick/libqtquick2plugin.dylib"
    );
    assert_eq!(
        module_dir("QtQuick.Controls.Fusion.impl"),
        "QtQuick/Controls/Fusion/impl"
    );
}

/// qmldirs as Qt 6.12 installs them.
#[test]
fn a_qmldir_names_its_plugin_whether_optional_or_not() {
    let fusion = "module QtQuick.Controls.Fusion\nlinktarget Qt6::qtquickcontrols2fusionstyleplugin\nplugin qtquickcontrols2fusionstyleplugin\nclassname QtQuickControls2FusionStylePlugin\ntypeinfo plugins.qmltypes\nimport QtQuick.Controls.Basic auto\ndepends QtQuick auto\nprefer :/qt-project.org/imports/QtQuick/Controls/Fusion/\nApplicationWindow 6.0 ApplicationWindow.qml\n";
    let layouts = "module QtQuick.Layouts\nlinktarget Qt6::qquicklayoutsplugin\noptional plugin qquicklayoutsplugin\nclassname QtQuickLayoutsPlugin\ndesignersupported\ntypeinfo plugins.qmltypes\ndepends QtQuick auto\nprefer :/qt-project.org/imports/QtQuick/Layouts/\n";
    let builtins = "module QML\ndesignersupported\nstatic\nsystem\ntypeinfo plugins.qmltypes\nprefer :/qt-project.org/imports/QML/\n";
    assert_eq!(plugin_of(fusion), Some("qtquickcontrols2fusionstyleplugin"));
    assert_eq!(plugin_of(layouts), Some("qquicklayoutsplugin"));
    assert_eq!(plugin_of(builtins), None);
}

/// A module directory's files as Qt 6.12 for macOS installs Fusion's
/// (macdeployqt's account of the Snapshot run 37238446842).
#[test]
fn a_modules_plugin_goes_to_quick_and_the_rest_stays_with_the_module() {
    assert_eq!(
        goes("libqtquickcontrols2fusionstyleplugin.dylib"),
        Goes::Quick
    );
    assert_eq!(
        goes("libqtquickcontrols2fusionstyleplugin_debug.dylib"),
        Goes::Nowhere
    );
    for file in ["qmldir", "plugins.qmltypes", "Button.qml"] {
        assert_eq!(goes(file), Goes::WithModule, "{file}");
    }
}

#[test]
fn the_scanners_answer_gives_each_imports_name_type_and_path() {
    let followed = scanned(FOLLOWED).expect("read");
    assert_eq!(followed.len(), 5);
    assert_eq!(
        followed[0],
        module("QtQuick", Some("C:/Qt/6.12.0/msvc2022_64/qml/QtQuick"))
    );
    assert_eq!(followed[1].name, "QtQuick.Controls.Fusion");
    assert_eq!(followed[2], module("platitude", None));
    assert_eq!(
        (followed[3].kind.as_str(), followed[4].kind.as_str()),
        ("directory", "javascript")
    );
    let written_names: Vec<String> = scanned(WRITTEN)
        .expect("read")
        .into_iter()
        .map(|import| import.name)
        .collect();
    assert_eq!(written_names.len(), 8);
    assert_eq!(scanned("[]\n").expect("read"), Vec::new());
    // What it answers for a `.js` holding `.import "helper.js" as Helper`.
    let script = "[\n    {\n        \"path\": \"helper.js\",\n        \"type\": \"javascript\"\n    }\n]\n\n";
    assert_eq!(
        scanned(script).expect("read"),
        [Import {
            name: String::new(),
            kind: "javascript".to_string(),
            path: Some("helper.js".to_string()),
        }]
    );
}

#[test]
fn an_answer_cut_short_or_running_on_is_refused() {
    let cut = &FOLLOWED[..FOLLOWED
        .find("\"relativePath\": \"QtQuick/Controls")
        .expect("cut")];
    assert!(scanned(cut).is_err());
    assert!(scanned("[]\nqmlimportscanner: warning").is_err());
    let nameless = scanned("[{\"type\": \"module\"}]").expect_err("refused");
    assert!(nameless.contains("without its name or type"), "{nameless}");
}

#[test]
fn each_module_ui_writes_must_be_carried() {
    let mut imports = scanned(WRITTEN).expect("read");
    assert_eq!(written(&imports), Vec::<String>::new());
    imports.push(module("QtQuick.Effects", None));
    imports.push(module("QtQuick.Particles", None));
    let parted = written(&imports);
    assert_eq!(parted.len(), 2, "{parted:?}");
    assert!(
        parted[0].starts_with("ui/ imports QtQuick.Effects, which the bundle leaves out as"),
        "{parted:?}"
    );
    assert_eq!(
        parted[1],
        "ui/ imports QtQuick.Particles, which the bundle does not carry"
    );
}

#[test]
fn each_module_reached_is_on_one_list_and_each_listed_is_reached() {
    let mut imports = runner_reached();
    assert_eq!(reached(&imports), Vec::<String>::new());
    imports.retain(|import| import.name != "QtQuick.Controls.iOS");
    imports.push(module(
        "QtQuick.Controls.Glass",
        Some("/qt/qml/QtQuick/Controls/Glass"),
    ));
    imports.push(module("QtQuick.Pdf", None));
    assert_eq!(
        reached(&imports),
        [
            "QtQuick.Controls.Glass is reached and on neither list: carry it or leave it \
             out, with why",
            "QtQuick.Pdf is reached and no directory of Qt's QML holds it",
            "QtQuick.Controls.iOS is listed and nothing reaches it",
        ]
    );
}

#[test]
fn a_bundle_as_listed_parts_from_the_lists_nowhere() {
    assert_eq!(parted(&as_listed()), Vec::<String>::new());
}

#[test]
fn a_bundle_that_moved_is_named_both_ways_with_why_the_extra_stays_out() {
    let mut moved = as_listed();
    moved.frameworks.retain(|name| name != "QtSvg.framework");
    moved.frameworks.push("QtWidgets.framework".to_string());
    moved.frameworks.push("libz.dylib".to_string());
    moved.plugins.push("styles/libqmacstyle.dylib".to_string());
    moved.modules.push((
        "QtQuick.Shapes".to_string(),
        Some("qmlshapesplugin".to_string()),
    ));
    moved.modules.push((
        "QtQuick.LocalStorage".to_string(),
        Some("qmllocalstorageplugin".to_string()),
    ));
    moved
        .quick
        .retain(|file| file != "libqquicklayoutsplugin.dylib");
    moved.quick.push("libqmlshapesplugin.dylib".to_string());
    moved
        .quick
        .push("libqmllocalstorageplugin.dylib".to_string());
    moved
        .quick
        .push("libqmlxmllistmodelplugin.dylib".to_string());
    let parted = parted(&moved);
    let expected = [
        "Frameworks holds libz.dylib, which is no framework",
        "a framework no list names: QtWidgets",
        "a framework listed and not carried: QtSvg",
        "a plugin no list names: styles/libqmacstyle.dylib — left out as QtWidgets' styles",
        "a QML module no list names: QtQuick.Shapes — left out as a module only Basic's \
         SelectionRectangle imports, which Fusion replaces",
        "a QML module no list names: QtQuick.LocalStorage — left out as a database for QML: \
         nothing in ui/ reaches it, and its plugin alone would bring QtSql",
        "a QML plugin no carried qmldir names: libqmlxmllistmodelplugin.dylib",
        "a QML plugin a carried qmldir names and PlugIns/quick lacks: \
         libqquicklayoutsplugin.dylib",
    ];
    assert_eq!(parted.len(), expected.len(), "{parted:#?}");
    for (said, want) in parted.iter().zip(expected) {
        assert!(
            said.starts_with(want),
            "{said}\n  does not start with\n{want}"
        );
    }
}

/// The directories a bundle stands in, minus the links: the next test
/// has those, where links can be made.
#[test]
fn carried_reads_a_bundles_directories_and_skips_what_is_no_module() {
    let root = crate::verify::claim_dir(&crate::temp::root().join("pgg-package"), "carried")
        .expect("a directory of its own");
    let contents = root.join("Contents");
    let dirs = [
        "Frameworks/QtCore.framework/Versions/A",
        "PlugIns/platforms",
        "PlugIns/quick",
        "Resources/qml/QtQuick/Controls/designer",
    ];
    for dir in dirs {
        std::fs::create_dir_all(contents.join(dir)).expect("a directory");
    }
    let files = [
        ("PlugIns/platforms/libqcocoa.dylib", ""),
        ("PlugIns/quick/libqtquick2plugin.dylib", ""),
        ("PlugIns/qt.conf", ""),
        (
            "Resources/qml/QtQuick/qmldir",
            "module QtQuick\noptional plugin qtquick2plugin\n",
        ),
        (
            "Resources/qml/QtQuick/Controls/qmldir",
            "module QtQuick.Controls\nplugin qtquickcontrols2plugin\n",
        ),
        (
            "Resources/qml/QtQuick/Controls/designer/ButtonSpecifics.qml",
            "",
        ),
    ];
    for (file, text) in files {
        std::fs::write(contents.join(file), text).expect("a file");
    }
    let read = carried(&root);
    let _ = std::fs::remove_dir_all(&root);
    assert_eq!(
        read.expect("read"),
        Carried {
            frameworks: vec!["QtCore.framework".to_string()],
            plugins: vec![
                "platforms/libqcocoa.dylib".to_string(),
                "qt.conf".to_string()
            ],
            quick: vec!["libqtquick2plugin.dylib".to_string()],
            modules: vec![
                ("QtQuick".to_string(), Some("qtquick2plugin".to_string())),
                (
                    "QtQuick.Controls".to_string(),
                    Some("qtquickcontrols2plugin".to_string())
                ),
            ],
        }
    );
}

/// A module's plugin as package lays it out — the file in
/// `PlugIns/quick`, a link to it in the module's directory — and a
/// directory linked back up, which a walk that followed links would go
/// round for ever. Off Windows only, where a link is made as on a Mac:
/// the gate's Linux side runs it.
#[cfg(unix)]
#[test]
fn a_modules_link_reaches_its_plugin_and_no_linked_directory_is_walked() {
    let root = crate::verify::claim_dir(&crate::temp::root().join("pgg-package"), "linked")
        .expect("a directory of its own");
    let quick = root.join("Contents/PlugIns/quick");
    let module = root.join("Contents/Resources/qml/QtQuick/Layouts");
    for dir in [&quick, &module] {
        std::fs::create_dir_all(dir).expect("a directory");
    }
    let plugin = "libqquicklayoutsplugin.dylib";
    std::fs::write(quick.join(plugin), "the plugin").expect("the plugin");
    std::fs::write(
        module.join("qmldir"),
        "module QtQuick.Layouts\noptional plugin qquicklayoutsplugin\n",
    )
    .expect("its qmldir");
    std::os::unix::fs::symlink(plugin_link("QtQuick.Layouts", plugin), module.join(plugin))
        .expect("the link");
    std::os::unix::fs::symlink("..", module.join("Up")).expect("a link up");
    let through_the_link = std::fs::read_to_string(module.join(plugin));
    let read = carried(&root);
    let _ = std::fs::remove_dir_all(&root);
    assert_eq!(through_the_link.expect("the link resolves"), "the plugin");
    let read = read.expect("read");
    assert_eq!(read.quick, [plugin]);
    assert_eq!(
        read.modules,
        [(
            "QtQuick.Layouts".to_string(),
            Some("qquicklayoutsplugin".to_string())
        )]
    );
}
