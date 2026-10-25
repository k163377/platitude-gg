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
    /// A key for one ask, unique for the window's life.
    property int keys: 0
    function key() {
        tips.keys++
        return "tip:" + tips.keys
    }
    /// Whether the tip standing was raised by a target inside `item` (the card that keeps itself up for it).
    function shownIn(item) {
        const seat = tips.seat
        if (seat === null || !seat.shownNow || seat.current === null || item === null || item === undefined)
            return false
        for (let at = seat.current.target; at !== null && at !== undefined; at = at.parent) {
            if (at === item)
                return true
        }
        return false
    }
}
