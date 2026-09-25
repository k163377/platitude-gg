//! One diff at a time: which lines of the file to walk before a hunk
//! begins, and whether the file still says what the hunk says.

use std::hash::{Hash, Hasher};

use syntect::highlighting::Highlighter;
use syntect::parsing::SyntaxReference;

use crate::parse::diff::{DiffHunk, DiffLineKind, FilePatch};

use super::theme::{Assets, assets, syntax_for};
use super::walk::Walk;
use super::{DiffColors, LexCache, LineColors, PatchColors};

/// How many lines of lexing one patch may spend, everything counted — the
/// walk down to each hunk, every row painted, the other side's readings
/// too — so a reading stays inside what opening a diff can spend on colour
/// (ci/baseline/code-costs-windows-x64.md §着色). Rows past it go out
/// plain. It bounds each reading: a walk resumed from a [`LexCache`]
/// checkpoint starts its budget from there.
const LEX_LINE_BUDGET: usize = 5_000;

/// How many lines apart a [`LexCache`]'s checkpoints stand: a re-read
/// walks at most this far to reach where it left off.
const CHECKPOINT_STRIDE: usize = 512;

/// What [`colors_quick`] may spend — at the lexer's rate, what a pane can
/// wear as immediate (ci/baseline/code-costs-windows-x64.md §着色).
const QUICK_LINE_BUDGET: usize = 1_000;

/// Whether a grammar or the fallback set can colour this path — asked
/// before fetching the file behind a diff, which costs a process.
pub fn knows(path: &str) -> bool {
    super::grammar::claims(path) || super::theme::knows(path)
}

/// Whether colouring this diff in full would keep a reader waiting (the
/// walk down to its deepest hunk plus every row, past
/// [`QUICK_LINE_BUDGET`]); where it would, callers send [`colors_quick`]'s
/// answer first. A path a grammar claims is never deep — the whole side
/// parses at once. A file that outgrew its grammar counts as not deep
/// too, since telling it apart would cost the very parse in question.
pub fn deep(patches: &[FilePatch]) -> bool {
    let mut rows = 0usize;
    let mut deepest = 0usize;
    for patch in patches {
        if patch.is_binary || patch.unmerged || patch.is_combined {
            continue;
        }
        if super::grammar::claims(patch.path()) {
            continue;
        }
        for hunk in &patch.hunks {
            deepest = deepest.max(hunk.new_start.max(hunk.old_start) as usize);
            rows += hunk.lines.len();
        }
    }
    deepest + rows > QUICK_LINE_BUDGET
}

/// The colours a diff can have now: no file walked, each hunk read cold,
/// at most [`QUICK_LINE_BUDGET`]. Shown while [`colors_cached`] walks the
/// file; the caller re-sends only where the full answer differs.
pub fn colors_quick(patches: &[FilePatch]) -> DiffColors {
    DiffColors {
        patches: patches
            .iter()
            .map(|patch| patch_colors(patch, None, None, QUICK_LINE_BUDGET))
            .collect(),
    }
}

/// Colours every text line of `patches`. Best effort: an unknown
/// language, a binary patch or a regex that will not run comes out
/// uncoloured. CPU-bound — callers on an async runtime hand it to a
/// blocking thread. `source` is the whole of the side the diff's line
/// numbers count in ([`crate::preview::source_text`]); without it every
/// hunk starts cold.
pub fn colors(patches: &[FilePatch], source: Option<&str>) -> DiffColors {
    colors_cached(patches, source, None).0
}

/// [`colors`], starting from — and handing back — a [`LexCache`] from a
/// previous reading. One built over different text is dropped and
/// rebuilt, so callers just keep the latest one returned.
pub fn colors_cached(
    patches: &[FilePatch],
    source: Option<&str>,
    cache: Option<LexCache>,
) -> (DiffColors, Option<LexCache>) {
    // Without the file the cache rides through untouched for the next
    // read that has one.
    let Some(src) = source else {
        let colors = DiffColors {
            patches: patches
                .iter()
                .map(|patch| patch_colors(patch, None, None, LEX_LINE_BUDGET))
                .collect(),
        };
        return (colors, cache);
    };
    let hash = source_hash(src);
    let mut store = match cache {
        Some(c) if c.source == hash => c,
        _ => LexCache {
            source: hash,
            syntax: String::new(),
            states: Vec::new(),
        },
    };
    let colors = DiffColors {
        patches: patches
            .iter()
            .map(|patch| patch_colors(patch, Some(src), Some(&mut store), LEX_LINE_BUDGET))
            .collect(),
    };
    (colors, Some(store))
}

/// Whether a [`LexCache`] was walked down this same text. Deliberately not
/// [`crate::details::fingerprint`]: a wrong answer here costs only a
/// repaint (rules-refs/core.md「staleness のハッシュは問いごとに持つ」).
fn source_hash(text: &str) -> u64 {
    let mut hasher = std::hash::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

fn patch_colors(
    patch: &FilePatch,
    source: Option<&str>,
    cache: Option<&mut LexCache>,
    budget: usize,
) -> PatchColors {
    if patch.is_binary || patch.unmerged {
        return Vec::new();
    }
    // A combined diff goes first, grammar or not: its markers are read as
    // structure (see the module note).
    if patch.is_combined {
        let assets = assets();
        let syntax = syntax_for(&assets.syntaxes, patch.path());
        return combined_colors(assets, patch, source, syntax);
    }
    // The fast path, without loading the fallback set (`theme::assets`).
    if let Some(lang) = super::grammar::for_path(patch.path()) {
        // …unless the file has outgrown its grammar (`tree::reads`) and
        // the fallback set has a real syntax for it; with none, the
        // grammar's partial answer is still the best there is.
        let outgrown = source.is_some_and(|text| !super::tree::reads(lang, text))
            && super::theme::reads(patch.path());
        if !outgrown {
            return super::tree::patch_colors(lang, patch, source);
        }
    }
    let assets = assets();
    let Some(syntax) = syntax_for(&assets.syntaxes, patch.path()) else {
        return Vec::new();
    };
    unified_colors(assets, patch, source, syntax, budget, cache)
}

/// Colours for an ordinary unified diff: one walk down the side the
/// diff's line numbers count in, forked once per hunk for the other
/// side's rows.
fn unified_colors(
    assets: &Assets,
    patch: &FilePatch,
    source: Option<&str>,
    syntax: &SyntaxReference,
    mut budget: usize,
    mut cache: Option<&mut LexCache>,
) -> PatchColors {
    if let Some(cache) = cache.as_deref_mut()
        && cache.syntax != syntax.name
    {
        cache.syntax = syntax.name.clone();
        cache.states.clear();
    }
    let highlighter = Highlighter::new(&assets.theme);
    // Which of the hunk's two line numbers the file counts in — a
    // deleted file has no new side.
    let new_side = patch.new_path.is_some();
    let lines: Vec<&str> = source.map(|s| s.lines().collect()).unwrap_or_default();
    let mut file = Walk::new(assets, &highlighter, Some(syntax));
    let mut at = 0usize;
    let mut out = Vec::with_capacity(patch.hunks.len());
    for hunk in &patch.hunks {
        let start = if new_side {
            hunk.new_start
        } else {
            hunk.old_start
        };
        let start = (start.saturating_sub(1)) as usize;
        if let Some((ck, state)) = cache.as_ref().and_then(|c| c.jump(at, start)) {
            file = Walk::resume(assets, &highlighter, state.clone());
            at = *ck;
        }
        let mut warm = false;
        if !lines.is_empty() && at <= start && start <= lines.len() && start <= at + budget {
            while at < start {
                if at.is_multiple_of(CHECKPOINT_STRIDE)
                    && let Some(cache) = cache.as_deref_mut()
                {
                    cache.record(at, file.snapshot());
                }
                file.paint(lines[at]);
                at += 1;
                budget -= 1;
            }
            warm = hunk_agrees(hunk, &lines, start, new_side);
        }
        if warm {
            let (hunk_colors, walked) = read_hunk(&mut file, hunk, new_side, &mut budget);
            at += walked;
            out.push(hunk_colors);
        } else {
            // No file, out of budget, or the file disagrees: this hunk
            // reads cold; the file's walk stays put for the hunks after it.
            let mut cold = Walk::new(assets, &highlighter, Some(syntax));
            out.push(read_hunk(&mut cold, hunk, new_side, &mut budget).0);
        }
    }
    out
}

/// Reads one hunk's rows: this side's (context and its own changes)
/// through `file`, the other side's changes through a fork taken where
/// the hunk begins that also steps over the context lines. One walk for
/// both sides would let a deleted `/*` paint everything after it as a
/// comment.
///
/// Every reading spends one of `budget` (a context line costs two, one
/// per side); past it the rest goes out plain. Answers the colours and
/// how many of this side's lines were consumed, spent or not, so the
/// caller's place in the file stays true.
fn read_hunk(
    file: &mut Walk,
    hunk: &DiffHunk,
    new_side: bool,
    budget: &mut usize,
) -> (Vec<LineColors>, usize) {
    let mut other = file.fork();
    // Whether the fork is still in step: once the budget stops it on a
    // context line, its own changes below would read out of place.
    let mut other_live = true;
    let mut walked = 0usize;
    let mut out = Vec::with_capacity(hunk.lines.len());
    for line in &hunk.lines {
        // `\ No newline at end of file` is on neither side: git talking.
        if line.kind == DiffLineKind::NoNewline {
            out.push(LineColors::nothing());
            continue;
        }
        if line.kind.on_side(new_side) {
            walked += 1;
            if *budget == 0 {
                out.push(LineColors::nothing());
                continue;
            }
            *budget -= 1;
            let spans = file.paint(&line.text);
            if line.kind == DiffLineKind::Context {
                if other_live && *budget > 0 {
                    *budget -= 1;
                    other.paint(&line.text);
                } else {
                    other_live = false;
                }
            }
            out.push(LineColors {
                spans,
                fence: false,
            });
        } else if other_live && *budget > 0 {
            *budget -= 1;
            out.push(LineColors {
                spans: other.paint(&line.text),
                fence: false,
            });
        } else {
            out.push(LineColors::nothing());
        }
    }
    (out, walked)
}

/// Colours for a combined diff: every row is read, conflict regions as
/// structure ([`Walk`]).
fn combined_colors(
    assets: &Assets,
    patch: &FilePatch,
    source: Option<&str>,
    syntax: Option<&SyntaxReference>,
) -> PatchColors {
    let highlighter = Highlighter::new(&assets.theme);
    let new_side = patch.new_path.is_some();
    let lines: Vec<&str> = source.map(|s| s.lines().collect()).unwrap_or_default();
    // The file's own walk, kept apart from the one that colours the rows:
    // a hunk's rows interleave the sides, and feeding them back would
    // leave the reading somewhere the file never goes.
    let mut file = Walk::new(assets, &highlighter, syntax);
    let mut at = 0usize;
    let mut out = Vec::with_capacity(patch.hunks.len());
    for hunk in &patch.hunks {
        let (start, count) = if new_side {
            (hunk.new_start, hunk.new_count)
        } else {
            (hunk.old_start, hunk.old_count)
        };
        let start = start.saturating_sub(1) as usize;
        let mut rows = None;
        // Here the budget only caps how deep a hunk the walk reaches for.
        if !lines.is_empty() && start <= lines.len() && start <= LEX_LINE_BUDGET {
            while at < start {
                file.paint(lines[at]);
                at += 1;
            }
            if hunk_agrees(hunk, &lines, start, new_side) {
                rows = Some(file.fork());
            }
        }
        let mut rows = rows.unwrap_or_else(|| Walk::new(assets, &highlighter, syntax));
        out.push(hunk.lines.iter().map(|line| rows.read(line)).collect());
        // Step the file over the lines this hunk covers, so the next one
        // starts from where the file really is.
        for _ in 0..count {
            let Some(line) = lines.get(at) else {
                break;
            };
            file.paint(line);
            at += 1;
        }
    }
    out
}

/// Whether the file says at `start` what the hunk says is there. A save
/// between reading the diff and the file makes them disagree, and a
/// reading walked through the wrong text is worse than a cold one.
fn hunk_agrees(hunk: &DiffHunk, lines: &[&str], start: usize, new_side: bool) -> bool {
    let first = hunk.lines.iter().find(|line| line.kind.on_side(new_side));
    first.is_none_or(|line| lines.get(start) == Some(&line.text.as_str()))
}

#[cfg(test)]
#[path = "patch_tests.rs"]
mod tests;
