//! The print the session keeps of a row it has already delivered, so a
//! rebuild can tell "the same picture" from "changed".

use super::*;

/// One delivered row, small enough to keep for every row on screen.
///
/// The only question asked of it is whether a rebuild arrived at the same
/// picture, which a hash answers in sixteen bytes where the row takes
/// hundreds. The window can be widened (`LogOptions::limit`), so this is
/// the part of the graph's cost that grows with what somebody asks to see.
///
/// **Two halves because the chips move on their own**: a refs read writes
/// new badges into rows already delivered (`apply_refs`) holding only the
/// new chips, so the half it restates is the only half it can.
///
/// **`rest` is itself two hashes, what the commit says and where it is
/// drawn**: a delete's stand-in lays rows out again from the walk's record
/// (`session::leaving`), which keeps the first and not the row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RowPrint {
    /// Everything except the chips.
    pub(super) rest: u64,
    pub(super) labels: u64,
}

impl crate::mem::Footprint for RowPrint {
    fn heap_bytes(&self) -> usize {
        0
    }
}

impl RowPrint {
    pub(super) fn of(row: &LogRow) -> Self {
        Self {
            rest: Self::rest_of(row),
            labels: Self::labels_of(&row.labels),
        }
    }

    /// The print of a row whose content hash is `content`
    /// ([`Self::content_of`]), drawn on `lanes` under `labels` — what
    /// [`Self::of`] gives the whole row.
    pub(super) fn laid(content: u64, lanes: &GraphRow, labels: &[RefLabel]) -> Self {
        let GraphRow {
            row,
            node_lane,
            node_color,
            segments,
            width,
        } = lanes;
        Self {
            rest: Self::placed(content, *row, *node_lane, *node_color, *width, segments),
            labels: Self::labels_of(labels),
        }
    }

    fn rest_of(row: &LogRow) -> u64 {
        Self::placed(
            Self::content_of(row),
            row.row,
            row.node_lane,
            row.node_color,
            row.width,
            &row.segments,
        )
    }

    /// Where a row is drawn, over what it says.
    fn placed(
        content: u64,
        row: u32,
        node_lane: u16,
        node_color: u8,
        width: u16,
        segments: &[Segment],
    ) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        content.hash(&mut h);
        row.hash(&mut h);
        node_lane.hash(&mut h);
        node_color.hash(&mut h);
        width.hash(&mut h);
        for s in segments {
            (s.kind as u8).hash(&mut h);
            s.lane.hash(&mut h);
            s.color.hash(&mut h);
            s.dashed.hash(&mut h);
        }
        segments.len().hash(&mut h);
        h.finish()
    }

    /// Everything a row says but its place and its chips.
    ///
    /// **Destructured on purpose**: a field added to [`LogRow`] and not
    /// hashed here or in [`Self::placed`] is a change the rebuild would
    /// call the old picture. Naming every field makes that a build error.
    pub(super) fn content_of(row: &LogRow) -> u64 {
        use std::hash::{Hash, Hasher};
        let LogRow {
            row: _,
            oid_hex,
            short_sha,
            author,
            author_email,
            co_authors,
            time,
            subject,
            body,
            node_lane: _,
            node_color: _,
            width: _,
            segments: _,
            labels: _,
            published,
            stash_ref,
            parents,
            carried,
            provisional,
        } = row;
        let mut h = std::collections::hash_map::DefaultHasher::new();
        oid_hex.hash(&mut h);
        short_sha.hash(&mut h);
        author.hash(&mut h);
        author_email.hash(&mut h);
        for a in co_authors {
            a.name.hash(&mut h);
            a.email.hash(&mut h);
        }
        co_authors.len().hash(&mut h);
        time.hash(&mut h);
        subject.hash(&mut h);
        body.hash(&mut h);
        stash_ref.hash(&mut h);
        published.hash(&mut h);
        parents.hash(&mut h);
        provisional.hash(&mut h);
        // Another worktree staging one more file moves nothing else on its row
        // (its id is all-zero whatever it holds).
        if let Some(carried) = carried {
            carried.name.hash(&mut h);
            carried.path.hash(&mut h);
            carried.head.hash(&mut h);
            let k = &carried.kinds;
            (k.added, k.modified, k.deleted).hash(&mut h);
            (k.renamed, k.copied, k.conflicted).hash(&mut h);
        }
        carried.is_some().hash(&mut h);
        h.finish()
    }

    pub(super) fn labels_of(labels: &[RefLabel]) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        for l in labels {
            l.text.hash(&mut h);
            (l.kind as u8).hash(&mut h);
            l.has_remote.hash(&mut h);
            l.is_head.hash(&mut h);
            l.here.hash(&mut h);
            l.remote.hash(&mut h);
        }
        labels.len().hash(&mut h);
        h.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(digit: char) -> Oid {
        Oid::from_hex_str(&digit.to_string().repeat(40)).unwrap()
    }

    fn a_row() -> LogRow {
        LogRow {
            row: 3,
            oid_hex: "a".repeat(40),
            short_sha: "aaaaaaa".into(),
            author: "Ada".into(),
            author_email: "ada@example.com".into(),
            co_authors: vec![crate::details::CoAuthor {
                name: "Bo".into(),
                email: "bo@example.com".into(),
            }],
            time: 1_700_000_000,
            subject: "a subject".into(),
            body: "a body".into(),
            node_lane: 1,
            node_color: 2,
            width: 4,
            segments: vec![Segment {
                kind: crate::graph::SegmentKind::Through,
                lane: 1,
                color: 2,
                dashed: false,
            }],
            labels: vec![RefLabel {
                text: "main".into(),
                kind: LabelKind::LocalBranch,
                has_remote: false,
                is_head: true,
                here: true,
                remote: String::new(),
                held_elsewhere: false,
                locked: false,
            }],
            stash_ref: String::new(),
            published: false,
            parents: Box::from([oid('b')]),
            carried: None,
            provisional: false,
        }
    }

    /// Every field is moved one at a time, so a field the print forgets
    /// fails here instead of quietly leaving the old row on screen.
    #[test]
    fn a_row_that_differs_anywhere_prints_differently() {
        let base = a_row();
        let print = RowPrint::of(&base);

        type Moved = (&'static str, Box<dyn Fn(&mut LogRow)>);
        let moved: Vec<Moved> = vec![
            ("row", Box::new(|r: &mut LogRow| r.row = 4)),
            ("oid_hex", Box::new(|r| r.oid_hex = "b".repeat(40))),
            ("short_sha", Box::new(|r| r.short_sha = "bbbbbbb".into())),
            ("author", Box::new(|r| r.author = "Bo".into())),
            (
                "author_email",
                Box::new(|r| r.author_email = "b@e.com".into()),
            ),
            ("co_authors", Box::new(|r| r.co_authors.clear())),
            ("time", Box::new(|r| r.time += 1)),
            ("subject", Box::new(|r| r.subject = "another".into())),
            ("body", Box::new(|r| r.body = "another".into())),
            ("node_lane", Box::new(|r| r.node_lane = 5)),
            ("node_color", Box::new(|r| r.node_color = 5)),
            ("width", Box::new(|r| r.width = 9)),
            ("segments", Box::new(|r| r.segments[0].dashed = true)),
            ("stash_ref", Box::new(|r| r.stash_ref = "stash@{0}".into())),
            ("published", Box::new(|r| r.published = true)),
            ("parents", Box::new(|r| r.parents = Box::from([oid('c')]))),
            (
                "carried",
                Box::new(|r| {
                    r.carried = Some(crate::session::Carried {
                        name: "worktree".into(),
                        path: "/tmp/worktree".into(),
                        head: oid('d'),
                        kinds: crate::status::Kinds {
                            modified: 1,
                            ..crate::status::Kinds::default()
                        },
                    })
                }),
            ),
        ];
        for (field, change) in moved {
            let mut row = base.clone();
            change(&mut row);
            assert_ne!(row, base, "the change to {field} landed");
            assert_ne!(
                RowPrint::of(&row).rest,
                print.rest,
                "{field} is not in the print, so a rebuild would call this \
                 row unchanged and leave the old one on screen"
            );
        }

        // The chips are the other half, and the half a refs read restates
        // on its own (`apply_refs`).
        let mut chipped = base.clone();
        chipped.labels[0].is_head = false;
        assert_eq!(RowPrint::of(&chipped).rest, print.rest, "the same row");
        assert_ne!(RowPrint::of(&chipped).labels, print.labels);
        assert_eq!(
            RowPrint::labels_of(&chipped.labels),
            RowPrint::of(&chipped).labels,
            "what `apply_refs` writes back is what a rebuild computes"
        );
    }

    /// A row laid out again from the walk's record prints as the row itself
    /// would (`session::leaving`).
    #[test]
    fn a_row_laid_again_from_its_content_prints_as_itself() {
        let base = a_row();
        let print = RowPrint::of(&base);
        let lanes = GraphRow {
            row: base.row,
            node_lane: base.node_lane,
            node_color: base.node_color,
            segments: base.segments.clone(),
            width: base.width,
        };
        assert_eq!(
            RowPrint::laid(RowPrint::content_of(&base), &lanes, &base.labels),
            print,
            "a stand-in's print would call the walk that follows it a change"
        );
    }
}
