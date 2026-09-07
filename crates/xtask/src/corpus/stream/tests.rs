use std::io::Write;

use super::{
    BASE_MARK, BLOB_MARK_BASE, History, Inline, Marked, Recorded, base, changes, newest_commit,
    refs, side_branches,
};
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
    base(&mut tally, &tree, &mut Inline::default()).expect("a tally never refuses a write");
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
    let mut bodies = Inline::default();
    for n in 2..2_000 {
        changes(&mut tally, &tree, &mut live, n, &mut bodies)
            .expect("a tally never refuses a write");
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
    let mut bodies = Inline::default();
    let mut marks = Marks::default();
    base(&mut marks, &tree, &mut bodies).expect("a sink never refuses a write");
    for n in 2..2_000 {
        changes(&mut marks, &tree, &mut live, n, &mut bodies).expect("a sink never refuses");
    }
    side_branches(&mut marks, &tree, &live, &mut bodies).expect("a sink never refuses");
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

/// The generator alone: the whole stream into a counting sink, timed.
/// Ignored because it takes minutes; run by hand to attribute the
/// build's import phase between this side and git's —
/// `cargo test -p xtask time_the_generator_alone -- --ignored --nocapture`.
#[test]
#[ignore = "minutes: the whole stream, for attribution rather than assertion"]
fn time_the_generator_alone() {
    struct Count(u64);
    impl Write for Count {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0 += buf.len() as u64;
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let tree = tree::build();
    // waits(measured): printed for the record, judged by nothing
    let began = std::time::Instant::now();
    let mut sink = std::io::BufWriter::with_capacity(4 * 1024 * 1024, Count(0));
    let newest = super::write(&mut sink, &tree, &mut Inline::default())
        .expect("a counting sink never refuses a write");
    let bytes = sink.into_inner().map(|count| count.0).unwrap_or(0);
    eprintln!(
        "generator alone: {:.1}s for {:.2}GB; the newest commit changes {} files",
        began.elapsed().as_secs_f64(),
        bytes as f64 / 1e9,
        newest.paths.len() + 1
    );
}

/// **The parallel build's two halves count the same placements.** The
/// blob passes mint marks by index over the recorded sequence and the
/// commit stream names marks by its own count; had either walked the
/// history differently, every file in the corpus would be the wrong
/// bytes under the right name — and fast-import would not notice.
#[test]
fn the_marks_the_commits_name_are_the_ones_the_blob_passes_mint() {
    const COMMITS: u64 = 500;
    let tree = tree::build();
    // The inline stream over the head of the history: what a single
    // process would place.
    let mut tally = Tally::default();
    let mut inline = Inline::default();
    let mut live = History::new(&tree);
    base(&mut tally, &tree, &mut inline).expect("a tally never refuses a write");
    for n in 2..COMMITS {
        changes(&mut tally, &tree, &mut live, n, &mut inline).expect("a tally never refuses");
    }
    let placed = tally.count("M 100644 inline ")
        + tally.count("M 100755 inline ")
        + tally.count("M 120000 inline ");
    // The same head, recorded. The ignore files are literals in the
    // stream rather than placements.
    let mut recorded = Recorded::default();
    let mut live = History::new(&tree);
    base(&mut std::io::sink(), &tree, &mut recorded).expect("a sink never refuses");
    for n in 2..COMMITS {
        changes(&mut std::io::sink(), &tree, &mut live, n, &mut recorded)
            .expect("a sink never refuses");
    }
    assert_eq!(
        recorded.placements.len() as u64,
        placed - 1 - shape::NESTED_IGNORES
    );
    // The same head, marked: one mark per placement, in order.
    let mut named = MarkLines::default();
    let mut marked = Marked::default();
    let mut live = History::new(&tree);
    base(&mut named, &tree, &mut marked).expect("a sink never refuses");
    for n in 2..COMMITS {
        changes(&mut named, &tree, &mut live, n, &mut marked).expect("a sink never refuses");
    }
    let expected: Vec<u64> = (0..recorded.placements.len() as u64)
        .map(|index| BLOB_MARK_BASE + index)
        .collect();
    assert_eq!(named.marks, expected);
    // And three shards of the blob pass together mint each mark once,
    // each over a body of exactly the size the tree drew for it.
    let head = &recorded.placements[..2_000];
    let mut minted = Vec::new();
    for shard in 0..3 {
        let mut blobs = BlobLines::default();
        super::blobs(&mut blobs, &tree, head, shard, 3).expect("a sink never refuses");
        let mut scratch = Vec::new();
        let drawn: Vec<usize> = head
            .iter()
            .filter(|placement| super::shard_of(**placement, &tree, 3, &mut scratch) == shard)
            .map(|placement| tree.sizes[placement.body as usize] as usize)
            .collect();
        assert_eq!(blobs.sizes, drawn, "shard {shard}");
        minted.extend(blobs.marks);
    }
    minted.sort_unstable();
    assert_eq!(minted, expected[..2_000]);
}

/// **One object goes through one pass.** fast-import deduplicates only
/// what one process sees, so the bytes a rename places again, and the
/// bytes of a file that cannot carry its revision, have to be dealt to
/// the pass that already has them — and the revisions of a large file
/// have to be spread, or one pass carries half the huge files' history.
#[test]
fn placements_with_the_same_bytes_go_through_the_same_pass() {
    let tree = tree::build();
    let at = |body: u64, rev: u32, at: u64| super::Placement { body, rev, at };
    let mut scratch = Vec::new();
    let mut shard_of = |placement, shards| super::shard_of(placement, &tree, shards, &mut scratch);
    let source = |floor: u32, ceiling: u32| {
        (0..tree::TRACKED)
            .find(|slot| {
                (floor..ceiling).contains(&tree.sizes[*slot as usize])
                    && tree.mode(*slot) == "100644"
                    && tree::extension(*slot) == "kt"
            })
            .expect("the tree has sources of every size")
    };
    let small = source(shape::TINY as u32, 256);
    let huge = tree.huge[0];
    // Two slots whose bytes are one package line cut at the same place.
    let (tiny, twin) = {
        let mut tiny = (0..tree::TRACKED).filter(|slot| tree.sizes[*slot as usize] == 8);
        (
            tiny.next().expect("an eight-byte file"),
            tiny.next().expect("a second eight-byte file"),
        )
    };
    assert_eq!(
        shape::content(tiny, 8, 0, "100644"),
        shape::content(twin, 8, 5, "100644"),
        "the two are the same bytes, or the test asks the wrong question"
    );
    for shards in [2, 3, 4, 7] {
        // A rename: the same slot and revision under another name.
        assert_eq!(
            shard_of(at(huge, 3, huge), shards),
            shard_of(at(huge, 3, tree::TRACKED + 1), shards)
        );
        // Two slots that are the same bytes.
        assert_eq!(
            shard_of(at(tiny, 0, tiny), shards),
            shard_of(at(twin, 5, twin), shards),
            "{shards} shards"
        );
        // A file too small to carry a revision: every revision alike.
        let passes: std::collections::HashSet<usize> = (0..200)
            .map(|rev| shard_of(at(small, rev, small), shards))
            .collect();
        assert_eq!(passes.len(), 1, "{shards} shards");
        // A huge file: its revisions across every pass.
        let passes: std::collections::HashSet<usize> = (0..200)
            .map(|rev| shard_of(at(huge, rev, huge), shards))
            .collect();
        assert_eq!(passes.len(), shards, "{shards} shards");
    }
}

/// The marks a commit stream names: every `M <mode> :<mark> <path>`.
#[derive(Default)]
struct MarkLines {
    marks: Vec<u64>,
    partial: Vec<u8>,
}

impl Write for MarkLines {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        for byte in buf {
            if *byte == b'\n' {
                let line = std::mem::take(&mut self.partial);
                if let Ok(line) = std::str::from_utf8(&line)
                    && let Some(rest) = line.strip_prefix("M ")
                    && let Some((_, rest)) = rest.split_once(" :")
                    && let Some((mark, _)) = rest.split_once(' ')
                    && let Ok(mark) = mark.parse()
                {
                    self.marks.push(mark);
                }
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

/// What a blob stream mints and how large each body was, read the way
/// fast-import reads it: `data <n>` is followed by exactly `n` bytes,
/// which are skipped rather than scanned for lines.
#[derive(Default)]
struct BlobLines {
    marks: Vec<u64>,
    sizes: Vec<usize>,
    partial: Vec<u8>,
    skip: usize,
}

impl Write for BlobLines {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut rest = buf;
        while !rest.is_empty() {
            if self.skip > 0 {
                let taken = self.skip.min(rest.len());
                self.skip -= taken;
                rest = &rest[taken..];
                continue;
            }
            let byte = rest[0];
            rest = &rest[1..];
            if byte != b'\n' {
                self.partial.push(byte);
                continue;
            }
            let line = std::mem::take(&mut self.partial);
            let line = String::from_utf8(line).expect("the blob stream's lines are ASCII");
            if let Some(mark) = line.strip_prefix("mark :") {
                self.marks.push(mark.parse().expect("a mark is a number"));
            } else if let Some(size) = line.strip_prefix("data ") {
                let size: usize = size.parse().expect("a size is a number");
                self.sizes.push(size);
                self.skip = size;
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
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
