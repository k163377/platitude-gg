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
    /// A file git stopped on: `git add` still runs, but means "this is dealt with", and the words follow that.
    property bool conflicted: false
    /// A conflicted file's two stage letters (`UU`, `DU`, …) and what each side is called. On the conflicts git prints
    /// no patch for, they are all the pane can say.
    property string conflictChange: ""
    property string sideOurs: ""
    property string sideTheirs: ""
    /// Each side's colour as an index into `Theme.graphLane` — the branch's own lane colour where it has one, and
    /// always two different ones (`encode::conflict_side_colors`).
    property int sideColorOurs: -1
    property int sideColorTheirs: -1
    // A write is running: staging buttons disable.
    property bool busy: false
    /// False for a file read out of another working copy, which reads the way a commit's file does (デザイン規約
    /// §別の作業コピーを読む): no whole-file word, no hunk or line seats. Not disabled — not offered: staging belongs
    /// to the copy that holds the index.
    property bool writable: true
    /// The copy the file was read from, where it is not this window's — said in the band before the path: while a
    /// diff is open nothing else on screen can say whose file it is.
    property string copyName: ""
    /// Whether a hunk and a line can be staged on their own. Not on a new file — its one hunk is the whole file
    /// (デザイン規約 §diff の中のステージ) — nor a conflicted one, whose combined diff `git apply` refuses (and so does
    /// core, `stage::refuse_combined`); both keep the header's one word. Nor on another working copy's file.
    readonly property bool partial: diffPane.fromWorkTree && diffPane.writable
                                    && !diffPane.diffModel.isNewFile && !diffPane.combined

    // ---- one column or two ----------------------------------------
    /// Side by side (old left, new right) rather than one column (デザイン規約 §diff を 2 列で読む). The model's: it
    /// lays the rows out, and the band's toggle asks the page to write it (`splitChosen`).
    readonly property bool split: diffPane.diffModel.split
    /// How wide the left column is; the right starts a hairline after it. The hairline is the row's, so the two
    /// columns share the rows' width to the pixel and a row is never a pixel wider than the list.
    readonly property real halfW: Math.floor((diffList.width - Theme.borderWidth) / 2)
    /// The band's toggle, forwarded: the page owns the choice.
    signal splitChosen(bool split)
    /// Automation: the toggle itself, so a run flips the view through the press a hand makes (`diff-split`).
    readonly property alias viewToggle: paneHeader.viewToggle

    // ---- a conflicted file's diff -----------------------------------
    /// This diff has more than one old side, so every row carries a marker
    /// column per side (`platitude_core::parse::diff`).
    readonly property bool combined: diffPane.diffModel.isCombined
    /// Whether the two sides are told apart by colour — always, on a conflicted file: where the graph has no two
    /// colours to lend, `encode::conflict_side_colors` completes the pair.
    readonly property bool sidesTold: diffPane.combined && diffPane.sideColorOurs >= 0 && diffPane.sideColorTheirs >= 0
    function sideColor(side) {
        const index = side === "ours" ? diffPane.sideColorOurs : diffPane.sideColorTheirs
        return Theme.graphLane[index % Theme.graphLane.length]
    }
    /// Automation (`conflict-sides`): how many rows each side is named on — the bands are too thin for a picture to
    /// answer. The model counts (`DiffModel.sideCount`).
    function sideTally() {
        const ours = diffPane.diffModel.sideCount("ours")
        const theirs = diffPane.diffModel.sideCount("theirs")
        return "combined=" + diffPane.combined + " told=" + diffPane.sidesTold
             + " both=" + (ours > 0 && theirs > 0) + " ours=" + ours + " theirs=" + theirs
    }

    /// A page right-click menu is standing: nothing behind it is hovered, or a row's marks would draw over the rows
    /// the menu is about (デザイン規約 §メニュー).
    property bool menuStanding: false

    signal closeRequested()
    /// Stage or unstage the whole file (direction follows `staged`).
    signal stageFileRequested()
    /// Stage or unstage one hunk (line < 0) or one line of it.
    signal stageSelectionRequested(int hunk, int line)
    /// Text the reader picked out of the rows, on its way to the clipboard (the page owns it).
    signal copyRequested(string text)
    /// A right-click on the text; the selection it acts on is already settled (`DiffTextSelect.askMenu`).
    signal codeMenuRequested()

    /// A press on one line's mark (デザイン規約 §diff の中のステージ). Named so a run enters where the mark does — the
    /// mark exists only under a pointer, and a press on it cannot be injected (verify-ui).
    function stageLine(hunk, line) {
        diffPane.stageSelectionRequested(hunk, line)
    }
    /// Where the pointer is. **The pane works out which row it is over, not the rows** — a rebuilt row is not hovered
    /// until the mouse moves again (rules-refs の `settlePointedRow` の行). **On the pane**: laid over the rows, a
    /// `HoverHandler` takes their hover away.
    HoverHandler {
        id: panePointer
        onPointChanged: diffPane.settlePointedRow()
        // Leaving clears the mark here, not in `settlePointedRow`, so the row a headless run named stands.
        onHoveredChanged: {
            if (panePointer.hovered)
                diffPane.settlePointedRow()
            else
                diffPane.showLineTools(-1, -1)
        }
    }
    // Every press in this pane: the hand is here now, so the arrows are (規約 §diff を上下に送る). A `PointHandler`
    // takes only passive grabs, so the buttons, the bars and the rows' marks underneath keep working.
    //
    // **On a sheet in front of everything** (rules-refs の「画面全体の入力観測」の行): on the pane itself it hears no
    // press — the list (a `Flickable`) and the text hand take them all first. A bare `Item`, so the rows keep their
    // hover (a `HoverHandler` here would take it).
    Item {
        id: pressDoor
        anchors.fill: parent
        z: 100
        PointHandler {
            acceptedButtons: Qt.AllButtons
            onActiveChanged: {
                if (active)
                    diffPane.handArrived()
            }
        }
    }
    /// Automation: whether that sheet is still in front of everything this pane draws — a run cannot inject a press
    /// (verify-ui). **Strictly above; a tie reads false** (one more declaration would put it behind). `childAt` cannot
    /// answer this: it walks the children in declaration order and ignores `z`.
    function doorOnTop() {
        const kids = diffPane.children
        for (let i = 0; i < kids.length; i++) {
            if (kids[i] !== pressDoor && kids[i].visible && kids[i].z >= pressDoor.z)
                return false
        }
        return true
    }
    /// Names the row the pointer is over, or none. A heading is named as line -1, which lights its whole hunk.
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
        // The row answers which of its lines the point is over — side by side it has two (`pointedAt`).
        const aim = row.pointedAt(at.x)
        diffPane.showLineTools(aim[0], aim[1])
    }
    // A menu standing takes the marks away there and then: the pointer has not moved, so nothing else would ask again.
    onMenuStandingChanged: diffPane.settlePointedRow()
    // After the rows are replaced: on the frame they land the list has not laid them out — the scroll restore's wait
    // (`DiffScrollPlace`).
    Timer {
        id: pointedRowTimer
        interval: Metrics.anchorDelayMs
        onTriggered: diffPane.settlePointedRow()
    }
    // The keyboard and the reading position live in `DiffRowWalk`; callers read them off this pane's forwards.
    DiffRowWalk {
        id: rowWalk
        view: diffList
    }
    function handArrived() { rowWalk.handArrived() }
    function stepRows(delta) { return rowWalk.stepRows(delta) }
    readonly property alias atEnd: rowWalk.atEnd
    /// Automation only: the list itself, for a run that reads where the view stands (as `GraphPane.view` is).
    readonly property alias view: diffList

    /// Throw one hunk away — unstaged side only, consented to by holding the heading's button (デザイン規約 §その他の操作).
    signal discardHunkRequested(int hunk)

    /// Automation: hold the first hunk's discard button to its end. Answers whether the press went in: not before the
    /// row is there, nor on a button waiting on git.
    function completeHold() {
        const row = diffList.itemAtIndex(0)
        return row !== null && row.discardButton !== null && row.discardButton.completeHold()
    }

    // ---- the row the pointer is over ----
    // Written by `settlePointedRow`, and by a run naming a row as though the pointer were on it — hover cannot be
    // injected (verify-ui).
    property int hoverHunk: -1
    property int hoverLine: -1
    function showLineTools(hunk, line) {
        diffPane.hoverHunk = hunk
        diffPane.hoverLine = line
    }
    /// Automation: the sideways offset and its limit, whether the bar is out, and the two ways of moving it — a middle
    /// button cannot be injected (verify-ui).
    readonly property alias codeAt: codeScroll.offset
    readonly property alias codeMax: codeScroll.maxOffset
    /// The longest line as measured (`DiffTextMetrics.codeW`) — how far the lines reach, where `codeMax` is what is
    /// left once the room is taken.
    readonly property alias codeMeasured: metrics.codeW
    /// What the rows themselves came to (`DiffReach.rowsWidth`) — whether a row wanted more than the measured pick.
    readonly property alias codeDrawn: reachTally.rowsWidth
    /// Which row that width was filed under (`DiffReach.widestRow`). Delegates are reused, and a width filed under the
    /// wrong row looks right while nothing on screen is that wide.
    readonly property alias codeWidestRow: reachTally.widestRow
    /// Automation: where the built rows' ink ends on the right, in the list's coordinates (`code-grow`). `codeMax`
    /// cannot answer this — it derives from the reach's own width, so a reach measured past every line's end agrees
    /// with itself. Rows the list has not built answer nothing.
    function codeInkRight() {
        let right = 0
        for (let i = 0; i < diffList.count; i++) {
            const row = diffList.itemAtIndex(i)
            if (row)
                right = Math.max(right, row.inkRightIn(diffList))
        }
        return right
    }
    readonly property alias codeBarShown: codeScroll.barShown
    readonly property alias codeHandOn: codeScroll.handScrolling
    function sendCode(dx) { codeScroll.shift(dx) }
    function startCodeHand(x, y) { codeScroll.startHand(x, y) }
    function driftCodeHand(x, y) { codeScroll.driftHand(x, y) }

    // ---- the text the reader picked out -------------------------------
    /// What `Copy` and Ctrl+C put on the clipboard (デザイン規約 §diff の中身をコピーする). An empty selection sends
    /// nothing and answers false, which lets the key fall through.
    function copySelection() {
        const text = diffPane.diffModel.selectionText()
        if (text === "")
            return false
        diffPane.copyRequested(text)
        return true
    }
    /// The menu key on the diff (デザイン規約 §メニュー のキーボード): the selection's menu, as a right-click inside the
    /// selection raises it, standing under the selection's moving end (`DiffTextSelect.endPlace`). Nothing selected,
    /// nothing opens; nor while the rows do not hold the keyboard (`KeyMenu.answer` comes here from anything inside
    /// the pane). Says whether the selection was asked.
    function menuFromKeys() {
        if (!diffList.activeFocus || !diffPane.diffModel.selHasNew && diffPane.diffModel.selRemoved === 0)
            return false
        const place = textPick.endPlace()
        if (place === null)
            return false
        KeyMenu.ask(place, () => diffPane.codeMenuRequested())
        return true
    }
    /// Automation: the text hand without a pointer (verify-ui), through the same three functions the `MouseArea`'s
    /// handlers call, so a run cannot pass while they do something else. `side`: 0 for the rows' own lines, 1 for the
    /// right of a split row.
    function pickText(side, fromRow, fromAt, toRow, toAt) {
        textPick.pressText(side, fromRow, fromAt)
        textPick.dragText(toRow, toAt)
        textPick.releaseText()
    }
    /// Automation: the hand itself, for runs that go in by pixel — the ground under the last row only a coordinate
    /// can name (`diff-sweep`).
    readonly property alias textHand: textPick
    /// Automation: whether the band's word takes a press (`DiffPaneHeader.stageOffered`), and whether a hunk or a line
    /// puts a seat out — both false on another working copy's file (`carried-read`).
    readonly property alias stageOffered: paneHeader.stageOffered
    readonly property bool piecesOffered: diffPane.partial
    /// Automation: the band's own hand, which drags over the path from the air beside it (`diff-band-sweep`).
    readonly property alias headerHand: paneHeader.pad
    /// Automation: whether that band ran out of room for the path — the half a picture of a wide pane cannot answer.
    readonly property alias headerCut: paneHeader.cut
    /// Automation: how far apart the band's caption and its path stand, baseline to baseline, and the same for the
    /// first heading's two words (-1 until that row is built) — `PGG_AUTO_ACT=hunk-tools`.
    function captionApart() { return paneHeader.captionApart() }
    function hunkWordsApart() {
        const row = diffList.itemAtIndex(0)
        return row !== null ? row.hunkWordsApart() : -1
    }
    /// Automation: the right-click at a place in one column's text, where the menu's answer is settled.
    function askCodeMenu(side, row, at) { textPick.askMenu(side, row, at) }
    /// Automation: the first row holding a removed line, or -1 — what the menu's second word is about. Side by side
    /// it is the left of its row.
    function firstRemovedRow() {
        for (let i = 0; i < diffList.count; i++) {
            const row = diffList.itemAtIndex(i)
            if (row && row.kind === "del")
                return i
        }
        return -1
    }
    /// Automation: the first row read across — a removed line with the added one that replaced it on its right — or
    /// -1 (`diff-split`).
    function firstPairedRow() {
        for (let i = 0; i < diffList.count; i++) {
            const row = diffList.itemAtIndex(i)
            if (row && row.kind === "del" && row.pair_kind === "add")
                return i
        }
        return -1
    }
    /// Automation: how many rows wear the selection, and what the two menu rows would take. Side by side a row wears
    /// it on one column, never both (`DiffModel::selection`).
    function pickTally() {
        let washed = 0
        for (let i = 0; i < diffList.count; i++) {
            const row = diffList.itemAtIndex(i)
            if (row && (row.sel || row.pair_sel))
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

    /// The first line of a hunk that a partial write can act on, or -1. Lines are numbered through the hunk's context,
    /// so line 0 is usually unchanged — a patch of it is empty and core refuses it.
    function firstChangedLine(hunk) {
        for (let i = 0; i < diffList.count; i++) {
            const row = diffList.itemAtIndex(i)
            if (!row || row.hunk !== hunk)
                continue
            if (row.kind === "add" || row.kind === "del")
                return row.line
            // Side by side, an added line nothing was removed for stands on the right of a row with an empty left.
            if (row.pair_kind === "add")
                return row.pair_line
        }
        return -1
    }

    /// Automation: whether the read is over and came back with something to look at — rows, or a picture, a binary
    /// or an embedded repository, which have none. Line-level verbs wait on `firstChangedLine` instead (true only
    /// once the list has built the rows).
    function diffSettled() {
        const m = diffPane.diffModel
        // `embedded` never becomes rows: git will not open a nested repository, so the pane shows a line about it
        // (`DiffFileNotices`).
        return !m.loading && (diffList.count > 0 || m.isBinary || m.previewKind !== "" || m.embedded)
    }
    /// Automation: a settled read is not yet a picture on screen — the decode is asynchronous
    /// (`DiffFileNotices.picturesSettled`).
    readonly property bool picturesSettled: fileNotices.picturesSettled
    readonly property int picturesShown: fileNotices.picturesShown

    // The widths the rows are laid out from, measured off rulers that are never drawn.
    DiffTextMetrics {
        id: metrics
        diffModel: diffPane.diffModel
        partial: diffPane.partial
        split: diffPane.split
    }
    /// Where a place in a row's line is drawn and which place a point is over, asked of the line laid out. One for the
    /// pane: a ruler handed a line answers about it in the same statement.
    LineRuler {
        id: lineRuler
        // The rows' own format and size (`markup::styled`, `DiffRowDelegate`): in any other, the ruler measures a line
        // this pane never draws.
        textFormat: TextEdit.RichText
        font.pixelSize: Theme.fontCode
    }
    /// How far this diff reaches sideways: the lines the model picked, measured before any row exists, and the rows
    /// themselves as they are laid out.
    DiffReach {
        id: reachTally
        rowsGen: diffPane.diffModel.rowsGen
        measured: metrics.codeW
        // A different file starts over, as `DiffCodeScroll`'s offset does on the same word; the same file read again
        // keeps its reach.
        file: diffPane.diffModel.title
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
            writable: diffPane.writable
            copyName: diffPane.copyName
            split: diffPane.split
            onStageFileRequested: diffPane.stageFileRequested()
            onCloseRequested: diffPane.closeRequested()
            onSplitChosen: split => diffPane.splitChosen(split)
        }
        // What the pane says about the file itself, and the picture that stands in for one no rows can show.
        DiffFileNotices {
            id: fileNotices
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
            // The sideways bar's strip below the last row; none where there is nowhere sideways to go.
            Layout.bottomMargin: codeScroll.barRoom
            // The middle button is the pane's: its hand goes sideways as well, over the hand that picks the text
            // (`DiffCodeScroll`).
            ownsHand: false
            model: diffPane.diffModel
            // The rows the write asked for have landed. The filled half only — a reset shows up here as zero first.
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
            /// How far down the view can go, and the one clamp every hand that sends it shares (デザイン規約 §diff を上下に送る).
            readonly property real maxY: Math.max(0, contentHeight - height)
            function clampY(y) {
                return Math.max(0, Math.min(y, diffList.maxY))
            }
            /// One notch, sent — and the way in for a run, since a wheel cannot be injected (verify-ui スキル).
            function sendRows(pixels) {
                wheelGlide.sendTo(diffList.clampY(wheelGlide.at - pixels))
            }
            /// Something else is moving the view: the notch in flight loses it (`WheelGlide.halt`).
            function haltGlide() {
                wheelGlide.halt()
            }
            WheelGlide {
                id: wheelGlide
                view: diffList
            }
            // The bar's arrows and track glide their steps on this same glide (`AutoScrollBar.stepGlide`).
            Binding {
                target: diffList.ScrollBar.vertical
                property: "stepGlide"
                value: wheelGlide
            }
            // The arrows are answered below (規約 §diff を上下に送る): Qt's own key navigation would scroll to a
            // `currentIndex` nothing here follows.
            keyNavigationEnabled: false
            // The one key this pane answers that is not a step; `StandardKey`, so the platform's copy is what matches.
            // Qt offers this handler only what the arrow handlers below leave unaccepted.
            Keys.onPressed: event => {
                if (event.matches(StandardKey.Copy))
                    event.accepted = diffPane.copySelection()
            }
            Keys.onUpPressed: event => event.accepted = diffPane.stepRows(-1)
            Keys.onDownPressed: event => event.accepted = diffPane.stepRows(1)
            // The menu key; Windows' Shift+F10 comes through the window (`Main.keyMenuAsked`).
            Keys.onMenuPressed: event => event.accepted = diffPane.menuFromKeys()
            // Mouse wheels send `Metrics.wheelRows` rows per notch, as every other surface does; touchpads keep
            // Flickable's own panning.
            WheelHandler {
                acceptedDevices: PointerDevice.Mouse
                onWheel: event => {
                    // Sending the rows is the hand arriving here without a press to say so (規約 §diff を上下に送る).
                    diffPane.handArrived()
                    diffList.cancelFlick()
                    // A sideways component (tilt wheel, touchpad) is answered wherever the pointer is (デザイン規約
                    // §グラフを横へ送る, same rule).
                    if (event.angleDelta.x !== 0)
                        codeScroll.shift(-event.angleDelta.x / 2)
                    diffList.sendRows((event.angleDelta.y / 120) * Metrics.wheelRows * Theme.rowHeight)
                }
            }
            delegate: DiffRowDelegate {
                id: diffRow
                rowWidth: diffList.width
                rowsGen: diffPane.diffModel.rowsGen
                onRowDrawn: (row, drawn) => reachTally.noteRow(row, drawn)
                codeX: codeScroll.offset
                charW: metrics.charW
                ruler: lineRuler
                seatW: metrics.seatW
                partial: diffPane.partial
                staged: diffPane.staged
                busy: diffPane.busy
                numberW: metrics.numberW
                sidesTold: diffPane.sidesTold
                oursColor: diffPane.sideColor("ours")
                theirsColor: diffPane.sideColor("theirs")
                hoverHunk: diffPane.hoverHunk
                hoverLine: diffPane.hoverLine
                split: diffPane.split
                halfW: diffPane.halfW
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
        gutterW: metrics.gutterW
        split: diffPane.split
        halfW: diffPane.halfW
        barRoom: diffList.barRoom
        codeX: codeScroll.offset
        ruler: lineRuler
        // Through the one clamp and the one shift every hand that sends this pane uses.
        onScrollWanted: (dy, dx) => {
            diffList.contentY = diffList.clampY(diffList.contentY + dy)
            codeScroll.shift(dx)
        }
        onMenuWanted: diffPane.codeMenuRequested()
    }
    // The hand that sends the rows sideways and up and down, and the sideways bar. Declared after the body so it
    // stands over the rows, and beside the list (`DiffCodeScroll` says why).
    DiffCodeScroll {
        id: codeScroll
        anchors.fill: parent
        view: diffList
        paneHovered: panePointer.hovered
        file: diffPane.diffModel.title
        // The reach, plus the pane's own gap past the longest line's last character.
        codeWidth: reachTally.width > 0 ? reachTally.width + Theme.spaceSm : 0
        // Side by side the room is a column's, not the row's: one offset sends both, and the narrower right one runs
        // out first.
        roomWidth: Math.max(0, (diffPane.split ? diffList.width - diffPane.halfW - Theme.borderWidth : diffList.width)
                               - metrics.gutterW)
    }
}
