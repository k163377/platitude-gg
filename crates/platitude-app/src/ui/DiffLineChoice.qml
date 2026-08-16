pragma ComponentBehavior: Bound

import QtQuick

// The lines of one diff picked by hand: a click takes one, Ctrl adds or
// removes, Shift reaches from the last one clicked — the three gestures the
// file rows already answer to (`WipPane`). The heading of a hunk with lines
// picked in it names the count instead of the hunk, and the mark on a
// picked line carries the whole choice over
// (デザイン規約 §diff の中のステージ).
//
// Held here rather than on the rows because a delegate is recycled the
// moment its line scrolls off, and keyed `<hunk>:<line>` because that pair
// is what a patch is addressed by.
QtObject {
    id: choice

    /// The rows of the diff these lines are in — only the automation hook
    /// walks them, to pick by hand what a pointer picks by being over it.
    required property var view

    property var lines: ({})
    property int count: 0
    /// The row a Shift-click reaches from — a row of the view, so that the
    /// reach crosses hunk headings the way the eye does. -1 = nothing to
    /// reach from yet.
    property int anchorRow: -1

    function chosen(hunk, line) {
        return choice.lines[hunk + ":" + line] === true
    }
    /// How many lines of one hunk are picked — what its heading says.
    function countIn(hunk) {
        const head = hunk + ":"
        let n = 0
        for (const key in choice.lines)
            if (key.indexOf(head) === 0)
                n++
        return n
    }
    function toggle(hunk, line) {
        // A fresh object every time: the rows follow this property, and
        // assigning the same one back changes nothing to follow.
        const key = hunk + ":" + line
        const next = ({})
        for (const k in choice.lines)
            next[k] = true
        if (next[key] === true)
            delete next[key]
        else
            next[key] = true
        choice.take(next)
        choice.anchorRow = choice.rowOf(hunk, line)
    }
    /// Applies a press on one line, with whatever was held down with it.
    function apply(hunk, line, modifiers) {
        if ((modifiers & Qt.ShiftModifier) && choice.anchorRow >= 0) {
            choice.reach(choice.anchorRow, choice.rowOf(hunk, line))
            return
        }
        if (modifiers & Qt.ControlModifier) {
            choice.toggle(hunk, line)
            return
        }
        const only = ({})
        only[hunk + ":" + line] = true
        choice.take(only)
        choice.anchorRow = choice.rowOf(hunk, line)
    }
    /// Everything changed between two rows of the view, ends included.
    /// The reach runs over rows rather than over line indices: a hunk
    /// numbers its own lines, so two hunks share the numbers in between.
    function reach(from, to) {
        if (from < 0 || to < 0)
            return
        const lo = Math.min(from, to)
        const hi = Math.max(from, to)
        const next = ({})
        for (let i = lo; i <= hi; i++) {
            const row = choice.view.itemAtIndex(i)
            if (row && (row.kind === "add" || row.kind === "del"))
                next[row.hunk + ":" + row.line] = true
        }
        choice.take(next)
    }
    /// Which row of the view a line sits on, or -1 while the list has not
    /// built it (a reach that starts off screen has nothing to count).
    function rowOf(hunk, line) {
        for (let i = 0; i < choice.view.count; i++) {
            const row = choice.view.itemAtIndex(i)
            if (row && row.hunk === hunk && row.line === line)
                return i
        }
        return -1
    }
    function take(next) {
        choice.lines = next
        choice.count = Object.keys(next).length
    }
    function clear() {
        choice.lines = ({})
        choice.count = 0
        choice.anchorRow = -1
    }
    /// The picked lines as `[hunk, line]` pairs, in the order a patch
    /// wants them — for the owner to hand to the bridge.
    function pairs() {
        const out = []
        for (const key in choice.lines) {
            const cut = key.indexOf(":")
            out.push([parseInt(key.substring(0, cut)),
                      parseInt(key.substring(cut + 1))])
        }
        out.sort((a, b) => a[0] === b[0] ? a[1] - b[1] : a[0] - b[0])
        return out
    }
    /// Automation: pick the first `count` changed lines of a hunk, the way
    /// a click on each of them would.
    function choose(hunk, count) {
        choice.clear()
        let taken = 0
        for (let i = 0; i < choice.view.count && taken < count; i++) {
            const row = choice.view.itemAtIndex(i)
            if (row && row.hunk === hunk
                    && (row.kind === "add" || row.kind === "del")) {
                choice.toggle(hunk, row.line)
                taken++
            }
        }
        return taken
    }
}
