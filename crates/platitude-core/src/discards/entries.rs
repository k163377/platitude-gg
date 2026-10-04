//! The entries, made of what the reading found: the moves git's reflogs
//! hold that left commits nothing reaches, and the record's lines — each
//! made into the parts a restore brings back (破棄記録仕様.md §4), named as
//! they would be now.
//!
//! A line that says which of git's own lines it goes with (§2) takes them
//! in: a `reset --hard`'s copy joins the reset that threw the work away,
//! and a rebase that moved branches together is one entry.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::lost::Walked;
use super::moves::Move;
use super::records::Line;
use super::{Discard, DiscardKind, Look, Part, Restore, Stands, free_name, suffixed};
use crate::oid::Oid;

/// A move that left commits nothing reaches, with those commits.
pub(super) struct Settled {
    pub(super) found: Move,
    pub(super) lost: Vec<Oid>,
}

/// The names already taken, which a restore's name keeps clear of (§4).
#[derive(Debug, Default)]
pub(super) struct Taken {
    pub(super) branches: HashSet<String>,
    pub(super) tags: HashSet<String>,
}

/// Where the session reads from, so an entry from another working copy
/// says which.
pub(super) struct Here<'a> {
    pub(super) workdir: Option<PathBuf>,
    /// Where each working copy stands now, by what the record's `Worktree:`
    /// names it ([`super::MAIN_COPY`], or its name under
    /// `$GIT_DIR/worktrees/`).
    pub(super) copies: HashMap<String, String>,
    pub(super) walked: &'a Walked,
    pub(super) taken: &'a Taken,
    /// `(record line, part)` a `restored` note names.
    pub(super) restored: &'a HashSet<(Oid, usize)>,
}

/// Every entry, the record's lines first in their order and then the moves
/// none of them took in.
pub(super) fn entries(moves: Vec<Settled>, lines: &[Line], here: &Here<'_>) -> Vec<Discard> {
    let mut moves: Vec<Option<Settled>> = moves.into_iter().map(Some).collect();
    let mut out = Vec::new();
    for line in lines {
        if let Some(entry) = from_line(line, &mut moves, here) {
            out.push(entry);
        }
    }
    for settled in moves.into_iter().flatten() {
        let mut names = Names::new(here.taken);
        let part = branch_part(&settled, &mut names);
        out.push(Discard {
            kind: settled.found.kind,
            name: settled.found.name,
            remote: String::new(),
            copy: settled.found.copy,
            at: settled.found.at,
            parts: vec![part],
        });
    }
    out
}

/// One record line's entry; `None` for a line every part of which is back,
/// a group none of whose moves is still listed, a line missing what it
/// stands on, or a word this build does not know.
fn from_line(line: &Line, moves: &mut [Option<Settled>], here: &Here<'_>) -> Option<Discard> {
    let mut entry = Discard {
        kind: DiscardKind::Reset,
        name: String::new(),
        remote: line.get("Remote").to_string(),
        copy: String::new(),
        at: line.at,
        parts: Vec::new(),
    };
    let mut names = Names::new(here.taken);
    let read = Reading {
        line,
        here,
        first: line.parents.first().copied(),
    };
    match line.operation.as_str() {
        "rebase" => read.group(moves, &mut entry, &mut names),
        "discard" | "hunk" | "lines" | "untracked" | "reset --hard" => {
            read.work(moves, &mut entry, &mut names)
        }
        "delete branch" => read.deleted_branch(&mut entry, &mut names)?,
        "delete remote branch" | "force push" => read.remote_branch(&mut entry, &mut names)?,
        "delete tag" | "delete remote tag" | "force push tag" => read.tags(&mut entry, &mut names),
        "remove worktree" => read.worktree(&mut entry)?,
        "stash drop" | "stash pop" => read.stash(&mut entry)?,
        _ => return None,
    }
    entry
        .parts
        .retain(|part| part.record.is_none_or(|at| !here.restored.contains(&at)));
    (!entry.parts.is_empty()).then_some(entry)
}

/// One record line being read into its entry: what it says, and the
/// commit it stands on (its first parent), where it has one.
struct Reading<'a> {
    line: &'a Line,
    here: &'a Here<'a>,
    first: Option<Oid>,
}

impl Reading<'_> {
    /// The branches one rebase moved together: the moves it names, taken
    /// out of the reflogs' list, one part each — two branches that stood on
    /// one tip are two parts.
    fn group(&self, moves: &mut [Option<Settled>], entry: &mut Discard, names: &mut Names<'_>) {
        let named: Vec<(&str, Oid, Oid)> = self
            .line
            .all("Moved")
            .into_iter()
            .filter_map(moved_of)
            .collect();
        let taken: Vec<Settled> = take_moves(moves, &named);
        entry.kind = DiscardKind::Rebase;
        entry.name = taken
            .iter()
            .map(|settled| settled.found.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        entry.parts = taken
            .iter()
            .map(|settled| branch_part(settled, names))
            .collect();
    }

    /// A copy of thrown-away work — after the reset it went with, where it
    /// names one that is still listed.
    fn work(&self, moves: &mut [Option<Settled>], entry: &mut Discard, names: &mut Names<'_>) {
        let line = self.line;
        entry.kind = match line.operation.as_str() {
            "untracked" => DiscardKind::DeletedUntracked,
            "reset --hard" => DiscardKind::Reset,
            _ => DiscardKind::Discarded,
        };
        entry.name = line.get("Branch").to_string();
        // The copy where it stands now — the one the record names, moved
        // since or not — else where the record says it was.
        let path = self
            .here
            .copies
            .get(line.get("Worktree"))
            .map_or_else(|| line.get("Working-copy").to_string(), Clone::clone);
        entry.copy = copy_label(&path, self.here);
        let named: Vec<(&str, Oid, Oid)> =
            line.all("Moved").into_iter().filter_map(moved_of).collect();
        let reset = take_moves(moves, &named).into_iter().next();
        if let Some(reset) = reset {
            entry.name = reset.found.name.clone();
            entry.copy = reset.found.copy.clone();
            entry.parts.push(branch_part(&reset, names));
        }
        entry.parts.push(Part {
            restore: Restore::Changes {
                copy: line.value,
                path,
                // Named once the list is read whole (`read::name_copies`).
                git_dir: String::new(),
            },
            tip: line.value,
            look: Look::Uncommitted,
            lost: vec![line.value],
            files: line.files,
            not_copied: line
                .all("Not-copied")
                .into_iter()
                .map(str::to_string)
                .collect(),
            remote: String::new(),
            record: Some((line.value, 0)),
            stands: Stands::of(&line.parents, line.get("Stands-on")),
        });
    }

    /// A branch deleted here with its upstream — and the remote's own tip,
    /// where a delete that reached it found another.
    fn deleted_branch(&self, entry: &mut Discard, names: &mut Names<'_>) -> Option<()> {
        let line = self.line;
        entry.kind = DiscardKind::Deleted;
        entry.name = line.get("Branch").to_string();
        let upstream = line
            .get("Upstream")
            .split_once(' ')
            .map(|(remote, branch)| (remote.to_string(), branch.to_string()));
        let own = Recorded {
            line,
            part: 0,
            name: line.get("Branch"),
            remote: "",
        };
        entry
            .parts
            .push(own.branch(self.first?, upstream, names, self.here));
        if let Ok(tip) = Oid::from_hex_str(line.get("Remote-tip")) {
            let theirs = Recorded {
                line,
                part: 1,
                name: line.get("Remote-branch"),
                remote: line.get("Remote"),
            };
            entry.parts.push(theirs.branch(tip, None, names, self.here));
        }
        Some(())
    }

    /// A remote's branch deleted, or pushed over: its tip as a branch here.
    fn remote_branch(&self, entry: &mut Discard, names: &mut Names<'_>) -> Option<()> {
        let line = self.line;
        entry.kind = if line.operation == "force push" {
            DiscardKind::ForcePushed
        } else {
            DiscardKind::DeletedRemoteBranch
        };
        entry.name = line.get("Branch").to_string();
        let theirs = Recorded {
            line,
            part: 0,
            name: line.get("Branch"),
            remote: line.get("Remote"),
        };
        entry
            .parts
            .push(theirs.branch(self.first?, None, names, self.here));
        Some(())
    }

    /// A tag deleted or pushed over: its object here — and the remote's
    /// own, where a delete that reached it found another.
    fn tags(&self, entry: &mut Discard, names: &mut Names<'_>) {
        let line = self.line;
        entry.kind = match line.operation.as_str() {
            "delete tag" => DiscardKind::DeletedTag,
            "delete remote tag" => DiscardKind::DeletedRemoteTag,
            _ => DiscardKind::ForcePushedTag,
        };
        entry.name = line.get("Tag").to_string();
        let objects = [line.get("Object"), line.get("Remote-object")];
        for (part, object) in objects.into_iter().enumerate() {
            let Ok(object) = Oid::from_hex_str(object) else {
                continue;
            };
            let name = names.tag(&entry.name);
            // Where it stands on the graph: the commit it peels to — the
            // note's parent here, `Remote-commit:` for the remote's own. A tag
            // whose commit this repository never had stands nowhere; the
            // restore is git's to refuse (§4, `Look::Absent`).
            let tip = if part == 0 {
                self.first.unwrap_or(object)
            } else {
                Oid::from_hex_str(line.get("Remote-commit")).unwrap_or(object)
            };
            let theirs = part == 1 || entry.kind != DiscardKind::DeletedTag;
            entry.parts.push(Part {
                restore: Restore::Tag { name, object },
                tip,
                look: Look::Commit,
                lost: self.here.walked.only_from(&tip),
                files: 0,
                not_copied: Vec::new(),
                remote: if theirs {
                    entry.remote.clone()
                } else {
                    String::new()
                },
                record: Some((line.value, part)),
                stands: None,
            });
        }
    }

    /// A working copy removed: back where it was, on what it had out.
    fn worktree(&self, entry: &mut Discard) -> Option<()> {
        let line = self.line;
        entry.kind = DiscardKind::RemovedWorktree;
        let path = line.get("Working-copy");
        entry.name = folder_of(path);
        let head = self.first?;
        let free = free_name(path, &|candidate: &str| Path::new(candidate).exists());
        // On the branch it had out while that branch stands; gone, git would
        // make one of its name off a remote's (`worktree add` guesses), so
        // the copy comes back detached on the commit it stood on.
        let branch = Some(line.get("Branch"))
            .filter(|branch| self.here.taken.branches.contains(*branch))
            .map(str::to_string);
        entry.parts.push(Part {
            restore: Restore::Worktree {
                path: free,
                branch,
                head,
            },
            tip: head,
            look: Look::Commit,
            lost: self.here.walked.only_from(&head),
            files: 0,
            not_copied: Vec::new(),
            remote: String::new(),
            record: Some((line.value, 0)),
            stands: None,
        });
        Some(())
    }

    /// A stash dropped, or popped: the entry itself, back in the list.
    fn stash(&self, entry: &mut Discard) -> Option<()> {
        let line = self.line;
        entry.kind = if line.operation == "stash pop" {
            DiscardKind::PoppedStash
        } else {
            DiscardKind::DroppedStash
        };
        entry.name = line.get("Message").to_string();
        let stash = self.first?;
        entry.parts.push(Part {
            restore: Restore::Stash {
                commit: stash,
                message: entry.name.clone(),
            },
            tip: stash,
            look: Look::Stash,
            lost: vec![stash],
            files: line.files,
            not_copied: Vec::new(),
            remote: String::new(),
            record: Some((line.value, 0)),
            stands: None,
        });
        Some(())
    }
}

/// A branch put back at a move's old tip: under its own name where that is
/// free — a branch another tool deleted, one renamed since — else with the
/// suffix; what a detached HEAD held under `detached-pgg-restored` (§4).
fn branch_part(settled: &Settled, names: &mut Names<'_>) -> Part {
    let name = if settled.found.detached {
        let free = suffixed("detached", &|candidate| names.taken_branch(candidate));
        names.claim(free)
    } else {
        names.branch(&settled.found.name)
    };
    Part {
        restore: Restore::Branch {
            name,
            tip: settled.found.old,
            upstream: None,
        },
        tip: settled.found.old,
        look: Look::Commit,
        lost: settled.lost.clone(),
        files: 0,
        not_copied: Vec::new(),
        remote: String::new(),
        record: None,
        stands: None,
    }
}

/// A branch the record names: part `part` of `line`, under `name` — taken
/// from `remote` where it was a remote's.
struct Recorded<'l> {
    line: &'l Line,
    part: usize,
    name: &'l str,
    remote: &'l str,
}

impl Recorded<'_> {
    fn branch(
        &self,
        tip: Oid,
        upstream: Option<(String, String)>,
        names: &mut Names<'_>,
        here: &Here<'_>,
    ) -> Part {
        Part {
            restore: Restore::Branch {
                name: names.branch(self.name),
                tip,
                upstream,
            },
            tip,
            look: Look::Commit,
            lost: here.walked.only_from(&tip),
            files: 0,
            not_copied: Vec::new(),
            remote: self.remote.to_string(),
            record: Some((self.line.value, self.part)),
            stands: None,
        }
    }
}

/// The names one entry's parts take: what the repository has, and what an
/// earlier part of the same entry took — a restore of the whole entry makes
/// them one after another.
struct Names<'a> {
    taken: &'a Taken,
    claimed: HashSet<String>,
}

impl<'a> Names<'a> {
    fn new(taken: &'a Taken) -> Self {
        Self {
            taken,
            claimed: HashSet::new(),
        }
    }

    fn taken_branch(&self, name: &str) -> bool {
        clashes(&self.taken.branches, name) || clashes(&self.claimed, name)
    }

    fn branch(&mut self, name: &str) -> String {
        let base = clear_of_folders(&[&self.taken.branches, &self.claimed], name);
        let free = free_name(&base, &|candidate| self.taken_branch(candidate));
        self.claim(free)
    }

    fn tag(&mut self, name: &str) -> String {
        let base = clear_of_folders(&[&self.taken.tags, &self.claimed], name);
        let free = free_name(&base, &|candidate| {
            clashes(&self.taken.tags, candidate) || clashes(&self.claimed, candidate)
        });
        self.claim(free)
    }

    fn claim(&mut self, name: String) -> String {
        self.claimed.insert(name.clone());
        name
    }
}

/// Whether `name` cannot be made beside `names` (one namespace): the same
/// name in any case — a loose ref on a case-folding disk is one file — or
/// one a folder of the other, which git refuses as a ref directory/file
/// conflict (`release` beside `release/1.0`).
fn clashes(names: &HashSet<String>, name: &str) -> bool {
    let name = name.to_lowercase();
    names.iter().any(|taken| {
        let taken = taken.to_lowercase();
        taken == name
            || taken
                .strip_prefix(name.as_str())
                .is_some_and(|rest| rest.starts_with('/'))
            || name
                .strip_prefix(taken.as_str())
                .is_some_and(|rest| rest.starts_with('/'))
    })
}

/// `name` with its folders made dashes where one of them is a name already
/// (`feature/x` beside a branch `feature`): under that folder no suffix is
/// ever free.
fn clear_of_folders(sets: &[&HashSet<String>], name: &str) -> String {
    let blocked = name.match_indices('/').any(|(at, _)| {
        let folder = name[..at].to_lowercase();
        sets.iter()
            .any(|set| set.iter().any(|taken| taken.to_lowercase() == folder))
    });
    if blocked {
        name.replace('/', "-")
    } else {
        name.to_string()
    }
}

/// The listed moves `named` (`Moved:`'s ref, old tip and new) stand for,
/// taken out of the list in their order so no other entry has them: by the
/// ref's name first, so two branches moved off one tip onto one tip each
/// find their own; then by the tips alone, for a branch renamed since.
fn take_moves(moves: &mut [Option<Settled>], named: &[(&str, Oid, Oid)]) -> Vec<Settled> {
    let mut found: Vec<Option<Settled>> = named
        .iter()
        .map(|(name, old, new)| take_move(moves, |m| m.name == *name, old, new))
        .collect();
    for (slot, (_, old, new)) in found.iter_mut().zip(named) {
        if slot.is_none() {
            *slot = take_move(moves, |_| true, old, new);
        }
    }
    found.into_iter().flatten().collect()
}

fn take_move(
    moves: &mut [Option<Settled>],
    fits: impl Fn(&Move) -> bool,
    old: &Oid,
    new: &Oid,
) -> Option<Settled> {
    moves
        .iter_mut()
        .find(|slot| {
            slot.as_ref().is_some_and(|settled| {
                settled.found.old == *old && settled.found.new == *new && fits(&settled.found)
            })
        })?
        .take()
}

/// `Moved:`'s value: the ref, its old tip, its new.
fn moved_of(value: &str) -> Option<(&str, Oid, Oid)> {
    let mut fields = value.split(' ');
    let reference = fields.next()?;
    let old = Oid::from_hex_str(fields.next()?).ok()?;
    let new = Oid::from_hex_str(fields.next()?).ok()?;
    Some((reference, old, new))
}

/// How an entry names the working copy at `path`: its folder, or nothing
/// for the one the session reads from.
fn copy_label(path: &str, here: &Here<'_>) -> String {
    let canonical = std::fs::canonicalize(path).ok();
    if canonical.is_some() && canonical == here.workdir {
        String::new()
    } else {
        folder_of(path)
    }
}

fn folder_of(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(byte: u8) -> Oid {
        Oid::from_hex_str(&format!("{byte:02x}").repeat(20)).expect("test oid")
    }

    fn line(operation: &str, value: u8, parents: &[u8], trailers: &[(&str, &str)]) -> Line {
        let mut all = vec![("Operation".to_string(), operation.to_string())];
        all.extend(
            trailers
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string())),
        );
        Line {
            value: oid(value),
            parents: parents.iter().map(|b| oid(*b)).collect(),
            at: 100,
            operation: operation.to_string(),
            trailers: all,
            files: 2,
        }
    }

    fn settled(kind: DiscardKind, name: &str, old: u8, new: u8) -> Settled {
        Settled {
            found: Move {
                kind,
                name: name.to_string(),
                copy: String::new(),
                at: 90,
                old: oid(old),
                new: oid(new),
                detached: kind == DiscardKind::LeftDetached,
            },
            lost: vec![oid(old)],
        }
    }

    fn here<'a>(
        walked: &'a Walked,
        taken: &'a Taken,
        restored: &'a HashSet<(Oid, usize)>,
    ) -> Here<'a> {
        Here {
            workdir: None,
            copies: HashMap::new(),
            walked,
            taken,
            restored,
        }
    }

    #[test]
    fn a_reset_hards_copy_joins_its_reset_and_the_branch_comes_first() {
        let walked = Walked::default();
        let taken = Taken {
            branches: ["main".to_string()].into(),
            ..Taken::default()
        };
        let restored = HashSet::new();
        let moved = format!("main {} {}", oid(3).to_hex(), oid(9).to_hex());
        let copy = line(
            "reset --hard",
            7,
            &[3, 6],
            &[("Branch", "main"), ("Moved", &moved)],
        );
        let found = entries(
            vec![settled(DiscardKind::Reset, "main", 3, 9)],
            &[copy],
            &here(&walked, &taken, &restored),
        );
        assert_eq!(found.len(), 1, "{found:#?}");
        let restores: Vec<&Restore> = found[0].parts.iter().map(|part| &part.restore).collect();
        assert!(
            matches!(restores[..], [Restore::Branch { name, .. }, Restore::Changes { .. }] if name == "main-pgg-restored"),
            "{restores:#?}"
        );
    }

    #[test]
    fn a_part_brought_back_leaves_the_rest_and_the_last_takes_the_entry() {
        let walked = Walked::default();
        let taken = Taken::default();
        let delete = line(
            "delete branch",
            7,
            &[3],
            &[
                ("Branch", "spike"),
                ("Remote", "origin"),
                ("Remote-branch", "spike"),
                ("Remote-tip", &oid(4).to_hex()),
            ],
        );
        let restored: HashSet<(Oid, usize)> = [(oid(7), 0)].into();
        let found = entries(
            Vec::new(),
            std::slice::from_ref(&delete),
            &here(&walked, &taken, &restored),
        );
        assert_eq!(found[0].parts.len(), 1);
        assert_eq!(found[0].parts[0].record, Some((oid(7), 1)));
        let restored: HashSet<(Oid, usize)> = [(oid(7), 0), (oid(7), 1)].into();
        assert!(entries(Vec::new(), &[delete], &here(&walked, &taken, &restored)).is_empty());
    }

    #[test]
    fn two_parts_of_one_entry_do_not_take_the_same_name() {
        let walked = Walked::default();
        let taken = Taken::default();
        let restored = HashSet::new();
        let delete = line(
            "delete branch",
            7,
            &[3],
            &[
                ("Branch", "spike"),
                ("Remote-branch", "spike"),
                ("Remote-tip", &oid(4).to_hex()),
            ],
        );
        let found = entries(Vec::new(), &[delete], &here(&walked, &taken, &restored));
        let names: Vec<&str> = found[0]
            .parts
            .iter()
            .filter_map(|part| match &part.restore {
                Restore::Branch { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(names, vec!["spike", "spike-pgg-restored"]);
    }

    #[test]
    fn the_branches_one_rebase_moved_are_one_entry() {
        let walked = Walked::default();
        let taken = Taken::default();
        let restored = HashSet::new();
        let group = line(
            "rebase",
            7,
            &[3],
            &[
                (
                    "Moved",
                    &format!("topic {} {}", oid(3).to_hex(), oid(5).to_hex()),
                ),
                (
                    "Moved",
                    &format!("base {} {}", oid(2).to_hex(), oid(4).to_hex()),
                ),
            ],
        );
        let found = entries(
            vec![
                settled(DiscardKind::Rebase, "topic", 3, 5),
                settled(DiscardKind::Rebase, "base", 2, 4),
                settled(DiscardKind::Amend, "other", 8, 9),
            ],
            &[group],
            &here(&walked, &taken, &restored),
        );
        let seen: Vec<(DiscardKind, &str, usize)> = found
            .iter()
            .map(|entry| (entry.kind, entry.name.as_str(), entry.parts.len()))
            .collect();
        assert_eq!(
            seen,
            vec![
                (DiscardKind::Rebase, "topic, base", 2),
                (DiscardKind::Amend, "other", 1)
            ]
        );
    }

    /// A name git would refuse beside the ones here is no free name: a
    /// folder of one (`release` beside `release/1.0`), one under a branch
    /// (`feature/x` beside `feature` — flattened, since no suffix gets out
    /// from under it), or the same name in another case.
    #[test]
    fn a_name_clear_of_the_folders_and_cases_here() {
        let taken = Taken {
            branches: ["release/1.0", "feature", "Spike"]
                .map(str::to_string)
                .into(),
            tags: ["v1/rc".to_string()].into(),
        };
        let mut names = Names::new(&taken);
        assert_eq!(names.branch("release"), "release-pgg-restored");
        assert_eq!(names.branch("feature/x"), "feature-x");
        assert_eq!(names.branch("spike"), "spike-pgg-restored");
        assert_eq!(names.tag("v1"), "v1-pgg-restored");
        assert_eq!(
            names.branch("release"),
            "release-pgg-restored-1",
            "nor one an earlier part took"
        );
    }

    #[test]
    fn a_detached_heads_commits_come_back_under_the_detached_name() {
        let walked = Walked::default();
        let taken = Taken::default();
        let restored = HashSet::new();
        let found = entries(
            vec![settled(DiscardKind::LeftDetached, "repo", 3, 9)],
            &[],
            &here(&walked, &taken, &restored),
        );
        assert!(
            matches!(&found[0].parts[0].restore, Restore::Branch { name, .. } if name == "detached-pgg-restored")
        );
    }
}
