pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The graph's rows: the card a row puts out, the chips' own list, and the parts a row is made of. The gesture that
/// names one has a family of its own (`AutoActGraphReclickVerbs`).
// `Item` because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    /// `var`: typing it `AutoActDriver` would be circular — that file builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var worktree: driver.worktree
    readonly property var graphModel: driver.graphModel
    readonly property var graphPane: driver.graphPane
    readonly property var detailsModel: driver.detailsModel
    readonly property var commitMenu: driver.commitMenu
    readonly property var commitBranchCard: driver.commitBranchCard
    readonly property var commitTagCard: driver.commitTagCard
    readonly property var refList: driver.refList
    readonly property var rowCard: driver.rowCard
    readonly property var rowHost: driver.rowHost
    readonly property var detailsPane: driver.detailsPane
    readonly property var diffPane: driver.diffPane

    /// `graph-choose`'s rows, `:`-separated; `fallback` when fewer than two are named.
    function rowsArg(arg, fallback) {
        const named = arg === "" ? [] : arg.split(":").map(Number)
        return named.length >= 2 ? named : fallback
    }
    /// How many rows draw themselves chosen, counted to a little past `last` (the last row pressed or stood on) so a
    /// row lit that should not be is counted too.
    function litRowsTo(last) {
        let lit = 0
        for (let i = 0; i < Math.min(graphModel.rowTotal, last + 3); i++) {
            const item = graphPane.view.itemAtIndex(i)
            if (item && item.selected)
                lit++
        }
        return lit
    }

    /// Whether the card a chip unfolds into covers `front` (the chip's front card), read off both boxes in the scene —
    /// not off the numbers the card was sized by, which a check would only agree with. Each side is how far the chip
    /// stands outside the card (negative = inside); `over=`, the far edge, is the one a card sized to its rows falls
    /// short on.
    function coverOf(front) {
        const face = refList.background
        const card = face.mapToItem(null, 0, 0)
        const chip = front.mapToItem(null, 0, 0)
        const over = Math.round(chip.x + front.width - (card.x + face.width))
        const left = Math.round(card.x - chip.x)
        const top = Math.round(card.y - chip.y)
        const foot = Math.round(chip.y + front.height - (card.y + face.height))
        const covers = over <= 0 && left <= 0 && top <= 0 && foot <= 0
        return "covers=" + covers + " over=" + over + " left=" + left + " top=" + top + " foot=" + foot
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
    /// A row of the stacked list as the card has built it, or null — read where it stands, not sent into view: the
    /// press before it brought it there (`RefListPopup.rowAt` sends a row into view, which a reading must not).
    function listRow(i) {
        return refList.list.itemAtIndex(i)
    }

    /// Runs `act` if it is one of this family's, and says whether it was.
    function run(act, arg) {
        if (act === "row-part") {
            // Hover cannot be injected, so this writes what a real pointer writes (`GraphRowDelegate.pointerRowX`) and
            // leaves the decision to the row — the decision is what the verb tests, so `chipExpandRequested` (as
            // `ref-list` does) would prove nothing about the boundary.
            const parts = arg.split(":")
            rowPartTimer.row = Number(parts[0])
            rowPartTimer.want = parts[2]
            rowPartTimer.x = Number(parts[1])
            rowPartTimer.start()
        } else if (act === "ref-list-leave") {
            // The hand onto the stretch of a chip column the chip's card leaves bare, the list out, and then the hand
            // gone — to no row (`off`), back onto the same stretch within the beat (`back`), into the card for a
            // right-click and back onto the stretch before the menu goes (`menu`), or onto another row's (its
            // number), each through the row's own hover door (`GraphRowDelegate.pointerCrossed`). The argument is
            // `<行|head>:<先>`; `head` is HEAD's stand-in (`--preset deep`, sent to the end).
            const leave = arg.split(":")
            leaveTimer.from = leave[0]
            leaveTimer.to = leave.length > 1 ? leave[1] : ""
            leaveTimer.start()
        } else if (act === "list-menu") {
            // The right-click on a row of the stacked list (デザイン規約 §グラフ行の右クリック). The argument is
            // `<行>[:<カードの行>]`.
            const at = arg.split(":")
            refListOpenTimer.row = at[0] === "" ? 0 : Number(at[0])
            refListOpenTimer.cards = false
            refListOpenTimer.menuRow = at.length > 1 ? Number(at[1]) : 0
            refListOpenTimer.start()
        } else if (act === "ref-list" || act === "ref-list-card") {
            // Entered where the hover timer would (hover cannot be injected). `-card` walks row → card → chip → asked
            // again from under the list: both card closes have to hold, and either failing leaves `open=true`. No row
            // given is HEAD's, the one commit sure to have a chip (row 0 is the uncommitted row on a dirty preset).
            refListOpenTimer.row = arg === "" ? -1 : Number(arg)
            refListOpenTimer.cards = act === "ref-list-card"
            refListOpenTimer.start()
        } else if (act === "graph-head-list") {
            // Written where the pointer writes on the stand-in (`GraphHeadPin.pointerX`): whether the stand-in asks at
            // all is the point, so `chipExpandRequested` directly would prove nothing. `click` then presses the card's
            // first row — the stand-in's press — off a selection moved off HEAD first, and probes the row left under
            // the still pointer.
            headListTimer.click = arg === "click"
            headListTimer.order = headListTimer.click ? ["moved_off", "pin_up", "pointed", "pressed", "probed"]
                                                      : ["pin_up", "pointed"]
            headListTimer.start()
        } else if (act === "ref-list-lit") {
            // The pointer resting on a row of the stacked list. The argument is `<行>[:<カードの行>]`.
            const lit = arg.split(":")
            refListOpenTimer.row = lit[0] === "" ? 0 : Number(lit[0])
            refListOpenTimer.cards = false
            refListOpenTimer.litRow = lit.length > 1 ? Number(lit[1]) : 0
            refListOpenTimer.start()
        } else if (act === "ref-list-follow" || act === "ref-list-follow-lit") {
            // The name a row of that list opens under itself, pressed (デザイン規約 §グラフ行のダブルクリック) or,
            // `-lit`, rested on. The argument is `<行>[:<カードの行>]`; no row given is HEAD's.
            const follow = arg.split(":")
            refListOpenTimer.row = follow[0] === "" ? -1 : Number(follow[0])
            refListOpenTimer.cards = false
            refListOpenTimer.followRow = follow.length > 1 ? Number(follow[1]) : 0
            refListOpenTimer.start()
        } else if (act === "ref-list-choose") {
            // A Ctrl click at a row of that list moves the choice as the graph row's would
            // (デザイン規約 §複数のコミットを選ぶ). The argument is `<行>[:<カードの行>]`; the row defaults to 1 because it
            // has to be a commit other than the one read — a held press there takes nothing out of a choice of one.
            const held = arg.split(":")
            refListOpenTimer.row = held[0] === "" ? 1 : Number(held[0])
            refListOpenTimer.cards = false
            refListOpenTimer.chooseRow = held.length > 1 ? Number(held[1]) : 0
            refListOpenTimer.start()
        } else if (act === "row-card" || act === "card-sweep") {
            // Entered where the row's own ask goes in (`rowHoverRequested`, as `settlePointed` sends it). The sweep
            // defaults to row 1: its presets have a dirty tree, whose top row opens a card with no commit in it.
            const at = act === "card-sweep" && arg === "" ? 1 : Number(arg)
            const hovered = graphPane.view.itemAtIndex(at)
            if (hovered)
                graphPane.view.rowHoverRequested(hovered, true)
            if (act === "card-sweep") {
                cardSweepTimer.start()
            } else {
                rowCardTimer.oidHex = hovered ? hovered.oid_hex : ""
                rowCardTimer.row = at
                rowCardTimer.byNumber = hovered ? hovered.isWip : false
                rowCardTimer.start()
            }
        } else if (act === "row-card-return") {
            // Entered at the row's own pointer property, the one thing a real pointer writes here, so the run goes
            // through `settlePointed` — `row-card`'s door is one step further in and cannot see this. Row 1 by
            // default: the dirty tree's top row has no card (`GraphRowDelegate.partAt` answers nothing for it).
            cardReturnTimer.row = arg === "" ? 1 : Number(arg)
            cardReturnTimer.start()
        } else if (act === "card-message" || act === "card-message-esc") {
            // The note under a cut message, pressed at `CommitHoverCard.askMessage` (the click handler's own body).
            cardMessageTimer.row = arg === "" ? 0 : Number(arg)
            cardMessageTimer.escapes = act === "card-message-esc"
            cardMessageTimer.order = cardMessageTimer.escapes ? ["card_up", "asked", "escaped"] : ["card_up", "asked"]
            cardMessageTimer.start()
        } else if (act === "card-note-lit") {
            // The same note rested on, written where a real pointer writes (verify-ui スキル §hover の絵の撮り方).
            cardNoteTimer.row = arg === "" ? 0 : Number(arg)
            cardNoteTimer.start()
        } else if (act === "graph-choose-dbl") {
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
        } else if (act === "graph-choose-said" || act === "graph-choose-sweep") {
            // A row of the chosen list: its card saying the whole message (`said`), or its words dragged away
            // (`sweep`). The argument is the rows, as `graph-choose` takes them.
            acts.choose(acts.rowsArg(arg, [1, 3, 5]), false, act === "graph-choose-said" ? "said" : "sweep")
        } else if (act === "graph-choose" || act === "graph-choose-range") {
            // The first press is plain; the held ones after it move only the choice (デザイン規約 §複数のコミットを選ぶ).
            // The argument is the rows, `:`-separated — Ctrl one at a time, or `-range`'s one Shift press from the
            // first to the last, so that one wants exactly two.
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
        /// What `chosenRowTimer` goes on to once the choice stands (`said` / `sweep` / `drop` / `diff` / `dbl`); `""`
        /// stops at the choice itself.
        property string goesOn: ""
        /// The file `graph-choose-diff` opens: its argument, `src/topic.txt` by default.
        readonly property string diffPath: Harness.autoAct !== "graph-choose-diff" ? ""
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
            return acts.litRowsTo(Math.max(0, ...chooseTimer.rows))
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
        // means the words underneath kept their press; the double's own answer and the row's (`dblLed`,
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
        /// waited for.
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
        /// Two presses at the row's own functions; only the held second one is the row's, the plain first the words'.
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
    // The chip expansion, entered once the row's delegate exists; `-card` hands over to `rowCardTimer`, the others to
    // the operation that does their one thing on the list.
    PageStage {
        id: refListOpenTimer
        name: "ref_list_open"
        order: ["expanded"]
        property int row: 0
        property bool cards: false
        /// Which row of the list `list-menu` presses; -1 for the verbs that only open it.
        property int menuRow: -1
        /// Which row of the list `ref-list-lit` rests the pointer on; -1 for the verbs that leave it off the card.
        property int litRow: -1
        /// Which row of the list `ref-list-choose` presses with Ctrl held; -1 for the verbs that press none.
        property int chooseRow: -1
        /// Which row of the list `ref-list-follow` presses the name under; -1 for the verbs that press none.
        property int followRow: -1
        // `row`: the row the list was opened at, witnessed as it opened — what the operations after it read.
        /// The row asked for: `row`, or HEAD's where it is -1, which is only known once the graph has loaded.
        function rowAsked() {
            if (refListOpenTimer.row >= 0)
                return refListOpenTimer.row
            return graphModel.loading || graphModel.rowTotal === 0 ? -1 : graphModel.headRow
        }
        onTriggered: {
            const op = refListOpenTimer
            const at = op.rowAsked()
            const stacked = at < 0 ? null : graphPane.view.itemAtIndex(at)
            if (!Awaited.all("ref_list_open_row", { "row": !!stacked }))
                return
            // The press starts the operation that waits for what it opened.
            op.input("expanded", () => {
                op.witness("row", at)
                if (op.cards)
                    graphPane.view.rowHoverRequested(stacked, true)
                graphPane.view.chipExpandRequested(
                    stacked.oid_hex, stacked.index, stacked.chipItem.records, stacked.chipItem)
                if (op.cards) {
                    graphPane.view.rowHoverRequested(stacked, true)
                    // Asked back by commit (`byNumber` left false): HEAD's row has one.
                    rowCardTimer.oidHex = stacked.oid_hex
                    rowCardTimer.row = at
                    rowCardTimer.start()
                } else if (op.menuRow >= 0) {
                    listMenuTimer.start()
                } else if (op.litRow >= 0) {
                    listLitTimer.start()
                } else if (op.chooseRow >= 0) {
                    listChooseTimer.start()
                } else if (op.followRow >= 0) {
                    listFollowTimer.start()
                } else {
                    refListShownTimer.start()
                }
            })
            op.stop()
        }
    }
    // The name under a row of that list, pressed or rested on once the list is up. The landing is read off the graph —
    // the selection is the press's own bookkeeping — as the graph's row for that commit lit and laid out (`nav-jump`).
    // No `order`: a row with nothing under its name ends once the row is built.
    PageStage {
        id: listFollowTimer
        name: "list_follow"
        readonly property bool lit: Harness.autoAct === "ref-list-follow-lit"
        // Witnessed at the presses: where the row's name goes (`to`), once the row is built; whether the rows under
        // the card were told the rest of the gesture is not theirs (`hushed`), at the press — the hush lasts one
        // double-click window, and a busy machine reads the landing later.
        function said() {
            return " row=" + refListOpenTimer.witnessed("row") + " card=" + refListOpenTimer.followRow
        }
        /// The graph's row for the commit the name goes to, laid out where it stands.
        function landing() {
            const want = listFollowTimer.witnessed("to").oid
            const landedAt = graphModel.rowOf(want)
            const item = landedAt >= 0 ? graphPane.view.itemAtIndex(landedAt) : null
            return { "list": !refList.opened,
                     "selected": page.selectedOid === want,
                     "card": driver.cardSettled,
                     "row": landedAt < 0 || !!item }
        }
        onTriggered: {
            const op = listFollowTimer
            const at = refListOpenTimer.followRow
            if (op.at("start")) {
                // A row the list has not built is not a row with nothing under it: sent into view until it is.
                const built = op.input("row_built", () => {
                    if (!refList.opened || !refList.rowAt(at))
                        return false
                    op.witness("to", refList.mateTo(at))
                    return true
                })
                if (!built)
                    return
            }
            if (op.at("row_built")) {
                if (op.witnessed("to") === null) {
                    // Nothing under that name to press: that is the answer.
                    op.complete({ "name": "list_follow_none", "when": () => ({}),
                                  "report": () => (op.lit ? "ref_list_follow_lit" : "ref_list_follow") + op.said()
                                                  + " followed=false list=" + refList.opened })
                    return
                }
                if (op.lit) {
                    // A real hand on the name is on its row too and lights both; the band is judged against that wash.
                    op.input("pointed", () => {
                        refList.pointRow(at)
                        refList.pointMate(at)
                    })
                    op.complete({ "name": "list_follow_pointed", "when": () => ({}), "report": () => {
                        const row = acts.listRow(refListOpenTimer.followRow)
                        return "ref_list_follow_lit" + op.said()
                               + " list=" + refList.opened
                               + " aimed=" + (row ? row.chip.mateLit : false)
                               + " to=" + op.witnessed("to").key
                    } })
                    return
                }
                op.input("followed", () => {
                    refList.followRow(at)
                    op.witness("hushed", graphPane.rowClicksHushed)
                })
                return
            }
            op.complete({
                "name": "list_follow_landed",
                "when": () => op.landing(),
                "report": () => {
                    const want = op.witnessed("to").oid
                    const now = graphModel.rowOf(want)
                    const landed = now >= 0 ? graphPane.view.itemAtIndex(now) : null
                    return "ref_list_follow row=" + refListOpenTimer.witnessed("row")
                           + " card=" + refListOpenTimer.followRow
                           + " followed=true list=" + refList.opened
                           + " landed=" + (!!landed && landed.oid_hex === want && landed.selected)
                           + " hushed=" + op.witnessed("hushed")
                           + " to=" + op.witnessed("to").key + " at=" + now
                } })
        }
    }
    // The held click on a row of that list once it is up; the rows draw the choice a beat behind the press.
    PageStage {
        id: listChooseTimer
        name: "list_choose"
        order: ["chosen"]
        // `readOid`: the commit read before the press, witnessed at it, for the report's `read=`.
        onTriggered: {
            const op = listChooseTimer
            if (op.at("start")) {
                if (!Awaited.all("list_choose_ready", { "ready": refList.opened }))
                    return
                op.input("chosen", () => {
                    const read = page.selectedOid
                    if (!refList.chooseRow(refListOpenTimer.chooseRow, Qt.ControlModifier))
                        return false
                    op.witness("readOid", read)
                    return true
                })
                return
            }
            // `selectionLoaded` for the reason `chooseTimer` gives.
            op.complete({
                "name": "list_choose_landed",
                "when": () => ({
                    "ready": refList.opened,
                    "chosen": page.chosenCount === 2,
                    "lit": acts.litRowsTo(refListOpenTimer.witnessed("row")) === 2,
                    "read": detailsModel.selectionLoaded,
                    "page": PageSettled.settled(page) }),
                "report": () => "ref_list_choose chosen=" + page.chosenCount
                                + " lit=" + acts.litRowsTo(refListOpenTimer.witnessed("row"))
                                + " read=" + (page.selectedOid === op.witnessed("readOid"))
                                + " list=" + refList.opened
                                + " row=" + refListOpenTimer.witnessed("row") })
        }
    }
    // The right-click on a row of that list, once it is up (an unopened popup has no rows).
    PageStage {
        id: listMenuTimer
        name: "list_menu"
        order: ["menu_raised"]
        onTriggered: {
            const op = listMenuTimer
            if (!Awaited.all("list_menu_ready", { "ready": refList.opened }))
                return
            if (!op.input("menu_raised", () => refList.menuRow(refListOpenTimer.menuRow)))
                return
            // `list=` is read with the menu up: the card has to still stand under it (デザイン規約 §メニュー の例外).
            // `worktree=` / `branch=` / `tag=`: which card the named row brought up is what this gesture decides. The
            // WORKTREE card stands on every commit (`Create worktree here…`), so `worktree=` is whether it stands on a
            // worktree.
            op.complete({ "name": "list_menu_raised", "when": () => ({}),
                          "report": () => "list_menu list=" + refList.opened
                                          + " menu=" + commitMenu.opened
                                          + " worktree=" + (driver.commitWorktreeCard.path !== "")
                                          + " branch=" + commitBranchCard.applies
                                          + " tag=" + commitTagCard.applies
                                          + " rows=" + commitMenu.offeredRows })
        }
    }
    PageStage {
        id: listLitTimer
        name: "list_lit"
        order: ["pointed"]
        onTriggered: {
            const op = listLitTimer
            if (!Awaited.all("list_lit_ready", { "ready": refList.opened }))
                return
            if (!op.input("pointed", () => refList.pointRow(refListOpenTimer.litRow)))
                return
            // `lit=` is read off the row: the wash is one shade over the card's ground. `nowhere=`: only a row with
            // nowhere to go has no name colour to stand in for its wash.
            op.complete({ "name": "list_lit_pointed", "when": () => ({}), "report": () => {
                const row = acts.listRow(refListOpenTimer.litRow)
                return "ref_list_lit row=" + refListOpenTimer.litRow
                       + " list=" + refList.opened
                       + " lit=" + (row ? row.lit : false)
                       + " nowhere=" + (row ? row.leadsNowhere : false)
            } })
        }
    }
    // The bare verb's own end: the list is up (an unopened popup frames as the plain screen).
    PageStage {
        id: refListShownTimer
        name: "ref_list_shown"
        onTriggered: refListShownTimer.complete({
            "name": "ref_list_shown_ready",
            "when": () => ({ "ready": refList.opened,
                             "row": !!graphPane.view.itemAtIndex(refListOpenTimer.witnessed("row")) }),
            "report": () => {
                const now = graphPane.view.itemAtIndex(refListOpenTimer.witnessed("row"))
                return "ref_list " + (now === null ? "covers=false" : acts.coverOf(now.chipItem.chipItem))
                       + " row=" + refListOpenTimer.witnessed("row")
            } })
    }
    // The view is sent to its end until the stand-in is up on the top edge (as `graph-head` does); then the pointer
    // goes onto its chip and the card is waited for on that chip, since a card on a row's chip is up too.
    PageStage {
        id: headListTimer
        name: "graph_head_list"
        property bool click: false
        // Witnessed at the press: the commit it lands on (`want`), and the hush as `listFollowTimer`'s (`hushed`).
        // Witnessed by the probe after it, as it went: the list as the press left it (`listAfter`), the first row in
        // view (`underAt`), the part of it the still pointer is on (`part`), whether that opened anything (`opened`,
        // `held`), and the pane's hold on the hand (`holding`).
        function line() {
            const pin = graphPane.headPin
            const at = graphModel.headRow
            const item = at >= 0 ? graphPane.view.itemAtIndex(at) : null
            return "graph_head_list_click list=" + headListTimer.witnessed("listAfter")
                   // The row read, lit, and the one the keyboard walks from — all three are the press's.
                   + " landed=" + (item !== null && item.oid_hex === headListTimer.witnessed("want") && item.selected
                                   && graphPane.view.currentIndex === at)
                   + " pin=" + pin.visible
                   + " hushed=" + headListTimer.witnessed("hushed")
                   + " held=" + headListTimer.witnessed("held")
                   + " at=" + at
                   // What `held=` is made of, for a reader of a red run.
                   + " under=" + headListTimer.witnessed("underAt") + " part=" + headListTimer.witnessed("part")
                   + " opened=" + headListTimer.witnessed("opened")
                   + " holding=" + headListTimer.witnessed("holding")
        }
        onTriggered: {
            const op = headListTimer
            const pin = graphPane.headPin
            // Pressed: the landing is read off the graph — the stand-in's row laid out and lit, the card gone.
            if (op.at("pressed")) {
                const at = graphModel.headRow
                const item = at >= 0 ? graphPane.view.itemAtIndex(at) : null
                if (!Awaited.all("graph_head_list_landed", {
                    "list": !refList.opened,
                    "selected": page.selectedOid === op.witnessed("want"),
                    "card": driver.cardSettled,
                    "row": item !== null,
                    "page": PageSettled.settled(page) }))
                    return
                // The row now where the stand-in stood is told the still pointer is on its chip and must open nothing
                // — the hand it came under did not walk to it (`HandWatch.rowsMoved`, which the press sent; the seat
                // keeps the ask for the hand's first walk) — the chip's half, where a card opens at once, is the one
                // that can go wrong.
                op.input("probed", () => {
                    op.witness("listAfter", refList.opened)
                    // The first row in view: at the head of the list `contentY` sits in the top margin, above row 0.
                    const underAt = graphPane.view.indexAt(1, Math.max(0, graphPane.view.contentY) + 1)
                    const under = underAt >= 0 ? graphPane.view.itemAtIndex(underAt) : null
                    if (under)
                        under.pointerRowX = under.labelsW / 2
                    op.witness("underAt", underAt)
                    const part = op.witness("part", under ? under.pointedPart : "")
                    const opened = op.witness("opened", refList.opened || rowHost.listPointed)
                    op.witness("held", part === "chip" && !opened)
                    if (under)
                        under.pointerRowX = -1
                    op.witness("holding", graphPane.rowHandHeld)
                })
                op.complete({ "name": "graph_head_list_probed", "when": () => ({}), "report": () => op.line() })
                return
            }
            if (!Awaited.all("graph_head_list_graph", {
                    "graph": !graphModel.loading && graphModel.rowTotal !== 0, "wanted": pin.wanted }))
                return
            // Off HEAD first, once: the page opens on HEAD, so a press that picked nothing would still read green.
            if (op.click && op.at("start")) {
                const off = graphModel.headRow + 1
                if (!Awaited.all("graph_head_list_off", { "below": off < graphModel.rowTotal }))
                    return
                op.input("moved_off", () => {
                    page.activateRow(graphModel.oidAt(off), off)
                })
                return
            }
            if (op.at("start") || op.at("moved_off")) {
                // The view sent to its end until the stand-in stands up on the top edge.
                const up = op.input("pin_up", () => {
                    if (!(pin.visible && pin.rowAbove))
                        graphPane.view.positionViewAtEnd()
                    return pin.visible && pin.rowAbove
                })
                if (!up)
                    return
            }
            if (op.at("pin_up")) {
                // The middle of the front card, where a hand reaching for the name lands — put on the pane's hand
                // (`HandWatch.stir`), which the stand-in reads it off, so the press below has a hand to hold still.
                op.input("pointed", () => {
                    const front = pin.chipItem.chipItem
                    graphPane.hand.stir(front.mapToItem(null, front.width / 2, front.height / 2))
                })
                return
            }
            if (op.click) {
                if (!Awaited.all("graph_head_list_open", { "list": refList.opened,
                                                           "on": rowHost.listAnchor === pin.chipItem }))
                    return
                // In at the card's own row, the way a hand presses it (`RefListPopup.clickRow`).
                op.input("pressed", () => {
                    const want = graphModel.oidAt(graphModel.headRow)
                    if (!refList.clickRow(0))
                        return false
                    op.witness("want", want)
                    op.witness("hushed", graphPane.rowClicksHushed)
                    return true
                })
                return
            }
            // `on=` is the stand-in's own "is the card on me", which takes its sheets down; `covers=` is read after.
            op.complete({
                "name": "graph_head_list_open",
                "when": () => ({ "list": refList.opened, "on": rowHost.listAnchor === graphPane.headPin.chipItem }),
                "report": () => "graph_head_list shown=" + graphPane.headPin.visible
                                + " list=" + refList.opened
                                + " on=" + graphPane.headPin.listOnThisChip
                                + " " + acts.coverOf(graphPane.headPin.chipItem.chipItem)
                                + " above=" + graphPane.headPin.rowAbove
                                + " names=" + graphPane.headPin.records.length })
        }
    }
    // Rests on the row until the card is up (`tipDelayMs` after the rest), then makes the return in one turn.
    PageStage {
        id: cardReturnTimer
        name: "card_return"
        order: ["rested", "returned"]
        /// The row the hand rests on, leaves and comes back to.
        property int row: 0
        // Witnessed: the pointer x that put the card out the first time (`firstX`) — the return is at another x, and
        // the seat follows the pointer, so only then would a reopened card move; and the return read in its own turn:
        // the hold (`held`), the card (`cardOpen`), whether it moved (`moved`), and whose card it is (`sameOid`).
        onTriggered: {
            const op = cardReturnTimer
            const at = graphPane.view.itemAtIndex(op.row)
            if (!Awaited.all("card_return_ready", { "ready": at }))
                return
            // The rest, and then the wait it opens after, written again each beat until the card is out. Both are the
            // row's own (`GraphRowDelegate.settlePointed`).
            if (op.at("start")) {
                const rested = op.input("rested", () => {
                    const x = Math.round(at.width / 3)
                    at.pointerRowX = x
                    if (!rowCard.opened)
                        return false
                    op.witness("firstX", x)
                    return true
                })
                if (!rested)
                    return
            }
            // Off the row into the card and back a little further along, read in the same turn: pointer to hold is one
            // synchronous stretch, so the hold is back before the beat can start or not at all
            // (rules-refs/app-ui.md「動詞の前提条件は入力を出す枝で読む」).
            op.input("returned", () => {
                const seat = rowCard.x
                at.pointerRowX = -1
                at.pointerRowX = op.witnessed("firstX") * 2
                op.witness("held", rowHost.cardPointed)
                op.witness("cardOpen", rowCard.opened)
                op.witness("moved", rowCard.x !== seat)
                op.witness("sameOid", rowHost.cardOid === at.oid_hex)
            })
            op.complete({ "name": "card_returned", "when": () => ({}),
                          "report": () => "row_card_return held=" + op.witnessed("held")
                                          + " open=" + op.witnessed("cardOpen")
                                          + " moved=" + op.witnessed("moved")
                                          + " oid=" + op.witnessed("sameOid") })
        }
    }
    // The card is opened synchronously; this just lets the layout settle before it is measured and photographed.
    PageStage {
        id: rowCardTimer
        name: "row_card"
        /// The commit the card was asked of, so its row can be asked back whether it is lit — by commit, as the row
        /// answers (`GraphRowDelegate.cardOnThisRow`): another worktree's rows land on a pass of their own and can
        /// shift the row number between the hover and this read.
        property string oidHex: ""
        /// The row number instead, for a row with no commit of its own (`byNumber`): the worktree's and every
        /// other worktree's row share the all-zero id.
        property int row: 0
        property bool byNumber: false
        /// The row the card was asked of, as it stands now.
        function rowNow() {
            return rowCardTimer.byNumber ? rowCardTimer.row : graphModel.rowOf(rowCardTimer.oidHex)
        }
        /// Something out, and the row standing: a pass that moved the row lays its delegates out again, so the row can
        /// be a beat from standing.
        function ready() {
            const row = rowCardTimer.rowNow()
            const asked = row >= 0 ? graphPane.view.itemAtIndex(row) : null
            return { "ready": !(!rowCard.opened && !refList.opened), "row": row < 0 || !!asked }
        }
        onTriggered: rowCardTimer.complete({
            "name": "row_card_ready",
            "when": () => rowCardTimer.ready(),
            "report": () => {
                const at = rowCardTimer.rowNow()
                const now = at >= 0 ? graphPane.view.itemAtIndex(at) : null
                // `lit=` next to `open=`: the pair is judged as one unbroken stretch (`verify::verbs::must_say`).
                return "row_card open=" + rowCard.opened
                       + " lit=" + (now ? now.lit : false)
                       + " credit=" + Math.round(rowCard.creditWidth)
                       + " cut=" + rowCard.creditCut
                       + " list=" + refList.opened
                       + " subject=" + (rowCard.subject !== "")
                       + " body=" + (rowCard.body !== "")
            } })
    }
    // The press on that card's note, and where it leaves the reader; the shot is of the arrival.
    PageStage {
        id: cardMessageTimer
        name: "card_message"
        property int row: 0
        /// Whether the run goes on to put the mark away with Escape, entered at the key handler's body
        /// (`RepoPage.escapePressed`); the key reaching it is held by `tests/qml/tst_escape.qml`.
        property bool escapes: false
        // Witnessed at the presses: which commit the press was of (`oidHex`), kept because the card takes its own copy
        // down with it; whether this page answered the Escape (`took`).
        /// The arrival: the card gone and the pane holding that commit's message — a shot before it frames whatever
        /// was open before.
        function arrived() {
            return { "card": !rowCard.opened,
                     "read": detailsPane.details.shaHex === cardMessageTimer.witnessed("oidHex") }
        }
        onTriggered: {
            const op = cardMessageTimer
            const hovered = graphPane.view.itemAtIndex(op.row)
            if (op.at("start")) {
                if (!Awaited.all("card_message_row", { "row": !!hovered }))
                    return
                // Asked of the row each beat until the card is up with the note: it stands only under a cut message
                // (規約 §hover のツールチップ), and a card fills its fields in after it opens.
                const up = op.input("card_up", () => {
                    graphPane.view.rowHoverRequested(hovered, true)
                    return rowCard.opened && rowCard.messageCut
                })
                if (!up)
                    return
            }
            if (op.at("card_up")) {
                op.input("asked", () => {
                    op.witness("oidHex", hovered.oid_hex)
                    rowCard.askMessage()
                })
                return
            }
            // Its own sentence: the two runs are judged on opposite answers, each as one unbroken stretch of a line.
            if (op.escapes) {
                if (!Awaited.all("card_message_arrived", op.arrived()))
                    return
                op.input("escaped", () => {
                    op.witness("took", page.escapePressed())
                })
                op.complete({ "name": "card_message_escaped", "when": () => ({}),
                              "report": () => "card_message_esc mark=" + detailsPane.attention
                                              + " took=" + op.witnessed("took")
                                              + " picked=" + (page.selectedRow === op.row) })
                return
            }
            op.complete({ "name": "card_message_arrived", "when": () => op.arrived(),
                          "report": () => "card_message open=" + rowCard.opened
                                          + " picked=" + (page.selectedRow === op.row)
                                          + " mark=" + detailsPane.attention
                                          + " shown=" + (detailsPane.boxSubject !== "")
                                          + " row=" + page.selectedRow })
        }
    }
    // The hand resting on that note: put in once the note is there (as the press waits), read a beat later off the
    // paint.
    PageStage {
        id: cardNoteTimer
        name: "card_note"
        order: ["card_up", "pointed"]
        property int row: 0
        onTriggered: {
            const op = cardNoteTimer
            if (op.at("start")) {
                const hovered = graphPane.view.itemAtIndex(op.row)
                if (!Awaited.all("card_note_row", { "row": !!hovered }))
                    return
                // Asked of the row each beat until the card is up with the note under its cut message.
                const up = op.input("card_up", () => {
                    graphPane.view.rowHoverRequested(hovered, true)
                    return rowCard.opened && rowCard.messageCut
                })
                if (!up)
                    return
            }
            if (op.at("card_up")) {
                op.input("pointed", () => {
                    rowCard.notePointedAt = true
                })
                return
            }
            // `lit=` is the rule (the line under the words is the words' colour); `word=` pins the step, which `lit=`
            // alone cannot (規約 §hover のツールチップ).
            op.complete({ "name": "card_note_pointed", "when": () => ({ "pointed": rowCard.notePointedAt }),
                          "report": () => "card_note lit=" + Qt.colorEqual(rowCard.noteRuleColor, rowCard.noteWordColor)
                                          + " word=" + rowCard.noteWordColor
                                          + " rule=" + rowCard.noteRuleColor })
        }
    }
    // The same card's words taken from the air around them (規約 §hover のツールチップ); the starts are the air itself,
    // sampled (`SweepPad.airPoints`).
    PageStage {
        id: cardSweepTimer
        name: "card_sweep"
        order: ["swept"]
        // `swept`: what the sweep came away with, witnessed as it swept.
        onTriggered: {
            const op = cardSweepTimer
            // The commit's words first: a card fills in after it opens, and one swept before hands back `1970-01-01`.
            if (!Awaited.all("card_sweep_ready", { "ready": !(!rowCard.opened || rowCard.subject === "") }))
                return
            // And stopped laying out: mid-layout its fields are at some other width (the wait `details-sweep` takes).
            const geom = Math.round(rowCard.width) + "x" + Math.round(rowCard.height)
            if (!Awaited.all("card_sweep_still", { "geom": op.still("card_sweep_geom", geom) }))
                return
            op.input("swept", () => {
                op.witness("swept", rowCard.background.pad.sweepAir(9, "open=" + rowCard.opened))
            })
            op.complete({ "name": "card_swept", "when": () => ({}),
                          "report": () => "card_sweep " + op.witnessed("swept") })
        }
    }
    // Waits for either card: a moved boundary opens the other, and waiting for the right one would run to the watchdog.
    PageStage {
        id: rowPartTimer
        name: "row_part"
        order: ["pointed"]
        // The point asked along the row and the card expected of it, for the report.
        property int row: 0
        property string want: ""
        property real x: 0
        onTriggered: {
            const op = rowPartTimer
            if (op.at("start")) {
                const item = graphPane.view.itemAtIndex(op.row)
                if (!Awaited.all("row_part_row", { "row": !!item }))
                    return
                op.input("pointed", () => {
                    item.pointerRowX = op.x
                })
                return
            }
            op.complete({
                "name": "row_part_card",
                "when": () => ({ "card": refList.opened || rowCard.opened }),
                "report": () => {
                    const got = refList.opened ? "chip" : "row"
                    return "row_part x=" + op.x
                           + " want=" + op.want
                           + " got=" + got
                           + " list=" + refList.opened
                           + " card=" + rowCard.opened
                           + " agrees=" + (got === op.want && refList.opened !== rowCard.opened)
                } })
        }
    }
    // PGG_AUTO_ACT=ref-list-leave. Each stage goes in once the one before has landed, and the last waits out the beat
    // the leave started (`RowHoverHost.listSettling`) — a leave that starts none is read at once, which is the list
    // left standing with no hand near it. No `order`: a row whose chip leaves no bare stretch ends once it is in view.
    PageStage {
        id: leaveTimer
        name: "ref_list_leave"
        /// Where the hand comes on (a row number or `head`), and where it goes after (`off` / `back` / `menu` / a row
        /// number).
        property string from: ""
        property string to: ""
        /// The hand goes on to another row's column (`to` is its number) rather than staying about this one.
        function crosses() {
            return leaveTimer.to !== "off" && leaveTimer.to !== "back" && leaveTimer.to !== "menu"
        }
        // Witnessed at the presses: the point along the row the hand came on at (`handX`), and whether the card that
        // opened lies over it (`covered`) — read off the scene's two boxes, so a card that covers the point makes the
        // run say so rather than pass for another; the card's box as it first stood (`firstBox`) — a hand come back
        // holds the list rather than laying it out again, and the list laid out again under its unstacked chip is
        // measured off a chip without its fan; the ask and the beat as they stood in the turn of the leave
        // (`heldAfter`, `settlingAfter`).
        function boxOf() {
            const face = refList.background
            const at = face.mapToItem(null, 0, 0)
            return Math.round(at.x) + "," + Math.round(at.y)
                   + " " + Math.round(face.width) + "x" + Math.round(face.height)
        }
        /// Where the hand is along `item` — the middle of the stretch its chip's card will leave bare, left of where
        /// the card is placed (`RowHoverHost.openRefList`); -1 where the chip leaves none.
        function bareX(item) {
            const room = item.chipItem.mapToItem(item, 0, 0).x - Theme.spaceXs - Theme.borderWidth
            return room >= 2 * Theme.spaceXs ? Math.floor(room / 2) : -1
        }
        /// The row or stand-in called `which`, or null while it is not laid out.
        function surface(which) {
            if (which === "head")
                return graphPane.headPin
            return graphPane.view.itemAtIndex(Number(which))
        }
        /// Where the hand is held across `item`: a row's middle, the stand-in's row line.
        function handY(item) {
            return item === graphPane.headPin ? item.rowMidY : item.height / 2
        }
        function coveredAt(item, x) {
            const face = refList.background
            const card = face.mapToItem(null, 0, 0)
            const hand = item.mapToItem(null, x, leaveTimer.handY(item))
            return hand.x >= card.x && hand.x < card.x + face.width
                   && hand.y >= card.y && hand.y < card.y + face.height
        }
        /// Sends the row or stand-in the hand comes on into view; answers whether it stands there.
        function bringOn() {
            const item = leaveTimer.surface(leaveTimer.from)
            if (leaveTimer.from === "head" && item.wanted && (!item.visible || !item.rowAbove)) {
                graphPane.view.positionViewAtEnd()
                return false
            }
            if (leaveTimer.from !== "head" && item === null) {
                graphPane.view.positionViewAtIndex(Number(leaveTimer.from), ListView.Contain)
                return false
            }
            return true
        }
        onTriggered: {
            const op = leaveTimer
            if (!Awaited.all("ref_list_leave_graph", { "graph": !graphModel.loading && graphModel.rowTotal !== 0 }))
                return
            const item = op.surface(op.from)
            // Sent into view each beat until it stands there.
            if (op.at("start") && !op.input("shown", () => op.bringOn()))
                return
            if (op.at("shown")) {
                const other = op.crosses() ? op.surface(op.to) : null
                if (!Awaited.all("ref_list_leave", {
                        "from": item !== null && item.visible && item.chipItem.visible,
                        "to": !op.crosses() || (other !== null && other.visible && other.chipItem.visible)
                    }))
                    return
                const handX = op.bareX(item)
                if (handX < 0 || (other !== null && op.bareX(other) < 0)) {
                    op.complete({ "name": "ref_list_leave_bare", "when": () => ({}),
                                  "report": () => "ref_list_leave to=" + op.to + " bare=false" })
                    return
                }
                op.input("came_on", () => {
                    op.witness("handX", handX)
                    item.pointerCrossed(true, handX)
                })
                return
            }
            if (op.at("came_on")) {
                // A beat after the ask, so the card stands where it was placed.
                if (!Awaited.all("ref_list_leave", {
                        "list": refList.opened, "on": rowHost.listAnchor === item.chipItem }))
                    return
                // The card as it stood when the hand went: whether it lay over the hand, and its box.
                const covered = op.coveredAt(item, op.witnessed("handX"))
                const box = op.boxOf()
                if (op.to === "menu") {
                    // Into the card, and a right-click on its first row: that row's menu stands on the list
                    // (`RepoPage.menuArea` = `refList`). Not built yet: no press has gone in.
                    op.input("menu_raised", () => {
                        if (!refList.menuRow(0))
                            return false
                        op.witness("covered", covered)
                        op.witness("firstBox", box)
                        item.pointerCrossed(false, 0)
                        return true
                    })
                    return
                }
                op.input("left", () => {
                    op.witness("covered", covered)
                    op.witness("firstBox", box)
                    if (op.to === "off") {
                        item.pointerCrossed(false, 0)
                    } else if (op.to === "back") {
                        item.pointerCrossed(false, 0)
                        item.pointerCrossed(true, op.witnessed("handX"))
                    } else {
                        // Qt tells the row the hand came onto before the row it left (`deliverHoverEvent`: the rows
                        // under the point, then the ones no longer under it).
                        const other = op.surface(op.to)
                        other.pointerCrossed(true, op.bareX(other))
                        item.pointerCrossed(false, 0)
                    }
                    op.witness("heldAfter", rowHost.listPointed)
                    op.witness("settlingAfter", rowHost.listSettling)
                })
                return
            }
            if (op.at("menu_raised")) {
                // The hand back on the chip's column while the menu stands, and the menu let go of as Escape does:
                // its close lets the menu's hold go and the hand decides again (`RowHoverHost.menuOnList`).
                if (!Awaited.all("ref_list_leave", { "menu": commitMenu.opened }))
                    return
                op.input("back_on", () => {
                    item.pointerCrossed(true, op.witnessed("handX"))
                    op.witness("heldAfter", rowHost.listPointed)
                    op.witness("settlingAfter", rowHost.listSettling)
                    commitMenu.dismiss()
                })
                return
            }
            op.complete({
                "name": "ref_list_leave",
                "when": () => ({ "menu": !commitMenu.visible, "beat": !rowHost.listSettling }),
                "report": () => {
                    const came = op.surface(op.from)
                    const target = op.to === "off" ? null
                                 : op.crosses() ? op.surface(op.to) : came
                    const firstBox = op.witnessed("firstBox")
                    return "ref_list_leave to=" + op.to
                           + " covered=" + op.witnessed("covered")
                           + " list=" + refList.opened
                           + " on=" + (target !== null && rowHost.listAnchor === target.chipItem)
                           + " same=" + (op.boxOf() === firstBox)
                           // How the leave was taken, for a reader of a red run.
                           + " held=" + op.witnessed("heldAfter")
                           + " settling=" + op.witnessed("settlingAfter")
                           + " x=" + op.witnessed("handX")
                           + " box=" + firstBox + "->" + op.boxOf()
                } })
        }
    }
}
