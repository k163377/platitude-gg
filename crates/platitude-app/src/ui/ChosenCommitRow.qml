pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// One commit of a choice, as the right pane lists it: face, summary cut to one line, and the hash plate. The only press
// the row takes is a Ctrl one, which drops the commit; its words are fields (デザイン規約 §複数のコミットを選ぶ).
Item {
    id: commitRow

    /// One row as `GraphModel.chosenRows` names it (`oid`, `sha8`, `subject`, `body`, `author`, `atime`, `avatar`,
    /// `avatarUrl`, `mates`).
    required property var modelData
    /// The commit whose card is out; its row keeps its band while the card stands over it.
    property string cardOid: ""

    /// The names the card's opener reads off a row, whichever list it is in (`RowHoverHost.openRowCard`).
    readonly property string oid_hex: commitRow.modelData.oid
    readonly property string subject: commitRow.modelData.subject
    readonly property string body: commitRow.modelData.body
    readonly property string author: commitRow.modelData.author
    readonly property double atime: commitRow.modelData.atime
    readonly property var co_authors: commitRow.modelData.mates
    /// Always -1: the index a card carries is a graph row, and this list's own numbering would pick a commit at random
    /// (`RepoPage.activateRow` looks the commit up when given -1).
    readonly property int index: -1
    /// Where the pointer is along the row, so the card opens under the hand.
    readonly property real pointerX: rowHover.point.position.x
    /// The row cut the message to one line, so its card shows it whole (`RowHoverHost.openRowCard`).
    readonly property bool wholeMessage: true

    signal hoverRequested(var row, bool inside)
    signal copyRequested(string text)
    /// A Ctrl press takes this commit back out of the choice.
    signal dropRequested(string oidHex)
    /// Whether a press is the row's own: only a Ctrl one is. A refused press goes on down to the words and the hand
    /// under them, which keeps a plain drag a drag over the text.
    function takesPress(modifiers) {
        return (modifiers & Qt.ControlModifier) !== 0
    }
    /// The click that follows a press the row took, asked the same question so an entry here cannot skip it. Named so
    /// a run presses where a hand does (verify-ui §壊れない動詞の実装と反復).
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

    // The hand that drags over the words from the air around them. Declared first so it lies beneath, and a press
    // reaches it only where no field took one (規約 §右のペインの字は掴める).
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
        // The scroll bar's gutter: the bar is opaque, and a row reaching under it would hide its hash.
        anchors.rightMargin: Theme.navBarGutter

        IdentIcon {
            id: face
            anchors.left: parent.left
            anchors.verticalCenter: parent.verticalCenter
            code: commitRow.modelData.avatar
            imageUrl: commitRow.modelData.avatarUrl
        }
        // No parent row under the plate: that is a question for the pane that shows one commit.
        HashPlate {
            id: plate
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            sha8: commitRow.modelData.sha8
            fullSha: commitRow.modelData.oid
            // A step under the summary beside it (デザイン規約 §複数のコミットを選ぶ).
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
            // A summary is read from the left; the whole of it stays in the field for a drag.
            cutAt: "end"
            // The two layers this row paints, in that order — the mark's opaque patch in the wrong colour is a box
            // around the `…`.
            ground: Theme.bgSurface
            groundOverlay: commitRow.lit ? Theme.bgHover : "transparent"
            text: commitRow.subject
        }
    }

    // Declared last, so the Ctrl press it takes is taken before the words can select on it; every other press is
    // refused and goes on down as if this were not here. Deaf to hover, which would take the pointer from the fields
    // and the row's handler (rules-refs/app-ui.md「行に重ねる面の `HoverHandler` は祖先が持つ」).
    MouseArea {
        id: dropHand
        anchors.fill: parent
        hoverEnabled: false
        // The row keeps the drag it is handed (rules-refs/app-ui.md「行は渡されたドラッグを手放さない」).
        preventStealing: true
        onPressed: mouse => { mouse.accepted = commitRow.takesPress(mouse.modifiers) }
        onClicked: mouse => commitRow.leftClick(mouse.modifiers)
    }
    /// Automation only: that the hand stands, since a run enters `leftClick` itself and would pass with the area gone.
    readonly property bool dropStands: dropHand.enabled && dropHand.width === commitRow.width
                                       && dropHand.height === commitRow.height

    // Passive, on the row's root: it leaves the fields their presses and still sees hover over its children
    // (rules-refs/app-ui.md「行の hover は `HoverHandler`」).
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
    // The card comes out on a rest (規約 §hover のツールチップ).
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
    /// Whether the summary was cut — read back, since the mark is too small to judge on a picture.
    readonly property alias summaryCut: subjectLine.clipped
}
