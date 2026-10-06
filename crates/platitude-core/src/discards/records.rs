//! Reading Platitude GG's own record ([`RECORD_REF`], 破棄記録仕様.md §2):
//! one line of the record's reflog each, in one of two shapes (`record`
//! writes them):
//!
//! - **a copy** of thrown-away work — the stash-shaped commit itself
//!   (§2.1), its `Operation:` one of `discard`, `hunk`, `lines`,
//!   `untracked`, `reset --hard`;
//! - **a note** of anything else — a commit whose first parent is what was
//!   taken (a branch's tip, a stash entry, the commit a tag or a worktree
//!   stood on) and whose trailers say what to put back.
//!
//! A note `restored` names a part of a record already brought back, which
//! is not read as a part again (`entries`).

use std::collections::{HashMap, HashSet};
use std::path::Path;

use tokio_util::sync::CancellationToken;

use super::RECORD_REF;
use super::moves::selector;
use crate::error::GitError;
use crate::oid::Oid;
use crate::process::{GitCommand, GitExecutor};

/// One line of the record, read as written: what it says, and the commits
/// it stands on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Line {
    /// The line's value: the copy, or the note.
    pub(super) value: Oid,
    pub(super) parents: Vec<Oid>,
    /// When the line was written, epoch seconds.
    pub(super) at: i64,
    /// The `Operation:` trailer.
    pub(super) operation: String,
    /// Every trailer, in order, keys as written.
    pub(super) trailers: Vec<(String, String)>,
    /// The paths the value changes against its first parent (a copy's and
    /// a stash's own count comes from the stash, `counted`).
    pub(super) files: u32,
}

impl Line {
    /// The first value of trailer `key`, empty when there is none.
    pub(super) fn get(&self, key: &str) -> &str {
        self.trailers
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
            .unwrap_or_default()
    }

    /// Every value of trailer `key`.
    pub(super) fn all(&self, key: &str) -> Vec<&str> {
        self.trailers
            .iter()
            .filter(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
            .collect()
    }

    /// Whether this line is a copy of thrown-away work (the stash-shaped
    /// value is itself what comes back).
    pub(super) fn is_copy(&self) -> bool {
        COPY_OPERATIONS.contains(&self.operation.as_str())
    }
}

/// The `Operation:` words a copy carries (§2.1).
pub(super) const COPY_OPERATIONS: [&str; 5] =
    ["discard", "hunk", "lines", "untracked", "reset --hard"];

/// The record as read: its lines, newest first, to `limit` (the graph's
/// window, §3), and the parts `restored` notes name, by line and part
/// number.
#[derive(Debug, Default)]
pub(super) struct Read {
    pub(super) lines: Vec<Line>,
    pub(super) restored_parts: HashSet<(Oid, usize)>,
}

/// Reads the record: one `log -g` of its reflog, and one count of what the
/// stash-shaped values hold apart from their first parent where they have
/// untracked files or are a stash a note stands on (`--shortstat` counts
/// the first parent's side alone). A repository with no record reads none
/// (`--ignore-missing`).
pub(super) async fn read(
    executor: &GitExecutor,
    workdir: &Path,
    limit: Option<usize>,
    cancel: &CancellationToken,
) -> Result<Read, GitError> {
    let mut cmd = GitCommand::new().cwd(workdir).args([
        "log",
        "--walk-reflogs",
        "--ignore-missing",
        "--date=unix",
        // A copy is a merge, which `--shortstat` passes over unless told which side.
        "--diff-merges=first-parent",
        "--shortstat",
    ]);
    if let Some(limit) = limit {
        cmd = cmd.arg(format!("--max-count={limit}"));
    }
    let cmd = cmd.args([
        "--format=%x1e%H%x1f%gD%x1f%P%x1f%(trailers:only,unfold,separator=%x1d)",
        RECORD_REF,
        "--",
    ]);
    let out = executor.run(cmd, cancel).await?;
    let mut read = Read::default();
    for line in out.stdout_utf8().split('\u{1e}').filter_map(line_of) {
        if line.operation != "restored" {
            read.lines.push(line);
        } else if let (Ok(record), Ok(part)) = (
            Oid::from_hex_str(line.get("Record")),
            line.get("Part").parse::<usize>(),
        ) {
            read.restored_parts.insert((record, part));
        }
    }
    counted(executor, workdir, &mut read.lines, cancel).await?;
    Ok(read)
}

/// One line's block: its fields, then the `--shortstat` line.
fn line_of(block: &str) -> Option<Line> {
    let mut lines = block.lines().filter(|line| !line.trim().is_empty());
    let fields: Vec<&str> = lines.next()?.split('\u{1f}').collect();
    let [hex, at_selector, parents, trailers] = fields[..] else {
        return None;
    };
    let value = Oid::from_hex_str(hex).ok()?;
    let (_, at) = selector(at_selector)?;
    let trailers: Vec<(String, String)> = trailers
        .split('\u{1d}')
        .filter_map(|trailer| trailer.split_once(':'))
        .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
        .collect();
    let operation = trailers
        .iter()
        .find(|(key, _)| key == "Operation")
        .map(|(_, value)| value.clone())?;
    Some(Line {
        value,
        parents: parents
            .split(' ')
            .filter_map(|hex| Oid::from_hex_str(hex).ok())
            .collect(),
        at,
        operation,
        trailers,
        files: files_in(lines.next()),
    })
}

/// Counts what the stash-shaped commits hold that the record's own count
/// cannot see: a copy's untracked files (its third parent) and a dropped
/// stash's paths, whole (a note stands on the stash, its own diff empty) —
/// one `log --no-walk` for all of them.
async fn counted(
    executor: &GitExecutor,
    workdir: &Path,
    lines: &mut [Line],
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    // (line, commit, whether the commit's own count replaces the line's)
    let mut asked: Vec<(usize, Oid, bool)> = Vec::new();
    for (at, line) in lines.iter().enumerate() {
        if line.is_copy() {
            if let Some(untracked) = line.parents.get(2) {
                asked.push((at, *untracked, false));
            }
        } else if line.operation.starts_with("stash ")
            && let Some(stash) = line.parents.first()
        {
            asked.push((at, *stash, true));
            if let Ok(untracked) = Oid::from_hex_str(line.get("Untracked")) {
                asked.push((at, untracked, false));
            }
        }
    }
    let mut held: HashMap<Oid, u32> = HashMap::new();
    // In batches, as the walk takes its tips (`lost::TIPS_PER_WALK`): the
    // whole record can name more commits than one command line holds.
    for chunk in asked.chunks(super::lost::TIPS_PER_WALK) {
        let cmd = GitCommand::new()
            .cwd(workdir)
            .args([
                "log",
                "--no-walk",
                "--diff-merges=first-parent",
                "--shortstat",
                "--format=%x1e%H",
            ])
            .args(chunk.iter().map(|(_, commit, _)| commit.to_hex()))
            .arg("--");
        let out = executor.run(cmd, cancel).await?;
        held.extend(out.stdout_utf8().split('\u{1e}').filter_map(count_of));
    }
    for (at, commit, replaces) in asked {
        let Some(line) = lines.get_mut(at) else {
            continue;
        };
        let count = held.get(&commit).copied().unwrap_or(0);
        if replaces {
            line.files = count;
        } else {
            line.files += count;
        }
    }
    Ok(())
}

/// A commit's block of `log --shortstat`: its id, and the paths it changes.
fn count_of(block: &str) -> Option<(Oid, u32)> {
    let mut lines = block.lines().filter(|line| !line.trim().is_empty());
    let oid = Oid::from_hex_str(lines.next()?.trim()).ok()?;
    Some((oid, files_in(lines.next())))
}

/// ` 2 files changed, 3 insertions(+)`; none where nothing changed.
fn files_in(stat: Option<&str>) -> u32 {
    stat.and_then(|stat| stat.trim().split(' ').next()?.parse().ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const COPY: &str = "471ccd351a3c56a1da359cdeefe4d8e22813dace";
    const BASE: &str = "1e27d408998b7a15504415dec671a402807247ac";
    const INDEX: &str = "df8167abc262df5a953eb8711df2411dedcd0bb1";
    const UNTRACKED: &str = "bbebd226d70dcfbb96012a16e596e1e76ad638e7";

    #[test]
    fn a_line_reads_its_value_time_parents_trailers_and_paths() {
        let block = format!(
            "{COPY}\u{1f}refs/pgg/discards@{{1790976391}}\u{1f}{BASE} {INDEX} {UNTRACKED}\
             \u{1f}Operation: discard\u{1d}Worktree-path: C:/work/repo\u{1d}Branch: main\
             \u{1d}Not-copied: big.bin\u{1d}Not-copied: other.bin\
             \n\n 1 file changed, 1 insertion(+), 1 deletion(-)\n"
        );
        let line = line_of(&block).expect("a line");
        assert_eq!(
            (line.at, line.files, line.parents.len()),
            (1790976391, 1, 3)
        );
        assert!(line.is_copy());
        assert_eq!(line.get("Worktree-path"), "C:/work/repo");
        assert_eq!(line.all("Not-copied"), vec!["big.bin", "other.bin"]);
    }

    #[test]
    fn a_line_with_no_operation_is_no_record() {
        let block = format!("{COPY}\u{1f}refs/pgg/discards@{{1}}\u{1f}{BASE}\u{1f}\n");
        assert_eq!(line_of(&block), None);
    }

    #[test]
    fn the_untracked_files_count_off_their_own_block() {
        let block = format!("{UNTRACKED}\n\n 3 files changed, 3 insertions(+)\n");
        assert_eq!(
            count_of(&block),
            Some((Oid::from_hex_str(UNTRACKED).expect("oid"), 3))
        );
    }
}
