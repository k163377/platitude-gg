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

    /// Whether the row that opens this menu is worth offering at all, for
    /// a menu that hangs off a row of another one (RepoPage's reset
    /// submenu). A submenu cannot say this through `visible` — on a Menu
    /// that means "the card is on screen" — and the row is built by the
    /// menu above, so the two meet here.
    property bool applies: true

    /// How many rows this menu is actually offering. A menu is assembled
    /// for the thing that was clicked and shows only what can be chosen
    /// on it (デザイン規約 §メニュー), so it can come out with nothing in
    /// it at all — the current branch has no row of its own, and a write
    /// in flight takes every row with it.
    readonly property int offeredRows: {
        let n = 0
        for (let i = 0; i < appMenu.count; i++) {
            const row = appMenu.itemAt(i)
            // A divider is not a row: it is what stands between them, and
            // it decides for itself (AppMenuSeparator).
            if (row && row.offered && row.codeColSeat !== undefined)
                n++
        }
        return n
    }
    /// Opens the menu where the pointer is, unless there is nothing in it
    /// — an empty card is not an answer, and a right-click that finds
    /// nothing to offer already does nothing on a folder row
    /// (デザイン規約 §メニュー). Says whether it opened.
    function offer() {
        if (!appMenu.applies || appMenu.offeredRows === 0)
            return false
        appMenu.popup()
        return true
    }

    // Every divider is handed the menu it was declared in: a
    // MenuSeparator, unlike a MenuItem, has no `menu` of its own, and
    // what a divider has to know — whether rows are left on both sides of
    // it — can only be read off the menu around it. Declaring
    // `Component.onCompleted` on an AppMenu instance would take this with
    // it; nothing does.
    Component.onCompleted: {
        for (let i = 0; i < appMenu.count; i++) {
            const row = appMenu.itemAt(i)
            if (row && row.inMenu !== undefined)
                row.inMenu = appMenu
        }
    }

    // The one column this menu's chips share: as wide as the widest
    // chip on offer, so every row's words start on the same x and the
    // chips read as a table's first column. A chip that ends its row
    // holds a seat like any other — left out, the widest command would
    // end past where the other rows' words begin (`cherry-pick` past
    // the head of "into main") and the column would break (デザイン規約
    // §git 用語のコード表記).
    readonly property real codeColW: {
        let widest = 0
        for (let i = 0; i < appMenu.count; i++) {
            const row = appMenu.itemAt(i)
            if (row && row.offered && row.codeColSeat !== undefined)
                widest = Math.max(widest, row.codeColSeat)
        }
        return widest
    }

    // The seat every row of a menu that holds one held row leaves for the
    // hold mark, whether or not that row is the held one: a mark on some
    // rows and not others would start their words on different x, and a
    // menu is read down its first letters (デザイン規約 §長押し). Zero
    // where nothing here is held, so the everyday menus keep their words
    // hard against the padding.
    //
    // Carried as extra left padding rather than as a seat in the row's
    // layout: a seat would have to pay the layout's own gap on top of its
    // width, and that gap is shared with the code chip — the words would
    // end up further from the mark than they are from anything else in
    // the row. This is the whole distance the words move, and the mark is
    // placed inside it.
    readonly property real holdIndent: {
        for (let i = 0; i < appMenu.count; i++) {
            const row = appMenu.itemAt(i)
            if (row && row.offered && row.holdMs !== undefined && row.holdMs > 0)
                return Theme.spaceSm
        }
        return 0
    }

    // Fusion measures a menu by its background (a flat 200) and by its
    // list view (which has no implicit width at all), so the rows never
    // get a say. Here the widest row decides.
    readonly property int widestRow: {
        let widest = 0
        for (let i = 0; i < appMenu.count; i++) {
            const row = appMenu.itemAt(i)
            // A row that is not being offered does not get to set the
            // width either. (A divider has no `offered` and no width of
            // its own, so it never had a say here.)
            if (row && row.offered)
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
        // A submenu with nothing to offer takes its own title row with
        // it, the way any other row that cannot be chosen goes.
        offered: !subMenu || subMenu.applies !== false
    }

    background: Rectangle {
        color: Theme.bgElevated
        radius: Theme.radiusMd
        border.color: Theme.borderDefault
        border.width: Theme.borderWidth
    }
}
