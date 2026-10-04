pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

// The history list: the rows, the viewport arithmetic every mover in the pane goes through (`clampY` from
// `AppListView`, `firstVisibleRow`, `rowOnScreen`), where the keyboard goes, and the per-row state the delegates read
// off `ListView.view` — held here because a delegate is recycled the moment its row scrolls off. Outside reaches it as
// `GraphPane.view`.
AppListView {
    id: graphList

    required property var graphModel
    required property var workTree
    /// The three columns' arithmetic (`GraphColumnMetrics`).
    required property var columns

    model: graphList.graphModel
    // The middle button is the pane's, whose hand also pans the lanes sideways (`GraphPane`).
    ownsHand: false
    // The arrows are the pane's (`GraphRowWalk.stepRow`, 規約 §矢印で履歴を辿る): Qt's own key navigation moves
    // `currentIndex` and tells nobody, so the highlight would walk off screen (the chase is off in `AppListView`)
    // while the right panes kept showing the old commit.
    keyNavigationEnabled: false
    /// Where the keyboard goes when a press lands in this pane; every way in (row, lanes, find card closing, headless
    /// hook) comes through here.
    function takeKeyboard() {
        graphList.forceActiveFocus()
    }
    /// Gives the keyboard up when the pane leaves the screen: Qt keeps active focus on an item it made invisible and
    /// keys keep arriving (only `focus = false` lets go) — with a diff open over the graph, the arrows would walk the
    /// selection behind it and so close the diff.
    onVisibleChanged: {
        if (!graphList.visible)
            graphList.focus = false
    }
    flickDeceleration: 8000
    maximumFlickVelocity: 9000
    // No header band here: this sliver drops the first row so its bottom line meets the neighbouring bands' bottom
    // edge when scrolled to the top.
    topMargin: Theme.headerHeight - Theme.graphRowHeight
    // A run-out so the oldest row does not read as clipped, plus the lane bar's strip while the lanes overflow, so the
    // last row ends above the bar.
    bottomMargin: Theme.spaceSm + (graphList.columns.graphXMax > 0 ? graphList.columns.laneBarRoom : 0)
    property real labelWidth: graphList.columns.labelW
    property real graphColWidth: graphList.columns.graphColW
    property real graphFullWidth: graphList.columns.graphFullW
    property real graphXOffset: graphList.columns.graphX
    property int wipAdded: graphList.workTree.wipAdded
    property int wipModified: graphList.workTree.wipModified
    property int wipDeleted: graphList.workTree.wipDeleted
    property int wipRenamed: graphList.workTree.wipRenamed
    property int wipCopied: graphList.workTree.wipCopied
    property int wipConflicted: graphList.workTree.conflictCount
    // Which row's chip column is a name box, and what has been typed into it.
    property string namingOid: ""
    property string namingText: ""
    // Which of the four the box is asking for ("branch" / "tag" / "worktree" / "rename").
    property string namingMode: "branch"
    // What a rename box is naming ("branch" / "remote" / "tag", as `renameRow` takes it), so the frame says the kind
    // (規約 §ref の種別: 枠 = 種別); empty for the three boxes that make a name.
    property string namingKind: ""
    // Whether the box's text can be accepted, and why not — decided by the page (`RepoPage.graphNameRefusedWhy`) —
    // and the working copy's folder in that line, which its tip stands the tree mark in front of.
    property bool namingRefused: false
    property string namingRefusedWhy: ""
    property string namingRefusedMark: ""
    // The two clicks the rows answer with one gesture (デザイン規約 §グラフ行のダブルクリック). One for the whole graph,
    // shared with the chip's card (via `GraphPane.noteRowClick`): a pooled delegate would drop or misplace the wait,
    // and the card opens on the chip's seat, so a second click landing on it must still count as the second.
    // Keyed by the ref's kind and name (`encode::Chip::key`) — the same on both surfaces and stable across repaints.
    ReclickGesture {
        id: reclick
        onRenameAsked: (key, names) => graphList.rowRenameRequested(names.oid, names.chip)
    }
    /// A row was left-pressed: answers whether it is a press of its own (`ReclickGesture.click`). `chip` is the chip's
    /// first ref, read now (the row may show something else when the wait ends); null where the row draws no ref.
    function noteClick(oidHex, chip) {
        // While a box stands, a click only takes it down (`RepoPage.activateRow`); arming a rename here would reopen it
        // a beat later. The key still counts, so the next click is an ordinary second one.
        const nameable = chip !== null && graphList.namingOid === ""
        return reclick.click(chip === null ? "" : chip.key,
                             nameable ? { "oid": oidHex, "chip": chip } : null)
    }
    function dropRename() {
        reclick.drop()
    }
    /// The rest of a gesture begun on something over these rows (a card, the stand-in) lands here
    /// (`ReclickGesture.hush`).
    function hushClicks() {
        reclick.hush()
    }
    readonly property alias clicksHushed: reclick.hushed
    /// The rows moved under a still hand; nothing opens under it until it moves (`GraphPane.settleUnderHand`).
    property bool handHeld: false
    /// The box was taken down: the gesture is spent, so the click that took it down is no second click
    /// (`GraphPane.stopNaming`).
    function forgetClicks() {
        reclick.forget()
    }
    /// For the runs: which target the last left click landed on, and the gesture's state.
    readonly property alias clickedKey: reclick.activeKey
    readonly property alias clickGuarded: reclick.guarded
    /// A second click is waiting out the double-click window; meanwhile the rows open and close nothing
    /// (`GraphRowDelegate.settlePointed`).
    readonly property alias renameWaiting: reclick.armed
    function renameArmed(chip) {
        return chip !== null && reclick.armedFor(chip.key)
    }
    // Which row the standing question is about, and in which tone; the words are on the bar.
    property string askOid: ""
    property bool askDanger: false
    // Rows dim while a search is on; written by the pane, which owns the find card.
    property bool findOn: false
    // The row the working tree stands on writes its message in the branch's blue (規約 §グラフの中で HEAD を見失わない).
    readonly property int headRow: graphList.graphModel.headRow
    /// The commits the page is holding, as a set of ids, and their count (デザイン規約 §複数のコミットを選ぶ). Empty while
    /// the working tree's row is shown — it is no commit and never joins a choice.
    property var chosenOids: ({})
    property int chosenCount: 0
    /// A row was clicked, with the modifiers held and the row it sits on (`RepoPage.activateRow`).
    signal rowSelected(string oidHex, int atRow, int modifiers)
    /// A row was right-clicked. `chip` is the name its chip draws (`encode::Chip`), null where it draws none
    /// (デザイン規約 §グラフ行の右クリック).
    signal rowMenuRequested(string oidHex, var chip)
    signal rowSwitchRequested(string oidHex, var chip)
    /// Another working copy's uncommitted row was opened: the repository's tab moves onto that copy
    /// (`tabs::strip::landing_for`).
    signal carriedOpenRequested(string path)
    /// A second click, late enough to rule out a double-click: rename the chip's first ref, whatever its kind.
    signal rowRenameRequested(string oidHex, var chip)
    signal chipExpandRequested(string oidHex, int atRow, var records, var anchor)
    /// `anchor` is the chip the pointer left (`GraphRowDelegate.settlePointed`).
    signal chipCollapseRequested(var anchor)
    signal rowHoverRequested(var row, bool inside)
    property var chipListAnchor: null
    property string rowCardOid: ""
    signal namingSubmitted(string oidHex, string name, string mode)
    signal namingCancelled()
    /// The wheel asked for the lanes sideways; the pane owns how far they may go.
    signal wheelPanned(real delta)
    delegate: GraphRowDelegate {}
    footer: GraphTailFooter {
        width: graphList.width
        graphModel: graphList.graphModel
        labelWidth: graphList.labelWidth
        graphColWidth: graphList.graphColWidth
        graphXOffset: graphList.graphXOffset
        graphFullWidth: graphList.graphFullWidth
        findOn: graphList.findOn
    }
    /// Topmost row with any of itself on screen; 0 while the view is in its own top margin.
    function firstVisibleRow() {
        const row = graphList.indexAt(0, graphList.contentY + 1)
        return row >= 0 ? row : 0
    }
    /// Whether all of `row` is on screen. No index at the bottom means a list shorter than its viewport, where nothing
    /// is out of sight.
    function rowOnScreen(row) {
        const bottom = graphList.indexAt(0, graphList.contentY + graphList.height - Theme.graphRowHeight)
        return row >= graphList.firstVisibleRow() && (bottom < 0 || row <= bottom)
    }
    /// One notch, sent — also a run's way in, since a wheel cannot be injected (verify-ui スキル).
    function sendRows(pixels) {
        wheelGlide.sendTo(graphList.clampY(wheelGlide.at - pixels))
    }
    /// Something else is moving the view: the notch in flight stops (`WheelGlide.halt`), or it drags the view back.
    function haltGlide() {
        wheelGlide.halt()
    }
    WheelGlide {
        id: wheelGlide
        view: graphList
    }
    // The bar's arrows and track glide their steps on this same glide (`AutoScrollBar.stepGlide`).
    Binding {
        target: graphList.ScrollBar.vertical
        property: "stepGlide"
        value: wheelGlide
    }
    // Mouse wheels scroll a fixed number of rows per notch; touchpads keep native Flickable panning.
    WheelHandler {
        acceptedDevices: PointerDevice.Mouse
        onWheel: event => {
            graphList.cancelFlick()
            if (event.angleDelta.x !== 0)
                graphList.wheelPanned(event.angleDelta.x)
            graphList.sendRows((event.angleDelta.y / 120) * Metrics.wheelRows * Theme.graphRowHeight)
        }
    }
}
