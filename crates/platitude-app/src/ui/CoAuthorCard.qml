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
Popup {
    id: mateCard

    /// Packed co-author records (name, address, identicon — see
    /// `encode::encode_co_authors`), in the order the message lists them.
    property var records: []

    padding: Theme.spaceXs
    // Nothing stands between the underlined stretch and this: the pointer
    // has to be able to walk down into it without leaving both.
    margins: 0
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    /// Whether the pointer is over this. `HoverHandler` rather than a
    /// `MouseArea`: handlers are passive, so nothing inside takes this
    /// one's hover away.
    readonly property alias pointerInside: insideHover.hovered

    background: Rectangle {
        color: Theme.bgElevated
        radius: Theme.radiusMd
        border.color: Theme.borderDefault
        border.width: Theme.borderWidth
    }

    contentItem: Column {
        HoverHandler {
            id: insideHover
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

                implicitWidth: rowContent.implicitWidth + 2 * Theme.spaceSm
                implicitHeight: Theme.rowHeight
                width: implicitWidth
                height: Theme.rowHeight

                RowLayout {
                    id: rowContent
                    x: Theme.spaceSm
                    anchors.verticalCenter: parent.verticalCenter
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
                        Layout.alignment: Qt.AlignVCenter
                    }
                    // Meta about the row rather than part of the name, so
                    // it takes the colour the chips' `+N` takes.
                    Label {
                        visible: mateRow.parts[1] !== ""
                        text: "<" + mateRow.parts[1] + ">"
                        color: Theme.textSecondary
                        font.pixelSize: Theme.fontSm
                        Layout.alignment: Qt.AlignVCenter
                    }
                }
            }
        }
    }
}
