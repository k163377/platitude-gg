pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The sidebar's own verbs: folding and peeking at a section, the second click on a row that is already lit,
/// filtering, closing, and the rows that answer for a tag.
// `Item` because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    /// `var`: typing it `AutoActDriver` would be circular — that file builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var repoTab: driver.repoTab
    readonly property var worktree: driver.worktree
    readonly property var graphModel: driver.graphModel
    readonly property var graphPane: driver.graphPane
    readonly property var branchesModel: driver.branchesModel
    readonly property var remotesModel: driver.remotesModel
    readonly property var stashesModel: driver.stashesModel
    readonly property var tagsModel: driver.tagsModel
    readonly property var sidebarPane: driver.sidebarPane
    readonly property var diffPane: driver.diffPane
    readonly property var navProbe: driver.navProbe
    readonly property var detailsPane: driver.detailsPane
    readonly property var remoteDialog: driver.remoteDialog
    readonly property var refSwitchItem: driver.refSwitchItem
    readonly property var commitMenu: driver.commitMenu
    readonly property var dropCommitItem: driver.dropCommitItem

    /// Runs `act` if it is this family's and says whether it was; `AutoActDriver` asks each family in turn.
    function run(act, arg) {
        if (act === "nav-dbl") {
            // A double-click in the left menu, entered where the row enters it. The argument is `<section>:<name>`.
            const cut = arg.indexOf(":")
            const section = arg.substring(0, cut)
            const rowName = arg.substring(cut + 1)
            const model = section === "tag" ? tagsModel : section === "remote" ? remotesModel : branchesModel
            sidebarPane.activateRow(section, rowName, rowName,
                                    model.oidOfName(rowName))
        } else if (act === "nav-fold" || act === "nav-peek"
                   || act === "nav-unfold" || act === "nav-peek-rename"
                   || act === "nav-peek-away" || act === "nav-peek-into"
                   || act === "nav-peek-out" || act === "nav-peek-shut") {
            // Hover cannot be injected, so the rail cell is named. "-away" walks the pointer off the cell, "-into" down
            // into the opened list, "-out" on out the far side, "-shut" clicks the cell; only "-into" leaves the
            // section standing. "nav-peek" on an empty section stays shut (NavRail.enterAt).
            //
            // The section goes a beat after the pointer leaves (`HoverSeat`, `Metrics.hoverKeepMs`), so the three
            // whose subject is its going wait for it (`peekGoneWanted`) — finished on the rail's answer, the run
            // frames it standing.
            if (arg === "no-tags")
                repoTab.setTagsShown(false)
            page.foldByHand(true)
            if (act === "nav-peek")
                navProbe.peekAt(arg)
            else if (act === "nav-peek-away") {
                navProbe.peekAt(arg)
                navProbe.peekAway(arg)
            } else if (act === "nav-peek-into" || act === "nav-peek-out") {
                navProbe.peekAt(arg)
                navProbe.peekInto(arg)
                if (act === "nav-peek-out")
                    navProbe.peekOut()
            } else if (act === "nav-peek-shut") {
                navProbe.peekAt(arg)
                navProbe.peekTap(arg)
            } else if (act === "nav-unfold")
                page.foldByHand(false)
            else if (act === "nav-peek-rename") {
                // The box lands on the peeked row and holds the section open — the list is not put back for it
                // (SidebarRowGestures.startEdit).
                navProbe.peekAt("branch")
                sidebarPane.beginRename("branch", worktree.branch,
                                        worktree.branch)
            }
            navRailTimer.start()
        } else if (act === "nav-peek-again") {
            // The cell clicked shut and clicked again with the hand where it was: the second click opens — a press is
            // a new ask, past the latch the first click left (`SectionPeekPopup.toggleAt`, `HoverSeat.press`).
            page.foldByHand(true)
            peekAgainTimer.kind = arg === "" ? "branch" : arg
            peekAgainTimer.start()
        } else if (act === "menu-peek") {
            // The folded rail's section and a standing menu (デザイン規約 §左メニューを畳む). `behind` rests on the
            // BRANCHES cell with a graph row's menu up — it opens — and walks off it; `under` raises that menu over the
            // open section; `own` raises the menu on one of the section's rows and walks the hand out — the one menu it
            // stays under; `own-swap` rests on the TAGS cell under that menu and walks off it — the held section is not
            // swapped: BRANCHES stays and TAGS never comes in.
            page.foldByHand(true)
            menuPeekTimer.how = arg === "" ? "behind" : arg
            // The menu or the section first, as `how` names, then the other.
            menuPeekTimer.order = menuPeekTimer.how === "behind" ? ["menu_raised", "walked"]
                                : menuPeekTimer.how === "under" ? ["peek_asked", "menu_raised"]
                                : ["peek_asked", "walked"]
            menuPeekTimer.start()
        } else if (act === "nav-peek-open") {
            // A row opened inside the peeked section: what is read is that the popup makes room and stays up while
            // the rows under it move.
            page.foldByHand(true)
            navProbe.peekAt("branch")
            peekOpenTimer.want = arg === "" ? 0 : Number(arg)
            peekOpenTimer.start()
        } else if (act === "tags-eye") {
            // The eye pressed at the TAGS band. The argument names the tag whose commit is the subject; `<tag>:back`
            // presses again so the round trip is read whole.
            const backing = arg.endsWith(":back")
            const tagName = backing ? arg.substring(0, arg.length - ":back".length) : arg
            tagEyeTimer.oid = tagsModel.oidOfName(tagName)
            tagEyeTimer.back = backing
            tagEyeTimer.order = backing ? ["hidden", "shown"] : ["hidden"]
            tagEyeTimer.start()
        } else if (act === "nav-jump") {
            // One click on a row of the left panel (デザイン規約 §左メニューの所作), put in at the row's own `leftClick`
            // (`NavList.clickRow`) so the routing tested is the delegate's.
            jumpTimer.start()
        } else if (act === "nav-reclick" || act === "nav-reclick-away") {
            // The rename gesture (two clicks on one row) in the peeked section. "-away" lets the pointer leave in
            // between, and a list that went and came back reads the second as a first click (デザイン規約 §左メニューの所作).
            page.foldByHand(true)
            reclickTimer.away = act === "nav-reclick-away"
            // The section peeked before each click: again after the walk off for "-away".
            reclickTimer.order = reclickTimer.away
                ? ["peeked", "first_click", "walked_off", "gone", "peeked_again", "second_click"]
                : ["peeked", "first_click", "second_click"]
            reclickTimer.start()
        } else if (act === "nav-reclick-elsewhere") {
            // The same gesture on the open list by the hand's own road — each click a press through the window's
            // watcher, then the release the row answers at — with a press on the details pane after the first
            // (デザイン規約 §左メニューの所作「間を空けた 2 回目」).
            elsewhereTimer.start()
        } else if (act === "nav-add-remote") {
            // The `+` of the REMOTES band, live even while the band is unavailable (デザイン規約 §左メニューの所作), put
            // in at the band's own signal. "folded" is the other door: the rail's REMOTES cell, which at zero answers
            // its click with the dialog, put in at `NavRail.tapAt` (`peekTap`) so the routing tested is the cell's.
            if (arg === "folded") {
                page.foldByHand(true)
                navProbe.peekTap("remote")
            } else {
                navProbe.tapAddRemote()
            }
            navAddRemoteTimer.start()
        } else if (act === "doors-held") {
            // Every door onto the history (left pane and graph) while a replaying write runs; nothing freezes or dims
            // (`SidebarPane.doorsHeld`). The rebase is real — the argument is the branch rebased onto and the menus'
            // subject (`heldBranch`) — and its rise is latched below.
            repoTab.rebase(arg, "", true)
            navHeldTimer.start()
        } else if (act === "nav-pin-edge") {
            // The current branch's stand-in riding the edge its row scrolled out of (`HeadPinRow`): the list is taken
            // to its end so the row leaves above. The other way the stand-in shows (`nav-open head:<filter>`) has no
            // row to ride, so only the scroll reads this. No preset can make the lower edge: each checks out a
            // name that sorts early.
            pinEdgeTimer.start()
        } else if (act === "nav-pin-seat") {
            // The stand-in in the seat a fold opened for it, with no hand on it (`nav-open head::<row>` has to light
            // it). The argument is `<畳む行>[:<手を置く行>]`: the BRANCHES row shut at its own click (`NavProbe.clickRow`),
            // and optionally a row to rest on, whose opening pushes the seat down — a stand-in placed by counting
            // whole rows would stay put.
            const seatParts = ("" + arg).split(":")
            pinSeatTimer.row = seatParts[0] === "" ? 0 : Number(seatParts[0])
            pinSeatTimer.rest = seatParts.length > 1 ? Number(seatParts[1]) : -1
            pinSeatTimer.order = pinSeatTimer.rest >= 0 ? ["folded", "rested"] : ["folded"]
            pinSeatTimer.start()
        } else if (act === "nav-close") {
            // Sections stay packed against the top; a closed header resting at the pane's foot is the failure.
            navProbe.closeSection(arg)
            navSectionTimer.start()
        } else if (act === "nav-filter") {
            navProbe.typeFilter(arg)
            navFilterTimer.start()
        } else if (act === "diff-fold" || act === "diff-unfold"
                   || act === "diff-fold-by-hand"
                   || act === "diff-fold-by-rename"
                   || act === "diff-keep-folded") {
            // "-by-hand" brings the list back and so takes the diff down; "-by-rename" opens a box in the peek, so
            // neither the fold nor the file goes; "-keep-folded" folds first, so closing the diff leaves it folded.
            page.showWip()
            if (act === "diff-keep-folded")
                page.foldByHand(true)
            page.toggleDiff("unstaged", arg, "")
            if (act === "diff-fold-by-hand")
                page.foldByHand(false)
            else if (act === "diff-fold-by-rename") {
                navProbe.peekAt("branch")
                sidebarPane.beginRename("branch", worktree.branch,
                                        worktree.branch)
            } else if (act !== "diff-fold")
                page.closeDiff()
            navRailTimer.start()
        } else if (act === "worktree-menu" || act === "worktree-remove" || act === "worktree-remove-refused"
                   || act === "worktree-graph" || act === "worktree-held") {
            // A worktree's WORKTREE card, raised where a hand raises it: its WORKTREES row's right-click (the row's
            // own press), or the graph row its chip is on (`worktree-graph`, the row's own right-click door). The
            // argument is the worktree's folder, `:go` on the graph one to press. The menu verb stands the card (a
            // greyed row wears its tip); the others press `worktree remove` — landing, or refused by git into the
            // report bar. `worktree-held` holds the in-between open: what the press took off the screen before
            // anything was read again.
            const go = arg.endsWith(":go")
            worktreeMenuTimer.worktree = go ? arg.slice(0, -3) : arg
            worktreeMenuTimer.onGraph = act === "worktree-graph"
            worktreeMenuTimer.press = act === "worktree-remove" || act === "worktree-remove-refused" || go
                                      || act === "worktree-held"
            worktreeMenuTimer.refused = act === "worktree-remove-refused"
            worktreeMenuTimer.holding = act === "worktree-held"
            worktreeMenuTimer.start()
        } else {
            return false
        }
        return true
    }

    // ---- a worktree's card (デザイン規約 §左メニューの所作 の削除の表) ----
    readonly property var worktreesModel: driver.worktreesModel
    readonly property var refMenu: driver.refMenu
    /// The WORKTREES row showing the worktree called `name`, -1 while none does.
    function worktreeRow(name) {
        const list = navProbe.listOf("worktree")
        if (!list)
            return -1
        for (let at = 0; at < list.count; at++) {
            if (worktreesModel.nameAt(at) === name)
                return at
        }
        return -1
    }
    /// Puts a menu a verb just raised where a hand's right-click would have: headless, the pointer never leaves the
    /// screen's origin, and `popup()` opens there (rules-refs/app-ui.md「ヘッドレスの窓には手が乗っている」).
    function standMenuOn(menu, row) {
        const p = row.mapToItem(menu.parent, row.width * 0.35, row.height * 0.6)
        menu.x = p.x
        menu.y = p.y
    }
    // The menu and its card are asked for again each beat until they stand (`raised`, `carded`); the removal is pressed
    // once.
    PageStage {
        id: worktreeMenuTimer
        name: "worktree_menu"
        property string worktree: ""
        property bool onGraph: false
        property bool press: false
        property bool refused: false
        property bool holding: false
        readonly property var menu: worktreeMenuTimer.onGraph ? acts.commitMenu : acts.refMenu
        readonly property var card: worktreeMenuTimer.onGraph ? driver.commitWorktreeCard : driver.refWorktreeCard
        /// Raises the menu, through the row's own door. False while the row is not there yet.
        function raise() {
            const at = acts.worktreeRow(worktreeMenuTimer.worktree)
            if (at < 0)
                return false
            if (!worktreeMenuTimer.onGraph) {
                const list = navProbe.listOf("worktree")
                if (!list.rightClickRow(at))
                    return false
                acts.standMenuOn(acts.refMenu, list.itemAtIndex(at))
                return true
            }
            // The graph row the worktree stands on, pressed where its own right-click goes: the menu aimed at the row's
            // first chip (`GraphRowDelegate.nameChip`).
            const facts = worktreesModel.worktreeFacts(worktreesModel.fullAt(at))
            const row = facts === undefined ? -1 : graphModel.rowOf(facts.head)
            const item = row < 0 ? null : graphPane.view.itemAtIndex(row)
            if (item === null)
                return false
            graphPane.view.rowMenuRequested(item.oid_hex, item.nameChip)
            acts.standMenuOn(acts.commitMenu, item)
            return true
        }
        /// Whether the menu stands on the branch the worktree has out, which brings the BRANCH card.
        function branchCard() {
            return worktreeMenuTimer.onGraph ? driver.commitBranchCard.applies : driver.refBranchCard.applies
        }
        /// The menu up, and not greyed: the row's gate is read as the menu opens (`RefWorktreeMenu.standOn`) —
        /// opened while a write runs, every row greys until it ends (`AppMenu.runningWhy`).
        function menuUp() {
            return { "live": driver.menusLive(), "menu": worktreeMenuTimer.menu.opened }
        }
        /// The menu up and its card open beside it.
        function standing() {
            return { "live": driver.menusLive(), "menu": worktreeMenuTimer.menu.opened,
                     "card": worktreeMenuTimer.card.opened }
        }
        /// Whether the removal row wants its hover: a greyed row says why on its hover alone, and a cut folder comes
        /// back whole on it. A row not on offer has no hover, and its folder no room to be cut in.
        function wantsTip() {
            const item = worktreeMenuTimer.card.removeWorktreeItem
            return item.offered && (item.blocked || item.nameCut)
        }
        function menuLine() {
            const row = worktreeMenuTimer.card.removeWorktreeItem
            return "worktree_menu worktree=" + worktreeMenuTimer.worktree
                   + " card=" + worktreeMenuTimer.card.opened
                   + " offered=" + row.offered
                   + " blocked=" + row.blocked
                   + " tip=" + row.tip.shown
                   + " code=" + row.code
                   + " branch=" + worktreeMenuTimer.branchCard()
                   + " presses=" + row.holdPresses
                   // Whether the folder came out whole: the row bids for it, and a bid a fraction short elides it on
                   // one OS only (`AppMenuItem.implicitWidth`).
                   + " cut=" + row.nameCut
                   // What the hover says. Last, because it is a sentence.
                   + " says=" + (row.tip.shown ? row.tip.words : "")
        }
        onTriggered: {
            const op = worktreeMenuTimer
            const card = op.card
            if (op.at("start")) {
                if (!Awaited.all("worktree_menu_open", { "live": driver.menusLive() }))
                    return
                if (!op.input("raised", () => {
                        if (!op.menu.opened)
                            op.raise()
                        return op.menu.opened
                    }))
                    return
            }
            // A row the card has nothing for (no place for a new worktree, nothing to take away): the menu is the
            // whole answer. The repository's own worktree has a card — the rows that make a worktree — with no
            // `worktree remove`.
            if (!card.applies && !op.press) {
                op.complete({
                    "name": "worktree_menu_open",
                    "when": () => op.menuUp(),
                    "report": () => "worktree_menu worktree=" + op.worktree + " card=false"
                                    + " branch=" + op.branchCard() })
                return
            }
            // The card is one level in (デザイン規約 §メニュー の入れ子); opened so the picture shows its row.
            if (op.at("raised")) {
                if (!Awaited.all("worktree_menu_open", op.menuUp()))
                    return
                if (!op.input("carded", () => {
                        if (!op.card.opened)
                            op.menu.openSub(op.card)
                        return op.card.opened
                    }))
                    return
            }
            const item = card.removeWorktreeItem
            // The menu and its card still up as the next press goes in.
            if (op.at("carded") && !Awaited.all("worktree_menu_card", op.standing()))
                return
            if (!op.press) {
                if (op.at("carded")) {
                    // Whether the folder is cut is the row's layout's to say (`AutoActDriver.cardLaidOut`).
                    if (!Awaited.all("worktree_menu", { "card_laid_out": driver.cardLaidOut(op, card) }))
                        return
                    // The hover forced until it stands beside the row (`AutoActDriver.rowTipStood`).
                    if (!op.input("tipped", () => driver.rowTipStood(op.card.removeWorktreeItem, op.wantsTip())))
                        return
                }
                op.complete({
                    "name": "worktree_menu",
                    "when": () => {
                        const up = op.standing()
                        const row = op.card.removeWorktreeItem
                        return { "live": up.live, "menu": up.menu, "card": up.card,
                                 "tip_beside": !op.wantsTip() || (row.tip.shown && driver.tipAside(Tips.box)) }
                    },
                    "report": () => op.menuLine() })
                return
            }
            // The presses, as a hand makes them: the row's handler folds the menus and asks for the removal. A row out
            // of reach takes none (`AppMenuItem.completeHold`), and the sampler presses again on its next beat. The
            // in-between is held before the press: a demo repository's listing would let it go before a picture could
            // be grabbed (`RepoPage.holdGoneRows`).
            if (!op.input("pressed", () => {
                    if (op.holding)
                        page.holdGoneRows = true
                    return driver.pressWrite("worktree-remove", () => item.completeHold())
                }))
                return
            op.stop()
            // git keeps the worktree and says why: the bar is the answer, and the log stays shut (`write_notice log=`).
            if (op.refused)
                driver.barrierNotice.start()
            else if (op.holding)
                worktreeHeldTimer.start()
            else
                worktreeGoneTimer.start()
        }
    }
    // What the press took off the screen, with git done and the listing that would let it go withheld: the row, and
    // what the worktree marked — its chip on the graph (on no branch), or its branch's frame and the branches list's
    // tree mark. The screen cannot tell a mark the window hides from one the refs no longer carry, so the claim is the
    // gone set and the branch row's own word (`mark=`, empty where `HELD` was).
    PageStage {
        id: worktreeHeldTimer
        name: "worktree_held"
        onTriggered: {
            worktreeHeldTimer.complete({
                "name": "worktree_held_ready",
                "when": () => ({ "ready": driver.wroteAndSettled(), "page": PageSettled.settled(page) }),
                "report": () => {
                    const branch = repoTab.goneWorktreeBranch
                    const list = navProbe.listOf("branch")
                    const at = branch === "" ? -1 : branchesModel.rowOfName(branch)
                    const row = at < 0 || !list ? null : list.itemAtIndex(at)
                    return "worktree_held row=" + acts.worktreeRow(worktreeMenuTimer.worktree)
                           + " chips=" + graphModel.goneChips.join(",")
                           + " branch=" + branch
                           + " mark=" + (row === null ? "" : row.change)
                           + " worktree=" + worktreeMenuTimer.worktree
                } })
        }
    }
    // A removed worktree, judged on the list it leaves: gone at the press and still gone once git's listing is in.
    PageStage {
        id: worktreeGoneTimer
        name: "worktree_gone"
        onTriggered: {
            // The judged two first; the worktree and the count are the preset's.
            worktreeGoneTimer.complete({
                "name": "worktree_gone_ready",
                "when": () => ({ "ready": !(!driver.wroteAndSettled() || repoTab.goneWorktree !== "") }),
                "report": () => "worktree_gone row=" + acts.worktreeRow(worktreeMenuTimer.worktree)
                                + " log=" + page.commandsOpen
                                + " worktree=" + worktreeMenuTimer.worktree
                                + " total=" + worktreesModel.total })
        }
    }
    PageStage {
        id: pinEdgeTimer
        name: "nav_pin_edge"
        /// The row leaving first, then the stand-in: one up because the row left the list some other way is a
        /// different state wearing the same picture.
        function pinned() {
            return { "left": !navProbe.rowInView("branch", branchesModel.headRow),
                     "pin": navProbe.headPinShown && navProbe.headPinAbove }
        }
        onTriggered: {
            const op = pinEdgeTimer
            // No row at all is the other state (`nav-open head:<filter>`).
            if (!Awaited.all("pin_edge_ready", { "ready": !(branchesModel.headRow < 0) }))
                return
            // Re-applied every tick until it pins: a list asked for its end before its layout pass has nowhere to go.
            // A section showing every row keeps answering 0 and the run waits out its watchdog — hence the preset in
            // verbs.md. Where the list came to rest at the last ask (`NavProbe.scrollBranchesToEnd`) is kept.
            if (op.at("start") && !op.input("pinned", () => {
                    const rested = navProbe.scrollBranchesToEnd()
                    const pinned = op.pinned()
                    if (!pinned.left || !pinned.pin)
                        return false
                    op.witness("rested", rested)
                }))
                return
            op.complete({
                "name": "pin_edge_pinned",
                "when": () => op.pinned(),
                "report": () => "nav_pin_edge pin=" + navProbe.headPinShown
                                + " above=" + navProbe.headPinAbove
                                + " rowshown=" + navProbe.rowInView("branch", branchesModel.headRow)
                                + " name=" + branchesModel.headName
                                // Riding above, that is the list's top whatever the pane's height, so the number can
                                // be claimed.
                                + " y=" + Math.round(navProbe.headPinY)
                                // Last: the repository's own numbers, outside the judgement.
                                + " row=" + branchesModel.headRow
                                + " rested=" + Math.round(op.witnessed("rested")) })
        }
    }
    PageStage {
        id: pinSeatTimer
        name: "nav_pin_seat"
        order: ["folded"]
        property int row: 0
        /// The row the hand rests on once the fold has landed, -1 for none (`NavProbe.pointTipAt`).
        property int rest: -1
        /// The row leaving first, then the stand-in (as in `nav-pin-edge`), under the row the fold shut.
        function seated() {
            return branchesModel.headRow < 0 && navProbe.headPinShown && navProbe.headPinUnder === pinSeatTimer.row
        }
        onTriggered: {
            const op = pinSeatTimer
            // BRANCHES arrives on a read of its own, and a click on a still-empty list folds nothing.
            if (op.at("start")) {
                op.input("folded", () => navProbe.clickRow("branch", op.row))
                return
            }
            if (op.rest >= 0 && op.at("folded")) {
                if (!Awaited.all("nav_pin_seat_seated", { "seated": op.seated() }))
                    return
                // Re-applied every beat until the row opens: the delegate arrives a layout after the model's rows, and
                // a miss reads as a row with nothing to open.
                if (!op.input("rested", () => {
                        navProbe.pointTipAt("branch", op.rest)
                        return navProbe.rowFactsOpen
                    }))
                    return
            }
            // The seat standing across a polish pass: the layout pass handing it out has settled.
            const geom = Math.round(navProbe.headPinY) + "+" + Math.round(navProbe.headPinSeatY())
            if (!Awaited.all("nav_pin_seat_still", { "still": op.seated() && op.still("nav_pin_seat_geom", geom) }))
                return
            op.complete({
                "name": "nav_pin_seat",
                "when": () => ({
                    "gone": branchesModel.headRow < 0,
                    "pin": navProbe.headPinShown,
                    "under": navProbe.headPinUnder === op.row,
                    "open": op.rest < 0 || navProbe.rowFactsOpen }),
                "report": () => "nav_pin_seat row=" + op.row
                                + " pin=" + navProbe.headPinShown
                                // The rested row opened — what moved the seat.
                                + " open=" + navProbe.rowFactsOpen
                                // The judged fields are one stretch (`under=` .. `says=`). `sat=` compares the
                                // stand-in's own `y` with the seat row's geometry — one number worked out from both
                                // ends.
                                + " under=" + navProbe.headPinUnder
                                + " sat=" + (Math.round(navProbe.headPinY) === Math.round(navProbe.headPinSeatY()))
                                + " lit=" + navProbe.headPinLit
                                + " depth=" + branchesModel.headDepth
                                + " says=" + branchesModel.headShownName
                                // Unjudged, for a reader.
                                + " at=" + Math.round(navProbe.headPinY)
                                + " gap=" + Math.round(navProbe.headPinSeatY())
                                + " above=" + navProbe.headPinAbove
                                + " name=" + branchesModel.headName })
        }
    }
    PageStage {
        id: peekOpenTimer
        name: "nav_peek_open"
        property int want: 0
        /// The peek's row there, and open.
        function opened() {
            return { "row": navProbe.peekNameAt(peekOpenTimer.want) !== "", "open": navProbe.rowFactsOpen }
        }
        onTriggered: {
            const op = peekOpenTimer
            // Re-applied every beat until the row opens, as the rest in `pinSeatTimer` is.
            if (op.at("start") && !op.input("rested", () => {
                    navProbe.pointPeekTipAt(op.want)
                    const opened = op.opened()
                    return opened.row && opened.open
                }))
                return
            op.complete({
                "name": "nav_peek_open",
                "when": () => op.opened(),
                "report": () => "nav_peek_open row=" + op.want
                                + " name=" + navProbe.peekNameAt(op.want)
                                + " peek=" + navProbe.peekStanding + " open=" + navProbe.rowFactsOpen
                                + " says=" + navProbe.rowFactsWords() })
        }
    }
    // PGG_AUTO_ACT=menu-peek. Each stage waits for the one before to land: the rail and the sections with rows to open
    // (else a cell refuses for want of rows, not for the menu), the section or the menu up, then — for those that walk
    // the hand off — the beat that closes a section left alone, run out.
    PageStage {
        id: menuPeekTimer
        name: "menu_peek"
        property string how: ""
        // `opened` (witnessed at the stage that walks the hand off or raises the menu): the section came out for the
        // rest — what makes its absence at the end a close, not a refusal.
        onTriggered: {
            const op = menuPeekTimer
            if (op.at("start")) {
                if (!Awaited.all("menu_peek_rail", {
                        "folded": page.sidebarCollapsed && sidebarPane.width > 0,
                        "branches": branchesModel.total > 0,
                        "tags": op.how !== "own-swap" || tagsModel.total > 0,
                        "head": graphModel.rowOf(worktree.headOid) >= 0
                    }))
                    return
                if (op.how === "behind") {
                    op.input("menu_raised", () => {
                        page.openRowMenu(worktree.headOid)
                    })
                } else {
                    op.input("peek_asked", () => {
                        navProbe.peekAt("branch")
                    })
                }
                return
            }
            if (op.how === "behind" && op.at("menu_raised")) {
                if (!Awaited.all("menu_peek_menu", { "menu": commitMenu.opened }))
                    return
                op.input("walked", () => {
                    navProbe.peekAt("branch")
                    // Read at the rest, before the walk off: a section that never opened also reads as gone.
                    op.witness("opened", sidebarPane.peekSection.visible)
                    navProbe.peekAway("branch")
                })
                return
            }
            if (op.how === "under" && op.at("peek_asked")) {
                if (!Awaited.all("menu_peek_open", { "peek": navProbe.peekStanding }))
                    return
                op.input("menu_raised", () => {
                    op.witness("opened", navProbe.peekStanding)
                    page.openRowMenu(worktree.headOid)
                })
                return
            }
            if (op.at("peek_asked")) {
                // The current branch's row: the list's first row may be a folder, which raises no menu.
                const row = navProbe.peek.sectionModel
                          ? navProbe.peek.sectionModel.rowOfName(worktree.branch) : -1
                const standing = navProbe.peekStanding
                if (!Awaited.all("menu_peek_open", { "peek": standing, "row": row >= 0 }))
                    return
                op.input("walked", () => {
                    // Not built yet: no press has gone in.
                    if (!navProbe.peek.list.rightClickRow(row))
                        return false
                    navProbe.peekAway("branch")
                    navProbe.peekOut()
                    if (op.how === "own-swap") {
                        // On to the TAGS cell and off it: `opened=` says whether TAGS took the held section's place.
                        navProbe.peekAt("tag")
                        op.witness("opened", sidebarPane.peekKind === "tag")
                        navProbe.peekAway("tag")
                    } else {
                        op.witness("opened", standing)
                    }
                })
                return
            }
            op.complete({
                "name": "menu_peek_after",
                "when": () => ({
                    "menu": page.menuStanding,
                    "beat": op.how === "under" || !navProbe.peek.settling }),
                "report": () => "menu_peek how=" + op.how
                                + " menu=" + page.menuStanding
                                + " opened=" + op.witnessed("opened")
                                + " peek=" + sidebarPane.peekKind
                                + " shown=" + sidebarPane.peekSection.visible
                                + " raised=" + page.menuArea })
        }
    }
    /// Whether this run's subject is the peeked section going away: the three that walk off it or click it shut. Not
    /// `nav-peek-again`'s, whose subject is the section standing again, nor the diff verbs' — "-by-rename" opens one
    /// and holds it open for the box.
    readonly property bool peekGoneWanted: Harness.autoAct === "nav-peek-away" || Harness.autoAct === "nav-peek-out"
                                           || Harness.autoAct === "nav-peek-shut"
    // PGG_AUTO_ACT=nav-peek-again: rest on the cell, click it shut, click it again. Each click waits for the one
    // before to land — the section gone, then standing — and the rail's answer is read as `nav-peek`'s is.
    PageStage {
        id: peekAgainTimer
        name: "peek_again"
        order: ["rested", "shut_tap", "open_tap", "back"]
        property string kind: ""
        onTriggered: {
            const op = peekAgainTimer
            const up = sidebarPane.peekSection.visible
            if (op.at("start")) {
                // Rested on the cell each beat until the section stands: asked again, the seat runs on
                // (`HoverSeat.point`).
                op.input("rested", () => {
                    if (!sidebarPane.peekSection.visible)
                        navProbe.peekAt(op.kind)
                    return sidebarPane.peekSection.visible
                })
                return
            }
            if (op.at("rested")) {
                if (!Awaited.all("peek_again_open", { "up": up }))
                    return
                op.input("shut_tap", () => {
                    navProbe.peekTap(op.kind)
                })
            } else if (op.at("shut_tap")) {
                if (!Awaited.all("peek_again_shut", { "down": !up }))
                    return
                op.input("open_tap", () => {
                    navProbe.peekTap(op.kind)
                })
            } else {
                if (!Awaited.all("peek_again_back", { "up": up }))
                    return
                op.advance("back")
                op.stop()
                navRailTimer.start()
            }
        }
    }
    // Three waits: the splitter handing the pane its width; the diff's read having landed (`DiffPane.diffSettled`),
    // since a pane still reading frames like one that has; and, for `peekGoneWanted`, the section having gone — the
    // popup itself, not `kind`: that clears as the exit begins (`SectionPeekPopup.onAboutToHide`).
    PageStage {
        id: navRailTimer
        name: "nav_rail"
        function line() {
            const counts = navProbe.countsInk()
            // The first three are neighbours so one `must_say` substring judges them (デザイン規約 §左メニューを畳む).
            return "nav_rail collapsed=" + page.sidebarCollapsed
                   + " diff=" + page.diffShown
                   + " editing=" + sidebarPane.editKey
                   + " width=" + Math.round(sidebarPane.width)
                   + " peek=" + sidebarPane.peekKind
                   // `top` equals `cell` (the opening mark's top edge) or the list walked off its cell; `end` <=
                   // `pane` says it grew down and stopped at the pane's foot.
                   + " top=" + Math.round(navProbe.peekY)
                   + " cell=" + Math.round(sidebarPane.peekTop)
                   + " end=" + Math.round(navProbe.peekBottom)
                   + " pane=" + Math.round(sidebarPane.height)
                   // The counts clear of their marks, and where their ink fell in the cells, for a run that came
                   // out false — with the face's own numbers the rail seated them by (`NavProbe.countsInk`), which
                   // a run on a machine with no desk (macOS) cannot be asked for again.
                   + " counts=" + counts.clear
                   + " ink=" + counts.top.toFixed(1) + ":" + counts.bottom.toFixed(1)
                   + " face=" + counts.face
        }
        onTriggered: {
            navRailTimer.complete({
                "name": "nav_rail_ready",
                "when": () => ({ "ready": !(sidebarPane.width <= 0 || sidebarPane.height <= 0),
                                 "page": PageSettled.settled(page),
                                 "gone": !acts.peekGoneWanted || !sidebarPane.peekSection.visible }),
                "report": () => navRailTimer.line() })
        }
    }
    /// PGG_AUTO_ACT=doors-held: the branch rebased onto and right-clicked, the argument. Not the current one —
    /// `switch` is only offered elsewhere, and that row is the point of the hold.
    readonly property string heldBranch: Harness.autoActArg
    /// The replay's rising edge, latched (`RepoPage.autoReplayHeld`): on a demo repository the write starts and ends
    /// inside one sampler beat.
    Connections {
        target: acts.repoTab
        function onReplayingChanged() {
            if (Harness.autoAct === "doors-held" && acts.repoTab.replaying)
                page.autoReplayHeld = true
        }
    }
    // The doors, tried once the replay is under way. Each is read back, since a refused door frames like an untouched
    // pane; the picture is for the rest of the pane staying live. After that the sampler waits for the blocked row's
    // tooltip, the last thing to arrive (`Metrics.tipDelayMs`).
    PageStage {
        id: navHeldTimer
        name: "doors_held"
        order: ["tried"]
        onTriggered: {
            const op = navHeldTimer
            if (op.at("start")) {
                if (!Awaited.all("doors_held_replay", { "replay": page.autoReplayHeld }))
                    return
                // What each door did, read at the press.
                op.input("tried", () => {
                    // ---- the graph's two doors, before the pane's ----------------
                    // Every switch on the graph side ends at `RepoPage.switchToRef`; `false` is a press it turned away.
                    op.witness("road", page.switchToRef("branch", acts.heldBranch) === false)
                    // The row menu keeps its rows, greyed; `drop` is read as the one row that cannot be undone — on
                    // show, as a replay's mid-rewrite HEAD would take it off a menu standing on it
                    // (`RepoPage.treeAtAsk`).
                    page.openRowMenu(worktree.headOid)
                    op.witness("rowmenu", commitMenu.opened)
                    op.witness("drop", dropCommitItem.present && dropCommitItem.blocked)
                    commitMenu.close()
                    // ---- and the pane's ------------------------------------------
                    // The box, asked for where a hand asks. The double-click's refusal shares its gate
                    // (`SidebarRowGestures.held`) and goes unreported: a run cannot tell a refused switch from one not
                    // landed.
                    sidebarPane.beginRename("branch", acts.heldBranch, acts.heldBranch)
                    // The ref menu's rows stay and grey (`AppMenu.heldReason`); the reason is a tooltip, forced since a
                    // pointer cannot be put on a row (verify-ui §hover の絵の撮り方).
                    op.witness("menu", page.openRefMenu("branch", acts.heldBranch, acts.heldBranch,
                                                        branchesModel.oidOfName(acts.heldBranch), true))
                    op.witness("box", sidebarPane.editKey !== "")
                    refSwitchItem.tipForced = true
                })
                return
            }
            op.complete({
                "name": "doors_held_tip",
                "when": () => ({ "tip": refSwitchItem.tip.shown }),
                "report": () => acts.heldLine() })
        }
    }
    /// The doors as the press found them (`navHeldTimer`'s witnesses), beside the pane as it stands.
    function heldLine() {
        // `frozen=`: a run that caught the plan's whole-pane freeze instead would pass for that verb's picture.
        return "doors_held held=" + (page.doorsHeldWhy !== "")
               + " frozen=" + page.sidebarFrozen
               // The graph's side.
               + " road=" + navHeldTimer.witnessed("road")
               + " rowmenu=" + navHeldTimer.witnessed("rowmenu")
               + " drop=" + navHeldTimer.witnessed("drop")
               // The pane's.
               + " box=" + navHeldTimer.witnessed("box")
               + " plus=" + navProbe.headOf("remote").addHeld
               + " menu=" + navHeldTimer.witnessed("menu")
               + " switchrow=" + refSwitchItem.blocked
               + " why=" + (refSwitchItem.blockedWhy !== "")
    }
    // PGG_AUTO_ACT=tags-eye: the eye moves the walk, so the answer is the named tag's commit's row (a tag whose
    // commit a branch also reaches keeps its row, and frames like a switch that did nothing).
    //
    // The graph is emptied before it is rebuilt, so the row is read only after `finishCount` moves. Tags back on
    // lands twice, tag-less first (rules-refs/core.md「2 段ストリーミング」), and the row being back tells the second
    // pass. The press waits for the row too, at the start alone: the page is current after the tag-less pass
    // (rules-refs/app-ui.md「動詞の前提条件は入力を出す枝で読む」).
    PageStage {
        id: tagEyeTimer
        name: "tag_eye"
        /// The named tag's commit, and whether the eye is pressed again to bring the tags back.
        property string oid: ""
        property bool back: false
        /// Whether the graph has finished a pass since the eye was pressed as `stage` — the passes as it was pressed
        /// are witnessed in that press (`GraphModel.finishCount`).
        function passed(stage) {
            return graphModel.finishCount !== tagEyeTimer.witnessed(stage + "_passes")
        }
        /// The row gone after the first press; back, after the second.
        function landed() {
            const there = graphModel.rowOf(tagEyeTimer.oid) >= 0
            return tagEyeTimer.back ? { "passed": tagEyeTimer.passed("shown"), "there": there }
                                    : { "passed": tagEyeTimer.passed("hidden"), "gone": !there }
        }
        onTriggered: {
            const op = tagEyeTimer
            const there = graphModel.rowOf(op.oid) >= 0
            if (op.at("start")) {
                if (!Awaited.all("tag_eye_there", { "there": there }))
                    return
                op.input("hidden", () => {
                    op.witness("hidden_passes", graphModel.finishCount)
                    navProbe.tapTagEye()
                })
                return
            }
            if (op.at("hidden") && op.back) {
                if (!Awaited.all("tag_eye_gone", { "passed": op.passed("hidden"), "gone": !there }))
                    return
                op.input("shown", () => {
                    op.witness("shown_passes", graphModel.finishCount)
                    navProbe.tapTagEye()
                })
                return
            }
            // `shown=` is the switch's own answer: a press that never reached the band photographs a real state.
            op.complete({
                "name": op.back ? "tag_eye_back" : "tag_eye_gone",
                "when": () => op.landed(),
                "report": () => "tags_eye shown=" + repoTab.tagsShown
                                + " there=" + (graphModel.rowOf(op.oid) >= 0)
                                + " tag=" + Harness.autoActArg
                                + " rows=" + graphModel.rowTotal
                                + " tags=" + tagsModel.total })
        }
    }
    // PGG_AUTO_ACT=nav-jump: the claim is read off the graph's row for the commit, lit and laid out
    // (`GraphRowDelegate.selected`) — the page's selection is the click's own bookkeeping.
    PageStage {
        id: jumpTimer
        name: "nav_jump"
        order: ["clicked"]
        /// The argument `<section>[:<row>]`, the row 0 by default.
        readonly property string kind: {
            const arg = Harness.autoActArg
            const cut = arg.indexOf(":")
            return cut < 0 ? arg : arg.substring(0, cut)
        }
        readonly property int row: {
            const arg = Harness.autoActArg
            const cut = arg.indexOf(":")
            return cut < 0 ? 0 : Number(arg.substring(cut + 1))
        }
        /// The landing, on the commit the row named when pressed (`want`, witnessed before the click: a recycled
        /// delegate asked afterwards can show another row's name). A commit the walk never reached is an answer: the
        /// page had settled before the click (`PageSettled`), so the row will not arrive. Not so another worktree's
        /// HEAD: the walk that draws it follows the worktrees' listing, which the settle does not wait for. A row the
        /// model has but the view has not laid out yet is waited for.
        function landed() {
            const want = jumpTimer.witnessed("want")
            const landing = graphModel.rowOf(want)
            return { "list": !!navProbe.listOf(jumpTimer.kind),
                     "selected": page.selectedOid === want,
                     "card": driver.cardSettled,
                     "row": landing >= 0 || jumpTimer.kind !== "worktree",
                     "laid": landing < 0 || !!graphPane.view.itemAtIndex(landing) }
        }
        onTriggered: {
            const op = jumpTimer
            const list = navProbe.listOf(op.kind)
            if (op.at("start")) {
                // A row not laid out yet, or naming no commit, is not a press (`NavList.rowOidAt`).
                if (!Awaited.all("nav_jump_row", { "list": !!list, "row": !!list && list.rowOidAt(op.row) !== "" }))
                    return
                op.input("clicked", () => {
                    const oid = list.rowOidAt(op.row)
                    if (!list.clickRow(op.row))
                        return false
                    op.witness("want", oid)
                })
                return
            }
            op.complete({
                "name": "nav_jump",
                "when": () => op.landed(),
                "report": () => {
                    const want = op.witnessed("want")
                    const at = graphModel.rowOf(want)
                    const row = at >= 0 ? graphPane.view.itemAtIndex(at) : null
                    return "nav_jump section=" + op.kind
                           + " row=" + op.row
                           // `same=` makes `lit=` about this jump.
                           + " same=" + (!!row && row.oid_hex === want)
                           + " lit=" + (!!row && row.selected)
                           // The left panel's half: the clicked row keeps the mark (`NavItemDelegate` の chosenBox).
                           + " marked=" + (sidebarPane.activeKey !== "")
                           // A jump puts the history up whatever was being read (`RepoPage.jumpToRef`).
                           + " wip=" + page.wipShown
                           + " at=" + at
                           + " name=" + navProbe.listOf(op.kind).rowNameAt(op.row)
                } })
        }
    }
    // PGG_AUTO_ACT=nav-reclick / nav-reclick-away: every stage waits for its own answer — the row existing, the first
    // click's double-click window passing (`rowGuarded`), and for "-away" the section having gone.
    PageStage {
        id: reclickTimer
        name: "nav_reclick"
        /// The argument `<section>[:<row>]`, the row 0 by default — a section folding names into folders has no
        /// row to rename at its top.
        readonly property string kind: {
            const arg = Harness.autoActArg
            const cut = arg.indexOf(":")
            return cut < 0 ? arg : arg.substring(0, cut)
        }
        readonly property int row: {
            const arg = Harness.autoActArg
            const cut = arg.indexOf(":")
            return cut < 0 ? 0 : Number(arg.substring(cut + 1))
        }
        /// Whether the pointer leaves the section between the two clicks.
        property bool away: false
        /// The press that peeks the section: a section with no rows yet opens nothing (NavRail.enterAt), so the
        /// arrival is repeated until one stands.
        function peek() {
            if (sidebarPane.peekKind !== "")
                return true
            navProbe.peekAt(reclickTimer.kind)
            return false
        }
        onTriggered: {
            const op = reclickTimer
            const section = sidebarPane.peekSection
            if (op.at("start") && !op.input("peeked", () => op.peek()))
                return
            if (op.at("peeked")) {
                op.input("first_click", () => section.clickRow(op.row))
                return
            }
            if (op.away && op.at("first_click")) {
                // The hand off the section it is on.
                if (!Awaited.all("nav_reclick_away", { "peek": sidebarPane.peekKind !== "" }))
                    return
                op.input("walked_off", () => {
                    navProbe.peekAway(op.kind)
                })
                return
            }
            if (op.at("walked_off")) {
                // The pointer leaving is answered a beat later (SectionPeekPopup.settle).
                if (!Awaited.all("nav_reclick_gone", { "gone": sidebarPane.peekKind === "" }))
                    return
                op.advance("gone")
                return
            }
            // Back on the section the walk took the hand off.
            if (op.at("gone") && !op.input("peeked_again", () => op.peek()))
                return
            if (op.at("first_click") || op.at("peeked_again")) {
                // Inside the double-click window a second click would be half a double-click.
                const peeked = sidebarPane.peekKind !== ""
                const open = peeked && !section.rowGuarded(op.row)
                if (!Awaited.all("nav_reclick_second", { "peeked": peeked, "window": open }))
                    return
                // Whether the second click armed the rename, read at that click: the wait is short and turns into
                // the box.
                op.input("second_click", () => {
                    if (!section.clickRow(op.row))
                        return false
                    op.witness("armed", section.rowArmed(op.row))
                })
                return
            }
            // Armed, the box opens in the peeked section once that wait runs out (SidebarRowGestures.startEdit).
            // The click chose the row's commit too: its read is the page's.
            op.complete({
                "name": "nav_reclick_landed",
                "when": () => ({
                    "box": !op.witnessed("armed") || sidebarPane.editKey !== "",
                    "page": PageSettled.settled(page) }),
                "report": () => "nav_reclick section=" + op.kind
                                + " row=" + op.row
                                + " marked=" + sidebarPane.activeKey
                                + " away=" + op.away
                                + " armed=" + op.witnessed("armed")
                                // The fold stays for a box, the box is there, and it has the keyboard
                                // (デザイン規約 §左メニューを畳む).
                                + " collapsed=" + page.sidebarCollapsed
                                + " box=" + (sidebarPane.editKey !== "")
                                + " focused=" + sidebarPane.peekSection.rowFocused(op.row)
                                + " peek=" + sidebarPane.peekKind
                                + " editing=" + sidebarPane.editKey })
        }
    }
    // PGG_AUTO_ACT=nav-reclick-elsewhere: a click is two beats, as a hand's is — the press (the window's watcher, then
    // the row's own handler) and then the release the row answers at; the gesture settles the press in between. The
    // first click, a press on the details pane, the click back (`pane=`: no box), and the next one (`armed=`, `box=`).
    PageStage {
        id: elsewhereTimer
        name: "nav_reclick_elsewhere"
        order: ["first_press", "first_release", "pane_press", "back_press", "back_release", "next_press",
                "next_release"]
        /// The argument `<section>[:<row>]`, the row 0 by default.
        readonly property string kind: {
            const arg = Harness.autoActArg
            const cut = arg.indexOf(":")
            return cut < 0 ? arg : arg.substring(0, cut)
        }
        readonly property int row: {
            const arg = Harness.autoActArg
            const cut = arg.indexOf(":")
            return cut < 0 ? 0 : Number(arg.substring(cut + 1))
        }
        /// The press half on the row; false while the row is not built.
        function press(list) {
            const at = list.rowPoint(elsewhereTimer.row)
            if (at === null)
                return false
            page.Window.window.caretHand.pressedAt(at)
            return list.pressRow(elsewhereTimer.row)
        }
        onTriggered: {
            const op = elsewhereTimer
            const list = navProbe.listOf(op.kind)
            if (!Awaited.all("elsewhere_ready", { "ready": list }))
                return
            if (op.at("next_release")) {
                // Armed, the box opens on the row once the wait runs out (`SidebarRowGestures.startEdit`); the clicks
                // chose rows, whose reads land on the page.
                op.complete({
                    "name": "nav_reclick_elsewhere_landed",
                    "when": () => ({
                        "box": !op.witnessed("armed") || sidebarPane.editKey !== "",
                        "page": PageSettled.settled(page) }),
                    "report": () => "nav_reclick_elsewhere section=" + op.kind
                                    + " row=" + op.row
                                    + " marked=" + sidebarPane.activeKey
                                    + " pane=" + op.witnessed("pane")
                                    + " armed=" + op.witnessed("armed")
                                    + " box=" + (sidebarPane.editKey !== "")
                                    + " editing=" + sidebarPane.editKey })
                return
            }
            const pressing = op.at("start") ? "first_press"
                           : op.at("pane_press") ? "back_press"
                           : op.at("back_release") ? "next_press" : ""
            if (pressing !== "") {
                // Spaced from the click before: inside its window this would be half a double-click.
                if (!Awaited.all("elsewhere_press/" + pressing, { "window": !list.rowGuarded(op.row) }))
                    return
                op.input(pressing, () => op.press(list))
                return
            }
            if (op.at("first_release")) {
                if (!Awaited.all("elsewhere_window", { "window": !list.rowGuarded(op.row) }))
                    return
                op.input("pane_press", () => {
                    page.Window.window.caretHand.pressedAt(detailsPane.mapToItem(null, detailsPane.width / 2,
                                                                                 detailsPane.height / 2))
                })
                return
            }
            // The release half, once its press is in; false while the row is not built. Whether the click back armed
            // the rename (`pane`), and whether the next one did (`armed`), each read at its release.
            const releasing = op.at("first_press") ? "first_release"
                            : op.at("back_press") ? "back_release" : "next_release"
            op.input(releasing, () => {
                if (!list.releaseRow(op.row))
                    return false
                if (releasing === "back_release")
                    op.witness("pane", list.rowArmed(op.row))
                if (releasing === "next_release")
                    op.witness("armed", list.rowArmed(op.row))
            })
        }
    }
    /// PGG_AUTO_ACT=nav-add-remote: the run stops with the real dialog on screen. `collapsed=` says which door it came
    /// through, and that the folded one left the list folded (デザイン規約 §左メニューを畳む).
    PageStage {
        id: navAddRemoteTimer
        name: "nav_add_remote"
        onTriggered: {
            navAddRemoteTimer.complete({
                "name": "nav_add_remote_ready",
                "when": () => ({ "ready": remoteDialog.visible }),
                "report": () => "nav_add_remote dialog=" + remoteDialog.visible
                                + " collapsed=" + page.sidebarCollapsed
                                + " remotes=" + remotesModel.total
                                + " name=" + remoteDialog.wantedName })
        }
    }
    // The column has to be laid out again before the header that was closed can say where it ended up.
    PageStage {
        id: navSectionTimer
        name: "nav_section"
        onTriggered: {
            navSectionTimer.complete({
                "name": "nav_section_ready",
                "when": () => ({ "ready": !(sidebarPane.height <= 0) }),
                "report": () => "nav_section closed=" + Harness.autoActArg
                                + " header=" + Math.round(navProbe.headerTopOf(Harness.autoActArg))
                                + " ground=" + Math.round(navProbe.groundTop)
                                + " pane=" + Math.round(sidebarPane.height) })
        }
    }
    /// What each section's model kept under the filter; the picture says what was drawn.
    PageStage {
        id: navFilterTimer
        name: "nav_filter"
        onTriggered: {
            navFilterTimer.complete({
                "name": "nav_filter_ready",
                "when": () => ({ "ready": !(sidebarPane.width <= 0) }),
                "report": () => "nav_filter typed=" + Harness.autoActArg
                                + " branches=" + branchesModel.shown() + "/" + branchesModel.total
                                + " remotes=" + remotesModel.shown() + "/" + remotesModel.total
                                + " tags=" + tagsModel.shown() + "/" + tagsModel.total
                                + " stashes=" + stashesModel.shown() + "/" + stashesModel.total })
        }
    }
}
