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
//! it a broken repository rather than no repository. Nothing a run depends
//! on stands there — `verify-ui` starts the app outside every checkout,
//! on a git configuration of its own (`verify::run`).

use std::io::IsTerminal;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::keepsakes;

mod bare;

/// The image. Its tag names the stage and fingerprints what built it.
const IMAGE: &str = "pg-linux";

/// Where the checkout, the build directory, the download cache and anything
/// a run means to leave behind land inside the container.
const WORK: &str = "/work";
const TARGET_MOUNT: &str = "/work/target";
const REGISTRY_MOUNT: &str = "/usr/local/cargo/registry";
const OUT_MOUNT: &str = "/out";

/// Task-runner verbs worth running in there. Naming one means `cargo xtask
/// <verb>`, so the command reads the same as on the host.
const XTASK_VERBS: [&str; 2] = ["verify-ui", "demo-repo"];

/// Cargo verbs that build something, and so care which stage they run in.
const BUILD_VERBS: [&str; 7] = ["build", "check", "test", "clippy", "bench", "run", "doc"];

/// The packages that hold no Qt. A build restricted to these needs no Qt
/// either; anything else reaches platitude-app, including a bare `cargo
/// test`, whose default members have the app in them.
const QT_FREE: [&str; 2] = ["platitude-core", "xtask"];

pub fn run(args: &[String]) -> Result<(), String> {
    let mut rebuild = false;
    let mut shell = false;
    let mut forced_stage: Option<String> = None;
    // Options are the leading tokens only: everything from the first one
    // that is not ours belongs to the command, `--` and all.
    let mut at = 0;
    while let Some(arg) = args.get(at) {
        match arg.as_str() {
            "--rebuild" => rebuild = true,
            "--shell" => shell = true,
            "--stage" => {
                at += 1;
                let name = args.get(at).ok_or("--stage needs core or app")?;
                if !matches!(name.as_str(), "core" | "app") {
                    return Err(format!("unknown stage {name:?}: core or app"));
                }
                forced_stage = Some(name.clone());
            }
            _ => break,
        }
        at += 1;
    }
    let rest = &args[at..];
    if !shell && rest.is_empty() {
        return Err(
            "linux needs a command, e.g. `cargo xtask linux test -p platitude-core`".into(),
        );
    }
    if shell && !rest.is_empty() {
        // Silently dropping the command would run bash where a check was
        // asked for.
        return Err(format!(
            "--shell takes no command (got {:?}) — drop --shell to run it, \
             or drop the command to get the shell",
            rest.join(" ")
        ));
    }

    let root = crate::workspace_root();
    // The one verb that is about a different machine rather than a
    // different toolchain: it runs in the container even on Linux, because
    // what it asks is whether a stock Ubuntu is enough.
    if rest.first().is_some_and(|verb| verb == "bare") {
        let discover = rest.iter().any(|word| word == "--discover");
        return bare::bare(&root, discover);
    }
    let command = command_line(rest);
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
    let tag = ensure_image(&root, &stage, rebuild)?;
    in_container(&root, &tag, &command, shell)
}

fn ensure_image(root: &Path, stage: &str, rebuild: bool) -> Result<String, String> {
    let tag = image_tag(root, stage)?;
    if rebuild || !image_exists(&tag)? {
        build_image(root, stage, &tag)?;
        forget_older_images(stage, &tag);
    }
    Ok(tag)
}

/// What to run inside: a cargo command, with `xtask` folded in when the
/// first word is one of the task runner's own verbs.
fn command_line(rest: &[String]) -> Vec<String> {
    let mut line = vec!["cargo".to_string()];
    if rest
        .first()
        .is_some_and(|verb| XTASK_VERBS.contains(&verb.as_str()))
    {
        line.push("xtask".to_string());
    }
    line.extend(rest.iter().cloned());
    line
}

/// Which image the command needs. Nothing here is a guess about Qt itself:
/// either the command names only Qt-free packages, or it can reach the app.
fn stage_for(rest: &[String]) -> &'static str {
    let Some(verb) = rest.first().map(String::as_str) else {
        // A bare `--shell`. The small image opens now; --stage app asks for
        // the other one.
        return "core";
    };
    if XTASK_VERBS.contains(&verb) {
        // verify-ui builds the app and runs it; demo-repo only wants git,
        // but it is not worth a second answer.
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
/// fingerprint, not a security claim, and the tree is LF everywhere
/// (.gitattributes) so it comes out the same on all three systems.
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
    let status = cmd
        .arg(root)
        .status()
        .map_err(|e| format!("failed to run docker build: {e}"))?;
    if !status.success() {
        return Err("docker build failed".into());
    }
    Ok(())
}

/// Removes the older images of this stage once a new one has built. The
/// tag is a fingerprint, so every edit to the Dockerfile leaves the last
/// image behind — 1.5GB for the core stage, over 4GB for the app stage —
/// and nothing else would ever name them again.
///
/// Two things keep this from taking something out from under anybody.
/// Docker refuses to remove an image a container is still running, so a
/// run in progress next door is safe by construction; and what a rebuild
/// costs after this is the layer cache, not the download, because the
/// layers stay. Every removal is printed: a command that quietly frees
/// gigabytes is one nobody can audit.
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

fn in_container(root: &Path, tag: &str, command: &[String], shell: bool) -> Result<(), String> {
    let mut cmd = Command::new("docker");
    cmd.arg("run").arg("--rm");
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
        .arg("--workdir")
        .arg(WORK);

    let mut command = command.to_vec();
    let keepsake = keepsakes::bridge(&mut command, OUT_MOUNT)?;
    if let Some(out) = &keepsake {
        cmd.arg("--volume")
            .arg(format!("{}:{OUT_MOUNT}", mount_path(out)));
    }

    cmd.arg(tag);
    if shell {
        cmd.arg("bash");
    } else {
        cmd.args(&command);
    }
    let status = cmd
        .status()
        .map_err(|e| format!("failed to run docker: {e}"))?;
    if status.success() {
        keepsakes::onto_the_board(keepsake.as_deref(), &command);
        return Ok(());
    }
    Err(match status.code() {
        Some(code) => format!("the run in {tag} exited {code}"),
        None => format!("the run in {tag} was killed"),
    })
}

fn here(root: &Path, command: &[String]) -> Result<(), String> {
    let (program, arguments) = command.split_first().ok_or("nothing to run")?;
    let status = Command::new(program)
        .args(arguments)
        .current_dir(root)
        .status()
        .map_err(|e| format!("failed to run {program}: {e}"))?;
    if status.success() {
        return Ok(());
    }
    // The child's own code, the way the container path reports it — a
    // runner must not flatten a child's exit into an anonymous failure.
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
fn volume(root: &Path, kind: &str) -> String {
    let name = root
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "root".to_string());
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

#[cfg(test)]
mod tests {
    use super::*;

    fn words(line: &str) -> Vec<String> {
        line.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn the_small_image_takes_everything_that_holds_no_qt() {
        assert_eq!(
            stage_for(&words("test -p platitude-core --test it")),
            "core"
        );
        assert_eq!(stage_for(&words("test -p xtask -p platitude-core")), "core");
        assert_eq!(stage_for(&words("fmt --all --check")), "core");
        assert_eq!(stage_for(&[]), "core");
    }

    #[test]
    fn anything_that_can_reach_the_app_takes_the_other_one() {
        assert_eq!(stage_for(&words("test")), "app");
        assert_eq!(stage_for(&words("build --workspace")), "app");
        assert_eq!(stage_for(&words("clippy -p platitude-app")), "app");
        assert_eq!(stage_for(&words("verify-ui commit --preset basic")), "app");
    }

    #[test]
    fn an_xtask_verb_is_run_through_the_task_runner() {
        assert_eq!(
            command_line(&words("verify-ui commit --preset basic")),
            words("cargo xtask verify-ui commit --preset basic")
        );
        assert_eq!(
            command_line(&words("test -p platitude-core")),
            words("cargo test -p platitude-core")
        );
    }

    #[test]
    fn volume_names_survive_a_windows_path() {
        assert_eq!(
            volume(
                Path::new("C:\\Users\\x\\IdeaProjects\\platitude-gg"),
                "target"
            ),
            "pg-linux-target-platitude-gg"
        );
    }

    #[test]
    fn mount_paths_are_forward_slashed() {
        assert_eq!(
            mount_path(Path::new("C:\\Users\\x\\platitude-gg")),
            "C:/Users/x/platitude-gg"
        );
    }

    #[test]
    fn the_qt_version_comes_from_the_workflow() {
        let root = crate::workspace_root();
        let version = qt_version(&root).expect("QT_VERSION in ci.yml");
        assert!(
            version.split('.').all(|part| part.parse::<u32>().is_ok()),
            "{version:?} does not look like a version"
        );
    }
}
