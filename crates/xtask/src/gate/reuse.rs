//! The built graph, kept so that the next run over the same tree need not
//! read the sources again (internal-docs/反映前テストの機械化.md §実測).
//!
//! The key is the commit's root tree object and the fingerprint of this
//! executable's bytes: size and mtime cannot establish a reader's
//! contents. Nothing is kept or reused for a tree with uncommitted or
//! untracked files: the graph is read off the working tree, which the
//! commit's tree would not be describing.
//!
//! Only what outlives the build is kept — the edges, their reverse, the
//! modules, and the paths that resolved nowhere; the rest of [`Graph`] is
//! scaffolding resolution uses. The round trip is tested against the real
//! tree's graph, so a field that stops being scaffolding is caught.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::graph::{Graph, Module};
use crate::subprocess::{common_git_dir, git_query};

/// The format the file is written in. A reader that does not know this
/// number reads nothing.
const VERSION: &str = "graph-cache 4";

/// How many built graphs a repository keeps: more than the commit at hand
/// and its parent, so a seat rebasing onto a tree another seat has read
/// finds it read.
const KEEP: usize = 24;

/// Land removes its running image from the build slot before rebuilding.
/// Keep that reader's identity, including a failed read, so a replacement
/// executable at the same path can never name this process's graph.
static READER: std::sync::OnceLock<Result<u64, String>> = std::sync::OnceLock::new();

fn reader() -> Result<u64, String> {
    READER
        .get_or_init(|| {
            let exe =
                std::env::current_exe().map_err(|e| format!("reader path unavailable: {e}"))?;
            let bytes =
                std::fs::read(&exe).map_err(|e| format!("reader bytes unavailable: {e}"))?;
            Ok(fingerprint(&bytes))
        })
        .clone()
}

pub(crate) fn preserve_reader() -> Result<(), String> {
    reader().map(|_| ())
}

/// Refused when the working tree holds anything the commit does not.
fn key(dir: &Path) -> Result<Key, String> {
    let here = dir.display().to_string();
    let status = git_query(&here, &["status", "--porcelain", "--untracked-files=all"])
        .ok_or("git status unavailable")?;
    if !status.is_empty() {
        return Err("uncommitted or untracked files".into());
    }
    let tree = git_query(&here, &["rev-parse", "HEAD^{tree}"]).ok_or("HEAD tree unavailable")?;
    Ok(Key {
        tree,
        reader: reader()?,
    })
}

struct Key {
    tree: String,
    reader: u64,
}

impl Key {
    fn name(&self) -> String {
        format!(
            "{:016x}",
            fnv(&format!(
                "{VERSION} reader-bytes {} {:016x}",
                self.tree, self.reader
            ))
        )
    }

    fn description(&self) -> String {
        format!("tree={} reader={:016x}", self.tree, self.reader)
    }
}

/// FNV-64a, as the step stamps use: a name for a set of bytes (no
/// adversary chooses them).
fn fnv(text: &str) -> u64 {
    fingerprint(text.as_bytes())
}

fn fingerprint(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash
}

/// Where the graphs of this repository live — beside its `.git`, so every
/// seat of it reads the same ones.
fn shelf(dir: &Path) -> Result<PathBuf, String> {
    let common =
        common_git_dir(&dir.display().to_string()).ok_or("git common directory unavailable")?;
    Ok(PathBuf::from(common).join("pgg-gate").join("graphs"))
}

/// The graph kept under `key`, if one is.
fn load(dir: &Path, key: &str) -> Result<Graph, String> {
    let text = std::fs::read_to_string(shelf(dir)?.join(key)).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "entry absent".to_string()
        } else {
            format!("cache read failed: {e}")
        }
    })?;
    read(&text).ok_or_else(|| "invalid cache payload".to_string())
}

/// Keeps `graph` under `key` and takes away all but the newest [`KEEP`].
/// A graph nobody could write is not worth a red gate.
fn keep(dir: &Path, key: &str, graph: &Graph) -> Result<(), String> {
    let shelf = shelf(dir)?;
    std::fs::create_dir_all(&shelf).map_err(|e| format!("cache directory: {e}"))?;
    // Written under another name and moved into place, so a reader never
    // meets half a graph.
    let staging = shelf.join(format!("{key}.{}.part", std::process::id()));
    if let Err(e) = std::fs::write(&staging, write(graph)) {
        let _ = std::fs::remove_file(&staging);
        return Err(format!("cache staging write: {e}"));
    }
    if let Err(e) = std::fs::rename(&staging, shelf.join(key)) {
        let _ = std::fs::remove_file(&staging);
        return Err(format!("cache publish: {e}"));
    }
    sweep(&shelf);
    Ok(())
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
/// and everything it reads, `B` a reader and the files it reads as a
/// binding alone (`graph::Resolved`), `T` a reader and the files it takes
/// as data alone (`graph::Carried::AsData`), `U` a path that resolved
/// nowhere. Tabs because a workspace path can hold a space and never one
/// of these.
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
    for (tag, readers) in [("B", &graph.bindings), ("T", &graph.read_as_data)] {
        for (reader, files) in readers {
            out.push_str(tag);
            out.push('\t');
            out.push_str(reader);
            for file in files {
                out.push('\t');
                out.push_str(file);
            }
            out.push('\n');
        }
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
    // A cut at a record boundary still parses; only the digest over the
    // whole payload refuses it.
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
            "B" => {
                let reader = fields.next()?.to_string();
                let bound: BTreeSet<String> = fields.map(str::to_string).collect();
                graph.bindings.insert(reader, bound);
            }
            "T" => {
                let reader = fields.next()?.to_string();
                let data: BTreeSet<String> = fields.map(str::to_string).collect();
                graph.read_as_data.insert(reader, data);
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

/// The graph of `dir`, from the shelf or read off the sources (and kept),
/// with the cache verdict for the run's record.
pub(crate) struct Loaded {
    pub graph: Graph,
    pub reused: bool,
    pub note: String,
}

pub(crate) fn graph_of(dir: &Path) -> Result<Loaded, String> {
    let key = match key(dir) {
        Ok(key) => key,
        Err(why) => {
            return Ok(Loaded {
                graph: super::graph::build(dir)?,
                reused: false,
                note: format!("not used: {why}"),
            });
        }
    };
    let name = key.name();
    let reason = match load(dir, &name) {
        Ok(graph) => {
            return Ok(Loaded {
                graph,
                reused: true,
                note: format!("hit; {}", key.description()),
            });
        }
        Err(why) => why,
    };
    let graph = super::graph::build(dir)?;
    let saved = match keep(dir, &name, &graph) {
        Ok(()) => "saved".to_string(),
        Err(why) => format!("not saved: {why}"),
    };
    Ok(Loaded {
        graph,
        reused: false,
        note: format!("miss: {reason}; {}; {saved}", key.description()),
    })
}

/// What a graph read back must answer the same as the one written: the
/// things everything downstream asks it (`plan`, `deps`).
#[cfg(test)]
fn same(one: &Graph, other: &Graph) -> Result<(), String> {
    if one.deps != other.deps {
        return Err("the edges differ".into());
    }
    if one.rdeps != other.rdeps {
        return Err("the readers differ".into());
    }
    if one.bindings != other.bindings {
        return Err("the bindings differ".into());
    }
    if one.read_as_data != other.read_as_data {
        return Err("the reads as data differ".into());
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

    /// Against the real tree: what the shelf must survive is the shapes
    /// this workspace holds — a crate-root module with an empty path, an
    /// integration binary, a directory node, a non-ASCII name.
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
