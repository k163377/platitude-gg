pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

// The history itself: the rows, the viewport arithmetic every mover in the pane goes through (`clampY` /
// `firstVisibleRow` / `rowOnScreen`), where the keyboard goes, and the state the delegates read back off
// `ListView.view` — that state is held here rather than in the delegate, which is recycled the moment its row scrolls
// off. Everything outside reaches it through this component's root, which is the list (`GraphPane.view`).
AppListView {
    id: graphList

    required property var graphModel
    // The uncommitted row's tallies ride on the list for the delegate.
    required property var workTree
    /// The three columns' arithmetic (`GraphColumnMetrics`), which the rows are laid out against.
    required property var columns

    model: graphList.graphModel
    // Nothing but the pane's own functions move the view — the chase is off in `AppListView`, and the arrow keys
    // move it themselves by as little as will do (`revealStep`).
    //
    // Qt's own key navigation moves `currentIndex` and tells nobody: the highlight would walk off screen — the
    // chase above is off — while the panes on the right went on showing the commit it set off from. The arrows are
    // answered by the pane instead, where the page hears about where they landed (規約 §矢印で履歴を辿る).
    keyNavigationEnabled: false
    /// Where the keyboard goes when a press lands in this pane. Every way in comes through here — a row click, a
    /// press on the lanes, the find card closing, the headless hook — so there is one answer to "what does a press
    /// do to the keyboard".
    function takeKeyboard() {
        graphList.forceActiveFocus()
    }
    /// And gives it up when this pane is taken off the screen. Qt leaves active focus on an item it has just made
    /// invisible, and the keys go on arriving there (qmltestrunner で実測 2026-08-11: a StackLayout child swapped away
    /// reports `visible=false activeFocus=true`, and the next Down still fires; `focus = false` is what lets go).
    /// Opening a diff over the graph did exactly that: the arrows walked the selection behind the diff, and moving
    /// the selection closes the diff — so the screen was pulled back to the graph (2026-08-11 ユーザー報告).
    onVisibleChanged: {
        if (!graphList.visible)
            graphList.focus = false
    }
    flickDeceleration: 8000
    maximumFlickVelocity: 9000
    // The graph is the one pane with no header band; this sliver of margin drops the first row so its bottom line
    // meets the neighbouring bands' bottom edge when scrolled to the top.
    topMargin: Theme.headerHeight - Theme.graphRowHeight
    // A sliver of run-out at the end: without it the oldest row sits flush on the pane edge and reads as clipped
    // rather than as the end of what is loaded. Just enough to see the break.
    bottomMargin: Theme.spaceSm
    // Bridge into the delegate (GraphRowDelegate reads its column geometry off ListView.view).
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
    // Which row's chip column is a name box, and what has been typed into it. Held here rather than in the
    // delegate: the delegate is recycled the moment its row scrolls off.
    property string namingOid: ""
    property string namingText: ""
    // Which of the three the box is asking for ("branch" / "tag" / "rename"). The field is the same one for all of
    // them — what is typed is a ref name either way — and the question it stands there holding is the whole of what
    // tells them apart.
    property string namingMode: "branch"
    // What a rename box is naming, in the word the page's `renameRow` branches on ("branch" / "remote" / "tag"), so
    // the frame can say which kind is being typed (規約 §ref の種別: 枠 = 種別). Empty for the two boxes that make a name
    // rather than change one.
    property string namingKind: ""
    // Whether what is in the box can be accepted at all, and the one line that says why not. Decided by the page,
    // which is where the models that answer it are (`RepoPage.namingRefusedWhy`) — the row only draws the answer.
    property bool namingRefused: false
    property string namingRefusedWhy: ""
    // The two clicks the rows answer with one gesture (デザイン規約 §グラフ行のダブルクリック).
    //
    // **One for the whole graph, not one per row — and the card a chip unfolds into shares it** (`RefListPopup` takes
    // it through `GraphPane.rowGesture`). Two reasons, and both are things that go wrong without it:
    //
    // - the delegate is pooled the moment its row scrolls off, so a wait carried by the row is either dropped or
    //   comes back on whatever commit the recycled row is now showing;
    // - **the card opens on the chip's own seat after a rest**, so the reader's two clicks at one spot land on two
    //   different surfaces — the row, then the card. Separate memories make the second one a first click, and the
    //   gesture reads as "sometimes it does nothing" (2026-08-26 ユーザー報告).
    //
    // The key is what the reader is pointing at — the ref's kind and name (`GitFacts.recordKey`), which is the same
    // string on both surfaces and does not change when a background pass rewrites the chip's flags.
    ReclickGesture {
        id: reclick
        onRenameAsked: (key, names) => graphList.rowRenameRequested(names.oid, names.record)
    }
    /// The gesture itself, for the card that stands on these rows. Nothing else reaches past this list for it: what
    /// it holds is one answer to "which target was clicked last", and a second holder would be a second answer.
    readonly property alias rowGesture: reclick
    /// A row was left-clicked: answers whether it is a click of its own (see the gesture), and takes the wait with it.
    /// `record` is the chip's first one, read now rather than when the wait ends — by then the row may be showing
    /// something else, or be another row altogether.
    function noteClick(oidHex, record, held) {
        return reclick.click(record === "" ? "" : GitFacts.recordKey(record),
                             record === "" ? null : { "oid": oidHex, "record": record },
                             held)
    }
    function dropRename() {
        reclick.drop()
    }
    /// Which target the last left click landed on, and the gesture's own state for the runs that photograph it.
    readonly property alias clickedKey: reclick.activeKey
    readonly property alias clickGuarded: reclick.guarded
    function renameArmed(record) {
        return record !== "" && reclick.armedFor(GitFacts.recordKey(record))
    }
    // Which row the standing question is about, and in which tone — held here for the same recycling reason. The
    // words are on the bar; the row only marks itself.
    property string askOid: ""
    property bool askDanger: false
    // Mirrored for the delegates, which can only see the view: rows dim while a search is on. Written by the pane,
    // which is where the find card is.
    property bool findOn: false
    // Which row the working tree stands on, mirrored for the delegates the same way: that row writes its message in
    // the branch's blue, wherever it is read (規約 §グラフの中で HEAD を見失わない).
    readonly property int headRow: graphList.graphModel.headRow
    /// A row was clicked. **The row number travels with the commit**: the page has to place the selection, and
    /// looking a row up is a walk over every loaded one (`RepoPage.activateRow`).
    signal rowSelected(string oidHex, int atRow)
    signal rowMenuRequested(string oidHex)
    signal chipMenuRequested(string oidHex, string record)
    signal rowSwitchRequested(string oidHex, string record)
    /// A row was clicked a second time, late enough that the double-click has been ruled out: the name on its chip is
    /// being changed. `record` is the chip's first one, whatever kind it names.
    signal rowRenameRequested(string oidHex, string record)
    signal chipExpandRequested(string oidHex, var records, var anchor)
    signal chipCollapseRequested()
    signal rowHoverRequested(var row, bool inside)
    property var chipListAnchor: null
    property string rowCardOid: ""
    signal namingSubmitted(string oidHex, string name, string mode)
    signal namingCancelled()
    /// The wheel took the view over: whatever gesture was carrying it ends here.
    signal wheelTaken()
    /// The wheel asked for the lanes sideways. The pane owns how far they may go, so it is given the turn rather
    /// than the destination.
    signal wheelPanned(real delta)
    delegate: GraphRowDelegate {}
    footer: GraphTailFooter {
        width: graphList.width
        graphModel: graphList.graphModel
        labelWidth: graphList.labelWidth
        graphColWidth: graphList.graphColWidth
        graphXOffset: graphList.graphXOffset
        graphFullWidth: graphList.graphFullWidth
    }
    // Manual contentY math must respect originY: after positionViewAtIndex jumps, the ListView shifts its
    // coordinate origin as item positions are fixed up, so [0, contentHeight-height] no longer matches the real
    // scroll range (top rows become unreachable, the bottom overshoots the truncation footer).
    function clampY(y) {
        // topMargin lives above the content origin — forgetting it makes the top gap unreachable by wheel after any
        // scroll.
        const minY = graphList.originY - graphList.topMargin
        const maxY = Math.max(minY, graphList.originY + graphList.contentHeight
                                    - graphList.height + graphList.bottomMargin)
        return Math.max(minY, Math.min(y, maxY))
    }
    /// Topmost row with any of itself on screen; 0 while the view is in its own top margin, where there is no row
    /// to be over.
    function firstVisibleRow() {
        const row = graphList.indexAt(0, graphList.contentY + 1)
        return row >= 0 ? row : 0
    }
    /// Whether all of `row` is on screen. A row below the last one drawn reports no index at all, which is what a
    /// list shorter than its viewport answers for its whole lower half — there, nothing is out of sight.
    function rowOnScreen(row) {
        const bottom = graphList.indexAt(0, graphList.contentY + graphList.height - Theme.graphRowHeight)
        return row >= graphList.firstVisibleRow() && (bottom < 0 || row <= bottom)
    }
    // Mouse wheels scroll a fixed number of rows per notch; touchpads keep native Flickable panning.
    WheelHandler {
        acceptedDevices: PointerDevice.Mouse
        onWheel: event => {
            // Wheel input exits middle-click autoscroll (Chrome-like behavior).
            graphList.wheelTaken()
            graphList.cancelFlick()
            if (event.angleDelta.x !== 0)
                graphList.wheelPanned(event.angleDelta.x)
            const step = (event.angleDelta.y / 120) * Metrics.wheelRows * Theme.graphRowHeight
            graphList.contentY = graphList.clampY(graphList.contentY - step)
        }
    }
}
