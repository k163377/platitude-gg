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
    // press on its name opens it, the row pressed, and the window read where the press put it.
    SampleTimer {
        id: pickTimer
        running: Harness.autoAct === "ops-branch-pick" || Harness.autoAct === "ops-worktree-pick"
                 || Harness.autoAct === "ops-repo-pick"
        /// 0 = open the card, 1 = open its tier (the repository's two), 2 = press the row, 3 = wait for the landing.
        property int step: 0
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
        /// The frame after the press, grabbed to `pressed.png`: 0 while owed, 1 saved, -1 not.
        property int pressShot: 0
        onTriggered: {
            const page = acts.window.curPage
            if (!acts.window.visible || page === null)
                return
            const door = Harness.autoAct === "ops-branch-pick" ? "branch"
                       : Harness.autoAct === "ops-worktree-pick" ? "worktree" : "repo"
            if (pickTimer.step === 0) {
                if (!acts.ready(page))
                    return
                // The repository door needs a second tab; with one, the run says so now rather than at the ceiling.
                if (door === "repo" && acts.pageRepeater.count < 2) {
                    if (acts.tabsModel.opening)
                        return
                    Harness.report("ops_pick step=open door=repo tabs=" + acts.pageRepeater.count)
                    acts.finish(door, false)
                    return
                }
                // For the repository's two, the name's own card first: a tier hangs off a row on that card.
                const opened = door === "branch" ? acts.topBar.openBranchMenu() : acts.topBar.openStandMenu()
                pickTimer.step = door === "branch" ? 2 : 1
                Harness.report("ops_pick step=open door=" + door + " opened=" + opened)
                return
            }
            if (pickTimer.step === 1) {
                if (!acts.topBar.standMenuOpen)
                    return
                const opened = door === "worktree" ? acts.topBar.openStandWorktrees() : acts.topBar.openStandRepos()
                pickTimer.step = 2
                Harness.report("ops_pick step=tier door=" + door + " opened=" + opened)
                return
            }
            if (pickTimer.step === 2) {
                // Card and tab read in the turn that presses, not the tick that opened the card: a read landing
                // between the two can start something the move is then refused over.
                const wanted = door === "branch" ? "branch" : door === "worktree" ? "worktrees" : "repos"
                if (acts.topBar.standDoor !== wanted || !acts.ready(page))
                    return
                pickTimer.from = door === "branch" ? acts.topBar.branchName : page.pageTab.repoPath
                pickTimer.fromNames = acts.names()
                pickTimer.errorBefore = page.pageTab.lastError
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
                            pickTimer.wantedTab = i
                    }
                }
                pickTimer.pressNames = acts.names()
                // waits(measured): the origin of `headMs=` below, which the report prints and nothing here reads
                pickTimer.pressedAt = Date.now()
                // The panel of the tab moved to before it has read anything, which the run's own shot, taken at the
                // landing, cannot show; the next frame is drawn long before git can answer. A grab that could not be
                // asked for is said as such rather than waited on.
                if (door === "repo" && pressed) {
                    const asked = Harness.shotDir !== "" && acts.topBar.grabToImage(shot => {
                        pickTimer.pressShot = shot.saveToFile(Harness.shotDir + "/pressed.png") ? 1 : -1
                    })
                    if (!asked)
                        pickTimer.pressShot = -1
                }
                pickTimer.step = 3
                Harness.report("ops_pick step=pressed door=" + door + " found=" + pressed)
                // A row not on the card: nothing to wait for, so finish now rather than at the ceiling.
                if (!pressed)
                    acts.finish(door, false)
                return
            }
            if (door === "repo" && pickTimer.headAt === 0 && acts.topBar.branchName !== "")
                // waits(measured): the other end of `headMs=`, to the sampler's tick — printed and compared with nothing
                pickTimer.headAt = Date.now()
            // A move turned away (a worded refusal, or a worktree that would not open) is an answer too, not a ceiling.
            const refused = page.pageTab.state === "error"
                            || (page.pageTab.lastError !== "" && page.pageTab.lastError !== pickTimer.errorBefore)
            if (!refused && (!acts.landed(door, page) || (door === "repo" && pickTimer.pressShot === 0)))
                return
            acts.finish(door, true)
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

    function finish(door, found) {
        pickTimer.stop()
        const page = acts.window.curPage
        Harness.report("ops_pick door=" + door + " found=" + found
                       + " landed=" + (found && acts.landed(door, page))
                       + " card=" + acts.topBar.standDoor
                       // The repository door moves the tab in front in the press itself: names already the landing's
                       // then, and not the ones before, were known at the move without waiting on git (the run's
                       // repositories are named apart, `Route::NamedPresets`).
                       + (door === "repo" ? " named=" + (found && pickTimer.pressNames === acts.names()
                                                         && pickTimer.pressNames !== pickTimer.fromNames) : "")
                       // Diagnosis only: the tab moved to reads its repository from the start (`Hub::release_tab`),
                       // so this door's press leaves the branch unknown until the status answers.
                       + (door === "repo"
                          ? " headMs=" + (pickTimer.headAt > 0 ? Math.round(pickTimer.headAt - pickTimer.pressedAt) : -1)
                            + " pressShot=" + (pickTimer.pressShot === 1)
                          : "")
                       + " error=" + (page.pageTab.lastError !== pickTimer.errorBefore)
                       + " from=" + pickTimer.from
                       + " branch=" + acts.topBar.branchName
                       + " where=" + GitFacts.pathLeaf(page.pageTab.repoPath))
        acts.window.finishAutoAct()
    }
}
