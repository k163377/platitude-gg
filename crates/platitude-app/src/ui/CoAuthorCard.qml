pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The people a commit message credits beside its author, one to a row with the address that tells two of the same
// name apart. A popup: declared inside the pane's column it would be clipped and painted under the list below.
AppCard {
    id: mateCard

    /// The co-author records (`{name, email, face}` — `encode::Mates`), in the order the message lists them.
    property var records: []
    /// How wide this card may stand, set as the author's card's is (`AuthorCard.maxWidth`,
    /// `DetailsAuthorCards.takeRoom`).
    property real maxWidth: 0
    readonly property real rowCap: mateCard.maxWidth > 0
        ? mateCard.maxWidth - 2 * mateCard.padding - 2 * Theme.spaceSm : Number.MAX_VALUE

    padding: Theme.spaceXs
    // Against the underlined stretch, so the pointer walks down into it without leaving both.
    margins: 0
    // The pointer walks in to read it, so both halves (rules-refs/app-ui.md「hover で開くものの 5 つの罠」の (2)).
    tracksPointer: true
    contentPointed: contentHover.hovered
    // Every gap in this card can start a selection (規約 §hover のツールチップ).
    textContent: cardBody

    contentItem: Column {
        id: cardBody
        HoverHandler {
            id: contentHover
        }
        Repeater {
            model: mateCard.records
            delegate: Item {
                id: mateRow
                required property var modelData
                readonly property bool hasAddress: mateRow.modelData.email !== ""

                // Laid out from the start: an address shown only under the pointer resized the row, which moved out
                // from under the hand and shut the card. On a second line, since height is what this card has to
                // spare. Both wrap: this is where they are read in full (規約 §hover のツールチップ).
                readonly property real headHeight: Math.max(Theme.rowHeight, rowContent.implicitHeight)
                implicitWidth: Math.max(rowContent.implicitWidth,
                                        address.x - Theme.spaceSm + address.width)
                               + 2 * Theme.spaceSm
                implicitHeight: mateRow.headHeight + (mateRow.hasAddress ? address.height : 0)
                width: implicitWidth
                height: implicitHeight

                RowLayout {
                    id: rowContent
                    x: Theme.spaceSm
                    y: (mateRow.headHeight - height) / 2
                    spacing: Theme.spaceXs
                    IdentIcon {
                        code: mateRow.modelData.face
                        width: Theme.iconMd
                        height: Theme.iconMd
                        Layout.alignment: Qt.AlignVCenter
                    }
                    CardText {
                        text: mateRow.modelData.name
                        color: Theme.textPrimary
                        pixelSize: Theme.fontMd
                        Layout.maximumWidth: mateCard.rowCap - Theme.iconMd - Theme.spaceXs
                        Layout.alignment: Qt.AlignVCenter
                    }
                }
                CardText {
                    id: address
                    visible: mateRow.hasAddress
                    width: Math.min(implicitWidth, mateCard.rowCap - Theme.iconMd - Theme.spaceXs)
                    // Under the name, indented past the face so the two read as one person.
                    x: Theme.spaceSm + Theme.iconMd + Theme.spaceXs
                    y: mateRow.headHeight - Theme.spaceXs
                    text: mateRow.modelData.email
                    color: Theme.textSecondary
                    pixelSize: Theme.fontSm
                }
            }
        }
    }
}
