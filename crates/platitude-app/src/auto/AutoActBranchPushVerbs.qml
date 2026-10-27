pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The push of a branch the worktree is not on, from its row's menu beside `pull` (デザイン規約 §リモートへ送る): the
/// row's shape as the menu opens, its press, and the first push's question it raises. `[chip:]<branch>` — bare is the
/// sidebar's entrance, `chip:` the graph row's menu aimed at the branch's chip: each carries the row (`BranchPushRow`)
/// on its own BRANCH card's answers, and only a run through each says both are wired.
// `Item` because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    /// `var`: naming its type would be a cycle — the driver is the file that builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var repoTab: driver.repoTab
    readonly property var worktree: driver.worktree
    readonly property var branchesModel: driver.branchesModel
    readonly property var graphPane: driver.graphPane
    readonly property var publishFlow: driver.publishFlow
    readonly property var refMenu: driver.refMenu
    readonly property var commitMenu: driver.commitMenu
    readonly property var writeBarrier: driver.barrierWrite

    /// The run's branch and entrance, off its argument.
    property string branch: ""
    property bool onChip: false
    /// What the run does once the menu stands: `menu` reads the row, `ask` presses a first push's row and stops at the
    /// question, `go` presses it through to git's answer — a question answered too.
    property string after: ""
    /// The BRANCH card whose answers the push row stands on: the entrance's own.
    readonly property var card: acts.onChip ? driver.commitBranchCard : driver.refBranchCard

    /// Runs `act` if it is one of this family's, and says whether it was.
    function run(act, arg) {
        if (act !== "push-branch-menu" && act !== "push-branch-ask" && act !== "push-branch-go")
            return false
        acts.onChip = arg.startsWith("chip:")
        acts.branch = acts.onChip ? arg.slice("chip:".length) : arg
        acts.after = act === "push-branch-menu" ? "menu" : act === "push-branch-ask" ? "ask" : "go"
        menuTimer.start()
        return true
    }

    /// The menu on the run's branch, through the door a right-click takes there.
    function openMenu() {
        const oid = branchesModel.oidOfName(acts.branch)
        if (acts.onChip)
            page.openRowMenu(oid, { "kind": "branch", "name": acts.branch })
        else
            page.openRefMenu("branch", acts.branch, acts.branch, oid)
    }
    /// The push row the run reads and presses, beside `pull` on the menu's first level.
    function pushRow() {
        return acts.onChip ? driver.commitRowMenu.pushBranchItem : driver.refRowMenu.pushBranchItem
    }
    /// A hand's click takes the menu down; a row's `triggered()` called from here does not, so it is told, and
    /// completion waits out its exit (`menuGoing` — the census walks what is visible).
    function closeMenu() {
        driver.menuGoing = acts.onChip ? commitMenu : refMenu
        driver.menuGoing.close()
    }

    // The menu goes up once nothing is out (a menu raised under a write greys its rows), and the row is read off it
    // from the next beat: its whole shape — seat, chip, presses, sentence, reason — is fixed as it opens
    // (デザイン規約 §メニュー).
    SampleTimer {
        id: menuTimer
        onTriggered: {
            // The precondition is read here and nowhere else (app-ui.md §UI 自動化).
            if (!driver.menusLive())
                return
            menuTimer.stop()
            acts.openMenu()
            rowTimer.start()
        }
    }
    // A row that stands out says why under a hand (デザイン規約 §リモートへ送る): its line waits for that tip, up beside
    // the row — forced, since no pointer can be put on a row from here (verify-ui §hover の絵の撮り方).
    SampleTimer {
        id: rowTimer
        onTriggered: {
            const row = acts.pushRow()
            if (!Awaited.all("row_ready", { "ready": driver.rowTipStood(row, row.blocked) }))
                return
            rowTimer.stop()
            // The free words last: a claim is one substring.
            Harness.report("push_branch_menu where=" + (acts.onChip ? "chip" : "row")
                              + " offered=" + row.offered
                              + " state=" + acts.card.pushState
                              + " code=" + row.code
                              + " held=" + (row.holdPresses > 0)
                              + " blocked=" + row.blocked
                              + " tip=" + row.tip.shown
                              + " text=" + row.text
                              + (row.blocked ? " says=" + row.blockedReason : ""))
            if (acts.after === "menu" || !row.offered || row.blocked) {
                driver.complete()
                return
            }
            if (acts.card.pushState === "publish") {
                // The question is raised by the row's own handler, and answered from it (`askTimer`).
                row.triggered()
                acts.closeMenu()
                askTimer.start()
                return
            }
            if (row.holdPresses > 0) {
                // The held row closes the menu itself (`BranchPushRow`'s `onHeld`).
                driver.holdToEnd(row)
            } else {
                driver.pressWrite("push-branch", () => {
                    row.triggered()
                    return true
                })
                acts.closeMenu()
            }
            landedTimer.start()
        }
    }
    // The first push's question the row raised, once the remote has answered for the name and the bar is all the way
    // down. `local=` is the branch it is about — the row's, not the worktree's (デザイン規約 §はじめてリモートへ送る
    // 「対象は呼んだ所が決め」) — which only this line can tell from a picture of the same name.
    SampleTimer {
        id: askTimer
        onTriggered: {
            if (!Awaited.all("ask_ready", {
                "ready": !(!publishFlow.publishChecked || !graphPane.askCard.settled || !graphPane.askAnswerable) }))
                return
            askTimer.stop()
            Harness.report("push_branch_ask local=" + publishFlow.publishLocal
                              + " head=" + worktree.branch
                              + " remote=" + publishFlow.publishRemote
                              + " branch=" + publishFlow.publishBranch
                              + " far=" + publishFlow.publishState)
            if (acts.after === "ask") {
                driver.complete()
                return
            }
            // The gesture a person is given: a hold is not answered by a click
            // (`AutoActPublishVerbs.publishAnswerTimer`).
            if (publishFlow.publishRefused)
                driver.holdToEnd(graphPane)
            else
                driver.pressWrite("answer-ask", () => {
                    page.answerRowAsk()
                    return true
                })
            landedTimer.start()
        }
    }
    // git's answer, judged on the branch's own row: a push that landed leaves it level with where it went — what the
    // card reads (`NavSectionModel.branchPush`), off the listing the write's own refresh publishes. `head=` is the
    // worktree's branch, which nothing here moves.
    SampleTimer {
        id: landedTimer
        onTriggered: {
            if (!Awaited.all("landed_ready", {
                "ready": !(repoTab.busyCount !== 0 || repoTab.writeSeq === driver.writeSeqBefore) }))
                return
            const facts = branchesModel.branchPush(acts.branch, repoTab.defaultRemote)
            if (facts.state !== "clean" && !repoTab.writeRefused)
                return
            landedTimer.stop()
            Harness.report("push_branch_landed refused=" + repoTab.writeRefused
                              + " state=" + facts.state
                              + " head=" + worktree.branch
                              + " target=" + facts.target)
            driver.complete()
        }
    }
}
