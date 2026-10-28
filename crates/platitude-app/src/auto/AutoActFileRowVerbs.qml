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

    /// Runs `act` if it is one of this family's, and says whether it was.
    function run(act, arg) {
        if (act === "stage-all" || act === "unstage-all" || act === "resolve-all") {
            page.showWip()
            bucketAllTimer.from = act === "stage-all" ? "unstaged" : act === "unstage-all" ? "staged" : "conflicts"
            bucketAllTimer.start()
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
    PageStage {
        id: bucketAllTimer
        name: "bucket_all"
        order: ["pressed"]
        /// Which bucket is being emptied.
        property string from: ""
        /// What is left in that bucket, by the model's counts.
        function left() {
            return bucketAllTimer.from === "staged"
                   ? worktree.stagedCount
                   : bucketAllTimer.from === "conflicts"
                   ? worktree.conflictCount
                   : worktree.unstagedCount + worktree.untrackedCount
        }
        onTriggered: {
            if (bucketAllTimer.at("start")) {
                // The heading exists once the list has laid its sections out, which is a frame after the rows arrive.
                if (!Awaited.all("bucket_all_heading", { "heading": wipPane.rowAt(0) !== null }))
                    return
                bucketAllTimer.input("pressed", () => driver.pressWrite("bucket:" + bucketAllTimer.from,
                                                                        () => wipPane.moveBucket(bucketAllTimer.from)))
                return
            }
            // Counts first: the headings are the list's, and trail the model's counts.
            if (!Awaited.all("bucket_all_emptied", { "written": driver.wroteAndSettled(),
                                                     "emptied": bucketAllTimer.left() === 0 }))
                return
            // Headings first and together: `must_say` matches one unbroken stretch of the line. `conflicts=` answers
            // the other way — resolving the whole bucket takes its heading off (規約 §その他の操作).
            bucketAllTimer.complete(() => "wip_heads from=" + bucketAllTimer.from
                                           + " unstaged=" + wipPane.bucketHeaded("unstaged")
                                           + " staged=" + wipPane.bucketHeaded("staged")
                                           + " conflicts=" + wipPane.bucketHeaded("conflicts")
                                           + " unstaged_count="
                                           + (worktree.unstagedCount + worktree.untrackedCount)
                                           + " staged_count=" + worktree.stagedCount
                                           + " conflict_count=" + worktree.conflictCount)
        }
    }
    /// The file-row acts, run once the rows they name are walkable (`fileRowsTimer`). Each line is read where the
    /// gesture stands as it is said here: a `-go` form's own press takes the chosen rows, and the menu's words with
    /// them, before any completion reads the report.
    function runFileRowAct(act, arg) {
        const lines = []
        if (act === "stage-many" || act === "stage-many-go") {
            const head = wipPane.rowAt(0)
            if (head)
                wipPane.chooseOnly(head.bucket, head.fullName)
            const mate = wipPane.rowFor(arg)
            // Named before the press: afterwards the list may have taken the delegate away.
            const moved = mate ? mate.bucket + ":" + mate.fullName : ""
            if (mate)
                wipPane.applyClick(mate.bucket, mate.fullName, Qt.ControlModifier)
            lines.push("chosen count=" + wipPane.chosenCount)
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
            lines.push("chosen count=" + wipPane.chosenCount)
            page.openFileMenu(other ? other.bucket : "unstaged", arg)
            lines.push("discard_row " + fileDiscardItem.text)
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
                lines.push("conflict_kind " + (row ? row.conflictWords() : "-"))
            } else {
                lines.push("discard_row " + fileDiscardItem.text)
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
            lines.push("merge_tool " + wipPane.worktree.mergeTool + " paths=" + handed)
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
            lines.push("discard_row " + fileDiscardItem.text)
            if (act.endsWith("-go")) {
                fileDiscardItem.completeHold()
                driver.treeGoneRow = bucket + ":" + arg
            }
        }
        driver.reportAtCompletion(() => lines)
    }
    // `rowAt` / `rowFor` read delegates, born a layout after the model has the rows: fired on arrival they answer
    // nothing, and a "-go" has nothing to write. So the act waits for the row it names; a row that never lands
    // leaves the run to the watchdog, which is the diagnosis.
    PageStage {
        id: fileRowsTimer
        name: "file_rows"
        order: ["dispatched"]
        onTriggered: {
            const act = Harness.autoAct
            const many = (act === "stage-many" || act === "stage-many-go"
                          || act === "discard-many" || act === "discard-many-go")
            if (!Awaited.all("file_rows_ready", { "row": wipPane.rowFor(Harness.autoActArg) !== null,
                                                  "head": !many || wipPane.rowAt(0) !== null }))
                return
            fileRowsTimer.advance("dispatched")
            acts.runFileRowAct(act, Harness.autoActArg)
            driver.dispatchFinished(fileRowsTimer)
            // A verb that completed here stopped this already.
            fileRowsTimer.stop()
        }
    }
    /// How many rows the graph holds once the opening's fetch has landed — under `--preset behind` only that fetch
    /// brings the last one in.
    property int openFetchRows: 0
    PageStage {
        id: openFetchTimer
        name: "open_fetch"
        /// The chips the top row shows.
        function topChips() {
            return GitFacts.chipsShown(graphModel.labelsAt(0), graphModel.goneChips)
        }
        onTriggered: {
            // The rows; the fetch being over, so the count is settled and the button at rest (seeing it turn is
            // optional — it can end before this page exists; verify-ui verbs.md §通信中(リング)と起動直後の狙い方); and
            // `headBehind`: the refs are a feed of their own and land after the rows, and under `--preset behind`
            // only the fetch moves it off zero. The fetched row arrives chipless and takes the remote name a pass
            // later. Row zero is that row: the preset opens on a clean tree, so no worktree row stands above it.
            if (!Awaited.all("open_fetch_ready", {
                "ready": !(graphModel.rowTotal < acts.openFetchRows || repoTab.autoFetchRunning
                    || branchesModel.headBehind < 1),
                "chips": openFetchTimer.topChips().length > 0 }))
                return
            openFetchTimer.complete(() => "open_fetch fails=" + repoTab.fetchFailures
                                           + " behind=" + branchesModel.headBehind
                                           + " top=" + openFetchTimer.topChips()[0].name
                                           + " rows=" + graphModel.rowTotal
                                           + " wanted=" + acts.openFetchRows)
        }
    }
}
