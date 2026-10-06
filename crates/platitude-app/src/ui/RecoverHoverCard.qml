pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// What an entry of the discard log says when the pointer rests on it, as a graph row's card does (`CommitHoverCard`):
// the operation in full, what it took and where, and when — written as the band writes it, the moment first
// (破棄記録仕様.md). A card, not a tooltip: a tooltip lays its string out in one block.
//
// Owned by the list: its rows are recycled the moment they scroll off, and a popup parented to a row goes with it.
AppCard {
    id: card

    /// The entry, as the list words it (`RecoverEntries.entryOf`); null for none.
    property var entry: null
    /// The moment in full, then how long ago (`RecoverPane.whenWords`).
    property string when: ""
    /// How wide the words may run before they wrap: a share of the window, set by the owner.
    property real textWidth: 0
    /// The lines under the title: what it took, then where, when that was another worktree.
    readonly property var facts: {
        if (card.entry === null)
            return []
        const lines = card.entry.parts.map(part => part.text)
        if (card.entry.worktree !== "")
            lines.push(qsTr("In %1").arg(card.entry.worktree))
        return lines
    }

    margins: Theme.spaceXs
    // The pointer walks into this card, so both halves of `AppCard.pointerInside` are wired.
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
        CardText {
            Layout.fillWidth: true
            Layout.maximumWidth: card.textWidth
            text: card.entry === null ? "" : card.entry.title
            color: Theme.textPrimary
            pixelSize: Theme.fontMd
            weight: Theme.fontWeightStrong
        }
        Repeater {
            model: card.facts
            delegate: CardText {
                required property string modelData
                Layout.fillWidth: true
                Layout.maximumWidth: card.textWidth
                text: modelData
                color: Theme.textSecondary
                pixelSize: Theme.fontMd
            }
        }
        // The moment last, at the date's size and ink a commit's card gives its own.
        CardText {
            text: card.when
            color: Theme.textSecondary
            pixelSize: Theme.fontSm
        }
    }
}
