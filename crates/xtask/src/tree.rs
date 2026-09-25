//! The tree this runner serves, and the app it builds out of it. Not in
//! main.rs: .claude/rules/structure.md §分割「クレート root」.

use std::path::{Path, PathBuf};

/// The workspace root, fixed at compile time: a worktree builds its own
/// task runner, so this is that worktree's root.
pub(crate) fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap_or(Path::new("."))
        .to_path_buf()
}

/// The primary checkout the tree at `cwd` belongs to — read off the
/// filesystem, not asked of git, because the readers are hooks that run
/// on every line a session types.
///
/// A checkout and its seats must give one answer, or what a session wrote
/// before entering a seat is left behind (`hook::repeat`, `seats::entry`).
pub(crate) fn primary_root(cwd: &str) -> Option<PathBuf> {
    let mut at: &Path = Path::new(cwd);
    loop {
        let dot_git = at.join(".git");
        if dot_git.is_dir() {
            return Some(at.to_path_buf());
        }
        if dot_git.is_file() {
            return linked_primary(&dot_git);
        }
        at = at.parent()?;
    }
}

/// The primary checkout a linked worktree's `.git` file names; a gitdir
/// not under `<primary>/.git/worktrees/` (`--separate-git-dir`) has none.
fn linked_primary(dot_git: &Path) -> Option<PathBuf> {
    let text = std::fs::read_to_string(dot_git).ok()?;
    let named = text
        .lines()
        .find_map(|line| line.trim().strip_prefix("gitdir:"))?;
    let root = Path::new(named.trim()).ancestors().nth(3)?.to_path_buf();
    root.join(".git").is_dir().then_some(root)
}

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

#[cfg(test)]
mod tests {
    use super::primary_root;

    #[test]
    fn a_seat_and_its_checkout_name_one_root() {
        let base = crate::verify::claim_dir(&std::env::temp_dir().join("pgg-hook"), "roots")
            .expect("a directory of its own");
        let primary = base.join("platitude-gg");
        let seat = primary.join(".claude/worktrees/a");
        std::fs::create_dir_all(primary.join(".git/worktrees/a")).expect("a primary checkout");
        std::fs::create_dir_all(seat.join("crates/xtask")).expect("a seat");
        std::fs::write(
            seat.join(".git"),
            format!("gitdir: {}/.git/worktrees/a\n", primary.display()),
        )
        .expect("a seat's .git file");

        let named = |at: &std::path::Path| primary_root(&at.to_string_lossy());
        assert_eq!(named(&primary).as_deref(), Some(primary.as_path()));
        assert_eq!(named(&seat).as_deref(), Some(primary.as_path()));
        // From a subdirectory that carries no .git of its own.
        assert_eq!(
            named(&seat.join("crates/xtask")).as_deref(),
            Some(primary.as_path())
        );
        // A .git naming a layout this reader does not know.
        std::fs::write(seat.join(".git"), "gitdir: /elsewhere\n").expect("a foreign .git file");
        assert_eq!(named(&seat), None);

        std::fs::remove_dir_all(&base).ok();
    }
}
