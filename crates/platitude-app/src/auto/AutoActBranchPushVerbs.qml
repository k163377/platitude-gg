pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The push of a branch the worktree is not on, from the BRANCH card of its row (デザイン規約 §リモートへ送る): the
/// row's shape as the card opens, its press, and the first push's question it raises. `[chip:]<branch>` — bare is the
/// sidebar's entrance, `chip:` the graph row's menu aimed at the branch's chip: the two carry one card, and only a run
/// through each says both are wired to it.
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
    /// What the run does once the card stands: `menu` reads it, `ask` presses a first push's row and stops at the
    /// question, `go` presses it through to git's answer — a question answered too.
    property string after: ""
    /// The card the run stood up.
    property var card: null

    /// Runs `act` if it is one of this family's, and says whether it was.
    function run(act, arg) {
        if (act !== "push-branch-menu" && act !== "push-branch-ask" && act !== "push-branch-go")
            return false
        acts.onChip = arg.startsWith("chip:")
        acts.branch = acts.onChip ? arg.slice("chip:".length) : arg
        acts.after = act === "push-branch-menu" ? "menu" : act === "push-branch-ask" ? "ask" : "go"
        cardTimer.start()
        return true
    }

    /// The BRANCH card on the run's branch, through the door a right-click takes there; answers the card.
    function standCard() {
        const oid = branchesModel.oidOfName(acts.branch)
        if (acts.onChip) {
            page.openRowMenu(oid, { "kind": "branch", "name": acts.branch })
            commitMenu.openSub(driver.commitBranchCard)
            return driver.commitBranchCard
        }
        page.openRefMenu("branch", acts.branch, acts.branch, oid)
        refMenu.openSub(driver.refBranchCard)
        return driver.refBranchCard
    }
    /// A hand's click takes the menu down; a row's `triggered()` called from here does not, so it is told, and
    /// completion waits out its exit (`menuGoing` — the census walks what is visible).
    function closeCard() {
        driver.menuGoing = acts.onChip ? commitMenu : refMenu
        driver.menuGoing.close()
    }

    // The card goes up once nothing is out (a card raised under a write greys its rows), and the row is read off it in
    // the same turn: its whole shape — seat, chip, presses, sentence — is fixed as it opens (デザイン規約 §メニュー).
    SampleTimer {
        id: cardTimer
        onTriggered: {
            // The precondition is read here and nowhere else (app-ui.md §UI 自動化).
            if (!driver.menusLive())
                return
            cardTimer.stop()
            acts.card = acts.standCard()
            const row = acts.card.pushItem
            // The free words last: a claim is one substring.
            Harness.report("push_branch_menu where=" + (acts.onChip ? "chip" : "row")
                              + " offered=" + row.offered
                              + " state=" + acts.card.pushState
                              + " code=" + row.code
                              + " held=" + (row.holdPresses > 0)
                              + " text=" + row.text)
            if (acts.after === "menu" || !row.offered) {
                driver.complete()
                return
            }
            if (acts.card.pushState === "publish") {
                // The question is raised by the row's own handler, and answered from it (`askTimer`).
                row.triggered()
                acts.closeCard()
                askTimer.start()
                return
            }
            if (row.holdPresses > 0) {
                // The held row closes the card itself (`RefBranchMenu`'s `onHeld`).
                driver.holdToEnd(row)
            } else {
                driver.pressWrite("push-branch", () => {
                    row.triggered()
                    return true
                })
                acts.closeCard()
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
            if (!publishFlow.publishChecked || !graphPane.askCard.settled || !graphPane.askAnswerable)
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
            // The gesture a person is given: a hold is not answered by a click (`AutoActPublishVerbs.publishAnswerTimer`).
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
            if (repoTab.busyCount !== 0 || repoTab.writeSeq === driver.writeSeqBefore)
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
