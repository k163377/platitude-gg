pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Center pane, diff mode: one file's unified diff with per-file, per-hunk and per-line staging affordances, plus
// image / binary previews.
Rectangle {
    id: diffPane

    required property var diffModel
    // Whether the shown diff is a working-tree file (stageable).
    property bool fromWorkTree: false
    // Whether it is the staged side (flips the affordance wording).
    property bool staged: false
    /// Whether the file is one git stopped on. `git add` still runs, but what it means there is "this is dealt with",
    /// not "this goes in the next commit", and the words follow that.
    property bool conflicted: false
    /// The two stage letters git reports for this file (`UU`, `DU`, …) and what each side is called — only a conflicted
    /// one has them. On the conflicts git prints no patch for, they are the whole of what the pane can say.
    property string conflictChange: ""
    property string sideOurs: ""
    property string sideTheirs: ""
    /// The colour each side is drawn in, as an index into `Theme.graphLane` — the graph's own lane colour for that
    /// branch where it has one, and never the same on both sides (`encode::conflict_side_colors` decides both).
    property int sideColorOurs: -1
    property int sideColorTheirs: -1
    // A write is running: staging buttons disable.
    property bool busy: false
    /// Whether this diff has pieces worth naming. A file the repository is seeing for the first time has none: its one
    /// hunk is the whole file, so `Stage hunk` would be the header's `Stage file` said again in a smaller voice
    /// (デザイン規約 §diff の中のステージ). A conflicted file has none either, for a different reason: its diff compares the
    /// working tree against **both** sides at once, and that shape is not a patch — `git apply` refuses it, and so does
    /// core (`stage::refuse_combined`). What stays in either case is the one word in the header.
    readonly property bool partial: diffPane.fromWorkTree && !diffPane.diffModel.isNewFile && !diffPane.combined

    // ---- a conflicted file's diff -----------------------------------
    /// This diff has more than one old side, so every row carries a marker
    /// column per side (`platitude_core::parse::diff`).
    readonly property bool combined: diffPane.diffModel.isCombined
    /// Whether the two sides are being told apart by colour. Always, on a conflicted file: the graph does not always
    /// have two colours to lend (a branch outside the walk's window has none, and the palette cycles), and where it
    /// falls short the pair is completed rather than dropped (`encode::conflict_side_colors`).
    readonly property bool sidesTold: diffPane.combined && diffPane.sideColorOurs >= 0 && diffPane.sideColorTheirs >= 0
    function sideColor(side) {
        const index = side === "ours" ? diffPane.sideColorOurs : diffPane.sideColorTheirs
        return Theme.graphLane[index % Theme.graphLane.length]
    }
    /// How many rows each side is named on, for the automation (`conflict-sides`). The bands are a few pixels wide and
    /// a picture cannot be asked whether the ones that should be there are. The rows are the model's, so the count is
    /// too (`DiffModel.sideCount`).
    function sideTally() {
        const ours = diffPane.diffModel.sideCount("ours")
        const theirs = diffPane.diffModel.sideCount("theirs")
        return "combined=" + diffPane.combined + " told=" + diffPane.sidesTold
             + " both=" + (ours > 0 && theirs > 0) + " ours=" + ours + " theirs=" + theirs
    }

    /// One of the page's right-click menus is standing, so nothing behind it is being hovered — the marks a row puts
    /// out under the pointer would be drawn over the very rows the menu is about (デザイン規約 §メニュー).
    property bool menuStanding: false

    signal closeRequested()
    /// Stage or unstage the whole file (direction follows `staged`).
    signal stageFileRequested()
    /// Stage or unstage one hunk (line < 0) or one line of it.
    signal stageSelectionRequested(int hunk, int line)
    /// Text the reader picked out of the rows, on its way to the clipboard (the page owns it).
    signal copyRequested(string text)
    /// A right-click landed on the text. What the menu will act on is already the selection — the hand settled that
    /// before asking (`DiffTextSelect.askMenu`).
    signal codeMenuRequested()

    /// A press on one line's mark, the only thing in a row that takes one (デザイン規約 §diff の中のステージ). Named so the
    /// automation can enter where the mark does — a press on a square that only exists under a pointer cannot be
    /// injected (verify-ui).
    function stageLine(hunk, line) {
        diffPane.stageSelectionRequested(hunk, line)
    }
    /// Where the pointer is, and whether it is in this pane at all.
    ///
    /// **The rows do not answer for themselves.** Every write rebuilds the list under the hand, and a freshly built
    /// item is not hovered until the mouse moves again (Qt delivers hover on movement), so the mark would stay away
    /// after a press and the next line could not be staged without waggling the mouse first. The pane works out which
    /// row the pointer is over instead, and writes the same pair of properties the automation writes.
    ///
    /// **On the pane, not on an overlay**: a handler laid over the rows takes their hover away entirely.
    HoverHandler {
        id: panePointer
        onPointChanged: diffPane.settlePointedRow()
        // Leaving takes the mark with it. Said here rather than in `settlePointedRow`, which a headless run must not
        // reach: there the pointer never arrives and never leaves, and the row the automation named has to stand.
        onHoveredChanged: {
            if (panePointer.hovered)
                diffPane.settlePointedRow()
            else
                diffPane.showLineTools(-1, -1)
        }
    }
    // Every press in this pane, whatever it was for: the hand is here now, so the arrows are (規約 §diff のファイル一覧). A
    // `PointHandler` because it is the one handler specified to take only passive grabs — the buttons, the bars and the
    // rows' own marks all keep working underneath (`Main.qml`'s window watcher is one for the same reason). On the pane
    // rather than an overlay: a handler laid over the rows takes their hover away.
    PointHandler {
        acceptedButtons: Qt.AllButtons
        onActiveChanged: {
            if (active)
                diffPane.handArrived()
        }
    }
    /// Names the row the pointer is over, or nothing where it is over none of them. A heading is named as itself (line
    /// -1), which is what lights its whole hunk.
    function settlePointedRow() {
        // A menu is what the pointer is over now; the row it was opened on is behind it.
        if (diffPane.menuStanding) {
            diffPane.showLineTools(-1, -1)
            return
        }
        if (!panePointer.hovered)
            return
        const at = diffPane.mapToItem(diffList, panePointer.point.position.x, panePointer.point.position.y)
        const row = at.y >= 0 && at.y <= diffList.height
                  ? diffList.itemAt(at.x + diffList.contentX, at.y + diffList.contentY)
                  : null
        if (!row) {
            diffPane.showLineTools(-1, -1)
            return
        }
        diffPane.showLineTools(row.hunk, row.kind === "hunk" ? -1 : row.line)
    }
    // A menu standing takes the marks away there and then: the pointer has not moved, so nothing else would ask again.
    onMenuStandingChanged: diffPane.settlePointedRow()
    // The rows the pointer is over have just been replaced. The wait is the one the scroll restore takes, and for the
    // same reason: on the frame the rows land the list has not laid them out (`DiffScrollPlace`).
    Timer {
        id: pointedRowTimer
        interval: Metrics.anchorDelayMs
        onTriggered: diffPane.settlePointedRow()
    }
    // The keyboard and where the reading position stands both live in `DiffRowWalk`; the three forwards below keep the
    // page, the keys and the headless runs reading them off this pane.
    DiffRowWalk {
        id: rowWalk
        view: diffList
    }
    function handArrived() { rowWalk.handArrived() }
    function stepRows(delta) { return rowWalk.stepRows(delta) }
    readonly property alias atEnd: rowWalk.atEnd
    /// Automation only: the list itself, for a run that reads where the view stands (as `GraphPane.view` is).
    readonly property alias view: diffList

    /// Throw one hunk away. Offered on the unstaged side only — the staged side unstages first. No question comes
    /// before it: the heading's own button is held down (デザイン規約 §その他の操作).
    signal discardHunkRequested(int hunk)

    /// Automation: hold the heading's discard button to its end, on the first hunk — the one every smoke run acts on.
    function completeHold() {
        const row = diffList.itemAtIndex(0)
        if (row && row.discardButton)
            row.discardButton.completeHold()
    }

    // ---- automation: the tools a line only shows under the pointer ----
    // Hover cannot be injected on Windows (verify-ui skill), so the squares a line puts out — stage this line, throw it
    // away — have no headless way to be seen. These name a row as though the pointer were on it, the way `ref-list`
    // enters where the hover timer would. Nothing in the app writes them.
    property int hoverHunk: -1
    property int hoverLine: -1
    function showLineTools(hunk, line) {
        diffPane.hoverHunk = hunk
        diffPane.hoverLine = line
    }
    /// Automation: how far sideways the code stands and how far it may go, whether the bar is out, and the two ways of
    /// moving it — a middle button cannot be injected any more than a hover can (verify-ui).
    readonly property alias codeAt: codeScroll.offset
    readonly property alias codeMax: codeScroll.maxOffset
    readonly property alias codeBarShown: codeScroll.barShown
    readonly property alias codeHandOn: codeScroll.handScrolling
    function sendCode(dx) { codeScroll.shift(dx) }
    function startCodeHand(x, y) { codeScroll.startHand(x, y) }
    function driftCodeHand(x, y) { codeScroll.driftHand(x, y) }

    // ---- the text the reader picked out -------------------------------
    /// What the plain `Copy` and Ctrl+C both put on the clipboard: the unchanged and added lines the selection covers
    /// (デザイン規約 §diff の中身をコピーする). Nothing goes out for a selection that holds nothing, and saying so is what
    /// lets the key fall through to whoever else wants it.
    function copySelection() {
        const text = diffPane.diffModel.selectionText()
        if (text === "")
            return false
        diffPane.copyRequested(text)
        return true
    }
    /// Automation: the hand that picks text, without a pointer behind it (verify-ui). It enters the same three
    /// functions the `MouseArea`'s own handlers call, so a run cannot pass while the handlers do something else.
    function pickText(fromRow, fromAt, toRow, toAt) {
        textPick.pressText(fromRow, fromAt)
        textPick.dragText(toRow, toAt)
        textPick.releaseText()
    }
    /// Automation: the hand itself, for the runs that go in through the pixels rather than through a row number — the
    /// ground under the last row is a place only a coordinate can name (`PG_AUTO_ACT=diff-sweep`). Handed over whole,
    /// the way the view is: four forwards here would be four more lines of this file saying nothing.
    readonly property alias textHand: textPick
    /// Automation: the band's own hand — the one the path at the top is dragged over from the air beside it. Handed
    /// over whole for the reason `textHand` is (`PG_AUTO_ACT=diff-band-sweep`).
    readonly property alias headerHand: paneHeader.pad
    /// Automation: whether that band ran out of room for the path — the half a picture of a wide pane cannot answer.
    readonly property alias headerCut: paneHeader.cut
    /// Automation: the right-click, at a place in the text — which is where the menu's answer is settled.
    function askCodeMenu(row, at) { textPick.askMenu(row, at) }
    /// Automation: the first row of the view that is a removed line, or -1 — the one row the menu's second word is
    /// about, and a drag that never reaches one proves half the rule.
    function firstRemovedRow() {
        for (let i = 0; i < diffList.count; i++) {
            const row = diffList.itemAtIndex(i)
            if (row && row.kind === "del")
                return i
        }
        return -1
    }
    /// Automation: what a row is wearing of the selection, and what the two menu rows would take.
    function pickTally() {
        let washed = 0
        for (let i = 0; i < diffList.count; i++) {
            const row = diffList.itemAtIndex(i)
            if (row && row.sel !== "")
                washed++
        }
        return "new=" + diffPane.diffModel.selHasNew
             + " removed=" + diffPane.diffModel.selRemoved
             + " washed=" + washed
    }

    // ---- the view's place in a diff that is about to be rebuilt -------
    DiffScrollPlace {
        id: scrollPlace
        view: diffList
    }
    /// Automation: where the rebuild put the view back, and -1 until it has put it anywhere.
    readonly property real placeLandedY: scrollPlace.landedY
    function holdScroll() { scrollPlace.hold() }
    function restoreScroll() { scrollPlace.restore() }
    function dropScroll() { scrollPlace.drop() }
    /// Automation: read the view away from the top, so that a rebuild can be seen to put it back where it was.
    function scrollTo(y) { scrollPlace.scrollTo(y) }

    /// The first line of a hunk that a partial write can act on, or -1. A hunk's lines are numbered through the context
    /// it carries, so line 0 is usually a line that is not part of the change at all — selecting one builds a patch
    /// with nothing in it, which core refuses ("the selected part is no longer in its diff"). A pointer never has this
    /// problem: it picks its line by being over it, and the squares only come out on the lines that changed.
    function firstChangedLine(hunk) {
        for (let i = 0; i < diffList.count; i++) {
            const row = diffList.itemAtIndex(i)
            if (row && row.hunk === hunk && (row.kind === "add" || row.kind === "del"))
                return row.line
        }
        return -1
    }

    /// Automation: whether the read this pane was asked for is over and came back with something to look at. The
    /// line-level verbs want `firstChangedLine` instead — only that is true once the rows exist *and* the list has
    /// built them — but a verb that only opens a file has pictures and binary files to allow for, and those have no
    /// rows at all.
    function diffSettled() {
        const m = diffPane.diffModel
        return !m.loading && (diffList.count > 0 || m.isBinary || m.previewKind !== "")
    }

    /// How wide a line number is: the widest one this diff carries (デザイン規約 §レイアウト初期値). The columns are cut to it
    /// rather than fixed, so the row reads `gap 140 gap 153 gap }`; a fixed column leaves the slack of the numbers it
    /// is *not* holding between the two numbers, while the code — which has none — sits one gap away.
    ///
    /// Measured with a Label that is never drawn, the way `ActionButton` and `TopBar` measure: `TextMetrics` reports a
    /// few pixels tighter than the Label the number is actually set in. Whole pixels — two columns and the code's run
    /// are laid out from this one number.
    readonly property int numberW: Math.ceil(numberMeasure.implicitWidth)
    Label {
        id: numberMeasure
        visible: false
        text: diffPane.diffModel.widestNo
        font.family: Theme.monoFamily
        font.pixelSize: Theme.fontSm
    }

    // ---- how far sideways the code goes ------------------------------
    /// The room between the two numbers where a changed line puts its mark out, and the whole gutter that room sits in.
    /// Worked out once here: the rows lay themselves out from it, and the pane subtracts it to know how much shows.
    readonly property int seatW: diffPane.partial ? Theme.iconMd + 2 * Theme.borderWidth : Theme.spaceXs
    readonly property int gutterW: 2 * (Theme.spaceXs + diffPane.numberW) + diffPane.seatW
    /// How wide the longest line of this diff is drawn. The model counts the columns (`encode::widest_columns`); one
    /// measured character turns them into pixels, and the font is mono so that one character speaks for all of them.
    /// Measured with a never-drawn Label: a metric read off a method is taken once, before the font arrives (app-ui.md).
    readonly property real codeW: diffPane.diffModel.widestColumns * diffPane.charW + Theme.spaceSm
    /// One measured column of the mono font. The divisor is however many characters `charMeasure` holds, so the two
    /// cannot drift apart; everything column-addressed — the code width above, the emphasis wash in the rows —
    /// multiplies this one number. Measured at regular weight: changed rows draw bold, which JetBrains Mono advances
    /// identically — a mono family whose bold face advances differently would drift the wash, so a swap of
    /// `Theme.monoFamily` re-checks that.
    readonly property real charW: charMeasure.implicitWidth / charMeasure.text.length
    Label {
        id: charMeasure
        visible: false
        // Ten of them, so a single advance's rounding does not multiply up over a two-hundred-column line.
        text: "0000000000"
        font.family: Theme.monoFamily
        font.pixelSize: Theme.fontCode
    }
    /// What a wide glyph costs on top of the two columns `encode::columns::step_of` counts it as. Zero wherever the
    /// mono family carries the wide glyphs itself at two of its own advances; where it is Latin-only they arrive from a
    /// fallback that advances one em instead (Windows, Cascadia Mono at `fontCode`: charW 8px against a wide advance of
    /// 13px, so −3px a glyph, which slid the wash that far right of the characters it names). Measured rather than
    /// assumed, because it is a property of whichever fallback this OS hands the glyphs to.
    readonly property real wideDelta: wideMeasure.implicitWidth / wideMeasure.text.length - 2 * diffPane.charW
    Label {
        id: wideMeasure
        visible: false
        // Ten U+65E5, for the same reason charMeasure holds ten. Built from the code point rather than written as the
        // glyph: this is a ruler and not a word, and a line of Japanese sitting in a `text:` reads like the hardcoded
        // wording the rules forbid (CLAUDE.md 絶対制約).
        text: String.fromCharCode(0x65e5).repeat(10)
        font.family: Theme.monoFamily
        font.pixelSize: Theme.fontCode
    }

    color: Theme.bgSurface
    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        DiffPaneHeader {
            id: paneHeader
            Layout.fillWidth: true
            title: diffPane.diffModel.title
            fromWorkTree: diffPane.fromWorkTree
            staged: diffPane.staged
            conflicted: diffPane.conflicted
            busy: diffPane.busy
            onStageFileRequested: diffPane.stageFileRequested()
            onCloseRequested: diffPane.closeRequested()
        }
        // No question bar here: the only thing this pane throws away is a hunk, and that is held down on the hunk's own
        // heading (デザイン規約 §その他の操作). What follows is what the pane has to say about the file rather than about
        // any line in it — and the picture that stands in for one no rows can show.
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
            // The strip the sideways bar stands on, below the last row rather than over it (`DiffCodeScroll.barRoom`).
            // A file with nowhere sideways to go has no bar and gets the strip back.
            Layout.bottomMargin: codeScroll.barRoom
            model: diffPane.diffModel
            // The rows the write asked for have landed: put the view back where it was reading, and work out which of
            // the new rows the pointer is over. Not the empty half of the swap — a reset shows up here as zero first.
            onCountChanged: {
                if (count > 0) {
                    diffPane.restoreScroll()
                    pointedRowTimer.restart()
                }
            }
            // A row moving under a pointer standing still is the same question as the pointer moving over the rows.
            onContentYChanged: diffPane.settlePointedRow()
            // An image with no text rows hands its space to the preview (SVG edits keep both).
            visible: diffPane.diffModel.previewKind !== "image" || count > 0
            /// How far down the view can go, and the clamp both hands that send it share — the wheel by the notch, the
            /// arrows by the row. One surface moves within one set of bounds (デザイン規約 §diff を上下に送る).
            readonly property real maxY: Math.max(0, contentHeight - height)
            function clampY(y) {
                return Math.max(0, Math.min(y, diffList.maxY))
            }
            // Qt's own key navigation moves `currentIndex` and tells nobody, and would scroll the view to a selection
            // nothing here follows; the arrows are answered below, where they move the view (規約 §diff を上下に送る).
            keyNavigationEnabled: false
            // The one key this pane answers that is not a step. `StandardKey` rather than a spelling of our own, so the
            // platform's idea of copy is what is matched. Declared before the two below because the general handler is
            // offered every key first; the arrows carry on to their own handlers whenever this one leaves it alone.
            Keys.onPressed: event => {
                if (event.matches(StandardKey.Copy))
                    event.accepted = diffPane.copySelection()
            }
            Keys.onUpPressed: event => event.accepted = diffPane.stepRows(-1)
            Keys.onDownPressed: event => event.accepted = diffPane.stepRows(1)
            // Mouse wheels scroll a fixed number of rows per notch — the same Metrics.wheelRows every other surface
            // answers a notch with (GraphPane, the right panes). Touchpads keep native Flickable panning.
            WheelHandler {
                acceptedDevices: PointerDevice.Mouse
                onWheel: event => {
                    // Sending the rows is the hand arriving here without a press to say so (規約 §diff のファイル一覧).
                    diffPane.handArrived()
                    diffList.cancelFlick()
                    // The wheel's own sideways component — a tilt wheel, a touchpad — says where it wants to go in the
                    // input itself, so it is answered wherever the pointer is (デザイン規約 §グラフを横へ送る, same rule).
                    if (event.angleDelta.x !== 0)
                        codeScroll.shift(-event.angleDelta.x / 2)
                    const step = (event.angleDelta.y / 120) * Metrics.wheelRows * Theme.rowHeight
                    diffList.contentY = diffList.clampY(diffList.contentY - step)
                }
            }
            delegate: DiffRowDelegate {
                id: diffRow
                rowWidth: diffList.width
                codeX: codeScroll.offset
                charW: diffPane.charW
                wideDelta: diffPane.wideDelta
                seatW: diffPane.seatW
                partial: diffPane.partial
                staged: diffPane.staged
                busy: diffPane.busy
                numberW: diffPane.numberW
                sidesTold: diffPane.sidesTold
                oursColor: diffPane.sideColor("ours")
                theirsColor: diffPane.sideColor("theirs")
                hoverHunk: diffPane.hoverHunk
                hoverLine: diffPane.hoverLine
                onLineStageRequested: (hunk, line) => diffPane.stageLine(hunk, line)
                onDiscardRequested: hunk => diffPane.discardHunkRequested(hunk)
                onStageHunkRequested: hunk => diffPane.stageSelectionRequested(hunk, -1)
            }
        }
    }
    // The hand that picks the text out of the rows. Declared after the body so it stands over them, and before the
    // scroll below so that the bar along the bottom edge and the middle-click hand keep their own presses.
    DiffTextSelect {
        id: textPick
        view: diffList
        diffModel: diffPane.diffModel
        gutterW: diffPane.gutterW
        barRoom: diffList.barRoom
        codeX: codeScroll.offset
        charW: diffPane.charW
        wideDelta: diffPane.wideDelta
        // One clamp and one shift, the ones every other hand that sends this pane goes through
        // (デザイン規約 §diff を上下に送る / §diff を横へ送る).
        onScrollWanted: (dy, dx) => {
            diffList.contentY = diffList.clampY(diffList.contentY + dy)
            codeScroll.shift(dx)
        }
        onMenuWanted: diffPane.codeMenuRequested()
    }
    // The hand that sends the rows sideways and up and down, and the bar that says how far there is to go. Declared
    // after the body so it stands over the rows, and beside the list rather than inside it (`DiffCodeScroll` says why).
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
