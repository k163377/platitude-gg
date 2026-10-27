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
            peekHoldTimer.step = 0
            peekHoldTimer.start()
        } else if (act === "menu-block") {
            // `<owner>[:<stage>]` — the display that stands behind a menu raised over it (owners and stages at
            // `menuBlockTimer`).
            menuBlockTimer.owner = head === "" ? "eol" : head
            menuBlockTimer.stage = tail === "" ? "menu" : tail
            menuBlockTimer.step = 0
            menuBlockTimer.waitedFrom = 0
            menuBlockTimer.start()
        } else if (act === "author-swap") {
            // `<from>:<to>` among `author` / `mates` / `parents`: one card out, the hand off its line and onto the
            // next — read in the same turn, before any beat could run.
            authorSwapTimer.from = head === "" ? "author" : head
            authorSwapTimer.to = tail === "" ? "mates" : tail
            authorSwapTimer.step = 0
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
    SampleTimer {
        id: peekHoldTimer
        property string how: "menu"
        property bool released: false
        property int step: 0
        /// The door the menu was raised through, to tell it still stands for that row.
        property string door: ""
        function done(stage) {
            peekHoldTimer.stop()
            Harness.report("peek_hold how=" + peekHoldTimer.how + " stage=" + stage
                           + " peek=" + sidebarPane.peekKind
                           + " shown=" + sidebarPane.peekSection.visible
                           + " menu=" + page.menuStanding
                           + " door=" + (peekHoldTimer.door === "" ? "none" : !page.menuStanding ? "gone"
                                         : page.menuKey === peekHoldTimer.door ? "kept" : "moved")
                           + " edit=" + (sidebarPane.editKey !== ""))
            driver.complete()
        }
        onTriggered: {
            if (peekHoldTimer.step === 0) {
                if (!Awaited.all("peek_hold_rail", {
                        "folded": page.sidebarCollapsed && sidebarPane.width > 0,
                        "branches": branchesModel.total > 0,
                        "tags": tagsModel.total > 0
                    }))
                    return
                navProbe.peekAt("branch")
                peekHoldTimer.step = 1
                return
            }
            if (peekHoldTimer.step === 1) {
                const row = navProbe.peek.sectionModel ? navProbe.peek.sectionModel.rowOfName(worktree.branch) : -1
                if (!Awaited.all("peek_hold_open", {
                        "peek": navProbe.peekStanding && sidebarPane.peekKind === "branch",
                        "row": row >= 0
                    }))
                    return
                if (peekHoldTimer.how === "none") {
                    navProbe.peekAway("branch")
                    navProbe.peekAt("tag")
                    peekHoldTimer.done("swap")
                    return
                }
                if (peekHoldTimer.how === "menu") {
                    // The row's own right-click (`NavList.rightClickRow`); not built yet, no press went in.
                    if (!Awaited.all("peek_hold_menu", { "row": navProbe.peek.list.rightClickRow(row) }))
                        return
                } else {
                    sidebarPane.beginRename("branch", worktree.branch, worktree.branch)
                }
                peekHoldTimer.step = 2
                return
            }
            if (peekHoldTimer.step === 2) {
                const held = peekHoldTimer.how === "menu" ? page.menuStanding && page.menuArea === "peek"
                                                         : sidebarPane.editKey !== ""
                if (!Awaited.all("peek_hold_held", { "held": held }))
                    return
                peekHoldTimer.door = peekHoldTimer.how === "menu" ? page.menuKey : ""
                navProbe.peekAway("branch")
                navProbe.peekAt("tag")
                if (!peekHoldTimer.released) {
                    peekHoldTimer.done("held")
                    return
                }
                // The hold lets go as the reader's Escape takes it: the menu closing by itself, the box dropped.
                if (peekHoldTimer.how === "menu")
                    driver.refMenu.close()
                else
                    sidebarPane.stopEdit()
                peekHoldTimer.step = 3
                return
            }
            if (!Awaited.all("peek_hold_released", {
                    "tags": sidebarPane.peekKind === "tag" && sidebarPane.peekSection.visible
                }))
                return
            peekHoldTimer.done("released")
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
    SampleTimer {
        id: menuBlockTimer
        property string owner: "eol"
        property string stage: "menu"
        property int step: 0
        /// The display went in the turn the menu's door returned.
        property bool downAtOnce: false
        property real waitedFrom: 0
        readonly property string first: "src/flipped.kt"
        readonly property string second: "src/mixed.kt"
        function shown() {
            const owner = menuBlockTimer.owner
            if (owner === "eol" || owner === "eol-key")
                return wipPane.eolCardOpen
            if (owner === "author" || owner === "author-files")
                return detailsPane.authorCards.authorCardOpen || detailsPane.authorCards.matesCardOpen
            if (owner === "recover")
                return sidebarPane.autoRecover.cardOpen
            return Tips.current !== null && Tips.box !== null && Tips.box.visible
        }
        function said() {
            const owner = menuBlockTimer.owner
            if (owner === "eol" || owner === "eol-key")
                return "path=" + (wipPane.eolCardOpen ? wipPane.eolHoverCard.path : "")
            if (owner === "author" || owner === "author-files")
                return "author=" + detailsPane.authorCards.authorCardOpen
                       + " mates=" + detailsPane.authorCards.matesCardOpen
            return ""
        }
        /// Whether this owner has a second target to ask for under the menu (the later stages).
        readonly property bool asksAgain: owner === "eol" || owner === "eol-key" || owner === "author"
        function done(stage) {
            menuBlockTimer.stop()
            Harness.report("menu_block owner=" + menuBlockTimer.owner + " stage=" + stage
                           + " shown=" + menuBlockTimer.shown()
                           + " menu=" + page.menuStanding
                           + " down=" + menuBlockTimer.downAtOnce
                           + " " + menuBlockTimer.said())
            driver.complete()
        }
        /// Puts the display out the way its run stands in for a hand; says whether it is out.
        function putOut() {
            const owner = menuBlockTimer.owner
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
            const owner = menuBlockTimer.owner
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
            const owner = menuBlockTimer.owner
            if (owner === "eol" || owner === "eol-key")
                return driver.fileMenu
            if (owner === "author-files")
                return driver.commitFileMenu.menu
            return driver.commitMenu
        }
        function askOther() {
            const owner = menuBlockTimer.owner
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
                Harness.report("menu_block no row to take")
            } else {
                menu.close()
            }
        }
        onTriggered: {
            const owner = menuBlockTimer.owner
            // Past the first step, the page the stage reads is whole (`PageSettled`): the row activated at step 0
            // brings a card and a file list, and `author-files` a diff.
            if (menuBlockTimer.step > 0
                    && !Awaited.all("menu_block_page/" + menuBlockTimer.step, { "page": PageSettled.settled(page) }))
                return
            if (menuBlockTimer.step === 0) {
                if (owner === "eol" || owner === "eol-key") {
                    page.showWip()
                    if (!Awaited.all("menu_block_rows", {
                            "first": wipPane.rowIn("unstaged", menuBlockTimer.first) !== null,
                            "second": wipPane.rowIn("unstaged", menuBlockTimer.second) !== null
                        }))
                        return
                } else if (owner === "recover") {
                    if (!page.recoverOpen)
                        page.toggleRecover()
                    if (!Awaited.all("menu_block_log", { "read": page.recoverAnswered }))
                        return
                } else if (owner === "author" || owner === "tip") {
                    page.activateRow(graphModel.oidAt(owner === "tip" ? 1 : 4))
                } else if (owner === "author-files") {
                    page.activateRow(graphModel.oidAt(1))
                }
                menuBlockTimer.step = 1
                return
            }
            if (menuBlockTimer.step === 1) {
                if (!Awaited.all("menu_block_out", { "out": menuBlockTimer.putOut() }))
                    return
                if (menuBlockTimer.stage === "card") {
                    menuBlockTimer.done("card")
                    return
                }
                // The menu key's door asks for the row the keyboard stands on: the second file chosen and the
                // keyboard on its list — no press, which would be another input over the card (`WipPane.menuFromKeys`).
                if (owner === "eol-key") {
                    const row = wipPane.filesWalk.rowFor("unstaged", menuBlockTimer.second)
                    // Not built yet: the menu key would ask a list the keyboard is not on.
                    if (!Awaited.all("menu_block_keys", { "row": row !== null && row !== undefined }))
                        return
                    wipPane.chooseOnly("unstaged", menuBlockTimer.second)
                    row.ListView.view.forceActiveFocus()
                }
                menuBlockTimer.step = 2
                return
            }
            if (menuBlockTimer.step === 2) {
                // Asked again each beat until the door takes it — named, so a stall says this stage.
                if (!Awaited.all("menu_block_raise", { "menu": menuBlockTimer.raise() }))
                    return
                menuBlockTimer.downAtOnce = !menuBlockTimer.shown()
                menuBlockTimer.step = 3
                return
            }
            if (menuBlockTimer.step === 3) {
                // The commit's file menu opens the file's diff on the way (`RepoPage.openCommitFileMenu`): read once
                // that has landed, so the frame — and the census it records — has the file's rows every time.
                if (!Awaited.all("menu_block_menu", {
                        "menu": page.menuStanding,
                        "read": owner !== "author-files"
                                || (page.diffPath === "src/login.txt" && diffPane.diffSettled())
                    }))
                    return
                if (menuBlockTimer.stage === "menu" || !menuBlockTimer.asksAgain) {
                    menuBlockTimer.done("menu")
                    return
                }
                menuBlockTimer.askOther()
                // waits(timed): the absence is read from here — nothing may come under the menu in twice its rest
                menuBlockTimer.waitedFrom = Date.now()
                menuBlockTimer.step = 4
                return
            }
            if (menuBlockTimer.step === 4) {
                // Past the rest the other target would have waited: nothing came out under the menu.
                // waits(timed): the product's rest, as a floor: a card that would come has come by then
                if (Date.now() - menuBlockTimer.waitedFrom < 2 * Metrics.tipDelayMs)
                    return
                if (menuBlockTimer.stage === "other") {
                    menuBlockTimer.done("other")
                    return
                }
                menuBlockTimer.takeDown(menuBlockTimer.stage)
                menuBlockTimer.step = 5
                return
            }
            if (!Awaited.all("menu_block_back", { "menu": !page.menuStanding, "shown": menuBlockTimer.shown() }))
                return
            menuBlockTimer.done(menuBlockTimer.stage)
        }
    }

    // PGG_AUTO_ACT=author-swap. The hand on one line of the author row (`from`) with its card out, then off it and
    // onto another line (`to`) in one turn — what a hand crossing the row does, the stand-ins written as the real
    // hover writes them (`CommitAuthorRow.show*`): the card opening takes the one before down at once, so only one is
    // ever out (`DetailsAuthorCards.giveWay`), read in that same turn and again a beat later.
    SampleTimer {
        id: authorSwapTimer
        property string from: "author"
        property string to: "mates"
        property int step: 0
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
            if (authorSwapTimer.step === 0) {
                const settled = driver.cardSettled
                if (settled)
                    authorSwapTimer.show(authorSwapTimer.from, true)
                if (!Awaited.all("author_swap_from", {
                        "settled": settled,
                        "out": settled && authorSwapTimer.open(authorSwapTimer.from)
                    }))
                    return
                authorSwapTimer.show(authorSwapTimer.from, false)
                authorSwapTimer.show(authorSwapTimer.to, true)
                authorSwapTimer.atOnce = "open=" + authorSwapTimer.count()
                    + " " + authorSwapTimer.from + "=" + authorSwapTimer.open(authorSwapTimer.from)
                    + " " + authorSwapTimer.to + "=" + authorSwapTimer.open(authorSwapTimer.to)
                authorSwapTimer.step = 1
                return
            }
            authorSwapTimer.stop()
            Harness.report("author_swap from=" + authorSwapTimer.from + " to=" + authorSwapTimer.to
                           + " now: " + authorSwapTimer.atOnce
                           + " later: open=" + authorSwapTimer.count()
                           + " " + authorSwapTimer.to + "=" + authorSwapTimer.open(authorSwapTimer.to))
            driver.complete()
        }
    }
}
