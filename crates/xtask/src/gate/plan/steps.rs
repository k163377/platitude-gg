//! The steps a reach selects: how each is spelled, the reach sorted
//! by package and binary, and the selection itself.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::{
    Ask, CARGO, Counted, DENY, DOCKERFILE, DOCS, QMLTEST, Reading, Side, Step, app, core, harness,
    qml_dirs, under,
};
use crate::gate::census;
use crate::gate::graph::{Graph, stem_of};
use crate::gate::tiers::Tier;

fn words(line: &[&str]) -> Vec<String> {
    line.iter().map(|w| (*w).to_string()).collect()
}

/// `cargo xtask <line>`, spelled unquieted the way `check` does so that
/// cargo's build-lock line reaches the log — and `--locked`, like every
/// cargo the gate starts: a cargo that would rewrite `Cargo.lock` says
/// so and stops
/// (internal-docs/反映前テストの機械化.md §gate).
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
///
/// Read by the sweep as well: a test binary the container never builds
/// is one no line in there may ask for, because asking is building
/// (`crate::sweep::replays`).
pub(crate) fn tested_on_linux(package: &str) -> bool {
    matches!(package, "platitude-core" | "xtask")
}

/// `whole` runs every package's tests unfiltered — the reach is the
/// whole tree, and a filter naming every module says the same thing
/// less plainly.
fn sort(
    g: &Graph,
    reach: &BTreeSet<String>,
    whole: bool,
    worn: &BTreeMap<String, BTreeSet<String>>,
) -> Sorted {
    let (_, qml_tests) = qml_dirs();
    let mut sorted = Sorted::default();
    for file in reach {
        if file.ends_with(".qml") {
            // A QtTest file is not a component of the app: nothing ships
            // it and no verb's census can name it, so it belongs to
            // `qmltest_steps` alone.
            if !under(file, &qml_tests) {
                // Under every name a run could have met it: a component
                // that is only ever somebody's root type is in the tree
                // under the wearer's name (`census::worn_by`).
                census::through_wearers(&stem_of(file), worn, &mut sorted.qml);
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

pub(super) fn select(
    g: &Graph,
    read: &Reading<'_>,
    reach: &BTreeSet<String>,
    changed: &[String],
    ask: &Ask<'_>,
) -> (Vec<Step>, Counted) {
    let sorted = sort(g, reach, read.whole, read.worn);
    let mut steps = always_steps();
    steps.extend(deny_steps(g, changed, read.whole));
    steps.extend(qmltest_steps(reach, read.whole));
    steps.extend(wedge_steps(reach, read.whole));
    steps.extend(clippy_steps(&sorted));
    steps.extend(unit_steps(g, &sorted));
    steps.extend(it_steps(g, &sorted));
    if ask.all {
        steps.extend(periodic_steps(g, read.dir, &sorted));
    }
    let (binary, counted) = binary_steps(read, &sorted, reach, changed, ask);
    steps.extend(binary);
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
        step("docs", Side::Host, true, xtask(&["docs"]), &DOCS),
        // What it fails on is a census line no verb answers — a renamed
        // or deleted verb whose line the gate would go on running, where
        // the app ignores the name and the run waits out its ceiling
        // saying nothing. The verbs with no line at all are counted and
        // printed: that is a backlog to record
        // (internal-docs/P3-確認事項.md).
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

/// cargo-deny over the resolved graph — the networking and libgit2 bans,
/// the registries, the license allow list. Nothing in the source graph
/// reads `deny.toml`, so it is named here on its own; every other way the
/// closure can move is a manifest, and a manifest already sets `whole`
/// (`moves_everything`). It goes ahead of every build on its side: a
/// crate the policy forbids should be said in seconds.
///
/// Host only. The policy names no `targets` and takes the graph with
/// `all-features`, so the set of crates it reads is the same on every OS
/// and the container would be answering a question already answered.
fn deny_steps(g: &Graph, changed: &[String], whole: bool) -> Vec<Step> {
    if !whole && !changed.iter().any(|f| f == DENY) {
        return Vec::new();
    }
    // The lock is the closure, and the manifests carry what the lock does
    // not: a license field and the features a dependency is taken with.
    let mut inputs: BTreeSet<String> = ["Cargo.toml", "Cargo.lock", DENY]
        .into_iter()
        .map(str::to_string)
        .collect();
    inputs.extend(
        g.modules
            .values()
            .map(|module| format!("crates/{}/Cargo.toml", module.package)),
    );
    let inputs: Vec<String> = inputs.into_iter().collect();
    vec![step("deny", Side::Host, false, xtask(&["deny"]), &inputs)]
}

/// The QtTest files, when the change reaches them or the QML module they
/// read. Both sides: the two Qt builds paint through different stacks,
/// and what these ask about is when a Canvas has painted.
///
/// Ahead of clippy because it compiles nothing of the app — the runner
/// stages the product's QML and hands it to `qmltestrunner`. Its inputs
/// are the whole module: the
/// staging copies all of it, so any of it can be what a test resolves
/// through (`crate::qmltest`).
///
/// `whole` selects it outright, the way it does cargo-deny: neither the
/// qmldir nor a QtTest file is a node of the source graph, so a reach of
/// "everything" is not one that can be asked about them.
fn qmltest_steps(reach: &BTreeSet<String>, whole: bool) -> Vec<Step> {
    let (ui, tests) = qml_dirs();
    // The module is reached whole — the qmldir as much as the components,
    // since it is what declares the singletons — but of the tests
    // directory only what the runner picks up by name, so the README
    // beside them is a document like any other. The runner itself counts:
    // the staging is what a run resolves through, and nothing else here
    // would exercise a change to it.
    let reads = |file: &String| {
        file == QMLTEST
            || under(file, &ui)
            || (under(file, &tests) && file.ends_with(".qml") && stem_of(file).starts_with("tst_"))
    };
    if !whole && !reach.iter().any(reads) {
        return Vec::new();
    }
    let inputs = [ui, tests, QMLTEST.to_string()];
    let mut linux_inputs = inputs.to_vec();
    linux_inputs.push(DOCKERFILE.to_string());
    vec![
        step("qmltest", Side::Host, false, xtask(&["qmltest"]), &inputs),
        step(
            "qmltest-linux",
            Side::Linux,
            false,
            xtask(&["linux", "qmltest"]),
            &linux_inputs,
        ),
    ]
}

/// What the record a stopped run leaves is made of, on both sides of the
/// pipe: the stations and the trail the app writes, the teardown that
/// passes them, and the parent that reads them back and looks at what
/// still stands (`verify::faults`).
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

/// Three runs stopped on purpose, when a change reaches what would read
/// them back.
///
/// **Selected by its own files.** Every one of
/// these is a held run paid for in wall clock, and what they check is one
/// mechanism: nothing outside [`record_of_a_wedge`] can quietly stop a
/// stopped run from being readable.
///
/// **Host only.** The two sides run the same record through the same
/// mount, and every verb step already proves the container carries a
/// file out of it; a second copy of these would buy the same answer at
/// the container's pace.
fn wedge_steps(reach: &BTreeSet<String>, whole: bool) -> Vec<Step> {
    let inputs = record_of_a_wedge();
    if !whole && !reach.iter().any(|file| inputs.contains(file)) {
        return Vec::new();
    }
    let mut wedge = step(
        "wedge-check",
        Side::Host,
        false,
        xtask(&["wedge-check"]),
        &inputs,
    );
    // It drives verify-ui, which is the release build with the harness in
    // it: the same one the verbs are run against, and built once for all
    // of them.
    wedge.builds_app = true;
    wedge.release = true;
    vec![wedge]
}

/// clippy for every crate the reach enters, on both sides: the host's
/// cannot answer for the names behind `cfg(not(windows))`.
fn clippy_steps(sorted: &Sorted) -> Vec<Step> {
    let mut steps = Vec::new();
    for package in sorted.rust_in.keys() {
        let crate_dir = format!("crates/{package}");
        let mut clippy = words(&["cargo", "clippy", "--locked", "-p", package]);
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
        if tested_on_linux(package) {
            steps.push(on_linux(&id, &command, &inputs));
        }
    }
    steps
}

/// The filter that picks out the tests the pre-merge tiers leave out: every
/// test under a `periodic` module, `#[ignore]`d so no plain run takes it.
const PERIODIC: &str = "::periodic::";

/// Stage 3's own steps: the `periodic` tests of every test target that
/// holds some, on both sides where the package is. Seldom-changed
/// promises whose check is slow on some OS — not worth the pre-merge run,
/// still owed a full one.
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
        if tested_on_linux(package) {
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

/// Components for a narrower candidate only.
/// The census records only the final state: `settings-escape` exercises
/// SettingsDialog but closes it before that snapshot. Absence there is
/// no proof a run did not use a component. `None` keeps the actual set
/// when inputs cannot be represented by component names at all.
fn verbs_in_snapshot(read: &Reading<'_>, changed: &[String]) -> Option<BTreeSet<String>> {
    if read.whole {
        return None;
    }
    let (ui, _) = qml_dirs();
    let mut shown = BTreeSet::new();
    for file in changed {
        let read_by_a_verb = verb_inputs().iter().any(|input| under(file, input));
        if !read_by_a_verb {
            continue;
        }
        if !(file.ends_with(".qml") && under(file, &ui) && census::instantiable(read.dir, file)) {
            return None;
        }
        census::through_wearers(&stem_of(file), read.worn, &mut shown);
    }
    Some(shown)
}

/// What the verify-ui verbs read besides the census: the app and the core
/// they build, the harness they run through, and the manifests every
/// build reads.
fn verb_inputs() -> Vec<String> {
    let mut inputs = cargo_inputs(&[&app(), &core()]);
    inputs.extend(harness());
    inputs
}

/// The app as a built thing: shipped when its QML or entry point moved,
/// the verify-ui verbs reached by a change (and the ones asked
/// for), bare when the binary moved at all. The first verb of each side
/// builds the release; the rest reuse it (`check` does the same).
fn binary_steps(
    read: &Reading<'_>,
    sorted: &Sorted,
    reach: &BTreeSet<String>,
    changed: &[String],
    ask: &Ask<'_>,
) -> (Vec<Step>, Counted) {
    let census = read.census;
    let mut steps = Vec::new();
    // Spelled in pieces: a whole path in a string here would be read as
    // this file reading the app's entry point.
    let entry = format!("{}/src/main.rs", app());
    let qml_moved = !sorted.qml.is_empty() || reach.contains(&entry);
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
    // Harness behavior can change without any static edge into QML.
    // Every recorded verb runs through it, even when the QML reach is empty.
    let harness_moved = reach
        .iter()
        .any(|file| harness().iter().any(|dir| under(file, dir)));
    let lines = if read.whole || harness_moved {
        census.lines.keys().cloned().collect()
    } else {
        census.verbs_touching(&sorted.qml)
    };
    let shadow = verbs_in_snapshot(read, changed)
        .map_or(lines.len(), |shown| census.verbs_touching(&shown).len());
    let selected = lines.len();
    let mut lines = owed_lines(read, &sorted.qml, lines, ask.all);
    let left = selected - lines.len();
    for extra in ask.extra_verbs {
        if !lines.contains(extra) {
            lines.push(extra.clone());
        }
    }
    let mut verb_inputs = binary_inputs.clone();
    verb_inputs.extend(harness());
    // The runs rewrite their own census lines — the container's does not
    // (`verify::options::census_line` refuses there), so the file stays
    // one machine's answer, the host's. That is why the gate
    // records the file itself: it re-runs exactly the lines a QML
    // change made stale, and a census only hand-typed runs refresh
    // goes dirty in the middle of unrelated work. A gate whose runs
    // moved it stops before it stamps, because the stamp names a
    // commit and the tree that passed is no longer the one it holds
    // (`gate::execute::execute`).
    for line in &lines {
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
        steps.push(host_step);
        // Asked for by name, the line runs where the asker can see it.
        if !(read.tiers.on_linux(line, ask.all) || ask.extra_verbs.contains(line)) {
            continue;
        }
        let mut linux = xtask(&["linux", "verify-ui"]);
        linux.extend(crate::verify::suite_words(line));
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
        linux_step.release = true;
        steps.push(linux_step);
    }
    if binary_moved || changed.iter().any(|f| f == DOCKERFILE) {
        let mut bare_inputs = binary_inputs;
        bare_inputs.extend(["crates/xtask/src/linux", DOCKERFILE].map(String::from));
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
    (steps, Counted { shadow, left })
}

/// The selected census lines this gate owes (`tiers`): the full gate owes
/// every line but a twin; a gate before a merge leaves the full lines out,
/// **except a full line that is the only one showing a component the
/// change reaches** — leaving it would leave that component without a run
/// before the merge, which is what the census is there to rule out (the
/// perf driver is shown by the perf lines alone).
fn owed_lines(
    read: &Reading<'_>,
    reached: &BTreeSet<String>,
    lines: Vec<String>,
    full: bool,
) -> Vec<String> {
    let (owed, left): (Vec<String>, Vec<String>) = lines
        .into_iter()
        .filter(|line| read.tiers.tier(line) != Some(Tier::Twin))
        .partition(|line| read.tiers.owed(line, full));
    let shown: BTreeSet<&String> = owed
        .iter()
        .filter_map(|line| read.census.lines.get(line))
        .flatten()
        .collect();
    let only_witnesses: Vec<String> = left
        .into_iter()
        .filter(|line| {
            read.census.lines.get(line).is_some_and(|names| {
                names
                    .iter()
                    .any(|name| reached.contains(name) && !shown.contains(name))
            })
        })
        .collect();
    let mut owed = owed;
    owed.extend(only_witnesses);
    owed
}

#[cfg(test)]
mod tests {
    use super::holds_periodic;

    /// A `periodic` module is found inline and declared, and a module
    /// that only starts with the word is not one.
    #[test]
    fn a_periodic_module_is_found_by_its_declaration() {
        assert!(holds_periodic(
            "fn a() {}\nmod periodic {\n    use super::*;\n}\n"
        ));
        assert!(holds_periodic("    mod periodic;\n"));
        assert!(!holds_periodic("mod periodic_helpers;\n"));
        assert!(!holds_periodic("// see mod periodic in merge_tools.rs\n"));
    }

    /// The runner's own steps are spelled `--locked`, like every cargo
    /// the gate starts: a cargo that would rewrite the lock says so and
    /// stops. The
    /// spelling is also a stamp's key, so a change here re-runs every
    /// step once (`gate::stamp`).
    #[test]
    fn the_runners_steps_are_spelled_locked() {
        assert_eq!(
            super::xtask(&["structure"]),
            ["cargo", "run", "--locked", "-p", "xtask", "--", "structure"]
        );
    }
}
