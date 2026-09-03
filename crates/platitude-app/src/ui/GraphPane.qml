pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// Center pane: the commit graph.
Rectangle {
    id: graphArea

    required property var graphModel
    // The uncommitted row's tallies ride on the ListView for the delegate. They come off status rather than off the
    // file list — the same status the list itself is built from, counted by kind (`status::Kinds`).
    required property var workTree
    // No repository behind this pane: the empty-window call to action.
    property bool blank: false

    signal rowActivated(string oidHex, int atRow)
    /// Right-click on a row, wherever along it. `record` is the name its chip draws (kind + flags + name), empty on a
    /// row that draws none — the menu is the row's either way, and that name is what its cards are about.
    signal rowMenuOpenRequested(string oidHex, string record)
    /// A row was double-clicked. `record` is the chip it shows (kind + flags + name); empty when the row shows no
    /// branch at all.
    signal rowSwitchRequested(string oidHex, string record)
    /// The chip whose stacked list the page has out (null when none). Rows read it back through the view: a hand that
    /// walked down into the list and comes back to this chip is not opening anything, so it is not made to sit out the
    /// opening rest again.
    property var chipListAnchor: null
    /// The commit whose card the page has out (empty when none). Rows read it back through the view the same way: the
    /// row the card came out of stays lit under it, the card having taken the pointer off it.
    property string rowCardOid: ""
    /// A stacked chip was hovered: unstack it under the chip. The row comes with it — every name in the card is on
    /// that one commit, so a click in the card is a click on that row.
    signal chipExpandRequested(string oidHex, int atRow, var records, var anchor)
    /// A row was clicked a second time, late enough for the double-click to have been ruled out: the name on its chip
    /// is being changed (デザイン規約 §グラフ行のダブルクリック). `record` is the chip's first one, whatever kind it names.
    ///
    /// **Raised for the card the chip unfolds into as well** — its rows answer through the four below, because a hand
    /// clicking one spot twice clicks the row and then the card, and those are one target.
    signal rowRenameRequested(string oidHex, string record)
    /// How the graph's rows answer a click, for the card that stands on them (`RowHoverHost`). **The card's rows are
    /// these rows**, so they go through this door rather than building the same key and the same payload beside it —
    /// two surfaces that had to agree on what "the same target" means is exactly what was wrong before.
    function noteRowClick(oidHex, record, held) {
        return graphList.noteClick(oidHex, record, held)
    }
    function dropRowRename() {
        graphList.dropRename()
    }
    function rowRenameArmed(record) {
        return graphList.renameArmed(record)
    }
    readonly property alias rowClickGuarded: graphList.clickGuarded
    /// The pointer settled on a row (or left it): open the commit card under it. `row` is the delegate, which the page
    /// needs for its position and its fields — it must not hold on to it.
    signal rowHoverRequested(var row, bool inside)
    /// The pointer left that chip — put it back, unless it went into the list itself (only the owner can tell).
    signal chipCollapseRequested()
    signal createBranchRequested(string oidHex, string name)
    /// The same box, answered with a tag instead. Two signals rather than one with a word in it: what the page does
    /// with the answer is a different command, and the pane has already read which box it was.
    signal createTagRequested(string oidHex, string name)
    /// And the same box again, answered with a new name for something that already has one. The kind travels with it:
    /// what git is asked is a different command for each (`RepoPage.renameRow`).
    signal renameSubmitted(string kind, string id, string name)
    signal openRepositoryRequested()

    // ---- naming a branch or a tag on a row -------------------------
    /// Puts the chip column of one row into a name box. `startNaming` asks for a branch — the answer to a row with
    /// nowhere to move to — and `startTagging` for a tag on the same commit; the mode goes in before the row that
    /// carries it, so the box comes up already holding its own question.
    function startNaming(oidHex) {
        graphArea.openNameBox(oidHex, "branch", "", "", "")
    }
    function startTagging(oidHex) {
        graphArea.openNameBox(oidHex, "tag", "", "", "")
    }
    /// The third of them: the box opens holding a name that is already there, to be typed over. `kind` is what git is
    /// being asked to rename ("branch" / "remote" / "tag"), `id` the ref it answers to, and `text` the name as it is
    /// typed — a remote branch is typed without the remote it is on, so the two are not the same string.
    function startRenaming(oidHex, kind, id, text) {
        graphArea.openNameBox(oidHex, "rename", kind, id, text)
    }
    function openNameBox(oidHex, mode, kind, id, text) {
        graphList.askOid = ""
        // Before the row that carries it: the box comes up already holding its own question, and what it opened
        // holding is what a press elsewhere weighs against (`dropEmptyBoxes`).
        graphList.namingText = text
        graphList.namingMode = mode
        graphList.namingKind = kind
        graphArea.namingId = id
        graphArea.namingOpenedWith = text
        graphList.namingOid = oidHex
    }
    /// A rename is out of this box and git has not answered yet, and what git said about the name still in it — the
    /// pair the left menu's box carries under its own names (`SidebarRowGestures.editWaiting` / `editGitRefusal`).
    property bool namingWaiting: false
    property string namingGitRefusal: ""
    // What git said is about the name it was asked about; one key on top of it and that is no longer the name in the
    // box.
    Connections {
        target: graphList
        function onNamingTextChanged() {
            graphArea.namingGitRefusal = ""
        }
    }
    /// The answer: the box has done its job, or git would not have the name and the box is where that belongs.
    function renameLanded() {
        if (graphArea.namingWaiting)
            graphArea.stopNaming()
    }
    function renameRefused(why) {
        if (!graphArea.namingWaiting)
            return
        graphArea.namingWaiting = false
        graphArea.namingGitRefusal = why
    }
    function stopNaming() {
        graphArea.namingWaiting = false
        graphArea.namingGitRefusal = ""
        // **A box coming down spends the gesture that opened it.** The press that walks away from a box lands before
        // the click it belongs to (`FocusRelease` fires on the press, `MouseArea.clicked` on the release), so by the
        // time the click is answered the box is already gone — and a click that found no box would come up as a
        // second one and open it again a window later, which is the blink that was reported twice (observed). Asked
        // here rather than at the click, because this is the one place that knows a box was standing.
        if (graphList.namingOid !== "")
            graphList.forgetClicks()
        graphList.namingOid = ""
        graphList.namingText = ""
        graphList.namingKind = ""
        graphArea.namingId = ""
        graphArea.namingOpenedWith = ""
    }
    /// The ref a rename box is changing the name of, and the name it came up holding. Empty for the two boxes that
    /// make a name rather than change one.
    property string namingId: ""
    property string namingOpenedWith: ""
    /// What the box is standing on, for the page to decide whether what is typed can be accepted at all — the models
    /// that answer that are the page's (`RepoPage.graphNameRefusedWhy`).
    readonly property alias namingMode: graphList.namingMode
    readonly property alias namingKind: graphList.namingKind
    readonly property alias namingOid: graphList.namingOid
    readonly property alias namingText: graphList.namingText
    property alias namingRefused: graphList.namingRefused
    property alias namingRefusedWhy: graphList.namingRefusedWhy

    // ---- looking for a commit --------------------------------------
    /// The card itself — automation-only exposure, the same one `view` and `headPin` are (app-ui.md). A headless run
    /// reads its `open` / `query` / `matches` / `atMatch` / `width` / `opacity` / `findClears` off it and types into
    /// its box; eight names on this pane said nothing the card does not, and the key that opens it cannot be pressed
    /// from there.
    readonly property alias findCard: findBar
    /// How far the graph has stepped down out from under the card. This one is the list's, not the card's.
    readonly property real findShift: graphList.anchors.topMargin
    signal findLanded(string oidHex)
    function startFind() { findBar.startFind() }
    function findNext() { findBar.findNext() }
    function findPrevious() { findBar.findPrevious() }

    // ---- a press that landed somewhere else -------------------------
    /// Both boxes this pane can be standing on are offers rather than work half done: the find card and the name box on
    /// a row. An empty one goes away with the press that landed elsewhere (`RepoPage.releasePressedAway`, `scenePos` =
    /// where it landed); one with something typed in it stays, because the typing is what there would be to lose.
    function dropEmptyBoxes(scenePos) {
        findBar.dropIfEmpty(scenePos)
        // "Empty" is "nothing has been put in it": for a box that makes a name that is no text at all, and for one
        // changing a name it is the name it opened holding — a rename that has not been typed in has nothing to lose,
        // and one that has is the same half-written name the two others keep.
        if (graphList.namingOid !== "" && graphList.namingText === graphArea.namingOpenedWith)
            graphArea.stopNaming()
    }
    /// Whether all of `row` is on screen — asked of the list, which is where the viewport arithmetic lives.
    function rowOnScreen(row) { return graphList.rowOnScreen(row) }

    // ---- a standing question ---------------------------------------
    // The bar comes down from the top of the pane and pushes the history down (デザイン規約 §可否・警告の出し場所); the row it
    // concerns is marked rather than worded, so the question is written exactly once.

    /// Raises the bar. `oidHex` is the row it is about ("" for none, and a row outside the loaded window simply goes
    /// unmarked — the bar stands either way). `hold` takes the answer as a press held down instead of a click, and
    /// `tip` is what the pill says on hover — the questions whose write leaves this machine ask that way (デザイン規約 §長押し).
    /// `form` is what the question needs in order to take an answer at all — a chooser, a name box. Most questions have
    /// none: they are answered by the pill and nothing else. `code` is the git command the question is about, said at
    /// the head of the question and on the pill both, where the act has one word of its own (デザイン規約 §git 用語のコード表記);
    /// `accept` carries the wording everywhere else. `refName` is the ref the question names inside its own words, and
    /// **`label` comes with the name's seat still in it (`%1`) whenever one is handed over**: the name is drawn in its
    /// own colour, so the sentence has to be cut at the seat rather than the name looked for in the finished line
    /// (`AskBar.labelSentence`, デザイン規約 §ref の種別).
    function startAsking(oidHex, label, detail, accept, danger, hold = false, tip = "", form = null, code = "",
                         refName = "") {
        graphArea.stopNaming()
        // Down before the new one goes in, or the last question's typing comes back under these words (`AskBar.form`).
        askBar.form = null
        askBar.form = form
        askBar.answerable = true
        askBar.neutral = false
        askBar.alert = false
        askBar.labelSentence = refName === "" ? "" : label
        askBar.labelRef = refName
        askBar.label = refName === "" ? label : label.arg(refName)
        askBar.detail = detail
        askBar.accept = accept
        askBar.code = code
        askBar.danger = danger
        askBar.hold = hold
        askBar.tip = tip
        graphList.askDanger = danger
        graphList.askOid = oidHex === undefined ? "" : oidHex
        // Last of all: dressed, then raised — so nothing on a bar the reader can see is ever written (`AskBar.open`).
        askBar.open = true
    }
    /// Whether the standing question can be answered yet. A question with a form turns this off until the form has
    /// something to send.
    property alias askAnswerable: askBar.answerable
    /// Whether it is asking for information rather than for consent, how it is answered, and what it says while it
    /// stands — these change under a publish question as the remote answers what the typed name means. The wording of
    /// the pill is not among them: the command it names is settled when the question opens (デザイン規約 §はじめてリモートへ送る).
    property alias askNeutral: askBar.neutral
    property alias askHold: askBar.hold
    /// The command the pill answers with, and whether the far side could be read at all — both move under a publish
    /// question as the remote answers, because what would run depends on what is over there.
    property alias askCode: askBar.code
    property alias askAlert: askBar.alert
    property alias askDetail: askBar.detail
    property alias askTip: askBar.tip
    /// The live form, so its owner can read what was typed into it.
    readonly property alias askForm: askBar.formItem
    /// The bar itself — automation-only exposure, the same one `view` and `findCard` are (app-ui.md). A headless run
    /// reads its `settled` / `shut` / `label` / `accept` off it and presses the ✕ through `dismiss()`; five names on
    /// this pane said nothing the bar does not, and none of them is a thing that can be pressed from there.
    readonly property alias askCard: askBar
    function stopAsking() {
        // Only lowered. The words and the colour are left where they are — the bar is on screen for the whole 200ms it
        // takes to go, and the next question is what re-dresses it (`AskBar.open`).
        askBar.open = false
        graphList.askOid = ""
    }
    /// Automation: answer a held question the way a person does, by keeping the pill down to the end.
    function completeHold() {
        askBar.completeHold()
    }
    signal askConfirmed()
    signal askCancelled()

    /// The list itself — for automation hooks (bench / scroll-to / screenshot flows) only; app code goes through the
    /// functions.
    readonly property alias view: graphList

    /// Moves the selection without scrolling.
    function setCurrentRow(row) {
        graphList.currentIndex = row
    }

    // ---- walking the history with the arrow keys, and where the view
    // stands for it (規約 §矢印で履歴を辿る) ---------------------------
    GraphRowWalk {
        id: rowWalk
        view: graphList
        graphModel: graphArea.graphModel
        asking: askBar.open
        // The walk names the commit it landed on, not the row — the page looks that one up.
        onActivated: oidHex => graphArea.rowActivated(oidHex, -1)
    }
    function stepRow(delta, held) { return rowWalk.stepRow(delta, held) }
    function stepLanding(row, wasY) { return rowWalk.stepLanding(row, wasY) }
    /// Automation only, like `view` above: whether a step is still waiting to be read (`GraphRowWalk.settling`).
    readonly property alias stepSettling: rowWalk.settling
    function jumpToRow(row) { rowWalk.jumpToRow(row) }
    function anchorSoon() { rowWalk.anchorSoon() }
    function shiftRows(rows) { rowWalk.shiftRows(rows) }
    function showRowSoon(row) { rowWalk.showRowSoon(row) }

    // The three columns' widths, and how far the lanes have been sent sideways. Worked out in GraphColumnMetrics; the
    // pane's own name for each answer is the alias below it, so the rows, the dividers, the find bar and the page all
    // keep reading them off this pane.
    GraphColumnMetrics {
        id: metrics
        paneW: graphArea.width
        maxLanes: graphArea.graphModel.maxLanes
    }
    property alias labelWManual: metrics.labelWManual
    property alias graphColWManual: metrics.graphColWManual
    readonly property alias labelW: metrics.labelW
    readonly property alias contentMinW: metrics.contentMinW
    readonly property alias graphFullW: metrics.graphFullW
    readonly property alias graphColWMin: metrics.graphColWMin
    readonly property alias graphColWMax: metrics.graphColWMax
    readonly property alias graphColW: metrics.graphColW
    property alias graphX: metrics.graphX
    readonly property alias graphXMax: metrics.graphXMax
    readonly property alias subjectTextX: metrics.subjectTextX

    color: Theme.bgSurface

    // Divider hover-lines live under the list so the message ticks and lane strokes stay in front.
    Rectangle {
        id: labelDividerLine
        x: columnDividers.labelX + Theme.borderWidth
        width: Theme.splitterWidth - 2 * Theme.borderWidth
        height: parent.height
        color: Theme.borderStrong
        visible: columnDividers.labelLineWanted
    }
    Rectangle {
        id: graphDividerLine
        x: columnDividers.graphDividerX + Theme.borderWidth
        width: Theme.splitterWidth - 2 * Theme.borderWidth
        height: parent.height
        color: Theme.borderStrong
        visible: columnDividers.graphLineWanted
    }
    // The list starts under the question — the graph moves down rather than losing its top rows behind it. A closed
    // bar has no height at all, so it costs nothing while none stands.
    //
    // **The report is not here.** It is about a write, and a write can be answered while the reader is looking at a
    // file rather than at the history, so it stands above the whole middle of the page rather than inside this pane
    // (`RepoPage`, デザイン規約 §答えの要らない報せ). What is left here is the question, which is always about the rows below it.
    AskBar {
        id: askBar
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        z: 3
        onConfirmed: graphArea.askConfirmed()
        onCancelled: graphArea.askCancelled()
    }
    // Hangs from the top-right corner, over the list rather than above it — declared here rather than inside the view
    // because a Flickable adopts what is declared in it and scrolls it away. Only one of the two is ever up: a question
    // already standing keeps the place, since it is one gesture from being over.
    GraphFind {
        id: findBar
        anchors.top: parent.top
        anchors.right: parent.right
        anchors.rightMargin: Theme.spaceLg
        z: 4
        graphModel: graphArea.graphModel
        view: graphList
        asking: askBar.open
        // How far left the card may reach: `spaceXs` past where a subject starts, which is about half of the first
        // character (§コミットを探す). Measured from the columns rather than from the pane, so the cap follows the dividers
        // when they are dragged — it is a distance from the tick the messages begin at, not a fraction of the window.
        maxWidth: graphArea.width - graphArea.subjectTextX - Theme.spaceXs - anchors.rightMargin
        onDismissed: {
            findBar.runFind()
            graphList.takeKeyboard()
        }
        onLanded: oidHex => graphArea.findLanded(oidHex)
    }
    GraphList {
        id: graphList
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.top: askBar.bottom
        // The card hangs over the top rows rather than pushing them down — but when the newest commit is itself one of
        // the answers, the graph steps down by the card's height so that answer is not the one thing the search covers.
        // It goes back the moment the top row stops matching, so the band is not a place the eye learns to expect (規約
        // §コミットを探す).
        anchors.topMargin: findBar.findClears ? findBar.height : 0
        Behavior on anchors.topMargin {
            NumberAnimation { duration: 200 }
        }
        graphModel: graphArea.graphModel
        workTree: graphArea.workTree
        columns: metrics
        findOn: findBar.findOn
        chipListAnchor: graphArea.chipListAnchor
        rowCardOid: graphArea.rowCardOid
        // The arrows are answered here rather than in the list, because it is this pane that walks the history with
        // them and the page that hears where they landed (規約 §矢印で履歴を辿る). The key says whether it was already down,
        // which is the only thing that tells a run apart from a press (`GraphRowWalk.noteStep`).
        Keys.onUpPressed: event => event.accepted = graphArea.stepRow(-1, event.isAutoRepeat)
        Keys.onDownPressed: event => event.accepted = graphArea.stepRow(1, event.isAutoRepeat)
        onRowMenuRequested: (oidHex, record) => graphArea.rowMenuOpenRequested(oidHex, record)
        onRowSelected: (oidHex, atRow) => graphArea.rowActivated(oidHex, atRow)
        onRowSwitchRequested: (oidHex, record) => graphArea.rowSwitchRequested(oidHex, record)
        onRowRenameRequested: (oidHex, record) => graphArea.rowRenameRequested(oidHex, record)
        onChipExpandRequested: (oidHex, atRow, records, anchor) =>
            graphArea.chipExpandRequested(oidHex, atRow, records, anchor)
        onChipCollapseRequested: graphArea.chipCollapseRequested()
        onRowHoverRequested: (row, inside) => graphArea.rowHoverRequested(row, inside)
        onNamingSubmitted: (oidHex, name, mode) => {
            const kind = graphList.namingKind
            const id = graphArea.namingId
            const was = graphArea.namingOpenedWith
            // **A rename keeps its box until git answers** — the same rule the left menu's box follows, and for the
            // same reason (`SidebarRowGestures.submitEdit`, デザイン規約 §答えの要らない報せ). Everything else is done with its
            // box the moment it is submitted.
            if (mode === "rename" && name !== "" && name !== was) {
                graphArea.namingWaiting = true
                graphArea.renameSubmitted(kind, id, name)
                return
            }
            graphArea.stopNaming()
            // An empty box is the way out of the offer, not a branch or a tag called nothing — and a rename that
            // reaches here is one left holding the name it opened with, which is the same way out.
            if (name === "" || mode === "rename")
                return
            if (mode === "tag")
                graphArea.createTagRequested(oidHex, name)
            else
                graphArea.createBranchRequested(oidHex, name)
        }
        onNamingCancelled: graphArea.stopNaming()
        onWheelTaken: graphArea.autoScrolling = false
        onWheelPanned: delta => graphArea.graphX = Math.max(0,
            Math.min(graphArea.graphX - delta / 2, graphArea.graphXMax))
    }
    /// Whether a middle-click autoscroll is under way, and whether it carries the lanes sideways as well — read by the
    /// wheel, which ends the gesture, and by the automation hook.
    property alias autoScrolling: autoScroll.scrolling
    readonly property alias autoPanning: autoScroll.panning
    /// Starts autoscroll from a point in this pane's frame, and moves the pointer of one already under way. The presses
    /// and the automation hook both come through here.
    function startAutoScroll(x, y) {
        autoScroll.start(x, y)
    }
    function driftPointer(x, y) {
        autoScroll.drift(x, y)
    }
    MiddleAutoScroll {
        id: autoScroll
        anchors.fill: parent
        panFrom: graphArea.labelW
        panTo: graphArea.labelW + graphArea.graphColW
        canPan: graphArea.graphXMax > 0
        onDrifted: (dy, dx) => {
            graphList.contentY = graphList.clampY(graphList.contentY + dy)
            if (dx !== 0)
                graphArea.graphX = Math.max(0, Math.min(graphArea.graphX + dx, graphArea.graphXMax))
        }
    }
    GraphLanePan {
        id: lanePan
        columns: metrics
        view: graphList
    }
    /// The lane strip — automation-only exposure, the same one `view` is (app-ui.md). A run has no pointer to press
    /// with, and the strip is the one way in that covers the column between the two dividers.
    readonly property alias lanePan: lanePan
    // The current branch's stand-in, riding whichever edge its own row went out of. **Over the lane strip and under the
    // dividers**: the strip takes presses across the lane column, so a stand-in below it would answer its own lanes
    // with the row scrolling underneath — and the dividers stay on top, because a boundary that can be dragged is only
    // a few pixels wide wherever it crosses. It follows the list's frame instead of living in it (`view`), which is
    // what keeps it still while the rows go by.
    GraphHeadPin {
        id: headPin
        z: 1
        graphModel: graphArea.graphModel
        view: graphList
        labelWidth: graphArea.labelW
        graphColWidth: graphArea.graphColW
        graphFullWidth: graphArea.graphFullW
        graphXOffset: graphArea.graphX
        // The list's own bar, which this lies on top of — the strip every hand laid over a list gives back
        // (`AppListView.barRoom`).
        barRoom: graphList.barRoom
        onActivated: row => {
            graphList.takeKeyboard()
            graphArea.jumpToRow(row)
            graphArea.rowActivated(graphArea.graphModel.oidAt(row), row)
        }
    }
    /// The stand-in itself — automation-only exposure, the same one `view` is (app-ui.md). A headless run reads what it
    /// drew (`visible` / `rowAbove` / `lit`), rests the pointer on it by writing the one property the pointer's own
    /// arrival writes, and presses it through its own signal: five names on this pane said nothing the item does not.
    readonly property alias headPin: headPin

    // Draggable column dividers (labels | graph | message). The hand is in there; the two lines it raises are drawn
    // above, under the list. Same seat in the stack as the two dividers had: over the lane pan, under the lane bar.
    GraphColumnDividers {
        id: columnDividers
        anchors.fill: parent
        z: 2
        columns: metrics
        blank: graphArea.blank
    }
    /// Whether either divider is refusing, and where the hand is while it does (**scene coordinates**) — the page draws
    /// the one badge, since a drag carries the hand out past this pane.
    readonly property alias refused: columnDividers.refused
    readonly property alias refusedAt: columnDividers.refusedAt
    /// Automation: the pointer resting on the graph divider, and a drag carried out past one of the four bounds these
    /// two dividers have.
    function restDividerPointer(inside) {
        columnDividers.restDividerPointer(inside)
    }
    function dragDividerPast(which) {
        columnDividers.dragDividerPast(which)
    }
    /// What is drawn, not what was asked for: the automation hook reports the line and the badge themselves, so a
    /// column that cannot be resized but still promises a drag cannot pass.
    readonly property alias graphDividerShown: columnDividers.graphDividerShown
    readonly property alias graphDividerLineShown: graphDividerLine.visible
    /// Whether the badge the page draws for this pane is up. The page owns it, so this is the pane's half of that
    /// answer.
    readonly property alias graphDividerRefuses: columnDividers.refused
    /// The line of whichever divider has the hand. A refused drag has to leave it drawn — the boundary still moves the
    /// other way — so this is the half of the picture the badge does not hold, and it is the pane that knows which of
    /// the two lines is being asked about.
    readonly property bool refusedLineShown: columnDividers.labelDragging ? labelDividerLine.visible
        : graphDividerLine.visible
    /// Whether the pointer is anywhere in this pane. A `HoverHandler` rather than a `MouseArea`: handlers are passive,
    /// so the rows', chips' and dividers' own hover does not take this one away. Real hover and the automation hook
    /// write the same property — hover cannot be injected (verify-ui).
    ///
    /// **It stays on the pane itself.** Moved into an item stacked over the list it took the hover away from every row
    /// under it: hover goes to the topmost item that accepts it, and an item carrying a handler accepts it for its
    /// whole area (measured with qmltestrunner — rows that would not light, cards that would not close).
    property bool pointerInside: false
    HoverHandler {
        onHoveredChanged: graphArea.pointerInside = hovered
    }
    /// Automation: the pointer resting in the pane, which is what puts the lane bar on screen at all
    /// (`PG_AUTO_ACT=graph-bar`).
    function restPointer(inside) { graphArea.pointerInside = inside }
    /// What is drawn and how brightly, not what was asked for: the hooks report the bar itself, so a broken binding
    /// cannot pass (verify-ui).
    readonly property alias laneBarShown: laneBar.visible
    readonly property alias laneBarInk: laneBar.opacity
    // The lanes' horizontal bar (`GraphLaneBar`), on the pane's bottom edge. **Its own `z`, on the bar itself**: a QML
    // stack is the parent's one number, and at the default it would go under the lane strip and the dividers it has to
    // sit on top of.
    GraphLaneBar {
        id: laneBar
        anchors.bottom: parent.bottom
        z: 2
        columns: metrics
        pointerInside: graphArea.pointerInside
    }
    // What stands in the middle while there is no history to draw.
    GraphEmptyState {
        anchors.fill: parent
        graphModel: graphArea.graphModel
        blank: graphArea.blank
        workTree: graphArea.workTree
        onOpenRepositoryRequested: graphArea.openRepositoryRequested()
    }
}
