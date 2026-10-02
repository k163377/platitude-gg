pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// Center pane: the commit graph.
//
// Every part that could leave has (`GraphList`, `GraphFind`, `AskBar`, `GraphRowWalk`, `GraphColumnMetrics`,
// `GraphColumnDividers`, `GraphLanePan`, `GraphLaneBar`, `GraphHeadPin`, `MiddleAutoScroll`, `GraphEmptyState`).
// What is left is the seats and the page⇄part forwarding, and a further cut adds a level of forwarding and makes
// the whole longer. The seats stay in one file: the list is laid from the question bar and steps down by the find
// card, and the pane-wide `HoverHandler` sits on the pane itself
// (rules-refs/app-ui.md「ペイン全体の hover を測る handler はペイン自身に置く」).
Rectangle {
    id: graphArea

    required property var graphModel
    // The uncommitted row's tallies: the status the list is built from, counted by kind (`status::Kinds`).
    required property var workTree
    // No repository behind this pane: the empty-window call to action.
    property bool blank: false

    /// The commits the page holds chosen, and how many. The page owns the set — it re-resolves it by id after every
    /// background pass (デザイン規約 §複数のコミットを選ぶ).
    property var chosenOids: ({})
    property int chosenCount: 0

    /// A row was picked. `modifiers` tells "read this commit" from "add it to what is held"; what lands a row without
    /// a hand on it (the arrows, a landing) says `Qt.NoModifier`, the plain click.
    signal rowActivated(string oidHex, int atRow, int modifiers)
    /// Right-click anywhere along a row. `chip` is the name its chip draws (`encode::Chip`), null when none — the menu
    /// is the row's either way, and that name is what its cards are about.
    signal rowMenuOpenRequested(string oidHex, var chip)
    /// A row was double-clicked. `chip` is the chip it shows; null when the row shows no branch at all.
    signal rowSwitchRequested(string oidHex, var chip)
    /// Another working copy's uncommitted row was opened (see `GraphList`).
    signal carriedOpenRequested(string path)
    /// The chip whose stacked list the page has out (null when none). Rows read it through the view
    /// (`GraphRowDelegate.listOnThisChip`).
    property var chipListAnchor: null
    /// The commit whose card the page has out (empty when none). Rows read it the same way: the row the card came out
    /// of stays lit under it, though the card took the pointer off it.
    property string rowCardOid: ""
    /// A stacked chip was hovered: unstack it under the chip. The row comes with it — every name in the card is on
    /// that one commit, so a click in the card is a click on that row.
    signal chipExpandRequested(string oidHex, int atRow, var records, var anchor)
    /// A row was clicked a second time, late enough to rule out a double-click: the name on its chip is being changed
    /// (デザイン規約 §グラフ行のダブルクリック). `chip` is the chip's first one, whatever kind it names. **Raised for
    /// the card the chip unfolds into as well** — its rows answer through the four below: a hand clicking one spot
    /// twice clicks the row and then the card, and those are one target.
    signal rowRenameRequested(string oidHex, var chip)
    /// How the graph's rows answer a click, for the card that stands on them (`RowHoverHost`) — one door, so the two
    /// cannot disagree on "the same target".
    function noteRowClick(oidHex, chip) {
        return graphList.noteClick(oidHex, chip)
    }
    function dropRowRename() {
        graphList.dropRename()
    }
    /// A press on something standing over these rows sent the graph somewhere while the hand stayed still — a card's
    /// line (`RowHoverHost.mateFollowed`), the stand-in or its card (`pinPressed`). **The rest of that gesture is not
    /// these rows'** (`ReclickGesture.hush`), **and the row that slid under the still hand opens nothing until the
    /// hand moves** (`GraphList.handHeld`, デザイン規約 §左メニューの所作「手が動いていない所へ来た行は開かない」).
    function settleUnderHand() {
        graphList.hushClicks()
        graphArea.heldAt = paneHand.hovered ? paneHand.point.position : Qt.point(-1, -1)
        graphList.handHeld = true
    }
    /// A press on the current branch's stand-in (`GraphHeadPin`) or on the card its chip unfolds into
    /// (`RefListPopup.standInPressed`): brings the off-screen row on, then picks it with the modifiers the rows read.
    /// The stand-in goes once its row is on screen, so **the row now under the hand is not what the hand was on**
    /// (`settleUnderHand`) — a double-click there would move the working tree.
    function pinPressed(row, modifiers) {
        graphArea.settleUnderHand()
        graphList.takeKeyboard()
        graphArea.jumpToRow(row)
        graphArea.rowActivated(graphArea.graphModel.oidAt(row), row, modifiers)
    }
    readonly property alias rowClicksHushed: graphList.clicksHushed
    readonly property alias rowHandHeld: graphList.handHeld
    function rowRenameArmed(chip) {
        return graphList.renameArmed(chip)
    }
    /// A held click made in that card, put in at the row it stands on: the row's own `leftClick` decides what Ctrl or
    /// Shift does with the choice, the name box and the keyboard — a copy beside the card would leave the keyboard and
    /// the current item behind. False when the row is scrolled off (no delegate to take it).
    function heldRowClick(oidHex, modifiers) {
        const row = graphList.itemAtIndex(graphArea.graphModel.rowOf(oidHex))
        if (!row)
            return false
        row.leftClick(modifiers)
        return true
    }
    readonly property alias rowClickGuarded: graphList.clickGuarded
    /// The pointer settled on a row (or left it): open the commit card under it. `row` is the delegate, for its
    /// position and fields — the page lets it go at once.
    signal rowHoverRequested(var row, bool inside)
    /// The pointer left the chip `anchor` — put its list back, unless it went into the list itself (only the owner can
    /// tell).
    signal chipCollapseRequested(var anchor)
    signal createBranchRequested(string oidHex, string name)
    /// The same box, answered with a tag.
    signal createTagRequested(string oidHex, string name)
    /// The same box, answered with a new name for something that has one. `kind` picks the git command
    /// (`RepoPage.renameRow`).
    signal renameSubmitted(string kind, string id, string name)
    signal openRepositoryRequested()

    // ---- naming a branch or a tag on a row -------------------------
    /// Puts the chip column of one row into a name box: `startNaming` for a branch, `startTagging` for a tag.
    function startNaming(oidHex) {
        graphArea.openNameBox(oidHex, "branch", "", "", "")
    }
    function startTagging(oidHex) {
        graphArea.openNameBox(oidHex, "tag", "", "", "")
    }
    /// The box opens holding the existing name, to be typed over. `kind` is what git renames ("branch" / "remote" /
    /// "tag"), `id` the ref, `text` the name as typed — a remote branch is typed without its remote, so the two differ.
    function startRenaming(oidHex, kind, id, text) {
        graphArea.openNameBox(oidHex, "rename", kind, id, text)
    }
    function openNameBox(oidHex, mode, kind, id, text) {
        graphList.askOid = ""
        // All before `namingOid`: the box comes up already holding its own question, and what it opened holding is
        // what a press elsewhere weighs against (`dropEmptyBoxes`).
        graphList.namingText = text
        graphList.namingMode = mode
        graphList.namingKind = kind
        graphArea.namingId = id
        graphArea.namingOpenedWith = text
        graphList.namingOid = oidHex
    }
    /// A rename is out of this box and git has not answered yet, and what git said about the name still in it — the
    /// pair the left menu's box carries as `SidebarRowGestures.editWaiting` / `editGitRefusal`.
    property bool namingWaiting: false
    property string namingGitRefusal: ""
    // git's refusal is about the name it was asked about; one key later that is no longer the name in the box.
    Connections {
        target: graphList
        function onNamingTextChanged() {
            graphArea.namingGitRefusal = ""
        }
    }
    /// git's answer: the box has done its job, or the refusal belongs in the box.
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
        // **A box coming down spends the gesture that opened it**: the press away from a box lands before its click,
        // and a click that found no box would count as a second one and reopen it (rules-refs/app-ui.md
        // 「箱が立っている間の押下は press の側で先に効く」).
        if (graphList.namingOid !== "")
            graphList.forgetClicks()
        graphList.namingOid = ""
        graphList.namingText = ""
        graphList.namingKind = ""
        graphArea.namingId = ""
        graphArea.namingOpenedWith = ""
    }
    /// The ref a rename box is changing the name of, and the name it came up holding. Empty for the two
    /// boxes that make a name.
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
    /// The card itself — automation-only exposure, like `view`. A run types into its box, but the key that opens it
    /// cannot be pressed from there.
    readonly property alias findCard: findBar
    /// How far the graph has stepped down out from under the card — the list's number, not the card's.
    readonly property real findShift: graphList.anchors.topMargin
    signal findLanded(string oidHex)
    function startFind() { findBar.startFind() }
    function findNext() { findBar.findNext() }
    function findPrevious() { findBar.findPrevious() }

    // ---- a press that landed somewhere else -------------------------
    /// The find card and a row's name box go with a press that landed elsewhere (`RepoPage.releasePressedAway`,
    /// `scenePos` = where it landed) while empty; typing in one keeps it.
    function dropEmptyBoxes(scenePos) {
        findBar.dropIfEmpty(scenePos)
        // "Empty" is "nothing put in it": no text for a box that makes a name, the name it opened holding for a rename.
        if (graphList.namingOid !== "" && graphList.namingText === graphArea.namingOpenedWith)
            graphArea.stopNaming()
    }
    /// Whether all of `row` is on screen.
    function rowOnScreen(row) { return graphList.rowOnScreen(row) }

    // ---- a standing question ---------------------------------------
    // The bar comes down from the top of the pane and pushes the history down (デザイン規約 §可否・警告の出し場所); the row it
    // concerns is marked, so the question is written exactly once.

    /// Raises the bar. `oidHex` is the row it is about ("" for none; a row outside the loaded window goes unmarked —
    /// the bar stands either way). `hold` takes the answer as a held press (デザイン規約 §進行中・長押しの定数) and
    /// `tip` is the pill's hover text. `form` is what the question needs to take an answer at all (a chooser, a name
    /// box). `code` is the git command, said at the head of the question and on the pill where the act has one word of
    /// its own (デザイン規約 §git 用語のコード表記); `accept` carries the wording otherwise. `refName` is the ref the
    /// question names, and **`label` then comes with the name's seat still in it (`%1`)**: the name is drawn in its own
    /// colour, so the sentence is cut at the seat (`AskBar.labelSentence`).
    function startAsking(oidHex, label, detail, accept, danger, hold = false, tip = "", form = null, code = "",
                         refName = "") {
        graphArea.stopNaming()
        // Down before the new one goes in, or the last question's typing comes back under these words (`AskBar.form`).
        askBar.form = null
        askBar.form = form
        askBar.answerable = true
        askBar.neutral = false
        askBar.alert = false
        askBar.plainWords = false
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
    /// Whether it is asking for information, how it is answered, and what it says while it stands — these change
    /// under a publish question as the remote answers what the typed name means.
    property alias askNeutral: askBar.neutral
    property alias askHold: askBar.hold
    /// The word on the pill. **It may move while the question stands, and the gesture moves with it**
    /// (デザイン規約 §はじめてリモートへ送る: `push` becomes `push -f`, the click a hold). A publish question leaves it
    /// empty for `askCode`; one answered from a chooser writes the picked answer's word here (`RenameCarryFlow`).
    property alias askAccept: askBar.accept
    /// The command the pill answers with, and whether the far side could be read at all — both move under a publish
    /// question as the remote answers, because what would run depends on what is over there.
    property alias askCode: askBar.code
    property alias askAlert: askBar.alert
    /// Whether the heading is in the plain ink — for the one question whose colour follows what is picked in it
    /// (`AskBar.plainWords`).
    property alias askPlainWords: askBar.plainWords
    property alias askDetail: askBar.detail
    property alias askTip: askBar.tip
    /// The live form, so its owner can read what was typed into it.
    readonly property alias askForm: askBar.formItem
    /// The bar itself — automation-only exposure, like `view`.
    readonly property alias askCard: askBar
    /// Whether a question is standing in this pane — **product-facing**, unlike the bar above: the report bar over the
    /// middle of the page reads it to know Escape is not its own (`RepoPage`).
    readonly property alias asking: askBar.open
    function stopAsking() {
        // Only lowered: the bar stays on screen for its whole exit, and the next question is what re-dresses it
        // (`AskBar.open`).
        askBar.open = false
        graphList.askOid = ""
    }
    /// Automation: answer a held question the way a person does, by keeping the pill down to the end.
    function completeHold() {
        askBar.completeHold()
    }
    signal askConfirmed()
    signal askCancelled()

    /// The list itself — automation-only exposure (rules-refs/app-ui.md「製品の部品はハーネスへ答える相手を丸ごと渡す」);
    /// app code goes through the functions.
    readonly property alias view: graphList

    /// Moves the selection without scrolling.
    function setCurrentRow(row) {
        graphList.currentIndex = row
    }
    /// The arrows are this pane's again, asked for from outside once what stood over the graph is put away
    /// (`RepoPage.closeDiffToGraph`) — through the list's own door, like every press inside the pane.
    function takeKeyboard() {
        graphList.takeKeyboard()
    }

    // ---- walking the history with the arrow keys, and where the view
    // stands for it (規約 §矢印で履歴を辿る) ---------------------------
    GraphRowWalk {
        id: rowWalk
        view: graphList
        graphModel: graphArea.graphModel
        asking: askBar.open
        onActivated: (oidHex, atRow) => graphArea.rowActivated(oidHex, atRow, Qt.NoModifier)
    }
    function stepRow(delta, held) { return rowWalk.stepRow(delta, held) }
    function menuFromKeys() { return rowWalk.menuFromKeys() }
    function stepLanding(row, wasY) { return rowWalk.stepLanding(row, wasY) }
    /// Automation only: whether a step is still waiting to be read (`GraphRowWalk.settling`).
    readonly property alias stepSettling: rowWalk.settling
    /// Automation only: whether the view still has a move owed to it (`GraphRowWalk.placing`).
    readonly property alias placing: rowWalk.placing
    function jumpToRow(row) { rowWalk.jumpToRow(row) }
    function anchorSoon() { rowWalk.anchorSoon() }
    function shiftRows(rows) { rowWalk.shiftRows(rows) }
    function showRowSoon(row) { rowWalk.showRowSoon(row) }

    // The three columns' widths and the lanes' sideways offset, aliased below so everyone reads them off this pane.
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
    // The list starts under the question — the graph moves down; a closed bar has no height. **The report bar is not
    // here**: a write can be answered while a file is being read, so it stands over the whole middle of the page
    // (`RepoPage`, デザイン規約 §答えの要らない報せ).
    AskBar {
        id: askBar
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        z: 3
        onConfirmed: graphArea.askConfirmed()
        onCancelled: graphArea.askCancelled()
    }
    // Hangs from the top-right corner, over the list — declared here because a Flickable adopts what is declared in
    // it and scrolls it away. Only one of the two is ever up: a question already standing keeps the place, since it
    // is one gesture from being over.
    GraphFind {
        id: findBar
        anchors.top: parent.top
        anchors.right: parent.right
        anchors.rightMargin: Theme.spaceLg
        z: 4
        graphModel: graphArea.graphModel
        view: graphList
        asking: askBar.open
        // Reaches left to `spaceXs` past where a subject starts (規約 §コミットを探す) — measured from the columns, so
        // the cap follows the dividers when they are dragged.
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
        // When the newest commit is itself a match, the graph steps down by the card's height so the search does not
        // cover it, and back once the top row stops matching (規約 §コミットを探す).
        anchors.topMargin: findBar.findClears ? findBar.height : 0
        Behavior on anchors.topMargin {
            NumberAnimation { duration: 200 }
        }
        graphModel: graphArea.graphModel
        workTree: graphArea.workTree
        columns: metrics
        findOn: findBar.findOn
        chosenOids: graphArea.chosenOids
        chosenCount: graphArea.chosenCount
        chipListAnchor: graphArea.chipListAnchor
        rowCardOid: graphArea.rowCardOid
        // The arrows are answered here, where the history is walked (規約 §矢印で履歴を辿る). `isAutoRepeat` is the only
        // thing that tells a run apart from a press (`GraphRowWalk.noteStep`).
        Keys.onUpPressed: event => event.accepted = graphArea.stepRow(-1, event.isAutoRepeat)
        Keys.onDownPressed: event => event.accepted = graphArea.stepRow(1, event.isAutoRepeat)
        // The menu key; Windows' Shift+F10 comes through the window (`Main.keyMenuAsked`).
        Keys.onMenuPressed: event => event.accepted = graphArea.menuFromKeys()
        onRowMenuRequested: (oidHex, chip) => graphArea.rowMenuOpenRequested(oidHex, chip)
        onRowSelected: (oidHex, atRow, modifiers) => graphArea.rowActivated(oidHex, atRow, modifiers)
        onRowSwitchRequested: (oidHex, chip) => graphArea.rowSwitchRequested(oidHex, chip)
        onCarriedOpenRequested: path => graphArea.carriedOpenRequested(path)
        onRowRenameRequested: (oidHex, chip) => graphArea.rowRenameRequested(oidHex, chip)
        onChipExpandRequested: (oidHex, atRow, records, anchor) =>
            graphArea.chipExpandRequested(oidHex, atRow, records, anchor)
        onChipCollapseRequested: anchor => graphArea.chipCollapseRequested(anchor)
        onRowHoverRequested: (row, inside) => graphArea.rowHoverRequested(row, inside)
        onNamingSubmitted: (oidHex, name, mode) => {
            const kind = graphList.namingKind
            const id = graphArea.namingId
            const was = graphArea.namingOpenedWith
            // **A rename keeps its box until git answers**, as the left menu's does (`SidebarRowGestures.submitEdit`,
            // デザイン規約 §答えの要らない報せ).
            if (mode === "rename" && name !== "" && name !== was) {
                graphArea.namingWaiting = true
                graphArea.renameSubmitted(kind, id, name)
                return
            }
            graphArea.stopNaming()
            // An empty box is the way out of the offer, and so is a rename reaching here (it holds its opening name).
            if (name === "" || mode === "rename")
                return
            if (mode === "tag")
                graphArea.createTagRequested(oidHex, name)
            else
                graphArea.createBranchRequested(oidHex, name)
        }
        onNamingCancelled: graphArea.stopNaming()
        onWheelPanned: delta => graphArea.graphX = Math.max(0,
            Math.min(graphArea.graphX - delta / 2, graphArea.graphXMax))
    }
    /// Whether a middle-click autoscroll is under way, and whether it carries the lanes sideways as well (automation).
    readonly property alias autoScrolling: autoScroll.scrolling
    readonly property alias autoPanning: autoScroll.panning
    /// Starts autoscroll from a point in this pane's frame, and moves the pointer of one already under way — for the
    /// presses and the automation hook alike.
    function startAutoScroll(x, y) {
        // A wheel notch still in flight would pull against the drift for its last beat (`WheelGlide.halt`).
        graphList.haltGlide()
        autoScroll.start(x, y)
    }
    function driftPointer(x, y) {
        autoScroll.drift(x, y)
    }
    /// The middle button came up. **Which exit that is was decided while it was down** (`MiddleAutoScroll.letGo`).
    function letGoAutoScroll() {
        autoScroll.letGo()
    }
    /// Whether the hand left the dead zone before letting go, and how many times the drift has been asked for a
    /// distance — automation only.
    readonly property alias autoTravelled: autoScroll.travelled
    readonly property alias autoTicks: autoScroll.ticks
    MiddleAutoScroll {
        id: autoScroll
        anchors.fill: parent
        // **Over the rows**: at the default z the anchor mark goes behind the history and shows only past the last
        // row. At the lane strip's z and below the dividers, as the stand-in is.
        z: 1
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
    /// The lane strip — automation-only exposure, like `view`: the one way in for a run's press on the lane column.
    readonly property alias lanePan: lanePan
    // The current branch's stand-in, riding whichever edge its own row went out of. **Over the lane strip and under the
    // dividers**: below the strip, its own lanes would answer with the row scrolling underneath; the dividers stay on
    // top, a draggable boundary being only a few pixels wide.
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
        // **The modifiers come with it**: the stand-in is the row (デザイン規約 §複数のコミットを選ぶ) — dropped, a held
        // press would put the choice back down to one commit.
        onActivated: (row, modifiers) => graphArea.pinPressed(row, modifiers)
        // Its chip unfolds through the rows' own door: the card is the one the page holds for every chip.
        onChipExpandRequested: (oidHex, atRow, records, anchor) =>
            graphArea.chipExpandRequested(oidHex, atRow, records, anchor)
        onChipCollapseRequested: anchor => graphArea.chipCollapseRequested(anchor)
    }
    /// The stand-in itself — automation-only exposure, like `view`.
    readonly property alias headPin: headPin

    // Draggable column dividers (labels | graph | message). The hand is in there; the two lines it raises are drawn
    // above, under the list. Over the lane pan, under the lane bar.
    GraphColumnDividers {
        id: columnDividers
        anchors.fill: parent
        z: 2
        columns: metrics
        blank: graphArea.blank
    }
    /// Whether either divider is refusing, and where the hand is while it does (**scene coordinates**) — the page draws
    /// the badge, since a drag carries the hand out past this pane.
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
    /// What is drawn: the automation hook reports the line and the badge themselves, so a column that cannot be
    /// resized but still promises a drag cannot pass.
    readonly property alias graphDividerShown: columnDividers.graphDividerShown
    readonly property alias graphDividerLineShown: graphDividerLine.visible
    /// Whether the badge the page draws for this pane is up — the pane's half of that answer.
    readonly property alias graphDividerRefuses: columnDividers.refused
    /// The line of whichever divider has the hand. A refused drag has to leave it drawn — the boundary still moves the
    /// other way — and only the pane knows which of the two lines is being asked about.
    readonly property bool refusedLineShown: columnDividers.labelDragging ? labelDividerLine.visible
        : graphDividerLine.visible
    /// Whether the pointer is anywhere in this pane; real hover and the automation hook write it alike. A passive
    /// `HoverHandler`, so the rows', chips' and dividers' own hover does not take it away. **It stays on the pane
    /// itself**: on an item stacked over the list it takes the hover from every row under it (rules-refs/app-ui.md
    /// 「ペイン全体の hover を測る handler はペイン自身に置く」).
    property bool pointerInside: false
    HoverHandler {
        id: paneHand
        onHoveredChanged: {
            graphArea.pointerInside = paneHand.hovered
            if (!paneHand.hovered)
                graphList.handHeld = false
        }
        // **The hold lasts until the hand itself moves** (`settleUnderHand`). With no place to take at the press (the
        // hand was in a card, and the card going brings it back over the rows), the first point heard is the place;
        // rows moving under it arrive at that same place, so they do not count as a move.
        onPointChanged: {
            if (!graphList.handHeld)
                return
            const at = paneHand.point.position
            if (graphArea.heldAt.x < 0) {
                graphArea.heldAt = at
                return
            }
            if (at.x !== graphArea.heldAt.x || at.y !== graphArea.heldAt.y)
                graphList.handHeld = false
        }
    }
    /// Where the hand was when a press sent the rows moving under it (`settleUnderHand`), in this pane's own
    /// coordinates; (-1, -1) until it is heard.
    property point heldAt: Qt.point(-1, -1)
    /// Automation: the pointer resting in the pane, which is what puts the lane bar on screen at all
    /// (`PGG_AUTO_ACT=graph-bar`).
    function restPointer(inside) { graphArea.pointerInside = inside }
    /// What is drawn and how brightly: the hooks report the bar itself, so a broken binding cannot pass.
    readonly property alias laneBarShown: laneBar.visible
    readonly property alias laneBarInk: laneBar.opacity
    // The lanes' horizontal bar, on the pane's bottom edge. **Its own `z`**: at the default it would go under the lane
    // strip and the dividers it has to sit on top of.
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
