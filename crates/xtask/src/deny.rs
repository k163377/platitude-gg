//! `cargo xtask deny` — the dependency policy, read against the tree that
//! stands here.
//!
//! `deny.toml` is where two of CLAUDE.md's 絶対制約 are held by machine
//! rather than by review: nothing in the closure may open a socket, and
//! nothing in it may be a git implementation. The license allow list is
//! the third. cargo-deny is the only thing that reads that file, so a
//! change to it — or to any manifest, which is the only way the closure
//! itself moves — is unchecked until something runs this.
//!
//! `advisories` is deliberately not among the checks: it is the one that
//! goes out to github.com for the RustSec database, and stage 2 asks for
//! nothing the machine has not already got. The other three read the
//! resolved graph and the policy, so what they cost is what `cargo
//! metadata` costs — a fetch only where a build would have fetched too.
//! CI keeps the full set, advisories included
//! (`.github/workflows/ci.yml`, the `deny` job).

use std::process::Command;

/// The checks that read only what is here: the resolved graph
/// (`Cargo.lock`) and the policy (`deny.toml`).
const OFFLINE_CHECKS: [&str; 3] = ["bans", "sources", "licenses"];

pub fn run(args: &[String]) -> Result<(), String> {
    if let Some(unknown) = args.first() {
        return Err(format!("unknown option {unknown:?} (deny takes none)"));
    }
    installed()?;
    let root = crate::tree::workspace_root();
    // `--locked`: the graph it checks is the committed lock, never one it
    // resolved afresh — a stale lock is its own red, not a policy pass.
    let status = Command::new("cargo")
        .args(["deny", "--locked", "check"])
        .args(OFFLINE_CHECKS)
        .current_dir(&root)
        .status()
        .map_err(|e| format!("failed to run cargo deny: {e}"))?;
    if !status.success() {
        return Err(format!(
            "cargo deny check {} failed — the lines above name the crate and the clause",
            OFFLINE_CHECKS.join(" ")
        ));
    }
    Ok(())
}

/// Whether cargo-deny is here at all, asked first so its absence reads as
/// the missing tool it is instead of as cargo's "no such command" line —
/// the courtesy `linux` pays docker and `qt` pays qmake. An absent tool is
/// red and never a skip: a skipped policy check would still stamp the
/// commit as gated, which is the hole this step exists to close.
fn installed() -> Result<(), String> {
    match Command::new("cargo").args(["deny", "--version"]).output() {
        Ok(out) if out.status.success() => Ok(()),
        _ => Err(
            "cargo-deny is not installed, and the dependency policy cannot be checked \
                  without it: `cargo install --locked cargo-deny`"
                .into(),
        ),
    }
}
