pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The sidebar's header band: a frameless filter and the fold control, at the header height the other panes' headers
// line up with. The band's hairline runs on under the button.
Item {
    id: filterRow

    /// What is typed into the band. Automation writes it into the field itself, as a typist would.
    property alias text: field.text

    signal foldRequested()

    implicitHeight: Theme.headerHeight
    RowLayout {
        anchors.fill: parent
        spacing: 0
        TextField {
            id: field
            Layout.fillWidth: true
            Layout.fillHeight: true
            font.pixelSize: Theme.fontMd
            // At the section headers' x, not the rows' (デザイン規約 §余白).
            leftPadding: Theme.spaceXs
            rightPadding: Theme.spaceXs
            topPadding: 0
            bottomPadding: 0
            placeholderText: qsTr("Filter")
            background: null
            // The product's right-click menu, not the style's, and the menu key's (`FieldMenuSeat`).
            ContextMenu.menu: null
            ContextMenu.onRequested: menuSeat.offer()
            Keys.onMenuPressed: event => event.accepted = menuSeat.offer()
            FieldMenuSeat {
                id: menuSeat
                editor: field
            }
        }
        // The whole block is the button, the band's full height.
        FoldBlock {
            Layout.fillHeight: true
            Layout.preferredWidth: Theme.headerHeight
            onActivated: filterRow.foldRequested()
        }
    }
    BandRule {
        color: field.activeFocus || menuSeat.holding ? Theme.borderFocus : Theme.borderSubtle
    }
}
