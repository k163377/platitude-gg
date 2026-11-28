import QtQuick
import platitude.ui

// ---- a menu drawn off core's answer ------------------------------------
// One stand's answer (`GitFacts.menuOn` and the other doors — core's `menu::lines`, `encode::MenuAnswer`): which rows
// the menu and its cards have, why each is greyed, and the seat each stands in. The answer decides the menu (core's
// `menu` module says the rule): a row is declared once, in its menu, and names its kind (`AppMenuItem.word`); this draws
// the first level `menu` and the type's cards it carries (`AppMenu.word`) off the answer — which rows are on offer, why
// each greys (`AppMenuItem.why`), their order, and a divider where the group changes. No row of the type writes its
// own `offered` or `blockedReason`, and no divider stands between two of them (`cargo xtask structure`). What a press
// sends stays the row's own.
//
// Drawn with every answer. A fresh stand (the shape let go — `release`) moves the rows into the answer's order and
// parts the groups; standing again with the menu up (`again`, the shape held — `AppMenu.holdShape`) moves only what
// each row says: a row gone from the answer greys where it stands (`AppMenuItem.withdrawn`), one come stays out until
// the menu is raised again (デザイン規約 §メニュー). The rows are the menu's own and are never made anew — moved
// (`Menu.moveItem`), not rebuilt —, so a press under way and the held shape keep. Rows outside the type (`Copy path`,
// the GITHUB card) name no kind and stand after the drawn ones, in the order declared.
//
// It also says when a standing menu stands again, one rule for every opener: what the rows read moved while nothing is
// out (`moved`), or the command that was out ended (`waiting` off) — once, as a wait when a level on show had waited for
// it, else as a follow (`commandEnded`). The opener answers `standAgain`: it reads what the rows stand on now and hands
// a new answer — or closes the menu, the thing it was opened on gone.
//
// A `QtObject`: the menus hold it as a property value (rules-refs/app-ui.md「Item の中に置く `QtObject` 製の部品は
// 読み取り専用プロパティの値にする」). Nets: `tst_menulines` (the drawing), `tst_menuwait` / `tst_commitrowmenu` (the
// stands).
QtObject {
    id: menuLines

    /// The stand's answer, whole: its `lines` in drawing order, each a row's `word` (`menu::Row::word`), `why` it is
    /// greyed (`menu::Why::word`, empty while live) and its seat — `group` the first-level group (a card's row, its
    /// card's), `card` the card it hangs behind and `cardGroup` its group in there, both empty on the first level
    /// (`encode::MenuLine`). Undefined before the first stand.
    property var answer: undefined
    /// The first level the answer is drawn into, and through it the cards of the type it carries; null while the
    /// answer is only read (`has` / `why`). An opener with several first levels (`CommitRowMenu`: a commit's, a
    /// stash's, another worktree's) sets the one it raises before it hands the answer: a new level draws nothing, the
    /// answer meant for it does.
    property AppMenu menu: null
    onAnswerChanged: menuLines.draw()
    /// The answer's rows and cards the last stand found no part for on the level their seat names, `card/row` for a
    /// card's row — none, unless a menu lacks a part its class has, which the stand says as a warning (`cargo xtask
    /// structure` sees only that every row has a part somewhere).
    property var undrawn: []

    /// Whatever moves each time what the rows read has moved (`RepoPage.menuReadRevision`, the names a menu reads):
    /// a standing menu follows it a turn later, while nothing is out.
    property var moved: undefined
    /// `moved` moved since the menu last stood: a follow is still to come. Any stand reads what moved and settles it,
    /// so a command's end landing in the turn of its move stands the menu once.
    property bool followDue: false
    /// A command the menus wait for is out (`RepoPage.treeAtAsk` set): the rows stand on the tree as it was asked, and
    /// follow nothing until it ends (`commandEnded`).
    property bool waiting: false
    /// The menu is up and stands again: the opener reads again what the rows stand on and hands the answer — or closes
    /// the menu, what it was opened on gone. `again`: `wait` for a command a level on show waited for that ended,
    /// `follow` for what moved with the menu up, a command's end that none waited for included — the cards' rows that
    /// reach over there keep their leases either way, and git's answers about a branch hold only while following
    /// (`RefBranchMenu.standOn`).
    signal standAgain(string again)

    /// Whether the answer has the row `row` (its word).
    function has(row) {
        return menuLines.lineOf(row) !== undefined
    }
    /// Why that row is greyed, as core words it — `Words.menuWhy` says it; empty while it is live and for no row.
    function why(row) {
        const line = menuLines.lineOf(row)
        return line === undefined ? "" : line.why
    }
    /// The answer's line for the row `row`; undefined for none.
    function lineOf(row) {
        return menuLines.answer === undefined ? undefined : menuLines.answer.lines.find(line => line.word === row)
    }
    /// Whether the row is there and can be pressed.
    function live(row) {
        return menuLines.has(row) && menuLines.why(row) === ""
    }

    /// Before a fresh stand: the shape a standing menu held goes, on every level (`AppMenu.releaseShape`). The stand
    /// reads whatever moved.
    function release() {
        menuLines.followDue = false
        menuLines.eachLevel(level => level.releaseShape())
    }
    /// The menu stands again with it up — `why` as `standAgain` says it. Nothing while it is down. The stand reads
    /// whatever moved, so a follow still due has nothing left to do.
    function again(why) {
        if (menuLines.menu === null || !menuLines.menu.visible)
            return
        menuLines.followDue = false
        menuLines.eachLevel(level => level.holdShape())
        menuLines.standAgain(why)
    }
    function follow() {
        if (menuLines.followDue && !menuLines.waiting)
            menuLines.again("follow")
    }
    /// The command that was out ended: every level lets its wait go (`AppMenu.waiting`) and the menu stands again
    /// once — as a wait when a level on show had waited for it, else following what it moved. The one end the rule
    /// reads: the levels' own `runningWhy` ends with it, and standing on both would stand the menu twice.
    function commandEnded() {
        let waited = false
        menuLines.eachLevel(level => {
            waited = waited || (level.visible && level.waiting)
            level.waiting = false
        })
        menuLines.again(waited ? "wait" : "follow")
    }
    onMovedChanged: {
        menuLines.followDue = true
        followLater.restart()
    }
    onWaitingChanged: if (!menuLines.waiting) menuLines.commandEnded()
    readonly property TurnLater followLater: TurnLater {
        id: followLater
        onTriggered: menuLines.follow()
    }

    /// `act` on the first level and on every card under it — one outside the type too (`GitHubMenu`), whose rows
    /// nothing here draws but which keep their shape with the menu's all the same (`again`): a row of it going off
    /// offer with the menu up greys where it stands (`AppMenuItem.withdrawn`) rather than leave the card.
    function eachLevel(act) {
        if (menuLines.menu === null)
            return
        const levels = [menuLines.menu]
        for (let at = 0; at < levels.length; at++) {
            act(levels[at])
            for (let i = 0; i < levels[at].count; i++) {
                const item = levels[at].itemAt(i)
                if (item && item.subMenu && item.subMenu.holdShape !== undefined)
                    levels.push(item.subMenu)
            }
        }
    }
    /// The card of the type the title row `item` opens; null for any other row.
    function cardOf(item) {
        return item && item.subMenu && item.subMenu.word !== undefined && item.subMenu.word !== "" ? item.subMenu
                                                                                                    : null
    }
    /// A row of the type: one that names its kind.
    function drawnRow(item) {
        return item && item.word !== undefined && item.word !== ""
    }

    function draw() {
        if (menuLines.menu === null || menuLines.answer === undefined)
            return
        const missed = []
        menuLines.drawLevel(menuLines.menu, "", missed)
        menuLines.undrawn = missed
        // ASCII, as Qt's own warnings: a run reads it whatever the console's code page (`verify::gaps`).
        if (missed.length > 0)
            console.warn("MenuLines: a row with no part: " + missed.join(", "))
    }
    /// What the level of `card` ("" for the first level) draws, in order: its rows, and on the first level each card's
    /// title where the card's first row stands (a card's rows are one run in drawing order).
    function entriesOf(card) {
        const entries = []
        for (const line of menuLines.answer.lines) {
            if (card !== "") {
                if (line.card === card)
                    entries.push({ "card": "", "word": line.word, "why": line.why, "group": line.cardGroup })
            } else if (line.card === "") {
                entries.push({ "card": "", "word": line.word, "why": line.why, "group": line.group })
            } else if (entries.length === 0 || entries[entries.length - 1].card !== line.card) {
                entries.push({ "card": line.card, "word": "", "why": "", "group": line.group })
            }
        }
        return entries
    }
    /// Draws the level of `card` ("" for the first level) and the cards on it, naming in `missed` what the answer has
    /// that the level has no part for.
    function drawLevel(level, card, missed) {
        const entries = menuLines.entriesOf(card)
        const parts = []
        for (let i = 0; i < level.count; i++) {
            const item = level.itemAt(i)
            const sub = menuLines.cardOf(item)
            if (sub !== null) {
                parts.push("card:" + sub.word)
                continue
            }
            if (!menuLines.drawnRow(item))
                continue
            parts.push(item.word)
            const entry = entries.find(each => each.card === "" && each.word === item.word)
            item.offered = entry !== undefined
            item.why = entry !== undefined ? entry.why : ""
        }
        for (const entry of entries) {
            if (parts.indexOf(entry.card !== "" ? "card:" + entry.card : entry.word) < 0)
                missed.push(card !== "" ? card + "/" + entry.word : entry.card !== "" ? "card " + entry.card
                                                                                       : entry.word)
        }
        if (!level.shapeHeld)
            menuLines.arrange(level, entries)
        // Every card of the type on this level, one the answer has no rows for too: its rows go out with it. Whether it
        // applies is the fresh stand's, as which rows stand is: standing again, a card whose rows the answer took away
        // keeps its title row, greyed with them and still opening (`AppMenuItem.cardOut`), where each row says why.
        for (let i = 0; i < level.count; i++) {
            const sub = menuLines.cardOf(level.itemAt(i))
            if (sub === null)
                continue
            if (!level.shapeHeld)
                sub.applies = entries.some(each => each.card === sub.word)
            menuLines.drawLevel(sub, sub.word, missed)
        }
    }
    /// The level's drawn rows and cards in the answer's order at its head, a divider wherever the group changes; the
    /// rest — rows the answer has not, and what stands outside the type — after them as they were.
    function arrange(level, entries) {
        for (let i = level.count - 1; i >= 0; i--) {
            const item = level.itemAt(i)
            if (item && item.drawn === true)
                level.removeItem(item)
        }
        const groups = []
        for (let e = 0; e < entries.length; e++) {
            const entry = entries[e]
            let from = -1
            for (let i = groups.length; i < level.count && from < 0; i++) {
                const item = level.itemAt(i)
                if (entry.card !== "" ? menuLines.cardOf(item) !== null && item.subMenu.word === entry.card
                                      : menuLines.drawnRow(item) && item.word === entry.word)
                    from = i
            }
            // A row this level has no part for stands nowhere; `drawLevel` named it.
            if (from < 0)
                continue
            if (from !== groups.length)
                level.moveItem(from, groups.length)
            groups.push(entry.group)
        }
        // Made under the menu's face, an item every menu has, then moved into the list by `insertItem`; `removeItem` above
        // destroys it. Made under the menu itself, which is no item, it stands out of the scene and Qt warns; made under
        // the menu's list, it does not divide (`tst_menulines`).
        for (let at = groups.length - 1; at > 0; at--) {
            if (groups[at] !== groups[at - 1])
                level.insertItem(at, dividerPart.createObject(level.background, { "inMenu": level }))
        }
    }
    /// A divider between two groups of drawn rows; `drawn` tells it from one the menu declares.
    readonly property Component dividerPart: Component {
        id: dividerPart
        AppMenuSeparator {
            readonly property bool drawn: true
        }
    }
}
