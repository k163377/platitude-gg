pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// Walking the lists with the arrow keys — the graph, the changed files, the diff — and the folds the walk runs
/// into. Each reads where it arrived.
// `Item` because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    /// `var`: naming its type would be a cycle — the driver is the file that builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var graphModel: driver.graphModel
    readonly property var detailsModel: driver.detailsModel
    readonly property var branchesModel: driver.branchesModel
    readonly property var worktreeModel: driver.worktreeModel
    readonly property var workTree: driver.workTree
    readonly property var graphPane: driver.graphPane
    readonly property var detailsPane: driver.detailsPane
    readonly property var diffPane: driver.diffPane
    readonly property var wipPane: driver.wipPane

    /// Runs `act` if it is one of this family's, and says whether it was.
    function run(act, arg) {
        if (act === "graph-step" || act === "graph-step-edge"
            || act === "graph-step-far" || act === "graph-step-named"
            || act === "graph-step-dirty" || act === "graph-step-diff") {
            // `-edge` walks off the bottom.
            page.activateRow(workTree.branchOid !== "" ? workTree.branchOid : graphModel.oidAt(0))
            graphStepTimer.named = act === "graph-step-named"
            graphStepTimer.dirty = act === "graph-step-dirty"
            graphStepTimer.away = act === "graph-step-far"
            graphStepTimer.diffPath = act === "graph-step-diff" ? arg : ""
            graphStepTimer.steps = act === "graph-step-named" ? 1 : act === "graph-step-dirty" ? 2
                                 : act === "graph-step-edge" ? 10 : act === "graph-step-far" ? 1
                                 : act === "graph-step-diff" ? 1 : arg === "" ? 1 : Number(arg)
            graphStepTimer.start()
        } else if (act === "graph-step-hold") {
            page.activateRow(workTree.branchOid !== "" ? workTree.branchOid : graphModel.oidAt(0))
            graphHoldTimer.steps = arg === "" ? 6 : Number(arg)
            graphHoldTimer.start()
        } else if (act === "changes-step" || act === "changes-step-edge"
                   || act === "wip-step"
                   || act === "changes-shut" || act === "wip-shut") {
            // The argument is the file to start on — `<bucket>:<path>` for the working tree's list, where a file
            // changed on both sides has a row under each.
            if (act === "wip-step" || act === "wip-shut") {
                const cut = arg.indexOf(":")
                const head = cut > 0 ? arg.substring(0, cut) : ""
                const named = head === "staged" || head === "unstaged"
                              || head === "untracked" || head === "conflicts"
                page.showWip()
                fileStepTimer.pane = "wip"
                fileStepTimer.bucket = named ? head : "unstaged"
                fileStepTimer.path = named ? arg.substring(cut + 1) : arg
            } else {
                page.activateRow(workTree.branchOid !== "" ? workTree.branchOid : graphModel.oidAt(0))
                fileStepTimer.pane = "changes"
                fileStepTimer.bucket = ""
                fileStepTimer.path = arg
            }
            fileStepTimer.overrun = act === "changes-step-edge"
            fileStepTimer.shut = act === "changes-shut" || act === "wip-shut"
            fileStepTimer.begin()
        } else if (act === "changes-fold" || act === "changes-unfold") {
            // The argument is the directory as the row is keyed (a lone chain is one row, `a/b/c`), and must be a row
            // the list has built (`itemAtIndex`). The default is the first row, the one a picture can hold: a folder
            // shut below the fold frames like one left open.
            page.activateRow(workTree.branchOid !== "" ? workTree.branchOid : graphModel.oidAt(0))
            // Said outright: a run that inherited the paths view would wait out the watchdog for a folder row.
            detailsModel.setTreeView(true)
            changesFoldTimer.path = arg === "" ? "assets/icons" : arg
            changesFoldTimer.reopen = act === "changes-unfold"
            changesFoldTimer.begin()
        } else if (act === "diff-step" || act === "diff-step-edge") {
            // Moves the view (規約 §diff を上下に送る). Needs the 320x240 seed: no demo diff outgrows a default window,
            // and in the seed two rows lie below the fold — so the plain walk is one row.
            page.showWip()
            page.toggleDiff("untracked", arg, "")
            // The hand walks in by the wheel's door: the diff does not take the keyboard by appearing
            // (規約 §diff のファイル一覧), so without this the arrows are still the file list's.
            diffPane.handArrived()
            diffStepTimer.steps = act === "diff-step-edge" ? 20 : 1
            diffStepTimer.start()
        } else {
            return false
        }
        return true
    }
    // The arrow keys, which no headless run can press: the walk enters where `Keys.onDownPressed` does
    // (`GraphPane.stepRow`) after taking the keyboard as a row click does — a graph nobody pressed hears no arrows
    // (規約 §矢印で履歴を辿る). The wait is for the selected commit's message, which `-dirty` types over.
    SampleTimer {
        id: graphStepTimer
        /// How many rows, and which way. The refusing runs fix their own.
        property int steps: 1
        /// A name box open on the row: the refusing ground a run can stand up (a question on the bar is refused by
        /// the same expression).
        property bool named: false
        /// A half-written message, which refuses nothing: the arrows walk off it and the draft goes, as with a click
        /// (デザイン規約 §コミットメッセージの 2 つの枠). Here so a hold on a draft cannot come back unnoticed.
        property bool dirty: false
        /// The view sent away from the selection before the step, so the row stepped onto has no reading position to
        /// preserve.
        property bool away: false
        /// A diff opened over the graph from CHANGES (a file the selected commit touched): the graph, swapped off
        /// screen, has to let the keyboard go or the arrows walk the selection behind the diff.
        property string diffPath: ""
        onTriggered: {
            if (!driver.cardSettled)
                return
            graphStepTimer.stop()
            if (graphStepTimer.dirty)
                detailsPane.setMessageText("wip: half of a subject", "")
            if (graphStepTimer.named)
                graphPane.startNaming(
                    graphModel.oidAt(graphPane.view.currentIndex))
            // The keyboard before the diff: the diff is what must take it away, and taking it after would press a pane
            // already off screen.
            graphPane.view.takeKeyboard()
            if (graphStepTimer.diffPath !== "")
                page.toggleDiff("commit", graphStepTimer.diffPath, "")
            graphStepWalk.start()
        }
    }
    // A beat between setup and walk: the layout swaps the graph away in its own pass, so a step in the diff's opening
    // tick would still find the pane on screen.
    SampleTimer {
        id: graphStepWalk
        onTriggered: {
            // The diff's rows first, as `chosen_diff` waits: a step the moment the pane arrived leaves the picture and
            // the census holding an empty diff.
            if (graphStepTimer.diffPath !== ""
                    && (!page.diffShown || !diffPane.diffSettled() || diffPane.view.count === 0))
                return
            graphStepWalk.stop()
            if (graphStepTimer.away)
                graphPane.view.contentY = graphPane.view.clampY(Infinity)
            graphStepReport.from = graphPane.view.currentIndex
            graphStepReport.refused = 0
            const way = graphStepTimer.steps < 0 ? -1 : 1
            for (let n = 0; n < Math.abs(graphStepTimer.steps); n++) {
                // Before each step, so `landing=` reads the last one: a walk off the bottom moves the view every row.
                graphStepReport.wasY = graphPane.view.contentY
                if (!graphPane.stepRow(way))
                    graphStepReport.refused++
            }
            graphStepReport.start()
        }
    }
    // Read once the settle behind the walk (`keyStepSettleMs`) has landed: a walk that moved the highlight but never
    // the selection must be told apart from one that did, and both frame alike. Two edges, because a walk reads its
    // start and end rows (`GraphRowWalk.noteStep`): `selected=` is the selection reaching the lit row, `card=` the card
    // reaching the selection — waiting on the first alone photographs a card still holding a row passed through.
    SampleTimer {
        id: graphStepReport
        property int from: -1
        property real wasY: 0
        property int refused: 0
        onTriggered: {
            const row = graphPane.view.currentIndex
            if (row < 0 || (graphStepTimer.diffPath === "" && page.selectedOid
                            !== graphModel.oidAt(row))
                    || !driver.cardSettled)
                return
            graphStepReport.stop()
            Harness.report(
                "graph_step from=" + graphStepReport.from
                + " row=" + row
                + " steps=" + graphStepTimer.steps
                + " landing=" + graphPane.stepLanding(row, graphStepReport.wasY)
                + " back=" + (row === graphStepReport.from)
                + " refused=" + graphStepReport.refused
                + " focused=" + graphPane.view.activeFocus
                + " diff=" + page.diffShown
                + " onscreen=" + graphPane.rowOnScreen(row)
                + " selected=" + (page.selectedOid === graphModel.oidAt(row))
                + " card=" + (detailsModel.shaHex === page.selectedOid))
            driver.complete()
        }
    }
    // The arrow held down: the opening press, then the repeats once its settle has expired, as an OS's first repeat
    // always arrives after it (`GraphRowWalk.noteStep`). `reads=` is the claim — a held walk reads two rows, its start
    // and its end — and no picture holds it.
    SampleTimer {
        id: graphHoldTimer
        /// How many rows the run walks, the opening press included.
        property int steps: 6
        onTriggered: {
            if (!driver.cardSettled)
                return
            graphHoldTimer.stop()
            graphPane.view.takeKeyboard()
            acts.holdReads = 0
            graphHoldReport.from = graphPane.view.currentIndex
            graphHoldReport.refused = 0
            if (!graphPane.stepRow(1))
                graphHoldReport.refused++
            graphHoldRepeat.start()
        }
    }
    // The repeats: waited on the settle being gone (app-ui.md §UI 自動化), then all in one tick — the claim is that a
    // repeat is not read.
    SampleTimer {
        id: graphHoldRepeat
        onTriggered: {
            if (graphPane.stepSettling)
                return
            graphHoldRepeat.stop()
            for (let n = 1; n < graphHoldTimer.steps; n++) {
                if (!graphPane.stepRow(1, true))
                    graphHoldReport.refused++
            }
            graphHoldReport.start()
        }
    }
    // The hand off the key: the selection landed and the card caught up — the pair `graph_step` waits on.
    SampleTimer {
        id: graphHoldReport
        property int from: -1
        property int refused: 0
        onTriggered: {
            const row = graphPane.view.currentIndex
            if (row < 0 || graphPane.stepSettling
                    || page.selectedOid !== graphModel.oidAt(row) || !driver.cardSettled)
                return
            graphHoldReport.stop()
            Harness.report(
                "graph_hold from=" + graphHoldReport.from
                + " row=" + row
                + " steps=" + graphHoldTimer.steps
                + " reads=" + acts.holdReads
                + " refused=" + graphHoldReport.refused
                + " selected=" + (page.selectedOid === graphModel.oidAt(row))
                + " card=" + (detailsModel.shaHex === page.selectedOid))
            driver.complete()
        }
    }
    /// Commits the walk has asked for since the hold set off, counted on the signal: the asks come and go inside a beat
    /// (app-ui.md §UI 自動化).
    property int holdReads: 0
    Connections {
        target: driver.graphPane
        function onRowActivated(oidHex) { acts.holdReads++ }
    }
    // The file list's arrows: the light and the diff move together, one file per press (規約 §diff のファイル一覧).
    // The click goes in at the row itself — its handler hands the list the keyboard, so calling past it leaves
    // `focused=` proving nothing — and the step at `stepFile`, where `Keys.onDownPressed` enters. The diff opens
    // without taking the keyboard, so an arrow still belongs to the list.
    SampleTimer {
        id: fileStepTimer
        /// Which list (`changes` / `wip`) and how far. `overrun` (`-edge`) walks past the end: the count is only known
        /// once the details arrive, so it cannot be set up here.
        property string pane: "changes"
        property int steps: 1
        property bool overrun: false
        /// The file clicked, and the bucket its row sits in (empty for the commit's list, whose files sit in none).
        property string bucket: ""
        property string path: ""
        property bool clicked: false
        property bool stopped: false
        /// `-shut`: a second click on the same row shuts the diff (`RepoPage.toggleDiff`) and the run stops there,
        /// both lists lighting nothing — the two verbs are read as a pair (デザイン規約 §diff のファイル一覧).
        property bool shut: false
        property bool closed: false
        function begin() {
            fileStepTimer.clicked = false
            fileStepTimer.closed = false
            fileStepTimer.stopped = false
            fileStepTimer.steps = 1
            fileStepTimer.start()
        }
        /// The click, through the row itself (twice for `-shut`).
        function strike() {
            const row = fileStepTimer.walk.rowFor(fileStepTimer.bucket, fileStepTimer.path)
            if (!row)
                return false
            if (fileStepTimer.pane === "wip")
                row.fileClicked(fileStepTimer.bucket, fileStepTimer.path,
                                worktreeModel.origOf(fileStepTimer.path),
                                Qt.NoModifier)
            else
                // The row's own press, with its own reading of the model: handed the path from beside it, this goes
                // green on a row that reads nothing (verify-ui §壊れない動詞の実装と反復).
                row.press()
            return true
        }
        function walkNow() {
            fileStepTimer.stop()
            if (fileStepTimer.overrun)
                fileStepTimer.steps = fileStepTimer.walk.view.count + 5
            const way = fileStepTimer.steps < 0 ? -1 : 1
            for (let n = 0; n < Math.abs(fileStepTimer.steps); n++) {
                if (!fileStepTimer.walk.stepFile(way))
                    fileStepTimer.stopped = true
            }
            fileStepReport.start()
        }
        readonly property var walk:
            fileStepTimer.pane === "wip" ? wipPane.filesWalk : detailsPane.filesWalk
        onTriggered: {
            if (!fileStepTimer.clicked) {
                if (!fileStepTimer.strike())
                    return
                fileStepTimer.clicked = true
                return
            }
            // The click has to have landed: a walk with nothing being read is refused, and reading that as "the end"
            // would go green on a click that never arrived.
            if (page.diffShown && page.diffPath === fileStepTimer.path) {
                if (!fileStepTimer.shut) {
                    fileStepTimer.walkNow()
                    return
                }
                // The second click once the read has landed — shutting a diff still coming would test a race.
                if (!fileStepTimer.closed && diffPane.diffSettled())
                    fileStepTimer.closed = fileStepTimer.strike()
                return
            }
            if (fileStepTimer.shut && fileStepTimer.closed && !page.diffShown) {
                fileStepTimer.stop()
                fileShutReport.start()
            }
        }
    }
    // `lit=` is the claim, read off the rectangles (`litPath`) so a light only ever in the model says false;
    // `open=false` makes the answer that list's own.
    SampleTimer {
        id: fileShutReport
        onTriggered: {
            if (page.diffShown)
                return
            fileShutReport.stop()
            const walk = fileStepTimer.walk
            // `open` / `lit` / `focused` stay together and in this order: a `must_say` is one stretch of the line
            // (`verify::verbs`).
            Harness.report(
                "file_shut path=" + fileStepTimer.path
                + " pane=" + fileStepTimer.pane
                + " open=" + page.diffShown
                + " lit=" + (walk.litPath() !== "")
                + " focused=" + walk.view.activeFocus)
            driver.complete()
        }
    }
    // The light runs at the key's rate and the diff catches up after the settle (`keyStepSettleMs`), so a walk that
    // moved the light but never the diff must be told apart from one that did (規約 §diff のファイル一覧).
    SampleTimer {
        id: fileStepReport
        onTriggered: {
            const walk = fileStepTimer.walk
            // The landed diff asked for, arrived (a pane still reading photographs empty), and its lit row built.
            if (!page.diffShown || page.diffPath === fileStepTimer.path || !diffPane.diffSettled()
                    || walk.litPath() === "")
                return
            fileStepReport.stop()
            Harness.report(
                "file_step pane=" + fileStepTimer.pane
                + " from=" + fileStepTimer.path
                + " steps=" + fileStepTimer.steps
                + " read=" + (page.diffKind + ":" + page.diffPath)
                + " moved=" + (page.diffPath !== fileStepTimer.path)
                + " stopped=" + fileStepTimer.stopped
                + " lit=" + (walk.litPath() === page.diffPath)
                + " focused=" + walk.view.activeFocus)
            driver.complete()
        }
    }
    // A folder row of CHANGES struck shut, and open again for `-unfold`, by the row's own signal — the pane's handler
    // carries it to the model, so calling past it proves nothing. The two lists that draw a fold arrow keep the answer
    // in different fields (`NameCell.folded`), so `turn=` is read off the icon: reading the flag back would go green
    // with the arrow unwired.
    SampleTimer {
        id: changesFoldTimer
        /// The directory row struck, and whether a second strike reopens it. Read as a pair: one arrow alone says
        /// nothing about which way it turned.
        property string path: ""
        property bool reopen: false
        /// Strikes gone out. The row is shut exactly when this is odd, which is the wait between strikes: the toggle
        /// rebuilds the list, so a later tick's row is a new row.
        property int struck: 0
        function begin() {
            changesFoldTimer.struck = 0
            changesFoldTimer.start()
        }
        /// Walked, because `FileRowWalk.rowFor` answers by `walkKey`, which a folder deliberately lacks.
        function folderRow() {
            const view = detailsPane.filesWalk.view
            for (let i = 0; i < view.count; i++) {
                const row = view.itemAtIndex(i)
                if (row && row.isFolder && row.pathText === changesFoldTimer.path)
                    return row
            }
            return null
        }
        onTriggered: {
            const row = changesFoldTimer.folderRow()
            if (!row)
                return
            const strikes = changesFoldTimer.reopen ? 2 : 1
            if (row.isFolded !== (changesFoldTimer.struck % 2 === 1))
                return
            if (changesFoldTimer.struck < strikes) {
                // The commit's own files first, read only on this branch: a read landing after a strike clears every
                // fold choice (`DetailsModel::take_files`), and the row swings open under a wait that never ends. Read
                // every tick, the verb's own answer would break its own precondition.
                if (detailsModel.loading || detailsModel.shaHex !== page.selectedOid)
                    return
                changesFoldTimer.struck++
                row.folderToggled(row.pathText)
                return
            }
            changesFoldTimer.stop()
            Harness.report(
                "changes_fold path=" + changesFoldTimer.path
                + " strikes=" + changesFoldTimer.struck
                + " shut=" + row.isFolded
                + " turn=" + row.foldTurn
                + " rows=" + detailsPane.filesWalk.view.count
                + " tree=" + detailsModel.treeView)
            driver.complete()
        }
    }
    // The diff's arrows enter at `DiffPane.stepRows`, where `Keys.onDownPressed` does. The wait is for the view, not
    // only the rows (`diffSettled()`): a list whose `contentHeight` is still 0 clamps every step, reading like a diff
    // with nothing to scroll — so it waits for `atEnd` false over a laid-out height
    // (rules-refs/app-ui.md「『まだ答えが無い』と値 0 / false を分ける」).
    SampleTimer {
        id: diffStepTimer
        property int steps: 1
        onTriggered: {
            if (!page.diffShown || !diffPane.diffSettled() || diffPane.view.height <= 0 || diffPane.atEnd)
                return
            diffStepTimer.stop()
            diffStepReport.from = acts.diffRow()
            diffStepReport.stopped = false
            const way = diffStepTimer.steps < 0 ? -1 : 1
            for (let n = 0; n < Math.abs(diffStepTimer.steps); n++) {
                // A step that moved nothing is the end answering. Read beside `atEnd=`: a walk refused throughout (the
                // pane never on screen) also leaves the view at row 0, like an unscrollable diff.
                if (!diffPane.stepRows(way))
                    diffStepReport.stopped = true
            }
            diffStepReport.start()
        }
    }
    SampleTimer {
        id: diffStepReport
        property int from: -1
        property bool stopped: false
        onTriggered: {
            if (!diffPane.diffSettled())
                return
            diffStepReport.stop()
            Harness.report(
            "diff_step from=" + diffStepReport.from
            + " rows=" + acts.diffRow()
            + " steps=" + diffStepTimer.steps
            + " moved=" + (acts.diffRow() !== diffStepReport.from)
            + " atEnd=" + diffPane.atEnd
            + " stopped=" + diffStepReport.stopped
            + " focused=" + diffPane.view.activeFocus)
            driver.complete()
        }
    }
    /// Where the diff's view stands, in rows — the walk's unit, steadier than pixels on a report line.
    function diffRow() {
        return Math.round(diffPane.view.contentY / Theme.rowHeight)
    }
}
