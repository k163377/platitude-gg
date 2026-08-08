//! `cargo xtask linux <cargo command…>` — the workspace on Linux, from a
//! workstation that has none.
//!
//! Everything after the options goes to cargo inside a container built from
//! ci/linux/Dockerfile, so `cargo xtask linux test -p platitude-core` is
//! `cargo test -p platitude-core` on Ubuntu. Run on Linux the container
//! drops out and the same line runs where it stands: one verb, three
//! operating systems (CLAUDE.md: no Windows-only dev tooling).
//!
//! The build directory is a docker volume mounted over /work/target, never
//! the host's. One target/ shared between two operating systems is two
//! cargos on one build lock and two sets of fingerprints for the same paths
//! — the serialized-and-rebuilding failure worktrees exist to avoid, one
//! boundary further out.

use std::io::IsTerminal;
use std::path::Path;
use std::process::{Command, Stdio};

/// The image, and the stage of the Dockerfile built into it.
const IMAGE: &str = "pg-linux";
const STAGE: &str = "core";

/// Where the checkout, the build directory and the download cache land
/// inside the container.
const WORK: &str = "/work";
const TARGET_MOUNT: &str = "/work/target";
const REGISTRY_MOUNT: &str = "/usr/local/cargo/registry";

pub fn run(args: &[String]) -> Result<(), String> {
    let mut rebuild = false;
    let mut shell = false;
    // Options are the leading tokens only: everything from the first one
    // that is not ours belongs to cargo, `--` and all.
    let mut at = 0;
    while let Some(arg) = args.get(at) {
        match arg.as_str() {
            "--rebuild" => rebuild = true,
            "--shell" => shell = true,
            _ => break,
        }
        at += 1;
    }
    let rest = &args[at..];
    if !shell && rest.is_empty() {
        return Err(
            "linux needs a cargo command, e.g. `cargo xtask linux test -p platitude-core`".into(),
        );
    }
    if let Some(objection) = would_need_qt(rest) {
        return Err(objection);
    }

    let root = crate::workspace_root();
    if cfg!(target_os = "linux") {
        if shell {
            return Err("--shell has nothing to enter: this is already Linux".into());
        }
        println!("already on Linux — running here, no container");
        return cargo_here(&root, rest);
    }

    let tag = image_tag(&root)?;
    if rebuild || !image_exists(&tag)? {
        build_image(&root, &tag)?;
    }
    run_in_container(&root, &tag, rest, shell)
}

/// The core image carries no Qt, so a build that reaches platitude-app dies
/// somewhere inside qtbridge with nothing in the message about why. Say it
/// here, before the twenty minutes.
fn would_need_qt(rest: &[String]) -> Option<String> {
    let verb = rest.first()?.as_str();
    if !matches!(
        verb,
        "build" | "check" | "test" | "clippy" | "bench" | "run"
    ) {
        return None;
    }
    let names_core = rest
        .windows(2)
        .any(|pair| matches!(pair[0].as_str(), "-p" | "--package") && pair[1] == "platitude-core");
    if names_core {
        return None;
    }
    Some(format!(
        "`cargo {verb}` without `-p platitude-core` reaches platitude-app, and \
         the {STAGE} image carries no Qt. Name the package. (The Qt-carrying \
         stage lands with the app work.)"
    ))
}

/// The image is named after what builds it: change the Dockerfile or the
/// toolchain pin and the tag changes with it, so a stale image can never be
/// the one that answers. Docker's layer cache keeps the rebuild cheap. FNV-1a
/// over both files — a fingerprint, not a security claim, and the tree is LF
/// everywhere (.gitattributes) so the same tree hashes the same on all three
/// operating systems.
fn image_tag(root: &Path) -> Result<String, String> {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for relative in ["ci/linux/Dockerfile", "rust-toolchain.toml"] {
        let path = root.join(relative);
        let bytes =
            std::fs::read(&path).map_err(|e| format!("failed to read {}: {e}", path.display()))?;
        for byte in bytes {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    Ok(format!("{IMAGE}:{STAGE}-{hash:016x}"))
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

fn build_image(root: &Path, tag: &str) -> Result<(), String> {
    println!("building {tag} — first time takes a few minutes (Ubuntu + the pinned toolchain)");
    let status = Command::new("docker")
        .arg("build")
        .arg("--file")
        .arg(root.join("ci").join("linux").join("Dockerfile"))
        .arg("--target")
        .arg(STAGE)
        .arg("--tag")
        .arg(tag)
        .arg(root)
        .status()
        .map_err(|e| format!("failed to run docker build: {e}"))?;
    if !status.success() {
        return Err("docker build failed".into());
    }
    Ok(())
}

fn run_in_container(root: &Path, tag: &str, rest: &[String], shell: bool) -> Result<(), String> {
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
        .arg(WORK)
        .arg(tag);
    if shell {
        cmd.arg("bash");
    } else {
        cmd.arg("cargo").args(rest);
    }
    let status = cmd
        .status()
        .map_err(|e| format!("failed to run docker: {e}"))?;
    if status.success() {
        return Ok(());
    }
    Err(match status.code() {
        Some(code) => format!("the run in {tag} exited {code}"),
        None => format!("the run in {tag} was killed"),
    })
}

fn cargo_here(root: &Path, rest: &[String]) -> Result<(), String> {
    let status = Command::new("cargo")
        .args(rest)
        .current_dir(root)
        .status()
        .map_err(|e| format!("failed to run cargo: {e}"))?;
    if status.success() {
        return Ok(());
    }
    Err("the cargo command failed".into())
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

    #[test]
    fn refuses_a_build_that_would_reach_the_app() {
        assert!(would_need_qt(&["test".to_string()]).is_some());
        assert!(would_need_qt(&["build".to_string(), "--workspace".to_string()]).is_some());
    }

    #[test]
    fn lets_a_core_only_build_and_the_commands_that_need_nothing_through() {
        let core = ["test", "-p", "platitude-core", "--test", "it"].map(String::from);
        assert_eq!(would_need_qt(&core), None);
        let fmt = ["fmt", "--all", "--check"].map(String::from);
        assert_eq!(would_need_qt(&fmt), None);
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
}
