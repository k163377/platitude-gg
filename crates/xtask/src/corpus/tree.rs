//! The working tree the corpus checks out: which files exist, where they
//! sit, and how large each one is.
//!
//! **This is the axis the startup number hangs off.** `status::read`
//! runs `git status --porcelain=v2 -z --branch -uall`, whose cost is one
//! `lstat` per tracked file and whose output the session holds. The
//! reference repository has 106,581 tracked files and a status to match;
//! a corpus of a few hundred files answers in a fraction of that, and the
//! difference is most of what the record calls startup
//! (ci/baseline/code-costs-windows-x64.md §git のプロセス代,
//! ci/baseline/perf-windows-x64.md §判定).
//!
//! **The tables are the reference repository's own histograms**, so the
//! shape is readable here without running anything: how deep its
//! directories go, how many files each holds, and how large those files
//! are. Each is walked by a stride coprime to its length, which makes
//! the walk a permutation — every band is drawn exactly as often as the
//! table says, rather than approximately.

use super::shape::{self, mix};

/// Tracked files at HEAD, which is what [`SIZES`] sums to.
pub(super) const TRACKED: u64 = 106_581;

/// Paths the history has room to move between. The tree holds more than
/// it tracks so that a commit can delete one path and add another and
/// leave the count where it was — which is what the reference
/// repository's history does, 36% adds against 36% deletes, and what a
/// corpus whose every commit is a modification never shows:
/// `--find-renames` with no add-delete pair to score, `orig_path` never
/// set, and the whole added-file side of the diff pane unreachable.
pub(super) const SPARE: u64 = 4_096;

/// Every path the tree can name, tracked or waiting.
pub(super) const SLOTS: u64 = TRACKED + SPARE;

/// Directories holding at least one file. The reference repository has
/// 14,609 of them under 22,887 trees.
///
/// **This is a multiplier on the build, not just on the tree.** Every
/// commit rewrites one tree object per directory on the path of every
/// file it touches, and `fast-import` hashes and deflates each of them
/// on a single thread — so the directory count decides both what the
/// walk costs the application and what the corpus costs to build.
///
/// **It is not a knob on its own.** The walk makes directories until
/// every file has a home, so what decides how many there are is this
/// against [`FANOUT`]: the mean fan-out has to come to `SLOTS / DIRS`,
/// or the walk runs past the table and keeps inventing directories
/// whatever this says.
const DIRS: u64 = 22_887;

/// Files whose size is in `[lo, 2*lo)`, at the counts the reference
/// repository holds them. Sums to [`TRACKED`], so the walk over it draws
/// every band exactly.
///
/// **The tail is the point.** `preview::SOURCE_BYTE_CAP` is 4MB, and the
/// reference repository's largest file sits just under it — an edge a
/// corpus whose largest file is a tenth of that never approaches.
const SIZES: [(u64, u64); 24] = [
    (310, 0),
    (40, 1),
    (242, 2),
    (93, 4),
    (601, 8),
    (2_153, 16),
    (5_446, 32),
    (8_874, 64),
    (13_392, 128),
    (19_265, 256),
    (19_233, 512),
    (14_827, 1_024),
    (9_647, 2_048),
    (5_880, 4_096),
    (3_394, 8_192),
    (1_881, 16_384),
    (892, 32_768),
    (284, 65_536),
    (89, 131_072),
    (21, 262_144),
    (6, 524_288),
    (9, 1_048_576),
    (1, 2_097_152),
    (1, 4_194_304),
];

/// Directories at each depth, the index being the depth. Sums to
/// [`DIRS`].
const DIR_DEPTHS: [u64; 20] = [
    2, 39, 340, 696, 743, 2_968, 4_599, 3_233, 2_225, 2_363, 1_368, 1_529, 1_445, 649, 290, 254,
    118, 16, 8, 2,
];

/// Directories holding `[lo, 2*lo)` files, at the counts the reference
/// repository holds them.
/// The shares are the reference repository's; the sizes are doubled, so
/// the mean comes to `SLOTS / DIRS` and the walk ends where the table
/// does instead of running past it.
const FANOUT: [(u64, u64); 11] = [
    (5_929, 2),
    (3_846, 4),
    (2_221, 8),
    (1_281, 16),
    (718, 32),
    (372, 64),
    (159, 128),
    (60, 256),
    (18, 512),
    (4, 1_024),
    (1, 2_048),
];

/// Extensions per ten thousand files. The reference repository is mostly
/// Kotlin and test data, and the mix decides which files take the
/// grammar path in the diff pane and which take the lexer fallback.
const EXTS: [(u64, &str); 12] = [
    (6_006, "kt"),
    (2_393, "txt"),
    (392, "java"),
    (238, "kts"),
    (134, "info"),
    (73, "def"),
    (53, "out"),
    (52, "xml"),
    (51, "h"),
    (47, "log"),
    (44, "args"),
    (13, "png"),
];

/// Everything else, in the order a walk reaches it.
const EXT_TAIL: [&str; 12] = [
    "md",
    "swift",
    "js",
    "json",
    "cpp",
    "ts",
    "properties",
    "gradle",
    "hpp",
    "dot",
    "iml",
    "yaml",
];

/// Strides for the three walks. Each is coprime to the length it walks,
/// which is what makes the walk a permutation rather than a sample. The
/// two over directories differ so that depth and fan-out are not paired
/// by index (identity pairing put the median file fourteen deep where
/// the reference repository's is eight); the size walk is over slots,
/// a different table, so sharing a value with the depth walk pairs
/// nothing. **Changing any of them changes the corpus** (`corpus::token`).
const SIZE_STEP: u64 = 40_009;
const DEPTH_STEP: u64 = 40_009;
const FANOUT_STEP: u64 = 30_011;

/// Files carrying the executable bit, and the one symlink. The reference
/// repository has 144 and 1 against 106,436 ordinary files.
const EXEC_EVERY: u64 = TRACKED / 144;
const SYMLINK_SLOT: u64 = TRACKED / 2;

/// The tree, built once and carried: recomputing a hundred thousand
/// paths per commit would cost more than the import.
pub(super) struct Tree {
    pub(super) paths: Vec<String>,
    pub(super) sizes: Vec<u32>,
    /// Slots whose file is a megabyte or more — the ones the record can
    /// open deliberately to say what the grammar path costs.
    pub(super) huge: Vec<u64>,
}

pub(super) fn build() -> Tree {
    let mut paths = Vec::with_capacity(SLOTS as usize);
    let mut segments: Vec<String> = Vec::new();
    let mut d = 0;
    while (paths.len() as u64) < SLOTS {
        let dir = directory(d, &mut segments);
        let want = fanout(d).min(SLOTS - paths.len() as u64);
        for _ in 0..want {
            let slot = paths.len() as u64;
            paths.push(format!("{dir}{}", file_name(slot)));
        }
        d += 1;
    }
    let sizes: Vec<u32> = (0..SLOTS)
        .map(|slot| u32::try_from(size_of(slot)).unwrap_or(u32::MAX))
        .collect();
    // The tail has to be tracked at HEAD to be openable, so it is
    // looked for among the slots the base commit places.
    let huge = (0..TRACKED)
        .filter(|slot| sizes[*slot as usize] >= 1_048_576)
        .collect();
    Tree { paths, sizes, huge }
}

impl Tree {
    /// The file mode git is told for a slot. Modes are an axis of their
    /// own: a checkout on Windows turns the symlink into a plain file
    /// holding its target, which is a diff nothing else in the corpus
    /// produces.
    pub(super) fn mode(&self, slot: u64) -> &'static str {
        if slot == SYMLINK_SLOT {
            "120000"
        } else if slot.is_multiple_of(EXEC_EVERY) {
            "100755"
        } else {
            "100644"
        }
    }
}

/// One directory, sharing a prefix with the one before it.
///
/// The shared prefix is what makes the interior trees: a forest of
/// 14,609 leaf directories that each spelled their whole path afresh
/// would be 14,609 disjoint chains, where the reference repository's
/// 22,887 trees are mostly shared.
fn directory(d: u64, segments: &mut Vec<String>) -> String {
    let want = depth_of(d).max(1) as usize;
    // Keep a little of what the last one had, so siblings and cousins
    // exist rather than only chains.
    let keep = segments.len().min(want.saturating_sub(1));
    let keep = keep.saturating_sub((mix(d ^ 0x00D1_2E00) % 3) as usize);
    segments.truncate(keep);
    while segments.len() < want {
        let level = segments.len() as u64;
        segments.push(segment(d, level));
    }
    let mut path = segments.join("/");
    path.push('/');
    path
}

/// A path segment. Whole words and a number: a cut word can be a
/// Windows device name, and git refuses the whole stream on one.
fn segment(d: u64, level: u64) -> String {
    let seed = d ^ (level << 32) ^ 0x0000_05E6;
    format!(
        "{}-{}{d}",
        shape::word(seed),
        shape::word(seed ^ 0x5E6D_0001)
    )
}

fn file_name(slot: u64) -> String {
    let seed = slot ^ 0x000F_11E0;
    format!(
        "{}_{}{slot}.{}",
        shape::word(seed),
        shape::word(seed ^ 0xF11E_0001),
        ext_of(slot)
    )
}

fn depth_of(d: u64) -> u64 {
    let k = d.wrapping_mul(DEPTH_STEP) % DIRS;
    let mut seen = 0;
    for (depth, count) in DIR_DEPTHS.iter().enumerate() {
        seen += count;
        if k < seen {
            return depth as u64;
        }
    }
    1
}

fn fanout(d: u64) -> u64 {
    let k = d.wrapping_mul(FANOUT_STEP) % DIRS;
    let mut seen = 0;
    for (count, lo) in FANOUT {
        seen += count;
        if k < seen {
            return lo + mix(d ^ 0x00FA_A007) % lo;
        }
    }
    1
}

fn size_of(slot: u64) -> u64 {
    // Over `TRACKED`, because that is what the table sums to: the spare
    // slots take a second lap of it rather than falling off the end
    // into a band that does not exist.
    let k = slot.wrapping_mul(SIZE_STEP) % TRACKED;
    let mut seen = 0;
    for (count, lo) in SIZES {
        seen += count;
        if k < seen {
            return if lo == 0 {
                0
            } else {
                lo + mix(slot ^ 0x0000_512E) % lo
            };
        }
    }
    0
}

/// The extension a slot's file carries, which is what decides whether
/// the diff pane reads it through a grammar, through the lexer
/// fallback, or as a picture.
pub(super) fn extension(slot: u64) -> &'static str {
    ext_of(slot)
}

fn ext_of(slot: u64) -> &'static str {
    let k = mix(slot ^ 0x000E_7E09) % 10_000;
    let mut seen = 0;
    for (share, ext) in EXTS {
        seen += share;
        if k < seen {
            return ext;
        }
    }
    EXT_TAIL[(k as usize) % EXT_TAIL.len()]
}

#[cfg(test)]
mod tests {
    use super::{DIR_DEPTHS, DIRS, SIZES, SLOTS, TRACKED, build};

    /// The tables have to describe what they say they describe, or the
    /// walks over them are samples rather than permutations.
    #[test]
    fn the_tables_sum_to_what_they_are_walked_against() {
        assert_eq!(SIZES.iter().map(|(count, _)| count).sum::<u64>(), TRACKED);
        assert_eq!(DIR_DEPTHS.iter().sum::<u64>(), DIRS);
    }

    /// **The tracked file count is the startup number.** `status::read`
    /// pays one `lstat` per file and the index carries one entry, so a
    /// corpus with a fraction of the reference repository's files has a
    /// fraction of its `git status`.
    #[test]
    fn the_tree_is_the_size_the_reference_repository_is() {
        let tree = build();
        assert_eq!(tree.paths.len() as u64, SLOTS);
        assert_eq!(tree.sizes.len() as u64, SLOTS);
        const { assert!(TRACKED >= 106_581) };
        let unique: std::collections::HashSet<&String> = tree.paths.iter().collect();
        assert_eq!(unique.len() as u64, SLOTS, "every path is its own");
    }

    /// Eleven files of a megabyte or more, one of them within a
    /// whisker of `preview::SOURCE_BYTE_CAP`.
    #[test]
    fn the_tail_reaches_the_cap_the_application_carries() {
        let tree = build();
        assert_eq!(tree.huge.len(), 11, "{:?}", tree.huge.len());
        let largest = tree
            .huge
            .iter()
            .map(|slot| tree.sizes[*slot as usize])
            .max()
            .expect("eleven of them");
        assert!(largest >= 4_194_304, "the largest was {largest}");
    }

    /// No segment may be a Windows device name: git refuses the whole
    /// stream on one, a hundred seconds after it was written.
    #[test]
    fn no_path_carries_a_reserved_name() {
        const RESERVED: [&str; 6] = ["con", "aux", "nul", "prn", "com1", "lpt1"];
        let tree = build();
        for path in &tree.paths {
            for part in path.split('/') {
                let stem = part.split('.').next().unwrap_or(part).to_ascii_lowercase();
                assert!(!RESERVED.contains(&stem.as_str()), "{path}");
            }
        }
    }

    /// Paths at least as long as the reference repository's, because
    /// they are what the index, the status output and the details
    /// pane's tree are mostly made of.
    #[test]
    fn paths_are_no_shorter_than_the_reference_repositorys() {
        let tree = build();
        let mut lengths: Vec<usize> = tree.paths.iter().map(String::len).collect();
        lengths.sort_unstable();
        let p50 = lengths[lengths.len() / 2];
        assert!(p50 >= 89, "path p50 was {p50}");
    }
}
