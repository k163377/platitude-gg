//! The tree this runner serves, and the app it builds out of it.
//!
//! Kept out of main.rs on purpose: the crate root is what every module
//! reaches for and what dispatches to every module, and a helper that
//! lives there makes every module depend on every other through it. The
//! gate's dependency graph (`gate::graph`) reads that as "any change
//! touches everything", which is the one answer it must never give for
//! nothing (.claude/rules/structure.md §クレート root).

use std::path::{Path, PathBuf};

/// The workspace root, resolved at compile time from this crate's location.
/// A worktree builds its own task runner, so this is that worktree's root.
pub(crate) fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap_or(Path::new("."))
        .to_path_buf()
}

/// The app's verification harness, which is not in a build that did not
/// ask for it: the `PG_*` protocol, the drivers, and the `platitude.auto`
/// QML module (`platitude-app` §features). Everything this task runner
/// starts drives the app over that protocol, so everything it builds asks
/// for it — the shipped build is the plain `cargo build --release` nobody
/// here runs.
pub(crate) const HARNESS_FEATURE: &str = "automation";

/// Builds the app in release unless `build` says not to, and answers
/// where its binary sits either way — `--no-build` still needs the path.
///
/// Always with [`HARNESS_FEATURE`], including the window `launch` opens:
/// it is inert without a `PG_*` variable, and asking for it every time is
/// what keeps one release binary between the two commands instead of a
/// relink every time somebody moves from a window to a verify-ui run.
///
/// `extra` follows `build --release`: the package and any further features
/// a caller needs, and every feature names itself in the building line.
/// `path` is the PATH the *build* runs with, which is not always the one
/// the run itself gets — verify-ui stages a git shim onto its child's
/// PATH, and building against that would build against the shim.
pub(crate) fn app_exe(
    root: &Path,
    path: &std::ffi::OsStr,
    build: bool,
    extra: &[&str],
) -> Result<PathBuf, String> {
    if build {
        let mut features = vec![HARNESS_FEATURE];
        features.extend(
            extra
                .windows(2)
                .filter(|pair| pair[0] == "--features")
                .map(|pair| pair[1]),
        );
        println!("building (release, {})…", features.join(" + "));
        let status = std::process::Command::new("cargo")
            .args(["build", "--release", "--features", HARNESS_FEATURE])
            .args(extra)
            .current_dir(root)
            .env("PATH", path)
            .status()
            .map_err(|e| format!("failed to run cargo: {e}"))?;
        if !status.success() {
            return Err("cargo build --release failed".into());
        }
    }
    let exe = root.join("target").join("release").join(if cfg!(windows) {
        "platitude-gg.exe"
    } else {
        "platitude-gg"
    });
    if !exe.is_file() {
        return Err(format!(
            "{} not found — build first (or drop --no-build)",
            exe.display()
        ));
    }
    Ok(exe)
}
