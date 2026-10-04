//! `cargo xtask linux <command…>` — the workspace on Linux, from a
//! workstation that has none. The rest of the line goes to cargo in a
//! container built from ci/linux/Dockerfile — by WSL's own engine on
//! Windows, docker elsewhere ([`engine`]); an xtask verb gets `cargo
//! xtask` in front. On Linux the container drops out and the command runs
//! where it stands.
//!
//! Four images (the Dockerfile's stages): `core` and `app` are chosen by
//! what the command needs ([`stage_for`]) — the small one when it will do,
//! so a run need not download Qt first; `bare` and `runtime` belong to the
//! `bare` verb.
//!
//! The build directory (/work/target) and the cargo registry are the
//! engine's volumes, never the host's: one target/ shared between two
//! operating systems is two cargos on one build lock and two sets of
//! fingerprints.
//! Only source reading crosses the host filesystem — slow, but once per
//! build, and a copy inside a volume would be a second answer to "which
//! tree is the real one".
//!
//! A worktree's `.git` names an absolute Windows path that git inside
//! cannot follow, so any git run with /work as its working directory sees
//! a broken repository. Nothing a run depends on stands there
//! (`verify::run` starts the app outside every checkout).

use std::collections::BTreeSet;
use std::io::IsTerminal;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::command::{self, Permission, Where};
use crate::keepsakes;

pub(crate) static RUN: command::Command = command::Command {
    id: "linux.run",
    call: "linux <command>",
    purpose: "run a command against this checkout on Ubuntu, in the container",
    run_in: Where::Seat,
    needs: &[
        "the container engine: WSL 3's wslc on Windows, docker elsewhere (on Linux the command \
         runs where it stands)",
    ],
    permission: Permission::Plain,
};

pub(crate) static VERIFY: command::Command = command::Command {
    id: "linux.verify",
    call: "linux verify-ui <verb>",
    purpose: "the same headless run on the other OS",
    run_in: Where::Seat,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static RUNNER: command::Command = command::Command {
    id: "linux.runner",
    call: "linux runner <name> --gate <pid>",
    purpose: "the gate's own step: build the task runner in there and install this run's copy",
    run_in: Where::Seat,
    needs: &["a gate running in this tree — it names the pid, and this refuses any other"],
    permission: Permission::Plain,
};

pub(crate) static STOP: command::Command = command::Command {
    id: "linux.stop",
    call: "linux --container <name> --step <mark> stop",
    purpose: "end what a verb left running in the gate's container, by the mark it carries",
    run_in: Where::Seat,
    needs: &["the container's name and the step's mark, off the gate's failure line"],
    permission: Permission::Plain,
};

pub(crate) static BARE: command::Command = command::Command {
    id: "linux.bare",
    call: "linux bare --discover",
    purpose: "whether the app runs on an Ubuntu carrying only the declared dependencies",
    run_in: Where::Seat,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static WITNESS: command::Command = command::Command {
    id: "linux.witness",
    call: "linux witness",
    purpose: "keep what stands on this host around a container engine that gives no answer, \
              before anything ends it",
    run_in: Where::Either,
    needs: &["Windows: what it takes is of wslc's session process"],
    permission: Permission::Plain,
};

pub(crate) static COMMANDS: &[&command::Command] =
    &[&RUN, &VERIFY, &RUNNER, &STOP, &BARE, &WITNESS];

mod bare;
pub(crate) mod container;
pub(crate) mod engine;
mod offline;
pub(crate) mod runner;
mod tested;
#[cfg(test)]
mod tests;
mod witness;

pub(crate) use runner::a_runner_verb;
pub(crate) use tested::tested_on_linux;

/// The image. Its tag names the stage and fingerprints what built it.
const IMAGE: &str = "pgg-linux";

/// Every stage ci/linux/Dockerfile builds (a test holds the two together).
/// A stage missing here is one whose images [`forget_older_images`] takes
/// away from the checkout using them.
const STAGES: [&str; 4] = ["core", "app", "bare", "runtime"];

/// The kinds of volume a checkout mounts ([`volume`]), and so leaves
/// behind when gone. `{IMAGE}-registry` is the machine's, not a
/// checkout's.
const VOLUME_KINDS: [&str; 2] = ["target", "demo"];

/// Where the checkout, the build directory, the download cache and anything
/// a run means to leave behind land inside the container.
const WORK: &str = "/work";
const TARGET_MOUNT: &str = "/work/target";
const REGISTRY_MOUNT: &str = "/usr/local/cargo/registry";
const OUT_MOUNT: &str = "/out";
/// Where the demo repositories a run builds go, and the template each is
/// copied from (`demo::template`). A volume so the template outlives the
/// one-verb container; one per checkout, since a template is keyed by
/// that checkout's preset sources.
pub(crate) const DEMO_MOUNT: &str = "/tmp/pgg-demo";
/// Set for everything the container runs, and by nothing else: the mark a
/// run reads to know it is not on the machine whose checkout it is
/// writing.
pub(crate) const IN_CONTAINER: &str = "PGG_IN_CONTAINER";

/// Task-runner verbs worth running in there. Naming one means `cargo xtask
/// <verb>`, so the command reads the same as on the host.
const XTASK_VERBS: [&str; 4] = ["verify-ui", "demo-repo", "qmltest", "sweep"];

/// Cargo verbs that build something (one-letter aliases included), and
/// so care which stage they run in. A verb missing here only picks the
/// smaller image and fails to build, loudly — unlike a missing lock flag
/// ([`UNLOCKED_VERBS`]).
const BUILD_VERBS: [&str; 11] = [
    "build", "check", "test", "clippy", "bench", "run", "doc", "b", "c", "t", "r",
];

/// The cargo verbs handed to the container without `--locked`; everything
/// else gets it.
///
/// Locked by default because /work is the host's checkout: a verb this
/// forgot would silently rewrite the host's lock file (CLAUDE.md 絶対制約:
/// a dependency change is a human's decision), while a verb wrongly given
/// the flag stops loudly. So only `fmt` (`cargo-fmt` rejects the flag) and
/// the verbs whose purpose is to move the lock or the manifest. A
/// third-party subcommand that rejects the flag: run it through
/// `--shell`, or add it here.
const UNLOCKED_VERBS: [&str; 9] = [
    "fmt",
    "update",
    "generate-lockfile",
    "add",
    "remove",
    "clean",
    "new",
    "init",
    "help",
];

/// The packages that hold no Qt. Anything else reaches platitude-app,
/// including a bare `cargo test` (the default members have the app).
const QT_FREE: [&str; 2] = ["platitude-core", "xtask"];

/// The leading options of a `linux` line, and where the command begins.
struct Options {
    rebuild: bool,
    shell: bool,
    stage: Option<String>,
    /// `--runner`: the name of a prepared copy to start the verb from
    /// ([`runner`]).
    copy: Option<String>,
    /// `--container`: the gate's own container to `exec` the verb
    /// in ([`container::exec_in`]). Only with `--runner`.
    container: Option<String>,
    /// `--step`: the mark every process of this verb carries inside
    /// that container, which a stop is addressed to
    /// ([`container::stop_step`]). Only with `--container`.
    step: Option<String>,
    at: usize,
}

/// Options are the leading tokens only: everything from the first one
/// that is not ours belongs to the command, `--` and all.
fn options(args: &[String]) -> Result<Options, String> {
    let mut opts = Options {
        rebuild: false,
        shell: false,
        stage: None,
        copy: None,
        container: None,
        step: None,
        at: 0,
    };
    while let Some(arg) = args.get(opts.at) {
        match arg.as_str() {
            "--rebuild" => opts.rebuild = true,
            "--shell" => opts.shell = true,
            "--runner" => {
                opts.at += 1;
                opts.copy = Some(
                    args.get(opts.at)
                        .ok_or("--runner needs the name a `linux runner <name>` prepared")?
                        .clone(),
                );
            }
            "--container" => {
                opts.at += 1;
                opts.container = Some(
                    args.get(opts.at)
                        .ok_or("--container needs the name of the gate's container")?
                        .clone(),
                );
            }
            "--step" => {
                opts.at += 1;
                opts.step = Some(
                    args.get(opts.at)
                        .ok_or("--step needs the mark this step's processes carry")?
                        .clone(),
                );
            }
            "--stage" => {
                opts.at += 1;
                let name = args.get(opts.at).ok_or("--stage needs core or app")?;
                if !matches!(name.as_str(), "core" | "app") {
                    return Err(format!("unknown stage {name:?}: core or app"));
                }
                opts.stage = Some(name.clone());
            }
            _ => break,
        }
        opts.at += 1;
    }
    let rest = &args[opts.at..];
    if !opts.shell && rest.is_empty() {
        return Err(
            "linux needs a command, e.g. `cargo xtask linux test -p platitude-core`".into(),
        );
    }
    if opts.shell && !rest.is_empty() {
        // Silently dropping the command would run bash where a check was
        // asked for.
        return Err(format!(
            "--shell takes no command (got {:?}) — drop --shell to run it, \
             or drop the command to get the shell",
            rest.join(" ")
        ));
    }
    // A container to exec into is for a prepared copy — except on `stop`,
    // which is about what is already in there — and a mark is for a
    // process in such a container.
    let stopping = rest.first().is_some_and(|verb| verb == "stop");
    if opts.container.is_some() && opts.copy.is_none() && !stopping {
        return Err(
            "--container starts a prepared copy in the gate's container: it needs \
                    --runner <name> beside it"
                .into(),
        );
    }
    if opts.step.is_some() != opts.container.is_some() {
        return Err(
            "--container <name> and --step <mark> go together: a verb in the gate's container \
             is addressed by the mark its processes carry, and a mark is for nothing else"
                .into(),
        );
    }
    if opts.rebuild && opts.container.is_some() {
        return Err(
            "--rebuild builds an image, and --container names a container already \
                    running from one: a line cannot ask for both"
                .into(),
        );
    }
    Ok(opts)
}

pub fn run(args: &[String]) -> Result<(), String> {
    let Options {
        rebuild,
        shell,
        stage: forced_stage,
        copy,
        container,
        step,
        at,
    } = options(args)?;
    let rest = &args[at..];
    let root = crate::tree::workspace_root();
    // Both are written into a shell script and a container name, so
    // they are names and nothing else (`runner::spelled`).
    if let Some(name) = &container {
        runner::spelled(name)?;
    }
    if let Some(mark) = &step {
        runner::spelled(mark)?;
    }
    // Ahead of the ticket: neither builds, and neither may wait for room.
    match rest.first().map(String::as_str) {
        // By hand, what the gate does at a ceiling (`gate::runner`), for
        // a gate that was killed with a verb still in.
        Some("stop") => {
            return stop(
                &root,
                container.as_deref(),
                step.as_deref(),
                copy.is_some() || shell,
                rest,
            );
        }
        // By hand, what a line takes when the engine gives it no answer.
        Some("witness") => return witness::by_hand(rest, at != 0),
        _ => {}
    }
    // A prepared copy starts the task runner's own verbs only; anything
    // else is refused rather than run through cargo with the flag
    // silently dropped (`runner`).
    if let Some(name) = &copy {
        runner::spelled(name)?;
        if !runner::a_runner_verb(rest) {
            return Err(format!(
                "--runner {name} starts one of the task runner's own verbs ({}) and nothing \
                 else — got {:?}. Drop --runner to run it through cargo in there.",
                XTASK_VERBS.join(" / "),
                rest.join(" ")
            ));
        }
    }
    let command = command_line(rest, copy.as_deref().map(|name| runner::at(&root, name)));
    // The container's work (image build included) is this machine's, and
    // is counted here (`crate::budget`), ahead of every road out of this
    // verb. Everything inside the container is under this ticket
    // ([`carried`] hands the mark across).
    let _room = crate::budget::standalone(
        &root,
        crate::budget::weight_of(&command, false),
        crate::budget::Rank::Normal,
        &format!("linux {}", rest.join(" ")),
    )?;
    // Declared after the ticket, so dropped before it ([`TrimTheCache`]).
    let _cache = TrimTheCache;
    // The verb that prepares what the others start from, on any host
    // (`runner::prepare`).
    if rest.first().is_some_and(|verb| verb == "runner") {
        return prepare_the_runner(&root, rest);
    }
    // In the container even on Linux: what it asks is whether a stock
    // Ubuntu is enough.
    if rest.first().is_some_and(|verb| verb == "bare") {
        let discover = rest.iter().any(|word| word == "--discover");
        return bare::bare(&root, discover);
    }
    // In the container even on Linux: what it asks is whether the suite
    // passes with no network, which the host has.
    if rest.first().is_some_and(|verb| verb == "offline") {
        if let Some(extra) = rest.get(1) {
            return Err(format!("offline takes no arguments (got {extra:?})"));
        }
        return offline::offline(&root);
    }
    if cfg!(target_os = "linux") {
        if shell {
            return Err("--shell has nothing to enter: this is already Linux".into());
        }
        if let Some(name) = &container {
            return Err(format!(
                "--container {name} has nothing to exec into: this is already Linux, and the \
                 command runs where it stands"
            ));
        }
        println!("already on Linux — running here, no container");
        return here(&root, &command);
    }

    // Announced here as well (its own xtask is under the announcement);
    // a verb run where it stands announces itself.
    let _busy = crate::still::busy(&root, "linux")?;
    // The gate's own container, already up, by `exec`: one that is not
    // there is a red step saying so, never a `run` in its place.
    if let (Some(name), Some(mark)) = (&container, &step) {
        return container::exec_in(name, &command, mark);
    }
    let stage = match &forced_stage {
        Some(name) => name.clone(),
        None => stage_for(rest).to_string(),
    };
    let tag = ensure_image(&root, &stage, rebuild)?;
    in_container(&root, &tag, &command, shell, copy.as_deref(), None)
}

/// `runner`, typed (`rest` from the verb on). A preparation builds in
/// this checkout's volume at its one preparation container, so the line
/// names the gate it is under, which has to be the live one
/// (`runner::owned_by_the_gate`).
fn prepare_the_runner(root: &Path, rest: &[String]) -> Result<(), String> {
    let [_, name, flag, pid] = rest else {
        return Err(format!(
            "`{}` is the gate's own step — it builds in this checkout's volume and \
             stands at its one preparation container, so it runs under a gate or not \
             at all. Run `cargo xtask gate`. (got {:?})",
            RUNNER.call,
            rest[1..].join(" ")
        ));
    };
    if flag != "--gate" {
        return Err(format!(
            "linux runner takes `{}` (got {flag:?})",
            RUNNER.call
        ));
    }
    let pid = pid
        .parse::<u32>()
        .map_err(|_| format!("--gate takes the pid of the gate that sent this; got {pid:?}"))?;
    runner::prepare(root, name, pid)
}

/// `stop`, typed: the container and the mark are the line's, and
/// nothing else on it may ask to start something.
fn stop(
    root: &Path,
    container: Option<&str>,
    mark: Option<&str>,
    starting: bool,
    rest: &[String],
) -> Result<(), String> {
    let (Some(name), Some(mark)) = (container, mark) else {
        return Err(
            "stop takes --container <name> --step <mark>: the container the verb is in \
             and the mark its processes carry (the step's log name after the run's)"
                .into(),
        );
    };
    if let Some(extra) = rest.get(1) {
        return Err(format!("stop takes no arguments (got {extra:?})"));
    }
    if starting {
        return Err("stop starts nothing: drop --runner and --shell".into());
    }
    // Exits as it found things: a walk that left something carrying
    // the mark is this command's failure, said in the same line.
    println!(
        "{}",
        container::stop_step(name, mark, &root.join("target").join("gate-logs"))?
    );
    Ok(())
}

/// The container's half of a tail's sweep (`crate::sweep`), run against
/// this checkout's build volume.
///
/// The volume carries its own stamp, so this starts whatever this
/// machine's key says and the verb compares the volume's keys
/// (`sweep::asks`); `whatever_the_key_says` (stage 3) skips that on both
/// sides alike. In a container of its own: the gate's goes down with its
/// side, and the volume outlives it. It never builds an image: a tag that
/// is not there is one nothing has built in the volume under.
pub(crate) fn sweep_the_volume(root: &Path, whatever_the_key_says: bool) -> Result<(), String> {
    if cfg!(target_os = "linux") {
        // There is no volume: the container drops out here and both
        // sides build in the one directory the caller has just swept.
        return Ok(());
    }
    let told = if whatever_the_key_says {
        &crate::sweep::SWEEP
    } else {
        &crate::sweep::IF_MOVED
    };
    let rest: Vec<String> = told.call.split_whitespace().map(str::to_string).collect();
    let tag = image_tag(root, stage_for(&rest))?;
    if !built(root, &tag)? {
        return Err(format!(
            "{tag} is not built, so nothing here has built in the volume either"
        ));
    }
    let command = command_line(&rest, None);
    let _room = crate::budget::standalone(
        root,
        crate::budget::weight_of(&command, false),
        crate::budget::Rank::Normal,
        "the container's sweep",
    )?;
    let _busy = crate::still::busy(root, "linux")?;
    // Housekeeping on this road can take a generation's last image away
    // ([`TrimTheCache`]).
    let _cache = TrimTheCache;
    in_container(root, &tag, &command, false, None, None)
}

fn ensure_image(root: &Path, stage: &str, rebuild: bool) -> Result<String, String> {
    let tag = image_tag(root, stage)?;
    if rebuild || !built(root, &tag)? {
        build_image(root, stage, &tag)?;
    }
    Ok(tag)
}

/// Whether the engine holds `tag` — an error, never `false`, where it
/// could not say: an engine that will not answer what it holds is in no
/// state to build, and a build started on its silence would bury the
/// trouble under a failure of its own.
///
/// The line's first word to the engine, so [`engine::Asked::First`].
fn built(root: &Path, tag: &str) -> Result<bool, String> {
    // This process's own: two lines of one tree can be asking at once.
    let said = root
        .join("target")
        .join(format!("linux-image-{}.txt", std::process::id()));
    if let Some(dir) = said.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let presence = engine::presence(engine::Kind::Image, tag, &said, engine::Asked::First);
    let _ = std::fs::remove_file(&said);
    built_off(tag, presence)
}

/// [`built`]'s reading of the engine's answer. Pure, so a test holds
/// that only an answered listing may send a line on to build.
fn built_off(tag: &str, presence: engine::Presence) -> Result<bool, String> {
    match presence {
        engine::Presence::There => Ok(true),
        engine::Presence::Gone => Ok(false),
        engine::Presence::Unknown(why) => Err(format!(
            "could not learn whether {tag} is built, and built nothing in its place — {why}"
        )),
    }
}

/// Cargo's own options that take their value in the next word: in
/// `cargo --config net.retry=2 test` the subcommand is not the first word
/// without a dash. (`--flag=value` is one word.)
const GLOBAL_OPTIONS_WITH_A_VALUE: [&str; 5] = ["--explain", "--color", "--config", "-C", "-Z"];

/// Where the subcommand stands in the line, past cargo's own options, or
/// `None` for a line that has none (`--version`, `--list`).
///
/// An unknown option is read as a flag, so its value would be taken for
/// the subcommand: the loud kind of wrong (cargo says "no such
/// subcommand"), never a line quietly unlocked ([`UNLOCKED_VERBS`]).
fn subcommand_at(rest: &[String]) -> Option<usize> {
    let mut at = 0;
    while let Some(word) = rest.get(at) {
        if word == "--" {
            return None;
        }
        if !word.starts_with('-') {
            return Some(at);
        }
        if GLOBAL_OPTIONS_WITH_A_VALUE.contains(&word.as_str()) {
            at += 1;
        }
        at += 1;
    }
    None
}

/// What to run inside: a cargo command, with `xtask` folded in for the
/// task runner's own verbs (the alias carries `--locked` already,
/// .cargo/config.toml), and `--locked` spelled on unless the subcommand
/// is one of [`UNLOCKED_VERBS`].
///
/// With `copy` (a prepared task runner, [`runner`]) the line starts from
/// it: no cargo, no resolve across the mount.
fn command_line(rest: &[String], copy: Option<String>) -> Vec<String> {
    if let Some(copy) = copy {
        let mut line = vec![copy];
        line.extend(rest.iter().cloned());
        return line;
    }
    let mut line = vec!["cargo".to_string()];
    line.extend(rest.iter().cloned());
    // A line with no subcommand resolves nothing, so there is nothing to
    // lock and nowhere to put it.
    let Some(at) = subcommand_at(rest) else {
        return line;
    };
    let verb = rest[at].as_str();
    if XTASK_VERBS.contains(&verb) {
        // At the subcommand, not always the front: `--offline verify-ui`.
        line.insert(at + 1, "xtask".to_string());
        return line;
    }
    // Only the caller's options: a `--locked` past `--` belongs to the
    // program being run. `--frozen` includes `--locked`.
    let options = rest.split(|word| word == "--").next().unwrap_or(rest);
    let spelled = options
        .iter()
        .any(|word| word == "--locked" || word == "--frozen");
    if !UNLOCKED_VERBS.contains(&verb) && !spelled {
        // Right after the subcommand, where the gate spells it
        // (`gate::plan`) — `line` carries "cargo" in front of `rest`.
        line.insert(at + 2, "--locked".to_string());
    }
    line
}

/// Which image the command needs. Nothing here is a guess about Qt itself:
/// either the command names only Qt-free packages, or it can reach the app.
fn stage_for(rest: &[String]) -> &'static str {
    let Some(verb) = subcommand_at(rest).map(|at| rest[at].as_str()) else {
        // A bare `--shell`, or a line that is all options. The small
        // image opens; --stage app asks for the other one.
        return "core";
    };
    if XTASK_VERBS.contains(&verb) {
        // verify-ui and qmltest need Qt; the rest are not worth a second
        // answer.
        return "app";
    }
    if !BUILD_VERBS.contains(&verb) {
        return "core";
    }
    let packages: Vec<&str> = rest
        .windows(2)
        .filter(|pair| matches!(pair[0].as_str(), "-p" | "--package"))
        .map(|pair| pair[1].as_str())
        .collect();
    if !packages.is_empty() && packages.iter().all(|p| QT_FREE.contains(p)) {
        "core"
    } else {
        "app"
    }
}

/// What every stage is built from; the stages past core add Qt's pin.
const IMAGE_INPUTS: [&str; 2] = ["ci/linux/Dockerfile", "rust-toolchain.toml"];

/// The image is named after what builds it, so a stale image can never be
/// the one that answers.
///
/// A dot after the stage. The tags hashed off CI's workflow had a dash
/// there ([`former_image_tag`]), and a runner from before the pin had a
/// file of its own removes every dashed tag its own formula does not
/// arrive at ([`stale_images`]): spelled its way, this tree's images would
/// go each time a seat that is behind ran a container.
fn image_tag(root: &Path, stage: &str) -> Result<String, String> {
    let mut inputs = IMAGE_INPUTS.to_vec();
    if stage != "core" {
        // Every stage past core is built with the Qt version the tree pins
        // (runtime inherits it through bare), so the tag has to move when
        // that does.
        inputs.push(crate::qt::PIN);
    }
    Ok(format!(
        "{IMAGE}:{stage}.{:016x}",
        fingerprint(root, &inputs)?
    ))
}

/// The tag a tree's own runner answers to where the tree holds no pin file:
/// a seat that is behind, or the perf rig at an older commit. Its runner
/// hashes CI's workflow for the Qt version and spells the tag with a dash.
/// Never this tree's own — the workflow here pins nothing.
fn former_image_tag(root: &Path, stage: &str) -> Result<String, String> {
    let workflow = crate::qt::former_pin();
    let mut inputs = IMAGE_INPUTS.to_vec();
    if stage != "core" {
        inputs.push(&workflow);
    }
    Ok(format!(
        "{IMAGE}:{stage}-{:016x}",
        fingerprint(root, &inputs)?
    ))
}

/// The tag `tree`'s own runner answers to, by the formula that runner has:
/// the pin file is what says which.
fn tag_named_by(tree: &Path, stage: &str) -> Result<String, String> {
    if tree.join(crate::qt::PIN).is_file() {
        image_tag(tree, stage)
    } else {
        former_image_tag(tree, stage)
    }
}

/// FNV-1a over the files: the same on all three systems, the tree being
/// LF everywhere (.gitattributes).
fn fingerprint(root: &Path, inputs: &[&str]) -> Result<u64, String> {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for relative in inputs {
        let path = root.join(relative);
        let bytes =
            std::fs::read(&path).map_err(|e| format!("failed to read {}: {e}", path.display()))?;
        for byte in bytes {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    Ok(hash)
}

/// The Qt version, read from the tree's pin: the one place it is pinned.
fn qt_version(root: &Path) -> Result<String, String> {
    let path = root.join(crate::qt::PIN);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    crate::qt::pinned_in(&text).ok_or_else(|| format!("no Qt version in {}", path.display()))
}

fn build_image(root: &Path, stage: &str, tag: &str) -> Result<(), String> {
    println!("building {tag} — the first one takes a while");
    // Ahead of the build: one that stops halfway has written cache
    // entries with no image to belong to.
    SHARING_MOVED.store(true, Ordering::Relaxed);
    let mut cmd = engine::command();
    cmd.arg("build")
        .arg("--file")
        .arg(root.join("ci").join("linux").join("Dockerfile"))
        .arg("--target")
        .arg(stage)
        .arg("--tag")
        .arg(tag);
    if stage != "core" {
        cmd.arg("--build-arg")
            .arg(format!("QT_VERSION={}", qt_version(root)?));
    }
    let status = crate::budget::watched(cmd.arg(root))
        .map_err(|e| format!("failed to run {} build: {e}", engine::NAME))?;
    if !status.success() {
        return Err(format!("{} build failed", engine::NAME));
    }
    Ok(())
}

/// The tags of `stage` that no checkout on this machine names, in either
/// shape a runner spells one ([`image_tag`], [`former_image_tag`]). Pure,
/// so a test holds the rule.
fn stale_images<'a>(listed: &'a str, stage: &str, keep: &BTreeSet<String>) -> Vec<&'a str> {
    let shapes = [format!("{IMAGE}:{stage}."), format!("{IMAGE}:{stage}-")];
    listed
        .lines()
        .map(str::trim)
        .filter(|tag| shapes.iter().any(|shape| tag.starts_with(shape)) && !keep.contains(*tag))
        .collect()
}

/// Removes the images that no checkout names: every change to what builds
/// an image leaves the last one behind, and nothing would name it again.
///
/// `keep` is every living checkout's tags, not this one's: a seat is
/// normally behind main (CLAUDE.md §Git 運用), so two generations can be
/// alive at once, each the only one some tree can use. Whether this line
/// built anything decides nothing — a tag dies with the last checkout
/// naming it.
///
/// The engine refuses to remove an image a running container uses, so a
/// run next door is safe. Every removal is printed. `said` is where the
/// engine's words go ([`engine::presence`]).
///
/// An error is an engine that gave no answer: the caller takes nothing
/// more away ([`housekeeping`]).
fn forget_older_images(
    listed: &str,
    stage: &str,
    keep: &BTreeSet<String>,
    said: &Path,
) -> Result<(), String> {
    for tag in stale_images(listed, stage, keep) {
        match engine::remove_image(tag, said) {
            engine::Removal::Done => {
                SHARING_MOVED.store(true, Ordering::Relaxed);
                println!("removed the older image {tag}");
            }
            engine::Removal::Refused(refused) => {
                after_a_failed_removal(engine::Kind::Image, tag, &refused, said)?;
            }
            engine::Removal::Unanswered(why) => return Err(why),
        }
    }
    Ok(())
}

/// What a removal the engine refused left: nothing said when the object
/// is gone (the seat next door swept it since the listing — the state
/// wanted), a line when it is still there. Where the engine could not be
/// asked, that is the error — an object that could not be asked after is
/// never reported gone, and the engine is asked for nothing more.
fn after_a_failed_removal(
    kind: engine::Kind,
    name: &str,
    refused: &str,
    said: &Path,
) -> Result<(), String> {
    match engine::presence(kind, name, said, engine::Asked::Behind) {
        engine::Presence::Gone => Ok(()),
        engine::Presence::There => {
            println!(
                "left {name} alone — {} would not remove it ({})",
                engine::NAME,
                refused.trim()
            );
            Ok(())
        }
        engine::Presence::Unknown(why) => Err(format!(
            "did not confirm {name} removed — {} would not remove it ({}), and asking whether \
             it is still there failed ({why})",
            engine::NAME,
            refused.trim()
        )),
    }
}

/// The `repository:tag` of every image of [`IMAGE`]'s, one a line, off a
/// `--format json` listing ([`engine::field`]).
fn image_tags(listing: &str) -> String {
    listing
        .lines()
        .filter_map(|line| {
            let repository = engine::field(line, "Repository")?;
            let tag = engine::field(line, "Tag")?;
            (repository == IMAGE).then(|| format!("{repository}:{tag}\n"))
        })
        .collect()
}

/// What the checkouts on this machine still name: the volumes their
/// mounts are spelled with ([`volume`]) and the image tags their builds
/// answer to ([`image_tag`]).
///
/// Every tree `git worktree list` shows, read when used: the set changes
/// while the machine runs, so no roster of seats is written down.
struct Alive {
    /// [`checkout_name`] of every tree.
    names: BTreeSet<String>,
    /// Every stage's tag for every tree, or `None` when one tree would
    /// not say — and then every image is kept: the hole may be the tag a
    /// tree still needs. Volumes are unaffected: a tree that will not open
    /// still has a name.
    tags: Option<BTreeSet<String>>,
}

/// The names of the trees a `git worktree list --porcelain` listing
/// holds. Pure, so a test can hold the naming.
fn checkout_names(trees: &[crate::seats::WorktreeBlock]) -> BTreeSet<String> {
    trees
        .iter()
        .map(|tree| checkout_name(Path::new(&tree.path)))
        .collect()
}

/// Every stage's tag for each of `trees`, each tree asked by the formula
/// its own runner has ([`tag_named_by`]), beside the trees that would not
/// say and why. Apart from [`alive`], so a test holds that a tree from
/// before the pin file is asked the former way.
fn tags_named_by_all<'a>(trees: &[&'a Path]) -> (BTreeSet<String>, Vec<(&'a Path, String)>) {
    let mut known = BTreeSet::new();
    let mut silent = Vec::new();
    for tree in trees {
        for stage in STAGES {
            match tag_named_by(tree, stage) {
                Ok(tag) => {
                    known.insert(tag);
                }
                Err(trouble) => {
                    silent.push((*tree, trouble));
                    break;
                }
            }
        }
    }
    (known, silent)
}

fn alive(root: &Path) -> Result<Alive, String> {
    let listing = crate::subprocess::git_query(
        &crate::seats::slashed(root),
        &["worktree", "list", "--porcelain"],
    )
    .ok_or("git worktree list failed — is git on PATH and this a repository?")?;
    let trees = crate::seats::worktree_blocks(&listing);
    if trees.is_empty() {
        return Err("git worktree list named no tree at all".into());
    }
    let roots: Vec<&Path> = trees.iter().map(|tree| Path::new(&tree.path)).collect();
    let (known, silent) = tags_named_by_all(&roots);
    for (tree, trouble) in &silent {
        println!(
            "left every {IMAGE} image alone — {} would not say which it needs ({trouble})",
            tree.display()
        );
    }
    Ok(Alive {
        names: checkout_names(&trees),
        tags: silent.is_empty().then_some(known),
    })
}

/// The volumes on this machine that no checkout names any more (pure, as
/// [`stale_images`] is). A name is the directory's last segment only
/// ([`checkout_name`]): a same-named clone elsewhere would see these
/// volumes as orphans (P3-確認事項).
fn orphan_volumes<'a>(listed: &'a str, names: &BTreeSet<String>) -> Vec<&'a str> {
    listed
        .lines()
        .map(str::trim)
        .filter(|volume| {
            VOLUME_KINDS.iter().any(|kind| {
                volume
                    .strip_prefix(&format!("{IMAGE}-{kind}-"))
                    .is_some_and(|whose| !names.contains(whose))
            })
        })
        .collect()
}

/// Takes away what no checkout on this machine names any more (gone
/// checkouts' volumes, and the images of the generations they were last
/// to need), once per process, ahead of a container mounting this
/// checkout's own.
///
/// Here rather than in [`ensure_image`]: [`sweep_the_volume`] reaches a
/// container without ensuring an image. Not on a Linux host nor inside
/// the container, where the names are the outside machine's.
///
/// A volume already gone was swept by another seat; one still mounted
/// the engine refuses, and that is printed.
///
/// **An engine that leaves any of it unanswered stops the line**: what
/// that command left is not known, the next removal would be put to an
/// engine in the same state, and so would the container this stands
/// ahead of — which nothing ends when the engine stands silent on it.
/// The build cache is left untrimmed on that road ([`TrimTheCache`]).
fn housekeeping(root: &Path) -> Result<(), String> {
    static KEPT: std::sync::OnceLock<Result<(), String>> = std::sync::OnceLock::new();
    if cfg!(target_os = "linux") || std::env::var_os(IN_CONTAINER).is_some() {
        return Ok(());
    }
    KEPT.get_or_init(|| {
        let alive = match alive(root) {
            Ok(alive) => alive,
            Err(trouble) => {
                println!("{trouble} — this run takes no image and no volume away");
                return Ok(());
            }
        };
        // This process's own: two lines of one tree can be at it at once.
        let said = root
            .join("target")
            .join(format!("linux-housekeeping-{}.txt", std::process::id()));
        let forgotten = forget(&alive, &said);
        let _ = std::fs::remove_file(&said);
        forgotten.map_err(|why| {
            SHARING_MOVED.store(false, Ordering::Relaxed);
            format!("started no container: the engine left the housekeeping ahead of it unanswered — {why}")
        })
    })
    .clone()
}

/// [`housekeeping`]'s removals: the orphan volumes, then the images no
/// checkout names — nothing off a half-read list (`Alive::tags`), and
/// `alive` has said which checkout would not answer. It ends at the
/// first thing the engine gives no answer to.
///
/// The listings are asked as a line's first question
/// ([`engine::Asked::First`]): the wait for the machine's ticket stands
/// between the image's question and these, long enough for the engine to
/// have taken its VM down.
fn forget(alive: &Alive, said: &Path) -> Result<(), String> {
    forget_orphan_volumes(&alive.names, said)?;
    let Some(keep) = &alive.tags else {
        return Ok(());
    };
    let mut list = engine::command();
    list.args([
        "images",
        "--filter",
        &format!("reference={IMAGE}"),
        "--format",
        "json",
    ]);
    let listed = image_tags(&engine::listed(list, engine::Asked::First, said)?);
    for stage in STAGES {
        forget_older_images(&listed, stage, keep, said)?;
    }
    Ok(())
}

fn forget_orphan_volumes(names: &BTreeSet<String>, said: &Path) -> Result<(), String> {
    let mut list = engine::command();
    list.args(["volume", "ls", "--quiet"]);
    let listed = engine::listed(list, engine::Asked::First, said)?;
    for orphan in orphan_volumes(&listed, names) {
        match engine::remove_volume(orphan, said) {
            engine::Removal::Done => {
                println!("removed the orphan volume {orphan} — no checkout here names it");
            }
            engine::Removal::Refused(refused) => {
                after_a_failed_removal(engine::Kind::Volume, orphan, &refused, said)?;
            }
            engine::Removal::Unanswered(why) => return Err(why),
        }
    }
    Ok(())
}

/// What the build cache may hold of what no image holds. buildkit never
/// counts or prunes a record an image shares (code-costs-windows-x64.md
/// §コンテナのイメージと build cache), so this is not what keeps a checkout
/// that is behind from building cold — the keep set of
/// [`forget_older_images`] is.
///
/// Every stage is tagged, so unshared cache is dead generations and
/// context snapshots: hoard none. Not zero, which reads as no ceiling at
/// all. A size, not an age: what is bounded is disk.
const CACHE_CEILING: u64 = 64 * 1024 * 1024;

/// Set when this line moved what the build cache is shared with: it
/// built an image ([`build_image`]) or took one away
/// ([`forget_older_images`]). Removing is the one that usually fires: it
/// leaves a generation's records nobody's, on a line that built nothing.
static SHARING_MOVED: AtomicBool = AtomicBool::new(false);

/// The build cache, cut back to [`CACHE_CEILING`] when the line that
/// moved the sharing ([`SHARING_MOVED`]) is done.
///
/// Not between the stages of one line: `bare` builds an image out of the
/// previous one's cache, so a trim in [`ensure_image`] would take what the
/// next stage asks for. A guard, so it runs on every road out; declared
/// under the ticket, so its engine command is counted. Held by [`run`] and by
/// [`sweep_the_volume`], whose housekeeping can move the sharing too.
struct TrimTheCache;

impl Drop for TrimTheCache {
    fn drop(&mut self) {
        if !SHARING_MOVED.load(Ordering::Relaxed) {
            return;
        }
        // This process's own, as [`built`]'s is.
        let said = crate::tree::workspace_root()
            .join("target")
            .join(format!("linux-trim-{}.txt", std::process::id()));
        let trimmed = engine::trim_build_cache(CACHE_CEILING, &said);
        let _ = std::fs::remove_file(&said);
        match trimmed {
            Ok(freed) => println!(
                "trimmed the unreferenced build cache to {} MiB — freed {freed}",
                CACHE_CEILING >> 20
            ),
            Err(said) => println!(
                "left the build cache alone — {} would not trim it ({said})",
                engine::NAME
            ),
        }
    }
}

/// `copy` is the name of the prepared task runner the line starts from
/// ([`runner`]).
///
/// No container started here is named or reaped: an interrupted one
/// cannot reach this checkout's copies (`runner::SCRIPT`), and the
/// volume's cargo lock it holds the next build waits out under the step's
/// own ceiling (`check::run_step`).
fn in_container(
    root: &Path,
    tag: &str,
    command: &[String],
    shell: bool,
    copy: Option<&str>,
    note: Option<&Path>,
) -> Result<(), String> {
    // Ahead of the mounts below: every host-side container that names a
    // checkout's volumes starts here.
    housekeeping(root)?;
    let mut cmd = carried();
    // The tree's gate note, read-only, for the one line that has to
    // read it late (`runner::note_of`). It cannot come in through
    // `/work`: the build volume covers the `target` it stands in.
    if let Some(note) = note {
        cmd.arg("--volume")
            .arg(format!("{}:{}:ro", mount_path(note), runner::NOTE_MOUNT));
    }
    // A terminal only when there is one to attach: the engine refuses
    // --tty outright when the harness runs this with a pipe for stdin.
    if std::io::stdin().is_terminal() {
        cmd.arg("--interactive")
            .arg("--tty")
            .stdin(Stdio::inherit());
    }
    cmd.arg("--volume")
        .arg(format!("{}:{WORK}", mount_path(root)))
        .arg("--volume")
        .arg(format!("{}:{TARGET_MOUNT}", volume(root, "target")))
        .arg("--volume")
        .arg(format!("{IMAGE}-registry:{REGISTRY_MOUNT}"))
        .arg("--volume")
        .arg(format!("{}:{DEMO_MOUNT}", volume(root, "demo")))
        .arg("--workdir")
        .arg(WORK)
        // The checkout is mounted, so what a run generates here would land
        // in the host's files: the mark keeps another machine's census out
        // (`verify::options::census_line`).
        .arg("--env")
        .arg(format!("{IN_CONTAINER}=1"));
    given_commit(&mut cmd, root);

    let mut inside = command.to_vec();
    // Kept in scope past the run below: the directory behind `/out` is
    // this run's for as long as this value lives.
    let keepsake = keepsakes::bridge(&mut inside, keepsakes::Landing::Whole(OUT_MOUNT))?;
    if let Some(out) = &keepsake {
        cmd.arg("--volume")
            .arg(format!("{}:{OUT_MOUNT}", mount_path(&out.dir)));
    }

    cmd.arg(tag);
    if shell {
        cmd.arg("bash");
    } else {
        cmd.args(watched_from_inside(
            &inside,
            copy.map(|name| runner::at(root, name)).as_deref(),
        ));
    }
    // Through the budget's runner (`crate::budget::watched`): the
    // container goes on running when the launcher is killed. It sees the
    // engine's CLI only; a container whose CLI was killed too is the
    // engine's `ps` to find.
    let status = crate::budget::watched(&mut cmd)
        .map_err(|e| format!("failed to run {}: {e}", engine::NAME))?;
    if status.success() {
        // The line as typed: the one `bridge` wrote says
        // `--no-board` whether or not the caller did.
        keepsakes::onto_the_board(keepsake.as_ref().map(|out| out.dir.as_path()), command);
        return Ok(());
    }
    Err(match status.code() {
        Some(code) => format!("the run in {tag} exited {code}"),
        None => format!("the run in {tag} was killed"),
    })
}

/// The files whose bytes decide what a cargo in there resolves, as the
/// mount hands them over: the three at the root and every member's own
/// manifest (a glob, so a new member counts). The host's side of the same
/// evidence reads the same set (`gate::evidence::read_to_resolve`).
const READ_TO_RESOLVE: [&str; 4] = [
    "Cargo.lock",
    "Cargo.toml",
    ".cargo/config.toml",
    "crates/*/Cargo.toml",
];

/// The command with a look at those files bracketed around it. A
/// container is `--rm`, so nothing survives it that it did not say while
/// it ran: a later look sees a mount that has since settled. Both looks
/// are printed only if the command fails.
///
/// `bytes=0` on the near side of a failure says this container was handed
/// an empty file. Both ends intact says nothing about the read cargo
/// itself made in between.
///
/// A line that starts from a prepared task runner ([`runner`]) gets a
/// guard instead: a look would start a `cargo --version` on every red
/// step, the very thing the copy exists to stop. The guard names the
/// copy's one failure — not prepared — where `sh` would say `not found`.
/// The line the command was is printed either way.
fn watched_from_inside(inside: &[String], copy: Option<&str>) -> Vec<String> {
    // The line as the container was handed it, on either road: what a red
    // step leaves for a reader who was not standing there.
    let ran = "printf 'pgg-probe ran'; for w in \"$@\"; do printf ' [%s]' \"$w\"; done; echo\n";
    let script = match copy {
        Some(copy) => format!(
            "if [ ! -x {copy} ]; then\n  echo \"pgg-runner {copy} is not here — this run's \
             task runner was not prepared, and nothing here falls back to cargo\"\n  \
             exit 127\nfi\n\"$@\"\ncode=$?\n\
             if [ \"$code\" -ne 0 ]; then\n  {ran}fi\nexit \"$code\"\n"
        ),
        // Unquoted so the glob is the shell's to expand; a pattern that
        // matches nothing stays as typed and is reported absent.
        // `${f#/work/}`: the name as the host's side of the evidence
        // spells it.
        None => {
            let looks = READ_TO_RESOLVE
                .map(|file| format!("{WORK}/{file}"))
                .join(" ");
            format!(
                "look() {{\n  for f in {looks}; do\n    \
                 if [ -f \"$f\" ]; then echo \"pgg-probe $1 ${{f#{WORK}/}} \
                 bytes=$(wc -c < \"$f\") sha=$(sha256sum \"$f\" | cut -c1-16)\"; \
                 else echo \"pgg-probe $1 ${{f#{WORK}/}} absent\"; fi\n  done\n}}\n\
                 before=$(look before)\n\"$@\"\ncode=$?\n\
                 if [ \"$code\" -ne 0 ]; then\n  echo \"$before\"\n  look after\n  \
                 echo \"pgg-probe cargo $(cargo --version 2>&1)\"\n  {ran}\
                 fi\nexit \"$code\"\n"
            )
        }
    };
    let mut line = vec![
        "sh".to_string(),
        "-c".to_string(),
        script,
        IMAGE.to_string(),
    ];
    line.extend(inside.iter().cloned());
    line
}

/// An engine's `run` already carrying this command's announcement to a
/// measurement and its ticket, neither of which the container may take
/// again (`still::UNDER`, `budget::HELD`).
///
/// Every container starts here: a road that forgot a mark would count the
/// machine twice — the launcher holding weight while what it started
/// queues for its own, a pair where neither half can move. `here` marks
/// its child through `budget::under`, and an `exec` into the gate's
/// container takes [`marked`] (`container::exec_line`).
///
/// No stdin: nothing in there reads one it was not given a terminal for
/// (`in_container` hands the terminal over).
pub(super) fn carried() -> Command {
    let mut cmd = engine::command();
    cmd.arg("run").arg("--rm").stdin(Stdio::null());
    marked(&mut cmd);
    cmd
}

/// The two marks, on a `run` or an `exec` — one place, so
/// that no road can carry one and forget the other.
pub(super) fn marked(cmd: &mut Command) {
    for mark in [crate::still::UNDER, crate::budget::HELD] {
        cmd.arg("--env").arg(format!("{mark}=1"));
    }
}

/// The commit the mounted checkout stands at and the tags on it, for the
/// app's build.rs to name the build by: inside, a worktree's `.git` names
/// a host path nothing mounts, so git there cannot answer. Read on every
/// start, never kept — a container lives for one commit at most.
pub(super) fn given_commit(cmd: &mut Command, root: &Path) {
    let dir = root.display().to_string();
    let head = crate::subprocess::git_query(&dir, &["rev-parse", "--verify", "-q", "HEAD"]);
    let tags = crate::subprocess::git_query(&dir, &["tag", "--points-at", "HEAD"]);
    cmd.arg("--env")
        .arg(format!(
            "PLATITUDE_GIVEN_COMMIT={}",
            head.unwrap_or_default()
        ))
        .arg("--env")
        .arg(format!(
            "PLATITUDE_GIVEN_TAGS={}",
            tags.unwrap_or_default()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        ));
}

fn here(root: &Path, command: &[String]) -> Result<(), String> {
    let (program, arguments) = command.split_first().ok_or("nothing to run")?;
    let mut cmd = Command::new(program);
    cmd.args(arguments).current_dir(root);
    // Under this command's ticket, as the container's contents are
    // ([`carried`] says why).
    crate::budget::under(&mut cmd);
    let status =
        crate::budget::watched(&mut cmd).map_err(|e| format!("failed to run {program}: {e}"))?;
    if status.success() {
        return Ok(());
    }
    // The child's own code, the way the container path reports it — a
    // runner carries a child's exit through.
    Err(match status.code() {
        Some(code) => format!("the command exited {code}"),
        None => "the command was killed".into(),
    })
}

/// A host path as the engine wants it in --volume: forward slashes, drive
/// letter and all (backslashes do not mount).
fn mount_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

/// One volume per checkout, named after it: two worktrees sharing a build
/// directory would share one build lock.
pub(super) fn volume(root: &Path, kind: &str) -> String {
    format!("{IMAGE}-{kind}-{}", checkout_name(root))
}

/// The checkout's own name, as the engine may spell it: the volumes and the
/// gate's container are named after it (`container::container_of`).
pub(super) fn checkout_name(root: &Path) -> String {
    // The last segment after either separator: the path is a Windows one
    // when the host is, and off Windows (where the tests also run) a
    // backslash is an ordinary character, so `file_name` would answer
    // with the whole path.
    let text = root.display().to_string();
    let name = text
        .rsplit(['/', '\\'])
        .find(|segment| !segment.is_empty())
        .unwrap_or("root");
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-') {
                c
            } else {
                '-'
            }
        })
        .collect()
}
