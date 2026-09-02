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

/// Distinct source paths the history rotates through, and distinct file
/// bodies. Both powers of two so the tree repeats often enough to pack
/// small — the history is here for its *count*, and a tree that never
/// repeated would cost gigabytes to say the same thing.
pub(super) const PATHS: u64 = 512;
pub(super) const BODIES: u64 = 64;

/// Source sizes, in bytes, at the shares the reference repository holds
/// them: half its Kotlin under 565 bytes, nine tenths under 4KB, and
/// ninety-nine hundredths under 24KB. **Four sizes, not a distribution**
/// — the percentiles land, nothing between them does, and the tail is
/// cut at 1.5MB where the reference repository reaches 3.9MB.
///
/// **The tail is the point.** A diff of a small file says nothing about
/// what the grammar path costs; the pane parses the whole source and
/// spans every line of it, so one large file is where highlighting shows
/// up in both time and memory (internal-docs/P3-確認事項.md §文法経路の
/// 巨大ファイル). The corpus carries one so the record can open it
/// deliberately rather than never meeting one.
const SIZES: [(u64, usize); 4] = [(33, 565), (25, 4_035), (5, 24_570), (1, 1_500_000)];

/// The body every other one is small beside, and the path it sits at.
/// Named so the record can open exactly this diff (`perf --file`) and
/// say what the grammar path costs, beside the ordinary one.
pub(super) const HUGE_BODY: u64 = BODIES - 1;

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
    format!("{head}{body}")
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
    unique_name(n ^ 0x7A6_5F1, spread(n ^ 0x7A6_5F1, 8, 13, 40), n)
}

/// A remote-tracking branch, under the same remote name the reference
/// repository uses for the bulk of its own.
pub(super) fn remote_branch(n: u64) -> String {
    let want = spread(n ^ 0x2C4_A1B, 6, 9, 30);
    format!("JetBrains/rr/{}", unique_name(n ^ 0xABCD, want, n))
}

/// An author. The reference repository has 277 of them and their names
/// run 15 characters in the middle.
pub(super) fn author(n: u64) -> (String, String) {
    let who = n % AUTHORS;
    let name = words_to(who ^ 0x4155_5448, spread(who, 9, 5, 8), ' ');
    let mail = format!("{who}@example.com");
    (name, mail)
}

/// How many bytes body `n` runs to, from the shares in [`SIZES`].
pub(super) fn body_bytes(n: u64) -> usize {
    let mut seen = 0;
    for (share, bytes) in SIZES {
        seen += share;
        if n % BODIES < seen {
            return bytes;
        }
    }
    SIZES[0].1
}

/// One source file's body, in a language the diff pane highlights
/// through a grammar rather than the lexer fallback, grown to the size
/// its index calls for.
pub(super) fn body(n: u64) -> String {
    let want = body_bytes(n);
    let mut text = format!("package org.example.p{n}\n\nclass Sample{n} {{\n");
    let mut line = 0;
    while text.len() < want {
        text.push_str(&format!(
            "    fun {}(): Int = {}\n",
            words_to(n ^ (line << 8), 12, '_'),
            mix(n ^ line) % 1000
        ));
        line += 1;
    }
    text.push_str("}\n");
    text
}

/// Where a body sits. Deep enough that the pane has a path to elide,
/// which is what the reference repository's own paths do.
///
/// **Whole words between the separators.** Cutting one leaves a segment
/// the vocabulary never contained, and git refuses a path whose segment
/// is a Windows device name — `context` cut to ten characters is
/// `con/text`, and `fast-import` stops on it (measured). The trailing
/// number keeps the file name out of the same trap.
pub(super) fn path(n: u64) -> String {
    let word = |salt: u64| WORDS[(mix(n ^ salt) % WORDS.len() as u64) as usize];
    format!(
        "compiler/{}-{}/src/org/jetbrains/kotlin/{}/{}{}.kt",
        word(0x9001),
        word(0x9002),
        word(0x9003),
        word(0x9004),
        n
    )
}

/// The same body after the newest commit touched it: [`NEWEST_REMOVED`]
/// of its lines gone from the middle and [`NEWEST_ADDED`] new ones in
/// their place. Bounded whatever the file's size, so the diff of the
/// large source is a large *file* and a small *change* — which is the
/// shape that makes highlighting, not diffing, the cost.
pub(super) fn body_edited(n: u64) -> String {
    let mut lines: Vec<String> = body(n).lines().map(str::to_string).collect();
    // Inside the class body, not over the closing brace: a source the
    // grammar cannot parse is not a measurement of the grammar.
    let at = (lines.len() / 2).max(3).min(lines.len().saturating_sub(1));
    let removed = (NEWEST_REMOVED as usize).min(lines.len().saturating_sub(at + 1));
    let added: Vec<String> = (0..NEWEST_ADDED)
        .map(|line| {
            format!(
                "    fun {}(): Int = {}",
                words_to(n ^ (line << 16) ^ 0xED17, 14, '_'),
                mix(n ^ line ^ 0xED17) % 1000
            )
        })
        .collect();
    lines.splice(at..at + removed, added);
    let mut text = lines.join("\n");
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::{author, body_edited, path, remote_branch, subject, tag};

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
        let mut remotes = lengths(7_000, |n| format!("refs/remotes/{}", remote_branch(n)));
        let p50 = at(&mut remotes, 50);
        assert!((30..=50).contains(&p50), "remote p50 was {p50}");
    }

    #[test]
    fn authors_are_the_length_a_row_has_room_for() {
        let mut names = lengths(20_000, |n| author(n).0);
        let p50 = at(&mut names, 50);
        assert!((11..=19).contains(&p50), "author p50 was {p50}");
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
        assert_ne!(path(1), path(2));
    }

    /// The diff the interaction opens has to reach the grammar path, and
    /// that is chosen by the extension.
    #[test]
    fn sources_are_a_language_the_diff_pane_highlights() {
        assert!(path(7).ends_with(".kt"), "{}", path(7));
        assert!(super::body(7).contains("class Sample7"));
    }

    /// Half the sources under 565 bytes, nine tenths under 4KB, and one
    /// large enough that opening its diff is a measurement of the
    /// grammar path rather than of the pane.
    #[test]
    fn sources_carry_the_tail_the_reference_repository_has() {
        let mut sizes: Vec<usize> = (0..super::BODIES).map(|n| super::body(n).len()).collect();
        sizes.sort_unstable();
        assert!(
            sizes[sizes.len() / 2] <= 700,
            "p50 was {}",
            sizes[sizes.len() / 2]
        );
        assert!(
            sizes[sizes.len() * 9 / 10] <= 4_500,
            "p90 was {}",
            sizes[sizes.len() * 9 / 10]
        );
        let biggest = sizes[sizes.len() - 1];
        assert!(biggest > 1_000_000, "the tail was only {biggest}");
        // And it is reachable by name, so the record can open it.
        assert!(super::body(super::HUGE_BODY).len() > 1_000_000);
    }

    /// The newest commit's change is a few lines whatever the file is,
    /// so opening the large source measures the grammar walking a
    /// megabyte to colour a handful of changed lines.
    #[test]
    fn the_newest_commit_changes_a_few_lines_of_a_large_file() {
        for body in [3_u64, super::HUGE_BODY] {
            let before = super::body(body);
            let after = body_edited(body);
            let (was, now) = (before.lines().count(), after.lines().count());
            assert_eq!(
                now as i64 - was as i64,
                super::NEWEST_ADDED as i64 - super::NEWEST_REMOVED as i64,
                "body {body} changed by the wrong number of lines"
            );
            assert!(after.starts_with(&format!("package org.example.p{body}")));
            assert!(
                after.trim_end().ends_with('}'),
                "body {body} lost its brace"
            );
        }
        assert!(body_edited(super::HUGE_BODY).len() > 1_000_000);
    }
}
