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
    /// and the words follow that.
    property bool conflicted: false
    /// The two stage letters git reports for this file (`UU`, `DU`, …) and what each side is called — only a conflicted
    /// one has them. On the conflicts git prints no patch for, they are the whole of what the pane can say.
    property string conflictChange: ""
    property string sideOurs: ""
    property string sideTheirs: ""
    /// The colour each side is drawn in, as an index into `Theme.graphLane` — the graph's own lane colour for that
    /// branch where it has one, and always two different ones (`encode::conflict_side_colors` decides both).
    property int sideColorOurs: -1
    property int sideColorTheirs: -1
    // A write is running: staging buttons disable.
    property bool busy: false
    /// Whether the file on screen is this window's own to write. **A file read out of another working copy is read
    /// the way a commit's file is** (デザイン規約 §別の作業コピーを読む): the whole-file word does not stand, and
    /// neither do the seats a hunk and a line put out under the hand. Nothing here is disabled, because nothing here
    /// is offered — staging is an act in the copy that holds the index, and the pane behind this one is the door.
    property bool writable: true
    /// The copy the file was read from, where it is not this window's — said in the band, in front of the path,
    /// because while a diff is open the band is the only thing on screen that can say whose file this is.
    property string copyName: ""
    /// Whether this diff has pieces worth naming. A file the repository is seeing for the first time has none: its one
    /// hunk is the whole file, so `Stage hunk` would be the header's `Stage file` said again in a smaller voice
    /// (デザイン規約 §diff の中のステージ). A conflicted file has none either, for a different reason: its diff compares the
    /// working tree against **both** sides at once, and that shape is not a patch — `git apply` refuses it, and so does
    /// core (`stage::refuse_combined`). What stays in either case is the one word in the header.
    /// A third reason there are none: the file is another working copy's. Nothing can be taken out of it from here,
    /// so the hunk's row and the line's gutter close to the plain gap they close to on a commit's diff.
    readonly property bool partial: diffPane.fromWorkTree && diffPane.writable
                                    && !diffPane.diffModel.isNewFile && !diffPane.combined

    // ---- one column or two ----------------------------------------
    /// Whether the rows are read side by side — the old side on the left, the new on the right — rather than as one
    /// column (デザイン規約 §diff を 2 列で読む). The model's: the rows are laid out by it, and the band's toggle asks
    /// the page, which writes it there (`splitChosen`).
    readonly property bool split: diffPane.diffModel.split
    /// How wide the left column is; the right starts a hairline after it. The hairline is the row's, so the two
    /// columns share the rows' width to the pixel and a row is never a pixel wider than the list.
    readonly property real halfW: Math.floor((diffList.width - Theme.borderWidth) / 2)
    /// The reader wants the rows the other way round — the band's toggle, forwarded to the page, which owns the
    /// choice (app-ui.md 配線規約: state moves through the page).
    signal splitChosen(bool split)
    /// Automation: the toggle itself, so a run flips the view through the same press a hand makes
    /// (`PGG_AUTO_ACT=diff-split`).
    readonly property alias viewToggle: paneHeader.viewToggle

    // ---- a conflicted file's diff -----------------------------------
    /// This diff has more than one old side, so every row carries a marker
    /// column per side (`platitude_core::parse::diff`).
    readonly property bool combined: diffPane.diffModel.isCombined
    /// Whether the two sides are being told apart by colour. Always, on a conflicted file: the graph does not always
    /// have two colours to lend (a branch outside the walk's window has none, and the palette cycles), and where it
    /// falls short the pair is completed (`encode::conflict_side_colors`).
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
    /// **The pane answers for the rows.** Every write rebuilds the list under the hand, and a freshly built
    /// item is not hovered until the mouse moves again (Qt delivers hover on movement), so the mark would stay away
    /// after a press and the next line could not be staged without waggling the mouse first. The pane works out which
    /// row the pointer is over, and writes the same pair of properties the automation writes.
    ///
    /// **On the pane**: a handler laid over the rows takes their hover away entirely.
    HoverHandler {
        id: panePointer
        onPointChanged: diffPane.settlePointedRow()
        // Leaving takes the mark with it. Said here, so the row a run named stands: in `settlePointedRow`
        // the pointer never arrives and never leaves.
        onHoveredChanged: {
            if (panePointer.hovered)
                diffPane.settlePointedRow()
            else
                diffPane.showLineTools(-1, -1)
        }
    }
    // Every press in this pane, whatever it was for: the hand is here now, so the arrows are (規約 §diff のファイル一覧). A
    // `PointHandler` because it is the one handler specified to take only passive grabs — the buttons, the bars and the
    // rows' own marks all keep working underneath (`Main.qml`'s window watcher is one for the same reason).
    //
    // **On a sheet in front of everything** (規約 §画面全体の入力観測). Press delivery stops at the
    // first item that accepts, and a handler beneath that item never hears the press — so on the pane this heard
    // nothing at all: the list is a `Flickable` and takes every press on the rows, and the hand that picks the text
    // takes the rest. The keyboard came by wheel and by nothing else, which left `Ctrl+C` over a selection going
    // wherever the reader had pressed last (measured, qmltestrunner: with the handler under the hand, a press-and-drag
    // over the rows left `activeFocus=false` and the copy key unanswered; over it, both stand).
    //
    // The rows keep their hover under it, where a `HoverHandler` would take it: a bare `Item`
    // answers no pointer at all, so nothing below it loses one (measured the same way — the pane's own handler and a
    // row's both still stand).
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
    /// Automation: whether that sheet is still in front of everything this pane draws. A run cannot inject a press
    /// (verify-ui), so this is the half of "a selection you can take away" a headless drag can be asked for — where
    /// the door stands, read off the pane's own children.
    ///
    /// **Strictly above, and a tie reads as false.** `childAt` cannot answer this: it walks the children in the order
    /// they were declared and never looks at `z` at all (measured, qmltestrunner — the last-declared sibling comes
    /// back from a point the front-most one covers). A sheet level with something else is one more declaration away
    /// from being behind it, and that is not a state to go green on.
    function doorOnTop() {
        const kids = diffPane.children
        for (let i = 0; i < kids.length; i++) {
            if (kids[i] !== pressDoor && kids[i].visible && kids[i].z >= pressDoor.z)
                return false
        }
        return true
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
        // The row answers which of its lines the point is over — side by side it has two (`pointedAt`).
        const aim = row.pointedAt(at.x)
        diffPane.showLineTools(aim[0], aim[1])
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

    /// Throw one hunk away. Offered on the unstaged side only — the staged side unstages first. The heading's own
    /// button is held down, and the hold is the consent (デザイン規約 §その他の操作).
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
    /// What the pane measured the diff's longest line at (`DiffTextMetrics.codeW`). Read beside `codeMax` because the
    /// two answer different questions — how far the lines reach, and how much of that is left once the room is taken.
    readonly property alias codeMeasured: metrics.codeW
    /// And what the rows themselves have come to (`DiffReach.rowsWidth`) — the two are read apart because they answer
    /// different questions: whether the pick was any good, and whether a row wanted more than the pick knew about.
    readonly property alias codeDrawn: reachTally.rowsWidth
    /// And which row that width was filed under (`DiffReach.widestRow`). Read beside the width because the two fail
    /// apart: a delegate is reused for another row as the reader goes up and down, and a width filed under the wrong
    /// row leaves the number looking right while nothing on screen is that wide.
    readonly property alias codeWidestRow: reachTally.widestRow
    /// Automation: how far right the ink of the rows the list has built stands, in the list's own coordinates — where
    /// the text ends, as against the width the reach is keeping. **The one thing `codeMax` cannot be asked**: it is
    /// worked out from that same width, so a reach measured past the end of every line agrees with itself all the way
    /// down and shows only as room left over past the last character at the far end of a send (verify-ui,
    /// `code-grow`). Rows the list has not built answer nothing, which is what makes this a question about the rows on
    /// screen.
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
    /// `side` is the column the drag is of: 0 for the rows' own lines, 1 for the right of a split row.
    function pickText(side, fromRow, fromAt, toRow, toAt) {
        textPick.pressText(side, fromRow, fromAt)
        textPick.dragText(toRow, toAt)
        textPick.releaseText()
    }
    /// Automation: the hand itself, for the runs that go in through the pixels — the
    /// ground under the last row is a place only a coordinate can name (`PGG_AUTO_ACT=diff-sweep`). Handed over whole,
    /// the way the view is: four forwards here would be four more lines of this file saying nothing.
    readonly property alias textHand: textPick
    /// Automation: whether the band's own word would take a press (`DiffPaneHeader.stageOffered`), and whether a hunk
    /// or a line puts a seat out at all. The two halves of "nothing here stages" — a file read out of another working
    /// copy answers false to both (`carried-read`).
    readonly property alias stageOffered: paneHeader.stageOffered
    readonly property bool piecesOffered: diffPane.partial
    /// Automation: the pointer's stand-in on that word, and what it says under one.
    /// Automation: the band's own hand — the one the path at the top is dragged over from the air beside it. Handed
    /// over whole for the reason `textHand` is (`PGG_AUTO_ACT=diff-band-sweep`).
    readonly property alias headerHand: paneHeader.pad
    /// Automation: whether that band ran out of room for the path — the half a picture of a wide pane cannot answer.
    readonly property alias headerCut: paneHeader.cut
    /// Automation: the right-click, at a place in the text of one column — which is where the menu's answer is
    /// settled.
    function askCodeMenu(side, row, at) { textPick.askMenu(side, row, at) }
    /// Automation: the first row of the view that is a removed line, or -1 — the one row the menu's second word is
    /// about, and a drag that never reaches one proves half the rule. Side by side a removed line is the left of
    /// its row, and the row is still the one.
    function firstRemovedRow() {
        for (let i = 0; i < diffList.count; i++) {
            const row = diffList.itemAtIndex(i)
            if (row && row.kind === "del")
                return i
        }
        return -1
    }
    /// Automation: the first row of the view read across — a removed line with the added one that replaced it on
    /// its right — or -1. What `diff-split` is about (デザイン規約 §diff を 2 列で読む).
    function firstPairedRow() {
        for (let i = 0; i < diffList.count; i++) {
            const row = diffList.itemAtIndex(i)
            if (row && row.kind === "del" && row.pair_kind === "add")
                return i
        }
        return -1
    }
    /// Automation: what a row is wearing of the selection, and what the two menu rows would take. Side by side a
    /// row wears it on one column or the other, never both (`DiffModel::selection`).
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

    /// The first line of a hunk that a partial write can act on, or -1. A hunk's lines are numbered through the context
    /// it carries, so line 0 is usually a line that is not part of the change at all — selecting one builds a patch
    /// with nothing in it, which core refuses ("the selected part is no longer in its diff"). A pointer never has this
    /// problem: it picks its line by being over it, and the squares only come out on the lines that changed.
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

    /// Automation: whether the read this pane was asked for is over and came back with something to look at. The
    /// line-level verbs want `firstChangedLine` instead — only that is true once the rows exist *and* the list has
    /// built them — but a verb that only opens a file has pictures and binary files to allow for, and those have no
    /// rows at all.
    function diffSettled() {
        const m = diffPane.diffModel
        // `embedded` is the one answer that is not rows and never will be: git will not open a repository of its own,
        // so what the pane has to show for that row is the line about it (`DiffFileNotices`).
        return !m.loading && (diffList.count > 0 || m.isBinary || m.previewKind !== "" || m.embedded)
    }
    /// Automation: a settled read is not yet a picture on screen — the decode is asynchronous — so a verb that
    /// photographs the preview waits for the pictures too (`DiffFileNotices.picturesSettled`).
    readonly property bool picturesSettled: fileNotices.picturesSettled
    readonly property int picturesShown: fileNotices.picturesShown

    // Everything the rows are laid out from — the line-number column, the gutter, one column of the mono font and what
    // a wide glyph costs over it — measured off three rulers that are never drawn (`DiffTextMetrics`).
    DiffTextMetrics {
        id: metrics
        diffModel: diffPane.diffModel
        partial: diffPane.partial
        split: diffPane.split
    }
    /// Where a place in a row's line is drawn, and which place a point of it is over — asked of the line itself, laid
    /// out. One for the pane: the answer belongs to the line, so a ruler that has just been
    /// handed one answers about it in the same statement (`LineRuler`).
    LineRuler {
        id: lineRuler
        // The rows' own two. Every row of this pane is markup, coloured or not (`markup::styled`, `DiffRowDelegate`),
        // and is set at the size an editor puts source at — a ruler reading the same line in another format or size
        // is measuring a line this pane never draws.
        textFormat: TextEdit.RichText
        font.pixelSize: Theme.fontCode
    }
    /// How far this diff reaches sideways: the lines the model picked, measured before any row exists, and the rows
    /// themselves as they are laid out. A file of its own since it owns state the pane does not — a width per row,
    /// and which reading of the rows they belong to.
    DiffReach {
        id: reachTally
        rowsGen: diffPane.diffModel.rowsGen
        measured: metrics.codeW
        // A different file starts over: the place the reader was at goes with it (`DiffCodeScroll` sends the offset
        // back to the left edge on the same word), so the width it was clamped against goes too. The same file read
        // again keeps both — which is the whole of why the reach holds its answer through a reading.
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
        // The only thing this pane throws away is a hunk, and that is held down on the hunk's own
        // heading (デザイン規約 §その他の操作). What follows is what the pane has to say about the file
        // itself — and the picture that stands in for one no rows can show.
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
            // The strip the sideways bar stands on, below the last row (`DiffCodeScroll.barRoom`).
            // A file with nowhere sideways to go has no bar and gets the strip back.
            Layout.bottomMargin: codeScroll.barRoom
            // The middle button is the pane's: its hand goes sideways as well, over the hand that picks the text
            // (`DiffCodeScroll`).
            ownsHand: false
            model: diffPane.diffModel
            // The rows the write asked for have landed: put the view back where it was reading, and work out which of
            // the new rows the pointer is over. The filled half only — a reset shows up here as zero first.
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
            // The arrows are answered below, where they move the view (規約 §diff を上下に送る): Qt's own key navigation
            // moves `currentIndex` and tells nobody, and would scroll to a selection nothing here follows.
            keyNavigationEnabled: false
            // The one key this pane answers that is not a step. `StandardKey`, so the
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
        // One clamp and one shift, the ones every other hand that sends this pane goes through
        // (デザイン規約 §diff を上下に送る / §diff を横へ送る).
        onScrollWanted: (dy, dx) => {
            diffList.contentY = diffList.clampY(diffList.contentY + dy)
            codeScroll.shift(dx)
        }
        onMenuWanted: diffPane.codeMenuRequested()
    }
    // The hand that sends the rows sideways and up and down, and the bar that says how far there is to go. Declared
    // after the body so it stands over the rows, and beside the list (`DiffCodeScroll` says why).
    DiffCodeScroll {
        id: codeScroll
        anchors.fill: parent
        view: diffList
        paneHovered: panePointer.hovered
        file: diffPane.diffModel.title
        // The ink the rows and the picked lines came to, plus the room this pane holds past the last character of the
        // longest one — the one gap that is the pane's own.
        codeWidth: reachTally.width > 0 ? reachTally.width + Theme.spaceSm : 0
        // Side by side, a line has a column to be read in, not the row: both columns are sent by the one offset, so
        // the room is the column's and the narrower right one is what runs out first.
        roomWidth: Math.max(0, (diffPane.split ? diffList.width - diffPane.halfW - Theme.borderWidth : diffList.width)
                               - metrics.gutterW)
    }
}
