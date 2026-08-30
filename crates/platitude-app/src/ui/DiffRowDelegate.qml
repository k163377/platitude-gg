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
    required property string text
    /// Whether `text` is markup rather than the line itself (`encode::DiffRow`). Read from the row rather than guessed
    /// at: a line of C++ is full of `<` and `>`.
    required property bool rich
    /// Display columns of what changed inside this row — `"col:wides:width:wides,…"`, where each run says where it
    /// starts and how far it runs in columns, then how many wide glyphs stand in each of those (`encode::DiffRow.emph`).
    /// Empty for nothing. Drawn as the stronger wash under the text; the quiet parts of the row keep the line's own
    /// background (デザイン規約 §シンタックスハイライト).
    required property string emph
    /// One of git's conflict fences (`encode::DiffRow`).
    required property bool fence
    /// This line ends the file without a newline, on the side its own numbers name (`encode::DiffRow`). git says it in
    /// a note of its own between the two sides; the row says it at the end of the line the note was about
    /// (デザイン規約 §行末の改行が無いこと).
    required property bool no_newline
    /// Where the reader's own selection falls on this row (`DiffModel.sel`): `"*"` for a line taken end to end —
    /// which is what almost every selected row is — and otherwise the same `col:wides:width:wides` runs `emph`
    /// carries, for the one or two rows a drag cuts through.
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
    /// One measured column of the mono font (`DiffPane.charMeasure`), which is what turns `emph`'s columns into x.
    required property real charW
    /// What a wide glyph costs beyond the two columns it is counted as (`DiffPane.wideDelta`, measured — zero where
    /// the mono family carries them itself). `emph` says how many stand in each run, and this is what those are worth.
    required property real wideDelta
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

    /// A press on the line's own mark: this line goes over to the other side now.
    signal lineStageRequested(int hunk, int line)
    signal discardRequested(int hunk)
    signal stageHunkRequested(int hunk)

    // The hunk's discard lives in its heading. Reached from outside for the smoke run, which holds it the way a hand
    // does.
    readonly property alias discardButton: discardHunkButton

    width: diffRow.rowWidth
    height: Theme.rowHeight
    color: kind === "add" ? Theme.diffAddedBg
           : kind === "del" ? Theme.diffRemovedBg : kind === "hunk" ? Theme.diffHunkHeaderBg : "transparent"
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
    readonly property int stageSeatW: diffRow.kind === "hunk" ? 0 : diffRow.seatW
    /// How much of the row's right edge the hunk heading has to give up to the two words that act on the hunk. git's
    /// `@@` line is as long as the enclosing signature and would otherwise run under them.
    readonly property real toolsRoom: hunkTools.visible ? hunkTools.width + Theme.spaceSm * 2 : 0

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
            width: diffRow.kind === "hunk" ? 0 : Theme.spaceXs + diffRow.numberW
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
            width: diffRow.kind === "hunk" ? 0 : diffRow.numberW + Theme.spaceXs
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
            model: diffRow.emph === "" ? [] : diffRow.emph.split(",")
            delegate: Rectangle {
                required property string modelData
                // `col:wides:width:wides`. A column is one advance of the mono font, and each wide glyph standing
                // among them is worth `wideDelta` more than the two columns it was counted as — zero on a family
                // that draws the wide glyphs itself. Laid on columns alone, the wash sat right of the characters it
                // names by that much a glyph (measured, on Windows: −3px each).
                readonly property var run: modelData.split(":")
                x: -diffRow.codeX + Number(run[0]) * diffRow.charW + Number(run[1]) * diffRow.wideDelta
                width: Number(run[2]) * diffRow.charW + Number(run[3]) * diffRow.wideDelta
                height: codeRoom.height
                color: diffRow.kind === "add" ? Theme.diffAddedEmphBg : Theme.diffRemovedEmphBg
            }
        }
        // The reader's own selection, over the emphasis and under the text — the same place the row's other two
        // washes stand, and for the same reason: the strong/weak split of a diff is rectangles, not letters
        // (デザイン規約 §シンタックスハイライト), so a selected line keeps its syntax colours.
        Repeater {
            model: diffRow.sel === "" ? [] : diffRow.sel === "*" ? [diffRow.sel] : diffRow.sel.split(",")
            delegate: Rectangle {
                required property string modelData
                readonly property bool whole: modelData === "*"
                readonly property var run: whole ? [] : modelData.split(":")
                x: -diffRow.codeX
                   + (whole ? 0 : Number(run[0]) * diffRow.charW + Number(run[1]) * diffRow.wideDelta)
                // A line taken whole is washed to its own end — and never to nothing: an empty line is still a line
                // the copy takes, and a wash of no width would leave a hole in the middle of a selection.
                width: whole ? Math.max(codeLine.implicitWidth, diffRow.charW)
                             : Number(run[2]) * diffRow.charW + Number(run[3]) * diffRow.wideDelta
                height: parent.height
                color: Theme.bgSelected
            }
        }
        Label {
            id: codeLine
            // A hunk heading does not travel: it is the pane's own words about the rows below, and words that slid off
            // the left while the code was read would take with them the only thing saying which hunk this is. It gives
            // up the right of the row to the two buttons and elides into what is left.
            x: diffRow.kind === "hunk" ? 0 : -diffRow.codeX
            width: diffRow.kind === "hunk" ? codeRoom.width : implicitWidth
            elide: diffRow.kind === "hunk" ? Text.ElideRight : Text.ElideNone
            height: parent.height
            verticalAlignment: Text.AlignVCenter
            // A hunk heading starts at the row's own left edge: the two columns beside it are empty
            // — a heading has no line to number — so indenting it by them lines the pane's own words up with the
            // file's, behind a gutter that says nothing. One `spaceXs`, the same gap everything else in the gutter
            // stands at.
            leftPadding: diffRow.kind === "hunk" ? Theme.spaceXs : 0
            text: diffRow.text
            // A coloured line arrives already marked up, and the plain ones must stay plain: `StyledText` on a line of
            // source would read its `<T>` as a tag and drop it (規約 §シンタックスハイライト).
            textFormat: diffRow.rich ? Text.StyledText : Text.PlainText
            font.family: Theme.monoFamily
            // A changed line is bold (規約 §シンタックスハイライト) — the wash says which side it is, the weight is what
            // makes it stand off the context around it. Not the fences: git's scaffolding is not a change to read.
            font.bold: !diffRow.fence && (diffRow.kind === "add" || diffRow.kind === "del")
            // The size an editor puts source at rather than a step in the UI's scale — `fontCode`, matched to
            // IntelliJ's default (デザイン規約 §タイポグラフィ).
            //
            // The hunk heading is smaller still: it is the pane's own words rather than the file's, and at the file's
            // size its `@@` line runs under the two words sitting at the right of the same row.
            font.pixelSize: diffRow.kind === "hunk" ? Theme.fontSm : Theme.fontCode
            // Where the theme said nothing — an uncoloured language, a row past the lexer's budget, the moment before
            // the colours land — this is still the whole of the row's colour. A changed line reads in the window's own
            // words (規約 §シンタックスハイライト): the wash and the weight already name it, and green-on-green said the
            // same thing twice while reading worse.
            //
            // A fence drops its voice: `<<<<<<<` is git's scaffolding round the two sides, not a line the file has
            // anything to say with, and painting it the added-line green puts the loudest thing in the pane on the part
            // nobody is reading (デザイン規約 §シンタックスハイライト). It keeps its background — it really is in the file.
            color: diffRow.fence ? Theme.textMuted
                   : diffRow.kind === "hunk" ? Theme.diffHunkHeaderFg : diffRow.kind === "meta" ? Theme.textMuted
                   : Theme.textPrimary
        }
        // git's `\ No newline at end of file`, said where the thing it is about is: at the end of this line
        // (デザイン規約 §行末の改行が無いこと). As a row it stood between the removed line and the added one and parted the
        // pair the eye reads as one change; as a mark it travels with the text, so it is at the end of the line
        // whichever way the pane has been sent.
        NavIcon {
            id: noEolMark
            visible: diffRow.no_newline
            kind: "no-entry"
            // A mark that stands at the end of a line of words (デザイン規約 §寸法), with the stroke dropped to that step
            // so it carries the weight of the letters beside it rather than 4/3 of it.
            width: Theme.iconSm
            height: Theme.iconSm
            stroke: Metrics.iconStroke * Theme.iconSm / Theme.iconMd
            tint: Theme.danger
            // Off the ink, not the box: the mark's square holds air past its ring, and a whole `spaceXs` on top of
            // that would leave the last character further from this than any other pair in the row (§余白).
            x: codeLine.x + codeLine.implicitWidth + Theme.spaceXs - (width - inkWidth) / 2
            anchors.verticalCenter: parent.verticalCenter
            // The words the note used to carry, kept where a mark can still hand them over. It takes no button: the
            // press over the code belongs to the hand picking text out of the rows (`DiffTextSelect`).
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
    // Hunk-level staging. The row carries the hunk index the patch builder needs, so what is staged is exactly what is
    // shown — and so is what is thrown away. Absent on a diff with no pieces in it (see `partial`).
    Row {
        id: hunkTools
        visible: diffRow.partial && diffRow.kind === "hunk"
        anchors.right: parent.right
        anchors.rightMargin: Theme.spaceSm
        anchors.verticalCenter: parent.verticalCenter
        spacing: Theme.spaceXs
        // Only the unstaged side has a piece to throw away: on the staged side the button beside this one puts the hunk
        // back where it can be.
        //
        // Held, not asked about (デザイン規約 §長押し): the button sits in the hunk's own heading, so what it takes is the thing
        // it is standing on, and a bar coming down over the diff to say so is machinery a hunk does not need.
        //
        // Shaped like the held row of a right-click menu, not like the toolbar's framed button: this one sits in a line
        // of other words rather than in a row of other buttons, and a frame around one word in a heading reads as a box
        // that has come loose. The mark says it is held, and the hold fills the words' own ground edge to edge.
        ActionButton {
            id: discardHunkButton
            visible: !diffRow.staged
            text: qsTr("Discard hunk")
            font.pixelSize: Theme.fontSm
            // Asleep until the pointer is on this heading (デザイン規約 §diff の中のステージ). The mark is what still says this one
            // is held rather than clicked — the colour is saying something else.
            tone: diffRow.underPointer ? Theme.danger : Theme.textSecondary
            holdTone: Theme.danger
            holdMs: Metrics.holdMs
            enabled: !diffRow.busy
            onHeld: diffRow.discardRequested(diffRow.hunk)
        }
        // The same pair of colours the file rows put on their own `+` and `−`: staging is the green half of the gesture
        // and unstaging the red one, and the heading should not name them in a different voice than the list does — but
        // it says it at the volume of a heading that is not being pointed at. At rest both words wear `textSecondary`,
        // which is the colour the `@@` beside them already has, so the whole heading reads as one grey line until the
        // pointer arrives (デザイン規約 §diff の中のステージ). A file diff carries 2 hunks at the middle and 8 at the ninetieth
        // percentile — measured over 52 files — so leaving them all lit puts 2 to 4 coloured words on screen against
        // the header's one.
        ActionButton {
            text: diffRow.staged ? qsTr("Unstage hunk") : qsTr("Stage hunk")
            font.pixelSize: Theme.fontSm
            tone: !diffRow.underPointer ? Theme.textSecondary
                  : diffRow.staged ? Theme.diffRemovedFg : Theme.diffAddedFg
            enabled: !diffRow.busy
            onActivated: diffRow.stageHunkRequested(diffRow.hunk)
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
    Rectangle {
        visible: diffRow.partial && diffRow.underPointer && (diffRow.kind === "add" || diffRow.kind === "del")
        x: oldNoCol.width + Theme.borderWidth
        anchors.verticalCenter: parent.verticalCenter
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
    // No square for throwing one line away. A line can be staged on its own because staging loses nothing — the line
    // stays on disk either way — but the smallest thing that can be thrown away is a hunk (デザイン規約 §その他の操作): a bare `×`
    // has no words to say what it takes, and a control that must be held has to say it.
}
