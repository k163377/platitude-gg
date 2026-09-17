pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The people a commit message credits beside its author, one to a row. The date line shows the first of them and counts
// the rest; this is where the rest are read, along with the address that tells two people of the same name apart.
//
// A popup, for the same reason the ref list is one: anything declared inside the
// card's column would be clipped by the pane and painted under the list below it.
AppCard {
    id: mateCard

    /// Packed co-author records (name, address, identicon — see `encode::encode_co_authors`), in the order the message
    /// lists them.
    property var records: []
    /// How wide this card may stand — the same ceiling the author's card carries, and set the same way
    /// (`AuthorCard.maxWidth`, `DetailsAuthorCards.takeRoom`).
    property real maxWidth: 0
    readonly property real rowCap: mateCard.maxWidth > 0
        ? mateCard.maxWidth - 2 * mateCard.padding - 2 * Theme.spaceSm : Number.MAX_VALUE

    padding: Theme.spaceXs
    // The card sits against the underlined stretch: the pointer has to be able to walk down into it without
    // leaving both.
    margins: 0
    // The pointer walks into this one and reads it. The rows here accept no hover today; giving `AppCard` both halves
    // is what keeps that an implementation detail.
    tracksPointer: true
    contentPointed: contentHover.hovered
    // Every gap in this card is a place a selection can start — the inset the rows keep, the step under a name, the
    // room beside a short address (規約 §hover のツールチップ). The rows answer no press of their own, so nothing here
    // loses one.
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
                required property string modelData
                // Name, address, identicon code — three fields always, so an address-less trailer is indexed by
                // position.
                readonly property var parts: mateRow.modelData.split(String.fromCharCode(30))
                readonly property bool hasAddress: mateRow.parts[1] !== ""

                // Laid out from the start: showing the address only under the pointer changed the row's
                // own size, so the row moved out from under the hand that asked for it and the card shut
                // (observed). What a hover reveals keeps the size of what is being hovered.
                //
                // On a second line: side by side makes every row as wide as a name and an
                // address end to end, and height is what this card has to spare — width is what it has to ask the
                // window for.
                //
                // **Both wrap**, and for the same reason the height was always the side that gave:
                // this card is where the names and addresses the date line could not fit are read in full and taken
                // away (規約 §hover のツールチップ).
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
                        code: parseInt(mateRow.parts[2])
                        width: Theme.iconMd
                        height: Theme.iconMd
                        Layout.alignment: Qt.AlignVCenter
                    }
                    CardText {
                        text: mateRow.parts[0]
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
                    text: mateRow.parts[1]
                    color: Theme.textSecondary
                    pixelSize: Theme.fontSm
                }
            }
        }
    }
}
