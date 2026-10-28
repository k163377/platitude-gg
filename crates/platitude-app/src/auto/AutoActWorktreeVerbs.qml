pragma ComponentBehavior: Bound

import QtQuick
// For the attached types alone (rules-refs/app-ui.md carries what an unimported one answers).
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

/// The verbs that make a worktree from a menu (デザイン規約 §worktree を作る): the WORKTREE card's two making rows
/// — `Create worktree here…` and `worktree add` — raised from a left menu row or a graph row with the card opened as a
/// hand opens it, the name box the first one opens, and the presses. A press that lands stands the tab in the new
/// worktree, which is the window's to read (`WindowTabActs`): the page is stood elsewhere by it.
///
/// The argument is `<kind>:<ref>[=<name>][@graph]` — `kind` is `branch` / `remote` / `tag` / `worktree` (by its
/// folder) or `commit` (`row:<n>` or an id, graph only); `name` what is typed into the box; `@graph` raises the graph
/// row's menu instead of the left menu row's. The box can only be typed into on the graph: the left menu's box is
/// filled as it opens, and nothing opens it holding a name.
// An `Item` only because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    /// `var` because naming its type would be a cycle: the driver is the file that builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var repoTab: driver.repoTab
    readonly property var graphModel: driver.graphModel
    readonly property var branchesModel: driver.branchesModel
    readonly property var remotesModel: driver.remotesModel
    readonly property var tagsModel: driver.tagsModel
    readonly property var worktreesModel: driver.worktreesModel
    readonly property var graphPane: driver.graphPane
    readonly property var sidebarPane: driver.sidebarPane
    readonly property var navProbe: driver.navProbe

    /// Runs `act` if it is this family's and says whether it was (`AutoActDriver`).
    function run(act, arg) {
        const modes = {
            "worktree-add-menu": "menu",
            "worktree-add": "add",
            "worktree-box": "box",
            "worktree-new": "new",
            "worktree-new-refused": "refused"
        }
        if (act === "worktree-rename-box" || act === "worktree-rename") {
            renameTimer.begin(arg, act === "worktree-rename")
            return true
        }
        if (modes[act] === undefined)
            return false
        worktreeTimer.begin(arg, modes[act])
        return true
    }

    /// The section model a kind's rows are listed in.
    function modelOf(kind) {
        return kind === "branch" ? acts.branchesModel
             : kind === "remote" ? acts.remotesModel
             : kind === "tag" ? acts.tagsModel : acts.worktreesModel
    }
    /// The row showing `ref` in its section, -1 while none does. A worktree is named by its folder, the others by
    /// their ref.
    function rowOf(kind, ref) {
        const model = acts.modelOf(kind)
        if (kind !== "worktree")
            return model.rowOfName(ref)
        const list = acts.navProbe.listOf("worktree")
        for (let at = 0; list && at < list.count; at++) {
            if (model.nameAt(at) === ref)
                return at
        }
        return -1
    }
    /// The commit `ref` stands on.
    function oidOf(kind, ref) {
        if (kind === "commit")
            return acts.driver.autoActOid(ref)
        if (kind === "worktree") {
            const row = acts.rowOf(kind, ref)
            return row < 0 ? "" : acts.worktreesModel.headOfWorktree(acts.worktreesModel.fullAt(row))
        }
        return acts.modelOf(kind).oidOfName(ref)
    }

    // The menu raised, its WORKTREE card opened, and the mode's one thing done on the card's rows. Only the menu's run
    // has an `order`: the add and the box end at `carded` where their row is not offered. The commit the ref stood on
    // as the menu was raised is a witness (`oid`): the box is keyed on it, and the write sends it.
    PageStage {
        id: worktreeTimer
        name: "worktree_make"
        property string kind: ""
        property string ref: ""
        /// What is typed into the box.
        property string naming: ""
        property bool onGraph: false
        /// `menu` stands the rows; `add` presses `worktree add`; `box` stops at the name box; `new` types a name and
        /// sends it; `refused` makes a branch of that name first, so git turns the worktree down (デザイン規約
        /// §答えの要らない報せ).
        property string mode: ""
        // The menu, its WORKTREE card and the card's two rows are asked for, not held: the census follows what a timer
        // holds by name, and would count a row held here as shown in every run (`WindowCensus.visit`).
        function menu() {
            return worktreeTimer.onGraph ? acts.driver.commitMenu : acts.driver.refMenu
        }
        /// Puts the menu just raised where a hand's right-click would have (as `AutoActNavVerbs.standMenuOn`).
        function standMenuOn(row) {
            const menu = worktreeTimer.menu()
            const p = row.mapToItem(menu.parent, row.width * 0.35, row.height * 0.6)
            menu.x = p.x
            menu.y = p.y
        }
        function card() {
            return worktreeTimer.onGraph ? acts.driver.commitWorktreeCard : acts.driver.refWorktreeCard
        }
        function hereItem() {
            return worktreeTimer.onGraph ? acts.driver.commitWorktreeHereItem : acts.driver.refWorktreeHereItem
        }
        function addItem() {
            return worktreeTimer.onGraph ? acts.driver.commitWorktreeAddItem : acts.driver.refWorktreeAddItem
        }
        /// The word the window's tip stands the tree mark in front of, read off the ask it stands on as the tip reads
        /// it (`TipAsk.markWordNow`, else the folder among the names it seats itself, `marksNow`). Empty while it is
        /// down.
        function tipMark(tip) {
            const ask = tip.visible ? Tips.current : null
            if (ask === null)
                return ""
            if (ask.markWordNow !== "")
                return ask.markWordNow
            // A row that is not turned down names its folder in its whole line, among the line's names (`tipMarks`).
            const folder = ask.marksNow.find(mark => mark[1] === "tree" || mark[1] === "home")
            return folder === undefined ? "" : String(ask.words).substr(folder[0], folder[3])
        }

        function begin(arg, mode) {
            let rest = "" + arg
            worktreeTimer.onGraph = rest.endsWith("@graph")
            if (worktreeTimer.onGraph)
                rest = rest.slice(0, -6)
            const named = rest.indexOf("=")
            worktreeTimer.naming = named < 0 ? "" : rest.substring(named + 1)
            rest = named < 0 ? rest : rest.substring(0, named)
            const cut = rest.indexOf(":")
            worktreeTimer.kind = rest.substring(0, cut)
            worktreeTimer.ref = rest.substring(cut + 1)
            worktreeTimer.onGraph = worktreeTimer.onGraph || worktreeTimer.kind === "commit"
            // A remote's root folder starts closed (`models::nav::tree`), leaving its rows in no list
            // (as `AutoActNavBoxVerbs.navNameBoxTimer`).
            if (worktreeTimer.kind === "remote" && !worktreeTimer.onGraph)
                acts.remotesModel.toggleFolder(GitFacts.remoteOfRef(worktreeTimer.ref, acts.repoTab.remoteNames))
            worktreeTimer.mode = mode
            worktreeTimer.order = mode === "menu" ? ["raised", "carded", "tipped"] : []
            worktreeTimer.start()
        }
        /// Raises the menu through the row's own door, keeping the commit it stands on. False while the row is not
        /// there yet.
        function raise() {
            const oid = acts.oidOf(worktreeTimer.kind, worktreeTimer.ref)
            if (oid === "")
                return false
            if (!worktreeTimer.onGraph) {
                const at = acts.rowOf(worktreeTimer.kind, worktreeTimer.ref)
                const list = acts.navProbe.listOf(worktreeTimer.kind)
                if (at < 0 || !list || !list.rightClickRow(at))
                    return false
                worktreeTimer.standMenuOn(list.itemAtIndex(at))
                worktreeTimer.witness("oid", oid)
                return true
            }
            const row = acts.graphModel.rowOf(oid)
            const item = row < 0 ? null : acts.graphPane.view.itemAtIndex(row)
            if (item === null)
                return false
            // The row's own right-click: aimed at the chip of the name, or at the row's first one for a worktree, or at
            // nothing on a bare commit (`GraphRowDelegate.nameChip`).
            const chip = worktreeTimer.kind === "commit" ? null
                       : worktreeTimer.kind === "worktree" ? item.nameChip
                       : { "kind": worktreeTimer.kind, "name": worktreeTimer.ref }
            acts.graphPane.view.rowMenuRequested(oid, chip)
            worktreeTimer.standMenuOn(item)
            worktreeTimer.witness("oid", oid)
            return true
        }
        /// The rows are one level in (デザイン規約 §メニュー の入れ子): opened as a hand opens it, so the picture shows
        /// them — asked each beat until the card stands.
        function openCard() {
            const card = worktreeTimer.card()
            if (card.applies && !card.opened)
                worktreeTimer.menu().openSub(card)
            return !card.applies || card.opened
        }
        onTriggered: {
            const op = worktreeTimer
            // Asked as the menu opens (`offers::worktree_rows`): opened while a write runs, the rows grey until it ends
            // (`AppMenu.runningWhy`).
            if (!Awaited.all("worktree_menus_live", { "live": acts.driver.menusLive() }))
                return
            if (op.at("start")) {
                op.input("raised", () => op.raise())
                return
            }
            if (op.at("raised")) {
                if (!Awaited.all("worktree_menu_open", { "menu": op.menu().opened }))
                    return
                if (!op.input("carded", () => op.openCard()))
                    return
            }
            if (op.at("carded") || op.at("tipped")) {
                op.act()
                return
            }
            op.awaitBox()
        }
        /// Whether the `worktree add` row's tip stands where a hand's would, for a row that wants one: what
        /// `rowTipStood` answers, read without forcing anything. A greyed row says why on its hover alone, and a cut
        /// folder comes back whole on it.
        function addTipStands() {
            const add = worktreeTimer.addItem()
            const wants = add.offered && (add.blocked || add.nameCut)
            return !wants || (add.tip.shown && acts.driver.tipAside(Tips.box))
        }
        function act() {
            const op = worktreeTimer
            const add = op.addItem()
            const here = op.hereItem()
            if (op.mode === "menu") {
                if (op.at("carded")) {
                    // Whether the folder is cut is the row's layout's to say (`AutoActDriver.cardLaidOut`).
                    const laidOut = acts.driver.cardLaidOut(op, op.card())
                    if (!Awaited.all("worktree_add_menu", { "card_laid_out": laidOut }))
                        return
                    // The tip forced up beside the row answers for itself: false until it stands there.
                    const wants = add.offered && (add.blocked || add.nameCut)
                    if (!op.input("tipped", () => acts.driver.rowTipStood(add, wants)))
                        return
                }
                op.complete({ "name": "worktree_add_menu", "when": () => ({ "tip_beside": op.addTipStands() }),
                              "report": () => op.addMenuLine() })
                return
            }
            if (op.mode === "add") {
                // Nothing to press: the line says why, and the run ends on it rather than at the watchdog.
                if (!add.offered || add.blocked) {
                    op.complete({ "name": "worktree_add_withheld",
                                  "when": () => ({ "withheld": !op.addItem().offered || op.addItem().blocked }),
                                  "report": () => "worktree_add offered=" + op.addItem().offered
                                                  + " blocked=" + op.addItem().blocked })
                    return
                }
                // A click, as a hand makes it: the row's handler, then the menus fold. The landing is the window's.
                op.input("pressed", () => acts.driver.pressWrite("worktree-add", () => {
                    add.triggered()
                    op.menu().dismiss()
                    return true
                }))
                op.stop()
                return
            }
            if (!here.offered) {
                op.complete({ "name": "worktree_box_withheld", "when": () => ({ "withheld": !op.hereItem().offered }),
                              "report": () => "worktree_box offered=false" })
                return
            }
            // The row opens the box where the menu was raised (`RepoPage.startWorktreeAt`).
            op.input("box_asked", () => {
                here.triggered()
                op.menu().dismiss()
            })
        }
        function addMenuLine() {
            const op = worktreeTimer
            const row = op.addItem()
            const tip = Tips.box
            return "worktree_add_menu card=" + op.card().opened
                   + " here=" + op.hereItem().offered
                   + " add=" + row.offered
                   + " folder=" + row.codeName
                   + " blocked=" + row.blocked
                   + " tip=" + row.tip.shown
                   + " aside=" + acts.driver.tipAside(tip)
                   + " mark=" + op.tipMark(tip)
                   + " cut=" + row.nameCut
                   // What the hover says. Last, because it is a sentence.
                   + " says=" + (tip.visible ? tip.text : "")
        }
        /// Whether the box refuses what it holds, and why — on the graph's box, or the left menu's.
        function boxRefused() {
            return worktreeTimer.onGraph ? acts.graphPane.namingRefused : acts.sidebarPane.editRefused
        }
        function boxWhy() {
            return worktreeTimer.onGraph ? acts.page.graphNameRefusedWhy : acts.sidebarPane.editRefusedWhy
        }
        /// The graph row the box opens on, null while it is not laid out.
        function boxItem() {
            const row = acts.graphModel.rowOf(worktreeTimer.witnessed("oid"))
            return row < 0 ? null : acts.graphPane.view.itemAtIndex(row)
        }
        /// Whether the box is up where the run raised it: on the graph's row, or the left menu's.
        function boxUp() {
            if (!worktreeTimer.onGraph)
                return acts.sidebarPane.editKey !== ""
            const item = worktreeTimer.boxItem()
            return acts.graphPane.view.namingOid === worktreeTimer.witnessed("oid") && item !== null && item.naming
        }
        /// Whether the box holds what the run types: only the graph's is typed into.
        function boxTyped() {
            return !worktreeTimer.onGraph || worktreeTimer.naming === ""
                   || acts.graphPane.view.namingText === worktreeTimer.naming
        }
        /// A refused name says why in the box's tip, which comes up after the tip's delay: read off the window's one
        /// tip (`Tips.box`).
        function boxTipStands() {
            const tip = Tips.box
            return !worktreeTimer.boxRefused() || (tip.visible && tip.text === worktreeTimer.boxWhy())
        }
        function boxLine() {
            const op = worktreeTimer
            const shown = Tips.box
            const why = op.boxWhy()
            return "worktree_box where=" + (op.onGraph ? "graph" : "nav")
                   + " open=true"
                   + " mode=" + (op.onGraph ? acts.graphPane.namingMode : "worktree")
                   + " refused=" + op.boxRefused()
                   + " tip=" + (shown.visible && shown.text === why)
                   + " mark=" + op.tipMark(shown)
                   + " why=" + why
        }
        /// The box is up: typed into on the graph, then stood, sent, or sent behind a branch of the same name.
        function awaitBox() {
            const op = worktreeTimer
            if (!Awaited.all("worktree_box_open", { "box": op.boxUp() }))
                return
            // In the way a recycled delegate puts it back (`GraphRowDelegate.takeNamingFocus`), so the box on screen
            // holds what the run says: asked each beat until it does.
            if (op.at("box_asked") && !op.boxTyped() && !op.input("typed", () => {
                const view = acts.graphPane.view
                if (view.namingText !== op.naming) {
                    view.namingText = op.naming
                    op.boxItem().takeNamingFocus()
                }
                return view.namingText === op.naming
            }))
                return
            if (op.mode === "box") {
                // The picture waits for a refusal's tip; `open=true` is the box still up as the line is read.
                op.complete({ "name": "worktree_box_tip",
                              "when": () => ({ "box": op.boxUp(), "typed": op.boxTyped(), "tip": op.boxTipStands() }),
                              "report": () => op.boxLine() })
                return
            }
            if (op.mode === "refused") {
                // Somebody else takes the name first — through the product's own door, ahead in the queue — while the
                // box still holds it unrefused: the refs that would refuse it have not been read again yet.
                op.input("taken", () => {
                    acts.repoTab.createBranch(op.naming, op.witnessed("oid"), false)
                })
            }
            // The box's Enter, past its refusal gate (which has nothing to refuse yet), on the road it takes.
            op.input("sent", () => acts.driver.pressWrite("worktree-new", () => {
                if (op.onGraph)
                    acts.graphPane.view.namingSubmitted(op.witnessed("oid"), op.naming, "worktree")
                else
                    acts.sidebarPane.submitEdit(op.naming)
                return true
            }))
            op.stop()
            // git turns the worktree down: the bar is the answer, the log stays shut (`write_notice log=`).
            if (op.mode === "refused")
                acts.driver.barrierNotice.start()
        }
    }

    /// PGG_AUTO_ACT=worktree-rename-box / worktree-rename: the spaced second click on a worktree — its WORKTREES row,
    /// or with `@graph` the chip on the row it stands on — through the row's own clicks (`NavList.clickRow` /
    /// `GraphRowDelegate.leftClick`), so the row decides what the click renames. The argument is
    /// `<folder>[=<name>][@graph]`: `-box` stops at the box (holding `<name>` where one is given — the left menu's
    /// opened through its door holding it, since nothing types into it) or at the reason no box opened; `worktree-rename`
    /// sends `<name>` and waits for the listing that names the new folder (デザイン規約 §グラフ行のダブルクリック).
    /// `worktree-rename` has no `order`: a rename withheld from the box ends on the reason after the second click.
    /// The worktree as the run found it — its folder's whole path, and the commit it stands on — are witnesses (`full`,
    /// `oid`): the rename takes the folder's row away.
    PageStage {
        id: renameTimer
        name: "worktree_rename"
        property string folder: ""
        /// What is typed into the box.
        property string naming: ""
        property bool typed: false
        property bool onGraph: false
        property bool sends: false
        function begin(arg, sends) {
            let rest = "" + arg
            renameTimer.onGraph = rest.endsWith("@graph")
            if (renameTimer.onGraph)
                rest = rest.slice(0, -6)
            const named = rest.indexOf("=")
            renameTimer.typed = named >= 0
            renameTimer.naming = named < 0 ? "" : rest.substring(named + 1)
            renameTimer.folder = named < 0 ? rest : rest.substring(0, named)
            renameTimer.sends = sends
            // A name typed into the graph's box is its own input, after the second click.
            renameTimer.order = sends ? []
                              : renameTimer.typed && !renameTimer.onGraph ? ["found", "box_asked"]
                              : ["found", "first_click", "second_click"]
                                .concat(renameTimer.typed && renameTimer.onGraph ? ["typed"] : [])
            renameTimer.start()
        }
        /// The graph row the worktree stands on, null while it is not laid out.
        function graphItem() {
            const row = acts.graphModel.rowOf(renameTimer.witnessed("oid"))
            return row < 0 ? null : acts.graphPane.view.itemAtIndex(row)
        }
        /// One click on the row the gesture is made on; false while it is not there.
        function click() {
            if (renameTimer.onGraph) {
                const item = renameTimer.graphItem()
                if (item === null)
                    return false
                item.leftClick(Qt.NoModifier)
                return true
            }
            const list = acts.navProbe.listOf("worktree")
            return !!list && list.clickRow(acts.rowOf("worktree", renameTimer.folder))
        }
        function guarded() {
            if (renameTimer.onGraph) {
                const item = renameTimer.graphItem()
                return item === null || item.clickGuarded
            }
            const list = acts.navProbe.listOf("worktree")
            return !list || list.rowGuarded(acts.rowOf("worktree", renameTimer.folder))
        }
        onTriggered: {
            const op = renameTimer
            if (op.at("start")) {
                const at = acts.rowOf("worktree", op.folder)
                if (!Awaited.all("worktree_rename_row", { "row": at >= 0 }))
                    return
                const full = op.witness("full", acts.worktreesModel.fullAt(at))
                op.witness("oid", acts.worktreesModel.headOfWorktree(full))
                op.advance("found")
            }
            if (op.at("found")) {
                // The left menu's box holds a name only through its door (`SidebarPane.beginRename`).
                if (op.typed && !op.onGraph && !op.sends) {
                    op.input("box_asked", () => {
                        acts.sidebarPane.beginRename("worktreeFolder", op.witnessed("full"), op.naming)
                    })
                    return
                }
                op.input("first_click", () => op.click())
                return
            }
            if (op.at("first_click")) {
                // Inside the double-click window a second click would be half a double-click.
                if (!Awaited.all("worktree_rename_second", { "window": !op.guarded() }))
                    return
                op.input("second_click", () => op.click())
                return
            }
            if (op.at("sent")) {
                op.awaitListing()
                return
            }
            op.awaitBox()
        }
        /// Where the box stands: open (on the graph's row, or the left menu's), or withheld with a reason.
        function boxOpen() {
            const view = acts.graphPane.view
            const item = renameTimer.onGraph ? renameTimer.graphItem() : null
            const list = acts.navProbe.listOf("worktree")
            const at = acts.rowOf("worktree", renameTimer.folder)
            return renameTimer.onGraph ? view.namingOid === renameTimer.witnessed("oid") && item !== null && item.naming
                                       : acts.sidebarPane.editKey !== "" && at >= 0 && list.rowBoxWidth(at) > 0
        }
        function withheld() {
            return renameTimer.onGraph ? acts.graphPane.renameWithheld : acts.sidebarPane.renameWithheld
        }
        function boxRefused() {
            return renameTimer.boxOpen()
                   && (renameTimer.onGraph ? acts.graphPane.namingRefused : acts.sidebarPane.editRefused)
        }
        function boxWhy() {
            return renameTimer.withheld()
                   ? (renameTimer.onGraph ? acts.graphPane.view.withheldWhy : acts.sidebarPane.withheldWhy)
                   : renameTimer.onGraph ? acts.page.graphNameRefusedWhy : acts.sidebarPane.editRefusedWhy
        }
        /// A reason comes up after the tip's delay, on the window's one tip.
        function tipStands() {
            const tip = Tips.box
            return (tip.visible && tip.text === renameTimer.boxWhy())
                   || !(renameTimer.boxRefused() || renameTimer.withheld())
        }
        function boxLine() {
            const op = renameTimer
            const view = acts.graphPane.view
            const shown = op.boxOpen()
            const why = op.boxWhy()
            const tip = Tips.box
            const list = acts.navProbe.listOf("worktree")
            const at = acts.rowOf("worktree", op.folder)
            const kind = !shown ? "" : op.onGraph ? view.namingKind : acts.sidebarPane.editKind
            const placeholder = !shown ? "" : op.onGraph ? op.graphItem().nameBoxPlaceholder
                                                         : list.rowBoxPlaceholder(at)
            const text = !shown ? "" : op.onGraph ? view.namingText : acts.sidebarPane.editText
            return "worktree_rename_box where=" + (op.onGraph ? "graph" : "nav")
                   + " open=" + shown
                   + " withheld=" + op.withheld()
                   + " kind=" + kind
                   + " placeholder=" + placeholder
                   + " text=" + text
                   + " refused=" + op.boxRefused()
                   + " tip=" + (tip.visible && tip.text === why)
                   + " mark=" + worktreeTimer.tipMark(tip)
                   // Last, because it is a sentence.
                   + " why=" + why
        }
        function awaitBox() {
            const op = renameTimer
            const view = acts.graphPane.view
            const open = op.boxOpen()
            if (!Awaited.all("worktree_rename_box", { "box_or_reason": open || op.withheld() }))
                return
            // The name typed into the graph's box in the way a recycled delegate puts it back
            // (`GraphRowDelegate.takeNamingFocus`), asked each beat until the box holds it; nothing to type where no
            // box opened.
            if (op.onGraph && op.typed && op.at("second_click") && !op.input("typed", () => {
                if (op.boxOpen() && view.namingText !== op.naming) {
                    view.namingText = op.naming
                    op.graphItem().takeNamingFocus()
                }
                return !op.boxOpen() || view.namingText === op.naming
            }))
                return
            if (open && op.sends) {
                op.input("sent", () => acts.driver.pressWrite("worktree-rename", () => {
                    if (op.onGraph)
                        view.namingSubmitted(op.witnessed("oid"), op.naming, "rename")
                    else
                        acts.sidebarPane.submitEdit(op.naming)
                    return true
                }))
                return
            }
            op.complete({ "name": "worktree_rename_tip", "when": () => ({ "tip": op.tipStands() }),
                          "report": () => op.boxLine() })
        }
        /// The listing as it stands: whether the box is down, the new folder listed and the old one gone.
        function listing() {
            return {
                "box": renameTimer.onGraph ? acts.graphPane.view.namingOid !== "" : acts.sidebarPane.editKey !== "",
                "listed": acts.rowOf("worktree", renameTimer.naming) >= 0,
                "gone": acts.rowOf("worktree", renameTimer.folder) < 0
            }
        }
        /// The new folder listed, the old one gone, the box down, nothing running.
        function awaitListing() {
            const op = renameTimer
            // Settled too: the new listing sets the other worktrees' reads going, and a picture taken under them is of
            // a page still arriving.
            op.complete({
                "name": "worktree_renamed",
                "when": () => {
                    const now = op.listing()
                    return { "idle": acts.repoTab.busyCount === 0, "box_down": !now.box, "listed": now.listed,
                             "gone": now.gone, "settled": PageSettled.settled(acts.page) }
                },
                "report": () => {
                    const landed = op.listing()
                    return "worktree_renamed where=" + (op.onGraph ? "graph" : "nav")
                           + " from=" + op.folder
                           + " to=" + op.naming
                           + " listed=" + landed.listed
                           + " gone=" + landed.gone
                           + " box=" + landed.box
                } })
        }
    }
}
