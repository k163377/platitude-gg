pragma ComponentBehavior: Bound

import QtQuick

// How far the diff reaches sideways, in the only unit that answers: pixels a row was actually laid out in.
//
// **A row's own layout is the truth here.** A wide glyph is two columns
// and whatever the fallback carrying it advances (measured on Windows at `fontCode`, against 8px for the font's own
// columns: `日` 13, `の` 11, `。` 9, `「` 7, an emoji 18), and a combining mark is a character the walk counts and a
// glyph the font draws nothing for. So the model's pick of the longest lines (`encode::widest_lines`, measured by
// `DiffTextMetrics`) is a head start — it says something before a row exists — and the rows correct it: **a row that
// has been laid out is always reachable to its end, whether or not the pick named it**.
//
// Three rules hold this together, and each of them is a bug that was shipped:
//
//  - **a width belongs to a row and a generation.** Holding the largest number ever seen
//    latched a width measured mid-update and never gave it back. Here every width is filed under its row, so a second
//    look at the same row replaces the first — down as well as up — and a generation the rows have left is dropped
//    whole.
//  - **what is filed is what a row was drawn at.** The rows are one format now (`markup::styled`), so a row is never
//    measured with its own `<font …>` tags counted as text, and a row is never asked before its text is set.
//  - **an unmeasured width keeps the last one.** Between one reading of a file and the next there is a
//    moment with no answer in hand; publishing it as zero would clamp the reader's place back to the left edge, so
//    what was standing goes on standing until something has been measured.
QtObject {
    id: reach

    /// Which reading of the rows the widths belong to (`DiffModel.rowsGen`). A new reading files nothing under the
    /// old one: the rows may be a different file, or the same file with lines gone.
    required property int rowsGen
    /// What the lines the model picked came to, measured (`DiffTextMetrics.codeW`), and 0 while none of them has been
    /// laid out — which means "nothing measured yet".
    required property real measured
    /// The file on screen. The same one read again keeps the width it was reading at; a different one starts over,
    /// the way the place along it does (`DiffCodeScroll.file`).
    required property string file

    /// How far the widest row of this diff is drawn, as far as anything has been measured. The ink alone: whoever
    /// draws the room past the last character adds it (`DiffPane`).
    property real width: 0

    /// The widths filed by row, and the row holding the widest of them — kept so that the common answer costs one
    /// comparison and only a row that *shrank* pays for a walk.
    property var seen: ({})
    property real rowsWidth: 0
    property int widestRow: -1

    /// A row says what it was laid out at. Both ways: a row that came back narrower than it was takes the reach with
    /// it, or a fallback that resolved late would leave the bar reaching a width no row has any more.
    function noteRow(row, drawn) {
        if (reach.seen[row] === drawn)
            return
        reach.seen[row] = drawn
        if (drawn >= reach.rowsWidth) {
            reach.rowsWidth = drawn
            reach.widestRow = row
        } else if (row === reach.widestRow) {
            reach.settleRows()
        }
        reach.settle()
    }

    /// The widest of what is filed, walked. Only when the row that was holding the answer gives it up.
    function settleRows() {
        let widest = 0
        let holder = -1
        for (const row in reach.seen) {
            if (reach.seen[row] > widest) {
                widest = reach.seen[row]
                holder = Number(row)
            }
        }
        reach.rowsWidth = widest
        reach.widestRow = holder
    }

    /// The rows on screen are of a different reading now: what was filed was measured on lines that may be gone. The
    /// width itself stays until this reading has measured something (see the note above).
    function forgetRows() {
        reach.seen = ({})
        reach.rowsWidth = 0
        reach.widestRow = -1
    }

    /// Everything, including the width — the pane is on another file, and the place the reader was at goes with it.
    function forgetFile() {
        reach.forgetRows()
        reach.width = 0
    }

    function settle() {
        const fresh = Math.max(reach.measured, reach.rowsWidth)
        if (fresh > 0)
            reach.width = fresh
    }

    onRowsGenChanged: reach.forgetRows()
    onMeasuredChanged: reach.settle()
    onFileChanged: reach.forgetFile()
}
