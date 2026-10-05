//! What a branch owes main, worked out before anything runs: the reach of
//! its diff, the steps that reach selects, and which of those are already
//! green for the inputs they read.

mod pins;
mod steps;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::census::{self, Census};
use super::graph::{self, Carried, Reach, stem_of};
use super::record::Spent;
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
    pub command: Vec<String>,
    /// What the step reads, as workspace paths (a directory covers
    /// everything under it): the cache key is their object ids.
    pub inputs: BTreeSet<String>,
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
    /// Every file in the tree counted as reached, and why: `--all`, a
    /// moved version, or the build input that changed.
    pub everything: Option<String>,
    /// The full tier's steps (`periodic`, every census line): `--all`, or
    /// a moved version ([`pins`]) before a merge.
    pub full: bool,
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
    /// QML components the change reaches that no verb's census names —
    /// nothing headless shows them, so the gate cannot pass them.
    pub uncovered: Vec<String>,
    /// Changed files no step reads, listed so that a kind of file nothing
    /// tests shows in the report.
    pub unclaimed: Vec<String>,
    /// What the graph says is wrong with the tree itself
    /// (`graph::complaints`); no run over such a tree can be stamped.
    pub complaints: Vec<String>,
    /// Candidate count from the final census, for comparison only.
    /// A component absent at the end may have been exercised earlier.
    pub verbs_in_shadow: usize,
    /// Census lines the reach selected that this gate leaves to the full
    /// one (`tiers`) — before a merge only; the full gate leaves none.
    pub verbs_left: usize,
}

pub(crate) struct Ask<'a> {
    pub main_ref: &'a str,
    pub host_only: bool,
    pub all: bool,
    pub fresh: bool,
    /// verify-ui lines to run besides the census's, as typed.
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
/// What a QML test run reads besides [`qml_dirs`]: the recipe that
/// stages the module for it.
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
    // Documents stay in the reported diff but select no tests.
    let executable_changes: Vec<String> = changed
        .iter()
        .filter(|file| !graph::is_markdown(file))
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
    let census = Census::load(dir)?;
    let tiers = Tiers::load(dir)?;
    let worn = census::worn_by(dir);
    spent.census = at.elapsed();
    let read = |scope: &Scope| Reading {
        dir,
        census: &census,
        tiers: &tiers,
        worn: &worn,
        whole: scope.everything.is_some(),
        full: scope.full,
    };
    let (
        mut steps,
        Counted {
            shadow: verbs_in_shadow,
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
    let uncovered = uncovered(dir, &census, &touched, &worn);
    let unclaimed = changed
        .iter()
        .filter(|file| {
            !required
                .iter()
                .any(|r| !r.step.always && r.step.inputs.iter().any(|i| under(file, i)))
        })
        .cloned()
        .collect();
    spent.plan_rest = started
        .elapsed()
        .saturating_sub(spent.graph + spent.census + spent.ids);
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
        verbs_in_shadow,
        verbs_left,
    })
}

/// The reach of everything: every node, and every QML file — a component
/// with no edge is no node, and the verbs only its census line names
/// would be missed. All handed whole: the change is the build itself.
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
            let (key, cached) = if step.always {
                (String::new(), false)
            } else {
                let key = cache_key(&ids, &step);
                let cached = !ask.fresh && store.step_green(&key);
                (key, cached)
            };
            Required { step, key, cached }
        })
        .collect())
}

/// The object id of every path in the tree at `rev` from one `git ls-tree`;
/// a directory's is a fingerprint of its entries besides Markdown
/// (`inputs::from_listing`).
fn tree_ids(here: &str, rev: &str) -> Result<BTreeMap<String, String>, String> {
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
    Ok(parse_ls_tree(&output.stdout))
}

/// `ls-tree -z`: `<mode> <type> <id>\t<path>` per entry, NUL after each
/// (`-z`, or a non-ASCII name comes back octal-escaped in quotes).
fn parse_ls_tree(listing: &[u8]) -> BTreeMap<String, String> {
    super::inputs::from_listing(listing)
}

/// A file every build reads and no graph of the sources sees: a change to
/// it is a change to everything. clippy's configuration is clippy's
/// (`steps::select`), and the container's image its own side's
/// ([`scopes`]).
fn moves_everything(file: &str) -> bool {
    matches!(file, "Cargo.toml" | "Cargo.lock" | "rust-toolchain.toml")
        || file.starts_with(".cargo/")
        || under(file, VENDOR)
        || (file.starts_with("crates/")
            && (file.ends_with("/Cargo.toml") || file.ends_with("/build.rs")))
        // What the app's build script links in: no source names it.
        || file.starts_with("crates/platitude-app/assets/")
}

/// What each side owes beyond the reach: the host's, and the container's.
/// The daily tier stays the host's quick half; stage 2 owes the full tier
/// for a new version, on the side built with it ([`pins`]).
fn scopes(
    dir: &Path,
    ask: &Ask<'_>,
    here: &str,
    base: &str,
    head: &str,
    changed: &[String],
) -> (Scope, Scope) {
    if ask.all {
        let all = Scope {
            everything: Some("--all".to_string()),
            full: true,
        };
        return (all.clone(), all);
    }
    let pinned = if ask.host_only {
        None
    } else {
        pins::moved(here, base, head, changed)
    };
    let host = match &pinned {
        Some(moved) if !moved.container_only => Scope {
            everything: Some(moved.why.clone()),
            full: true,
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
            full: false,
        },
    };
    let container = match pinned {
        Some(moved) if !host.full => Scope {
            everything: Some(moved.why),
            full: true,
        },
        _ if host.everything.is_none() => Scope {
            everything: changed
                .iter()
                .find(|f| IMAGE.contains(&f.as_str()))
                .map(|f| format!("{f} builds its image")),
            full: false,
        },
        _ => host.clone(),
    };
    (host, container)
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
    let touched = g.reach_on(changed, std::env::consts::OS);
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
        _ => g.reach_on(changed, CONTAINER_OS),
    };
    Ok(Reaches {
        touched,
        host: on_the_host,
        container: on_the_container,
    })
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
    census: &'a Census,
    /// Which of the census's lines each gate owes, and where (`tiers`).
    tiers: &'a Tiers,
    worn: &'a BTreeMap<String, BTreeSet<String>>,
    whole: bool,
    /// Owes the full tier ([`Scope::full`]).
    full: bool,
}

/// What the verb selection counted beside the steps it made.
struct Counted {
    /// The narrower candidate count (`steps::verbs_in_snapshot`).
    shadow: usize,
    /// Selected lines left to the full gate.
    left: usize,
}

/// QML components in the reach that stand in the item tree and no verb's
/// census names — the wearers counting for the worn (`census::worn_by`).
fn uncovered(
    dir: &Path,
    census: &Census,
    reach: &Reach,
    worn: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<String> {
    let (_, qml_tests) = qml_dirs();
    reach
        .keys()
        .filter(|f| f.ends_with(".qml"))
        // A QtTest file is shown by its own runner (`qmltest_steps`).
        .filter(|f| !under(f, &qml_tests))
        .filter(|f| census::instantiable(dir, f))
        .filter(|f| !shown_as(&stem_of(f), worn).iter().any(|s| census.covers(s)))
        .cloned()
        .collect()
}

/// The names a run could have met this component under: its own, and
/// every wearer's — an item declared as the wearer is one of these too.
fn shown_as(stem: &str, worn: &BTreeMap<String, BTreeSet<String>>) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    census::through_wearers(stem, worn, &mut names);
    names
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
        out.push_str("no verb's census names these — nothing headless shows them:\n");
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
    if plan.verbs_in_shadow > 0 || chosen > 0 || on_linux > 0 {
        // The container's lines are the host's own unless its side owes
        // more than the host's does; what waits for the full gate is the
        // host's count.
        let (container, left) = if plan.container == plan.host {
            (format!("{on_linux} of them on the container too"), "")
        } else {
            (format!("{on_linux} on the container"), " on the host")
        };
        out.push_str(&format!(
            "verbs: {chosen} selected, {container}; {} left{left} to the full gate ({}); shadow \
             candidate: {} (final census only, not used to skip runs)\n",
            plan.verbs_left,
            super::tiers::FILE,
            plan.verbs_in_shadow
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
        if scope.full {
            said.push("the full tier".to_string());
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
    if keys && !r.step.always {
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
