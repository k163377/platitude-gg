pragma ComponentBehavior: Bound

import QtQuick

// How far the diff reaches sideways, in pixels rows were actually laid out in. Wide glyphs advance by whatever the
// fallback font gives and combining marks draw nothing, so the model's pick of the widest lines
// (`encode::widest_lines`, measured by `DiffTextMetrics`) is only a head start: a row that has been laid out is always
// reachable to its end, whether or not the pick named it. **No formula from columns to pixels exists**: the fallback
// Windows' `Cascadia Mono` hands full-width glyphs to is not even monospaced.
//
// Three rules (デザイン規約 §diff を横へ送る):
//  - a width belongs to a row and a generation (`DiffModel.rowsGen`, which rises only when the rows are laid out
//    again — colours landing keep it): a second look replaces the first, down as well as up, and a generation the
//    rows have left is dropped whole. Not a running maximum: it would keep a width caught mid-update for good;
//  - what is filed is what a row was drawn at (one markup format, `markup::styled`, coloured or not — the pick is
//    measured in the same string, format, font and weight; two formats would file, on the turn the markup lands
//    before its format, the width of the tags set as text);
//  - an unmeasured width keeps the last one — publishing 0 would clamp the reader's place back to the left edge. The
//    same file read again keeps it (`settle`); only another file drops it (`file`).
// Only lines of the file count: hunk headings and the no-newline note are left out.
//
// Pinned by `tests/qml/tst_diffreach.qml`, its expectations built another way than the product's. Headless:
// `code-send`, and per rule on `--preset widelate`: the first `code-shrink` / `code-swap`, the second `colour-place`'s
// `held=`, the third `code-grow` — whose `ends=` is the one claim looking from outside these widths' own books.
QtObject {
    id: reach

    /// Which reading of the rows the widths belong to (`DiffModel.rowsGen`); a new one drops what was filed.
    required property int rowsGen
    /// The model's picked lines, measured (`DiffTextMetrics.codeW`); 0 means nothing measured yet.
    required property real measured
    /// The file on screen: the same one read again keeps its width, a different one starts over
    /// (`DiffCodeScroll.file`).
    required property string file

    /// How far the widest row is drawn, ink only — the room past the last character is `DiffPane`'s to add.
    property real width: 0

    /// Widths by row, and the row holding the widest — so only that row shrinking pays for a walk.
    property var seen: ({})
    property real rowsWidth: 0
    property int widestRow: -1

    /// A row reports what it was laid out at. Narrower counts too, or a late-resolved fallback leaves a reach no row
    /// has any more.
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

    /// Walks for the widest — only when the row holding it gives it up.
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

    /// A new reading: drop what was filed; the width stands until something is measured.
    function forgetRows() {
        reach.seen = ({})
        reach.rowsWidth = 0
        reach.widestRow = -1
    }

    /// Another file: everything goes, the width included.
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
