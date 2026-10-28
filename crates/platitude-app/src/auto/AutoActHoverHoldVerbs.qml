pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// What holds a hover display up and what keeps one from coming out, read stage by stage through the doors a reader
/// uses — the folded rail's cells, a row's right-click and the menu key, the author row's lines: a section held by its
/// own rows (`peek-hold`), what stands behind a menu (`menu-block`), and the author row's cards one at a time
/// (`author-swap`). A run stops at the moment its argument names and is photographed there, so the moments laid side
/// by side on the board read as the behaviour (verify-ui スキル §board).
// `Item` because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    /// `var`: typing it `AutoActDriver` would be circular — that file builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var worktree: driver.worktree
    readonly property var graphModel: driver.graphModel
    readonly property var branchesModel: driver.branchesModel
    readonly property var tagsModel: driver.tagsModel
    readonly property var sidebarPane: driver.sidebarPane
    readonly property var navProbe: driver.navProbe
    readonly property var wipPane: driver.wipPane
    readonly property var detailsPane: driver.detailsPane
    readonly property var diffPane: driver.diffPane

    function run(act, arg) {
        const cut = arg.indexOf(":")
        const head = cut < 0 ? arg : arg.substring(0, cut)
        const tail = cut < 0 ? "" : arg.substring(cut + 1)
        if (act === "peek-hold") {
            // `<how>[:released]`: the BRANCHES section held by a menu raised on its current branch's row (`menu`), by
            // a name box open on it (`rename`), or by nothing (`none`), and the hand then on the TAGS cell. `released`
            // goes on until the hold lets go with the hand still on TAGS.
            page.foldByHand(true)
            peekHoldTimer.how = head === "" ? "menu" : head
            peekHoldTimer.released = tail === "released"
            // Nothing held is swapped at once; a hold is walked off, and let go where `released` asks.
            peekHoldTimer.order = peekHoldTimer.how === "none" ? ["peeked", "swapped"]
                                : peekHoldTimer.released ? ["peeked", "held", "walked", "let_go"]
                                : ["peeked", "held", "walked"]
            peekHoldTimer.start()
        } else if (act === "menu-block") {
            // `<owner>[:<stage>]` — the display that stands behind a menu raised over it (owners and stages at
            // `menuBlockTimer`).
            menuBlockTimer.display = head === "" ? "eol" : head
            menuBlockTimer.stage = tail === "" ? "menu" : tail
            // The menu key's door asks for the keyboard on the second file before it raises; an owner with no second
            // target ends at the menu whatever stage was named.
            const ready = menuBlockTimer.display === "eol-key" ? "keyed" : "out"
            menuBlockTimer.order = menuBlockTimer.stage === "card" ? ["rows"]
                                 : menuBlockTimer.stage === "menu" || !menuBlockTimer.asksAgain
                                 ? ["rows", ready, "raised"]
                                 : menuBlockTimer.stage === "other" ? ["rows", ready, "raised", "asked"]
                                 : ["rows", ready, "raised", "asked", "down"]
            menuBlockTimer.start()
        } else if (act === "author-swap") {
            // `<from>:<to>` among `author` / `mates` / `parents`: one card out, the hand off its line and onto the
            // next — read in the same turn, before any beat could run.
            authorSwapTimer.from = head === "" ? "author" : head
            authorSwapTimer.to = tail === "" ? "mates" : tail
            page.activateRow(graphModel.oidAt(0))
            authorSwapTimer.start()
        } else {
            return false
        }
        return true
    }

    // PGG_AUTO_ACT=peek-hold. The hand on the BRANCHES cell of the folded rail, the section held by its own row, then
    // the hand off it and onto the TAGS cell (`NavRail.enterAt` / `leaveAt`, the cells' own doors): held, the section
    // stays — the menu over its row, the box on it — and TAGS is an ask kept for the hold letting go
    // (`SectionPeekPopup`); let go with the hand on TAGS, TAGS comes out at once. With nothing held, TAGS is swapped in
    // the turn the hand reaches it.
    PageStage {
        id: peekHoldTimer
        name: "peek_hold"
        property string how: "menu"
        property bool released: false
        /// The door the menu was raised through, to tell it still stands for that row.
        property string door: ""
        function line(stage) {
            return "peek_hold how=" + peekHoldTimer.how + " stage=" + stage
                   + " peek=" + sidebarPane.peekKind
                   + " shown=" + sidebarPane.peekSection.visible
                   + " menu=" + page.menuStanding
                   + " door=" + (peekHoldTimer.door === "" ? "none" : !page.menuStanding ? "gone"
                                 : page.menuKey === peekHoldTimer.door ? "kept" : "moved")
                   + " edit=" + (sidebarPane.editKey !== "")
        }
        onTriggered: {
            const op = peekHoldTimer
            if (op.at("start")) {
                if (!Awaited.all("peek_hold_rail", {
                        "folded": page.sidebarCollapsed && sidebarPane.width > 0,
                        "branches": branchesModel.total > 0,
                        "tags": tagsModel.total > 0
                    }))
                    return
                op.input("peeked", () => {
                    navProbe.peekAt("branch")
                })
                return
            }
            if (op.at("peeked")) {
                const row = navProbe.peek.sectionModel ? navProbe.peek.sectionModel.rowOfName(worktree.branch) : -1
                if (!Awaited.all("peek_hold_open", {
                        "peek": navProbe.peekStanding && sidebarPane.peekKind === "branch",
                        "row": row >= 0
                    }))
                    return
                if (op.how === "none") {
                    op.input("swapped", () => {
                        navProbe.peekAway("branch")
                        navProbe.peekAt("tag")
                    })
                    op.complete(() => op.line("swap"))
                    return
                }
                op.input("held", () => {
                    // The row's own right-click (`NavList.rightClickRow`); not built yet, no press went in.
                    if (op.how === "menu")
                        return navProbe.peek.list.rightClickRow(row)
                    sidebarPane.beginRename("branch", worktree.branch, worktree.branch)
                })
                return
            }
            if (op.at("held")) {
                const held = op.how === "menu" ? page.menuStanding && page.menuArea === "peek"
                                               : sidebarPane.editKey !== ""
                if (!Awaited.all("peek_hold_held", { "held": held }))
                    return
                op.input("walked", () => {
                    op.door = op.how === "menu" ? page.menuKey : ""
                    navProbe.peekAway("branch")
                    navProbe.peekAt("tag")
                })
                if (!op.released) {
                    op.complete(() => op.line("held"))
                    return
                }
                // The hold lets go as the reader's Escape takes it: the menu closing by itself, the box dropped.
                op.input("let_go", () => {
                    if (op.how === "menu")
                        driver.refMenu.close()
                    else
                        sidebarPane.stopEdit()
                })
                return
            }
            if (!Awaited.all("peek_hold_released", {
                    "tags": sidebarPane.peekKind === "tag" && sidebarPane.peekSection.visible
                }))
                return
            op.complete(() => op.line("released"))
        }
    }

    // PGG_AUTO_ACT=menu-block. One display out, then a menu raised over it through a real door, then the display
    // asked for elsewhere under the menu, then the menu gone — stage by stage:
    //   `card`   — the display out;
    //   `menu`   — a menu raised: what was out goes the moment it stands (`down=` is read in the turn the door
    //              returns, before any beat could run);
    //   `other`  — another target asked for under the menu, waited out past its rest: nothing comes out;
    //   `back` / `back-item` / `back-outside` — the menu gone by Escape (it closes by itself), by one of its rows
    //              taken, by a press outside (`OutsidePress.closeAll`): the ask kept for the other target comes out.
    // Owners: `eol` (the worktree's line-ending card; the file menu raised by the right-click door), `eol-key` (the
    // same through the menu key), `author` (the author row's cards; the graph row's menu) — every stage; and for the
    // first two stages alone, `author-files` (the author card; the commit's changed file's menu, `--preset github`),
    // `recover` (the discard log's card; the graph row's menu) and `tip` (the window's tooltip on a changed file's row;
    // the graph row's menu).
    PageStage {
        id: menuBlockTimer
        name: "menu_block"
        /// The owner of the display put out (`owner=` in the line).
        property string display: "eol"
        property string stage: "menu"
        /// The display went in the turn the menu's door returned.
        property bool downAtOnce: false
        readonly property string first: "src/flipped.kt"
        readonly property string second: "src/mixed.kt"
        function shown() {
            const owner = menuBlockTimer.display
            if (owner === "eol" || owner === "eol-key")
                return wipPane.eolCardOpen
            if (owner === "author" || owner === "author-files")
                return detailsPane.authorCards.authorCardOpen || detailsPane.authorCards.matesCardOpen
            if (owner === "recover")
                return sidebarPane.autoRecover.cardOpen
            return Tips.current !== null && Tips.box !== null && Tips.box.visible
        }
        function said() {
            const owner = menuBlockTimer.display
            if (owner === "eol" || owner === "eol-key")
                return "path=" + (wipPane.eolCardOpen ? wipPane.eolHoverCard.path : "")
            if (owner === "author" || owner === "author-files")
                return "author=" + detailsPane.authorCards.authorCardOpen
                       + " mates=" + detailsPane.authorCards.matesCardOpen
            return ""
        }
        /// Whether this owner has a second target to ask for under the menu (the later stages).
        readonly property bool asksAgain: menuBlockTimer.display === "eol" || menuBlockTimer.display === "eol-key"
                                          || menuBlockTimer.display === "author"
        function line(stage) {
            return "menu_block owner=" + menuBlockTimer.display + " stage=" + stage
                   + " shown=" + menuBlockTimer.shown()
                   + " menu=" + page.menuStanding
                   + " down=" + menuBlockTimer.downAtOnce
                   + " " + menuBlockTimer.said()
        }
        /// Puts the display out the way its run stands in for a hand; says whether it is out.
        function putOut() {
            const owner = menuBlockTimer.display
            if (owner === "eol" || owner === "eol-key") {
                wipPane.pointEol("unstaged:" + menuBlockTimer.first)
                return wipPane.eolCardOpen
            }
            if (owner === "author" || owner === "author-files") {
                if (!driver.cardSettled)
                    return false
                detailsPane.valueRow.showAuthor(true)
                return detailsPane.authorCards.authorCardOpen
            }
            if (owner === "recover")
                return sidebarPane.autoRecover.restOnShown(0) && sidebarPane.autoRecover.cardOpen
            detailsPane.pointedTipRow = 0
            return Tips.current !== null && Tips.box.visible
        }
        /// Raises the menu over it through the owner's door; says whether it went in.
        function raise() {
            const owner = menuBlockTimer.display
            if (owner === "eol") {
                page.openFileMenu("unstaged", menuBlockTimer.second)
                return true
            }
            if (owner === "eol-key")
                return wipPane.menuFromKeys()
            if (owner === "author-files")
                return page.openCommitFileMenu("src/login.txt", "", "M")
            page.openRowMenu(worktree.headOid)
            return true
        }
        function menuItself() {
            const owner = menuBlockTimer.display
            if (owner === "eol" || owner === "eol-key")
                return driver.fileMenu
            if (owner === "author-files")
                return driver.commitFileMenu.menu
            return driver.commitMenu
        }
        function askOther() {
            const owner = menuBlockTimer.display
            if (owner === "eol" || owner === "eol-key") {
                wipPane.pointEol("unstaged:" + menuBlockTimer.second)
            } else if (owner === "author" || owner === "author-files") {
                detailsPane.valueRow.showAuthor(false)
                detailsPane.valueRow.showCoAuthors(true)
            }
        }
        function takeDown(how) {
            const menu = menuBlockTimer.menuItself()
            if (how === "back-outside") {
                OutsidePress.closeAll()
            } else if (how === "back-item") {
                // A row that changes nothing on disk, taken as a press takes it: the file's path to the clipboard.
                for (let i = 0; i < menu.count; i++) {
                    const row = menu.itemAt(i)
                    if (row && row.present && row.enabled && row.text !== undefined && row.text.indexOf("Copy") === 0) {
                        row.click()
                        return
                    }
                }
                Awaited.note("menu_block no row to take")
            } else {
                menu.close()
            }
        }
        onTriggered: {
            const op = menuBlockTimer
            const owner = op.display
            // Past the start, the page the stage reads is whole (`PageSettled`): the row activated at the start
            // brings a card and a file list, and `author-files` a diff.
            if (!op.at("start") && !Awaited.all("menu_block_page", { "page": PageSettled.settled(page) }))
                return
            if (op.at("start")) {
                // Asked again each beat until the rows are there.
                const eol = owner === "eol" || owner === "eol-key"
                if (eol)
                    page.showWip()
                else if (owner === "recover" && !page.recoverOpen)
                    page.toggleRecover()
                else if (owner === "author" || owner === "tip")
                    page.activateRow(graphModel.oidAt(owner === "tip" ? 1 : 4))
                else if (owner === "author-files")
                    page.activateRow(graphModel.oidAt(1))
                // What the owner's display hangs off: the two file rows, or the log's answer; a row activated is in.
                if (!Awaited.all("menu_block_rows", {
                        "first": !eol || wipPane.rowIn("unstaged", op.first) !== null,
                        "second": !eol || wipPane.rowIn("unstaged", op.second) !== null,
                        "read": owner !== "recover" || page.recoverAnswered
                    }))
                    return
                op.advance("rows")
                return
            }
            if (op.at("rows")) {
                // Put out again each beat until it is.
                const out = op.putOut()
                if (!Awaited.all("menu_block_out", { "out": out }))
                    return
                if (op.stage === "card") {
                    op.complete(() => op.line("card"))
                    return
                }
                // The menu key's door asks for the row the keyboard stands on: the second file chosen and the
                // keyboard on its list — no press, which would be another input over the card (`WipPane.menuFromKeys`).
                if (owner === "eol-key") {
                    const row = wipPane.filesWalk.rowFor("unstaged", op.second)
                    // Not built yet: the menu key would ask a list the keyboard is not on.
                    if (!Awaited.all("menu_block_keys", { "row": row !== null && row !== undefined }))
                        return
                    op.input("keyed", () => {
                        wipPane.chooseOnly("unstaged", op.second)
                        row.ListView.view.forceActiveFocus()
                    })
                    return
                }
                op.advance("out")
                return
            }
            if (op.at("out") || op.at("keyed")) {
                // Asked again each beat until the door takes it; what was out is read in the turn it returns.
                op.input("raised", () => {
                    if (!op.raise())
                        return false
                    op.downAtOnce = !op.shown()
                })
                return
            }
            if (op.at("raised")) {
                // The commit's file menu opens the file's diff on the way (`RepoPage.openCommitFileMenu`): read once
                // that has landed, so the frame — and the census it records — has the file's rows every time.
                if (!Awaited.all("menu_block_menu", {
                        "menu": page.menuStanding,
                        "read": owner !== "author-files"
                                || (page.diffPath === "src/login.txt" && diffPane.diffSettled())
                    }))
                    return
                if (op.stage === "menu" || !op.asksAgain) {
                    op.complete(() => op.line("menu"))
                    return
                }
                op.input("asked", () => {
                    op.askOther()
                })
                return
            }
            if (op.at("asked")) {
                // Nothing may come out under the menu: a seat the ask reached is deciding until its rest runs out
                // (`PageSettled`'s `hover:` term), one the menu holds back keeps the ask and decides nothing — read
                // after a frame and the page, what is not out was held back.
                if (!Awaited.all("menu_block_asked", { "drawn": op.drawnSince("menu_block_other"),
                                                       "page": PageSettled.settled(page) }))
                    return
                if (op.stage === "other") {
                    op.complete(() => op.line("other"))
                    return
                }
                op.input("down", () => {
                    op.takeDown(op.stage)
                })
                return
            }
            if (!Awaited.all("menu_block_back", { "menu": !page.menuStanding, "shown": op.shown() }))
                return
            op.complete(() => op.line(op.stage))
        }
    }

    // PGG_AUTO_ACT=author-swap. The hand on one line of the author row (`from`) with its card out, then off it and
    // onto another line (`to`) in one turn — what a hand crossing the row does, the stand-ins written as the real
    // hover writes them (`CommitAuthorRow.show*`): the card opening takes the one before down at once, so only one is
    // ever out (`DetailsAuthorCards.giveWay`), read in that same turn and again a beat later.
    PageStage {
        id: authorSwapTimer
        name: "author_swap"
        order: ["swapped"]
        property string from: "author"
        property string to: "mates"
        /// The cards as the swap left them, read in its turn.
        property string atOnce: ""
        function show(which, on) {
            if (which === "author")
                detailsPane.valueRow.showAuthor(on)
            else if (which === "mates")
                detailsPane.valueRow.showCoAuthors(on)
            else
                detailsPane.valueRow.showParents(on)
        }
        function open(which) {
            const cards = detailsPane.authorCards
            return which === "author" ? cards.authorCardOpen
                 : which === "mates" ? cards.matesCardOpen : cards.parentCardOpen
        }
        function count() {
            const cards = detailsPane.authorCards
            return (cards.authorCardOpen ? 1 : 0) + (cards.matesCardOpen ? 1 : 0) + (cards.parentCardOpen ? 1 : 0)
        }
        onTriggered: {
            const op = authorSwapTimer
            if (op.at("start")) {
                // The hand on the first line, written again each beat until its card is out.
                const settled = driver.cardSettled
                if (settled)
                    op.show(op.from, true)
                if (!Awaited.all("author_swap_from", {
                        "settled": settled,
                        "out": settled && op.open(op.from)
                    }))
                    return
                op.input("swapped", () => {
                    op.show(op.from, false)
                    op.show(op.to, true)
                    op.atOnce = "open=" + op.count()
                        + " " + op.from + "=" + op.open(op.from)
                        + " " + op.to + "=" + op.open(op.to)
                })
                return
            }
            // A beat later, on the page the swap left — which the completion's hold waits for as well.
            if (!Awaited.all("author_swap_later", { "page": PageSettled.settled(page) }))
                return
            op.complete(() => "author_swap from=" + op.from + " to=" + op.to
                              + " now: " + op.atOnce
                              + " later: open=" + op.count()
                              + " " + op.to + "=" + op.open(op.to))
        }
    }
}
