pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
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
    /// Stage or unstage the lines picked by hand — all of them, in one
    /// write. The owner reads them back with `chosenPairs()`.
    signal stageChosenRequested()

    // ---- the lines picked by hand ----------------------------------
    // A click on a changed line takes it or puts it back; the heading of
    // the hunk it belongs to then names the count instead of the hunk
    // (デザイン規約 §diff の中のステージ). Held here rather than on the
    // rows because a delegate is recycled the moment its line scrolls off,
    // and keyed `<hunk>:<line>` because that pair is what a patch is
    // addressed by.
    property var chosenLines: ({})
    property int chosenCount: 0
    function lineChosen(hunk, line) {
        return diffPane.chosenLines[hunk + ":" + line] === true
    }
    /// How many lines of one hunk are picked — what its heading says.
    function chosenIn(hunk) {
        const head = hunk + ":"
        let n = 0
        for (const key in diffPane.chosenLines)
            if (key.indexOf(head) === 0)
                n++
        return n
    }
    function toggleLine(hunk, line) {
        // A fresh object every time: the rows follow this property, and
        // assigning the same one back changes nothing to follow.
        const key = hunk + ":" + line
        const next = ({})
        for (const k in diffPane.chosenLines)
            next[k] = true
        if (next[key] === true)
            delete next[key]
        else
            next[key] = true
        diffPane.chosenLines = next
        diffPane.chosenCount = Object.keys(next).length
    }
    function clearLines() {
        diffPane.chosenLines = ({})
        diffPane.chosenCount = 0
    }
    /// The picked lines as `[hunk, line]` pairs, in the order a patch
    /// wants them — for the owner to hand to the bridge.
    function chosenPairs() {
        const out = []
        for (const key in diffPane.chosenLines) {
            const cut = key.indexOf(":")
            out.push([parseInt(key.substring(0, cut)),
                      parseInt(key.substring(cut + 1))])
        }
        out.sort((a, b) => a[0] === b[0] ? a[1] - b[1] : a[0] - b[0])
        return out
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
    /// Automation: pick the first `count` changed lines of a hunk, the way
    /// a click on each of them would.
    function chooseLines(hunk, count) {
        diffPane.clearLines()
        let taken = 0
        for (let i = 0; i < diffList.count && taken < count; i++) {
            const row = diffList.itemAtIndex(i)
            if (row && row.hunk === hunk
                    && (row.kind === "add" || row.kind === "del")) {
                diffPane.toggleLine(hunk, row.line)
                taken++
            }
        }
        return taken
    }

    // ---- the view's place in a diff that is about to be rebuilt -------
    // A partial write ends by reading the file again, and the answer
    // arrives as a whole new list. Without this the view would come back
    // at the top, which on a long diff loses the place being worked
    // through — the same restore `GraphPane.shiftRows` does after a graph
    // swap, and delayed for the same reason (`contentHeight` is still the
    // old one on the frame the rows land).
    property real heldY: -1
    function holdScroll() {
        diffPane.heldY = diffList.contentY
    }
    function restoreScroll() {
        if (diffPane.heldY >= 0)
            placeTimer.restart()
    }
    // The wait is the graph's (`Metrics.anchorDelayMs`, and `shiftRows`
    // learned it the same way): on the frame the rows land the list has
    // not laid them out yet, so `contentHeight` is still the old one and
    // the clamp below would take the view to the top instead of back to
    // its place.
    Timer {
        id: placeTimer
        interval: Metrics.anchorDelayMs
        onTriggered: {
            const want = diffPane.heldY
            diffPane.heldY = -1
            diffList.contentY = Math.max(0, Math.min(want, diffList.contentHeight
                                                     - diffList.height))
            if (AppBackend.autoAct !== "")
                AppBackend.report("diff_place " + Math.round(diffList.contentY))
        }
    }
    /// Automation: read the view away from the top, so that a rebuild can
    /// be seen to put it back where it was.
    function scrollTo(y) {
        diffList.contentY = y
    }

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
        // -- a conflict with only one side left: git has two versions of
        //    the path but not the third to compare them against, so it
        //    prints no patch at all (`* Unmerged path`). The one thing
        //    worth saying is what the two sides each did — and this pane
        //    is the one place that sentence is shown (デザイン規約
        //    §conflict の種別). Which way out to take is still the file
        //    row's right-click, unchanged.
        Label {
            visible: diffPane.diffModel.unmerged
            Layout.margins: Theme.spaceSm
            Layout.fillWidth: true
            elide: Text.ElideRight
            text: Words.conflict(diffPane.conflictChange, diffPane.sideOurs,
                                 diffPane.sideTheirs)
            color: Theme.textMuted
        }
        // -- which branch each of the two colours is. The colours are the
        //    ones the graph already gives those branches, so this line is
        //    the whole of what has to be learned; it is only here when
        //    there are two colours to bind names to (`sidesTold`), since
        //    otherwise it would explain a distinction the rows are not
        //    making. The names swap over during a rebase and the model has
        //    already sorted that out (デザイン規約 §conflict の ours /
        //    theirs), so this says whatever it is handed.
        ConflictSideLegend {
            visible: diffPane.sidesTold
            Layout.leftMargin: Theme.spaceSm
            Layout.rightMargin: Theme.spaceSm
            Layout.topMargin: Theme.spaceXs
            Layout.bottomMargin: Theme.spaceXs
            Layout.fillWidth: true
            oursName: Words.ourSide(diffPane.sideOurs)
            theirsName: Words.theirSide(diffPane.sideTheirs)
            oursColor: diffPane.sideColor("ours")
            theirsColor: diffPane.sideColor("theirs")
            nameCap: diffPane.width / 3
        }
        // -- line endings: one line for the file, never a mark per row.
        //    A CR is invisible and has nowhere inside a line to sit, and
        //    the mixed case is already saying how many lines it is about.
        //    It does not ask anything and does not hold anything up —
        //    `warning` because it is a change that carries past this
        //    machine, not because something is wrong here
        //    (デザイン規約 §状態の 3 段).
        Label {
            visible: diffPane.diffModel.endingKind !== ""
            // The binary notice's seat, down to the margins: both are one
            // line about the file rather than about anything in it.
            Layout.margins: Theme.spaceSm
            Layout.fillWidth: true
            elide: Text.ElideRight
            text: Words.lineEndings(diffPane.diffModel.endingKind,
                                    diffPane.diffModel.endingFrom,
                                    diffPane.diffModel.endingTo,
                                    diffPane.diffModel.endingLines,
                                    diffPane.diffModel.endingScope,
                                    diffPane.diffModel.endingExt)
            font.pixelSize: Theme.fontSm
            color: Theme.warning
        }
        // -- content preview: binaries summarized by size, images
        //    rendered (added = After only, deleted = Before only,
        //    modified = both).
        BinarySizeLine {
            visible: diffPane.diffModel.previewKind === "binary"
                     || (diffPane.diffModel.isBinary
                         && diffPane.diffModel.previewKind === "")
            Layout.margins: Theme.spaceSm
            Layout.fillWidth: true
            oldSize: diffPane.diffModel.previewOldSize
            newSize: diffPane.diffModel.previewNewSize
        }
        // -- a diff whose body is empty. git prints headers and no hunks
        //    for a rename that changed nothing, for a mode-only change and
        //    for an empty file added, and a pane that answers all three
        //    with a blank frame reads as one that failed to load. The
        //    binary notice's seat and voice: one line about the file
        //    rather than about anything in it.
        //
        //    Said the same way for all three rather than naming the
        //    rename: what the pane knows is that there is nothing to
        //    show, and git is not asked a second question to find out why.
        Label {
            visible: diffList.count === 0 && !diffPane.diffModel.loading
                     && !diffPane.diffModel.isBinary
                     && !diffPane.diffModel.unmerged
                     && diffPane.diffModel.previewKind === ""
                     && diffPane.diffModel.title !== ""
            Layout.margins: Theme.spaceSm
            Layout.fillWidth: true
            elide: Text.ElideRight
            text: qsTr("No changes to show")
            color: Theme.textMuted
        }
        RowLayout {
            visible: diffPane.diffModel.previewKind === "image"
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: Theme.spaceSm
            spacing: Theme.spaceSm
            ImagePreviewCell {
                Layout.fillWidth: true
                Layout.fillHeight: true
                caption: qsTr("Before")
                url: diffPane.diffModel.previewOldUrl
                sizeText: diffPane.diffModel.previewOldSize
            }
            ImagePreviewCell {
                Layout.fillWidth: true
                Layout.fillHeight: true
                caption: qsTr("After")
                url: diffPane.diffModel.previewNewUrl
                sizeText: diffPane.diffModel.previewNewSize
            }
        }
        ListView {
            id: diffList
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: diffPane.diffModel
            reuseItems: true
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: AutoScrollBar {}
            // The rows the write asked for have landed: put the view back
            // where it was reading. The empty half of the swap is not it —
            // a reset shows up here as a count of zero first.
            onCountChanged: if (count > 0) diffPane.restoreScroll()
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
                    const step = (event.angleDelta.y / 120)
                               * Metrics.wheelRows * Theme.rowHeight
                    diffList.contentY = diffList.clampY(
                        diffList.contentY - step)
                }
            }
            delegate: DiffRowDelegate {
                id: diffRow
                rowWidth: diffList.width
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
                picked: diffPane.partial
                        && diffPane.lineChosen(diffRow.hunk, diffRow.line)
                        && (diffRow.kind === "add" || diffRow.kind === "del")
                pickedInHunk: diffPane.partial && diffRow.kind === "hunk"
                              ? diffPane.chosenIn(diffRow.hunk) : 0
                onHunkPointedAt: (inside, hunk) => {
                    if (inside) {
                        diffPane.hoverHunk = hunk
                        diffPane.hoverLine = -1
                    } else if (diffPane.hoverHunk === hunk) {
                        diffPane.hoverHunk = -1
                    }
                }
                onLineToggleRequested: (hunk, line) =>
                    diffPane.toggleLine(hunk, line)
                onChoiceCleared: diffPane.clearLines()
                onDiscardRequested: hunk => diffPane.discardHunkRequested(hunk)
                onStageHunkRequested: hunk =>
                    diffPane.stageSelectionRequested(hunk, -1)
                onStageChosenRequested: diffPane.stageChosenRequested()
            }
        }
    }
}
