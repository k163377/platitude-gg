//! The reading as a whole: the worktrees, the record and the names
//! taken side by side, then one walk of every reflog, then one walk of what
//! they and the record name that no ref reaches.

use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use tokio_util::sync::CancellationToken;

use super::entries::{Here, Settled, Taken, entries};
use super::lost::{Walked, unreached};
use super::moves::{Line, Move, branch_moves, head_moves, parse_lines};
use super::records;
use super::{Discard, Look, Restore};
use crate::error::GitError;
use crate::oid::Oid;
use crate::process::{GitCommand, GitExecutor};
use crate::worktrees::WorktreeEntry;

/// A worktree whose HEAD reflog is read.
struct Worktree {
    /// [`super::MAIN_WORKTREE`], or its name under `$GIT_DIR/worktrees/` — what
    /// the record's `Worktree:` names it by — and where it stands now.
    id: String,
    path: String,
    /// The ref that names its HEAD from the worktree the session stands in.
    head: String,
    /// The branch it has out; `None` while detached.
    on: Option<String>,
    folder: String,
    /// How an entry names it: empty for the worktree the session stands in.
    label: String,
    /// The commit a detached HEAD holds, which no branch may.
    detached_at: Option<Oid>,
}

/// What destructive operations took away in the repository `workdir`
/// stands in, newest first: every move off a tip that left commits no ref
/// reaches, read off each reflog's newest `limit` lines (the graph's
/// window, §3; `None` reads them whole), and the record's lines to the same
/// depth — each with the parts a restore brings back (§4).
pub async fn read_repo(
    executor: &GitExecutor,
    workdir: &Path,
    limit: Option<usize>,
    cancel: &CancellationToken,
) -> Result<Vec<Discard>, GitError> {
    let (listed, recorded, taken, stashed) = tokio::try_join!(
        crate::worktrees::load(executor, workdir, cancel),
        records::read(executor, workdir, limit, cancel),
        taken_names(executor, workdir, cancel),
        stash_entries(executor, workdir, cancel),
    )?;
    let worktrees = worktrees_of(listed, workdir);
    let mut reflogs = reflogs(executor, workdir, &worktrees, cancel).await?;
    if let Some(limit) = limit {
        reflogs.values_mut().for_each(|lines| lines.truncate(limit));
    }
    let mut by_head = Vec::new();
    for worktree in &worktrees {
        if let Some(lines) = reflogs.remove(&worktree.head) {
            by_head.extend(head_moves(
                &worktree.folder,
                &worktree.label,
                worktree.on.as_deref(),
                &lines,
            ));
        }
    }
    // What is left is the branches'.
    let by_branch: Vec<Move> = reflogs
        .iter()
        .flat_map(|(branch, lines)| branch_moves(branch, lines))
        .collect();
    let mut named = HashSet::new();
    let starts: Vec<Oid> = by_branch
        .iter()
        .chain(&by_head)
        .flat_map(|found| [found.old, found.new])
        .chain(recorded.lines.iter().flat_map(record_tips))
        .filter(|oid| named.insert(*oid))
        .collect();
    // Every stash entry holds what it stood on, not only the newest one
    // `refs/stash` names (§3).
    let held: Vec<Oid> = worktrees
        .iter()
        .filter_map(|worktree| worktree.detached_at)
        .chain(stashed)
        .collect();
    let walked = unreached(executor, workdir, &starts, &held, cancel).await?;
    let settled = settle(by_branch, by_head, &walked);
    let here = Here {
        workdir: std::fs::canonicalize(workdir).ok(),
        worktrees: worktrees
            .iter()
            .map(|worktree| (worktree.id.clone(), worktree.path.clone()))
            .collect(),
        walked: &walked,
        taken: &taken,
        restored: &recorded.restored_parts,
    };
    let mut found = entries(settled, &recorded.lines, &here);
    mark_absent(executor, workdir, &mut found, cancel).await?;
    name_worktrees(executor, &mut found, cancel).await?;
    found.sort_by_key(|d| Reverse(d.at));
    Ok(found)
}

/// Names each worktree a part puts work back into by its git directory
/// as a session opened there names it ([`super::restore::worktree_at`]): the key
/// of that worktree's write order, which the restore takes its turn in (§4).
/// Once per worktree; a worktree gone — a folder left where it stood included — is
/// named by nothing.
async fn name_worktrees(
    executor: &GitExecutor,
    found: &mut [Discard],
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let mut named: HashMap<String, String> = HashMap::new();
    for part in found.iter_mut().flat_map(|entry| entry.parts.iter_mut()) {
        let Restore::Changes { path, git_dir, .. } = &mut part.restore else {
            continue;
        };
        if let Some(known) = named.get(path.as_str()) {
            git_dir.clone_from(known);
            continue;
        }
        let dir = super::restore::worktree_at(executor, path, cancel)
            .await?
            .unwrap_or_default();
        named.insert(path.clone(), dir.clone());
        *git_dir = dir;
    }
    Ok(())
}

/// The commit of every stash entry, the oldest included.
async fn stash_entries(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Vec<Oid>, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["stash", "list", "--format=%H"]);
    let out = executor.run(cmd, cancel).await?;
    Ok(out
        .stdout_utf8()
        .lines()
        .filter_map(|line| Oid::from_hex_str(line.trim()).ok())
        .collect())
}

/// Takes every part whose tip is no commit this repository has off the
/// graph (`Look::Absent`): a remote's tag never fetched, a commit gc took.
/// No walk can draw it, so nothing waits for one to — its restore is git's
/// to refuse (§4). One `rev-list --no-walk` per batch of tips, which names
/// the commits it finds and passes over what is missing.
async fn mark_absent(
    executor: &GitExecutor,
    workdir: &Path,
    found: &mut [Discard],
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let mut asked: Vec<Oid> = found
        .iter()
        .flat_map(|entry| &entry.parts)
        .filter(|part| part.look != Look::Uncommitted)
        .map(|part| part.tip)
        .collect();
    asked.sort_unstable();
    asked.dedup();
    let mut present = HashSet::new();
    for chunk in asked.chunks(super::lost::TIPS_PER_WALK) {
        let cmd = GitCommand::new()
            .cwd(workdir)
            .args(["rev-list", "--no-walk", "--ignore-missing"])
            .args(chunk.iter().map(Oid::to_hex))
            .arg("--");
        let out = executor.run(cmd, cancel).await?;
        present.extend(
            out.stdout_utf8()
                .lines()
                .filter_map(|line| Oid::from_hex_str(line.trim()).ok()),
        );
    }
    for part in found.iter_mut().flat_map(|entry| entry.parts.iter_mut()) {
        if part.look != Look::Uncommitted && !present.contains(&part.tip) {
            part.look = Look::Absent;
        }
    }
    Ok(())
}

/// The commits a note's restore would stand a name on, which the walk
/// counts what only they reach from: what it was taken off, and a remote's
/// own tip where it was another.
fn record_tips(line: &records::Line) -> Vec<Oid> {
    if line.is_copy() || line.operation.starts_with("stash ") {
        return Vec::new();
    }
    let mut tips: Vec<Oid> = line.parents.first().copied().into_iter().collect();
    tips.extend(Oid::from_hex_str(line.get("Remote-tip")).ok());
    tips
}

/// The branch and tag names the repository has, which a restore's name
/// keeps clear of (§4).
async fn taken_names(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Taken, GitError> {
    let cmd = GitCommand::new().cwd(workdir).args([
        "for-each-ref",
        "--format=%(refname)",
        "refs/heads",
        "refs/tags",
    ]);
    let out = executor.run(cmd, cancel).await?;
    let mut taken = Taken::default();
    for name in out.stdout_utf8().lines() {
        if let Some(branch) = name.strip_prefix("refs/heads/") {
            taken.branches.insert(branch.to_string());
        } else if let Some(tag) = name.strip_prefix("refs/tags/") {
            taken.tags.insert(tag.to_string());
        }
    }
    Ok(taken)
}

/// The worktrees with a HEAD reflog to read: not bare, the folder still
/// there. The one `workdir` stands in reads as `HEAD`, the others by the
/// names git gives every worktree's HEAD (`main-worktree/HEAD`,
/// `worktrees/<id>/HEAD` — git-worktree(1) §REFS).
fn worktrees_of(listed: Vec<WorktreeEntry>, workdir: &Path) -> Vec<Worktree> {
    let here = std::fs::canonicalize(workdir).ok();
    listed
        .into_iter()
        .filter(|entry| !entry.bare && !entry.prunable)
        .filter_map(|entry| {
            let path = PathBuf::from(&entry.path);
            let folder = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            let is_here = here.is_some() && std::fs::canonicalize(&path).ok() == here;
            let id = if entry.main {
                super::MAIN_WORKTREE.to_string()
            } else {
                linked_id(&path)?
            };
            let head = if is_here {
                "HEAD".to_string()
            } else if entry.main {
                "main-worktree/HEAD".to_string()
            } else {
                format!("worktrees/{id}/HEAD")
            };
            let detached_at = entry
                .head_hex
                .as_deref()
                .filter(|_| entry.detached)
                .and_then(|hex| Oid::from_hex_str(hex).ok());
            Some(Worktree {
                id,
                path: entry.path,
                head,
                on: entry.branch,
                label: if is_here {
                    String::new()
                } else {
                    folder.clone()
                },
                folder,
                detached_at,
            })
        })
        .collect()
}

/// A linked worktree's name under `$GIT_DIR/worktrees/`: the last part of the
/// `gitdir:` its `.git` file points to (gitrepository-layout(5)).
fn linked_id(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path.join(".git")).ok()?;
    let gitdir = text.trim().strip_prefix("gitdir:")?.trim();
    Some(
        Path::new(gitdir)
            .file_name()?
            .to_string_lossy()
            .into_owned(),
    )
}

/// Every branch's reflog and every worktree's HEAD reflog in one `log -g`, by
/// ref: each ref's lines newest first. A ref with no reflog has no lines;
/// an unborn HEAD is passed over (`--ignore-missing`).
async fn reflogs(
    executor: &GitExecutor,
    workdir: &Path,
    worktrees: &[Worktree],
    cancel: &CancellationToken,
) -> Result<BTreeMap<String, Vec<Line>>, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args([
            "log",
            "--walk-reflogs",
            "--ignore-missing",
            "--date=unix",
            "--format=%H%x1f%gD%x1f%gs",
            "--branches",
        ])
        .args(worktrees.iter().map(|worktree| worktree.head.as_str()))
        .arg("--");
    let out = executor.run(cmd, cancel).await?;
    let mut by_ref: BTreeMap<String, Vec<Line>> = BTreeMap::new();
    for (reference, line) in parse_lines(&out.stdout_utf8()) {
        by_ref.entry(reference).or_default().push(line);
    }
    Ok(by_ref)
}

/// The moves that left commits no ref reaches, newest first, with what only
/// the old tip reaches: each branch's move once, its own line before
/// HEAD's — HEAD holds the same move under the same name and old tip, or
/// the same old and new tips where the branch was renamed since. Two
/// branches moved off one tip are two moves (one rebase moving both with
/// `--update-refs`). A move onto a commit the old tip is an ancestor of took
/// nothing: what it left is the later move's.
fn settle(mut by_branch: Vec<Move>, mut by_head: Vec<Move>, walked: &Walked) -> Vec<Settled> {
    by_branch.sort_by_key(|found| Reverse(found.at));
    by_head.sort_by_key(|found| Reverse(found.at));
    let took = |found: &Move| walked.holds(&found.old) && !walked.reaches(&found.new, &found.old);
    let mut named = HashSet::new();
    let mut spans = HashSet::new();
    let mut kept: Vec<Move> = Vec::new();
    for found in by_branch.into_iter().filter(took) {
        if named.insert((found.name.clone(), found.old)) {
            spans.insert((found.old, found.new));
            kept.push(found);
        }
    }
    for found in by_head.into_iter().filter(took) {
        if !spans.contains(&(found.old, found.new)) && named.insert((found.name.clone(), found.old))
        {
            kept.push(found);
        }
    }
    kept.into_iter()
        .map(|found| Settled {
            lost: walked.only_from(&found.old),
            found,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discards::DiscardKind;

    fn oid(byte: u8) -> Oid {
        Oid::from_hex_str(&format!("{byte:02x}").repeat(20)).expect("test oid")
    }

    fn moved(kind: DiscardKind, name: &str, at: i64, old: u8, new: u8) -> Move {
        Move {
            kind,
            name: name.to_string(),
            worktree: String::new(),
            at,
            old: oid(old),
            new: oid(new),
            detached: false,
        }
    }

    /// 3 → 2 → 1 nobody reaches; 5 → 4 the same, 4 forked off a reached 9.
    fn walked() -> Walked {
        let mut walked = Walked::default();
        walked.add(oid(3), vec![oid(2)]);
        walked.add(oid(2), vec![oid(1)]);
        walked.add(oid(1), vec![oid(9)]);
        walked.add(oid(5), vec![oid(4)]);
        walked.add(oid(4), vec![oid(9)]);
        walked
    }

    #[test]
    fn the_same_move_in_a_branch_and_head_is_the_branchs_entry() {
        let found = settle(
            vec![moved(DiscardKind::Reset, "main", 20, 3, 9)],
            vec![
                moved(DiscardKind::Reset, "main", 20, 3, 9),
                moved(DiscardKind::Amend, "gone", 10, 5, 9),
            ],
            &walked(),
        );
        let seen: Vec<(&str, usize)> = found
            .iter()
            .map(|d| (d.found.name.as_str(), d.lost.len()))
            .collect();
        assert_eq!(seen, vec![("main", 3), ("gone", 2)]);
    }

    #[test]
    fn a_move_off_a_reached_tip_or_onto_its_own_descendant_is_not_an_entry() {
        let found = settle(
            vec![
                moved(DiscardKind::Reset, "main", 30, 9, 1),
                moved(DiscardKind::Reset, "main", 20, 1, 3),
            ],
            Vec::new(),
            &walked(),
        );
        assert!(found.is_empty());
    }
}
