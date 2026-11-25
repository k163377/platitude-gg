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
    /// The address as git hands it over (mailmapped), kept so re-reading
    /// the assignments needs no git; a face is looked up by its key
    /// (`AvatarUrls::url_of`).
    pub(super) author_email: String,
    pub(super) author_time: i64,
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
/// worktree's row nor a stash; `None` where the window holds no commit.
/// A stash is no branch's history (デザイン規約 §変更を退避する), yet a
/// fresh one's time stands it above the tip — so the rule is written once
/// here, not by each asker (`GraphModel::newest_commit_row`).
///
/// A scan, but of the top: only the worktree's row and the stashes
/// can stand over the first commit.
pub(super) fn newest_commit_row(rows: &[GraphRowItem]) -> Option<usize> {
    rows.iter()
        .position(|r| r.stash_ref.is_empty() && r.names_a_commit())
}

impl GraphRowItem {
    /// Whether a commit stands on the row: a row of uncommitted work names
    /// none, and its id is empty — the bridge's word for none
    /// (`encode::oid_text`).
    pub(super) fn names_a_commit(&self) -> bool {
        !self.oid_hex.is_empty()
    }
}

/// A row as the delegate reads it. A row of uncommitted work carries its
/// place and nothing else: whose it is is asked of the model
/// (`GraphModel.carriedAt` / `keyAt`), as `GraphRowItem` has spent its
/// fifteen fields.
pub(super) fn to_row_item(
    row: &LogRow,
    avatars: &crate::hub::AvatarUrls,
    pr: &std::collections::HashSet<String>,
) -> GraphRowItem {
    let placed = GraphRowItem {
        node_lane: i32::from(row.node_lane),
        node_color: i32::from(row.node_color),
        geometry: crate::encode::lanes_of(&row.segments),
        // Set by `mark_incoming` before anyone sees the row.
        matched: false,
        ..GraphRowItem::default()
    };
    let Some(commit) = row.commit() else {
        return placed;
    };
    GraphRowItem {
        oid_hex: commit.oid.to_hex(),
        author: commit.author.clone(),
        author_email: commit.author_email.clone(),
        author_time: commit.time,
        subject: commit.subject.clone(),
        avatar: crate::encode::avatar_code(&commit.author),
        avatar_url: avatars.url_of(&commit.author_email),
        co_authors: crate::encode::mates_of(&commit.co_authors),
        body: commit.body.clone(),
        labels: crate::encode::chips_of(&commit.labels, pr),
        stash_ref: commit.stash_ref.clone(),
        ..placed
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
    pub author_time: i64,
    pub avatar: i32,
    pub avatar_url: String,
    pub mates: Mates,
    /// The lane to the row listed above and to the one below — "", "solid"
    /// or "dashed" (`GraphModel::chosen_lanes`).
    pub lane_above: String,
    pub lane_below: String,
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

    fn from_map(map: &QVariantMap) -> Option<Self> {
        Some(Self {
            added: field(map, "added")?,
            modified: field(map, "modified")?,
            deleted: field(map, "deleted")?,
            renamed: field(map, "renamed")?,
            copied: field(map, "copied")?,
            conflicted: field(map, "conflicted")?,
        })
    }
}

/// What a row of another worktree's uncommitted work answers when the
/// delegate asks: whose it is, where it is, the commit it stands on, and its
/// six tallies. Beside the items, since `GraphRowItem` has spent all fifteen
/// fields.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct CarriedRow {
    pub(super) name: String,
    pub(super) path: String,
    /// The repository's own worktree, which wears the house.
    pub(super) home: bool,
    /// That worktree's HEAD, the commit the row is drawn over — where a
    /// reading of the row lands once the row is gone and the worktree stands
    /// on nothing drawn (`RepoPage.settleCarriedAfterPass`).
    pub(super) head: String,
    pub(super) tally: Option<Tally>,
}

/// What `GraphModel.carriedAt` hands QML, under the same names.
impl crate::encode::Record for CarriedRow {
    fn to_map(&self) -> qtbridge::qtbridge_type_lib::QVariantMap {
        crate::encode::Fields::new()
            .put("name", &self.name)
            .put("path", &self.path)
            .put("home", &self.home)
            .put("head", &self.head)
            .put_record("tally", self.tally.as_ref())
            .done()
    }

    fn from_map(map: &qtbridge::qtbridge_type_lib::QVariantMap) -> Option<Self> {
        use crate::encode::field;
        Some(Self {
            name: field(map, "name")?,
            path: field(map, "path")?,
            home: field(map, "home")?,
            head: field(map, "head")?,
            tally: crate::encode::optional_record(map, "tally")?,
        })
    }
}

/// The graph colour each side of a conflict is drawn in (`GraphModel.sideColors`):
/// always one of the palette's, the two apart (`encode::conflict_side_colors`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SideColors {
    pub ours: i32,
    pub theirs: i32,
}

impl Record for SideColors {
    fn to_map(&self) -> QVariantMap {
        Fields::new()
            .put("ours", &self.ours)
            .put("theirs", &self.theirs)
            .done()
    }

    fn from_map(map: &QVariantMap) -> Option<Self> {
        Some(Self {
            ours: field(map, "ours")?,
            theirs: field(map, "theirs")?,
        })
    }
}

/// A remote-tracking branch's tip and the remote it belongs to, as an index
/// into the asker's list (`GraphModel.remotesHolding`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RemoteTip {
    pub oid: String,
    pub owner: i32,
}

impl Record for RemoteTip {
    fn to_map(&self) -> QVariantMap {
        Fields::new()
            .put("oid", &self.oid)
            .put("owner", &self.owner)
            .done()
    }

    fn from_map(map: &QVariantMap) -> Option<Self> {
        Some(Self {
            oid: field(map, "oid")?,
            owner: field(map, "owner")?,
        })
    }
}

impl platitude_core::mem::Footprint for Tally {
    fn heap_bytes(&self) -> usize {
        0
    }
}

impl ChosenRow {
    /// `row` as the list shows it, with the lanes its neighbours in the
    /// choice give it.
    pub(super) fn of(row: &GraphRowItem, (lane_above, lane_below): (&str, &str)) -> Self {
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
            author_time: row.author_time,
            avatar: row.avatar,
            avatar_url: row.avatar_url.clone(),
            mates: row.co_authors.clone(),
            lane_above: lane_above.to_string(),
            lane_below: lane_below.to_string(),
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
            .put("authorTime", &self.author_time)
            .put("avatar", &self.avatar)
            .put("avatarUrl", &self.avatar_url)
            .put("mates", &self.mates)
            .put("laneAbove", &self.lane_above)
            .put("laneBelow", &self.lane_below)
            .done()
    }

    fn from_map(map: &QVariantMap) -> Option<Self> {
        Some(Self {
            oid_hex: field(map, "oid")?,
            sha8: field(map, "sha8")?,
            subject: field(map, "subject")?,
            body: field(map, "body")?,
            author: field(map, "author")?,
            author_time: field(map, "authorTime")?,
            avatar: field(map, "avatar")?,
            avatar_url: field(map, "avatarUrl")?,
            mates: field(map, "mates")?,
            lane_above: field(map, "laneAbove")?,
            lane_below: field(map, "laneBelow")?,
        })
    }
}

/// The authors of `rows` as the settings card lists them — `Name <address>`
/// (`trailers::spell_identity`) as the newest of their rows writes it, one per
/// person (`avatar::key`, the key a face is filed under, so picking either of
/// a person's spellings files the same face), sorted; a prefill goes first as
/// written, and its person is not repeated (デザイン規約
/// §リポジトリの文は原文で描く).
pub(super) fn author_choices(
    rows: &[GraphRowItem],
    prefill_name: &str,
    prefill_email: &str,
) -> Vec<String> {
    use platitude_core::avatar::key;
    use platitude_core::trailers::spell_identity;
    let mut people = std::collections::HashSet::from([key(prefill_email)]);
    // A person's key is worked out once per spelling, not once per row.
    let mut spellings = std::collections::HashSet::new();
    let mut listed: Vec<String> = Vec::new();
    for row in rows {
        let email = row.author_email.as_str();
        if email.is_empty() || !spellings.insert(email) || !people.insert(key(email)) {
            continue;
        }
        listed.push(spell_identity(&row.author, email));
    }
    listed.sort();
    let mut out: Vec<String> = Vec::new();
    if !prefill_email.is_empty() {
        out.push(spell_identity(prefill_name, prefill_email));
    }
    out.extend(listed);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use qtbridge::QVariantConvertible;

    /// A person is listed once, under the newest spelling of their rows,
    /// and the one a prefill brings stands for them as it is written.
    #[test]
    fn the_author_choices_list_a_person_once_as_written() {
        let row = |author: &str, email: &str| GraphRowItem {
            author: author.into(),
            author_email: email.into(),
            ..GraphRowItem::default()
        };
        let rows = [
            row("Taro Yamada", "taro.yamada@example.com"),
            row("Taro Yamada", "Taro.Yamada@Example.com"),
            row("Ada", "Ada@Example.com"),
            row("Ada", "ada@example.com"),
            row("Nobody", ""),
        ];
        assert_eq!(
            author_choices(&rows, "Taro Yamada", "Taro.Yamada@Example.com"),
            [
                "Taro Yamada <Taro.Yamada@Example.com>",
                "Ada <Ada@Example.com>"
            ]
        );
        assert_eq!(
            author_choices(&rows, "", ""),
            [
                "Ada <Ada@Example.com>",
                "Taro Yamada <taro.yamada@example.com>"
            ]
        );
    }

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
        GraphRowItem::default()
    }

    #[test]
    fn newest_commit_row_is_the_top_of_a_plain_walk() {
        let rows = [commit("a1"), commit("b2")];
        assert_eq!(newest_commit_row(&rows), Some(0));
    }

    #[test]
    fn newest_commit_row_walks_past_the_worktree_and_the_stashes() {
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
            author_time: 1_700_000_000,
            avatar: 42,
            avatar_url: "file:///a.png".into(),
            co_authors: crate::encode::mates_of(&[platitude_core::details::CoAuthor {
                name: "Bob".into(),
                email: "bob@example.com".into(),
            }]),
            ..GraphRowItem::default()
        };
        let chosen: ChosenRows = Listed::new(vec![ChosenRow::of(&row, ("solid", "dashed"))]);
        assert_eq!(chosen[0].sha8, "01234567");
        assert_eq!(chosen[0].mates[0].name, "Bob");
        assert_eq!(
            <ChosenRows as QVariantConvertible>::try_from_qvariant(&chosen.to_qvariant()),
            Some(chosen)
        );
    }

    /// Another worktree's row comes back as it went, its tallies nested or
    /// left out; the two sides' colours alike.
    #[test]
    fn a_carried_row_and_the_side_colours_round_trip() {
        use crate::encode::Record;
        let row = CarriedRow {
            name: "topic".into(),
            path: "C:/work/topic".into(),
            home: false,
            head: "1".repeat(40),
            tally: Some(Tally {
                added: 1,
                modified: 2,
                conflicted: 3,
                ..Tally::default()
            }),
        };
        assert_eq!(CarriedRow::from_map(&row.to_map()), Some(row.clone()));
        let bare = CarriedRow {
            home: true,
            tally: None,
            ..row
        };
        assert_eq!(CarriedRow::from_map(&bare.to_map()), Some(bare));
        let sides = SideColors { ours: 2, theirs: 5 };
        assert_eq!(SideColors::from_map(&sides.to_map()), Some(sides));
    }
}
