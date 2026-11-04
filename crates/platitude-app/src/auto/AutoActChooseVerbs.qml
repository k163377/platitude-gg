pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// Several commits held at once (デザイン規約 §複数のコミットを選ぶ): the presses on the graph that build the choice,
/// and what goes on from the list it puts up in the right pane.
// `Item` because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    /// `var`: typing it `AutoActDriver` would be circular — that file builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var worktree: driver.worktree
    readonly property var graphPane: driver.graphPane
    readonly property var detailsModel: driver.detailsModel
    readonly property var rowCard: driver.rowCard
    readonly property var detailsPane: driver.detailsPane
    readonly property var diffPane: driver.diffPane

    /// `graph-choose`'s rows, `:`-separated; `fallback` when fewer than two are named.
    function rowsArg(arg, fallback) {
        const named = arg === "" ? [] : arg.split(":").map(Number)
        return named.length >= 2 ? named : fallback
    }
    /// Sets the choice going: `rows` pressed one per beat, the first plain and the rest held (Shift where `sweeps`,
    /// else Ctrl), and — where `goesOn` names one — what `chosenRowTimer` goes on to once the choice stands.
    function choose(rows, sweeps, goesOn) {
        chooseTimer.rows = rows
        chooseTimer.sweeps = sweeps
        chooseTimer.goesOn = goesOn
        chooseTimer.order = rows.map((_, i) => chooseTimer.pressStage(i)).concat(goesOn === "" ? [] : ["chosen"])
        chooseTimer.start()
    }

    /// Runs `act` if it is one of this family's, and says whether it was.
    function run(act, arg) {
        if (act === "graph-choose-dbl") {
            // Toggling a row out of the choice and back in, quickly, arrives as a double-click, modifier and all
            // (`tst_moddblclick`), and the plain double-click is `switch` (デザイン規約 §複数のコミットを選ぶ). The
            // argument is the rows, as `graph-choose` takes them.
            acts.choose(acts.rowsArg(arg, [1, 3, 5]), false, "dbl")
        } else if (act === "graph-choose-diff") {
            // A row of the merged file list opens each chosen commit's own patch of it, stacked
            // (デザイン規約 §複数のコミットを選ぶ). The argument is the path (`chooseTimer.diffPath`); the rows are
            // fixed at three of `--preset basic`, two of which touch `src/topic.txt`.
            acts.choose([3, 5, 6], false, "diff")
        } else if (act === "graph-choose-drop") {
            // A commit taken back out of the chosen list with Ctrl (デザイン規約 §複数のコミットを選ぶ). The argument
            // is the rows, as `graph-choose` takes them.
            acts.choose(acts.rowsArg(arg, [1, 3, 5]), false, "drop")
        } else if (act === "graph-choose-show") {
            // A plain click on the chosen list's first row finds its commit on the graph (デザイン規約 §複数のコミット
            // を選ぶ). The argument is the rows, as `graph-choose` takes them: on screen together once the last is
            // pressed (the choice is counted off the rows' delegates, `AutoActDriver.litRowsTo`), the first far enough
            // down to be centred, and history enough below them to scroll them all away — the defaults are for
            // `--preset deep`.
            acts.choose(acts.rowsArg(arg, [40, 42, 44]), false, "show")
        } else if (act === "graph-choose-show-diff") {
            // The same click with the stacked diff over the graph: the diff closes and the graph comes back to the row
            // (user decision, `RepoPage.showChosen`). The argument and the rows are `graph-choose-diff`'s.
            acts.choose([3, 5, 6], false, "showdiff")
        } else if (act === "graph-choose-said" || act === "graph-choose-sweep") {
            // A row of the chosen list: its card saying the whole message (`said`), or its words dragged away
            // (`sweep`). The argument is the rows, as `graph-choose` takes them. Run on `--preset co-authors`: every
            // message on `basic` is one line with no body, so there is nothing to cut.
            acts.choose(acts.rowsArg(arg, [1, 3, 5]), false, act === "graph-choose-said" ? "said" : "sweep")
        } else if (act === "graph-choose" || act === "graph-choose-range") {
            // The first press is plain; the held ones after it move only the choice (デザイン規約 §複数のコミットを選ぶ).
            // The argument is the rows, `:`-separated — Ctrl one at a time, or `-range`'s one Shift press from the
            // first to the last, so that one wants exactly two. Where row 0 is the working tree's, a range starts at 1:
            // `--preset basic` is such a preset, which the defaults fit. `compared=` is 2 for a choice of two (the
            // two-point comparison) and more for the union of three or more.
            const sweeps = act === "graph-choose-range"
            acts.choose(acts.rowsArg(arg, sweeps ? [1, 5] : [1, 3, 5]), sweeps, "")
        } else {
            return false
        }
        return true
    }
    // Pressed at the row's own click (`GraphRowDelegate.leftClick`): what a held modifier does is decided there and in
    // the page, so writing the choice directly would test neither.
    PageStage {
        id: chooseTimer
        name: "graph_choose"
        property var rows: []
        /// Whether the presses after the first sweep a range (Shift) or take one row each (Ctrl).
        property bool sweeps: false
        /// What `chosenRowTimer` goes on to once the choice stands (`said` / `sweep` / `drop` / `show` / `showdiff` /
        /// `diff` / `dbl`); `""` stops at the choice itself.
        property string goesOn: ""
        /// The file `graph-choose-diff` and `graph-choose-show-diff` open: the argument, `src/topic.txt` by default.
        readonly property string diffPath: Harness.autoAct !== "graph-choose-diff"
                                           && Harness.autoAct !== "graph-choose-show-diff" ? ""
                                         : Harness.autoActArg === "" ? "src/topic.txt" : Harness.autoActArg
        // `readOid`: the commit the plain press landed on, witnessed at it, for the report's `read=`.
        /// The stage the press on the `i`th row is, named by that row: the plain one, then the held ones.
        function pressStage(i) {
            return (i === 0 ? "plain_" : "held_") + chooseTimer.rows[i]
        }
        /// How many presses have gone in, read off where the generation stands.
        function pressesIn() {
            for (let i = chooseTimer.rows.length - 1; i >= 0; i--) {
                if (chooseTimer.at(chooseTimer.pressStage(i)))
                    return i + 1
            }
            return 0
        }
        /// How many commits the presses should end up holding.
        function wanted() {
            if (!chooseTimer.sweeps)
                return chooseTimer.rows.length
            return Math.abs(chooseTimer.rows[chooseTimer.rows.length - 1] - chooseTimer.rows[0]) + 1
        }
        function litRows() {
            return driver.litRowsTo(Math.max(0, ...chooseTimer.rows))
        }
        /// Every press is in; four things have to land: the tally, the rows' highlight of it, the pane having
        /// answered for this choice — `selectionLoaded`, since a failed read also stops loading and leaves the empty
        /// list a no-change choice has — and the commit list laid out, since an unlaid-out view measures 0.
        function landed() {
            return {
                "chosen": page.chosenCount === chooseTimer.wanted(),
                "lit": chooseTimer.litRows() === chooseTimer.wanted(),
                "read": detailsModel.selectionLoaded,
                "drawn": detailsPane.chosenListDrawn,
                "page": PageSettled.settled(page) }
        }
        onTriggered: {
            const op = chooseTimer
            const done = op.pressesIn()
            if (done < op.rows.length) {
                const want = op.rows[done]
                // A row far down has no delegate until the view is over it: sent there, and pressed on a later beat.
                // The row built is what the press stands on, and the stage is named by it.
                op.input(op.pressStage(done), () => {
                    const item = graphPane.view.itemAtIndex(want)
                    if (item === null) {
                        graphPane.view.positionViewAtIndex(want, ListView.Contain)
                        return false
                    }
                    if (done === 0) {
                        op.witness("readOid", item.oid_hex)
                        item.leftClick(Qt.NoModifier)
                    } else {
                        item.leftClick(op.sweeps ? Qt.ShiftModifier : Qt.ControlModifier)
                    }
                    return true
                })
                return
            }
            if (op.goesOn !== "") {
                if (!Awaited.all("graph_choose_landed", op.landed()))
                    return
                op.advance("chosen")
                op.stop()
                // The said card's and the diff's own waits are not stages; the double, the drop and the sweep press.
                chosenRowTimer.order = op.goesOn === "dbl" ? ["double_clicked"]
                                     : op.goesOn === "diff" ? ["diff_asked"]
                                     : op.goesOn === "drop" ? ["plain_press", "held_press"]
                                     : op.goesOn === "show" ? ["scrolled", "air_click", "words_click"]
                                     : op.goesOn === "showdiff" ? ["diff_asked", "words_click"]
                                     : op.goesOn === "sweep" ? ["swept"] : []
                chosenRowTimer.start()
                return
            }
            op.complete({
                "name": "graph_choose_landed",
                "when": () => op.landed(),
                "report": () => "graph_choose chosen=" + page.chosenCount
                                + " lit=" + op.litRows()
                                + " read=" + (page.selectedOid === op.witnessed("readOid"))
                                + " wip=" + page.wipShown
                                // How far the commit list runs past its view, margins counted: a choice this small
                                // fits, so anything but 0 scrolls to reach nothing.
                                + " spare=" + detailsPane.chosenListSpare
                                + " compared=" + detailsModel.comparing
                                + " files=" + detailsModel.fileTotal
                                + " rows=" + op.rows.join(",") })
        }
    }
    // Everything that goes on from a standing choice (`chooseTimer.goesOn`), each put in at the thing that answers it.
    PageStage {
        id: chosenRowTimer
        name: "chosen_row"
        // Witnessed at the presses — the drop's: the commit they land on (`dropOid`), what the row said about a press
        // without the modifier (`dropTakes`), and the commits held after the plain press (`dropKept`) — unchanged
        // means the plain click dropped nothing; the double's own answer and the row's (`dblLed`,
        // `dblMovable`), the report waiting for the reads the clicks set going; and what the sweep came away with
        // (`swept`).
        /// The two clicks, then the double the second is also reported as.
        function doubled() {
            const op = chosenRowTimer
            if (op.at("start")) {
                const at = graphPane.view.itemAtIndex(chooseTimer.rows[chooseTimer.rows.length - 1])
                if (!Awaited.all("chosen_dbl_row", { "row": !!at }))
                    return
                // As the area delivers the pair. `led=` is the row's own answer: a `switch` would land beats later, so
                // the branch on screen says nothing yet. `movable=` rules out a row that turns the gesture down for
                // leading nowhere.
                op.input("double_clicked", () => {
                    at.leftClick(Qt.ControlModifier)
                    op.witness("dblLed", at.doubleClick(Qt.ControlModifier))
                    op.witness("dblMovable", at.movable)
                })
                return
            }
            // The clicks moved the choice; its read is the page's.
            op.complete({
                "name": "chosen_dbl_landed",
                "when": () => ({ "page": PageSettled.settled(page) }),
                "report": () => "chosen_dbl led=" + op.witnessed("dblLed")
                                + " movable=" + op.witnessed("dblMovable")
                                + " naming=" + (graphPane.namingOid !== "")
                                + " branch=" + worktree.branch })
        }
        /// Through the page's own opener, the one a file row's press goes to; the stack is the shot, so its rows are
        /// waited for. `bands=` is the claim; the picture is read for the order band, `@@`, lines, band, `@@`, lines.
        function stacked() {
            const op = chosenRowTimer
            if (op.at("start")) {
                op.input("diff_asked", () => {
                    if (!page.diffShown)
                        page.toggleDiff("commit", chooseTimer.diffPath, "")
                })
                return
            }
            // Each band's own mode change is a row of its own (`encode::flatten_patches`), worded by the pane:
            // `modes=` counts them, `mode=` is the first as the row puts it. The stack is a few rows, read off their
            // delegates — so once every one is built, which comes a beat after the rows.
            op.complete({
                "name": "chosen_diff_rows",
                "when": () => {
                    let built = true
                    for (let i = 0; i < diffPane.view.count; i++)
                        built = built && diffPane.view.itemAtIndex(i) !== null
                    return {
                        "shown": page.diffShown,
                        "diff": page.diffShown && diffPane.diffSettled(),
                        "rows": diffPane.view.count !== 0,
                        "built": built }
                },
                "report": () => {
                    const modeRows = []
                    for (let i = 0; i < diffPane.view.count; i++) {
                        const item = diffPane.view.itemAtIndex(i)
                        if (item !== null && item.kind === "mode")
                            modeRows.push(Words.modeLineOf(item.text))
                    }
                    return "chosen_diff bands=" + diffPane.diffModel.commitBands
                           + " chosen=" + page.chosenCount
                           + " path=" + page.diffPath
                           + " modes=" + modeRows.length
                           + " mode=" + (modeRows.length > 0 ? modeRows[0] : "")
                } })
        }
        /// Two clicks at the row's own function; only the held second one is the drop hand's and takes a commit out.
        function dropped(row) {
            const op = chosenRowTimer
            if (op.at("start")) {
                op.input("plain_press", () => {
                    op.witness("dropOid", row.oid_hex)
                    op.witness("dropTakes", row.takesPress(Qt.NoModifier))
                    row.leftClick(Qt.NoModifier)
                })
                return
            }
            if (op.at("plain_press")) {
                op.input("held_press", () => {
                    op.witness("dropKept", page.chosenCount)
                    row.leftClick(Qt.ControlModifier)
                })
                return
            }
            op.complete({
                "name": "chosen_drop_landed",
                "when": () => ({ "chosen": page.chosenCount === chooseTimer.wanted() - 1,
                                 "read": detailsModel.selectionLoaded }),
                "report": () => {
                    const now = detailsPane.chosenRowAt(0)
                    return "chosen_drop takes=" + op.witnessed("dropTakes")
                           + " kept=" + op.witnessed("dropKept")
                           + " dropped=" + (page.chosenOids[op.witnessed("dropOid")] !== true)
                           + " hand=" + (!!now && now.dropStands)
                           + " chosen=" + page.chosenCount
                           + " lit=" + chooseTimer.litRows()
                } })
        }
        /// The graph scrolled a hundred rows past the choice, as a reader's wheel would take it, then two plain clicks
        /// on the list's first row — the newest held commit, the graph's highest row of the choice — each entered where
        /// its hand's handlers enter: the first in the row's air (`SweepPad`), with the row out of the graph's sight,
        /// the second on the words (`RowHand`), with it on screen. Witnessed at the presses: whether the row was out
        /// of sight (`showAway`), where the view stood before each (`showFromY` / `showOnY`), how the first left the
        /// row (`showFirst`, `GraphRowWalk.stepLanding`), and the row the arrows step from and the commit being read
        /// (`showCurrent` / `showRead`), which neither may move.
        function shown(row) {
            const op = chosenRowTimer
            const target = Math.min(...chooseTimer.rows)
            if (op.at("start")) {
                op.input("scrolled", () => {
                    graphPane.view.positionViewAtIndex(Math.max(...chooseTimer.rows) + 100, ListView.Center)
                })
                return
            }
            if (op.at("scrolled")) {
                op.input("air_click", () => {
                    op.witness("showAway", !graphPane.rowOnScreen(target))
                    op.witness("showFromY", graphPane.view.contentY)
                    op.witness("showCurrent", graphPane.view.currentIndex)
                    op.witness("showRead", page.selectedOid)
                    const air = row.sweep.airPoints(3)[0]
                    row.sweep.pressAt(air)
                    row.sweep.releaseNow()
                })
                return
            }
            if (op.at("air_click")) {
                op.input("words_click", () => {
                    op.witness("showFirst", graphPane.stepLanding(target, op.witnessed("showFromY")))
                    op.witness("showOnY", graphPane.view.contentY)
                    row.wordsHand.takeAt(Qt.point(row.wordsHand.width / 2, row.wordsHand.height / 2))
                    row.wordsHand.releaseNow()
                })
                return
            }
            op.complete({
                "name": "chosen_show_landed",
                "when": () => ({ "page": PageSettled.settled(page) }),
                "report": () => {
                    const at = Math.min(...chooseTimer.rows)
                    const now = detailsPane.chosenRowAt(0)
                    return "chosen_show away=" + op.witnessed("showAway")
                           + " first=" + op.witnessed("showFirst")
                           + " second=" + graphPane.stepLanding(at, op.witnessed("showOnY"))
                           + " chosen=" + page.chosenCount
                           + " current=" + (graphPane.view.currentIndex === op.witnessed("showCurrent"))
                           + " read=" + (page.selectedOid === op.witnessed("showRead"))
                           + " hands=" + (!!now && now.showStands)
                           + " row=" + at
                } })
        }
        /// The stack opened over the graph through the page's own opener (as `stacked` opens it), then a plain click
        /// on the list's first row, entered at the hand over its words. Witnessed at the click: that the diff stood
        /// (`showDiffOpen`) and the commit being read (`showRead`), which the click may not move.
        function shownOverDiff(row) {
            const op = chosenRowTimer
            if (op.at("start")) {
                op.input("diff_asked", () => {
                    if (!page.diffShown)
                        page.toggleDiff("commit", chooseTimer.diffPath, "")
                })
                return
            }
            if (op.at("diff_asked")) {
                if (!Awaited.all("chosen_show_diff_open", { "diff": page.diffShown && diffPane.diffSettled() }))
                    return
                op.input("words_click", () => {
                    op.witness("showDiffOpen", page.diffShown)
                    op.witness("showRead", page.selectedOid)
                    row.wordsHand.takeAt(Qt.point(row.wordsHand.width / 2, row.wordsHand.height / 2))
                    row.wordsHand.releaseNow()
                })
                return
            }
            op.complete({
                "name": "chosen_show_diff_landed",
                "when": () => ({ "page": PageSettled.settled(page) }),
                "report": () => "chosen_show_diff open=" + op.witnessed("showDiffOpen")
                                + " closed=" + !page.diffShown
                                + " graph=" + graphPane.visible
                                + " shown=" + graphPane.rowOnScreen(Math.min(...chooseTimer.rows))
                                + " chosen=" + page.chosenCount
                                + " read=" + (page.selectedOid === op.witnessed("showRead")) })
        }
        onTriggered: {
            const op = chosenRowTimer
            const goesOn = chooseTimer.goesOn
            if (goesOn === "dbl") {
                op.doubled()
                return
            }
            if (goesOn === "diff") {
                op.stacked()
                return
            }
            const row = detailsPane.chosenRowAt(0)
            if (!Awaited.all("chosen_row_ready", { "row": !!row && row.rowReady() }))
                return
            if (goesOn === "drop") {
                op.dropped(row)
                return
            }
            if (goesOn === "show") {
                op.shown(row)
                return
            }
            if (goesOn === "showdiff") {
                op.shownOverDiff(row)
                return
            }
            if (goesOn === "said") {
                // Entered where the row's rest would open its card (hover cannot be injected), until it is up.
                const asked = op.at("card_asked") || op.input("card_asked", () => {
                    if (!rowCard.opened)
                        row.askCard()
                    return rowCard.opened
                })
                if (!asked)
                    return
                // `held=` stays right after `lit=`: the wall's run is judged on the three before it, and `must_say`
                // reads one unbroken stretch of the line.
                op.complete({
                    "name": "chosen_said_card",
                    "when": () => ({ "card": rowCard.opened }),
                    "report": () => {
                        const now = detailsPane.chosenRowAt(0)
                        return "chosen_said open=" + rowCard.opened
                               + " door=" + rowCard.asksForMore
                               + " lit=" + (!!now && now.lit)
                               + " held=" + rowCard.messageCut
                               + " cut=" + (!!now && now.summaryCut)
                    } })
                return
            }
            op.input("swept", () => {
                op.witness("swept", row.sweep.sweepAir(9, ""))
            })
            op.complete({ "name": "chosen_swept", "when": () => ({}),
                          "report": () => "chosen_sweep " + op.witnessed("swept") })
        }
    }
}
