//! The steps a reach selects: how each is spelled, the reach sorted
//! by package and binary, and the selection itself.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::{
    Ask, CARGO, Counted, DENY, DOCKERFILE, DOCS, QMLTEST, Reading, Side, Step, app, core, harness,
    qml_dirs, under,
};
use crate::gate::census;
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

/// The same step for the container.
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
    /// The QML components in the reach, by name.
    qml: BTreeSet<String>,
}

/// `whole` runs every package's tests unfiltered.
fn sort(
    g: &Graph,
    reach: &Reach,
    whole: bool,
    worn: &BTreeMap<String, BTreeSet<String>>,
) -> Sorted {
    let (_, qml_tests) = qml_dirs();
    let mut sorted = Sorted::default();
    for (file, carried) in reach {
        if file.ends_with(".qml") {
            // A QtTest file is no component: no census names it, so it
            // belongs to `qmltest_steps` alone.
            if !under(file, &qml_tests) {
                // Under every name a run could have met it (`census::worn_by`).
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
        if *carried == Carried::Whole {
            sorted.rust_moved.insert(module.package.clone());
        }
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
                // binary.
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
    reach: &Reach,
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
        // Fails on a census line no verb answers: the app would ignore the
        // name and the run wait out its ceiling silently. Verbs with no
        // line are only counted (`verify::coverage`).
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

/// cargo-deny over the resolved graph. `deny.toml` is named on its own
/// because nothing in the source graph reads it; every other way the
/// closure moves is a manifest, which already sets `whole`
/// (`moves_everything`). Ahead of every build, so a forbidden crate is
/// said in seconds.
///
/// Host only: the policy names no `targets` and takes `all-features`, so
/// every OS reads the same crates.
fn deny_steps(g: &Graph, changed: &[String], whole: bool) -> Vec<Step> {
    if !whole && !changed.iter().any(|f| f == DENY) {
        return Vec::new();
    }
    // The manifests carry what the lock does not: a license field and the
    // features a dependency is taken with.
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
/// and these ask when a Canvas has painted.
///
/// Ahead of clippy: it compiles nothing of the app. Its inputs are the
/// whole module, because the staging copies all of it and a test can
/// resolve through any of it (`crate::qmltest`).
///
/// `whole` selects it outright: neither the qmldir nor a QtTest file is a
/// node of the source graph.
fn qmltest_steps(reach: &Reach, whole: bool) -> Vec<Step> {
    let (ui, tests) = qml_dirs();
    // The qmldir counts (it declares the singletons), and so does the
    // runner, whose staging a run resolves through — when its code moved,
    // not when the QML it stages did (`graph::Carried`); of the tests
    // directory only the files the runner picks up by name.
    let reads = |(file, carried): (&String, &Carried)| {
        (file == QMLTEST && *carried == Carried::Whole)
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
fn wedge_steps(reach: &Reach, whole: bool) -> Vec<Step> {
    let inputs = record_of_a_wedge();
    // Its xtask files read the product tree, so any product change
    // reaches them as data (`graph::Carried`); what stops a stopped run
    // from being readable is a change to their code.
    let moved = |file: &String| reach.get(file) == Some(&Carried::Whole);
    if !whole && !inputs.iter().any(moved) {
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

/// Components for the shadow candidate only: absence from the census is
/// no proof a run did not use a component
/// (反映前テストの機械化.md §動詞の絞り込みは比較表示に留める). `None`
/// when the changed inputs are not all component names.
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

/// What the verify-ui verbs read besides the census.
fn verb_inputs() -> Vec<String> {
    let mut inputs = cargo_inputs(&[&app(), &core()]);
    inputs.extend(harness());
    inputs
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
    let census = read.census;
    let mut steps = Vec::new();
    // Spelled in pieces: a whole path in a string here would be read as
    // this file reading the app's entry point.
    let entry = format!("{}/src/main.rs", app());
    let qml_moved = !sorted.qml.is_empty() || reach.contains_key(&entry);
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
    // Every verb runs through the harness, which has no static edge into
    // QML. The harness reads the product tree, so every product change
    // reaches it as data (`graph::Carried`) — that is no change to the
    // harness, and what it changed the census names.
    let harness_dirs = harness();
    let harness_moved = reach.iter().any(|(file, carried)| {
        *carried == Carried::Whole && harness_dirs.iter().any(|dir| under(file, dir))
    });
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
    // The host's runs rewrite their census lines, the container's do not
    // (`verify::options::census_line`); a gate whose runs moved the census
    // stops before it stamps (`gate::execute::execute`).
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
/// except one that is the only witness of a component the change reaches
/// (反映前テストの機械化.md「唯一の証人の例外」).
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
    use std::collections::BTreeMap;

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
        let sorted = sort(&g, &reach, false, &BTreeMap::new());
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
        let ids: Vec<String> = clippy_steps(&sort(&g, &both, false, &BTreeMap::new()))
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
}
