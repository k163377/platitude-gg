//! `cargo xtask deny` — the dependency policy of `deny.toml`, checked
//! against this tree. That file holds two of CLAUDE.md's 絶対制約 by
//! machine (nothing in the closure opens a socket or implements git) plus
//! the license allow list; only cargo-deny reads it, so a change to it or
//! to any manifest is unchecked until this runs.
//!
//! `advisories` is left to CI (`.github/workflows/ci.yml`, the `deny`
//! job): it fetches the RustSec database, and stage 2 fetches nothing a
//! build would not.

use std::process::Command;

use crate::command::{self, Permission, Where};

pub(crate) static DENY: command::Command = command::Command {
    id: "deny.policy",
    call: "deny",
    purpose: "the dependency policy of deny.toml, against the tree standing here",
    run_in: Where::Either,
    needs: &["cargo-deny on PATH"],
    permission: Permission::Plain,
};

pub(crate) static COMMANDS: &[&command::Command] = &[&DENY];

/// The checks that read only what is here: the resolved graph
/// (`Cargo.lock`) and the policy (`deny.toml`).
const OFFLINE_CHECKS: [&str; 3] = ["bans", "sources", "licenses"];

pub fn run(args: &[String]) -> Result<(), String> {
    if let Some(unknown) = args.first() {
        return Err(format!("unknown option {unknown:?} (deny takes none)"));
    }
    installed()?;
    let root = crate::tree::workspace_root();
    // `--locked`: the graph it checks is the committed lock, and a
    // stale lock is its own red.
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

/// Asked first so an absent cargo-deny reads as a missing tool. Absent is
/// red, not skipped: a skipped policy check would still stamp the commit
/// as gated.
fn installed() -> Result<(), String> {
    match Command::new("cargo").args(["deny", "--version"]).output() {
        Ok(out) if out.status.success() => Ok(()),
        _ => Err(
            "cargo-deny is missing, and the dependency policy needs it: \
                  `cargo install --locked cargo-deny`"
                .into(),
        ),
    }
}
