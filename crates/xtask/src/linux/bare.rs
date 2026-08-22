//! `cargo xtask linux bare` -- the built binary on a stock Ubuntu, and
//! the discovery loop that works its runtime package list out again.

use std::path::Path;
use std::process::Command;

use super::{OUT_MOUNT, ensure_image, in_container, mount_path, volume};
use crate::keepsakes::keepsake_dir;

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
pub(super) fn bare(root: &Path, discover: bool) -> Result<(), String> {
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
         timeout 50s env QT_QPA_PLATFORM=offscreen \
         QT_FORCE_STDERR_LOGGING=1 PG_AUTO_ACT=band \
         PG_AUTO_WATCHDOG_MS={BARE_WATCHDOG_MS} PG_SHOT_DIR={OUT_MOUNT} /built/release/platitude-gg"
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

const BARE_WATCHDOG_MS: u32 = 30_000;

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
/// round again. Installing more than the named library per round hides
/// libraries that are needed but are not the current blocker.
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
        PG_AUTO_ACT=band PG_AUTO_WATCHDOG_MS={BARE_WATCHDOG_MS} PG_SHOT_DIR={OUT_MOUNT} \
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
