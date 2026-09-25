pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// What a pending file's row says about its line endings when the pointer rests on it: the full path, and the same
// sentence the diff pane puts above the hunks.
//
// A card, not a tooltip: a tooltip holds one string in one colour, and only the second line here is warning-coloured.
//
// Owned by the pane: rows are recycled the moment they scroll off, and a popup parented to a row goes with it.
AppCard {
    id: eolCard

    /// The file the card is about, spelled in full.
    property string path: ""
    /// The whole sentence, already worded (`Words.lineEndings`).
    property string notice: ""

    // The hand walks in to take the path away, so both halves of `AppCard.pointerInside` are wanted
    // (rules-refs/app-ui.md「hover で開くものの 5 つの罠」(2)).
    tracksPointer: true
    contentPointed: contentHover.hovered
    // Every gap in this card is a place a selection can start (規約 §hover のツールチップ).
    textContent: cardBody

    contentItem: ColumnLayout {
        id: cardBody
        spacing: Theme.spaceXs
        HoverHandler {
            id: contentHover
        }
        // The path first, and wrapped — the row already cut it down (規約 §hover のツールチップ). The commit button's
        // card has no one file to name and leaves this out.
        CardText {
            visible: eolCard.path !== ""
            text: eolCard.path
            color: Theme.textSecondary
            pixelSize: Theme.fontSm
            Layout.fillWidth: true
            Layout.maximumWidth: eolCard.parent !== null ? eolCard.parent.width : implicitWidth
        }
        CardText {
            text: eolCard.notice
            color: Theme.warning
            pixelSize: Theme.fontSm
            Layout.fillWidth: true
            Layout.maximumWidth: eolCard.parent !== null ? eolCard.parent.width : implicitWidth
        }
    }
}
