pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The operation panel's cards, a row of each pressed: the branch a card offers moved to, the copy it offers stood
/// in, the repository it offers put in front. Each is photographed where the move lands — **what is judged is the
/// landing**, because a row that closes its card and reaches nothing frames exactly like a row that is still on its
/// way.
///
/// Built by `WindowAutoActDriver`, which is what `Main` builds when a verb was given; what these verbs act on is
/// handed down below, one property per part of the window they reach into.
// An `Item` only because `QtObject` has no default property to hold the timers below; it is a sizeless holder.
Item {
    id: acts

    required property var window
    required property TabsModel tabsModel
    required property Repeater pageRepeater
    required property TopBar topBar

    // PGG_AUTO_ACT=ops-branch-pick <branch> / ops-copy-pick <folder> / ops-repo-pick: the card opened the way a
    // press on its name opens it, the row pressed, and the window read where the press put it.
    //
    // **Each step named as it is entered** (rules/app-ui.md §UI 自動化): a run that ends on its ceiling says with its
    // last line which of them it never got past — a card that did not open, a row that was not there, or a press that
    // went in and landed nowhere.
    SampleTimer {
        id: pickTimer
        running: Harness.autoAct === "ops-branch-pick" || Harness.autoAct === "ops-copy-pick"
                 || Harness.autoAct === "ops-repo-pick"
        /// 0 = open the card, 1 = open its tier (the repository's two), 2 = press the row, 3 = wait for the landing.
        property int step: 0
        /// The error line the tab stood on as the row was pressed — a new one is the move turned away.
        property string errorBefore: ""
        /// Which tab the repository row was pressed for, read off the row the run chose.
        property int wantedTab: -1
        /// What the window stood on as the row was pressed — the other half of `moved=`.
        property string from: ""
        /// The names the panel said just before the press, and in the same turn as it — before the tab moved to has
        /// read anything.
        property string fromNames: ""
        property string pressNames: ""
        /// When the press went in, and the first tick the panel named a branch after it — `headMs=` in the report.
        property real pressedAt: 0
        property real headAt: 0
        /// The band and the panel as the frame after the press drew them, saved beside the run's own pictures
        /// (`pressed.png`): 0 while the grab is owed, 1 saved, -1 not.
        property int pressShot: 0
        onTriggered: {
            const page = acts.window.curPage
            if (!acts.window.visible || page === null)
                return
            const door = Harness.autoAct === "ops-branch-pick" ? "branch"
                       : Harness.autoAct === "ops-copy-pick" ? "copy" : "repo"
            if (pickTimer.step === 0) {
                if (!acts.ready(page))
                    return
                // The repository door needs a second tab to offer; a strip that finished opening with one is a run
                // with nothing to press, said now rather than sat out to the ceiling.
                if (door === "repo" && acts.pageRepeater.count < 2) {
                    if (acts.tabsModel.opening)
                        return
                    Harness.report("ops_pick step=open door=repo tabs=" + acts.pageRepeater.count)
                    acts.finish(door, false)
                    return
                }
                // The card the way a press on the name opens it — for the repository's two, the name's own card
                // first: a tier hangs off its row, and a row on a card that is not up has nowhere to stand the tier.
                const opened = door === "branch" ? acts.topBar.openBranchMenu() : acts.topBar.openStandMenu()
                pickTimer.step = door === "branch" ? 2 : 1
                Harness.report("ops_pick step=open door=" + door + " opened=" + opened)
                return
            }
            if (pickTimer.step === 1) {
                if (!acts.topBar.standMenuOpen)
                    return
                const opened = door === "copy" ? acts.topBar.openStandCopies() : acts.topBar.openStandRepos()
                pickTimer.step = 2
                Harness.report("ops_pick step=tier door=" + door + " opened=" + opened)
                return
            }
            if (pickTimer.step === 2) {
                // **The card the row is on is the one standing, and the tab is still one a move can be asked of** —
                // read here, in the turn that presses, not on the tick that opened the card (a read that lands
                // between the two can start something the move is then refused over).
                const wanted = door === "branch" ? "branch" : door === "copy" ? "copies" : "repos"
                if (acts.topBar.standDoor !== wanted || !acts.ready(page))
                    return
                pickTimer.from = door === "branch" ? acts.topBar.branchName : page.pageTab.repoPath
                pickTimer.fromNames = acts.names()
                pickTimer.errorBefore = page.pageTab.lastError
                let pressed = false
                if (door === "branch") {
                    pressed = acts.topBar.pickBranchRow(Harness.autoActArg)
                } else if (door === "copy") {
                    pressed = acts.topBar.pickCopyRow(Harness.autoActArg)
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
                // **The panel of the tab moved to before it has read anything** — the one picture of the names said
                // at the move and the branch still waited on, which the run's own shot, taken at the landing, cannot
                // be. The next frame is drawn long before a process git starts can answer. A grab that could not be
                // asked for, or nowhere to put it, is said as such rather than waited on.
                if (door === "repo" && pressed) {
                    const asked = Harness.shotDir !== "" && acts.topBar.grabToImage(shot => {
                        pickTimer.pressShot = shot.saveToFile(Harness.shotDir + "/pressed.png") ? 1 : -1
                    })
                    if (!asked)
                        pickTimer.pressShot = -1
                }
                pickTimer.step = 3
                Harness.report("ops_pick step=pressed door=" + door + " found=" + pressed)
                // A row that is not on the card is a run with nothing to wait for: said now, and judged by the
                // line rather than sat out to the ceiling.
                if (!pressed)
                    acts.finish(door, false)
                return
            }
            if (door === "repo" && pickTimer.headAt === 0 && acts.topBar.branchName !== "")
                // waits(measured): the other end of `headMs=`, to the sampler's tick — printed and compared with nothing
                pickTimer.headAt = Date.now()
            // **A move turned away is an answer too**: a refusal the tab put into words, or a copy that would not open,
            // lands nowhere — said as that, with the line it ended on, rather than sat out to the ceiling.
            const refused = page.pageTab.state === "error"
                            || (page.pageTab.lastError !== "" && page.pageTab.lastError !== pickTimer.errorBefore)
            if (!refused && (!acts.landed(door, page) || (door === "repo" && pickTimer.pressShot === 0)))
                return
            acts.finish(door, true)
        }
    }

    /// Whether the tab is one a card can be opened on and a move asked of: every listing the card is assembled from,
    /// and nothing running — a card asked for between the reads is a card with rows still missing, and a move asked
    /// of a busy tab is refused where it stands (`RepoPage.switchToRef`).
    function ready(page) {
        return page.pageTab.state === "open" && page.pageTab.busyCount === 0
            && page.pageRefsLoaded && page.pageWorktrees.total > 0 && PageSettled.settled(page)
    }

    /// Whether the press has landed where it was aimed. **The window's own answers**, not the card's: the branch the
    /// panel says it stands on, the copy the tab reads, the tab in front.
    function landed(door, page) {
        if (page.pageTab.state !== "open" || page.pageTab.busyCount !== 0 || !PageSettled.settled(page))
            return false
        if (door === "branch")
            return acts.topBar.branchName === Harness.autoActArg
        if (door === "copy")
            return GitFacts.pathLeaf(page.pageTab.repoPath) === Harness.autoActArg
        return acts.tabsModel.currentIndex === pickTimer.wantedTab
    }

    /// The two names the panel writes for where the window stands.
    function names() {
        return acts.topBar.repoName + "/" + acts.topBar.copyName
    }

    function finish(door, found) {
        pickTimer.stop()
        const page = acts.window.curPage
        Harness.report("ops_pick door=" + door + " found=" + found
                       + " landed=" + (found && acts.landed(door, page))
                       + " card=" + acts.topBar.standDoor
                       // **The repository door moves the tab in front in the press itself**, and the tab moved to has
                       // read nothing yet: names that were already the landing's then, and were not the ones before
                       // the press, are names the panel knew at the move — not ones it waited on git for, and not the
                       // tab it left still standing (the run's repositories are named apart, `Route::NamedPresets`).
                       // The other doors reach their landing through git.
                       + (door === "repo" ? " named=" + (found && pickTimer.pressNames === acts.names()
                                                         && pickTimer.pressNames !== pickTimer.fromNames) : "")
                       // How long the branch took to be named after the press: the tab moved to reads its repository
                       // from the start (`Hub::release_tab`), and the branch is the status's answer. Diagnosis only —
                       // and only for this door, the one whose press leaves the branch unknown.
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
