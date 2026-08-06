import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// Context-menu shell: AppDialog's elevated card at menu scale, so a
// menu reads as a surface lifted off the graph rather than a hole in
// it (デザイン規約: メニューは bgElevated + 枠).
//
// The width comes from the rows. Fusion's own menu background is 200px
// wide whatever it holds, which silently clips every longer row; here
// the only limit is the window the menu opens over, and a row that
// reaches it elides and says its whole line on hover (AppMenuItem).
Menu {
    id: appMenu
    // Also what keeps the card's rounded corner clear of a highlighted
    // first or last row.
    padding: Theme.spaceXs

    /// A git term for this menu's own title row, said as a code chip
    /// when the menu opens from a row of another menu (RepoPage's reset
    /// submenu). Empty for a title that is all words.
    property string titleCode: ""

    // The one column this menu's chips share when a sentence follows
    // them: as wide as the widest such chip, so every row's words start
    // on the same x and the chips read as a table's first column. A
    // chip that ends its row (cherry-pick) has nothing to line up and
    // stays its own width, claiming no seat.
    readonly property real codeColW: {
        let widest = 0
        for (let i = 0; i < appMenu.count; i++) {
            const row = appMenu.itemAt(i)
            if (row && row.visible && row.codeColSeat !== undefined)
                widest = Math.max(widest, row.codeColSeat)
        }
        return widest
    }

    // Fusion measures a menu by its background (a flat 200) and by its
    // list view (which has no implicit width at all), so the rows never
    // get a say. Here the widest row decides.
    readonly property int widestRow: {
        let widest = 0
        for (let i = 0; i < appMenu.count; i++) {
            const row = appMenu.itemAt(i)
            // A row that is not being offered does not get to set the
            // width either.
            if (row && row.visible)
                widest = Math.max(widest, row.implicitWidth)
        }
        return widest
    }
    // ...but never narrower than `menuMinW`. A held row reports itself by
    // filling from the left, and on a row only as wide as its own words
    // there is too little travel to read as progress (デザイン規約
    // §進行中・長押しの定数).
    implicitWidth: Math.max(Metrics.menuMinW,
                            appMenu.widestRow + appMenu.leftPadding
                            + appMenu.rightPadding)

    // The window is reached through the item the menu was declared
    // under: a menu that opens as a window of its own would otherwise
    // measure itself and never find a limit.
    readonly property Item ownerItem: appMenu.parent
    readonly property int roomForRows:
        appMenu.ownerItem && appMenu.ownerItem.Window.window
        ? appMenu.ownerItem.Window.window.width - 2 * Theme.spaceXxl : 0
    width: appMenu.roomForRows > 0
           ? Math.min(appMenu.implicitWidth, appMenu.roomForRows)
           : appMenu.implicitWidth

    // Rows the menu builds itself (a submenu's own title row) get the
    // same treatment as the declared ones — including the code chip,
    // when the submenu asked for one on its title.
    delegate: AppMenuItem {
        code: subMenu && subMenu.titleCode !== undefined
              ? subMenu.titleCode : ""
    }

    background: Rectangle {
        color: Theme.bgElevated
        radius: Theme.radiusMd
        border.color: Theme.borderDefault
        border.width: Theme.borderWidth
    }
}
