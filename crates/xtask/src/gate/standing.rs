//! Where a seat's gate stands, said by the Stop hook.

use std::path::Path;

use super::stamp::Store;

/// A seat ahead of main whose tip carries no full stamp is work reported
/// before it was gated — said as a system message, so a turn that ends
/// mid-work is not held to a gate it was never claiming.
pub(crate) fn standing(cwd: &str) -> Option<String> {
    let root = crate::seats::worktree_root(cwd)?;
    let head = crate::subprocess::git_query(&root, &["rev-parse", "HEAD"])?;
    let ahead = crate::seats::commits_in(&root, "main..HEAD")?;
    if ahead == 0 {
        return None;
    }
    let store = Store::open(Path::new(&root)).ok()?;
    let seat = root.rsplit('/').next().unwrap_or_default();
    Some(match store.commit(&head) {
        None => format!(
            "gate: seat {seat} is {ahead} commit(s) ahead of main and its tip is not gated — \
             `cargo xtask gate` before 「マージ可」."
        ),
        Some(found) if !found.full => format!(
            "gate: seat {seat} is {ahead} commit(s) ahead of main; its tip is gated on the host \
             side only — the full `cargo xtask gate` still owes the container side."
        ),
        Some(found) if !found.onto_main => format!(
            "gate: seat {seat} is {ahead} commit(s) ahead of main; its tip was gated off main — \
             `land` will rebase and gate again."
        ),
        Some(_) => {
            format!("gate: seat {seat} is {ahead} commit(s) ahead of main, tip gated in full.")
        }
    })
}
