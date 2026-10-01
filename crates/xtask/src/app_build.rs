//! The app as this runner builds it: the feature every build asks for,
//! the release and shipped builds, and where each binary lands. Out of
//! `tree` so that the root lookup every module reads carries no build
//! (反映前テストの機械化.md §依存木).

use std::path::{Path, PathBuf};

/// The feature that builds the app's verification harness in
/// (platitude-app's Cargo.toml `[features]`). Every build this runner
/// drives asks for it; [`shipped_exe`] builds the one without.
pub(crate) const HARNESS_FEATURE: &str = "automation";

/// Builds the app in release unless `build` is false, and answers where
/// its binary sits either way.
///
/// Always with [`HARNESS_FEATURE`], `launch`'s window included: it is
/// inert without a `PGG_*` variable, and asking every time keeps one
/// release binary between `launch` and verify-ui.
///
/// `extra` follows `build --release` (package, further features). `path`
/// is the PATH the *build* runs with — not the run's, which may carry
/// verify-ui's git shim.
pub(crate) fn app_exe(
    root: &Path,
    path: &std::ffi::OsStr,
    with: BuildEnv<'_>,
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
        // A build and a measurement wait for each other (`still`).
        let _busy = crate::still::busy(root, "cargo build --release")?;
        // Through the budget's runner, so a ledger reading this process's
        // ticket after it is killed knows which cargo still compiles
        // (`crate::budget::watched`).
        let mut cargo = std::process::Command::new("cargo");
        cargo
            .args([
                "build",
                "--locked",
                "--release",
                "--features",
                HARNESS_FEATURE,
            ])
            .args(extra)
            .current_dir(root)
            .env("PATH", path);
        with.apply(&mut cargo);
        let status =
            crate::budget::watched(&mut cargo).map_err(|e| format!("failed to run cargo: {e}"))?;
        if !status.success() {
            return Err(format!("cargo build --release failed{}", held_by(root)));
        }
    }
    where_it_lands(root, "release")
}

/// What a build's cargo starts with beyond its tree and PATH. The default
/// is this process's environment as it stands.
#[derive(Clone, Copy, Default)]
pub(crate) struct BuildEnv<'a> {
    /// Named in `QMAKE` too, for a build that may follow another Qt in
    /// the same target directory (`crate::qt`).
    pub(crate) qmake: Option<&'a Path>,
    /// Not handed down: what this tree's cargo configuration put into this
    /// process, which a build of another tree would take for its own
    /// (`perf::identity::injected`).
    pub(crate) unset: &'a [String],
}

impl BuildEnv<'_> {
    fn apply(self, cargo: &mut std::process::Command) {
        if let Some(qmake) = self.qmake {
            cargo.env("QMAKE", qmake);
        }
        for key in self.unset {
            cargo.env_remove(key);
        }
    }
}

/// The profile the shipped build lands in: the release settings, in a
/// directory only this build writes (Cargo.toml).
const SHIPPED_PROFILE: &str = "shipped";

/// The release binary with no features: the build a person installs,
/// for `shipped` (the QML loads without the harness) and `perf --shipped`
/// (what the harness weighs). A profile of its own, so it never replaces
/// the binary the gate's verbs read from `target/release/` while
/// `shipped` runs beside them. A `--no-build` run measures whatever the
/// directory holds; the evidence names the feature set asked for
/// (`perf::Options::features`).
pub(crate) fn shipped_exe(
    root: &Path,
    path: &std::ffi::OsStr,
    with: BuildEnv<'_>,
    build: bool,
) -> Result<PathBuf, String> {
    if build {
        println!("building (release, no features — the shipped set)…");
        let _busy = crate::still::busy(root, "cargo build --profile shipped")?;
        let mut cargo = std::process::Command::new("cargo");
        cargo
            .args(["build", "--locked", "--profile", SHIPPED_PROFILE])
            .current_dir(root)
            .env("PATH", path);
        with.apply(&mut cargo);
        let status =
            crate::budget::watched(&mut cargo).map_err(|e| format!("failed to run cargo: {e}"))?;
        if !status.success() {
            return Err("cargo build --profile shipped failed".into());
        }
    }
    where_it_lands(root, SHIPPED_PROFILE)
}

/// What to add to a failed release build when a run of this tree is
/// standing on the binary it links, and nothing when none is.
///
/// Windows locks a running image, so a leftover run turns the next
/// `--release` red with a linker error naming a path and no reason. Only a
/// hint: the exit status cannot tell that from a compile error. A `launch`
/// window is never the one — it runs from a copy
/// (`crate::gui::standing_copy`).
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
