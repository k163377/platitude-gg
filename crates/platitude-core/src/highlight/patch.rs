//! One diff at a time: which lines of the file to walk before a hunk
//! begins, and whether the file still says what the hunk says.

use std::hash::{Hash, Hasher};

use syntect::highlighting::Highlighter;
use syntect::parsing::SyntaxReference;

use crate::parse::diff::{DiffHunk, DiffLineKind, FilePatch};

use super::theme::{Assets, assets, syntax_for};
use super::walk::Walk;
use super::{DiffColors, LexCache, LineColors, PatchColors};

/// How many lines of lexing one ordinary patch may spend, everything
/// counted — the walk down to each hunk, every row painted inside one,
/// the other side's readings too. The lexer runs at ~16,000 lines a
/// second (2026-08-23, release, this repo's own source), so this is
/// about 300ms — as long as opening a diff can spend on colour. Rows
/// past it go out plain, which is what every file the set does not know
/// looks like anyway.
///
/// One budget per patch, not per hunk: what it bounds is the whole cost
/// of colouring one file, whatever shape its hunks take (the unbounded
/// reading measured 1.2s over a 20,000-line rewrite). It bounds each
/// *reading*, not how deep colours can ever reach: a walk resumed from a
/// [`LexCache`] checkpoint starts its budget from there.
const LEX_LINE_BUDGET: usize = 5_000;

/// How many lines apart a [`LexCache`]'s checkpoints stand. The price of
/// one is a clone of the lexer's place (small); the saving is that a
/// re-read of the same file walks at most this far to reach where it
/// left off.
const CHECKPOINT_STRIDE: usize = 512;

/// What [`colors_quick`] may spend: a fraction of [`LEX_LINE_BUDGET`],
/// around 60ms — what a pane can wear as "immediate".
const QUICK_LINE_BUDGET: usize = 1_000;

/// Whether the language set can say anything about this path at all —
/// asked before the file behind a diff is fetched: reading it costs a
/// process, and a file nothing will colour is not worth one. A grammar
/// or the fallback set, either counts.
pub fn knows(path: &str) -> bool {
    super::grammar::claims(path) || super::theme::knows(path)
}

/// Whether colouring this diff in full would keep a reader waiting —
/// the walk down to its deepest hunk plus every row of it, against the
/// regex lexer's ~16,000 lines a second. Where it would, callers send
/// [`colors_quick`]'s answer first. A path a grammar claims is never
/// deep: the whole side parses in milliseconds, so the full answer is
/// the quick one.
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

/// The colours a diff can have *now*: no file walked, each hunk read
/// from its own top, and no more than [`QUICK_LINE_BUDGET`] of it. What
/// the pane shows while [`colors_cached`] walks the file — almost
/// always the same answer, and the caller re-sends only where it turns
/// out not to be.
pub fn colors_quick(patches: &[FilePatch]) -> DiffColors {
    DiffColors {
        patches: patches
            .iter()
            .map(|patch| patch_colors(patch, None, None, QUICK_LINE_BUDGET))
            .collect(),
    }
}

/// Reads every text line of `patches` and answers what colour each run of
/// it is. Best effort throughout: a language that is not in the set, a
/// binary patch, a regex that will not run — each of those is a diff that
/// comes out the colour it always was, never an error and never a gap.
///
/// This is CPU work with no waiting in it. Callers on an async runtime
/// should hand it to a blocking thread rather than hold a worker.
/// `source` is the whole of the side the diff's line numbers count in
/// ([`crate::preview::source_text`]); without it every hunk starts cold
/// (see the module note).
pub fn colors(patches: &[FilePatch], source: Option<&str>) -> DiffColors {
    colors_cached(patches, source, None).0
}

/// [`colors`], starting from — and handing back — the lexer states a
/// previous reading of the same source remembered ([`LexCache`]). The
/// cache carries its own staleness check: one built over different text
/// (its hash disagrees) is dropped and rebuilt, so callers keep the
/// latest returned cache and nothing else.
pub fn colors_cached(
    patches: &[FilePatch],
    source: Option<&str>,
    cache: Option<LexCache>,
) -> (DiffColors, Option<LexCache>) {
    // Without the file there is nothing to remember states down — and
    // nothing to invalidate either: the cache rides through untouched
    // for the next read that does have one.
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

/// Whether a [`LexCache`] was walked down this same text.
///
/// **Deliberately not [`crate::details::fingerprint`]** (rules-refs/core.md
/// holds the decision). That one hashes the bytes of a `git diff`, goes out
/// to the pane as hex and comes back attached to a hunk selection, so a
/// wrong "unchanged" there lets a partial write land on bytes that moved.
/// This one hashes the file's text, never leaves the struct
/// (`LexCache::source` is private and the type is neither `Clone` nor
/// serialisable), and both sides of every comparison are made by the same
/// binary in the same run — so a wrong answer costs a repaint, and
/// `DefaultHasher`'s freedom to change between std releases cannot reach it.
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
    // A combined diff — the form git prints for a path it stopped on —
    // has its markers read as structure, and its fences are read whether
    // or not the language is one the set knows (see the module note).
    if patch.is_combined {
        let assets = assets();
        let syntax = syntax_for(&assets.syntaxes, patch.path());
        return combined_colors(assets, patch, source, syntax);
    }
    // The fast path: a grammar parses the whole side at once — no walk
    // to budget, nothing for the cache to remember, and none of the
    // fallback set's load time (`theme::assets`) spent.
    if let Some(lang) = super::grammar::for_path(patch.path()) {
        return super::tree::patch_colors(lang, patch, source);
    }
    let assets = assets();
    let Some(syntax) = syntax_for(&assets.syntaxes, patch.path()) else {
        return Vec::new();
    };
    unified_colors(assets, patch, source, syntax, budget, cache)
}

/// Colours for an ordinary unified diff: one walk down the side the
/// diff's line numbers count in, forked once per hunk for the other
/// side's rows. `budget` is [`LEX_LINE_BUDGET`], a parameter so a test
/// does not need thousands of lines to reach the far side of it.
fn unified_colors(
    assets: &Assets,
    patch: &FilePatch,
    source: Option<&str>,
    syntax: &SyntaxReference,
    mut budget: usize,
    mut cache: Option<&mut LexCache>,
) -> PatchColors {
    // States are only worth resuming under the grammar that made them
    // (`LexCache::syntax`); a cache walked with another one is cleared.
    if let Some(cache) = cache.as_deref_mut()
        && cache.syntax != syntax.name
    {
        cache.syntax = syntax.name.clone();
        cache.states.clear();
    }
    let highlighter = Highlighter::new(&assets.theme);
    // Which side the file we were handed is, and therefore which of the
    // hunk's two line numbers counts in it. A deleted file has no new
    // side and every row of its diff comes from the old one.
    let new_side = patch.new_path.is_some();
    let lines: Vec<&str> = source.map(|s| s.lines().collect()).unwrap_or_default();
    // The file itself, stepped to wherever the next hunk starts, then
    // through the hunk's own rows — this side's rows are the file, so
    // one walk serves both. The other side's rows are never fed into it:
    // they are what this file does not say, and they read through a fork
    // instead ([`read_hunk`]).
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
        // A remembered place at or below this hunk that is further than
        // the walk has come is a jump: the lines between have been read
        // before, over this same text.
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
            // Nothing to start from — no file, a hunk out of budget, or
            // one the file no longer agrees with. This hunk reads cold
            // from its own top; the file's walk stays where it was for
            // the hunks after it.
            let mut cold = Walk::new(assets, &highlighter, Some(syntax));
            out.push(read_hunk(&mut cold, hunk, new_side, &mut budget).0);
        }
    }
    out
}

/// Reads one hunk's rows: this side's — the context lines and its own
/// changes — through `file`, and the other side's changes through a fork
/// taken where the hunk begins. The fork steps over the context lines
/// too (they are in both sides), so its own changes stay in context;
/// what it never sees is this side's changes, which is exactly what the
/// other side's file never says. The old reading fed both sides through
/// one walk, and a deleted `/*` would leave everything after it painted
/// as the inside of a comment that is not there.
///
/// Every reading spends one of `budget` — a context line costs two, one
/// per side. Once it is gone the rest of the hunk goes out plain.
/// Answers the colours and how many of this side's lines were consumed —
/// spent or not, so the caller's place in the file stays true.
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

/// Colours for the combined diff git prints for a path it stopped on:
/// every row is read, the sides of each conflict region are stood beside
/// each other rather than after one another, and the fences are read as
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
    // The file itself, walked to wherever the next hunk starts. Kept
    // apart from the walk that colours the rows: a hunk's rows are the
    // sides interleaved, and feeding those back would leave the reading
    // somewhere the file never goes.
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
        // A combined diff keeps the older, simpler bound — the budget
        // here caps how deep a hunk the walk will reach for.
        if !lines.is_empty() && start <= lines.len() && start <= LEX_LINE_BUDGET {
            while at < start {
                file.paint(lines[at]);
                at += 1;
            }
            if hunk_agrees(hunk, &lines, start, new_side) {
                rows = Some(file.fork());
            }
        }
        // Nothing to start from — no file, or one that does not say what
        // this hunk says it does.
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

/// Whether the file we were handed says at `start` what the hunk says is
/// there. A diff and a file read a moment apart can disagree — someone
/// saved between them — and a reading walked through the wrong text is
/// worse than one that admits it does not know.
fn hunk_agrees(hunk: &DiffHunk, lines: &[&str], start: usize, new_side: bool) -> bool {
    let first = hunk.lines.iter().find(|line| line.kind.on_side(new_side));
    first.is_none_or(|line| lines.get(start) == Some(&line.text.as_str()))
}

#[cfg(test)]
#[path = "patch_tests.rs"]
mod tests;
