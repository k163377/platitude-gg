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
    /// Puts a menu a verb just raised where a hand's right-click would have (as `AutoActNavVerbs.standMenuOn`).
    function standMenuOn(menu, row) {
        const p = row.mapToItem(menu.parent, row.width * 0.35, row.height * 0.6)
        menu.x = p.x
        menu.y = p.y
    }

    // The menu raised, its WORKTREE card opened, and the mode's one thing done on the card's rows. Only the menu's run
    // has an `order`: the add and the box end at `raised` where their row is not offered.
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
        property string oid: ""
        // The menu, its WORKTREE card and the card's two rows are asked for, not held: the census follows what a timer
        // holds by name, and would count a row held here as shown in every run (`WindowCensus.visit`).
        function menu() {
            return worktreeTimer.onGraph ? acts.driver.commitMenu : acts.driver.refMenu
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
            worktreeTimer.order = mode === "menu" ? ["raised"] : []
            worktreeTimer.start()
        }
        /// Raises the menu through the row's own door. False while the row is not there yet.
        function raise() {
            worktreeTimer.oid = acts.oidOf(worktreeTimer.kind, worktreeTimer.ref)
            if (worktreeTimer.oid === "")
                return false
            if (!worktreeTimer.onGraph) {
                const at = acts.rowOf(worktreeTimer.kind, worktreeTimer.ref)
                const list = acts.navProbe.listOf(worktreeTimer.kind)
                if (at < 0 || !list || !list.rightClickRow(at))
                    return false
                acts.standMenuOn(worktreeTimer.menu(), list.itemAtIndex(at))
                return true
            }
            const row = acts.graphModel.rowOf(worktreeTimer.oid)
            const item = row < 0 ? null : acts.graphPane.view.itemAtIndex(row)
            if (item === null)
                return false
            // The row's own right-click: aimed at the chip of the name, or at the row's first one for a worktree, or at
            // nothing on a bare commit (`GraphRowDelegate.nameChip`).
            const chip = worktreeTimer.kind === "commit" ? null
                       : worktreeTimer.kind === "worktree" ? item.nameChip
                       : { "kind": worktreeTimer.kind, "name": worktreeTimer.ref }
            acts.graphPane.view.rowMenuRequested(worktreeTimer.oid, chip)
            acts.standMenuOn(worktreeTimer.menu(), item)
            return true
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
                // The rows are one level in (デザイン規約 §メニュー の入れ子): opened as a hand opens it, so the
                // picture shows them.
                const card = op.card()
                if (!Awaited.all("worktree_card_open", { "card": !card.applies || card.opened })) {
                    op.menu().openSub(card)
                    return
                }
                op.act()
                return
            }
            op.awaitBox()
        }
        function act() {
            const op = worktreeTimer
            const add = op.addItem()
            const here = op.hereItem()
            if (op.mode === "menu") {
                // Whether the folder is cut is the row's layout's to say (`AutoActDriver.cardLaidOut`).
                const laidOut = acts.driver.cardLaidOut(op, op.card())
                if (!Awaited.all("worktree_add_menu", { "card_laid_out": laidOut }))
                    return
                // A greyed row says why on its hover alone, and a cut folder comes back whole on it.
                const wants = add.offered && (add.blocked || add.nameCut)
                if (!Awaited.all("worktree_add_menu", { "tip_beside": acts.driver.rowTipStood(add, wants) }))
                    return
                op.complete(() => {
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
                })
                return
            }
            if (op.mode === "add") {
                // Nothing to press: the line says why, and the run ends on it rather than at the watchdog.
                if (!add.offered || add.blocked) {
                    op.complete(() => "worktree_add offered=" + op.addItem().offered
                                      + " blocked=" + op.addItem().blocked)
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
                op.complete(() => "worktree_box offered=false")
                return
            }
            // The row opens the box where the menu was raised (`RepoPage.startWorktreeAt`).
            op.input("box_asked", () => {
                here.triggered()
                op.menu().dismiss()
            })
        }
        /// Whether the box refuses what it holds, and why — on the graph's box, or the left menu's.
        function boxRefused() {
            return worktreeTimer.onGraph ? acts.graphPane.namingRefused : acts.sidebarPane.editRefused
        }
        function boxWhy() {
            return worktreeTimer.onGraph ? acts.page.graphNameRefusedWhy : acts.sidebarPane.editRefusedWhy
        }
        /// The box is up: typed into on the graph, then stood, sent, or sent behind a branch of the same name.
        function awaitBox() {
            const op = worktreeTimer
            if (op.onGraph) {
                const view = acts.graphPane.view
                const row = acts.graphModel.rowOf(op.oid)
                const item = row < 0 ? null : view.itemAtIndex(row)
                if (!Awaited.all("worktree_box_open", {
                        "box": view.namingOid === op.oid && item !== null && item.naming }))
                    return
                if (!Awaited.all("worktree_box_typed", {
                        "typed": op.naming === "" || view.namingText === op.naming })) {
                    // In the way a recycled delegate puts it back (`GraphRowDelegate.takeNamingFocus`), so the box on
                    // screen holds what the run says.
                    view.namingText = op.naming
                    item.takeNamingFocus()
                    return
                }
            } else if (!Awaited.all("worktree_box_open", { "box": acts.sidebarPane.editKey !== "" })) {
                return
            }
            if (op.mode === "box") {
                // A refused name says why in the box's tip, which comes up after the tip's delay: the picture waits for
                // it, read off the window's one tip (`Tips.box`).
                const tip = Tips.box
                if (!Awaited.all("worktree_box_tip", {
                        "tip": !op.boxRefused() || (tip.visible && tip.text === op.boxWhy()) }))
                    return
                op.complete(() => {
                    const shown = Tips.box
                    const why = op.boxWhy()
                    return "worktree_box where=" + (op.onGraph ? "graph" : "nav")
                           + " open=true"
                           + " mode=" + (op.onGraph ? acts.graphPane.namingMode : "worktree")
                           + " refused=" + op.boxRefused()
                           + " tip=" + (shown.visible && shown.text === why)
                           + " mark=" + op.tipMark(shown)
                           + " why=" + why
                })
                return
            }
            if (op.mode === "refused") {
                // Somebody else takes the name first — through the product's own door, ahead in the queue — while the
                // box still holds it unrefused: the refs that would refuse it have not been read again yet.
                op.input("taken", () => {
                    acts.repoTab.createBranch(op.naming, op.oid, false)
                })
            }
            // The box's Enter, past its refusal gate (which has nothing to refuse yet), on the road it takes.
            op.input("sent", () => acts.driver.pressWrite("worktree-new", () => {
                if (op.onGraph)
                    acts.graphPane.view.namingSubmitted(op.oid, op.naming, "worktree")
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
    PageStage {
        id: renameTimer
        name: "worktree_rename"
        property string folder: ""
        /// What is typed into the box.
        property string naming: ""
        property bool typed: false
        property bool onGraph: false
        property bool sends: false
        property string full: ""
        property string oid: ""
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
            renameTimer.order = sends ? []
                              : renameTimer.typed && !renameTimer.onGraph ? ["box_asked"]
                              : ["first_click", "second_click"]
            renameTimer.start()
        }
        /// The graph row the worktree stands on, null while it is not laid out.
        function graphItem() {
            const row = acts.graphModel.rowOf(renameTimer.oid)
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
                op.full = acts.worktreesModel.fullAt(at)
                op.oid = acts.worktreesModel.headOfWorktree(op.full)
                // The left menu's box holds a name only through its door (`SidebarPane.beginRename`).
                if (op.typed && !op.onGraph && !op.sends) {
                    op.input("box_asked", () => {
                        acts.sidebarPane.beginRename("worktreeFolder", op.full, op.naming)
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
            return renameTimer.onGraph ? view.namingOid === renameTimer.oid && item !== null && item.naming
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
        function awaitBox() {
            const op = renameTimer
            const view = acts.graphPane.view
            const open = op.boxOpen()
            const withheld = op.withheld()
            if (!Awaited.all("worktree_rename_box", { "box_or_reason": open || withheld }))
                return
            if (!Awaited.all("worktree_rename_typed", {
                    "typed": !(open && op.onGraph && op.typed) || view.namingText === op.naming })) {
                // In the way a recycled delegate puts it back (`GraphRowDelegate.takeNamingFocus`).
                view.namingText = op.naming
                op.graphItem().takeNamingFocus()
                return
            }
            if (open && op.sends) {
                op.input("sent", () => acts.driver.pressWrite("worktree-rename", () => {
                    if (op.onGraph)
                        view.namingSubmitted(op.oid, op.naming, "rename")
                    else
                        acts.sidebarPane.submitEdit(op.naming)
                    return true
                }))
                return
            }
            // A reason comes up after the tip's delay, on the window's one tip.
            const tip = Tips.box
            const tipped = tip.visible && tip.text === op.boxWhy()
            if (!Awaited.all("worktree_rename_tip", { "tip": tipped || !(op.boxRefused() || withheld) }))
                return
            op.complete(() => {
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
            })
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
            const now = op.listing()
            // Settled too: the new listing sets the other worktrees' reads going, and a picture taken under them is of
            // a page still arriving.
            if (!Awaited.all("worktree_renamed", { "idle": acts.repoTab.busyCount === 0, "box_down": !now.box,
                                                   "listed": now.listed, "gone": now.gone,
                                                   "settled": PageSettled.settled(acts.page) }))
                return
            op.complete(() => {
                const landed = op.listing()
                return "worktree_renamed where=" + (op.onGraph ? "graph" : "nav")
                       + " from=" + op.folder
                       + " to=" + op.naming
                       + " listed=" + landed.listed
                       + " gone=" + landed.gone
                       + " box=" + landed.box
            })
        }
    }
}
