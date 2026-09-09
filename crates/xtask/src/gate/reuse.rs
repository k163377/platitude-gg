//! The built graph, kept so that the next run over the same tree need not
//! read the sources again.
//!
//! [`super::graph::build`] opens every Rust and QML file of the workspace.
//! Warm that is a fifth of a second; behind a cargo build, which has just
//! written gigabytes through the machine's file cache, it is five and a
//! half — measured on Windows, reproducibly, as the first read after any
//! `cargo build` (internal-docs/反映前テストの機械化.md §実測). Every gate
//! pays it, and a landing pays it twice.
//!
//! What the graph is a function of: the sources, the set of files, and
//! the reader that walked them. The key is all three — the commit's root
//! tree object, which moves when any tracked byte or name does, and the
//! fingerprint of this very executable, which moves when the reader is
//! rebuilt. **Nothing is kept or reused for a tree with uncommitted or
//! untracked files**: the graph is read off the working tree and the
//! commit's tree would not be describing it. The gate itself refuses to
//! run over such a tree anyway; a `--dry-run` there simply reads the
//! sources.
//!
//! Only what outlives the build is kept — the edges, their reverse, the
//! modules, and the paths that resolved nowhere. The rest of [`Graph`]
//! (the index, the re-exports, the crate roots) is scaffolding that
//! resolution uses and nothing reads afterwards; the round trip is tested
//! against the real tree's graph so that a field which stops being
//! scaffolding is caught here rather than in a selection.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::graph::{Graph, Module};
use crate::subprocess::{common_git_dir, git_query};

/// The format the file is written in. A reader that does not know this
/// number reads nothing rather than guessing.
const VERSION: &str = "graph-cache 2";

/// How many built graphs a repository keeps. One is a few hundred
/// kilobytes, and what is ever reached for is the commit at hand and the
/// one before it — the rest are here so that a seat rebasing onto a tree
/// another seat has already read finds it read.
const KEEP: usize = 24;

/// What a kept graph is a function of: the tree it was read from and the
/// program that read it. `None` when the working tree holds anything the
/// commit does not, in which case the tree is not what the key names.
pub(crate) fn key(dir: &Path) -> Option<String> {
    let here = dir.display().to_string();
    if !git_query(&here, &["status", "--porcelain", "--untracked-files=all"])?.is_empty() {
        return None;
    }
    let tree = git_query(&here, &["rev-parse", "HEAD^{tree}"])?;
    let exe = std::env::current_exe().ok()?;
    let read = std::fs::metadata(&exe).ok()?;
    let stamped = read
        .modified()
        .ok()
        .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |since| since.as_nanos());
    Some(format!(
        "{:016x}",
        fnv(&format!("{VERSION} {tree} {} {stamped}", read.len()))
    ))
}

/// FNV-64a, as the step stamps use: a name for a set of bytes, not a
/// guard against anyone choosing them.
fn fnv(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash
}

/// Where the graphs of this repository live — beside its `.git`, so every
/// seat of it reads the same ones.
fn shelf(dir: &Path) -> Option<PathBuf> {
    let common = common_git_dir(&dir.display().to_string())?;
    Some(PathBuf::from(common).join("pgg-gate").join("graphs"))
}

/// The graph kept under `key`, if one is.
pub(crate) fn load(dir: &Path, key: &str) -> Option<Graph> {
    let text = std::fs::read_to_string(shelf(dir)?.join(key)).ok()?;
    read(&text)
}

/// Keeps `graph` under `key` and takes away all but the newest [`KEEP`].
/// A graph nobody could write is not worth a red gate — the run has the
/// graph in hand either way.
pub(crate) fn keep(dir: &Path, key: &str, graph: &Graph) {
    let Some(shelf) = shelf(dir) else {
        return;
    };
    if std::fs::create_dir_all(&shelf).is_err() {
        return;
    }
    // Written whole under another name and moved into place: a reader
    // arriving mid-write would otherwise take half a graph for a whole
    // one, and half a graph is a selection with edges missing.
    let staging = shelf.join(format!("{key}.{}.part", std::process::id()));
    if std::fs::write(&staging, write(graph)).is_err() {
        let _ = std::fs::remove_file(&staging);
        return;
    }
    if std::fs::rename(&staging, shelf.join(key)).is_err() {
        let _ = std::fs::remove_file(&staging);
        return;
    }
    sweep(&shelf);
}

/// All but the newest [`KEEP`], by when each was last written.
fn sweep(shelf: &Path) {
    let Ok(entries) = std::fs::read_dir(shelf) else {
        return;
    };
    let mut kept: Vec<(std::time::SystemTime, PathBuf)> = entries
        .flatten()
        .filter(|entry| entry.path().extension().is_none())
        .filter_map(|entry| {
            let at = entry.metadata().ok()?.modified().ok()?;
            Some((at, entry.path()))
        })
        .collect();
    if kept.len() <= KEEP {
        return;
    }
    kept.sort();
    let over = kept.len() - KEEP;
    for (_, stale) in kept.into_iter().take(over) {
        let _ = std::fs::remove_file(stale);
    }
}

/// One record per line, fields tab-separated: `M` a module, `D` a file
/// and everything it reads, `U` a path that resolved nowhere. Tabs
/// because a workspace path can hold a space and never one of these.
fn write(graph: &Graph) -> String {
    let mut out = String::from(VERSION);
    out.push('\n');
    for (file, module) in &graph.modules {
        out.push_str(&format!(
            "M\t{file}\t{}\t{}\t{}\t{}\t{}\n",
            module.krate,
            module.path.join("::"),
            module.package,
            module.test_binary.as_deref().unwrap_or("-"),
            u8::from(module.has_tests),
        ));
    }
    for (from, to) in &graph.deps {
        out.push_str("D\t");
        out.push_str(from);
        for target in to {
            out.push('\t');
            out.push_str(target);
        }
        out.push('\n');
    }
    for (file, path) in &graph.unresolved {
        out.push_str(&format!("U\t{file}\t{path}\n"));
    }
    out.push_str(&format!("END\t{:016x}\n", fnv(&out)));
    out
}

/// The graph a [`write`] wrote, or `None` for anything else — a file from
/// a version that spelled it differently, or one cut short.
fn read(text: &str) -> Option<Graph> {
    // A complete record boundary is still a truncated graph. Verify the
    // entire payload before trusting any edges or absence of edges.
    let (payload, digest) = text.strip_suffix('\n')?.rsplit_once("END\t")?;
    if digest != format!("{:016x}", fnv(payload)) {
        return None;
    }
    let mut lines = payload.lines();
    if lines.next()? != VERSION {
        return None;
    }
    let mut graph = Graph::default();
    for line in lines {
        let mut fields = line.split('\t');
        match fields.next()? {
            "M" => {
                let file = fields.next()?.to_string();
                let krate = fields.next()?.to_string();
                let path = fields.next()?;
                let package = fields.next()?.to_string();
                let binary = fields.next()?;
                let has_tests = fields.next()? == "1";
                graph.modules.insert(
                    file,
                    Module {
                        krate,
                        path: if path.is_empty() {
                            Vec::new()
                        } else {
                            path.split("::").map(str::to_string).collect()
                        },
                        package,
                        test_binary: (binary != "-").then(|| binary.to_string()),
                        has_tests,
                    },
                );
            }
            "D" => {
                let from = fields.next()?.to_string();
                let to: BTreeSet<String> = fields.map(str::to_string).collect();
                for target in &to {
                    graph
                        .rdeps
                        .entry(target.clone())
                        .or_default()
                        .insert(from.clone());
                }
                graph.deps.insert(from, to);
            }
            "U" => {
                let file = fields.next()?.to_string();
                graph.unresolved.push((file, fields.next()?.to_string()));
            }
            _ => return None,
        }
    }
    Some(graph)
}

/// The graph of `dir`, from the shelf when one answers for this tree and
/// this reader, and read off the sources otherwise — kept on the way out.
/// Whether it came off the shelf is the second answer, for the record.
pub(crate) fn graph_of(dir: &Path) -> Result<(Graph, bool), String> {
    let key = key(dir);
    if let Some(key) = &key
        && let Some(graph) = load(dir, key)
    {
        return Ok((graph, true));
    }
    let graph = super::graph::build(dir)?;
    if let Some(key) = &key {
        keep(dir, key, &graph);
    }
    Ok((graph, false))
}

/// What a graph read back must answer the same as the one written: the
/// four things everything downstream asks it (`plan`, `deps`).
#[cfg(test)]
fn same(one: &Graph, other: &Graph) -> Result<(), String> {
    if one.deps != other.deps {
        return Err("the edges differ".into());
    }
    if one.rdeps != other.rdeps {
        return Err("the readers differ".into());
    }
    if one.unresolved != other.unresolved {
        return Err("the unresolved paths differ".into());
    }
    let named = |g: &Graph| -> std::collections::BTreeMap<String, String> {
        g.modules
            .iter()
            .map(|(file, m)| {
                (
                    file.clone(),
                    format!(
                        "{} {} {} {:?} {}",
                        m.krate,
                        m.path.join("::"),
                        m.package,
                        m.test_binary,
                        m.has_tests
                    ),
                )
            })
            .collect()
    };
    if named(one) != named(other) {
        return Err("the modules differ".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{VERSION, read, same, write};

    /// The tree's own graph, written and read back: every edge, every
    /// reader, every module and every unresolved path as it was. Against
    /// the real tree rather than a made-up graph, because what this has
    /// to survive is the shapes this workspace actually holds — a module
    /// at a crate root with an empty path, an integration binary, a
    /// directory node, a non-ASCII name.
    #[test]
    fn the_trees_graph_survives_the_shelf() {
        let root = crate::tree::workspace_root();
        let built = super::super::graph::build(&root).expect("the graph of this tree");
        let back = read(&write(&built)).expect("reads back");
        same(&built, &back).expect("the same graph");
    }

    #[test]
    fn a_file_from_another_version_or_cut_short_reads_as_nothing() {
        assert!(read("").is_none());
        assert!(read("graph-cache 0\n").is_none());
        assert!(read(&format!("{VERSION}\nM\tonly-a-file\n")).is_none());
        assert!(read(&format!("{VERSION}\nX\tsomething\n")).is_none());
        assert!(read(&format!("{VERSION}\n")).is_none());
    }

    #[test]
    fn missing_or_damaged_edges_never_read_as_a_complete_graph() {
        let mut graph = super::Graph::default();
        graph.deps.insert("reader".into(), ["input".into()].into());
        let text = write(&graph);
        assert!(read(&text).is_some());
        for (end, _) in text.char_indices() {
            assert!(
                read(&text[..end]).is_none(),
                "accepted prefix {end}: {:?}",
                &text[..end]
            );
        }
        assert!(read(&text.replace("input", "other")).is_none());
    }
}
