//! The `fast-import` stream the corpus is built from.
//!
//! One stream rather than one git process per commit: 200,000 processes
//! would cost hours, and one stream costs seconds (the same reason
//! `demo::deep` is written this way). Refs are written by the stream too
//! — `reset` inside the import is what makes 50,000 of them free, where
//! `update-ref --stdin` afterwards costs half a minute (measured).

use std::io::{self, Write};

use super::shape;

/// Marks 1..=BODIES are the file bodies as the history leaves them, and
/// the BODIES after those are the same bodies as the newest commit
/// leaves them. Commits start above both.
const EDITED_MARK: u64 = shape::BODIES;
const FIRST_COMMIT_MARK: u64 = 2 * shape::BODIES + 1;

/// What the newest commit is, so the caller can say where the
/// measurement will land without re-deriving it.
pub(super) struct Newest {
    /// The ordinary paths it changes, sorted. **Not all of them** —
    /// `huge` is the other one, and git diffs the two sets together, so
    /// anything asking what the commit touches has to chain them.
    pub(super) paths: Vec<String>,
    /// The one large enough for its diff to be a measurement of the
    /// grammar path (`shape::HUGE_BODY`).
    pub(super) huge: String,
}

/// The whole corpus as one import stream, written as it is generated,
/// and what its newest commit holds.
///
/// **It is written rather than returned.** The stream is every revision
/// of every file spelled out in full — fast-import has no delta input —
/// so it is larger than the repository it produces by more than an
/// order of magnitude, and holding it would mean holding all of that at
/// once. What git needs is a pipe, and a pipe is what this fills.
pub(super) fn write(out: &mut dyn Write) -> io::Result<Newest> {
    for body in 0..shape::BODIES {
        let text = shape::body(body);
        write!(
            out,
            "blob\nmark :{}\ndata {}\n{text}\n",
            body + 1,
            text.len()
        )?;
        let edited = shape::body_edited(body);
        write!(
            out,
            "blob\nmark :{}\ndata {}\n{edited}\n",
            EDITED_MARK + body + 1,
            edited.len()
        )?;
    }
    let holds = history(out)?;
    side_branches(out)?;
    let newest = newest_commit(out, &holds)?;
    refs(out)?;
    Ok(newest)
}

/// The chains the graph's window is mostly made of: short, unmerged, each
/// ending in a remote-tracking ref, forked from near the trunk's tip and
/// dated after it so they land in the window rather than behind it.
///
/// The commit command names the ref, so these need no `reset` of their
/// own — the branch and its tip arrive together.
fn side_branches(out: &mut dyn Write) -> io::Result<()> {
    let trunk_tip = shape::FIRST_COMMIT_AT + shape::TRUNK * shape::STEP_SECS;
    for branch in 0..shape::SIDE_BRANCHES {
        let name = shape::remote_branch(branch);
        // Forked from a near neighbour rather than from the trunk, so
        // the lane closes a few rows down instead of staying open to the
        // bottom of the window (`shape::FORK_BACK`). The oldest few have
        // no neighbour behind them and take the trunk's tip.
        let from = if branch < shape::FORK_BACK {
            FIRST_COMMIT_MARK + shape::TRUNK
        } else {
            FIRST_COMMIT_MARK
                + shape::TRUNK
                + 1
                + (branch - shape::FORK_BACK) * shape::SIDE_LENGTH
                + branch % shape::SIDE_LENGTH
        };
        for step in 0..shape::SIDE_LENGTH {
            let n = branch * shape::SIDE_LENGTH + step;
            let mark = FIRST_COMMIT_MARK + shape::TRUNK + 1 + n;
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
            let at = n % shape::PATHS;
            write!(
                out,
                "M 100644 :{} {}\n\n",
                1 + n % (shape::BODIES - 1),
                shape::path(at)
            )?;
        }
    }
    Ok(())
}

/// The long line of development every ref hangs off. Almost linear, as
/// the reference repository's is: one merge every `MERGE_EVERY`, from a
/// commit far enough back that the lane is visible in the window.
///
/// Answers which body each path was left holding, because the newest
/// commit has to edit *that* body — a commit that put an unrelated body
/// at a path would diff as a whole file rewritten, and the shape being
/// built is a few lines changed in a file of whatever size.
fn history(out: &mut dyn Write) -> io::Result<Vec<u64>> {
    // Bodies, not paths. Every slot is written before the newest commit
    // reads it — the trunk is hundreds of times longer than `PATHS` —
    // but a body number is what belongs here, and a path number left in
    // one would be emitted as a blob mark that names a commit.
    let mut holds: Vec<u64> = vec![0; shape::PATHS as usize];
    for n in 1..=shape::TRUNK {
        let mark = FIRST_COMMIT_MARK + n;
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
        if n > shape::MERGE_EVERY && n % shape::MERGE_EVERY == 0 {
            writeln!(
                out,
                "merge :{}",
                FIRST_COMMIT_MARK + n - shape::MERGE_EVERY / 2
            )?;
        }
        let at = n % shape::PATHS;
        // The one large source keeps its body: the newest commit's edit
        // to it is what makes a small diff in a big file measurable, and
        // an ordinary body left there would turn that into an add.
        let body = if at == shape::PATHS - 1 {
            shape::HUGE_BODY
        } else {
            (at + n / shape::PATHS) % (shape::BODIES - 1)
        };
        holds[at as usize] = body;
        write!(out, "M 100644 :{} {}\n\n", body + 1, shape::path(at))?;
    }
    Ok(holds)
}

/// The commit the interaction measurement opens: many files, so the
/// details pane has a list to build, and among them one large enough
/// that its diff is a measurement of the grammar path.
///
/// Its ordinary files come first by path so that the default scenario —
/// which opens the first changed file — measures the common case, and
/// the large one is opened by name when that is the question being asked.
fn newest_commit(out: &mut dyn Write, holds: &[u64]) -> io::Result<Newest> {
    let n = shape::COMMITS;
    let mark = FIRST_COMMIT_MARK + n;
    let (name, mail) = shape::author(n);
    let (by, by_mail) = shape::committer(n);
    let when = shape::FIRST_COMMIT_AT + n * shape::STEP_SECS;
    let message = shape::message(n);
    writeln!(out, "commit refs/heads/main")?;
    writeln!(out, "mark :{mark}")?;
    writeln!(out, "author {name} <{mail}> {when} +0000")?;
    writeln!(out, "committer {by} <{by_mail}> {when} +0000")?;
    write!(out, "data {}\n{message}\n", message.len())?;
    let mut paths = Vec::new();
    for file in 0..shape::NEWEST_FILES {
        // Offset into the path pool by a stride the history's own
        // rotation does not land on, so these are files with a past
        // rather than ones the newest commit invents. Every one of them
        // takes the *edited* body, which is a few lines away from what
        // the history left there.
        let at = (file * 7 + 3) % shape::PATHS;
        let path = shape::path(at);
        writeln!(
            out,
            "M 100644 :{} {path}",
            EDITED_MARK + holds[at as usize] + 1
        )?;
        paths.push(path);
    }
    let huge = shape::path(shape::PATHS - 1);
    write!(
        out,
        "M 100644 :{} {huge}\n\n",
        EDITED_MARK + shape::HUGE_BODY + 1
    )?;
    paths.sort();
    Ok(Newest { paths, huge })
}

/// The refs. Tags and remote-tracking branches spread across the whole
/// history rather than bunched at its tip: what the ref tables cost is
/// their names, and what the graph draws is where they land.
fn refs(out: &mut dyn Write) -> io::Result<()> {
    for n in 0..shape::TAGS {
        let at = FIRST_COMMIT_MARK + 1 + (n * shape::TRUNK / shape::TAGS);
        write!(out, "reset refs/tags/{}\nfrom :{at}\n\n", shape::tag(n))?;
    }
    // The first `SIDE_BRANCHES` of them already exist: their commits
    // named them, which is what put their tips in the graph's window.
    for n in shape::SIDE_BRANCHES..shape::REMOTE_BRANCHES {
        let at = FIRST_COMMIT_MARK + 1 + (n * shape::TRUNK / shape::REMOTE_BRANCHES);
        write!(
            out,
            "reset refs/remotes/{}\nfrom :{at}\n\n",
            shape::remote_branch(n)
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{FIRST_COMMIT_MARK, Newest, write};
    use crate::corpus::shape;

    /// The whole stream in memory, which only a test can afford.
    fn build() -> (String, Newest) {
        let mut buffer = Vec::new();
        let newest = write(&mut buffer).expect("a vector never refuses a write");
        (
            String::from_utf8(buffer).expect("the stream is generated as UTF-8"),
            newest,
        )
    }

    /// The stream is what git will be handed, so the things that must be
    /// true of it are counted here rather than after a two-minute
    /// import: every commit accounted for, one ref per name, and a blob
    /// for every body.
    #[test]
    fn the_stream_carries_the_shape_it_was_asked_for() {
        let (stream, newest) = build();
        let trunk = stream.matches("commit refs/heads/main\n").count() as u64;
        let side = stream.matches("commit refs/remotes/").count() as u64;
        assert_eq!(trunk, shape::TRUNK + 1, "the trunk and the newest commit");
        assert_eq!(side, shape::SIDE_BRANCHES * shape::SIDE_LENGTH);
        assert_eq!(trunk + side, shape::COMMITS, "every commit is spoken for");
        assert_eq!(
            stream.matches("reset refs/tags/").count() as u64,
            shape::TAGS
        );
        // The branches whose commits named them need no `reset`.
        assert_eq!(
            stream.matches("reset refs/remotes/").count() as u64,
            shape::REMOTE_BRANCHES - shape::SIDE_BRANCHES
        );
        // Every body twice: as the history leaves it, and as the newest
        // commit leaves it.
        assert_eq!(
            stream.matches("\nblob\n").count() as u64 + 1,
            2 * shape::BODIES
        );
        assert_eq!(newest.paths.len() as u64, shape::NEWEST_FILES);
    }

    /// A trunk that is almost linear, and beside it the unmerged chains
    /// the window is mostly made of — which is the shape of the
    /// reference repository's own newest 2,000 commits.
    #[test]
    fn the_window_is_many_chains_rather_than_one_column() {
        let (stream, _) = build();
        let merges = stream.matches("\nmerge :").count() as u64;
        assert_eq!(merges, shape::TRUNK / shape::MERGE_EVERY - 1);
        assert!(!stream.contains(&format!("merge :{FIRST_COMMIT_MARK}\n")));
        // One fork per branch and no more: a chain whose every commit
        // said `from` would be that many one-commit branches.
        assert_eq!(
            stream.matches("\nfrom :").count() as u64,
            shape::SIDE_BRANCHES + shape::TAGS + shape::REMOTE_BRANCHES - shape::SIDE_BRANCHES
        );
    }

    /// Every file a commit places must name a blob, and blob marks stop
    /// below [`FIRST_COMMIT_MARK`]. fast-import says `Mark :N not a
    /// blob` and dies at the end of a stream that got this wrong, which
    /// is a hundred seconds after the mistake was made and names a mark
    /// rather than the constant behind it.
    #[test]
    fn every_file_a_commit_places_names_a_blob() {
        let (text, _) = build();
        let marks = text
            .lines()
            .filter_map(|line| line.strip_prefix("M 100644 :"))
            .filter_map(|rest| rest.split(' ').next())
            .filter_map(|mark| mark.parse::<u64>().ok());
        let mut seen = 0;
        for mark in marks {
            assert!(mark < FIRST_COMMIT_MARK, "{mark} is a commit, not a blob");
            seen += 1;
        }
        assert!(seen > shape::TRUNK, "{seen} files placed");
    }

    /// The default scenario opens the first changed file. It has to be
    /// an ordinary one, or every run would be measuring the tail.
    ///
    /// Which file that is, git decides by sorting every path the commit
    /// touched — the large one included, which is why it is not enough
    /// to ask what the ordinary paths start with. The large file's name
    /// is drawn from the same word list as the rest, so nothing but the
    /// draw keeps it out of first place.
    #[test]
    fn the_first_file_of_the_newest_commit_is_an_ordinary_one() {
        let (_, newest) = build();
        let first = newest
            .paths
            .iter()
            .chain(std::iter::once(&newest.huge))
            .min()
            .expect("the newest commit changes files");
        assert_ne!(first, &newest.huge);
        assert!(first.ends_with(".kt"), "{first}");
    }
}
