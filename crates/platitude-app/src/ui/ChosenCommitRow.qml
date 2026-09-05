pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// One commit of a choice, as the pane on the right lists it (デザイン規約 §複数のコミットを選ぶ): the face of whoever
// wrote it, what it says, and its short id.
//
// **The row draws no choice of its own.** Commits are picked in the graph and this list is what was picked, so there is
// nothing here to select, unselect or aim a menu at — a highlight on these rows would be a second place the choice
// appeared to live.
//
// **What it does have is the two things the pane is for**: the words are a field a reader can drag over and take away
// (規約 §右のペインの字は掴める), and a rest opens the graph's own card over the commit (規約 §hover のツールチップ). The
// card is the graph's — same component, same fields, filled from the same row this list was built out of — because a
// commit does not read differently for being listed somewhere else.
Item {
    id: commitRow

    /// One row as `GraphModel.chosenRows` packed it, already taken apart by the pane.
    required property var modelData
    /// The commit whose card the page has out. The row it came from keeps its band while the card stands: the card
    /// opens off the row's own bottom edge, so the hand that walks down into it is off the row from that moment
    /// (`RowHoverHost.rowCardOid`).
    property string cardOid: ""

    /// The names the card's opener reads off a row, whichever list the row is in (`RowHoverHost.openRowCard`).
    readonly property string oid_hex: commitRow.modelData.oid
    readonly property string subject: commitRow.modelData.subject
    readonly property string body: commitRow.modelData.body
    readonly property string author: commitRow.modelData.author
    readonly property double atime: commitRow.modelData.atime
    readonly property string co_authors: commitRow.modelData.mates
    /// **-1, and not this row's place in the list.** The number that travels with a card is a row of the graph, which
    /// is what a press in it lands on; this list has its own numbering and handing that over would pick a commit at
    /// random (`RepoPage.activateRow` looks the commit up when it is given -1).
    readonly property int index: -1
    /// Where the pointer is along the row, so the card opens under the hand rather than at the row's left edge.
    readonly property real pointerX: rowHover.point.position.x

    signal hoverRequested(var row, bool inside)

    width: ListView.view ? ListView.view.width : 0
    height: Theme.rowHeight

    Rectangle {
        anchors.fill: parent
        color: Theme.bgHover
        visible: rowHover.hovered || commitRow.cardOid === commitRow.oid_hex
    }

    // The hand a range selection is taken with, under everything the row draws: a press reaches it only where no value
    // took one, which is every gap in the row (規約 §右のペインの字は掴める). Declared before the content so it lies
    // beneath it.
    SweepRoom {
        id: sweepHand
        anchors.fill: parent
        row: commitRow
    }

    // ---- what a sweep over this row needs to know (`SweepRoom`) ------
    //
    // One line, so there is no border to find: everything the row draws stands on it.
    function lineAt(y) {
        return 0
    }
    function lineValues(line) {
        return [subjectLine, shaLine]
    }
    /// Nothing on this row is a control, so no press belongs to one.
    function claimedAt(item, x, y) {
        return false
    }
    /// One selection in the window (規約 §右のペインの字は掴める), so a sweep clears this row before it puts one anywhere.
    function dropValues() {
        subjectLine.deselect()
        shaLine.deselect()
    }
    /// Automation only: the sweep as a hand makes it, and what it came away with (verify-ui).
    readonly property alias sweep: sweepHand
    /// The field one value is drawn in, the slack a hand reaches it through, and the band its line runs down — **the
    /// whole row**, the air over and under a value being the row's too (`SweepRoom.sweepAt`).
    function fieldFor(which) {
        return which === "sha" ? shaLine : subjectLine
    }
    function slackFor(which) {
        return which === "sha" ? shaSlack : subjectSlack
    }
    function bandFor(which) {
        return [0, commitRow.height]
    }

    IdentIcon {
        id: face
        anchors.left: parent.left
        anchors.leftMargin: Theme.spaceXs
        anchors.verticalCenter: parent.verticalCenter
        code: commitRow.modelData.avatar
        imageUrl: commitRow.modelData.avatarUrl
    }
    /// Whether the face has what it will paint — a shot of this list waits on it, the picture being read off disk
    /// (`IdentIcon.pictureReady`).
    function faceReady() {
        return face.pictureReady()
    }

    LineText {
        id: subjectLine
        anchors.left: face.right
        anchors.leftMargin: Theme.spaceSm
        anchors.right: shaLine.left
        anchors.rightMargin: Theme.spaceSm
        anchors.verticalCenter: parent.verticalCenter
        // A subject is read from the left, and nothing after the mark is worth keeping.
        cutAt: "end"
        ground: Theme.bgBase
        text: commitRow.subject
    }

    LineText {
        id: shaLine
        anchors.right: parent.right
        anchors.rightMargin: Theme.spaceXs
        anchors.verticalCenter: parent.verticalCenter
        width: Math.min(implicitWidth, commitRow.width / 3)
        // git's own spelling wears git's own family (規約 §git 用語のコード表記).
        mono: true
        cutAt: "end"
        ground: Theme.bgBase
        color: Theme.textMuted
        text: commitRow.modelData.sha8
    }

    // The gaps a hand reaches the values through. Declared rather than left as bare margins so the row keeps them and
    // a run can aim at them: a layout that closed them would leave the words reachable only on the glyphs themselves,
    // which is the corner readers actually run into (規約 §右のペインの字は掴める).
    Item {
        id: subjectSlack
        anchors.left: face.right
        anchors.right: subjectLine.left
        anchors.top: parent.top
        anchors.bottom: parent.bottom
    }
    Item {
        id: shaSlack
        anchors.left: subjectLine.right
        anchors.right: shaLine.left
        anchors.top: parent.top
        anchors.bottom: parent.bottom
    }

    // Passive, and on the row's own root: a handler here leaves the fields under it their presses, and hover reaches
    // this row's children rather than being eaten by a layer over them (app-ui.md).
    HoverHandler {
        id: rowHover
        onHoveredChanged: {
            if (rowHover.hovered) {
                restDelay.restart()
                return
            }
            restDelay.stop()
            commitRow.hoverRequested(commitRow, false)
        }
    }
    // The card comes out on a rest, not on arrival — the same beat the graph's rows keep (規約 §hover のツールチップ).
    Timer {
        id: restDelay
        interval: Metrics.tipDelayMs
        onTriggered: commitRow.hoverRequested(commitRow, true)
    }
    /// Automation only: hover cannot be injected, so a run enters where this timer would (verify-ui).
    function askCard() {
        commitRow.hoverRequested(commitRow, true)
    }
}
