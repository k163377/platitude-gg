pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The graph's rows: the card a row puts out, the chips' own list, the parts a row is made of, and the
/// gesture that names one — the two clicks at a row, the same pair put in through the strip over the lane
/// column, and the pair (with the double-click that moves beside it) put in at a row of the card a chip
/// unfolds into.
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
    readonly property var workTree: driver.workTree
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

    /// How many rows draw themselves chosen — **the output side** of a press that moves the choice. Counted from the
    /// top to past `last`, the last row the run pressed or stood on, so a row lit that should not be is counted with
    /// the ones that should.
    function litRowsTo(last) {
        let lit = 0
        for (let i = 0; i < Math.min(graphModel.rowTotal, last + 3); i++) {
            const item = graphPane.view.itemAtIndex(i)
            if (item && item.selected)
                lit++
        }
        return lit
    }

    /// Runs `act` if it is one of this family's, and says whether it was. The families are asked in turn
    /// and the first to know a verb runs it — each verb is named by one (`AutoActDriver`).
    function run(act, arg) {
        if (act === "row-part") {
            // Where the row divides, asked at a point along it. Hover cannot be injected, so this writes the one
            // property a real pointer writes (`GraphRowDelegate.pointerRowX`) and leaves every decision after that to
            // the row — **the point of the verb is the decision**, so reaching past it to `chipExpandRequested` (which
            // is what `ref-list` does) would prove nothing about the boundary. The probe runs from the sampler, so an
            // early `itemAtIndex` miss retries.
            const parts = arg.split(":")
            rowPartReport.row = Number(parts[0])
            rowPartReport.want = parts[2]
            rowPartReport.x = Number(parts[1])
            rowPartTimer.start()
        } else if (act === "list-menu") {
            // The stacked list, and then the right-click on one of its rows: the row's own menu, aimed at the name
            // that was pressed (デザイン規約 §グラフ行の右クリック). The
            // argument is `<行>[:<カードの行>]`, the same shape `graph-reclick-list` takes.
            const at = arg.split(":")
            refListOpenTimer.row = at[0] === "" ? 0 : Number(at[0])
            refListOpenTimer.cards = false
            refListOpenTimer.menuRow = at.length > 1 ? Number(at[1]) : 0
            refListOpenTimer.start()
        } else if (act === "ref-list" || act === "ref-list-card") {
            // Hover cannot be injected, so this enters where the hover timer would — from the sampler, so an early
            // `itemAtIndex` miss retries. `-card` walks row → card → chip → asked again from under the list: both
            // card closes have to hold, and either failing leaves `open=true`.
            refListOpenTimer.row = Number(arg)
            refListOpenTimer.cards = act === "ref-list-card"
            refListOpenTimer.start()
        } else if (act === "ref-list-lit") {
            // The stacked list with the pointer resting on one of its rows. The hand that walked down off the chip is
            // the only way a reader ever reads these names, so the wash it brings is part of the card's own picture.
            // The argument is `<行>[:<カードの行>]`, the same shape `list-menu` takes.
            const lit = arg.split(":")
            refListOpenTimer.row = lit[0] === "" ? 0 : Number(lit[0])
            refListOpenTimer.cards = false
            refListOpenTimer.litRow = lit.length > 1 ? Number(lit[1]) : 0
            refListOpenTimer.start()
        } else if (act === "ref-list-choose") {
            // A held click put in at a row of that list: the card's rows are the graph's row, so Ctrl there moves the
            // choice the way the row does and never reaches the name box or the switch (デザイン規約 §複数のコミットを
            // 選ぶ). The argument is `<行>[:<カードの行>]`, the same shape `ref-list-lit` takes; the row has to be
            // a commit other than the one read — a held press on that one takes nothing out of a choice of one.
            const held = arg.split(":")
            refListOpenTimer.row = held[0] === "" ? 1 : Number(held[0])
            refListOpenTimer.cards = false
            refListOpenTimer.chooseRow = held.length > 1 ? Number(held[1]) : 0
            refListOpenTimer.start()
        } else if (act === "row-card" || act === "card-sweep") {
            // Hover cannot be injected, so this enters where the row's delay timer would. **The sweep's own default is
            // row 1**: the presets it runs on carry a dirty working tree, whose row stands at the top and
            // opens a card with no commit in it (measured — `subject` empty, the stamp `1970-01-01`).
            const at = act === "card-sweep" && arg === "" ? 1 : Number(arg)
            const hovered = graphPane.view.itemAtIndex(at)
            if (hovered)
                graphPane.view.rowHoverRequested(hovered, true)
            if (act === "card-sweep") {
                cardSweepTimer.start()
            } else {
                rowCardTimer.row = at
                rowCardTimer.start()
            }
        } else if (act === "row-card-return") {
            // The hand that walked down into the card and came back to the row it came off. **Entered at the row's
            // own pointer property**, which is the one thing a real pointer writes here (`onPositionChanged` /
            // `onContainsMouseChanged` write nothing else), so the run goes through `settlePointed`
            // — the door `row-card` uses is one step further in and cannot see this at all.
            // **Row 1 by default**: the presets this runs on carry a dirty working tree, whose row stands
            // at the top and has no card at all (`GraphRowDelegate.partAt` answers nothing for it).
            cardReturnTimer.row = arg === "" ? 1 : Number(arg)
            cardReturnTimer.start()
        } else if (act === "card-message" || act === "card-message-esc") {
            // The note under a message the card had to stop, pressed where a hand presses it
            // (`CommitHoverCard.askMessage` is the click handler's own body). The card is opened the way `row-card`
            // opens it, and the press waits for the note to be there — the note only stands under a cut message, and
            // a card fills its fields in after it is opened.
            cardMessageTimer.row = arg === "" ? 0 : Number(arg)
            cardMessageTimer.escapes = act === "card-message-esc"
            cardMessageTimer.asked = false
            cardMessageTimer.start()
        } else if (act === "card-note-lit") {
            // The same note with the hand resting on it. Written at the property a real pointer writes —
            // hover cannot be injected (verify-ui スキル §hover の絵の撮り方) — and read back off the paint.
            cardNoteTimer.row = arg === "" ? 0 : Number(arg)
            cardNoteTimer.pointed = false
            cardNoteTimer.start()
        } else if (act === "menu-hover") {
            // Same row and same default as `commit-menu`: the menu goes up, and then the row it is standing on is
            // asked for its hover card.
            let hoverOid = arg
            if (hoverOid === "")
                hoverOid = graphModel.oidAt(graphModel.rowOf(workTree.headOid) + 1)
            page.openRowMenu(hoverOid)
            menuHoverTimer.oidHex = hoverOid
            menuHoverTimer.asked = false
            menuHoverTimer.start()
        } else if (act === "graph-choose-dbl") {
            // **The gesture a choice tells apart.** Toggling a row out of the choice and back in, quickly, is
            // two presses at one spot — which Qt hands over as a double-click, modifier and all (measured,
            // `tst_moddblclick`). On the other side of the plain double-click is `switch`, so the row has to tell the
            // two apart (デザイン規約 §複数のコミットを選ぶ). The argument is the rows, as `graph-choose` takes them.
            const dblRows = arg === "" ? [] : arg.split(":").map(Number)
            chooseTimer.sweeps = false
            chooseTimer.rows = dblRows.length >= 2 ? dblRows : [1, 3, 5]
            chooseTimer.step = 0
            chooseTimer.readOid = ""
            chooseTimer.after = "dbl"
            chooseTimer.start()
        } else if (act === "graph-choose-diff") {
            // What a row of the merged file list opens: **each chosen commit's own patch of that file,
            // stacked** (デザイン規約 §複数のコミットを選ぶ). The argument is the path, the rows being
            // fixed at three that share one file (`--preset basic`'s `src/topic.txt`, touched by two of them).
            chooseTimer.sweeps = false
            chooseTimer.rows = [3, 5, 6]
            chooseTimer.step = 0
            chooseTimer.readOid = ""
            chooseTimer.after = "diff"
            chooseTimer.diffPath = arg === "" ? "src/topic.txt" : arg
            chooseTimer.start()
        } else if (act === "graph-choose-drop") {
            // A commit taken back out from the list that shows what is held — the same Ctrl the graph takes one out
            // with (デザイン規約 §複数のコミットを選ぶ). The argument is the rows, as `graph-choose` takes them.
            const kept = arg === "" ? [] : arg.split(":").map(Number)
            chooseTimer.sweeps = false
            chooseTimer.rows = kept.length >= 2 ? kept : [1, 3, 5]
            chooseTimer.step = 0
            chooseTimer.readOid = ""
            chooseTimer.after = "drop"
            chooseTimer.start()
        } else if (act === "graph-choose-said" || act === "graph-choose-sweep") {
            // The two things a row of that list is for, once the choice is standing: saying the whole of what the
            // commit says, and letting a reader drag the words away (デザイン規約 §複数のコミットを選ぶ). The choice is
            // built by the same presses `graph-choose` makes, so the argument is the same rows.
            const held = arg === "" ? [] : arg.split(":").map(Number)
            chooseTimer.sweeps = false
            chooseTimer.rows = held.length >= 2 ? held : [1, 3, 5]
            chooseTimer.step = 0
            chooseTimer.readOid = ""
            chooseTimer.after = act === "graph-choose-said" ? "said" : "sweep"
            chooseTimer.start()
        } else if (act === "graph-choose" || act === "graph-choose-range") {
            // The choice several commits are held in (デザイン規約 §複数のコミットを選ぶ). The first press is plain and
            // settles both the choice and what is read; the ones after it are held, and move only the choice. The
            // argument is the rows, `:`-separated — `graph-choose` takes them one at a time with Ctrl, and
            // `-range` sweeps from the first to the last with one Shift press, so that one wants exactly two.
            const picked = arg === "" ? [] : arg.split(":").map(Number)
            chooseTimer.sweeps = act === "graph-choose-range"
            chooseTimer.rows = picked.length >= 2 ? picked : (chooseTimer.sweeps ? [1, 5] : [1, 3, 5])
            chooseTimer.step = 0
            chooseTimer.readOid = ""
            chooseTimer.after = ""
            chooseTimer.start()
        } else if (act === "graph-reclick-lanes") {
            // The argument is the row the presses land on.
            laneClickTimer.row = Number(arg === "" ? "0" : arg)
            laneClickTimer.start()
        } else if (act === "graph-reclick" || act === "graph-rename"
                   || act === "graph-reclick-scrolled" || act === "graph-reclick-mark"
                   || act === "graph-reclick-still") {
            // The gesture at the row itself. The argument is `<行>[:<付ける名前>]` — the first row is the default; a
            // run that wants a chip says which (`--preset tags`). `graph-rename` carries the gesture through to git.
            const renameCut = arg.indexOf(":")
            reclickGraphTimer.row = Number(renameCut < 0 ? (arg === "" ? "0" : arg) : arg.substring(0, renameCut))
            reclickGraphTimer.name = renameCut < 0 ? "" : arg.substring(renameCut + 1)
            reclickGraphTimer.scrolls = act === "graph-reclick-scrolled"
            reclickGraphTimer.marks = act === "graph-reclick-mark"
            reclickGraphTimer.points = act === "graph-reclick-still"
            // The submit is ticks away; the watch is armed here and catches the ask whenever it comes
            // (`AutoActDriver.beginWrite`), so what it waits on in between is the input.
            if (act === "graph-rename")
                driver.beginWrite("graph-rename")
            reclickGraphTimer.start()
        } else if (act === "graph-reclick-list" || act === "graph-reclick-across" || act === "ref-list-pick") {
            // The same gesture, and the double-click beside it, put in at the card the chip unfolds into. The
            // argument `<行>[:<カードの行>]` defaults to the card's first row — the one on the chip's own seat.
            const listParts = arg.split(":")
            reclickListTimer.row = listParts[0] === "" ? 0 : Number(listParts[0])
            reclickListTimer.card = listParts.length > 1 ? Number(listParts[1]) : 0
            reclickListTimer.picks = act === "ref-list-pick"
            reclickListTimer.across = act === "graph-reclick-across"
            if (reclickListTimer.picks)
                driver.beginWrite("ref-list-pick")
            reclickListTimer.start()
        } else {
            return false
        }
        return true
    }
    // The presses that build a choice, put in at the row's own click function (`GraphRowDelegate.leftClick`) — what a
    // held modifier does to a click is decided there and in the page, and a run that wrote the choice itself would say
    // nothing about either.
    SampleTimer {
        id: chooseTimer
        /// The rows pressed, in order. The first press is the plain one.
        property var rows: []
        /// Whether the presses after the first sweep a range (Shift) or take one row each (Ctrl).
        property bool sweeps: false
        property int step: 0
        /// The commit the plain press landed on, so the report can say what is read stayed on it while the choice grew.
        property string readOid: ""
        /// What to do once the choice is standing, all of it handed to `chosenRowTimer`: `said` rests on a row of the
        /// list the pane put up, `sweep` drags a value out of one, `drop` takes a commit back out of the choice from
        /// there, `diff` opens a file of the merged list, `dbl` tries the gesture a choice tells apart. `""`
        /// stops at the choice itself.
        property string after: ""
        /// The file `graph-choose-diff` opens out of the merged list.
        property string diffPath: ""
        /// How many commits the presses should end up holding.
        function wanted() {
            if (!chooseTimer.sweeps)
                return chooseTimer.rows.length
            return Math.abs(chooseTimer.rows[chooseTimer.rows.length - 1] - chooseTimer.rows[0]) + 1
        }
        /// How many rows draw themselves chosen, counted to past the last row this run pressed (`litRowsTo`).
        function litRows() {
            let last = 0
            for (const row of chooseTimer.rows)
                last = Math.max(last, row)
            return acts.litRowsTo(last)
        }
        onTriggered: {
            if (chooseTimer.step < chooseTimer.rows.length) {
                const want = chooseTimer.rows[chooseTimer.step]
                const item = graphPane.view.itemAtIndex(want)
                // A row the view has not laid out yet is not a row that was pressed (`graph-reclick`) — and a row far
                // down the history has no delegate at all until the view is over it, which is what a hand does before
                // it presses one. Sent there and retried on the next tick, the layout being a frame behind.
                if (!item) {
                    graphPane.view.positionViewAtIndex(want, ListView.Contain)
                    return
                }
                if (chooseTimer.step === 0) {
                    chooseTimer.readOid = item.oid_hex
                    item.leftClick(0, Qt.NoModifier)
                } else {
                    item.leftClick(0, chooseTimer.sweeps ? Qt.ShiftModifier : Qt.ControlModifier)
                }
                chooseTimer.step++
                return
            }
            // Every press is in. **Four things have to land, and each is on the output side**: the page's tally, the
            // highlight the rows drew of it (a choice nothing draws is a number in a property), the pane on the right
            // having answered *for this choice* — `selectionLoaded`, because a read that
            // failed also stops loading and leaves an empty list, which is the same shape as a choice of commits that
            // changed nothing — and the list of commits having laid its rows out, since what is asked about that
            // list below is a measurement and an unlaid-out view answers 0 to every one (規約 §UI 自動化の因果性).
            const lit = chooseTimer.litRows()
            if (page.chosenCount !== chooseTimer.wanted() || lit !== chooseTimer.wanted()
                    || !detailsModel.selectionLoaded || !detailsPane.chosenListDrawn)
                return
            chooseTimer.stop()
            // The choice is standing and the pane has answered for it. Two verbs go on from here into the list it put
            // up; the rest report the choice itself.
            if (chooseTimer.after !== "") {
                chosenRowTimer.start()
                return
            }
            Harness.report("graph_choose chosen=" + page.chosenCount
                              + " lit=" + lit
                              + " read=" + (page.selectedOid === chooseTimer.readOid)
                              + " wip=" + page.wipShown
                              // How far the commit list runs past the view it stands in, the view's own margins
                              // counted. A choice this small fits, so anything but 0 is a list that scrolls to
                              // reach nothing.
                              + " spare=" + detailsPane.chosenListSpare
                              + " compared=" + detailsModel.comparing
                              + " files=" + detailsModel.fileTotal
                              + " rows=" + chooseTimer.rows.join(","))
            driver.complete()
        }
    }
    // Everything that goes on from a standing choice (`chooseTimer.after`): the card a rest opens over one of the
    // listed commits, the drag a reader takes its words away with, the press that drops one, the file the merged list
    // opens, and the gesture the graph's rows have to turn down. Each goes in at the thing that answers it — the row's
    // own functions, the pad under its words, the page's diff opener.
    SampleTimer {
        id: chosenRowTimer
        /// How far the drop has got: the plain press, the held one, then the wait for the pane to answer for what is
        /// left. Latched, because both presses are made once and the sampler goes on firing around them.
        property int dropStep: 0
        /// The commit the presses land on, and what the row said about a press without the modifier.
        property string dropOid: ""
        property bool dropTakes: true
        /// How many commits were held after the plain press — the half of the claim that says the words below still
        /// have their press.
        property int dropKept: -1
        /// **Two presses on one row of the list, and only the second is the row's.** The first is plain, and a choice
        /// that came back one commit smaller after it would mean the words underneath had lost their press to this
        /// hand. Both go in at the row's own functions, where the two are told apart.
        function dropOne(row) {
            if (chosenRowTimer.dropStep === 0) {
                chosenRowTimer.dropOid = row.oid_hex
                chosenRowTimer.dropTakes = row.takesPress(Qt.NoModifier)
                row.leftClick(Qt.NoModifier)
                chosenRowTimer.dropStep = 1
                return
            }
            if (chosenRowTimer.dropStep === 1) {
                chosenRowTimer.dropKept = page.chosenCount
                row.leftClick(Qt.ControlModifier)
                chosenRowTimer.dropStep = 2
                return
            }
            // The choice is one smaller and the pane has answered for what is left —
            // `selectionLoaded`, the same readiness the presses themselves waited on (規約 §UI 自動化の因果性).
            if (page.chosenCount !== chooseTimer.wanted() - 1 || !detailsModel.selectionLoaded)
                return
            chosenRowTimer.stop()
            Harness.report("chosen_drop takes=" + chosenRowTimer.dropTakes
                              + " kept=" + chosenRowTimer.dropKept
                              + " dropped=" + (page.chosenOids[chosenRowTimer.dropOid] !== true)
                              + " hand=" + row.dropStands
                              + " chosen=" + page.chosenCount
                              + " lit=" + chooseTimer.litRows())
            driver.complete()
        }
        onTriggered: {
            if (chooseTimer.after === "dbl") {
                // The pair as the area delivers it: two clicks, then the double the second one is also reported as.
                // Put in at the row's own functions, which is where the two are told apart.
                const at = graphPane.view.itemAtIndex(chooseTimer.rows[chooseTimer.rows.length - 1])
                if (!at)
                    return
                chosenRowTimer.stop()
                at.leftClick(0, Qt.ControlModifier)
                const led = at.doubleClick(Qt.ControlModifier)
                // **`switch` is what the plain double-click leads to**, and it lands ticks later — so the branch on
                // screen at the moment of the press says nothing either way. What the row answered is the decision
                // itself, read back off the same function a hand goes through. `movable=` is the other half: a row
                // that leads nowhere turns the gesture down for a reason of its own, and this one does lead somewhere.
                Harness.report("chosen_dbl led=" + led
                                  + " movable=" + at.movable
                                  + " naming=" + (graphPane.namingOid !== "")
                                  + " branch=" + workTree.branch)
                driver.complete()
                return
            }
            if (chooseTimer.after === "diff") {
                // Through the page's own opener, the one a press on a file row goes to (`graph-step-diff` enters the
                // same way). The stack is the shot, so the run waits for the rows to be laid out under it.
                if (!page.diffShown) {
                    page.toggleDiff("commit", chooseTimer.diffPath, "")
                    return
                }
                if (!diffPane.diffSettled() || diffPane.view.count === 0)
                    return
                chosenRowTimer.stop()
                Harness.report("chosen_diff bands=" + diffPane.diffModel.commitBands
                                  + " chosen=" + page.chosenCount
                                  + " path=" + page.diffPath)
                driver.complete()
                return
            }
            const row = detailsPane.chosenRowAt(0)
            // A row the list has not built or laid out yet is not a row anybody is reading.
            if (!row || !row.rowReady())
                return
            if (chooseTimer.after === "drop") {
                chosenRowTimer.dropOne(row)
                return
            }
            if (chooseTimer.after === "said") {
                // The card the row's own rest opens, entered where that rest would (hover cannot be injected).
                if (!rowCard.opened) {
                    row.askCard()
                    return
                }
                chosenRowTimer.stop()
                // **The row cuts and the card does not** — the two halves of the same claim, and neither is a thing a
                // picture answers: a mark is a few pixels wide, so a row that dropped the tail of a summary frames the
                // same as one that did not, and a card that held its body back frames as a shorter card.
                // **`held=` last of the four**: the wall's own run asks for the three in front of it and nothing
                // about this one, and a judgement is a run of words that touch (verify-ui).
                Harness.report("chosen_said open=" + rowCard.opened
                                  + " door=" + rowCard.asksForMore
                                  + " lit=" + (row.cardOid === row.oid_hex)
                                  + " held=" + rowCard.messageCut
                                  + " cut=" + row.summaryCut)
                driver.complete()
                return
            }
            chosenRowTimer.stop()
            // **From every corner of the row's air.** A reach that works from the middle
            // and nowhere else is the fault this kind of row ships with, and the pad says the whole of it in one line
            // (`SweepPad.sweepAir` — the same sentence `card-sweep` reads).
            Harness.report("chosen_sweep " + row.sweep.sweepAir(9, ""))
            driver.complete()
        }
    }
    // The chip expansion, entered once the row's delegate exists; `-card` hands over to `rowCardTimer`.
    SampleTimer {
        id: refListOpenTimer
        property int row: 0
        property bool cards: false
        /// Which row of the list `list-menu` presses; -1 for the verbs that only open it.
        property int menuRow: -1
        /// Which row of the list `ref-list-lit` rests the pointer on; -1 for the verbs that leave it off the card.
        property int litRow: -1
        /// Which row of the list `ref-list-choose` presses with Ctrl held; -1 for the verbs that press none.
        property int chooseRow: -1
        onTriggered: {
            const stacked = graphPane.view.itemAtIndex(refListOpenTimer.row)
            if (!stacked)
                return
            refListOpenTimer.stop()
            if (refListOpenTimer.cards)
                graphPane.view.rowHoverRequested(stacked, true)
            graphPane.view.chipExpandRequested(
                stacked.oid_hex, stacked.index, stacked.chipItem.records, stacked.chipItem)
            if (refListOpenTimer.cards) {
                graphPane.view.rowHoverRequested(stacked, true)
                rowCardTimer.row = refListOpenTimer.row
                rowCardTimer.start()
            } else if (refListOpenTimer.menuRow >= 0)
                listMenuTimer.start()
            else if (refListOpenTimer.litRow >= 0)
                listLitTimer.start()
            else if (refListOpenTimer.chooseRow >= 0)
                listChooseTimer.start()
            else
                refListShownTimer.start()
        }
    }
    // The held click on a row of that list, put in once the list is actually up, and the choice read back once the
    // rows draw it: the rows draw a choice a tick behind the press, the wait `graph-choose` takes.
    SampleTimer {
        id: listChooseTimer
        property bool pressed: false
        /// The commit that was being read before the press, so the report can say the held press left it alone.
        property string readOid: ""
        onTriggered: {
            if (!refList.opened)
                return
            if (!listChooseTimer.pressed) {
                listChooseTimer.readOid = page.selectedOid
                if (!refList.chooseRow(refListOpenTimer.chooseRow, Qt.ControlModifier))
                    return
                listChooseTimer.pressed = true
                return
            }
            // The choice and the highlight the rows drew of it, counted to past the row the card stands on, and the
            // pane having answered for this choice (`selectionLoaded`, for the reason `graph-choose` gives).
            const lit = acts.litRowsTo(refListOpenTimer.row)
            if (page.chosenCount !== 2 || lit !== 2 || !detailsModel.selectionLoaded)
                return
            listChooseTimer.stop()
            // `read=` is what is read having stayed where it was — the held press moved the choice and nothing else
            // — and `list=` the card the press was made in still standing under the hand: a press that closed it
            // would have been the plain one.
            Harness.report("ref_list_choose chosen=" + page.chosenCount
                              + " lit=" + lit
                              + " read=" + (page.selectedOid === listChooseTimer.readOid)
                              + " list=" + refList.opened
                              + " row=" + refListOpenTimer.row)
            driver.complete()
        }
    }
    // The right-click on a row of that list, put in once the list is actually up — the rows are the popup's own, and
    // an unopened popup has none to press.
    SampleTimer {
        id: listMenuTimer
        onTriggered: {
            if (!refList.opened)
                return
            if (!refList.menuRow(refListOpenTimer.menuRow))
                return
            listMenuTimer.stop()
            // **`list=` is read after the menu is up**: the card the press was made on has to still be standing under
            // it, or what the reader named goes out from under the hand that named it (デザイン規約 §メニュー の
            // 例外). `branch=` / `tag=` are the cards themselves — which one the naming brought up is the whole of
            // what this gesture decides.
            Harness.report("list_menu list=" + refList.opened
                              + " menu=" + commitMenu.opened
                              + " branch=" + commitBranchCard.applies
                              + " tag=" + commitTagCard.applies
                              + " rows=" + commitMenu.offeredRows)
            driver.complete()
        }
    }
    // The pointer coming to rest on one of that list's rows, put in once the list is actually up — the rows are the
    // popup's own, and an unopened popup has none to rest on.
    SampleTimer {
        id: listLitTimer
        onTriggered: {
            if (!refList.opened)
                return
            if (!refList.pointRow(refListOpenTimer.litRow))
                return
            listLitTimer.stop()
            // **`lit=` is read back off the row**: the wash is one shade over the
            // card's own ground, so a row that never took the answer frames the same as one that did. `nowhere=` is
            // the half that says which kind of row this was — a row with nowhere to go is the one whose wash the
            // colour of its name cannot stand in for, so a picture of a row that leads somewhere proves nothing.
            Harness.report("ref_list_lit row=" + refListOpenTimer.litRow
                              + " list=" + refList.opened
                              + " lit=" + refList.rowLit(refListOpenTimer.litRow)
                              + " nowhere=" + refList.rowLeadsNowhere(refListOpenTimer.litRow))
            driver.complete()
        }
    }
    // The bare verb's own end: the list is up (an unopened popup frames as the plain screen).
    SampleTimer {
        id: refListShownTimer
        onTriggered: {
            if (!refList.opened)
                return
            refListShownTimer.stop()
            renderedBarrier.begin()
        }
    }
    // The rest the row opens its card after is `tipDelayMs` away, so this samples until the card is up and then makes
    // the return in one turn.
    SampleTimer {
        id: cardReturnTimer
        /// The row the hand rests on, leaves and comes back to.
        property int row: 0
        /// Where the card stood the first time, and the pointer that put it there. The second rest is made at a
        /// different x on purpose: the seat is the pointer's, so a card that is opened again moves,
        /// and a run that came back to the same x could not tell the two apart.
        property real seat: 0
        property real firstX: 0
        onTriggered: {
            const at = graphPane.view.itemAtIndex(cardReturnTimer.row)
            if (!at)
                return
            // The rest, and then the wait it opens after. Both are the row's own (`GraphRowDelegate.settlePointed`).
            if (!rowCard.opened) {
                cardReturnTimer.firstX = Math.round(at.width / 3)
                at.pointerRowX = cardReturnTimer.firstX
                return
            }
            cardReturnTimer.stop()
            cardReturnTimer.seat = rowCard.x
            // The hand walks off the row into the card, and back onto the row a little further along. **Read in the
            // same turn as the return**: everything from the pointer to the hold is one synchronous stretch, so what
            // the beat would have done to a card nobody re-held is not something this has to wait to find out — the
            // hold is either back before the beat can start or it is not (規約 §前提条件は入力を出す枝で読む).
            at.pointerRowX = -1
            at.pointerRowX = cardReturnTimer.firstX * 2
            Harness.report(
                "row_card_return held=" + rowHost.rowCardWanted
                + " open=" + rowCard.opened
                + " moved=" + (rowCard.x !== cardReturnTimer.seat)
                + " oid=" + (rowHost.rowCardOid === at.oid_hex))
            driver.complete()
        }
    }
    // The card is opened synchronously; this just lets the layout settle before it is measured and photographed.
    SampleTimer {
        id: rowCardTimer
        /// The row the card was asked of, so the report can ask it back whether it is still lit. Read off the row
        /// — the whole point is that the row got the answer.
        property int row: 0
        onTriggered: {
            if (!rowCard.opened && !refList.opened)
                return
            rowCardTimer.stop()
            const asked = graphPane.view.itemAtIndex(rowCardTimer.row)
            Harness.report(
            // `lit=` sits next to `open=`: the pair is what the run is judged on, and the judge reads one unbroken
            // stretch of the line (`verify::verbs::must_say`).
            "row_card open=" + rowCard.opened
            + " lit=" + (asked ? asked.lit : false)
            + " credit=" + Math.round(rowCard.creditWidth)
            + " cut=" + rowCard.creditCut
            + " list=" + refList.opened
            + " subject=" + (rowCard.subject !== "")
            + " body=" + (rowCard.body !== ""))
            driver.complete()
        }
    }
    // The press on that card's note, and where it leaves the reader. Two beats: the card has to be up and holding a
    // message it had to cut before the note is anywhere on screen, and the pane it sends them to answers a request
    // that goes out at the press — so the picture is of the arrival.
    SampleTimer {
        id: cardMessageTimer
        property int row: 0
        property bool asked: false
        /// Whether the run carries on and puts the mark away again with Escape. **Entered where the key handler's
        /// own body is** (`RepoPage.escapePressed`); that the key reaches that handler at all is Qt's business and
        /// is held by `tests/qml/tst_escape.qml`.
        property bool escapes: false
        /// Which commit the press was of, kept because the card takes its own copy down with it.
        property string oidHex: ""
        onTriggered: {
            if (!cardMessageTimer.asked) {
                const hovered = graphPane.view.itemAtIndex(cardMessageTimer.row)
                if (!hovered)
                    return
                graphPane.view.rowHoverRequested(hovered, true)
                // **The note is what is being pressed**, so a card without one is not this verb's card: a message
                // that fits says so by not offering anywhere further to go (規約 §hover のツールチップ).
                if (!rowCard.opened || !rowCard.messageCut)
                    return
                cardMessageTimer.oidHex = hovered.oid_hex
                rowCard.askMessage()
                cardMessageTimer.asked = true
                return
            }
            // The arrival: the card is gone, the row it was of is the page's selection, and the pane holds that
            // commit's own message. **`details` is asked for at the press**, so waiting on it is waiting on the very
            // thing the note promised — a shot taken before it frames the message of whatever was open before.
            if (rowCard.opened || detailsPane.details.shaHex !== cardMessageTimer.oidHex)
                return
            cardMessageTimer.stop()
            // The mark put away again, for the run that carries on that far. **Its own sentence**, because the two
            // are judged on opposite answers and the judge reads one unbroken stretch of a line.
            if (cardMessageTimer.escapes) {
                const took = page.escapePressed()
                Harness.report(
                    "card_message_esc mark=" + detailsPane.attention
                    + " took=" + took
                    + " picked=" + (page.selectedRow === cardMessageTimer.row))
                driver.complete()
                return
            }
            Harness.report(
                "card_message open=" + rowCard.opened
                + " picked=" + (page.selectedRow === cardMessageTimer.row)
                + " mark=" + detailsPane.attention
                + " shown=" + (detailsPane.boxSubject !== "")
                + " row=" + page.selectedRow)
            driver.complete()
        }
    }
    // The hand resting on that note. **Two beats for the same reason the press has one**: the
    // note only stands under a message the card had to cut, and a card fills its fields in after it is opened — so the
    // rest goes in once the note is there, and the answer is read on a later sample, off the paint.
    SampleTimer {
        id: cardNoteTimer
        property int row: 0
        property bool pointed: false
        onTriggered: {
            if (!cardNoteTimer.pointed) {
                const hovered = graphPane.view.itemAtIndex(cardNoteTimer.row)
                if (!hovered)
                    return
                graphPane.view.rowHoverRequested(hovered, true)
                // A card whose message fits offers nowhere further to go, so there is no note to rest on: this verb's
                // card is the one that had to stop (規約 §hover のツールチップ).
                if (!rowCard.opened || !rowCard.messageCut)
                    return
                rowCard.notePointedAt = true
                cardNoteTimer.pointed = true
                return
            }
            cardNoteTimer.stop()
            // **`lit=` is the rule's own sentence read off the paint** — the line under the words is the words'
            // colour — and `word=` is what this note's step is: the pair apart, a note whose word stayed at the
            // dimmest text there is would say `lit=true` just as loudly (規約 §hover のツールチップ).
            Harness.report(
                "card_note lit=" + Qt.colorEqual(rowCard.noteRuleColor, rowCard.noteWordColor)
                + " word=" + rowCard.noteWordColor
                + " rule=" + rowCard.noteRuleColor)
            driver.complete()
        }
    }
    // ...and the same card's words taken from the air around them: the padding band, the step between two lines, the
    // room beside a short one (規約 §hover のツールチップ). The card is the graph row's, because it is the one with
    // several lines in it and a badge row beside them — a card with one sentence proves the padding band and nothing
    // else.
    //
    // **The starts are the air itself, sampled** (`SweepPad.airPoints`), for the reason `tip-sweep` carries: a grid
    // over the card with the points standing on a field dropped is exactly what a real press could reach the pad at,
    // and a run that pressed the middle would be pressing on the words.
    SampleTimer {
        id: cardSweepTimer
        /// The card's geometry at the previous sample, for the settle below.
        property string lastGeom: ""
        onTriggered: {
            // **The commit's own words have to be in it first.** A card opens the frame it is asked for and fills in
            // afterwards, and one swept before that hands back `1970-01-01` — a stamp of a commit nobody made
            // (measured).
            if (!rowCard.opened || rowCard.subject === "")
                return
            // **And it has to have stopped laying out**, for the reason `details-sweep` waits: a card mid-layout has
            // its fields at some other width, and the air a run samples is the air of a frame nobody sees.
            const geom = Math.round(rowCard.width) + "x" + Math.round(rowCard.height)
            if (geom !== cardSweepTimer.lastGeom) {
                cardSweepTimer.lastGeom = geom
                return
            }
            cardSweepTimer.stop()
            // The whole of the sweep is the pad's own sentence now (`SweepPad.sweepAir`) — eight surfaces carry this
            // hand and were each asking it the same four things. `open=` is this verb's own half and goes in where it
            // always stood.
            Harness.report("card_sweep "
                + rowCard.background.pad.sweepAir(9, "open=" + rowCard.opened))
            driver.complete()
        }
    }
    // What the row was asked and what it answered, kept for the report — the ask is a point along the row and the
    // answer is which of the two cards came out of it.
    QtObject {
        id: rowPartReport
        property int row: 0
        property string want: ""
        property real x: 0
        property bool probed: false
    }
    // Waits for either card: a boundary that moved opens the other, and waiting for the right
    // answer would spend the watchdog finding that out. The verb is judged on `agrees`.
    SampleTimer {
        id: rowPartTimer
        onTriggered: {
            if (!rowPartReport.probed) {
                const item = graphPane.view.itemAtIndex(rowPartReport.row)
                if (!item)
                    return
                item.pointerRowX = rowPartReport.x
                rowPartReport.probed = true
                return
            }
            if (!refList.opened && !rowCard.opened)
                return
            rowPartTimer.stop()
            const got = refList.opened ? "chip" : "row"
            Harness.report(
            "row_part x=" + rowPartReport.x
            + " want=" + rowPartReport.want
            + " got=" + got
            + " list=" + refList.opened
            + " card=" + rowCard.opened
            + " agrees=" + (got === rowPartReport.want
                            && refList.opened !== rowCard.opened))
            driver.complete()
        }
    }
    // The row under a standing menu, asked for its card the way its own delay timer would ask (hover cannot be
    // injected — verify-ui スキル §hover の絵の撮り方). Read as a pair with `row-card`, which proves that same input does
    // open the card: on its own, a card that stayed shut says nothing about why.
    //
    // The request goes in only once the menu is actually up — before that there is nothing for the card to be behind —
    // and the answer is read a sampler turn later, since a card that was going to open opens synchronously
    // (`rowCardTimer`).
    SampleTimer {
        id: menuHoverTimer
        property string oidHex: ""
        property bool asked: false
        onTriggered: {
            if (!commitMenu.opened)
                return
            if (!menuHoverTimer.asked) {
                const row = graphPane.view.itemAtIndex(graphModel.rowOf(menuHoverTimer.oidHex))
                // A row the view has not laid out yet is not a row that was asked: latching here would wait for an
                // answer to a question nobody put (app-ui.md §UI 自動化の因果性).
                if (!row)
                    return
                menuHoverTimer.asked = true
                graphPane.view.rowHoverRequested(row, true)
                return
            }
            menuHoverTimer.stop()
            Harness.report("menu_hover menu=" + commitMenu.opened + " card=" + rowCard.opened
                              + " list=" + refList.opened)
            driver.complete()
        }
    }
    // PGG_AUTO_ACT=graph-reclick / graph-reclick-list / ref-list-pick: the two clicks of the rename gesture put in at a
    // graph row, and at a row of the card its chip unfolds into — and, on the same card, the double-click that is the
    // way to move. Every step waits for its own answer: the row has to exist before it can be clicked, the card has to
    // be up before one of its rows can be, and the double-click window the first click opened has to have passed
    // before a second one counts as a second (app-ui.md §UI 自動化の因果性).
    property bool reclickGraphArmed: false
    property int reclickGraphStep: 0
    /// When the second click went in, so the report can say how long the box made the reader wait. **A diagnosis**
    /// — the wait is the double-click window and the sampler reads through it (app-ui.md §UI 自動化の因果性);
    /// it is here because a lag is the one thing about this gesture a picture cannot show.
    property real reclickGraphAt: 0
    SampleTimer {
        id: reclickGraphTimer
        property int row: 0
        /// The name to put in the box once it is open, for the run that carries the gesture through to git
        /// (`graph-rename`); empty for the one that stops at the box.
        property string name: ""
        /// Whether the run scrolls the history away between the second click and the box. **The delegate is pooled by
        /// that scroll**, which the gesture survives (`ReclickGesture`) — and the box that
        /// opens has to be sent back into view.
        property bool scrolls: false
        /// Whether the run ends inside the wait, on the mark the chip wears while it runs.
        property bool marks: false
        /// Whether the pointer is rested on the chip first, so the card it opens has a rest running under the wait —
        /// what the gesture has to hold still.
        property bool points: false
        onTriggered: {
            const item = graphPane.view.itemAtIndex(reclickGraphTimer.row)
            // A row the view has not laid out yet is not a row that was clicked: latching here would wait for an
            // answer to a question nobody put.
            if (!item)
                return
            if (acts.reclickGraphStep === 0) {
                // The pointer comes to rest on the chip first, which starts the rest that opens the card
                // (`GraphRowDelegate.restDelay`). Written where a real pointer writes it, so the row makes every
                // decision after that for itself (`row-part`).
                if (reclickGraphTimer.points)
                    item.pointerRowX = item.labelsW - 2
                item.leftClick(0)
                acts.reclickGraphStep = 1
            } else if (acts.reclickGraphStep === 1) {
                // Inside the window the first click opened, a second click is the other half of a double-click and not
                // a second click at all.
                if (item.clickGuarded)
                    return
                // waits(measured): the origin of the `wait=` below, which the report prints and nothing here reads
                acts.reclickGraphAt = Date.now()
                item.leftClick(0)
                // Read where it is set: the wait is short and the box is what it turns into.
                acts.reclickGraphArmed = item.renameArmed
                // The history walks away under the wait: the row that was clicked is pooled, and what was armed on it
                // has to survive that. Far enough that the row is well outside the view's own buffer.
                if (reclickGraphTimer.scrolls)
                    graphPane.view.positionViewAtIndex(reclickGraphTimer.row + 200, ListView.Beginning)
                if (reclickGraphTimer.marks) {
                    // **The mark is what the reader has to see during the wait**, so this run ends inside
                    // it. Read off the chip itself (`RefChip.waiting`), because a picture taken a beat
                    // late frames the box instead and would say nothing either way.
                    reclickGraphTimer.stop()
                    Harness.report(
                    "graph_reclick_mark row=" + reclickGraphTimer.row
                    + " armed=" + acts.reclickGraphArmed
                    + " mark=" + item.chipWaiting
                    + " box=" + (graphPane.namingOid !== "")
                    + " window=" + Application.styleHints.mouseDoubleClickInterval)
                    driver.complete()
                    return
                }
                acts.reclickGraphStep = 2
            } else if (acts.reclickGraphStep === 2) {
                if (acts.reclickGraphArmed && graphPane.namingOid === "")
                    return
                // The box is open: the row it is on has to have been sent back into view before the shot
                // (`RepoPage.startRename`), which the walk does a beat after the box opens.
                if (reclickGraphTimer.scrolls && !graphPane.rowOnScreen(reclickGraphTimer.row))
                    return
                reclickGraphTimer.stop()
                Harness.report(
                "graph_reclick row=" + reclickGraphTimer.row
                + " armed=" + acts.reclickGraphArmed
                + " box=" + (graphPane.namingOid !== "")
                // Nothing hover opened or closed under the wait — the rest that was running when the second click
                // landed is part of the same beat (`GraphList.renameWaiting`).
                + " list=" + refList.opened
                + " card=" + rowCard.opened
                + " mode=" + graphPane.namingMode
                + " kind=" + graphPane.namingKind
                + " typed=" + graphPane.namingText
                // The tree has not moved: the gesture the reader made was not the double-click, and this is the half
                // of the report a picture of an open box cannot make.
                + " branch=" + workTree.branch
                // Whether the row the box is on is in sight — the whole of the scrolled run's claim, and true of the
                // plain one for nothing having moved it.
                + " shown=" + graphPane.rowOnScreen(reclickGraphTimer.row)
                // How long the box took, against the window it is waiting out. The sampler's own beat is in the
                // difference, so this is read as "about the window".
                // waits(measured): printed beside the window it is read against, and judged by nothing
                + " wait=" + (Date.now() - acts.reclickGraphAt)
                + " window=" + Application.styleHints.mouseDoubleClickInterval)
                if (reclickGraphTimer.name === "") {
                    driver.complete()
                    return
                }
                // Through to git, by the path the field's own Enter takes. The write barrier finishes this one.
                graphPane.view.namingSubmitted(graphModel.oidAt(reclickGraphTimer.row),
                                               reclickGraphTimer.name, graphPane.namingMode)
                driver.inputWent(true)
            }
        }
    }
    // PGG_AUTO_ACT=graph-reclick-lanes: the same gesture put in through **the strip over the lane column** — the half
    // of the row between the two dividers, which has a press-taking layer of its own wherever the lanes overflow their
    // column (`GraphLanePan`). A reader aiming at the middle of a wide graph is aiming at that strip, and a strip that
    // answered with a copy of half of what a click does left the gesture doing nothing there.
    property bool laneClickArmed: false
    property int laneClickStep: 0
    SampleTimer {
        id: laneClickTimer
        property int row: 0
        onTriggered: {
            const item = graphPane.view.itemAtIndex(laneClickTimer.row)
            if (!item)
                return
            if (acts.laneClickStep === 0) {
                // The strip is only up while the lanes have somewhere sideways to go, so the column is squeezed to its
                // floor first — the state a repository wide enough to need panning is in from the start.
                page.setGraphColumns(graphPane.labelW, 0)
                acts.laneClickStep = 1
            } else if (acts.laneClickStep === 1) {
                if (!graphPane.lanePan.visible)
                    return
                // A point in the strip's own frame, over this row — the strip works out which row that is.
                if (!graphPane.lanePan.clickAt(1, item.mapToItem(graphPane, 0, item.height / 2).y))
                    return
                acts.laneClickStep = 2
            } else if (acts.laneClickStep === 2) {
                if (graphPane.view.clickGuarded)
                    return
                if (!graphPane.lanePan.clickAt(1, item.mapToItem(graphPane, 0, item.height / 2).y))
                    return
                acts.laneClickArmed = item.renameArmed
                acts.laneClickStep = 3
            } else if (acts.laneClickStep === 3) {
                if (acts.laneClickArmed && graphPane.namingOid === "")
                    return
                laneClickTimer.stop()
                Harness.report(
                "graph_reclick_lanes row=" + laneClickTimer.row
                + " strip=" + graphPane.lanePan.visible
                + " armed=" + acts.laneClickArmed
                + " box=" + (graphPane.namingOid !== "")
                + " typed=" + graphPane.namingText
                + " branch=" + workTree.branch)
                driver.complete()
            }
        }
    }
    property bool reclickListArmed: false
    property int reclickListStep: 0
    SampleTimer {
        id: reclickListTimer
        property int row: 0
        property int card: 0
        /// Whether this run is the double-click that moves or the two clicks that name.
        property bool picks: false
        /// Whether the two clicks are put in at **different surfaces** — the first at the row, the second at the card
        /// its chip opens into. That is what a reader does without knowing it: the card comes up on the chip's own
        /// seat after a rest, so the second click at one spot lands somewhere else.
        property bool across: false
        onTriggered: {
            if (acts.reclickListStep === 0) {
                const item = graphPane.view.itemAtIndex(reclickListTimer.row)
                if (!item)
                    return
                if (reclickListTimer.across)
                    item.leftClick(0)
                // Hover cannot be injected, so this enters where the row's own rest timer would (`ref-list`).
                graphPane.view.chipExpandRequested(item.oid_hex, reclickListTimer.row,
                                                   item.chipItem.records, item.chipItem)
                acts.reclickListStep = 1
            } else if (acts.reclickListStep === 1) {
                // The card lays its rows out as it is shown; until it is up there is no row to click.
                if (!refList.opened)
                    return
                if (reclickListTimer.picks) {
                    if (!driver.inputWent(refList.doubleClickRow(reclickListTimer.card)))
                        return
                    reclickListTimer.stop()
                    Harness.report("ref_list_pick row=" + reclickListTimer.row
                                      + " card=" + reclickListTimer.card
                                      + " list=" + refList.opened)
                    // The write barrier is what finishes this one: the move is the whole of it.
                    return
                }
                // Inside the window the row's click opened, a click here is the other half of a double-click.
                if (reclickListTimer.across && refList.rowGuarded(reclickListTimer.card))
                    return
                if (!refList.clickRow(reclickListTimer.card))
                    return
                if (reclickListTimer.across) {
                    // The row's click was the first: one click on the card is already the second.
                    acts.reclickListArmed = refList.rowArmed(reclickListTimer.card)
                    acts.reclickListStep = 3
                    return
                }
                acts.reclickListStep = 2
            } else if (acts.reclickListStep === 2) {
                if (refList.rowGuarded(reclickListTimer.card))
                    return
                if (!refList.clickRow(reclickListTimer.card))
                    return
                acts.reclickListArmed = refList.rowArmed(reclickListTimer.card)
                acts.reclickListStep = 3
            } else if (acts.reclickListStep === 3) {
                if (acts.reclickListArmed && graphPane.namingOid === "")
                    return
                reclickListTimer.stop()
                Harness.report(
                "graph_reclick_list row=" + reclickListTimer.row
                + " card=" + reclickListTimer.card
                + " armed=" + acts.reclickListArmed
                + " box=" + (graphPane.namingOid !== "")
                // The card came down for the box: it was standing on the column the box opens in, and one left up
                // would be covering what the run is about.
                + " list=" + refList.opened
                + " mode=" + graphPane.namingMode
                + " kind=" + graphPane.namingKind
                + " typed=" + graphPane.namingText
                + " branch=" + workTree.branch)
                driver.complete()
            }
        }
    }
}
