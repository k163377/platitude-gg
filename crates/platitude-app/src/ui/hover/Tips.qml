pragma Singleton

import QtQuick

/// Where a target's ask for the tooltip goes (`TipAsk` → the window's `TipSeat`): the seats standing, one per window,
/// and the one a target's window answers for. The harness reads the newest seat's box here, since the page it drives
/// cannot see the window's.
QtObject {
    id: tips

    /// The seats standing, replaced never changed, so a binding hears every change.
    property var seats: []
    /// The newest seat — the window's, in the app; the file's, in a test.
    readonly property var seat: tips.seats.length > 0 ? tips.seats[tips.seats.length - 1] : null
    /// Its box (the one `ToolTip`), null while no seat stands.
    readonly property var box: tips.seat !== null ? tips.seat.box : null
    /// The ask the newest seat is showing or waiting on, null for none.
    readonly property var current: tips.seat !== null ? tips.seat.current : null

    function enter(seat) {
        tips.seats = tips.seats.filter(s => s !== seat).concat([seat])
    }
    function leave(seat) {
        tips.seats = tips.seats.filter(s => s !== seat)
    }
    /// The seat of the window `item` is in, else the newest.
    function seatFor(item) {
        const window = item !== null && item !== undefined && item.Window !== undefined ? item.Window.window : null
        for (let i = tips.seats.length - 1; i >= 0; i--) {
            if (window === null || tips.seats[i].host.Window.window === window)
                return tips.seats[i]
        }
        return null
    }
    /// The hand over the surface `item` moves with — the nearest item over it that names one (`rowsHand`: a list whose
    /// rows move under the hand, the pane block another commit lands in) — null where none does (`TipAsk.hand`).
    function rowsHandOf(item) {
        for (let at = item; at !== null && at !== undefined; at = at.parent) {
            if (at.rowsHand !== undefined && at.rowsHand !== null)
                return at.rowsHand
        }
        return null
    }
    /// The box `dropIn` held off a pooled row — the seat's ask holding it — and its seat; null for none (`takeBack`,
    /// `settleParked`).
    property var parked: null
    property var parkedSeat: null
    /// A key for one ask, unique for the window's life.
    property int keys: 0
    function key() {
        tips.keys++
        return "tip:" + tips.keys
    }
    /// The tip standing or resting on a target inside `item` goes now: `item` stands for something else from here (a
    /// row that left the list or the screen, which the list pools — `ListView.onPooled`). **But the box the hand is
    /// reading, its row pushed off by a read** (`HandWatch.reading`), stays where it stands, held off the delegate
    /// (`TipSeat.detach`) until the hand leaves it: the row is still there, and the read's keeper stands it again
    /// (`takeBack`) or finds it went (`settleParked`).
    function dropIn(item) {
        const seat = tips.seatFor(item)
        if (seat === null || seat.current === null || !tips.within(seat.current.target, item))
            return
        const hand = seat.current.hand
        if (seat.pointed && hand !== null && hand.reading) {
            seat.detach()
            tips.parked = seat.current
            tips.parkedSeat = seat
        } else {
            seat.drop(seat.current)
        }
    }
    /// `ask`, one of a row a read's keeper stood again (`HandWatch.rowBack`), takes back the box held off the row's
    /// pooled delegate (`TipSeat.takeBack`).
    function takeBack(ask, row) {
        const seat = tips.parkedSeat
        if (seat !== null && tips.parked !== null && seat.current === tips.parked)
            seat.takeBack(ask, row)
    }
    /// A read's keeper's word on the box `dropIn` parked (`RowKeep`): the row it came off went with the read
    /// (`gone`), and it goes too, the hand in it or not (§「外乱では閉じない」: what is gone closes); else it stays where
    /// it stands until the hand leaves it.
    function settleParked(gone) {
        const ask = tips.parked
        const seat = tips.parkedSeat
        tips.parked = null
        tips.parkedSeat = null
        if (gone && seat !== null && ask !== null && seat.current === ask)
            seat.drop(ask)
    }
    /// Whether the tip standing was raised by a target inside `item` (the card that keeps itself up for it).
    function shownIn(item) {
        const seat = tips.seat
        if (seat === null || !seat.shownNow || seat.current === null)
            return false
        return tips.within(seat.current.target, item)
    }
    /// Whether `item` is `outer` or inside it; false for no `outer`.
    function within(item, outer) {
        if (outer === null || outer === undefined)
            return false
        for (let at = item; at !== null && at !== undefined; at = at.parent) {
            if (at === outer)
                return true
        }
        return false
    }
}
