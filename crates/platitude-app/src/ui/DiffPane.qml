pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Center pane, diff mode: one file's unified diff with per-file,
// per-hunk and per-line staging affordances, plus image / binary
// previews.
Rectangle {
    id: diffPane

    required property var diffModel
    // Whether the shown diff is a working-tree file (stageable).
    property bool fromWorkTree: false
    // Whether it is the staged side (flips the affordance wording).
    property bool staged: false
    /// Whether the file is one git stopped on. `git add` still runs, but
    /// what it means there is "this is dealt with", not "this goes in the
    /// next commit", and the words follow that.
    property bool conflicted: false
    /// The two stage letters git reports for this file (`UU`, `DU`, …) and
    /// what each side is called — only a conflicted one has them. What
    /// they are for: on the conflicts git prints no patch for, they are
    /// the whole of what the pane can say.
    property string conflictChange: ""
    property string sideOurs: ""
    property string sideTheirs: ""
    /// The colour each side is drawn in, as an index into
    /// `Theme.graphLane` — the graph's own lane colour for that branch
    /// where it has one, and never the same on both sides
    /// (`encode::conflict_side_colors` decides both).
    property int sideColorOurs: -1
    property int sideColorTheirs: -1
    // A write is running: staging buttons disable.
    property bool busy: false
    /// Whether this diff has pieces worth naming. A file the repository is
    /// seeing for the first time has none: its one hunk is the whole file,
    /// so `Stage hunk` would be the header's `Stage file` said a second
    /// time in a smaller voice, and the `+` on every row would be that
    /// same word again once per line (デザイン規約 §diff の中のステージ).
    /// What stays is the one word in the header.
    ///
    /// A conflicted file has none either, for a different reason: its diff
    /// compares the working tree against **both** sides at once, and that
    /// shape is not a patch — `git apply` refuses it, so there is no such
    /// thing as staging a part of it (core refuses too, under
    /// `stage::refuse_combined`). The one word that stays there is
    /// `Mark resolved`.
    readonly property bool partial: diffPane.fromWorkTree
                                    && !diffPane.diffModel.isNewFile
                                    && !diffPane.combined

    // ---- a conflicted file's diff -----------------------------------
    /// This diff has more than one old side, so every row carries a marker
    /// column per side (`platitude_core::parse::diff`).
    readonly property bool combined: diffPane.diffModel.isCombined
    /// Which side a combined row's line came from, read off its markers:
    /// a column holds a space where that side has the line. A line both
    /// sides have is context, one neither has is a marker git wrote (or a
    /// line typed while resolving), and both answer "".
    function sideOf(markers) {
        if (markers.length < 2 || markers.indexOf("-") >= 0)
            return ""
        const inOurs = markers.charAt(0) === " "
        const inTheirs = markers.charAt(1) === " "
        if (inOurs === inTheirs)
            return ""
        return inOurs ? "ours" : "theirs"
    }
    /// Whether the two sides are being told apart by colour. Always, on a
    /// conflicted file: the graph does not always have two colours to lend
    /// (a branch outside the walk's window has none, and the palette
    /// cycles, so two chains far enough apart share one), and where it
    /// falls short the pair is completed rather than dropped
    /// (`encode::conflict_side_colors`).
    readonly property bool sidesTold: diffPane.combined
                                      && diffPane.sideColorOurs >= 0
                                      && diffPane.sideColorTheirs >= 0
    function sideColor(side) {
        const index = side === "ours" ? diffPane.sideColorOurs
                                      : diffPane.sideColorTheirs
        return Theme.graphLane[index % Theme.graphLane.length]
    }

    signal closeRequested()
    /// The side being read has nothing left in it — everything that was
    /// here has been staged (or unstaged, or thrown away), so what the
    /// pane is standing on is gone.
    signal nothingLeft()
    /// Stage or unstage the whole file (direction follows `staged`).
    signal stageFileRequested()
    /// Stage or unstage one hunk (line < 0) or one line of it.
    signal stageSelectionRequested(int hunk, int line)

    /// A press on one line's mark, which is the only thing in a row that
    /// takes one (デザイン規約 §diff の中のステージ). Named so the
    /// automation can enter where the mark enters — a press on a square
    /// that only exists under a pointer cannot be injected (verify-ui).
    function stageLine(hunk, line) {
        diffPane.stageSelectionRequested(hunk, line)
    }
    /// Where the pointer is, and whether it is in this pane at all.
    ///
    /// **The rows do not answer for themselves.** Every write rebuilds the
    /// list under the hand, and a freshly built item is not hovered until
    /// the mouse moves again — Qt delivers hover on movement — so the mark
    /// stayed away after a press and the next line could not be staged
    /// without waggling the mouse first (2026-08-17 ユーザー報告). The
    /// pane works out which row the pointer is over instead, and writes
    /// the same pair of properties the automation writes.
    ///
    /// **On the pane, not on an overlay**: a handler laid over the rows
    /// takes their hover away entirely (2026-08-17 実測).
    HoverHandler {
        id: panePointer
        onPointChanged: diffPane.settlePointedRow()
        // Leaving takes the mark with it. Said here rather than in
        // `settlePointedRow`, which a headless run must not reach: there
        // the pointer never arrives and never leaves, and the row the
        // automation named has to stand (verify-ui).
        onHoveredChanged: {
            if (panePointer.hovered)
                diffPane.settlePointedRow()
            else
                diffPane.showLineTools(-1, -1)
        }
    }
    /// Names the row the pointer is over, or nothing where it is over none
    /// of them. A heading is named as itself (line -1), which is what
    /// lights its whole hunk.
    function settlePointedRow() {
        if (!panePointer.hovered)
            return
        const at = diffPane.mapToItem(diffList, panePointer.point.position.x,
                                      panePointer.point.position.y)
        const row = at.y >= 0 && at.y <= diffList.height
                  ? diffList.itemAt(at.x + diffList.contentX,
                                    at.y + diffList.contentY)
                  : null
        if (!row) {
            diffPane.showLineTools(-1, -1)
            return
        }
        diffPane.showLineTools(row.hunk, row.kind === "hunk" ? -1 : row.line)
    }
    // The rows the pointer is over have just been replaced. The wait is
    // the one the scroll restore takes, and for the same reason: on the
    // frame the rows land the list has not laid them out, and nothing is
    // under the pointer yet (`DiffScrollPlace`).
    Timer {
        id: pointedRowTimer
        interval: Metrics.anchorDelayMs
        onTriggered: diffPane.settlePointedRow()
    }
    // ---- the keyboard -----------------------------------------------
    /// Where the keyboard goes when this pane comes on screen. Unlike the
    /// graph, which waits to be clicked because a window has several
    /// places worth typing into, the diff arrives *because* a hand pressed
    /// a file in CHANGES — that press already said "read here", so the
    /// pane takes the keyboard by arriving (デザイン規約 §diff を上下に送る).
    ///
    /// Refused to a list that is not on screen: an image-only preview
    /// hands its space to the picture and draws no rows. Focus on
    /// something invisible is the hole the graph closed from the other
    /// side — Qt keeps active focus there and the keys go on arriving.
    function takeKeyboard() {
        if (diffList.visible)
            diffList.forceActiveFocus()
    }
    /// Sends the view `delta` rows (∓1 per press) and answers whether it
    /// moved. The keys and the automation hook both come through here — a
    /// headless run cannot inject a keystroke, so the step has to be
    /// callable as well as pressable (verify-ui).
    ///
    /// The view is what moves, not a selection: nothing in this pane
    /// follows a lit row, and the "selection" it does own is the set of
    /// lines the next write carries, which the arrows must not touch
    /// (デザイン規約 §diff を上下に送る). Answering `false` at either end
    /// is how it stops rather than wraps — the key goes unaccepted there.
    function stepRows(delta) {
        if (!diffPane.visible || !diffList.visible || diffList.count === 0)
            return false
        const was = diffList.contentY
        diffList.cancelFlick()
        diffList.contentY = diffList.clampY(was + delta * Theme.rowHeight)
        return diffList.contentY !== was
    }
    /// Whether the view is as far down as it goes — the end the arrows
    /// stop at, which a picture of a diff cannot be told from a short one.
    /// A diff with nothing to scroll reads as at its end, because it is.
    readonly property bool atEnd: diffList.contentY >= diffList.maxY - 0.5
    /// Automation only: the list itself, for a run that has to read where
    /// the view stands (`GraphPane.view` is the same exposure).
    readonly property alias view: diffList

    // Taken on the way in, let go on the way out. The second half is the
    // rule the graph is already keeping (規約 §矢印で履歴を辿る「画面から
    // 退いたペインはキーボードを手放す」): a pane swapped off the screen
    // that keeps focus goes on answering arrows nobody can see.
    onVisibleChanged: {
        if (diffPane.visible)
            diffPane.takeKeyboard()
        else
            diffList.focus = false
    }

    /// Throw one hunk away. Offered on the unstaged side only — the
    /// staged side unstages first. No question comes before it: the
    /// heading's own button is held down (デザイン規約 §その他の操作).
    signal discardHunkRequested(int hunk)

    /// Automation: hold the heading's discard button to its end. The
    /// hunk is the first one, which is the one every smoke run acts on.
    function completeHold() {
        const row = diffList.itemAtIndex(0)
        if (row && row.discardButton)
            row.discardButton.completeHold()
    }

    // ---- automation: the tools a line only shows under the pointer ----
    // Hover cannot be injected on Windows (verify-ui skill), so the squares a
    // line puts out — stage this line, throw it away — have no headless
    // way to be seen. These name a row as though the pointer were on it,
    // the way `ref-list` enters where the hover timer would. Nothing in
    // the app writes them: the pointer is the only other way in.
    property int hoverHunk: -1
    property int hoverLine: -1
    function showLineTools(hunk, line) {
        diffPane.hoverHunk = hunk
        diffPane.hoverLine = line
    }
    /// Automation: how far sideways the code stands and how far it may go,
    /// whether the bar is out, and the two ways of moving it — a middle
    /// button cannot be injected any more than a hover can (verify-ui).
    readonly property alias codeAt: codeScroll.offset
    readonly property alias codeMax: codeScroll.maxOffset
    readonly property alias codeBarShown: codeScroll.barShown
    readonly property alias codeHandOn: codeScroll.handScrolling
    function sendCode(dx) { codeScroll.shift(dx) }
    function startCodeHand(x, y) { codeScroll.startHand(x, y) }
    function driftCodeHand(x, y) { codeScroll.driftHand(x, y) }

    // ---- the view's place in a diff that is about to be rebuilt -------
    DiffScrollPlace {
        id: scrollPlace
        view: diffList
    }
    /// Automation: where the rebuild put the view back, and -1 until it
    /// has put it anywhere.
    readonly property real placeLandedY: scrollPlace.landedY
    function holdScroll() { scrollPlace.hold() }
    function restoreScroll() { scrollPlace.restore() }
    /// Automation: read the view away from the top, so that a rebuild can
    /// be seen to put it back where it was.
    function scrollTo(y) { scrollPlace.scrollTo(y) }

    // ---- the side that ran out --------------------------------------
    // A write empties the diff it was made in as soon as the last of it
    // goes over to the other side, and an empty frame with a live
    // `Stage file` in it is a pane standing on nothing. Only a rebuild
    // asks the question — opening a file that has nothing to show is a
    // different story and not one a close would explain.
    property bool closeWhenEmpty: false
    Connections {
        target: diffPane.diffModel
        // Sent once the swap is complete, so the count is the new one.
        function onChanged() {
            if (!diffPane.closeWhenEmpty || diffPane.diffModel.loading)
                return
            diffPane.closeWhenEmpty = false
            // A picture is content even where there are no rows to count.
            if (diffList.count === 0 && !diffPane.diffModel.isBinary
                    && diffPane.diffModel.previewKind === "")
                diffPane.nothingLeft()
        }
    }
    /// The first line of a hunk that a partial write can act on. A hunk's
    /// lines are numbered through the context it carries, so line 0 is
    /// usually a line that is not part of the change at all — selecting
    /// one builds a patch with nothing in it, which core refuses ("the
    /// selected part is no longer in its diff"). A pointer never has this
    /// problem: it picks its line by being over it, and the squares only
    /// come out on the lines that changed. -1 when the hunk has none.
    function firstChangedLine(hunk) {
        for (let i = 0; i < diffList.count; i++) {
            const row = diffList.itemAtIndex(i)
            if (row && row.hunk === hunk
                    && (row.kind === "add" || row.kind === "del"))
                return row.line
        }
        return -1
    }

    /// Automation: whether the read this pane was asked for is over and
    /// came back with something to look at. `firstChangedLine` is the
    /// answer the line-level verbs want — it is only true once the rows
    /// exist *and* the list has built them, which is what naming a row
    /// needs — but a verb that only opens a file has pictures and binary
    /// files to allow for, and those have no rows at all.
    function diffSettled() {
        const m = diffPane.diffModel
        return !m.loading && (diffList.count > 0 || m.isBinary
                              || m.previewKind !== "")
    }

    /// How wide a line number is: the widest one this diff carries
    /// (デザイン規約 §レイアウト初期値).
    ///
    /// The columns are cut to it rather than fixed, so the row reads
    /// `gap 140 gap 153 gap }`. A fixed column leaves the slack of the
    /// numbers it is *not* holding to the left of the one it is, and
    /// that slack lands between the two numbers while the code — which
    /// has none — sits one gap away: the same distance came out several
    /// times over in one row (2026-08-13 ユーザー指示).
    ///
    /// Measured with a Label that is never drawn, the way `ActionButton`
    /// and `TopBar` measure: `TextMetrics` reports a few pixels tighter
    /// than the Label the number is actually set in. Whole pixels — two
    /// columns and the code's run are laid out from this one number.
    readonly property int numberW: Math.ceil(numberMeasure.implicitWidth)
    Label {
        id: numberMeasure
        visible: false
        text: diffPane.diffModel.widestNo
        font.family: Theme.monoFamily
        font.pixelSize: Theme.fontSm
    }

    // ---- how far sideways the code goes ------------------------------
    /// The room between the two numbers where a changed line puts its mark
    /// out, and the whole gutter that room sits in. Worked out once here
    /// rather than per row: the rows lay themselves out from it, and the
    /// pane subtracts it to know how much of a line is on screen.
    readonly property int seatW: diffPane.partial
                                 ? Theme.iconMd + 2 * Theme.borderWidth
                                 : Theme.spaceXs
    readonly property int gutterW: 2 * (Theme.spaceXs + diffPane.numberW)
                                   + diffPane.seatW
    /// How wide the longest line of this diff is drawn. The model counts
    /// the columns (`encode::widest_columns`); one measured character is
    /// what turns them into pixels, and the font is mono so the one
    /// character speaks for all of them. Measured with a Label that is
    /// never drawn, the way the numbers above are — a metric read off a
    /// method would be taken once, before this Label's own font arrived
    /// (app-ui.md).
    readonly property real codeW:
        diffPane.diffModel.widestColumns * charMeasure.implicitWidth / 10
        + Theme.spaceSm
    Label {
        id: charMeasure
        visible: false
        // Ten of them, so the fraction a single advance rounds to does not
        // multiply up over a line of two hundred columns.
        text: "0000000000"
        font.family: Theme.monoFamily
        font.pixelSize: Theme.fontCode
    }

    color: Theme.bgSurface
    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        DiffPaneHeader {
            Layout.fillWidth: true
            title: diffPane.diffModel.title
            fromWorkTree: diffPane.fromWorkTree
            staged: diffPane.staged
            conflicted: diffPane.conflicted
            busy: diffPane.busy
            onStageFileRequested: diffPane.stageFileRequested()
            onCloseRequested: diffPane.closeRequested()
        }
        // No question bar here: the only thing this pane throws away is a
        // hunk, and that is held down on the hunk's own heading
        // (デザイン規約 §その他の操作).
        // What this pane has to say about the file rather than about any
        // line in it — and the picture that stands in for one no rows can
        // show.
        DiffFileNotices {
            Layout.fillWidth: true
            Layout.fillHeight: diffPane.diffModel.previewKind === "image"
            diffModel: diffPane.diffModel
            rowCount: diffList.count
            conflictChange: diffPane.conflictChange
            sideOurs: diffPane.sideOurs
            sideTheirs: diffPane.sideTheirs
            sidesTold: diffPane.sidesTold
            oursColor: diffPane.sideColor("ours")
            theirsColor: diffPane.sideColor("theirs")
            nameCap: diffPane.width / 3
        }
        AppListView {
            id: diffList
            Layout.fillWidth: true
            Layout.fillHeight: true
            model: diffPane.diffModel
            // The rows the write asked for have landed: put the view back
            // where it was reading, and work out again which of the new
            // rows the pointer is over. The empty half of the swap is not
            // it — a reset shows up here as a count of zero first.
            onCountChanged: {
                if (count > 0) {
                    diffPane.restoreScroll()
                    pointedRowTimer.restart()
                }
            }
            // A row moving under a pointer that is standing still is the
            // same question as the pointer moving over the rows.
            onContentYChanged: diffPane.settlePointedRow()
            // An image with no text rows hands its space to the preview
            // (SVG edits keep both).
            visible: diffPane.diffModel.previewKind !== "image"
                     || count > 0
            /// How far down the view can go, and the clamp both hands that
            /// send it share — the wheel by the notch, the arrows by the
            /// row. One surface moves within one set of bounds
            /// (デザイン規約 §diff を上下に送る); two expressions of it
            /// drift apart the moment one of them is fixed.
            readonly property real maxY: Math.max(0, contentHeight - height)
            function clampY(y) {
                return Math.max(0, Math.min(y, diffList.maxY))
            }
            // Qt's own key navigation moves `currentIndex` and tells
            // nobody. Nothing in this pane follows a current row, so it
            // would scroll the view to a selection that means nothing;
            // the arrows are answered below instead, where they move the
            // view itself (規約 §diff を上下に送る).
            keyNavigationEnabled: false
            Keys.onUpPressed: event => event.accepted = diffPane.stepRows(-1)
            Keys.onDownPressed: event => event.accepted = diffPane.stepRows(1)
            // Mouse wheels scroll a fixed number of rows per notch — the
            // same Metrics.wheelRows every other surface answers a notch
            // with (GraphPane, the right panes). This list was the one
            // place still on Flickable's default wheel, a pseudo-flick
            // that eases in and moves less per notch, which read as
            // sluggish next to the rest of the app (2026-08-11 ユーザー
            // 報告). Touchpads keep native Flickable panning.
            WheelHandler {
                acceptedDevices: PointerDevice.Mouse
                onWheel: event => {
                    diffList.cancelFlick()
                    // The wheel's own sideways component — a tilt wheel, a
                    // touchpad — says where it wants to go in the input
                    // itself, so it is answered wherever the pointer is
                    // (デザイン規約 §グラフを横へ送る, same rule).
                    if (event.angleDelta.x !== 0)
                        codeScroll.shift(-event.angleDelta.x / 2)
                    const step = (event.angleDelta.y / 120)
                               * Metrics.wheelRows * Theme.rowHeight
                    diffList.contentY = diffList.clampY(
                        diffList.contentY - step)
                }
            }
            delegate: DiffRowDelegate {
                id: diffRow
                rowWidth: diffList.width
                codeX: codeScroll.offset
                seatW: diffPane.seatW
                side: diffPane.sideOf(diffRow.markers)
                partial: diffPane.partial
                staged: diffPane.staged
                busy: diffPane.busy
                numberW: diffPane.numberW
                sidesTold: diffPane.sidesTold
                oursColor: diffPane.sideColor("ours")
                theirsColor: diffPane.sideColor("theirs")
                hoverHunk: diffPane.hoverHunk
                hoverLine: diffPane.hoverLine
                onLineStageRequested: (hunk, line) =>
                    diffPane.stageLine(hunk, line)
                onDiscardRequested: hunk => diffPane.discardHunkRequested(hunk)
                onStageHunkRequested: hunk =>
                    diffPane.stageSelectionRequested(hunk, -1)
            }
        }
    }
    // The hand that sends the rows sideways and up and down, and the bar
    // that says how far there is to go. Declared after the body, so it
    // stands over the rows — and beside the list rather than inside it,
    // for the two measured reasons in `DiffCodeScroll`.
    DiffCodeScroll {
        id: codeScroll
        anchors.fill: parent
        view: diffList
        paneHovered: panePointer.hovered
        file: diffPane.diffModel.title
        codeWidth: diffPane.codeW
        roomWidth: Math.max(0, diffList.width - diffPane.gutterW)
    }
}
