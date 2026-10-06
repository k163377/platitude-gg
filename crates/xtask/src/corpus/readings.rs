//! The readings a repository is described by: what the corpus's own
//! report prints, and what `--against` prints of the reference.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use super::git;

/// What both columns of the distance table are made of.
pub(super) fn readings(at: &Path) -> Result<(), String> {
    println!("profile: refs (counts and navigation structure; no aggregate workload score)");
    tags(at)?;
    branch_tree(at)?;
    println!("profile: startup/status (tracked tree, index and ignored paths)");
    working_tree(at)?;
    println!("profile: graph/scroll (lanes, chips, visible text and font coverage)");
    graph(at)?;
    window(at)?;
    println!("profile: diff (changed-file count and source-size distribution)");
    diffs(at)
}

/// What the timed diff costs, over the whole window. The harness opens the
/// first changed file of the first row (`--selection first`,
/// `perf::options`), but a tip moves; the window's distribution holds
/// still, and a generated tip has to sit inside it.
fn diffs(at: &Path) -> Result<(), String> {
    let raw = git(
        at,
        &[
            "log",
            "--max-count=2000",
            "--date-order",
            "HEAD",
            "--branches",
            "--remotes",
            "--tags",
            "--format=commit %H",
            "--raw",
            "--no-abbrev",
            "--no-renames",
        ],
    )?;
    let mut counts: Vec<u64> = Vec::new();
    let mut opened: Vec<&str> = Vec::new();
    let mut here = 0;
    let mut first = None;
    // The two newest rows with a file to open, for `perf --cases` (the
    // slots measurement).
    let mut cases: Vec<(String, String)> = Vec::new();
    let mut commit = "";
    for line in raw.lines() {
        if let Some(oid) = line.strip_prefix("commit ") {
            if here > 0 {
                counts.push(here);
                if let Some(oid) = first {
                    opened.push(oid);
                }
            }
            here = 0;
            first = None;
            commit = oid;
        } else if let Some(fields) = line.strip_prefix(':') {
            here += 1;
            // `:<srcmode> <dstmode> <srcsha> <dstsha> <status>\t<path>`,
            // and a deletion's destination is all zeroes — there is no
            // file to open for it.
            if first.is_none() {
                first = fields
                    .split_whitespace()
                    .nth(3)
                    .filter(|oid| !oid.bytes().all(|byte| byte == b'0'));
                if first.is_some()
                    && cases.len() < 2
                    && let Some((_, path)) = fields.split_once('\t')
                {
                    cases.push((commit.to_string(), path.to_string()));
                }
            }
        }
    }
    if here > 0 {
        counts.push(here);
        if let Some(oid) = first {
            opened.push(oid);
        }
    }
    println!("  perf cases (the two newest rows with a file to open, as --cases takes them):");
    for (name, (oid, path)) in ["newest", "second"].iter().zip(&cases) {
        println!("    {name}\t{oid}\t{path}\traw");
    }
    let mut bytes = sizes(at, &opened)?;
    counts.sort_unstable();
    bytes.sort_unstable();
    let at_percent = |of: &[u64], percent: usize| {
        of.get(of.len() * percent / 100)
            .copied()
            .unwrap_or_default()
    };
    println!(
        "  diffs {} rows | files/commit p50 {} p90 {} max {} | opened file p50 {}B p90 {}B max {}B",
        counts.len(),
        at_percent(&counts, 50),
        at_percent(&counts, 90),
        counts.last().copied().unwrap_or_default(),
        at_percent(&bytes, 50),
        at_percent(&bytes, 90),
        bytes.last().copied().unwrap_or_default(),
    );
    Ok(())
}

/// Each blob's size, through one `--batch-check` rather than a process
/// each.
fn sizes(at: &Path, oids: &[&str]) -> Result<Vec<u64>, String> {
    if oids.is_empty() {
        return Ok(Vec::new());
    }
    let mut child = Command::new("git")
        .current_dir(at)
        .args(["cat-file", "--batch-check=%(objectsize)"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("failed to run git cat-file: {e}"))?;
    let asking = child.stdin.take().ok_or("git cat-file took no input")?;
    let mut asking = std::io::BufWriter::new(asking);
    for oid in oids {
        // `\n` only: git takes the whole line as the name, so a carriage
        // return makes every object `missing`.
        writeln!(asking, "{oid}").map_err(|e| format!("could not ask about {oid}: {e}"))?;
    }
    drop(asking);
    let out = child
        .wait_with_output()
        .map_err(|e| format!("git cat-file failed: {e}"))?;
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| line.trim().parse().ok())
        .collect())
}

/// What the tags are, and how many remotes there are to read them from:
/// an annotated tag is a second `ls-remote` line (the peeled one
/// `remote::parse_ls_remote_tags` pairs), and a repository with no remote
/// never asks (`session::auto_fetch::known_to_have_no_remote`).
fn tags(at: &Path) -> Result<(), String> {
    let kinds = git(at, &["for-each-ref", "--format=%(objecttype)", "refs/tags"])?;
    let total = kinds.lines().filter(|kind| !kind.is_empty()).count();
    let annotated = kinds.lines().filter(|kind| *kind == "tag").count();
    let remotes = git(at, &["remote"])?
        .lines()
        .filter(|name| !name.is_empty())
        .count();
    let share = if total == 0 {
        0.0
    } else {
        annotated as f64 * 100.0 / total as f64
    };
    println!("  tags {total} | {annotated} annotated ({share:.1}%) | {remotes} remotes configured");
    Ok(())
}

/// What the working tree costs to read, which is most of startup:
/// `status::load` pays one `lstat` per tracked file.
///
/// The time is a reading of the walk: the corpus does not copy the
/// reference clone's `core.fsmonitor`, which makes its status slower
/// (ci/baseline/code-costs-windows-x64.md §コーパス生成) and would time a
/// daemon's health on the day.
fn working_tree(at: &Path) -> Result<(), String> {
    let tracked = git(at, &["ls-files"])?.lines().count();
    let index = std::fs::metadata(at.join(".git").join("index"))
        .map(|meta| meta.len())
        .unwrap_or_default();
    // waits(measured): the status's time, one of the readings the corpus is described by
    let began = std::time::Instant::now();
    // `--no-optional-locks`: this reading is also taken of repositories
    // that are only read, and a status without it rewrites their index.
    git(
        at,
        &[
            "--no-optional-locks",
            "status",
            "--porcelain=v2",
            "-z",
            "--branch",
            "-uall",
        ],
    )?;
    let status = began.elapsed();
    let objects = git(at, &["count-objects", "-vH"])?;
    let read = |key: &str| {
        objects
            .lines()
            .find_map(|line| line.strip_prefix(key))
            .unwrap_or("?")
            .trim()
            .to_string()
    };
    let dirs = git(at, &["ls-tree", "-r", "-d", "--name-only", "HEAD"])?
        .lines()
        .count();
    // The size histogram: a file count cannot tell stubs from sources,
    // which cost differently to check out, status and open.
    let mut bytes: Vec<u64> = git(at, &["ls-tree", "-r", "--format=%(objectsize)", "HEAD"])?
        .lines()
        .filter_map(|line| line.trim().parse().ok())
        .collect();
    bytes.sort_unstable();
    let at_percent = |percent: usize| {
        bytes
            .get(bytes.len() * percent / 100)
            .copied()
            .unwrap_or_default()
    };
    let total: u64 = bytes.iter().sum();
    let packs = std::fs::read_dir(at.join(".git").join("objects").join("pack"))
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "pack"))
                .count()
        })
        .unwrap_or_default();
    let midx = at
        .join(".git")
        .join("objects")
        .join("pack")
        .join("multi-pack-index")
        .exists();
    println!(
        "  packs {packs}{} — every git process maps these first",
        if midx { " + a multi-pack-index" } else { "" }
    );
    println!(
        "  working tree {tracked} files in {dirs} dirs | index {}MB | status {:.2}s | {} objects in {}",
        index / 1_000_000,
        status.as_secs_f64(),
        read("in-pack:"),
        read("size-pack:")
    );
    println!(
        "  file bytes p25 {} p50 {} p75 {} p90 {} max {} | {}MB of tree",
        at_percent(25),
        at_percent(50),
        at_percent(75),
        at_percent(90),
        bytes.last().copied().unwrap_or_default(),
        total / 1_000_000,
    );
    Ok(())
}

/// How many folder rows the sidebar's remotes section would build:
/// `nav::tree::build_tree` makes a `NavItem` per distinct directory prefix
/// on every arrange, without compacting single-child chains.
fn branch_tree(at: &Path) -> Result<(), String> {
    let names = git(
        at,
        &[
            "for-each-ref",
            "--format=%(refname:lstrip=2)",
            "refs/remotes",
        ],
    )?;
    let mut folders = std::collections::BTreeSet::new();
    let mut leaves = 0;
    let mut deepest = 0;
    for name in names.lines().filter(|name| !name.is_empty()) {
        leaves += 1;
        let mut at = 0;
        let mut depth = 0;
        while let Some(slash) = name[at..].find('/') {
            at += slash;
            folders.insert(name[..at].to_string());
            at += 1;
            depth += 1;
        }
        deepest = deepest.max(depth);
    }
    println!(
        "  remotes {leaves} leaves | {} folder rows | {deepest} deep",
        folders.len()
    );
    Ok(())
}

/// The shape of the graph the window draws: how many refs land on its
/// rows, and how wide it gets. A lane is open from a row with a
/// not-yet-emitted parent until that parent is emitted, as in
/// `GraphBuilder`.
fn graph(at: &Path) -> Result<(), String> {
    let rows = git(
        at,
        &[
            "log",
            "--max-count=2000",
            "--date-order",
            "HEAD",
            "--branches",
            "--remotes",
            "--tags",
            "--format=%H\x1f%P\x1f%D",
        ],
    )?;
    let mut open: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut widths = Vec::new();
    let (mut chips, mut carrying, mut busiest, mut merges) = (0, 0, 0, 0);
    for row in rows.lines().filter(|row| !row.is_empty()) {
        let mut field = row.split('\x1f');
        let oid = field.next().unwrap_or_default();
        let parents = field.next().unwrap_or_default();
        let refs = field.next().unwrap_or_default();
        open.remove(oid);
        if parents.split_whitespace().nth(1).is_some() {
            merges += 1;
        }
        for parent in parents.split_whitespace() {
            open.insert(parent.to_string());
        }
        widths.push(open.len());
        let here = refs
            .split(',')
            .filter(|name| !name.trim().is_empty())
            .count();
        if here > 0 {
            carrying += 1;
            chips += here;
            busiest = busiest.max(here);
        }
    }
    widths.sort_unstable();
    let at_percent = |p: usize| widths.get(widths.len() * p / 100).copied().unwrap_or(0);
    println!(
        "  graph lanes p25 {} / p50 {} / p75 {} / max {} | {chips} chips on {carrying} rows, \
         busiest {busiest} | {merges} merges",
        at_percent(25),
        at_percent(50),
        at_percent(75),
        widths.last().copied().unwrap_or(0)
    );
    Ok(())
}

/// How many rows the graph's window would draw, and what they carry: the
/// body and credits `parse::log` asks for, the fallback font a non-ASCII
/// glyph loads, the second identity the details card shows. Printed so a
/// generator that stopped carrying one shows here.
fn window(at: &Path) -> Result<(), String> {
    const ROWS: &str = "--max-count=2000";
    let log = git(
        at,
        &[
            "log",
            ROWS,
            "--date-order",
            "HEAD",
            "--branches",
            "--remotes",
            "--tags",
            "--format=%aN\x1f%cN\x1f%s\x1f%(trailers:key=Co-authored-by,valueonly)\x1f%b\x1e",
        ],
    )?;
    let mut rows = 0;
    let mut wide = 0;
    let mut credited = 0;
    let mut applied = 0;
    let mut body_bytes = 0;
    // Distinct names: the text held grows with authors, not rows.
    let mut authors = std::collections::BTreeSet::new();
    for row in log.split('\x1e').filter(|row| !row.trim().is_empty()) {
        let mut field = row.trim_start_matches('\n').split('\x1f');
        let (author, committer) = (field.next().unwrap_or_default(), field.next());
        let subject = field.next().unwrap_or_default();
        let credits = field.next().unwrap_or_default();
        let body = field.next().unwrap_or_default();
        rows += 1;
        authors.insert(author);
        if !author.is_ascii() || !subject.is_ascii() {
            wide += 1;
        }
        if !credits.trim().is_empty() {
            credited += 1;
        }
        if committer.is_some_and(|by| by != author) {
            applied += 1;
        }
        body_bytes += body.trim().len();
    }
    println!(
        "  window {rows} rows | {} authors | body {body_bytes}B | {credited} credited | \
         {applied} applied by another | {wide} needing a fallback font",
        authors.len()
    );
    Ok(())
}
