pragma ComponentBehavior: Bound

import QtQuick

// What a hand laid over a list's rows to pick their text holds whichever list it stands over (`CommandsTextSelect`,
// `DiffTextSelect`): which row a point is over, the ground under a short list's last row, and whether a drag has taken
// text yet. Where in a row a point lands and what the selection holds stay each hand's — its model says.
//
// Every place in the frame is a start, the ground under the last row included (`rowAt`;
// 規約 §右のペインの字は掴める).
Item {
    id: frame

    /// The rows, for the frame the hand stands over and for asking which row a point is on.
    required property var view

    /// Whether a drag is running: from the press that started one (`pressedAt`) until the hand lets go.
    property bool dragging: false
    /// Where the hand was last seen, in this item's coordinates (the hand's edge ticker re-reads it).
    property point handPosition: Qt.point(0, 0)
    /// Whether the drag has taken text yet — left the place it was pressed at, which the two below hold. Only then
    /// does the hand's edge send: a browser starts its selection's autoscroll the same way, so a click on the last row
    /// in sight, most of which is the edge's belt, leaves the rows where they are.
    property bool selecting: false
    property int fromRow: -1
    property int fromAt: 0
    /// Where the ground rows shorter than the frame leave under the last one begins, in this item's coordinates, and
    /// whether there is any — a sweep against rows that fill the frame proves nothing, so it says so (verify-ui
    /// `commands-sweep`, `diff-sweep`).
    readonly property real groundTop: Math.max(0, frame.view.originY + frame.view.contentHeight
                                                  - frame.view.contentY)
    readonly property bool hasGround: frame.groundTop < frame.height - 2

    // ---- the drag --------------------------------------------------------------------------------------------------
    /// A press has started a drag at that place of that row; it holds nothing until the hand moves.
    function pressedAt(row, at) {
        frame.selecting = false
        frame.fromRow = row
        frame.fromAt = at
        frame.dragging = true
    }
    /// The drag has reached that place: text is taken once it is not the one pressed.
    function draggedTo(row, at) {
        if (row !== frame.fromRow || at !== frame.fromAt)
            frame.selecting = true
    }

    // ---- pixels to rows ----------------------------------------------------------------------------------------
    /// Which row a point of this item is over, or -1 where the list has no rows. **Clamped twice**: to the frame, so a
    /// drag past either edge keeps naming the row it can still see while the edge scrolls, and onto the rows
    /// (`onRows`), so the ground under the last row belongs to that row.
    function rowAt(y) {
        const inside = Math.max(0, Math.min(y, frame.height - 1))
        // One pixel in from the left: rows are as wide as the list, so any x inside it finds the same row.
        return frame.view.indexAt(1, frame.onRows(frame.view.contentY + inside))
    }
    /// A content y brought onto the band the rows stand on, from `originY`, which a list of rows of differing heights
    /// moves once it is sent to its end (`AppListView.clampY`). An empty list has no band: `indexAt` finds nothing and
    /// the press goes down to the list.
    function onRows(y) {
        const first = frame.view.originY
        const last = first + frame.view.contentHeight - 1
        return Math.max(first, Math.min(y, last))
    }

    // ---- the ground under the last row (`groundTop`) -------------------------------------------------------------
    /// Automation: where a sweep starts — a point on that ground, across `[0, to)` and down it by the fractions — or
    /// null with no ground. It **starts in the ground**: begun on a row, it would go green with the ground rule taken
    /// back out (`SweepRoom`).
    function groundPoint(fx, fy, to) {
        if (!frame.hasGround)
            return null
        const x = Math.max(1, Math.min(to - 1, to * fx))
        return Qt.point(x, frame.groundTop + 1 + (frame.height - frame.groundTop - 2) * fy)
    }
    /// Automation: where the sweep goes, the first row's middle at the x it started from — straight up, since a
    /// sideways jump would let a run pass while the real gesture fails.
    function firstRowPoint(x) {
        const top = frame.view.itemAtIndex(0)
        return Qt.point(x, top ? top.y - frame.view.contentY + top.height / 2 : 1)
    }
}
