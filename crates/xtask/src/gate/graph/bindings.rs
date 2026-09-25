//! What a reader of a re-exported name depends on: the file that defines
//! the name and the file whose `pub use` binds it. Repointing the binding
//! changes what the reader gets without touching what it named before,
//! so the reader is owed by either; a `pub use` passes no data, so what
//! the binding's file reads elsewhere is not handed on through it. Built
//! on a tree of its own under the temp directory, so the shape can be
//! rewritten mid-test.

use std::path::{Path, PathBuf};

use super::{Carried, build, complaints};

/// A workspace laid out for one test and taken away after it.
struct Tree(PathBuf);

impl Tree {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir()
            .join("pgg-tests")
            .join(format!("graph-bindings-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a root for the probe tree");
        Self(root)
    }

    fn write(&self, relative: &str, text: &str) {
        let path = self.0.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("the file's directory");
        }
        std::fs::write(&path, text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    }

    fn root(&self) -> &Path {
        &self.0
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const FACADE: &str = "crates/tool/src/facade/mod.rs";
const A: &str = "crates/tool/src/facade/a.rs";
const B: &str = "crates/tool/src/facade/b.rs";
const LEAF: &str = "crates/tool/src/facade/leaf.rs";
const CONSUMER: &str = "crates/tool/src/consumer.rs";
const DIRECT: &str = "crates/tool/src/direct.rs";
const INLINE: &str = "crates/tool/src/inline.rs";

/// A path of the probe tree's product crates, spelled in pieces so that
/// this file names nothing of the real product ([`super::is_product`]).
fn core(rest: &str) -> String {
    format!("crates/{}/{rest}", "platitude-core")
}

fn app(rest: &str) -> String {
    format!("crates/{}/{rest}", "platitude-app")
}

/// A test module that asserts one expression.
fn test_of(body: &str) -> String {
    format!("#[cfg(test)]\nmod tests {{\n    #[test]\n    fn t() {{\n        {body}\n    }}\n}}\n")
}

/// A facade that binds `answer` to one of two modules holding one each,
/// reads a product directory as data in a function of its own, and holds
/// a leaf nothing of that reaches. Three readers: one through the
/// binding, one of the facade's own function, one by the leaf's path.
/// `b` reads the product directory too when `b_reads` is set.
fn laid_out(name: &str, bound_to: &str, b_reads: bool) -> Tree {
    let tree = Tree::new(name);
    tree.write(
        "crates/tool/src/main.rs",
        "mod consumer;\nmod direct;\nmod facade;\nmod inline;\nfn main() {}\n",
    );
    tree.write(FACADE, &facade(bound_to));
    tree.write(A, "pub(crate) fn answer() -> u32 {\n    1\n}\n");
    let b = if b_reads {
        format!(
            "pub(crate) fn answer() -> u32 {{\n    \"{}\".len() as u32\n}}\n",
            core("src")
        )
    } else {
        "pub(crate) fn answer() -> u32 {\n    2\n}\n".to_string()
    };
    tree.write(B, &b);
    tree.write(LEAF, "pub(crate) fn shared() -> u32 {\n    3\n}\n");
    tree.write(CONSUMER, &test_of("assert!(crate::facade::answer() > 0);"));
    tree.write(
        INLINE,
        &test_of("assert!(!crate::facade::scanned().is_empty());"),
    );
    tree.write(
        DIRECT,
        &test_of("assert_eq!(crate::facade::leaf::shared(), 3);"),
    );
    tree.write(&core("src/lib.rs"), "pub mod x;\n");
    tree.write(&core("src/x.rs"), "pub fn x() {}\n");
    tree
}

fn facade(bound_to: &str) -> String {
    format!(
        "pub(crate) mod a;\npub(crate) mod b;\npub(crate) mod leaf;\n\
         pub(crate) use {bound_to}::answer;\n\
         pub(crate) fn scanned() -> &'static str {{\n    \"{}\"\n}}\n",
        core("src")
    )
}

fn reach_of(g: &super::Graph, file: &str) -> super::Reach {
    g.reach(&[file.to_string()])
}

/// The binding alone repointed, the named definition changed, and the
/// key that has to move for either: the reader is owed each way, and by
/// nothing the binding does not name.
#[test]
fn a_reader_through_a_binding_is_owed_by_the_binding_and_by_what_it_names() {
    let tree = laid_out("binding", "a", false);
    let g = build(tree.root()).expect("the graph of the probe tree");
    assert!(g.unresolved.is_empty(), "{:?}", g.unresolved);
    assert_eq!(
        reach_of(&g, FACADE).get(CONSUMER),
        Some(&Carried::Whole),
        "the binding moved"
    );
    assert_eq!(
        reach_of(&g, A).get(CONSUMER),
        Some(&Carried::Whole),
        "the definition moved"
    );
    assert!(
        !reach_of(&g, B).contains_key(CONSUMER),
        "nothing the binding names moved"
    );
    let inputs = g.inputs(&[CONSUMER.to_string()]);
    assert!(
        inputs.contains(FACADE) && inputs.contains(A),
        "the reader's key holds the binding and what it names: {inputs:?}"
    );

    // The binding switched to the other module's `answer`: the reader is
    // owed by the switch (a change to the facade) and, from then on, by
    // the module now named.
    tree.write(FACADE, &facade("b"));
    let g = build(tree.root()).expect("the graph after the switch");
    assert!(reach_of(&g, FACADE).contains_key(CONSUMER));
    assert!(reach_of(&g, B).contains_key(CONSUMER), "now bound to b");
    assert!(
        !reach_of(&g, A).contains_key(CONSUMER),
        "no longer bound to a"
    );
    let inputs = g.inputs(&[CONSUMER.to_string()]);
    assert!(inputs.contains(FACADE) && inputs.contains(B), "{inputs:?}");
}

/// A product file the facade reads in a function of its own reaches the
/// reader of that function, and neither the reader through the binding
/// (a `pub use` passes no data) nor the reader of a leaf by its own path.
/// Bound to a module that reads the product, the reader through the
/// binding is reached — through what it names, not through the binding.
#[test]
fn a_product_file_the_facade_reads_passes_through_no_binding() {
    let tree = laid_out("product", "a", false);
    let g = build(tree.root()).expect("the graph of the probe tree");
    let reach = reach_of(&g, &core("src/x.rs"));
    assert_eq!(
        reach.get(FACADE),
        Some(&Carried::AsProductFile),
        "{reach:?}"
    );
    assert_eq!(
        reach.get(INLINE),
        Some(&Carried::AsProductFile),
        "{reach:?}"
    );
    for spared in [CONSUMER, LEAF, DIRECT] {
        assert!(!reach.contains_key(spared), "{spared}: {reach:?}");
    }
    // Nor does the product directory enter the bound reader's key, while
    // the facade's text does.
    let inputs = g.inputs(&[CONSUMER.to_string()]);
    assert!(
        inputs.contains(FACADE) && !inputs.iter().any(|f| f.starts_with(&core(""))),
        "{inputs:?}"
    );
    assert!(
        reach_of(&g, LEAF).contains_key(DIRECT),
        "the leaf's own change"
    );

    let tree = laid_out("product-named", "b", true);
    let g = build(tree.root()).expect("the graph with the product read behind the name");
    let reach = reach_of(&g, &core("src/x.rs"));
    assert_eq!(reach.get(B), Some(&Carried::AsProductFile), "{reach:?}");
    assert_eq!(
        reach.get(CONSUMER),
        Some(&Carried::AsProductFile),
        "{reach:?}"
    );
}

/// A crate root binds names for other crates: a reader of one is owed by
/// the root's `pub use` and by the definition, and the root stays no
/// helper's home (`complaints`).
#[test]
fn a_crate_roots_binding_owes_its_readers_and_makes_it_no_hub() {
    let tree = Tree::new("root");
    tree.write(&core("src/lib.rs"), "pub mod x;\npub use x::answer;\n");
    tree.write(&core("src/x.rs"), "pub fn answer() -> u32 {\n    1\n}\n");
    tree.write(
        &app(&format!("src/{}", "main.rs")),
        "mod uses;\nfn main() {}\n",
    );
    let uses = app("src/uses.rs");
    tree.write(&uses, &test_of("assert_eq!(platitude_core::answer(), 1);"));
    let g = build(tree.root()).expect("the graph of the probe tree");
    assert!(g.unresolved.is_empty(), "{:?}", g.unresolved);
    let root = core("src/lib.rs");
    assert_eq!(reach_of(&g, &root).get(&uses), Some(&Carried::Whole));
    assert_eq!(
        reach_of(&g, &core("src/x.rs")).get(&uses),
        Some(&Carried::Whole)
    );
    let read: Vec<String> = complaints(&g)
        .into_iter()
        .filter(|line| line.contains("is read by"))
        .collect();
    assert!(read.is_empty(), "{read:?}");
    let inputs = g.inputs(&[uses]);
    assert!(
        inputs.contains(&root) && inputs.contains(&core("src/x.rs")),
        "{inputs:?}"
    );
}
