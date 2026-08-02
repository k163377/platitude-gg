//! Pure encoding helpers: core DTOs → compact strings the QML layer decodes
//! mechanically (draw tokens / chip records). All *semantic* work already
//! happened in platitude-core; QML only draws what these strings say.

use platitude_core::details::DiffTarget;
use platitude_core::graph::{Segment, SegmentKind};
use platitude_core::parse::diff::{DiffLineKind, FilePatch};
use platitude_core::session::{LabelKind, RefLabel};

/// Record separator for label chips (cannot occur in refnames).
pub const LABEL_SEP: char = '\u{1f}';

/// Segments → `;`-joined draw tokens: `t<lane>.<color>` (through),
/// `i<lane>.<color>` (into node), `o<lane>.<color>` (out of node).
pub fn encode_geometry(segments: &[Segment]) -> String {
    let mut out = String::with_capacity(segments.len() * 6);
    for (i, s) in segments.iter().enumerate() {
        if i > 0 {
            out.push(';');
        }
        out.push(match s.kind {
            SegmentKind::Through => 't',
            SegmentKind::IntoNode => 'i',
            SegmentKind::OutOfNode => 'o',
        });
        out.push_str(&s.lane.to_string());
        out.push('.');
        out.push_str(&s.color.to_string());
    }
    out
}

/// Labels → `\u{1f}`-joined chip records `KHRtext`:
/// K = `H`ead / `L`ocal / `R`emote / `T`ag, H = head?, R = has-remote?
pub fn encode_labels(labels: &[RefLabel]) -> String {
    let mut out = String::new();
    for (i, l) in labels.iter().enumerate() {
        if i > 0 {
            out.push(LABEL_SEP);
        }
        out.push(match l.kind {
            LabelKind::Head => 'H',
            LabelKind::LocalBranch => 'L',
            LabelKind::RemoteBranch => 'R',
            LabelKind::Tag => 'T',
        });
        out.push(if l.is_head { '1' } else { '0' });
        out.push(if l.has_remote { '1' } else { '0' });
        out.push_str(&l.text);
    }
    out
}

/// Deterministic identicon code for an author (GitHub-style 5x5 pattern,
/// generated locally — fetching real avatars would violate the
/// no-network-except-git constraint).
///
/// Layout: bits 0..15 = left 3 columns of a 5x5 grid (row-major, mirrored
/// to the right by the renderer), bits 15..18 = palette index (0..8).
pub fn avatar_code(author: &str) -> i32 {
    // FNV-1a 32-bit.
    let mut hash: u32 = 0x811c_9dc5;
    for b in author.as_bytes() {
        hash ^= u32::from(*b);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    let mut pattern = hash & 0x7fff;
    if pattern == 0 {
        pattern = 0b00000_00100_00000; // center dot fallback
    }
    let color = (hash >> 15) % 8;
    (pattern | (color << 15)) as i32
}

/// Lanes that touch the bottom edge of a row (from its geometry tokens):
/// `t` segments plus `o` targets. Used by the truncation footer to draw
/// the lanes running off the end of the window. Output: `lane.color;...`.
pub fn tail_lanes(geometry: &str) -> String {
    let mut seen = std::collections::HashSet::new();
    let mut out = String::new();
    for token in geometry.split(';').filter(|t| t.len() > 1) {
        let kind = token.as_bytes()[0];
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
            out.push_str(rest);
        }
    }
    out
}

/// Stable identity of a diff request (stale-response guard in the pane).
pub fn diff_key(target: &DiffTarget) -> String {
    match target {
        DiffTarget::Commit { oid, path, .. } => format!("commit:{}:{path}", oid.to_hex()),
        DiffTarget::Staged { path, .. } => format!("staged:{path}"),
        DiffTarget::Unstaged { path } => format!("unstaged:{path}"),
        DiffTarget::Untracked { path } => format!("untracked:{path}"),
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
    pub text: String,
}

/// Flattens parsed patches into displayable rows (hunk headers inline).
pub fn flatten_patches(patches: &[FilePatch]) -> Vec<DiffRow> {
    let mut rows = Vec::new();
    for patch in patches {
        if patch.is_binary {
            rows.push(DiffRow {
                kind: "meta",
                old_no: -1,
                new_no: -1,
                text: String::from("(binary file)"),
            });
            continue;
        }
        for hunk in &patch.hunks {
            let heading = if hunk.heading.is_empty() {
                String::new()
            } else {
                format!(" {}", hunk.heading)
            };
            rows.push(DiffRow {
                kind: "hunk",
                old_no: -1,
                new_no: -1,
                text: format!(
                    "@@ -{},{} +{},{} @@{heading}",
                    hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count
                ),
            });
            for line in &hunk.lines {
                let kind = match line.kind {
                    DiffLineKind::Context => "ctx",
                    DiffLineKind::Addition => "add",
                    DiffLineKind::Deletion => "del",
                    DiffLineKind::NoNewline => "meta",
                };
                rows.push(DiffRow {
                    kind,
                    old_no: line.old_no.map_or(-1, |n| n as i32),
                    new_no: line.new_no.map_or(-1, |n| n as i32),
                    text: line.text.clone(),
                });
            }
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use platitude_core::parse::diff::parse_patch;

    #[test]
    fn geometry_tokens_round_trip_by_eye() {
        let segs = [
            Segment {
                kind: SegmentKind::Through,
                lane: 0,
                color: 3,
            },
            Segment {
                kind: SegmentKind::IntoNode,
                lane: 2,
                color: 11,
            },
            Segment {
                kind: SegmentKind::OutOfNode,
                lane: 1,
                color: 0,
            },
        ];
        assert_eq!(encode_geometry(&segs), "t0.3;i2.11;o1.0");
        assert_eq!(encode_geometry(&[]), "");
    }

    #[test]
    fn tail_lanes_picks_bottom_touching_segments() {
        assert_eq!(tail_lanes("t0.3;i2.11;o1.0"), "0.3;1.0");
        assert_eq!(tail_lanes("i2.5"), "", "into-node stops at the node");
        assert_eq!(tail_lanes("t0.1;o0.2"), "0.1", "deduped by lane");
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
            },
            RefLabel {
                text: "v1.0".into(),
                kind: LabelKind::Tag,
                has_remote: false,
                is_head: false,
            },
        ];
        assert_eq!(encode_labels(&labels), "L11main\u{1f}T00v1.0");
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
        let rows = flatten_patches(&parse_patch(patch.as_bytes()));
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
    fn binary_patch_flattens_to_meta_row() {
        let patch = "\
diff --git a/x.png b/x.png
Binary files a/x.png and b/x.png differ
";
        let rows = flatten_patches(&parse_patch(patch.as_bytes()));
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, "meta");
    }
}
