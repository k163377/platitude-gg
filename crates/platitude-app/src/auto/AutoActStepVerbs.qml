pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// Walking the lists with the arrow keys — the graph, the changed files, the diff — and the folds the walk runs
/// into. Each of these reads where it arrived rather than how far it asked to go.
///
/// Built by `AutoActDriver`, which `RepoPage` builds only when a verb was given. What these verbs act on
/// hangs off that driver; the names it owns are
/// read back once below so the verbs can name them bare.
// An `Item` only because `QtObject` has no default property to hold the timers below; it draws nothing
// and is never given a size.
Item {
    id: acts

    /// The driver these verbs belong to. `var` because naming its type here would be a circle: it is
    /// the file that builds this one.
    required property var driver

    // The driver's own names, read once so the verbs can name them bare.
    readonly property var page: driver.page
    readonly property var graphModel: driver.graphModel
    readonly property var detailsModel: driver.detailsModel
    readonly property var branchesModel: driver.branchesModel
    readonly property var worktreeModel: driver.worktreeModel
    readonly property var graphPane: driver.graphPane
    readonly property var detailsPane: driver.detailsPane
    readonly property var diffPane: driver.diffPane
    readonly property var wipPane: driver.wipPane

    /// Runs `act` if it is one of this family's, and says whether it was. The families are asked in turn
    /// and the first to know a verb runs it — no verb is named by two of them (`AutoActDriver`).
    function run(act, arg) {
        if (act === "graph-step" || act === "graph-step-edge"
            || act === "graph-step-far" || act === "graph-step-named"
            || act === "graph-step-dirty" || act === "graph-step-diff") {
            // Keystrokes cannot be injected, so the run enters at the same `stepRow` the key handler enters — and takes
            // the keyboard first through the same call a row click makes, since a graph nobody has pressed hears no
            // arrows at all (規約 §矢印で履歴を辿る). `-dirty` writes a half-written message and then walks anyway —
            // nothing holds the selection for a draft. `-edge` walks off the bottom; `-far` sends the view away first
            // so the stepped-off row is off screen. `-diff` opens a file over the graph: the pane swapped off screen
            // has to let the keyboard go, or the arrows walk the selection behind the diff.
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
            // The same door with the key held down: every step behind the first says the key was already down. What it
            // proves is `reads=` — the settle cannot tell a repeat from a press on its own (`GraphRowWalk.noteStep`),
            // and this is the run where it would get it wrong.
            page.activateRow(workTree.branchOid !== "" ? workTree.branchOid : graphModel.oidAt(0))
            graphHoldTimer.steps = arg === "" ? 6 : Number(arg)
            graphHoldTimer.start()
        } else if (act === "changes-step" || act === "changes-step-edge"
                   || act === "wip-step") {
            // The file list's arrows: one file per press, the light and the diff moving together (規約 §diff のファイル一覧).
            // `-edge` walks further than the list is long, so the last presses are refused and it stops rather than
            // wrapping. The argument is the file to start on — `<bucket>:<path>` for the working tree's list, where a
            // file changed on both sides has a row under each.
            if (act === "wip-step") {
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
            fileStepTimer.begin()
        } else if (act === "changes-fold" || act === "changes-unfold") {
            // The commit's CHANGES tree opened and shut by its folder rows. The argument is the directory, written the
            // way the row is keyed — a chain with nothing beside it is one row and one key (`a/b/c`) — and it has to
            // name a row the list has actually built, since `itemAtIndex` answers for no other. The default is the
            // first row of the list, which is also the one the picture can hold: a folder struck shut below the fold
            // frames exactly like one left open.
            page.activateRow(workTree.branchOid !== "" ? workTree.branchOid : graphModel.oidAt(0))
            // Said rather than assumed: the tree is the list's resting look, but a run that inherited the paths view
            // would wait out the watchdog looking for a folder row that flat paths never put there.
            detailsModel.setTreeView(true)
            changesFoldTimer.path = arg === "" ? "assets/icons" : arg
            changesFoldTimer.reopen = act === "changes-unfold"
            changesFoldTimer.begin()
        } else if (act === "diff-step" || act === "diff-step-edge") {
            // Moves the view, not a selection (規約 §diff を上下に送る). Rides the 320x240 seed: no demo file's diff is longer
            // than a default window, and even there the room below the fold is two rows (measured) — which is why
            // the plain walk is one row.
            page.showWip()
            page.toggleDiff("untracked", arg, "")
            // The hand walks into the pane, through the same door the wheel comes in by: the diff does not take the
            // keyboard by appearing (規約 §diff のファイル一覧), so without this the arrows are still the file list's.
            diffPane.handArrived()
            diffStepTimer.steps = act === "diff-step-edge" ? 20 : 1
            diffStepTimer.start()
        } else {
            return false
        }
        return true
    }
    // The arrow keys, which no headless run can press: the walk enters where `Keys.onDownPressed` enters
    // (`GraphPane.stepRow`) after taking the keyboard the way a row click takes it. The selected commit's message has
    // to have arrived before it can be typed over, which is what the wait is for — the same one the reword verbs keep.
    SampleTimer {
        id: graphStepTimer
        /// How many rows, and which way. The refusing runs fix their own.
        property int steps: 1
        /// The one ground a step is refused on that a run can stand up: a name box open on the row. (The other — a
        /// question standing on the bar — is refused by the same expression, and its pill holds the keyboard anyway.)
        property bool named: false
        /// A half-written message in the details pane, which refuses **nothing** — the arrows walk off it and the
        /// draft goes, the same as a click (デザイン規約 §コミットメッセージの 2 つの枠). Here so a hold cannot come
        /// back unnoticed.
        property bool dirty: false
        /// The view sent away from the selection before the step, so the row stepped onto has no reading position to
        /// preserve.
        property bool away: false
        /// The third refusing ground, and the one that was reported: a diff opened over the graph from CHANGES. The
        /// path is the argument — the file has to be one the selected commit touched.
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
            // The press that says the keyboard works here comes first, because the diff below is what has to take it
            // away again: a run that opened the diff and only then reached for the keyboard would be proving nothing
            // (it would be pressing on a pane that is no longer on the screen).
            graphPane.view.takeKeyboard()
            if (graphStepTimer.diffPath !== "")
                page.toggleDiff("commit", graphStepTimer.diffPath, "")
            graphStepWalk.start()
        }
    }
    // A beat between the setup and the walk: the layout swaps the graph away in its own pass, so a step taken in the
    // same tick as the diff opened would still find the pane on screen.
    SampleTimer {
        id: graphStepWalk
        onTriggered: {
            if (graphStepTimer.diffPath !== "" && !page.diffShown)
                return
            graphStepWalk.stop()
            if (graphStepTimer.away)
                graphPane.view.contentY = graphPane.view.clampY(Infinity)
            graphStepReport.from = graphPane.view.currentIndex
            graphStepReport.refused = 0
            const way = graphStepTimer.steps < 0 ? -1 : 1
            for (let n = 0; n < Math.abs(graphStepTimer.steps); n++) {
                // Where the view stood before each step, so what is read is how the last one landed: a walk that runs
                // off the bottom moves the view once per row from there on.
                graphStepReport.wasY = graphPane.view.contentY
                if (!graphPane.stepRow(way))
                    graphStepReport.refused++
            }
            graphStepReport.start()
        }
    }
    // Longer than the settle behind the walk (`keyStepSettleMs`), so what is read is the reading a hand coming off the
    // key would get: a run that moved the highlight and never landed the selection has to be told apart from one that
    // did, and both frame alike from the waist down.
    //
    // Two edges, because a walk asks twice: a held arrow is read where it set off and again where it stopped
    // (`GraphRowWalk.noteStep`), so the first row's details are still in flight when the last row's request goes out.
    // `selected=` is the selection reaching the lit row, `card=` the pane on the right reaching the selection —
    // waiting the first out alone photographs the highlight on the row the walk stopped on beside a card still
    // holding one it passed through (observed on Windows), the wait `file_step` keeps on the diff side.
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
    // The arrow held down, which a run of steps taken inside one tick cannot be: the settle behind the opening press
    // expires long before any OS sends its first repeat, so the run has to wait it out before the rest of the steps
    // arrive with the key still down (`GraphRowWalk.noteStep`). `reads=` is the whole of the report — a walk asks for
    // a commit twice, at the row it set off from and at the row it stopped on — and no picture holds it: a run that
    // read every row it passed through frames exactly like one that read two.
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
    // The repeats. Waited on the settle being gone rather than on a count of beats (app-ui.md §UI 自動化の因果性) —
    // that is the edge a real keyboard's first repeat always arrives behind. They go in one tick once it has: what is
    // being proven is that a repeat is not read, not how fast one arrives.
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
    // The hand off the key: the settle behind the last repeat has landed the selection and the card has caught up to
    // it — the same pair `graph_step` waits out, and here also what says the run is over.
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
    /// How many commits the walk has asked for since the hold verb set off. Taken on the signal rather than sampled:
    /// the asks this verb is about are the ones that come and go inside a beat (app-ui.md §UI 自動化の因果性).
    property int holdReads: 0
    Connections {
        target: driver.graphPane
        function onRowActivated(oidHex) { acts.holdReads++ }
    }
    // The file list's arrows: the light and the diff move together, one file per press (規約 §diff のファイル一覧). Two things
    // have to be real for this to say anything, so both go through the door a hand goes through:
    //
    // - the click. The row's own signal is raised by name, not the pane's handler — the handler is where the keyboard
    // is handed to the list, and calling past it would leave `focused=` proving nothing (the same reason `nav-peek`
    // strikes the cell and not `SidebarPane`). - the step, which enters at `stepFile` where `Keys.onDownPressed`
    // enters. A keystroke cannot be injected (verify-ui).
    //
    // Nothing here reaches for the keyboard, and that is the point: the diff opened without taking it, so an arrow
    // still belongs to the list.
    SampleTimer {
        id: fileStepTimer
        /// Which list, `changes` or `wip`, and how far to walk. `overrun` asks for more files than the list holds,
        /// which is how the end it stops at is reached — the count is only known once the commit's details have
        /// arrived, so it cannot be a number set up here.
        property string pane: "changes"
        property int steps: 1
        property bool overrun: false
        /// The file clicked, and the bucket its row sits in (empty for the commit's list, whose files sit in none).
        property string bucket: ""
        property string path: ""
        /// Whether the click has gone out, so the tick that follows is waiting for the diff rather than for the row.
        property bool clicked: false
        property bool stopped: false
        function begin() {
            fileStepTimer.clicked = false
            fileStepTimer.stopped = false
            fileStepTimer.steps = 1
            fileStepTimer.start()
        }
        readonly property var walk:
            fileStepTimer.pane === "wip" ? wipPane.filesWalk : detailsPane.filesWalk
        onTriggered: {
            if (!fileStepTimer.clicked) {
                const row = fileStepTimer.walk.rowFor(fileStepTimer.bucket,
                                                      fileStepTimer.path)
                if (!row)
                    return
                fileStepTimer.clicked = true
                if (fileStepTimer.pane === "wip")
                    row.fileClicked(fileStepTimer.bucket, fileStepTimer.path,
                                    worktreeModel.origOf(fileStepTimer.path),
                                    Qt.NoModifier)
                else
                    row.activated("", fileStepTimer.path,
                                  detailsModel.origOf(fileStepTimer.path))
                return
            }
            // The click has to have landed before a step means anything: a walk with nothing being read is refused, and
            // reading that as "the end" would go green on a click that never arrived.
            if (!page.diffShown || page.diffPath !== fileStepTimer.path)
                return
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
    }
    // Longer than the settle behind the walk (`keyStepSettleMs`), because what is read is the reading a hand coming off
    // the key gets: the light runs at the key's rate and the diff catches up after it, so a run that moved the light
    // and never moved the diff has to be told apart from one that did (規約 §diff のファイル一覧).
    SampleTimer {
        id: fileStepReport
        onTriggered: {
            const walk = fileStepTimer.walk
            // The diff the walk landed on has been asked for, has arrived, and the row that says which file it is has
            // been built. All three are the output; the step was the cause. The middle one is what keeps the picture
            // worth looking at — a pane still waiting on its read photographs empty.
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
    // A folder row of the commit's CHANGES tree struck shut, and struck open again (`-unfold`). The strike is the row's
    // own signal, where a click lands — the pane's handler is what carries it to the model, and calling past it would
    // leave the report proving nothing.
    //
    // The two lists that draw a fold arrow keep the answer in different fields (`NameCell.folded`), so `turn=` is read
    // off the icon rather than off either flag: a run that read the flag back would go green with the arrow unwired,
    // which is exactly the shape this verb was cut for.
    SampleTimer {
        id: changesFoldTimer
        /// The directory row struck, and whether the run leaves it shut or strikes it a second time back open. **Read
        /// as a pair** — one arrow on its own says nothing about which way it turned.
        property string path: ""
        property bool reopen: false
        /// How many strikes have gone out. After `n` of them the row is shut exactly when `n` is odd, which is the
        /// wait between one strike and the next: the toggle rebuilds the list, so the row answering a later tick is a
        /// later row.
        property int struck: 0
        function begin() {
            changesFoldTimer.struck = 0
            changesFoldTimer.start()
        }
        /// The folder row for this directory, once the list has built it. Not `FileRowWalk.rowFor`, which answers by
        /// `walkKey` — the name a folder deliberately has none of.
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
                // The commit's own files first, and **read only here**: a read landing after a strike puts the rows
                // back with every fold choice cleared (`DetailsModel::set_files`), so the row swings open under a wait
                // that then never ends (observed — 1 run in a handful reached the watchdog in silence).
                // Read every tick instead, and the verb's own answer would break its own precondition.
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
    // The diff's own arrows, which no headless run can press either: the walk enters where `Keys.onDownPressed` enters
    // (`DiffPane.stepRows`). The hand is walked into the pane first, through the same door the wheel comes in by
    // (`DiffPane.handArrived`) — the diff does not take the keyboard by appearing, so without that the arrows are still
    // the file list's and `focused=` would be false for the right reason (規約 §diff を上下に送る).
    //
    // The wait is for the view, not for the model. `diffSettled()` says the rows arrived; it says nothing about the
    // list having laid them out, and a list whose `contentHeight` is still zero clamps every step to where it already
    // was — the walk then reads exactly like a diff with nothing to scroll (measured, 1 run in 3 came through with
    // `contentHeight` 0 at the step and 216 by the time it was reported). So what is waited for is the output the step
    // consumes: a view with room to be sent, which is `atEnd` answering false over a laid-out height (app-ui.md §UI
    // 自動化の因果性「まだ答えが無い」と値を分ける).
    SampleTimer {
        id: diffStepTimer
        /// How many rows, and which way.
        property int steps: 1
        onTriggered: {
            if (!page.diffShown || !diffPane.diffSettled() || diffPane.view.height <= 0 || diffPane.atEnd)
                return
            diffStepTimer.stop()
            diffStepReport.from = acts.diffRow()
            diffStepReport.stopped = false
            const way = diffStepTimer.steps < 0 ? -1 : 1
            for (let n = 0; n < Math.abs(diffStepTimer.steps); n++) {
                // A step that moved nothing is the end answering. Read beside `atEnd=`: a walk that was refused every
                // step because the pane was never on screen leaves the view at row 0, which is also where an
                // unscrollable diff sits.
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
    /// Where the diff's view stands, in rows — what the walk is counted in, and steadier than a pixel count to read off
    /// a report line.
    function diffRow() {
        return Math.round(diffPane.view.contentY / Theme.rowHeight)
    }
}
