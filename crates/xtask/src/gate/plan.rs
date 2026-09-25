//! What a branch owes main, worked out before anything runs: the reach of
//! its diff, the steps that reach selects, and which of those are already
//! green for the inputs they read.

mod steps;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::census::{self, Census};
use super::graph::{self, stem_of};
use super::record::Spent;
use super::stamp::Store;
use super::tiers::Tiers;
use crate::subprocess::{git_query, run_captured};
use steps::select;
pub(crate) use steps::tested_on_linux;

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
    /// Runs on every gate regardless of the diff, and is never cached:
    /// the seconds these take are not worth a stamp.
    pub always: bool,
    /// A verify-ui run, which builds the release unless told not to. The
    /// runner tells it not to once an earlier verb of the same side has
    /// built in this very invocation (a cached step's build may have
    /// happened in another tree).
    pub builds_app: bool,
    /// Reads the release the side's verbs build — a verb, or `bare` — and
    /// so runs in the side's built-app group, beside the checks that
    /// build elsewhere (`gate::sides::side`).
    pub release: bool,
    pub command: Vec<String>,
    /// What the step reads, as workspace paths (a directory covers
    /// everything under it): the cache key is their object ids.
    pub inputs: BTreeSet<String>,
}

/// One step the branch owes, with its key in this tree and whether a
/// stamp already answers for it.
pub(crate) struct Required {
    pub step: Step,
    /// Empty for an always-step, which is never cached.
    pub key: String,
    pub cached: bool,
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
    /// Every step asked for again whether or not a stamp answers for it.
    /// Kept past the selection because the run has one more chance to
    /// find a step already green — another tree's, stamped while this
    /// one queued — and a fresh run runs it anyway (`gate::step::run_one`).
    pub fresh: bool,
    /// Every file in the tree counted as reached, and why: `--all`, or the
    /// build input that changed.
    pub everything: Option<String>,
    pub changed: Vec<String>,
    pub reach: BTreeSet<String>,
    pub required: Vec<Required>,
    /// QML components the change reaches that no verb's census names —
    /// nothing headless shows them, so the gate cannot pass them.
    pub uncovered: Vec<String>,
    /// Changed files nothing reads and no step covers — said out loud,
    /// so a kind of file nothing tests is visible in the gate's
    /// report.
    pub unclaimed: Vec<String>,
    /// What the graph says is wrong with the tree itself
    /// (`graph::complaints`): a path resolving nowhere leaves every
    /// selection short by whatever that edge carried, so no run over this
    /// tree can be stamped.
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
/// its runs read. Spelled in pieces for the reason [`qml_dirs`] gives: a
/// whole path in a string here would be an edge from this file to
/// everything under it, and this file reads none of them — it names them
/// as what a step's cache key has to cover, which is a different thing
/// from reading them. Left whole, a change to any source of the core made
/// this file its reader, and through it every test of the task runner.
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
/// The dependency policy, which nothing in the source graph reads.
const DENY: &str = "deny.toml";
/// What every cargo build reads.
const CARGO: [&str; 3] = ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml"];
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

/// The two commits, the base between them, and the diff.
///
/// quotepath off: a non-ASCII name would otherwise come back
/// octal-escaped in quotes and match no file. Renames off: with them on,
/// a file moved is listed under its new name alone, and the old one —
/// the path every reader still names — is never seen to have gone
/// ([`gone_source`]).
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

/// The plan, with every phase of the making of it timed into `spent`:
/// the graph off the sources, the one listing of the tree the cache keys
/// are made of, the census, and the rest.
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
    // Keep documents in the reported diff, but only executable inputs
    // select tests, including under asset and harness directories.
    let executable_changes: Vec<String> = changed
        .iter()
        .filter(|file| !graph::is_markdown(file))
        .cloned()
        .collect();
    let everything = if ask.all {
        Some("--all".to_string())
    } else if let Some(input) = executable_changes.iter().find(|f| moves_everything(f)) {
        Some(input.clone())
    } else {
        executable_changes
            .iter()
            .find(|f| gone_source(dir, f))
            .map(|f| format!("{f} is gone"))
    };
    // What the change itself reaches is what has to be shown; a build
    // input widens only what runs.
    let touched = g.reach(&executable_changes);
    let reach = if everything.is_some() {
        // Every node of the graph, and every QML file whether or not it
        // is one: a component that names nothing — or names only what
        // is gone — has no edge, and so no key here, and the verbs whose
        // census names it alone would be the ones "everything" missed.
        let mut every: BTreeSet<String> = g
            .deps
            .keys()
            .chain(g.rdeps.keys())
            .chain(g.modules.keys())
            .filter(|f| !f.ends_with('/'))
            .cloned()
            .collect();
        every.extend(graph::qml_files(dir)?);
        every
    } else {
        touched.clone()
    };
    // waits(measured): the phase's cost, for the record
    let at = std::time::Instant::now();
    let census = Census::load(dir)?;
    let tiers = Tiers::load(dir);
    let worn = census::worn_by(dir);
    spent.census = at.elapsed();
    let read = Reading {
        dir,
        census: &census,
        tiers: &tiers,
        worn: &worn,
        whole: everything.is_some(),
    };
    let (
        steps,
        Counted {
            shadow: verbs_in_shadow,
            left: verbs_left,
        },
    ) = select(&g, &read, &reach, &executable_changes, ask);
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
        everything,
        changed,
        reach,
        required,
        uncovered,
        unclaimed,
        complaints: graph::complaints(&g),
        verbs_in_shadow,
        verbs_left,
    })
}

/// Each step with the stamp that already answers for it, the other side's
/// dropped when only this one was asked for. An always-step carries no key:
/// the seconds it takes are not worth one.
///
/// The object ids the keys are made of come from one listing of the
/// tree: a whole plan is
/// hundreds of steps with dozens of inputs each, and a process for every
/// pair is minutes of every gate spent starting git before the first
/// step runs — more under the load of other seats gating beside it.
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

/// The object id of every path in the tree at `rev`, directories
/// included (a directory's is its tree's), from one `git ls-tree`.
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

/// `ls-tree -z`: `<mode> <type> <id>\t<path>` per entry, NUL after each.
/// `-z` so that a path is spelled as it is (a non-ASCII name otherwise
/// comes back octal-escaped in quotes).
fn parse_ls_tree(listing: &[u8]) -> BTreeMap<String, String> {
    super::inputs::from_listing(listing)
}

/// A file every build reads: a change to it is a change to everything,
/// and no graph of the sources can say otherwise.
fn moves_everything(file: &str) -> bool {
    matches!(
        file,
        "Cargo.toml" | "Cargo.lock" | "rust-toolchain.toml" | "clippy.toml" | DOCKERFILE
    ) || file.starts_with(".cargo/")
        || (file.starts_with("crates/")
            && (file.ends_with("/Cargo.toml") || file.ends_with("/build.rs")))
        // What the app's build script links in: no source names it.
        || file.starts_with("crates/platitude-app/assets/")
}

/// A source the tree no longer holds, whose readers the graph cannot
/// name: the graph is read off the tree, and a `use` or a QML type name
/// still pointing at the file resolves to nothing.
/// Its reach would be the file alone — no crate entered, so no clippy,
/// no test, no verb — and a module deleted with another crate still
/// naming it would be stamped green. Everything is the one reach that
/// cannot miss the reader.
fn gone_source(dir: &Path, file: &str) -> bool {
    (file.ends_with(".rs") || file.ends_with(".qml") || file.ends_with("/qmldir"))
        && !gone_test(file)
        && !dir.join(file).exists()
}

/// A test taken out has no reader the graph could miss. A module under
/// `tests/` is named by nothing outside its binary, and the root or
/// `mod.rs` declaring it moves with it — a declaration left behind is
/// one `fmt` refuses to resolve — which reaches that binary whole. A
/// QtTest file is named by nothing at all, and `qmltest_steps` reads it
/// off the reach by path, where the changed file stands whether or not
/// the tree holds it.
fn gone_test(file: &str) -> bool {
    file.contains("/tests/") && (file.ends_with(".rs") || stem_of(file).starts_with("tst_"))
}

fn under(file: &str, input: &str) -> bool {
    file == input || file.starts_with(&format!("{}/", input.trim_end_matches('/')))
}

/// The steps the reach selects, host first: the always-steps, clippy per
/// crate entered, the tests in the reach, and the app as a built thing.
/// What a selection reads off the tree besides the reach: the tree
/// itself, the census of what each verb shows, who wears whom
/// (`census::worn_by`), and whether the reach is everything.
struct Reading<'a> {
    dir: &'a Path,
    census: &'a Census,
    /// Which of the census's lines each gate owes, and where (`tiers`).
    tiers: &'a Tiers,
    worn: &'a BTreeMap<String, BTreeSet<String>>,
    whole: bool,
}

/// What the verb selection counted beside the steps it made.
struct Counted {
    /// The narrower candidate count ([`verbs_in_snapshot`]).
    shadow: usize,
    /// Selected lines left to the full gate.
    left: usize,
}

/// QML components in the reach that stand in the item tree and no verb's
/// census names — the wearers counting for the worn (`census::worn_by`).
fn uncovered(
    dir: &Path,
    census: &Census,
    reach: &BTreeSet<String>,
    worn: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<String> {
    let (_, qml_tests) = qml_dirs();
    reach
        .iter()
        .filter(|f| f.ends_with(".qml"))
        // A QtTest file stands in a runner of its own
        // (`qmltest_steps` is what shows it).
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
/// security claim (the same hash `linux::image_tag` uses).
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

/// The plan as text: what changed, what that reaches, what it owes, and
/// what is already green.
pub(crate) fn describe(plan: &Plan) -> String {
    let short = |sha: &str| sha.chars().take(10).collect::<String>();
    let mut out = format!(
        "gate: HEAD {} against main {} (base {}): {}{}{}\n",
        short(&plan.head),
        short(&plan.main),
        short(&plan.base),
        if plan.onto_main {
            "on main"
        } else {
            "off main — a pseudo-run; land rebases and gates again"
        },
        if plan.host_only {
            ", host side only"
        } else {
            ""
        },
        match &plan.everything {
            Some(why) => format!(", everything ({why})"),
            None => String::new(),
        }
    );
    if plan.everything.is_some() {
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
    if plan.verbs_in_shadow > 0 || chosen > 0 {
        out.push_str(&format!(
            "verbs: {chosen} selected, {on_linux} of them on the container too; {} left to the \
             full gate ({}); shadow candidate: {} (final census only, not used to skip runs)\n",
            plan.verbs_left,
            super::tiers::FILE,
            plan.verbs_in_shadow
        ));
    }
    out.push_str(&format!("steps ({}):\n", plan.required.len()));
    for r in &plan.required {
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
        out.push_str(&format!("  {standing:<6} [{side:<5}] {}\n", r.step.id));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::parse_ls_tree;

    /// The listing is read the way `-z` writes it: trees and blobs alike,
    /// a path spelled whole however it is named, nothing for a line that
    /// is not an entry.
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
