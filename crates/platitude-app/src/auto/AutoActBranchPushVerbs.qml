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
    readonly property bool onChip: Harness.autoActArg.startsWith("chip:")
    readonly property string branch: acts.onChip ? Harness.autoActArg.slice("chip:".length) : Harness.autoActArg
    /// What the run does once the menu stands: `menu` reads the row, `ask` presses a first push's row and stops at the
    /// question, `go` presses it through to git's answer — a question answered too.
    readonly property string after: Harness.autoAct === "push-branch-menu" ? "menu"
                                    : Harness.autoAct === "push-branch-ask" ? "ask" : "go"
    /// The BRANCH card whose answers the push row stands on: the entrance's own.
    readonly property var card: acts.onChip ? driver.commitBranchCard : driver.refBranchCard

    /// Runs `act` if it is one of this family's, and says whether it was.
    function run(act, arg) {
        if (act !== "push-branch-menu" && act !== "push-branch-ask" && act !== "push-branch-go")
            return false
        rowTimer.start()
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
        const menu = acts.onChip ? commitMenu : refMenu
        driver.menuLeaving(menu)
        menu.close()
    }
    /// The row as the menu stands. The free words last: a claim is one substring.
    function rowLine() {
        const row = acts.pushRow()
        return "push_branch_menu where=" + (acts.onChip ? "chip" : "row")
               + " offered=" + row.offered
               + " state=" + acts.card.pushState
               + " code=" + row.code
               + " held=" + (row.holdPresses > 0)
               + " blocked=" + row.blocked
               + " tip=" + row.tip.shown
               + " text=" + row.text
               + (row.blocked ? " says=" + row.blockedReason : "")
    }
    /// The row's press: the first push's question raised by the row's own handler (`asks`, answered from it,
    /// `askTimer`), the held row run to its end (it closes the menu itself, `BranchPushRow`'s `onHeld`), or the plain
    /// row's click. Answers whether it went in.
    function pressRow(row, asks) {
        if (asks) {
            row.triggered()
            acts.closeMenu()
            return true
        }
        if (row.holdPresses > 0)
            return driver.holdToEnd(row)
        const went = driver.pressWrite("push-branch", () => {
            row.triggered()
            return true
        })
        acts.closeMenu()
        return went
    }

    // The menu goes up once nothing is out (a menu raised under a write greys its rows), and the row is read off it
    // from the next beat: its whole shape — seat, chip, presses, sentence, reason — is fixed as it opens
    // (デザイン規約 §メニュー). A row that stands out says why under a hand (デザイン規約 §リモートへ送る): its line waits
    // for that tip, up beside the row — forced, since no pointer can be put on a row from here (verify-ui §hover の絵の
    // 撮り方).
    PageStage {
        id: rowTimer
        name: "push_branch_row"
        // The menu verb ends on the row. The other two end on it too where the row is out, and press it where it is
        // not — which only the row says.
        order: acts.after === "menu" ? ["opened", "tipped"] : []
        /// Whether the row's tip stands where a hand's would, for a row that wants one: what `rowTipStood` answers,
        /// read without forcing anything.
        function tipStands() {
            const row = acts.pushRow()
            return !row.blocked || (row.tip.shown && driver.tipAside(Tips.box))
        }
        onTriggered: {
            const op = rowTimer
            if (op.at("start")) {
                // The precondition is read here and nowhere else (app-ui.md §UI 自動化).
                if (!Awaited.all("push_branch_live", { "live": driver.menusLive() }))
                    return
                op.input("opened", () => {
                    acts.openMenu()
                })
                return
            }
            const row = acts.pushRow()
            // The tip forced up beside the row answers for itself: false until it stands there, and nothing moved.
            if (op.at("opened") && !op.input("tipped", () => driver.rowTipStood(row, row.blocked)))
                return
            if (op.at("tipped")) {
                if (acts.after === "menu" || !row.offered || row.blocked) {
                    op.complete({ "name": "push_branch_row_ready", "when": () => ({ "ready": op.tipStands() }),
                                  "report": () => acts.rowLine() })
                    return
                }
                if (!Awaited.all("push_branch_row_ready", { "ready": op.tipStands() }))
                    return
                // Read before the press, which takes the menu down.
                op.advance("read", () => acts.rowLine())
            }
            const asks = acts.card.pushState === "publish"
            if (!op.input("pressed", () => acts.pressRow(row, asks)))
                return
            op.stop()
            if (asks)
                askTimer.start()
            else
                landedTimer.start()
        }
    }
    // The first push's question the row raised, once the remote has answered for the name and the bar is all the way
    // down. `local=` is the branch it is about — the row's, not the worktree's (デザイン規約 §はじめてリモートへ送る
    // 「対象は呼んだ所が決め」) — which only this line can tell from a picture of the same name.
    PageStage {
        id: askTimer
        name: "push_branch_ask"
        order: acts.after === "go" ? ["read", "answered"] : []
        /// The question standing to be answered: the remote has answered for the name, the bar all the way down.
        function ready() {
            return {
                "checked": publishFlow.publishChecked,
                "card": graphPane.askCard.settled,
                "answerable": graphPane.askAnswerable }
        }
        function line() {
            return "push_branch_ask local=" + publishFlow.publishLocal
                   + " head=" + worktree.branch
                   + " remote=" + publishFlow.publishRemote
                   + " branch=" + publishFlow.publishBranch
                   + " far=" + publishFlow.publishState
        }
        /// The gesture a person is given: a hold is not answered by a click (`AutoActPublishVerbs.publishAnswerTimer`).
        /// Answers whether it went in.
        function answer() {
            if (publishFlow.publishRefused)
                return driver.holdToEnd(graphPane)
            return driver.pressWrite("answer-ask", () => {
                page.answerRowAsk()
                return true
            })
        }
        onTriggered: {
            const op = askTimer
            if (op.at("start")) {
                if (acts.after === "ask") {
                    op.complete({ "name": "push_branch_ask_ready", "when": () => op.ready(),
                                  "report": () => op.line() })
                    return
                }
                if (!Awaited.all("push_branch_ask_ready", op.ready()))
                    return
                // Read before the answer, which takes the question down.
                op.advance("read", () => op.line())
            }
            if (!op.input("answered", () => op.answer()))
                return
            op.stop()
            landedTimer.start()
        }
    }
    // git's answer, judged on the branch's own row: a push that landed leaves it level with where it went — what the
    // card reads (`NavSectionModel.branchPush`), off the listing the write's own refresh publishes. `head=` is the
    // worktree's branch, which nothing here moves.
    PageStage {
        id: landedTimer
        name: "push_branch_landed"
        onTriggered: landedTimer.complete({
            "name": "push_branch_landed_ready",
            "when": () => ({
                "ready": !(repoTab.busyCount !== 0 || repoTab.writeSeq === driver.writeSeqBefore),
                "level": branchesModel.branchPush(acts.branch, repoTab.defaultRemote).state === "clean"
                         || repoTab.writeRefused }),
            "report": () => {
                const landed = branchesModel.branchPush(acts.branch, repoTab.defaultRemote)
                return "push_branch_landed refused=" + repoTab.writeRefused
                       + " state=" + landed.state
                       + " head=" + worktree.branch
                       + " target=" + landed.target
            } })
    }
}
