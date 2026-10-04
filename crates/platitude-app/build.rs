//! Hands the linker the Windows resource with the exe's icon and the name
//! the shell shows for it (without it, `platitude-gg.exe`), and the binary
//! the commit it is built from (`models::build_id`).
//!
//! `assets/platitude.res` is checked in compiled: the resource compiler
//! comes only with the Windows SDK. `assets/platitude.rc` says how to
//! rebuild it.
//!
//! The running window's icon is separate (`winframe::set_icon`): the shell
//! reads this one off the file, the taskbar that one off the window.

use std::process::Command;

const GIVEN_COMMIT: &str = "PLATITUDE_GIVEN_COMMIT";
const GIVEN_TAGS: &str = "PLATITUDE_GIVEN_TAGS";

fn main() {
    println!("cargo:rerun-if-changed=assets/platitude.res");
    stamp_commit();
    link_resource();
}

/// The commit `HEAD` names and the one the version's tag names, each empty
/// where nobody can say — a source archive. Re-run when this tree's `HEAD`
/// or branch moves, a tag comes or goes, or refs are packed: each re-run
/// rebuilds the crate, so nothing else is watched.
fn stamp_commit() {
    let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_default();
    // A Linux container sees this tree but not the repository its `.git`
    // file points into; xtask reads the two out there and hands them in
    // (`linux::given_commit`).
    println!("cargo:rerun-if-env-changed={GIVEN_COMMIT}");
    println!("cargo:rerun-if-env-changed={GIVEN_TAGS}");
    if let Ok(head) = std::env::var(GIVEN_COMMIT) {
        let tagged = std::env::var(GIVEN_TAGS)
            .unwrap_or_default()
            .split(' ')
            .any(|tag| tag == format!("v{version}"));
        println!("cargo:rustc-env=PLATITUDE_BUILD_COMMIT={head}");
        println!(
            "cargo:rustc-env=PLATITUDE_RELEASE_COMMIT={}",
            if tagged { head.as_str() } else { "" }
        );
        return;
    }
    let tag = format!("refs/tags/v{version}^{{commit}}");
    let head = git(&["rev-parse", "--verify", "-q", "HEAD"]).unwrap_or_default();
    let tagged = git(&["rev-parse", "--verify", "-q", &tag]).unwrap_or_default();
    println!("cargo:rustc-env=PLATITUDE_BUILD_COMMIT={head}");
    println!("cargo:rustc-env=PLATITUDE_RELEASE_COMMIT={tagged}");

    let mut watched = vec![
        "HEAD".to_string(),
        "logs/HEAD".to_string(),
        "packed-refs".to_string(),
        "refs/tags".to_string(),
    ];
    // The branch's log, for a move made from another tree (`update-ref`),
    // which this tree's `HEAD` log does not hear. The log, not the ref's
    // own file: that one is gone while the ref is packed.
    if let Some(branch) = git(&["symbolic-ref", "-q", "HEAD"]) {
        watched.push(format!("logs/{branch}"));
    }
    let mut args = vec!["rev-parse"];
    for path in &watched {
        args.extend(["--git-path", path.as_str()]);
    }
    // Only paths that are there: cargo reads a missing one as changed, and
    // would re-run (and rebuild) on every build.
    for path in git(&args).iter().flat_map(|out| out.lines()) {
        if std::path::Path::new(path).exists() {
            println!("cargo:rerun-if-changed={path}");
        }
    }
}

/// git's answer, trimmed; `None` when it could not be run or said no.
fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn link_resource() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let Ok(root) = std::env::var("CARGO_MANIFEST_DIR") else {
        return;
    };
    let res = std::path::Path::new(&root)
        .join("assets")
        .join("platitude.res");
    if res.is_file() {
        // Bins only: the test harnesses link without a resource.
        println!("cargo:rustc-link-arg-bins={}", res.display());
    } else {
        println!(
            "cargo:warning=assets/platitude.res is missing, so the exe keeps the shell's generic icon and is named after its own file"
        );
    }
}
