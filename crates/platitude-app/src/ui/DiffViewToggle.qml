import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The unified ⇄ split view toggle on the diff pane's band — the same pair as the file lists' tree ⇄ paths toggle: the
// lit half is the way the rows are read now (`ViewMarkButton`). Its seat is where the band's air ends, before
// `Stage file` (`DiffPaneHeader`'s end trio). **On every diff, pictures and binaries included**: a quiet control that
// comes and goes per file moves everything right of it on the band.
// The words are GitHub's PR setting's (`Unified view` / `Split view`). The marks draw the two shapes the rows take
// (`NavIcon`, after Octicons' `rows` / `columns` on this family's 16 grid). The choice is the machine's, one column by
// default (`LayoutState.diff_split`); the rows are laid out again in place, the reading position held
// (`DiffRowDelegate`).
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
