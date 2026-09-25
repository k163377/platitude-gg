//! The dimensions the benchmark corpus is built to, and the strings that
//! fill them: `JetBrains/kotlin` class (CLAUDE.md §性能予算), rounded so a
//! re-take measures the same corpus. The reference repository is named
//! in `ci/baseline/perf-windows-x64.md` §計測条件.
//!
//! **Lengths matter more than words.** Memory is mostly the ref table and
//! the on-screen subjects (ci/baseline/perf-windows-x64.md §Rust ヒープ),
//! so the generator reproduces the distribution of lengths, held by the
//! tests at the foot; the words only have to read like a repository.

/// Reachable commits: over the budget's hundred thousand.
pub(super) const COMMITS: u64 = 200_000;

/// Refs, split the way the reference repository splits them (85.6% tags).
pub(super) const TAGS: u64 = 42_800;
pub(super) const REMOTE_BRANCHES: u64 = 7_200;

pub(super) const AUTHORS: u64 = 277;

/// One merge this often: the reference repository's history is nearly
/// linear (14 merges in 20,000 commits).
pub(super) const MERGE_EVERY: u64 = 1_429;

/// Branch tips the graph's window holds, and how long each chain is: the
/// reference repository's newest 2,000 commits are 232 unmerged chains
/// of about eight, tipped by remote-tracking refs. A one-line window
/// draws a fraction of the lanes and chips and flatters every number
/// (ci/baseline/code-costs-windows-x64.md §コーパス生成).
pub(super) const SIDE_BRANCHES: u64 = 232;
pub(super) const SIDE_LENGTH: u64 = 8;

/// How many files a commit off the trunk changes, as (share in a
/// thousand, files). The window is nine parts side branch, so this is
/// what its details panes list; the reference repository's window
/// answers 3 files at the median, 20 at the ninth decile and 1,340 at
/// worst (`cargo xtask corpus --against`).
pub(super) const SIDE_FILES: [(u64, u64); 11] = [
    (300, 1),
    (120, 2),
    (100, 3),
    (120, 5),
    (100, 8),
    (80, 13),
    (80, 20),
    (50, 40),
    (30, 90),
    (15, 300),
    (5, 1_200),
];

/// How many candidates a commit weighs before it picks a file to edit,
/// keeping the largest: the reference repository's window opens 10,366
/// bytes at the median against a 565-byte median tracked file, and the
/// tree has the same size histogram, so this picks the decile. Held by
/// `stream::tests`.
pub(super) const EDIT_DRAWS: u64 = 12;

/// How many files the `n`th side-branch commit changes.
pub(super) fn side_files(n: u64) -> u64 {
    let mut at = mix(n ^ 0x51DE_F11E) % 1_000;
    for (share, files) in SIDE_FILES {
        if at < share {
            return files;
        }
        at -= share;
    }
    1
}

/// The fork distance [`fork_back`] falls back to.
///
/// **Fork distance sets the graph's width.** A lane stays open from a
/// branch's tip down to its fork point, so branches all forking from one
/// trunk commit keep all 232 lanes open, against the reference
/// repository's 23 on average and 33 at its widest. Forking from a near
/// neighbour closes each lane a few rows down.
///
/// **Each branch has to fork from its own commit**: branches sharing one
/// leave a single lane open between them, and the graph comes out too
/// narrow.
pub(super) const FORK_BACK: u64 = 27;

/// How far back each branch forks, as (share in a hundred, branches
/// back): a fixed distance gives a constant width, which demands nothing
/// of a renderer. The reference repository's window, counted by
/// `corpus::readings::graph`, is p25 21 / p50 25 / p75 27 / max 33.
pub(super) const FORK_BACKS: [(u64, u64); 7] = [
    (16, 6),
    (20, 14),
    (22, 26),
    (18, 38),
    (12, 52),
    (9, 70),
    (3, 100),
];

/// How far back branch `n` forks.
pub(super) fn fork_back(n: u64) -> u64 {
    let draw = mix(n ^ 0x00F0_4B0C) % 100;
    let mut seen = 0;
    for (share, back) in FORK_BACKS {
        seen += share;
        if draw < seen {
            return back;
        }
    }
    FORK_BACK
}

/// Tags whose commit is inside the graph's 2,000-row window: spread
/// evenly along the trunk, about thirty would land there, where the
/// reference repository puts 461.
pub(super) const TAGS_IN_WINDOW: u64 = 520;

/// The most refs the corpus puts on any one commit. The reference
/// repository's busiest carries 42, which is one `LabelIndex` bucket,
/// one `Chips` list and one row laying out 42 chips.
pub(super) const REFS_ON_ONE: u64 = 44;

/// Tags in a thousand carrying a tag object, as the reference
/// repository's 99.3%: an annotated tag costs a peel (`%(*objectname)`)
/// where a lightweight one resolves in a single read.
pub(super) const ANNOTATED_SHARE: u64 = 993;

/// How large one pack may grow before `fast-import` starts another. The
/// pack count is the build's (`corpus::build::BLOB_IMPORTS`); this is
/// the ceiling on any one of them.
pub(super) const MAX_PACK_SIZE: &str = "2g";

/// The trunk, once the side branches and the newest commit have taken
/// their share of [`COMMITS`].
pub(super) const TRUNK: u64 = COMMITS - SIDE_BRANCHES * SIDE_LENGTH - 1;

/// Files the newest commit — the one the interaction measurement opens —
/// changes, and how many lines each gains and loses.
///
/// **A few lines, whatever the file's size**: the diff pane's expensive
/// case is a small diff in a large file, where the grammar path parses
/// the whole source to colour a handful of lines.
pub(super) const NEWEST_FILES: u64 = 75;
pub(super) const NEWEST_ADDED: u64 = 47;
pub(super) const NEWEST_REMOVED: u64 = 3;

/// The smallest file [`edited`] will change: below four lines it hands
/// the file back unchanged, and a path the newest commit names without
/// changing is one the details pane never lists.
pub(super) const EDITABLE_FLOOR: usize = 256;

/// The smallest file the default scenario is allowed to open: between
/// the reference repository's median opened file (10,366 bytes) and its
/// ninth decile (60,895). Drawn uniformly from the tree, the measured
/// commit would open a file nobody edits.
pub(super) const OPENED_BYTES: usize = 24_576;

/// The first commit's date and the step between commits, fixed so the
/// corpus regenerates to the same object ids (`corpus::token`).
pub(super) const FIRST_COMMIT_AT: u64 = 1_400_000_000;
pub(super) const STEP_SECS: u64 = 97;

/// SplitMix64 over a counter: stateless and the same on every machine,
/// which `corpus::token` depends on.
pub(super) fn mix(seed: u64) -> u64 {
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A length: two uniform draws summed, for the triangular middle a
/// length distribution has, and on one name in eight a third that
/// reaches the tail — the reference repository's refs run to 177
/// characters against a median of 30, and the long ones decide what the
/// ref table costs.
fn spread(seed: u64, low: u64, each: u64, tail: u64) -> usize {
    let a = mix(seed) % each;
    let b = mix(seed ^ 0x5DEE_CE66_D000_0000) % each;
    let long = if mix(seed ^ 0x1D87_2B41).is_multiple_of(8) {
        mix(seed ^ 0xA5A5_5A5A) % tail
    } else {
        0
    };
    (low + a + b + long) as usize
}

/// One whole word out of the vocabulary: a cut word can be a Windows
/// device name (`context` → `con`), and git refuses the whole stream on
/// one.
pub(super) fn word(seed: u64) -> &'static str {
    WORDS[(mix(seed) % WORDS.len() as u64) as usize]
}

const WORDS: [&str; 32] = [
    "resolve", "inline", "session", "parser", "codegen", "backend", "frontend", "analysis",
    "compiler", "runtime", "checker", "builder", "context", "symbol", "scope", "module", "binary",
    "native", "script", "plugin", "target", "config", "wasm", "kotlin", "gradle", "daemon",
    "cache", "index", "report", "sample", "fixture", "bridge",
];

/// Words joined and cut to about `want` characters: callers want a
/// length first.
fn words_to(seed: u64, want: usize, joiner: char) -> String {
    let mut text = String::with_capacity(want + 8);
    let mut n = 0;
    while text.len() < want {
        if n > 0 {
            text.push(joiner);
        }
        text.push_str(WORDS[(mix(seed ^ (n << 32)) % WORDS.len() as u64) as usize]);
        n += 1;
    }
    text.truncate(want);
    while text.ends_with(joiner) {
        text.pop();
    }
    text
}

/// A commit subject, its conventional-commit prefix counted in its
/// length: the reference repository's run 59 at the median and 91 at the
/// 95th percentile.
pub(super) fn subject(n: u64) -> String {
    let want = spread(n ^ 0x51_75_62_6A, 21, 36, 60);
    let kind = ["fix", "feat", "test", "chore", "refactor"][(mix(n) % 5) as usize];
    let head = format!("{kind}: ");
    let body = words_to(n ^ 0x9E37, want.saturating_sub(head.len()), ' ');
    format!("{}{head}{body}", opener(n))
}

/// What a subject opens with, which is nothing on all but a few. The
/// emoji goes to the system's colour font (glyphs cached as ARGB, not
/// 8-bit alpha); the em dash is the cheap case the UI family serves
/// itself.
fn opener(n: u64) -> &'static str {
    if mix(n ^ 0x000E_3031).is_multiple_of(EMOJI_EVERY) {
        return EMOJI[(mix(n ^ 0x000E_3032) % EMOJI.len() as u64) as usize];
    }
    if mix(n ^ 0xDA5_117).is_multiple_of(DASH_EVERY) {
        return "— ";
    }
    ""
}

/// One subject in this many opens with an emoji, and one in this many
/// with an em dash. The reference repository's window carries three
/// non-ASCII subjects in two thousand rows, two of them emoji.
const EMOJI_EVERY: u64 = 1_000;
const DASH_EVERY: u64 = 1_000;
const EMOJI: [&str; 2] = ["🍒 ", "🎯 "];

/// The whole commit message: the subject, a body, and the credits. The
/// graph walk (`parse::log`) reads `%b` and the `Co-authored-by`
/// trailers for every window row and holds them three times over
/// (`CommitMeta`, `LogRow`, `GraphRowItem`), so one-line messages would
/// measure none of it.
pub(super) fn message(n: u64) -> String {
    let mut text = subject(n);
    let want = body_bytes_of(n);
    if want > 0 {
        text.push_str("\n\n");
        text.push_str(&wrapped(n ^ 0xB0D_1E5, want));
    }
    let credits = co_authors(n);
    if credits > 0 {
        text.push('\n');
        for credit in 0..credits {
            let (who, mail) = author(mix(n ^ (0xC0_A0_71 + credit)));
            text.push_str(&format!("\nCo-authored-by: {who} <{mail}>"));
        }
    }
    text
}

/// How long a body runs, in bytes, at the shares the reference
/// repository holds them: 32 in the middle, 1,178 at the 95th, 6,072 at
/// the longest, and 240 bytes a row on average over the window.
const BODY_SIZES: [(u64, u64); 6] = [
    (50, 32),
    (25, 160),
    (15, 480),
    (7, 1_100),
    (2, 2_200),
    (1, 6_000),
];

/// How wide a body's lines run before they wrap: one 6,000-character
/// line is a different layout problem from ninety short ones.
const BODY_COLUMNS: usize = 68;

/// Rows in a thousand that credit somebody besides the author. Above
/// the reference repository's 172 in 2,000, because the window's rows
/// are not the ones this is drawn over and the corpus has to come out at
/// least as heavy.
const CO_AUTHORED_PER_MILLE: u64 = 105;

fn body_bytes_of(n: u64) -> usize {
    let draw = mix(n ^ 0xB0D_1E5) % 100;
    let mut seen = 0;
    for (share, bytes) in BODY_SIZES {
        seen += share;
        if draw < seen {
            return (bytes / 2 + mix(n ^ 0x1E_46) % bytes) as usize;
        }
    }
    0
}

fn co_authors(n: u64) -> u64 {
    if mix(n ^ 0xC0_A0_71) % 1_000 >= CO_AUTHORED_PER_MILLE {
        return 0;
    }
    1 + mix(n ^ 0x71_C0) % 2
}

/// Words to about `want` bytes, wrapped at [`BODY_COLUMNS`].
fn wrapped(seed: u64, want: usize) -> String {
    let mut text = String::with_capacity(want + 16);
    let mut n = 0u64;
    let mut column = 0usize;
    while text.len() < want {
        let word = WORDS[(mix(seed ^ (n << 32)) % WORDS.len() as u64) as usize];
        if column > 0 && column + 1 + word.len() > BODY_COLUMNS {
            text.push('\n');
            column = 0;
        } else if column > 0 {
            text.push(' ');
            column += 1;
        }
        text.push_str(word);
        column += word.len();
        n += 1;
    }
    text.truncate(want);
    while text.ends_with(' ') || text.ends_with('\n') {
        text.pop();
    }
    text
}

/// A name of about `want` characters that no other `n` can answer: the
/// words alone do not reach fifty thousand combinations, so the counter
/// is spelled in and the words cut to leave room.
fn unique_name(seed: u64, want: usize, n: u64) -> String {
    let tail = format!("-{n}");
    let words = words_to(seed, want.saturating_sub(tail.len()).max(3), '-');
    format!("{words}{tail}")
}

/// A tag name: a median of 30 characters counting `refs/tags/`, and a
/// tail well past that.
pub(super) fn tag(n: u64) -> String {
    let name = unique_name(n ^ 0x7A6_5F1, spread(n ^ 0x7A6_5F1, 8, 13, 40), n);
    if ZERO_WIDTH_TAGS.contains(&n) {
        // Invisible, yet rendered, sorted and elided — and unequal to the
        // name it looks like.
        return name.replacen('-', "-\u{200B}", 1);
    }
    name
}

/// Which tags carry a zero-width space, as two of the reference
/// repository's do.
const ZERO_WIDTH_TAGS: [u64; 2] = [7, 20_101];

/// A remote-tracking branch, under the reference repository's remote
/// names.
///
/// **The slashes are the sidebar's rows**: `nav::tree::build_tree` makes
/// a `NavItem` per distinct prefix, without compacting single-child
/// chains, on every refs snapshot. The reference repository has 1,295
/// folder rows over 7,823 leaves, eight segments deep.
pub(super) fn remote_branch(n: u64) -> String {
    let want = spread(n ^ 0x2C4_A1B, 8, 10, 34);
    let mut name = String::from(if n.is_multiple_of(REMOTE_ORIGIN_EVERY) {
        REMOTES[1]
    } else {
        REMOTES[0]
    });
    for level in 1..remote_depth(n) {
        name.push('/');
        name.push_str(&folder(n, level));
    }
    name.push('/');
    name.push_str(&unique_name(n ^ 0xABCD, want, n));
    name
}

/// One remote-tracking branch in this many belongs to the second
/// remote. The reference repository has two configured and its second
/// carries a handful.
const REMOTE_ORIGIN_EVERY: u64 = 977;

/// The remotes the corpus configures, under the reference repository's
/// names; the first holds the tags and the upstream (`corpus::remotes`).
pub(super) const REMOTES: [&str; 2] = ["JetBrains", "origin"];

/// How far `main` sits ahead of its upstream, as the reference
/// repository's does. A branch level with its upstream draws no
/// ahead-behind badge and answers the count with a walk that stops at
/// once.
pub(super) const AHEAD_OF_UPSTREAM: u64 = 1_943;

/// Every ref the corpus carries: the tags, the remote-tracking branches,
/// `main`, its upstream, and each remote's `HEAD`.
pub(super) const REFS: u64 = TAGS + REMOTE_BRANCHES + 2 + REMOTES.len() as u64;

/// How many segments a remote-tracking branch's name runs to, at the
/// shares the reference repository holds them.
const REMOTE_DEPTHS: [(u64, u64); 7] = [
    (612, 1),
    (2_024, 2),
    (4_597, 3),
    (533, 4),
    (55, 5),
    (1, 6),
    (1, 8),
];

/// How many distinct names an interior segment may take, by level. Small
/// pools are what make prefixes *repeat*, and a prefix that repeats is
/// one folder row.
const FOLDERS_BY_LEVEL: [u64; 3] = [64, 20, 8];

fn remote_depth(n: u64) -> u64 {
    let draw = mix(n ^ 0xD3_9711) % 7_823;
    let mut seen = 0;
    for (share, depth) in REMOTE_DEPTHS {
        seen += share;
        if draw < seen {
            return depth;
        }
    }
    1
}

fn folder(n: u64, level: u64) -> String {
    let pool = FOLDERS_BY_LEVEL[(level as usize - 1).min(FOLDERS_BY_LEVEL.len() - 1)];
    let which = mix(n ^ (level << 48) ^ 0x00F0_1DE7) % pool;
    // The counter is in the name: thirty-two words cut to seven
    // characters give at most thirty-two strings, and a pool that
    // collapses loses folder rows.
    format!(
        "{}-{which}",
        words_to(which ^ (level << 32) ^ 0x00F0_1DE7, 7, '-')
    )
}

/// Author names that are not ASCII: a glyph the UI family cannot serve
/// loads a fallback font — tens of megabytes of working set — from the
/// first visible row holding one, at startup. A pure-ASCII corpus never
/// weighs it.
///
/// Four scripts, because they land on different fallbacks: Latin-1,
/// Latin Extended-A, Cyrillic and CJK. The emoji is in [`subject`].
const ACCENTED: [&str; 12] = [
    "Zoltán Bakó",
    "Jürgen Kästner",
    "Michał Zieliński",
    "Tomáš Doležal",
    "Сергей Волков",
    "Анна Крылова",
    "梶原 詩織",
    "林 承恩",
    "Björn Öhman",
    "Renée Lévesque",
    "Ana Muñoz",
    "Ólafur Þórðarson",
];

/// An author: the reference repository's names run 15 characters at the
/// median, and the 12 non-ASCII ones land on about one window row in
/// twenty-three, against its one in twenty-two.
pub(super) fn author(n: u64) -> (String, String) {
    let who = n % AUTHORS;
    (name_of(who), format!("{who}@example.com"))
}

/// Who committed it: somebody other than the author on
/// [`APPLIED_SHARE`] rows in a hundred, which the details card shows as
/// a second identity.
pub(super) fn committer(n: u64) -> (String, String) {
    if mix(n ^ 0xC0_11_17) % 100 >= APPLIED_SHARE {
        return author(n);
    }
    let who = mix(n ^ 0x05EC_011D) % AUTHORS;
    (name_of(who), format!("{who}@example.com"))
}

/// Rows in a hundred whose committer is not their author. The reference
/// repository's window carries 945 in 2,000.
const APPLIED_SHARE: u64 = 47;

fn name_of(who: u64) -> String {
    match ACCENTED.get(who as usize) {
        Some(name) => (*name).to_string(),
        None => words_to(who ^ 0x4155_5448, spread(who, 9, 5, 8), ' '),
    }
}

/// Files a commit changes, and in how many clusters: the reference
/// repository's commits touch 16.90 files across 4.95 directories, and
/// changes scattered over the whole tree would rewrite a tree object per
/// file.
pub(super) const CHANGED_FILES: u64 = 18;
pub(super) const CLUSTERS: u64 = 5;

/// How far apart the clusters of one commit may sit, in slots. Slots
/// are handed out directory by directory, so a short reach keeps a
/// commit's clusters under shared parent trees.
pub(super) const CLUSTER_REACH: u64 = 400;

/// Touches in a hundred that delete a tracked path: pure modifications
/// give `--find-renames` nothing to score and never open an added
/// file's diff against `/dev/null`.
pub(super) const DELETED_SHARE: u64 = 12;

/// One commit in this many renames a path outright: a delete and an add
/// of the same content, which is what git scores as a rename. The
/// reference repository carries one in nineteen.
pub(super) const RENAME_EVERY: u64 = 19;

/// One commit in this many rewrites a megabyte file: the reference
/// repository's pack is mostly historical revisions of large files.
/// Each revision goes whole through `fast-import`'s single-threaded
/// reader (it takes no deltas), so the rate trades pack size against
/// build time.
pub(super) const HUGE_EVERY: u64 = 30;

/// The build output a working repository accumulates, as the reference
/// repository's 78,000: ignored and still paid for, since `status::load`
/// (`-uall`) stats each against the ignore rules.
///
/// **Beside the sources, matched by pattern**: a rule naming a directory
/// prunes the walk to one stat
/// (ci/baseline/code-costs-windows-x64.md §コーパス生成).
pub(super) const IGNORED_FILES: u64 = 78_000;

/// What the tracked ignore rules say. Patterns, for the reason
/// above.
pub(super) const GITIGNORE: &str = "*.class\n*.jar.tmp\n*.stamp\n";

/// Ignore files below the root, as the reference repository's sixteen:
/// git pushes and pops a per-directory exclude stack as it walks, which
/// a single root rule never exercises.
pub(super) const NESTED_IGNORES: u64 = 16;
pub(super) const NESTED_GITIGNORE: &str = "*.tmp\n!keep.tmp\n*.local\n";

/// One file's bytes, as of a revision, into a buffer the caller reuses.
pub(super) fn content_into(out: &mut Vec<u8>, slot: u64, want: usize, rev: u32, mode: &str) {
    out.clear();
    if mode == "120000" {
        out.extend_from_slice(b"../");
        out.extend_from_slice(word(slot).as_bytes());
        out.push(b'/');
        out.extend_from_slice(word(slot ^ 0x11).as_bytes());
        push_u64(out, slot);
        return;
    }
    if super::tree::extension(slot) == PICTURES[0] && want >= PICTURE_FLOOR {
        image_into(out, slot, want);
        return;
    }
    source(out, slot, want, rev);
}

/// The smallest picture: `image_into` writes the PNG signature and an
/// IHDR, then noise to the drawn size, and a body longer than asked for
/// is a file of the wrong size. Smaller ones go down the source path.
const PICTURE_FLOOR: usize = 29;

fn image_into(out: &mut Vec<u8>, slot: u64, want: usize) {
    out.extend_from_slice(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    out.extend_from_slice(&[0, 0, 0, 13, b'I', b'H', b'D', b'R']);
    out.extend_from_slice(&[0, 0, 1, 0, 0, 0, 1, 0, 8, 6, 0, 0, 0]);
    let mut n = 0u64;
    while out.len() < want {
        out.extend_from_slice(&mix(slot ^ n).to_le_bytes());
        n += 1;
    }
    out.truncate(want);
}

/// Whether a slot's bytes change from one revision to the next: not for
/// a picture, a symlink, or a source below [`REVISION_FLOOR`]. The blob
/// passes need this before git does (`stream::shard_of`).
pub(super) fn carries_revision(slot: u64, want: usize, mode: &str) -> bool {
    mode != "120000"
        && want >= REVISION_FLOOR
        && !(super::tree::extension(slot) == PICTURES[0] && want >= PICTURE_FLOOR)
}

/// A body short enough that its package line — slot number and all —
/// may be cut off, so two slots' bytes can be one object.
pub(super) const TINY: usize = 64;

/// The smallest source whose revision is spelled on two lines (every
/// sixteenth line, about thirty-four bytes each): one line can repeat
/// across revisions — thirty-two words, a thousand numbers — where two
/// do not in practice.
const REVISION_FLOOR: usize = 1_024;

#[cfg(test)]
pub(super) fn content(slot: u64, want: usize, rev: u32, mode: &str) -> Vec<u8> {
    let mut out = Vec::new();
    content_into(&mut out, slot, want, rev, mode);
    out
}

/// Extensions the preview pane reads as an image — a different path
/// through `preview::file_preview`.
const PICTURES: [&str; 1] = ["png"];

/// One source file's body, in a language the diff pane highlights
/// through a grammar, grown to the size the tree calls for.
///
/// **No allocation of its own**: this is the build's innermost loop, and
/// a `format!` per line is hundreds of millions of allocations.
fn source(out: &mut Vec<u8>, slot: u64, want: usize, rev: u32) {
    if want == 0 {
        return;
    }
    out.reserve(want + 64);
    out.extend_from_slice(b"package org.example.p");
    push_u64(out, slot);
    out.extend_from_slice(b"\n\nclass Sample");
    push_u64(out, slot);
    out.extend_from_slice(b" {\n");
    let mut line = 0u64;
    while out.len() < want {
        // The revision rides on one line in `CHURN_STRIDE`, so a later
        // revision is a delta of its predecessor.
        let churn = if line.is_multiple_of(CHURN_STRIDE) {
            u64::from(rev)
        } else {
            0
        };
        out.extend_from_slice(b"    fun ");
        push_words(out, slot ^ (line << 8) ^ (churn << 40), 12, b'_');
        out.extend_from_slice(b"(): Int = ");
        push_u64(out, mix(slot ^ line ^ (churn << 20)) % 1000);
        out.push(b'\n');
        line += 1;
    }
    out.truncate(want);
}

/// A number, by hand: `write!` goes through `fmt::Arguments`, and this
/// runs hundreds of millions of times.
fn push_u64(out: &mut Vec<u8>, mut value: u64) {
    let mut digits = [0u8; 20];
    let mut at = digits.len();
    loop {
        at -= 1;
        digits[at] = b'0' + u8::try_from(value % 10).unwrap_or(0);
        value /= 10;
        if value == 0 {
            break;
        }
    }
    out.extend_from_slice(&digits[at..]);
}

/// Words joined to about `want` bytes, appended to the buffer.
fn push_words(out: &mut Vec<u8>, seed: u64, want: usize, joiner: u8) {
    let began = out.len();
    let mut n = 0u64;
    while out.len() - began < want {
        if n > 0 {
            out.push(joiner);
        }
        out.extend_from_slice(
            WORDS[(mix(seed ^ (n << 32)) % WORDS.len() as u64) as usize].as_bytes(),
        );
        n += 1;
    }
    out.truncate(began + want);
    while out.last() == Some(&joiner) {
        out.pop();
    }
}

/// How often a line carries the revision: the first stride whose
/// revisions pack at or above the reference repository's megabyte files
/// (63,254 bytes a revision).
const CHURN_STRIDE: u64 = 16;

/// The same body after the newest commit touched it: [`NEWEST_REMOVED`]
/// of its lines gone from the middle and [`NEWEST_ADDED`] new ones in
/// their place, whatever the file's size.
pub(super) fn edited(slot: u64, want: usize, rev: u32, mode: &str) -> Vec<u8> {
    let mut whole = Vec::new();
    content_into(&mut whole, slot, want, rev, mode);
    let Ok(text) = String::from_utf8(whole.clone()) else {
        return whole;
    };
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    if lines.len() < 4 {
        return whole;
    }
    // Inside the class body: a source the grammar cannot parse is
    // not a measurement of the grammar.
    let at = (lines.len() / 2).max(3).min(lines.len().saturating_sub(1));
    let removed = (NEWEST_REMOVED as usize).min(lines.len().saturating_sub(at + 1));
    let added: Vec<String> = (0..NEWEST_ADDED)
        .map(|line| {
            format!(
                "    fun {}(): Int = {}",
                words_to(slot ^ (line << 16) ^ 0xED17, 14, '_'),
                mix(slot ^ line ^ 0xED17) % 1000
            )
        })
        .collect();
    lines.splice(at..at + removed, added);
    let mut text = lines.join("\n");
    text.push('\n');
    text.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::{author, committer, message, remote_branch, subject, tag};

    /// The percentile of a sample, taken the way the record's own
    /// numbers are read.
    fn at(values: &mut [usize], percent: usize) -> usize {
        values.sort_unstable();
        values[values.len() * percent / 100]
    }

    fn lengths(count: u64, of: impl Fn(u64) -> String) -> Vec<usize> {
        (0..count).map(|n| of(n).chars().count()).collect()
    }

    /// Held to a few characters either side: what matters is the volume
    /// of text 2,000 rows hold.
    #[test]
    fn subjects_are_the_length_the_reference_repository_writes() {
        let mut lengths = lengths(20_000, subject);
        let (p50, p95) = (at(&mut lengths, 50), at(&mut lengths, 95));
        assert!((54..=64).contains(&p50), "p50 was {p50}");
        assert!((84..=98).contains(&p95), "p95 was {p95}");
    }

    /// Ref names: 30 in the middle, 58 at the 95th, counting the
    /// `refs/tags/` and `refs/remotes/` prefixes the caller adds.
    #[test]
    fn ref_names_are_the_length_the_reference_repository_carries() {
        let mut tags = lengths(20_000, |n| format!("refs/tags/{}", tag(n)));
        let (p50, p95) = (at(&mut tags, 50), at(&mut tags, 95));
        assert!((26..=36).contains(&p50), "tag p50 was {p50}");
        assert!((50..=64).contains(&p95), "tag p95 was {p95}");
        // At least the reference repository's median of 52.
        let mut remotes = lengths(7_000, |n| format!("refs/remotes/{}", remote_branch(n)));
        let p50 = at(&mut remotes, 50);
        assert!((52..=72).contains(&p50), "remote p50 was {p50}");
    }

    /// The glyphs that load a fallback font (why: `ACCENTED`, `opener`).
    #[test]
    fn a_window_of_rows_carries_the_glyphs_that_load_a_fallback() {
        let names = (0..2_000).filter(|n| !author(*n).0.is_ascii()).count();
        assert!((60..=140).contains(&names), "{names} author names of 2,000");
        let subjects = (0..2_000).filter(|n| !subject(*n).is_ascii()).count();
        assert!((2..=12).contains(&subjects), "{subjects} subjects of 2,000");
        // One outside the basic multilingual plane: the colour font.
        let emoji = (0..20_000).any(|n| subject(n).chars().any(|c| c as u32 > 0xFFFF));
        assert!(
            emoji,
            "no subject reaches past the basic multilingual plane"
        );
        // And a ref name the eye cannot tell from its neighbour.
        let invisible = (0..super::TAGS).any(|n| tag(n).contains('\u{200B}'));
        assert!(invisible, "no tag carries a zero-width space");
    }

    #[test]
    fn messages_carry_the_body_and_the_credits_a_window_holds() {
        let mut bodies = 0;
        let mut credited = 0;
        for n in 0..2_000 {
            let whole = message(n);
            bodies += whole.len() - subject(n).len();
            if whole.contains("Co-authored-by: ") {
                credited += 1;
            }
        }
        // At least as heavy as the reference repository, either count.
        assert!(bodies >= 481_089, "{bodies} bytes of body over 2,000 rows");
        assert!(credited >= 172, "{credited} rows credited of 2,000");
    }

    #[test]
    fn nearly_half_the_rows_were_committed_by_somebody_else() {
        let applied = (0..2_000).filter(|n| committer(*n) != author(*n)).count();
        assert!((820..=1_060).contains(&applied), "{applied} of 2,000");
    }

    #[test]
    fn a_side_branch_commit_changes_what_the_reference_repository_does() {
        let mut files: Vec<u64> = (0..super::SIDE_BRANCHES * super::SIDE_LENGTH)
            .map(super::side_files)
            .collect();
        files.sort_unstable();
        let percentile = |at: usize| files[files.len() * at / 100];
        assert!((2..=4).contains(&percentile(50)), "p50 {}", percentile(50));
        assert!(
            (13..=40).contains(&percentile(90)),
            "p90 {}",
            percentile(90)
        );
        // The tail is the point: a whole-subtree commit makes the pane
        // build a list.
        assert!(
            files[files.len() - 1] >= 300,
            "max {}",
            files[files.len() - 1]
        );
    }

    #[test]
    fn authors_are_the_length_a_row_has_room_for() {
        let mut names = lengths(20_000, |n| author(n).0);
        let p50 = at(&mut names, 50);
        assert!((11..=19).contains(&p50), "author p50 was {p50}");
    }

    #[test]
    fn the_remotes_section_is_a_tree_rather_than_a_list() {
        let mut folders = std::collections::HashSet::new();
        let mut deepest = 0;
        for n in 0..super::REMOTE_BRANCHES {
            let name = remote_branch(n);
            deepest = deepest.max(name.matches('/').count());
            for (at, _) in name.match_indices('/') {
                folders.insert(name[..at].to_string());
            }
        }
        assert!(folders.len() >= 1_295, "{} folder rows", folders.len());
        assert!(deepest >= 7, "{deepest} segments at the deepest");
    }

    #[test]
    fn every_ref_name_is_its_own() {
        let tags: std::collections::HashSet<String> = (0..super::TAGS).map(tag).collect();
        assert_eq!(tags.len() as u64, super::TAGS);
        let remotes: std::collections::HashSet<String> =
            (0..super::REMOTE_BRANCHES).map(remote_branch).collect();
        assert_eq!(remotes.len() as u64, super::REMOTE_BRANCHES);
    }

    /// A drifting generator would rename the corpus (`corpus::token`) on
    /// every rebuild.
    #[test]
    fn the_same_counter_always_answers_the_same_string() {
        assert_eq!(subject(12_345), subject(12_345));
        assert_eq!(tag(9_999), tag(9_999));
        assert_ne!(subject(1), subject(2));
        assert_ne!(
            super::content(1, 600, 0, "100644"),
            super::content(2, 600, 0, "100644")
        );
    }

    /// `data` counts bytes: a body off its drawn size is a stream
    /// fast-import rejects, long after it was written.
    #[test]
    fn a_body_is_the_size_the_tree_asked_for() {
        for want in [0, 1, 17, 565, 4_035, 24_570, 1_048_577] {
            let body = super::content(11, want, 0, "100644");
            assert_eq!(body.len(), want, "asked for {want}");
        }
    }

    #[test]
    fn a_revision_differs_from_the_one_before_without_replacing_it() {
        let first = String::from_utf8(super::content(23, 24_570, 0, "100644")).expect("ascii");
        let second = String::from_utf8(super::content(23, 24_570, 1, "100644")).expect("ascii");
        assert_ne!(first, second, "the revision changed nothing");
        // Counted in lines, not bytes: a changed line's length shifts
        // every offset after it.
        let was: Vec<&str> = first.lines().collect();
        let same = second.lines().filter(|line| was.contains(line)).count();
        assert!(
            same * 100 / was.len() > 70,
            "only {}% of the lines survived the revision",
            same * 100 / was.len()
        );
    }

    #[test]
    fn the_newest_commit_changes_a_few_lines_of_a_large_file() {
        for want in [24_570_usize, 1_048_577] {
            let before = String::from_utf8(super::content(5, want, 2, "100644")).expect("ascii");
            let after = String::from_utf8(super::edited(5, want, 2, "100644")).expect("ascii");
            let was = before.lines().count();
            let now = after.lines().count();
            assert_eq!(
                now as i64 - was as i64,
                super::NEWEST_ADDED as i64 - super::NEWEST_REMOVED as i64,
                "a file of {want} bytes changed by the wrong number of lines"
            );
        }
    }
}
