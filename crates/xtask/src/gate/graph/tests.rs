use super::{
    Carried, bin_exe_names, build, defines_tests, literal_paths, mod_declaration, paths_in,
    reexports_in, string_bodies, strip_comments,
};
use std::collections::BTreeMap;

#[test]
fn markdown_never_enters_a_test_or_directory_dependency() {
    let mut graph = super::Graph::default();
    for reader in ["src/notice.rs", "src/"] {
        for document in ["docs/rules.md", "src/README.MD"] {
            graph.edge(reader, document);
        }
    }
    assert!(graph.deps.is_empty());
    graph.edge("src/notice.rs", "src/fixtures.md/");
    assert!(graph.deps["src/notice.rs"].contains("src/fixtures.md/"));
    graph.edge("src/notice.rs", "src/input.txt");
    assert!(graph.deps["src/notice.rs"].contains("src/input.txt"));
}

#[test]
fn a_test_of_any_runtime_counts_and_a_cfg_guard_does_not() {
    assert!(defines_tests("#[test]\nfn t() {}\n"));
    assert!(defines_tests("#[tokio::test]\nasync fn t() {}\n"));
    assert!(defines_tests(
        "#[tokio::test(flavor = \"multi_thread\")]\nasync fn t() {}\n"
    ));
    assert!(defines_tests("#[cfg(test)]\nmod tests {\n}\n"));
    assert!(!defines_tests("#[cfg(test)]\nmod state_tests;\n"));
}

#[test]
fn strings_are_read_by_the_tokenizer_not_by_pairing_quotes() {
    let bodies = string_bodies(
        "let a = '\"';\nlet b = \"crates/x.rs\"; // \"not/this.rs\"\nlet c = \"two\\\"quotes\";\n\
         let d = r#\"raw \"inner\" path/y.rs\"#;\n",
    );
    assert_eq!(
        bodies,
        vec!["crates/x.rs", "two\\\"quotes", "raw \"inner\" path/y.rs"]
    );
}

/// The `b` has to be its own word: `lib"x"` is not a byte string, and
/// the tokenizer still walks past it whole.
#[test]
fn a_byte_string_is_bytes_and_not_a_path() {
    assert_eq!(
        string_bodies("let a = b\"../\";\nlet b = \"kept/y.rs\";\n"),
        vec!["kept/y.rs"]
    );
    assert_eq!(
        string_bodies("let a = br#\"../\"#;\nlet b = \"kept/y.rs\";\n"),
        vec!["kept/y.rs"]
    );
    assert_eq!(
        string_bodies("let a = lib\"../\";\n"),
        vec!["../"],
        "a b that is the tail of a word opens no byte string"
    );
}

#[test]
fn a_literal_naming_an_ancestor_directory_or_the_census_is_no_edge() {
    let root = crate::tree::workspace_root();
    let named = |file: &str, literal: &str| literal_paths(&root, file, &[literal.to_string()]);
    let plan = "crates/xtask/src/gate/plan.rs";
    assert!(named(plan, "crates/").is_empty());
    assert!(named(plan, "../").is_empty());
    assert!(named("crates/xtask/src/hook/seat.rs", "crates/xtask").is_empty());
    assert!(named(plan, crate::gate::census::FILE).is_empty());
    assert!(named(plan, crate::gate::tiers::FILE).is_empty());
    // What the rule keeps: a directory the file is not in, and a file
    // of its own. Spelled in pieces, or this very file would read them.
    let ui = format!("crates/{}/src/ui", "platitude-app");
    assert_eq!(named(plan, &ui), vec![format!("{ui}/")]);
    let baseline = format!("crates/xtask/{}", "structure-baseline.txt");
    assert_eq!(named(plan, &baseline), vec![baseline.clone()]);
}

#[test]
fn reads_module_declarations_and_nothing_that_merely_mentions_mod() {
    assert_eq!(mod_declaration("mod stash;"), Some("stash"));
    assert_eq!(mod_declaration("pub(crate) mod refs;"), Some("refs"));
    assert_eq!(mod_declaration("mod tests {"), None);
    assert_eq!(mod_declaration("// mod x;"), None);
}

#[test]
fn splits_paths_and_groups_from_every_root_it_knows() {
    let roots: BTreeMap<String, String> = [("platitude_core".to_string(), "x".to_string())].into();
    // Bare words: the file's own child `stash`, and `refs` brought in
    // by a glob import of the parent.
    let bare: BTreeMap<String, Vec<String>> = [
        (
            "stash".to_string(),
            vec!["self".to_string(), "stash".to_string()],
        ),
        (
            "refs".to_string(),
            vec!["super".to_string(), "refs".to_string()],
        ),
    ]
    .into();
    let found = paths_in(
        "use crate::stash::{Stash, self};\nlet x = super::refs::RemoteBranches::new();\n\
         use platitude_core::session as s;\nuse std::io;\nfoo::bar();\nstash::Stash::new();\n\
         refs::RemoteBranches::new()",
        &roots,
        &bare,
    );
    let joined: Vec<String> = found.iter().map(|p| p.join("::")).collect();
    assert_eq!(
        joined,
        vec![
            "crate::stash::Stash",
            "crate::stash",
            "super::refs::RemoteBranches::new",
            "platitude_core::session",
            "self::stash::Stash::new",
            "super::refs::RemoteBranches::new",
        ]
    );
}

#[test]
fn super_inside_an_inline_module_is_the_file_itself() {
    let roots: BTreeMap<String, String> = BTreeMap::new();
    let bare: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let found = paths_in(
        "use super::sibling::X;\n#[cfg(test)]\nmod tests {\n    use super::*;\n    use super::super::other::Y;\n}\n",
        &roots,
        &bare,
    );
    let joined: Vec<String> = found.iter().map(|p| p.join("::")).collect();
    assert_eq!(
        joined,
        vec!["super::sibling::X", "self", "self::super::other::Y"]
    );
}

#[test]
fn a_comment_is_not_a_path() {
    let code = strip_comments("use crate::a; // see crate::b\n/* crate::c\n */ crate::d::e();\n");
    assert!(code.contains("crate::a") && code.contains("crate::d::e"));
    assert!(!code.contains("crate::b") && !code.contains("crate::c"));
    assert_eq!(
        code.lines().count(),
        3,
        "line structure survives for anything counting lines"
    );
}

#[test]
fn a_string_is_not_a_path_and_a_char_quote_opens_none() {
    let code = strip_comments(
        "let a = \"crate::x\";\nlet b = '\"';\nlet c = r#\"crate::y \"quoted\"\"#;\n\
         let d = 'a';\ncrate::z();\nlet e = \"two\\\"quotes\";\ncrate::w();\n",
    );
    assert!(
        code.contains("crate::z") && code.contains("crate::w"),
        "{code}"
    );
    assert!(
        !code.contains("crate::x") && !code.contains("crate::y"),
        "{code}"
    );
    assert_eq!(code.lines().count(), 7, "{code}");
}

#[test]
fn reads_re_exports_by_the_name_they_export() {
    let (named, globs) = reexports_in(
        "pub use error::GitError;\npub use model::{CommitMeta, StrPool as Pool};\n\
         pub(crate) use process::{\n    Executor,\n    outcome::Outcome,\n};\npub use walk::*;\n",
    );
    assert_eq!(named["GitError"], vec!["error", "GitError"]);
    assert_eq!(named["Pool"], vec!["model", "StrPool"]);
    assert_eq!(named["Executor"], vec!["process", "Executor"]);
    assert_eq!(named["Outcome"], vec!["process", "outcome", "Outcome"]);
    assert_eq!(globs, vec![vec!["walk".to_string()]]);
}

/// A root's re-exports usually stand below its `pub(crate) mod` and
/// `pub(crate) fn` items; the first `pub(` that is not a `use` must
/// not end the reading, or every reader of those names is put down
/// as reading the root itself.
#[test]
fn a_re_export_below_a_scoped_item_is_still_read() {
    let (named, globs) = reexports_in(
        "pub(crate) mod permit;\npub(crate) fn seat() {}\n\
         pub(crate) use approval::{FLAGS, gui as GUI};\npub(crate) use hooks::*;\n",
    );
    assert_eq!(named["FLAGS"], vec!["approval", "FLAGS"]);
    assert_eq!(named["GUI"], vec!["approval", "gui"]);
    assert_eq!(globs, vec![vec!["hooks".to_string()]]);
}

#[test]
fn a_bin_is_named_by_the_variable_cargo_sets_for_it() {
    // Spelled in pieces so this file names no binary of its own.
    let var = concat!("CARGO_BIN_EXE", "_pgg-todo-editor");
    let code = format!(
        "const EXE: &str = env!(\"{var}\");\nlet late = option_env!(\"{var}\");\n\
         // env!(\"{var}\") in a comment shoots nothing\n\
         let plain = \"not/a/bin\";\nlet bare = \"CARGO_BIN_EXE_\";\n"
    );
    assert_eq!(
        bin_exe_names(&string_bodies(&code)),
        vec!["pgg-todo-editor", "pgg-todo-editor"],
        "the comment is not one of them"
    );
}

/// Without this edge, a change to a binary, or to anything only it
/// reads, selects none of the tests that run it.
#[test]
fn a_test_that_shoots_a_binary_reads_the_binary() {
    let root = crate::tree::workspace_root();
    let g = build(&root).expect("the graph of this tree");
    // Spelled in pieces: whole paths here would be read as this very
    // file reading each of them.
    let pairs = [
        ("xtask/tests/gate", "support.rs", "xtask/src", "main.rs"),
        (
            "platitude-core/tests/it/support",
            "integrate.rs",
            "platitude-core/src/bin",
            "pgg-todo-editor.rs",
        ),
    ];
    for (test_dir, test_name, bin_dir, bin_name) in pairs {
        let test = format!("crates/{test_dir}/{test_name}");
        let bin = format!("crates/{bin_dir}/{bin_name}");
        assert!(
            g.deps.get(&test).is_some_and(|reads| reads.contains(&bin)),
            "{test} shoots {bin} and the graph does not know it"
        );
    }
}

/// The graph `edges` draw, read both ways, with the integration
/// binaries' modules named.
fn drawn(edges: &[(&str, &str)], sandboxed: &[&str]) -> super::Graph {
    let mut g = super::Graph::default();
    for (from, to) in edges {
        g.edge(from, to);
    }
    for file in sandboxed {
        g.modules.insert(
            (*file).to_string(),
            super::Module {
                krate: "probe".to_string(),
                path: Vec::new(),
                package: "probe".to_string(),
                test_binary: Some("probe".to_string()),
                has_tests: true,
            },
        );
    }
    for (from, to) in g.deps.clone() {
        for target in to {
            g.rdeps.entry(target).or_default().insert(from.clone());
        }
    }
    g
}

/// What still reaches the binary: the tool's own code, a file outside
/// the product, and a product change the binary's own crate compiles in.
#[test]
fn a_product_file_read_off_the_disk_stops_at_an_integration_binary() {
    // Spelled in pieces and named after nothing on disk, so that this
    // file reads none of them.
    let qml = format!("crates/{}/src/ui/Probe.qml", "platitude-app");
    let ui = format!("crates/{}/src/ui/", "platitude-app");
    let core = format!("crates/{}/src/probe.rs", "platitude-core");
    let core_it = format!("crates/{}/tests/probe/main.rs", "platitude-core");
    let tool = format!("crates/{}/src/probe_reader.rs", "xtask");
    let bin = format!("crates/{}/src/probe_main.rs", "xtask");
    let shooter = format!("crates/{}/tests/probe/main.rs", "xtask");
    let script = format!("{}/probe-hook", ".githooks");
    let g = drawn(
        &[
            (&ui, &qml),
            (&tool, &ui),
            (&tool, &script),
            (&bin, &tool),
            (&shooter, &bin),
            (&core_it, &core),
        ],
        &[&shooter, &core_it],
    );
    let reach =
        |changed: &[&String]| g.reach(&changed.iter().map(|f| (*f).clone()).collect::<Vec<_>>());
    let from_the_product = reach(&[&qml]);
    for owed in [&ui, &tool, &bin] {
        assert!(
            from_the_product.contains_key(owed),
            "{owed}: {from_the_product:?}"
        );
    }
    // The product's own directory changed; the tool only reads it.
    assert_eq!(from_the_product[&ui], Carried::Whole);
    assert_eq!(from_the_product[&tool], Carried::AsProductFile);
    assert_eq!(from_the_product[&bin], Carried::AsProductFile);
    assert!(
        !from_the_product.contains_key(&shooter),
        "a sandboxed binary is no reader of the product's files: {from_the_product:?}"
    );
    for changed in [&script, &tool] {
        assert_eq!(
            reach(&[changed]).get(&shooter),
            Some(&Carried::Whole),
            "{changed} is read or run by the binary itself"
        );
    }
    assert!(reach(&[&core]).contains_key(&core_it), "compiled in");
    // Handed on whole by one path, the binary is owed whatever else
    // handed the same file less.
    let both = reach(&[&qml, &tool]);
    assert!(both.contains_key(&shooter));
    assert_eq!(both[&tool], Carried::Whole);
    assert_eq!(
        g.why(&[qml.clone(), tool.clone()], &shooter),
        Some(vec![tool.clone(), bin.clone(), shooter.clone()])
    );
    assert_eq!(g.why(std::slice::from_ref(&qml), &shooter), None);

    // The stop read back: a binary's key names what would reach it.
    let inputs =
        |files: &[&String]| g.inputs(&files.iter().map(|f| (*f).clone()).collect::<Vec<_>>());
    let of_the_shooter = inputs(&[&shooter]);
    for named in [&bin, &tool, &script] {
        assert!(
            of_the_shooter.contains(named),
            "{named}: {of_the_shooter:?}"
        );
    }
    for spared in [&ui, &qml] {
        assert!(
            !of_the_shooter.contains(spared),
            "{spared} is the sandbox's to lay out: {of_the_shooter:?}"
        );
    }
    // A tool's own tests may read the data: nothing stops before them.
    let of_the_tool = inputs(&[&tool]);
    for named in [&ui, &qml, &script] {
        assert!(of_the_tool.contains(named), "{named}: {of_the_tool:?}");
    }
    // Compiled in, the product is the binary's own.
    assert!(inputs(&[&core_it]).contains(&core));
}

/// The same stop on the tree as it stands; the hook script the sandbox
/// tests copy off the real tree still reaches them.
#[test]
fn the_apps_window_owes_the_census_tests_and_not_the_gates_sandbox() {
    let root = crate::tree::workspace_root();
    let g = build(&root).expect("the graph of this tree");
    let window = format!("crates/{}/src/ui/{}.qml", "platitude-app", "Main");
    let reach = g.reach(&[window]);
    assert_eq!(
        reach.get("crates/xtask/src/gate/census.rs"),
        Some(&Carried::AsProductFile),
        "{reach:?}"
    );
    let sandbox = format!("crates/xtask/{}/", "tests");
    let shot: Vec<&String> = reach.keys().filter(|f| f.starts_with(&sandbox)).collect();
    assert!(shot.is_empty(), "{shot:?}");
    let hook = format!("{}/{}", ".githooks", "reference-transaction");
    let support = format!("crates/xtask/{}/gate/support.rs", "tests");
    assert!(g.reach(std::slice::from_ref(&hook)).contains_key(&support));
    // The suite's key names the runner it shoots and the hook it
    // copies, and no product path: the product is what it lays out.
    let suite: Vec<String> = g
        .modules
        .keys()
        .filter(|f| f.starts_with(&sandbox))
        .cloned()
        .collect();
    assert!(!suite.is_empty());
    let inputs = g.inputs(&suite);
    let runner = format!("crates/xtask/{}/main.rs", "src");
    for named in [&hook, &runner] {
        assert!(inputs.contains(named), "{named}: {inputs:?}");
    }
    let product: Vec<&String> = inputs.iter().filter(|f| super::is_product(f)).collect();
    assert!(product.is_empty(), "{product:?}");
}

/// The gate's own reading (`complaints`), against a graph read fresh
/// off the sources.
#[test]
fn the_crate_roots_have_no_readers_and_every_path_resolves() {
    let root = crate::tree::workspace_root();
    let g = build(&root).expect("the graph of this tree");
    assert_eq!(super::complaints(&g), Vec::<String>::new());
    // A file two crates declare by `#[path]` is one node (`lexical`).
    let doubled: Vec<&String> = g.modules.keys().filter(|f| f.contains("/../")).collect();
    assert!(doubled.is_empty(), "{doubled:?}");
}

/// CI's files are CI's alone: nothing a gate runs reads one, so an edit
/// there reaches no file but itself. Qt's version, which the workflow
/// once carried, is a file of its own (`qt::PIN`).
#[test]
fn nothing_reads_a_file_of_cis() {
    let root = crate::tree::workspace_root();
    let g = build(&root).expect("the graph of this tree");
    // In pieces: whole, the name would be this file reading all of it.
    let ci = format!(".{}", "github");
    let mut files = Vec::new();
    let mut dirs = vec![root.join(&ci)];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir).expect("a directory of CI's") {
            let path = entry.expect("an entry of it").path();
            if path.is_dir() {
                dirs.push(path);
            } else {
                files.push(super::rel(&root, &path));
            }
        }
    }
    // The workflow older commits pin Qt in is spelled as it stands here.
    assert!(files.contains(&crate::qt::former_pin()), "{files:?}");
    for file in files {
        let reach = g.reach(std::slice::from_ref(&file));
        assert_eq!(reach.keys().collect::<Vec<_>>(), [&file]);
    }
}

#[test]
fn a_path_declared_up_the_tree_folds_to_the_files_own_name() {
    use std::path::{Path, PathBuf};
    assert_eq!(
        super::lexical(Path::new("crates/x/tests/gate/../../src/wait.rs")),
        PathBuf::from("crates/x/src/wait.rs")
    );
    assert_eq!(
        super::lexical(Path::new("crates/x/src/./y.rs")),
        PathBuf::from("crates/x/src/y.rs")
    );
}
