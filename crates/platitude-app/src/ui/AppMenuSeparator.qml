import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// Divider between groups of menu rows: a hairline across the card. Fusion's own is 188px wide whatever the menu is,
// which would hold every menu open to that width.
MenuSeparator {
    id: divider

    /// The menu this divider was declared in, handed over by AppMenu as it is built. A MenuSeparator has no `menu` of
    /// its own, and this one has to look at what stands on either side of it.
    property Menu inMenu: null

    // A divider is a claim that a group ends here, so it is drawn only where that group has a row left and something
    // still follows it. With the rows that cannot be chosen gone rather than greyed (デザイン規約 §メニュー) a whole group can
    // empty out, and the line left behind would divide nothing — or, at the top or bottom of the card, divide the menu
    // from the air outside it.
    //
    // What it looks for above is the group *immediately* above (back as far as the divider before this one), which is
    // also what collapses a run of them: with the group between two dividers gone, the first of the two keeps the line
    // and the second finds nothing of its own.
    //
    // The rows are asked what they are offering, not whether they are visible: a closed menu's list has released them
    // and turned their visibility off, and this has to hold before the menu opens (AppMenuItem.offered).
    readonly property bool dividing: {
        if (!divider.inMenu)
            return false
        let groupAbove = false
        let below = false
        let past = false
        for (let i = 0; i < divider.inMenu.count; i++) {
            const row = divider.inMenu.itemAt(i)
            if (row === divider) {
                past = true
                continue
            }
            if (!row)
                continue
            if (row.codeColSeat === undefined) {
                // Another divider: whatever came before it was its group, not this one's.
                if (!past)
                    groupAbove = false
                continue
            }
            if (!row.offered)
                continue
            if (past) {
                below = true
                break
            }
            groupAbove = true
        }
        return groupAbove && below
    }
    visible: divider.dividing

    padding: 0
    topPadding: Theme.spaceXs
    bottomPadding: Theme.spaceXs
    // The list lays its items out by height, so a divider that is not drawn has to give its height back as well —
    // otherwise the air it was keeping stays behind as a hole between two rows (the same reason AppMenuItem collapses).
    implicitHeight: divider.dividing ? line.implicitHeight + divider.topPadding + divider.bottomPadding : 0

    contentItem: Rectangle {
        id: line
        implicitHeight: Theme.borderWidth
        color: Theme.borderSubtle
    }
}
