pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

// One line of a unified diff: its two numbers, the seat between them where a changed line puts its mark out, the line
// itself, and — on a hunk heading — the two words that act on the whole hunk.
//
// **The mark is the only thing in a row that takes a press** (デザイン規約 §diff の中のステージ). Nothing else in the row is a
// target at all, so the text stays free for the hand that wants to read or copy it — and whether the pointer is
// here is not the row's question either: the pane works it out and names the row (`hoverHunk` / `hoverLine`, see
// `DiffPane.settlePointedRow`).
//
// Everything the pane knows arrives as a property; everything the pane has to do about a press leaves as a signal.
Rectangle {
    id: diffRow

    required property string kind
    required property int old_no
    required property int new_no
    /// What this row draws, as `Text.StyledText` reads it — every row, coloured or not (`encode::DiffRow`). One format
    /// and not two: a row whose format arrived a moment after its text was measured with its own tags counted as
    /// letters, and the width that came off it set how far the whole diff could be sent (`DiffReach`).
    required property string text
    /// Which row this is, for the width it reports back (`DiffReach.noteRow`).
    required property int index
    /// Which reading of the rows this row's text is from (`DiffModel.rowsGen`). A new one makes every row on screen
    /// say its width again: the pane files widths per reading, and a row whose text came out the same width would
    /// otherwise never speak for the new one.
    required property int rowsGen
    /// Where what changed inside this row falls in the line as this row spells it — `"from:len,…"` in the places a
    /// layout counts, empty for nothing (`encode::DiffRow.emph`). Drawn as the stronger wash under the text; the
    /// quiet parts of the row keep the line's own background (デザイン規約 §シンタックスハイライト).
    required property string emph
    /// One of git's conflict fences (`encode::DiffRow`).
    required property bool fence
    /// This line ends the file without a newline, on the side its own numbers name (`encode::DiffRow`). git says it in
    /// a note of its own between the two sides; the row says it at the end of the line the note was about
    /// (デザイン規約 §行末の改行が無いこと).
    required property bool no_newline
    /// Where the reader's own selection falls on this row (`DiffModel.sel`): `"*"` for a line taken end to end —
    /// which is what almost every selected row is — and otherwise the same `from:len` runs `emph` carries, for the
    /// one or two rows a drag cuts through.
    ///
    /// Empty on every row the plain `Copy` does not take: outside the selection, and on the removed lines and hunk
    /// headings inside it. **The wash is the answer** — what is not washed is not copied
    /// (デザイン規約 §diff の中身をコピーする).
    required property string sel
    required property int hunk
    required property int line
    /// "ours" / "theirs" / "" — the model reads it off the marker columns once, by the parser's own rule
    /// (`platitude_core::parse::diff::side_of_markers`).
    required property string side

    required property real rowWidth
    /// How far the file's own text has been sent sideways (`DiffCodeScroll.offset`). The gutter and the hunk headings
    /// do not travel with it: the numbers, the mark and the pane's own words are about the row rather than in it
    /// (デザイン規約 §diff を横へ送る).
    required property real codeX
    /// One measured column of the mono font (`DiffPane.charMeasure`). The row draws nothing at it: it is how wide the
    /// wash on an **empty** line is, and that is the only place a width without characters behind it is needed.
    required property real charW
    /// Where the two washes go: this row's own line, laid out, asked where each run of it is drawn
    /// (`LineRuler`). The same ruler the hand over the rows reads a press against (`DiffTextSelect`).
    required property var ruler
    /// The room held between the two numbers for the mark, as the pane
    /// works it out once for every row (`DiffPane.seatW`).
    required property int seatW
    /// This diff has pieces worth naming (`DiffPane.partial`).
    required property bool partial
    /// Which way a write on this diff goes, and whether one is running.
    required property bool staged
    required property bool busy
    /// How wide a line number is (`DiffPane.numberW`).
    required property int numberW
    /// Whether the two sides are being told apart by colour, and the two colours themselves — held as properties rather
    /// than asked of the pane per row.
    required property bool sidesTold
    required property var oursColor
    required property var theirsColor
    /// Which hunk and line the pointer is on, as the pane works it out (`DiffPane.settlePointedRow`) — and as the
    /// automation names it, since hover cannot be injected (verify-ui). One pair, one answer, whichever way it was
    /// arrived at.
    required property int hoverHunk
    required property int hoverLine

    /// How wide this row was laid out, as soon as that is known and again whenever it changes — a `Text` answers with
    /// every glyph at the family's own advance until the fallback carrying the wide ones is resolved, so the first
    /// number out of it is not the last (`DiffTextMetrics`). The pane files it under `index`, so a second answer
    /// replaces the first rather than being added to a maximum (`DiffReach`).
    ///
    /// Headings do not send one: they stand at the pane's own left edge and never travel, so how wide their words are
    /// is not how far there is to go.
    signal rowDrawn(int row, real drawn)

    /// A press on the line's own mark: this line goes over to the other side now.
    signal lineStageRequested(int hunk, int line)
    signal discardRequested(int hunk)
    signal stageHunkRequested(int hunk)

    // The hunk's discard lives in its heading. Reached from outside for the smoke run, which holds it the way a hand
    // does. `null` on every row that is not a heading with tools on it — the tools are built only there (`hunkTools`).
    readonly property var discardButton: hunkTools.item ? hunkTools.item.discardButton : null

    width: diffRow.rowWidth
    height: Theme.rowHeight
    onRowsGenChanged: diffRow.tellWidth()
    /// Says what this row was laid out at. Guarded on the Label rather than assumed: a reading can turn over while
    /// this row is still being built, and the parts of a delegate exist only once the whole of it does.
    function tellWidth() {
        if (!diffRow.banded && codeLine)
            diffRow.rowDrawn(diffRow.index, codeLine.implicitWidth)
    }
    /// Automation: how far right this row's ink stands inside `frame` — **not `codeInk` carried through the send**,
    /// though the two describe the same edge. That arithmetic runs through the gutter and the offset, which is what
    /// `codeMax` is built out of, so a run holding the result against `codeMax` would be checking a number against
    /// itself. This asks the item where it was *placed*, which is the one reading of that edge owing the reach
    /// nothing — and the far end of a send is the only place a reach measured past the end of every line can be seen
    /// at all (verify-ui, `code-grow`). A heading does not travel, so it answers 0 rather than standing in for where
    /// the file's own text ends.
    function inkRightIn(frame) {
        return diffRow.banded || !codeLine
            ? 0 : codeLine.mapToItem(frame, codeLine.implicitWidth, 0).x
    }
    /// A row that names something rather than showing a line of a file: the hunk's own heading, and — where several
    /// commits' patches of one file stand one after another — the commit each block is of
    /// (デザイン規約 §複数のコミットを選ぶ). Neither has a line number, a stage seat or a place in the gutter.
    readonly property bool banded: diffRow.kind === "hunk" || diffRow.kind === "commit"
    /// A changed line is bold (規約 §シンタックスハイライト) — the wash says which side it is, the weight is what makes
    /// it stand off the context around it. Not the fences: git's scaffolding is not a change to read. Named here
    /// rather than written on the Label alone, because **the ruler has to be set in the weight the row is drawn in**
    /// — a family whose bold face advances differently would otherwise put every wash and every press out by the
    /// difference (`DiffTextSelect` reads it off this row for the same reason).
    readonly property bool codeBold: !diffRow.fence && (diffRow.kind === "add" || diffRow.kind === "del")
    /// Automation: how far this row's own line is drawn, which is where the blank right of it begins
    /// (`PGG_AUTO_ACT=diff-blank`). The same number the row files under `rowDrawn`, as a property — a run presses
    /// beside one row rather than beside the widest, and no count of that row's characters can find this edge.
    readonly property real codeInk: codeLine ? codeLine.implicitWidth : 0

    /// Where this row's two washes are drawn, taken from this row's own line laid out — `[{ x, w }, …]` in the
    /// code's own coordinates, before the send (`LineRuler.rectsOf`). A line taken end to end says so with one
    /// word and needs no ruler at all; a heading has neither wash.
    ///
    /// **Pushed, not bound** (the rule `DiffTextMetrics.codeW` is written under). Asking the ruler means putting
    /// this row's line on it, and a binding that writes while it is being evaluated is a binding loop — Qt says so
    /// by name and then holds whatever it had, which is a wash left on the row before it. The four properties the
    /// answer is made of say when to ask instead.
    property var emphRects: []
    property var selRects: []
    function settleEmph() {
        diffRow.emphRects = diffRow.ruler.rectsOf(diffRow.text, diffRow.codeBold, diffRow.emph)
    }
    function settleSel() {
        diffRow.selRects = diffRow.sel === "*"
                           ? [] : diffRow.ruler.rectsOf(diffRow.text, diffRow.codeBold, diffRow.sel)
    }
    function settleWashes() {
        diffRow.settleEmph()
        diffRow.settleSel()
    }
    // The line itself and the weight it is set in move both washes; each run moves its own. A delegate handed to
    // another row (`reuseItems`) arrives through these same three.
    onTextChanged: diffRow.settleWashes()
    onCodeBoldChanged: diffRow.settleWashes()
    onEmphChanged: diffRow.settleEmph()
    onSelChanged: diffRow.settleSel()
    Component.onCompleted: diffRow.settleWashes()
    color: kind === "add" ? Theme.diffAddedBg
           : kind === "del" ? Theme.diffRemovedBg
           : kind === "hunk" ? Theme.diffHunkHeaderBg
           // The window's own band ground, the one every heading in it wears (`PaneHeader`): a commit is a step
           // above the hunks it brought, and wearing the hunk's own would make the two read as one kind of row.
           : kind === "commit" ? Theme.bgElevated : "transparent"
    /// Under the pointer. A heading's own row is line -1, so a hunk named without a line means the heading.
    readonly property bool underPointer: diffRow.hoverHunk === diffRow.hunk && diffRow.hoverLine === diffRow.line
    /// The pointer is on this hunk's heading, so the whole hunk lights: the heading's two words act on exactly these
    /// rows, and this is what says so (デザイン規約 §diff の中のステージ). Only the heading does it — a pointer resting on a line is
    /// reading, not aiming at the hunk.
    readonly property bool inAimedHunk: diffRow.partial && diffRow.hoverLine < 0 && diffRow.hoverHunk === diffRow.hunk
    /// The room held between the two line numbers for the mark this line puts out for the hand. A
    /// hairline of air on each side of it: the mark belongs to neither number, and anything wider reads as the new
    /// number having drifted off its own column.
    ///
    /// Where no line can be staged on its own the seat closes to the plain gap — a diff with no pieces in it never puts
    /// a mark out, and holding the room open would leave a hole nothing ever stands in.
    readonly property int stageSeatW: diffRow.banded ? 0 : diffRow.seatW
    /// How much of the row's right edge the hunk heading has to give up to the two words that act on the hunk. git's
    /// `@@` line is as long as the enclosing signature and would otherwise run under them.
    readonly property real toolsRoom: hunkTools.item ? hunkTools.width + Theme.spaceSm * 2 : 0

    // The hunk under the pointer wears the wash a row anywhere else in the app wears under one.
    Rectangle {
        anchors.fill: parent
        color: Theme.bgHover
        visible: diffRow.inAimedHunk
    }
    // Which side this line came from, in the colour that branch wears in the graph. The diff's own green cannot say it
    // — git paints our side and theirs the same, because each is in the file and in neither of the other's — so the
    // head of the row says it instead.
    Rectangle {
        visible: diffRow.sidesTold && diffRow.side !== ""
        width: Theme.spaceXs
        height: parent.height
        color: diffRow.side === "ours" ? diffRow.oursColor : diffRow.theirsColor
    }
    // The gutter: two numbers with the mark's seat between them. It stays where it is however far the code is sent
    // sideways — a number belongs to the row rather than to the line, and a `+` that scrolled out of reach would take
    // partial staging with it.
    Row {
        id: gutter
        height: parent.height
        spacing: 0
        Label {
            id: oldNoCol
            // The pane's edge and the code stand one `spaceXs` from the numbers; what stands between the two numbers is
            // the line's own mark (`stageSeatW`), so this column keeps no padding on that side — the seat carries the
            // whole gap.
            //
            // Both columns close on a hunk heading, which has no line to number: the heading takes the row from its
            // left edge.
            width: diffRow.banded ? 0 : Theme.spaceXs + diffRow.numberW
            leftPadding: Theme.spaceXs
            height: parent.height
            verticalAlignment: Text.AlignVCenter
            text: diffRow.old_no >= 0 ? diffRow.old_no : ""
            horizontalAlignment: Text.AlignRight
            color: Theme.textMuted
            font.family: Theme.monoFamily
            font.pixelSize: Theme.fontSm
        }
        // Where the mark below stands. Empty, and held open on every row of a diff that can be taken apart: a context
        // line has no mark, and a seat that closed on the rows without one would walk the numbers left and right under
        // the pointer.
        Item {
            width: diffRow.stageSeatW
            height: parent.height
        }
        Label {
            id: newNoCol
            width: diffRow.banded ? 0 : diffRow.numberW + Theme.spaceXs
            height: parent.height
            verticalAlignment: Text.AlignVCenter
            text: diffRow.new_no >= 0 ? diffRow.new_no : ""
            horizontalAlignment: Text.AlignRight
            rightPadding: Theme.spaceXs
            color: Theme.textMuted
            font.family: Theme.monoFamily
            font.pixelSize: Theme.fontSm
        }
    }
    // The room the file's own text is read in. It is cut to what is left of the row, and the line inside it is not: the
    // text is as wide as it is and travels under this window, so a long line is read by sending it rather than by
    // having its end replaced with three dots (デザイン規約 §diff を横へ送る).
    Item {
        id: codeRoom
        x: gutter.width
        width: Math.max(0, diffRow.rowWidth - gutter.width - diffRow.toolsRoom)
        height: parent.height
        clip: true
        // The stronger wash under what actually changed inside the row (`emph`), while the quiet parts keep the
        // line's own background — the strong/weak split is these rectangles, not the text
        // (デザイン規約 §シンタックスハイライト). Under the Label, travelling with the same send.
        Repeater {
            model: diffRow.emphRects
            delegate: Rectangle {
                required property var modelData
                x: -diffRow.codeX + modelData.x
                width: modelData.w
                height: codeRoom.height
                color: diffRow.kind === "add" ? Theme.diffAddedEmphBg : Theme.diffRemovedEmphBg
            }
        }
        // The reader's own selection, over the emphasis and under the text — the same place the row's other two
        // washes stand, and for the same reason: the strong/weak split of a diff is rectangles, not letters
        // (デザイン規約 §シンタックスハイライト), so a selected line keeps its syntax colours.
        Repeater {
            model: diffRow.sel === "*" ? [diffRow.sel] : diffRow.selRects
            delegate: Rectangle {
                required property var modelData
                readonly property bool whole: modelData === "*"
                x: -diffRow.codeX + (whole ? 0 : modelData.x)
                // A line taken whole is washed to its own end — the ink it was drawn in, which is the one number no
                // walk of it can produce — and never to nothing: an empty line is still a line the copy takes, and a
                // wash of no width would leave a hole in the middle of a selection.
                width: whole ? Math.max(codeLine.implicitWidth, diffRow.charW) : modelData.w
                height: parent.height
                color: Theme.bgSelected
            }
        }
        Label {
            id: codeLine
            // A hunk heading does not travel: it is the pane's own words about the rows below, and words that slid off
            // the left while the code was read would take with them the only thing saying which hunk this is. It gives
            // up the right of the row to the two buttons and elides into what is left.
            x: diffRow.banded ? 0 : -diffRow.codeX
            width: diffRow.banded ? codeRoom.width : implicitWidth
            elide: diffRow.banded ? Text.ElideRight : Text.ElideNone
            height: parent.height
            verticalAlignment: Text.AlignVCenter
            // A hunk heading starts at the row's own left edge: the two columns beside it are empty
            // — a heading has no line to number — so indenting it by them lines the pane's own words up with the
            // file's, behind a gutter that says nothing. One `spaceXs`, the same gap everything else in the gutter
            // stands at.
            leftPadding: diffRow.banded ? Theme.spaceXs : 0
            text: diffRow.text
            // Every row is markup, coloured or not (`markup::styled`), so a line of source full of `<T>` arrives
            // escaped and this never changes under a row (規約 §シンタックスハイライト).
            textFormat: Text.StyledText
            font.family: Theme.monoFamily
            // The weight this row is set in, said once for the Label and for the ruler both washes are placed with
            // (`codeBold`).
            font.bold: diffRow.codeBold
            // The size an editor puts source at rather than a step in the UI's scale — `fontCode`, matched to
            // IntelliJ's default (デザイン規約 §タイポグラフィ).
            //
            // The hunk heading is smaller still: it is the pane's own words rather than the file's, and at the file's
            // size its `@@` line runs under the two words sitting at the right of the same row.
            font.pixelSize: diffRow.banded ? Theme.fontSm : Theme.fontCode
            // Where the theme said nothing — an uncoloured language, a row past the lexer's budget, the moment before
            // the colours land — this is still the whole of the row's colour. A changed line reads in the window's own
            // words (規約 §シンタックスハイライト): the wash and the weight already name it, and green-on-green said the
            // same thing twice while reading worse.
            //
            // A fence drops its voice: `<<<<<<<` is git's scaffolding round the two sides, not a line the file has
            // anything to say with, and painting it the added-line green puts the loudest thing in the pane on the part
            // nobody is reading (デザイン規約 §シンタックスハイライト). It keeps its background — it really is in the file.
            // How far this row is drawn, filed under its own row number (`diffRow.rowDrawn`).
            onImplicitWidthChanged: diffRow.tellWidth()
            Component.onCompleted: diffRow.tellWidth()
            color: diffRow.fence ? Theme.textMuted
                   : diffRow.kind === "hunk" ? Theme.diffHunkHeaderFg
                   // The band's own voice, the one every heading in this window speaks in (`PaneHeader`).
                   : diffRow.kind === "commit" ? Theme.textSecondary
                   : diffRow.kind === "meta" ? Theme.textMuted
                   : Theme.textPrimary
        }
        // git's `\ No newline at end of file`, said where the thing it is about is: at the end of this line
        // (デザイン規約 §行末の改行が無いこと). As a row it stood between the removed line and the added one and parted the
        // pair the eye reads as one change; as a mark it travels with the text, so it is at the end of the line
        // whichever way the pane has been sent.
        //
        // **Built only on the line that has it.** A delegate is built per line on screen, and the mark is a canvas
        // with a hover and a tip of its own — on every other line it would be built and hidden, which is the heap
        // this pane is measured by (the rule the hunk tools below follow; rules-refs/app-ui.md, the Loader rule).
        Loader {
            id: noEolMark
            active: diffRow.no_newline
            // Off the ink, not the box: the mark's square holds air past its ring, and a whole `spaceXs` on top of
            // that would leave the last character further from this than any other pair in the row (§余白).
            x: noEolMark.item
               ? codeLine.x + codeLine.implicitWidth + Theme.spaceXs - (noEolMark.item.width - noEolMark.item.inkWidth) / 2
               : 0
            anchors.verticalCenter: parent.verticalCenter
            sourceComponent: NavIcon {
                kind: "no-entry"
                // A mark that stands at the end of a line of words (デザイン規約 §寸法), with the stroke dropped to that
                // step so it carries the weight of the letters beside it rather than 4/3 of it.
                width: Theme.iconSm
                height: Theme.iconSm
                stroke: Metrics.iconStroke * Theme.iconSm / Theme.iconMd
                tint: Theme.danger
                // The words the note used to carry, kept where a mark can still hand them over. It takes no button:
                // the press over the code belongs to the hand picking text out of the rows (`DiffTextSelect`).
                ToolTip.visible: noEolHover.containsMouse
                ToolTip.delay: Metrics.tipDelayMs
                ToolTip.text: qsTr("No newline at end of file")
                MouseArea {
                    id: noEolHover
                    anchors.fill: parent
                    acceptedButtons: Qt.NoButton
                    hoverEnabled: true
                }
            }
        }
    }
    // Hunk-level staging. The row carries the hunk index the patch builder needs, so what is staged is exactly what is
    // shown — and so is what is thrown away. Absent on a diff with no pieces in it (see `partial`).
    //
    // **Built only on a heading that has them, never hidden on the other lines.** A delegate is built per line on
    // screen, and each `ActionButton` is a label with its rulers, a hold and its timers — on the lines that are not
    // headings the pair was the heaviest thing in the row while drawing nothing — tens of megabytes of heap on a diff
    // of a few dozen lines, most of it in parts the rows never showed (ci/baseline/code-costs-windows-x64.md §メモリの形).
    // Same rule as the dialogs behind `WindowDialogSeat` (rules-refs/app-ui.md).
    Loader {
        id: hunkTools
        active: diffRow.partial && diffRow.kind === "hunk"
        anchors.right: parent.right
        anchors.rightMargin: Theme.spaceSm
        anchors.verticalCenter: parent.verticalCenter
        sourceComponent: Row {
            /// The smoke run's handle on the discard, read through the loader (`diffRow.discardButton`).
            readonly property alias discardButton: discardHunkButton
            spacing: Theme.spaceXs
            // Only the unstaged side has a piece to throw away: on the staged side the button beside this one puts the
            // hunk back where it can be.
            //
            // Held, not asked about (デザイン規約 §長押し): the button sits in the hunk's own heading, so what it takes is the
            // thing it is standing on, and a bar coming down over the diff to say so is machinery a hunk does not need.
            //
            // Shaped like the held row of a right-click menu, not like the toolbar's framed button: this one sits in a
            // line of other words rather than in a row of other buttons, and a frame around one word in a heading reads
            // as a box that has come loose. The mark says it is held, and the hold fills the words' own ground edge to
            // edge.
            ActionButton {
                id: discardHunkButton
                visible: !diffRow.staged
                text: qsTr("Discard hunk")
                font.pixelSize: Theme.fontSm
                // Asleep until the pointer is on this heading (デザイン規約 §diff の中のステージ). The mark is what still says this
                // one is held rather than clicked — the colour is saying something else.
                tone: diffRow.underPointer ? Theme.danger : Theme.textSecondary
                holdTone: Theme.danger
                holdMs: Metrics.holdMs
                enabled: !diffRow.busy
                // Which hunk of which reading (`HoldDriver.premise`): the pane re-reads itself behind every write, and
                // a delegate re-used across that read would discard whatever it was pointed at when the fill ran out.
                premise: diffRow.rowsGen + ":" + diffRow.hunk
                onHeld: diffRow.discardRequested(diffRow.hunk)
            }
            // The same pair of colours the file rows put on their own `+` and `−`: staging is the green half of the
            // gesture and unstaging the red one, and the heading should not name them in a different voice than the
            // list does — but it says it at the volume of a heading that is not being pointed at. At rest both words
            // wear `textSecondary`, which is the colour the `@@` beside them already has, so the whole heading reads
            // as one grey line until the pointer arrives (デザイン規約 §diff の中のステージ). A file diff carries 2 hunks at the
            // middle and 8 at the ninetieth percentile — measured over 52 files — so leaving them all lit puts 2 to 4
            // coloured words on screen against the header's one.
            ActionButton {
                text: diffRow.staged ? qsTr("Unstage hunk") : qsTr("Stage hunk")
                font.pixelSize: Theme.fontSm
                tone: !diffRow.underPointer ? Theme.textSecondary
                      : diffRow.staged ? Theme.diffRemovedFg : Theme.diffAddedFg
                enabled: !diffRow.busy
                onActivated: diffRow.stageHunkRequested(diffRow.hunk)
            }
        }
    }
    // The mark a changed line puts out for the hand: `+` where a press takes the line into the staging area and `−`
    // where it takes it back out, in the pair of colours that gesture wears everywhere else (デザイン規約 §diff の中のステージ). It
    // names the *direction* of the write, not what the line did — an added and a deleted line are both staged by the
    // same `+`. No frame: a box around a mark this size reads as a control that came loose from the toolbar, and the
    // ground it needs is the one the pointer brings with it.
    //
    // **It writes, there and then**, and it is the only thing in the row that takes a press at all.
    //
    // It stands in the seat the row holds between the two numbers (`stageSeatW`).
    //
    // **Built only on a line that can be staged on its own** — a changed line of a diff with pieces in it. The rows of
    // a commit's diff never carry one, and the context lines of any diff do not either, so on those it is not built
    // rather than hidden (the hunk tools' rule). Whether the pointer is on the line is still what shows it: the
    // pointer moves faster than a canvas can be built and painted, so the mark waits built and hidden on the lines
    // the pointer can reach.
    Loader {
        active: diffRow.partial && (diffRow.kind === "add" || diffRow.kind === "del")
        x: oldNoCol.width + Theme.borderWidth
        anchors.verticalCenter: parent.verticalCenter
        sourceComponent: Rectangle {
            visible: diffRow.underPointer
            width: Theme.iconMd
            height: Theme.iconMd
            radius: Theme.radiusSm
            color: stageLineHover.containsMouse ? Theme.bgHover : "transparent"
            ToolTip.visible: stageLineHover.containsMouse
            ToolTip.delay: Metrics.tipDelayMs
            ToolTip.text: diffRow.staged ? qsTr("Unstage this line") : qsTr("Stage this line")
            NavIcon {
                anchors.centerIn: parent
                width: Theme.iconSm
                height: Theme.iconSm
                kind: diffRow.staged ? "minus" : "plus"
                tint: diffRow.staged ? Theme.diffRemovedFg : Theme.diffAddedFg
            }
            MouseArea {
                id: stageLineHover
                anchors.fill: parent
                hoverEnabled: true
                enabled: !diffRow.busy
                onClicked: diffRow.lineStageRequested(diffRow.hunk, diffRow.line)
            }
        }
    }
    // No square for throwing one line away. A line can be staged on its own because staging loses nothing — the line
    // stays on disk either way — but the smallest thing that can be thrown away is a hunk (デザイン規約 §その他の操作): a bare `×`
    // has no words to say what it takes, and a control that must be held has to say it.
}
