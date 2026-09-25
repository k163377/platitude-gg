import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The settings screen's top band: the title, laid on the chapters' centred block, and the exit, hung off the band's
// right edge (規約 §設定の画面). A plain `Item`, because a RowLayout could only place the two in one run.
Item {
    id: head

    /// The screen's block width (`SettingsDialog.blockWidth`), so the title starts level with the rail.
    required property real blockWidth
    /// The category showing, for the second half of the title (`SettingsDialog.categories`).
    required property string word
    /// That category's mark, off the same list.
    required property string categoryIcon
    /// Something on the screen is holding an edit git has not been given, so the way out costs something.
    required property bool unsaved
    /// A press was made over that and turned down: the next one goes through.
    required property bool armed

    /// The `✕` was pressed.
    signal closed()

    Layout.fillWidth: true
    implicitHeight: Theme.toolbarHeight + 2 * Theme.spaceLg

    RowLayout {
        // The block's left edge plus the rail's lane, set here: a margin on the band would fall outside the centred
        // width and shift the title off the rail.
        anchors.left: parent.left
        anchors.leftMargin: Math.max(0, (head.width - head.blockWidth) / 2) + Theme.spaceXxl
        anchors.right: closeMark.left
        anchors.rightMargin: Theme.spaceSm
        anchors.verticalCenter: parent.verticalCenter
        spacing: Theme.spaceSm

        NavIcon {
            Layout.preferredWidth: Theme.iconLg
            Layout.preferredHeight: Theme.iconLg
            kind: "gear"
            tint: Theme.textPrimary
        }
        Label {
            text: qsTr("Settings")
            font.pixelSize: Theme.fontXl
            font.weight: Font.DemiBold
        }
        // The category, beside the title: the same step, told apart by weight and ink — one title in two halves
        // (規約 §タイポグラフィ 「重み・色・大文字が作る」).
        Rectangle {
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: Theme.borderWidth
            implicitHeight: Theme.iconLg
            color: Theme.borderDefault
        }
        NavIcon {
            Layout.preferredWidth: Theme.iconLg
            Layout.preferredHeight: Theme.iconLg
            kind: head.categoryIcon
            tint: Theme.textSecondary
        }
        Label {
            text: head.word
            font.pixelSize: Theme.fontXl
            color: Theme.textSecondary
        }
        Item { Layout.fillWidth: true }
    }
    // The `✕` alone; Escape is said in its tooltip (規約 §設定の画面).
    CloseToolButton {
        id: closeMark
        // Flush with the band's right edge, all of its air inside the seat (規約 §当たり判定).
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        // Seat and mark sizes: 規約 §設定の画面 (`iconSm` would read as an afterthought beside the title).
        seat: Theme.toolbarHeight
        markSize: Theme.iconMd
        // Warning ink while something is unsaved, yet always pressable; armed changes shape too (規約 §設定の画面).
        tone: head.armed || !head.unsaved ? Theme.textPrimary : Theme.warning
        armed: head.armed
        Accessible.name: qsTr("Close the settings")
        tip: head.armed ? qsTr("Press again to close without saving")
           : head.unsaved ? qsTr("An identity here has not been given to git")
           : qsTr("Escape closes this too")
        onClicked: head.closed()
    }
}
