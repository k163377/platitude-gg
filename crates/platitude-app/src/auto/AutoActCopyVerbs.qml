pragma ComponentBehavior: Bound

import QtQuick
// For the attached types alone (rules-refs/app-ui.md carries what an unimported one answers).
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

/// The verbs that make a working copy from a menu (デザイン規約 §作業コピーを作る): the WORKTREE card's two making rows
/// — `Create worktree here…` and `worktree add` — raised from a left menu row or a graph row with the card opened as a
/// hand opens it, the name box the first one opens, and the presses. A press that lands stands the tab in the new
/// copy, which is the window's to read (`WindowTabActs`): the page is stood elsewhere by it.
///
/// The argument is `<kind>:<ref>[=<name>][@graph]` — `kind` is `branch` / `remote` / `tag` / `worktree` (a copy, by its
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
        if (modes[act] === undefined)
            return false
        copyTimer.begin(arg, modes[act])
        return true
    }

    /// The section model a kind's rows are listed in.
    function modelOf(kind) {
        return kind === "branch" ? acts.branchesModel
             : kind === "remote" ? acts.remotesModel
             : kind === "tag" ? acts.tagsModel : acts.worktreesModel
    }
    /// The row showing `ref` in its section, -1 while none does. A copy is named by its folder, the others by their
    /// ref.
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
            return row < 0 ? "" : acts.worktreesModel.headOfCopy(acts.worktreesModel.fullAt(row))
        }
        return acts.modelOf(kind).oidOfName(ref)
    }
    /// Puts a menu a verb just raised where a hand's right-click would have (as `AutoActNavVerbs.standMenuOn`).
    function standMenuOn(menu, row) {
        const p = row.mapToItem(menu.parent, row.width * 0.35, row.height * 0.6)
        menu.x = p.x
        menu.y = p.y
    }

    SampleTimer {
        id: copyTimer
        property string kind: ""
        property string ref: ""
        property string name: ""
        property bool onGraph: false
        /// `menu` stands the rows; `add` presses `worktree add`; `box` stops at the name box; `new` types a name and
        /// sends it; `refused` makes a branch of that name first, so git turns the copy down (デザイン規約
        /// §答えの要らない報せ).
        property string mode: ""
        property string oid: ""
        /// 0 = raise the menu, 1 = open its WORKTREE card and act on it, 2 = wait for the box.
        property int step: 0
        // The menu, its WORKTREE card and the card's two rows are asked for, not held: the census follows what a timer
        // holds by name, and would count a row held here as shown in every run (`WindowCensus.visit`).
        function menu() {
            return copyTimer.onGraph ? acts.driver.commitMenu : acts.driver.refMenu
        }
        function card() {
            return copyTimer.onGraph ? acts.driver.commitCopyCard : acts.driver.refCopyCard
        }
        function hereItem() {
            return copyTimer.onGraph ? acts.driver.commitCopyHereItem : acts.driver.refCopyHereItem
        }
        function addItem() {
            return copyTimer.onGraph ? acts.driver.commitCopyAddItem : acts.driver.refCopyAddItem
        }
        /// The word the shared tip stands the tree mark in front of, read off the target it stands on as the tip
        /// reads it (`SharedToolTip.tipMarkWord`). Empty while it is down.
        function tipMark(tip) {
            const at = tip.visible ? tip.parent : null
            return at !== null && at.tipMarkWord !== undefined ? at.tipMarkWord : ""
        }

        function begin(arg, mode) {
            let rest = "" + arg
            copyTimer.onGraph = rest.endsWith("@graph")
            if (copyTimer.onGraph)
                rest = rest.slice(0, -6)
            const named = rest.indexOf("=")
            copyTimer.name = named < 0 ? "" : rest.substring(named + 1)
            rest = named < 0 ? rest : rest.substring(0, named)
            const cut = rest.indexOf(":")
            copyTimer.kind = rest.substring(0, cut)
            copyTimer.ref = rest.substring(cut + 1)
            copyTimer.onGraph = copyTimer.onGraph || copyTimer.kind === "commit"
            // A remote's root folder starts closed (`models::nav::tree`), leaving its rows in no list
            // (as `AutoActNavBoxVerbs.navNameBoxTimer`).
            if (copyTimer.kind === "remote" && !copyTimer.onGraph)
                acts.remotesModel.toggleFolder(GitFacts.remoteOfRef(copyTimer.ref, acts.repoTab.remoteNames))
            copyTimer.mode = mode
            copyTimer.step = 0
            copyTimer.start()
        }
        /// Raises the menu through the row's own door. False while the row is not there yet.
        function raise() {
            copyTimer.oid = acts.oidOf(copyTimer.kind, copyTimer.ref)
            if (copyTimer.oid === "")
                return false
            if (!copyTimer.onGraph) {
                const at = acts.rowOf(copyTimer.kind, copyTimer.ref)
                const list = acts.navProbe.listOf(copyTimer.kind)
                if (at < 0 || !list || !list.rightClickRow(at))
                    return false
                acts.standMenuOn(copyTimer.menu(), list.itemAtIndex(at))
                return true
            }
            const row = acts.graphModel.rowOf(copyTimer.oid)
            const item = row < 0 ? null : acts.graphPane.view.itemAtIndex(row)
            if (item === null)
                return false
            // The row's own right-click: aimed at the chip of the name, or at the row's first one for a copy, or at
            // nothing on a bare commit (`GraphRowDelegate.menuChip`).
            const chip = copyTimer.kind === "commit" ? null
                       : copyTimer.kind === "worktree" ? item.menuChip
                       : { "kind": copyTimer.kind, "name": copyTimer.ref }
            acts.graphPane.view.rowMenuRequested(copyTimer.oid, chip)
            acts.standMenuOn(copyTimer.menu(), item)
            return true
        }
        function say(line) {
            copyTimer.stop()
            Harness.report(line)
        }
        onTriggered: {
            // Asked as the menu opens (`offers::copy_rows`): opened while a write runs, the rows are not there.
            if (acts.repoTab.busyCount !== 0)
                return
            if (copyTimer.step === 0) {
                if (copyTimer.raise())
                    copyTimer.step = 1
                return
            }
            if (copyTimer.step === 1) {
                if (!copyTimer.menu().opened)
                    return
                // The rows are one level in (デザイン規約 §メニュー の入れ子): opened as a hand opens it, so the
                // picture shows them.
                const card = copyTimer.card()
                if (card.applies && !card.opened) {
                    copyTimer.menu().openSub(card)
                    return
                }
                copyTimer.act()
                return
            }
            copyTimer.awaitBox()
        }
        function act() {
            const add = copyTimer.addItem()
            const here = copyTimer.hereItem()
            if (copyTimer.mode === "menu") {
                // A greyed row says why on its hover alone, and a cut folder comes back whole on it.
                const tip = add.ToolTip.toolTip
                const wants = add.offered && (add.blocked || add.nameCut)
                if (!Awaited.all("copy_menu", { "tip_beside": acts.driver.rowTipStood(add, wants) }))
                    return
                copyTimer.say("copy_menu card=" + copyTimer.card().opened
                              + " here=" + here.offered
                              + " add=" + add.offered
                              + " folder=" + add.markName
                              + " blocked=" + add.blocked
                              + " tip=" + add.ToolTip.visible
                              + " aside=" + acts.driver.tipAside(tip)
                              + " mark=" + copyTimer.tipMark(tip)
                              + " cut=" + add.nameCut
                              // What the hover says. Last, because it is a sentence.
                              + " says=" + (tip.visible ? tip.text : ""))
                acts.driver.complete()
                return
            }
            if (copyTimer.mode === "add") {
                // Nothing to press: the line says why, and the run ends on it rather than at the watchdog.
                if (!add.offered || add.blocked) {
                    copyTimer.say("copy_add offered=" + add.offered + " blocked=" + add.blocked)
                    acts.driver.complete()
                    return
                }
                copyTimer.stop()
                // A click, as a hand makes it: the row's handler, then the menus fold. The landing is the window's.
                acts.driver.pressWrite("worktree-add", () => {
                    add.triggered()
                    copyTimer.menu().dismiss()
                    return true
                })
                return
            }
            if (!here.offered) {
                copyTimer.say("copy_box offered=false")
                acts.driver.complete()
                return
            }
            // The row opens the box where the menu was raised (`RepoPage.startCopyAt`).
            here.triggered()
            copyTimer.menu().dismiss()
            copyTimer.step = 2
        }
        /// The box is up: typed into on the graph, then stood, sent, or sent behind a branch of the same name.
        function awaitBox() {
            if (copyTimer.onGraph) {
                const view = acts.graphPane.view
                const row = acts.graphModel.rowOf(copyTimer.oid)
                const item = row < 0 ? null : view.itemAtIndex(row)
                if (view.namingOid !== copyTimer.oid || item === null || !item.naming)
                    return
                if (copyTimer.name !== "" && view.namingText !== copyTimer.name) {
                    // In the way a recycled delegate puts it back (`GraphRowDelegate.takeNamingFocus`), so the box on
                    // screen holds what the run says.
                    view.namingText = copyTimer.name
                    item.takeNamingFocus()
                    return
                }
            } else if (acts.sidebarPane.editKey === "") {
                return
            }
            if (copyTimer.mode === "box") {
                const refused = copyTimer.onGraph ? acts.graphPane.namingRefused : acts.sidebarPane.editRefused
                const why = copyTimer.onGraph ? acts.page.graphNameRefusedWhy : acts.sidebarPane.editRefusedWhy
                // A refused name says why in the box's tip, which comes up after the tip's delay: the picture waits for
                // it, read off the window's one tip (the box's own attached `ToolTip`).
                const tip = acts.page.ToolTip.toolTip
                const tipped = tip.visible && tip.text === why
                if (refused && !tipped)
                    return
                copyTimer.say("copy_box where=" + (copyTimer.onGraph ? "graph" : "nav")
                              + " open=true"
                              + " mode=" + (copyTimer.onGraph ? acts.graphPane.namingMode : "worktree")
                              + " refused=" + refused
                              + " tip=" + tipped
                              + " mark=" + copyTimer.tipMark(tip)
                              + " why=" + why)
                acts.driver.barrierRendered.begin()
                return
            }
            copyTimer.stop()
            if (copyTimer.mode === "refused")
                // Somebody else takes the name first — through the product's own door, ahead in the queue — while the
                // box still holds it unrefused: the refs that would refuse it have not been read again yet.
                acts.repoTab.createBranch(copyTimer.name, copyTimer.oid, false)
            // The box's Enter, past its refusal gate (which has nothing to refuse yet), on the road it takes.
            acts.driver.pressWrite("worktree-new", () => {
                if (copyTimer.onGraph)
                    acts.graphPane.view.namingSubmitted(copyTimer.oid, copyTimer.name, "worktree")
                else
                    acts.sidebarPane.submitEdit(copyTimer.name)
                return true
            })
            // git turns the copy down: the bar is the answer, the log stays shut (`write_notice log=`).
            if (copyTimer.mode === "refused")
                acts.driver.barrierNotice.start()
        }
    }
}
