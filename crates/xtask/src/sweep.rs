//! `cargo xtask sweep` — one generation of build products in `target/`,
//! and nothing else.
//!
//! cargo never takes away what it has replaced. A lock file, a toolchain
//! or a profile setting that moves gives every unit a new hash, and the
//! old artifact, its build-script directory and its incremental session
//! all stay where they are, named by nothing, for as long as the tree
//! lives (measured: ci/baseline/code-costs-windows-x64.md §sweep).
//!
//! **What is alive is asked of cargo, not of the clock.** A fresh build
//! does not touch `.fingerprint/*/invoked.timestamp`, so an age is no
//! evidence at all: a dependency nothing has moved in a month is as live
//! as one compiled this minute. So the live set is read off
//! `--message-format=json` from a written-down list of cargo lines
//! ([`CANONICAL`]) — every line a session or the gate is known to run
//! here — which name every artifact and build-script directory they
//! stand on, fresh units and compiled ones alike. A line off that list
//! is not kept for: what it built is built again next time it is typed.
//!
//! **Run when the generation moves**, which is what actually leaves one
//! behind: `rustc -vV`, `Cargo.lock`, the manifest's profile settings and
//! the toolchain pin are hashed into one key ([`generation`]), each tree
//! keeps the key its last sweep ran under, and the tail of a gate or a
//! landing sweeps only when the two differ. A full gate sweeps whatever
//! the key says, because stage 3 is where the tree is asked for
//! everything anyway (CLAUDE.md 確認は 3 段).
//!
//! **What was written since the last sweep stays**, live set or not: a
//! configuration somebody used this week is one they are using, and
//! taking it away every time would be a rebuild every time.
//!
//! `target/hooks` is left alone — the pre-shell hook's own slot, in use
//! at unpredictable moments (internal-docs/反映前テストの機械化.md
//! §hook) — and so is everything under `target/` that is not cargo's:
//! `perf`, `gate-runs`, `land-runs`, `window`, `qmltest`, `shots`,
//! `footprint`, `gate-logs` and `doc` are kept on purpose or swept by
//! whoever writes them.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::TryLockError;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::command::{self, Permission, Where};
use crate::subprocess::Answer;

#[cfg(test)]
mod tests;

pub(crate) static SWEEP: command::Command = command::Command {
    id: "sweep.now",
    call: "sweep",
    purpose: "take away every build product but the generation the canonical cargo lines stand on",
    run_in: Where::Seat,
    needs: &[
        "the lines it reads the live set from compile whatever this tree has not built — \
              once per generation, and the same work a full gate asks for",
    ],
    permission: Permission::Plain,
};

pub(crate) static DRY: command::Command = command::Command {
    id: "sweep.dry-run",
    call: "sweep --dry-run",
    purpose: "say what a sweep would take away, and take nothing",
    run_in: Where::Seat,
    needs: &[],
    permission: Permission::Plain,
};

/// What the tail of a gate or a landing runs **in the container**, where
/// the volume it is looking at is a build directory with a rustc, a
/// generation key and a stamp of its own ([`asks`]).
pub(crate) static IF_MOVED: command::Command = command::Command {
    id: "sweep.if-the-generation-moved",
    call: "sweep --if-the-generation-moved",
    purpose: "sweep this build directory, and only if its own generation key has moved since \
              its own last sweep",
    run_in: Where::Seat,
    needs: &[],
    permission: Permission::Plain,
};

/// [`IF_MOVED`]'s option, spelled once. The command's own call is held
/// to carrying it (`tests`).
const ONLY_IF_MOVED: &str = "--if-the-generation-moved";

pub(crate) static COMMANDS: &[&command::Command] = &[&SWEEP, &DRY, &IF_MOVED];

/// The profile directories under `target/` this walks, in the order a
/// reader meets them.
///
/// `hooks` is not here on purpose: it is the pre-shell hook's slot, it
/// is small, and `cargo test -p xtask --profile hooks` lands there too,
/// so a sweep of it would be taking something out from under a hook that
/// may run at any moment.
const PROFILES: [&str; 3] = ["debug", "release", "shipped"];

/// The feature set every build of the app this runner starts asks for
/// (`crate::tree::HARNESS_FEATURE`), and the profile the shipped build
/// lands in (Cargo.toml). Spelled here as the replay of those lines
/// rather than read off them: what this reproduces is the command.
const HARNESS: &str = "automation";
const SHIPPED: &str = "shipped";

/// Extensions rustc puts a `lib` in front of. A unit's files share one
/// name but not one spelling — `libplatitude_core-<hash>.rlib` beside
/// `platitude_core-<hash>.d` — so the prefix comes off exactly where
/// rustc put it on, and nowhere else: `libc-<hash>.d` belongs to the
/// crate `libc`, and stripping a prefix there would answer for a crate
/// called `c`.
const LIB_PREFIXED: [&str; 5] = ["rlib", "rmeta", "so", "dylib", "a"];

/// How far an incremental session's directory may stand from the compile
/// that wrote it ([`kept_incremental`]). Measured on this tree: rustc
/// finalises the session within five seconds of the artifact, either
/// way. Wider than the measurement on purpose — keeping a dead session
/// costs disk, and taking a live one costs a compile that can reuse
/// nothing.
const SAME_COMPILE: Duration = Duration::from_secs(60);

/// How long one replayed cargo line may take: the same ceiling every
/// other step of this runner gets, because a line this tree has not
/// built is a compile like any other.
const REPLAY_CEILING: Duration = crate::check::STEP_CEILING;

/// Where a tree keeps the key its last sweep ran under, and when that
/// was. Inside the build directory it answers for, so the container's
/// volume carries its own (its rustc is not this machine's).
const STAMP: [&str; 2] = ["sweep", "generation.txt"];

/// What a line's own answer is kept under, beside the stamp: the
/// evidence for a reading that failed, and the one file a reader is sent
/// to when it does.
const REPLAY: &str = "replay-";

/// **The canonical set: every cargo line this tree is kept buildable
/// for.** One list, in one place, and the sweep prints it as it reads —
/// what is not here is not kept for, and is built again the next time
/// somebody types it.
///
/// Three kinds of line are in it. The gate's own, which is most of it
/// (`gate::plan`). The shapes a session types by hand between gates —
/// `cargo check` per crate while writing, the workspace-wide clippy and
/// test, the app built without the harness — which resolve features
/// differently from the gate's per-package lines and so are units of
/// their own, not a subset. And the two builds the runner makes for
/// itself: the task runner the gate's steps start from, and the release
/// every verify-ui run drives.
///
/// `Line::host_only` keeps the container's volume out of the lines
/// nothing in there ever runs. On a Windows or macOS host the container
/// builds in a volume of its own, so a tree here is the host's alone and
/// a run inside the container is that volume's alone; a Linux host
/// builds both sides in the one directory and every line is its.
struct Line {
    /// Not read inside the gate's container: a line in there that this
    /// tree has never built is a compile of the whole of it, for a
    /// shape nobody types there (`linux`).
    host_only: bool,
    /// The profile directory it fills, which is also the answer to
    /// whether it may be replayed at all: one that is not there yet
    /// would be built by the asking.
    profile: &'static str,
    /// The words after `cargo`, with `{package}` standing for each
    /// package in turn.
    words: &'static [&'static str],
    /// Only for the packages this says yes to, when the line has
    /// `{package}` in it.
    packages: fn(&str) -> bool,
}

fn every_package(_: &str) -> bool {
    true
}

/// For a line with no `{package}` in it — it names whatever it names.
fn no_package(_: &str) -> bool {
    false
}

/// This runner's own package, for the lines a session types about it and
/// nothing else.
fn the_runner(package: &str) -> bool {
    package == "xtask"
}

const CANONICAL: &[Line] = &[
    // The task runner the gate's steps start from (`gate::runner`), and
    // the one a session builds to run any verb at all. No `cargo test`
    // line covers it: a bin built for tests is a unit of its own.
    Line {
        host_only: false,
        profile: "debug",
        words: &["build", "--locked", "-p", "xtask"],
        packages: no_package,
    },
    // What a session runs while writing, crate by crate. Out here only:
    // nothing types a bare `check` into the container, and a line in
    // there that has never been built is a compile of the whole of it.
    Line {
        host_only: true,
        profile: "debug",
        words: &["check", "--locked", "-p", "{package}"],
        packages: every_package,
    },
    // The app checked with the harness on, which is what every verify-ui
    // run is a build of.
    Line {
        host_only: true,
        profile: "debug",
        words: &[
            "check",
            "--locked",
            "-p",
            "platitude-app",
            "--features",
            HARNESS,
        ],
        packages: no_package,
    },
    // The gate's clippy, per crate the reach enters (`gate::plan`).
    Line {
        host_only: false,
        profile: "debug",
        words: &[
            "clippy",
            "--locked",
            "-p",
            "{package}",
            "--all-targets",
            "--all-features",
        ],
        packages: every_package,
    },
    // `cargo xtask linux clippy -p xtask`, which a session runs in the
    // container ahead of the gate (internal-docs/反映前テストの機械化.md):
    // neither `--all-targets` nor `--all-features`, so its units are not
    // the line above's.
    Line {
        host_only: false,
        profile: "debug",
        words: &["clippy", "--locked", "-p", "{package}"],
        packages: the_runner,
    },
    // The whole workspace at once, which resolves features across the
    // members and so answers for units no per-package line does.
    Line {
        host_only: true,
        profile: "debug",
        words: &[
            "clippy",
            "--locked",
            "--workspace",
            "--all-targets",
            "--all-features",
        ],
        packages: no_package,
    },
    // The gate's unit and integration tests. `--no-run` is the whole
    // difference from the gate's line: which tests run is a filter, and
    // filters never decide which units are built.
    Line {
        host_only: false,
        profile: "debug",
        words: &["test", "--locked", "-p", "{package}", "--no-run"],
        packages: tested_here,
    },
    Line {
        host_only: true,
        profile: "debug",
        words: &["test", "--locked", "--workspace", "--no-run"],
        packages: no_package,
    },
    // The release every verify-ui run and every window drives
    // (`tree::app_exe`), which `bare` then reads out of the volume.
    Line {
        host_only: false,
        profile: "release",
        words: &["build", "--locked", "--release", "--features", HARNESS],
        packages: no_package,
    },
    // The same profile without the harness, which is what a session
    // types when it wants the product rather than a driven one.
    Line {
        host_only: true,
        profile: "release",
        words: &["build", "--locked", "--release", "-p", "platitude-app"],
        packages: no_package,
    },
    // Host only, as the gate's `shipped` step is: the container never
    // builds this profile (`gate::plan`).
    Line {
        host_only: true,
        profile: SHIPPED,
        words: &["build", "--locked", "--profile", SHIPPED],
        packages: no_package,
    },
];

/// Whether this side's tests are built for `package`. The container
/// builds the two packages that hold code behind `cfg(not(windows))`,
/// and not the app, which the gate does not test there either
/// (`gate::tested_on_linux`).
fn tested_here(package: &str) -> bool {
    on_the_host() || crate::gate::tested_on_linux(package)
}

/// Whether this process is out here rather than inside the gate's
/// container (`linux::IN_CONTAINER`).
fn on_the_host() -> bool {
    std::env::var_os(crate::linux::IN_CONTAINER).is_none()
}

impl Line {
    /// Whether this line belongs to the build directory this process is
    /// looking at.
    fn is_here(&self) -> bool {
        !self.host_only || on_the_host()
    }
}

pub fn run(args: &[String]) -> Result<(), String> {
    let told = match args {
        [] => Told::Now,
        [one] if one == "--dry-run" => Told::DryRun,
        [one] if one == ONLY_IF_MOVED => Told::IfTheGenerationMoved,
        other => {
            return Err(format!(
                "unknown option {:?} (sweep takes --dry-run or {ONLY_IF_MOVED})",
                other.join(" ")
            ));
        }
    };
    let (root, _busy) = crate::still::announced("sweep")?;
    match told {
        Told::Now => sweep(&root, false),
        Told::DryRun => sweep(&root, true),
        // The tail's half, run against the build directory this process
        // is in — which out here is a seat's `target/` and in the
        // container is the mounted volume. Never a verdict: it is
        // called by a gate or a landing that has already answered
        // ([`at_a_tail`]).
        Told::IfTheGenerationMoved => {
            here(&root, false);
            Ok(())
        }
    }
}

/// What the verb was told to do.
enum Told {
    Now,
    DryRun,
    IfTheGenerationMoved,
}

/// What the tail of a gate or a landing hands the sweep.
pub(crate) struct Tail {
    /// Sweep whatever the generation key says — on both sides, each
    /// against its own. Stage 3 has asked the tree for everything
    /// already, so the reading costs it nothing it was not going to pay.
    pub whatever_the_key_says: bool,
    /// The container's build volume is this run's to ask as well.
    /// **False for a tier that started no container**: the daily one
    /// promises not to, and a sweep is no reason to break that
    /// (CLAUDE.md 確認は 3 段). The volume then waits for the next run
    /// that has a Linux side.
    pub the_volume_too: bool,
}

impl Tail {
    /// A landing's: it rebased onto whatever main brought and gated both
    /// sides, so the volume is its to sweep — and the key decides, as it
    /// does for every tier but the full one.
    pub(crate) fn after_a_landing() -> Self {
        Self {
            whatever_the_key_says: false,
            the_volume_too: true,
        }
    }
}

/// The sweep a passing gate or landing ends with — **two build
/// directories, and each one's own key answers for it** ([`asks`]).
///
/// **The key is what makes it happen at all.** A seat runs the full tier
/// a handful of times a month while a generation is left behind by every
/// `Cargo.lock` that lands — five of them in three weeks, measured — so
/// a sweep hung on stage 3 alone would run long after the disk was gone.
///
/// Never a failure of the gate or the landing that calls it: the tests
/// have answered, and a sweep that could not read the tree is a sweep,
/// not a verdict. That holds per side as well — a tree out here that
/// could not be read says nothing about the volume, so the volume is
/// asked either way.
pub(crate) fn at_a_tail(dir: &Path, tail: &Tail) {
    here(dir, tail.whatever_the_key_says);
    let Some(whatever_the_key_says) = asks(tail) else {
        return;
    };
    if let Err(why) = crate::linux::sweep_the_volume(dir, whatever_the_key_says) {
        println!("sweep: the container's build volume was left alone — {why}");
    }
}

/// On what terms a tail asks the container's build volume, or `None`
/// for a tier that started no container.
///
/// **Never on this tree's key.** The volume is a build directory of its
/// own — its own rustc, its own generation key, its own stamp inside it
/// — and the verb that runs in there is what reads them ([`IF_MOVED`]).
/// A tail that let this tree's key answer for both would skip the
/// container every time something out here had already swept, which is
/// most days: `gate --host-only` and a hand-typed `cargo xtask sweep`
/// both write this tree's stamp and start nothing. Measured while that
/// was the rule: a volume of 10.9GB with no stamp in it at all, beside a
/// tree out here swept twice
/// (internal-docs/反映前テストの機械化.md §世代の掃除).
///
/// The start costs seconds against an image already built, so it is one
/// per tail whatever the answer turns out to be in there.
fn asks(tail: &Tail) -> Option<bool> {
    tail.the_volume_too.then_some(tail.whatever_the_key_says)
}

/// The half of a tail that sweeps the build directory this process is
/// looking at, against that directory's own stamp.
fn here(dir: &Path, whatever_the_key_says: bool) {
    let was = stamped(dir).map(|(key, _)| key);
    match generation(dir) {
        Ok(key) if !whatever_the_key_says && was.as_deref() == Some(key.as_str()) => {
            println!("sweep: generation unchanged ({key}) — nothing to take away");
            return;
        }
        // Said before the reading, because the reading is where the
        // minutes are: a line this tree has never built is compiled
        // here, once.
        Ok(key) => println!(
            "sweep: generation {} — reading the canonical lines",
            match &was {
                Some(was) => format!("{was} -> {key}"),
                None => format!("{key}, unswept"),
            }
        ),
        Err(why) => {
            println!("sweep: the generation could not be read — {why}");
            return;
        }
    }
    if let Err(why) = sweep(dir, false) {
        println!("sweep: nothing taken away — {why}");
    }
}

/// Reads the live set, walks each profile directory it answers for, and
/// leaves this tree's key behind.
fn sweep(root: &Path, dry_run: bool) -> Result<(), String> {
    let floor = stamped(root).map(|(_, at)| at);
    match floor {
        Some(at) => println!(
            "sweep: keeping everything written since the last sweep here ({})",
            ago(at)
        ),
        None => println!("sweep: nothing has been swept here before"),
    }
    let (live, compiled) = live(root)?;
    if compiled > 0 {
        println!(
            "sweep: {compiled} unit(s) were compiled to be read — this tree had not built them. \
             Once per generation, and the same work the full tier asks for."
        );
    }
    let mut freed = 0;
    let mut all_walked = true;
    for profile in PROFILES {
        let Some(alive) = live.get(profile) else {
            continue;
        };
        // A profile whose lines named nothing is a profile whose lines
        // did not run, and an empty live set reads as "all of it is
        // dead". Every road here is guarded by this: the sweep is the
        // one command whose failure mode is deleting a whole build.
        if alive.deps.is_empty() && alive.build.is_empty() {
            println!(
                "  {profile}: left alone — the canonical lines named nothing there, so nothing \
                 can be called dead"
            );
            all_walked = false;
            continue;
        }
        let swept = sweep_profile(
            &root.join("target").join(profile),
            profile,
            alive,
            floor,
            dry_run,
        )?;
        freed += swept.freed;
        all_walked &= swept.walked;
    }
    println!(
        "sweep: {} {}",
        size(freed),
        if dry_run { "would go" } else { "freed" }
    );
    // **The stamp says every profile was read**, so a run that skipped
    // one — cargo was building in it, or its lines named nothing — is
    // one the next tail does again. A stamp written over a partial sweep
    // would hold the generation closed until the next lock file moved.
    if !dry_run && all_walked {
        stamp(root)?;
    }
    Ok(())
}

/// What one profile directory holds that the canonical lines named.
#[derive(Default, Debug)]
struct Alive {
    /// Unit keys ([`unit_key`]) of the files under `deps/`.
    deps: BTreeSet<String>,
    /// Directory names under `build/` — a build script's own compile and
    /// the directory its run wrote into, which are two units and two
    /// names.
    build: BTreeSet<String>,
}

/// Every unit the canonical lines stand on, per profile directory, and
/// how many units the reading had to compile.
fn live(root: &Path) -> Result<(BTreeMap<String, Alive>, usize), String> {
    let said = root.join("target").join(STAMP[0]);
    std::fs::create_dir_all(&said).map_err(|e| format!("{}: {e}", said.display()))?;
    // The logs are named by the line's place in the list, so a list that
    // has since grown shorter would leave the last run's answers beside
    // this one's under names this run never wrote.
    for stale in read_dir(&said)? {
        if stale.file_name().to_string_lossy().starts_with(REPLAY) {
            let _ = std::fs::remove_file(stale.path());
        }
    }
    let mut live: BTreeMap<String, Alive> = BTreeMap::new();
    let mut compiled = 0;
    for (at, line) in lines(root)?.iter().enumerate() {
        let mut cmd = std::process::Command::new("cargo");
        cmd.args(line)
            .arg("--message-format=json-render-diagnostics")
            .current_dir(root);
        // Under this verb's own announcement, so a build inside it is
        // not a second thing for a measurement to wait for.
        crate::still::step(&mut cmd);
        let log = said.join(format!("{REPLAY}{at}.json"));
        println!("  cargo {}", line.join(" "));
        match crate::subprocess::bounded("a sweep's reading", cmd, REPLAY_CEILING, Some(&log)) {
            Answer::Ended { status, stdout } if status.success() => {
                compiled += read_artifacts(root, &stdout, &mut live);
            }
            Answer::Ended { status, .. } => {
                return Err(format!(
                    "`cargo {}` exited {:?} — the live set would be short by everything it \
                     stands on, so nothing is swept (its answer: {})",
                    line.join(" "),
                    status.code(),
                    log.display()
                ));
            }
            Answer::OutOfTime { after, .. } => {
                return Err(format!(
                    "`cargo {}` was still going after {:.0}s",
                    line.join(" "),
                    after.as_secs_f32()
                ));
            }
            Answer::Unstarted(why) => return Err(why),
        }
    }
    Ok((live, compiled))
}

/// [`CANONICAL`] as this side's lines over this tree's packages.
///
/// A profile directory that is not there is skipped: its line would
/// build the whole of it, and a sweep that builds something in order to
/// find out what to keep has kept the wrong thing.
fn lines(root: &Path) -> Result<Vec<Vec<String>>, String> {
    let packages = packages(root)?;
    let mut lines: Vec<Vec<String>> = Vec::new();
    for line in CANONICAL {
        if !line.is_here() || !root.join("target").join(line.profile).is_dir() {
            continue;
        }
        if !line.words.contains(&"{package}") {
            lines.push(line.words.iter().map(|w| (*w).to_string()).collect());
            continue;
        }
        for package in packages.iter().filter(|p| (line.packages)(p)) {
            lines.push(
                line.words
                    .iter()
                    .map(|word| {
                        if *word == "{package}" {
                            package.clone()
                        } else {
                            (*word).to_string()
                        }
                    })
                    .collect(),
            );
        }
    }
    Ok(lines)
}

/// The workspace's packages, by the directory each stands in — the same
/// reading the gate's graph does (`gate::graph::build`), and the same
/// answer, because this workspace names every package after its
/// directory.
fn packages(root: &Path) -> Result<Vec<String>, String> {
    let crates = root.join("crates");
    let mut packages: Vec<String> = std::fs::read_dir(&crates)
        .map_err(|e| format!("{}: {e}", crates.display()))?
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    packages.sort();
    Ok(packages)
}

/// Folds one line's messages into the live set, and answers how many of
/// its units cargo had to compile.
///
/// Two message kinds carry a path: `compiler-artifact`, whose
/// `filenames` name what rustc wrote, and `build-script-executed`, whose
/// `out_dir` names the directory a build script's run filled. Both are
/// emitted for units cargo found fresh, which is what makes reading a
/// built tree cost nothing.
fn read_artifacts(root: &Path, text: &str, live: &mut BTreeMap<String, Alive>) -> usize {
    let mut compiled = 0;
    for line in text.lines() {
        if line.contains("\"fresh\":false") {
            compiled += 1;
        }
        for path in strings_after(line, "\"filenames\":[").into_iter().chain(
            string_after(line, "\"out_dir\":")
                .into_iter()
                // The out directory is `<profile>/build/<unit>/out`; the
                // unit is its parent, as every other build entry is.
                .filter_map(|out| parent_of(&out)),
        ) {
            note(root, &path, live);
        }
    }
    compiled
}

/// `path`'s place in the tree, as the live set records it — and nothing
/// when it is not under a profile directory this sweeps.
fn note(root: &Path, path: &str, live: &mut BTreeMap<String, Alive>) {
    let under = format!("{}/target/", slashed(&root.display().to_string()));
    let Some(rest) = slashed(path).strip_prefix(&under).map(str::to_string) else {
        return;
    };
    let mut parts = rest.split('/');
    let (Some(profile), Some(first)) = (parts.next(), parts.next()) else {
        return;
    };
    if !PROFILES.contains(&profile) {
        return;
    }
    let alive = live.entry(profile.to_string()).or_default();
    match (first, parts.next()) {
        // `build/<unit>/…` — the directory is the unit.
        ("build", Some(unit)) => {
            alive.build.insert(unit.to_string());
        }
        ("deps", Some(name)) => {
            alive.deps.insert(unit_key(name));
        }
        // A lib or a bin cargo lifted to the profile's own directory.
        // Its twin under `deps/` carries the same key and no hash at
        // all, so the uplifted name answers for it — **under both
        // spellings**: a bin target named with a dash stands in `deps/`
        // as the dashed name and as the crate name rustc built it under
        // (`pgg-todo-editor.exe` beside `pgg_todo_editor.exe`), and on a
        // system with no separate debug file only one of the two is ever
        // reported.
        (name, None) => {
            let key = unit_key(name);
            alive.deps.insert(key.replace('-', "_"));
            alive.deps.insert(key);
        }
        _ => {}
    }
}

/// The name every file of one unit shares: the file name with its
/// extension off and, where rustc put one on, its `lib` prefix
/// ([`LIB_PREFIXED`]).
///
/// The last extension only, so a linker's byproduct beside a
/// proc-macro's library — `cxxbridge_macro-<hash>.dll.lib` — keys under
/// `…dll` rather than under the unit. That is not a hole: cargo names
/// those files itself in the unit's `filenames`, so the key they make is
/// one the live set holds.
fn unit_key(file_name: &str) -> String {
    match file_name.rsplit_once('.') {
        Some((stem, extension)) if LIB_PREFIXED.contains(&extension) => {
            stem.strip_prefix("lib").unwrap_or(stem).to_string()
        }
        Some((stem, _)) => stem.to_string(),
        // A Unix executable, which has no extension at all.
        None => file_name.to_string(),
    }
}

/// The crate rustc names an incremental directory after, off a unit key:
/// the key without the hash cargo hands it as `-C metadata`. A bin cargo
/// writes under its own name has no hash and is its own crate name.
fn crate_of(key: &str) -> &str {
    match key.rsplit_once('-') {
        Some((name, hash)) if is_hash(hash) => name,
        _ => key,
    }
}

/// The 16 hex digits cargo spells a unit hash with.
fn is_hash(text: &str) -> bool {
    text.len() == 16 && text.chars().all(|c| c.is_ascii_hexdigit())
}

/// Walks one profile directory, taking away what neither the live set
/// nor `floor` speaks for, and answers the bytes that went.
///
/// `.fingerprint/` is left alone whatever it holds: an entry there is
/// kilobytes, and the one road to reading it would be to name its
/// directories off the live set — which a bin cargo writes unhashed
/// cannot be, so the sweep would take away the bookkeeping of a unit
/// that is standing.
fn sweep_profile(
    dir: &Path,
    profile: &str,
    alive: &Alive,
    floor: Option<SystemTime>,
    dry_run: bool,
) -> Result<Swept, String> {
    if !dir.is_dir() {
        return Ok(Swept {
            freed: 0,
            walked: true,
        });
    }
    let _locked = match cargo_is_out_of(dir) {
        Ok(held) => held,
        Err(why) => {
            println!("  {profile}: left alone — {why}");
            return Ok(Swept {
                freed: 0,
                walked: false,
            });
        }
    };
    let mut gone = Gone::new(profile, dry_run);
    for entry in read_dir(&dir.join("deps"))? {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        if !alive.deps.contains(&unit_key(&name)) && !since(&path, floor) {
            gone.take(&path, &format!("deps/{name}"));
        }
    }
    for entry in read_dir(&dir.join("build"))? {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        if !alive.build.contains(&name) && !since(&path, floor) {
            gone.take(&path, &format!("build/{name}"));
        }
    }
    sweep_incremental(dir, alive, floor, &mut gone)?;
    Ok(Swept {
        freed: gone.done(),
        walked: true,
    })
}

/// What one profile directory came to: the bytes that went, and whether
/// it was read at all — a directory left alone is one the next tail owes
/// again.
struct Swept {
    freed: u64,
    walked: bool,
}

/// The incremental sessions, which no cargo message names.
///
/// **They are dated by the compile that wrote them.** rustc finalises a
/// unit's session in the same run that writes the unit's artifact, so a
/// session belongs to a live unit exactly when a live file of the same
/// crate carries the same moment ([`SAME_COMPILE`]). A session whose
/// crate has moved on — a feature set, a lock file, a toolchain — stands
/// alone at a moment nothing live shares, which is what makes it
/// readable at all. Taking a live one costs one compile that starts from
/// nothing; leaving a dead one costs the disk it is on, so the reading
/// is deliberately generous.
///
/// A `-working` session is rustc's own interrupted one — a compile that
/// was killed — and is dead whatever its date.
fn sweep_incremental(
    dir: &Path,
    alive: &Alive,
    floor: Option<SystemTime>,
    gone: &mut Gone,
) -> Result<(), String> {
    let incremental = dir.join("incremental");
    let compiles = compiles_of(dir, alive);
    for entry in read_dir(&incremental)? {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        if since(&path, floor) || kept_incremental(&name, &path, &compiles) {
            for session in read_dir(&path)? {
                let session_name = session.file_name().to_string_lossy().into_owned();
                if session_name.ends_with("-working") {
                    gone.take(
                        &session.path(),
                        &format!("incremental/{name}/{session_name}"),
                    );
                }
            }
            continue;
        }
        gone.take(&path, &format!("incremental/{name}"));
    }
    Ok(())
}

/// Whether the session directory `name` stands at a moment one of its
/// crate's live compiles does.
fn kept_incremental(name: &str, path: &Path, compiles: &BTreeMap<String, Vec<SystemTime>>) -> bool {
    let Some((krate, _)) = name.rsplit_once('-') else {
        // Not a name rustc wrote. Left alone: what this reads is the
        // shape rustc puts there, and anything else is somebody's.
        return true;
    };
    let Some(times) = compiles.get(krate) else {
        return false;
    };
    let Some(written) = written_at(path) else {
        // A date nobody can read is no grounds for deleting
        // (`verify::ownership::sweep_older_than` says the same).
        return true;
    };
    times.iter().any(|at| apart(*at, written) <= SAME_COMPILE)
}

/// When each crate's live units were last compiled, by the dates of the
/// files that compile wrote. A build script's compile is filed under the
/// crate rustc builds it as, which is the name its incremental session
/// carries.
fn compiles_of(dir: &Path, alive: &Alive) -> BTreeMap<String, Vec<SystemTime>> {
    let mut compiles: BTreeMap<String, Vec<SystemTime>> = BTreeMap::new();
    for entry in read_dir(&dir.join("deps")).unwrap_or_default() {
        let key = unit_key(&entry.file_name().to_string_lossy());
        if !alive.deps.contains(&key) {
            continue;
        }
        if let Some(at) = written_at(&entry.path()) {
            compiles
                .entry(crate_of(&key).to_string())
                .or_default()
                .push(at);
        }
    }
    for entry in read_dir(&dir.join("build")).unwrap_or_default() {
        if !alive
            .build
            .contains(&entry.file_name().to_string_lossy().into_owned())
        {
            continue;
        }
        // The compiled build script, which is the only half of a build
        // directory rustc wrote — a run's `out/` is the script's own
        // output and carries no incremental session of its own.
        for file in read_dir(&entry.path()).unwrap_or_default() {
            let stem = unit_key(&file.file_name().to_string_lossy()).replace('-', "_");
            if let Some(at) = written_at(&file.path())
                && stem.starts_with("build_script_")
            {
                compiles.entry(stem).or_default().push(at);
            }
        }
    }
    compiles
}

/// Whether `path` was written after `floor` — the recency floor, which
/// speaks for everything somebody has used since the last sweep here.
fn since(path: &Path, floor: Option<SystemTime>) -> bool {
    match (floor, written_at(path)) {
        (Some(floor), Some(written)) => written >= floor,
        // No floor is the first sweep in this tree, and a date nobody
        // can read is not a claim to have been used.
        _ => false,
    }
}

/// When `path` was written.
///
/// **For a directory that is the newest file inside it, not the
/// directory's own date.** A directory's date moves when an entry is
/// added or taken away — including by this very sweep, which takes
/// rustc's interrupted sessions out of the ones it keeps, and which
/// would then read every directory it had ever touched as live. The
/// files inside carry the compile that wrote them and nothing else.
/// Measured on this tree: over forty incremental directories, the newest
/// file two levels in is the directory's own date to the second, one
/// directory excepted and that one by a second.
fn written_at(path: &Path) -> Option<SystemTime> {
    let own = std::fs::metadata(path).ok()?;
    if !own.is_dir() {
        return own.modified().ok();
    }
    // Two levels: an incremental directory holds sessions, and a session
    // holds the files rustc wrote. A directory with nothing in it at all
    // has only its own date to give.
    newest_under(path, 2).or_else(|| own.modified().ok())
}

/// The newest file at most `depth` levels under `dir`.
fn newest_under(dir: &Path, depth: usize) -> Option<SystemTime> {
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter_map(|entry| {
            let meta = entry.metadata().ok()?;
            if !meta.is_dir() {
                return meta.modified().ok();
            }
            (depth > 1)
                .then(|| newest_under(&entry.path(), depth - 1))
                .flatten()
        })
        .max()
}

fn apart(one: SystemTime, other: SystemTime) -> Duration {
    one.duration_since(other)
        .or_else(|_| other.duration_since(one))
        .unwrap_or_default()
}

/// The entries of `dir`, sorted, and none where there is no such
/// directory — a profile that has never held one of these is not a
/// failure to read it.
fn read_dir(dir: &Path) -> Result<Vec<std::fs::DirEntry>, String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(Vec::new());
    };
    let mut entries: Vec<std::fs::DirEntry> = entries
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("{}: {e}", dir.display()))?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    Ok(entries)
}

/// What went, said as it goes.
///
/// **Every removal is printed.** A command that quietly frees gigabytes
/// is one nobody can audit, which is the rule `linux::forget_older_images`
/// already stands on; and what will not go is printed too, because on
/// Windows that means something is standing on it.
struct Gone {
    profile: String,
    dry_run: bool,
    bytes: u64,
    count: usize,
    stood: usize,
}

impl Gone {
    fn new(profile: &str, dry_run: bool) -> Self {
        Self {
            profile: profile.to_string(),
            dry_run,
            bytes: 0,
            count: 0,
            stood: 0,
        }
    }

    fn take(&mut self, path: &Path, name: &str) {
        let bytes = weight(path);
        if self.dry_run {
            self.bytes += bytes;
            self.count += 1;
            println!("  {}/{name} would go ({})", self.profile, size(bytes));
            return;
        }
        let removed = if path.is_dir() {
            std::fs::remove_dir_all(path)
        } else {
            std::fs::remove_file(path)
        };
        match removed {
            Ok(()) => {
                self.bytes += bytes;
                self.count += 1;
                println!("  {}/{name} ({})", self.profile, size(bytes));
            }
            Err(error) => {
                self.stood += 1;
                println!("  {}/{name} stayed — {error}", self.profile);
            }
        }
    }

    fn done(self) -> u64 {
        println!(
            "  {}: {} {} in {} {}{}",
            self.profile,
            if self.dry_run { "would free" } else { "freed" },
            size(self.bytes),
            self.count,
            if self.count == 1 { "entry" } else { "entries" },
            if self.stood == 0 {
                String::new()
            } else {
                format!(" ({} stayed)", self.stood)
            }
        );
        self.bytes
    }
}

/// Everything under `path`, or the file's own length. Best effort: an
/// entry that cannot be measured still goes, and the total says less
/// than went rather than refusing to say anything.
fn weight(path: &Path) -> u64 {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return 0;
    };
    if !meta.is_dir() {
        return meta.len();
    }
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries.flatten().map(|entry| weight(&entry.path())).sum()
}

fn size(bytes: u64) -> String {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a size is read, not computed on"
    )]
    let mut left = bytes as f64;
    let mut unit = "B";
    for next in ["KB", "MB", "GB"] {
        if left < 1024.0 {
            break;
        }
        left /= 1024.0;
        unit = next;
    }
    format!("{left:.1}{unit}")
}

/// The cargo build lock of `dir`, held for as long as the sweep walks
/// it — or why it could not be taken.
///
/// cargo holds these for the length of a build, so this is the one
/// question that has to be asked before anything is removed: a unit
/// being linked right now is live, whatever a reading taken before it
/// said. Every lock file cargo keeps there is taken, because which of
/// them a build holds is cargo's business and not this file's.
fn cargo_is_out_of(dir: &Path) -> Result<Vec<crate::locks::Locked>, String> {
    let mut held = Vec::new();
    for entry in read_dir(dir)? {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with(".cargo-") || !name.ends_with("lock") {
            continue;
        }
        let file = std::fs::File::open(entry.path())
            .map_err(|e| format!("{} could not be opened ({e})", entry.path().display()))?;
        match file.try_lock() {
            Ok(()) => held.push(crate::locks::Locked::new(file)),
            Err(TryLockError::WouldBlock) => {
                return Err(format!("cargo is building here — {name} is held"));
            }
            Err(TryLockError::Error(error)) => {
                return Err(format!("{name} could not be probed ({error})"));
            }
        }
    }
    Ok(held)
}

// ---- the generation ---------------------------------------------------

/// The key a build directory's whole contents hang on: the compiler, the
/// resolved dependency graph, the profile settings every unit is
/// compiled under, and the toolchain the tree pins. Move any of them and
/// cargo gives every unit a new hash, which is exactly the event that
/// leaves a generation behind.
///
/// FNV-1a, as `linux::image_tag` fingerprints an image: a name, not a
/// digest to defend anything with.
fn generation(root: &Path) -> Result<String, String> {
    let rustc = crate::subprocess::run_captured(std::process::Command::new("rustc").arg("-vV"))?;
    if !rustc.status.success() {
        return Err("rustc -vV failed".into());
    }
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |bytes: &[u8]| {
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    eat(&rustc.stdout);
    for name in ["Cargo.lock", "rust-toolchain.toml"] {
        let path = root.join(name);
        eat(&std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?);
    }
    let manifest = root.join("Cargo.toml");
    let text =
        std::fs::read_to_string(&manifest).map_err(|e| format!("{}: {e}", manifest.display()))?;
    eat(profiles_in(&text).as_bytes());
    Ok(format!("{hash:016x}"))
}

/// Every `[profile…]` section of a manifest, in the order it stands. The
/// rest of the file moves with the lock — a dependency added is a
/// dependency resolved — but a profile setting moves nothing else and
/// rehashes everything (Cargo.toml: `[profile.dev.package.xtask]`).
fn profiles_in(manifest: &str) -> String {
    let mut kept = String::new();
    let mut inside = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            inside = trimmed.starts_with("[profile");
        }
        if inside {
            kept.push_str(trimmed);
            kept.push('\n');
        }
    }
    kept
}

/// The key this tree's last sweep ran under, and when it ran.
fn stamped(root: &Path) -> Option<(String, SystemTime)> {
    let text = std::fs::read_to_string(stamp_at(root)).ok()?;
    let (key, at) = text.trim().split_once(' ')?;
    Some((
        key.to_string(),
        UNIX_EPOCH + Duration::from_secs(at.parse().ok()?),
    ))
}

/// Leaves this run's key and this moment behind. A sweep that cannot
/// write its stamp is one the next gate repeats, which is the harmless
/// way round.
fn stamp(root: &Path) -> Result<(), String> {
    let key = generation(root)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    let at = stamp_at(root);
    std::fs::write(&at, format!("{key} {now}\n")).map_err(|e| format!("{}: {e}", at.display()))
}

fn stamp_at(root: &Path) -> std::path::PathBuf {
    root.join("target").join(STAMP[0]).join(STAMP[1])
}

/// How long ago `at` was, for a line a person reads.
fn ago(at: SystemTime) -> String {
    let Ok(since) = SystemTime::now().duration_since(at) else {
        return "just now".to_string();
    };
    let hours = since.as_secs() / 3600;
    if hours < 24 {
        return format!("{hours}h ago");
    }
    format!("{}d ago", hours / 24)
}

// ---- the small JSON reading this needs -------------------------------
//
// cargo's messages are one object a line, and what is wanted of them is
// two fields' worth of paths. A reader for those two is a dozen lines
// and carries no dependency (CLAUDE.md 技術スタック: xtask is std
// alone); a parser for the whole of JSON would be a page of code for
// fields nothing here reads.

/// The strings of the array that follows `key` in `line`.
fn strings_after(line: &str, key: &str) -> Vec<String> {
    let Some(at) = line.find(key) else {
        return Vec::new();
    };
    let mut rest = &line[at + key.len()..];
    let mut out = Vec::new();
    loop {
        match rest.find(['"', ']']) {
            Some(quote) if rest.as_bytes()[quote] == b'"' => {
                let (text, after) = json_string(&rest[quote + 1..]);
                out.push(text);
                rest = after;
            }
            _ => return out,
        }
    }
}

/// The one string that follows `key` in `line`.
fn string_after(line: &str, key: &str) -> Option<String> {
    let at = line.find(key)?;
    let rest = &line[at + key.len()..];
    let quote = rest.find('"')?;
    Some(json_string(&rest[quote + 1..]).0)
}

/// The string literal `text` opens (past its quote), and what follows
/// it. Only the escapes cargo writes into a path are read back —
/// anything else is a byte of the path, which is what a name carrying a
/// backslash wants anyway.
fn json_string(text: &str) -> (String, &str) {
    let mut out = String::new();
    let mut chars = text.char_indices();
    while let Some((at, c)) = chars.next() {
        match c {
            '"' => return (out, &text[at + 1..]),
            '\\' => match chars.next() {
                Some((_, '\\')) => out.push('\\'),
                Some((_, '"')) => out.push('"'),
                Some((_, '/')) => out.push('/'),
                Some((_, 'n')) => out.push('\n'),
                Some((_, 'r')) => out.push('\r'),
                Some((_, 't')) => out.push('\t'),
                Some((_, other)) => out.push(other),
                None => return (out, ""),
            },
            other => out.push(other),
        }
    }
    (out, "")
}

fn slashed(path: &str) -> String {
    path.replace('\\', "/")
}

/// The directory `path` stands in, as a slashed string.
fn parent_of(path: &str) -> Option<String> {
    let slashed = slashed(path);
    let at = slashed.trim_end_matches('/').rfind('/')?;
    Some(slashed[..at].to_string())
}
