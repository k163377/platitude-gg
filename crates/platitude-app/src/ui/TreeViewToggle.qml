import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The tree ⇄ paths toggle on both file lists' header band. No word stands beside the pair, so each mark is its
// cell's content and takes the step the cell's height leaves (デザイン規約 §寸法「印がセルの中身そのものである時」).
// The hit area runs the band's full depth; the wash keeps the mark's box (§当たり判定「広げるのは判定だけ」).
RowLayout {
    id: toggle

    property bool treeView: false
    signal chosen(bool tree)

    Layout.fillHeight: true
    // Nothing between the two: the marks' own air either side of the ink is the gap
    // (デザイン規約 §余白「印が自分で持っている余白は、隣の詰めに数える」).
    spacing: 0

    ViewMarkButton {
        Layout.fillHeight: true
        mark: "hier"
        lit: toggle.treeView
        tip: qsTr("Tree view")
        onClicked: toggle.chosen(true)
    }
    ViewMarkButton {
        Layout.fillHeight: true
        mark: "list"
        lit: !toggle.treeView
        tip: qsTr("Paths view")
        onClicked: toggle.chosen(false)
    }
}
