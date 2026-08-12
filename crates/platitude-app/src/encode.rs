//! Pure encoding helpers: core DTOs → compact strings the QML layer decodes
//! mechanically (draw tokens / chip records). All *semantic* work already
//! happened in platitude-core; QML only draws what these strings say.

use base64::Engine as _;
use platitude_core::details::DiffTarget;
use platitude_core::graph::{Segment, SegmentKind};
use platitude_core::highlight::{DiffColors, Span};
use platitude_core::parse::diff::{DiffLineKind, FilePatch};
use platitude_core::patch::HunkSelect;
use platitude_core::session::{LabelKind, RefLabel};
use std::collections::{BTreeMap, BTreeSet};

/// Record separator for the packed lists QML unpacks itself: label chips
/// and co-authors. Neither a refname, a person's name nor an address can
/// hold it.
pub const RECORD_SEP: char = '\u{1f}';

/// Field separator inside one record — a chip's name from the remotes it
/// was read off, a co-author's name from their address. Present only when
/// there is a second field, and like [`RECORD_SEP`] it cannot occur in any
/// of those.
pub const FIELD_SEP: char = '\u{1e}';

/// Segments → `;`-joined draw tokens: `t<lane>.<color>` (through),
/// `i<lane>.<color>` (into node), `o<lane>.<color>` (out of node).
/// Uppercase letters mark dashed segments (the WIP edge).
pub fn encode_geometry(segments: &[Segment]) -> String {
    let mut out = String::with_capacity(segments.len() * 6);
    for (i, s) in segments.iter().enumerate() {
        if i > 0 {
            out.push(';');
        }
        let ch = match s.kind {
            SegmentKind::Through => 't',
            SegmentKind::IntoNode => 'i',
            SegmentKind::OutOfNode => 'o',
        };
        out.push(if s.dashed {
            ch.to_ascii_uppercase()
        } else {
            ch
        });
        out.push_str(&s.lane.to_string());
        out.push('.');
        out.push_str(&s.color.to_string());
    }
    out
}

/// A comma-separated env var as a name set (empty when unset).
fn env_name_set(var: &str) -> std::collections::HashSet<String> {
    std::env::var(var)
        .map(|v| v.split(',').map(str::to_string).collect())
        .unwrap_or_default()
}

/// Branch names previewing the PR badge (`PG_FAKE_PR=a,b`). Real PR data
/// joins in Phase 4; this hook exists so the design can be reviewed.
pub(crate) fn fake_pr_set() -> &'static std::collections::HashSet<String> {
    static SET: std::sync::OnceLock<std::collections::HashSet<String>> = std::sync::OnceLock::new();
    SET.get_or_init(|| env_name_set("PG_FAKE_PR"))
}

/// Labels → `\u{1f}`-joined chip records: a kind letter, four flag digits,
/// the name, and — only when the ref was read off a remote — the field
/// separator and the remotes it came from.
///
/// The letter is `H`ead / `L`ocal / `R`emote / `T`ag; the flags, in order,
/// are is-head, has-remote, has-PR (preview via [`fake_pr_set`] until Phase
/// 4) and is-it-here. The last one is what the chip writes in the name's
/// colour: a remote branch and a tag only a remote has are both somewhere
/// else, and read the same way for it.
///
/// Records arrive sorted HEAD → local → remote → tag, and stay that way:
/// the row's one chip shows the first of them, so a branch is what a
/// commit that is also tagged reads as.
pub fn encode_labels(labels: &[RefLabel]) -> String {
    let mut out = String::new();
    for (i, l) in labels.iter().enumerate() {
        if i > 0 {
            out.push(RECORD_SEP);
        }
        out.push(match l.kind {
            LabelKind::Head => 'H',
            LabelKind::LocalBranch => 'L',
            LabelKind::RemoteBranch => 'R',
            LabelKind::Tag => 'T',
        });
        out.push(if l.is_head { '1' } else { '0' });
        out.push(if l.has_remote { '1' } else { '0' });
        let pr =
            matches!(l.kind, LabelKind::LocalBranch) && fake_pr_set().contains(l.text.as_str());
        out.push(if pr { '1' } else { '0' });
        out.push(if l.here { '1' } else { '0' });
        out.push_str(&l.text);
        if !l.remote.is_empty() {
            out.push(FIELD_SEP);
            out.push_str(&l.remote);
        }
    }
    out
}

/// The refnames in a chip record string, in order — the inverse of the
/// name half of [`encode_labels`].
///
/// Written beside the encoder so the two cannot drift: a reader that
/// guessed the layout would break the first time a flag is added.
pub fn label_names(encoded: &str) -> impl Iterator<Item = &str> {
    encoded.split(RECORD_SEP).filter_map(|record| {
        // Kind letter plus four flag digits, then the name, then — only
        // when the ref was read off a remote — the remotes it came from.
        let rest = record.get(5..)?;
        Some(rest.split(FIELD_SEP).next().unwrap_or(rest))
    })
}

/// Co-authors → `\u{1f}`-joined records of name, address and identicon
/// code, in that order, separated by [`FIELD_SEP`].
///
/// The identicon is computed here rather than in QML for the same reason
/// the graph's is: [`avatar_code`] is what decides a person's face, and
/// one decider is the whole point. A trailer with no address still gets
/// its two separators, so the reader can index without counting.
pub fn encode_co_authors(mates: &[platitude_core::details::CoAuthor]) -> String {
    let mut out = String::new();
    for (i, m) in mates.iter().enumerate() {
        if i > 0 {
            out.push(RECORD_SEP);
        }
        out.push_str(&m.name);
        out.push(FIELD_SEP);
        out.push_str(&m.email);
        out.push(FIELD_SEP);
        out.push_str(&avatar_code(&m.name).to_string());
    }
    out
}

/// The (name, address) of each credited person in a packed record
/// string — the inverse of the first two fields of [`encode_co_authors`].
///
/// Beside the encoder for the same reason [`label_names`] is: the third
/// field is a number, and a reader that searched the packed string whole
/// would answer a typed `12345` with somebody's identicon code.
pub fn co_author_pairs(encoded: &str) -> impl Iterator<Item = (&str, &str)> {
    encoded.split(RECORD_SEP).filter_map(|record| {
        let mut fields = record.split(FIELD_SEP);
        let name = fields.next()?;
        Some((name, fields.next().unwrap_or("")))
    })
}

/// FNV-1a 32-bit. Stands in for randomness wherever a name has to pick
/// something arbitrary but has to pick the *same* thing every time.
fn fnv1a(text: &str) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for b in text.as_bytes() {
        hash ^= u32::from(*b);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

/// Deterministic identicon code for an author (GitHub-style 5x5 pattern,
/// generated locally — fetching real avatars would violate the
/// no-network-except-git constraint).
///
/// Layout: bits 0..15 = left 3 columns of a 5x5 grid (row-major, mirrored
/// to the right by the renderer), bits 15..18 = palette index (0..8).
pub fn avatar_code(author: &str) -> i32 {
    let hash = fnv1a(author);
    let mut pattern = hash & 0x7fff;
    if pattern == 0 {
        pattern = 0b00000_00100_00000; // center dot fallback
    }
    let color = (hash >> 15) % 8;
    (pattern | (color << 15)) as i32
}

/// The palette indices a conflict's two sides are drawn with, given what
/// the graph could lend (`-1` = nothing) and what each side is called.
///
/// The two must never come out the same — that is the whole job of the
/// colour, and a conflicted file is the worst place to be told "these are
/// different" by two identical marks. So:
///
/// - the graph's answer is kept wherever it has one;
/// - a side it has none for takes a colour off its own name, which is
///   arbitrary the way a colour with no meaning should be, and **stable**
///   the way a random one would not: the same conflict reopens in the same
///   colours, and a screenshot of it is reproducible;
/// - if the two still land together, **ours keeps its colour and theirs
///   moves on by one**. The side already in place is the one worth leaving
///   alone (during a rebase that is the upstream — `conflict::sides()` has
///   already sorted out which is which).
///
/// An unnamed side still gets a colour: the bar says *which of the two*,
/// and the legend beside it is where the naming happens.
pub fn conflict_side_colors(ours: (i32, &str), theirs: (i32, &str)) -> (i32, i32) {
    let size = i32::try_from(platitude_core::graph::GRAPH_PALETTE_SIZE).unwrap_or(8);
    let borrowed_or_named = |(color, name): (i32, &str)| -> i32 {
        if (0..size).contains(&color) {
            color
        } else {
            (fnv1a(name) % size.unsigned_abs()) as i32
        }
    };
    let ours = borrowed_or_named(ours);
    let mut theirs = borrowed_or_named(theirs);
    if theirs == ours {
        theirs = (theirs + 1) % size;
    }
    (ours, theirs)
}

/// Lanes that touch the bottom edge of a row (from its geometry tokens):
/// `t` segments plus `o` targets. Used by the truncation footer to draw
/// the lanes running off the end of the window. Output keeps the row
/// tokens' shape — `t<lane>.<color>`, uppercase for a dashed leash — so
/// one rule reads the same on both sides of the cut. A leash does reach
/// here: with many starting refs the walk can emit thousands of commits
/// before it gets to HEAD, and the WIP row waits on that lane the whole
/// way (measured on JetBrains/kotlin).
pub fn tail_lanes(geometry: &str) -> String {
    let mut seen = std::collections::HashSet::new();
    let mut out = String::new();
    for token in geometry.split(';').filter(|t| t.len() > 1) {
        let head = token.as_bytes()[0];
        let kind = head.to_ascii_lowercase();
        if kind != b't' && kind != b'o' {
            continue;
        }
        let rest = &token[1..];
        let Some((lane, _color)) = rest.split_once('.') else {
            continue;
        };
        if seen.insert(lane.to_string()) {
            if !out.is_empty() {
                out.push(';');
            }
            out.push(if head.is_ascii_uppercase() { 'T' } else { 't' });
            out.push_str(rest);
        }
    }
    out
}

/// One hunk or one line of it, as the diff pane addresses them.
///
/// The indices come straight off the row the user clicked, so nothing is
/// parsed and nothing can drift: a wrong index would stage a different
/// line than the one under the cursor. A negative line means the whole
/// hunk.
pub fn hunk_selection(hunk: i32, line: i32) -> Vec<HunkSelect> {
    let Ok(hunk) = usize::try_from(hunk) else {
        return Vec::new();
    };
    match usize::try_from(line) {
        Ok(line) => vec![HunkSelect::lines(hunk, [line])],
        Err(_) => vec![HunkSelect::whole(hunk)],
    }
}

/// Groups hand-picked `(hunk, line)` pairs into one selection per hunk.
///
/// Sorted and de-duplicated on both keys: the patch builder addresses
/// lines by position, so a repeated index would emit the same line twice,
/// and hunks handed over out of order would build a patch git refuses.
/// A pair with a negative index is dropped rather than taken for "the
/// whole hunk" — this way in, unlike [`hunk_selection`], only ever means
/// lines that were pointed at.
pub fn line_selection(pairs: &[(i32, i32)]) -> Vec<HunkSelect> {
    let mut by_hunk: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    for &(hunk, line) in pairs {
        let (Ok(hunk), Ok(line)) = (usize::try_from(hunk), usize::try_from(line)) else {
            continue;
        };
        by_hunk.entry(hunk).or_default().insert(line);
    }
    by_hunk
        .into_iter()
        .map(|(hunk, lines)| HunkSelect::lines(hunk, lines))
        .collect()
}

/// Rebuilds the diff target a working-tree selection refers to.
/// `kind` is the prefix [`diff_key`] uses (`staged` / `unstaged` /
/// `untracked`); a committed diff is not stageable and yields `None`.
pub fn worktree_target(kind: &str, path: &str, orig_path: &str) -> Option<DiffTarget> {
    let orig = (!orig_path.is_empty()).then(|| orig_path.to_string());
    match kind {
        "staged" => Some(DiffTarget::Staged {
            path: path.to_string(),
            orig_path: orig,
        }),
        "unstaged" => Some(DiffTarget::Unstaged {
            path: path.to_string(),
        }),
        "untracked" => Some(DiffTarget::Untracked {
            path: path.to_string(),
        }),
        _ => None,
    }
}

pub fn diff_key(target: &DiffTarget) -> String {
    match target {
        DiffTarget::Commit { oid, path, .. } => format!("commit:{}:{path}", oid.to_hex()),
        DiffTarget::Staged { path, .. } => format!("staged:{path}"),
        DiffTarget::Unstaged { path } => format!("unstaged:{path}"),
        DiffTarget::Untracked { path } => format!("untracked:{path}"),
    }
}

/// `data:` URL a QML `Image` loads directly — no temp files, no image
/// providers, and blob content works the same as working-tree content.
pub fn image_data_url(mime: &str, bytes: &[u8]) -> String {
    let mut out = format!("data:{mime};base64,");
    base64::engine::general_purpose::STANDARD.encode_string(bytes, &mut out);
    out
}

/// A line-ending notice taken apart into the pieces its sentence needs.
///
/// Two places say the same sentence — the diff pane's line and the hover of
/// the pending file that carries the mark — so they take the notice apart
/// the same way. The sentence itself is `Words.lineEndings`; working out
/// which of the four it is, and how far the sample behind it reached, is
/// not QML's job.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct EndingWords {
    /// `""` (nothing to say) / `flipped` / `mixed` / `new` / `first`.
    pub kind: String,
    /// The two endings in the order the sentence names them.
    pub from: String,
    pub to: String,
    pub lines: i32,
    /// How far the sample reached: `""` / `here` / `ext` / `repo`.
    pub scope: String,
    pub ext: String,
}

pub fn ending_words(notice: Option<&platitude_core::eol::Notice>) -> EndingWords {
    use platitude_core::eol::{Notice, Scope};
    let mut out = EndingWords::default();
    let Some(notice) = notice else { return out };
    let (kind, from, to) = match notice {
        Notice::Flipped { from, to } => ("flipped", *from, *to),
        Notice::Mixed {
            lines, added, file, ..
        } => {
            out.lines = i32::try_from(*lines).unwrap_or(i32::MAX);
            ("mixed", *added, *file)
        }
        Notice::NewFile { eol, baseline } => ("new", *eol, baseline.eol),
        Notice::FirstEnding { eol, baseline } => ("first", *eol, baseline.eol),
    };
    let baseline = match notice {
        Notice::NewFile { baseline, .. } | Notice::FirstEnding { baseline, .. } => Some(baseline),
        _ => None,
    };
    if let Some(baseline) = baseline {
        let (scope, ext) = match &baseline.scope {
            Scope::Here(ext) => ("here", ext.as_str()),
            Scope::Ext(ext) => ("ext", ext.as_str()),
            Scope::Repo => ("repo", ""),
        };
        out.scope = scope.to_string();
        out.ext = ext.to_string();
    }
    out.kind = kind.to_string();
    out.from = from.as_str().to_string();
    out.to = to.as_str().to_string();
    out
}

/// How a rename's source is written beside the new name.
///
/// The old path is cut back exactly as far as the new one is: a file that
/// moved inside its own directory shows two bare names, and one that came
/// from somewhere else keeps the path that says where. `cut` is how many
/// bytes of the new path its row does not have to spell — the folders
/// above it already do (a flat list passes 0, and both names stay whole).
///
/// Nothing is guessed from the paths themselves: a prefix the new name
/// dropped is a prefix the old one can drop only if it had the same one.
pub fn rename_source<'a>(orig_path: &'a str, path: &str, cut: usize) -> &'a str {
    if cut == 0 || orig_path.is_empty() {
        return orig_path;
    }
    let Some(prefix) = path.get(..cut) else {
        return orig_path;
    };
    orig_path.strip_prefix(prefix).unwrap_or(orig_path)
}

/// Human-readable byte size ("67 B", "1.5 KB", "234 KB", "1.2 MB").
/// 1024-based; one decimal below ten so small differences stay visible.
pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
    let mut unit = UNITS[0];
    for u in UNITS {
        value /= 1024.0;
        unit = u;
        if value < 1024.0 {
            break;
        }
    }
    if value < 10.0 {
        format!("{value:.1} {unit}")
    } else {
        format!("{value:.0} {unit}")
    }
}

/// One flattened row of the diff pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffRow {
    /// `hunk` / `ctx` / `add` / `del` / `meta`.
    pub kind: &'static str,
    /// -1 when the side has no line number.
    pub old_no: i32,
    pub new_no: i32,
    /// What the row draws — plain text, or the same line marked up for
    /// `Text.StyledText` when [`DiffRow::rich`]. Only one of the two is
    /// ever held: keeping the plain copy beside the marked-up one doubles
    /// what a long diff costs and nothing reads it.
    pub text: String,
    /// Whether [`DiffRow::text`] is markup. False wherever the theme had
    /// nothing to say — a language the set has never heard of, a hunk
    /// heading, git's own `\ No newline` note, a conflict marker.
    pub rich: bool,
    /// One of git's conflict fences (`<<<<<<<` / `|||||||` / `=======` /
    /// `>>>>>>>`). It is a line of the working tree like any other and
    /// carries the same background, but it is git talking rather than the
    /// file, and the pane says so by dropping its voice
    /// (デザイン規約 §シンタックスハイライト).
    pub fence: bool,
    /// Which hunk of the file this row belongs to, and which line of that
    /// hunk it is (-1 on the hunk header). These are the same indices
    /// [`platitude_core::patch::HunkSelect`] addresses, so a row can be
    /// staged straight from what the pane is showing.
    pub hunk: i32,
    pub line: i32,
    /// One marker column per side of a combined diff (`" +"`, `"++"`,
    /// `"- "`), empty on every ordinary row. Which side a line came from
    /// is in here and nowhere else: the colour cannot carry it, since git
    /// paints our side and theirs the same green.
    pub markers: String,
}

/// Whether the patch has a new side and no old one — a file the repository
/// is seeing for the first time (untracked, or newly added to the index).
///
/// Such a diff is every line an addition under a single heading, so there
/// is nothing in it smaller than the file itself: the pane reads this to
/// drop the pieces it would otherwise offer (デザイン規約 §diff の中の
/// ステージ). A deleted file is not one of these — its lines still exist
/// on the old side, and a part of them can still be staged.
///
/// The new side has to be named, not merely inferred from a missing old
/// one: a patch that carries no `diff --git` header at all parses with
/// both sides empty ([`platitude_core::parse::diff::parse_patch`]
/// synthesizes the file entry), and that is a diff whose shape is unknown
/// rather than one that is known to be new.
pub fn is_new_file(patches: &[FilePatch]) -> bool {
    !patches.is_empty()
        && patches.iter().all(|p| {
            // A conflicted path is never one of these, however its sides
            // read. `AA` — both branches invented the file — has no old
            // side by construction, and an unmerged entry has neither
            // side because git printed no patch at all; taking either for
            // a new file would withhold the pane's pieces for a reason
            // that is not this one.
            !p.is_combined && !p.unmerged && p.old_path.is_none() && p.new_path.is_some()
        })
}

/// Whether the diff compares its file against **more than one** side —
/// the form git prints for a conflicted path. Nothing in it can be staged
/// or thrown away piecemeal (`platitude_core::patch::is_combined` is the
/// floor under that), and its rows carry [`DiffRow::markers`].
pub fn is_combined(patches: &[FilePatch]) -> bool {
    patches.iter().any(|p| p.is_combined)
}

/// Whether git named the path as unmerged and printed no patch for it:
/// one of the two sides does not exist, so there is nothing to compare
/// (`DU` / `UD` / `AU` / `UA`). The pane has no rows to show and says what
/// the two sides did instead.
pub fn is_unmerged_only(patches: &[FilePatch]) -> bool {
    !patches.is_empty() && patches.iter().all(|p| p.unmerged)
}

/// Flattens parsed patches into displayable rows (hunk headers inline).
/// `binary_note` inserts the "(binary file)" meta row; the caller turns it
/// off when a preview (image / size summary) already covers that file.
pub fn flatten_patches(
    patches: &[FilePatch],
    binary_note: bool,
    colors: &DiffColors,
) -> Vec<DiffRow> {
    let mut rows = Vec::new();
    // The colours are addressed by the same three indices this walk is
    // already counting out, so the two are read together rather than
    // matched up afterwards (`platitude_core::highlight::DiffColors`).
    for (patch_index, patch) in patches.iter().enumerate() {
        if patch.unmerged {
            // No patch, and no words for one here: what the two sides did
            // is a sentence the pane builds from the stage letters, in the
            // one place that wording lives (`Words.conflict`).
            continue;
        }
        if patch.is_binary {
            if binary_note {
                rows.push(DiffRow {
                    kind: "meta",
                    old_no: -1,
                    new_no: -1,
                    text: String::from("(binary file)"),
                    rich: false,
                    fence: false,
                    hunk: -1,
                    line: -1,
                    markers: String::new(),
                });
            }
            continue;
        }
        for (hunk_index, hunk) in patch.hunks.iter().enumerate() {
            let heading = if hunk.heading.is_empty() {
                String::new()
            } else {
                format!(" {}", hunk.heading)
            };
            let hunk_no = i32::try_from(hunk_index).unwrap_or(-1);
            rows.push(DiffRow {
                kind: "hunk",
                old_no: -1,
                new_no: -1,
                text: hunk_header(hunk, &heading),
                rich: false,
                fence: false,
                hunk: hunk_no,
                line: -1,
                markers: String::new(),
            });
            for (line_index, line) in hunk.lines.iter().enumerate() {
                let kind = match line.kind {
                    DiffLineKind::Context => "ctx",
                    DiffLineKind::Addition => "add",
                    DiffLineKind::Deletion => "del",
                    DiffLineKind::NoNewline => "meta",
                };
                let read = colors.line(patch_index, hunk_index, line_index);
                let markup = styled(&line.text, &read.spans);
                let rich = !markup.is_empty();
                rows.push(DiffRow {
                    kind,
                    old_no: line.old_no.map_or(-1, |n| n as i32),
                    new_no: line.new_no.map_or(-1, |n| n as i32),
                    text: if rich { markup } else { line.text.clone() },
                    rich,
                    fence: read.fence,
                    hunk: hunk_no,
                    line: i32::try_from(line_index).unwrap_or(-1),
                    markers: line.markers.clone(),
                });
            }
        }
    }
    rows
}

/// Lays the theme's runs over one line and writes what `Text.StyledText`
/// reads. Empty when there are no runs — the row then draws its own text
/// in the colour its kind gives it, which is what every row did before
/// there was any of this.
///
/// Built here rather than in QML because it is data, not drawing: the
/// pane is handed a string and shows it (規約 §QML にビジネスロジックを
/// 書かない).
fn styled(text: &str, spans: &[Span]) -> String {
    if spans.is_empty() {
        return String::new();
    }
    // An opening tag with six hex digits in it and a closing one, per
    // run: twice the line is close enough to save the regrowth.
    let mut out = String::with_capacity(text.len() * 2);
    // `StyledText` reads the line as HTML does, and HTML throws leading
    // whitespace away and folds the rest into single spaces — which on
    // source code means every line starts at the left margin (2026-08-12
    // 実測: a 0-space `pub fn` and a 4-space `let` landed on the same
    // pixel). `<pre>` is the one thing in the subset that turns that off,
    // and it costs nothing else: the row is one line either way.
    out.push_str("<pre>");
    let mut at = 0;
    for span in spans {
        let end = (at + span.len).min(text.len());
        // The runs are byte offsets into this same string, so they cut
        // where characters do. `None` would mean they did not, and the
        // rest of the line goes out plain rather than half a character
        // going out at all.
        let Some(piece) = text.get(at..end) else {
            break;
        };
        out.push_str("<font color=\"#");
        push_hex(&mut out, span.color.r);
        push_hex(&mut out, span.color.g);
        push_hex(&mut out, span.color.b);
        out.push_str("\">");
        push_escaped(&mut out, piece);
        out.push_str("</font>");
        at = end;
    }
    // Whatever the runs did not reach — a lexer that stopped short still
    // leaves a whole line on screen.
    if let Some(rest) = text.get(at..) {
        push_escaped(&mut out, rest);
    }
    out.push_str("</pre>");
    out
}

/// The three characters `Text.StyledText` would otherwise read as markup.
/// Source lines are full of them: `&&`, `->`, `<T>`.
fn push_escaped(out: &mut String, text: &str) {
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(ch),
        }
    }
}

fn push_hex(out: &mut String, byte: u8) {
    for digit in [byte >> 4, byte & 0xf] {
        out.push(char::from_digit(u32::from(digit), 16).unwrap_or('0'));
    }
}

/// The `@@` line as git writes it: one range per old side, and a run of
/// `@` one longer than that count on both ends.
fn hunk_header(hunk: &platitude_core::parse::diff::DiffHunk, heading: &str) -> String {
    let ats = "@".repeat(hunk.extra_old.len() + 2);
    let mut out = format!("{ats} -{},{}", hunk.old_start, hunk.old_count);
    for (start, count) in &hunk.extra_old {
        out.push_str(&format!(" -{start},{count}"));
    }
    out.push_str(&format!(
        " +{},{} {ats}{heading}",
        hunk.new_start, hunk.new_count
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use platitude_core::highlight::Rgb;
    use platitude_core::parse::diff::parse_patch;

    fn run(len: usize, r: u8, g: u8, b: u8) -> Span {
        Span {
            len,
            color: Rgb { r, g, b },
        }
    }

    #[test]
    fn a_line_without_runs_stays_plain() {
        assert!(styled("plain", &[]).is_empty());
    }

    #[test]
    fn runs_become_markup_and_source_characters_are_escaped() {
        // The second run is the three characters `StyledText` would
        // otherwise read as markup, which is what source code is full of.
        let out = styled(
            "fn a<&b>",
            &[run(3, 0x11, 0x22, 0x33), run(5, 0xaa, 0xbb, 0xcc)],
        );
        assert_eq!(
            out,
            "<pre><font color=\"#112233\">fn </font>\
             <font color=\"#aabbcc\">a&lt;&amp;b&gt;</font></pre>"
        );
    }

    #[test]
    fn a_line_the_runs_fall_short_of_is_still_whole() {
        let out = styled("ab cd", &[run(2, 0, 0, 0)]);
        assert!(out.ends_with(" cd</pre>"), "{out}");
    }

    #[test]
    fn rows_carry_the_markup_the_theme_gave_them() {
        let patch = "\
diff --git a/src/a.rs b/src/a.rs
--- a/src/a.rs
+++ b/src/a.rs
@@ -1,2 +1,2 @@
 fn main() {
-    let a = 1;
+    let a = 2;
";
        let patches = parse_patch(patch.as_bytes());
        let rows = flatten_patches(
            &patches,
            true,
            &platitude_core::highlight::colors(&patches, None),
        );
        assert!(
            rows.iter()
                .filter(|r| r.kind == "add" || r.kind == "del" || r.kind == "ctx")
                .all(|r| r.rich && r.text.starts_with("<pre>")),
            "every line of a language the set knows is marked up: {rows:?}"
        );
        // The heading is the pane's own words, not the file's.
        assert!(rows.first().is_some_and(|r| r.kind == "hunk" && !r.rich));
    }

    #[test]
    fn a_rename_inside_one_directory_drops_the_prefix_both_names_share() {
        // The tree row already sits under `docs/a`, so neither name has
        // to spell it.
        let path = "docs/a/new.txt";
        let cut = path.len() - "new.txt".len();
        assert_eq!(rename_source("docs/a/old.txt", path, cut), "old.txt");
    }

    #[test]
    fn a_rename_from_elsewhere_keeps_the_path_that_says_where() {
        let path = "docs/b/new.txt";
        let cut = path.len() - "new.txt".len();
        assert_eq!(rename_source("docs/a/old.txt", path, cut), "docs/a/old.txt");
    }

    #[test]
    fn a_flat_list_leaves_both_names_whole() {
        assert_eq!(
            rename_source("docs/a/old.txt", "docs/a/new.txt", 0),
            "docs/a/old.txt"
        );
    }

    #[test]
    fn a_cut_that_lands_inside_a_character_changes_nothing() {
        // The offset comes from the row's own name, so this cannot
        // happen — but a panic here would take the whole list down.
        let path = "文/new.txt";
        assert_eq!(rename_source("文/old.txt", path, 1), "文/old.txt");
    }

    #[test]
    fn geometry_tokens_round_trip_by_eye() {
        let segs = [
            Segment {
                kind: SegmentKind::Through,
                lane: 0,
                color: 3,
                dashed: false,
            },
            Segment {
                kind: SegmentKind::IntoNode,
                lane: 2,
                color: 11,
                dashed: false,
            },
            Segment {
                kind: SegmentKind::OutOfNode,
                lane: 1,
                color: 0,
                dashed: true,
            },
        ];
        assert_eq!(
            encode_geometry(&segs),
            "t0.3;i2.11;O1.0",
            "dashed segments encode uppercase"
        );
        assert_eq!(encode_geometry(&[]), "");
    }

    #[test]
    fn tail_lanes_picks_bottom_touching_segments() {
        assert_eq!(tail_lanes("t0.3;i2.11;o1.0"), "t0.3;t1.0");
        assert_eq!(tail_lanes("i2.5"), "", "into-node stops at the node");
        assert_eq!(tail_lanes("t0.1;o0.2"), "t0.1", "deduped by lane");
        assert_eq!(
            tail_lanes("T0.4;O1.5"),
            "T0.4;T1.5",
            "a leash keeps dotting past the cut"
        );
        assert_eq!(tail_lanes(""), "");
    }

    #[test]
    fn avatar_codes_are_deterministic_and_bounded() {
        let a = avatar_code("Alice <a@example.com>");
        assert_eq!(a, avatar_code("Alice <a@example.com>"), "stable");
        assert_ne!(a, avatar_code("Bob <b@example.com>"));
        assert!(a >= 0);
        let color = (a >> 15) & 0x7;
        assert!((0..8).contains(&color));
        assert_ne!(a & 0x7fff, 0, "pattern is never empty");
        assert_ne!(avatar_code("") & 0x7fff, 0, "empty author still draws");
    }

    #[test]
    fn label_records_have_fixed_prefix() {
        let labels = [
            RefLabel {
                text: "main".into(),
                kind: LabelKind::LocalBranch,
                has_remote: true,
                is_head: true,
                here: true,
                remote: String::new(),
            },
            RefLabel {
                text: "v1.0".into(),
                kind: LabelKind::Tag,
                has_remote: false,
                is_head: false,
                here: true,
                remote: String::new(),
            },
        ];
        assert_eq!(encode_labels(&labels), "L1101main\u{1f}T0001v1.0");
    }

    #[test]
    fn co_authors_pack_name_address_and_face_into_one_record_each() {
        use platitude_core::details::CoAuthor;
        let packed = encode_co_authors(&[
            CoAuthor {
                name: "Claude Opus 5".into(),
                email: "noreply@anthropic.com".into(),
            },
            CoAuthor {
                name: "Nameless".into(),
                email: String::new(),
            },
        ]);
        let records: Vec<&str> = packed.split(RECORD_SEP).collect();
        assert_eq!(records.len(), 2);

        let first: Vec<&str> = records[0].split(FIELD_SEP).collect();
        assert_eq!(first[0], "Claude Opus 5");
        assert_eq!(first[1], "noreply@anthropic.com");
        assert_eq!(
            first[2],
            avatar_code("Claude Opus 5").to_string(),
            "the face comes off the name, the way the graph rows' do"
        );

        // An address-less trailer still leaves three fields, so the
        // reader indexes rather than counts.
        let second: Vec<&str> = records[1].split(FIELD_SEP).collect();
        assert_eq!(second.len(), 3);
        assert_eq!(second[1], "");
    }

    #[test]
    fn no_co_authors_pack_into_nothing() {
        assert_eq!(encode_co_authors(&[]), "");
    }

    #[test]
    fn a_tag_carries_the_remote_bit_like_a_branch() {
        let labels = [RefLabel {
            text: "v1.0".into(),
            kind: LabelKind::Tag,
            has_remote: true,
            is_head: false,
            here: true,
            remote: String::new(),
        }];
        assert_eq!(encode_labels(&labels), "T0101v1.0");
    }

    #[test]
    fn a_tag_only_a_remote_has_says_it_is_not_here_and_whose_it_is() {
        let labels = [RefLabel {
            text: "v9.9".into(),
            kind: LabelKind::Tag,
            has_remote: true,
            is_head: false,
            here: false,
            remote: "origin, fork".into(),
        }];
        assert_eq!(encode_labels(&labels), "T0100v9.9\u{1e}origin, fork");
    }

    #[test]
    fn names_come_back_out_of_the_records_they_went_into() {
        let labels = [
            RefLabel {
                text: "main".into(),
                kind: LabelKind::LocalBranch,
                has_remote: true,
                is_head: true,
                here: true,
                remote: String::new(),
            },
            RefLabel {
                text: "v9.9".into(),
                kind: LabelKind::Tag,
                has_remote: true,
                is_head: false,
                here: false,
                remote: "origin, fork".into(),
            },
        ];
        let encoded = encode_labels(&labels);
        assert_eq!(
            label_names(&encoded).collect::<Vec<_>>(),
            vec!["main", "v9.9"],
            "the remotes a record was read from are not part of its name"
        );
        assert_eq!(label_names("").count(), 0);
        // A record too short to hold the fixed prefix is not a name.
        assert_eq!(label_names("L11").count(), 0);
    }

    #[test]
    fn a_record_with_no_remote_carries_no_field_separator() {
        // The name runs to the end of the record, which is what every
        // reader assumes when the separator is absent.
        let labels = [RefLabel {
            text: "v9.9".into(),
            kind: LabelKind::Tag,
            has_remote: false,
            is_head: false,
            here: true,
            remote: String::new(),
        }];
        assert!(!encode_labels(&labels).contains(FIELD_SEP));
    }

    #[test]
    fn flatten_produces_hunk_headers_and_numbers() {
        let patch = "\
--- a/f
+++ b/f
@@ -1,2 +1,2 @@ heading
 same
-old
+new
";
        let rows = flatten_patches(&parse_patch(patch.as_bytes()), true, &DiffColors::default());
        assert_eq!(rows[0].kind, "hunk");
        assert!(rows[0].text.contains("@@ -1,2 +1,2 @@ heading"));
        assert_eq!(rows[1].kind, "ctx");
        assert_eq!((rows[1].old_no, rows[1].new_no), (1, 1));
        assert_eq!(rows[2].kind, "del");
        assert_eq!((rows[2].old_no, rows[2].new_no), (2, -1));
        assert_eq!(rows[3].kind, "add");
        assert_eq!((rows[3].old_no, rows[3].new_no), (-1, 2));
    }

    #[test]
    fn a_negative_line_selects_the_whole_hunk() {
        assert_eq!(hunk_selection(3, -1), vec![HunkSelect::whole(3)]);
        assert_eq!(hunk_selection(3, 4), vec![HunkSelect::lines(3, [4])]);
    }

    #[test]
    fn a_row_with_no_hunk_selects_nothing() {
        // Rows outside any hunk (the binary-file note) carry -1, and
        // staging one of those must not fall back to hunk zero.
        assert!(hunk_selection(-1, -1).is_empty());
        assert!(hunk_selection(-1, 2).is_empty());
    }

    #[test]
    fn picked_lines_gather_into_one_selection_per_hunk() {
        // Clicked in whatever order the hand went, twice on one line, and
        // across two hunks: what comes out is one selection per hunk, in
        // hunk order, with each hunk's lines in theirs.
        let picked = [(1, 5), (0, 3), (1, 2), (0, 1), (1, 5)];
        assert_eq!(
            line_selection(&picked),
            vec![HunkSelect::lines(0, [1, 3]), HunkSelect::lines(1, [2, 5])]
        );
    }

    #[test]
    fn picked_lines_never_stand_for_a_whole_hunk() {
        // `hunk_selection` reads a negative line as "all of it"; this way
        // in must not, or a row that carries -1 would quietly widen a
        // choice made line by line.
        assert!(line_selection(&[(0, -1)]).is_empty());
        assert!(line_selection(&[(-1, 4)]).is_empty());
        assert!(line_selection(&[]).is_empty());
    }

    #[test]
    fn flattened_rows_carry_the_indices_that_address_them() {
        let patch = "\
--- a/f
+++ b/f
@@ -1,2 +1,3 @@
 keep
+added
@@ -10,2 +11,1 @@
-removed
 tail
";
        let rows = flatten_patches(&parse_patch(patch.as_bytes()), true, &DiffColors::default());
        // The header carries its hunk but no line; the lines that follow
        // are numbered from zero within that hunk — exactly what
        // `HunkSelect` addresses.
        assert_eq!((rows[0].kind, rows[0].hunk, rows[0].line), ("hunk", 0, -1));
        assert_eq!((rows[1].kind, rows[1].hunk, rows[1].line), ("ctx", 0, 0));
        assert_eq!((rows[2].kind, rows[2].hunk, rows[2].line), ("add", 0, 1));
        assert_eq!((rows[3].kind, rows[3].hunk, rows[3].line), ("hunk", 1, -1));
        assert_eq!((rows[4].kind, rows[4].hunk, rows[4].line), ("del", 1, 0));
        assert_eq!((rows[5].kind, rows[5].hunk, rows[5].line), ("ctx", 1, 1));
    }

    #[test]
    fn worktree_targets_exclude_committed_diffs() {
        assert_eq!(
            worktree_target("unstaged", "f.txt", ""),
            Some(DiffTarget::Unstaged {
                path: "f.txt".into()
            })
        );
        assert_eq!(
            worktree_target("staged", "new.txt", "old.txt"),
            Some(DiffTarget::Staged {
                path: "new.txt".into(),
                orig_path: Some("old.txt".into())
            })
        );
        assert_eq!(worktree_target("commit", "f.txt", ""), None);
    }

    // The shapes below are `git diff` output as it stands, taken off git
    // 2.55 in a throwaway repository. An untracked file read the way
    // `details::file_diff` reads one (`--no-index` against `/dev/null`)
    // and the same file once added come out **byte for byte the same** —
    // git labels the old side `a/fresh.txt` in the header either way and
    // only `---` tells the truth about it — so one constant covers both.
    const ADDED: &str = "\
diff --git a/fresh.txt b/fresh.txt
new file mode 100644
index 0000000..fbbee86
--- /dev/null
+++ b/fresh.txt
@@ -0,0 +1,2 @@
+alpha
+beta
";
    const EDITED: &str = "\
diff --git a/kept.txt b/kept.txt
index 814f4a4..879de50 100644
--- a/kept.txt
+++ b/kept.txt
@@ -1,2 +1,2 @@
 one
-two
+TWO
";
    const DELETED: &str = "\
diff --git a/gone.txt b/gone.txt
deleted file mode 100644
index bd43ee2..0000000
--- a/gone.txt
+++ /dev/null
@@ -1 +0,0 @@
-doomed
";

    #[test]
    fn a_file_with_only_a_new_side_is_a_new_file() {
        let patches = parse_patch(ADDED.as_bytes());
        assert!(is_new_file(&patches));
        // The header names the old side too; `---` is what takes it away.
        assert_eq!(patches[0].new_path.as_deref(), Some("fresh.txt"));
        assert_eq!(patches[0].old_path, None);
    }

    #[test]
    fn a_file_that_existed_before_is_not_a_new_one() {
        assert!(!is_new_file(&parse_patch(EDITED.as_bytes())));
        // A removal is the other way round: its lines all still exist on
        // the old side, so a part of them can still be taken.
        assert!(!is_new_file(&parse_patch(DELETED.as_bytes())));
    }

    #[test]
    fn a_diff_of_unknown_shape_is_not_read_as_new() {
        // Nothing parsed at all, and a patch with no `diff --git` header —
        // which parses with both sides empty. Neither is known to be a new
        // file, and reading them as one would take the pane's pieces away.
        assert!(!is_new_file(&[]));
        let headerless = "\
@@ -1,1 +1,1 @@
-old
+new
";
        assert!(!is_new_file(&parse_patch(headerless.as_bytes())));
    }

    #[test]
    fn binary_patch_flattens_to_meta_row() {
        let patch = "\
diff --git a/x.png b/x.png
Binary files a/x.png and b/x.png differ
";
        let rows = flatten_patches(&parse_patch(patch.as_bytes()), true, &DiffColors::default());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, "meta");
        // With a preview covering the file, the note is dropped entirely.
        assert!(
            flatten_patches(
                &parse_patch(patch.as_bytes()),
                false,
                &DiffColors::default()
            )
            .is_empty()
        );
    }

    /// `git diff` on a conflicted path, verbatim (git 2.55).
    const CONFLICTED: &str = "\
diff --cc shared.txt
index 804ce7b,ba44bb1..0000000
--- a/shared.txt
+++ b/shared.txt
@@@ -1,3 -1,3 +1,7 @@@ heading
  one
++<<<<<<< HEAD
 +OURS
++=======
+ THEIRS
++>>>>>>> topic
  three
";

    #[test]
    fn conflict_sides_keep_the_colours_the_graph_lent_them() {
        assert_eq!(conflict_side_colors((3, "main"), (5, "topic")), (3, 5));
    }

    #[test]
    fn conflict_sides_that_landed_together_move_theirs_on() {
        // The palette cycles, so two chains far enough apart share one
        // colour. Ours is the side already in place and keeps it.
        assert_eq!(conflict_side_colors((3, "main"), (3, "topic")), (3, 4));
        // And the move wraps rather than running off the end.
        let size = i32::try_from(platitude_core::graph::GRAPH_PALETTE_SIZE).unwrap();
        assert_eq!(
            conflict_side_colors((size - 1, "main"), (size - 1, "topic")),
            (size - 1, 0)
        );
    }

    #[test]
    fn a_side_the_graph_has_no_colour_for_takes_one_off_its_name() {
        // Outside the walk's window, or named something no chip carries
        // (`main~3`, which is what a rebase onto an older commit reports).
        let (ours, theirs) = conflict_side_colors((-1, "main~3"), (5, "topic"));
        assert_eq!(theirs, 5, "the side that had one keeps it");
        assert_ne!(ours, theirs);
        let size = i32::try_from(platitude_core::graph::GRAPH_PALETTE_SIZE).unwrap();
        assert!((0..size).contains(&ours));
        // Same name, same colour, every time — a reopened conflict must
        // not repaint itself, and a screenshot of one has to be repeatable.
        assert_eq!(
            conflict_side_colors((-1, "main~3"), (5, "topic")),
            (ours, 5)
        );
    }

    #[test]
    fn two_colourless_sides_still_come_out_apart() {
        let (a, b) = conflict_side_colors((-1, "main"), (-1, "topic"));
        assert_ne!(a, b);
        // Including when git could not name either of them, which hashes
        // both to the same place before the move.
        let (a, b) = conflict_side_colors((-1, ""), (-1, ""));
        assert_ne!(a, b);
        // And when the colour handed in is nonsense rather than -1.
        let (a, b) = conflict_side_colors((99, "main"), (-7, "main"));
        assert_ne!(a, b);
    }

    #[test]
    fn a_combined_diff_flattens_with_its_marker_columns() {
        let rows = flatten_patches(
            &parse_patch(CONFLICTED.as_bytes()),
            true,
            &DiffColors::default(),
        );
        // The heading counts its sides on both ends, so the row reads the
        // way git printed it rather than as a unified one that lost a
        // range.
        assert_eq!(rows[0].kind, "hunk");
        assert_eq!(rows[0].text, "@@@ -1,3 -1,3 +1,7 @@@ heading");
        assert_eq!(rows[0].markers, "", "a heading has no side of its own");

        let seen: Vec<(&str, &str, &str)> = rows[1..]
            .iter()
            .map(|r| (r.kind, r.markers.as_str(), r.text.as_str()))
            .collect();
        assert_eq!(
            seen,
            vec![
                ("ctx", "  ", "one"),
                ("add", "++", "<<<<<<< HEAD"),
                ("add", " +", "OURS"),
                ("add", "++", "======="),
                ("add", "+ ", "THEIRS"),
                ("add", "++", ">>>>>>> topic"),
                ("ctx", "  ", "three"),
            ]
        );
    }

    #[test]
    fn a_conflicted_diff_is_combined_and_not_a_new_file() {
        let patches = parse_patch(CONFLICTED.as_bytes());
        assert!(is_combined(&patches));
        assert!(!is_unmerged_only(&patches));
        assert!(!is_new_file(&patches));
    }

    #[test]
    fn a_conflict_both_sides_added_is_not_read_as_a_new_file() {
        // No old side at all — the shape `is_new_file` was written for —
        // and yet it is a conflict, which has no pieces to offer for a
        // different reason. Reading it as new would take them away with
        // the wrong words attached.
        let patch = "\
diff --cc added.txt
index 5b79a82,34a1fdf..0000000
--- a/added.txt
+++ b/added.txt
@@@ -1,1 -1,1 +1,5 @@@
++<<<<<<< HEAD
 +ours
++=======
+ theirs
++>>>>>>> topic
";
        let patches = parse_patch(patch.as_bytes());
        assert!(is_combined(&patches));
        assert!(!is_new_file(&patches));
    }

    #[test]
    fn an_unmerged_path_contributes_no_rows() {
        // git printed no patch, so there is nothing to flatten; the pane
        // says what the two sides did instead, in the words the file row's
        // icon already uses.
        let patches = parse_patch(b"* Unmerged path ours-del.txt\n");
        assert!(is_unmerged_only(&patches));
        assert!(!is_combined(&patches));
        assert!(!is_new_file(&patches), "it is not a new file either");
        assert!(flatten_patches(&patches, true, &DiffColors::default()).is_empty());
    }

    #[test]
    fn a_unified_hunk_heading_is_unchanged_by_the_combined_form() {
        let rows = flatten_patches(
            &parse_patch(EDITED.as_bytes()),
            true,
            &DiffColors::default(),
        );
        assert_eq!(rows[0].text, "@@ -1,2 +1,2 @@");
        assert!(rows.iter().all(|r| r.markers.is_empty()));
    }

    #[test]
    fn image_data_urls_are_base64_with_the_mime_up_front() {
        assert_eq!(
            image_data_url("image/png", b"abc"),
            "data:image/png;base64,YWJj"
        );
        assert_eq!(image_data_url("image/gif", b""), "data:image/gif;base64,");
    }

    #[test]
    fn human_sizes_step_through_units() {
        assert_eq!(human_size(0), "0 B");
        assert_eq!(human_size(1023), "1023 B");
        assert_eq!(human_size(1024), "1.0 KB");
        assert_eq!(human_size(1536), "1.5 KB");
        assert_eq!(human_size(239_616), "234 KB");
        assert_eq!(human_size(1_258_291), "1.2 MB");
        assert_eq!(human_size(17 * 1024 * 1024 * 1024), "17 GB");
    }
}
