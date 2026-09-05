//! The dimensions the benchmark corpus is built to, and the strings that
//! fill them.
//!
//! The numbers are the ones the budget is written against — a repository
//! of `JetBrains/kotlin` class (CLAUDE.md §性能予算) — rounded so that the
//! record means the same thing next month. What they were taken from is
//! in `ci/baseline/perf-windows-x64.md` §計測条件.
//!
//! **Lengths matter more than words.** What the memory is mostly made of
//! is the ref table and the subjects of the rows on screen
//! (ci/baseline/perf-windows-x64.md §Rust ヒープ), so the generator's job
//! is to reproduce the *distribution of lengths*, which the tests at the
//! foot hold it to. The words themselves only have to read like a
//! repository so that a screenshot of the corpus is legible.

/// Reachable commits. Over the hundred thousand the budget names, and a
/// round number so a re-take a month from now measures the same corpus
/// rather than whatever the day's clone happened to hold.
pub(super) const COMMITS: u64 = 200_000;

/// Refs, split the way the reference repository splits them (85.6% tags).
/// The count is what the ref tables are sized by, and they are the
/// largest named thing on the Rust heap.
pub(super) const TAGS: u64 = 42_800;
pub(super) const REMOTE_BRANCHES: u64 = 7_200;

/// Distinct authors. Their names ride on every row the graph draws.
pub(super) const AUTHORS: u64 = 277;

/// One merge this often. The reference repository's `--all` history is
/// very nearly linear — 14 merges in 20,000 commits — because almost
/// every ref is a tag on one line of development.
pub(super) const MERGE_EVERY: u64 = 1_429;

/// Branch tips the graph's window holds, and how long each chain is.
///
/// **A window is not a column.** The reference repository's newest 2,000
/// commits are 232 separate chains of about eight commits with no merge
/// among them — review branches that were never merged, whose tips are
/// remote-tracking refs. A corpus whose window were one line of
/// development would draw a fraction of the lanes, the edges and the
/// chips a real one does, and would flatter every number taken off it
/// (measured: a single-column corpus weighed 95MB less).
pub(super) const SIDE_BRANCHES: u64 = 232;
pub(super) const SIDE_LENGTH: u64 = 8;

/// How many files a commit off the trunk changes, as (share in a
/// thousand, files).
///
/// **The window is nine parts side branch.** Its two thousand rows are
/// the newest by date and the trunk contributes a few hundred of them,
/// so what a row in the window changes is what a side branch changes —
/// and one file each makes every details pane in the measurement a list
/// of one. The reference repository's window answers three files at the
/// median, twenty at the ninth decile and 1,340 at its worst
/// (`cargo xtask corpus --against`), which is what these shares are.
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
/// keeping the largest.
///
/// **Set by where it lands, not by taste.** The reference repository's
/// window opens a 10,366-byte file at the median against a 565-byte
/// median tracked file; the corpus's tree carries the same size
/// histogram, so the only question is which decile of it the commits
/// reach for. Held by `stream::tests`.
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

/// How many branches back each one forks from.
///
/// **This, not the number of branches, is what the graph's width is.** A
/// lane stays open from a branch's tip down to the commit it forked
/// from, so branches that all fork from one trunk commit below the
/// window keep every lane open at once: 232 of them, against the
/// reference repository's 23 on average and 33 at its widest (measured
/// — and a corpus that wide rendered so slowly the bench never
/// finished). Forking from a near neighbour closes each lane a few rows
/// down, and the width settles at about this many branches' worth.
///
/// **Each branch has to fork from its own commit.** Branches that share
/// one leave one lane open between them, not one each: a fork distance
/// that varied in step with the branch number put nine of them on the
/// same commit and the graph came out four lanes wide instead of
/// twenty-three (measured). A fixed distance back, landing on a
/// different step of that branch each time, gives one open lane per
/// branch and a widest row wider than the average.
pub(super) const FORK_BACK: u64 = 27;

/// How far back each branch forks, at the shares that give the window
/// the reference repository's spread of lane widths.
///
/// **A fixed distance is a comb.** Every branch keeping exactly one
/// lane open for exactly the same number of rows gives a constant
/// width, and a constant demands nothing of a renderer. The reference
/// repository's window, walked by the same lane count `corpus::graph`
/// applies to both, is p25 21 / p50 25 / p75 27 / max 33; what a
/// spread of distances buys is that shape rather than a single number.
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

/// Tags whose commit is inside the graph's 2,000-row window, which is
/// what decides how many chips it draws.
///
/// **Spread over the whole history is not spread over the window.** The
/// window is the newest 2,000 commits and the trunk contributes only a
/// few hundred of them, so tags placed evenly along 198,000 commits put
/// thirty on screen where the reference repository puts 461.
pub(super) const TAGS_IN_WINDOW: u64 = 520;

/// The most refs the corpus puts on any one commit. The reference
/// repository's busiest carries 42, which is one `LabelIndex` bucket,
/// one `encode_labels` string and one row laying out 42 chips.
pub(super) const REFS_ON_ONE: u64 = 44;

/// Tags carrying a tag object rather than pointing straight at a
/// commit. The reference repository's are 99.3% annotated, and an
/// annotated tag costs a peel (`%(*objectname)`) where a lightweight
/// one resolves in a single read.
pub(super) const ANNOTATED_SHARE: u64 = 993;

/// How large one pack may grow before `fast-import` starts another.
/// The packs themselves come from the build's shape — one per blob pass
/// and one for the commits and trees (`corpus::BLOB_IMPORTS`), five
/// like the reference repository's, under a multi-pack-index like its —
/// and this is the ceiling on any one of them. Every git process an
/// opening spawns maps all of them before it resolves anything.
pub(super) const MAX_PACK_SIZE: &str = "2g";

/// The trunk, once the side branches and the newest commit have taken
/// their share of [`COMMITS`].
pub(super) const TRUNK: u64 = COMMITS - SIDE_BRANCHES * SIDE_LENGTH - 1;

/// Files the newest commit changes, and how many lines each of them
/// gains and loses. That commit is what the interaction measurement
/// opens, so this is the shape of the details pane and of the diff being
/// timed.
///
/// **A few lines, whatever the file's size.** The expensive case for the
/// diff pane is not a large diff but a small one *in a large file*: the
/// grammar path parses the whole source and spans every line of it to
/// colour the handful that changed. A newest commit that rewrote its
/// files whole would measure the diff and hide the highlighting.
pub(super) const NEWEST_FILES: u64 = 75;
pub(super) const NEWEST_ADDED: u64 = 47;
pub(super) const NEWEST_REMOVED: u64 = 3;

/// The smallest file [`edited`] will change. Below four lines it has
/// nowhere to splice and hands the file back as it found it, and a path
/// the newest commit names but does not change is one the details pane
/// never lists.
pub(super) const EDITABLE_FLOOR: usize = 256;

/// The smallest file the default scenario is allowed to open.
///
/// **What people edit is not what a tree is mostly made of.** The
/// reference repository's median tracked file is 565 bytes and the
/// median file its window opens is 10,366; drawn uniformly from the
/// tree, the commit the measurement selects opens three kilobytes, and
/// the operation-response number is then a reading of a file nobody
/// edits. This sits between that repository's median opened file and
/// its ninth decile (60,895), and beside the 25KB its own tip carried
/// when this was first measured.
pub(super) const OPENED_BYTES: usize = 24_576;

/// The first commit's date and the step between commits. Fixed rather
/// than taken from the clock: a corpus that regenerates to the same
/// object ids is one the record can name and a cleared machine can get
/// back (`corpus::token`).
pub(super) const FIRST_COMMIT_AT: u64 = 1_400_000_000;
pub(super) const STEP_SECS: u64 = 97;

/// One value out of a counter. SplitMix64 — a few instructions, no
/// state to carry, and the same answer on every machine, which is the
/// whole point: the corpus is identified by a token taken over its refs
/// (`corpus::token`), and that token only holds still if this does.
pub(super) fn mix(seed: u64) -> u64 {
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A length. Two draws added together rather than one, because a sum of
/// uniforms is triangular and that is the shape a length distribution
/// has in the middle — one draw would put as many 12-character subjects
/// on screen as 60-character ones.
///
/// The third draw lands on one name in eight and is what reaches the
/// tail. A triangle alone cannot: the reference repository's refs run to
/// 177 characters where their median is 30, and it is the long ones that
/// decide what the ref table costs.
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

/// One whole word out of the vocabulary. **Whole**, because a cut word
/// can be a Windows device name and git refuses the entire stream on
/// one — `context` cut to three characters is `con` (measured).
pub(super) fn word(seed: u64) -> &'static str {
    WORDS[(mix(seed) % WORDS.len() as u64) as usize]
}

const WORDS: [&str; 32] = [
    "resolve", "inline", "session", "parser", "codegen", "backend", "frontend", "analysis",
    "compiler", "runtime", "checker", "builder", "context", "symbol", "scope", "module", "binary",
    "native", "script", "plugin", "target", "config", "wasm", "kotlin", "gradle", "daemon",
    "cache", "index", "report", "sample", "fixture", "bridge",
];

/// Words joined to about `want` characters, cut to fit. Every caller
/// wants a length first and a shape second, so the cut is the point
/// rather than a failure to plan.
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
    // A trailing joiner reads as a mistake rather than as a name.
    while text.ends_with(joiner) {
        text.pop();
    }
    text
}

/// A commit subject. The reference repository's are 59 characters in the
/// middle and 91 at the 95th percentile; the conventional-commit prefix
/// is part of that length, not on top of it.
pub(super) fn subject(n: u64) -> String {
    let want = spread(n ^ 0x51_75_62_6A, 21, 36, 60);
    let kind = ["fix", "feat", "test", "chore", "refactor"][(mix(n) % 5) as usize];
    let head = format!("{kind}: ");
    let body = words_to(n ^ 0x9E37, want.saturating_sub(head.len()), ' ');
    format!("{}{head}{body}", opener(n))
}

/// What a subject opens with, which is nothing on all but a few.
///
/// **The emoji is the expensive one.** No Latin UI family carries it, so
/// the row that holds it hands it to the system's colour font, whose
/// glyphs are cached as ARGB rather than as 8-bit alpha. The em dash is
/// the cheap half of the same axis: the family serves it itself.
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

/// The whole commit message: the subject, a body, and the credits.
///
/// **The body is not decoration.** `parse::log` asks the graph walk for
/// `%b` and the `Co-authored-by` trailers on every one of the window's
/// 2,000 rows, and what comes back is held three times over — in
/// `CommitMeta`, in `LogRow` inside the sent-row cache, and again in
/// `RowItem` — before the details pane asks for `%B`. A corpus whose
/// every message is one line carries none of that and measures none of
/// it.
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

/// How wide a body's lines run before they wrap. A body is paragraphs,
/// and six thousand characters on one line is a different layout
/// problem from ninety short ones.
const BODY_COLUMNS: usize = 68;

/// Rows in a thousand that credit somebody besides the author. The
/// reference repository's window carries 172 in two thousand; the rate
/// is above that because the rows the window holds are not the rows
/// this is drawn over, and the corpus may not come out lighter (86
/// measured 151 of 2,000 in the window, which was under).
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

/// A name of about `want` characters that no other `n` can answer.
///
/// **Every ref has to be distinct or there are fewer of them than the
/// corpus says.** Thirty-two words joined to a dozen characters do not
/// reach fifty thousand combinations — measured: 7,200 remote branches
/// collapsed into 799 — so the counter is spelled into the name and the
/// words are cut to leave room for it.
fn unique_name(seed: u64, want: usize, n: u64) -> String {
    let tail = format!("-{n}");
    let words = words_to(seed, want.saturating_sub(tail.len()).max(3), '-');
    format!("{words}{tail}")
}

/// A tag name. Tags are where the ref table's bytes are: 42,800 of them
/// at a median of 30 characters counting `refs/tags/`, and a tail well
/// past that.
pub(super) fn tag(n: u64) -> String {
    let name = unique_name(n ^ 0x7A6_5F1, spread(n ^ 0x7A6_5F1, 8, 13, 40), n);
    if ZERO_WIDTH_TAGS.contains(&n) {
        // A character the sidebar renders, sorts and elides, and that a
        // reader cannot see — the reference repository carries two of
        // them and a name that looks equal to another is a name that
        // compares unequal.
        return name.replacen('-', "-\u{200B}", 1);
    }
    name
}

/// Which tags carry a zero-width space, as two of the reference
/// repository's do.
const ZERO_WIDTH_TAGS: [u64; 2] = [7, 20_101];

/// A remote-tracking branch, under the same remote names the reference
/// repository uses.
///
/// **The slashes are the sidebar's rows.** `nav::tree::build_tree`
/// makes one `NavItem` per distinct directory prefix, with two heap
/// strings each, and rebuilds the whole tree on every arrange — which
/// is every refs snapshot. It does not compact single-child chains, so
/// every segment is its own row. A name shaped `remote/rr/leaf` for all
/// of them gives the section two folder rows; the reference repository
/// has 1,295 over 7,823 leaves, and runs eight segments deep.
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

/// The remotes the corpus configures, first one first. Their names are
/// the reference repository's own, and the first is where the tags and
/// the upstream live (`corpus::remotes`).
pub(super) const REMOTES: [&str; 2] = ["JetBrains", "origin"];

/// How far `main` sits ahead of its upstream, as the reference
/// repository's does. A branch level with its upstream draws no
/// ahead-behind badge and answers the count with a walk that stops at
/// once.
pub(super) const AHEAD_OF_UPSTREAM: u64 = 1_943;

/// Every ref the corpus carries, which is what the ref tables are sized
/// by: the tags, the remote-tracking branches, `main`, the upstream
/// `main` tracks, and the `HEAD` each remote is read as pointing at.
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
/// one folder row rather than one per branch.
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
    // The counter is in the name because the words are not enough to
    // tell a pool of forty-eight apart: thirty-two of them cut to seven
    // characters answer at most thirty-two distinct strings, and a pool
    // that collapses is a folder row that never appears (measured — 833
    // rows where the tables describe eighteen hundred).
    format!(
        "{}-{which}",
        words_to(which ^ (level << 32) ^ 0x00F0_1DE7, 7, '-')
    )
}

/// Author names that are not ASCII, and why they have to exist.
///
/// **The graph pane has no gate of the kind the diff pane keeps.**
/// `encode::has_wide` exists so that setting a wide glyph — and with it
/// loading a fallback font, tens of megabytes of working set in every
/// window, repository open or not (`DiffTextMetrics`, measured) —
/// happens only where a diff carries one. Nothing asks that question
/// before the graph draws a row's author or subject, so the first
/// visible row holding a glyph the UI family cannot serve loads the
/// fallback at startup. A corpus of pure ASCII never loads one and so
/// never weighs one.
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

/// An author. The reference repository has 277 of them and their names
/// run 15 characters in the middle; 12 of these carry a glyph outside
/// ASCII, which lands on about one window row in twenty-three, against
/// the reference repository's one in twenty-two.
pub(super) fn author(n: u64) -> (String, String) {
    let who = n % AUTHORS;
    (name_of(who), format!("{who}@example.com"))
}

/// Who committed it, which is not who wrote it on [`APPLIED_SHARE`]
/// rows in a hundred — a patch somebody else applied, which the details
/// card shows as a second identity beside the author's.
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

/// Files a commit changes, and how many places in the tree it changes
/// them in. The reference repository's commits touch 16.90 files across
/// 4.95 directories; changes scattered over the whole tree would
/// rewrite one tree object per file and cost the pack what a real
/// commit does not.
pub(super) const CHANGED_FILES: u64 = 18;
pub(super) const CLUSTERS: u64 = 5;

/// How far apart the clusters of one commit may sit, in slots. Slots
/// are handed out directory by directory, so a short reach keeps a
/// commit's changes in one subtree — which is what makes its clusters
/// share parent trees rather than rewriting five deep paths whole.
pub(super) const CLUSTER_REACH: u64 = 400;

/// Touches of a tracked path that delete it rather than rewriting it.
/// The reference repository's first-parent history is 35.8% deletes
/// against 36.3% adds — a corpus of pure modifications gives
/// `--find-renames` nothing to score and never opens an added file's
/// diff against `/dev/null`.
pub(super) const DELETED_SHARE: u64 = 12;

/// One commit in this many renames a path outright: a delete and an add
/// of the same content, which is what git scores as a rename. The
/// reference repository carries one in nineteen.
pub(super) const RENAME_EVERY: u64 = 19;

/// One commit in this many rewrites a megabyte file. The reference
/// repository's pack is mostly historical revisions of large files —
/// without them the corpus packs to three quarters of its size however
/// many small files it churns (measured: 3.52GiB against 4.66).
///
/// **And it is most of the import stream.** Every revision is written
/// whole, because `fast-import` takes no deltas, so each of these puts
/// a megabyte or more through a single-threaded reader. The rate is
/// what holds the pack above the reference repository's without the
/// stream growing past what a ten-minute build can carry.
pub(super) const HUGE_EVERY: u64 = 30;

/// The build output a working repository accumulates. Ignored, and
/// therefore not free: `status::read` asks for every untracked path
/// (`-uall`), so git stats each of these and matches it against the
/// ignore rules before deciding it has nothing to say. The reference
/// repository carries about 78,000.
///
/// **Beside the sources, not under one directory.** A rule naming a
/// directory prunes the walk — git stats the directory once and skips
/// everything beneath it, so 78,000 files under `build/` cost one stat
/// and the status stays at half the reference repository's (measured:
/// 0.44s against 1.01s). A pattern is what makes git walk them.
pub(super) const IGNORED_FILES: u64 = 78_000;

/// What the tracked ignore rules say. Patterns rather than directories,
/// for the reason above.
pub(super) const GITIGNORE: &str = "*.class\n*.jar.tmp\n*.stamp\n";

/// Ignore files below the root. **git builds a per-directory exclude
/// stack**, pushing and popping one of these as it walks, and a
/// repository with a single root rule never makes it do that: the
/// reference repository carries sixteen and its status is twice this
/// one's over the same number of files.
pub(super) const NESTED_IGNORES: u64 = 16;
pub(super) const NESTED_GITIGNORE: &str = "*.tmp\n!keep.tmp\n*.local\n";

/// One file's bytes, as of a revision, into a buffer the caller reuses.
///
/// **Per revision, not per path.** A body that never changed would make
/// every later commit that names it a tree change and no blob, and the
/// object database — five million objects in the reference repository,
/// mapped by every git process the application spawns — would be a
/// hundredth of the size.
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

/// Enough of a PNG that the preview pane reads it as one: the eight
/// signature bytes and an IHDR, then noise to the size the table drew.
///
/// **Only where the table drew room for one.** A file smaller than the
/// header cannot be a picture, and a body that came out longer than it
/// was asked for is a file of the wrong size — the caller sends the
/// small ones down the source path instead.
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

/// Whether a slot's bytes change from one revision to the next.
///
/// A source carries its revision on its first `fun` line
/// (`CHURN_STRIDE`), so a file too short to reach past that line's
/// words is the same bytes at every revision; a picture and a symlink
/// never carry one. Two placements with the same bytes are one object,
/// and the blob passes have to know that before git does
/// (`stream::shard_of`).
pub(super) fn carries_revision(slot: u64, want: usize, mode: &str) -> bool {
    mode != "120000"
        && want >= REVISION_FLOOR
        && !(super::tree::extension(slot) == PICTURES[0] && want >= PICTURE_FLOOR)
}

/// A body short enough that its slot's number may be cut off it: the
/// package line is cut at the size the tree drew, so two slots' bytes
/// can be one object, and only the bytes themselves say so.
pub(super) const TINY: usize = 64;

/// The smallest source whose revision is spelled on two lines rather
/// than one: every sixteenth line carries it, at about thirty-four
/// bytes a line. One line can repeat across revisions — its words are
/// drawn from thirty-two and its number from a thousand, and a floor of
/// one line left 174 objects in two packs (measured) — where two do not
/// in practice.
const REVISION_FLOOR: usize = 1_024;

#[cfg(test)]
pub(super) fn content(slot: u64, want: usize, rev: u32, mode: &str) -> Vec<u8> {
    let mut out = Vec::new();
    content_into(&mut out, slot, want, rev, mode);
    out
}

/// Extensions the preview pane reads as an image rather than as text,
/// which is a different path through `preview::file_preview`: the blob
/// is written to a file of the run's own and shown from there.
const PICTURES: [&str; 1] = ["png"];

/// One source file's body, in a language the diff pane highlights
/// through a grammar rather than the lexer fallback, grown to the size
/// the tree calls for.
///
/// **Written into a buffer the caller keeps, and with no allocation of
/// its own.** This is the innermost loop of the whole build: four
/// million placements of a few dozen lines each, so a `format!` per
/// line is hundreds of millions of allocations.
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
        // revision is a delta of its predecessor rather than a rewrite
        // — which is what makes the pack the shape a real one is.
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

/// A number, without the formatting machinery. `itoa` by hand because
/// `write!` on a `Vec<u8>` goes through `fmt::Arguments` and this is
/// called a few hundred million times.
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

/// Words joined to about `want` bytes, appended rather than returned.
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

/// How often a line carries the revision. Measured on the reference
/// repository's own megabyte files: its historical revisions pack to
/// 63,254 bytes each, and this is the first stride at or above that.
const CHURN_STRIDE: u64 = 16;

/// The same body after the newest commit touched it: [`NEWEST_REMOVED`]
/// of its lines gone from the middle and [`NEWEST_ADDED`] new ones in
/// their place. Bounded whatever the file's size, so the diff of the
/// large source is a large *file* and a small *change* — which is the
/// shape that makes highlighting, not diffing, the cost.
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
    // Inside the class body, not over the closing brace: a source the
    // grammar cannot parse is not a measurement of the grammar.
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

    /// The reference repository's subjects: 59 in the middle, 91 at the
    /// 95th. Held to a few characters either side — what matters is the
    /// volume of text 2,000 rows hold, not the exact shape.
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
        // The reference repository's remote names run 52 in the middle
        // counting `refs/remotes/`, and the ref table is mostly their
        // bytes — so this may not come out shorter than that.
        let mut remotes = lengths(7_000, |n| format!("refs/remotes/{}", remote_branch(n)));
        let p50 = at(&mut remotes, 50);
        assert!((52..=72).contains(&p50), "remote p50 was {p50}");
    }

    /// **The glyphs that load a fallback font.** The graph pane draws
    /// every row's author and subject with no gate of the kind the diff
    /// pane keeps (`encode::has_wide`), so the first visible row with a
    /// glyph the UI family cannot serve loads the system fallback —
    /// tens of megabytes of working set, measured. A corpus of pure
    /// ASCII never loads one, and the reference repository loads two.
    #[test]
    fn a_window_of_rows_carries_the_glyphs_that_load_a_fallback() {
        let names = (0..2_000).filter(|n| !author(*n).0.is_ascii()).count();
        assert!((60..=140).contains(&names), "{names} author names of 2,000");
        let subjects = (0..2_000).filter(|n| !subject(*n).is_ascii()).count();
        assert!((2..=12).contains(&subjects), "{subjects} subjects of 2,000");
        // One of them outside the basic multilingual plane: that is the
        // colour font, whose glyphs are cached as ARGB rather than as
        // 8-bit alpha.
        let emoji = (0..20_000).any(|n| subject(n).chars().any(|c| c as u32 > 0xFFFF));
        assert!(
            emoji,
            "no subject reaches past the basic multilingual plane"
        );
        // And a ref name the eye cannot tell from its neighbour.
        let invisible = (0..super::TAGS).any(|n| tag(n).contains('\u{200B}'));
        assert!(invisible, "no tag carries a zero-width space");
    }

    /// The reference repository's window carries 481,089 bytes of
    /// message body and 172 rows crediting somebody besides the author.
    /// `parse::log` asks for both on every row and holds what comes back
    /// three times over, so a one-line message measures none of it.
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
        // Never lighter than the reference repository, on either count.
        assert!(bodies >= 481_089, "{bodies} bytes of body over 2,000 rows");
        assert!(credited >= 172, "{credited} rows credited of 2,000");
    }

    /// Nearly half the reference repository's rows were applied by
    /// somebody other than their author, and the details card shows the
    /// two identities separately.
    #[test]
    fn nearly_half_the_rows_were_committed_by_somebody_else() {
        let applied = (0..2_000).filter(|n| committer(*n) != author(*n)).count();
        assert!((820..=1_060).contains(&applied), "{applied} of 2,000");
    }

    /// The window is nine parts side branch, so what a side-branch
    /// commit changes is what the details pane in the measurement gets
    /// handed. The reference repository's window answers three files at
    /// the median and twenty at the ninth decile.
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
        // The tail is the point of the table: a commit that touches a
        // whole subtree is what makes the pane build a list rather than
        // a line.
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

    /// **The sidebar's rows are the slashes.** `nav::tree::build_tree`
    /// makes one `NavItem` per distinct directory prefix, two heap
    /// strings each, rebuilt on every arrange — and it does not compact
    /// single-child chains. The reference repository's remotes section
    /// carries 1,295 folder rows over 7,823 leaves and runs eight deep;
    /// names all shaped `remote/rr/leaf` would carry two.
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

    /// A corpus that says it has 50,000 refs has to have 50,000 names.
    #[test]
    fn every_ref_name_is_its_own() {
        let tags: std::collections::HashSet<String> = (0..super::TAGS).map(tag).collect();
        assert_eq!(tags.len() as u64, super::TAGS);
        let remotes: std::collections::HashSet<String> =
            (0..super::REMOTE_BRANCHES).map(remote_branch).collect();
        assert_eq!(remotes.len() as u64, super::REMOTE_BRANCHES);
    }

    /// The same counter always answers the same string: the corpus is
    /// named by a token over its refs, and a generator that drifted
    /// would rename the corpus every time it was rebuilt.
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

    /// A body is exactly the size the tree drew for it. **`data` counts
    /// bytes**, and every placement pairs the two — a body that ran over
    /// or under would be a stream fast-import rejects, a hundred seconds
    /// after it was written.
    #[test]
    fn a_body_is_the_size_the_tree_asked_for() {
        for want in [0, 1, 17, 565, 4_035, 24_570, 1_048_577] {
            let body = super::content(11, want, 0, "100644");
            assert_eq!(body.len(), want, "asked for {want}");
        }
    }

    /// **A later revision is a delta of the one before it**, not a
    /// rewrite. A body that never changed would leave the history one
    /// blob per path however many commits named it, and the object
    /// database is what every git process maps before it resolves
    /// anything.
    #[test]
    fn a_revision_differs_from_the_one_before_without_replacing_it() {
        let first = String::from_utf8(super::content(23, 24_570, 0, "100644")).expect("ascii");
        let second = String::from_utf8(super::content(23, 24_570, 1, "100644")).expect("ascii");
        assert_ne!(first, second, "the revision changed nothing");
        // Lines rather than bytes: a changed line is a different
        // length, so everything after it sits at a new offset — which
        // is what a delta encodes and a byte-for-byte comparison does
        // not see.
        let was: Vec<&str> = first.lines().collect();
        let same = second.lines().filter(|line| was.contains(line)).count();
        assert!(
            same * 100 / was.len() > 70,
            "only {}% of the lines survived the revision",
            same * 100 / was.len()
        );
    }

    /// The newest commit's change is a few lines whatever the file is,
    /// so opening the large source measures the grammar walking a
    /// megabyte to colour a handful of changed lines.
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
