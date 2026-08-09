pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Ctrl+F over the graph: a card that hangs from the top-right corner of
// the list, over it rather than above it.
//
// Not the shape a standing question takes (デザイン規約 §可否・警告の出し場所).
// A question pushes the rows down because what is being judged has to stay
// in sight; a search has no target row yet, covers a corner nobody is
// reading, and is gone the moment it is dismissed. Every browser puts it
// here, and that is the gesture people arrive with.
Rectangle {
    id: findBar

    /// Whether the card is up. Nothing else opens or shuts it.
    property bool open: false
    /// What has been typed, for whoever comes to read it. Writable so the
    /// headless run can put something in the box — the key that opens this
    /// card cannot be pressed from there, and neither can the letters.
    property alias query: field.text
    /// How many rows the query matched, and which of them the view is on.
    /// Both stay at zero while nothing is doing the matching, and the count
    /// keeps its place empty rather than saying "0 / 0".
    property int matches: 0
    property int atMatch: 0
    /// Rows loaded, which is the largest either half of the count can be.
    /// The count's place is held at that width for as long as the card is
    /// up: the box grows leftward from the count, so a count that resized
    /// itself as the matches narrowed would slide the caret out from
    /// under the hand still typing into it.
    property int loaded: 0
    /// Something was typed that nothing answers. The frame says so where
    /// the typing is and the reason waits in the tooltip
    /// (§可否・警告の出し場所) — an empty count would be read as "still
    /// thinking", and "0 / 0" spends the widest number the bar ever shows
    /// on the one state that has nothing to count.
    property bool refused: false
    /// Why nothing answered, when the loaded window is the reason.
    property string refusedTip: ""
    /// How wide this card may grow before it starts covering the thing
    /// underneath that has to stay readable — the first character of a
    /// commit message (§コミットを探す). The owner measures it, because
    /// the owner is where the columns are.
    property real maxWidth: 0

    signal dismissed()
    /// Enter and Shift+Enter: on to the next match, back to the previous.
    signal nextRequested()
    signal previousRequested()

    /// Brings the card up and puts the caret in it. Raising one that is
    /// already up takes the focus back and selects what is there, which is
    /// what pressing the key twice should do.
    function raise() {
        findBar.open = true
        field.forceActiveFocus()
        field.selectAll()
    }
    function dismiss() {
        if (!findBar.open)
            return
        findBar.open = false
        findBar.dismissed()
    }

    // Hangs from the edge it is anchored to, so the corners that meet it
    // stay square and the two below it are the card's own.
    bottomLeftRadius: Theme.radiusMd
    bottomRightRadius: Theme.radiusMd
    color: Theme.bgElevated
    border.width: Theme.borderWidth
    border.color: Theme.borderDefault

    // The box grows with what is in it and the card grows with the box,
    // up to the cap — a search anybody would want to read back is worth
    // the width, and the cap is what keeps the reading underneath.
    // Below the cap the card keeps its own minimum: a window too narrow
    // to hold both is a window where the bar has to stay usable, and the
    // rows it covers are covered from the right, tail first.
    //
    // Both terms are built from what the parts *want*, never from what
    // the card currently is: a width that reads its own width back
    // through a child's `Layout.maximumWidth` is a loop, and Qt settles
    // it by leaving the card at its minimum (measured — the box stopped
    // growing at 160 however long the query got).
    implicitWidth: Math.max(findBar.minWidth,
                            Math.min(findRow.wantedWidth + 2 * Theme.spaceSm,
                                     findBar.maxWidth))
    implicitHeight: Theme.headerHeight
    readonly property real minWidth: findRow.minWidth + 2 * Theme.spaceSm

    // Nothing is clipped on the way in or out — the card is there or it is
    // not, and what fades is the whole of it. A height that animates while
    // the contents keep their size is what makes an opening bar look
    // broken halfway through.
    visible: opacity > 0
    opacity: findBar.open ? 1 : 0
    Behavior on opacity {
        NumberAnimation { duration: 120 }
    }

    RowLayout {
        id: findRow
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceSm
        anchors.rightMargin: Theme.spaceXs
        spacing: Theme.spaceSm
        // What the row needs with the box at its narrowest — the card's
        // floor, and what the box is given back when the cap bites.
        readonly property real minWidth:
            findRow.fieldMinWidth + findRow.fixedWidth
        // What it would like, with the box grown to what is typed.
        readonly property real wantedWidth:
            Math.max(findRow.fieldMinWidth,
                     field.contentWidth + field.leftPadding + field.rightPadding)
            + findRow.fixedWidth
        // Everything to the right of the box, which never changes width.
        readonly property real fixedWidth:
            widest.implicitWidth + closeButton.implicitWidth + 2 * findRow.spacing

        // The fixed width the design document gives a search box
        // (§レイアウト初期値), which here is its floor rather than its
        // width.
        readonly property real fieldMinWidth: 160

        SlimField {
            id: field
            // The box is the only part with anything to give, so it
            // takes whatever the card ended up with — grown to the text
            // where there is room, cut back to its floor where the cap
            // bit.
            Layout.fillWidth: true
            Layout.minimumWidth: findRow.fieldMinWidth
            placeholderText: qsTr("Find commits")
            refused: findBar.refused
            ToolTip.visible: findBar.refused && hovered
            ToolTip.delay: Metrics.tipDelayMs
            ToolTip.text: findBar.refusedTip
            Keys.onEscapePressed: findBar.dismiss()
            // Enter walks the matches; nothing here is submitted, so the
            // key is free to mean what it means in every other find box.
            Keys.onPressed: event => {
                if (event.key !== Qt.Key_Return && event.key !== Qt.Key_Enter)
                    return
                if (event.modifiers & Qt.ShiftModifier)
                    findBar.previousRequested()
                else
                    findBar.nextRequested()
                event.accepted = true
            }
        }
        Label {
            id: count
            text: findBar.matches > 0
                  ? qsTr("%1 / %2").arg(findBar.atMatch).arg(findBar.matches)
                  : ""
            color: Theme.textSecondary
            font.pixelSize: Theme.fontSm
            // Held open at the widest it could ever be here, and read
            // from the right so the numbers grow away from the box
            // rather than into it.
            Layout.preferredWidth: widest.implicitWidth
            horizontalAlignment: Text.AlignRight
            Label {
                id: widest
                visible: false
                font: count.font
                text: qsTr("%1 / %2").arg(findBar.loaded).arg(findBar.loaded)
            }
        }
        HoverToolButton {
            id: closeButton
            padding: 0
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: Theme.iconLg
            implicitHeight: Theme.iconLg
            Accessible.name: qsTr("Close the find bar")
            contentItem: Item {
                // A step under what it closes (デザイン規約 §寸法).
                NavIcon {
                    anchors.centerIn: parent
                    width: Theme.iconSm
                    height: Theme.iconSm
                    kind: "close"
                    tint: Theme.textSecondary
                }
            }
            onClicked: findBar.dismiss()
        }
    }
}
