pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// What a pending file's row says about its line endings when the pointer
// rests on it: the path the tree view cut its name down from, and the
// same sentence the diff pane puts above the hunks.
//
// A card rather than a `ToolTip`, for the reason `CommitHoverCard` is one:
// a tooltip holds one string in one colour, and this has two lines of
// which only the second is warning-coloured. The ground is no longer the
// reason — tooltips stand on this same card now (`Main.dressToolTip`).
//
// It answers and offers nothing. Whoever wants the lines themselves opens
// the diff, which is one click away on the row the card came from.
//
// Owned by the pane, not the row: rows are recycled the moment they
// scroll off, and a popup parented to one goes with it (app-ui.md).
Popup {
    id: eolCard

    /// The file the card is about, spelled in full.
    property string path: ""
    /// The whole sentence, already worded (`Words.lineEndings`).
    property string notice: ""

    padding: Theme.spaceSm
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    background: Rectangle {
        color: Theme.bgElevated
        radius: Theme.radiusMd
        border.color: Theme.borderDefault
        border.width: Theme.borderWidth
    }

    contentItem: ColumnLayout {
        spacing: Theme.spaceXs
        // The path first: someone who cannot see which file this is about
        // has lost more than the notice gives them. The commit button's
        // card has no one file to name and leaves this out.
        Label {
            visible: eolCard.path !== ""
            text: eolCard.path
            color: Theme.textSecondary
            font.pixelSize: Theme.fontSm
            elide: Text.ElideMiddle
            Layout.maximumWidth: eolCard.parent !== null
                                 ? eolCard.parent.width : implicitWidth
        }
        Label {
            text: eolCard.notice
            color: Theme.warning
            font.pixelSize: Theme.fontSm
            wrapMode: Text.WordWrap
            Layout.maximumWidth: eolCard.parent !== null
                                 ? eolCard.parent.width : implicitWidth
        }
    }
}
