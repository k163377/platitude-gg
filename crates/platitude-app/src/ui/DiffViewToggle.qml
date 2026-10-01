import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The unified ⇄ split view toggle on the diff pane's band (デザイン規約 §diff を 2 列で読む) — the same pair as the
// file lists' tree ⇄ paths toggle. The marks draw the two shapes the rows take (`NavIcon`, after Octicons' `rows` /
// `columns`).
RowLayout {
    id: toggle

    property bool split: false
    signal chosen(bool split)

    Layout.fillHeight: true
    // Nothing between the two, for the reason the tree toggle has none (`TreeViewToggle`).
    spacing: 0

    ViewMarkButton {
        Layout.fillHeight: true
        mark: "unified"
        showing: !toggle.split
        tip: qsTr("Unified view")
        onClicked: toggle.chosen(false)
    }
    ViewMarkButton {
        Layout.fillHeight: true
        mark: "split"
        showing: toggle.split
        tip: qsTr("Split view")
        onClicked: toggle.chosen(true)
    }
}
