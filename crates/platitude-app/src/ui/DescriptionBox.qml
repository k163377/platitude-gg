import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The description box, in both places a commit message is written or read
// (デザイン規約 §コミットメッセージの 2 つの枠). Two lines tall at rest,
// dim until a caret is in it, and the one box in the app that can be
// pulled taller by the grip in its corner: a description worth reading is
// routinely longer than the shared cap, while a summary that long is an
// accident.
//
// The pane it sits in owns the bound. It measures how much room is still
// free below the box (`room`) and how far it is already past its own edge
// (`owed`) — nothing here knows what a file list or an author card is.
Rectangle {
    id: box

    /// How much taller the box may still be drawn before the pane runs
    /// out of what it was willing to lend.
    property real room: 0
    /// How much the pane needs back. The far side of the same measure:
    /// the window shrinks, a panel opens, a row appears, and what the
    /// hand was given has to be returned.
    property real owed: 0

    property alias text: area.text
    property alias readOnly: area.readOnly
    property alias placeholderText: area.placeholderText
    /// What the box paints — read by the smoke hooks, which report the
    /// painted side rather than the input side (see app-ui.md).
    readonly property color textColor: area.color
    readonly property bool focused: area.activeFocus

    /// Where the hand has put the ceiling. Kept while the pane lives, so
    /// reading down a run of long messages does not mean pulling once per
    /// commit; not written down, because it belongs to the reading rather
    /// than to the repository.
    property real cap: Theme.messageMaxHeight
    /// What the text would take if nothing capped it. The two-line floor
    /// is the box's resting shape, empty or not.
    readonly property real wants:
        Math.max(area.implicitHeight, 2 * Theme.fontMdLine) + Theme.spaceSm
    /// What the box is given: its own text, never more — a ceiling held
    /// above short text would leave an empty frame on every message after
    /// the long one that earned it.
    readonly property real boxHeight:
        Math.min(box.wants, Math.max(Theme.messageMaxHeight, box.cap))
    /// Offered exactly while the cap is what stands between the reader and
    /// the rest of the text — and it keeps standing there once the text is
    /// out, because putting the box back is the same grip.
    readonly property bool grips:
        box.wants > Theme.messageMaxHeight
        && (box.room > 0 || box.cap > Theme.messageMaxHeight)

    /// The one place the ceiling moves. The drag and the smoke hook both
    /// come through here, so neither can reach a height the other is
    /// refused.
    function setBoxHeight(want) {
        box.cap = Math.max(Theme.messageMaxHeight,
                           Math.min(want, box.wants, box.boxHeight + box.room))
    }
    /// Smoke hook: pull the grip down by dy, the way a drag does, and
    /// through the same clamp. Headless has no pointer at all.
    function grow(dy) {
        box.setBoxHeight(box.boxHeight + dy)
    }
    /// Smoke hook: the caret in the box, the way a click puts it there.
    /// Not `focus()` -- Item already has a `focus` property, and the name
    /// resolves to that one, so the call is a TypeError at the point it
    /// is made rather than at the point it is written.
    function takeCaret() {
        area.forceActiveFocus()
    }
    // The pane gives its room back rather than letting its own column run
    // under its edge — the accident the shared cap was put there to stop,
    // and one no screenshot shows.
    onOwedChanged: {
        if (box.owed > 0)
            box.setBoxHeight(box.boxHeight - box.owed)
    }
    // TextEdit draws only the part of itself its viewport can see, and a
    // viewport that grows without scrolling never tells it so: the box
    // opens and the text stops on the line the old height ended at, with
    // the rest of the frame empty (measured). Nudging the flickable and
    // putting it straight back is what says "look again" -- the text item
    // is watching for the viewport to move under it, which is the one
    // thing a resize does not do.
    onBoxHeightChanged: Qt.callLater(box.repaint)
    function repaint() {
        const flick = view.contentItem
        const was = flick.contentY
        flick.contentY = was + 1
        flick.contentY = was
    }

    Layout.fillWidth: true
    Layout.preferredHeight: box.boxHeight
    color: Theme.bgBase
    radius: Theme.radiusMd
    border.color: Theme.borderSubtle
    border.width: Theme.borderWidth

    ScrollView {
        id: view
        anchors.fill: parent
        anchors.margins: Theme.spaceXs
        // ScrollView keeps its Flickable private -- reach it once it
        // exists.
        Component.onCompleted:
            contentItem.boundsBehavior = Flickable.StopAtBounds
        TextArea {
            id: area
            wrapMode: TextArea.Wrap
            font.pixelSize: Theme.fontMd
            // Dimmer than the summary while it is being read -- that pair
            // is the message's own hierarchy -- but never while it is
            // being written: text under a caret is what the eye is on, and
            // secondary is the shade this theme spends on what the eye is
            // not on. Read-only does not count as writing it: a stash and
            // a commit off this line take a caret for selecting, and
            // nothing typed there would land.
            color: !area.readOnly && area.activeFocus
                   ? Theme.textPrimary : Theme.textSecondary
            background: null
            padding: 0
        }
    }
    // Declared after the ScrollView, so the corner belongs to the grip
    // rather than to the last few pixels of the scrollbar. Hit area and
    // ink are the one 16 square: the mark hangs off its lower-right,
    // which puts the ink on the same inset the text keeps from the frame.
    MouseArea {
        id: grip
        visible: box.grips
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        width: Theme.iconMd
        height: Theme.iconMd
        hoverEnabled: true
        // Down is the only way this goes, so the cursor says so rather
        // than promising the corner drag a browser's box takes.
        cursorShape: Qt.SizeVerCursor
        /// Where the pull started, in the pane's own coordinates — the
        /// grip itself travels with the edge it is moving.
        property real fromY: 0
        property real fromHeight: 0
        onPressed: mouse => {
            grip.fromY = mapToItem(box.parent, 0, mouse.y).y
            grip.fromHeight = box.boxHeight
        }
        onPositionChanged: mouse => {
            if (!grip.pressed)
                return
            box.setBoxHeight(grip.fromHeight
                             + mapToItem(box.parent, 0, mouse.y).y - grip.fromY)
        }
        NavIcon {
            anchors.fill: parent
            kind: "grip"
            // A structural line at rest, since what it marks is an edge
            // and not a word; the pointer brings it up to the shade the
            // pane spends on things being read.
            tint: grip.containsMouse || grip.pressed
                  ? Theme.textSecondary : Theme.borderStrong
        }
    }
}
