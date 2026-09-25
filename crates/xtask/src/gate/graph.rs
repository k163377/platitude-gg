//! The file dependency graph, and the reach of a change read off it
//! (反映前テストの機械化.md §依存木).
//!
//! Nodes are workspace-relative files; a directory node ends in `/`. An
//! edge `a -> b` says a reads b: a `use` path or an inline `crate::` /
//! `super::` / `self::` / `platitude_core::` / bare child-module path that
//! resolves into b's module (through `pub use` re-exports), a string
//! literal naming b's path, an insta snapshot b of a's tests, a QML type
//! name that is b's file, a QML mention of a `#[qobject]` type b defines, a
//! `CARGO_BIN_EXE_<name>` naming the binary b is the root of. A `mod x;`
//! declaration is not an edge, or the declaring crate root or mod.rs would
//! be the hub every file reaches through; for the same reason a crate root
//! defines nothing anybody names
//! (`the_crate_roots_have_no_readers_and_every_path_resolves`).
//!
//! The tests to run are the tests in the reverse closure of the changed
//! files, read by module path, which is what `cargo test`'s filter takes.
//!
//! Everything here over-approximates on purpose: a module is the unit, a
//! glob re-export lands on every file it could mean, and a path that stops
//! resolving early lands on the deepest module it did reach. The one
//! failure it guards against is an edge missing, so unresolvable paths are
//! counted and printed.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Clone, Debug)]
pub(crate) struct Module {
    /// `platitude_core`, `platitude_app`, `xtask`, or — for the binaries
    /// that are each a crate of their own — `platitude_core::it`,
    /// `platitude_core::pgg_todo_editor`.
    pub krate: String,
    /// Module path from the crate root, empty at the root.
    pub path: Vec<String>,
    /// The cargo package (`-p`).
    pub package: String,
    /// The `--test` name for an integration binary; None for the lib/bin.
    pub test_binary: Option<String>,
    /// Tests are defined in the file itself ([`defines_tests`]).
    pub has_tests: bool,
}

#[derive(Default)]
pub(crate) struct Graph {
    pub deps: BTreeMap<String, BTreeSet<String>>,
    pub rdeps: BTreeMap<String, BTreeSet<String>>,
    pub modules: BTreeMap<String, Module>,
    /// (krate, module path) -> file
    index: BTreeMap<(String, Vec<String>), String>,
    /// crate ident -> root file, for the crates another crate can name.
    crate_roots: BTreeMap<String, String>,
    /// (package, bin name) -> the bin's root file. Nothing `use`s a
    /// binary, so this is what a `CARGO_BIN_EXE_<name>` resolves through.
    binaries: BTreeMap<(String, String), String>,
    /// file -> the child modules it declares, which its own code names bare.
    children: BTreeMap<String, BTreeSet<String>>,
    /// file -> exported name -> the path it re-exports.
    reexports: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    /// file -> the paths it re-exports by glob.
    globs: BTreeMap<String, Vec<Vec<String>>>,
    pub unresolved: Vec<(String, String)>,
    /// `#[qobject]` type names the app defines -> file.
    app_types: BTreeMap<String, String>,
}

/// How much of a change a file is handed.
///
/// The product reaches the tooling only as files a tool reads off the disk
/// (nothing outside the product compiles it), and an integration binary
/// points its tool at a sandbox it laid out itself, so a product change
/// handed on that way stops at an integration binary's modules. Everything
/// else is handed on whole, including a non-product file a binary reads
/// off the real tree (the hook script `tests/gate` copies into its
/// sandbox). Without the stop, every app or core change would owe the
/// gate's sandbox tests on both sides, since the census reads the app.
///
/// The selection reads the difference too: a tool file handed
/// `AsProductFile` did not change, so a step that file's name selects
/// (the verbs through `verify` / `demo`, `qmltest` through its runner,
/// the wedge through its record) is owed only by a file handed `Whole`,
/// while the steps its code is built into (clippy, its own tests) are
/// owed either way — a test of the tool may read the data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Carried {
    /// A product file, as a tool that does not build it reads it.
    AsProductFile,
    /// The change itself, or code built from it.
    Whole,
}

type Handed = (String, Carried);

/// The reach of a change: each file under the most it was handed.
pub(crate) type Reach = BTreeMap<String, Carried>;

/// Whether a file is part of the product — the two crates the app is
/// built from, tests and all.
fn is_product(file: &str) -> bool {
    // Spelled in pieces: a whole path here would be read as this file
    // reading the product.
    ["platitude-app", "platitude-core"]
        .iter()
        .any(|name| file.starts_with(&format!("crates/{name}/")))
}

/// Whether `reader` takes `file` as data off the disk: a product file
/// read by a file outside the product ([`Carried::AsProductFile`]).
fn as_data(file: &str, reader: &str) -> bool {
    is_product(file) && !is_product(reader)
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Markdown is checked by the always-run harness checks alone.
pub(crate) fn is_markdown(file: &str) -> bool {
    !file.ends_with('/')
        && Path::new(file)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
}

impl Graph {
    fn edge(&mut self, from: &str, to: &str) {
        if from == to || is_markdown(to) {
            return;
        }
        self.deps
            .entry(from.to_string())
            .or_default()
            .insert(to.to_string());
    }

    /// Everything that reads one of `changed`, transitively, plus
    /// `changed` itself, each under the most it was handed. A QML node
    /// hands on only to QML readers and to directory nodes (a Rust test
    /// reading the QML tree off the disk), and a product file read off
    /// the disk stops at an integration binary ([`Carried`]).
    pub(crate) fn reach(&self, changed: &[String]) -> Reach {
        let mut reach = Reach::new();
        for (file, carried) in self.walk(changed).into_keys() {
            let most = reach.entry(file).or_insert(carried);
            *most = (*most).max(carried);
        }
        reach
    }

    /// How `target` got into the reach of `changed`: the chain of readers
    /// from a changed file to it, when there is one.
    pub(crate) fn why(&self, changed: &[String], target: &str) -> Option<Vec<String>> {
        let handed = self.walk(changed);
        let mut at = handed
            .keys()
            .filter(|(file, _)| file == target)
            .max_by_key(|(_, carried)| *carried)?
            .clone();
        let mut chain = vec![at.0.clone()];
        while let Some(Some(from)) = handed.get(&at) {
            chain.push(from.0.clone());
            at = from.clone();
        }
        chain.reverse();
        Some(chain)
    }

    /// The reach, breadth first: each file under the most it was handed,
    /// against the file that handed it on (none for a changed file).
    fn walk(&self, changed: &[String]) -> BTreeMap<Handed, Option<Handed>> {
        let mut handed: BTreeMap<Handed, Option<Handed>> = BTreeMap::new();
        let mut most: BTreeMap<String, Carried> = BTreeMap::new();
        let mut queue: std::collections::VecDeque<Handed> = std::collections::VecDeque::new();
        for file in changed {
            if most.insert(file.clone(), Carried::Whole).is_none() {
                handed.insert((file.clone(), Carried::Whole), None);
                queue.push_back((file.clone(), Carried::Whole));
            }
        }
        while let Some((file, carried)) = queue.pop_front() {
            let Some(readers) = self.rdeps.get(&file) else {
                continue;
            };
            for reader in readers {
                let Some(next) = self.hands_on(&file, carried, reader) else {
                    continue;
                };
                if most.get(reader).is_some_and(|had| *had >= next) {
                    continue;
                }
                most.insert(reader.clone(), next);
                handed.insert((reader.clone(), next), Some((file.clone(), carried)));
                queue.push_back((reader.clone(), next));
            }
        }
        handed
    }

    /// What `reader` is handed of a change `file` carries, if anything.
    fn hands_on(&self, file: &str, carried: Carried, reader: &str) -> Option<Carried> {
        if file.ends_with(".qml") && !(reader.ends_with(".qml") || reader.ends_with('/')) {
            return None;
        }
        let carried = if carried == Carried::Whole && as_data(file, reader) {
            Carried::AsProductFile
        } else {
            carried
        };
        (carried == Carried::Whole || !self.sandboxed(reader)).then_some(carried)
    }

    /// Whether `file` is a module of an integration binary — one that
    /// points its tool at a sandbox it laid out itself.
    fn sandboxed(&self, file: &str) -> bool {
        self.modules
            .get(file)
            .is_some_and(|module| module.test_binary.is_some())
    }

    /// What `files` read, transitively — the inputs a cache key for tests
    /// selected in `files` has to name: what a change would have to touch
    /// to reach them ([`Graph::reach`]). So [`Carried`]'s stop is read
    /// back: past an integration binary's module, a product file a tool
    /// takes as data off the disk is no input (the binary points the tool
    /// at its own sandbox), while the tool's code and a file outside the
    /// product it reads off the real tree are. The QML rule is not read
    /// back: a Rust file naming a QML file keeps it in its key.
    pub(crate) fn inputs(&self, files: &[String]) -> BTreeSet<String> {
        // Each file found, with whether a sandboxed module stands between
        // it and a start (`guarded`); found without one, it stays found.
        type Queue = std::collections::VecDeque<(String, bool)>;
        fn take(found: &mut BTreeMap<String, bool>, queue: &mut Queue, file: &str, guarded: bool) {
            if found.get(file).is_some_and(|had| !*had || guarded) {
                return;
            }
            found.insert(file.to_string(), guarded);
            queue.push_back((file.to_string(), guarded));
        }
        let mut found: BTreeMap<String, bool> = BTreeMap::new();
        let mut queue = Queue::new();
        for file in files {
            take(&mut found, &mut queue, file, self.sandboxed(file));
        }
        while let Some((reader, guarded)) = queue.pop_front() {
            let Some(read) = self.deps.get(&reader) else {
                continue;
            };
            for file in read {
                if guarded && as_data(file, &reader) {
                    continue;
                }
                take(
                    &mut found,
                    &mut queue,
                    file,
                    guarded || self.sandboxed(file),
                );
            }
        }
        found.into_keys().collect()
    }
}

pub(crate) fn build(root: &Path) -> Result<Graph, String> {
    let mut g = Graph::default();
    let crates = root.join("crates");
    let mut packages: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(&crates).map_err(|e| format!("{}: {e}", crates.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.path().is_dir() {
            packages.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    packages.sort();
    for package in &packages {
        roots_of(root, &crates.join(package), package, &mut g)?;
    }
    let files: Vec<String> = g.modules.keys().cloned().collect();
    let mut texts: BTreeMap<String, (String, String)> = BTreeMap::new();
    for file in &files {
        let raw = std::fs::read_to_string(root.join(file)).map_err(|e| format!("{file}: {e}"))?;
        let code = strip_comments(&raw);
        // Spelled in pieces: a whole path here would be read as this
        // file reading everything under the app.
        if file.starts_with(&format!("crates/{}/", "platitude-app")) {
            for name in qobject_names(&code) {
                g.app_types.entry(name).or_insert_with(|| file.clone());
            }
        }
        let (named, globs) = reexports_in(&code);
        g.reexports.insert(file.clone(), named);
        g.globs.insert(file.clone(), globs);
        if let Some(module) = g.modules.get_mut(file) {
            module.has_tests = defines_tests(&code);
        }
        texts.insert(file.clone(), (raw, code));
    }
    for file in &files {
        let (raw, code) = &texts[file];
        let bare = bare_roots(&g, file, code);
        for segments in paths_in(code, &g.crate_roots, &bare) {
            let targets = resolve(&g, file, &segments, 0);
            if targets.is_empty() {
                g.unresolved.push((file.clone(), segments.join("::")));
            }
            for target in targets {
                g.edge(file, &target);
            }
        }
        // An integration binary's strings lay out its sandbox; they read
        // nothing of this tree.
        let bodies = string_bodies(raw);
        if g.modules.get(file).is_some_and(|m| m.test_binary.is_none()) {
            for target in literal_paths(root, file, &bodies) {
                g.edge(file, &target);
            }
        }
        // cargo hands a test the built bin by name, within the package only.
        let package = g
            .modules
            .get(file)
            .map(|m| m.package.clone())
            .unwrap_or_default();
        for name in bin_exe_names(&bodies) {
            match g.binaries.get(&(package.clone(), name.clone())).cloned() {
                Some(bin) => g.edge(file, &bin),
                None => g
                    .unresolved
                    .push((file.clone(), format!("{BIN_EXE}{name}"))),
            }
        }
    }
    snapshots(root, &mut g)?;
    qml(root, &mut g)?;
    directories(root, &mut g)?;
    let deps = g.deps.clone();
    for (from, to) in deps {
        for target in to {
            g.rdeps.entry(target).or_default().insert(from.clone());
        }
    }
    Ok(g)
}

/// Whether the file defines tests of its own: a test attribute of any
/// runtime (`#[test]`, `#[tokio::test]`, `#[tokio::test(flavor = …)]`)
/// or an inline `mod tests {`. A `#[cfg(test)]` is not one — the tests
/// it guards may be a declared sibling's.
fn defines_tests(code: &str) -> bool {
    code.contains("mod tests {")
        || code.lines().any(|line| {
            let line = line.trim_start();
            line.starts_with("#[") && (line.ends_with("test]") || line.contains("test("))
        })
}

/// The words `code` may start a path with bare, and what each stands
/// for: the file's own child modules (`stash::x` after `mod stash;`
/// reads as `self::stash::x`), and the children and re-exports of every
/// module it glob-imports (`use super::*;` puts the parent's `refs` in
/// scope, so a following `refs::x` reads as `super::refs::x`).
fn bare_roots(g: &Graph, file: &str, code: &str) -> BTreeMap<String, Vec<String>> {
    let mut bare: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for child in g.children.get(file).into_iter().flatten() {
        bare.insert(child.clone(), vec!["self".to_string(), child.clone()]);
    }
    let inline = inline_modules(code);
    let mut from = 0;
    while let Some(at) = code[from..].find("::*") {
        let at = from + at;
        from = at + 3;
        let head = &code[..at];
        let start = head
            .rfind(|c: char| !(is_ident(c) || c == ':'))
            .map_or(0, |p| p + 1);
        let mut segments: Vec<String> = head[start..]
            .split("::")
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        let Some(first) = segments.first_mut() else {
            continue;
        };
        if first == "super" && inline.iter().any(|(open, close)| at > *open && at < *close) {
            *first = "self".to_string();
        }
        for target in resolve(g, file, &segments, 0) {
            for child in g.children.get(&target).into_iter().flatten() {
                let mut path = segments.clone();
                path.push(child.clone());
                bare.entry(child.clone()).or_insert(path);
            }
            for name in g
                .reexports
                .get(&target)
                .into_iter()
                .flat_map(BTreeMap::keys)
            {
                let mut path = segments.clone();
                path.push(name.clone());
                bare.entry(name.clone()).or_insert(path);
            }
        }
    }
    bare
}

/// Every `.rs` directly under `dir`, sorted.
fn rust_files_in(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().is_some_and(|e| e == "rs"))
        .collect();
    files.sort();
    files
}

/// The crate roots of one package, each walked into the module index:
/// the lib, the bins (`src/main.rs` and each `src/bin/*.rs`, a crate
/// apiece), and the integration binaries — every file directly under
/// tests/ and every `tests/<name>/main.rs` (core's `it`, the gate's own).
fn roots_of(root: &Path, dir: &Path, package: &str, g: &mut Graph) -> Result<(), String> {
    let ident = package.replace('-', "_");
    // The root files, spelled in pieces: a whole path in a string here
    // would be read as this file reading every crate root.
    let src = dir.join("src");
    let lib = src.join("lib.rs");
    if lib.is_file() {
        g.crate_roots.insert(ident.clone(), rel(root, &lib));
    }
    let main = src.join("main.rs");
    if main.is_file() {
        g.binaries
            .insert((package.to_string(), package.to_string()), rel(root, &main));
    }
    let mut roots = vec![(lib, ident.clone(), None), (main, ident.clone(), None)];
    for file in rust_files_in(&src.join("bin")) {
        let name = stem_of(&file.display().to_string());
        g.binaries
            .insert((package.to_string(), name.clone()), rel(root, &file));
        // Under the package, as the test binaries are: a bin named after
        // its own package would otherwise take the lib's ident and its
        // place in the index, and every `platitude_core::…` in the tree
        // would resolve into the bin.
        roots.push((file, format!("{ident}::{}", name.replace('-', "_")), None));
    }
    for file in rust_files_in(&dir.join("tests")) {
        let stem = stem_of(&file.display().to_string());
        roots.push((file, format!("{ident}::{stem}"), Some(stem)));
    }
    if let Ok(entries) = std::fs::read_dir(dir.join("tests")) {
        for entry in entries.flatten() {
            let main = entry.path().join("main.rs");
            let name = entry.file_name().to_string_lossy().into_owned();
            if entry.path().is_dir() && main.is_file() {
                roots.push((main, format!("{ident}::{name}"), Some(name)));
            }
        }
    }
    for (file, krate, binary) in roots {
        if !file.is_file() {
            continue;
        }
        let module = Module {
            krate,
            path: Vec::new(),
            package: package.to_string(),
            test_binary: binary,
            has_tests: false,
        };
        walk_modules(root, &rel(root, &file), module, g)?;
    }
    Ok(())
}

/// Registers `file` as `module` and follows its `mod x;` declarations
/// into the module index only.
fn walk_modules(root: &Path, file: &str, module: Module, g: &mut Graph) -> Result<(), String> {
    g.index.insert(
        (module.krate.clone(), module.path.clone()),
        file.to_string(),
    );
    g.modules.insert(file.to_string(), module.clone());
    let text = std::fs::read_to_string(root.join(file)).map_err(|e| format!("{file}: {e}"))?;
    let path = Path::new(file);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy())
        .unwrap_or_default();
    let parent = path.parent().unwrap_or(Path::new(""));
    let owns_dir = matches!(name.as_ref(), "mod.rs" | "lib.rs" | "main.rs");
    let children_dir = if owns_dir {
        parent.to_path_buf()
    } else {
        parent.join(
            path.file_stem()
                .map(|s| s.to_string_lossy())
                .unwrap_or_default()
                .as_ref(),
        )
    };
    let mut explicit: Option<String> = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("#[path = \"") {
            explicit = rest.split('"').next().map(str::to_string);
            continue;
        }
        if line.starts_with("#[") {
            continue;
        }
        let Some(child) = mod_declaration(line) else {
            explicit = None;
            continue;
        };
        let candidates: Vec<std::path::PathBuf> = match explicit.take() {
            Some(named) => vec![lexical(&parent.join(named))],
            None => vec![
                children_dir.join(format!("{child}.rs")),
                children_dir.join(child).join("mod.rs"),
            ],
        };
        let Some(found) = candidates.into_iter().find(|c| root.join(c).is_file()) else {
            continue;
        };
        let found = found.display().to_string().replace('\\', "/");
        let mut path = module.path.clone();
        path.push(child.to_string());
        let sub = Module {
            path,
            ..module.clone()
        };
        g.children
            .entry(file.to_string())
            .or_default()
            .insert(child.to_string());
        walk_modules(root, &found, sub, g)?;
    }
    Ok(())
}

/// `path` with its `.` and `..` components folded away, so a file two
/// trees declare — the runner's `mod wait;` and the gate suite's
/// `#[path = "../../src/wait.rs"]` — is one node under one name, and a
/// change to it reaches both.
fn lexical(path: &Path) -> std::path::PathBuf {
    let mut folded = std::path::PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                folded.pop();
            }
            std::path::Component::CurDir => {}
            other => folded.push(other),
        }
    }
    folded
}

/// The name in `mod x;` / `pub mod x;` / `pub(crate) mod x;`, if the line
/// is one.
fn mod_declaration(line: &str) -> Option<&str> {
    let line = line.strip_suffix(';')?;
    let words: Vec<&str> = line.split_whitespace().collect();
    if !(words.first() == Some(&"mod") || words.first().is_some_and(|w| w.starts_with("pub"))) {
        return None;
    }
    let at = words.iter().rposition(|w| *w == "mod")?;
    let name = words.get(at + 1)?;
    (words.len() == at + 2 && name.chars().all(is_ident)).then_some(*name)
}

/// `text` without its comments (`//` to end of line, nested `/* */`) and
/// without the insides of its string literals (`"…"`, `r"…"`, `r#"…"#`),
/// so that neither reads as a path. The quotes and newlines stay, so
/// lines and spans still line up. Paths in strings are read off the raw
/// text ([`literal_paths`]).
pub(crate) fn strip_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;
    let mut depth = 0;
    while i < bytes.len() {
        if depth > 0 {
            if bytes[i..].starts_with(b"/*") {
                depth += 1;
                i += 2;
            } else if bytes[i..].starts_with(b"*/") {
                depth -= 1;
                i += 2;
            } else {
                if bytes[i] == b'\n' {
                    out.push('\n');
                }
                i += 1;
            }
            continue;
        }
        if bytes[i..].starts_with(b"/*") {
            depth = 1;
            i += 2;
        } else if bytes[i..].starts_with(b"//") {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else if bytes[i] == b'\'' {
            // A char literal ('x', '\n', '"') is skipped whole so its quote
            // cannot open a string; a lifetime tick is copied as it is.
            let end = char_literal_end(bytes, i);
            out.push_str(&text[i..end]);
            i = end;
        } else if let Some((body_start, end, hashes)) = string_literal(bytes, i) {
            out.push_str(&text[i..body_start]);
            for b in &bytes[body_start..end] {
                if *b == b'\n' {
                    out.push('\n');
                }
            }
            out.push('"');
            out.push_str(&"#".repeat(hashes));
            i = end + 1 + hashes;
        } else {
            let c = text[i..].chars().next().unwrap_or('\0');
            out.push(c);
            i += c.len_utf8();
        }
    }
    out
}

/// Where the char literal at `at` ends (one past its closing tick), or
/// `at + 1` when the tick is a lifetime's.
fn char_literal_end(bytes: &[u8], at: usize) -> usize {
    let rest = &bytes[at + 1..];
    let body = if rest.first() == Some(&b'\\') {
        // '\n', '\'', '\\', '\u{…}': everything up to the closing tick.
        rest.iter().skip(1).position(|b| *b == b'\'').map(|p| p + 2)
    } else {
        let c = std::str::from_utf8(rest)
            .ok()
            .and_then(|s| s.chars().next())
            .map_or(1, char::len_utf8);
        (rest.get(c) == Some(&b'\'')).then_some(c + 1)
    };
    match body {
        Some(len) => at + 1 + len,
        None => at + 1,
    }
}

/// Whether the literal opening at `at` carries a byte string's `b` — the
/// `b` of `b"…"` sits at `at - 1` and the `b` of `br"…"` at `at - 1` with
/// the `r` at `at`. A `b` that is the tail of an identifier is not one.
fn byte_prefix(bytes: &[u8], at: usize) -> bool {
    at > 0
        && bytes[at - 1] == b'b'
        && at
            .checked_sub(2)
            .is_none_or(|i| !is_ident(bytes[i] as char))
}

/// The string literal opening at `at`, if one does: (start of its body,
/// index of its closing quote, hashes of a raw string).
fn string_literal(bytes: &[u8], at: usize) -> Option<(usize, usize, usize)> {
    let (body_start, hashes) = if bytes[at] == b'"' {
        (at + 1, 0)
    } else if bytes[at] == b'r'
        && (at == 0 || !is_ident(bytes[at - 1] as char) || byte_prefix(bytes, at))
        && bytes[at + 1..]
            .first()
            .is_some_and(|b| *b == b'"' || *b == b'#')
    {
        let hashes = bytes[at + 1..].iter().take_while(|b| **b == b'#').count();
        if bytes.get(at + 1 + hashes) != Some(&b'"') {
            return None;
        }
        (at + 2 + hashes, hashes)
    } else {
        return None;
    };
    let mut i = body_start;
    while i < bytes.len() {
        if hashes == 0 && bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == b'"' {
            let closes = bytes[i + 1..].iter().take(hashes).all(|b| *b == b'#')
                && bytes[i + 1..].len() >= hashes;
            if closes {
                return Some((body_start, i, hashes));
            }
        }
        i += 1;
    }
    Some((body_start, bytes.len(), hashes))
}

/// Every path in `code` that starts from a root this graph knows: the
/// keyword roots, the crates another crate can name, and the words
/// `bare` says stand for a path ([`bare_roots`]). Braced groups are
/// split into one path each; a `self` or `*` item is the group's base.
fn paths_in(
    code: &str,
    crate_roots: &BTreeMap<String, String>,
    bare: &BTreeMap<String, Vec<String>>,
) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    let bytes = code.as_bytes();
    let inline = inline_modules(code);
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if !is_ident(c) || (i > 0 && is_ident(bytes[i - 1] as char)) {
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && is_ident(bytes[i] as char) {
            i += 1;
        }
        let word = &code[start..i];
        let keyword = matches!(word, "crate" | "super" | "self");
        let stands_for = bare.get(word);
        if !(keyword || stands_for.is_some() || crate_roots.contains_key(word))
            || !code[i..].starts_with("::")
        {
            continue;
        }
        // Inside an inline `mod tests { … }`, `super` is this very file.
        // One level deep is the shape the tree has; a deeper nest keeps its
        // first `super` for the file.
        let nested = inline
            .iter()
            .any(|(open, close)| start > *open && start < *close);
        let mut segments = if let Some(prefix) = stands_for {
            prefix.clone()
        } else if nested && word == "super" {
            vec!["self".to_string()]
        } else {
            vec![word.to_string()]
        };
        while code[i..].starts_with("::") {
            i += 2;
            if code[i..].starts_with('{') {
                let close = matching_brace(bytes, i);
                for item in group_items(&code[i + 1..close]) {
                    let mut path = segments.clone();
                    path.extend(item);
                    out.push(path);
                }
                i = close + 1;
                segments.clear();
                break;
            }
            let seg_start = i;
            while i < bytes.len() && is_ident(bytes[i] as char) {
                i += 1;
            }
            if i == seg_start {
                break;
            }
            segments.push(code[seg_start..i].to_string());
        }
        if !segments.is_empty() {
            out.push(segments);
        }
    }
    out
}

/// The byte spans of the inline `mod name { … }` blocks in `code`.
fn inline_modules(code: &str) -> Vec<(usize, usize)> {
    let bytes = code.as_bytes();
    let mut spans = Vec::new();
    let mut from = 0;
    while let Some(at) = code[from..].find("mod ") {
        let at = from + at;
        from = at + 4;
        let line_start = code[..at].rfind('\n').map_or(0, |n| n + 1);
        let head = code[line_start..at].trim();
        if !(head.is_empty() || head.starts_with("pub") || head.starts_with("#[")) {
            continue;
        }
        let rest = &code[at + 4..];
        let name_len = rest.chars().take_while(|c| is_ident(*c)).count();
        if name_len == 0 {
            continue;
        }
        let after = rest[name_len..].trim_start();
        if !after.starts_with('{') {
            continue;
        }
        let open = at + 4 + name_len + (rest[name_len..].len() - after.len());
        let close = matching_brace(bytes, open);
        spans.push((open, close));
        from = open + 1;
    }
    spans
}

/// The items of a `{…}` group, each as its own segments; nested groups
/// contribute their prefix and each inner item.
fn group_items(group: &str) -> Vec<Vec<String>> {
    let mut items = Vec::new();
    let mut depth = 0;
    let mut current = String::new();
    for c in group.chars() {
        match c {
            '{' => {
                depth += 1;
                current.push(c);
            }
            '}' => {
                depth -= 1;
                current.push(c);
            }
            ',' if depth == 0 => {
                items.push(std::mem::take(&mut current));
            }
            _ => current.push(c),
        }
    }
    items.push(current);
    let mut out = Vec::new();
    for item in items {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        if let Some(open) = item.find('{') {
            let close = item.rfind('}').unwrap_or(item.len());
            let prefix: Vec<String> = item[..open]
                .split("::")
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
            for inner in group_items(&item[open + 1..close]) {
                let mut path = prefix.clone();
                path.extend(inner);
                out.push(path);
            }
            continue;
        }
        let item = item.split(" as ").next().unwrap_or("").trim();
        let path: Vec<String> = item
            .split("::")
            .map(str::trim)
            .filter(|s| !s.is_empty() && *s != "self" && *s != "*")
            .map(str::to_string)
            .collect();
        out.push(path);
    }
    out
}

fn matching_brace(bytes: &[u8], open: usize) -> usize {
    let mut depth = 0;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return i;
                }
            }
            _ => {}
        }
        i += 1;
    }
    bytes.len().saturating_sub(1)
}

/// The `pub use` lines of a file: exported name -> path, and the glob
/// paths. The path is as written (relative to the file's module), for
/// [`resolve`] to follow from there.
fn reexports_in(code: &str) -> (BTreeMap<String, Vec<String>>, Vec<Vec<String>>) {
    let mut named = BTreeMap::new();
    let mut globs = Vec::new();
    let mut rest = code;
    // The earliest `pub use` or `pub(…) use`.
    let next = |text: &str| -> Option<usize> {
        let plain = text.find("pub use ");
        let scoped = text.find("pub(").filter(|&p| {
            text[p..]
                .find(')')
                .is_some_and(|close| text[p + close..].starts_with(") use "))
        });
        match (plain, scoped) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    };
    while let Some(at) = next(rest) {
        let stmt_start = rest[at..].find("use ").map(|u| at + u + 4).unwrap_or(at);
        let Some(end) = rest[stmt_start..].find(';') else {
            break;
        };
        let stmt = &rest[stmt_start..stmt_start + end];
        rest = &rest[stmt_start + end + 1..];
        let stmt: String = stmt.split_whitespace().collect::<Vec<_>>().join(" ");
        /// One exported item: its path as written, and its `as` name.
        type Export = (Vec<String>, Option<String>);
        let (prefix, items): (Vec<String>, Vec<Export>) = match stmt.find('{') {
            Some(open) => {
                let close = stmt.rfind('}').unwrap_or(stmt.len());
                let prefix: Vec<String> = stmt[..open]
                    .split("::")
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect();
                let mut items = Vec::new();
                for raw in split_top_level(&stmt[open + 1..close]) {
                    let raw = raw.trim();
                    let (path, alias) = match raw.split_once(" as ") {
                        Some((p, a)) => (p.trim(), Some(a.trim().to_string())),
                        None => (raw, None),
                    };
                    let path: Vec<String> = path
                        .split("::")
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string)
                        .collect();
                    items.push((path, alias));
                }
                (prefix, items)
            }
            None => {
                let (path, alias) = match stmt.split_once(" as ") {
                    Some((p, a)) => (p.trim(), Some(a.trim().to_string())),
                    None => (stmt.as_str(), None),
                };
                let path: Vec<String> = path
                    .split("::")
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect();
                (Vec::new(), vec![(path, alias)])
            }
        };
        for (item, alias) in items {
            let mut full = prefix.clone();
            full.extend(item);
            match full.last().map(String::as_str) {
                Some("*") => {
                    full.pop();
                    globs.push(full);
                }
                Some("self") => {
                    full.pop();
                    let name = alias.or_else(|| full.last().cloned());
                    if let Some(name) = name {
                        named.insert(name, full);
                    }
                }
                Some(last) => {
                    let name = alias.unwrap_or_else(|| last.to_string());
                    named.insert(name, full);
                }
                None => {}
            }
        }
    }
    (named, globs)
}

/// The comma-separated items of a group, braces flattened away — a
/// braced sub-group inside a `pub use` lands on the group's prefix file,
/// which is the over-approximating side.
fn split_top_level(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0;
    let mut current = String::new();
    for c in text.chars() {
        match c {
            '{' => {
                depth += 1;
                current.push(c);
            }
            '}' => {
                depth -= 1;
                current.push(c);
            }
            ',' if depth == 0 => out.push(std::mem::take(&mut current)),
            _ => current.push(c),
        }
    }
    if !current.trim().is_empty() {
        out.push(current);
    }
    out.into_iter()
        .map(|item| item.replace(['{', '}'], ""))
        .collect()
}

/// The files the path lands in: the deepest module along it that exists,
/// followed through `pub use` re-exports; a glob re-export lands on
/// every file it could mean, and on the re-exporting file itself.
fn resolve(g: &Graph, from: &str, segments: &[String], depth: usize) -> Vec<String> {
    if depth > 8 {
        return Vec::new();
    }
    let Some(module) = g.modules.get(from) else {
        return Vec::new();
    };
    let mut at = 1;
    let (krate, mut path) = match segments.first().map(String::as_str) {
        Some("crate") => (module.krate.clone(), Vec::new()),
        Some("self") => (module.krate.clone(), module.path.clone()),
        Some("super") => {
            let mut path = module.path.clone();
            if path.pop().is_none() {
                return Vec::new();
            }
            (module.krate.clone(), path)
        }
        Some(other) if g.crate_roots.contains_key(other) => (other.to_string(), Vec::new()),
        _ => return Vec::new(),
    };
    while segments.get(at).is_some_and(|s| s == "super") {
        if path.pop().is_none() {
            return Vec::new();
        }
        at += 1;
    }
    let Some(mut file) = g.index.get(&(krate.clone(), path.clone())).cloned() else {
        return Vec::new();
    };
    while let Some(seg) = segments.get(at) {
        let mut next = path.clone();
        next.push(seg.clone());
        match g.index.get(&(krate.clone(), next.clone())) {
            Some(found) => {
                file = found.clone();
                path = next;
                at += 1;
            }
            None => break,
        }
    }
    let Some(seg) = segments.get(at) else {
        return vec![file];
    };
    if let Some(target) = g.reexports.get(&file).and_then(|m| m.get(seg)) {
        let mut through = resolve(g, &file, &self_relative(g, target), depth + 1);
        if through.is_empty() {
            through.push(file);
        }
        return through;
    }
    let mut targets = vec![file.clone()];
    for glob in g.globs.get(&file).into_iter().flatten() {
        let mut through = glob.clone();
        through.push(seg.clone());
        targets.extend(resolve(g, &file, &self_relative(g, &through), depth + 1));
    }
    targets
}

/// A `pub use` path as written is relative to its own module unless it
/// starts from a root: `pub use error::GitError` means `self::error`.
fn self_relative(g: &Graph, path: &[String]) -> Vec<String> {
    match path.first().map(String::as_str) {
        Some("crate" | "self" | "super") => path.to_vec(),
        Some(first) if g.crate_roots.contains_key(first) => path.to_vec(),
        _ => {
            let mut out = vec!["self".to_string()];
            out.extend(path.iter().cloned());
            out
        }
    }
}

/// The string literals among `bodies` that name a file or directory of
/// the workspace, resolved against the file's own directory, its crate,
/// and the workspace root. A literal has to look like a path (a slash or
/// a dot) — `"crates"` alone would otherwise pull in the world.
fn literal_paths(root: &Path, file: &str, bodies: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let file_dir = Path::new(file).parent().unwrap_or(Path::new(""));
    let crate_dir: std::path::PathBuf = Path::new(file).iter().take(2).collect();
    let Ok(real_root) = root.canonicalize() else {
        return out;
    };
    for literal in bodies {
        let literal = literal.as_str();
        if literal.len() < 3
            || literal.len() > 200
            || !(literal.contains('/') || literal.contains('.'))
            || literal.contains(' ')
            || literal.starts_with("http")
            || literal.contains("::")
        {
            continue;
        }
        for base in [file_dir, crate_dir.as_path(), Path::new("")] {
            let candidate = root.join(base).join(literal);
            if !candidate.exists() {
                continue;
            }
            let Ok(real) = candidate.canonicalize() else {
                continue;
            };
            let Ok(inside) = real.strip_prefix(&real_root) else {
                continue;
            };
            let mut name = inside.display().to_string().replace('\\', "/");
            if name.starts_with("target") || name.is_empty() {
                continue;
            }
            // The census and the tier table are plan inputs no test opens
            // (sandboxes lay out their own). As edges they would enter the
            // keys of every test their readers reach, so a landing's census
            // commit would rerun them all to the same answer.
            if name == super::census::FILE || name == super::tiers::FILE {
                continue;
            }
            if real.is_dir() {
                name.push('/');
                // A file does not read the directory it lives in: that
                // edge is the whole-tree answer, not a dependency, and it
                // is what `../` or a prefix test like
                // `starts_with("crates/")` resolves to.
                if file.starts_with(&name) {
                    continue;
                }
            }
            out.push(name);
            break;
        }
    }
    out
}

/// The prefix of the variable cargo sets per bin of a package, holding the
/// path of the built executable.
const BIN_EXE: &str = "CARGO_BIN_EXE_";

/// The bins these string literals shoot: the `<name>` of every
/// `CARGO_BIN_EXE_<name>` among them, however the file reads the
/// variable (`env!`, `option_env!`, `std::env::var`). The prefix bare,
/// as this file spells it, names no bin and is dropped.
fn bin_exe_names(bodies: &[String]) -> Vec<String> {
    bodies
        .iter()
        .filter_map(|body| body.strip_prefix(BIN_EXE))
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect()
}

/// The bodies of the string literals in `text`, read with the same
/// tokenizer [`strip_comments`] uses: a `"` in a comment or a `'"'` char
/// literal opens nothing, and a `\"` closes nothing — pairing quotes by
/// `find` would read the rest of the file inside out after either.
fn string_bodies(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    let mut depth = 0;
    while i < bytes.len() {
        if depth > 0 {
            if bytes[i..].starts_with(b"/*") {
                depth += 1;
                i += 2;
            } else if bytes[i..].starts_with(b"*/") {
                depth -= 1;
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }
        if bytes[i..].starts_with(b"/*") {
            depth = 1;
            i += 2;
        } else if bytes[i..].starts_with(b"//") {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else if bytes[i] == b'\'' {
            i = char_literal_end(bytes, i);
        } else if let Some((body_start, end, hashes)) = string_literal(bytes, i) {
            // A byte string is content, not a path: nothing opens the
            // `b"../"` a generator writes (`corpus::shape::content_into`),
            // and as a path it would make its file a reader of every
            // source beside it.
            if !byte_prefix(bytes, i) {
                out.push(text[body_start..end].to_string());
            }
            i = end + 1 + hashes;
        } else {
            i += text[i..].chars().next().map_or(1, char::len_utf8);
        }
    }
    out
}

/// The type each `#[qobject]` block is for: the `impl X` under it.
fn qobject_names(code: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut armed = false;
    for line in code.lines() {
        let line = line.trim();
        if line.starts_with("#[qobject") {
            armed = true;
            continue;
        }
        if !armed || line.starts_with("#[") || line.is_empty() {
            continue;
        }
        armed = false;
        if let Some(rest) = line.strip_prefix("impl ") {
            let name: String = rest.chars().take_while(|c| is_ident(*c)).collect();
            if !name.is_empty() {
                out.push(name);
            }
        }
    }
    out
}

/// An insta snapshot belongs to the module its name spells:
/// `<crate>__<module>__…__<test>.snap`.
fn snapshots(root: &Path, g: &mut Graph) -> Result<(), String> {
    let mut snaps = Vec::new();
    collect(root, &root.join("crates"), "snap", &mut snaps)?;
    for snap in snaps {
        let stem = Path::new(&snap)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let parts: Vec<&str> = stem.split("__").collect();
        if parts.len() < 3 {
            continue;
        }
        let krate = parts[0].to_string();
        let mut path: Vec<String> = parts[1..parts.len() - 1]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let owner = loop {
            if let Some(file) = g.index.get(&(krate.clone(), path.clone())) {
                break Some(file.clone());
            }
            if path.pop().is_none() {
                break None;
            }
        };
        if let Some(owner) = owner {
            g.edge(&owner, &snap);
        }
    }
    Ok(())
}

/// QML: a capitalised word that is another QML file's name is a type
/// reference, and one that is a `#[qobject]` type is a model reference.
fn qml(root: &Path, g: &mut Graph) -> Result<(), String> {
    let files = qml_files(root)?;
    let by_name: BTreeMap<String, String> = files.iter().map(|f| (stem_of(f), f.clone())).collect();
    for file in &files {
        let raw = std::fs::read_to_string(root.join(file)).map_err(|e| format!("{file}: {e}"))?;
        let code = strip_comments(&raw);
        let mut targets = BTreeSet::new();
        let bytes = code.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let c = bytes[i] as char;
            if !c.is_ascii_uppercase() || (i > 0 && is_ident(bytes[i - 1] as char)) {
                i += 1;
                continue;
            }
            let start = i;
            while i < bytes.len() && is_ident(bytes[i] as char) {
                i += 1;
            }
            let word = &code[start..i];
            if let Some(target) = by_name.get(word) {
                targets.insert(target.clone());
            } else if let Some(target) = g.app_types.get(word) {
                targets.insert(target.clone());
            }
        }
        for target in targets {
            g.edge(file, &target);
        }
    }
    Ok(())
}

/// Every QML file of the app, product and harness alike, as workspace
/// paths — the set the census can name, edges or none.
pub(crate) fn qml_files(root: &Path) -> Result<Vec<String>, String> {
    let mut files = Vec::new();
    collect(
        root,
        &root.join("crates/platitude-app/src"),
        "qml",
        &mut files,
    )?;
    Ok(files)
}

/// A file's name without directory or extension — what a QML type is
/// called, and what the census records.
pub(crate) fn stem_of(file: &str) -> String {
    Path::new(file)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// A directory node reads every file under it, so a reader of the
/// directory is a reader of each.
fn directories(root: &Path, g: &mut Graph) -> Result<(), String> {
    // Each named directory once, however many files name it: the walk is
    // of the whole subtree, and `crates/` alone is every source.
    let dirs: BTreeSet<String> = g
        .deps
        .values()
        .flatten()
        .filter(|t| t.ends_with('/'))
        .cloned()
        .collect();
    for dir in dirs {
        let mut files = Vec::new();
        collect(root, &root.join(&dir), "", &mut files)?;
        for file in files {
            g.edge(&dir, &file);
        }
    }
    Ok(())
}

/// What the graph says is wrong with the tree it was read from: a path that
/// resolves nowhere (an edge not drawn, so every selection through it is
/// short), and a crate root somebody reads (the hub every change reaches
/// everything through — .claude/rules/structure.md §分割「クレート root」).
///
/// Neither is the change's fault nor a step's to answer, so the gate reads
/// this on every run; a test would sit in only some selections.
pub(crate) fn complaints(g: &Graph) -> Vec<String> {
    let mut out = Vec::new();
    for (file, path) in g.unresolved.iter().take(10) {
        out.push(format!("{file} names {path}, which resolves nowhere"));
    }
    // Spelled in pieces: a whole path in a string here would be read as
    // this very file reading the root.
    let roots = [
        ("platitude-core", "src", "lib.rs"),
        ("platitude-app", "src", "main.rs"),
        ("xtask", "src", "main.rs"),
        ("platitude-core", "tests/it", "main.rs"),
    ];
    for (package, dir, name) in roots {
        let file = format!("crates/{package}/{dir}/{name}");
        if !g.modules.contains_key(&file) {
            out.push(format!("{file} is not in the graph"));
            continue;
        }
        let readers: Vec<&String> = g
            .rdeps
            .get(&file)
            .into_iter()
            .flatten()
            // A directory node reads everything under it, the
            // `*_tests.rs` siblings a root declares read its scope with
            // `use super::*`, and an integration binary names files in
            // its fixtures — none is a helper on the root.
            .filter(|r| !r.ends_with('/') && !r.ends_with("_tests.rs"))
            .filter(|r| g.modules.get(*r).is_none_or(|m| m.test_binary.is_none()))
            .collect();
        if !readers.is_empty() {
            out.push(format!(
                "{file} is read by {readers:?}: a crate root holds declarations and re-exports \
                 only, or every change reaches everything through it"
            ));
        }
    }
    out
}

/// Every file under `dir` with `extension` (any, when empty), as
/// workspace-relative paths.
pub(crate) fn collect(
    root: &Path,
    dir: &Path,
    extension: &str,
    out: &mut Vec<String>,
) -> Result<(), String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "target" || name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            collect(root, &path, extension, out)?;
        } else if extension.is_empty() || path.extension().is_some_and(|e| e == extension) {
            out.push(rel(root, &path));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
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
        let roots: BTreeMap<String, String> =
            [("platitude_core".to_string(), "x".to_string())].into();
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
        let code =
            strip_comments("use crate::a; // see crate::b\n/* crate::c\n */ crate::d::e();\n");
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
        let reach = |changed: &[&String]| {
            g.reach(&changed.iter().map(|f| (*f).clone()).collect::<Vec<_>>())
        };
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
}
