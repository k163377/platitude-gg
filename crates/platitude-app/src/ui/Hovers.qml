pragma Singleton
import QtQuick

/// Every hover seat standing (`HoverSeat`): the one place to ask whether a hover decision is on its way. A seat
/// `waiting` (its rest running) or `leaving` (its beat) is deciding — what the hand is on will stand or go in a
/// moment — and a reading taken then is of a state about to move (`platitude.auto`'s `PageSettled`). Read by
/// calling: a seat's state is its own property, which no binding on the set hears. Nothing in the product reads it;
/// a seat is entered and left in constant time, since one stands in every sidebar row's delegate
/// (`SidebarRowGestures`) and the rows come and go with the scroll.
QtObject {
    id: hovers

    /// The seats standing. A `Set`, changed in place: no binding hangs on it.
    readonly property var seats: new Set()

    function enter(seat) {
        hovers.seats.add(seat)
    }
    function leave(seat) {
        hovers.seats.delete(seat)
    }

    /// The keys of the targets the seats are deciding on now, `seat` for one with no target.
    function deciding() {
        const out = []
        for (const seat of hovers.seats) {
            if (seat.waiting || seat.leaving)
                out.push(seat.targetKey !== "" ? seat.targetKey : "seat")
        }
        return out
    }
}
