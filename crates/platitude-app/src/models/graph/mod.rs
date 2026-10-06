//! GraphModel: the commit graph rows.

use std::sync::Arc;
use std::time::Instant;

use platitude_core::Oid;
use platitude_core::find::{Query, Row};
use platitude_core::session::{LogRow, RelaidRow};
use qtbridge::{QListModel, QListModelBase, QModelItem, QObjectHolder, QmlObject, qobject};

use crate::encode::{Chips, Lanes, Optional};
use crate::hub::{Feed, GraphMsg};

use super::{impl_extend_notified, impl_notify_runs, push_run, qml_register};

mod find;
mod head;
mod item;
mod marks;
mod provisional;
mod qobject;
mod relay;
mod stream;

use item::{ChosenRow, ChosenRows, GraphRowItem, Tally, to_row_item};
use marks::RowMark;

/// What a row of another worktree's uncommitted work answers when the
/// delegate asks: whose it is, where it is, and its six tallies. Beside
/// the items, since `GraphRowItem` has spent all fifteen fields.
#[derive(Default, Clone)]
struct CarriedRow {
    name: String,
    path: String,
    tally: Optional<Tally>,
}

#[derive(Default)]
pub struct GraphModel {
    rows: Vec<GraphRowItem>,
    /// Per-row walk marks no delegate draws, in `rows` order (`marks.rs`).
    marks: Vec<RowMark>,
    /// The parent ids the spans in `marks` point into, flattened.
    parent_oids: Vec<Oid>,
    /// Rows of other worktrees' uncommitted work, by row index — a
    /// handful, where the window is thousands.
    carried: std::collections::HashMap<usize, CarriedRow>,
    /// Bumped whenever `carried` is written. The delegate reads it beside
    /// its slot calls, which a rewritten map does not re-run — else a row
    /// spliced into a rebuilt graph keeps the answer for whatever stood at
    /// its index before.
    carried_revision: i32,
    /// Every drawn row by its id, in id order, for a binary search per ask
    /// (`marks::row_at`). Kept in step with `marks` at the same places.
    index: Vec<(Oid, u32)>,
    /// Rows a relaying took off the graph (`relay.rs`), by id: a refused
    /// delete lays them out again by id, and nothing else holds them. A
    /// handful: taken back by a walk that draws them again, let go of at a
    /// reset.
    set_aside: std::collections::HashMap<Oid, relay::SetAside>,
    /// Where HEAD stands, as the one record has it (`GraphMsg::Head`);
    /// `None` until reported. Read off the record, not the chips, which
    /// land a pass after the rows. A range question from HEAD takes the
    /// record's value from the asker (`rebaseRewritesPublished`), so the
    /// rows and the headline never answer for two HEADs in one frame.
    head_oid: Option<Oid>,
    /// The commit the stand-in is drawn for — HEAD where its row is
    /// drawn, else the last HEAD whose row was, until a pass lands
    /// (`head::settle_head`).
    pinned_oid: Option<Oid>,
    generation: u64,
    loading: bool,
    /// The keys of the chips to leave undrawn (`goneChips`). The rows go
    /// out by reference (`QListModel::get`), so the leaving-out happens
    /// where the chips are drawn.
    gone_chips: Vec<String>,
    row_total: i32,
    /// Commits the walk emitted — the truncation footer's number. Equals
    /// the window limit whenever `truncated`, where `row_total` drifts
    /// off it (the WIP row adds one, sifted stash parents subtract).
    walked_total: i32,
    max_lanes: i32,
    first_chunk_ms: i32,
    total_ms: i32,
    truncated: bool,
    /// How many commits one press of the tail would add
    /// (`RepoSession::log_window_step`), read when a walk settles — the
    /// window it is a step of is the session's.
    window_step: i32,
    /// A press of the tail is out and the wider walk has not landed; the
    /// footer wears the wait (デザイン規約 §進行中・長押しの定数).
    growing: bool,
    /// Completed stream passes (direct + replacements + reloads). QML
    /// watches this edge to re-resolve the selection by oid.
    finish_count: i32,
    /// Whether the rows this pass left start with this window's
    /// working-tree row (`session::rows::wip_row`). Read against what the
    /// status says (`WorkingTreeModel.wipRowStands`): the opening walks before
    /// the first status, so the two disagreeing means the graph is one read
    /// behind. Settled with the footer, once the pass is whole.
    wip_row: bool,
    /// Whether the row this pass left first is another worktree's —
    /// the other half of [`Self::wip_row`]'s answer.
    carried_top: bool,
    /// Model resets (streaming restarts only — in-place replacements do
    /// not reset). QML re-anchors the viewport only when this moves,
    /// because only a reset zeroes the scroll position.
    reset_count: i32,
    /// What the find bar is looking for, and how many rows answer it —
    /// held beside the rows so every pass that rebuilds them re-marks them.
    query: Option<Query>,
    match_count: i32,
    /// Whether a query is set, whatever it found — what dims the rows.
    searching: bool,
    /// Whether the query is hex too short to be read as an object name
    /// (`find::Query::short_of_an_oid`) — what the find bar's hint answers.
    short_of_an_oid: bool,
    /// Whether the newest row is one of the answers; the graph steps down
    /// from under the card for it (規約 §コミットを探す). The working-tree
    /// row never matches, so true also means a clean tree.
    first_matched: bool,
    /// Lanes running off the end of the window (`encode::tail_lanes`),
    /// drawn by the truncation footer.
    tail_geometry: Lanes,
    /// Whether the oldest loaded row is one of the answers: the footer's
    /// lanes carry on from that row's, so they start at its strength.
    tail_matched: bool,
    /// The loaded row the working tree stands on, and what its stand-in
    /// draws; -1 and empty outside the window (`head::settle_head`).
    head_row: i32,
    head_labels: Chips,
    head_subject: String,
    head_color: i32,
    head_lane: i32,
    head_avatar: i32,
    head_avatar_url: String,
    head_geometry: Lanes,
    /// Whether the search found that row — the stand-in dims with it, or
    /// the row would change its light as it scrolled off.
    head_matched: bool,
    /// The discard log's picked entry on the graph (`provisional.rs`): the
    /// rows it would bring back are its parts' tips and the commits only
    /// those tips reach, by id. Empty while nothing is picked.
    provisional: std::collections::HashSet<String>,
    /// The first part's tip: the row the entry lands on.
    provisional_tip: String,
    /// How each tip draws, by id: `uncommitted` for a copy of thrown-away
    /// work, `stash` for a dropped stash; a commit is not in it.
    provisional_looks: std::collections::HashMap<String, String>,
    provisional_on: bool,
    /// Where those rows stand: the first and the last, and the tip's own;
    /// -1 for none loaded.
    provisional_first: i32,
    provisional_last: i32,
    provisional_tip_row: i32,
    /// Bumped whenever the answers above may have moved under a row, for
    /// the reason `carried_revision` is.
    provisional_revision: i32,
    /// Whether the walk that took the entry's tips has landed
    /// (`GraphMsg::DiscardWalked`): a tip still undrawn then lies past the
    /// window, not past the walk.
    provisional_walked: bool,
    error: String,
    /// The walk stopped, so the rows drawn are not all of them. Apart from
    /// `error`: a walk that fell over has no git words, and the screen says
    /// it in its own (`BandStateCard`, rules-refs/app-ui.md「Rust に
    /// 文言を置かない」).
    failed: bool,
    /// Every row is drawn and out of date: the rebuild that would have
    /// replaced this graph did not land (`SessionEvent::LogStale`).
    /// `STALE GRAPH` stands for this and `failed` alike; the card's line
    /// says which.
    stale: bool,
    started_at: Option<Instant>,
    feed: Option<Arc<Feed<GraphMsg>>>,
    tab_id: i32,
}

impl QListModel for GraphModel {
    type Item = GraphRowItem;

    fn len(&self) -> usize {
        self.rows.len()
    }
    fn get(&self, index: usize) -> Option<&GraphRowItem> {
        self.rows.get(index)
    }
    fn set_unnotified(&mut self, index: usize, value: GraphRowItem) -> bool {
        match self.rows.get_mut(index) {
            Some(slot) => {
                *slot = value;
                true
            }
            None => false,
        }
    }
    fn reset_unnotified(&mut self) {
        self.rows.clear();
        self.clear_marks();
    }
}

impl_extend_notified!(GraphModel, rows, GraphRowItem);
impl_notify_runs!(GraphModel);

qml_register!(GraphModel, "GraphModel", singleton = false);
