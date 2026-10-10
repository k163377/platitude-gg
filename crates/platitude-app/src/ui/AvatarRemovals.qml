import QtQuick
import platitude.ui

// Avatar removals pressed through and waiting out their floor (デザイン規約 §回数押し): `{ email, url }` — the person,
// and the picture the press saw. Kept apart from the rows: an assignment landing meanwhile rebuilds every row of the
// settings list, and a removal must not go with its row. Each is given once — by its row's floor (`give`), or, the row
// rebuilt away, by this list's own (`floor`) — and a person given another picture or none meanwhile is struck off for
// good.
QtObject {
    id: removals

    /// The assignments as filed now (`AppBackend.avatars` — `{ email, url, … }`).
    property var filed: []
    /// Taken at a last press and not given yet.
    property var waiting: []

    /// Given: this person's picture comes off.
    signal remove(string email)

    function accept(email, url) {
        removals.waiting = removals.waiting.filter(entry => entry.email !== email)
            .concat([{ email: email, url: url }])
        removals.floor.restart()
    }
    function give(email) {
        const entry = removals.waiting.find(taken => taken.email === email)
        if (entry === undefined)
            return
        removals.waiting = removals.waiting.filter(taken => taken !== entry)
        removals.remove(email)
    }

    // A picture changed or gone under a removal strikes it off, and nothing puts it back.
    onFiledChanged: removals.waiting = removals.waiting.filter(entry => removals.filed.some(
        assignment => assignment.email === entry.email && assignment.url === entry.url))

    /// The floor once more, for a removal whose row was rebuilt away before its own floor passed.
    readonly property Timer floor: Timer {
        interval: Metrics.holdFloorMs
        onTriggered: {
            for (const entry of removals.waiting.slice())
                removals.give(entry.email)
        }
    }
}
