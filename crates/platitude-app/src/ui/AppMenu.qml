import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// Context-menu shell on the elevated card face (デザイン規約 §メニュー). The width comes from the rows, capped by the
// window; a row that reaches it elides and says its whole line on hover (AppMenuItem).
Menu {
    id: appMenu
    // Also what keeps the card's rounded corner clear of a highlighted first or last row.
    padding: Theme.spaceXs
    // The rows' tips time themselves, as the window's do: a popup inherits no policy (`Main.qml`).
    ToolTip.policy: ToolTip.Manual

    /// A git term for this submenu's title row, drawn as a code chip; empty for a title of words.
    property string titleCode: ""
    /// A ref name in the title and the sentence with its `%1` seat, as a row takes them (`AppMenuItem.refSentence`);
    /// the title is made from them. A title with no name sets `title` instead.
    property string titleSentence: ""
    property string titleRef: ""
    /// The mark that name wears in front of it (`AppMenuItem.refMark`); empty for none.
    property string titleRefMark: ""
    title: appMenu.titleSentence !== "" ? appMenu.titleSentence.arg(appMenu.titleRef) : ""

    /// The title row's mark instead of a chip, for a submenu of one ref kind: its `NavIcon` kind and colour
    /// (デザイン規約 §メニュー の入れ子).
    property string titleKind: ""
    property color titleTint: Theme.textSecondary

    /// The rows keep the left menu's mark seat, so every name starts on one x (`AppMenuItem.seated`, §操作パネル).
    property bool keepsSeat: false
    /// A row on offer wears a mark: the seat is held open only then, or it reads as a stray margin.
    readonly property bool seatWorn: {
        if (!appMenu.keepsSeat)
            return false
        for (let i = 0; i < appMenu.count; i++) {
            const row = appMenu.itemAt(i)
            if (row && row.present && row.wearsSeat === true)
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
            if (row && row.present && row.headingRoom !== undefined && row.headingRoom > 0)
                widest = Math.max(widest, row.headingRoom + Theme.spaceXs)
        }
        return widest
    }
    /// The hold seat of the menu this card hangs off (`holdIndent`), handed down by that menu's row for this card
    /// (`delegate`); zero on a menu that hangs off none.
    property real outerIndent: 0
    /// The narrowest width: a hold fills its row from the left and needs the travel to read as progress
    /// (デザイン規約 §進行中・回数押しの定数). A card with no held row may drop it to fit its rows.
    property real widthFloor: Metrics.menuMinW

    /// For a submenu: whether its title row is offered. Not `visible`, which on a Menu means the card is on screen.
    property bool applies: true

    /// Non-empty: every row is out right now, for this reason (read through `AppMenuItem.blockedWhy`). The rows grey
    /// and stay (デザイン規約 §メニュー「「今できない」行は無効で残す」); title rows stay live, since the rows inside say why.
    property string heldReason: ""
    /// Non-empty while a git command this window asked for is out, its reading included (`RepoPage.runningWhy`): what
    /// a row says while it waits for it. Live, but **what greys the rows is whether one was out as this menu opened**
    /// (`waiting`): a menu opened under one keeps the rows it has with nothing out, so its shape does not depend on
    /// when it was opened (デザイン規約 §メニュー). One opened before it greys nothing — a row of its own may have
    /// started it and still stand for git's answer (`RefBranchMenu`'s plain delete).
    property string runningWhy: ""
    /// Opened while `runningWhy` stood, and it has not ended since.
    property bool waiting: false
    /// What this menu waited for has ended with the menu up. The opener reads again what the menu stands on and stands
    /// the rows on it, the menu keeping its shape (`holdShape`): the rows a press would now act on wrongly — a stash
    /// renumbered under its selector, a reset over a merge that stopped — are only right off the new reading.
    signal waitEnded()
    onRunningWhyChanged: {
        if (appMenu.runningWhy !== "" || !appMenu.waiting)
            return
        appMenu.waiting = false
        if (appMenu.visible)
            appMenu.waitEnded()
    }
    /// Why every row is out right now, as the rows read it: the held doors first, then the command this menu waits for.
    readonly property string rowsHeldWhy: appMenu.heldReason !== "" ? appMenu.heldReason
                                        : appMenu.waiting ? appMenu.runningWhy : ""

    /// Before the opener stands the rows on a newer reading with the menu up: the rows on show stay and no other comes
    /// (`AppMenuItem.shapeHeld`), so no row moves under the hand. A row that stops applying greys where it is; one that
    /// starts applying waits for the menu to be raised again. Holding a held shape again changes nothing.
    function holdShape() {
        for (let i = 0; i < appMenu.count; i++) {
            const row = appMenu.itemAt(i)
            // Dividers have no `shapeHeld`: they follow the rows (AppMenuSeparator).
            if (!row || row.shapeHeld !== false)
                continue
            row.shownAtHold = row.offered
            row.shapeHeld = true
        }
    }
    /// Lets the shape go, for an opener about to stand a fresh menu.
    function releaseShape() {
        for (let i = 0; i < appMenu.count; i++) {
            const row = appMenu.itemAt(i)
            if (row && row.shapeHeld === true)
                row.shapeHeld = false
        }
    }

    /// Rows on show — possibly none, since a menu shows only what can be chosen (デザイン規約 §メニュー).
    readonly property int offeredRows: {
        let n = 0
        for (let i = 0; i < appMenu.count; i++) {
            const row = appMenu.itemAt(i)
            // Not dividers, which decide for themselves (AppMenuSeparator).
            if (row && row.present && row.codeColSeat !== undefined)
                n++
        }
        return n
    }
    /// Opens at the pointer unless nothing is on offer (デザイン規約 §メニュー) — or, asked from the keyboard, where the
    /// keyboard's target stands (`KeyMenu.at`), kept inside the window as `popup()` keeps the pointer's. Says whether
    /// it opened.
    function offer() {
        if (!appMenu.applies || appMenu.offeredRows === 0)
            return false
        if (KeyMenu.at !== null)
            appMenu.popup(appMenu.parent.mapFromItem(null, KeyMenu.at))
        else
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

    Component.onCompleted: {
        // Fusion's list clips for its scroll alone, and a row's hold fill reaches past its side out to the card's
        // frame (`HoldFill.bleed`). So the list clips only while it scrolls.
        appMenu.contentItem.clip = Qt.binding(() => appMenu.contentItem.interactive)
        // Hands every divider its menu: a MenuSeparator has no `menu` of its own
        // (rules-refs/app-ui.md「区切りは `AppMenuSeparator` が自分で決める」).
        for (let i = 0; i < appMenu.count; i++) {
            const row = appMenu.itemAt(i)
            if (row && row.inMenu !== undefined)
                row.inMenu = appMenu
        }
    }

    // Written in while it stands: some presses outside never reach the scene to close it (`OutsidePress`).
    readonly property bool inOutsidePress: OutsidePress.counts(appMenu.visible, appMenu.closePolicy)
    onInOutsidePressChanged: OutsidePress.stand(appMenu, appMenu.inOutsidePress)
    Component.onDestruction: OutsidePress.stand(appMenu, false)

    // The chips' shared column, as wide as the widest chip on offer, a chip that ends its row included
    // (デザイン規約 §git 用語のコード表記「メニュー内のチップは 1 本の列を共有する」). Settled as the menu opens, so a
    // chip that changes while the card stands (`branch -D`) moves no row's words.
    property real codeColW: 0
    onAboutToShow: {
        appMenu.waiting = appMenu.runningWhy !== ""
        let widest = 0
        for (let i = 0; i < appMenu.count; i++) {
            const row = appMenu.itemAt(i)
            if (row && row.present && row.codeColSeat !== undefined)
                widest = Math.max(widest, row.codeColSeat)
        }
        appMenu.codeColW = widest
    }

    // The hold mark's seat, left by every row once any row on offer carries the mark, so words start on one x
    // (デザイン規約 §回数押し); 0 otherwise. Extra left padding, not a layout seat, which would also pay the layout's gap.
    // The seat (`iconSm`) the hold mark and the `!` stand in: it stands at the row's edge, and the row's own padding is
    // the step from it to the words. **A card under a menu keeping the seat keeps it too** (`outerIndent`) — the levels
    // of one menu start their words alike, a heading's mark in the seat.
    readonly property real holdIndent: {
        if (appMenu.outerIndent > 0)
            return Theme.iconSm
        for (let i = 0; i < appMenu.count; i++) {
            const row = appMenu.itemAt(i)
            if (!row || !row.present)
                continue
            // Either mark takes the same seat, so either one opens it (`AppMenuItem.asks`).
            if ((row.holdPresses !== undefined && row.holdPresses > 0) || row.asks === true)
                return Theme.iconSm
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
            // A divider has no `present`.
            if (row && row.present)
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
        refMark: subMenu && subMenu.titleRefMark !== undefined ? subMenu.titleRefMark : ""
        markKind: subMenu && subMenu.titleKind !== undefined ? subMenu.titleKind : ""
        markTint: subMenu && subMenu.titleTint !== undefined ? subMenu.titleTint : Theme.textSecondary
        // A submenu with nothing to offer, by `applies` or by coming out empty, takes its title row with it.
        offered: !subMenu || (subMenu.applies !== false && (subMenu.offeredRows > 0 || subMenu.promises === true))
        // The card keeps this menu's hold seat (`outerIndent`). Bound once Qt hands the row its card, which it does
        // after the row is built.
        onSubMenuChanged: {
            if (subMenu !== null && subMenu.outerIndent !== undefined)
                subMenu.outerIndent = Qt.binding(() => appMenu.holdIndent)
        }
    }

    background: AppCardFace {}
}
