pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The file rows of the worktree: the menu a row opens, the whole-bucket moves, and the reads an opening
/// fires.
// `Item` because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    /// `var`: typing it `AutoActDriver` would be circular — that file builds this one.
    required property var driver

    // The driver's own names, read once so the verbs can name them bare.
    readonly property var page: driver.page
    readonly property var repoTab: driver.repoTab
    readonly property var worktree: driver.worktree
    readonly property var graphModel: driver.graphModel
    readonly property var branchesModel: driver.branchesModel
    readonly property var wipPane: driver.wipPane
    readonly property var fileRowMenu: driver.fileRowMenu
    readonly property var fileMenu: driver.fileMenu
    readonly property var fileDiscardItem: driver.fileDiscardItem
    readonly property var renderedBarrier: driver.barrierRendered

    /// Runs `act` if it is one of this family's, and says whether it was.
    function run(act, arg) {
        if (act === "stage-all" || act === "unstage-all" || act === "resolve-all") {
            page.showWip()
            bucketAllTimer.begin(act === "stage-all" ? "unstaged"
                                 : act === "unstage-all" ? "staged" : "conflicts")
        } else if (driver.fileRowActs.indexOf(act) >= 0) {
            page.showWip()
            fileRowsTimer.start()
        } else if (act === "open-fetches") {
            acts.openFetchRows = Math.max(1, Number(arg))
            openFetchTimer.start()
        } else {
            return false
        }
        return true
    }
    // Empties one whole bucket from its own heading and reads back which headings the list keeps
    // (デザイン規約 §その他の操作). Read off the list's own children: a band bound to nothing would still be counted
    // by the condition that asks for it.
    SampleTimer {
        id: bucketAllTimer
        /// Which bucket is being emptied, and whether the press went in.
        property string from: ""
        property bool pressed: false
        function begin(bucket) {
            bucketAllTimer.from = bucket
            bucketAllTimer.pressed = false
            bucketAllTimer.start()
        }
        onTriggered: {
            if (!bucketAllTimer.pressed) {
                // The heading exists once the list has laid its sections out, which is a frame after the rows arrive.
                if (wipPane.rowAt(0) === null)
                    return
                if (!driver.pressWrite("bucket:" + bucketAllTimer.from,
                                       () => wipPane.moveBucket(bucketAllTimer.from)))
                    return
                bucketAllTimer.pressed = true
                return
            }
            if (!driver.wroteAndSettled())
                return
            // Counts first: the headings are the list's, and trail the model's counts.
            const emptied = bucketAllTimer.from === "staged"
                          ? worktree.stagedCount
                          : bucketAllTimer.from === "conflicts"
                          ? worktree.conflictCount
                          : worktree.unstagedCount + worktree.untrackedCount
            if (emptied !== 0)
                return
            bucketAllTimer.stop()
            // Headings first and together: `must_say` matches one unbroken stretch of the line. `conflicts=` answers
            // the other way — resolving the whole bucket takes its heading off (規約 §その他の操作).
            Harness.report("wip_heads from=" + bucketAllTimer.from
                              + " unstaged=" + wipPane.bucketHeaded("unstaged")
                              + " staged=" + wipPane.bucketHeaded("staged")
                              + " conflicts=" + wipPane.bucketHeaded("conflicts")
                              + " unstaged_count="
                              + (worktree.unstagedCount + worktree.untrackedCount)
                              + " staged_count=" + worktree.stagedCount
                              + " conflict_count=" + worktree.conflictCount)
            renderedBarrier.begin()
        }
    }
    /// The file-row acts, run once the rows they name are walkable (`fileRowsTimer`).
    function runFileRowAct(act, arg) {
        if (act === "stage-many" || act === "stage-many-go") {
            const head = wipPane.rowAt(0)
            if (head)
                wipPane.chooseOnly(head.bucket, head.fullName)
            const mate = wipPane.rowFor(arg)
            // Named before the press: afterwards the list may have taken the delegate away.
            const moved = mate ? mate.bucket + ":" + mate.fullName : ""
            if (mate)
                wipPane.applyClick(mate.bucket, mate.fullName, Qt.ControlModifier)
            Harness.report("chosen count=" + wipPane.chosenCount)
            if (head) {
                wipPane.showStageTools(head.bucket, head.fullName)
                if (act.endsWith("-go")) {
                    const row = wipPane.rowAt(0)
                    if (row)
                        row.stageClicked(head.bucket, head.fullName)
                    driver.treeGoneRow = moved
                }
            }
        } else if (act === "discard-many" || act === "discard-many-go") {
            const first = wipPane.rowAt(0)
            if (first)
                wipPane.chooseOnly(first.bucket, first.fullName)
            const other = wipPane.rowFor(arg)
            // Read while the row is in hand, for the reason `stage-many` gives above.
            const dropped = other ? other.bucket + ":" + other.fullName : ""
            if (other)
                wipPane.applyClick(other.bucket, other.fullName, Qt.ControlModifier)
            Harness.report("chosen count=" + wipPane.chosenCount)
            page.openFileMenu(other ? other.bucket : "unstaged", arg)
            Harness.report("discard_row " + fileDiscardItem.text)
            if (act.endsWith("-go")) {
                fileDiscardItem.completeHold()
                driver.treeGoneRow = dropped
            }
        } else if (act === "file-menu" || act === "file-menu-untracked"
                   || act === "file-menu-staged" || act === "file-menu-conflict") {
            const menuBucket = act === "file-menu" ? "unstaged" : act === "file-menu-staged" ? "staged"
                             : act === "file-menu-conflict" ? "conflicts" : "untracked"
            wipPane.chooseOnly(menuBucket, arg)
            page.openFileMenu(menuBucket, arg)
            if (menuBucket === "conflicts") {
                const row = wipPane.rowFor(arg)
                Harness.report("conflict_kind " + (row ? row.conflictWords() : "-"))
            } else {
                Harness.report("discard_row " + fileDiscardItem.text)
            }
        } else if (act === "take-side-ours" || act === "take-side-theirs") {
            wipPane.chooseOnly("conflicts", arg)
            page.openFileMenu("conflicts", arg)
            fileMenu.close()
            // Counted before the press: a row that is not conflicted takes no side, and a barrier armed anyway waits
            // on a bucket this run never wrote to.
            const taken = fileRowMenu.chosenConflicts().length
            fileRowMenu.takeSideNow(act === "take-side-ours" ? "ours" : "theirs")
            if (taken > 0)
                driver.treeGoneRow = "conflicts:" + arg
        } else if (act === "open-mergetool") {
            // With a tool configured this holds the write queue until it exits, so a demo tool that blocks leaves the
            // wait on screen.
            wipPane.chooseOnly("conflicts", arg)
            page.openFileMenu("conflicts", arg)
            fileMenu.close()
            // `paths=` says whether anything was handed over: a file already resolved (a fixture a previous run
            // consumed) chooses nothing, and `openInMergeTool` then queues no write.
            const handed = fileRowMenu.chosenConflicts().length
            fileRowMenu.openInMergeTool()
            Harness.report("merge_tool " + wipPane.worktree.mergeTool + " paths=" + handed)
            // Only where something was handed over — else the barrier reports a landing nothing here caused.
            if (handed > 0)
                driver.treeGoneRow = "conflicts:" + arg
        } else if (act === "discard-file" || act === "discard-file-go"
                   || act === "delete-file" || act === "delete-file-go"
                   || act === "discard-staged" || act === "discard-staged-go") {
            // The plain verb leaves the menu standing for the shot; "-go" runs the hold to its end.
            const bucket = act.startsWith("delete-file") ? "untracked"
                         : act.startsWith("discard-staged") ? "staged" : "unstaged"
            wipPane.chooseOnly(bucket, arg)
            page.openFileMenu(bucket, arg)
            Harness.report("discard_row " + fileDiscardItem.text)
            if (act.endsWith("-go")) {
                fileDiscardItem.completeHold()
                driver.treeGoneRow = bucket + ":" + arg
            }
        }
    }
    // `rowAt` / `rowFor` read delegates, born a layout after the model has the rows: fired on arrival they answer
    // nothing, and a "-go" has nothing to write. So the act waits for the row it names; a row that never lands
    // leaves the run to the watchdog, which is the diagnosis.
    SampleTimer {
        id: fileRowsTimer
        onTriggered: {
            const act = Harness.autoAct
            if (wipPane.rowFor(Harness.autoActArg) === null)
                return
            if ((act === "stage-many" || act === "stage-many-go"
                 || act === "discard-many" || act === "discard-many-go")
                && wipPane.rowAt(0) === null)
                return
            fileRowsTimer.stop()
            acts.runFileRowAct(act, Harness.autoActArg)
            driver.dispatchFinished()
        }
    }
    /// How many rows the graph holds once the opening's fetch has landed — under `--preset behind` only that fetch
    /// brings the last one in.
    property int openFetchRows: 0
    SampleTimer {
        id: openFetchTimer
        onTriggered: {
            // The rows; the fetch being over, so the count is settled and the button at rest (seeing it turn is
            // optional — it can end before this page exists; verify-ui verbs.md §通信中(リング)と起動直後の狙い方); and
            // `headBehind`: the refs are a feed of their own and land after the rows, and under `--preset behind`
            // only the fetch moves it off zero.
            if (graphModel.rowTotal < acts.openFetchRows || repoTab.autoFetchRunning
                    || branchesModel.headBehind < 1)
                return
            // The fetched row arrives chipless and takes the remote name a pass later. Row zero is that row: the
            // preset opens on a clean tree, so no worktree row stands above it.
            const topChips = GitFacts.chipsShown(graphModel.labelsAt(0), graphModel.goneChips)
            if (topChips.length === 0)
                return
            openFetchTimer.stop()
            Harness.report("open_fetch fails=" + repoTab.fetchFailures
                              + " behind=" + branchesModel.headBehind
                              + " top=" + topChips[0].name
                              + " rows=" + graphModel.rowTotal
                              + " wanted=" + acts.openFetchRows)
            driver.complete()
        }
    }
}
