pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Ctrl+F over the graph: a bar that comes down from the top of the list
// and pushes its rows down rather than covering them, the way a standing
// question does (デザイン規約 §可否・警告の出し場所).
//
// It is not a question, and the difference is worth keeping visible: no
// answer is waited on, nothing is at stake, and nothing happens when it
// is sent away. So it wears the accent line rather than a state colour,
// carries no pill, and stays up until it is dismissed. Only one bar comes
// down at a time — a question already standing keeps the place, because
// it is one gesture away from being over.
//
// The frame only for now. What is typed here is not read yet and the
// count stands at nothing; both are wired when the search behind them is
// (P3-確認事項 §ウィンドウ chrome の要判断).
Rectangle {
    id: findBar

    /// Whether the bar is down. Nothing else opens or shuts it.
    property bool open: false
    /// What has been typed, for whoever comes to read it. Writable so the
    /// headless run can put something in the box — the key that opens this
    /// bar cannot be pressed from there, and neither can the letters.
    property alias query: field.text
    /// How many rows the query matched, and which of them the view is on.
    /// Both stay at zero while nothing is doing the matching, and the
    /// count keeps its place empty rather than saying "0 / 0".
    property int matches: 0
    property int atMatch: 0

    signal dismissed()

    /// Brings the bar down and puts the caret in it. Raising a bar that is
    /// already down just takes the focus back and selects what is there,
    /// which is what pressing the key twice should do.
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

    implicitHeight: findBar.open ? Theme.headerHeight : 0
    clip: true
    color: Theme.bgElevated
    Behavior on implicitHeight {
        NumberAnimation { duration: 200 }
    }

    // The line a bar that came down from an edge closes itself with. The
    // accent rather than a state colour: this one is not reporting on
    // anything (デザイン規約 §状態).
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: Theme.borderWidth
        color: Theme.accent
    }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceSm
        anchors.rightMargin: Theme.spaceXs
        spacing: Theme.spaceSm

        SlimField {
            id: field
            // The fixed width the design document gives a search box
            // (§レイアウト初期値). Not filled to the bar: what goes in is a
            // few words, and a box the width of the window would say
            // otherwise.
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
        Item { Layout.fillWidth: true }
        HoverToolButton {
            padding: 0
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: Theme.iconLg
            implicitHeight: Theme.iconLg
            Accessible.name: qsTr("Close the find bar")
            contentItem: Item {
                NavIcon {
                    anchors.centerIn: parent
                    width: Theme.iconMd
                    height: Theme.iconMd
                    kind: "close"
                    tint: Theme.textPrimary
                }
            }
            onClicked: findBar.dismiss()
        }
    }
}
