pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The operation panel's cards, a row of each pressed: branch, worktree, repository. What is judged is the landing,
/// because a row that closes its card and reaches nothing frames like a row still on its way.
// `Item`, not `QtObject`: rules-refs/app-ui.md「ドライバの root は `Item`」.
Item {
    id: acts

    required property var window
    required property TabsModel tabsModel
    required property Repeater pageRepeater
    required property TopBar topBar

    // PGG_AUTO_ACT=ops-branch-pick <branch> / ops-worktree-pick <folder> / ops-repo-pick: the card opened the way a
    // press on its name opens it, the row pressed, and the window read where the press put it. The branch's card holds
    // its rows; the repository's two hang off a tier of the name's card.
    // No order: the repository door with one tab says so and ends before any card (a precondition missing).
    WindowStage {
        id: pickTimer
        name: "ops_pick"
        running: Harness.autoAct === "ops-branch-pick" || Harness.autoAct === "ops-worktree-pick"
                 || Harness.autoAct === "ops-repo-pick"
        /// What the card's (or the tier's) press answered (`opened=`).
        property bool opened: false
        /// Whether the row was on the card as it was pressed (`found=`).
        property bool found: false
        /// The error line the tab stood on as the row was pressed — a new one is the move turned away.
        property string errorBefore: ""
        property int wantedTab: -1
        /// What the window stood on as the row was pressed (`from=`).
        property string from: ""
        /// The panel's names just before the press and in its turn — before the tab moved to has read anything.
        property string fromNames: ""
        property string pressNames: ""
        /// When the press went in, and the first tick the panel named a branch after it (`headMs=`).
        property real pressedAt: 0
        property real headAt: 0
        /// Whether the panel's own picture (`pressed.png`, the frame after the press) was saved: `saved` /
        /// `failed`, "" while not asked.
        property string pressShot: ""
        onTriggered: {
            const op = pickTimer
            const page = acts.window.curPage
            const door = Harness.autoAct === "ops-branch-pick" ? "branch"
                       : Harness.autoAct === "ops-worktree-pick" ? "worktree" : "repo"
            if (op.at("start")) {
                // Read here alone: the move the later stages wait for unsettles the page, and a guard read every beat
                // across them would hold them up for the move's own reads.
                if (!Awaited.all("ops_pick_ready", { "shown": acts.window.visible,
                                                 "ready": page !== null && acts.ready(page),
                                                 "page": PageSettled.windowSettled(window) }))
                    return
                // The repository door needs a second tab: with one, the staging went wrong, said and ended at once.
                if (door === "repo" && acts.pageRepeater.count < 2) {
                    const tabs = acts.pageRepeater.count
                    op.complete(() => ["ops_pick step=open door=repo tabs=" + tabs, acts.pickLine(door, false)])
                    return
                }
                // For the repository's two, the name's own card first: a tier hangs off a row on that card.
                op.input("card", () => {
                    op.opened = door === "branch" ? acts.topBar.openBranchMenu() : acts.topBar.openStandMenu()
                }, () => "ops_pick step=open door=" + door + " opened=" + op.opened)
                return
            }
            if (op.at("card") && door !== "branch") {
                if (!Awaited.all("ops_pick_stand_menu", { "menu": acts.topBar.standMenuOpen }))
                    return
                op.input("tier", () => {
                    op.opened = door === "worktree" ? acts.topBar.openStandWorktrees() : acts.topBar.openStandRepos()
                }, () => "ops_pick step=tier door=" + door + " opened=" + op.opened)
                return
            }
            if (!op.at("pressed")) {
                // Card and tab read in the turn that presses, not the tick that opened the card: a read landing
                // between the two can start something the move is then refused over.
                const wanted = door === "branch" ? "branch" : door === "worktree" ? "worktrees" : "repos"
                if (!Awaited.all("ops_pick_tier", { "door": acts.topBar.standDoor === wanted,
                                                    "ready": acts.ready(page) }))
                    return
                // A row not on the card goes in as a press that found nothing: the landing then has nothing to wait
                // for, and the run ends on `found=false` rather than at the ceiling.
                op.input("pressed", () => {
                    op.from = door === "branch" ? acts.topBar.branchName : page.pageTab.repoPath
                    op.fromNames = acts.names()
                    op.errorBefore = page.pageTab.lastError
                    let pressed = false
                    if (door === "branch") {
                        pressed = acts.topBar.pickBranchRow(Harness.autoActArg)
                    } else if (door === "worktree") {
                        pressed = acts.topBar.pickWorktreeRow(Harness.autoActArg)
                    } else {
                        for (let i = 0; i < acts.pageRepeater.count && !pressed; i++) {
                            if (i === acts.tabsModel.currentIndex)
                                continue
                            pressed = acts.topBar.pickRepoRow(i)
                            if (pressed)
                                op.wantedTab = i
                        }
                    }
                    op.found = pressed
                    op.pressNames = acts.names()
                    // waits(measured): the origin of `headMs=` below, which the report prints and nothing here reads
                    op.pressedAt = Date.now()
                    // The panel of the tab moved to before it has read anything, which the run's own shot, taken at
                    // the landing, cannot show; the next frame is drawn long before git can answer. A grab that could
                    // not be asked for is said as such rather than waited on.
                    if (door === "repo" && pressed) {
                        const asked = Harness.shotDir !== "" && acts.topBar.grabToImage(shot => {
                            op.pressShot = shot.saveToFile(Harness.shotDir + "/pressed.png") ? "saved" : "failed"
                        })
                        op.pressShot = asked ? op.pressShot : "failed"
                    }
                }, () => "ops_pick step=pressed door=" + door + " found=" + op.found)
                return
            }
            if (door === "repo" && op.headAt === 0 && acts.topBar.branchName !== "")
                // waits(measured): the other end of `headMs=`, to the sampler's tick — printed and compared with nothing
                op.headAt = Date.now()
            // A move turned away (a worded refusal, or a worktree that would not open) is an answer too, not a ceiling.
            const refused = page.pageTab.state === "error"
                            || (page.pageTab.lastError !== "" && page.pageTab.lastError !== op.errorBefore)
            if (!Awaited.all("ops_pick_landed", {
                    "landed": !op.found || refused
                              || (acts.landed(door, page) && (door !== "repo" || op.pressShot !== "")) }))
                return
            op.complete(() => acts.pickLine(door, op.found))
        }
    }

    /// Whether a card can be opened and a move asked: every listing the card is built from is in (else rows are
    /// missing), and nothing is running (a busy tab refuses the move, `RepoPage.switchToRef`).
    function ready(page) {
        return page.pageTab.state === "open" && page.pageTab.busyCount === 0
            && page.pageRefsLoaded && page.pageWorktrees.total > 0 && PageSettled.settled(page)
    }

    /// Whether the press landed, read off the window's own answers, not the card's.
    function landed(door, page) {
        if (page.pageTab.state !== "open" || page.pageTab.busyCount !== 0 || !PageSettled.settled(page))
            return false
        if (door === "branch")
            return acts.topBar.branchName === Harness.autoActArg
        if (door === "worktree")
            return GitFacts.pathLeaf(page.pageTab.repoPath) === Harness.autoActArg
        return acts.tabsModel.currentIndex === pickTimer.wantedTab
    }

    function names() {
        return acts.topBar.repoName + "/" + acts.topBar.worktreeName
    }

    /// The run's line, read as the run completes: where the press left the window.
    function pickLine(door, found) {
        const page = acts.window.curPage
        return "ops_pick door=" + door + " found=" + found
               + " landed=" + (found && acts.landed(door, page))
               + " card=" + acts.topBar.standDoor
               // The repository door moves the tab in front in the press itself: names already the landing's then,
               // and not the ones before, were known at the move without waiting on git (the run's repositories are
               // named apart, `Route::NamedPresets`).
               + (door === "repo" ? " named=" + (found && pickTimer.pressNames === acts.names()
                                                 && pickTimer.pressNames !== pickTimer.fromNames) : "")
               // Diagnosis only: the tab moved to reads its repository from the start (`Hub::release_tab`), so this
               // door's press leaves the branch unknown until the status answers.
               + (door === "repo"
                  ? " headMs=" + (pickTimer.headAt > 0 ? Math.round(pickTimer.headAt - pickTimer.pressedAt) : -1)
                    + " pressShot=" + (pickTimer.pressShot === "saved")
                  : "")
               + " error=" + (page.pageTab.lastError !== pickTimer.errorBefore)
               + " from=" + pickTimer.from
               + " branch=" + acts.topBar.branchName
               + " where=" + GitFacts.pathLeaf(page.pageTab.repoPath)
    }
}
