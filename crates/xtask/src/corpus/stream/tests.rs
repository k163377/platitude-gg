use std::io::Write;

use super::{BASE_MARK, History, base, changes, newest_commit, refs, side_branches};
use crate::corpus::{shape, tree};

/// A sink that counts the lines the tests ask about and holds none of
/// them. **The stream is tens of gigabytes at this shape** — every
/// revision of every file spelled out, because fast-import has no delta
/// input — which is why it is written rather than built, and why a test
/// that collected it would take the process down.
#[derive(Default)]
struct Tally {
    bytes: u64,
    counts: std::collections::BTreeMap<&'static str, u64>,
    partial: Vec<u8>,
}

/// The line openings a test may ask about. Only these, because a body's
/// own lines go past here too and a prefix a body could start with
/// would count those as well.
const PREFIXES: [&str; 10] = [
    "commit refs/heads/main",
    "commit refs/remotes/",
    "M 100644 inline ",
    "M 100755 inline ",
    "M 120000 inline ",
    "D ",
    "reset refs/tags/",
    "reset refs/remotes/",
    "tag ",
    "tagger ",
];

impl Tally {
    fn count(&self, prefix: &str) -> u64 {
        self.counts.get(prefix).copied().unwrap_or(0)
    }

    fn line(&mut self, line: &[u8]) {
        let Ok(line) = std::str::from_utf8(line) else {
            return;
        };
        for prefix in PREFIXES {
            if line.starts_with(prefix) {
                *self.counts.entry(prefix).or_default() += 1;
            }
        }
    }
}

impl Write for Tally {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.bytes += buf.len() as u64;
        for byte in buf {
            if *byte == b'\n' {
                let line = std::mem::take(&mut self.partial);
                self.line(&line);
            } else if self.partial.len() < 512 {
                self.partial.push(*byte);
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Every file the base commit places is one the index carries and one
/// `git status` walks — which is the whole of the startup number this
/// axis exists to reach.
#[test]
fn the_base_commit_places_the_whole_tree() {
    let tree = tree::build();
    let mut tally = Tally::default();
    base(&mut tally, &tree, &mut Vec::new()).expect("a tally never refuses a write");
    let placed = tally.count("M 100644 inline ")
        + tally.count("M 100755 inline ")
        + tally.count("M 120000 inline ");
    // The tree, the root ignore file, and the nested ones git builds
    // its exclude stack out of.
    assert_eq!(placed, tree::TRACKED + 1 + shape::NESTED_IGNORES);
    assert_eq!(tally.count("commit refs/heads/main"), 1);
    // Modes are an axis of their own, and a tree of one mode never
    // produces the diff a mode change does.
    assert!(tally.count("M 100755 inline ") >= 100, "no executables");
    assert_eq!(tally.count("M 120000 inline "), 1, "no symlink");
}

/// **A history is not only modifications.** The reference repository's
/// first-parent history is 36% adds against 36% deletes; a corpus of
/// pure `M` gives `--find-renames` nothing to score, never sets
/// `FileChange.orig_path`, and never opens an added file's diff against
/// `/dev/null`.
#[test]
fn the_history_adds_and_deletes_as_well_as_changes() {
    let tree = tree::build();
    let mut live = History::new(&tree);
    let mut tally = Tally::default();
    let mut body = Vec::new();
    for n in 2..2_000 {
        changes(&mut tally, &tree, &mut live, n, &mut body).expect("a tally never refuses a write");
    }
    let deletes = tally.count("D ");
    let placed = tally.count("M 100644 inline ")
        + tally.count("M 100755 inline ")
        + tally.count("M 120000 inline ");
    assert!(deletes > 1_000, "{deletes} deletes over 2,000 commits");
    assert!(
        placed > deletes * 3,
        "{placed} placements, {deletes} deletes"
    );
    // And the tree does not drain: a delete takes a path out and an add
    // puts one back, so what `git status` walks stays the size it was.
    let tracked = live.tracked.iter().filter(|held| **held).count() as u64;
    assert!(
        tracked.abs_diff(tree::TRACKED) < tree::SPARE,
        "{tracked} tracked against {}",
        tree::TRACKED
    );
}

/// The default scenario opens the first changed file, and git decides
/// which that is by sorting every path the commit touched — the large
/// one included. Measuring the tail by accident is measuring the
/// grammar path where the record says it measures the common case.
#[test]
fn the_first_file_of_the_newest_commit_is_an_ordinary_one() {
    let tree = tree::build();
    let live = History::new(&tree);
    let mut tally = Tally::default();
    let newest = newest_commit(&mut tally, &tree, &live).expect("a tally never refuses a write");
    assert_eq!(newest.paths.len() as u64, shape::NEWEST_FILES);
    let first = newest
        .paths
        .iter()
        .chain(std::iter::once(&newest.huge))
        .min()
        .expect("the newest commit changes files");
    assert_ne!(first, &newest.huge);
}

/// Every mark a commit or a ref reaches for was minted, and no mark is
/// minted twice.
///
/// **Nothing else holds this.** A `from` or a `merge` naming a mark the
/// stream never wrote is a stream fast-import rejects — at the end,
/// after ten minutes, naming a number rather than the arithmetic behind
/// it. The counting sink cannot answer this, so this one reads the
/// marks themselves out of the parts that mint and use them.
#[test]
fn every_mark_reached_for_was_minted_exactly_once() {
    let tree = tree::build();
    let mut live = History::new(&tree);
    let mut body = Vec::new();
    let mut marks = Marks::default();
    base(&mut marks, &tree, &mut body).expect("a sink never refuses a write");
    for n in 2..2_000 {
        changes(&mut marks, &tree, &mut live, n, &mut body).expect("a sink never refuses");
    }
    side_branches(&mut marks, &tree, &live, &mut body).expect("a sink never refuses");
    newest_commit(&mut marks, &tree, &live).expect("a sink never refuses");
    refs(&mut marks).expect("a sink never refuses");
    assert!(marks.minted.contains(&BASE_MARK), "no base mark");
    assert_eq!(
        marks.minted.len(),
        marks.minted_count,
        "a mark was minted twice"
    );
    // The side branches and the refs reach back into the trunk, and
    // this test only wrote its head — so what it can hold is that every
    // reach lands inside the range the whole stream mints.
    let last = BASE_MARK + shape::TRUNK + shape::SIDE_BRANCHES * shape::SIDE_LENGTH;
    for reached in &marks.reached {
        assert!(
            (BASE_MARK..=last).contains(reached),
            "a stream that mints :{BASE_MARK}..:{last} reaches for :{reached}"
        );
    }
    assert!(!marks.reached.is_empty(), "nothing reached for a mark");
}

/// What the marks test collects: the ones written and the ones asked
/// for.
#[derive(Default)]
struct Marks {
    minted: std::collections::BTreeSet<u64>,
    minted_count: usize,
    reached: Vec<u64>,
    partial: Vec<u8>,
}

impl Marks {
    fn line(&mut self, line: &str) {
        for (prefix, into) in [("mark :", true), ("from :", false), ("merge :", false)] {
            let Some(rest) = line.strip_prefix(prefix) else {
                continue;
            };
            let Ok(mark) = rest.trim().parse::<u64>() else {
                continue;
            };
            if into {
                self.minted.insert(mark);
                self.minted_count += 1;
            } else {
                self.reached.push(mark);
            }
        }
    }
}

impl Write for Marks {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        for byte in buf {
            if *byte == b'\n' {
                let line = std::mem::take(&mut self.partial);
                if let Ok(line) = std::str::from_utf8(&line) {
                    self.line(line);
                }
            } else if self.partial.len() < 64 {
                self.partial.push(*byte);
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// **The window is what gets clicked, and the window is side branches.**
/// A row whose diff opens the tree's median file measures a sixteenth
/// of what the same click costs against the reference repository, whose
/// window opens 10,366 bytes at the median and 60,895 at the ninth
/// decile. Held here rather than found in a ten-minute build.
#[test]
fn a_side_branch_edits_the_kind_of_file_people_work_in() {
    let tree = tree::build();
    let live = History::new(&tree);
    let mut sizes: Vec<u32> = Vec::new();
    for n in 0..shape::SIDE_BRANCHES * shape::SIDE_LENGTH {
        let seed = n ^ 0x5B10;
        for file in 0..shape::side_files(n) {
            if let Some(slot) = super::edited_slot(&tree, &live, seed ^ (file << 24)) {
                sizes.push(tree.sizes[slot as usize]);
            }
        }
    }
    sizes.sort_unstable();
    let percentile = |at: usize| sizes[sizes.len() * at / 100];
    assert!(
        (6_000..=20_000).contains(&percentile(50)),
        "p50 {} over {} edits",
        percentile(50),
        sizes.len()
    );
    assert!(
        (30_000..=200_000).contains(&percentile(90)),
        "p90 {}",
        percentile(90)
    );
}

/// One ref per name, and every one of them hanging off a mark the
/// stream mints — a `from` naming a mark that was never minted is a
/// stream fast-import rejects at the end of a build.
#[test]
fn the_refs_are_all_there_and_all_reachable() {
    let mut tally = Tally::default();
    refs(&mut tally).expect("a tally never refuses a write");
    let annotated = tally.count("tag ");
    assert_eq!(annotated + tally.count("reset refs/tags/"), shape::TAGS);
    assert_eq!(annotated, tally.count("tagger "), "a tag with no tagger");
    // **Annotated is the expensive kind and it is what the reference
    // repository has**: 99.3% of its tags carry an object, so reading
    // one costs a peel where a lightweight tag is a single read.
    assert!(
        annotated * 1_000 / shape::TAGS >= shape::ANNOTATED_SHARE - 5,
        "{annotated} annotated of {}",
        shape::TAGS
    );
    assert_eq!(
        tally.count("reset refs/remotes/"),
        shape::REMOTE_BRANCHES - shape::SIDE_BRANCHES
    );
    let last = BASE_MARK + shape::TRUNK + shape::SIDE_BRANCHES * shape::SIDE_LENGTH;
    for n in [0, shape::TAGS / 2, shape::TAGS - 1] {
        let at = BASE_MARK + (n * (shape::TRUNK - 1) / shape::TAGS);
        assert!((BASE_MARK..=last).contains(&at), "tag {n} hangs off :{at}");
    }
}
