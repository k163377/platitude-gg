import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// Context-menu shell: AppDialog's elevated card at menu scale, so a menu reads as a surface lifted off the graph
// (デザイン規約: メニューは bgElevated + 枠).
//
// The width comes from the rows. Fusion's own menu background is 200px wide whatever it holds, which silently clips
// every longer row; here the only limit is the window the menu opens over, and a row that reaches it elides and says
// its whole line on hover (AppMenuItem).
Menu {
    id: appMenu
    // Also what keeps the card's rounded corner clear of a highlighted first or last row.
    padding: Theme.spaceXs

    /// A git term for this menu's own title row, said as a code chip when the menu opens from a row of another menu
    /// (RepoPage's reset submenu). Empty for a title that is all words.
    property string titleCode: ""
    /// A ref name in that title, and the sentence with the name's own seat left in it — the two an ordinary row takes
    /// (`AppMenuItem.refSentence`), so a title that says a name is spelled the way the rows under it are. The title
    /// itself is made from them; a card whose title says no name writes its `title` and leaves these alone.
    property string titleSentence: ""
    property string titleRef: ""
    title: appMenu.titleSentence !== "" ? appMenu.titleSentence.arg(appMenu.titleRef) : ""

    /// The mark that row wears instead, for a submenu holding everything one kind of ref answers for: the `NavIcon`
    /// kind and the kind's own colour (デザイン規約 §メニュー の入れ子). A submenu takes one or the other — a title that
    /// names a command says so with the chip, one that names a kind says so with the mark.
    property string titleKind: ""
    property color titleTint: Theme.textSecondary
    /// Whether that row is **a folder of the rows under it** rather than a heading — a card the left menu draws as a
    /// folder row, said here the way that row says it: its name in the quieter ink, nothing in the seat, and the card
    /// of the names filed under it opening off it (`OpsBranchMenu`).
    property bool titleFolder: false

    /// Whether the rows of this card keep **the left menu's seat** — the column a mark about the row stands in, held
    /// open on the rows that have none, so every name begins on one x (`AppMenuItem.seated`). For a card that lists
    /// what a section of the left menu lists and writes its rows the way that section does (§操作パネル).
    property bool keepsSeat: false
    /// …and whether any row on offer wears a mark in it. **The seat is held open for the rows without one only while
    /// a row has one to put there**: a card whose rows carry no mark at all starts its words where every other card
    /// does, and air held for a mark nobody wears reads as a margin nobody set.
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
    /// Whether this card, **made as it opens**, has rows to offer once it is — said before any row exists, so the
    /// row that opens it is on offer while the card is still empty (`OpsBranchMenu`'s folders). A card whose rows
    /// already stand answers with them (`offeredRows`) and leaves this alone.
    property bool promises: false
    /// Whether the rows of this card that open cards of their own begin their words on one x — **the heading marks'
    /// step, held open on the headings that wear none**. For a card made of nothing but such rows (`TopBar`'s
    /// REPOSITORY and WORKTREE), where a heading with a mark beside one without starts its words a mark later, and
    /// the two read as two margins (デザイン規約 §操作パネル).
    property bool alignsHeadings: false
    /// That step: the widest heading mark on offer and the step after its ink, or nothing where none wears one.
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
    /// The narrowest this card is drawn at. **The floor is for a card holding a held row** (`Metrics.menuMinW`): a
    /// hold reports itself by filling the row from the left, and a row only as wide as its words gives it too little
    /// travel to read as progress. A card that lists places to go holds none, and fits its rows — held to the floor,
    /// the air after its longest name is wider than the air before its first mark.
    property real widthFloor: Metrics.menuMinW

    /// Whether the row that opens this menu is worth offering at all, for a menu that hangs off a row of another one
    /// (RepoPage's reset submenu). A submenu cannot say this through `visible` — on a Menu that means "the card is on
    /// screen" — and the row is built by the menu above, so the two meet here.
    property bool applies: true

    /// Why every row of this menu is out right now, in one line — and, by being non-empty, that they are. The rows read
    /// it off the menu the way they read `holdIndent`, so a whole card is held with one line
    /// (`AppMenuItem.blockedWhy`).
    ///
    /// **The rows stay and grey** — the app-menu rule (デザイン規約 §メニュー の例外: 「今できない」行は無効で残る). What is out here is the
    /// window's answer about right now, and a row that vanished for it would read as a menu that never had it. **A
    /// card left standing keeps its title row live**: it is the rows inside it that say why, and a greyed title row
    /// would offer nothing to open and no line to read (`AppMenuItem.menuHeldReason`).
    property string heldReason: ""

    /// How many rows this menu is actually offering. A menu is assembled for the thing that was clicked and shows only
    /// what can be chosen on it (デザイン規約 §メニュー), so it can come out with nothing in it at all — the current branch has
    /// no row of its own, and a write in flight takes every row with it.
    readonly property int offeredRows: {
        let n = 0
        for (let i = 0; i < appMenu.count; i++) {
            const row = appMenu.itemAt(i)
            // A divider is what stands between rows, and it decides for itself (AppMenuSeparator).
            if (row && row.offered && row.codeColSeat !== undefined)
                n++
        }
        return n
    }
    /// Opens the menu where the pointer is, unless there is nothing in it — an empty card is not an answer, and a
    /// right-click that finds nothing to offer already does nothing on a folder row (デザイン規約 §メニュー). Says whether it
    /// opened.
    function offer() {
        if (!appMenu.applies || appMenu.offeredRows === 0)
            return false
        appMenu.popup()
        return true
    }

    /// The same, for a menu that drops from the control it belongs to rather than from the pointer: the card places
    /// itself under that control by its own `x`/`y` (`AppMenuButton`), so all this has left to decide is whether
    /// there is anything to open. Says whether it opened.
    function offerHere() {
        if (!appMenu.applies || appMenu.offeredRows === 0)
            return false
        appMenu.open()
        return true
    }

    /// Opens the card one of this menu's rows carries, and lights that row the way resting on it would. The
    /// automation's only way in — hover cannot be injected (app-ui.md §UI 自動化の因果性) — and the same `offer()`
    /// the card would get from a hand, so a card with nothing in it still does not come up. Says whether it opened.
    function openSub(sub) {
        for (let i = 0; i < appMenu.count; i++) {
            const row = appMenu.itemAt(i)
            if (row && row.subMenu === sub)
                appMenu.currentIndex = i
        }
        return sub.offer()
    }

    // Every divider is handed the menu it was declared in: a MenuSeparator, unlike a MenuItem, has no `menu` of its
    // own, and what a divider has to know — whether rows are left on both sides of it — can only be read off the menu
    // around it. Declaring `Component.onCompleted` on an AppMenu instance would take this with it; nothing does.
    Component.onCompleted: {
        for (let i = 0; i < appMenu.count; i++) {
            const row = appMenu.itemAt(i)
            if (row && row.inMenu !== undefined)
                row.inMenu = appMenu
        }
    }

    // The one column this menu's chips share: as wide as the widest chip on offer, so every row's words start on the
    // same x and the chips read as a table's first column. A chip that ends its row holds a seat like any other — left
    // out, the widest command would end past where the other rows' words begin (`cherry-pick` past the head of "into
    // main") and the column would break (デザイン規約 §git 用語のコード表記).
    //
    // Settled as the menu opens: a row that changes its chip while the card stands — the delete row
    // morphing to `branch -D` — leaves every other row's words where they are. What the menu shows is decided as it
    // opens and left alone (デザイン規約 §メニュー); the forced spelling is narrower than the plain one, so it sits inside the
    // column it inherited.
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

    // The seat every row of a menu that holds one held row leaves for the hold mark, whether or not that row is the
    // held one: a mark on some rows and not others would start their words on different x, and a menu is read down its
    // first letters (デザイン規約 §長押し). Zero where nothing here is held, so the everyday menus keep their words hard against
    // the padding.
    //
    // Carried as extra left padding: a seat would have to pay the layout's own gap on top of its width, and that gap
    // is shared with the code chip — the words would end up further from the mark than they are from anything else in
    // the row. This is the whole distance the words move, and the mark is
    // placed inside it.
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

    // Fusion measures a menu by its background (a flat 200) and by its list view (which has no implicit width at all),
    // so the rows never get a say. Here the widest row decides.
    //
    // **Rounded up, and the rounding is the whole point.** A row's width is measured — a mono chip
    // and a sentence in the UI family both land on fractions wherever the platform's metrics do — so the widest row
    // routinely asks for something like 249.27. Rounded to a whole pixel the other way, the menu hands that row 249:
    // a quarter of a pixel short, which puts its `RowLayout` over budget, and a layout over budget takes the shortfall
    // out of the one item that can give — the words, which then elide. **The row that set the width is the one row
    // drawn whole** (デザイン規約 §メニュー: 幅は最も広い行に合わせる).
    readonly property int widestRow: {
        let widest = 0
        for (let i = 0; i < appMenu.count; i++) {
            const row = appMenu.itemAt(i)
            // Only an offered row sets the width. (A divider has no `offered` and no
            // width of its own, so it never had a say here.)
            if (row && row.offered)
                widest = Math.max(widest, row.implicitWidth)
        }
        return Math.ceil(widest)
    }
    // ...and at least its floor wide (`widthFloor` — `menuMinW` unless the card says otherwise). A held row reports
    // itself by filling from the left, and on a row only as wide as its own words there is too little travel to read
    // as progress (デザイン規約 §進行中・長押しの定数).
    implicitWidth: Math.max(appMenu.widthFloor, appMenu.widestRow + appMenu.leftPadding + appMenu.rightPadding)

    // The window is reached through the item the menu was declared under: a menu that opens as a window of its own
    // would otherwise measure itself and never find a limit.
    readonly property Item ownerItem: appMenu.parent
    readonly property int roomForRows:
        appMenu.ownerItem && appMenu.ownerItem.Window.window
        ? appMenu.ownerItem.Window.window.width - 2 * Theme.spaceXxl : 0
    width: appMenu.roomForRows > 0 ? Math.min(appMenu.implicitWidth, appMenu.roomForRows) : appMenu.implicitWidth

    // Rows the menu builds itself (a submenu's own title row) get the same treatment as the declared ones — including
    // the code chip, when the submenu asked for one on its title.
    delegate: AppMenuItem {
        // **The card's own title, bound.** Qt hands the row the title once, in the middle of building it — before
        // the row's own binding on `text` is first read, which then writes over it — and says it again only when the
        // title changes. A card declared in a file gets its title after its row is built, so it comes through; a card
        // that already has one when it is added (one built as it opens — `OpsBranchMenu`) draws an empty row
        // (`tests/qml/tst_opsbranchmenu.qml`).
        text: subMenu !== null ? subMenu.title : ""
        code: subMenu && subMenu.titleCode !== undefined ? subMenu.titleCode : ""
        // The title's own name, so the row draws it the colour every other place draws it. The title itself is made
        // from the two halves already (`titleSentence` / `titleRef`) — the row only needs to know where in it the name
        // sits.
        refSentence: subMenu && subMenu.titleSentence !== undefined ? subMenu.titleSentence : ""
        refName: subMenu && subMenu.titleRef !== undefined ? subMenu.titleRef : ""
        markKind: subMenu && subMenu.titleKind !== undefined ? subMenu.titleKind : ""
        markTint: subMenu && subMenu.titleTint !== undefined ? subMenu.titleTint : Theme.textSecondary
        folderRow: subMenu !== null && subMenu.titleFolder === true
        // A submenu with nothing to offer takes its own title row with it, the way any other row that cannot be chosen
        // goes — whether it said so itself (`applies`) or simply came out empty. Without the second half a row would
        // open a card with nothing in it, which `offer()` already refuses to do at the top level.
        offered: !subMenu || (subMenu.applies !== false && (subMenu.offeredRows > 0 || subMenu.promises === true))
    }

    background: AppCardFace {}
}
