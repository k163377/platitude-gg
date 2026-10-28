//! What a branch owes main, worked out before anything runs: the reach of
//! its diff, the steps that reach selects, and which of those are already
//! green for the inputs they read.

mod imports;
mod pins;
mod steps;
mod urls;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::graph::{self, Carried, Reach, stem_of};
use super::lines::{Lines, Shown};
use super::record::Spent;
use super::shown::{Composition, Module};
use super::stamp::Store;
use super::tiers::Tiers;
use crate::subprocess::{git_query, run_captured};
use steps::select;

/// Which side of stage 2 a step runs on: the host, or the Linux container
/// (which is "here" when the host is Linux — `cargo xtask linux`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Side {
    Host,
    Linux,
}

#[derive(Clone, Debug)]
pub(crate) struct Step {
    pub id: String,
    pub side: Side,
    /// Runs on every gate regardless of the diff, never cached: its
    /// seconds are not worth a stamp.
    pub always: bool,
    /// A verify-ui run, which builds the release unless the runner says an
    /// earlier verb of the side built it in this invocation (a cached
    /// step's build may have been another tree's).
    pub builds_app: bool,
    /// Reads the release the side's verbs build (a verb, or `bare`), so
    /// runs in the side's built-app group (`gate::sides::side`).
    pub release: bool,
    /// Shares no build directory with either group — neither cargo's
    /// `target/debug` and `target/shipped` nor the verbs' release — so runs
    /// beside both instead of in the checks' chain (`qmltest`: Qt's own
    /// runner over a staging copy of the QML).
    pub beside: bool,
    pub command: Vec<String>,
    /// What the step reads, as workspace paths (a directory covers
    /// everything under it): the cache key is their object ids.
    pub inputs: BTreeSet<String>,
    /// Which id of each input the key takes, for a step that neither
    /// builds nor reads the app ([`owed`] keys those on the code).
    pub keyed: Keyed,
    /// What the step is held to beyond its inputs, in the key too: a
    /// verify-ui line's promised parts and judgment (`steps::binary_steps`),
    /// so a row or a claim moved runs the line again and a green of
    /// another contract answers for nothing. Empty for the other steps.
    pub contract: String,
    /// Why no key answers for what it reads, where none can — a load
    /// nothing names (`steps::qmltest_steps`): it runs on every gate, and
    /// its green stamps nothing.
    pub unkeyed: Option<String>,
}

/// Which id of an input a step's key takes ([`TreeIds`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Keyed {
    /// Every byte: a tool that reads the sources sees them all.
    Bytes,
    /// A Rust file's code, every other file's bytes: a program built from
    /// Rust, which no comment of its reaches, over files it or what it
    /// runs reads as they stand (`steps::qmltest_steps`).
    RustCode,
}

/// One step the branch owes.
pub(crate) struct Required {
    pub step: Step,
    /// Empty for an always-step.
    pub key: String,
    pub cached: bool,
}

/// What one side owes beyond the reach of the diff.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub(crate) struct Scope {
    /// Every file in the tree counted as reached, and why: `--whole`, a
    /// moved version, or the build input that changed.
    pub everything: Option<String>,
    /// The whole tier's steps (`periodic`, every registered line): `--whole`,
    /// or a moved version ([`pins`]) before a merge.
    pub whole_tier: bool,
}

pub(crate) struct Plan {
    pub dir: PathBuf,
    pub head: String,
    pub main: String,
    pub base: String,
    /// The branch sits on main: its diff against main is exactly what
    /// landing would add.
    pub onto_main: bool,
    pub host_only: bool,
    /// Every step runs whether or not a stamp answers for it, even one
    /// another tree stamps while this run queues (`gate::step::run_one`
    /// looks again).
    pub fresh: bool,
    pub host: Scope,
    /// The host's, and more where the change is the container's own: the
    /// files its image is built from, or a version it alone runs.
    pub container: Scope,
    pub changed: Vec<String>,
    pub reach: BTreeSet<String>,
    pub required: Vec<Required>,
    /// Product components the change reaches that no registered line
    /// shows (`lines::Shown`) — nothing headless brings them up, so the
    /// gate cannot pass them.
    pub uncovered: Vec<String>,
    /// Changed files no step reads, listed so that a kind of file nothing
    /// tests shows in the report.
    pub unclaimed: Vec<String>,
    /// What the graph says is wrong with the tree itself
    /// (`graph::complaints`); no run over such a tree can be stamped.
    pub complaints: Vec<String>,
    /// How many of the selected lines show a component the change itself
    /// touched (`lines::Shown`), for comparison only: what a selection by
    /// what each line shows would run, against the selection by reach
    /// (反映前テストの機械化.md §動詞の絞り込みは比較表示に留める).
    pub verbs_by_shown: usize,
    /// Registered lines the reach selected that this gate leaves to the
    /// whole one (`tiers`) — before a merge only; the whole gate leaves none.
    pub verbs_left: usize,
}

pub(crate) struct Ask<'a> {
    pub main_ref: &'a str,
    pub host_only: bool,
    pub whole: bool,
    pub fresh: bool,
    /// Registered verify-ui lines to run besides the reach's, as typed.
    pub extra_verbs: &'a [String],
}

/// The two crates the built app is made of, and the harness directories
/// its runs read. Spelled in pieces: a whole path in a string here would
/// make this file a reader of everything under it, and through it owe
/// every test of the task runner to any core change (反映前テストの機械化.md
/// §依存木).
fn core() -> String {
    format!("crates/{}", "platitude-core")
}

fn app() -> String {
    format!("crates/{}", "platitude-app")
}

fn harness() -> [String; 2] {
    ["verify", "demo"].map(|part| format!("crates/xtask/src/{part}"))
}

const DOCKERFILE: &str = "ci/linux/Dockerfile";
/// Where Qt's version is pinned, for a desk, the container and CI alike
/// (`qt::PIN`).
const QT_PIN: &str = ".qt-version";
/// What the container's image is built from (`linux::image_tag`), in the
/// key of every step that runs in it: a new Qt is a new image.
const IMAGE: [&str; 2] = [DOCKERFILE, QT_PIN];
/// The system the container's side compiles for, as
/// `std::env::consts::OS` names it there.
const CONTAINER_OS: &str = "linux";
/// The dependency policy, which nothing in the source graph reads.
/// Spelled in pieces, as [`clippy_config`] is: a name here is a step's input,
/// and whole it would make this file a reader of the policy.
fn deny_policy() -> String {
    format!("{}.toml", "deny")
}
/// clippy's own configuration, which only clippy reads.
fn clippy_config() -> String {
    format!("{}.toml", "clippy")
}
/// The crates the workspace `Cargo.toml` patches in (`[patch.crates-io]`):
/// built into whatever depends on them, read by no graph of the sources.
const VENDOR: &str = "vendor";
/// What every cargo build reads besides the sources: the manifest, the
/// lock and the toolchain, cargo's own configuration (`.cargo/` — its
/// `[env]` reaches every C and C++ compile a build script runs), and the
/// patched-in crates.
const CARGO: [&str; 5] = [
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    ".cargo",
    VENDOR,
];
/// The QML test runner's entry, whose closure as built is what a run of
/// it is (`steps::qmltest_reads`).
const QMLTEST: &str = "crates/xtask/src/qmltest.rs";
/// The markdown this tree reads as rules (`crate::docs`).
const DOCS: [&str; 4] = [
    "internal-docs",
    ".claude/rules",
    ".claude/rules-refs",
    "CLAUDE.md",
];

/// The app's QML: the module the product ships, and the QtTest files that
/// read it. Spelled in pieces for the reason [`core`] gives.
fn qml_dirs() -> (String, String) {
    let app = app();
    (format!("{app}/src/ui"), format!("{app}/tests/qml"))
}

/// Where the branch stands against main, and what it changed getting
/// there.
struct Standing {
    head: String,
    main: String,
    base: String,
    onto_main: bool,
    changed: Vec<String>,
}

/// quotepath off, or a non-ASCII name comes back octal-escaped in quotes
/// and matches no file. Renames off, or a moved file is listed under its
/// new name alone and the old one, which every reader still names, is
/// never seen to go ([`gone_source`]).
fn standing(here: &str, main_ref: &str) -> Result<Standing, String> {
    let rev = |what: &str| {
        git_query(
            here,
            &[
                "rev-parse",
                "--verify",
                "--quiet",
                &format!("{what}^{{commit}}"),
            ],
        )
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("{what} names no commit in {here}"))
    };
    let head = rev("HEAD")?;
    let main = rev(main_ref)?;
    let base = git_query(here, &["merge-base", &main, &head]).ok_or("git merge-base failed")?;
    let changed = git_query(
        here,
        &[
            "-c",
            "core.quotepath=false",
            "diff",
            "--no-renames",
            "--name-only",
            &base,
            &head,
        ],
    )
    .ok_or("git diff failed")?
    .lines()
    .filter(|l| !l.is_empty())
    .map(str::to_string)
    .collect();
    Ok(Standing {
        onto_main: base == main,
        head,
        main,
        base,
        changed,
    })
}

/// The plan, with each phase of making it timed into `spent`.
pub(crate) fn make(dir: &Path, ask: &Ask<'_>, spent: &mut Spent) -> Result<Plan, String> {
    // waits(measured): the plan's cost, for the record
    let started = std::time::Instant::now();
    let here = dir.display().to_string();
    let Standing {
        head,
        main,
        base,
        onto_main,
        changed,
    } = standing(&here, ask.main_ref)?;
    // waits(measured): the phase's cost, for the record
    let at = std::time::Instant::now();
    let loaded = super::reuse::graph_of(dir)?;
    spent.graph = at.elapsed();
    spent.graph_reused = loaded.reused;
    spent.graph_cache = loaded.note;
    let g = loaded.graph;
    // Documents stay in the reported diff but select no tests, and so do
    // the comments of a configuration.
    let executable_changes: Vec<String> = changed
        .iter()
        .filter(|file| !graph::is_markdown(file) && !comments_only(&here, &base, &head, file))
        .cloned()
        .collect();
    let (host, container) = scopes(dir, ask, &here, &base, &head, &executable_changes);
    let Reaches {
        touched,
        host: reach,
        container: container_reach,
    } = reaches(&g, dir, &host, &container, &executable_changes)?;
    // waits(measured): the phase's cost, for the record
    let at = std::time::Instant::now();
    let lines = Lines::load(dir)?;
    let tiers = Tiers::load(dir)?;
    let added = rows_the_tables_owe(&here, &base, &changed, &lines, &tiers);
    let composition = Composition::read(dir)?;
    let shown = lines.shown(&composition);
    spent.lines = at.elapsed();
    registered_or_refused(&lines, ask.extra_verbs)?;
    let read = |scope: &Scope| Reading {
        dir,
        lines: &lines,
        added: &added,
        shown: &shown,
        tiers: &tiers,
        everything: scope.everything.is_some(),
        whole_tier: scope.whole_tier,
    };
    let (
        mut steps,
        Counted {
            by_shown: verbs_by_shown,
            left: verbs_left,
        },
    ) = select(&g, &read(&host), &reach, &executable_changes, ask);
    if !ask.host_only && (container != host || container_reach != reach) {
        let (theirs, _) = select(
            &g,
            &read(&container),
            &container_reach,
            &executable_changes,
            ask,
        );
        steps.retain(|step| step.side == Side::Host);
        steps.extend(theirs.into_iter().filter(|step| step.side == Side::Linux));
    }
    let store = Store::open(dir)?;
    let required = owed(&here, &head, &store, steps, ask, spent)?;
    let uncovered = uncovered(&composition, &lines.promised(), &touched);
    // The two tables are the plan's own inputs (`steps::binary_steps`),
    // answered by what the plan owes, not by a step's key.
    let tables = [super::lines::FILE, super::tiers::FILE];
    let unclaimed = changed
        .iter()
        .filter(|file| {
            !tables.contains(&file.as_str())
                && !required
                    .iter()
                    .any(|r| !r.step.always && r.step.inputs.iter().any(|i| under(file, i)))
        })
        .cloned()
        .collect();
    spent.plan_rest = started
        .elapsed()
        .saturating_sub(spent.graph + spent.lines + spent.ids);
    Ok(Plan {
        dir: dir.to_path_buf(),
        head,
        main,
        base,
        onto_main,
        host_only: ask.host_only,
        fresh: ask.fresh,
        host,
        container,
        changed,
        reach: reach.into_keys().collect(),
        required,
        uncovered,
        unclaimed,
        complaints: graph::complaints(&g),
        verbs_by_shown,
        verbs_left,
    })
}

/// The lines the two tables' own move owes, whatever else moved
/// (`steps::binary_steps`): a row added to the registry, which no stamp
/// answers for, and a line whose tier moved, which this gate may owe now
/// where it did not — on the container too, where `fonts` says so. Read
/// against the tables as the base holds them; a base without a table
/// makes every row new.
fn rows_the_tables_owe(
    here: &str,
    base: &str,
    changed: &[String],
    lines: &Lines,
    tiers: &Tiers,
) -> Vec<String> {
    let moved = |file: &str| changed.iter().any(|f| f == file);
    let before = |file: &str| git_query(here, &["show", &format!("{base}:{file}")]);
    let mut owed = Vec::new();
    if moved(super::lines::FILE) {
        // A row added, or one whose promise moved: a new contract either
        // way, which the green of the old one answers for nothing.
        let registered = before(super::lines::FILE).and_then(|text| Lines::parse(&text).ok());
        owed.extend(
            lines
                .lines()
                .filter(|line| {
                    !registered
                        .as_ref()
                        .is_some_and(|b| b.declared(line) == lines.declared(line))
                })
                .cloned(),
        );
    }
    if moved(super::tiers::FILE) {
        let tiered = before(super::tiers::FILE).map(|text| Tiers::parse(&text));
        let retiered: Vec<String> = lines
            .lines()
            .filter(|line| {
                tiered
                    .as_ref()
                    .is_none_or(|b| b.tier(line) != tiers.tier(line))
            })
            .filter(|line| !owed.contains(line))
            .cloned()
            .collect();
        owed.extend(retiered);
    }
    owed
}

/// A line asked for by name has to be a registered one: the gate runs
/// nothing the table does not hold.
fn registered_or_refused(lines: &Lines, extra: &[String]) -> Result<(), String> {
    for line in extra {
        if !lines.contains(line) {
            return Err(format!(
                "--verb {line:?} is not a registered line: register it in {} (with what its run \
                 brings up) and the gate runs it from then on, or run it with `cargo xtask \
                 verify-ui` by itself",
                super::lines::FILE
            ));
        }
    }
    Ok(())
}

/// The reach of everything: every node, and every QML file — a component
/// with no edge is no node, and would be missed. All handed whole: the
/// change is the build itself.
fn every_file(g: &graph::Graph, dir: &Path) -> Result<Reach, String> {
    let mut every: Reach = g
        .deps
        .keys()
        .chain(g.rdeps.keys())
        .chain(g.modules.keys())
        .filter(|f| !f.ends_with('/'))
        .map(|f| (f.clone(), Carried::Whole))
        .collect();
    every.extend(
        graph::qml_files(dir)?
            .into_iter()
            .map(|f| (f, Carried::Whole)),
    );
    Ok(every)
}

/// Each step with its key and whether a stamp already answers for it, the
/// Linux side's dropped under `--host-only`. The keys' object ids come from
/// one listing of the tree: a git per step and input is minutes of planning.
fn owed(
    here: &str,
    head: &str,
    store: &Store,
    steps: Vec<Step>,
    ask: &Ask<'_>,
    spent: &mut Spent,
) -> Result<Vec<Required>, String> {
    // waits(measured): the phase's cost, for the record
    let at = std::time::Instant::now();
    let ids = tree_ids(here, head)?;
    spent.ids = at.elapsed();
    Ok(steps
        .into_iter()
        .filter(|step| !(ask.host_only && step.side == Side::Linux))
        .map(|step| {
            let (key, cached) = if step.always || step.unkeyed.is_some() {
                (String::new(), false)
            } else {
                // A step that builds or reads the app sees no comment (`shipped`, the verbs, `bare`), so its key
                // is of the code alone; a tool that reads the sources (fmt, clippy, waits with its markers, docs,
                // structure, the tests with their doc-tests) sees every byte; a step says otherwise (`Keyed`).
                let ids = if step.release || step.id == "shipped" {
                    &ids.code
                } else {
                    match step.keyed {
                        Keyed::Bytes => &ids.whole,
                        Keyed::RustCode => &ids.rust,
                    }
                };
                let key = cache_key(ids, &step);
                let cached = !ask.fresh && store.step_green(&key);
                (key, cached)
            };
            Required { step, key, cached }
        })
        .collect())
}

/// The object ids of every path in the tree, three times over: as git
/// holds them (`whole`), with a Rust or QML file's id its code's (`code`),
/// and with a Rust file's alone its code's (`rust`) — `inputs::with_code_ids`.
struct TreeIds {
    whole: BTreeMap<String, String>,
    code: BTreeMap<String, String>,
    rust: BTreeMap<String, String>,
}

/// The object id of every path in the tree at `rev` from one `git ls-tree`;
/// a directory's is a fingerprint of its entries besides Markdown
/// (`inputs::from_listing`).
fn tree_ids(here: &str, rev: &str) -> Result<TreeIds, String> {
    let mut command = std::process::Command::new("git");
    command
        .arg("-C")
        .arg(here)
        .args(["ls-tree", "-r", "-t", "-z", "--full-tree", rev]);
    let output = run_captured(&mut command)?;
    if !output.status.success() {
        return Err(format!(
            "git ls-tree {rev} failed in {here}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    // A Rust or QML file's code id, so a comment moved stamps no run of the
    // built app out (`inputs::with_code_ids`); the tools that read the
    // sources key on the whole file.
    let whole = parse_ls_tree(&output.stdout);
    let listings = super::inputs::with_code_ids(here, &output.stdout)?;
    let code = parse_ls_tree(&listings.code);
    let rust = parse_ls_tree(&listings.rust);
    // A file the lexical reading stopped short on is keyed on its bytes
    // (`inputs::code_id`), and said: a key that moves on every edit of it
    // is then no mystery.
    let read_short: Vec<&str> = code
        .iter()
        .filter(|(_, id)| id.starts_with("bytes:"))
        .map(|(path, _)| path.as_str())
        .collect();
    if !read_short.is_empty() {
        println!(
            "gate: {} file(s) read short, keyed on their bytes: {}",
            read_short.len(),
            read_short.join(" ")
        );
    }
    Ok(TreeIds { whole, code, rust })
}

/// `ls-tree -z`: `<mode> <type> <id>\t<path>` per entry, NUL after each
/// (`-z`, or a non-ASCII name comes back octal-escaped in quotes).
fn parse_ls_tree(listing: &[u8]) -> BTreeMap<String, String> {
    super::inputs::from_listing(listing)
}

/// A file every build reads and no graph of the sources sees: a change to
/// it is a change to everything. A crate's own manifest and build script
/// are its modules' reading (`graph::build`), clippy's configuration is
/// clippy's (`steps::select`), and the container's image its own side's
/// ([`scopes`]).
fn moves_everything(file: &str) -> bool {
    matches!(file, "Cargo.toml" | "Cargo.lock" | "rust-toolchain.toml")
        || file.starts_with(".cargo/")
        || under(file, VENDOR)
}

/// What each side owes beyond the reach: the host's, and the container's.
/// The daily tier stays the host's quick half; stage 2 owes the whole tier
/// for a new version, on the side built with it ([`pins`]).
fn scopes(
    dir: &Path,
    ask: &Ask<'_>,
    here: &str,
    base: &str,
    head: &str,
    changed: &[String],
) -> (Scope, Scope) {
    if ask.whole {
        let whole = Scope {
            everything: Some("--whole".to_string()),
            whole_tier: true,
        };
        return (whole.clone(), whole);
    }
    let pinned = if ask.host_only {
        None
    } else {
        pins::moved(here, base, head, changed)
    };
    let host = match &pinned {
        Some(moved) if !moved.container_only => Scope {
            everything: Some(moved.why.clone()),
            whole_tier: true,
        },
        _ => Scope {
            everything: changed
                .iter()
                .find(|f| moves_everything(f))
                .cloned()
                .or_else(|| {
                    changed
                        .iter()
                        .find(|f| gone_source(dir, f))
                        .map(|f| format!("{f} is gone"))
                }),
            whole_tier: false,
        },
    };
    let container = match pinned {
        Some(moved) if !host.whole_tier => Scope {
            everything: Some(moved.why),
            whole_tier: true,
        },
        _ if host.everything.is_none() => Scope {
            everything: changed
                .iter()
                .find(|f| IMAGE.contains(&f.as_str()))
                .map(|f| format!("{f} builds its image")),
            whole_tier: false,
        },
        _ => host.clone(),
    };
    (host, container)
}

/// A TOML file the branch changed in its comments alone: no build, tool or
/// test reads a comment. Read as whole lines — a `#` after a value may
/// sit in a string — and never in a file holding a multi-line string,
/// where a line of the string may start with one.
fn comments_only(here: &str, base: &str, head: &str, file: &str) -> bool {
    if !file.ends_with(".toml") {
        return false;
    }
    // As stored: `git_query` trims and turns backslashes, which would
    // read a change to a value's backslash as no change.
    let settings = |rev: &str| {
        let mut show = std::process::Command::new("git");
        show.arg("-C")
            .arg(here)
            .args(["show", &format!("{rev}:{file}")]);
        let shown = run_captured(&mut show)
            .ok()
            .filter(|o| o.status.success())?;
        let text = String::from_utf8_lossy(&shown.stdout);
        if text.contains("\"\"\"") || text.contains("'''") {
            return None;
        }
        Some(
            text.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .map(str::to_string)
                .collect::<Vec<String>>(),
        )
    };
    matches!((settings(base), settings(head)), (Some(old), Some(new)) if old == new)
}

/// What each side selects from, and what the change itself reaches on the
/// host — what has to be shown.
struct Reaches {
    touched: Reach,
    host: Reach,
    container: Reach,
}

/// What the change reaches, each side read for the code its own system
/// compiles (`graph::Graph::reach_on`); a side owing everything selects
/// from every file, since a build input widens only what runs.
fn reaches(
    g: &graph::Graph,
    dir: &Path,
    host: &Scope,
    container: &Scope,
    changed: &[String],
) -> Result<Reaches, String> {
    let moved = with_readers_of_the_gone(g, dir, changed);
    let touched = g.reach_on(&moved, std::env::consts::OS);
    let every = if host.everything.is_some() || container.everything.is_some() {
        Some(every_file(g, dir)?)
    } else {
        None
    };
    let on_the_host = match (&host.everything, &every) {
        (Some(_), Some(every)) => every.clone(),
        _ => touched.clone(),
    };
    let on_the_container = match (&container.everything, every) {
        (Some(_), Some(every)) => every,
        _ => g.reach_on(&moved, CONTAINER_OS),
    };
    Ok(Reaches {
        touched,
        host: on_the_host,
        container: on_the_container,
    })
}

/// `changed`, and what still names a file of it the tree no longer holds
/// (`graph::Graph::naming`): read off the tree, the graph has no edge to
/// that file, and a build embedding it breaks with its reader unchanged —
/// so the reader is owed as changed, on the systems that compile it.
fn with_readers_of_the_gone(g: &graph::Graph, dir: &Path, changed: &[String]) -> Vec<String> {
    let gone: BTreeSet<String> = changed
        .iter()
        .filter(|file| !dir.join(file).exists())
        .cloned()
        .collect();
    let mut moved = changed.to_vec();
    if !gone.is_empty() {
        moved.extend(g.naming(dir, &gone));
    }
    moved
}

/// A source the tree no longer holds: the graph, read off the tree, has no
/// edge to it, so its reach would be the file alone and a deletion still
/// named elsewhere would be stamped green. Everything is the one reach
/// that cannot miss the reader.
fn gone_source(dir: &Path, file: &str) -> bool {
    (file.ends_with(".rs") || file.ends_with(".qml") || file.ends_with("/qmldir"))
        && !gone_test(file)
        && !dir.join(file).exists()
}

/// A test taken out has no reader the graph could miss: a module under
/// `tests/` is named by nothing outside its binary, and its declaring root
/// or `mod.rs` changes with it (`fmt` refuses a declaration left behind),
/// which reaches the binary whole; a QtTest file is named by nothing, and
/// `qmltest_steps` reads the reach by path.
fn gone_test(file: &str) -> bool {
    file.contains("/tests/") && (file.ends_with(".rs") || stem_of(file).starts_with("tst_"))
}

fn under(file: &str, input: &str) -> bool {
    file == input || file.starts_with(&format!("{}/", input.trim_end_matches('/')))
}

/// What a selection reads off the tree besides the reach.
struct Reading<'a> {
    dir: &'a Path,
    /// The registered lines (`lines`).
    lines: &'a Lines,
    /// The lines the tables' own move owes — a row added, a tier moved —
    /// whatever else moved (`rows_the_tables_owe`).
    added: &'a [String],
    /// What each of them shows, read off the sources.
    shown: &'a Shown,
    /// Which of the lines each gate owes, and where (`tiers`).
    tiers: &'a Tiers,
    everything: bool,
    /// Owes the whole tier ([`Scope::whole_tier`]).
    whole_tier: bool,
}

/// What the verb selection counted beside the steps it made.
struct Counted {
    /// The selected lines that show a reached component
    /// (`steps::lines_showing_the_reach`).
    by_shown: usize,
    /// Selected lines left to the whole gate.
    left: usize,
}

/// The product components the change reaches that no registered line
/// promises to show (`Lines::promised`): a green of every line is
/// evidence of none of them. What a line's run brings up by composition
/// alone is no promise — the picture is held to the row, not to the
/// sources' closure (反映前テストの機械化 §選択の正本).
fn uncovered(composition: &Composition, promised: &BTreeSet<String>, reach: &Reach) -> Vec<String> {
    let (_, qml_tests) = qml_dirs();
    reach
        .keys()
        .filter(|f| f.ends_with(".qml"))
        // A QtTest file is shown by its own runner (`qmltest_steps`); the
        // harness's own parts are the verbs, not what they show
        // (`verify::verbs` reads them whole).
        .filter(|f| !under(f, &qml_tests))
        .filter(|f| composition.module(&stem_of(f)) == Some(Module::Product))
        .filter(|f| composition.instantiable(&stem_of(f)))
        // A root another file wears is met under the wearer's name, so the
        // wearer's promise is its (`verify::observed`).
        .filter(|f| !composition.is_worn(&stem_of(f)))
        .filter(|f| !promised.contains(&stem_of(f)) && stem_of(f) != super::shown::MAIN)
        .cloned()
        .collect()
}

/// The cache key: the step's identity and command, and the object id of
/// each input in the tree under test (`ids`, from [`tree_ids`]; a path
/// the tree does not hold is `absent`). FNV-1a, a fingerprint and not a
/// security claim.
fn cache_key(ids: &BTreeMap<String, String>, step: &Step) -> String {
    let mut text = step.id.clone();
    text.push('\0');
    text.push_str(&step.command.join("\0"));
    text.push('\n');
    text.push_str(&step.contract);
    text.push('\n');
    for input in &step.inputs {
        let path = input.trim_end_matches('/');
        let oid = ids.get(path).map_or("absent", String::as_str);
        text.push_str(path);
        text.push('=');
        text.push_str(oid);
        text.push('\n');
    }
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// The plan as the gate prints it; `keys` adds each cached-or-run step's
/// key (a dry run's: what a change of a step's inputs moves).
pub(crate) fn describe(plan: &Plan, keys: bool) -> String {
    let mut out = headline(plan);
    if plan.host.everything.is_some() {
        out.push_str(&format!(
            "changed ({}); reach: every file ({})\n",
            plan.changed.len(),
            plan.reach.len()
        ));
    } else {
        out.push_str(&format!(
            "changed ({}), reach ({}):\n",
            plan.changed.len(),
            plan.reach.len()
        ));
        for file in &plan.changed {
            out.push_str(&format!("  * {file}\n"));
        }
        let mut readers: Vec<&String> = plan
            .reach
            .iter()
            .filter(|f| !plan.changed.contains(f) && !f.ends_with('/'))
            .collect();
        readers.sort();
        for file in readers {
            out.push_str(&format!("    {file}\n"));
        }
    }
    if !plan.unclaimed.is_empty() {
        out.push_str("no step reads these (docs, CI, or a kind of file no step knows):\n");
        for file in &plan.unclaimed {
            out.push_str(&format!("  {file}\n"));
        }
    }
    if !plan.uncovered.is_empty() {
        out.push_str("no registered line shows these — nothing headless brings them up:\n");
        for file in &plan.uncovered {
            out.push_str(&format!("  {file}\n"));
        }
    }
    if !plan.complaints.is_empty() {
        out.push_str("the graph cannot read this tree whole:\n");
        for line in &plan.complaints {
            out.push_str(&format!("  {line}\n"));
        }
    }
    let chosen = plan
        .required
        .iter()
        .filter(|r| r.step.id.starts_with("verify ") && r.step.side == Side::Host)
        .count();
    let on_linux = plan
        .required
        .iter()
        .filter(|r| r.step.id.starts_with("verify-linux "))
        .count();
    if chosen > 0 || on_linux > 0 {
        // The container's lines are the host's own unless its side owes
        // more than the host's does; what waits for the whole gate is the
        // host's count.
        let (container, left) = if plan.container == plan.host {
            (format!("{on_linux} of them on the container too"), "")
        } else {
            (format!("{on_linux} on the container"), " on the host")
        };
        out.push_str(&format!(
            "verbs: {chosen} selected, {container}; {} left{left} to the whole gate ({}); by what \
             they show: {} (a comparison, not used to skip runs)\n",
            plan.verbs_left,
            super::tiers::FILE,
            plan.verbs_by_shown
        ));
    }
    out.push_str(&format!("steps ({}):\n", plan.required.len()));
    for r in &plan.required {
        out.push_str(&step_row(r, keys));
    }
    out
}

/// The first line of [`describe`]: what is gated against what, and what
/// each side owes beyond the reach — the container's said apart when it
/// owes more than the host's.
fn headline(plan: &Plan) -> String {
    let short = |sha: &str| sha.chars().take(10).collect::<String>();
    let scope = |scope: &Scope| {
        let mut said: Vec<String> = Vec::new();
        if let Some(why) = &scope.everything {
            said.push(format!("everything ({why})"));
        }
        if scope.whole_tier {
            said.push("the whole tier".to_string());
        }
        said
    };
    let mut standing = vec![
        if plan.onto_main {
            "on main"
        } else {
            "off main — a pseudo-run; land rebases and gates again"
        }
        .to_string(),
    ];
    if plan.host_only {
        standing.push("host side only".to_string());
    }
    standing.extend(scope(&plan.host));
    let mut line = format!(
        "gate: HEAD {} against main {} (base {}): {}",
        short(&plan.head),
        short(&plan.main),
        short(&plan.base),
        standing.join(", ")
    );
    if !plan.host_only && plan.container != plan.host {
        line.push_str(&format!(
            "; the container: {}",
            scope(&plan.container).join(", ")
        ));
    }
    line.push('\n');
    line
}

/// One step of [`describe`]: where it stands, its side, and its key.
fn step_row(r: &Required, keys: bool) -> String {
    let standing = if r.step.always {
        "always"
    } else if r.cached {
        "cached"
    } else {
        "run"
    };
    let side = match r.step.side {
        Side::Host => "host",
        Side::Linux => "linux",
    };
    let mut row = format!("  {standing:<6} [{side:<5}] {}", r.step.id);
    if let Some(why) = &r.step.unkeyed {
        row.push_str(&format!("  unkeyed: {why}"));
    } else if keys && !r.step.always {
        row.push_str(&format!("  key {}", r.key));
    }
    row.push('\n');
    row
}

#[cfg(test)]
mod tests {
    use super::parse_ls_tree;

    /// A directory answers with a fingerprint, a file with its blob, and
    /// Markdown not at all.
    #[test]
    fn a_tree_listing_answers_for_files_and_directories_alike() {
        let listing = "040000 tree 1111111111111111111111111111111111111111\tcrates\0\
            100644 blob 2222222222222222222222222222222222222222\tcrates/a.rs\0\
            100644 blob 3333333333333333333333333333333333333333\tinternal-docs/規約.md\0";
        let ids = parse_ls_tree(listing.as_bytes());
        assert_eq!(ids.len(), 2);
        assert!(ids["crates"].starts_with("files:"));
        assert_eq!(
            ids["crates/a.rs"],
            "2222222222222222222222222222222222222222"
        );
        assert!(!ids.contains_key("internal-docs/規約.md"));
        assert!(parse_ls_tree(b"").is_empty());
    }
}
