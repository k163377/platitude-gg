//! The steps a reach selects: how each is spelled, the reach sorted
//! by package and binary, and the selection itself.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::{
    Ask, CARGO, Counted, DOCKERFILE, DOCS, IMAGE, Keyed, QMLTEST, Reading, Side, Step, app,
    clippy_config, core, deny_policy, harness, qml_dirs, under,
};
use crate::gate::graph::{Carried, Graph, Reach, stem_of};
use crate::gate::tiers::Tier;

fn words(line: &[&str]) -> Vec<String> {
    line.iter().map(|w| (*w).to_string()).collect()
}

/// `cargo xtask <line>` without the alias's `--quiet`, so cargo's
/// build-lock line reaches the log; `--locked`
/// (反映前テストの機械化.md「gate と check が撃つ cargo は全部 `--locked`」).
fn xtask(line: &[&str]) -> Vec<String> {
    let mut command = words(&["cargo", "run", "--locked", "-p", "xtask", "--"]);
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
        release: false,
        beside: false,
        command,
        inputs: inputs.iter().map(|s| s.as_ref().to_string()).collect(),
        keyed: Keyed::Bytes,
        contract: String::new(),
        unkeyed: None,
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

/// The same step for the container.
fn on_linux(id: &str, command: &[String], inputs: &[String]) -> Step {
    let mut linux = xtask(&["linux"]);
    linux.extend(command[1..].iter().cloned());
    let mut linux_inputs = inputs.to_vec();
    linux_inputs.extend(IMAGE.map(String::from));
    step(
        &format!("{id} linux"),
        Side::Linux,
        false,
        linux,
        &linux_inputs,
    )
}

/// The reach, sorted for the steps; `integration` is keyed by (package,
/// test binary) and holds its module filters and files.
#[derive(Default)]
struct Sorted {
    rust_in: BTreeMap<String, Vec<String>>,
    /// The packages the change reaches as code (`Carried::Whole`): what
    /// clippy is owed for. A package the product reaches only as data a
    /// tool reads is in `rust_in` and not here — its code did not change.
    rust_moved: BTreeSet<String>,
    unit_filters: BTreeMap<String, BTreeSet<String>>,
    unit_files: BTreeMap<String, Vec<String>>,
    integration: BTreeMap<(String, String), (BTreeSet<String>, Vec<String>)>,
    /// The QML components in the reach, by name — the product's and the
    /// harness's.
    qml: BTreeSet<String>,
}

/// `everything` runs every package's tests unfiltered.
fn sort(g: &Graph, reach: &Reach, everything: bool) -> Sorted {
    let (_, qml_tests) = qml_dirs();
    let mut sorted = Sorted::default();
    for (file, carried) in reach {
        if file.ends_with(".qml") {
            // A QtTest file is no component: no line shows it, so it
            // belongs to `qmltest_steps` alone.
            if !under(file, &qml_tests) {
                sorted.qml.insert(stem_of(file));
            }
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
        if *carried == Carried::Whole {
            sorted.rust_moved.insert(module.package.clone());
        }
        match &module.test_binary {
            None if module.has_tests => {
                let filter = if everything || module.path.is_empty() {
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
                // binary.
                let filter = match module.path.first() {
                    Some(top) if !everything && top != "support" => format!("{top}::"),
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

pub(super) fn select(
    g: &Graph,
    read: &Reading<'_>,
    reach: &Reach,
    changed: &[String],
    ask: &Ask<'_>,
) -> (Vec<Step>, Counted) {
    let mut sorted = sort(g, reach, read.everything);
    let mut steps = always_steps();
    steps.extend(deny_steps(g, changed, read.everything));
    steps.extend(qmltest_steps(
        &qmltest_reads(g, read.dir),
        reach,
        changed,
        read.everything,
    ));
    // Its configuration is clippy's alone, read by every crate's run and
    // by nothing a test or a build reads.
    if changed.contains(&clippy_config()) {
        sorted
            .rust_moved
            .extend(g.modules.values().map(|module| module.package.clone()));
    }
    steps.extend(clippy_steps(&sorted));
    steps.extend(unit_steps(g, &sorted));
    steps.extend(it_steps(g, &sorted));
    if read.whole_tier {
        steps.extend(periodic_steps(g, read.dir, &sorted));
    }
    let (binary, counted) = binary_steps(read, &sorted, reach, changed, ask);
    steps.extend(binary);
    // Behind the lines in the release block: the first line builds the
    // release alone, and the checks, each a handful of runs of their own,
    // reuse it beside the rest (`sides::verbs`).
    steps.extend(wedge_steps(reach, read.everything));
    steps.extend(shot_steps(g, reach, &sorted, read.everything));
    steps.extend(records_steps(g, reach, &sorted, read));
    (steps, counted)
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
        step("waits", Side::Host, true, xtask(&["waits"]), &["crates"]),
        // CI's directory in pieces, for the reason `versions` gives.
        step(
            "versions",
            Side::Host,
            true,
            xtask(&["versions"]),
            &[format!(".{}", "github"), DOCKERFILE.to_string()],
        ),
        step("docs", Side::Host, true, xtask(&["docs"]), &DOCS),
        // A form the table adds or changes is read against git's
        // releases (`relied-messages audit`) before it stands.
        step(
            "relied-messages",
            Side::Host,
            true,
            xtask(&["relied-messages", "check"]),
            &[
                "crates/platitude-core/src/relied_messages.rs",
                crate::relied_messages::LEDGER,
            ],
        ),
        // Fails on a registered line no verb answers (the app would ignore
        // the name and the run wait out its ceiling silently), on a verb
        // or a judgment row no line reaches, and on a row that names what
        // is no component (`verify::coverage`).
        step("verbs", Side::Host, true, xtask(&["verbs"]), &["crates"]),
        step(
            "fmt",
            Side::Host,
            true,
            words(&["cargo", "fmt", "--all", "--", "--check"]),
            &["crates"],
        ),
    ]
}

/// cargo-deny over the resolved graph, owed by a change to what it reads
/// — the policy, which nothing in the source graph reads, and every way
/// the closure moves. Ahead of every build, so a forbidden crate is said
/// in seconds.
///
/// Host only: the policy names no `targets` and takes `all-features`, so
/// every OS reads the same crates.
fn deny_steps(g: &Graph, changed: &[String], everything: bool) -> Vec<Step> {
    // The manifests carry what the lock does not: a license field and the
    // features a dependency is taken with — a patched-in crate's among
    // them; cargo's configuration ([`CARGO`]) can patch a dependency or
    // replace its source.
    let mut inputs: BTreeSet<String> = CARGO.into_iter().map(str::to_string).collect();
    inputs.insert(deny_policy());
    inputs.extend(
        g.modules
            .values()
            .map(|module| format!("crates/{}/Cargo.toml", module.package)),
    );
    if !everything && !changed.iter().any(|f| inputs.iter().any(|i| under(f, i))) {
        return Vec::new();
    }
    let inputs: Vec<String> = inputs.into_iter().collect();
    vec![step("deny", Side::Host, false, xtask(&["deny"]), &inputs)]
}

/// The xtask binary's entry: what dispatches each command to its module,
/// which no edge of the graph names (a `mod` declaration is none). Spelled
/// in pieces for the reason [`record_of_a_wedge`]'s files are — and a
/// crate root has no readers.
fn xtask_main() -> String {
    format!("{}/{}", "crates/xtask/src", "main.rs")
}

/// The module that starts a run in the container, spelled in pieces for
/// the reason [`record_of_a_wedge`]'s files are.
fn launcher() -> String {
    format!("{}/{}", "crates/xtask/src", "linux")
}

/// What a QML test run reads, by the boundary it reaches the run across —
/// each a set of the step's inputs.
#[derive(Default)]
struct QmlTestReads {
    /// The runner as cargo builds and runs it: the binary's entry that hands
    /// the run to it ([`xtask_main`]), and the static closure of its module
    /// as built (`Graph::built_inputs` of [`QMLTEST`], no read its tests
    /// alone make) — the staging and what it stages from, Qt's
    /// selection (the `PATH` that finds `qmltestrunner`, the offscreen
    /// fonts' variable) and the pin it holds the Qt to —, and what cargo
    /// builds it from ([`CARGO`]), whose `.cargo` `[env]` the run inherits.
    runner: BTreeSet<String>,
    /// Across the `qmltestrunner` process: what the runner stages and hands
    /// it — the product's module and its qmldir, the tests' directory with
    /// their stubs and helpers ([`qml_dirs`]) — and what the tests load off
    /// the tree past that, by relative import and as far as it leads
    /// (`imports::loaded_from`: the harness's module).
    process: BTreeSet<String>,
    /// Across the container, the Linux side's alone: the image the run
    /// starts in ([`IMAGE`] — the Dockerfile's `ENV`: the Qt on `PATH`,
    /// `QMAKE`, `LD_LIBRARY_PATH`, `LANG`; the pin its Qt is built at) and
    /// the module that builds it, starts the run in it and hands the run
    /// its environment, mounts and command line ([`launcher`]): the
    /// module's own files its build compiles. Past the module its closure
    /// queues, announces and records the run, and what it names across
    /// (the marks of a ticket and an announcement) no QML test reads.
    container: BTreeSet<String>,
    /// Why what the tests load cannot all be named, where it cannot
    /// (`imports::Loaded::unbounded`): no key answers for a run of them.
    unbounded: Option<String>,
}

fn qmltest_reads(g: &Graph, dir: &Path) -> QmlTestReads {
    let mut runner = g.built_inputs(&[QMLTEST.to_string()]);
    runner.insert(xtask_main());
    runner.extend(cargo_inputs(&[]));
    let (ui, tests) = qml_dirs();
    let loaded = super::imports::loaded_from(dir, std::slice::from_ref(&tests));
    let mut process = loaded.paths;
    process.extend([ui, tests]);
    let module = launcher();
    let entry = format!("{module}.rs");
    // A directory node is another reader's data, not a file compiled in.
    let mut container: BTreeSet<String> = g
        .built_inputs(std::slice::from_ref(&entry))
        .into_iter()
        .filter(|file| !file.ends_with('/') && (*file == entry || under(file, &module)))
        .collect();
    container.extend(IMAGE.map(String::from));
    QmlTestReads {
        runner,
        process,
        container,
        unbounded: loaded.unbounded,
    }
}

/// The QtTest files, owed by a change to anything a run of them reads
/// ([`QmlTestReads`]) and keyed on all of it. Both sides: the two Qt
/// builds paint through different stacks, and these ask when a Canvas has
/// painted.
///
/// Owed by what the change touched, not by what it reaches: a file read
/// only by what the runner's tests read reaches the runner in the source
/// graph and is no input of a run. The reach says only whether the side
/// compiles a touched file. `everything` selects it outright: the tests
/// are no nodes of the source graph.
///
/// Keyed on the Rust's code and every other file's bytes (`Keyed`): the
/// runner reads each test's text for its cases (`crate::qmltest`), and a
/// QML warning a test may hold itself to names its line and column, which
/// a comment moves. Where what the tests load cannot all be named
/// (`QmlTestReads::unbounded`), no key answers for a run: both are owed on
/// every gate, and run.
///
/// Ahead of clippy: it compiles nothing of the app.
fn qmltest_steps(
    reads: &QmlTestReads,
    reach: &Reach,
    changed: &[String],
    everything: bool,
) -> Vec<Step> {
    let host: Vec<String> = reads.runner.union(&reads.process).cloned().collect();
    let mut linux = host.clone();
    linux.extend(reads.container.iter().cloned());
    let moved = |inputs: &[String]| {
        everything
            || reads.unbounded.is_some()
            || changed.iter().any(|file| {
                reach.contains_key(file) && inputs.iter().any(|input| under(file, input))
            })
    };
    let mut steps = Vec::new();
    if moved(&host) {
        steps.push(step(
            "qmltest",
            Side::Host,
            false,
            xtask(&["qmltest"]),
            &host,
        ));
    }
    if moved(&linux) {
        steps.push(step(
            "qmltest-linux",
            Side::Linux,
            false,
            xtask(&["linux", "qmltest"]),
            &linux,
        ));
    }
    for qmltest in &mut steps {
        // Qt's runner over a staging copy: no cargo directory and no
        // release, so neither chain has to wait for it (`gate::sides::side`).
        qmltest.beside = true;
        qmltest.keyed = Keyed::RustCode;
        qmltest.unkeyed.clone_from(&reads.unbounded);
    }
    steps
}

/// The files a stopped run's record passes through: what the app writes,
/// the teardown, and the parent that reads it back (`verify::faults`).
fn record_of_a_wedge() -> [String; 7] {
    let (app, xtask) = (app(), "crates/xtask/src/verify");
    [
        format!("{app}/src/harness/deadline.rs"),
        format!("{app}/src/main.rs"),
        format!("{app}/src/hub/life.rs"),
        format!("{xtask}/wedge.rs"),
        format!("{xtask}/look.rs"),
        format!("{xtask}/faults.rs"),
        format!("{xtask}/outcome.rs"),
    ]
}

/// Three runs stopped on purpose, selected by [`record_of_a_wedge`]
/// alone: each is a held run paid in wall clock, and nothing outside
/// those files can stop a stopped run from being readable.
///
/// Host only: both sides pass the record through the same mount, and
/// every verb step already proves the container carries a file out.
fn wedge_steps(reach: &Reach, everything: bool) -> Vec<Step> {
    let inputs = record_of_a_wedge();
    // Its xtask files read the product tree, so any product change
    // reaches them as data (`graph::Carried`); what stops a stopped run
    // from being readable is a change to their code — and, for the app's,
    // to what the app builds in with it (the harness's QML module).
    let product = app();
    let moved = |file: &String| match reach.get(file) {
        Some(Carried::Whole) => true,
        Some(Carried::AsData) => under(file, &product),
        _ => false,
    };
    if !everything && !inputs.iter().any(moved) {
        return Vec::new();
    }
    let mut wedge = step(
        "wedge-check",
        Side::Host,
        false,
        xtask(&["wedge-check"]),
        &inputs,
    );
    // It drives verify-ui: the same release build as the verbs.
    wedge.builds_app = true;
    wedge.release = true;
    vec![wedge]
}

/// Whether the change reaches the app's QML, or the entry that embeds it:
/// what every run stands up.
fn qml_moved(sorted: &Sorted, reach: &Reach) -> bool {
    // Spelled in pieces: a whole path in a string here would be read as
    // this file reading the app's entry point.
    let entry = format!("{}/src/main.rs", app());
    !sorted.qml.is_empty() || reach.contains_key(&entry)
}

/// Whether the change reaches the harness's Rust half as code. Every verb
/// runs through the harness, which has no static edge into QML. The
/// harness reads the product tree, so every product change reaches it as
/// data (`graph::Carried`) — that is no change to the harness.
fn harness_moved(reach: &Reach) -> bool {
    let dirs = harness();
    reach.iter().any(|(file, carried)| {
        *carried == Carried::Whole && dirs.iter().any(|dir| under(file, dir))
    })
}

/// The shot check's own file, spelled in pieces as [`record_of_a_wedge`]'s are.
fn shot_check() -> String {
    format!("{}/{}", "crates/xtask/src/verify", "shot_check.rs")
}

/// What a picture a read lands in passes through (`verify::shot_check`),
/// as the step's inputs, each boundary named. This runner's own code,
/// from the check to what it reads (the pictures' decoding, the spawn):
/// the check's static closure (`Graph::inputs`). Across the process
/// boundary the check spawns, `verify-ui` in this same binary, the
/// harness's Rust half every verb reads ([`harness`]), which no `use` of
/// the check's reaches: the fault's option and its carriage into the
/// app's environment, the run's judgment and contract. Across the
/// boundary to the app it drives, the product unit every verb reads
/// ([`cargo_inputs`] over the app and core): the knob read off the
/// environment and published to QML — the Rust/QML boundary, which the
/// graph reads as the QML's mention of the singleton's type —, the
/// harness module that reads it and draws the mark, and the product it
/// draws. The key is the code's (`release`), so an edit no build reads
/// keeps the stamp.
fn shot_inputs(g: &Graph) -> Vec<String> {
    let mut inputs: BTreeSet<String> = g.inputs(&[shot_check()]).into_iter().collect();
    inputs.extend(cargo_inputs(&[&app(), &core()]));
    inputs.extend(harness());
    inputs.into_iter().collect()
}

/// Six runs with a read landed in a picture on purpose, owed by what they
/// pass through ([`shot_inputs`]): the check reached as code, the
/// harness's Rust half moved, or the app's QML — the reach every verb is
/// owed by. Host only: the pictures come off the disk the same way on
/// both sides, and every verb's run on the container already proves that
/// side carries a file out.
fn shot_steps(g: &Graph, reach: &Reach, sorted: &Sorted, everything: bool) -> Vec<Step> {
    let own = reach.get(&shot_check()) == Some(&Carried::Whole);
    if !everything && !own && !harness_moved(reach) && !qml_moved(sorted, reach) {
        return Vec::new();
    }
    let mut shot = step(
        "shot-check",
        Side::Host,
        false,
        xtask(&["shot-check"]),
        &shot_inputs(g),
    );
    // It drives verify-ui: the same release build as the verbs.
    shot.builds_app = true;
    shot.release = true;
    vec![shot]
}

/// The records check's own file, spelled in pieces as [`shot_check`]'s is.
fn records_check() -> String {
    format!("{}/{}", "crates/xtask/src/verify", "records_check.rs")
}

/// Three runs with a write's records held back from the command log
/// (`verify::records_check`), owed as the verbs are: by the check reached
/// as code, the harness's Rust half moved, or the app's QML — which the
/// app's write path reaches through its entry (`qml_moved`). Its inputs
/// are the same boundaries as [`shot_inputs`]'. The container's side is
/// the whole tier's, as the verbs' lines there are (`Tiers::on_linux`):
/// the order an event loop drains two feeds in is the platform's, and
/// before a merge the host's answers for it.
fn records_steps(g: &Graph, reach: &Reach, sorted: &Sorted, read: &Reading<'_>) -> Vec<Step> {
    let own = reach.get(&records_check()) == Some(&Carried::Whole);
    if !read.everything && !own && !harness_moved(reach) && !qml_moved(sorted, reach) {
        return Vec::new();
    }
    let mut inputs: BTreeSet<String> = g.inputs(&[records_check()]).into_iter().collect();
    inputs.extend(cargo_inputs(&[&app(), &core()]));
    inputs.extend(harness());
    let inputs: Vec<String> = inputs.into_iter().collect();
    let mut checks = vec![step(
        "records-check",
        Side::Host,
        false,
        xtask(&["records-check"]),
        &inputs,
    )];
    if read.whole_tier {
        let mut linux_inputs = inputs.clone();
        linux_inputs.extend(IMAGE.map(String::from));
        checks.push(step(
            "records-check-linux",
            Side::Linux,
            false,
            xtask(&["linux", "records-check"]),
            &linux_inputs,
        ));
    }
    // Each line it runs is held to its row's promise and its judgment, as the verb's own run is: in the key as text.
    let contract = crate::verify::records_lines()
        .map(|line| {
            format!(
                "{line}: promises={}\n{}",
                read.lines
                    .declared(line)
                    .map(|names| names.into_iter().collect::<Vec<_>>().join(" "))
                    .unwrap_or_default(),
                crate::verify::judgment_of(line)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    for check in &mut checks {
        // It drives verify-ui: the same release build as the verbs.
        check.builds_app = true;
        check.release = true;
        check.contract.clone_from(&contract);
    }
    checks
}

/// clippy for every crate the reach enters as code, on both sides: the
/// host's cannot answer for the names behind `cfg(not(windows))`. clippy
/// reads code alone, so a crate the product reaches only as data a tool
/// reads (`graph::Carried`) owes none; its tests may read that data, so
/// `unit_steps` / `it_steps` select them however the reach was handed.
fn clippy_steps(sorted: &Sorted) -> Vec<Step> {
    let mut steps = Vec::new();
    for package in &sorted.rust_moved {
        let crate_dir = format!("crates/{package}");
        let mut clippy = words(&["cargo", "clippy", "--locked", "-p", package]);
        clippy.extend(words(&[
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ]));
        let inputs = cargo_inputs(&[&crate_dir, &clippy_config()]);
        let id = format!("clippy {package}");
        steps.push(step(&id, Side::Host, false, clippy.clone(), &inputs));
        let mut linux = xtask(&["linux"]);
        linux.extend(clippy[1..].iter().cloned());
        let mut linux_inputs = inputs.clone();
        linux_inputs.extend(IMAGE.map(String::from));
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

/// The unit tests in the reach, per package, by module filter. Their
/// inputs are what the selected files read.
fn unit_steps(g: &Graph, sorted: &Sorted) -> Vec<Step> {
    let mut steps = Vec::new();
    for (package, filters) in &sorted.unit_filters {
        let files = sorted.unit_files.get(package).cloned().unwrap_or_default();
        let mut inputs: Vec<String> = g.inputs(&files).into_iter().collect();
        inputs.extend(cargo_inputs(&[]));
        let all = filters.contains("");
        let target = unit_target(package);
        let mut command = words(&["cargo", "test", "--locked", "-p", package, target[0]]);
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
        if crate::linux::tested_on_linux(package) {
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
        let mut command = words(&["cargo", "test", "--locked", "-p", package, "--test", binary]);
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
        if crate::linux::tested_on_linux(package) {
            steps.push(on_linux(&id, &command, &inputs));
        }
    }
    steps
}

/// The filter that picks out the tests the pre-merge tiers leave out: every
/// test under a `periodic` module, `#[ignore]`d so no plain run takes it.
const PERIODIC: &str = "::periodic::";

/// Stage 3's own steps: the `periodic` tests of every test target that
/// holds some, on both sides where the package is (rules-refs/core.md
/// `periodic`).
fn periodic_steps(g: &Graph, dir: &Path, sorted: &Sorted) -> Vec<Step> {
    let unit = sorted.unit_files.iter().map(|(package, files)| {
        let mut command = words(&["cargo", "test", "--locked", "-p", package]);
        command.extend(
            unit_target(package)
                .iter()
                .filter(|w| !w.is_empty())
                .map(|w| (*w).to_string()),
        );
        (format!("test {package} periodic"), package, command, files)
    });
    let integration = sorted
        .integration
        .iter()
        .map(|((package, binary), (_, files))| {
            let command = words(&["cargo", "test", "--locked", "-p", package, "--test", binary]);
            (format!("test {binary} periodic"), package, command, files)
        });
    let mut steps = Vec::new();
    for (id, package, mut command, files) in unit.chain(integration) {
        let holds = files.iter().any(|file| {
            std::fs::read_to_string(dir.join(file)).is_ok_and(|text| holds_periodic(&text))
        });
        if !holds {
            continue;
        }
        command.extend(words(&["--", "--ignored", PERIODIC]));
        let mut inputs: Vec<String> = g.inputs(files).into_iter().collect();
        inputs.extend(cargo_inputs(&[]));
        steps.push(step(&id, Side::Host, false, command.clone(), &inputs));
        if crate::linux::tested_on_linux(package) {
            steps.push(on_linux(&id, &command, &inputs));
        }
    }
    steps
}

/// Whether a test file declares a `periodic` module — inline, or in a
/// file of its own beside it.
fn holds_periodic(text: &str) -> bool {
    text.lines().any(|line| {
        line.trim_start()
            .strip_prefix("mod periodic")
            .is_some_and(|rest| rest.is_empty() || rest.starts_with([' ', '{', ';']))
    })
}

/// How many of `lines` show a component the reach holds — the selection
/// a gate choosing by what each line shows would make, printed beside
/// the selection by reach and never used to skip a run
/// (反映前テストの機械化.md §動詞の絞り込みは比較表示に留める). Counted
/// over the components the change itself touched, not their readers:
/// composition reads up to `Main`, which every line shows. Every line
/// counts where the change is the whole tree or has no QML in it to
/// compare against.
fn lines_showing_the_change(
    read: &Reading<'_>,
    changed: &BTreeSet<String>,
    lines: &[String],
) -> usize {
    if read.everything || changed.is_empty() {
        return lines.len();
    }
    lines
        .iter()
        .filter(|line| {
            read.shown
                .of(line)
                .is_some_and(|shown| changed.iter().any(|stem| shown.contains(stem)))
        })
        .count()
}

/// The QML components among the changed files, by name.
fn changed_qml(changed: &[String]) -> BTreeSet<String> {
    let (_, qml_tests) = qml_dirs();
    changed
        .iter()
        .filter(|file| file.ends_with(".qml") && !under(file, &qml_tests))
        .map(|file| stem_of(file))
        .collect()
}

/// The app as a built thing: `shipped`, the verify-ui verbs, `bare`. The
/// first verb of each side builds the release; the rest reuse it.
fn binary_steps(
    read: &Reading<'_>,
    sorted: &Sorted,
    reach: &Reach,
    changed: &[String],
    ask: &Ask<'_>,
) -> (Vec<Step>, Counted) {
    let mut steps = Vec::new();
    let qml_moved = qml_moved(sorted, reach);
    let binary_moved = qml_moved
        || sorted.rust_in.contains_key("platitude-app")
        || sorted.rust_in.contains_key("platitude-core");
    let binary_inputs = cargo_inputs(&[&app(), &core()]);
    if qml_moved {
        steps.push(step(
            "shipped",
            Side::Host,
            false,
            xtask(&["shipped"]),
            &binary_inputs,
        ));
    }
    let harness_moved = harness_moved(reach);
    // Every registered line, whenever the change reaches the app's QML or
    // the harness that drives it: every run stands the whole window up,
    // so a component reached is one every run can meet
    // (反映前テストの機械化.md §選択の正本). Narrower selections are
    // counted for comparison (`lines_showing_the_change`), never run.
    //
    // A row added to the registry is a line no stamp answers for, and a
    // line whose tier moved one this gate may owe where it did not: owed
    // whatever else moved (`plan::rows_the_tables_owe`). A row that stood
    // owes nothing again — its step's inputs are the tree's, and its
    // contract is in its key.
    let lines: Vec<String> = if read.everything || harness_moved || qml_moved {
        read.lines.lines().cloned().collect()
    } else {
        read.added.to_vec()
    };
    let by_shown = lines_showing_the_change(read, &changed_qml(changed), &lines);
    let selected = lines.len();
    let mut lines = owed_lines(read, &sorted.qml, lines, read.whole_tier);
    let left = selected - lines.len();
    for extra in ask.extra_verbs {
        if !lines.contains(extra) {
            lines.push(extra.clone());
        }
    }
    // A run reads the product, the harness that drives it, the fixtures it
    // stands on and the judgment it is held to; its row's promise and its
    // judgment are its contract, in the key as text.
    let mut verb_inputs = binary_inputs.clone();
    verb_inputs.extend(harness());
    for line in &lines {
        let contract = format!(
            "promises={}\n{}",
            read.lines
                .declared(line)
                .map(|names| names.into_iter().collect::<Vec<_>>().join(" "))
                .unwrap_or_default(),
            crate::verify::judgment_of(line)
        );
        let mut host = xtask(&["verify-ui"]);
        host.extend(crate::verify::suite_words(line));
        let mut host_step = step(
            &format!("verify {line}"),
            Side::Host,
            false,
            host,
            &verb_inputs,
        );
        host_step.builds_app = true;
        host_step.release = true;
        host_step.contract = contract.clone();
        steps.push(host_step);
        // Asked for by name, the line runs where the asker can see it.
        if !(read.tiers.on_linux(line, read.whole_tier) || ask.extra_verbs.contains(line)) {
            continue;
        }
        let mut linux = xtask(&["linux", "verify-ui"]);
        linux.extend(crate::verify::suite_words(line));
        let mut linux_inputs = verb_inputs.clone();
        linux_inputs.extend(IMAGE.map(String::from));
        let mut linux_step = step(
            &format!("verify-linux {line}"),
            Side::Linux,
            false,
            linux,
            &linux_inputs,
        );
        linux_step.builds_app = true;
        linux_step.release = true;
        linux_step.contract = contract;
        steps.push(linux_step);
    }
    // The image it runs in moving is the container's whole reach
    // (`plan::scopes`), which moves the binary too.
    if binary_moved {
        let mut bare_inputs = binary_inputs;
        bare_inputs.push("crates/xtask/src/linux".to_string());
        bare_inputs.extend(IMAGE.map(String::from));
        let mut bare = step(
            "bare",
            Side::Linux,
            false,
            xtask(&["linux", "bare"]),
            &bare_inputs,
        );
        // Built in the container's release directory, after the verbs
        // there have built it: the same group as they are.
        bare.release = true;
        steps.push(bare);
    }
    (steps, Counted { by_shown, left })
}

/// The selected lines this gate owes (`tiers`): the whole gate owes every
/// line but a twin; a gate before a merge leaves the whole lines out,
/// except one that is the only witness of a component the change reaches
/// — the only line whose run shows it (`lines::Shown`;
/// 反映前テストの機械化.md「唯一の証人の例外」).
fn owed_lines(
    read: &Reading<'_>,
    reached: &BTreeSet<String>,
    lines: Vec<String>,
    whole_tier: bool,
) -> Vec<String> {
    let (owed, left): (Vec<String>, Vec<String>) = lines
        .into_iter()
        .filter(|line| read.tiers.tier(line) != Some(Tier::Twin))
        .partition(|line| read.tiers.owed(line, whole_tier));
    // By what the rows promise, which a green run is evidence of — not by
    // what the sources' closure says a run would bring up.
    let promised: BTreeSet<String> = owed
        .iter()
        .filter_map(|line| read.lines.declared(line))
        .flatten()
        .collect();
    let only_witnesses: Vec<String> = left
        .into_iter()
        .filter(|line| {
            read.lines.declared(line).is_some_and(|names| {
                names
                    .iter()
                    .any(|name| reached.contains(name) && !promised.contains(name))
            })
        })
        .collect();
    let mut owed = owed;
    owed.extend(only_witnesses);
    owed
}

#[cfg(test)]
mod tests {
    use super::{Carried, Graph, Reach, clippy_steps, holds_periodic, sort};
    use crate::gate::graph::Module;

    /// A crate the product reaches only as data a tool reads owes no
    /// clippy; the crate the change is code of does. The same reach sorts
    /// both into `rust_in`, which the tests select from.
    #[test]
    fn clippy_is_owed_by_a_crate_the_change_reaches_as_code_alone() {
        let module = |package: &str, has_tests: bool| Module {
            krate: package.replace('-', "_"),
            path: vec!["probe".to_string()],
            package: package.to_string(),
            test_binary: None,
            has_tests,
        };
        let mut g = Graph::default();
        let core = format!("crates/{}/src/probe.rs", "platitude-core");
        let tool = format!("crates/{}/src/probe_reader.rs", "xtask");
        g.modules
            .insert(core.clone(), module("platitude-core", false));
        g.modules.insert(tool.clone(), module("xtask", true));
        let reach: Reach = [
            (core, Carried::Whole),
            (tool.clone(), Carried::AsProductFile),
        ]
        .into_iter()
        .collect();
        let sorted = sort(&g, &reach, false);
        assert!(sorted.rust_in.contains_key("xtask"));
        assert_eq!(sorted.unit_files["xtask"], vec![tool]);
        let ids: Vec<String> = clippy_steps(&sorted).into_iter().map(|s| s.id).collect();
        assert_eq!(
            ids,
            ["clippy platitude-core", "clippy-linux platitude-core"]
        );

        // Handed whole by any path, the crate is owed again.
        let both: Reach = reach
            .into_keys()
            .map(|file| (file, Carried::Whole))
            .collect();
        let ids: Vec<String> = clippy_steps(&sort(&g, &both, false))
            .into_iter()
            .map(|s| s.id)
            .collect();
        assert!(ids.contains(&"clippy xtask".to_string()), "{ids:?}");
    }

    /// Inline and declared both count; a module that only starts with the
    /// word does not.
    #[test]
    fn a_periodic_module_is_found_by_its_declaration() {
        assert!(holds_periodic(
            "fn a() {}\nmod periodic {\n    use super::*;\n}\n"
        ));
        assert!(holds_periodic("    mod periodic;\n"));
        assert!(!holds_periodic("mod periodic_helpers;\n"));
        assert!(!holds_periodic("// see mod periodic in merge_tools.rs\n"));
    }

    /// The spelling is also a stamp's key, so a change here re-runs every
    /// step once (`gate::stamp`).
    #[test]
    fn the_runners_steps_are_spelled_locked() {
        assert_eq!(
            super::xtask(&["structure"]),
            ["cargo", "run", "--locked", "-p", "xtask", "--", "structure"]
        );
    }

    /// Qt's runner reads neither cargo directory nor the release, so its
    /// two steps run beside both chains (`gate::sides::side`); a step is
    /// in neither group unless it says so, and the always-steps run ahead
    /// of everything.
    #[test]
    fn the_qt_runner_runs_beside_both_chains() {
        let reads = super::QmlTestReads::default();
        let qml = super::qmltest_steps(&reads, &Reach::default(), &[], true);
        let ids: Vec<&str> = qml.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["qmltest", "qmltest-linux"]);
        assert!(qml.iter().all(|s| s.beside && !s.release), "{ids:?}");
        assert!(
            super::always_steps()
                .iter()
                .all(|s| !s.beside && !s.release)
        );
    }

    /// The boundaries as this tree draws them: the runner as built holds
    /// Qt's selection and its pin and none of the product's Rust, which
    /// only what its tests read leads to; the tests load the harness's
    /// module by relative import; the container adds the image and its
    /// launcher, and nothing that only schedules the run.
    #[test]
    fn a_qml_test_run_reads_its_runner_as_built_and_what_crosses_into_it() {
        let root = crate::tree::workspace_root();
        let g = crate::gate::graph::build(&root).expect("the graph of this tree");
        let reads = super::qmltest_reads(&g, &root);
        let (ui, tests) = super::qml_dirs();
        let qt = format!("{}/{}", "crates/xtask/src", "qt.rs");
        let main = super::xtask_main();
        let runner = [super::QMLTEST, main.as_str(), qt.as_str(), crate::qt::PIN];
        for named in runner.iter().chain(&super::CARGO) {
            assert!(reads.runner.contains(*named), "{named}: {:?}", reads.runner);
        }
        for named in [&ui, &tests] {
            assert!(
                reads.process.contains(named),
                "{named}: {:?}",
                reads.process
            );
        }
        let elsewhere: Vec<&String> = reads
            .runner
            .iter()
            .filter(|f| f.ends_with(".rs") && !f.starts_with("crates/xtask/src/"))
            .collect();
        assert!(elsewhere.is_empty(), "{elsewhere:?}");
        assert!(
            g.inputs(&[super::QMLTEST.to_string()]).len() > reads.runner.len(),
            "the closure its tests read is wider"
        );
        let harness = format!("crates/{}/src/auto", "platitude-app");
        assert!(reads.process.contains(&harness), "{:?}", reads.process);
        let module = super::launcher();
        assert!(reads.container.contains(&format!("{module}.rs")));
        assert!(reads.container.contains(&format!("{module}/container.rs")));
        assert!(!reads.container.contains(&format!("{module}/tests.rs")));
        let outside: Vec<&String> = reads
            .container
            .iter()
            .filter(|f| !super::under(f, &module) && !super::IMAGE.contains(&f.as_str()))
            .filter(|f| **f != format!("{module}.rs"))
            .collect();
        assert!(outside.is_empty(), "{outside:?}");
    }
}
