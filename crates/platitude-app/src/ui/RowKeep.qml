import QtQuick
import platitude.ui

/// Keeps the row under a pane's hand where it was through a read (デザイン規約 §hover のツールチップ「外乱では閉じない」):
/// rows a read wrote in or took out above it, and the list stepping down or up the pane with them, move it, and the
/// view is put back by as much — as far as it can scroll (the graph's own is `GraphRowWalk.shiftRows`). What it cannot
/// take back moves under the hand as the read's (`HandWatch.rowsMoved(true)`): what the hand is inside stays, and
/// nothing opens until the hand moves. Taken back whole, nothing moved under the hand (`HandWatch.rowsKept`).
///
/// **Weighed by the delegate**, not by what it shows: the lists are told what a read moved row by row
/// (`NavSectionModel.reshape`), so the row the hand was over is the same delegate after the read, at its new place —
/// unless the read pushed it past the view's cache, which pools it as it lays the read out: then it is built again by
/// its place in the model, which its delegate's `index` follows until the view pools it (-1 for a row the read took
/// out, which went), and what the hand was reading off it comes back onto it (`HandWatch.rowBack`). Rows above the
/// first one on screen come and go without moving the view (Qt's own `repositionFirstItem`); rows between it and the
/// hand's move the hand's.
///
/// Declared as a readonly property's value, never as a bare child (`TipAsk`).
QtObject {
    id: keep

    /// The hand over the pane the rows are in.
    property HandWatch hand: null
    /// The lists whose rows that hand is over (`AppListView`, with its `clampY`).
    property var views: []
    /// What places those lists in the pane by their counts, laid out before they are weighed: a `Layout` whose items'
    /// sizes follow their counts places them at its next polish, which the read's frame would draw. Null where nothing
    /// does — bindings, or a layout only resized, which places its items at once (`QQuickLayout::geometryChange`).
    property Item layout: null
    /// The model is telling the read's rows (`rowsArriving` … `rowsArrived`): a row asking the model something now
    /// waits a turn — a call into a model while it notifies takes its borrow twice (`WipBucketPane`).
    property bool telling: false
    /// A read is moving the rows (`NavSectionModel.rowsArriving` … `rowsArrived`): what moves now is the read's, and
    /// the views and the panes tell no move of their own meanwhile — the read is weighed whole at its end (`arrived`).
    property bool reading: false
    /// The view is being laid out for the read or put back after it: a move the views see now is this one's.
    property bool keeping: false
    /// Either: the moves the views see are the read's, and this tells the hand (`HandWatch.reading`).
    readonly property bool busy: keep.reading || keep.keeping

    /// The row the hand was last over, and its top in the window then; null for none — and for one whose delegate Qt
    /// deleted with the row a read took out, or pooled as a read moved the rows (`landing`), which `noted` still
    /// remembers.
    property Item row: null
    property real rowAt: 0
    property bool noted: false
    /// That row's place in its model as the read left it, and its list — read off its delegate as the read lands
    /// (`arrived`) and as the view pools it (`poolWatch`): by the turn the keeper weighs in, the view may have laid the
    /// read out, pooling the delegate of a row it pushed past the cache and using it again for another. -1 for none, or
    /// for a row the read took out.
    property int landing: -1
    property Item list: null

    /// Notes the row under the hand — on a row or off them; for a hand the rows do not hear now (inside something hung
    /// off them, under a menu), the row it was last over, where it stands now.
    function note() {
        const hand = keep.hand
        if (hand === null || keep.reading)
            return
        if (hand.known)
            keep.row = keep.rowUnder(hand.at)
        else if (keep.row !== null && !keep.standing(keep.row))
            keep.row = null
        keep.noted = keep.row !== null
        keep.rowAt = keep.noted ? keep.row.mapToItem(null, 0, 0).y : 0
    }
    /// The delegate of these lists under a window place, or null: off the lists, between their rows, under what stands
    /// over them and is no row (`NavList.shieldAt` — the current branch's stand-in).
    function rowUnder(at) {
        for (let i = 0; i < keep.views.length; i++) {
            const view = keep.views[i]
            if (view === null || view === undefined || !view.visible)
                continue
            const p = view.mapFromItem(null, at.x, at.y)
            if (p.x < 0 || p.y < 0 || p.x >= view.width || p.y >= view.height)
                continue
            if (view.shieldAt !== undefined && view.shieldAt(at))
                return null
            const row = view.itemAt(p.x + view.contentX, p.y + view.contentY)
            return row !== null && keep.standing(row) ? row : null
        }
        return null
    }
    /// Whether a delegate still stands for a row of its list — not pooled (`ListView.onPooled`), not destroyed.
    function standing(row) {
        return row !== null && row !== undefined && row.visible === true && row.pooled !== true
    }

    /// A read is about to move the rows: the row under the hand is noted where it stands now, nothing having moved yet
    /// — a place noted at the hand's last move may be stale by a move nobody told the hand of (an open row brought
    /// into view, `NavList.revealOpenRow`). A read arriving before the last one is weighed (another section's) adds to
    /// it: the row stands as the first one found it.
    function arriving() {
        if (!keep.reading) {
            keep.note()
            keep.landing = -1
            keep.list = null
        }
        keep.reading = true
        keep.telling = true
    }
    /// …and its rows are in: where the row under the hand went is weighed, and the view put back by as much. **A turn
    /// later**, once: the model is still telling its news, and the rows reading the hand again ask it things
    /// (`NavFacts.answers`) — a call into a model while it notifies takes its borrow twice. The view may lay the read
    /// out before that turn (its next polish), so the row's place in the model is noted now (`land`).
    function arrived() {
        keep.telling = false
        keep.land()
        Qt.callLater(keep.settle)
    }
    /// Notes where the row stands in its model now (`landing`), while its delegate still stands for it.
    function land() {
        if (!keep.noted || !keep.standing(keep.row))
            return
        keep.landing = keep.row.index !== undefined ? keep.row.index : -1
        keep.list = keep.row.ListView.view
    }
    function settle() {
        keep.reading = false
        keep.weigh()
    }
    function weigh() {
        const hand = keep.hand
        if (hand === null || !hand.present)
            return
        const at = keep.landing
        const view = keep.list
        keep.landing = -1
        keep.list = null
        keep.layOut()
        if (!keep.noted) {
            Tips.settleParked(false)
            // The hand was on no row: one there now came with the read.
            if (hand.known && keep.rowUnder(hand.at) !== null)
                hand.rowsMoved(true)
            return
        }
        const row = keep.row
        if (keep.standing(row)) {
            Tips.settleParked(false)
            keep.putBack(row.ListView.view, row, false)
            return
        }
        const back = at >= 0 && view !== null ? keep.bringBack(view, at) : null
        // What the hand was reading off the row's pooled delegate comes back onto it (`Tips.dropIn`).
        if (back !== null)
            hand.rowBack(back)
        Tips.settleParked(back === null)
        if (back !== null) {
            keep.row = back
            keep.putBack(view, back, true)
            return
        }
        // The row went with the read: what is under the hand now came there.
        keep.row = null
        keep.noted = false
        hand.rowsMoved(true)
    }
    /// Lays the read out: the lists, which count their rows; what places them by those counts (`layout`); and the
    /// lists again, in the places it gave them.
    function layOut() {
        keep.keeping = true
        keep.forceViews()
        if (keep.layout !== null) {
            keep.layout.ensurePolished()
            keep.forceViews()
        }
        keep.keeping = false
    }
    function forceViews() {
        for (let i = 0; i < keep.views.length; i++) {
            if (keep.views[i] !== null && keep.views[i] !== undefined)
                keep.views[i].forceLayout()
        }
    }
    /// The row at `at` of `view`, which the read pushed past the view's cache, built again and stood where the hand
    /// had it — as far as the view scrolls; null where the view builds none.
    function bringBack(view, at) {
        keep.keeping = true
        view.positionViewAtIndex(at, ListView.Beginning)
        view.forceLayout()
        const row = view.itemAtIndex(at)
        if (row !== null) {
            view.contentY = view.clampY(view.contentY + row.mapToItem(null, 0, 0).y - keep.rowAt)
            view.forceLayout()
        }
        keep.keeping = false
        return row
    }
    /// Puts `view` back by as much as `row` moved from where the hand had it, as far as it scrolls, and tells the hand:
    /// what it could not take back moved under it, as the read's; taken back whole — or `rebuilt`, another delegate
    /// standing for the row now — the rows under it read it again.
    function putBack(view, row, rebuilt) {
        const hand = keep.hand
        const moved = row.mapToItem(null, 0, 0).y - keep.rowAt
        if (Math.abs(moved) >= 1) {
            const from = view.contentY
            keep.keeping = true
            view.contentY = view.clampY(from + moved)
            keep.keeping = false
            if (Math.abs(view.contentY - from - moved) >= 1) {
                hand.rowsMoved(true)
                return
            }
        } else if (!rebuilt) {
            return
        }
        hand.rowsKept()
    }

    onBusyChanged: if (keep.hand !== null) keep.hand.reading = keep.busy

    // Every place the hand is heard at, and every move of the rows under it.
    property Connections handWatch: Connections {
        target: keep.hand
        function onMoved() { keep.note() }
        function onShiftsChanged() { keep.note() }
    }
    // The row's delegate going to the pool as a read moves the rows: where the row stood then is where the read left
    // it, and the delegate stands for nothing of the hand's from here — the view may use it again for another row
    // before the keeper's turn, and another read landing then would read that row's place off it (`land`). A target
    // changed from inside its own handler is let go a turn later (`QQmlConnections::setTarget`): the delegate may tell
    // again meanwhile.
    property Connections poolWatch: Connections {
        target: keep.reading ? keep.row : null
        ignoreUnknownSignals: true
        function onPooledChanged() {
            if (keep.row === null || keep.row.pooled !== true)
                return
            keep.landing = keep.row.index !== undefined ? keep.row.index : -1
            keep.list = keep.row.ListView.view
            keep.row = null
        }
    }
}
