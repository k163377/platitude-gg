import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui
import platitude.ui.face

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

    /// How far the caption moves up to stand its capitals on the band's middle (`WordFace.capitalDropOf`; デザイン規約
    /// §アイコンの席「大文字だけの見出しの語」); its count goes with it.
    readonly property real capsLift:
        -WordFace.capitalDropOf(changesCaption.baselineOffset, changesCaption.pixelSize, changesCaption.height)

    implicitHeight: Theme.headerHeight
    color: Theme.bgElevated
    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceXs
        anchors.rightMargin: Theme.spaceXs
        spacing: Theme.spaceXs
        // The words give way to a narrow pane, the count and the toggle never do (デザイン規約 §窓の床).
        CutName {
            id: changesCaption
            // A line in no column (`closed`): the mark straight after the word it cut.
            closed: true
            Layout.fillWidth: true
            Layout.maximumWidth: changesCaption.implicitWidth
            transform: Translate { y: changesBand.capsLift }
            text: changesBand.caption
            cutAt: "end"
            pixelSize: Theme.fontMd
            weight: Theme.fontWeightStrong
            color: Theme.textSecondary
        }
        // The caption's size; weight and colour mark it as a count (規約 §タイポグラフィ).
        PlainTextLabel {
            transform: Translate { y: changesBand.capsLift }
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
