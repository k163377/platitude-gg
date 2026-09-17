pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// One commit of a choice, as the pane on the right lists it (デザイン規約 §複数のコミットを選ぶ): the face of whoever
// wrote it, its summary, and its short id in the plate every hash in this window is copied from.
//
// **The summary is what a row shows, and only that.** A choice of a dozen is a list to run an eye down, so the rows
// stay one line each and what does not fit is cut; the description and the whole of a cut summary are read in the card
// a rest opens. What is written here in full was measured to take the pane over — a 2,000 byte summary filled the list
// on its own and left the others below the fold.
//
// **The only press the row takes is a held one.** Commits are picked in the graph
// and this list is what was picked, so there is nothing here to select or aim a menu at — but the list of what is held
// is where a reader looks to drop one, so Ctrl takes a commit back out from here as it does there. Its words are
// fields all the same — dragged over and taken away like the rest of the pane (規約 §右のペインの字は掴める).
Item {
    id: commitRow

    /// One row as `GraphModel.chosenRows` packed it, already taken apart by the pane.
    required property var modelData
    /// The commit whose card is out. The row it came off keeps its band while the card stands over it: the card opens
    /// off the row's own bottom edge, so the hand walking down into it is off the row from that moment.
    property string cardOid: ""

    /// The names the card's opener reads off a row, whichever list it is in (`RowHoverHost.openRowCard`).
    readonly property string oid_hex: commitRow.modelData.oid
    readonly property string subject: commitRow.modelData.subject
    readonly property string body: commitRow.modelData.body
    readonly property string author: commitRow.modelData.author
    readonly property double atime: commitRow.modelData.atime
    readonly property string co_authors: commitRow.modelData.mates
    /// **Always -1.** The number that travels with a card is a row of the graph, which
    /// is what a press in one lands on; this list has its own numbering and handing that over would pick a commit at
    /// random (`RepoPage.activateRow` looks the commit up when it is given -1).
    readonly property int index: -1
    /// Where the pointer is along the row, so the card opens under the hand.
    readonly property real pointerX: rowHover.point.position.x
    /// This row showed one cut line of the message, so its card is where the message is read: nothing held back, and
    /// no way out of it offered (`RowHoverHost.openRowCard`, デザイン規約 §複数のコミットを選ぶ).
    readonly property bool wholeMessage: true

    signal hoverRequested(var row, bool inside)
    signal copyRequested(string text)
    /// A held press takes this commit back out of the choice — the same modifier that put it in, in the list that
    /// shows what is in it (デザイン規約 §複数のコミットを選ぶ).
    signal dropRequested(string oidHex)
    /// Whether a press is the row's own at all: **only a held one is**. A press without the modifier is refused, and
    /// a refused press goes on down to the words and the hand under them — which is what keeps a plain drag over this
    /// list a drag over the text (規約 §右のペインの字は掴める).
    function takesPress(modifiers) {
        return (modifiers & Qt.ControlModifier) !== 0
    }
    /// The click that follows a press the row took. Asked the same question the press was, so an entry made here
    /// cannot skip it. Both are named so a run presses where a hand does (verify-ui §壊れない動詞の実装).
    function leftClick(modifiers) {
        if (commitRow.takesPress(modifiers))
            commitRow.dropRequested(commitRow.oid_hex)
    }

    width: ListView.view ? ListView.view.width : 0
    height: Theme.rowHeight

    /// The row is wearing the hover wash — under the pointer, or under the card it put out.
    readonly property bool lit: rowHover.hovered || commitRow.cardOid === commitRow.oid_hex
    Rectangle {
        anchors.fill: parent
        color: Theme.bgHover
        visible: commitRow.lit
    }

    // The hand the words are dragged over from the air around them, under everything the row draws: a press reaches it
    // only where no field and no control took one (規約 §右のペインの字は掴める). Declared first so it lies beneath.
    SweepPad {
        id: sweepHand
        anchors.fill: parent
        content: block
    }
    /// Automation only: the sweep as a hand makes it, and what it came away with (verify-ui).
    readonly property alias sweep: sweepHand

    Item {
        id: block
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceXs
        // The gutter this list's own scroll bar is drawn in. The bar is the pane's own slab — ink against
        // the edge, opaque — and a row that ended short of this would stand its hash under it (`FileRowDelegate` and
        // the left panel's rows take the same one).
        anchors.rightMargin: Theme.navBarGutter

        IdentIcon {
            id: face
            anchors.left: parent.left
            anchors.verticalCenter: parent.verticalCenter
            code: commitRow.modelData.avatar
            imageUrl: commitRow.modelData.avatarUrl
        }
        // The plate every hash in this window is copied from, with no parent row under it: what a row of this list is
        // about is the commit, and where it came from is a question for the pane that shows one at a time.
        HashPlate {
            id: plate
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            sha8: commitRow.modelData.sha8
            fullSha: commitRow.modelData.oid
            // A step under the summary beside it: the words are what a row is read by, and the id is what tells two
            // of them apart once they have been read (デザイン規約 §複数のコミットを選ぶ).
            shaColor: Theme.textMuted
            shaSize: Theme.fontCode
            onCopyRequested: text => commitRow.copyRequested(text)
        }
        LineText {
            id: subjectLine
            anchors.left: face.right
            anchors.leftMargin: Theme.spaceSm
            anchors.right: plate.left
            anchors.rightMargin: Theme.spaceSm
            anchors.verticalCenter: parent.verticalCenter
            // A summary is read from the left, and nothing after the mark is worth keeping. The whole of it stays in
            // the field for a drag to take, and the card has it too (規約 §右のペインの字は掴める).
            cutAt: "end"
            // The two layers this row paints, in that order — the mark is drawn on an opaque patch, and a patch in
            // the wrong colour is a box around the `…`.
            ground: Theme.bgSurface
            groundOverlay: commitRow.lit ? Theme.bgHover : "transparent"
            text: commitRow.subject
        }
    }

    // Over everything the row draws, and **it refuses all but a held press**: a refused press is not this area's, so
    // it goes on down to the words and the hand under them exactly as if this were not here. Declared last so the one
    // press it does take is taken before the words can select on it.
    //
    // Deaf to hover: an area that asked for it would take the pointer from the fields below and from the row's own
    // handler (app-ui.md §HoverHandler は下の hover を殺す), and it draws no cursor of its own either.
    MouseArea {
        id: dropHand
        anchors.fill: parent
        hoverEnabled: false
        onPressed: mouse => { mouse.accepted = commitRow.takesPress(mouse.modifiers) }
        onClicked: mouse => commitRow.leftClick(mouse.modifiers)
    }
    /// Automation only: that there is a hand at all, since a run enters `leftClick` itself — an area
    /// taken out, disabled or shrunk would answer every press it was asked and never see one (verify-ui).
    readonly property bool dropStands: dropHand.enabled && dropHand.width === commitRow.width
                                       && dropHand.height === commitRow.height

    // Passive, and on the row's own root: a handler here leaves the fields under it their presses, and hover reaches
    // this row's children (app-ui.md).
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
    // The card comes out on a rest — the same beat the graph's rows keep (規約 §hover のツールチップ).
    Timer {
        id: restDelay
        interval: Metrics.tipDelayMs
        onTriggered: commitRow.hoverRequested(commitRow, true)
    }
    /// Automation only: hover cannot be injected, so a run enters where this timer would (verify-ui).
    function askCard() {
        commitRow.hoverRequested(commitRow, true)
    }
    /// Whether the face has what it will paint and the words have been laid out — a shot of this list waits on both.
    function rowReady() {
        return face.pictureReady() && subjectLine.width > 0
    }
    /// Whether the summary ran past the room the row gave it. Read back: the mark is a
    /// few pixels wide, and a row that cut its summary frames the same as one that did not.
    readonly property alias summaryCut: subjectLine.clipped
}
