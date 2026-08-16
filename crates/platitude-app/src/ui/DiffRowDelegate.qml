pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

// One line of a unified diff: its two numbers, the seat between them
// where a changed line puts its mark out, the line itself, and — on a
// hunk heading — the two words that act on the whole hunk.
//
// Everything the pane knows arrives as a property; everything the pane
// has to do about a press leaves as a signal. The set of lines picked by
// hand lives on the pane, because a row is recycled the moment it
// scrolls off (DiffPane).
Rectangle {
    id: diffRow

    required property string kind
    required property int old_no
    required property int new_no
    required property string text
    /// Whether `text` is markup rather than the line itself
    /// (`encode::DiffRow`). Read from the row rather than
    /// guessed at: a line of C++ is full of `<` and `>`.
    required property bool rich
    /// One of git's conflict fences (`encode::DiffRow`).
    required property bool fence
    required property int hunk
    required property int line
    required property string markers
    /// "ours" / "theirs" / "" — see `DiffPane.sideOf`.
    required property string side

    required property real rowWidth
    /// This diff has pieces worth naming (`DiffPane.partial`).
    required property bool partial
    /// Which way a write on this diff goes, and whether one is running.
    required property bool staged
    required property bool busy
    /// How wide a line number is (`DiffPane.numberW`).
    required property int numberW
    /// Whether the two sides are being told apart by colour, and the two
    /// colours themselves — held as properties rather than asked of the
    /// pane per row.
    required property bool sidesTold
    required property var oursColor
    required property var theirsColor
    /// Which hunk and line the pointer is on, as the pane holds it — the
    /// heading's own hover writes it, and so does the automation hook.
    required property int hoverHunk
    required property int hoverLine
    /// Picked by hand — this line goes with the next write.
    required property bool picked
    /// How many of this hunk's lines are picked (heading rows).
    required property int pickedInHunk

    /// The pointer arrived on this hunk's heading, or left it.
    signal hunkPointedAt(bool inside, int hunk)
    signal lineToggleRequested(int hunk, int line)
    /// Anywhere in the diff that is not a changed line or a heading puts
    /// the whole choice down.
    signal choiceCleared()
    signal discardRequested(int hunk)
    signal stageHunkRequested(int hunk)
    signal stageChosenRequested()

    // The hunk's discard lives in its heading. Reached from
    // outside for the smoke run, which holds it the way a
    // hand does.
    readonly property alias discardButton: discardHunkButton

    width: diffRow.rowWidth
    height: Theme.rowHeight
    color: kind === "add" ? Theme.diffAddedBg
           : kind === "del" ? Theme.diffRemovedBg
           : kind === "hunk" ? Theme.diffHunkHeaderBg
           : "transparent"
    /// The pointer is on this hunk's heading, so the whole
    /// hunk lights: the heading's two words act on exactly
    /// these rows, and this is what says so
    /// (デザイン規約 §diff の中のステージ). Only the heading
    /// does it — a pointer resting on a line is reading, not
    /// aiming at the hunk.
    readonly property bool inAimedHunk:
        diffRow.partial && diffRow.hoverLine < 0
        && diffRow.hoverHunk === diffRow.hunk
    /// The room held between the two line numbers for the mark
    /// this line puts out for the hand (2026-08-13 ユーザー
    /// 指示). A hairline of air on each side of it: the mark
    /// belongs to neither number, and anything wider reads as
    /// the new number having drifted off its own column.
    ///
    /// Where no line can be staged on its own the seat closes
    /// to the plain gap — a diff with no pieces in it never
    /// puts a mark out, and holding the room open would leave
    /// a hole nothing ever stands in.
    readonly property int stageSeatW:
        diffRow.kind === "hunk" ? 0
        : diffRow.partial
          ? Theme.iconMd + 2 * Theme.borderWidth
          : Theme.spaceXs
    // The hunk under the pointer, and every line picked by
    // hand, wear the wash a row anywhere else in the app wears
    // under the pointer. A picked line also carries the mark
    // at its head — the diff's own colours own the row's
    // ground, so the choice cannot be shown by filling it.
    Rectangle {
        anchors.fill: parent
        color: Theme.bgHover
        visible: diffRow.picked || diffRow.inAimedHunk
    }
    Rectangle {
        width: Theme.spaceXs
        height: parent.height
        color: Theme.accent
        visible: diffRow.picked
    }
    // Which side this line came from, in the colour that
    // branch wears in the graph. The diff's own green cannot
    // say it — git paints our side and theirs the same,
    // because each is in the file and in neither of the
    // other's — so the head of the row says it instead. It
    // stands where the picked-line mark stands, which a
    // conflicted file never has: nothing in one can be staged
    // a piece at a time.
    Rectangle {
        visible: diffRow.sidesTold && diffRow.side !== ""
        width: Theme.spaceXs
        height: parent.height
        color: diffRow.side === "ours" ? diffRow.oursColor
                                       : diffRow.theirsColor
    }
    Row {
        anchors.fill: parent
        spacing: 0
        Label {
            id: oldNoCol
            // The pane's edge and the code stand one
            // `spaceXs` from the numbers; what stands between
            // the two numbers is the line's own mark
            // (`stageSeatW`), so this column keeps no padding
            // on that side — the seat carries the whole gap.
            //
            // Both columns close on a hunk heading, which has
            // no line to number: the heading takes the row
            // from its left edge.
            width: diffRow.kind === "hunk"
                   ? 0 : Theme.spaceXs + diffRow.numberW
            leftPadding: Theme.spaceXs
            height: parent.height
            verticalAlignment: Text.AlignVCenter
            text: diffRow.old_no >= 0 ? diffRow.old_no : ""
            horizontalAlignment: Text.AlignRight
            color: Theme.textMuted
            font.family: Theme.monoFamily
            font.pixelSize: Theme.fontSm
        }
        // Where the mark below stands. Empty, and held open on
        // every row of a diff that can be taken apart: a
        // context line has no mark, and a seat that closed on
        // the rows without one would walk the numbers left and
        // right under the pointer.
        Item {
            width: diffRow.stageSeatW
            height: parent.height
        }
        Label {
            id: newNoCol
            width: diffRow.kind === "hunk"
                   ? 0 : diffRow.numberW + Theme.spaceXs
            height: parent.height
            verticalAlignment: Text.AlignVCenter
            text: diffRow.new_no >= 0 ? diffRow.new_no : ""
            horizontalAlignment: Text.AlignRight
            rightPadding: Theme.spaceXs
            color: Theme.textMuted
            font.family: Theme.monoFamily
            font.pixelSize: Theme.fontSm
        }
        Label {
            // A hunk heading shares its row with the two words
            // that act on the hunk, and git's `@@` line is as
            // long as the enclosing signature: without giving
            // that space up the heading runs under them.
            width: parent.width - oldNoCol.width - newNoCol.width
                   - (hunkTools.visible
                      ? hunkTools.width + Theme.spaceSm * 2 : 0)
            height: parent.height
            verticalAlignment: Text.AlignVCenter
            // A hunk heading starts at the row's own left edge
            // (2026-08-13 ユーザー指示): the two columns beside
            // it are empty — a heading has no line to number —
            // so indenting it by them lines the pane's own
            // words up with the file's, behind a gutter that
            // says nothing. One `spaceXs`, the same gap
            // everything else in the gutter stands at.
            leftPadding: diffRow.kind === "hunk"
                         ? Theme.spaceXs : 0
            text: diffRow.text
            // A coloured line arrives already marked up, and
            // the plain ones must stay plain: `StyledText` on
            // a line of source would read its `<T>` as a tag
            // and drop it (規約 §シンタックスハイライト).
            textFormat: diffRow.rich ? Text.StyledText
                                     : Text.PlainText
            elide: Text.ElideRight
            font.family: Theme.monoFamily
            // The size an editor puts source at rather than a
            // step in the UI's scale — `fontCode`, matched to
            // IntelliJ's default (デザイン規約 §タイポグラフィ).
            //
            // The hunk heading is smaller still: it is the
            // pane's own words rather than the file's, and at
            // the file's size its `@@` line runs under the two
            // words sitting at the right of the same row.
            font.pixelSize: diffRow.kind === "hunk"
                            ? Theme.fontSm : Theme.fontCode
            // Where the theme said nothing — an uncoloured
            // language, a hunk heading — this is still the
            // whole of the row's colour.
            //
            // A fence drops its voice: `<<<<<<<` is git's
            // scaffolding round the two sides, not a line the
            // file has anything to say with, and painting it
            // the added-line green puts the loudest thing in
            // the pane on the part nobody is reading
            // (デザイン規約 §シンタックスハイライト). It keeps
            // its background — it really is in the file.
            color: diffRow.fence ? Theme.textMuted
                   : diffRow.kind === "add" ? Theme.diffAddedFg
                   : diffRow.kind === "del" ? Theme.diffRemovedFg
                   : diffRow.kind === "hunk" ? Theme.diffHunkHeaderFg
                   : diffRow.kind === "meta" ? Theme.textMuted
                   : Theme.textPrimary
        }
    }
    MouseArea {
        id: lineHover
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.LeftButton
        // Off entirely on a diff with no pieces in it: with
        // nothing to aim at, a row lighting up under the
        // pointer would be an offer that is not there.
        enabled: diffRow.partial
        // A heading under the pointer lights its own hunk, and
        // says so through the same pair the automation hook
        // writes — one answer to "which hunk is being aimed
        // at", whichever way the pointer got there.
        onContainsMouseChanged: {
            if (diffRow.kind !== "hunk")
                return
            diffRow.hunkPointedAt(lineHover.containsMouse, diffRow.hunk)
        }
        // One click, one line: a changed line joins what the
        // next write takes, or leaves it. Anywhere else in the
        // diff puts the whole choice down — the rows carrying
        // it are on screen, so there is nothing to lose track
        // of (デザイン規約 §diff の中のステージ).
        onClicked: {
            if (diffRow.kind === "add" || diffRow.kind === "del")
                diffRow.lineToggleRequested(diffRow.hunk, diffRow.line)
            else if (diffRow.kind !== "hunk")
                diffRow.choiceCleared()
        }
    }
    // Under the pointer, or named as if it were (see above).
    readonly property bool underPointer:
        lineHover.containsMouse
        || (diffRow.hoverHunk === diffRow.hunk
            && diffRow.hoverLine === diffRow.line)
    // Hunk-level staging. The row carries the hunk index the
    // patch builder needs, so what is staged is exactly what
    // is shown — and so is what is thrown away. Absent on a
    // diff with no pieces in it (see `partial`).
    Row {
        id: hunkTools
        visible: diffRow.partial && diffRow.kind === "hunk"
        anchors.right: parent.right
        anchors.rightMargin: Theme.spaceSm
        anchors.verticalCenter: parent.verticalCenter
        spacing: Theme.spaceXs
        // Only the unstaged side has a piece to throw away: on
        // the staged side the button beside this one puts the
        // hunk back where it can be.
        //
        // Held, not asked about (デザイン規約 §長押し): the
        // button sits in the hunk's own heading, so what it
        // takes is the thing it is standing on, and a bar
        // coming down over the diff to say so is machinery a
        // hunk does not need.
        //
        // Shaped like the held row of a right-click menu, not
        // like the toolbar's framed button: this one sits in a
        // line of other words rather than in a row of other
        // buttons, and a frame around one word in a heading
        // reads as a box that has come loose. The mark says it
        // is held, and the hold fills the words' own ground
        // edge to edge.
        ActionButton {
            id: discardHunkButton
            visible: !diffRow.staged
            text: qsTr("Discard hunk")
            font.pixelSize: Theme.fontSm
            // Asleep until the pointer is on this heading
            // (デザイン規約 §diff の中のステージ). The mark is
            // what still says this one is held rather than
            // clicked — the colour is saying something else.
            tone: diffRow.underPointer ? Theme.danger
                                       : Theme.textSecondary
            holdTone: Theme.danger
            holdMs: Metrics.holdMs
            enabled: !diffRow.busy
            onHeld: diffRow.discardRequested(diffRow.hunk)
        }
        // The same pair of colours the file rows put on their
        // own `+` and `−`: staging is the green half of the
        // gesture and unstaging the red one, and the heading
        // should not name them in a different voice than the
        // list does — but it says it at the volume of a
        // heading that is not being pointed at. At rest both
        // words wear `textSecondary`, which is the colour the
        // `@@` beside them already has, so the whole heading
        // reads as one grey line until the pointer arrives
        // (デザイン規約 §diff の中のステージ). A file diff
        // carries 2 hunks at the middle and 8 at the ninetieth
        // percentile — measured over 52 files — so leaving
        // them all lit puts 2 to 4 coloured words on screen
        // against the header's one.
        ActionButton {
            // With lines picked out of this hunk the word
            // names them instead: what is about to be written
            // is the choice, not the hunk. A choice is a
            // standing state, so the word is lit for as long
            // as it stands — the pointer is what wakes the
            // heading, but a choice keeps it awake.
            // Spelled out rather than left to `%n`: with no
            // translation loaded Qt keeps the source string as
            // it stands, and `2 line(s)` on a button is a
            // placeholder that shipped.
            text: diffRow.pickedInHunk === 0
                  ? (diffRow.staged ? qsTr("Unstage hunk")
                                    : qsTr("Stage hunk"))
                  : diffRow.pickedInHunk === 1
                    ? (diffRow.staged ? qsTr("Unstage 1 line")
                                      : qsTr("Stage 1 line"))
                    : (diffRow.staged
                       ? qsTr("Unstage %1 lines").arg(diffRow.pickedInHunk)
                       : qsTr("Stage %1 lines").arg(diffRow.pickedInHunk))
            font.pixelSize: Theme.fontSm
            tone: !diffRow.underPointer && diffRow.pickedInHunk === 0
                  ? Theme.textSecondary
                  : diffRow.staged ? Theme.diffRemovedFg
                                   : Theme.diffAddedFg
            enabled: !diffRow.busy
            onActivated: {
                if (diffRow.pickedInHunk > 0)
                    diffRow.stageChosenRequested()
                else
                    diffRow.stageHunkRequested(diffRow.hunk)
            }
        }
    }
    // The mark a changed line puts out for the hand: `+` where
    // a click takes the line into the staging area and `−`
    // where it takes it back out, in the pair of colours that
    // gesture wears everywhere else (デザイン規約 §diff の中の
    // ステージ). It names the *direction* of the write, not
    // what the line did — an added and a deleted line are both
    // staged by the same `+`. No frame: a box around a mark
    // this size reads as a control that came loose from the
    // toolbar, and the ground it needs is the one the pointer
    // brings with it.
    //
    // It stands in the seat the row holds between the two
    // numbers (`stageSeatW`), and stays out here rather than
    // in it: the row-wide hover area below the columns is
    // declared before this, so a mark laid out inside the
    // gutter would have its clicks taken by that instead.
    Rectangle {
        visible: diffRow.partial
                 && (diffRow.underPointer || diffRow.picked)
                 && (diffRow.kind === "add"
                     || diffRow.kind === "del")
        x: oldNoCol.width + Theme.borderWidth
        anchors.verticalCenter: parent.verticalCenter
        width: Theme.iconMd
        height: Theme.iconMd
        radius: Theme.radiusSm
        color: stageLineHover.containsMouse ? Theme.bgHover
                                            : "transparent"
        ToolTip.visible: stageLineHover.containsMouse
        ToolTip.delay: Metrics.tipDelayMs
        ToolTip.text: diffRow.staged ? qsTr("Unstage this line")
                                     : qsTr("Stage this line")
        NavIcon {
            anchors.centerIn: parent
            width: Theme.iconSm
            height: Theme.iconSm
            kind: diffRow.staged ? "minus" : "plus"
            tint: diffRow.staged ? Theme.diffRemovedFg
                                 : Theme.diffAddedFg
        }
        MouseArea {
            id: stageLineHover
            anchors.fill: parent
            hoverEnabled: true
            onClicked: diffRow.lineToggleRequested(diffRow.hunk,
                                                   diffRow.line)
        }
    }
    // No square for throwing one line away. A line can be
    // staged on its own because staging loses nothing — the
    // line stays on disk either way — but the smallest thing
    // that can be thrown away is a hunk (デザイン規約
    // §その他の操作): a bare `×` has no words to say what it
    // takes, and a control that must be held has to say it.
}
