//! What the reflogs name that no ref reaches: one walk of every value the
//! moves name, and the two questions asked of it without git — which
//! commits only a tip reaches, and whether a tip is an ancestor of another.
//!
//! The ancestor question needs no git of its own: a move's old tip that a
//! ref reaches took nothing (and is not listed), so the old tip is in the
//! walk; if the new one is not, a ref reaches the new tip, and the old one,
//! which no ref reaches, cannot be its ancestor; if it is, every commit
//! between them is in the walk too (an ancestor of the new tip, so no ref
//! reaches it), and the walk answers.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::oid::Oid;
use crate::process::{GitCommand, GitExecutor};

/// How many tips one `rev-list` takes on its command line (Windows ends one
/// at 32 767 characters; 200 ids are a quarter of that).
pub(super) const TIPS_PER_WALK: usize = 200;

/// The commits `starts` reach that HEAD, the branches, the remote-tracking
/// branches, the tags, the stash and `held` (the other worktrees' detached
/// HEADs) do not, each with its parents, in `rev-list` order. A start gc
/// has taken is passed over (`--ignore-missing`).
pub(super) async fn unreached(
    executor: &GitExecutor,
    cwd: &Path,
    starts: &[Oid],
    held: &[Oid],
    cancel: &CancellationToken,
) -> Result<Walked, GitError> {
    let mut walked = Walked::default();
    for chunk in starts.chunks(TIPS_PER_WALK) {
        let cmd = GitCommand::new()
            .cwd(cwd)
            .args(["rev-list", "--ignore-missing", "--parents"])
            .args(chunk.iter().map(Oid::to_hex))
            .args([
                "--not",
                "HEAD",
                "--branches",
                "--remotes",
                "--tags",
                "--glob=refs/stash*",
            ])
            .args(held.iter().map(Oid::to_hex))
            .arg("--");
        let found = executor.run(cmd, cancel).await?;
        for line in found.stdout_utf8().lines() {
            let mut oids = line
                .split(' ')
                .filter_map(|hex| Oid::from_hex_str(hex).ok());
            if let Some(commit) = oids.next() {
                walked.add(commit, oids.collect());
            }
        }
    }
    Ok(walked)
}

/// The walk: its commits in order, and each one's parents.
#[derive(Debug, Default)]
pub(super) struct Walked {
    order: Vec<Oid>,
    parents: HashMap<Oid, Vec<Oid>>,
}

impl Walked {
    pub(super) fn add(&mut self, commit: Oid, parents: Vec<Oid>) {
        if self.parents.insert(commit, parents).is_none() {
            self.order.push(commit);
        }
    }

    pub(super) fn holds(&self, oid: &Oid) -> bool {
        self.parents.contains_key(oid)
    }

    /// The commits of the walk `tip` reaches, in the walk's order.
    pub(super) fn only_from(&self, tip: &Oid) -> Vec<Oid> {
        let reached = self.reached(tip);
        self.order
            .iter()
            .copied()
            .filter(|oid| reached.contains(oid))
            .collect()
    }

    /// Whether `from` reaches `to` inside the walk.
    pub(super) fn reaches(&self, from: &Oid, to: &Oid) -> bool {
        self.reached(from).contains(to)
    }

    fn reached(&self, tip: &Oid) -> HashSet<Oid> {
        let mut reached = HashSet::new();
        let mut stack = vec![*tip];
        while let Some(oid) = stack.pop() {
            let Some(parents) = self.parents.get(&oid) else {
                continue;
            };
            if reached.insert(oid) {
                stack.extend(parents.iter().copied());
            }
        }
        reached
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(byte: u8) -> Oid {
        Oid::from_hex_str(&format!("{byte:02x}").repeat(20)).expect("test oid")
    }

    /// 3 → 2 → 1, and 5 → 4 on its own.
    fn walked() -> Walked {
        let mut walked = Walked::default();
        walked.add(oid(3), vec![oid(2)]);
        walked.add(oid(5), vec![oid(4)]);
        walked.add(oid(2), vec![oid(1)]);
        walked.add(oid(4), vec![oid(9)]);
        walked.add(oid(1), vec![]);
        walked
    }

    #[test]
    fn only_what_the_tip_reaches_is_its_own_in_the_walks_order() {
        assert_eq!(walked().only_from(&oid(2)), vec![oid(2), oid(1)]);
        assert_eq!(walked().only_from(&oid(5)), vec![oid(5), oid(4)]);
        assert!(walked().only_from(&oid(9)).is_empty(), "outside the walk");
    }

    #[test]
    fn reaching_stays_inside_the_walk() {
        assert!(walked().reaches(&oid(3), &oid(1)));
        assert!(!walked().reaches(&oid(1), &oid(3)));
        assert!(
            !walked().reaches(&oid(5), &oid(9)),
            "a parent outside the walk is not reached"
        );
    }
}
