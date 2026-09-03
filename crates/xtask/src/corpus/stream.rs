//! The `fast-import` stream the corpus is built from.
//!
//! One stream rather than one git process per commit: 200,000 processes
//! would cost hours, and one stream costs minutes (the same reason
//! `demo::deep` is written this way). Refs are written by the stream too
//! — `reset` inside the import is what makes 50,000 of them free, where
//! `update-ref --stdin` afterwards costs half a minute (measured).
//!
//! **Bodies are inline and per revision.** A prologue of shared blobs
//! would be a repository of a hundred distinct files however many
//! commits named them, and the object database is an axis of its own:
//! the reference repository's five million objects are what every git
//! process the application spawns maps before it can resolve anything.
//! So every placement writes its own bytes, and the stream is larger
//! than the repository it produces by more than an order of magnitude.

use std::io::{self, Write};

use super::{shape, tree};

/// The first commit's mark. Commits are the only marks now: bodies are
/// written inline, so there are no blob marks to leave room for.
const BASE_MARK: u64 = 1;

/// What the newest commit is, so the caller can say where the
/// measurement will land without re-deriving it.
pub(super) struct Newest {
    /// The ordinary paths it changes, sorted. **Not all of them** —
    /// `huge` is the other one, and git diffs the two sets together, so
    /// anything asking what the commit touches has to chain them.
    pub(super) paths: Vec<String>,
    /// The one large enough for its diff to be a measurement of the
    /// grammar path.
    pub(super) huge: String,
}

/// The whole corpus as one import stream, written as it is generated,
/// and what its newest commit holds.
///
/// **It is written rather than returned.** The stream is every revision
/// of every file spelled out in full — fast-import has no delta input —
/// so holding it would mean holding tens of gigabytes at once. What git
/// needs is a pipe, and a pipe is what this fills.
pub(super) fn write(out: &mut dyn Write) -> io::Result<Newest> {
    let tree = tree::build();
    let mut live = History::new(&tree);
    // One buffer for every file body the build ever writes. Four
    // million placements, so this is the difference between one
    // allocation and four million.
    let mut body = Vec::with_capacity(8 * 1024);
    base(out, &tree, &mut body)?;
    churn(out, &tree, &mut live, &mut body)?;
    side_branches(out, &tree, &live, &mut body)?;
    let newest = newest_commit(out, &tree, &live)?;
    refs(out)?;
    Ok(newest)
}

/// Which paths are tracked and what revision each is on.
struct History {
    revs: Vec<u32>,
    tracked: Vec<bool>,
    /// Paths not tracked at the moment, newest first — where a commit
    /// that deletes one path finds the path to add in its place.
    spare: Vec<u64>,
}

impl History {
    fn new(tree: &tree::Tree) -> Self {
        Self {
            revs: vec![0; tree.paths.len()],
            tracked: (0..tree.paths.len() as u64)
                .map(|slot| slot < tree::TRACKED)
                .collect(),
            spare: (tree::TRACKED..tree::SLOTS).rev().collect(),
        }
    }
}

/// The commit that places the whole tree. Everything after it is a
/// change to what this left.
fn base(out: &mut dyn Write, tree: &tree::Tree, body: &mut Vec<u8>) -> io::Result<()> {
    let message = shape::message(1);
    let (name, mail) = shape::author(1);
    let when = shape::FIRST_COMMIT_AT + shape::STEP_SECS;
    writeln!(out, "commit refs/heads/main")?;
    writeln!(out, "mark :{BASE_MARK}")?;
    writeln!(out, "author {name} <{mail}> {when} +0000")?;
    writeln!(out, "committer {name} <{mail}> {when} +0000")?;
    write!(out, "data {}\n{message}\n", message.len())?;
    // Tracked, so `status` reads them and the per-directory exclude
    // stack it pushes and pops as it walks is the one the reference
    // repository's status pays for.
    writeln!(out, "M 100644 inline .gitignore")?;
    writeln!(out, "data {}", shape::GITIGNORE.len())?;
    write!(out, "{}", shape::GITIGNORE)?;
    for n in 0..shape::NESTED_IGNORES {
        let slot = (n + 1) * tree::TRACKED / (shape::NESTED_IGNORES + 1);
        let Some((dir, _)) = tree.paths[slot as usize].rsplit_once('/') else {
            continue;
        };
        writeln!(out, "M 100644 inline {dir}/.gitignore")?;
        writeln!(out, "data {}", shape::NESTED_GITIGNORE.len())?;
        write!(out, "{}", shape::NESTED_GITIGNORE)?;
    }
    for slot in 0..tree::TRACKED {
        place(out, tree, body, slot, 0)?;
    }
    writeln!(out)
}

/// One file placed, with its own bytes.
///
/// **The body goes into a buffer that is reused.** This is the build's
/// innermost loop — four million placements — so a `Vec` allocated per
/// file, and a `String` per line inside it, is what the wall clock is
/// made of.
fn place(
    out: &mut dyn Write,
    tree: &tree::Tree,
    body: &mut Vec<u8>,
    slot: u64,
    rev: u32,
) -> io::Result<()> {
    let want = tree.sizes[slot as usize] as usize;
    shape::content_into(body, slot, want, rev, tree.mode(slot));
    writeln!(
        out,
        "M {} inline {}",
        tree.mode(slot),
        tree.paths[slot as usize]
    )?;
    writeln!(out, "data {}", body.len())?;
    out.write_all(body)?;
    writeln!(out)
}

/// `M <mode> inline <path>` and the bytes behind it. **`data` counts
/// bytes**, and a count that disagrees with what follows is a stream
/// fast-import rejects at the point it notices — which is after the
/// whole of it has been written.
fn inline(out: &mut dyn Write, tree: &tree::Tree, slot: u64, body: &[u8]) -> io::Result<()> {
    writeln!(
        out,
        "M {} inline {}",
        tree.mode(slot),
        tree.paths[slot as usize]
    )?;
    writeln!(out, "data {}", body.len())?;
    out.write_all(body)?;
    writeln!(out)
}

/// The long line of development every ref hangs off, and the changes
/// that make it a history rather than a snapshot.
///
/// **Clustered, as a change to a repository is.** The reference
/// repository's commits touch 16.90 files across 4.95 directories;
/// files scattered over the whole tree would rewrite a different tree
/// object for each one and cost the pack what a real commit does not.
fn churn(
    out: &mut dyn Write,
    tree: &tree::Tree,
    live: &mut History,
    body: &mut Vec<u8>,
) -> io::Result<()> {
    for n in 2..=shape::TRUNK {
        let mark = BASE_MARK + n - 1;
        let (name, mail) = shape::author(n);
        let (by, by_mail) = shape::committer(n);
        let when = shape::FIRST_COMMIT_AT + n * shape::STEP_SECS;
        let message = shape::message(n);
        writeln!(out, "commit refs/heads/main")?;
        writeln!(out, "mark :{mark}")?;
        writeln!(out, "author {name} <{mail}> {when} +0000")?;
        writeln!(out, "committer {by} <{by_mail}> {when} +0000")?;
        write!(out, "data {}\n{message}\n", message.len())?;
        // A second parent every so often. `from` is implicit for the
        // first parent: the ref already points at the previous commit.
        if n > shape::MERGE_EVERY && n.is_multiple_of(shape::MERGE_EVERY) {
            writeln!(out, "merge :{}", BASE_MARK + n - 1 - shape::MERGE_EVERY / 2)?;
        }
        changes(out, tree, live, n, body)?;
        writeln!(out)?;
    }
    Ok(())
}

/// What one commit does to the tree.
fn changes(
    out: &mut dyn Write,
    tree: &tree::Tree,
    live: &mut History,
    n: u64,
    body: &mut Vec<u8>,
) -> io::Result<()> {
    // **The clusters sit near each other, not across the tree.** A
    // commit rewrites one tree object per directory on the path of
    // every file it touches, so clusters in distant subtrees rewrite
    // that many deep paths whole; the reference repository rewrites
    // 16.9 trees a commit for its 16.9 files, which is what changes
    // sharing a subtree look like. Neighbouring slots are neighbouring
    // directories, so their parents are written once.
    let near = shape::mix(n ^ 0x00C1_57E0) % tree::TRACKED;
    for cluster in 0..shape::CLUSTERS {
        let at = (near + shape::mix(n ^ (cluster << 40)) % shape::CLUSTER_REACH) % tree::TRACKED;
        for step in 0..shape::CHANGED_FILES / shape::CLUSTERS {
            let slot = (at + step) % tree::TRACKED;
            touch(out, tree, live, n ^ (slot << 8), slot, body)?;
        }
    }
    // **A megabyte file rewritten now and then.** The reference
    // repository's pack is mostly historical revisions of large files;
    // without them the corpus packs to three quarters of it however
    // many small files it churns.
    if n.is_multiple_of(shape::HUGE_EVERY)
        && let Some(slot) = tree.huge.get((n as usize / 4) % tree.huge.len())
    {
        let at = *slot as usize;
        if live.tracked[at] {
            live.revs[at] += 1;
            place(out, tree, body, *slot, live.revs[at])?;
        }
    }
    // One rename now and then, so `--find-renames` has an add and a
    // delete of like content to score rather than nothing at all.
    if n.is_multiple_of(shape::RENAME_EVERY) {
        let from = shape::mix(n ^ 0x0000_5E4A) % tree::TRACKED;
        if live.tracked[from as usize]
            && let Some(to) = live.spare.pop()
        {
            live.tracked[from as usize] = false;
            live.tracked[to as usize] = true;
            live.revs[to as usize] = live.revs[from as usize];
            writeln!(out, "D {}", tree.paths[from as usize])?;
            // The same content under a new name is what `--find-renames`
            // scores; a fresh body would read as one delete and one add.
            let want = tree.sizes[from as usize] as usize;
            shape::content_into(body, from, want, live.revs[from as usize], tree.mode(from));
            inline(out, tree, to, body)?;
            live.spare.push(from);
        }
    }
    Ok(())
}

/// One path changed: rewritten where it is tracked, added back where it
/// is not, and deleted on the share of touches the reference
/// repository's history deletes.
fn touch(
    out: &mut dyn Write,
    tree: &tree::Tree,
    live: &mut History,
    seed: u64,
    slot: u64,
    body: &mut Vec<u8>,
) -> io::Result<()> {
    let at = slot as usize;
    if !live.tracked[at] {
        live.tracked[at] = true;
        live.revs[at] += 1;
        live.spare.retain(|spare| *spare != slot);
        return place(out, tree, body, slot, live.revs[at]);
    }
    if shape::mix(seed ^ 0x0000_DE1E) % 100 < shape::DELETED_SHARE {
        live.tracked[at] = false;
        writeln!(out, "D {}", tree.paths[at])?;
        // **Paired with an add, in the same commit.** A delete on its
        // own drains the tree — every touch that lands on a tracked
        // path can delete it and only a touch that lands on an
        // untracked one adds, so the count settles below the reference
        // repository's and the index and the status settle with it
        // (measured: 95,207 files where the tables describe 106,581).
        let Some(back) = live.spare.pop() else {
            live.spare.push(slot);
            return Ok(());
        };
        live.spare.push(slot);
        live.tracked[back as usize] = true;
        live.revs[back as usize] += 1;
        return place(out, tree, body, back, live.revs[back as usize]);
    }
    live.revs[at] += 1;
    place(out, tree, body, slot, live.revs[at])
}

/// The chains the graph's window is mostly made of: short, unmerged,
/// each ending in a remote-tracking ref, forked from near the trunk's
/// tip and dated after it so they land in the window rather than behind
/// it.
///
/// The commit command names the ref, so these need no `reset` of their
/// own — the branch and its tip arrive together.
fn side_branches(
    out: &mut dyn Write,
    tree: &tree::Tree,
    live: &History,
    body: &mut Vec<u8>,
) -> io::Result<()> {
    let tip = BASE_MARK + shape::TRUNK - 1;
    let trunk_tip = shape::FIRST_COMMIT_AT + shape::TRUNK * shape::STEP_SECS;
    for branch in 0..shape::SIDE_BRANCHES {
        let name = shape::remote_branch(branch);
        // Forked from a near neighbour rather than from the trunk, so
        // the lane closes a few rows down instead of staying open to
        // the bottom of the window (`shape::FORK_BACK`). The oldest few
        // have no neighbour behind them and take the trunk's tip.
        let back = shape::fork_back(branch);
        let from = if branch < back {
            tip
        } else {
            tip + 1 + (branch - back) * shape::SIDE_LENGTH + branch % shape::SIDE_LENGTH
        };
        for step in 0..shape::SIDE_LENGTH {
            let n = branch * shape::SIDE_LENGTH + step;
            let mark = tip + 1 + n;
            let seed = n ^ 0x5B10;
            let (who, mail) = shape::author(seed);
            let (by, by_mail) = shape::committer(seed);
            let when = trunk_tip + (n + 1) * shape::STEP_SECS;
            let message = shape::message(seed);
            writeln!(out, "commit refs/remotes/{name}")?;
            writeln!(out, "mark :{mark}")?;
            writeln!(out, "author {who} <{mail}> {when} +0000")?;
            writeln!(out, "committer {by} <{by_mail}> {when} +0000")?;
            write!(out, "data {}\n{message}\n", message.len())?;
            if step == 0 {
                writeln!(out, "from :{from}")?;
            }
            for file in 0..shape::side_files(n) {
                let Some(slot) = edited_slot(tree, live, seed ^ (file << 24)) else {
                    continue;
                };
                place(
                    out,
                    tree,
                    body,
                    slot,
                    live.revs[slot as usize] + 1 + step as u32 + file as u32,
                )?;
            }
            writeln!(out)?;
        }
    }
    Ok(())
}

/// Which file a commit off the trunk touches.
///
/// **What people edit is not what a tree is mostly made of.** The
/// reference repository's median tracked file is 565 bytes and the
/// median file its window opens is 10,366 — the files under active work
/// are not the ones a tree is filled with. Drawn uniformly the window
/// opens six hundred bytes a row, and every diff in it costs a
/// sixteenth of what the same click costs against the real repository.
/// The largest of [`shape::EDIT_DRAWS`] draws lands in the same decile
/// without a second index over the tree.
///
/// **A slot a rename has carried away is not a file any more**, and
/// naming it here would add one back on a branch the trunk does not
/// have. The scan past it is bounded rather than a search: nearly every
/// slot is tracked, and a tree with none left has nothing to say.
fn edited_slot(tree: &tree::Tree, live: &History, seed: u64) -> Option<u64> {
    let mut edited: Option<u64> = None;
    for draw in 0..shape::EDIT_DRAWS {
        let drawn = shape::mix(seed ^ (draw << 48)) % tree::TRACKED;
        let slot = (0..tree::TRACKED)
            .map(|step| (drawn + step) % tree::TRACKED)
            .find(|slot| live.tracked[*slot as usize])?;
        if edited.is_none_or(|had| tree.sizes[slot as usize] > tree.sizes[had as usize]) {
            edited = Some(slot);
        }
    }
    edited
}

/// The commit the interaction measurement opens: many files, so the
/// details pane has a list to build, and among them one large enough
/// that its diff is a measurement of the grammar path.
///
/// Its ordinary files come first by path so that the default scenario —
/// which opens the first changed file — measures the common case, and
/// the large one is opened by name when that is the question being
/// asked.
fn newest_commit(out: &mut dyn Write, tree: &tree::Tree, live: &History) -> io::Result<Newest> {
    let n = shape::COMMITS;
    let mark = BASE_MARK + shape::TRUNK + shape::SIDE_BRANCHES * shape::SIDE_LENGTH;
    let (name, mail) = shape::author(n);
    let (by, by_mail) = shape::committer(n);
    let when = shape::FIRST_COMMIT_AT + n * shape::STEP_SECS;
    let message = shape::message(n);
    writeln!(out, "commit refs/heads/main")?;
    writeln!(out, "mark :{mark}")?;
    writeln!(out, "author {name} <{mail}> {when} +0000")?;
    writeln!(out, "committer {by} <{by_mail}> {when} +0000")?;
    write!(out, "data {}\n{message}\n", message.len())?;
    writeln!(out, "from :{}", BASE_MARK + shape::TRUNK - 1)?;
    // **The large file must not sort first.** git diffs a commit's
    // paths in order and the default scenario opens the first, so a
    // large one at the head of the list would make every run a
    // measurement of the grammar path where the record says it measures
    // the common case. The ordinary files are drawn from across the
    // whole tree and the large one is the last-sorting of the eleven,
    // and then the pick is checked rather than assumed.
    let huge = tree
        .huge
        .iter()
        .copied()
        .max_by_key(|slot| &tree.paths[*slot as usize])
        .unwrap_or(0);
    // **The file the default scenario opens is chosen, not drawn.** It
    // is the first by path, so drawing all of them uniformly opens the
    // median file of the tree — six hundred bytes, where the reference
    // repository's window opens ten thousand (`shape::OPENED_BYTES`).
    // This is the lowest-sorting file large enough to be one somebody
    // works in, and everything else in the commit is held above it.
    let opened = (0..tree::TRACKED)
        .filter(|slot| live.tracked[*slot as usize] && *slot != huge)
        .filter(|slot| tree.sizes[*slot as usize] as usize >= shape::OPENED_BYTES)
        .filter(|slot| tree.paths[*slot as usize] < tree.paths[huge as usize])
        .min_by_key(|slot| &tree.paths[*slot as usize]);
    let mut slots: Vec<u64> = opened.into_iter().collect();
    let above = opened.map(|slot| tree.paths[slot as usize].clone());
    let mut slot = shape::mix(n ^ 0x004E_0E57) % tree::TRACKED;
    let mut looked = 0;
    while (slots.len() as u64) < shape::NEWEST_FILES && looked < tree::TRACKED {
        looked += 1;
        slot = (slot + tree::TRACKED / shape::NEWEST_FILES + 1) % tree::TRACKED;
        if above
            .as_ref()
            .is_some_and(|first| tree.paths[slot as usize] <= *first)
        {
            continue;
        }
        // Big enough to take the edit. **`shape::edited` leaves a file
        // of fewer than four lines exactly as it found it**, and a path
        // the commit names but does not change is a path the details
        // pane will not list — the record would say 76 files and git
        // would say fewer.
        if live.tracked[slot as usize]
            && slot != huge
            && tree.sizes[slot as usize] as usize >= shape::EDITABLE_FLOOR
            && !slots.contains(&slot)
        {
            slots.push(slot);
        }
    }
    if opened.is_none()
        && slots
            .iter()
            .all(|slot| tree.paths[*slot as usize] > tree.paths[huge as usize])
    {
        let lowest = (0..tree::TRACKED)
            .filter(|slot| live.tracked[*slot as usize] && *slot != huge)
            .min_by_key(|slot| &tree.paths[*slot as usize])
            .unwrap_or(0);
        slots[0] = lowest;
    }
    let mut paths = Vec::new();
    for slot in slots {
        let want = tree.sizes[slot as usize] as usize;
        let rev = live.revs[slot as usize];
        let body = shape::edited(slot, want, rev, tree.mode(slot));
        inline(out, tree, slot, &body)?;
        paths.push(tree.paths[slot as usize].clone());
    }
    let want = tree.sizes[huge as usize] as usize;
    let body = shape::edited(huge, want, live.revs[huge as usize], tree.mode(huge));
    inline(out, tree, huge, &body)?;
    writeln!(out)?;
    paths.sort();
    Ok(Newest {
        paths,
        huge: tree.paths[huge as usize].clone(),
    })
}

/// The refs. Tags and remote-tracking branches spread across the whole
/// history rather than bunched at its tip: what the ref tables cost is
/// their names, and what the graph draws is where they land.
fn refs(out: &mut dyn Write) -> io::Result<()> {
    let tip = BASE_MARK + shape::TRUNK - 1;
    let last = tip + shape::SIDE_BRANCHES * shape::SIDE_LENGTH;
    for n in 0..shape::TAGS {
        // Most of them spread over the whole history, where they cost
        // the ref tables their names and the graph nothing. The last
        // `TAGS_IN_WINDOW` land on commits the window holds, which is
        // what the graph actually draws — and some of those share a
        // commit, because the reference repository's busiest carries
        // forty-two.
        let at = if n + shape::TAGS_IN_WINDOW < shape::TAGS {
            BASE_MARK + (n * (shape::TRUNK - 1) / shape::TAGS)
        } else {
            // **Spread, not heaped.** A row that carries any chip lays
            // chips out, so what costs the renderer is the number of
            // rows carrying them as much as the count: the reference
            // repository puts 705 chips on 531 of its 2,000 rows. One
            // commit takes `REFS_ON_ONE` of them, because its busiest
            // carries forty-two and that is one `LabelIndex` bucket and
            // one row laying out forty-two.
            let into = n + shape::TAGS_IN_WINDOW - shape::TAGS;
            let span = last - tip;
            match into.checked_sub(shape::REFS_ON_ONE) {
                None => tip + 1 + span / 2,
                Some(spread) => tip + 1 + (spread * span / shape::TAGS_IN_WINDOW),
            }
        };
        annotated_or_not(out, n, at)?;
    }
    // The first `SIDE_BRANCHES` of them already exist: their commits
    // named them, which is what put their tips in the graph's window.
    for n in shape::SIDE_BRANCHES..shape::REMOTE_BRANCHES {
        let at = BASE_MARK + (n * (shape::TRUNK - 1) / shape::REMOTE_BRANCHES);
        writeln!(
            out,
            "reset refs/remotes/{}\nfrom :{at}\n",
            shape::remote_branch(n)
        )?;
    }
    Ok(())
}

/// One tag, as a tag object or as a ref pointing straight at a commit.
///
/// **Annotated is the expensive one, and it is what the reference
/// repository has**: 99.3% of its tags carry an object, so reading one
/// costs a peel where a lightweight tag resolves in a single read. A
/// corpus of lightweight tags makes forty thousand of those free.
fn annotated_or_not(out: &mut dyn Write, n: u64, at: u64) -> io::Result<()> {
    let name = shape::tag(n);
    if mix_share(n) >= shape::ANNOTATED_SHARE {
        return writeln!(out, "reset refs/tags/{name}\nfrom :{at}\n");
    }
    let (who, mail) = shape::author(n ^ 0x0000_7A69);
    // A tagger date of its own, later than the commit's: the sidebar
    // sorts tags by date, and a tag that took its commit's date would
    // sort in an order the ref list already has.
    let when = shape::FIRST_COMMIT_AT + (shape::COMMITS + n) * shape::STEP_SECS;
    let message = shape::subject(n ^ 0x0000_7A69);
    writeln!(out, "tag {name}")?;
    writeln!(out, "from :{at}")?;
    writeln!(out, "tagger {who} <{mail}> {when} +0000")?;
    write!(out, "data {}\n{message}\n", message.len())
}

fn mix_share(n: u64) -> u64 {
    shape::mix(n ^ 0x0000_A117) % 1_000
}

#[cfg(test)]
mod tests;
