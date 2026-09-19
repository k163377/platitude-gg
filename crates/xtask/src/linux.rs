//! `cargo xtask linux <command…>` — the workspace on Linux, from a
//! workstation that has none.
//!
//! Everything after the options goes to cargo inside a container built from
//! ci/linux/Dockerfile, so `cargo xtask linux test -p platitude-core` is
//! `cargo test -p platitude-core` on Ubuntu. Naming an xtask verb puts
//! `cargo xtask` in front of it instead, so `cargo xtask linux verify-ui
//! commit` is the line a person already knows, run somewhere else. On Linux
//! the container drops out and the command runs where it stands: one verb,
//! three operating systems (CLAUDE.md: no Windows-only dev tooling).
//!
//! Four images. Two are chosen by what the command needs: the core stage
//! is Ubuntu and the toolchain, the app stage adds Qt, a software GL stack
//! and the fonts デザイン規約 names for Ubuntu — asking for the small one
//! when it will do is the difference between a run that starts now and one
//! that downloads Qt first. The other two belong to the `bare` verb and
//! are the opposite of a build environment: stock Ubuntu carrying only
//! the Qt tree a distribution would ship beside the app. `bare` installs
//! no package at all — the blank sheet `--discover` works the declaration
//! out on — and `runtime` adds exactly the packages that came out, which
//! is the only place that can say the built thing runs somewhere it was
//! not built.
//!
//! The build directory is a docker volume mounted over /work/target, never
//! the host's. One target/ shared between two operating systems is two
//! cargos on one build lock and two sets of fingerprints for the same paths
//! — the serialized-and-rebuilding failure worktrees exist to avoid, one
//! boundary further out. The cargo registry is a volume for the same
//! reason, and the tests build their repositories under the container's own
//! /tmp, so the only thing crossing the host filesystem is reading source.
//!
//! Reading source across the host boundary is measurably slow, but cargo
//! pays it once per build to check fingerprints, against a compile
//! measured in seconds. Keeping a second copy of the tree inside a volume
//! would buy that back and cost a second answer to "which tree is the
//! real one", so the source stays where it is edited.
//!
//! One thing does not survive the boundary: a worktree's `.git` is a file
//! naming an absolute Windows path, which git inside reads as relative and
//! cannot follow, so any git run with /work as its working directory calls
//! it a broken repository. Nothing a run depends
//! on stands there — `verify-ui` starts the app outside every checkout,
//! on a git configuration of its own (`verify::run`).

use std::io::IsTerminal;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::command::{self, Permission, Where};
use crate::keepsakes;

pub(crate) static RUN: command::Command = command::Command {
    id: "linux.run",
    call: "linux <command>",
    purpose: "run a command against this checkout on Ubuntu, in the container",
    run_in: Where::Seat,
    needs: &["docker running (on Linux the command runs where it stands)"],
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

pub(crate) static BARE: command::Command = command::Command {
    id: "linux.bare",
    call: "linux bare --discover",
    purpose: "whether the app runs on an Ubuntu carrying only the declared dependencies",
    run_in: Where::Seat,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static COMMANDS: &[&command::Command] = &[&RUN, &VERIFY, &RUNNER, &BARE];

mod bare;
mod offline;
pub(crate) mod runner;
#[cfg(test)]
mod tests;

pub(crate) use runner::a_runner_verb;

/// The image. Its tag names the stage and fingerprints what built it.
const IMAGE: &str = "pgg-linux";

/// Where the checkout, the build directory, the download cache and anything
/// a run means to leave behind land inside the container.
const WORK: &str = "/work";
const TARGET_MOUNT: &str = "/work/target";
const REGISTRY_MOUNT: &str = "/usr/local/cargo/registry";
const OUT_MOUNT: &str = "/out";
/// Where the demo repositories a run builds itself go, and — the reason
/// it is a volume — where the template each of them is copied from
/// stands (`demo::template`).
///
/// A container is one verb, so nothing built in it outlives it: without
/// this, every run in here would build its template and then throw it
/// away. One volume per checkout, as the build directory is, because a
/// template is only good for the task runner that built it and two
/// checkouts build their own.
const DEMO_MOUNT: &str = "/tmp/pgg-demo";
/// Set for everything the container runs, and by nothing else: the mark a
/// run reads to know it is not on the machine whose checkout it is
/// writing.
pub(crate) const IN_CONTAINER: &str = "PGG_IN_CONTAINER";

/// Task-runner verbs worth running in there. Naming one means `cargo xtask
/// <verb>`, so the command reads the same as on the host.
const XTASK_VERBS: [&str; 3] = ["verify-ui", "demo-repo", "qmltest"];

/// Cargo verbs that build something, and so care which stage they run
/// in. A list of what needs Qt can be short without being unsafe: a verb
/// missing from it picks the smaller image and fails to build, in front
/// of the person who typed it — which is why the lock flag is decided
/// the other way round ([`UNLOCKED_VERBS`]). Cargo's own one-letter
/// aliases are here because `linux t` reaches the app exactly as
/// `linux test` does.
const BUILD_VERBS: [&str; 11] = [
    "build", "check", "test", "clippy", "bench", "run", "doc", "b", "c", "t", "r",
];

/// The cargo verbs that are handed to the container **without**
/// `--locked`. Everything else gets it.
///
/// **The default is locked, and the list is of the exceptions**, because
/// the two ways of being wrong are not the same size. The tree at /work
/// is the host's own checkout, so a cargo in there that rewrites the
/// lock writes it on the machine outside — a verb this forgot would do
/// that silently, which is the thing there must be no path to (CLAUDE.md
/// 絶対制約: a dependency change is a human's decision). A verb wrongly
/// given the flag says so and stops, in front of the person who typed
/// it. A list of the verbs that resolve is a list to be caught short by:
/// it was, twice — `metadata`, `tree` and then `fetch`, which generates
/// a lock file of its own if none is there.
///
/// So: `fmt`, which resolves nothing (`cargo-fmt` asks for `--no-deps`
/// metadata and would reject the flag), and the verbs whose whole
/// purpose is to move the lock or the manifest, which `--locked` exists
/// to refuse. A third-party subcommand that does not take the flag is
/// the loud kind of wrong: run it through `--shell`, or add it here if
/// it is one this project uses.
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

/// The packages that hold no Qt. A build restricted to these needs no Qt
/// either; anything else reaches platitude-app, including a bare `cargo
/// test`, whose default members have the app in them.
const QT_FREE: [&str; 2] = ["platitude-core", "xtask"];

/// The leading options of a `linux` line, and where the command begins.
struct Options {
    rebuild: bool,
    shell: bool,
    stage: Option<String>,
    /// `--runner`: the name of a prepared copy to start the verb from
    /// ([`runner`]).
    copy: Option<String>,
    at: usize,
}

/// **Options are the leading tokens only**: everything from the first one
/// that is not ours belongs to the command, `--` and all.
fn options(args: &[String]) -> Result<Options, String> {
    let mut opts = Options {
        rebuild: false,
        shell: false,
        stage: None,
        copy: None,
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
    Ok(opts)
}

pub fn run(args: &[String]) -> Result<(), String> {
    let Options {
        rebuild,
        shell,
        stage: forced_stage,
        copy,
        at,
    } = options(args)?;
    let rest = &args[at..];
    let root = crate::tree::workspace_root();
    // A prepared copy starts the task runner's own verbs and nothing
    // else: a cargo command in there is cargo's work, and a line that
    // silently dropped the flag would be a line nobody could read the
    // road of afterwards (`runner`).
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
    // The container's work is this machine's work — the image built as
    // much as the command run in it — and it is counted here
    // (`crate::budget`): a VM's worth of cargo is not less of this
    // machine for being behind a mount. Ahead of every road out of this
    // verb, because all four of them are that work: the container, the
    // two that stay in a container on Linux, and the one that runs the
    // command where it stands. Everything inside the container is under
    // this ticket ([`in_container`] hands the mark across, as it hands
    // the measurement's).
    let _room = crate::budget::standalone(
        &root,
        crate::budget::weight_of(&command, false),
        crate::budget::Rank::Normal,
        &format!("linux {}", rest.join(" ")),
    )?;
    // The verb that prepares what the others start from. Before the two
    // below it for the same reason they are before the road out: it is
    // this side's work whichever machine the host is, and on Linux it
    // runs where it stands like everything else (`runner::prepare`).
    if rest.first().is_some_and(|verb| verb == "runner") {
        // `--gate` is not a nicety: a preparation builds in this
        // checkout's volume and stands at its one preparation
        // container, so the line has to name the gate it is under and
        // that gate has to be the live one
        // (`runner::owned_by_the_gate`). Nothing types this by hand.
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
        return runner::prepare(&root, name, pid);
    }
    // The one verb that is about a different machine: it runs in the
    // container even on Linux, because what it asks is whether a stock
    // Ubuntu is enough.
    if rest.first().is_some_and(|verb| verb == "bare") {
        let discover = rest.iter().any(|word| word == "--discover");
        return bare::bare(&root, discover);
    }
    // The other verb that is about a machine, and the other one that
    // stays in the container on Linux: what it asks is whether the
    // suite passes with no network, which the host has.
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
        println!("already on Linux — running here, no container");
        return here(&root, &command);
    }

    let stage = match &forced_stage {
        Some(name) => name.clone(),
        None => stage_for(rest).to_string(),
    };
    // Announced here as well (its own xtask is under the announcement);
    // a verb run where it stands announces itself.
    let _busy = crate::still::busy(&root, "linux")?;
    let tag = ensure_image(&root, &stage, rebuild)?;
    in_container(&root, &tag, &command, shell, copy.as_deref(), None)
}

fn ensure_image(root: &Path, stage: &str, rebuild: bool) -> Result<String, String> {
    let tag = image_tag(root, stage)?;
    if rebuild || !image_exists(&tag)? {
        build_image(root, stage, &tag)?;
        forget_older_images(stage, &tag);
    }
    Ok(tag)
}

/// Cargo's own options that take their value in the next word. **A
/// subcommand is not simply the first word that is not an option**:
/// cargo takes its own options ahead of one (`cargo --offline fetch`),
/// and the value of one of these is a word that starts with no dash —
/// `cargo --config net.retry=2 test` would hand `net.retry=2` out as the
/// subcommand to anything that only looked for that. The `--flag=value`
/// spelling is one word and needs none of this.
const GLOBAL_OPTIONS_WITH_A_VALUE: [&str; 5] = ["--explain", "--color", "--config", "-C", "-Z"];

/// Where the subcommand stands in the line, past cargo's own options, or
/// `None` for a line that has none (`--version`, `--list`).
///
/// An option this does not know is read as a flag, so its value — if it
/// had one — is taken for the subcommand and `--locked` lands after it.
/// That is the loud kind of wrong: cargo answers "no such subcommand" in
/// front of the person who typed it, where a line that quietly went
/// unlocked would have rewritten the host's lock instead
/// ([`UNLOCKED_VERBS`]).
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

/// What to run inside: a cargo command, with `xtask` folded in when the
/// subcommand is one of the task runner's own verbs, and `--locked`
/// spelled on unless that subcommand is one of
/// [`UNLOCKED_VERBS`].
///
/// A task-runner verb needs neither: `cargo xtask` is an alias that
/// carries `--locked` already (.cargo/config.toml), and a second one
/// would be a second place to forget it.
///
/// `copy` is a prepared task runner ([`runner`]), and a line that has one
/// starts from it: no cargo, no resolve, no manifest read across the
/// mount. Only a line whose verb is the task runner's own ever carries
/// one, which `run` refuses anything else at.
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
        // Where the subcommand stands, which is not always the front:
        // `--offline verify-ui commit` is cargo's option and then ours.
        line.insert(at + 1, "xtask".to_string());
        return line;
    }
    // Only what the caller typed as options: past `--` the words belong
    // to the program being run, and one of those spelling `--locked` is
    // not this line carrying it. `--frozen` is `--locked` and
    // `--offline` in one word, so a line that has it is already locked.
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
    // The subcommand, for the reason [`subcommand_at`] gives:
    // `--offline test` is cargo's option and then the verb that reaches
    // the app.
    let Some(verb) = subcommand_at(rest).map(|at| rest[at].as_str()) else {
        // A bare `--shell`, or a line that is all options. The small
        // image opens now; --stage app asks for the other one.
        return "core";
    };
    if XTASK_VERBS.contains(&verb) {
        // verify-ui builds the app and runs it, and qmltest wants
        // qmltestrunner and the QtTest QML module, which ship with Qt;
        // demo-repo only wants git, but it is not worth a second answer.
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

/// The image is named after what builds it: change the Dockerfile, the
/// toolchain pin or (for the app stage) the Qt version and the tag changes
/// with it, so a stale image can never be the one that answers. Docker's
/// layer cache keeps the rebuild cheap. FNV-1a over the files — a
/// fingerprint, and the tree is LF everywhere (.gitattributes) so it
/// comes out the same on all three systems.
fn image_tag(root: &Path, stage: &str) -> Result<String, String> {
    let mut inputs = vec!["ci/linux/Dockerfile", "rust-toolchain.toml"];
    if stage != "core" {
        // Every stage past core is built with the Qt version CI pins
        // (runtime inherits it through bare), so the tag has to move when
        // that does.
        inputs.push(".github/workflows/ci.yml");
    }
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
    Ok(format!("{IMAGE}:{stage}-{hash:016x}"))
}

/// The Qt version, read from CI's workflow — which names itself this
/// project's place to pin a toolchain. A copy in the Dockerfile would be a
/// second place to forget.
fn qt_version(root: &Path) -> Result<String, String> {
    let path = root.join(".github").join("workflows").join("ci.yml");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    text.lines()
        .filter_map(|line| line.trim().strip_prefix("QT_VERSION:"))
        .map(|value| value.trim().trim_matches('"').trim_matches('\''))
        .find(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("no QT_VERSION in {}", path.display()))
}

fn image_exists(tag: &str) -> Result<bool, String> {
    let status = Command::new("docker")
        .args(["image", "inspect", tag])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| {
            format!(
                "failed to run docker: {e}. `cargo xtask linux` needs docker on \
                 PATH — it is the entire Linux side of a Windows workstation."
            )
        })?;
    Ok(status.success())
}

fn build_image(root: &Path, stage: &str, tag: &str) -> Result<(), String> {
    println!("building {tag} — the first one takes a while");
    let mut cmd = Command::new("docker");
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
        .map_err(|e| format!("failed to run docker build: {e}"))?;
    if !status.success() {
        return Err("docker build failed".into());
    }
    Ok(())
}

/// Removes the older images of this stage once a new one has built. The
/// tag is a fingerprint, so every edit to the Dockerfile leaves the last
/// image behind — gigabytes a stage, the app one the larger of the two —
/// and nothing else would ever name them again.
///
/// Two things keep this from taking something out from under anybody.
/// Docker refuses to remove an image a container is still running, so a
/// run in progress next door is safe by construction; and what a rebuild
/// costs after this is the layer cache, because the layers stay. Every
/// removal is printed: a command that quietly frees gigabytes is one
/// nobody can audit.
fn forget_older_images(stage: &str, keep: &str) {
    let prefix = format!("{IMAGE}:{stage}-");
    let Ok(out) = Command::new("docker")
        .args(["images", IMAGE, "--format", "{{.Repository}}:{{.Tag}}"])
        .stderr(Stdio::null())
        .output()
    else {
        return;
    };
    let listed = String::from_utf8_lossy(&out.stdout);
    let stale: Vec<&str> = listed
        .lines()
        .map(str::trim)
        .filter(|tag| tag.starts_with(&prefix) && *tag != keep)
        .collect();
    for tag in stale {
        let removed = Command::new("docker")
            .args(["image", "rm", tag])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
        if removed {
            println!("removed the older image {tag}");
        } else {
            println!("left {tag} alone — docker would not remove it (in use?)");
        }
    }
}

/// `copy` is the name of the prepared task runner the line starts from
/// ([`runner`]), which the script in there says is missing rather than
/// letting `sh` answer for it.
///
/// **No container started here is named, and none is reaped.** A name
/// would be for finding an interrupted one to take away, and taking one
/// away is a `docker rm` on this side with nothing watching how long it
/// takes. Neither is needed: an interrupted container cannot reach this
/// checkout's copies at all (`runner::SCRIPT`), and the one thing it
/// does hold — the volume's cargo lock — the next build waits out under
/// the step's own ceiling (`check::run_step`).
fn in_container(
    root: &Path,
    tag: &str,
    command: &[String],
    shell: bool,
    copy: Option<&str>,
    note: Option<&Path>,
) -> Result<(), String> {
    let mut cmd = carried();
    // The tree's gate note, read-only, for the one line that has to
    // read it late (`runner::note_of`). It cannot come in through
    // `/work`: the build volume covers the `target` it stands in.
    if let Some(note) = note {
        cmd.arg("--volume")
            .arg(format!("{}:{}:ro", mount_path(note), runner::NOTE_MOUNT));
    }
    // A terminal only when there is one to attach: docker refuses --tty
    // outright when the harness runs this with a pipe for stdin.
    if std::io::stdin().is_terminal() {
        cmd.arg("--interactive").arg("--tty");
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
        // The checkout is mounted, so anything in here writes the host's
        // own files. What a run generates says which machine ran it,
        // and the container is never that machine
        // (`verify::options::census_line`).
        .arg("--env")
        .arg(format!("{IN_CONTAINER}=1"));

    let mut inside = command.to_vec();
    // Kept in scope past the run below: the directory behind `/out` is
    // this run's for as long as this value lives.
    let keepsake = keepsakes::bridge(&mut inside, OUT_MOUNT)?;
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
    // Through the budget's runner: the container goes on running when
    // the launcher is killed, so the ledger has to know which number is
    // still holding the machine (`crate::budget::watched`). It knows the
    // docker command that is waiting on the container, which is as far
    // as a process table reaches — a container whose CLI has been killed
    // too is past what this can see, and `docker ps` is what finds it.
    let status =
        crate::budget::watched(&mut cmd).map_err(|e| format!("failed to run docker: {e}"))?;
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
/// mount hands them over: the three at the root and **every member's own
/// manifest**, which a resolve reads too. A glob, so a member
/// added to the workspace is one this reads — the host's side of
/// the same evidence reads the same set off the tree
/// (`gate::evidence::read_to_resolve`). All of them are the host's own
/// files, read across the boundary between a Windows checkout and a
/// Linux container, which is the one thing about /work that the machine
/// outside cannot see.
const READ_TO_RESOLVE: [&str; 4] = [
    "Cargo.lock",
    "Cargo.toml",
    ".cargo/config.toml",
    "crates/*/Cargo.toml",
];

/// The command with a look at those files bracketed around it.
///
/// **A container is `--rm`, so nothing survives it that it did not say
/// while it ran.** A cargo that stops on the lock file leaves a message
/// naming the file and nothing about the bytes it read, and every look
/// taken afterwards — from out here, or from a second container — is a
/// look at a different moment through a mount that has since settled.
/// The look before the command runs is held in a variable and printed
/// only if the command fails, so a green run says nothing new and a red
/// one carries both ends of the bracket.
///
/// **What it can and cannot settle**: `bytes=0` on the near side of a
/// failure says this container was handed an empty file. Both ends
/// intact says only that nothing was standing wrong before the command
/// and after it — **the read cargo itself made is in between, and is not
/// bracketed**, so neither end is evidence of what cargo read.
///
/// **A line that starts from a prepared task runner gets a guard instead
/// of the bracket** ([`runner`]). Two reasons, and the second is the one
/// that matters: a line with no cargo in it resolves nothing, so a look
/// at what a resolve reads could only say something about a read nobody
/// made — and taking it would start a `cargo --version` of its own on
/// every red step, which is the very thing the copy exists to stop. What
/// the guard answers instead is the one failure a copy has: it is not
/// there, because the preparation did not happen. It says so by name and
/// stops, where `sh` would say `not found` and a number. The line the
/// command was is printed either way.
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
        // matches nothing stays as it was typed, and the test below
        // reports it absent under its own name.
        // `${f#/work/}`: the name as the host's side of the evidence
        // spells it, so the two lines stand side by side.
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

/// A `docker run`, already carrying the marks everything a container
/// runs is under: this command's announcement to a measurement and this
/// command's ticket, neither of which the container may take again
/// (`still::UNDER`, `budget::HELD`).
///
/// **Every container starts here, because a road that carries one mark
/// and forgets the other is a road where the machine is counted twice**
/// — the launcher holding a compile's weight while what it started
/// queues for weight of its own, and a machine of such pairs where
/// neither half can move. Three roads run a container (the command,
/// `bare`, `offline`); the fourth (`here`, on a Linux host) marks its
/// child the same way through `budget::under`.
pub(super) fn carried() -> Command {
    let mut cmd = Command::new("docker");
    cmd.arg("run").arg("--rm");
    for mark in [crate::still::UNDER, crate::budget::HELD] {
        cmd.arg("--env").arg(format!("{mark}=1"));
    }
    cmd
}

fn here(root: &Path, command: &[String]) -> Result<(), String> {
    let (program, arguments) = command.split_first().ok_or("nothing to run")?;
    let mut cmd = Command::new(program);
    cmd.args(arguments).current_dir(root);
    // Under this command's ticket, exactly as the container's contents
    // are ([`carried`]). Without the mark a `linux verify-ui` here holds
    // four weight in this process while the verb it started queues for
    // its own — the machine counted twice, and a machine full of such
    // pairs where neither half can move.
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

/// A host path as docker wants it in --volume: forward slashes, drive letter
/// and all. `C:/Users/…` mounts, `C:\Users\…` is read as three arguments'
/// worth of colons and backslashes.
fn mount_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

/// One volume per checkout, named after it. Two worktrees sharing a build
/// directory would put back exactly what worktrees exist to prevent: one
/// lock, one incremental cache, two sessions.
pub(super) fn volume(root: &Path, kind: &str) -> String {
    // The last segment after either separator: the path being named is
    // a Windows one whenever the host is Windows, and everywhere else a
    // backslash is an ordinary character in a name, so `file_name` would
    // answer with the whole path.
    // Only the host ever calls this, so the difference is invisible in a
    // run and visible in a test — which is where it was found.
    let text = root.display().to_string();
    let name = text
        .rsplit(['/', '\\'])
        .find(|segment| !segment.is_empty())
        .unwrap_or("root");
    let name: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-') {
                c
            } else {
                '-'
            }
        })
        .collect();
    format!("{IMAGE}-{kind}-{name}")
}
