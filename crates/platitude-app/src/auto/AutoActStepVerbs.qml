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
    readonly property var unstagedModel: driver.unstagedModel
    readonly property var worktree: driver.worktree
    readonly property var graphPane: driver.graphPane
    readonly property var detailsPane: driver.detailsPane
    readonly property var diffPane: driver.diffPane
    readonly property var wipPane: driver.wipPane

    /// Runs `act` if it is one of this family's, and says whether it was.
    function run(act, arg) {
        if (act === "graph-step" || act === "graph-step-edge"
            || act === "graph-step-far" || act === "graph-step-named"
            || act === "graph-step-dirty" || act === "graph-step-diff") {
            // `-edge` walks off the bottom. `-far` sets off from the row its argument names: the row it steps onto is
            // centred only where half a view lies above it, which near the tip it never does on a window standing on
            // its floor.
            page.activateRow(act === "graph-step-far" && arg !== "" ? graphModel.oidAt(Number(arg))
                             : worktree.branchOid !== "" ? worktree.branchOid : graphModel.oidAt(0))
            graphStepTimer.named = act === "graph-step-named"
            graphStepTimer.dirty = act === "graph-step-dirty"
            graphStepTimer.away = act === "graph-step-far"
            graphStepTimer.diffPath = act === "graph-step-diff" ? arg : ""
            graphStepTimer.steps = act === "graph-step-named" ? 1 : act === "graph-step-dirty" ? 2
                                 : act === "graph-step-edge" ? 10 : act === "graph-step-far" ? 1
                                 : act === "graph-step-diff" ? 1 : arg === "" ? 1 : Number(arg)
            graphStepTimer.start()
        } else if (act === "graph-step-hold") {
            page.activateRow(worktree.branchOid !== "" ? worktree.branchOid : graphModel.oidAt(0))
            graphHoldTimer.steps = arg === "" ? 6 : Number(arg)
            graphHoldTimer.start()
        } else if (act === "changes-step" || act === "changes-step-edge"
                   || act === "wip-step"
                   || act === "changes-shut" || act === "wip-shut") {
            // The argument is the file to start on — `<bucket>:<path>` for the worktree's list, where a file
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
                page.activateRow(worktree.branchOid !== "" ? worktree.branchOid : graphModel.oidAt(0))
                fileStepTimer.pane = "changes"
                fileStepTimer.bucket = ""
                fileStepTimer.path = arg
            }
            fileStepTimer.overrun = act === "changes-step-edge"
            fileStepTimer.shut = act === "changes-shut" || act === "wip-shut"
            fileStepTimer.order = fileStepTimer.shut ? ["clicked", "shut"] : ["clicked", "walked"]
            fileStepTimer.start()
        } else if (act === "changes-fold" || act === "changes-unfold") {
            // The argument is the directory as the row is keyed (a lone chain is one row, `a/b/c`), and must be a row
            // the list has built (`itemAtIndex`). The default is the first row, the one a picture can hold: a folder
            // shut below the fold frames like one left open.
            page.activateRow(worktree.branchOid !== "" ? worktree.branchOid : graphModel.oidAt(0))
            // Said outright: a run that inherited the paths view would wait out the watchdog for a folder row.
            detailsModel.setTreeView(true)
            changesFoldTimer.path = arg === "" ? "assets/icons" : arg
            changesFoldTimer.reopen = act === "changes-unfold"
            changesFoldTimer.order = changesFoldTimer.reopen ? ["shut", "reopened"] : ["shut"]
            changesFoldTimer.start()
        } else if (act === "diff-step" || act === "diff-step-edge") {
            // Moves the view (規約 §diff を上下に送る). Needs the 320x240 seed: no demo diff outgrows a default window,
            // and in the seed — lifted to the floor the folded rail holds — rows lie below the fold, so the plain walk
            // is one row.
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
    //
    // Read once the settle behind the walk (`keyStepSettleMs`) has landed: a walk that moved the highlight but never
    // the selection must be told apart from one that did, and both frame alike. Two edges, because a walk reads its
    // start and end rows (`GraphRowWalk.noteStep`): `selected=` is the selection reaching the lit row, `card=` the card
    // reaching the selection — waiting on the first alone photographs a card still holding a row passed through.
    PageStage {
        id: graphStepTimer
        name: "graph_step"
        order: ["prepared", "walked"]
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
        /// The selection reached the lit row, the card caught up with it, and the settle behind the last step landed
        /// it (`GraphRowWalk.settling`) — the diff it brings is asked for then, and `page` waits for that read and its
        /// rows.
        function walked() {
            const row = graphPane.view.currentIndex
            return {
                "ready": !(row < 0 || (graphStepTimer.diffPath === "" && page.selectedOid !== graphModel.oidAt(row))
                    || !driver.cardSettled),
                "walked": !graphPane.stepSettling,
                "page": PageSettled.settled(page) }
        }
        /// The run's line. Witnessed as the walk went: the row it set off from (`from`), where the view stood before
        /// its last step (`wasY`, for `landing=`), and how many steps were refused (`refused`).
        function line() {
            const op = graphStepTimer
            const at = graphPane.view.currentIndex
            return "graph_step from=" + op.witnessed("from")
                   + " row=" + at
                   + " steps=" + op.steps
                   + " landing=" + graphPane.stepLanding(at, op.witnessed("wasY"))
                   + " back=" + (at === op.witnessed("from"))
                   + " refused=" + op.witnessed("refused")
                   + " focused=" + graphPane.view.activeFocus
                   + " diff=" + page.diffShown
                   + " onscreen=" + graphPane.rowOnScreen(at)
                   + " selected=" + (page.selectedOid === graphModel.oidAt(at))
                   + " card=" + (detailsModel.shaHex === page.selectedOid)
        }
        onTriggered: {
            const op = graphStepTimer
            if (op.at("start")) {
                if (!Awaited.all("graph_step_card", { "card": driver.cardSettled }))
                    return
                op.input("prepared", () => {
                    if (op.dirty)
                        detailsPane.setMessageText("wip: half of a subject", "")
                    if (op.named)
                        graphPane.startNaming(
                            graphModel.oidAt(graphPane.view.currentIndex))
                    // The keyboard before the diff: the diff is what must take it away, and taking it after would press
                    // a pane already off screen.
                    graphPane.view.takeKeyboard()
                    if (op.diffPath !== "")
                        page.toggleDiff("commit", op.diffPath, "")
                })
                return
            }
            if (op.at("prepared")) {
                // A beat between setup and walk: the layout swaps the graph away in its own pass, so a step in the
                // diff's opening beat would still find the pane on screen. The diff's rows first, as `chosen_diff`
                // waits: a step the moment the pane arrived leaves the picture and the census holding an empty diff.
                if (!Awaited.all("graph_step_diff", {
                        "diff": op.diffPath === ""
                                || (page.diffShown && diffPane.diffSettled() && diffPane.view.count !== 0) }))
                    return
                op.input("walked", () => {
                    if (op.away)
                        graphPane.view.contentY = graphPane.view.clampY(Infinity)
                    op.witness("from", graphPane.view.currentIndex)
                    let refused = 0
                    let wasY = 0
                    const way = op.steps < 0 ? -1 : 1
                    for (let n = 0; n < Math.abs(op.steps); n++) {
                        // Before each step, so `landing=` reads the last one: a walk off the bottom moves the view
                        // every row.
                        wasY = graphPane.view.contentY
                        if (!graphPane.stepRow(way))
                            refused++
                    }
                    op.witness("wasY", wasY)
                    op.witness("refused", refused)
                })
                return
            }
            op.complete({ "name": "graph_step_report_ready", "when": () => op.walked(), "report": () => op.line() })
        }
    }
    // The arrow held down: the opening press, then the repeats once its settle has expired, as an OS's first repeat
    // always arrives after it (`GraphRowWalk.noteStep`). `reads=` is the claim — a held walk reads two rows, its start
    // and its end — and no picture holds it. The hand off the key: the selection landed and the card caught up — the
    // pair `graph_step` waits on.
    PageStage {
        id: graphHoldTimer
        name: "graph_hold"
        order: ["opened", "repeated"]
        /// How many rows the run walks, the opening press included.
        property int steps: 6
        /// The hand off the key: the selection landed on the lit row and the card caught up.
        function landed() {
            const row = graphPane.view.currentIndex
            return {
                "row": row >= 0,
                "settled": !graphPane.stepSettling,
                "selected": row >= 0 && page.selectedOid === graphModel.oidAt(row),
                "card": driver.cardSettled }
        }
        /// The run's line. Witnessed as the walk went: the row it set off from (`from`), `holdReads` as the hold set
        /// off (`readsFrom`) — the reads the walk asked for are the ones counted past it — and how many steps were
        /// refused, by the opening press (`openRefused`) and by the whole walk (`refused`).
        function line() {
            const op = graphHoldTimer
            const at = graphPane.view.currentIndex
            return "graph_hold from=" + op.witnessed("from")
                   + " row=" + at
                   + " steps=" + op.steps
                   + " reads=" + (acts.holdReads - op.witnessed("readsFrom"))
                   + " refused=" + op.witnessed("refused")
                   + " selected=" + (page.selectedOid === graphModel.oidAt(at))
                   + " card=" + (detailsModel.shaHex === page.selectedOid)
        }
        onTriggered: {
            const op = graphHoldTimer
            if (op.at("start")) {
                if (!Awaited.all("graph_hold_card", { "card": driver.cardSettled }))
                    return
                op.input("opened", () => {
                    graphPane.view.takeKeyboard()
                    op.witness("readsFrom", acts.holdReads)
                    op.witness("from", graphPane.view.currentIndex)
                    let refused = 0
                    if (!graphPane.stepRow(1))
                        refused++
                    op.witness("openRefused", refused)
                })
                return
            }
            if (op.at("opened")) {
                // The repeats: waited on the settle being gone (app-ui.md §UI 自動化), then all in one beat — the claim
                // is that a repeat is not read.
                if (!Awaited.all("graph_hold_settled", { "settled": !graphPane.stepSettling }))
                    return
                op.input("repeated", () => {
                    let refused = op.witnessed("openRefused")
                    for (let n = 1; n < op.steps; n++) {
                        if (!graphPane.stepRow(1, true))
                            refused++
                    }
                    op.witness("refused", refused)
                })
                return
            }
            op.complete({ "name": "graph_hold_report_ready", "when": () => op.landed(), "report": () => op.line() })
        }
    }
    /// Commits the walk has asked for, counted on the signal: the asks come and go inside a beat
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
    //
    // The light runs at the key's rate and the diff catches up after the settle (`keyStepSettleMs`), so a walk that
    // moved the light but never the diff must be told apart from one that did (規約 §diff のファイル一覧). `-shut`'s
    // `lit=` is the claim, read off the rectangles (`litPath`) so a light only ever in the model says false;
    // `open=false` makes the answer that list's own.
    PageStage {
        id: fileStepTimer
        name: "file_step"
        /// Which list (`changes` / `wip`) and how far. `overrun` (`-edge`) walks past the end: the count is only known
        /// once the details arrive, so it is read as the walk goes. The steps taken (`steps`) and whether one was
        /// refused (`stopped`) are the walk's witnesses.
        property string pane: "changes"
        property int steps: 1
        property bool overrun: false
        /// The file clicked, and the bucket its row sits in (empty for the commit's list, whose files sit in none).
        property string bucket: ""
        property string path: ""
        /// `-shut`: a second click on the same row shuts the diff (`RepoPage.toggleDiff`) and the run stops there,
        /// both lists lighting nothing — the two verbs are read as a pair (デザイン規約 §diff のファイル一覧).
        property bool shut: false
        /// The click, through the row itself (twice for `-shut`); false while the row is not there.
        function strike() {
            const row = fileStepTimer.walk.rowFor(fileStepTimer.bucket, fileStepTimer.path)
            if (!row)
                return false
            if (fileStepTimer.pane === "wip")
                row.fileClicked(fileStepTimer.bucket, fileStepTimer.path,
                                unstagedModel.origOf(fileStepTimer.path),
                                Qt.NoModifier)
            else
                // The row's own press, with its own reading of the model: handed the path from beside it, this goes
                // green on a row that reads nothing (verify-ui §壊れない動詞の実装と反復).
                row.press()
            return true
        }
        readonly property var walk:
            fileStepTimer.pane === "wip" ? wipPane.filesWalk : detailsPane.filesWalk
        onTriggered: {
            const op = fileStepTimer
            if (op.at("start")) {
                op.input("clicked", () => op.strike())
                return
            }
            // The click has to have landed: a walk with nothing being read is refused, and reading that as "the end"
            // would go green on a click that never arrived.
            const read = page.diffShown && page.diffPath === op.path
            if (op.at("clicked") && !op.shut) {
                if (!Awaited.all("file_step_read", { "read": read }))
                    return
                op.input("walked", () => {
                    const steps = op.witness("steps", op.overrun ? op.walk.view.count + 5 : op.steps)
                    let stopped = false
                    const way = steps < 0 ? -1 : 1
                    for (let n = 0; n < Math.abs(steps); n++) {
                        if (!op.walk.stepFile(way))
                            stopped = true
                    }
                    op.witness("stopped", stopped)
                })
                return
            }
            if (op.at("clicked")) {
                // The second click once the read has landed — shutting a diff still coming would test a race.
                if (!Awaited.all("file_shut_read", { "read": read, "diff": read && diffPane.diffSettled() }))
                    return
                op.input("shut", () => op.strike())
                return
            }
            if (op.shut) {
                // `open` / `lit` / `focused` stay together and in this order: a `must_say` is one stretch of the line
                // (`verify::verbs`).
                op.complete({
                    "name": "file_shut_report_ready",
                    "when": () => ({ "ready": !(page.diffShown), "page": PageSettled.settled(page) }),
                    "report": () => "file_shut path=" + op.path
                                    + " pane=" + op.pane
                                    + " open=" + page.diffShown
                                    + " lit=" + (op.walk.litPath() !== "")
                                    + " focused=" + op.walk.view.activeFocus })
                return
            }
            // The landed diff asked for, arrived (a pane still reading photographs empty), and its lit row built.
            op.complete({
                "name": "file_step_report_ready",
                "when": () => ({
                    "shown": page.diffShown,
                    "moved": page.diffPath !== op.path,
                    "diff": diffPane.diffSettled(),
                    "lit": !!op.walk && op.walk.litPath() !== "",
                    "page": PageSettled.settled(page) }),
                "report": () => "file_step pane=" + op.pane
                                + " from=" + op.path
                                + " steps=" + op.witnessed("steps")
                                + " read=" + (page.diffKind + ":" + page.diffPath)
                                + " moved=" + (page.diffPath !== op.path)
                                + " stopped=" + op.witnessed("stopped")
                                + " lit=" + (op.walk.litPath() === page.diffPath)
                                + " focused=" + op.walk.view.activeFocus })
        }
    }
    // A folder row of CHANGES struck shut, and open again for `-unfold`, by the row's own signal — the pane's handler
    // carries it to the model, so calling past it proves nothing. The two lists that draw a fold arrow keep the answer
    // in different fields (`NameCell.folded`), so `turn=` is read off the icon: reading the flag back would go green
    // with the arrow unwired.
    PageStage {
        id: changesFoldTimer
        name: "changes_fold"
        /// The directory row struck, and whether a second strike reopens it. Read as a pair: one arrow alone says
        /// nothing about which way it turned.
        property string path: ""
        property bool reopen: false
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
        /// The folder row there, on a settled page, and `folded` as the last strike left it.
        function landed(folded) {
            const row = changesFoldTimer.folderRow()
            return { "ready": row !== null, "page": PageSettled.settled(page),
                     "landed": row !== null && row.isFolded === folded }
        }
        onTriggered: {
            const op = changesFoldTimer
            const row = op.folderRow()
            // The row is shut exactly while the generation stands at `shut`, which is the wait between strikes: the
            // toggle rebuilds the list, so a later beat's row is a new row. The run ends with it as its last strike
            // left it — shut, or open again for `-unfold`.
            if (!Awaited.all("changes_fold_ready", op.landed(op.at("shut"))))
                return
            if (op.at("start") || (op.reopen && op.at("shut"))) {
                // The commit's own files first, read only on this branch: a read landing after a strike clears every
                // fold choice (`DetailsModel::take_files`), and the row swings open under a wait that never ends. Read
                // every beat, the verb's own answer would break its own precondition.
                if (!Awaited.all("changes_fold_read", {
                        "read": !(detailsModel.loading || detailsModel.shaHex !== page.selectedOid) }))
                    return
                op.input(op.at("start") ? "shut" : "reopened", () => {
                    row.folderToggled(row.pathText)
                })
                return
            }
            op.complete({
                "name": "changes_fold_ready",
                "when": () => op.landed(!op.reopen),
                "report": () => {
                    const now = op.folderRow()
                    return "changes_fold path=" + op.path
                           // One strike, or two for `-unfold`: each a stage this run went through.
                           + " strikes=" + (op.reopen ? 2 : 1)
                           + " shut=" + (now !== null && now.isFolded)
                           + " turn=" + (now === null ? "" : now.foldTurn)
                           + " rows=" + detailsPane.filesWalk.view.count
                           + " tree=" + detailsModel.treeView
                } })
        }
    }
    // The diff's arrows enter at `DiffPane.stepRows`, where `Keys.onDownPressed` does. The wait is for the view, not
    // only the rows (`diffSettled()`): a list whose `contentHeight` is still 0 clamps every step, reading like a diff
    // with nothing to scroll — so it waits for `atEnd` false over a laid-out height
    // (rules-refs/app-ui.md「『まだ答えが無い』と値 0 / false を分ける」).
    PageStage {
        id: diffStepTimer
        name: "diff_step"
        order: ["walked"]
        property int steps: 1
        // Witnessed as the walk went: the row the view stood at (`from`), and whether a step was refused (`stopped`).
        onTriggered: {
            const op = diffStepTimer
            if (op.at("start")) {
                if (!Awaited.all("diff_step_ready", { "shown": page.diffShown,
                                                      "diff": page.diffShown && diffPane.diffSettled(),
                                                      "laid": diffPane.view.height > 0,
                                                      "scrolls": !diffPane.atEnd }))
                    return
                op.input("walked", () => {
                    op.witness("from", acts.diffRow())
                    let stopped = false
                    const way = op.steps < 0 ? -1 : 1
                    for (let n = 0; n < Math.abs(op.steps); n++) {
                        // A step that moved nothing is the end answering. Read beside `atEnd=`: a walk refused
                        // throughout (the pane never on screen) also leaves the view at row 0, like an unscrollable
                        // diff.
                        if (!diffPane.stepRows(way))
                            stopped = true
                    }
                    op.witness("stopped", stopped)
                })
                return
            }
            op.complete({
                "name": "diff_step_report_ready",
                "when": () => ({ "ready": diffPane.diffSettled(), "page": PageSettled.settled(page) }),
                "report": () => "diff_step from=" + op.witnessed("from")
                                + " rows=" + acts.diffRow()
                                + " steps=" + op.steps
                                + " moved=" + (acts.diffRow() !== op.witnessed("from"))
                                + " atEnd=" + diffPane.atEnd
                                + " stopped=" + op.witnessed("stopped")
                                + " focused=" + diffPane.view.activeFocus })
        }
    }
    /// Where the diff's view stands, in rows — the walk's unit, steadier than pixels on a report line.
    function diffRow() {
        return Math.round(diffPane.view.contentY / Theme.rowHeight)
    }
}
