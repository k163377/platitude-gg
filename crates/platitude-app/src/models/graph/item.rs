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
// way in: the lane count each row needs is taken off the `LogRow` while
// the item is built (`max_lanes`).
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
    /// resolved here, so a delegate coming back from the reuse pool has
    /// the answer already in its row.
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

/// The row "the newest commit" names: the first that is neither the
/// working tree's own row nor a stash. `None` where the window holds no
/// commit of the history at all.
///
/// **Written once for the three that ask it** (`GraphModel::newest_commit_row`):
/// the page's default landing when it has only row numbers to go by, and
/// the two automation drivers that pick a commit to photograph and to
/// time. Three copies of "skip the working tree's row" is how all three
/// came to keep the stash — a stash is no branch's history
/// (デザイン規約 §変更を退避する), and its commit time is when it was
/// written, so a fresh one stands above the branch's own tip.
///
/// A scan, but of the top: only the working tree's row and the stashes
/// can stand over the first commit, and a repository with more stashes
/// than commits is the whole of the walk either way (CLAUDE.md §性能予算).
pub(super) fn newest_commit_row(rows: &[GraphRowItem]) -> Option<usize> {
    rows.iter()
        .position(|r| r.stash_ref.is_empty() && !Oid::hex_is_zero(&r.oid_hex))
}

pub(super) fn to_row_item(
    row: &LogRow,
    avatars: &crate::hub::AvatarUrls,
    pr: &std::collections::HashSet<String>,
) -> GraphRowItem {
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
        labels: encode_labels(&row.labels, pr),
        stash_ref: row.stash_ref.clone(),
        // Set by the marking pass that runs before anyone sees the row
        // (`mark_incoming`), so a chunk arriving under a standing query
        // arrives already lit.
        matched: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit(oid_hex: &str) -> GraphRowItem {
        GraphRowItem {
            oid_hex: oid_hex.into(),
            ..GraphRowItem::default()
        }
    }

    fn stash(oid_hex: &str, stash_ref: &str) -> GraphRowItem {
        GraphRowItem {
            stash_ref: stash_ref.into(),
            ..commit(oid_hex)
        }
    }

    fn wip() -> GraphRowItem {
        commit("0000000000000000000000000000000000000000")
    }

    #[test]
    fn newest_commit_row_is_the_top_of_a_plain_walk() {
        let rows = [commit("a1"), commit("b2")];
        assert_eq!(newest_commit_row(&rows), Some(0));
    }

    #[test]
    fn newest_commit_row_walks_past_the_working_tree_and_the_stashes() {
        let rows = [
            wip(),
            stash("c3", "stash@{0}"),
            stash("d4", "stash@{1}"),
            commit("a1"),
        ];
        assert_eq!(newest_commit_row(&rows), Some(3));
    }

    #[test]
    fn newest_commit_row_is_none_where_the_window_holds_no_commit() {
        let rows = [wip(), stash("c3", "stash@{0}")];
        assert_eq!(newest_commit_row(&rows), None);
        assert_eq!(newest_commit_row(&[]), None);
    }
}
