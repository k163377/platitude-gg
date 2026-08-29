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

    // A divider is a claim that a group ends here, so it is drawn only where there is a row left on each side of it.
    // With the rows that cannot be chosen gone rather than greyed (デザイン規約 §メニュー) a whole group can empty out, and
    // the line left behind would divide nothing — or, at the top or bottom of the card, divide the menu from the air
    // outside it.
    //
    // **Above is anywhere above; below is this divider's own group** (as far as the next divider). That asymmetry is
    // what collapses a run of them down to one line and no more: of the dividers that share an empty stretch, only
    // the last has rows of its own underneath, so only the last draws. Asking for the group *immediately* above
    // instead loses the line altogether when a group in the middle of the card empties out — both dividers around it
    // find nothing on one side (measured, the current branch's menu, where `switch` / merge / rebase are all
    // out and `Create branch here…` ran straight into the BRANCH card).
    //
    // The rows are asked what they are offering, not whether they are visible: a closed menu's list has released them
    // and turned their visibility off, and this has to hold before the menu opens (AppMenuItem.offered).
    readonly property bool dividing: {
        if (!divider.inMenu)
            return false
        let above = false
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
                // Another divider: past this one it ends the group this line would be opening.
                if (past)
                    break
                continue
            }
            if (!row.offered)
                continue
            if (past) {
                below = true
                break
            }
            above = true
        }
        return above && below
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
