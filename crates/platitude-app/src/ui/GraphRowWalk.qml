pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

// The graph's selection and where the view stands to show it: the arrow keys (規約 §矢印で履歴を辿る), the page's
// jumps, and keeping the reader's place across a background rebuild. Draws nothing.
Item {
    id: walk

    required property var view
    required property var graphModel
    /// The current branch's stand-in (`GraphHeadPin`): the band it covers at the top edge, and whether it stands there
    /// at a given place of the view. Null for none.
    property var standIn: null
    /// A question is standing over a row.
    required property bool asking
    /// A step came to rest on a row: read it. The row travels with the id, since every worktree's uncommitted row
    /// carries the all-zero one (`RepoPage.openWipFor`).
    signal activated(string oidHex, int atRow)

    /// Moves the selection one row (`delta` = ∓1) and brings it into view; answers whether it moved. Keys and runs
    /// both come here. `held` = the key was already down (`KeyEvent.isAutoRepeat`), which only changes when the row
    /// is read (`noteStep`).
    ///
    /// Refused while the pane is off screen (verbs call this directly, past the keys), a question stands (it moves only
    /// once answered, 規約 §可否・警告の出し場所), or a name box is open (a half-typed name is not left behind) — a
    /// single-line box does not consume Up / Down (Left / Right it does), so they reach the list
    /// through the delegate. `namingOid` is asked by name: focus cannot tell, the list being a focus scope whose
    /// `activeFocus` stays true while a child holds it. Not refused for a message being written in the details pane:
    /// an arrow is a click, and leaving goes without asking about the draft (`DetailsPane.syncMessage`).
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
    /// (`GraphRowDelegate.askMenu` — inside a choice of two the pair's menu, elsewhere the choice drawn in to that row
    /// and its menu, none on the worktree's row),
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
        // This worktree's own uncommitted row has no menu; another worktree's has (`GraphRowDelegate.askMenu`).
        if (!item || item.isWip && !item.carried || item.menuShut())
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
    /// A centering or a row brought into view is still owed to the view (the timers below, `Metrics.anchorDelayMs`);
    /// `PageSettled` waits on it (`placing`). A reread's own send is placed in the turn it lands and owes nothing.
    /// A new delay that writes the view's `contentY` joins this property, or a run completes before the view moves.
    readonly property bool placing: anchorTimer.running || showTimer.running
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
    /// Keeps the viewport over the rows it was reading after `rows` commits arrived above them — a rebuild rewrites
    /// rows in place, so the view would show other commits — in the same turn as the rebuild, so no frame shows them.
    /// At the top it stays pinned to the newest, but for a hand over the rows: a reread is no reader's doing, and what
    /// the hand is on stays where it is (デザイン規約 §hover のツールチップ「外乱では閉じない」). What the view cannot
    /// take back — a history too short to scroll that far — moves under the hand as the reread's
    /// (`HandWatch.rowsMoved`). The rebuild's layout is forced here, kept or not: clamped against the content height
    /// before it, the correction would be swallowed, and the rows it moves — a row built at the end lands too — are
    /// weighed in this turn, taken back with the view (`keepingRows`) or moved as the reread's (`rereading`), not at
    /// the next frame as a reader's move.
    function shiftRows(rows) {
        const hand = walk.view.hand
        const keep = rows !== 0 && (walk.view.contentY > walk.view.originY - walk.view.topMargin || hand.present)
        if (keep)
            walk.view.haltGlide()
        walk.view.keepingRows = keep
        walk.view.rereading = !keep
        walk.view.forceLayout()
        walk.view.rereading = false
        if (!keep) {
            walk.view.keepingRows = false
            return
        }
        const from = walk.view.contentY
        const full = rows * Theme.graphRowHeight
        walk.view.contentY = walk.view.clampY(from + walk.keptRows(rows, from) * Theme.graphRowHeight)
        walk.view.keepingRows = false
        if (Math.abs(walk.view.contentY - from - full) >= 1)
            hand.rowsMoved(true)
    }
    /// How many of the `rows` written in above the view takes back: all of them, but where the current branch's
    /// stand-in stands at the top edge then over the row the hand is over — the hand's row comes down just as far as
    /// clears its band (`GraphHeadPin.topBand`), the view giving back the rows the band would cover and going back up
    /// for the rest: a few commits on the branch move the rows down as they came
    /// (デザイン規約 §hover のツールチップ「外乱では閉じない」). Never into the band under the stand-in (user decision —
    /// ask before changing; `GraphHeadPin`): as many commits as the band holds (three) come in with
    /// nothing taken back and the rows lowered by as many — the card opens under its row, so the lowered row still
    /// reads at the card's edge; more, and the hand's row stops just under the band, the rest hidden above.
    /// A hand's row the band covered before the reread stays where it is (`handUnderBand`).
    function keptRows(rows, from) {
        const pin = walk.standIn
        if (pin === null || walk.handOid === "" || !walk.view.hand.present || walk.handUnderBand)
            return rows
        const row = walk.graphModel.rowOf(walk.handOid)
        if (row < 0)
            return rows
        let kept = rows
        for (;;) {
            const to = from + kept * Theme.graphRowHeight
            if (!pin.aboveAt(to) || walk.view.originY + row * Theme.graphRowHeight - to >= pin.topBand)
                return kept
            kept--
        }
    }
    /// The commit the hand is over and its row — where it was last heard over the rows, kept while the hand is in a
    /// card hung off them, found again after every pass (`noteHand`) — by which a reread is weighed (`handShift`).
    property string handOid: ""
    property int handRow: -1
    /// …and whether the current branch's stand-in covered that row then (`keptRows`).
    property bool handUnderBand: false
    /// Notes the row under the hand; for a hand the rows do not hear now (in a card, under a menu), where the commit
    /// it was last over stands now.
    function noteHand() {
        const hand = walk.view.hand
        if (hand.known) {
            const p = walk.view.contentItem.mapFromItem(null, hand.at.x, hand.at.y)
            walk.handRow = walk.view.indexAt(p.x, p.y)
            walk.handOid = walk.handRow >= 0 ? walk.graphModel.oidAt(walk.handRow) : ""
        } else {
            walk.handRow = walk.handOid === "" ? -1 : walk.graphModel.rowOf(walk.handOid)
            if (walk.handRow < 0)
                walk.handOid = ""
        }
        const pin = walk.standIn
        walk.handUnderBand = pin !== null && walk.handRow >= 0 && pin.aboveAt(walk.view.contentY)
            && walk.view.originY + walk.handRow * Theme.graphRowHeight - walk.view.contentY < pin.topBand
    }
    /// How far a reread moved the row the hand is over (デザイン規約 §hover のツールチップ「外乱では閉じない」), or
    /// `undefined` for no answer: no hand over the rows, the commit it was over gone. A worktree's row stays at the top
    /// whatever is written in under it — and every worktree's row wears the same all-zero id.
    function handShift() {
        if (!walk.view.hand.present || walk.handOid === "")
            return undefined
        if (GitFacts.wipOid(walk.handOid))
            return 0
        const now = walk.graphModel.rowOf(walk.handOid)
        return now < 0 ? undefined : now - walk.handRow
    }
    // Every place the hand is heard at, and every move of the rows under it.
    property Connections handWatch: Connections {
        target: walk.view.hand
        function onMoved() { walk.noteHand() }
        function onShiftsChanged() { walk.noteHand() }
    }
    /// Brings `row` into view once its pass has settled, unless already on screen (as `goToMatch` does) — a beat later,
    /// weighed against where the view stands once the rows are laid out. Asked after `activateRow`. A reread's
    /// `shiftRows` does not wait for it: it runs in the turn that rewrote the rows (`stats_changed` closes the drain
    /// that wrote them → `RepoPage.onStatsChanged`), so "on screen" is weighed after that shift and no frame shows
    /// another commit.
    function showRowSoon(row) {
        showTimer.showRow = row
        showTimer.restart()
    }
    Timer {
        id: showTimer
        property int showRow: -1
        interval: Metrics.anchorDelayMs
        onTriggered: {
            const row = showTimer.showRow
            showTimer.showRow = -1
            if (row < 0 || walk.view.rowOnScreen(row))
                return
            walk.view.haltGlide()
            walk.view.positionViewAtIndex(row, ListView.Center)
        }
    }
    /// Brings `row` into the open and touches nothing else — not the selection, not `currentIndex`: a row standing
    /// whole in the view and clear of the current branch's stand-in stays where it is; one cut by an edge, under the
    /// stand-in or out of sight is centred (デザイン規約 §複数のコミットを選ぶ). What a click on a held commit asks
    /// (`RepoPage.showChosen`). -1 is no row.
    function revealRow(row) {
        if (row < 0 || walk.rowInTheOpen(row))
            return
        walk.view.haltGlide()
        walk.view.positionViewAtIndex(row, ListView.Center)
    }
    /// Whether all of `row` stands in the view, none of it under the band the stand-in covers at the top edge
    /// (`GraphHeadPin.topBand`, as `keptRows` weighs it). Weighed off the rows' places, not `rowOnScreen`, which counts
    /// a row the top edge cuts as on screen.
    function rowInTheOpen(row) {
        const top = walk.view.originY + row * Theme.graphRowHeight
        const pin = walk.standIn
        const covered = pin !== null && pin.aboveAt(walk.view.contentY) ? pin.topBand : 0
        return top >= walk.view.contentY + covered
            && top + Theme.graphRowHeight <= walk.view.contentY + walk.view.height
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
