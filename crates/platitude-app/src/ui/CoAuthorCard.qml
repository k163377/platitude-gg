pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The people a commit message credits beside its author, one to a row.
// The date line shows the first of them and counts the rest; this is
// where the rest are read, along with the address that tells two people
// of the same name apart.
//
// A popup rather than an item under the row for the same reason the ref
// list is one: anything declared inside the card's column would be
// clipped by the pane and painted under the list below it.
AppCard {
    id: mateCard

    /// Packed co-author records (name, address, identicon — see
    /// `encode::encode_co_authors`), in the order the message lists them.
    property var records: []
    /// How wide a name or address may run before it is elided. Neither
    /// has a length worth trusting — an address may be 254 characters —
    /// and a popup clamps its *position* to the window, never its width,
    /// so without a cap a long one simply runs off the edge. The owner
    /// sets it from the pane the card opens over.
    property real maxRowWidth: 0
    readonly property real rowCap:
        mateCard.maxRowWidth > 0 ? mateCard.maxRowWidth : Number.MAX_VALUE

    padding: Theme.spaceXs
    // Nothing stands between the underlined stretch and this: the pointer
    // has to be able to walk down into it without leaving both.
    margins: 0
    // The pointer walks into this one and reads it. The rows here accept
    // no hover today; giving `AppCard` both halves is what keeps that an
    // implementation detail rather than a load-bearing fact.
    tracksPointer: true
    contentPointed: contentHover.hovered

    contentItem: Column {
        HoverHandler {
            id: contentHover
        }
        Repeater {
            model: mateCard.records
            delegate: Item {
                id: mateRow
                required property string modelData
                // Name, address, identicon code — three fields always,
                // so an address-less trailer is indexed, not counted.
                readonly property var parts:
                    mateRow.modelData.split(String.fromCharCode(30))
                readonly property bool hasAddress: mateRow.parts[1] !== ""

                // Laid out from the start, never on hover: showing the
                // address only under the pointer changed the row's own
                // size, so the row moved out from under the hand that
                // asked for it and the card shut (2026-08-09 report).
                // Nothing that a hover reveals may resize what is being
                // hovered.
                //
                // On a second line rather than beside the name: side by
                // side made the card wider than the pane it opens in
                // (2026-08-08), and this way the address costs height,
                // which the card has to spare.
                implicitWidth: Math.max(rowContent.implicitWidth,
                                        address.x - Theme.spaceSm
                                        + address.width)
                               + 2 * Theme.spaceSm
                implicitHeight: Theme.rowHeight
                                + (mateRow.hasAddress ? Theme.fontSmLine : 0)
                width: implicitWidth
                height: implicitHeight

                RowLayout {
                    id: rowContent
                    x: Theme.spaceSm
                    y: (Theme.rowHeight - height) / 2
                    spacing: Theme.spaceXs
                    IdentIcon {
                        code: parseInt(mateRow.parts[2])
                        width: Theme.iconMd
                        height: Theme.iconMd
                        Layout.alignment: Qt.AlignVCenter
                    }
                    Label {
                        text: mateRow.parts[0]
                        color: Theme.textPrimary
                        font.pixelSize: Theme.fontMd
                        elide: Text.ElideRight
                        Layout.maximumWidth: mateCard.rowCap - Theme.iconMd
                                             - Theme.spaceXs
                        Layout.alignment: Qt.AlignVCenter
                    }
                }
                Label {
                    id: address
                    visible: mateRow.hasAddress
                    width: Math.min(implicitWidth,
                                    mateCard.rowCap - Theme.iconMd
                                    - Theme.spaceXs)
                    elide: Text.ElideRight
                    // Under the name, indented past the face so the two
                    // read as one person.
                    x: Theme.spaceSm + Theme.iconMd + Theme.spaceXs
                    y: Theme.rowHeight - Theme.spaceXs
                    text: mateRow.parts[1]
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontSm
                }
            }
        }
    }
}
