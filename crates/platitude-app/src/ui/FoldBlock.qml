import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The block at the end of the left pane's header band that folds the menu to its icons and back; the whole block is the
// button. Its own ground: on the bare ground of the filter beside it, it would read as part of the input.
//
// Folding and unfolding are one seat: a `headerHeight` square, the hit the band's full height; folded, the band is
// this block alone. The mark is the drawn double chevron (`chevrons`), only its direction differs — left folds, right
// unfolds. Lines, not glyphs: guillemets ride the lowercase band, so a centred line box drops their ink, and the
// triangles are not in the UI face (a different shape on each OS). The block keeps the band's `headerHeight` though
// the rail's cells below are `toolbarHeight` tall (that is the hand's height): the panes beside start where it ends
// (規約 §QML 実装ルール). The column's other end holds the same box, the `>_` (`CommandsToggle`).
//
// Hover washes the ground and says nothing (user decision — ask before changing), an exception to 規約 §hover の
// ツールチップ: the double chevron is the fold idiom itself, the answer shows in place, and a second press undoes it.
// The name stays on `Accessible.name` for whoever cannot read the mark. The rail's own block is this one too. The wash
// is the one overlay rows, bands and cells wear, square-cornered.
Rectangle {
    id: block
    /// Pressing folds the list (true) or unfolds it.
    property bool folds: true
    /// Draws the left hairline. False when folded: the block is then the whole band, at the window's edge.
    property bool divided: true
    signal activated()

    color: Theme.bgElevated

    HoverToolButton {
        anchors.fill: parent
        padding: 0
        // Square, like every wash it stands among.
        washRadius: 0
        Accessible.name: block.folds ? qsTr("Fold the list to its icons") : qsTr("Unfold the list")
        // A box of its own so the mark keeps `iconLg`, the size of the section icons beside it on the folded rail.
        contentItem: Item {
            NavIcon {
                anchors.centerIn: parent
                kind: "chevrons"
                tint: Theme.textSecondary
                width: Theme.iconLg
                height: Theme.iconLg
                rotation: block.folds ? 180 : 0
            }
        }
        onClicked: block.activated()
    }
    Rectangle {
        visible: block.divided
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: Theme.borderWidth
        color: Theme.borderSubtle
    }
}
