import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// Divider between groups of menu rows: a hairline across the card.
// Fusion's own is 188px wide whatever the menu is, which would hold
// every menu open to that width.
MenuSeparator {
    padding: 0
    topPadding: Theme.spaceXs
    bottomPadding: Theme.spaceXs

    contentItem: Rectangle {
        implicitHeight: Theme.borderWidth
        color: Theme.borderSubtle
    }
}
