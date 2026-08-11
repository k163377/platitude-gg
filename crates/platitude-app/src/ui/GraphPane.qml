pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// Center pane: the commit graph. Owns its column geometry (label /
// lane widths, horizontal pan) and every in-pane interaction (wheel,
// middle-click autoscroll, draggable dividers); row clicks go up as
// signals and the owner decides what selection means.
Rectangle {
    id: graphArea

    required property var graphModel
    // The uncommitted row's tallies ride on the ListView for the delegate.
    // They come off status rather than off the file list — the same status
    // the list itself is built from, counted by kind (`status::Kinds`).
    required property var workTree
    // No repository behind this pane: the empty-window call to action.
    property bool blank: false

    /// A row was clicked (or programmatically activated).
    signal rowActivated(string oidHex)
    /// Right-click on a row. What the row is decides which menu opens,
    /// and that is the page's call.
    signal rowMenuOpenRequested(string oidHex)
    /// A row was double-clicked. `record` is the chip it shows (kind +
    /// flags + name); empty when the row shows no branch at all.
    signal rowSwitchRequested(string oidHex, string record)
    /// The chip whose stacked list the page has out (null when none).
    /// Rows read it back through the view: a hand that walked down into
    /// the list and comes back to this chip is not opening anything, so
    /// it is not made to sit out the opening rest again.
    property var chipListAnchor: null
    /// A stacked chip was hovered: unstack it under the chip.
    signal chipExpandRequested(var records, var anchor)
    /// The pointer settled on a row (or left it): open the commit card
    /// under it. `row` is the delegate, which the page needs for its
    /// position and its fields — it must not hold on to it.
    signal rowHoverRequested(var row, bool inside)
    /// The pointer left that chip — put it back, unless it went into the
    /// list itself (only the owner can tell).
    signal chipCollapseRequested()
    /// A name was typed into a row that had no branch on it.
    signal createBranchRequested(string oidHex, string name)
    /// The blank pane's "Open repository…" button.
    signal openRepositoryRequested()

    // ---- naming a branch on a row that has none --------------------
    // In-pane state, like the column widths: the page says where to
    // start and hears back only when there is a name to act on.
    /// Puts the chip column of one row into a branch-name box.
    function startNaming(oidHex) {
        graphList.askOid = ""
        graphList.namingText = ""
        graphList.namingOid = oidHex
    }
    function stopNaming() {
        graphList.namingOid = ""
        graphList.namingText = ""
    }

    // ---- a standing question ---------------------------------------
    // The bar comes down from the top of the pane and pushes the history
    // down (デザイン規約 §可否・警告の出し場所); the row it concerns is
    // marked rather than worded, so the question is written exactly once.

    /// The find bar's state, and its box, for the headless run — the key
    /// that opens it cannot be pressed from there.
    readonly property alias findOpen: findBar.open
    property alias findQuery: findBar.query
    /// What the bar reports back, for the headless run and for whoever
    /// wants to read the search without opening the card.
    readonly property alias findMatches: findBar.matches
    readonly property alias findAt: findBar.atMatch
    readonly property alias findWidth: findBar.width
    /// How far the graph has stepped down out from under the card.
    readonly property real findShift: graphList.anchors.topMargin
    /// Something is being looked for — which is not the same as something
    /// being found. The one thing that lets a row dim, and it takes the
    /// query rather than the count: with a query and no answers, every
    /// row really is "not one of them", so the whole graph goes down.
    readonly property bool findOn: findBar.open && graphArea.graphModel.searching
    /// The newest row is one of the answers, so the graph steps out from
    /// under the card (規約 §コミットを探す). No separate "is the tree
    /// clean" test is needed: the working-tree row is not a commit and
    /// never matches, so a dirty tree answers false here by itself.
    readonly property bool findClears:
        findBar.open && graphArea.graphModel.firstMatched
    /// Where the row this search is on has landed. The page owns what
    /// selection means, so it hears about the row and decides.
    signal findLanded(string oidHex)

    /// Brings the find bar down over this list. Refused while a question
    /// is standing: two bars from one edge would leave neither readable,
    /// and the question is the one with something waiting on it.
    function startFind() {
        if (askBar.label !== "")
            return
        findBar.raise()
        // The card comes back up holding what was last typed into it, and
        // the marks came off when it went away — so the search is run
        // again rather than waiting for a key that may never come.
        graphArea.runFind()
    }

    // ---- the search itself -----------------------------------------
    // The rows are marked in Rust (`GraphModel.setFind`), which is also
    // where the rule about what a typed line matches lives
    // (`platitude-core::find`). Nothing here decides anything about the
    // query; this end moves the viewport and reads the count back.

    /// Re-runs the search and goes to where it lands. Called on every
    /// keystroke: the marking is a walk over rows already in memory, and
    /// no git runs for any of it.
    function runFind() {
        graphArea.graphModel.setFind(findBar.open ? findBar.query : "")
        // From where the reader is, wrapping. At the top of the graph —
        // where it opens, and where most searches start — that is the
        // first match there is.
        graphArea.goToMatch(graphArea.graphModel.matchFrom(graphArea.firstVisibleRow()))
    }
    /// Topmost row with any of itself on screen; 0 while the view is in
    /// its own top margin, where there is no row to be over.
    function firstVisibleRow() {
        const row = graphList.indexAt(0, graphList.contentY + 1)
        return row >= 0 ? row : 0
    }
    /// Whether all of `row` is on screen. A row below the last one drawn
    /// reports no index at all, which is what a list shorter than its
    /// viewport answers for its whole lower half — there, nothing is out
    /// of sight.
    function rowOnScreen(row) {
        const bottom = graphList.indexAt(
            0, graphList.contentY + graphList.height - Theme.graphRowHeight)
        return row >= graphArea.firstVisibleRow() && (bottom < 0 || row <= bottom)
    }
    function findNext() {
        graphArea.goToMatch(graphArea.graphModel.matchAfter(graphList.currentIndex))
    }
    function findPrevious() {
        graphArea.goToMatch(graphArea.graphModel.matchBefore(graphList.currentIndex))
    }
    /// Puts the search on `row`: the page is told so the right-hand panes
    /// follow, and the viewport moves only when the row is not already on
    /// screen — a match in sight is not worth taking the reader's place
    /// for. Which match it is comes out of the bar's own binding, so
    /// nothing here assigns it (that would break the binding, and the
    /// count would then stop following a background refresh).
    function goToMatch(row) {
        if (row < 0)
            return
        if (!graphArea.rowOnScreen(row))
            graphList.positionViewAtIndex(row, ListView.Center)
        graphArea.findLanded(graphArea.graphModel.oidAt(row))
    }

    /// Raises the bar. `oidHex` is the row it is about ("" for none, and
    /// a row outside the loaded window simply goes unmarked — the bar
    /// stands either way).
    /// `hold` takes the answer as a press held down instead of a click,
    /// and `tip` is what the pill says on hover — the questions whose
    /// write leaves this machine ask that way (デザイン規約 §長押し).
    /// `form` is what the question needs in order to take an answer at all
    /// — a chooser, a name box. Most questions have none: they are
    /// answered by the pill and nothing else.
    /// `code` is the git command the question is about, said at the head of
    /// the question and on the pill both, where the act has one word of its
    /// own (デザイン規約 §git 用語のコード表記); `accept` carries the
    /// wording everywhere else.
    function startAsking(oidHex, label, detail, accept, danger,
                         hold = false, tip = "", form = null, code = "") {
        graphList.namingOid = ""
        graphList.namingText = ""
        // Before the label, which is what opens the bar: the form has to
        // exist by the time opening decides where the focus goes.
        askBar.form = form
        askBar.answerable = true
        askBar.neutral = false
        askBar.alert = false
        askBar.label = label
        askBar.detail = detail
        askBar.accept = accept
        askBar.code = code
        askBar.danger = danger
        askBar.hold = hold
        askBar.tip = tip
        graphList.askDanger = danger
        graphList.askOid = oidHex === undefined ? "" : oidHex
    }
    /// Whether the standing question can be answered yet. A question with
    /// a form turns this off until the form has something to send.
    property alias askAnswerable: askBar.answerable
    /// Whether it is asking for information rather than for consent, how it
    /// is answered, and what it says while it stands — these change under a
    /// publish question as the remote answers what the typed name means.
    /// The wording of the pill is not among them: the command it names is
    /// settled when the question opens (デザイン規約 §はじめてリモートへ送る).
    property alias askNeutral: askBar.neutral
    property alias askHold: askBar.hold
    /// The command the pill answers with, and whether the far side could be
    /// read at all — both move under a publish question as the remote
    /// answers, because what would run depends on what is over there.
    property alias askCode: askBar.code
    property alias askAlert: askBar.alert
    property alias askDetail: askBar.detail
    property alias askTip: askBar.tip
    /// The live form, so its owner can read what was typed into it.
    readonly property alias askForm: askBar.formItem
    function stopAsking() {
        askBar.label = ""
        graphList.askOid = ""
    }
    /// Automation: answer a held question the way a person does, by
    /// keeping the pill down to the end.
    function completeHold() {
        askBar.completeHold()
    }
    /// The bar's accept was clicked; the page runs what it guarded.
    signal askConfirmed()
    /// The question was walked away from.
    signal askCancelled()

    /// The list itself — for automation hooks (bench / scroll-to /
    /// screenshot flows) only; app code goes through the functions.
    readonly property alias view: graphList

    /// Moves the selection without scrolling.
    function setCurrentRow(row) {
        graphList.currentIndex = row
    }

    // ---- walking the history with the arrow keys -------------------
    // (規約 §矢印で履歴を辿る)

    /// The page is holding the selection where it is: a half-written
    /// message has put a question up and the highlight back, and it
    /// stands until it is answered. The keys must not tug against it —
    /// each press would be pushed back and the pair would fight.
    property bool selectionHeld: false

    /// Moves the selection one row (`delta` = ∓1) and brings it into
    /// view; answers whether it moved. The keys and the automation hook
    /// both come through here — a headless run cannot inject a keystroke,
    /// so the step has to be callable as well as pressable (verify-ui).
    ///
    /// Refused while something stands over a row: a question waiting to
    /// be answered, a name box being typed into, or the page holding the
    /// selection back. The name box is the one that has to be named
    /// here rather than left to the focus: it lives inside a row, and a
    /// single-line box does not consume Up and Down — they come up
    /// through the delegate to this list.
    function stepRow(delta) {
        if (!graphArea.visible || graphArea.selectionHeld || askBar.label !== ""
                || graphList.namingOid !== "" || graphList.count === 0)
            return false
        const from = graphList.currentIndex
        const row = from < 0 ? 0
                  : Math.max(0, Math.min(from + delta, graphList.count - 1))
        if (row === from)
            return false
        // Asked before the move: whether there was a reading position to
        // keep. A step off the edge is one row short of being on screen,
        // but a selection that was nowhere in sight is nobody's place —
        // that one is centered instead.
        const near = graphArea.rowOnScreen(from)
        graphList.currentIndex = row
        graphArea.revealStep(row, near)
        graphArea.noteStep()
        return true
    }
    /// Brings a stepped-onto row into view. `near` moves as little as
    /// will do, which is the row itself; anything else centers.
    ///
    /// Row positions are worked out rather than read off the items:
    /// `itemAtIndex` answers null for rows the view has not built, and a
    /// walk that trusts it is cut to the viewport (P3-確認事項: the WIP
    /// list's range selection is the standing example).
    function revealStep(row, near) {
        if (!near) {
            graphList.positionViewAtIndex(row, ListView.Center)
            return
        }
        // The first row's box carries the list's top margin with it — its
        // highlight is painted over that sliver — so stepping onto it
        // goes the whole way to the top rather than leaving a dark band.
        const top = graphList.originY + row * Theme.graphRowHeight
                    - (row === 0 ? graphList.topMargin : 0)
        const bottom = graphList.originY + (row + 1) * Theme.graphRowHeight
        if (top < graphList.contentY)
            graphList.contentY = graphList.clampY(top)
        else if (bottom > graphList.contentY + graphList.height)
            graphList.contentY = graphList.clampY(bottom - graphList.height)
    }
    /// How the last step left `row` sitting in the viewport, for the
    /// headless run: `in` when the view never moved (the row was already
    /// there), `edge` when it came in flush against the top or the bottom
    /// (the least a step can move the view), `center` when it was put in
    /// the middle instead. Which of the three is right is the whole of
    /// 規約 §矢印で履歴を辿る's second paragraph, and a screenshot cannot
    /// tell an edge that was reached by one row from one that was jumped
    /// to. `wasY` is where the view stood before that step.
    function stepLanding(row, wasY) {
        if (Math.abs(graphList.contentY - wasY) < 1)
            return "in"
        const top = graphList.originY + row * Theme.graphRowHeight
        const bottom = top + Theme.graphRowHeight
        const viewBottom = graphList.contentY + graphList.height
        if (Math.abs(top - graphList.contentY) < 1
                || Math.abs(bottom - viewBottom) < 1)
            return "edge"
        if (Math.abs((top + bottom) / 2
                     - (graphList.contentY + viewBottom) / 2)
                < Theme.graphRowHeight)
            return "center"
        return "adrift"
    }

    /// Books the reading of the row stepped onto. The first step of a run
    /// is read at once — a single press has to answer inside the
    /// interaction budget — and the ones behind it only push the settle
    /// back. So a held arrow is read exactly twice: where it set off, and
    /// where it stopped. A selection carries three git processes with it
    /// (`git show`, the history question, the signature question), which
    /// is not a thing to run at the keyboard's repeat rate.
    function noteStep() {
        if (stepTimer.running) {
            graphArea.stepPending = true
            stepTimer.restart()
            return
        }
        graphArea.stepPending = false
        graphArea.landStep()
        stepTimer.restart()
    }
    /// Whether a step went by unread while the settle was running. Without
    /// it the settle behind a single press would read the same row twice.
    property bool stepPending: false
    function landStep() {
        // The row under the highlight as it stands, not the one the key
        // asked for: a background rebuild during the settle writes the
        // rows in place, and what was stepped onto is whatever is there
        // to be seen now.
        const oidHex = graphArea.graphModel.oidAt(graphList.currentIndex)
        if (oidHex !== "")
            graphArea.rowActivated(oidHex)
    }
    Timer {
        id: stepTimer
        interval: Metrics.keyStepSettleMs
        onTriggered: {
            if (!graphArea.stepPending)
                return
            graphArea.stepPending = false
            graphArea.landStep()
        }
    }
    /// Moves the selection and centers the viewport on it.
    function jumpToRow(row) {
        graphList.currentIndex = row
        graphList.positionViewAtIndex(row, ListView.Center)
    }
    /// Re-centers on the current row a beat from now. Centering must
    /// outlive the ListView's own relayout: a model reset (tag swap /
    /// reload) zeroes contentY during the polish that runs after our
    /// handlers, so the anchor is applied a beat later.
    function anchorSoon() {
        anchorTimer.restart()
    }
    /// Puts the viewport back over the rows it was reading after `rows`
    /// commits arrived above them. A background rebuild replaces the graph
    /// by writing over the rows in place, so newcomers at the top slide
    /// everything below them down while the view stays where it is and
    /// quietly shows different commits — the further down someone is
    /// reading, the more that costs them.
    ///
    /// At the top of the list there is nothing to preserve: the newest
    /// commits are exactly what belongs there, so it stays pinned.
    /// Applied a beat later, for the same reason as the anchor above: the
    /// list has not laid the new rows out yet, so its content is still the
    /// old height and clamping against it would swallow the correction.
    function shiftRows(rows) {
        if (rows === 0)
            return
        if (graphList.contentY <= graphList.originY - graphList.topMargin)
            return
        shiftTimer.pending += rows
        shiftTimer.restart()
    }
    /// Brings `row` into view once the pass that put it there has settled,
    /// and only if it is not already on screen — a row in sight is not
    /// worth taking the reader's place for (the same rule `goToMatch`
    /// answers to).
    ///
    /// Carried by the shift timer rather than one of its own: the two write
    /// the same contentY for opposite reasons, and whether the row is on
    /// screen is only true of the position the shift leaves behind. One
    /// timer settles the order.
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
            const rows = shiftTimer.pending
            shiftTimer.pending = 0
            if (rows !== 0)
                graphList.contentY = graphList.clampY(
                    graphList.contentY + rows * Theme.graphRowHeight)
            const row = shiftTimer.showRow
            shiftTimer.showRow = -1
            if (row >= 0 && !graphArea.rowOnScreen(row))
                graphList.positionViewAtIndex(row, ListView.Center)
        }
    }
    Timer {
        id: anchorTimer
        interval: Metrics.anchorDelayMs
        onTriggered: {
            if (graphList.currentIndex >= 0)
                graphList.positionViewAtIndex(graphList.currentIndex, ListView.Center)
        }
    }

    // Adjustable column widths (labels / graph); -1 = automatic. The
    // graph column starts at a default lane count and scrolls
    // horizontally when the full graph is wider.
    property real labelWManual: -1
    property real graphColWManual: -1
    // The narrowest the chip column goes, and the widest. `spaceXxl` is
    // where the divider has always refused to be dragged any further in.
    // The far end is where the two columns after it would stop being able
    // to say anything: one lane, and enough message column to show that
    // there is a message (規約 §グラフ列は最も広い所のレーンまで).
    readonly property real labelColWMin: Theme.spaceXxl
    readonly property real labelColWMax: Math.max(labelColWMin,
        width - graphColWMin - Metrics.messageMinW)
    // The chip column does not give when the pane narrows — the lanes do
    // (`graphColWMax`), and the window's floor holds this one's width in
    // reserve (規約 §窓の床). Squeezing it was tried and taken back out:
    // a column narrower than a chip draws a crushed one, and a column
    // that follows the pane changes width whenever the left menu folds,
    // which is a thing moving on screen that nobody asked to move
    // (2026-08-09 ユーザー報告 — 3 つの症状が全部これだった).
    //
    // A column somebody dragged is still held inside what the pane can
    // lay out: that one is as wide as a hand made it, and the message
    // column has to survive the window being narrowed afterwards.
    //
    // Read back rounded — here and in `graphColW` — because the two
    // kinds of reader sit on different grids: the rows' ticks are laid
    // out by RowLayouts, which snap to whole pixels, while the divider
    // hover lines read these raw. On a fractional width (a drag, or a
    // fractional floor) the tick and the line straddle the same half
    // pixel differently and stop meeting (2026-08-11 報告). Rounding
    // what everyone reads keeps one grid without touching what the
    // drag wrote down.
    readonly property real labelW: Math.round(
        labelWManual >= 0
        ? Math.max(labelColWMin, Math.min(labelWManual, labelColWMax))
        : Metrics.labelColW)
    // The narrowest this pane can be laid out with all three columns still
    // saying something. The page's floor is built on it (RepoPage), which
    // is what keeps the window from being dragged past it.
    readonly property real contentMinW:
        Metrics.labelColW + graphColWMin + Metrics.messageMinW
    readonly property real graphFullW: Metrics.laneInset
                                       + Math.max(1, graphModel.maxLanes) * Metrics.laneW
                                       + Theme.spaceSm
    // The narrowest the column goes: the message tick brought up
    // against lane 0's co-author badge without touching it. The badge
    // is the widest ink any row puts on that lane, and its geometry —
    // author up-left by a border, badge centre a border inside the
    // node's edge, outline half in half out — mirrors what
    // **GraphRowDelegate's canvases** draw. The ceiling lands that
    // edge on a whole pixel; the tick stands `spaceSm` past the
    // column's edge (the subject column's own margin), so that much
    // comes back off the width. The faces are not cut on the way down:
    // their clipper leans the same `spaceSm` past the column (the
    // delegate), so the floor is where ink meets ink, not where the
    // clip ran out. One place, so the display, the divider's clamp and
    // the question of whether there is a drag in it at all agree.
    readonly property real graphColWMin:
        Math.ceil(Metrics.laneInset + Metrics.laneW / 2
                  + Metrics.nodeIcon / 2 - 2 * Theme.borderWidth
                  + (Theme.iconSm + Theme.borderWidth) / 2)
        - Theme.spaceSm
    // How far the divider may be pulled: as wide as the lanes ever get,
    // and no wider — a column past the last lane is emptiness taken from
    // the message column. A narrow window stops it earlier still, where
    // the message column would stop showing that a message is there
    // (規約 §グラフ列は最も広い所のレーンまで).
    readonly property real graphColWMax: Math.max(graphColWMin,
        Math.min(graphFullW, width - labelW - Metrics.messageMinW))
    /// Whether the column is the only width it can be — nothing left
    /// between its ceiling and its floor but the tail gap. Even a
    /// one-lane history keeps a real drag now: its full width holds
    /// the gap after the last lane, the floor tucks the tick against
    /// the badge, and the stretch between the two does something. The
    /// divider draws no line and turns the cursor away only when the
    /// window has squeezed the ceiling down onto the floor and a drag
    /// would come to nothing (規約 §グラフ列は最も広い所のレーンまで).
    readonly property bool graphColWFixed: graphColWMax <= graphColWMin + Theme.spaceSm
    // Rounded for the same one-grid reason as `labelW`.
    readonly property real graphColW: Math.round(Math.min(graphColWMax,
        graphColWManual >= 0 ? Math.max(graphColWManual, graphColWMin)
                             : Metrics.laneInset + Metrics.graphDefaultLanes * Metrics.laneW
                               + Theme.spaceSm))
    property real graphX: 0
    readonly property real graphXMax: Math.max(0, graphFullW - graphColW)
    onGraphXMaxChanged: graphX = Math.min(graphX, graphXMax)
    /// Where a subject's first character sits — the two columns, then the
    /// tick and the gap after it. **Must match GraphRowDelegate's third
    /// column**, whose RowLayout lays out the same three steps; the find
    /// bar measures its cap from here (§コミットを探す).
    readonly property real subjectTextX: labelW + graphColW + Theme.spaceSm
                                         + 2 * Theme.borderWidth + Theme.spaceXs

    color: Theme.bgSurface

    // Divider hover-lines live under the list so the message ticks and
    // lane strokes stay in front.
    Rectangle {
        id: labelDividerLine
        x: labelDivider.x + Theme.borderWidth
        width: Theme.splitterWidth - 2 * Theme.borderWidth
        height: parent.height
        color: Theme.borderStrong
        // Reads the pane's own property rather than the MouseArea's hover,
        // so the automation hook can raise it too — hover cannot be
        // injected, and a line nothing can prove is a line nothing checks
        // (app-ui.md, the same shape the graph divider's line uses).
        visible: graphArea.labelPointed || labelDivider.pressed
    }
    Rectangle {
        id: graphDividerLine
        x: graphDivider.x + Theme.borderWidth
        width: Theme.splitterWidth - 2 * Theme.borderWidth
        height: parent.height
        color: Theme.borderStrong
        // The line says "this moves". A column with one width does not,
        // so it stays out and the cursor speaks instead
        // (規約 §グラフ列は最も広い所のレーンまで).
        visible: !graphArea.graphColWFixed
                 && (graphArea.dividerPointed || graphDivider.pressed)
    }
    // The list starts under the bar — the graph moves down rather than
    // losing its top rows behind it.
    AskBar {
        id: askBar
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        z: 3
        onConfirmed: graphArea.askConfirmed()
        onCancelled: graphArea.askCancelled()
    }
    // Hangs from the top-right corner, over the list rather than above it
    // — declared here rather than inside the view because a Flickable
    // adopts what is declared in it and scrolls it away. Only one of the
    // two is ever up: a question already standing keeps the place, since
    // it is one gesture from being over.
    FindBar {
        id: findBar
        anchors.top: parent.top
        anchors.right: parent.right
        anchors.rightMargin: Theme.spaceLg
        z: 4
        loaded: graphArea.graphModel.rowTotal
        // Bound, not assigned when a key is pressed: a background refresh
        // re-marks the rows without anybody typing (app-ui.md §QML バイン
        // ディングはプロパティにしか反応しない), and the count has to be
        // the rows' count rather than the last keystroke's.
        matches: graphArea.graphModel.matchCount
        // Reads two properties and asks the model where the current row
        // sits among the matches — so it settles again whenever either
        // the marks or the selection move.
        atMatch: graphArea.graphModel.matchCount > 0
                 ? graphArea.graphModel.matchOrdinal(graphList.currentIndex) : 0
        // "There is a query" is the model's answer, not a second reading
        // of the text here: what counts as a query at all (whitespace
        // does not) is `platitude-core::find`'s rule and belongs in one
        // place.
        refused: findBar.open && graphArea.graphModel.searching
                 && graphArea.graphModel.matchCount === 0
        refusedTip: graphArea.graphModel.truncated
                    ? qsTr("Nothing in the loaded history matches — older commits are not loaded")
                    : qsTr("Nothing in this history matches")
        // How far left the card may reach: `spaceXs` past where a subject
        // starts, which is about half of the first character
        // (§コミットを探す). Measured from the columns rather than from
        // the pane, so the cap follows the dividers when they are dragged
        // — it is a distance from the tick the messages begin at, not a
        // fraction of the window.
        maxWidth: graphArea.width - graphArea.subjectTextX - Theme.spaceXs
                  - anchors.rightMargin
        onDismissed: {
            graphArea.runFind()
            graphList.takeKeyboard()
        }
        onQueryChanged: graphArea.runFind()
        onNextRequested: graphArea.findNext()
        onPreviousRequested: graphArea.findPrevious()
    }
    ListView {
        id: graphList
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.top: askBar.bottom
        // The card hangs over the top rows rather than pushing them down
        // — but when the newest commit is itself one of the answers, the
        // graph steps down by the card's height so that answer is not
        // the one thing the search covers. It goes back the moment the
        // top row stops matching, so the band is not a place the eye
        // learns to expect (規約 §コミットを探す).
        anchors.topMargin: graphArea.findClears ? findBar.height : 0
        Behavior on anchors.topMargin {
            NumberAnimation { duration: 200 }
        }
        clip: true
        model: graphArea.graphModel
        reuseItems: true
        // Nothing but this pane's own functions move the view. Left on,
        // the list chases its current item: a background rebuild that
        // re-resolves the selection onto a row one further down drags a
        // reader parked in the history to wherever the selection is. The
        // arrow keys move the view themselves, by as little as will do
        // (`revealStep`), which is not what the chase would do for them.
        highlightFollowsCurrentItem: false
        // Qt's own key navigation moves `currentIndex` and tells nobody:
        // the highlight would walk off screen — the chase above is off —
        // while the panes on the right went on showing the commit it set
        // off from. The arrows are answered below instead, where the page
        // hears about where they landed (規約 §矢印で履歴を辿る).
        keyNavigationEnabled: false
        Keys.onUpPressed: event => event.accepted = graphArea.stepRow(-1)
        Keys.onDownPressed: event => event.accepted = graphArea.stepRow(1)
        /// Where the keyboard goes when a press lands in this pane. Every
        /// way in comes through here — a row click, a press on the lanes,
        /// the find card closing, the headless hook — so there is one
        /// answer to "what does a press do to the keyboard".
        function takeKeyboard() {
            graphList.forceActiveFocus()
        }
        /// And gives it up when this pane is taken off the screen. Qt
        /// leaves active focus on an item it has just made invisible, and
        /// the keys go on arriving there (qmltestrunner で実測
        /// 2026-08-11: a StackLayout child swapped away reports
        /// `visible=false activeFocus=true`, and the next Down still
        /// fires; `focus = false` is what lets go). Opening a diff over
        /// the graph did exactly that: the arrows walked the selection
        /// behind the diff, and moving the selection closes the diff — so
        /// the screen was pulled back to the graph (2026-08-11 ユーザー報告).
        onVisibleChanged: {
            if (!graphList.visible)
                graphList.focus = false
        }
        boundsBehavior: Flickable.StopAtBounds
        flickDeceleration: 8000
        maximumFlickVelocity: 9000
        ScrollBar.vertical: AutoScrollBar {}
        // The graph is the one pane with no header band; this sliver of
        // margin drops the first row so its bottom line meets the
        // neighbouring bands' bottom edge when scrolled to the top.
        topMargin: Theme.headerHeight - Theme.graphRowHeight
        // A sliver of run-out at the end: without it the oldest row
        // sits flush on the pane edge and reads as clipped rather than
        // as the end of what is loaded. Just enough to see the break.
        bottomMargin: Theme.spaceSm
        // Bridge into the delegate (GraphRowDelegate reads its column
        // geometry off ListView.view).
        property real labelWidth: graphArea.labelW
        property real graphColWidth: graphArea.graphColW
        property real graphFullWidth: graphArea.graphFullW
        property real graphXOffset: graphArea.graphX
        property int wipAdded: graphArea.workTree.wipAdded
        property int wipModified: graphArea.workTree.wipModified
        property int wipDeleted: graphArea.workTree.wipDeleted
        property int wipRenamed: graphArea.workTree.wipRenamed
        property int wipCopied: graphArea.workTree.wipCopied
        property int wipConflicted: graphArea.workTree.conflictCount
        // Which row's chip column is a name box, and what has been typed
        // into it. Held here rather than in the delegate: the delegate is
        // recycled the moment its row scrolls off.
        property string namingOid: ""
        property string namingText: ""
        // Which row the standing question is about, and in which tone —
        // held here for the same recycling reason. The words are on the
        // bar; the row only marks itself.
        property string askOid: ""
        property bool askDanger: false
        // Mirrored for the delegates, which can only see the view: rows
        // dim while a search is on.
        readonly property bool findOn: graphArea.findOn
        signal rowSelected(string oidHex)
        signal rowMenuRequested(string oidHex)
        signal rowSwitchRequested(string oidHex, string record)
        signal chipExpandRequested(var records, var anchor)
        signal chipCollapseRequested()
        signal rowHoverRequested(var row, bool inside)
        // Mirrored for the delegates, which can only see the view.
        readonly property var chipListAnchor: graphArea.chipListAnchor
        signal namingSubmitted(string oidHex, string name)
        signal namingCancelled()
        onRowMenuRequested: oidHex => graphArea.rowMenuOpenRequested(oidHex)
        onRowSelected: oidHex => graphArea.rowActivated(oidHex)
        onRowSwitchRequested: (oidHex, record) =>
            graphArea.rowSwitchRequested(oidHex, record)
        onChipExpandRequested: (records, anchor) =>
            graphArea.chipExpandRequested(records, anchor)
        onChipCollapseRequested: graphArea.chipCollapseRequested()
        onRowHoverRequested: (row, inside) =>
            graphArea.rowHoverRequested(row, inside)
        onNamingSubmitted: (oidHex, name) => {
            graphArea.stopNaming()
            // An empty box is the way out of the offer, not a branch
            // called nothing.
            if (name !== "")
                graphArea.createBranchRequested(oidHex, name)
        }
        onNamingCancelled: graphArea.stopNaming()
        delegate: GraphRowDelegate {}
        // Window cut: lanes keep running through the footer and the
        // message sits where subjects go. One row tall — the lanes carry
        // on for exactly one more commit's worth, which is all it takes
        // to read as "and it continues" — unless the message needs more
        // than that. It is the only thing explaining the cut, so a narrow
        // subject column grows the footer rather than eliding it.
        footer: Item {
            width: graphList.width
            height: graphArea.graphModel.truncated
                    ? Math.max(Theme.graphRowHeight,
                               tailMessage.contentHeight + 2 * Theme.spaceXs)
                    : 0
            visible: graphArea.graphModel.truncated
            Item {
                x: graphList.labelWidth
                width: graphList.graphColWidth
                height: parent.height
                clip: true
                Canvas {
                    id: tailCanvas
                    x: -graphList.graphXOffset
                    width: graphList.graphFullWidth
                    height: parent.height
                    onPaint: {
                        const ctx = getContext("2d")
                        ctx.clearRect(0, 0, width, height)
                        if (graphArea.graphModel.tailGeometry === "")
                            return
                        ctx.lineWidth = Metrics.laneStroke
                        ctx.globalAlpha = Metrics.dimFade
                        // Same tokens as a row's geometry (uppercase =
                        // dashed leash): a stash or WIP row whose target
                        // sits past the cut keeps dotting through here.
                        const toks = graphArea.graphModel.tailGeometry.split(";")
                        for (let n = 0; n < toks.length; n++) {
                            const t = toks[n]
                            const dot = t.indexOf(".")
                            const lane = parseInt(t.substring(1, dot))
                            const color = parseInt(t.substring(dot + 1))
                            const x = Metrics.laneInset + lane * Metrics.laneW + Metrics.laneW / 2
                            ctx.strokeStyle = Theme.graphLane[color % Theme.graphLane.length]
                            ctx.setLineDash(t[0] === t[0].toLowerCase()
                                            ? [] : Metrics.laneDash)
                            ctx.beginPath()
                            ctx.moveTo(x, 0)
                            ctx.lineTo(x, height)
                            ctx.stroke()
                        }
                        ctx.setLineDash([])
                    }
                    Connections {
                        target: graphArea.graphModel
                        function onStatsChanged() { tailCanvas.requestPaint() }
                    }
                }
            }
            // Bounded like a subject: the message stays in the subject
            // column rather than running under the next pane. Told in the
            // secondary colour, not a state one: the window is how the
            // graph is meant to work, and nothing is waiting on it
            // (デザイン規約 §状態).
            Label {
                id: tailMessage
                x: graphList.labelWidth + graphList.graphColWidth
                width: graphList.width - x
                // Its own laid-out height, centered in whatever the
                // footer ends up being: reading the footer's height back
                // here is the binding loop, since the footer is sized
                // from this. Nothing elides — the height follows the
                // wrap, so every line of the message is shown.
                height: contentHeight
                y: (parent.height - height) / 2
                leftPadding: Theme.spaceSm
                rightPadding: Theme.spaceSm
                wrapMode: Text.Wrap
                // The walk's own count, not the row count: the WIP row and
                // sifted stash parents move rows off the round window
                // limit, and this footer only stands when the walk hit it.
                text: qsTr("Only the first %L1 commits are loaded")
                      .arg(graphArea.graphModel.walkedTotal)
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMd
                font.weight: Font.DemiBold
            }
        }
        // Manual contentY math must respect originY: after
        // positionViewAtIndex jumps, the ListView shifts its coordinate
        // origin as item positions are fixed up, so
        // [0, contentHeight-height] no longer matches the real scroll
        // range (top rows become unreachable, the bottom overshoots the
        // truncation footer).
        function clampY(y) {
            // topMargin lives above the content origin — forgetting it
            // makes the top gap unreachable by wheel after any scroll.
            const minY = graphList.originY - graphList.topMargin
            const maxY = Math.max(minY, graphList.originY
                                        + graphList.contentHeight
                                        - graphList.height
                                        + graphList.bottomMargin)
            return Math.max(minY, Math.min(y, maxY))
        }
        // Mouse wheels scroll a fixed number of rows per notch;
        // touchpads keep native Flickable panning.
        WheelHandler {
            acceptedDevices: PointerDevice.Mouse
            onWheel: event => {
                // Wheel input exits middle-click autoscroll
                // (Chrome-like behavior).
                graphArea.autoScrolling = false
                graphList.cancelFlick()
                if (event.angleDelta.x !== 0)
                    graphArea.graphX = Math.max(0, Math.min(
                        graphArea.graphX - event.angleDelta.x / 2, graphArea.graphXMax))
                const step = (event.angleDelta.y / 120)
                           * Metrics.wheelRows * Theme.graphRowHeight
                graphList.contentY = graphList.clampY(graphList.contentY - step)
            }
        }
    }
    // Middle-click toggles autoscroll mode: the pointer distance from
    // the anchor sets the speed; any click exits.
    property bool autoScrolling: false
    property real autoAnchorX: 0
    property real autoAnchorY: 0
    property real autoCurrentX: 0
    property real autoCurrentY: 0
    /// Whether this autoscroll carries the lanes sideways as well. Decided
    /// by where the middle click landed, and kept for the whole gesture
    /// (デザイン規約 §グラフを横へ送る).
    property bool autoPanning: false
    /// Starts autoscroll from a point in this pane's frame. The press and
    /// the automation hook both come through here, so which column offers
    /// the sideways drift is answered in exactly one place.
    function startAutoScroll(x, y) {
        graphArea.autoAnchorX = x
        graphArea.autoAnchorY = y
        graphArea.autoCurrentX = x
        graphArea.autoCurrentY = y
        graphArea.autoPanning = x >= graphArea.labelW
                                && x < graphArea.labelW + graphArea.graphColW
        graphArea.autoScrolling = true
    }
    /// Where the pointer has drifted to since. Its distance from the
    /// anchor is what the ticker below reads as speed — the moving
    /// pointer and the automation hook write the same two values.
    function driftPointer(x, y) {
        graphArea.autoCurrentX = x
        graphArea.autoCurrentY = y
    }
    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.MiddleButton
        onClicked: mouse => graphArea.startAutoScroll(mouse.x, mouse.y)
    }
    MouseArea {
        visible: graphArea.autoScrolling
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.AllButtons
        // The cursor says which ways this gesture goes, so a press that
        // did not land on the lanes does not look broken when the lanes
        // stay put under a sideways drift.
        cursorShape: graphArea.autoPanning ? Qt.SizeAllCursor : Qt.SizeVerCursor
        onPositionChanged: mouse => graphArea.driftPointer(mouse.x, mouse.y)
        onPressed: mouse => {
            graphArea.autoScrolling = false
            mouse.accepted = true
        }
        Timer {
            running: graphArea.autoScrolling
            interval: 16
            repeat: true
            onTriggered: {
                const delta = (graphArea.autoCurrentY - graphArea.autoAnchorY)
                            * Metrics.middleScrollGain
                graphList.contentY = graphList.clampY(graphList.contentY + delta)
                // Sideways drift pans the lanes — for a gesture that
                // started on them.
                if (graphArea.autoPanning && graphArea.graphXMax > 0) {
                    const dx = (graphArea.autoCurrentX - graphArea.autoAnchorX)
                             * Metrics.middleScrollGain
                    graphArea.graphX = Math.max(0, Math.min(graphArea.graphX + dx,
                                                            graphArea.graphXMax))
                }
            }
        }
        // Anchor marker
        Rectangle {
            x: graphArea.autoAnchorX - Theme.iconMd / 2
            y: graphArea.autoAnchorY - Theme.iconMd / 2
            width: Theme.iconMd
            height: Theme.iconMd
            radius: Theme.iconMd / 2
            color: "transparent"
            border.color: Theme.borderStrong
            border.width: Theme.borderWidth
            Rectangle {
                anchors.centerIn: parent
                width: Theme.spaceXs
                height: Theme.spaceXs
                radius: Theme.spaceXs / 2
                color: Theme.borderStrong
            }
        }
    }
    // Left-drag inside the lanes pans them horizontally when they
    // overflow; a motionless press-release still selects the row
    // underneath.
    MouseArea {
        id: lanePan
        x: graphArea.labelW
        width: graphArea.graphColW
        height: parent.height
        z: 1
        visible: graphArea.graphXMax > 0
        acceptedButtons: Qt.LeftButton
        property real pressX: 0
        property real startGX: 0
        property bool panning: false
        onPressed: mouse => {
            pressX = mouse.x
            startGX = graphArea.graphX
            panning = false
        }
        onPositionChanged: mouse => {
            if (!pressed)
                return
            if (!panning && Math.abs(mouse.x - pressX) > Theme.spaceXs)
                panning = true
            if (panning)
                graphArea.graphX = Math.max(0, Math.min(
                    startGX - (mouse.x - pressX), graphArea.graphXMax))
        }
        // The lanes are part of the row, so a double-click on them means
        // what it means anywhere else on it. Without this the gesture
        // would die in exactly the repositories wide enough to need
        // panning, and nothing on screen would say why.
        onDoubleClicked: mouse => {
            // mouse.y is in this MouseArea's frame, which starts at the
            // pane's top; the list starts below the ask bar. Map, or a
            // standing question makes every lane click land rows lower.
            const p = lanePan.mapToItem(graphList, mouse.x, mouse.y)
            const idx = graphList.indexAt(graphArea.labelW + 1,
                                          graphList.contentY + p.y)
            if (idx < 0)
                return
            // Asked of the row itself, so which chip a row leads to is
            // worked out in exactly one place.
            const row = graphList.itemAtIndex(idx)
            if (row && row.movable)
                graphList.rowSwitchRequested(graphArea.graphModel.oidAt(idx),
                                             row.primaryRecord)
        }
        onReleased: mouse => {
            if (panning)
                return
            // Same frame correction as the double-click above.
            const p = lanePan.mapToItem(graphList, mouse.x, mouse.y)
            const idx = graphList.indexAt(graphArea.labelW + 1,
                                          graphList.contentY + p.y)
            if (idx >= 0) {
                // Same as a click on the row itself: the lanes are part
                // of the row, so a press that lands on them leaves the
                // keyboard here too (規約 §矢印で履歴を辿る).
                graphList.takeKeyboard()
                graphList.currentIndex = idx
                graphList.rowSelected(graphArea.graphModel.oidAt(idx))
            }
        }
    }

    // Draggable column dividers (labels | graph | message).
    MouseArea {
        id: labelDivider
        x: graphArea.labelW - Theme.splitterWidth / 2
        width: Theme.splitterWidth
        height: parent.height
        z: 2
        visible: !graphArea.blank
        hoverEnabled: true
        cursorShape: Qt.SplitHCursor
        preventStealing: true
        onContainsMouseChanged: {
            graphArea.labelPointed = containsMouse
            // Entering does not always bring a move with it, and the
            // badge is drawn where this says the hand is.
            if (containsMouse)
                graphArea.labelPoint = labelDivider.mapToItem(null, mouseX, mouseY)
        }
        onPositionChanged: mouse => {
            graphArea.labelPoint = labelDivider.mapToItem(null, mouse.x, mouse.y)
            if (!pressed)
                return
            graphArea.dragLabelTo(mapToItem(graphArea, mouse.x, 0).x)
        }
    }
    MouseArea {
        id: graphDivider
        // Sits behind the message tick column so the hover line
        // overlaps the ticks.
        x: graphArea.labelW + graphArea.graphColW + Theme.spaceSm
           + Theme.borderWidth - Theme.splitterWidth / 2
        width: Theme.splitterWidth
        height: parent.height
        z: 2
        visible: !graphArea.blank
        hoverEnabled: true
        // The cursor never changes: it is the platform's splitter shape
        // in both states, and a column that will not move says so with
        // the badge below instead (規約 §グラフ列は最も広い所のレーン
        // まで). Swapping in a drawn arrow made the refusal read as a
        // different tool from the divider one column over.
        cursorShape: Qt.SplitHCursor
        preventStealing: true
        onContainsMouseChanged: {
            graphArea.dividerPointed = containsMouse
            // Entering does not always bring a move with it, and the
            // mark is drawn where this says the hand is.
            if (containsMouse)
                graphArea.dividerPoint = graphDivider.mapToItem(null, mouseX, mouseY)
        }
        onPositionChanged: mouse => {
            graphArea.dividerPoint = graphDivider.mapToItem(null, mouse.x, mouse.y)
            if (!pressed)
                return
            graphArea.dragDividerTo(mapToItem(graphArea, mouse.x, 0).x)
        }
    }
    /// Where a drag on the graph divider leaves the column, and what it
    /// asked for on the way. The handler and the automation hook both come
    /// through here, so the clamp is one answer rather than two kept in
    /// step — and the refusal below reads the ask, so it is one too.
    function dragDividerTo(px) {
        if (graphArea.graphColWFixed)
            return
        graphArea.dividerAsked =
            px - Theme.spaceSm - Theme.borderWidth - graphArea.labelW
        graphArea.graphColWManual = Math.max(graphArea.graphColWMin,
            Math.min(graphArea.dividerAsked, graphArea.graphColWMax))
    }
    /// The same for the chip column's divider, whose asked width is simply
    /// where the pointer is — that column starts at the pane's edge.
    function dragLabelTo(px) {
        graphArea.labelAsked = px
        graphArea.labelWManual = Math.max(graphArea.labelColWMin,
            Math.min(px, graphArea.labelColWMax))
    }
    /// Whether the pointer is on each divider, and where. Real hover and
    /// the automation hooks write the same pairs — hover cannot be
    /// injected (verify-ui), and both the lines and the badge have to be
    /// provable.
    property bool dividerPointed: false
    property point dividerPoint: Qt.point(0, 0)
    property bool labelPointed: false
    property point labelPoint: Qt.point(0, 0)
    /// What each divider's last drag asked its column for, clamped or not.
    /// The clamped answer is the column's width; this is what the hand
    /// wanted, which is the only thing that can tell a refusal from a rest.
    property real dividerAsked: 0
    property real labelAsked: 0
    /// Whether a drag is under way. The hooks' own flags ride beside the
    /// real presses because a press is no more injectable than hover is
    /// (verify-ui) — and they ride here rather than inside the refusals,
    /// so each refusal stays one expression that both roads reach.
    readonly property bool dividerDragging: graphDivider.pressed || dividerHeld
    readonly property bool labelDragging: labelDivider.pressed || labelHeld
    /// Automation only: stand in for the presses the hooks cannot make.
    /// Nothing a hand can reach writes these.
    property bool dividerHeld: false
    property bool labelHeld: false
    /// Whether a drag is asking a column for a width it cannot have.
    /// **Either end** — a hand that has run out has run out whichever way
    /// it was going, and answering only one way leaves the other reading
    /// as a divider that broke.
    ///
    /// Held apart from hover on purpose: at either bound the column still
    /// moves the other way, so a badge worn merely for standing there
    /// would say "this does not move" about a divider that does.
    ///
    /// Bindings, not something set and taken back down: letting go ends
    /// the ask, and a badge left on screen by a teardown nobody ran is
    /// exactly the failure a hand would see and a run could not.
    readonly property bool dividerRefused:
        dividerDragging && graphArea.outOfRange(dividerAsked,
                                                graphColWMin, graphColWMax)
    readonly property bool labelRefused:
        labelDragging && graphArea.outOfRange(labelAsked,
                                              labelColWMin, labelColWMax)
    /// Whether an ask has run out past either end of what it is allowed,
    /// by more than the divider is wide — so a press on its own cannot
    /// trip it: grabbing a line lands its column anywhere within half that
    /// width of where it already sat, and at a bound half of those grabs
    /// would be asking for more.
    ///
    /// Safe to call from a binding: everything that varies arrives as an
    /// argument, so the binding takes its dependencies from the call site
    /// (unlike a method that reads them itself — app-ui.md).
    function outOfRange(asked, floor, ceiling) {
        return asked > ceiling + Theme.splitterWidth
            || asked < floor - Theme.splitterWidth
    }
    /// Automation: the pointer resting on the graph divider, at its middle
    /// (`PG_AUTO_ACT=graph-divider`). Where that is stays here rather
    /// than in the hook — one answer, not a second one to keep in step.
    function restDividerPointer(inside) {
        graphArea.dividerPointed = inside
        if (inside)
            graphArea.dividerPoint = graphDivider.mapToItem(
                null, graphDivider.width / 2, graphArea.height / 2)
    }
    /// Automation: a drag carried out past one of the four bounds these
    /// two dividers have (`PG_AUTO_ACT=divider-refuse`, whose argument
    /// names which). Presses are no more injectable than hover is
    /// (verify-ui), so each walks the road its own handler walks and takes
    /// the pointer where a hand would have carried it.
    function dragDividerPast(which) {
        const over = 2 * Theme.splitterWidth
        if (which === "label-min" || which === "label-max") {
            const lx = which === "label-max" ? graphArea.labelColWMax + over
                                             : graphArea.labelColWMin - over
            graphArea.labelPointed = true
            graphArea.labelHeld = true
            graphArea.labelPoint = graphArea.mapToItem(null, lx, graphArea.height / 2)
            graphArea.dragLabelTo(lx)
            return
        }
        const px = graphArea.labelW + Theme.spaceSm + Theme.borderWidth
                   + (which === "graph-min" ? graphArea.graphColWMin - over
                                            : graphArea.graphColWMax + over)
        graphArea.dividerPointed = true
        graphArea.dividerHeld = true
        graphArea.dividerPoint = graphArea.mapToItem(null, px, graphArea.height / 2)
        graphArea.dragDividerTo(px)
    }
    /// Whether either divider is refusing, and where the hand is while it
    /// does (**scene coordinates**). One pointer, so one answer at a time:
    /// the two dividers cannot both have the hand.
    ///
    /// Two ways to it, and they are asked differently. A column squeezed
    /// until it has no drag left in either direction answers on hover —
    /// there is nothing to try. A column at a bound answers only the drag
    /// that tried, because it still moves the other way.
    ///
    /// The badge is not drawn here: a drag carries the hand out past this
    /// pane, and a badge parented to it would be composited among the
    /// page's panes rather than over them. The page owns the one badge.
    readonly property bool refused:
        dividerRefused || labelRefused
        || (graphColWFixed && dividerPointed)
    readonly property point refusedAt: labelRefused ? labelPoint : dividerPoint
    /// What is drawn, not what was asked for: the automation hook reports
    /// the line and the badge themselves, so a column that cannot be
    /// resized but still promises a drag cannot pass.
    readonly property alias graphDividerShown: graphDivider.visible
    readonly property alias graphDividerLineShown: graphDividerLine.visible
    readonly property alias labelDividerLineShown: labelDividerLine.visible
    /// Whether the badge the page draws for this pane is up. The page owns
    /// it, so this is the pane's half of that answer.
    readonly property bool graphDividerRefuses: graphArea.refused
    /// The line of whichever divider has the hand. A refused drag has to
    /// leave it drawn — the boundary still moves the other way — so this
    /// is the half of the picture the badge does not hold, and it is the
    /// pane that knows which of the two lines is being asked about.
    readonly property bool refusedLineShown:
        labelDragging ? labelDividerLine.visible : graphDividerLine.visible
    /// Whether the pointer is anywhere in this pane. A `HoverHandler`
    /// rather than a `MouseArea`: handlers are passive, so the rows',
    /// chips' and dividers' own hover does not take this one away. Real
    /// hover and the automation hook write the same property — hover
    /// cannot be injected (verify-ui).
    property bool pointerInside: false
    HoverHandler {
        onHoveredChanged: graphArea.pointerInside = hovered
    }
    /// Automation: the pointer resting in the pane, which is the only
    /// thing that puts the lane bar on screen (`PG_AUTO_ACT=graph-bar`).
    function restPointer(inside) {
        graphArea.pointerInside = inside
    }
    /// What is drawn, not what was asked for: the automation hook reports
    /// the bar itself so a broken binding cannot pass.
    readonly property alias laneBarShown: laneBar.visible
    // Horizontal scroll of the lanes when the full graph is wider than
    // its column. The bar lies over the lanes of the last row, so it
    // comes out only while the pointer is in the pane — and stays out
    // for as long as it is being dragged, wherever that has taken the
    // pointer (デザイン規約 §グラフを横へ送る).
    ScrollBar {
        id: laneBar
        visible: graphArea.graphXMax > 0
                 && (graphArea.pointerInside || pressed)
        // Fusion draws its handle only in the style's "active" state,
        // which for a bar that is not attached to a Flickable means while
        // the pointer is on the bar itself — a 6px strip on the pane's
        // bottom edge that nobody would find. When the bar is out it is
        // because this pane put it there, so the style stops deciding.
        policy: ScrollBar.AlwaysOn
        orientation: Qt.Horizontal
        x: graphArea.labelW
        width: graphArea.graphColW
        anchors.bottom: parent.bottom
        z: 2
        size: graphArea.graphFullW > 0 ? graphArea.graphColW / graphArea.graphFullW : 1
        position: graphArea.graphFullW > 0 ? graphArea.graphX / graphArea.graphFullW : 0
        onPositionChanged: {
            if (pressed)
                graphArea.graphX = Math.max(0, Math.min(
                    position * graphArea.graphFullW, graphArea.graphXMax))
        }
    }
    // First load, before any row exists. The drawn ring, not Fusion's
    // BusyIndicator — the window has one turning mark and this is it
    // (規約 §進行中・長押しの定数).
    NavIcon {
        anchors.centerIn: parent
        width: Theme.iconLg
        height: Theme.iconLg
        kind: "spinner"
        tint: Theme.textSecondary
        visible: graphArea.graphModel.loading && graphArea.graphModel.rowTotal === 0
        // On the render thread, so it keeps turning while the GUI
        // thread drains models.
        RotationAnimator on rotation {
            running: graphArea.graphModel.loading
                     && graphArea.graphModel.rowTotal === 0
                     && AppBackend.shotDir === ""
            loops: Animation.Infinite
            from: 0
            to: 360
            duration: Metrics.spinMs
        }
    }
    Label {
        anchors.centerIn: parent
        visible: graphArea.graphModel.error !== ""
        text: graphArea.graphModel.error
        color: Theme.danger
        width: parent.width - 2 * Theme.spaceXl
        wrapMode: Text.Wrap
        horizontalAlignment: Text.AlignHCenter
    }
    // Empty window: the one thing worth doing sits in the column that
    // will hold the history.
    Column {
        anchors.centerIn: parent
        visible: graphArea.blank
        spacing: Theme.spaceLg
        Label {
            text: qsTr("Platitude GG")
            font.pixelSize: Theme.fontXl
            font.weight: Font.DemiBold
            anchors.horizontalCenter: parent.horizontalCenter
        }
        Label {
            text: qsTr("A thin, fast GUI over your installed git.")
            color: Theme.textSecondary
            // Its natural width while the column has room for it, and
            // wrapped inside the column when it has not: at the window's
            // floor this pane is narrower than the line asks for, and
            // unbounded it took the difference from the panes on either
            // side (規約 §窓の床). The same shape the error line above
            // already has.
            width: Math.min(implicitWidth,
                            graphArea.width - 2 * Theme.spaceXl)
            wrapMode: Text.Wrap
            horizontalAlignment: Text.AlignHCenter
            anchors.horizontalCenter: parent.horizontalCenter
        }
        // A plain frame, not the accent: the accent is for the button
        // somebody came to press, and this page is what stands there when
        // nobody has opened anything yet. A frame all the same — bare is
        // the shape an answer takes beside a framed one, and there is
        // nothing beside this to read it against; under two lines of
        // centred text a button with no edge is a third line
        // (規約 §肯定側のボタン / §枠を持てる場所にだけ枠を出す).
        ActionButton {
            implicitHeight: Theme.controlHeight
            text: qsTr("Open repository…")
            frameColor: Theme.borderDefault
            activeFocusOnTab: true
            anchors.horizontalCenter: parent.horizontalCenter
            onActivated: graphArea.openRepositoryRequested()
        }
    }
}
