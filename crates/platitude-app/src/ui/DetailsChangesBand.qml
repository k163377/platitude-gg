import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// CHANGES header with the tree ⇄ path view toggle — the file list's own
// band. The caller keeps the layout seat and the visibility (a list whose
// heading has scrolled away is a list of nothing in particular).
Rectangle {
    id: changesBand

    /// How many files the commit touched.
    required property int count
    /// Whether the list below is showing the tree.
    required property bool treeView
    /// What the band calls them. The default is the commit's word; the pane that reads another working copy heads the
    /// same list with the word that window's own pane uses, because they are the same kind of thing.
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
            font.weight: Font.DemiBold
            color: Theme.textSecondary
        }
        // The count stands at the caption's own step, the way every heading band in the window carries its own
        // (NavHeader, CommandsPane, WipBucketHeader) — the weight and the colour say it is a count (規約 §タイポグラフィ).
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
