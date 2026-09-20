import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The unified ⇄ split view toggle the diff pane's band carries: one column, or the old side beside the new
// (デザイン規約 §diff を 2 列で読む). The same pair the file lists' tree ⇄ paths toggle is — one lit, the other a
// step down in the same hue — so a reader who has met one has met both.
//
// **The marks are the two shapes the rows take.** `unified` is a frame with a rule across it: one column, the old
// and the new stacked. `split` is the same frame with the rule down the middle: two columns. Drawn on the
// family's own grid (`NavIcon`), after the two Octicons every diff reader knows the pair by (`rows` / `columns`).
RowLayout {
    id: toggle

    /// Whether the diff is showing as two columns.
    property bool split: false
    signal chosen(bool split)

    Layout.fillHeight: true
    // Nothing between the two, for the reason the tree toggle has none (`TreeViewToggle`).
    spacing: 0

    ViewMarkButton {
        Layout.fillHeight: true
        mark: "unified"
        lit: !toggle.split
        tip: qsTr("Unified view")
        onClicked: toggle.chosen(false)
    }
    ViewMarkButton {
        Layout.fillHeight: true
        mark: "split"
        lit: toggle.split
        tip: qsTr("Split view")
        onClicked: toggle.chosen(true)
    }
}
