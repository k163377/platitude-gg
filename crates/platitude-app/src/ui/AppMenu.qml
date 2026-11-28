import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui
import platitude.ui.input

// Context-menu shell on the elevated card face (デザイン規約 §メニュー). The width comes from the rows, capped by the
// window; a row that reaches it elides and says its whole line on hover (AppMenuItem).
Menu {
    id: appMenu
    // Also what keeps the card's rounded corner clear of a highlighted first or last row.
    padding: Theme.spaceXs

    /// A git term for this submenu's title row, drawn as a code chip; empty for a title of words.
    property string titleCode: ""
    /// A ref name in the title and the sentence with its `%1` seat, as a row takes them (`AppMenuItem.refSentence`);
    /// the title is made from them. A title with no name sets `title` instead. A name that has gone leaves the title
    /// its chip alone, as a row (`AppMenuItem.sentence`).
    property string titleSentence: ""
    property string titleRef: ""
    /// The mark that name wears in front of it (`AppMenuItem.refMark`); empty for none.
    property string titleRefMark: ""
    title: appMenu.titleSentence !== "" && (appMenu.titleSentence.indexOf("%1") < 0 || appMenu.titleRef !== "")
           ? appMenu.titleSentence.arg(appMenu.titleRef) : ""

    /// The title row's mark instead of a chip, for a submenu of one ref kind: its `NavIcon` kind and colour (how the
    /// title row wears it: `AppMenuItem.markKind`).
    property string titleKind: ""
    property color titleTint: Theme.textSecondary

    /// The rows keep the left menu's mark seat, so every name starts on one x (`AppMenuItem.seated`; the operation
    /// panel's cards, `TopBar` / `OpsBranchMenu`).
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
    /// REPOSITORY and WORKTREE — only WORKTREE wears a mark; user decision).
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
    /// For a submenu: its title row greys with its rows where none of them can be chosen (`AppMenuItem.cardOut`).
    /// False for a list of places to go to, whose title is a heading of contents, like the left menu's sections
    /// (the toolbar's WORKTREE card; user decision — ask before changing).
    property bool greysWithRows: true
    /// The card of core's type this is (`menu::Card::word`), which `MenuLines` draws off the answer of the menu it
    /// hangs off; empty for a menu outside the type.
    property string word: ""

    /// Non-empty: every row is out right now, for this reason (read through `AppMenuItem.blockedWhy`). The rows grey
    /// where they stand (デザイン規約 §メニュー「右クリックのメニューの行は、対象の分類で決まる」); a title row greys with
    /// its card's rows and still opens it, since the rows inside say why (`AppMenuItem.cardOut`).
    property string heldReason: ""
    /// Non-empty while a git command this window asked for is out, its reading included (`RepoPage.runningWhy`): what
    /// a row says while it waits for it. Live, but **what greys the rows is whether one was out as this menu opened**
    /// (`waiting`): a menu opened under one keeps the rows it has with nothing out, so its shape does not depend on
    /// when it was opened (デザイン規約 §メニュー). One opened before it greys nothing — a row of its own may have
    /// started it and still stand for git's answer (`BranchDeleteRow`'s plain delete): greyed live, that row would grey
    /// before the answer and blank its hold (`AppMenuItem.onBlockedChanged`). A replay's `heldReason`, by contrast, is
    /// bundled live. Nets: `tst_commitrowmenu` / `tst_menuwait` /
    /// `repo_tab::tests::drain_boundaries::a_write_settles_after_its_answer_not_at_it`; headless `menu-wait` /
    /// `menu-wait after` (`--preset slowhook`), `menu-wait stash` / `stash-ref` (`--preset stashes`),
    /// `menu-wait replay` (`--preset replay`).
    property string runningWhy: ""
    /// Opened — or the menu carrying it opened — while `runningWhy` stood (`settleWaiting`), until that command ends:
    /// the menu's `MenuLines` lets every level's go then and stands the menu again, as a wait when one on show had
    /// waited (`MenuLines.commandEnded`) — the rows a press would now act on wrongly, a stash renumbered under its
    /// selector or a reset over a merge that stopped, are only right off the new reading. Nothing else lets it go, so
    /// **only a level of a `MenuLines` (its `menu`, or a card that menu carries) is handed a `runningWhy`**: a menu of
    /// its own would still wait with the command over, and grey every row at once under the next one, its holds with
    /// them. The end is read once there, before any level lets its wait go, so it is never this menu's to undo.
    property bool waiting: false
    /// Why every row is out right now, as the rows read it: the held doors first, then the command this menu waits for.
    readonly property string rowsHeldWhy: appMenu.heldReason !== "" ? appMenu.heldReason
                                        : appMenu.waiting ? appMenu.runningWhy : ""

    /// Before the opener stands the rows on a newer reading with the menu up: the rows on show stay and no other comes
    /// (`AppMenuItem.shapeHeld`), so no row moves under the hand. A row that stops applying greys where it is; one that
    /// starts applying waits for the menu to be raised again. Holding a held shape again changes nothing. What is kept
    /// is which rows stand, down the card: what they say is the newer reading's, and the card settles as wide as a menu
    /// raised on it with those rows (`widestRow`; デザイン規約 §メニュー「凍らせるのは行の出入りとチップ列の幅だけ」). The words' x moves
    /// too as a hold mark or a `!` comes or goes (`holdIndent`) — the right press count wins.
    function holdShape() {
        appMenu.shapeHeld = true
        for (let i = 0; i < appMenu.count; i++) {
            const row = appMenu.itemAt(i)
            // Dividers have no `shapeHeld`: they follow the rows (AppMenuSeparator).
            if (!row || row.shapeHeld !== false)
                continue
            row.shownAtHold = row.offered
            row.shapeHeld = true
        }
    }
    /// Held (`holdShape`): what `MenuLines` draws moves only what the rows say, not which stand or where.
    property bool shapeHeld: false
    /// Lets the shape go, for an opener about to stand a fresh menu.
    function releaseShape() {
        appMenu.shapeHeld = false
        for (let i = 0; i < appMenu.count; i++) {
            const row = appMenu.itemAt(i)
            if (row && row.shapeHeld === true)
                row.shapeHeld = false
        }
    }

    /// How many rows the menu has on show, dividers aside; possibly none.
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
    /// Rows on show that can be pressed now: none on a card whose every row is greyed, which greys the title row it
    /// hangs off (`AppMenuItem.cardOut`).
    readonly property int liveRows: {
        let n = 0
        for (let i = 0; i < appMenu.count; i++) {
            const row = appMenu.itemAt(i)
            if (row && row.present && row.codeColSeat !== undefined && !row.greyed)
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
    /// whether it opened (_PGG_AUTO_ACT=branch-card / tag-card).
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
        // Hands every divider its menu: a MenuSeparator has no `menu` of its own, and each divider decides from
        // the rows whether it draws (`AppMenuSeparator.inMenu`).
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

    /// The hand on this card — a row, a divider, the padding band: what a surface that lost the hand weighs it by,
    /// under the card or gone (`OutsidePress.handIn`). The face hears the band and every gap; a row stands above it,
    /// takes the hover first, and Qt goes no further than the first item that takes it, so the rows say their own.
    /// **Handlers only** (`AppMenuItem.handOn`): they keep the hand until Qt takes it, after it has handed the hover to
    /// what is under the hand — a row's own `hovered` drops as the card goes.
    readonly property bool pointerInside: {
        if (face.pointed)
            return true
        for (let i = 0; i < appMenu.count; i++) {
            const row = appMenu.itemAt(i)
            if (row && (row.handOn !== undefined ? row.handOn : row.hovered) === true)
                return true
        }
        return false
    }
    // The card has the hand no more — the hand walked out, or the card went from under it: what lost the hand under it
    // weighs where it went (`OutsidePress.handLeft`). Qt hands the hover to what the hand is on before it takes it from
    // the card — a card gone included, on the next hover it delivers — so a surface the hand is still over has heard it
    // by then, and one it has left has not.
    onPointerInsideChanged: {
        if (!appMenu.pointerInside)
            OutsidePress.handLeft()
    }

    // The chips' shared column, as wide as the widest chip on offer, a chip that ends its row included
    // (デザイン規約 §git 用語のコード表記「メニュー内のチップは 1 本の列を共有する」). Settled as the menu opens, so a
    // chip that changes while the card stands (`branch -D`) moves no row's words.
    property real codeColW: 0
    /// Settles `waiting` as the menu opens, for the cards it carries too: their rows are this menu's, opened under the
    /// same command, so a card not open yet waits with them — and its title row greys from the first
    /// (`AppMenuItem.cardOut`). A card settles again as it opens.
    function settleWaiting() {
        appMenu.waiting = appMenu.runningWhy !== ""
        for (let i = 0; i < appMenu.count; i++) {
            const row = appMenu.itemAt(i)
            if (row && row.subMenu && row.subMenu.settleWaiting !== undefined)
                row.subMenu.settleWaiting()
        }
    }
    onAboutToShow: {
        appMenu.settleWaiting()
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
    // The seat (`iconMd`) the hold mark and the `!` stand in: it stands at the row's edge, and the row's own padding is
    // the step from it to the words. As wide as a heading's mark box, so a heading's mark — the tag drawn 1.12x
    // included — stands inside it (user decision — ask before changing). The mark keeps to the card's edge and the
    // words pay one seat of indent. **A card under a menu keeping the seat keeps it too** (`outerIndent`; user decision
    // — ask before changing) — the levels of one menu start their words alike, a heading's mark in the seat.
    readonly property real holdIndent: {
        if (appMenu.outerIndent > 0)
            return Theme.iconMd
        for (let i = 0; i < appMenu.count; i++) {
            const row = appMenu.itemAt(i)
            if (!row || !row.present)
                continue
            // Either mark takes the same seat, so either one opens it (`AppMenuItem.asks`).
            if ((row.holdPresses !== undefined && row.holdPresses > 0) || row.asks === true)
                return Theme.iconMd
        }
        return 0
    }

    // Fusion sizes a menu by its background (a flat 200), never its rows; here the widest row decides. Rounded up, or
    // that row is handed a fraction short and elides
    // (rules-refs/app-ui.md「子の入札から自分の幅を決める入れ物は `Math.ceil` で受ける」). A row bids each of its items
    // rounded up already, as its layout spends them (`AppMenuItem.besideWords`): the sum alone rounded here would
    // leave that row short by its items' fractions (`tst_menufit`).
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
        // (`tests/qml/tst_opsbranchmenu.qml`). A card declared in a file passes unbound: its title arrives after the
        // row.
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
        // The card keeps this menu's hold seat (`outerIndent`, デザイン規約 §回数押し). Bound once Qt hands the row its
        // card, which it does after the row is built. One way, a level to the level below it, so the binding never
        // loops.
        onSubMenuChanged: {
            if (subMenu !== null && subMenu.outerIndent !== undefined)
                subMenu.outerIndent = Qt.binding(() => appMenu.holdIndent)
        }
    }

    background: AppCardFace {
        id: face
        tracksPointer: true
    }
}
