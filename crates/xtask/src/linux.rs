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
//! are the opposite of a build environment: `runtime` carries exactly what
//! a package would declare, which is the only place that can say the built
//! thing runs somewhere it was not built, and `bare` carries nothing at
//! all — the blank sheet `--discover` works the declaration out on.
//!
//! The build directory is a docker volume mounted over /work/target, never
//! the host's. One target/ shared between two operating systems is two
//! cargos on one build lock and two sets of fingerprints for the same paths
//! — the serialized-and-rebuilding failure worktrees exist to avoid, one
//! boundary further out. The cargo registry is a volume for the same
//! reason, and the tests build their repositories under the container's own
//! /tmp, so the only thing crossing the host filesystem is reading source.
//!
//! Docker says that much is slow, and it is: over the 162 source files,
//! stat costs 331ms against 3ms on the container's own filesystem and
//! reading them 438ms against 6ms (`cargo metadata` 151 against 56). A
//! hundred times, and a third of a second — cargo pays it once per build to
//! check fingerprints, against a compile measured in seconds. Keeping a
//! second copy of the tree inside a volume would buy that back and cost a
//! second answer to "which tree is the real one", so the source stays where
//! it is edited.
//!
//! One thing does not survive the boundary: a worktree's `.git` is a file
//! naming an absolute Windows path, which git inside reads as relative and
//! cannot follow, so a run from a worktree logs "not a git repository"
//! about /work once. Nothing depends on it — the app is handed the
//! repositories it opens, and the identity it looks for in the checkout is
//! not in a container anyway.

use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

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

    let root = crate::workspace_root();
    // The one verb that is about a different machine rather than a
    // different toolchain: it runs in the container even on Linux, because
    // what it asks is whether a stock Ubuntu is enough.
    if rest.first().is_some_and(|verb| verb == "bare") {
        let discover = rest.iter().any(|word| word == "--discover");
        return bare(&root, discover);
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

/// The tag for `stage`, built if it is not on disk (or if asked again).
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

/// `cargo xtask linux bare` — the built binary on an Ubuntu that carries
/// only what a package would declare, with Qt staged beside it the way a
/// distribution would ship it.
///
/// This is not the discovery run that found the list (that one added a
/// package at a time and is written down in P5-確認事項); it is the check
/// that the list is still true. The runtime image installs exactly what is
/// claimed, so the claim either holds or the run stops with the name of
/// the library that broke it. Cheap enough to put in front of a merge:
/// nothing is downloaded, and the answer is a start or a missing symbol.
fn bare(root: &Path, discover: bool) -> Result<(), String> {
    // Workspace-wide, not `-p platitude-app`: pg-todo-editor is a bin of
    // platitude-core, and a release directory without it is one binary
    // short of what interactive rebase needs.
    let app = ensure_image(root, "app", false)?;
    println!("building the release, workspace-wide…");
    let build = ["cargo", "build", "--release"].map(String::from);
    in_container(root, &app, &build, false)?;

    if discover {
        return discover_deps(root);
    }

    let runtime = ensure_image(root, "runtime", false)?;
    let out = keepsake_dir("bare")?;
    println!("screenshot: {}", out.display());

    // The helper is checked before the app is started, because a run that
    // never opens a stopped rebase would not miss it.
    let script = format!(
        "set -e; \
         test -x /built/release/pg-todo-editor \
           || {{ echo 'pg-todo-editor is not beside the app'; exit 2; }}; \
         QT_QPA_PLATFORM=offscreen QT_FORCE_STDERR_LOGGING=1 \
         PG_AUTO_QUIT_MS={QUIT_MS} PG_SHOT_DIR={OUT_MOUNT} \
         /built/release/platitude-gg"
    );
    let status = run_on_bare(root, &runtime, &out, &script)?;
    if !status.success() {
        return Err(format!(
            "the app did not start on a stock Ubuntu (exit {}). The library \
             the loader stopped on names itself above; `cargo xtask linux \
             bare --discover` works the whole list out again, and the \
             runtime stage of ci/linux/Dockerfile is where it is declared.",
            status.code().unwrap_or(-1)
        ));
    }
    let shot = out.join("app.png");
    if !shot.is_file() {
        return Err(format!(
            "it started and quit without leaving {}: nothing drew",
            shot.display()
        ));
    }
    println!(
        "PASS: started on a stock Ubuntu and drew {}",
        shot.display()
    );
    Ok(())
}

/// How long a bare run gets before it quits itself.
const QUIT_MS: u32 = 4000;

/// Every library this has ever been stopped on, and the Ubuntu package
/// that carries it. A name that is not in here stops the discovery and
/// says so — guessing a package from a file name is how a list acquires
/// something nobody needed.
const SONAME_PACKAGES: [(&str, &str); 12] = [
    ("libglib-2.0.so.0", "libglib2.0-0t64"),
    ("libEGL.so.1", "libegl1"),
    ("libGL.so.1", "libgl1"),
    ("libfontconfig.so.1", "libfontconfig1"),
    ("libfreetype.so.6", "libfreetype6"),
    ("libxkbcommon.so.0", "libxkbcommon0"),
    ("libdbus-1.so.3", "libdbus-1-3"),
    ("libgssapi_krb5.so.2", "libgssapi-krb5-2"),
    ("libX11.so.6", "libx11-6"),
    ("libXext.so.6", "libxext6"),
    ("libXrender.so.1", "libxrender1"),
    ("libxcb.so.1", "libxcb1"),
];

/// Works the runtime dependencies out again from a stock Ubuntu: start the
/// binary, read the one library the loader names, install only that, go
/// round again.
///
/// The first attempt at this by hand installed a package per round whatever
/// the error said, which hid every library that was needed but not the
/// current blocker — libGL among them, and the pinned check caught it in
/// seconds. Only the named one goes in now.
fn discover_deps(root: &Path) -> Result<(), String> {
    let bare_tag = ensure_image(root, "bare", false)?;
    let out = keepsake_dir("discover")?;
    let map: String = SONAME_PACKAGES
        .iter()
        .map(|(soname, package)| format!("{soname} {package}\\n"))
        .collect();
    let script = format!(
        r#"set -u
printf '{map}' > /tmp/map
apt-get update -qq >/dev/null 2>&1
# git first: the product cannot run without it whatever the loader says.
apt-get install -y -qq --no-install-recommends git >/dev/null 2>&1
needed="git"
for _ in $(seq 1 20); do
  out=$(QT_QPA_PLATFORM=offscreen QT_FORCE_STDERR_LOGGING=1 \
        PG_AUTO_QUIT_MS={QUIT_MS} PG_SHOT_DIR={OUT_MOUNT} \
        timeout 60 /built/release/platitude-gg 2>&1)
  [ $? = 0 ] && break
  soname=$(printf '%s' "$out" | sed -n 's/.*error while loading shared libraries: \([^:]*\).*/\1/p' | head -1)
  if [ -z "$soname" ]; then
    echo "stopped without naming a library:"; printf '%s\n' "$out" | head -5; exit 1
  fi
  pkg=$(awk -v s="$soname" '$1 == s {{print $2}}' /tmp/map)
  if [ -z "$pkg" ]; then
    echo "no package known for $soname — add it to SONAME_PACKAGES"; exit 1
  fi
  echo "stopped on $soname -> $pkg"
  apt-get install -y -qq --no-install-recommends "$pkg" >/dev/null 2>&1
  needed="$needed $pkg"
done
echo
echo "DECLARE:$needed"
echo
apt-get install -y -qq --no-install-recommends fontconfig >/dev/null 2>&1
echo "a kanji on a stock Ubuntu resolves to: $(fc-match -s 'sans:lang=ja' 2>/dev/null | head -1)"
echo "Noto Sans CJK JP resolves to        : $(fc-match 'Noto Sans CJK JP' 2>/dev/null)"
"#
    );
    let status = run_on_bare(root, &bare_tag, &out, &script)?;
    if !status.success() {
        return Err("the discovery did not reach a start".into());
    }
    println!(
        "\ncompare DECLARE above with the runtime stage of ci/linux/Dockerfile: \
         they are meant to be the same list, and this is the only thing that \
         measures it"
    );
    Ok(())
}

/// Runs `script` on one of the two Ubuntu-only images, with the build
/// directory read-only and a host directory to leave a screenshot in.
fn run_on_bare(
    root: &Path,
    tag: &str,
    out: &Path,
    script: &str,
) -> Result<std::process::ExitStatus, String> {
    Command::new("docker")
        .arg("run")
        .arg("--rm")
        .arg("--volume")
        .arg(format!("{}:/built:ro", volume(root, "target")))
        .arg("--volume")
        .arg(format!("{}:{OUT_MOUNT}", mount_path(out)))
        .arg(tag)
        .args(["bash", "-lc", script])
        .status()
        .map_err(|e| format!("failed to run docker: {e}"))
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
    if let Some(out) = keepsakes(&command)? {
        cmd.arg("--volume")
            .arg(format!("{}:{OUT_MOUNT}", mount_path(&out)));
        command.push("--shot-dir".to_string());
        command.push(OUT_MOUNT.to_string());
        println!("screenshots and settings: {}", out.display());
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
        return Ok(());
    }
    Err(match status.code() {
        Some(code) => format!("the run in {tag} exited {code}"),
        None => format!("the run in {tag} was killed"),
    })
}

/// A host directory for what a run means to be looked at afterwards, or
/// None when the command leaves nothing. verify-ui writes its screenshot
/// and the settings it ran with into --shot-dir; inside a container that is
/// a place nobody can open, and the whole verdict is a PNG.
fn keepsakes(command: &[String]) -> Result<Option<PathBuf>, String> {
    if !command.iter().any(|word| word == "verify-ui") {
        return Ok(None);
    }
    if command.iter().any(|word| word == "--shot-dir") {
        // Named by the caller, who then owns where it lands.
        return Ok(None);
    }
    keepsake_dir("shots").map(Some)
}

/// A fresh host directory for a run to leave things in, named for what
/// kind of run it was.
fn keepsake_dir(kind: &str) -> Result<PathBuf, String> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let dir = std::env::temp_dir()
        .join("pg-linux")
        .join(format!("{kind}-{nanos}"));
    std::fs::create_dir_all(&dir).map_err(|e| format!("failed to make {}: {e}", dir.display()))?;
    Ok(dir)
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
    Err("the command failed".into())
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
    fn only_a_verify_run_without_a_directory_of_its_own_gets_one() {
        assert!(
            keepsakes(&words("cargo xtask verify-ui commit"))
                .expect("temp dir")
                .is_some()
        );
        assert_eq!(
            keepsakes(&words("cargo xtask verify-ui commit --shot-dir /somewhere")).expect("none"),
            None
        );
        assert_eq!(
            keepsakes(&words("cargo test -p platitude-core")).expect("none"),
            None
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
