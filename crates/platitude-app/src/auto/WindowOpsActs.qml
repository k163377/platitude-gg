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
        /// The card the verb presses a row of.
        readonly property string door: Harness.autoAct === "ops-branch-pick" ? "branch"
                                     : Harness.autoAct === "ops-worktree-pick" ? "worktree" : "repo"
        // Kept as the row is pressed: whether it was on the card (`found`), what the window stood on (`from`), the
        // error line the tab stood on — a new one is the move turned away (`errorBefore`) — the tab asked for
        // (`wantedTab`), the panel's names just before the press and in its turn, before the tab moved to has read
        // anything (`fromNames`, `pressNames`), when the press went in (`pressedAt`), and whether the panel's own
        // picture (`pressed.png`, the frame after the press) was saved (`pressShot`: `saved` / `failed`, kept as the
        // grab answers). After it, the first tick the panel named a branch (`headAt`, `headMs=`).
        /// The fact this generation kept under `name`, or `none` where it kept none: the run that ends before the
        /// press, a grab not answered yet, a branch not named yet.
        function keptOr(name, none) {
            try {
                return pickTimer.witnessed(name)
            } catch (e) {
                return none
            }
        }
        /// Whether the press has landed, read now. A move turned away (a worded refusal, or a worktree that would
        /// not open) is an answer too, not a ceiling.
        function moved() {
            const page = acts.window.curPage
            const refused = page.pageTab.state === "error"
                            || (page.pageTab.lastError !== ""
                                && page.pageTab.lastError !== pickTimer.witnessed("errorBefore"))
            return !pickTimer.witnessed("found") || refused
                || (acts.landed(pickTimer.door, page)
                    && (pickTimer.door !== "repo" || pickTimer.keptOr("pressShot", "") !== ""))
        }
        onTriggered: {
            const op = pickTimer
            const page = acts.window.curPage
            const door = op.door
            if (op.at("start")) {
                // Read here alone: the move the later stages wait for unsettles the page, and a guard read every beat
                // across them would hold them up for the move's own reads.
                if (!Awaited.all("ops_pick_ready", { "shown": acts.window.visible,
                                                 "ready": page !== null && acts.ready(page),
                                                 "page": PageSettled.windowSettled(window) }))
                    return
                // The repository door needs a second tab: with one, the staging went wrong, said and ended at once.
                if (door === "repo" && acts.pageRepeater.count < 2) {
                    op.complete({
                        "name": "ops_pick_alone",
                        "when": () => ({ "alone": acts.pageRepeater.count < 2 }),
                        "report": () => ["ops_pick step=open door=repo tabs=" + acts.pageRepeater.count,
                                         acts.pickLine(op.door, false)] })
                    return
                }
                // For the repository's two, the name's own card first: a tier hangs off a row on that card. What the
                // card's press answered is kept (`opened=`).
                op.input("card", () => {
                    op.witness("opened",
                               door === "branch" ? acts.topBar.openBranchMenu() : acts.topBar.openStandMenu())
                }, () => "ops_pick step=open door=" + op.door + " opened=" + op.witnessed("opened"))
                return
            }
            if (op.at("card") && door !== "branch") {
                if (!Awaited.all("ops_pick_stand_menu", { "menu": acts.topBar.standMenuOpen }))
                    return
                // …and what the tier's answered.
                op.input("tier", () => {
                    op.witness("tierOpened", door === "worktree" ? acts.topBar.openStandWorktrees()
                                                                 : acts.topBar.openStandRepos())
                }, () => "ops_pick step=tier door=" + op.door + " opened=" + op.witnessed("tierOpened"))
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
                    op.witness("from", door === "branch" ? acts.topBar.branchName : page.pageTab.repoPath)
                    op.witness("fromNames", acts.names())
                    op.witness("errorBefore", page.pageTab.lastError)
                    let pressed = false
                    if (door === "branch") {
                        pressed = acts.topBar.pickBranchRow(Harness.autoActArg)
                    } else if (door === "worktree") {
                        pressed = acts.topBar.pickWorktreeRow(Harness.autoActArg)
                    } else {
                        let wantedTab = -1
                        for (let i = 0; i < acts.pageRepeater.count && !pressed; i++) {
                            if (i === acts.tabsModel.currentIndex)
                                continue
                            pressed = acts.topBar.pickRepoRow(i)
                            if (pressed)
                                wantedTab = i
                        }
                        op.witness("wantedTab", wantedTab)
                    }
                    op.witness("found", pressed)
                    op.witness("pressNames", acts.names())
                    // waits(measured): the origin of `headMs=` below, which the report prints and nothing here reads
                    op.witness("pressedAt", Date.now())
                    // The panel of the tab moved to before it has read anything, which the run's own shot, taken at
                    // the landing, cannot show; the next frame is drawn long before git can answer. A grab that could
                    // not be asked for is said as such rather than waited on; one answering after the run ended is
                    // kept by none.
                    if (door === "repo" && pressed) {
                        const asked = Harness.shotDir !== "" && acts.topBar.grabToImage(shot => {
                            const saved = shot.saveToFile(Harness.shotDir + "/pressed.png") ? "saved" : "failed"
                            if (op.running)
                                op.witness("pressShot", saved)
                        })
                        if (!asked)
                            op.witness("pressShot", "failed")
                    }
                }, () => "ops_pick step=pressed door=" + op.door + " found=" + op.witnessed("found"))
                return
            }
            if (door === "repo" && op.keptOr("headAt", 0) === 0 && acts.topBar.branchName !== "")
                // waits(measured): the other end of `headMs=`, to the sampler's tick — printed and compared with nothing
                op.witness("headAt", Date.now())
            op.complete({
                "name": "ops_pick_landed",
                "when": () => ({ "landed": op.moved() }),
                "report": () => acts.pickLine(op.door, op.witnessed("found")) })
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
        return acts.tabsModel.currentIndex === pickTimer.witnessed("wantedTab")
    }

    function names() {
        return acts.topBar.repoName + "/" + acts.topBar.worktreeName
    }

    /// The run's line, read as the run completes: where the press left the window. A run that ended before the press
    /// kept nothing of it.
    function pickLine(door, found) {
        const page = acts.window.curPage
        const headAt = pickTimer.keptOr("headAt", 0)
        return "ops_pick door=" + door + " found=" + found
               + " landed=" + (found && acts.landed(door, page))
               + " card=" + acts.topBar.standDoor
               // The repository door moves the tab in front in the press itself: names already the landing's then,
               // and not the ones before, were known at the move without waiting on git (the run's repositories are
               // named apart, `Route::NamedPresets`).
               + (door === "repo" ? " named=" + (found && pickTimer.witnessed("pressNames") === acts.names()
                                                 && pickTimer.witnessed("pressNames")
                                                    !== pickTimer.witnessed("fromNames")) : "")
               // Diagnosis only: the tab moved to reads its repository from the start (`Hub::release_tab`), so this
               // door's press leaves the branch unknown until the status answers.
               + (door === "repo"
                  ? " headMs=" + (headAt > 0 ? Math.round(headAt - pickTimer.witnessed("pressedAt")) : -1)
                    + " pressShot=" + (pickTimer.keptOr("pressShot", "") === "saved")
                  : "")
               + " error=" + (page.pageTab.lastError !== pickTimer.keptOr("errorBefore", ""))
               + " from=" + pickTimer.keptOr("from", "")
               + " branch=" + acts.topBar.branchName
               + " where=" + GitFacts.pathLeaf(page.pageTab.repoPath)
    }
}
