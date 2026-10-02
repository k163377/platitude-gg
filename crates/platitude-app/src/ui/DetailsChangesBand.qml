import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The file list's CHANGES band with the tree ⇄ path toggle. The caller keeps the layout seat and the visibility.
Rectangle {
    id: changesBand

    /// How many files the commit touched.
    required property int count
    /// Whether the list below is showing the tree.
    required property bool treeView
    /// The band's word — the commit's by default; `CarriedPane` passes the one the WIP pane uses.
    property string caption: qsTr("CHANGES")

    signal chosen(bool tree)

    implicitHeight: Theme.headerHeight
    color: Theme.bgElevated
    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceXs
        anchors.rightMargin: Theme.spaceXs
        spacing: Theme.spaceXs
        Label {
            text: changesBand.caption
            font.pixelSize: Theme.fontMd
            font.weight: Theme.fontWeightStrong
            color: Theme.textSecondary
        }
        // The caption's size; weight and colour mark it as a count (規約 §タイポグラフィ).
        Label {
            text: "(" + changesBand.count + ")"
            font.pixelSize: Theme.fontMd
            color: Theme.textMuted
        }
        Item { Layout.fillWidth: true }
        TreeViewToggle {
            treeView: changesBand.treeView
            onChosen: tree => changesBand.chosen(tree)
        }
    }
}
