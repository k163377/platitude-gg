pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The graph's walk gone silent behind a write: what the window shows while git has answered a commit and the walk
/// its reading asked for says nothing. The fixture is the git the run stands on, which the verb brings itself
/// (`xtask::verify::run`'s `silent_walk_for`, a `--slow-git walk:<ms>:head=silent`): the opening's walks land, and
/// every one behind the commit this verb makes, whose subject says `silent`, is held (`xtask::verify::git_shim`) — the
/// order a repository whose storage stops answering in the middle of a walk leaves. `--preset dirty` has something
/// staged to commit.
// `Item` because `QtObject` has no default property to hold the sampler below.
Item {
    id: acts

    /// `var`: typing it `AutoActDriver` would be circular — that file builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var repoTab: driver.repoTab
    readonly property var worktree: driver.worktree
    readonly property var graphModel: driver.graphModel
    readonly property var graphPane: driver.graphPane
    readonly property var wipPane: driver.wipPane
    readonly property var commitMenu: driver.commitMenu

    /// Runs `act` if it is this family's, and says whether it was (`AutoActDriver` asks each family in turn).
    function run(act, arg) {
        if (act !== "walk-silent")
            return false
        silentTimer.landsToo = arg === "landed"
        silentTimer.waitsOut = arg === "after" || silentTimer.landsToo
        silentTimer.order = silentTimer.landsToo ? ["committed", "silent", "raised"] : ["committed", "raised"]
        silentTimer.start()
        return true
    }

    // `walk-silent`: the commit pressed, git's answer in, and the walk behind it held; then the graph row's menu raised
    // and the command log put up, as a reader who finds the window waiting would. `held=` is how many of the menu's
    // rows grey, `running=` whether the log has a row still running — the command the rows say to wait for.
    // `walk-silent after`: the same, raised once the walk has been silent for its budget
    // (`RepoSession::walk_went_silent`) — the reading given up on, the walk running on: `silent=true` and the band's
    // `STALE GRAPH` up. The budget is the product's own, so the run spends it in real time.
    // `walk-silent landed`, over a shim that answers after the budget (`walk:45000:head=silent`): the same walk once
    // git has answered after all — it lands with no ask behind it, the commit's row drawn and the band's mark down.
    PageStage {
        id: silentTimer
        name: "walk_silent"
        order: ["committed", "raised"]
        /// Whether the menu goes up once the walk is told silent (`after`), or while the reading still waits on it.
        /// Not `after`: that name is the stage timer's own (`StageTimer.after`).
        property bool waitsOut: false
        /// Whether it then waits for the silent walk to land (`landed`).
        property bool landsToo: false
        /// The menu and the log put up, as one input.
        function raiseAll() {
            // The reading never comes in while the walk is held: the run ends under it, said out loud.
            driver.letWriteGo()
            page.raiseCommands()
            return silentTimer.raise()
        }
        /// The row's own right-click (`GraphRowDelegate.askMenu`) on the row under HEAD's, as `commit-menu` takes.
        /// False while the row has no delegate to answer.
        function raise() {
            // A graph left behind has no row for the commit just made: then the newest drawn, under the worktree's.
            const head = graphModel.rowOf(worktree.headOid)
            const below = graphModel.oidAt((head >= 0 ? head : 1) + 1)
            const oid = below !== "" ? below : worktree.headOid
            const item = graphPane.view.itemAtIndex(graphModel.rowOf(oid))
            if (item === null)
                return false
            item.askMenu()
            return commitMenu.visible
        }
        /// How many of the rows that start git grey, as `menu-wait` counts them.
        function held() {
            let rows = 0
            let greyed = 0
            for (let i = 0; i < commitMenu.count; i++) {
                const row = commitMenu.itemAt(i)
                if (!row || !row.present || row.codeColSeat === undefined || row.subMenu)
                    continue
                if (row.blockedReason !== "" && !row.withdrawn)
                    continue
                rows++
                if (row.blocked)
                    greyed++
            }
            return greyed === rows ? "all" : greyed === 0 ? "none" : greyed + "/" + rows
        }
        function line() {
            const log = page.commandsPane
            return "walk_silent menu=" + commitMenu.visible
                   + " held=" + silentTimer.held()
                   + " why=" + (page.runningWhy !== "")
                   + " busy=" + repoTab.busyCount
                   + " reading=" + repoTab.settling
                   + " running=" + (log !== null && log.commandsModel.running)
                   + " silent=" + graphModel.silent
                   + " stale=" + (graphModel.stale || graphModel.failed || graphModel.silent)
        }
        onTriggered: {
            const op = silentTimer
            if (op.at("start")) {
                if (!Awaited.all("walk_silent_idle", { "open": repoTab.state === "open",
                                                       "idle": repoTab.busyCount === 0,
                                                       "page": PageSettled.settled(page) }))
                    return
                op.input("committed", () => {
                    page.showWip()
                    wipPane.setMessage("chore: commit under a silent walk", "")
                    return driver.pressWrite("commit", () => {
                        page.commitNow()
                        return true
                    })
                })
                return
            }
            if (op.at("committed")) {
                // git has answered the commit and the reading behind it is still out: the walk the shim holds. Or,
                // `after`, the walk told silent and the reading answered without it.
                const answered = repoTab.writeSeq > driver.writeSeqBefore && repoTab.busyCount === 0
                if (!Awaited.all("walk_silent_answered", silentTimer.waitsOut
                                 ? { "answered": answered, "silent": graphModel.silent, "read": !repoTab.settling }
                                 : { "answered": answered, "reading": repoTab.settling }))
                    return
                if (op.landsToo) {
                    op.advance("silent", () => "walk_silent step=silent")
                    return
                }
                op.input("raised", () => op.raiseAll())
                return
            }
            if (op.at("silent")) {
                if (!Awaited.all("walk_silent_landed", { "drawn": graphModel.rowOf(worktree.headOid) >= 0,
                                                         "quiet": !graphModel.silent }))
                    return
                op.input("raised", () => op.raiseAll())
                return
            }
            op.complete({
                "name": "walk_silent_menu",
                "when": () => ({ "menu": commitMenu.visible, "log": page.commandsPane !== null,
                                 "details": page.pageDetailsSettled, "rows": PageSettled.rowsBuilt(page) }),
                "report": () => op.line(), "arriving": ["*", "readings"] })
        }
    }
}
