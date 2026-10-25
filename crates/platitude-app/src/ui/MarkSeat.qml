pragma Singleton

import QtQuick
import platitude.ui

// The seat a mark set in a run of words takes (デザイン規約 §アイコンの席「字の中の席」): a whole number of the line
// face's no-break spaces — the fewest that hold the widest mark it seats, the cloud, with the face's own room round a
// `+` either side, or a stem where that room is thinner (`air`). Every mark takes the same seat whatever its kind, its
// ink in the seat's middle — in front of a name, the middle between the seat's head and the name's ink
// (`NavIcon.nameAir`) — and a name's code ground starts at the seat's head: the seat holds the mark's air. A line of
// words opens it as those spaces and nothing finer: a space the face has no glyph for sends Qt through every family on
// the machine (U+202F in Yu Gothic UI). The count is the face's and the size's, read at run time: a face that sets its
// advances to whole pixels (FreeType's, offscreen) rounds a space narrower or wider, so it can take one more space or
// one fewer at a size than one that keeps fractions.
QtObject {
    id: markSeat

    /// The face's no-break space, large: its own advance, where no hinting rounds it.
    readonly property TextMetrics space: TextMetrics {
        font.family: Theme.uiFamily
        font.pixelSize: 1000
        text: "\u00a0"
    }
    /// The widest mark a seat holds, drawn to the face large. A `lead` for its ink alone: a seated mark's cell is read
    /// off this one (`NavIcon.wordCell`).
    readonly property NavIcon widest: NavIcon {
        visible: false
        kind: "remote"
        set: "lead"
        size: 1000
    }

    /// The air a seat keeps either side of the widest mark, in shares of the pixel size: the face's own room round a
    /// `+`, but never thinner than a stem — a face that sets its `+` tight (Noto Sans CJK JP's, a twenty-seventh of an
    /// em) would otherwise stand the cloud against the ground's edge and its name.
    readonly property real air: Math.max(WordFace.signSide, Metrics.markStem)

    /// What a seat holds at `size`: the widest mark's ink as it is drawn there — to the capitals as the face draws them
    /// (`WordFace.capitalsAt`), so no share of the size — and the seat's `air` either side of it.
    function need(size) {
        const wide = markSeat.widest
        const span = (wide.inkAcross - wide.pen) / wide.unit
        return span * wide.unitAt(size) + Metrics.markStem * size + 2 * markSeat.air * size
    }
    /// How many spaces `space` wide the seat at `size` is: the fewest that hold `need`.
    function count(space, size) {
        return space > 0 ? Math.max(1, Math.ceil(markSeat.need(size) / space - 1e-6)) : 1
    }
    /// The seat as a line of words opens it in front of a name, its spaces `space` wide as the line lays them out:
    /// no-break spaces, which no line ends in.
    function room(space, size) {
        return markSeat.space.text.repeat(markSeat.count(space, size))
    }
    /// The seat at `size` where no line lays the spaces out: the face's own spaces, to whole pixels so what follows
    /// stands on one.
    function cell(size) {
        const space = markSeat.space.advanceWidth * size / 1000
        return Math.round(markSeat.count(space, size) * space)
    }
}
