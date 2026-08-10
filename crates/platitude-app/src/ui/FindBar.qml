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
//
// The frame only for now. What is typed here is not read yet and the count
// stands at nothing; both are wired when the search behind them is
// (P3-確認事項 §ウィンドウ chrome の要判断).
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

    signal dismissed()

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

    implicitWidth: findRow.implicitWidth + 2 * Theme.spaceSm
    implicitHeight: Theme.headerHeight

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

        SlimField {
            id: field
            // The fixed width the design document gives a search box
            // (§レイアウト初期値).
            implicitWidth: 160
            placeholderText: qsTr("Find commits")
            Keys.onEscapePressed: findBar.dismiss()
        }
        Label {
            text: findBar.matches > 0
                  ? qsTr("%1 / %2").arg(findBar.atMatch).arg(findBar.matches)
                  : ""
            color: Theme.textSecondary
            font.pixelSize: Theme.fontSm
        }
        HoverToolButton {
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
