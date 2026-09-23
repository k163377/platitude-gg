//! GraphModel: the commit graph rows.

use std::sync::Arc;
use std::time::Instant;

use platitude_core::Oid;
use platitude_core::find::{Query, Row};
use platitude_core::session::LogRow;
use qtbridge::{QListModel, QListModelBase, QModelItem, QObjectHolder, qobject};

use crate::encode::{Chips, Lanes, Optional};
use crate::hub::{Feed, GraphMsg};

use super::{impl_extend_notified, impl_notify_runs, push_run, qml_register};

mod find;
mod head;
mod item;
mod marks;
mod qobject;
mod stream;

use item::{ChosenRow, ChosenRows, GraphRowItem, Tally, to_row_item};
use marks::RowMark;

/// What a row of another working copy's uncommitted work answers when the
/// delegate asks about it: whose it is, where it is, and the six tallies
/// it says beside the words.
///
/// **Beside the items** — fifteen fields is the ceiling
/// the model macro allows and every one is spent
/// (`GraphRowItem`), and these rows are a handful where the
/// window is thousands.
#[derive(Default, Clone)]
struct CarriedRow {
    name: String,
    /// Where the copy is — what the row opens in a tab of its own, which
    /// is where its changes can be read and staged.
    path: String,
    /// The six the row draws, ready to hand over.
    tally: Optional<Tally>,
}

#[derive(Default)]
pub struct GraphModel {
    rows: Vec<GraphRowItem>,
    /// What the walk knew about each row and no delegate draws, in `rows`
    /// order — see `marks.rs` for what is in it and why it rides here.
    marks: Vec<RowMark>,
    /// The parent ids the spans in `marks` point into, flattened.
    parent_oids: Vec<Oid>,
    /// The rows another working copy's uncommitted work draws, by row
    /// index: its six tallies for the delegate, and the copy's name
    /// beside them. **A map** — fifteen is the ceiling the model macro
    /// allows and all of them are spent, and these rows are a handful
    /// where the window is thousands (`GraphModel::carried_tally`).
    carried: std::collections::HashMap<usize, CarriedRow>,
    /// Bumped every time those two are written. **A delegate's answer
    /// about them is a slot call, and a slot call is not made again
    /// because the map behind it was rewritten** — a row spliced into a
    /// rebuilt graph keeps the answer it was given for whatever stood at
    /// its index before (observed: this window's own row wearing another
    /// copy's tallies). The delegate reads this beside the call, so the
    /// binding comes back (`carriedRevision`).
    carried_revision: i32,
    /// Every drawn row by its id, in id order — how a row is found from
    /// outside (`marks::row_at`): the pin's row on every drain, a
    /// sidebar jump, a menu's question about a commit. A binary search
    /// per ask. Kept in step with `marks` at the same three
    /// places.
    index: Vec<(Oid, u32)>,
    /// Where HEAD stands, as the one record has it (`GraphMsg::Head`) —
    /// what the row the pin leads to is found by. `None` until the
    /// session has reported it. **Read off the record**: the chips
    /// arrive a pass after the rows, and are one refs read's own
    /// picture (`session::standing`). A question about a range
    /// from HEAD is handed the record's value by the asker
    /// (`rebaseRewritesPublished`), so the rows and the headline never
    /// answer for two HEADs in one frame.
    head_oid: Option<Oid>,
    /// The commit the stand-in is drawn for — HEAD where its row is
    /// drawn, else the last HEAD whose row was, until a pass lands
    /// (`head::settle_head`).
    pinned_oid: Option<Oid>,
    generation: u64,
    loading: bool,
    /// The keys of the chips to leave undrawn — see the property's own
    /// note in `qobject.rs`. Nothing here reads it: the rows are handed
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
    /// How many commits one press of the tail would add, as the session
    /// has it (`RepoSession::log_window_step`). Read from there when a
    /// walk settles: the window the step is a quarter of is the
    /// session's to say, and the footer names the step in the words it
    /// offers.
    window_step: i32,
    /// A press of the tail is out and the wider walk has not landed yet.
    /// The footer wears the wait where the words are, like every other
    /// press that goes to git (デザイン規約 §進行中・長押しの定数).
    growing: bool,
    /// Completed stream passes (direct + replacements + reloads). QML
    /// watches this edge to re-resolve the selection by oid.
    finish_count: i32,
    /// Whether the rows this pass left standing start with the synthetic
    /// working-tree row (`session::rows::wip_row`, an all-zero id).
    ///
    /// **What the rows hold, against what the status says they should**
    /// (`WorkTree::wip_row_stands`): the opening walks the log before the
    /// first status has said whether there is anything uncommitted, so a
    /// pass that lost that race carries no working-tree row and the status
    /// asks for another. The two disagreeing is how a reader tells that
    /// the graph on screen is still one read behind the repository.
    ///
    /// Settled with the footer, once the pass is whole: a pass half-way
    /// through its chunks is not a graph anybody reads this
    /// against.
    wip_row: bool,
    /// Whether the row this pass left first belongs to another working
    /// copy (`GraphModel::carried_row_of`) — the other half of the answer
    /// [`Self::wip_row`] gives, told apart here so no reader has to spell
    /// the all-zero id out for itself.
    carried_top: bool,
    /// Model resets (streaming restarts only — in-place replacements do
    /// not reset). QML re-anchors the viewport only when this moves,
    /// because only a reset zeroes the scroll position.
    reset_count: i32,
    /// What the find bar is looking for, and how many rows answer it.
    /// Held here because the rows are: every pass that rebuilds them has
    /// to re-mark them, or a background refresh would quietly put the
    /// light out while the bar still says how many are lit.
    query: Option<Query>,
    match_count: i32,
    /// Whether anything is being looked for at all — a query is set,
    /// whatever it has found. What dims the rows: with a query and no
    /// answers, every row is "not one of them".
    searching: bool,
    /// Whether the newest row is one of the answers. The graph steps down
    /// from under the card for it (規約 §コミットを探す), and since the
    /// working-tree row can never match, this is also the answer to "is
    /// the tree clean".
    first_matched: bool,
    /// Lanes running off the end of the window (`encode::tail_lanes`),
    /// drawn by the truncation footer.
    tail_geometry: Lanes,
    /// Whether the oldest loaded row is one of the answers: the footer's
    /// lanes carry on from that row's, so they start at its strength.
    tail_matched: bool,
    /// The loaded row the working tree stands on, and what a stand-in for
    /// it draws — the chips it carries and the subject it says.
    /// -1 and empty answers while HEAD is outside the window, which
    /// is a row nothing can lead to (`head::settle_head`).
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
    error: String,
    /// The walk stopped, so the rows drawn are not all of them. **A
    /// state apart from `error`**: a walk that ended without an answer at
    /// all — the task fell over — has nobody's words to show, and the
    /// screen says that in its own (`BandStateCard`, app-ui.md「Rust に
    /// 文言を置かない」).
    failed: bool,
    /// Every row is drawn and every one of them is out of date: an
    /// off-screen rebuild that would have replaced this graph did not
    /// land (`SessionEvent::LogStale`).
    ///
    /// **The other half of one badge.** `STALE GRAPH` stands for either,
    /// because either way what is on screen is not this repository's
    /// history; which of the two it was is the card's line.
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
