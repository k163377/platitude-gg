import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// Context-menu shell on the elevated card face (デザイン規約 §メニュー). The width comes from the rows, capped by the
// window; a row that reaches it elides and says its whole line on hover (AppMenuItem).
Menu {
    id: appMenu
    // Also what keeps the card's rounded corner clear of a highlighted first or last row.
    padding: Theme.spaceXs

    /// A git term for this submenu's title row, drawn as a code chip; empty for a title of words.
    property string titleCode: ""
    /// A ref name in the title and the sentence with its `%1` seat, as a row takes them (`AppMenuItem.refSentence`);
    /// the title is made from them. A title with no name sets `title` instead.
    property string titleSentence: ""
    property string titleRef: ""
    title: appMenu.titleSentence !== "" ? appMenu.titleSentence.arg(appMenu.titleRef) : ""

    /// The title row's mark instead of a chip, for a submenu of one ref kind: its `NavIcon` kind and colour
    /// (デザイン規約 §メニュー の入れ子).
    property string titleKind: ""
    property color titleTint: Theme.textSecondary
    /// The title row is a folder of the rows under it, drawn as the left menu's folder row (`OpsBranchMenu`).
    property bool titleFolder: false

    /// The rows keep the left menu's mark seat, so every name starts on one x (`AppMenuItem.seated`, §操作パネル).
    property bool keepsSeat: false
    /// A row on offer wears a mark: the seat is held open only then, or it reads as a stray margin.
    readonly property bool seatWorn: {
        if (!appMenu.keepsSeat)
            return false
        for (let i = 0; i < appMenu.count; i++) {
            const row = appMenu.itemAt(i)
            if (row && row.offered && row.wearsSeat === true)
                return true
        }
        return false
    }
    /// For a card built as it opens: it will have rows, so its title row is on offer while it is still empty
    /// (`OpsBranchMenu`'s folders).
    property bool promises: false
    /// Title rows start their words on one x, the heading-mark seat held open on those without one (`TopBar`'s
    /// REPOSITORY and WORKTREE, デザイン規約 §操作パネル).
    property bool alignsHeadings: false
    readonly property real headingSeat: {
        if (!appMenu.alignsHeadings)
            return 0
        let widest = 0
        for (let i = 0; i < appMenu.count; i++) {
            const row = appMenu.itemAt(i)
            if (row && row.offered && row.headInk !== undefined && row.headInk > 0)
                widest = Math.max(widest, row.headInk + Theme.spaceXs)
        }
        return widest
    }
    /// The narrowest width: a hold fills its row from the left and needs the travel to read as progress
    /// (デザイン規約 §進行中・長押しの定数). A card with no held row may drop it to fit its rows.
    property real widthFloor: Metrics.menuMinW

    /// For a submenu: whether its title row is offered. Not `visible`, which on a Menu means the card is on screen.
    property bool applies: true

    /// Non-empty: every row is out right now, for this reason (read through `AppMenuItem.blockedWhy`). The rows grey
    /// and stay (デザイン規約 §メニュー「「今できない」行は無効で残す」); title rows stay live, since the rows inside say why.
    property string heldReason: ""

    /// Rows on offer — possibly none, since a menu shows only what can be chosen (デザイン規約 §メニュー).
    readonly property int offeredRows: {
        let n = 0
        for (let i = 0; i < appMenu.count; i++) {
            const row = appMenu.itemAt(i)
            // Not dividers, which decide for themselves (AppMenuSeparator).
            if (row && row.offered && row.codeColSeat !== undefined)
                n++
        }
        return n
    }
    /// Opens at the pointer unless nothing is on offer (デザイン規約 §メニュー). Says whether it opened.
    function offer() {
        if (!appMenu.applies || appMenu.offeredRows === 0)
            return false
        appMenu.popup()
        return true
    }

    /// `offer()` for a card placed under its control by its own `x`/`y` (`TopBar`, `OpsBranchMenu`).
    function offerHere() {
        if (!appMenu.applies || appMenu.offeredRows === 0)
            return false
        appMenu.open()
        return true
    }

    /// Automation (hover cannot be injected): lights the row carrying `sub` and opens it through `sub.offer()`. Says
    /// whether it opened.
    function openSub(sub) {
        for (let i = 0; i < appMenu.count; i++) {
            const row = appMenu.itemAt(i)
            if (row && row.subMenu === sub)
                appMenu.currentIndex = i
        }
        return sub.offer()
    }

    // Hands every divider its menu: a MenuSeparator has no `menu` of its own
    // (rules-refs/app-ui.md「区切りは `AppMenuSeparator` が自分で決める」).
    Component.onCompleted: {
        for (let i = 0; i < appMenu.count; i++) {
            const row = appMenu.itemAt(i)
            if (row && row.inMenu !== undefined)
                row.inMenu = appMenu
        }
    }

    // The chips' shared column, as wide as the widest chip on offer, a chip that ends its row included
    // (デザイン規約 §git 用語のコード表記「メニュー内のチップは 1 本の列を共有する」). Settled as the menu opens, so a
    // chip that changes while the card stands (`branch -D`) moves no row's words.
    property real codeColW: 0
    onAboutToShow: {
        let widest = 0
        for (let i = 0; i < appMenu.count; i++) {
            const row = appMenu.itemAt(i)
            if (row && row.offered && row.codeColSeat !== undefined)
                widest = Math.max(widest, row.codeColSeat)
        }
        appMenu.codeColW = widest
    }

    // The hold mark's seat, left by every row once any row on offer carries the mark, so words start on one x
    // (デザイン規約 §長押し); 0 otherwise. Extra left padding, not a layout seat, which would also pay the layout's gap.
    readonly property real holdIndent: {
        for (let i = 0; i < appMenu.count; i++) {
            const row = appMenu.itemAt(i)
            if (!row || !row.offered)
                continue
            // Either mark takes the same seat, so either one opens it (`AppMenuItem.asks`).
            if ((row.holdMs !== undefined && row.holdMs > 0) || row.asks === true)
                return Theme.spaceSm
        }
        return 0
    }

    // Fusion sizes a menu by its background (a flat 200), never its rows; here the widest row decides. Rounded up, or
    // that row is handed a fraction short and elides
    // (rules-refs/app-ui.md「子の入札から自分の幅を決める入れ物は `Math.ceil` で受ける」).
    readonly property int widestRow: {
        let widest = 0
        for (let i = 0; i < appMenu.count; i++) {
            const row = appMenu.itemAt(i)
            // A divider has no `offered`.
            if (row && row.offered)
                widest = Math.max(widest, row.implicitWidth)
        }
        return Math.ceil(widest)
    }
    implicitWidth: Math.max(appMenu.widthFloor, appMenu.widestRow + appMenu.leftPadding + appMenu.rightPadding)

    // The window is reached through the item the menu was declared under: a menu opened as its own window would
    // otherwise find no limit.
    readonly property Item ownerItem: appMenu.parent
    readonly property int roomForRows:
        appMenu.ownerItem && appMenu.ownerItem.Window.window
        ? appMenu.ownerItem.Window.window.width - 2 * Theme.spaceXxl : 0
    width: appMenu.roomForRows > 0 ? Math.min(appMenu.implicitWidth, appMenu.roomForRows) : appMenu.implicitWidth

    // Submenus' title rows, which the menu builds itself.
    delegate: AppMenuItem {
        // Bound: Qt hands the row the title once mid-build, before the row's own `text` binding overwrites it, so a
        // card added with its title already set (`OpsBranchMenu`) would draw an empty row
        // (`tests/qml/tst_opsbranchmenu.qml`).
        text: subMenu !== null ? subMenu.title : ""
        code: subMenu && subMenu.titleCode !== undefined ? subMenu.titleCode : ""
        // The title's name, drawn in its ref colour.
        refSentence: subMenu && subMenu.titleSentence !== undefined ? subMenu.titleSentence : ""
        refName: subMenu && subMenu.titleRef !== undefined ? subMenu.titleRef : ""
        markKind: subMenu && subMenu.titleKind !== undefined ? subMenu.titleKind : ""
        markTint: subMenu && subMenu.titleTint !== undefined ? subMenu.titleTint : Theme.textSecondary
        folderRow: subMenu !== null && subMenu.titleFolder === true
        // A submenu with nothing to offer, by `applies` or by coming out empty, takes its title row with it.
        offered: !subMenu || (subMenu.applies !== false && (subMenu.offeredRows > 0 || subMenu.promises === true))
    }

    background: AppCardFace {}
}
