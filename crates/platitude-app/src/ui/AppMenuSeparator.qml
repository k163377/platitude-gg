import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// A hairline between groups of menu rows. Not Fusion's, which is 188px wide and holds every menu open to that width.
MenuSeparator {
    id: divider

    /// Handed over by AppMenu as it is built (a MenuSeparator has no `menu`).
    property Menu inMenu: null

    // Drawn only with an offered row on each side, since groups can empty out (デザイン規約 §メニュー). Above is anywhere
    // above, below is this divider's own group, so a run over an empty stretch draws once
    // (rules-refs/app-ui.md「上はどこかに行、下は自分の群に行」). Reads `offered`, not visibility: a closed menu's rows
    // are released, and this must hold before it opens.
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
    // Undrawn, it gives its height back too, or its air leaves a hole between rows.
    implicitHeight: divider.dividing ? line.implicitHeight + divider.topPadding + divider.bottomPadding : 0

    contentItem: Rectangle {
        id: line
        implicitHeight: Theme.borderWidth
        color: Theme.borderSubtle
    }
}
