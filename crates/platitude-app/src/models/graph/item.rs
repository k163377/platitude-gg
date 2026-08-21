//! One row of the graph: the item the view reads its roles off, and the
//! turning of a walked commit into one.

use std::collections::HashMap;

use super::*;

// Kept lean: one instance per commit in the window. The short sha is
// derived in QML from `oid_hex` (mechanical substring); `avatar` is a
// packed local identicon code (see encode::avatar_code). PartialEq
// feeds the in-place replacement: unchanged rows emit no dataChanged.
//
// **Fifteen fields is the ceiling** — `#[derive(QModelItem)]` refuses a
// sixteenth. Anything the rows need that QML never reads belongs on the
// way in rather than here: the lane count each row needs is taken off
// the `LogRow` while the item is built (`max_lanes`).
#[derive(QModelItem, Default, Clone, PartialEq)]
pub struct GraphRowItem {
    pub(super) oid_hex: String,
    pub(super) author: String,
    /// The address the picture is filed under — already folded and read
    /// through mailmap by the log parser. Kept on the row so re-reading
    /// the assignments needs no second pass over git.
    pub(super) author_email: String,
    pub(super) atime: i64,
    pub(super) subject: String,
    pub(super) node_lane: i32,
    pub(super) node_color: i32,
    pub(super) avatar: i32,
    /// A `file:` URL when this author has a picture, empty otherwise —
    /// resolved here rather than in QML so a delegate coming back from
    /// the reuse pool has the answer already in its row.
    pub(super) avatar_url: String,
    /// Packed `Co-authored-by` records (see `encode::encode_co_authors`) —
    /// the first draws the badge on the node, all of them are named in
    /// the row's hover.
    pub(super) co_authors: String,
    /// Message body without the co-author trailers — the row's hover.
    pub(super) body: String,
    pub(super) geometry: String,
    pub(super) labels: String,
    /// `stash@{n}` when the row is a stash; empty otherwise.
    pub(super) stash_ref: String,
    /// The find bar's line is somewhere in this row. False for every row
    /// while nothing is being searched for — the delegate dims off the
    /// pane's own "there is a search on", so an all-false model with no
    /// query dims nothing.
    pub(super) matched: bool,
}

impl platitude_core::mem::Footprint for GraphRowItem {
    fn heap_bytes(&self) -> usize {
        self.oid_hex.heap_bytes()
            + self.author.heap_bytes()
            + self.author_email.heap_bytes()
            + self.subject.heap_bytes()
            + self.avatar_url.heap_bytes()
            + self.co_authors.heap_bytes()
            + self.body.heap_bytes()
            + self.geometry.heap_bytes()
            + self.labels.heap_bytes()
            + self.stash_ref.heap_bytes()
    }
}

pub(super) fn to_row_item(row: &LogRow, avatars: &crate::hub::AvatarUrls) -> GraphRowItem {
    GraphRowItem {
        oid_hex: row.oid_hex.clone(),
        author: row.author.clone(),
        author_email: row.author_email.clone(),
        atime: row.time,
        subject: row.subject.clone(),
        node_lane: i32::from(row.node_lane),
        node_color: i32::from(row.node_color),
        avatar: crate::encode::avatar_code(&row.author),
        avatar_url: avatars.url_of(&row.author_email),
        co_authors: crate::encode::encode_co_authors(&row.co_authors),
        body: row.body.clone(),
        geometry: encode_geometry(&row.segments),
        labels: encode_labels(&row.labels),
        stash_ref: row.stash_ref.clone(),
        // Set by the marking pass that runs before anyone sees the row
        // (`mark_incoming`), so a chunk arriving under a standing query
        // arrives already lit.
        matched: false,
    }
}
