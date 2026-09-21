//! The graph pane's own shapes: the lane geometry of a row, and the
//! colours a name picks for itself.

use platitude_core::graph::{Segment, SegmentKind};
use qtbridge::qtbridge_type_lib::QVariantMap;

use super::wire::{Fields, Listed, Record, field};

/// The segments a row's lane cell draws, as it draws them: one record
/// per segment — `kind` (`through` / `into` / `out`), `lane`, `color`
/// and whether it is `dashed` (the WIP leash).
pub type Lanes = Listed<Segment>;

pub fn lanes_of(segments: &[Segment]) -> Lanes {
    Listed::new(segments.to_vec())
}

/// The word a segment's kind goes out under. **The list itself** — the
/// canvas branches on these three words (`GraphLaneCell`).
fn kind_word(kind: SegmentKind) -> &'static str {
    match kind {
        SegmentKind::Through => "through",
        SegmentKind::IntoNode => "into",
        SegmentKind::OutOfNode => "out",
    }
}

fn kind_from_word(word: &str) -> Option<SegmentKind> {
    match word {
        "through" => Some(SegmentKind::Through),
        "into" => Some(SegmentKind::IntoNode),
        "out" => Some(SegmentKind::OutOfNode),
        _ => None,
    }
}

impl Record for Segment {
    fn to_map(&self) -> QVariantMap {
        Fields::new()
            .put("kind", kind_word(self.kind))
            .put("lane", &i32::from(self.lane))
            .put("color", &i32::from(self.color))
            .put("dashed", &self.dashed)
            .done()
    }

    fn from_map(map: &QVariantMap) -> Result<Self, ()> {
        Ok(Self {
            kind: kind_from_word(&field::<String>(map, "kind")?).ok_or(())?,
            lane: u16::try_from(field::<i32>(map, "lane")?).map_err(|_| ())?,
            color: u8::try_from(field::<i32>(map, "color")?).map_err(|_| ())?,
            dashed: field(map, "dashed")?,
        })
    }
}

/// The segments spelled as one line, for the smoke hooks: a lane is a
/// stroke a couple of pixels wide, and whether one of them is dotted is
/// not a question a screenshot answers. `t` / `i` / `o` for the kind,
/// uppercase where dashed, then `<lane>.<color>`, `;` between segments.
/// Nothing draws from this — the delegate takes the records.
pub fn spell_lanes(segments: &[Segment]) -> String {
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
/// generated locally — git's own commands are the only network this
/// app has).
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
/// The two always come out apart. The graph's answer is kept wherever it
/// has one; a side it has none for takes a stable colour off its own name,
/// so the same conflict reopens in the same colours. If the two still land
/// together, ours keeps its colour and theirs moves on by one (during a
/// rebase ours is the upstream — `conflict::sides()` has already sorted
/// out which is which).
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

/// Lanes that touch the bottom edge of a row: `through` segments plus
/// `out` targets, each as a `through` on its lane, one per lane. Used by
/// the truncation footer to draw the lanes running off the end of the
/// window. A leash does reach here: with many starting refs the walk can
/// emit thousands of commits before it gets to HEAD, and the WIP row
/// waits on that lane the whole way (measured on JetBrains/kotlin).
pub fn tail_lanes(segments: &[Segment]) -> Lanes {
    let mut out: Vec<Segment> = Vec::new();
    for s in segments {
        if s.kind == SegmentKind::IntoNode || out.iter().any(|seen| seen.lane == s.lane) {
            continue;
        }
        out.push(Segment {
            kind: SegmentKind::Through,
            ..*s
        });
    }
    Listed::new(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use qtbridge::qtbridge_type_lib::QVariant;

    fn seg(kind: SegmentKind, lane: u16, color: u8, dashed: bool) -> Segment {
        Segment {
            kind,
            lane,
            color,
            dashed,
        }
    }

    #[test]
    fn lanes_round_trip_through_the_records_the_canvas_reads() {
        let lanes = lanes_of(&[
            seg(SegmentKind::Through, 0, 3, false),
            seg(SegmentKind::IntoNode, 2, 11, false),
            seg(SegmentKind::OutOfNode, 1, 0, true),
        ]);
        assert_eq!(Lanes::try_from(&QVariant::from(&lanes)), Ok(lanes.clone()));
        // The words the canvas branches on, spelled out whole.
        let first = lanes[0].to_map();
        assert_eq!(field::<String>(&first, "kind"), Ok("through".into()));
        assert_eq!(field::<i32>(&first, "lane"), Ok(0));
        assert_eq!(field::<i32>(&first, "color"), Ok(3));
        assert_eq!(field::<bool>(&first, "dashed"), Ok(false));
        assert_eq!(
            field::<String>(&lanes[1].to_map(), "kind"),
            Ok("into".into())
        );
        assert_eq!(
            field::<String>(&lanes[2].to_map(), "kind"),
            Ok("out".into())
        );
        assert_eq!(
            Lanes::try_from(&QVariant::from(&lanes_of(&[]))),
            Ok(Lanes::default())
        );
    }

    #[test]
    fn lanes_are_spelled_for_the_smoke_hooks_by_eye() {
        let segs = [
            seg(SegmentKind::Through, 0, 3, false),
            seg(SegmentKind::IntoNode, 2, 11, false),
            seg(SegmentKind::OutOfNode, 1, 0, true),
        ];
        assert_eq!(
            spell_lanes(&segs),
            "t0.3;i2.11;O1.0",
            "dashed segments spell uppercase"
        );
        assert_eq!(spell_lanes(&[]), "");
    }

    #[test]
    fn tail_lanes_picks_bottom_touching_segments() {
        let through = |lane, color, dashed| seg(SegmentKind::Through, lane, color, dashed);
        assert_eq!(
            tail_lanes(&[
                through(0, 3, false),
                seg(SegmentKind::IntoNode, 2, 11, false),
                seg(SegmentKind::OutOfNode, 1, 0, false),
            ]),
            lanes_of(&[through(0, 3, false), through(1, 0, false)])
        );
        assert_eq!(
            tail_lanes(&[seg(SegmentKind::IntoNode, 2, 5, false)]),
            Lanes::default(),
            "into-node stops at the node"
        );
        assert_eq!(
            tail_lanes(&[
                through(0, 1, false),
                seg(SegmentKind::OutOfNode, 0, 2, false)
            ]),
            lanes_of(&[through(0, 1, false)]),
            "deduped by lane"
        );
        assert_eq!(
            tail_lanes(&[through(0, 4, true), seg(SegmentKind::OutOfNode, 1, 5, true)]),
            lanes_of(&[through(0, 4, true), through(1, 5, true)]),
            "a leash keeps dotting past the cut"
        );
        assert_eq!(tail_lanes(&[]), Lanes::default());
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
