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
/// ask for it: the `PGG_*` protocol, the drivers, and the `platitude.auto`
/// QML module (`platitude-app` §features). Everything this task runner
/// starts drives the app over that protocol, so everything it builds asks
/// for it — the shipped build is the plain `cargo build --release` nobody
/// here runs.
pub(crate) const HARNESS_FEATURE: &str = "automation";

/// Builds the app in release unless `build` says not to, and answers
/// where its binary sits either way — `--no-build` still needs the path.
///
/// Always with [`HARNESS_FEATURE`], including the window `launch` opens:
/// it is inert without a `PGG_*` variable, and asking for it every time is
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
        // Announced for as long as it compiles: a build waits for a
        // measurement to end, and a measurement waits for it (`still`).
        let _busy = crate::still::busy(root, "cargo build --release")?;
        // Through the budget's own runner: this cargo is what the unit
        // that asked for a build is actually doing, and a ledger reading
        // this process's ticket after it is killed has to know which
        // number is still compiling (`crate::budget::watched`).
        let status = crate::budget::watched(
            std::process::Command::new("cargo")
                .args([
                    "build",
                    "--locked",
                    "--release",
                    "--features",
                    HARNESS_FEATURE,
                ])
                .args(extra)
                .current_dir(root)
                .env("PATH", path),
        )
        .map_err(|e| format!("failed to run cargo: {e}"))?;
        if !status.success() {
            return Err(format!("cargo build --release failed{}", held_by(root)));
        }
    }
    where_it_lands(root, "release")
}

/// The profile the shipped build lands in: the release settings, in a
/// directory the harness builds never write (Cargo.toml).
const SHIPPED_PROFILE: &str = "shipped";

/// The release binary with **no** features on it: the build a person
/// installs, and the one nothing here can drive.
///
/// Deliberately separate from [`app_exe`]: that one asks for the harness,
/// which is the whole of what the two callers here are about — `shipped`
/// checks that the QML still loads without it, and `perf --shipped`
/// weighs it. It is built in a profile of its own, `target/shipped/`, so
/// that neither build ever replaces the other's binary: the gate runs
/// `shipped` beside its verbs, which read `target/release/` as they run.
/// The evidence still names the feature set it asked for
/// (`perf::Options::features`) — a `--no-build` run is measuring whatever
/// the directory holds.
pub(crate) fn shipped_exe(
    root: &Path,
    path: &std::ffi::OsStr,
    build: bool,
) -> Result<PathBuf, String> {
    if build {
        println!("building (release, no features — the shipped set)…");
        let _busy = crate::still::busy(root, "cargo build --profile shipped")?;
        let status = crate::budget::watched(
            std::process::Command::new("cargo")
                .args(["build", "--locked", "--profile", SHIPPED_PROFILE])
                .current_dir(root)
                .env("PATH", path),
        )
        .map_err(|e| format!("failed to run cargo: {e}"))?;
        if !status.success() {
            return Err("cargo build --profile shipped failed".into());
        }
    }
    where_it_lands(root, SHIPPED_PROFILE)
}

/// What to add to a failed release build when a run of this tree is
/// standing on the binary it links, and nothing when none is.
///
/// Windows holds a running image against every write to it, so a run
/// left over from an earlier command turns the next `--release` step red
/// with a linker error that names a path and no reason. **Whether that
/// is what happened is the reader's to say**: the exit status is all
/// this has of the build, and a compile error under a standing run is
/// still a compile error. Who is standing is what cargo's lines cannot
/// tell them; why the build failed is already above. The window a person
/// opened is not the one to look at — `launch` stands it from a copy
/// (`crate::gui::standing_copy`). Best effort: a listing that will not
/// answer is not a reason to say less than cargo already did.
fn held_by(root: &Path) -> String {
    match crate::gui::standing_under(root) {
        Ok(standing) if !standing.is_empty() => {
            let who: Vec<String> = standing
                .iter()
                .map(|(pid, exe)| format!("{pid} ({exe})"))
                .collect();
            format!(
                " — and a run of this tree is standing on the binary it links: {}. \
                 If that is what the linker refused, `cargo xtask kill` reaps this \
                 tree's runs and nothing else.",
                who.join(", ")
            )
        }
        _ => String::new(),
    }
}

/// The app's file name on this platform.
pub(crate) fn exe_name() -> &'static str {
    if cfg!(windows) {
        "platitude-gg.exe"
    } else {
        "platitude-gg"
    }
}

/// The app's binary as cargo leaves it under `profile`'s directory.
fn where_it_lands(root: &Path, profile: &str) -> Result<PathBuf, String> {
    let exe = root.join("target").join(profile).join(exe_name());
    if !exe.is_file() {
        return Err(format!(
            "{} not found — build first (or drop --no-build)",
            exe.display()
        ));
    }
    Ok(exe)
}
