pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The file rows of the working tree: the menu a row opens, the whole-bucket moves, and the reads an opening
/// fires. Every one of these has to find its row before it can act, which is what the walk below waits for.
///
/// Built by `AutoActDriver`, which `RepoPage` builds only when a verb was given. What these verbs act on
/// hangs off that driver; the names it owns are
/// read back once below so the verbs can name them bare.
// An `Item` only because `QtObject` has no default property to hold the timers below; it is a
// sizeless holder.
Item {
    id: acts

    /// The driver these verbs belong to. `var` because naming its type here would be a circle: it is
    /// the file that builds this one.
    required property var driver

    // The driver's own names, read once so the verbs can name them bare.
    readonly property var page: driver.page
    readonly property var repoTab: driver.repoTab
    readonly property var workTree: driver.workTree
    readonly property var graphModel: driver.graphModel
    readonly property var branchesModel: driver.branchesModel
    readonly property var wipPane: driver.wipPane
    readonly property var fileRowMenu: driver.fileRowMenu
    readonly property var fileMenu: driver.fileMenu
    readonly property var fileDiscardItem: driver.fileDiscardItem
    readonly property var renderedBarrier: driver.barrierRendered

    /// Runs `act` if it is one of this family's, and says whether it was. The families are asked in turn
    /// and the first to know a verb runs it — each verb is named by one (`AutoActDriver`).
    function run(act, arg) {
        if (act === "stage-all" || act === "unstage-all" || act === "resolve-all") {
            page.showWip()
            bucketAllTimer.begin(act === "stage-all" ? "unstaged"
                                 : act === "unstage-all" ? "staged" : "conflicts")
        } else if (driver.fileRowActs.indexOf(act) >= 0) {
            // Rows first: every one of these names a row of the WIP lists, and the walk that
            // resolves a name reads delegates (`fileRowsTimer`, which then runs `runFileRowAct`).
            page.showWip()
            fileRowsTimer.start()
        } else if (act === "open-fetches") {
            // The fetch the opening fires is the whole verb, and the argument is how many rows the graph holds
            // once it has landed.
            acts.openFetchRows = Math.max(1, Number(arg))
            openFetchTimer.start()
        } else {
            return false
        }
        return true
    }
    // Emptying one whole bucket from its own heading, and reading back which headings the list is left with. The two
    // directions are one verb because the claim is that they are symmetrical: a bucket that has just been emptied keeps
    // its heading, whichever bucket it was (デザイン規約 §その他の操作).
    //
    // The heading itself is pressed, and what is read back is the list's own children — a band bound to nothing
    // would still be counted by the condition that asks for it.
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
            // The bucket that was emptied has to be empty before its heading means anything: the counts are the model's
            // answer and the headings are the list's, and reading the second before the first would report the state
            // that was.
            const emptied = bucketAllTimer.from === "staged"
                          ? workTree.stagedCount
                          : bucketAllTimer.from === "conflicts"
                          ? workTree.conflictCount
                          : workTree.unstagedCount + workTree.untrackedCount
            if (emptied !== 0)
                return
            bucketAllTimer.stop()
            // The three headings stand together at the front of the line, ahead of the counts: what a press claims is
            // about the headings side by side, and the judgement reads one unbroken stretch of the line
            // (`Outcome::must_say`) — a count in between would split the claim in two. `conflicts=` is the one that
            // answers the opposite way: that bucket comes and goes with git's own state, so marking the whole of it
            // resolved has to take its heading off the screen (規約 §その他の操作).
            Harness.report("wip_heads from=" + bucketAllTimer.from
                              + " unstaged=" + wipPane.bucketHeaded("unstaged")
                              + " staged=" + wipPane.bucketHeaded("staged")
                              + " conflicts=" + wipPane.bucketHeaded("conflicts")
                              + " unstaged_count="
                              + (workTree.unstagedCount + workTree.untrackedCount)
                              + " staged_count=" + workTree.stagedCount
                              + " conflict_count=" + workTree.conflictCount)
            renderedBarrier.begin()
        }
    }
    /// The file-row acts, run once the rows they name are walkable (`fileRowsTimer` holds them
    /// until then; `runAutoAct` has already put the WIP pane up).
    function runFileRowAct(act, arg) {
        if (act === "stage-many" || act === "stage-many-go") {
            const head = wipPane.rowAt(0)
            if (head)
                wipPane.chooseOnly(head.bucket, head.fullName)
            const mate = wipPane.rowFor(arg)
            // Named off the row while it is still in hand: the barrier waits on a bucket and a path, and a delegate
            // read back after the press is one the list has had a chance to take away.
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
            // Read before the press, for the reason the merge editor below gives: a row that is not conflicted takes
            // no side, and a barrier armed anyway waits on a bucket this run never wrote to.
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
            // The name comes from config and the paths from the choice, so the name alone cannot say whether
            // anything was handed over. A file that is already resolved — a fixture a previous run consumed —
            // chooses nothing, and `openInMergeTool` then returns without queueing a write, leaving the run to
            // the watchdog with the tool's name reported all the same. The count is what tells the two apart.
            const handed = fileRowMenu.chosenConflicts().length
            fileRowMenu.openInMergeTool()
            Harness.report("merge_tool " + wipPane.workTree.mergeTool + " paths=" + handed)
            // Named only where something was actually handed over: a fixture a previous run consumed queues no write
            // at all, and a barrier waiting for a row that left the bucket before this run began would report a
            // landing nothing here caused.
            if (handed > 0)
                driver.treeGoneRow = "conflicts:" + arg
        } else if (act === "discard-file" || act === "discard-file-go"
                   || act === "delete-file" || act === "delete-file-go"
                   || act === "discard-staged" || act === "discard-staged-go") {
            // Which row follows the verb: "delete-file" an untracked one, "discard-staged" the staged side, otherwise
            // the unstaged one. The plain verb leaves the menu standing for the shot; "-go" runs the hold to its end.
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
    // Every file-row act resolves the rows it names through the pane's walk — `rowAt` / `rowFor` /
    // `chosenRows` — and the walk reads delegates, which are born a layout after the model has the
    // rows. Fired on arrival the walk answers nothing: the choice stays empty, the menu opens over
    // it with an empty discard row, and a "-go" with nothing to write leaves the run to the
    // watchdog (measured, Windows wedged this way while the same build walked on Linux).
    // So the acting waits for the row it is about to name, the way `bucketAllTimer` waits for the
    // headings; a row that never lands leaves the run to the watchdog, which is the diagnosis.
    SampleTimer {
        id: fileRowsTimer
        onTriggered: {
            const act = Harness.autoAct
            // The named row has to be walkable — and for the pairs that start from the head row,
            // that row too. One walk answering is every walk answering: they read the same
            // delegates.
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
    /// Automation: how many rows the graph holds once the fetch the opening fired has landed. Its commit is one only
    /// the remote had (`--preset behind`), so a graph that reaches this many rows without anything being pressed is
    /// the fetch itself, said in the only place a headless run can read it.
    property int openFetchRows: 0
    SampleTimer {
        id: openFetchTimer
        onTriggered: {
            // A state to sample: rows only reach the count after the fetch has landed and the graph has been rebuilt
            // over it, and a run where that never happens has nothing to report.
            //
            // The tab's own word for "the fetch is over" is waited for as well, so the count read below is the settled
            // one and the picture holds a button at rest. Seeing it turn is optional: the fetch can be over
            // before this page exists (§通信中(リング)と起動直後の狙い方).
            //
            // The refs the fetch brought back are a feed of their own and land after the rows do, so the count alone
            // would picture a graph that has fetched beside a sidebar that has not. `headBehind` is the sidebar's end
            // of that feed, and under `--preset behind` only the fetch can move it off zero.
            if (graphModel.rowTotal < acts.openFetchRows || repoTab.autoFetchRunning
                    || branchesModel.headBehind < 1)
                return
            // The graph is the other side of that same feed, and it is answered a pass later still: the walk that
            // added the row the fetch brought in was drawn over the refs as they stood, so that row arrives wearing
            // no chip at all and is given the remote name once the listing is in. Row zero is that row: the preset
            // opens on a clean tree, so no working-tree row stands above what the fetch brought in.
            const topChips = GitFacts.labelsShown(graphModel.labelsAt(0), graphModel.goneChips)
            if (topChips === "")
                return
            openFetchTimer.stop()
            Harness.report("open_fetch fails=" + repoTab.fetchFailures
                              + " behind=" + branchesModel.headBehind
                              + " top=" + GitFacts.recordName(topChips.split(String.fromCharCode(31))[0])
                              + " rows=" + graphModel.rowTotal
                              + " wanted=" + acts.openFetchRows)
            driver.complete()
        }
    }
}
