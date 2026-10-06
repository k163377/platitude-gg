//! One row of the graph: the item the view reads its roles off, and the
//! turning of a walked commit into one.

use qtbridge::qtbridge_type_lib::QVariantMap;

use crate::encode::{Chips, Fields, Lanes, Listed, Mates, Record, field};

use super::*;

// Kept lean: one instance per commit in the window. PartialEq feeds the
// in-place replacement: unchanged rows emit no dataChanged.
//
// Fifteen fields is the ceiling — `#[derive(QModelItem)]` refuses a
// sixteenth. What QML never reads is taken off the `LogRow` on the way in
// (`max_lanes`).
#[derive(QModelItem, Default, Clone, PartialEq)]
pub struct GraphRowItem {
    pub(super) oid_hex: String,
    pub(super) author: String,
    /// The address the picture is filed under (mailmapped and folded by
    /// the log parser), kept so re-reading the assignments needs no git.
    pub(super) author_email: String,
    pub(super) atime: i64,
    pub(super) subject: String,
    pub(super) node_lane: i32,
    pub(super) node_color: i32,
    pub(super) avatar: i32,
    /// A `file:` URL when this author has a picture, else empty — resolved
    /// here so a delegate back from the reuse pool has it in its row.
    pub(super) avatar_url: String,
    /// The `Co-authored-by` records — the first draws the badge on the
    /// node, all of them are named in the row's hover.
    pub(super) co_authors: Mates,
    /// Message body without the co-author trailers — the row's hover.
    pub(super) body: String,
    /// The lane segments the row's cell draws.
    pub(super) geometry: Lanes,
    /// The chips the row carries, in the order the front card reads them.
    pub(super) labels: Chips,
    /// `stash@{n}` when the row is a stash; empty otherwise.
    pub(super) stash_ref: String,
    /// The find bar's line is in this row; false everywhere with no query.
    /// Dimming keys off `searching`, so all-false with no query dims
    /// nothing.
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
/// working tree's row nor a stash; `None` where the window holds no commit.
/// A stash is no branch's history (デザイン規約 §変更を退避する), yet a
/// fresh one's time stands it above the tip — so the rule is written once
/// here, not by each asker (`GraphModel::newest_commit_row`).
///
/// A scan, but of the top: only the working tree's row and the stashes
/// can stand over the first commit.
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
        co_authors: crate::encode::mates_of(&row.co_authors),
        body: row.body.clone(),
        geometry: crate::encode::lanes_of(&row.segments),
        labels: crate::encode::chips_of(&row.labels, pr),
        stash_ref: row.stash_ref.clone(),
        // Set by `mark_incoming` before anyone sees the row.
        matched: false,
    }
}

/// One row of a choice of commits, as the right pane lists it above the
/// changed files (デザイン規約 §複数のコミットを選ぶ). Carries the body and
/// the credits so its hover card is the graph's own (`CommitHoverCard`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChosenRow {
    pub oid_hex: String,
    pub sha8: String,
    pub subject: String,
    pub body: String,
    pub author: String,
    pub atime: i64,
    pub avatar: i32,
    pub avatar_url: String,
    pub mates: Mates,
}

pub type ChosenRows = Listed<ChosenRow>;

/// The six counts a worktree's uncommitted row shows, in the order
/// both sides draw them. Another worktree's row carries its own; this
/// window's row reads the view's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Tally {
    pub added: i32,
    pub modified: i32,
    pub deleted: i32,
    pub renamed: i32,
    pub copied: i32,
    pub conflicted: i32,
}

impl Record for Tally {
    fn to_map(&self) -> QVariantMap {
        Fields::new()
            .put("added", &self.added)
            .put("modified", &self.modified)
            .put("deleted", &self.deleted)
            .put("renamed", &self.renamed)
            .put("copied", &self.copied)
            .put("conflicted", &self.conflicted)
            .done()
    }

    fn from_map(map: &QVariantMap) -> Result<Self, ()> {
        Ok(Self {
            added: field(map, "added")?,
            modified: field(map, "modified")?,
            deleted: field(map, "deleted")?,
            renamed: field(map, "renamed")?,
            copied: field(map, "copied")?,
            conflicted: field(map, "conflicted")?,
        })
    }
}

impl platitude_core::mem::Footprint for Tally {
    fn heap_bytes(&self) -> usize {
        0
    }
}

impl ChosenRow {
    pub(super) fn of(row: &GraphRowItem) -> Self {
        Self {
            oid_hex: row.oid_hex.clone(),
            sha8: row
                .oid_hex
                .get(..8)
                .unwrap_or(row.oid_hex.as_str())
                .to_string(),
            subject: row.subject.clone(),
            body: row.body.clone(),
            author: row.author.clone(),
            atime: row.atime,
            avatar: row.avatar,
            avatar_url: row.avatar_url.clone(),
            mates: row.co_authors.clone(),
        }
    }
}

impl Record for ChosenRow {
    fn to_map(&self) -> QVariantMap {
        Fields::new()
            .put("oid", &self.oid_hex)
            .put("sha8", &self.sha8)
            .put("subject", &self.subject)
            .put("body", &self.body)
            .put("author", &self.author)
            .put("atime", &self.atime)
            .put("avatar", &self.avatar)
            .put("avatarUrl", &self.avatar_url)
            .put("mates", &self.mates)
            .done()
    }

    fn from_map(map: &QVariantMap) -> Result<Self, ()> {
        Ok(Self {
            oid_hex: field(map, "oid")?,
            sha8: field(map, "sha8")?,
            subject: field(map, "subject")?,
            body: field(map, "body")?,
            author: field(map, "author")?,
            atime: field(map, "atime")?,
            avatar: field(map, "avatar")?,
            avatar_url: field(map, "avatarUrl")?,
            mates: field(map, "mates")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qtbridge::QVariantConvertible;

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

    /// A chosen row carries the credits as the same list the graph row
    /// does, nested — and comes back out of Qt's containers whole.
    #[test]
    fn a_chosen_row_round_trips_with_its_credits_nested() {
        let row = GraphRowItem {
            oid_hex: "0123456789abcdef".into(),
            subject: "the subject".into(),
            body: "the body".into(),
            author: "Ada".into(),
            atime: 1_700_000_000,
            avatar: 42,
            avatar_url: "file:///a.png".into(),
            co_authors: crate::encode::mates_of(&[platitude_core::details::CoAuthor {
                name: "Bob".into(),
                email: "bob@example.com".into(),
            }]),
            ..GraphRowItem::default()
        };
        let chosen: ChosenRows = Listed::new(vec![ChosenRow::of(&row)]);
        assert_eq!(chosen[0].sha8, "01234567");
        assert_eq!(chosen[0].mates[0].name, "Bob");
        assert_eq!(
            <ChosenRows as QVariantConvertible>::try_from_qvariant(&chosen.to_qvariant()),
            Ok(chosen)
        );
    }
}
