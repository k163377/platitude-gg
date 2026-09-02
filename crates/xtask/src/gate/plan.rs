//! What a branch owes main, worked out before anything runs: the reach of
//! its diff, the steps that reach selects, and which of those are already
//! green for the inputs they read.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::census::{self, Census};
use super::graph::{self, Graph, stem_of};
use super::stamp::Store;
use crate::subprocess::git_query;

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
    /// built in this very invocation — never on the strength of a cached
    /// step, whose build may have happened in another tree.
    pub builds_app: bool,
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
    /// so a kind of file nothing tests is visible rather than silently
    /// green.
    pub unclaimed: Vec<String>,
}

pub(crate) struct Ask<'a> {
    pub main_ref: &'a str,
    pub host_only: bool,
    pub all: bool,
    pub fresh: bool,
    /// verify-ui lines to run besides the census's, as typed.
    pub extra_verbs: &'a [String],
}

const CORE: &str = "crates/platitude-core";
const APP: &str = "crates/platitude-app";
const DOCKERFILE: &str = "ci/linux/Dockerfile";
/// What every cargo build reads.
const CARGO: [&str; 3] = ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml"];
/// What the harness runs read besides the app: the runner and the demo
/// repositories it builds.
const HARNESS: [&str; 2] = ["crates/xtask/src/verify", "crates/xtask/src/demo"];

pub(crate) fn make(dir: &Path, ask: &Ask<'_>) -> Result<Plan, String> {
    let here = dir.display().to_string();
    let rev = |what: &str| {
        git_query(
            &here,
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
    let main = rev(ask.main_ref)?;
    let base = git_query(&here, &["merge-base", &main, &head]).ok_or("git merge-base failed")?;
    let onto_main = base == main;
    let g = graph::build(dir)?;
    // quotepath off: a non-ASCII name would otherwise come back
    // octal-escaped in quotes and match no file.
    let changed: Vec<String> = git_query(
        &here,
        &[
            "-c",
            "core.quotepath=false",
            "diff",
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
    let everything = if ask.all {
        Some("--all".to_string())
    } else {
        changed.iter().find(|f| moves_everything(f)).cloned()
    };
    // What the change itself reaches is what has to be shown; a build
    // input widens what runs, not what the census owes.
    let touched = g.reach(&changed);
    let reach = if everything.is_some() {
        g.deps
            .keys()
            .chain(g.modules.keys())
            .filter(|f| !f.ends_with('/'))
            .cloned()
            .collect()
    } else {
        touched.clone()
    };
    let census = Census::load(dir);
    let mut steps = select(&g, &census, &reach, &changed, ask, everything.is_some());
    let store = Store::open(dir)?;
    let mut required = Vec::new();
    for step in steps.drain(..) {
        if ask.host_only && step.side == Side::Linux {
            continue;
        }
        let (key, cached) = if step.always {
            (String::new(), false)
        } else {
            let key = cache_key(&here, &head, &step);
            let cached = !ask.fresh && store.step_green(&key);
            (key, cached)
        };
        required.push(Required { step, key, cached });
    }
    let uncovered = uncovered(dir, &census, &touched);
    let unclaimed = changed
        .iter()
        .filter(|file| {
            !required
                .iter()
                .any(|r| !r.step.always && r.step.inputs.iter().any(|i| under(file, i)))
        })
        .cloned()
        .collect();
    Ok(Plan {
        dir: dir.to_path_buf(),
        head,
        main,
        base,
        onto_main,
        host_only: ask.host_only,
        everything,
        changed,
        reach,
        required,
        uncovered,
        unclaimed,
    })
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

fn under(file: &str, input: &str) -> bool {
    file == input || file.starts_with(&format!("{}/", input.trim_end_matches('/')))
}

fn words(line: &[&str]) -> Vec<String> {
    line.iter().map(|w| (*w).to_string()).collect()
}

/// `cargo xtask <line>`, spelled unquieted the way `check` does so that
/// cargo's build-lock line reaches the log.
fn xtask(line: &[&str]) -> Vec<String> {
    let mut command = words(&["cargo", "run", "-p", "xtask", "--"]);
    command.extend(words(line));
    command
}

fn step<S: AsRef<str>>(
    id: &str,
    side: Side,
    always: bool,
    command: Vec<String>,
    inputs: &[S],
) -> Step {
    Step {
        id: id.to_string(),
        side,
        always,
        builds_app: false,
        command,
        inputs: inputs.iter().map(|s| s.as_ref().to_string()).collect(),
    }
}

/// The unit-test target of a package, for `cargo test -p <p> <target>`.
fn unit_target(package: &str) -> [&'static str; 2] {
    match package {
        "platitude-core" => ["--lib", ""],
        "platitude-app" => ["--bin", "platitude-gg"],
        _ => ["--bin", "xtask"],
    }
}

/// What every cargo build reads, and whatever else a step names.
fn cargo_inputs(extra: &[&str]) -> Vec<String> {
    CARGO
        .iter()
        .chain(extra.iter())
        .map(|s| (*s).to_string())
        .collect()
}

/// The same step for the container: `cargo <rest>` becomes `cargo xtask
/// linux <rest>`, and the Dockerfile joins its inputs.
fn on_linux(id: &str, command: &[String], inputs: &[String]) -> Step {
    let mut linux = xtask(&["linux"]);
    linux.extend(command[1..].iter().cloned());
    let mut linux_inputs = inputs.to_vec();
    linux_inputs.push(DOCKERFILE.to_string());
    step(
        &format!("{id} linux"),
        Side::Linux,
        false,
        linux,
        &linux_inputs,
    )
}

/// The reach, sorted for the steps: the Rust files per package, the files
/// with tests and their module filters per package, and per integration
/// binary (package, binary) its module filters and files.
#[derive(Default)]
struct Sorted {
    rust_in: BTreeMap<String, Vec<String>>,
    unit_filters: BTreeMap<String, BTreeSet<String>>,
    unit_files: BTreeMap<String, Vec<String>>,
    integration: BTreeMap<(String, String), (BTreeSet<String>, Vec<String>)>,
    /// The QML components in the reach, by name.
    qml: BTreeSet<String>,
}

/// The packages whose tests the container runs too: the ones that hold
/// code behind `cfg(not(windows))`, which the host never so much as
/// compiles. The app needs Qt in the image and is not among them, as it
/// was not in `check`.
fn tested_on_linux(package: &str) -> bool {
    matches!(package, "platitude-core" | "xtask")
}

/// `whole` runs every package's tests unfiltered — the reach is the
/// whole tree, and a filter naming every module says the same thing
/// less plainly.
fn sort(g: &Graph, reach: &BTreeSet<String>, whole: bool) -> Sorted {
    let mut sorted = Sorted::default();
    for file in reach {
        if file.ends_with(".qml") {
            sorted.qml.insert(stem_of(file));
            continue;
        }
        let Some(module) = g.modules.get(file) else {
            continue;
        };
        sorted
            .rust_in
            .entry(module.package.clone())
            .or_default()
            .push(file.clone());
        match &module.test_binary {
            None if module.has_tests => {
                let filter = if whole || module.path.is_empty() {
                    String::new()
                } else {
                    format!("{}::", module.path.join("::"))
                };
                sorted
                    .unit_filters
                    .entry(module.package.clone())
                    .or_default()
                    .insert(filter);
                sorted
                    .unit_files
                    .entry(module.package.clone())
                    .or_default()
                    .push(file.clone());
            }
            None => {}
            Some(binary) => {
                // main.rs and support/ are read by every module: the whole
                // binary. Otherwise the top-level module the file sits in.
                let filter = match module.path.first() {
                    Some(top) if !whole && top != "support" => format!("{top}::"),
                    _ => String::new(),
                };
                let entry = sorted
                    .integration
                    .entry((module.package.clone(), binary.clone()))
                    .or_default();
                entry.0.insert(filter);
                entry.1.push(file.clone());
            }
        }
    }
    sorted
}

/// The steps the reach selects, host first: the always-steps, clippy per
/// crate entered, the tests in the reach, and the app as a built thing.
fn select(
    g: &Graph,
    census: &Census,
    reach: &BTreeSet<String>,
    changed: &[String],
    ask: &Ask<'_>,
    whole: bool,
) -> Vec<Step> {
    let sorted = sort(g, reach, whole);
    let mut steps = always_steps();
    steps.extend(clippy_steps(&sorted));
    steps.extend(unit_steps(g, &sorted));
    steps.extend(it_steps(g, &sorted));
    steps.extend(binary_steps(census, &sorted, reach, changed, ask));
    steps
}

fn always_steps() -> Vec<Step> {
    vec![
        step(
            "structure",
            Side::Host,
            true,
            xtask(&["structure"]),
            &["crates", ".claude/rules-refs/structure.md"],
        ),
        step("waits", Side::Host, true, xtask(&["waits"]), &[CORE]),
        step(
            "fmt",
            Side::Host,
            true,
            words(&["cargo", "fmt", "--all", "--", "--check"]),
            &["crates"],
        ),
    ]
}

/// clippy for every crate the reach enters, on both sides: the host's
/// cannot answer for the names behind `cfg(not(windows))`.
fn clippy_steps(sorted: &Sorted) -> Vec<Step> {
    let mut steps = Vec::new();
    for package in sorted.rust_in.keys() {
        let crate_dir = format!("crates/{package}");
        let mut clippy = words(&["cargo", "clippy", "-p", package]);
        clippy.extend(words(&[
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ]));
        let inputs = cargo_inputs(&[&crate_dir, "clippy.toml", ".cargo"]);
        let id = format!("clippy {package}");
        steps.push(step(&id, Side::Host, false, clippy.clone(), &inputs));
        let mut linux = xtask(&["linux"]);
        linux.extend(clippy[1..].iter().cloned());
        let mut linux_inputs = inputs.clone();
        linux_inputs.push(DOCKERFILE.to_string());
        steps.push(step(
            &format!("clippy-linux {package}"),
            Side::Linux,
            false,
            linux,
            &linux_inputs,
        ));
    }
    steps
}

/// The unit tests in the reach, per package, by module filter; the core's
/// on the container too. Their inputs are what the selected files read.
fn unit_steps(g: &Graph, sorted: &Sorted) -> Vec<Step> {
    let mut steps = Vec::new();
    for (package, filters) in &sorted.unit_filters {
        let files = sorted.unit_files.get(package).cloned().unwrap_or_default();
        let mut inputs: Vec<String> = g.inputs(&files).into_iter().collect();
        inputs.extend(cargo_inputs(&[]));
        let all = filters.contains("");
        let target = unit_target(package);
        let mut command = words(&["cargo", "test", "-p", package, target[0]]);
        if !target[1].is_empty() {
            command.push(target[1].to_string());
        }
        if !all {
            command.push("--".to_string());
            command.extend(filters.iter().cloned());
        }
        let id = if all {
            format!("test {package} (all)")
        } else {
            format!("test {package} {}", filters.len())
        };
        steps.push(step(&id, Side::Host, false, command.clone(), &inputs));
        if tested_on_linux(package) {
            steps.push(on_linux(&id, &command, &inputs));
        }
    }
    steps
}

/// Each integration binary's modules in the reach, on both sides where
/// the package is.
fn it_steps(g: &Graph, sorted: &Sorted) -> Vec<Step> {
    let mut steps = Vec::new();
    for ((package, binary), (filters, files)) in &sorted.integration {
        let mut inputs: Vec<String> = g.inputs(files).into_iter().collect();
        inputs.extend(cargo_inputs(&[]));
        let all = filters.contains("");
        let mut command = words(&["cargo", "test", "-p", package, "--test", binary]);
        if !all {
            command.push("--".to_string());
            command.extend(filters.iter().cloned());
        }
        let id = if all {
            format!("test {binary} (all)")
        } else {
            format!("test {binary} {}", filters.len())
        };
        steps.push(step(&id, Side::Host, false, command.clone(), &inputs));
        if tested_on_linux(package) {
            steps.push(on_linux(&id, &command, &inputs));
        }
    }
    steps
}

/// The app as a built thing: shipped when its QML or entry point moved,
/// the verify-ui verbs whose census names a reached component (and the
/// ones asked for), bare when the binary moved at all. The first verb of
/// each side builds the release; the rest reuse it (`check` does the
/// same).
fn binary_steps(
    census: &Census,
    sorted: &Sorted,
    reach: &BTreeSet<String>,
    changed: &[String],
    ask: &Ask<'_>,
) -> Vec<Step> {
    let mut steps = Vec::new();
    // Spelled in pieces: a whole path in a string here would be read as
    // this file reading the app's entry point.
    let entry = format!("{APP}/src/main.rs");
    let qml_moved = !sorted.qml.is_empty() || reach.contains(&entry);
    let binary_moved = qml_moved
        || sorted.rust_in.contains_key("platitude-app")
        || sorted.rust_in.contains_key("platitude-core");
    let binary_inputs = cargo_inputs(&[APP, CORE]);
    if qml_moved {
        steps.push(step(
            "shipped",
            Side::Host,
            false,
            xtask(&["shipped"]),
            &binary_inputs,
        ));
    }
    let mut lines: Vec<String> = census.verbs_touching(&sorted.qml);
    for extra in ask.extra_verbs {
        if !lines.contains(extra) {
            lines.push(extra.clone());
        }
    }
    let mut verb_inputs = binary_inputs.clone();
    verb_inputs.extend(HARNESS.iter().map(|s| (*s).to_string()));
    // `--no-census`: a run the gate asks for is a re-run of a recorded
    // line, and writing the census back mid-gate would dirty the tree
    // under the stamp (and the seat `land` is about to hand back). The
    // census grows from the runs somebody types.
    for line in &lines {
        let verb_words: Vec<&str> = line.split_whitespace().collect();
        let mut host = xtask(&["verify-ui"]);
        host.extend(words(&verb_words));
        host.push("--no-census".to_string());
        let mut host_step = step(
            &format!("verify {line}"),
            Side::Host,
            false,
            host,
            &verb_inputs,
        );
        host_step.builds_app = true;
        steps.push(host_step);
        let mut linux = xtask(&["linux", "verify-ui"]);
        linux.extend(words(&verb_words));
        linux.push("--no-census".to_string());
        let mut linux_inputs = verb_inputs.clone();
        linux_inputs.push(DOCKERFILE.to_string());
        let mut linux_step = step(
            &format!("verify-linux {line}"),
            Side::Linux,
            false,
            linux,
            &linux_inputs,
        );
        linux_step.builds_app = true;
        steps.push(linux_step);
    }
    if binary_moved || changed.iter().any(|f| f == DOCKERFILE) {
        let mut bare_inputs = binary_inputs;
        bare_inputs.extend(["crates/xtask/src/linux", DOCKERFILE].map(String::from));
        steps.push(step(
            "bare",
            Side::Linux,
            false,
            xtask(&["linux", "bare"]),
            &bare_inputs,
        ));
    }
    steps
}

/// QML components in the reach that stand in the item tree and no verb's
/// census names.
fn uncovered(dir: &Path, census: &Census, reach: &BTreeSet<String>) -> Vec<String> {
    reach
        .iter()
        .filter(|f| f.ends_with(".qml"))
        .filter(|f| census::instantiable(dir, f))
        .filter(|f| !census.covers(&stem_of(f)))
        .cloned()
        .collect()
}

/// The cache key: the step's identity and command, and the object id of
/// each input in the tree under test. FNV-1a, a fingerprint and not a
/// security claim (the same hash `linux::image_tag` uses).
fn cache_key(here: &str, head: &str, step: &Step) -> String {
    let mut text = step.id.clone();
    text.push('\0');
    text.push_str(&step.command.join("\0"));
    text.push('\n');
    for input in &step.inputs {
        let path = input.trim_end_matches('/');
        let oid = git_query(
            here,
            &[
                "rev-parse",
                "--verify",
                "--quiet",
                &format!("{head}:{path}"),
            ],
        )
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "absent".to_string());
        text.push_str(path);
        text.push('=');
        text.push_str(&oid);
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
