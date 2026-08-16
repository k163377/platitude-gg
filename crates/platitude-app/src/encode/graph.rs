//! The graph pane's own encodings: the lane geometry of a row, and the
//! colours a name picks for itself.

use platitude_core::graph::{Segment, SegmentKind};

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
/// The two must never come out the same. The graph's answer is kept
/// wherever it has one; a side it has none for takes a stable colour off
/// its own name, so the same conflict reopens in the same colours. If the
/// two still land together, ours keeps its colour and theirs moves on by
/// one (during a rebase ours is the upstream — `conflict::sides()` has
/// already sorted out which is which).
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
/// tokens' shape — `t<lane>.<color>`, uppercase for a dashed leash. A
/// leash does reach here: with many starting refs the walk can emit
/// thousands of commits before it gets to HEAD, and the WIP row waits on
/// that lane the whole way (measured on JetBrains/kotlin).
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn conflict_sides_keep_the_colours_the_graph_lent_them() {
        assert_eq!(conflict_side_colors((3, "main"), (5, "topic")), (3, 5));
    }

    #[test]
    fn conflict_sides_that_landed_together_move_theirs_on() {
        assert_eq!(conflict_side_colors((3, "main"), (3, "topic")), (3, 4));
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
        assert_eq!(
            conflict_side_colors((-1, "main~3"), (5, "topic")),
            (ours, 5)
        );
    }

    #[test]
    fn two_colourless_sides_still_come_out_apart() {
        let (a, b) = conflict_side_colors((-1, "main"), (-1, "topic"));
        assert_ne!(a, b);
        let (a, b) = conflict_side_colors((-1, ""), (-1, ""));
        assert_ne!(a, b);
        let (a, b) = conflict_side_colors((99, "main"), (-7, "main"));
        assert_ne!(a, b);
    }
}
