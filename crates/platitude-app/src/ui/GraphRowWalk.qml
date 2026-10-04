pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The graph's selection and where the view stands to show it: the arrow keys (規約 §矢印で履歴を辿る), the page's
// jumps, and keeping the reader's place across a background rebuild. Draws nothing.
Item {
    id: walk

    required property var view
    required property var graphModel
    /// A question is standing over a row.
    required property bool asking
    /// A step came to rest on a row: read it. The row travels with the id, since every working copy's uncommitted row
    /// carries the all-zero one (`RepoPage.openWipFor`).
    signal activated(string oidHex, int atRow)

    /// Moves the selection one row (`delta` = ∓1) and brings it into view; answers whether it moved. Keys and runs
    /// both come here. `held` = the key was already down (`KeyEvent.isAutoRepeat`), which only changes when the row
    /// is read (`noteStep`).
    ///
    /// Refused while the pane is off screen, a question stands, or a name box is open — a single-line box does not
    /// consume Up / Down, so they reach the list through the delegate.
    function stepRow(delta, held) {
        if (!walk.visible || walk.asking
                || walk.view.namingOid !== "" || walk.view.count === 0)
            return false
        const from = walk.view.currentIndex
        // While the discard log's entry is on the graph the steps keep to its span, as the view does
        // (`GraphList.clampY`): a row past it could be selected and never shown.
        const spanned = walk.graphModel.provisionalOn && walk.graphModel.provisionalFirst >= 0
        const lowest = spanned ? walk.graphModel.provisionalFirst : 0
        const highest = spanned ? walk.graphModel.provisionalLast : walk.view.count - 1
        const row = from < 0 ? lowest : Math.max(lowest, Math.min(from + delta, highest))
        if (row === from)
            return false
        // Asked before the move: a selection that was out of sight is nobody's place, so that one is centered instead.
        const near = walk.view.rowOnScreen(from)
        walk.view.currentIndex = row
        walk.revealStep(row, near)
        walk.noteStep(held)
        return true
    }
    /// Brings a stepped-onto row into view: `near` moves as little as will do, anything else centers. Row positions
    /// are computed, since `itemAtIndex` answers null for rows the view has not built.
    function revealStep(row, near) {
        walk.view.haltGlide()
        if (!near) {
            walk.view.positionViewAtIndex(row, ListView.Center)
            return
        }
        // The first row's highlight covers the top margin, so stepping onto it goes all the way to the top.
        const top = walk.view.originY + row * Theme.graphRowHeight
                    - (row === 0 ? walk.view.topMargin : 0)
        const bottom = walk.view.originY + (row + 1) * Theme.graphRowHeight
        if (top < walk.view.contentY)
            walk.view.contentY = walk.view.clampY(top)
        else if (bottom > walk.view.contentY + walk.view.height)
            walk.view.contentY = walk.view.clampY(bottom - walk.view.height)
    }
    /// Whether any of `row` is on screen.
    function rowShows(row) {
        const top = walk.view.originY + row * Theme.graphRowHeight
        return top + Theme.graphRowHeight > walk.view.contentY && top < walk.view.contentY + walk.view.height
    }
    /// The menu key on the history (デザイン規約 §メニュー のキーボード): the current row's own right-click
    /// (`GraphRowDelegate.askMenu` — the choice drawn in to that row, and its menu, none on the working tree's row),
    /// standing under the row's left end. The row comes on screen first as a step lands on it: as little as will do,
    /// into the middle from out of sight. Only while the list holds the keyboard — the window hands Shift+F10 to the
    /// nearest surface up from whatever does (`KeyMenu.answer`). Says whether a row was asked.
    function menuFromKeys() {
        const row = walk.view.currentIndex
        if (!walk.visible || !walk.view.activeFocus || row < 0 || row >= walk.view.count)
            return false
        walk.revealStep(row, walk.rowShows(row))
        // The rows a move brought on are built at the next polish; the menu needs this one now.
        walk.view.forceLayout()
        const item = walk.view.itemAtIndex(row)
        if (!item || item.isWip)
            return false
        KeyMenu.ask(item.mapToItem(null, 0, Theme.graphRowHeight), () => item.askMenu())
        return true
    }
    /// For the runs: how the last step left `row` in the viewport, given `wasY` from before it — `in` (the view never
    /// moved), `edge` (flush against top or bottom), `center`, or `adrift` (規約 §矢印で履歴を辿る).
    function stepLanding(row, wasY) {
        if (Math.abs(walk.view.contentY - wasY) < 1)
            return "in"
        const top = walk.view.originY + row * Theme.graphRowHeight
        const bottom = top + Theme.graphRowHeight
        const viewBottom = walk.view.contentY + walk.view.height
        if (Math.abs(top - walk.view.contentY) < 1 || Math.abs(bottom - viewBottom) < 1)
            return "edge"
        if (Math.abs((top + bottom) / 2 - (walk.view.contentY + viewBottom) / 2) < Theme.graphRowHeight)
            return "center"
        return "adrift"
    }

    /// Books the reading of the row stepped onto. A press is read at once (it must answer inside the interaction
    /// budget); a step with the key held only pushes the settle back, so a held arrow is read twice — where it set off
    /// and where it stopped — rather than running a selection's git processes at the repeat rate.
    ///
    /// The repeat must say so itself (`held`): every OS's delay before the first repeat (Windows ≥ 250ms, macOS ≥
    /// 225ms, GNOME 500ms by default) outlasts the settle, so without it a held arrow's second row reads as a press.
    function noteStep(held) {
        if (held || stepTimer.running) {
            walk.stepPending = true
            stepTimer.restart()
            return
        }
        walk.stepPending = false
        walk.landStep()
        stepTimer.restart()
    }
    /// A step went by unread; without it the settle behind a single press would read the row twice.
    property bool stepPending: false
    function landStep() {
        // Read the row as it stands now: a background rebuild during the settle rewrites rows in place.
        const row = walk.view.currentIndex
        const oidHex = walk.graphModel.oidAt(row)
        if (oidHex !== "")
            walk.activated(oidHex, row)
    }
    /// Automation: a settle stands between the last step and its reading. The hold verb waits for it to expire, as an
    /// OS's first-repeat delay would, before sending the repeats.
    readonly property alias settling: stepTimer.running
    /// A centering or a shift is still owed to the view (the timers below); `PageSettled` waits on it.
    readonly property bool placing: anchorTimer.running || shiftTimer.running
    Timer {
        id: stepTimer
        interval: Metrics.keyStepSettleMs
        onTriggered: {
            if (!walk.stepPending)
                return
            walk.stepPending = false
            walk.landStep()
        }
    }
    function jumpToRow(row) {
        walk.view.haltGlide()
        walk.view.currentIndex = row
        walk.view.positionViewAtIndex(row, ListView.Center)
    }
    /// Re-centers on the current row a beat from now: a model reset zeroes contentY in the polish after our handlers.
    function anchorSoon() {
        anchorTimer.restart()
    }
    /// Keeps the viewport over the rows it was reading after `rows` commits arrived above them (a rebuild rewrites rows
    /// in place, so the view would silently show other commits). At the top it stays pinned to the newest. Applied a
    /// beat later: clamping against the not-yet-relaid content height would swallow the correction.
    function shiftRows(rows) {
        if (rows === 0)
            return
        if (walk.view.contentY <= walk.view.originY - walk.view.topMargin)
            return
        shiftTimer.pending += rows
        shiftTimer.restart()
    }
    /// Brings `row` into view once its pass has settled, unless already on screen (as `goToMatch` does). Rides the
    /// shift timer: whether the row is on screen is only true after the shift, so one timer fixes the order.
    function showRowSoon(row) {
        shiftTimer.showRow = row
        shiftTimer.restart()
    }
    Timer {
        id: shiftTimer
        property int pending: 0
        property int showRow: -1
        interval: Metrics.anchorDelayMs
        onTriggered: {
            walk.view.haltGlide()
            const rows = shiftTimer.pending
            shiftTimer.pending = 0
            if (rows !== 0)
                walk.view.contentY = walk.view.clampY(walk.view.contentY + rows * Theme.graphRowHeight)
            const row = shiftTimer.showRow
            shiftTimer.showRow = -1
            if (row >= 0 && !walk.view.rowOnScreen(row))
                walk.view.positionViewAtIndex(row, ListView.Center)
        }
    }
    Timer {
        id: anchorTimer
        interval: Metrics.anchorDelayMs
        onTriggered: {
            walk.view.haltGlide()
            if (walk.view.currentIndex >= 0)
                walk.view.positionViewAtIndex(walk.view.currentIndex, ListView.Center)
        }
    }
}
