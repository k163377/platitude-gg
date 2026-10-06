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
    readonly property var workingTree: driver.workingTree
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

    /// Runs `act` if it is one of this family's, and says whether it was.
    function run(act, arg) {
        if (act === "row-part") {
            // Hover cannot be injected, so this writes what a real pointer writes (`GraphRowDelegate.pointerRowX`) and
            // leaves the decision to the row — the decision is what the verb tests, so `chipExpandRequested` (as
            // `ref-list` does) would prove nothing about the boundary.
            const parts = arg.split(":")
            rowPartReport.row = Number(parts[0])
            rowPartReport.want = parts[2]
            rowPartReport.x = Number(parts[1])
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
            leaveTimer.step = 0
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
            // first row — the stand-in's press.
            headListTimer.pointed = false
            headListTimer.click = arg === "click"
            headListTimer.pressed = false
            headListTimer.movedOff = false
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
            listFollowTimer.lit = act === "ref-list-follow-lit"
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
            // Entered where the row's delay timer would. The sweep defaults to row 1: its presets have a dirty tree,
            // whose top row opens a card with no commit in it.
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
            cardMessageTimer.asked = false
            cardMessageTimer.start()
        } else if (act === "card-note-lit") {
            // The same note rested on, written where a real pointer writes (verify-ui スキル §hover の絵の撮り方).
            cardNoteTimer.row = arg === "" ? 0 : Number(arg)
            cardNoteTimer.pointed = false
            cardNoteTimer.start()
        } else if (act === "menu-hover") {
            // Same row and default as `commit-menu`; the row under the menu is then asked for its card.
            let hoverOid = arg
            if (hoverOid === "")
                hoverOid = graphModel.oidAt(graphModel.rowOf(workingTree.headOid) + 1)
            page.openRowMenu(hoverOid)
            menuHoverTimer.oidHex = hoverOid
            menuHoverTimer.asked = false
            menuHoverTimer.start()
        } else if (act === "menu-list") {
            // The chip's stacked list and a standing menu (デザイン規約 §メニュー): `behind` asks for HEAD's list with
            // a graph row's menu up, `under` raises that menu over the open list; `own` raises the menu on the list's
            // first row and takes the hand off the chip — the one menu it stays under (`list-menu` reads that menu's
            // cards, at once).
            menuListTimer.how = arg === "" ? "behind" : arg
            menuListTimer.step = 0
            menuListTimer.start()
        } else if (act === "graph-choose-dbl") {
            // Toggling a row out of the choice and back in, quickly, arrives as a double-click, modifier and all
            // (`tst_moddblclick`), and the plain double-click is `switch` (デザイン規約 §複数のコミットを選ぶ). The
            // argument is the rows, as `graph-choose` takes them.
            const dblRows = arg === "" ? [] : arg.split(":").map(Number)
            chooseTimer.sweeps = false
            chooseTimer.rows = dblRows.length >= 2 ? dblRows : [1, 3, 5]
            chooseTimer.step = 0
            chooseTimer.readOid = ""
            chooseTimer.after = "dbl"
            chooseTimer.start()
        } else if (act === "graph-choose-diff") {
            // A row of the merged file list opens each chosen commit's own patch of it, stacked
            // (デザイン規約 §複数のコミットを選ぶ). The argument is the path; the rows are fixed at three of
            // `--preset basic`, two of which touch `src/topic.txt`.
            chooseTimer.sweeps = false
            chooseTimer.rows = [3, 5, 6]
            chooseTimer.step = 0
            chooseTimer.readOid = ""
            chooseTimer.after = "diff"
            chooseTimer.diffPath = arg === "" ? "src/topic.txt" : arg
            chooseTimer.start()
        } else if (act === "graph-choose-drop") {
            // A commit taken back out of the chosen list with Ctrl (デザイン規約 §複数のコミットを選ぶ). The argument
            // is the rows, as `graph-choose` takes them.
            const kept = arg === "" ? [] : arg.split(":").map(Number)
            chooseTimer.sweeps = false
            chooseTimer.rows = kept.length >= 2 ? kept : [1, 3, 5]
            chooseTimer.step = 0
            chooseTimer.readOid = ""
            chooseTimer.after = "drop"
            chooseTimer.start()
        } else if (act === "graph-choose-said" || act === "graph-choose-sweep") {
            // A row of the chosen list: its card saying the whole message (`said`), or its words dragged away
            // (`sweep`). The argument is the rows, as `graph-choose` takes them.
            const held = arg === "" ? [] : arg.split(":").map(Number)
            chooseTimer.sweeps = false
            chooseTimer.rows = held.length >= 2 ? held : [1, 3, 5]
            chooseTimer.step = 0
            chooseTimer.readOid = ""
            chooseTimer.after = act === "graph-choose-said" ? "said" : "sweep"
            chooseTimer.start()
        } else if (act === "graph-choose" || act === "graph-choose-range") {
            // The first press is plain; the held ones after it move only the choice (デザイン規約 §複数のコミットを選ぶ).
            // The argument is the rows, `:`-separated — Ctrl one at a time, or `-range`'s one Shift press from the
            // first to the last, so that one wants exactly two.
            const picked = arg === "" ? [] : arg.split(":").map(Number)
            chooseTimer.sweeps = act === "graph-choose-range"
            chooseTimer.rows = picked.length >= 2 ? picked : (chooseTimer.sweeps ? [1, 5] : [1, 3, 5])
            chooseTimer.step = 0
            chooseTimer.readOid = ""
            chooseTimer.after = ""
            chooseTimer.start()
        } else {
            return false
        }
        return true
    }
    // Pressed at the row's own click (`GraphRowDelegate.leftClick`): what a held modifier does is decided there and in
    // the page, so writing the choice directly would test neither.
    SampleTimer {
        id: chooseTimer
        property var rows: []
        /// Whether the presses after the first sweep a range (Shift) or take one row each (Ctrl).
        property bool sweeps: false
        property int step: 0
        /// The commit the plain press landed on, for the report's `read=`.
        property string readOid: ""
        /// What `chosenRowTimer` goes on to once the choice stands (`said` / `sweep` / `drop` / `diff` / `dbl`); `""`
        /// stops at the choice itself.
        property string after: ""
        property string diffPath: ""
        /// How many commits the presses should end up holding.
        function wanted() {
            if (!chooseTimer.sweeps)
                return chooseTimer.rows.length
            return Math.abs(chooseTimer.rows[chooseTimer.rows.length - 1] - chooseTimer.rows[0]) + 1
        }
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
                // A row far down has no delegate until the view is over it: sent there, and pressed on a later tick.
                if (!item) {
                    graphPane.view.positionViewAtIndex(want, ListView.Contain)
                    return
                }
                if (chooseTimer.step === 0) {
                    chooseTimer.readOid = item.oid_hex
                    item.leftClick(Qt.NoModifier)
                } else {
                    item.leftClick(chooseTimer.sweeps ? Qt.ShiftModifier : Qt.ControlModifier)
                }
                chooseTimer.step++
                return
            }
            // Every press is in; four things have to land: the tally, the rows' highlight of it, the pane having
            // answered for this choice — `selectionLoaded`, since a failed read also stops loading and leaves the
            // empty list a no-change choice has — and the commit list laid out, since an unlaid-out view measures 0.
            const lit = chooseTimer.litRows()
            if (page.chosenCount !== chooseTimer.wanted() || lit !== chooseTimer.wanted()
                    || !detailsModel.selectionLoaded || !detailsPane.chosenListDrawn)
                return
            chooseTimer.stop()
            if (chooseTimer.after !== "") {
                chosenRowTimer.start()
                return
            }
            Harness.report("graph_choose chosen=" + page.chosenCount
                              + " lit=" + lit
                              + " read=" + (page.selectedOid === chooseTimer.readOid)
                              + " wip=" + page.wipShown
                              // How far the commit list runs past its view, margins counted: a choice this small
                              // fits, so anything but 0 scrolls to reach nothing.
                              + " spare=" + detailsPane.chosenListSpare
                              + " compared=" + detailsModel.comparing
                              + " files=" + detailsModel.fileTotal
                              + " rows=" + chooseTimer.rows.join(","))
            driver.complete()
        }
    }
    // Everything that goes on from a standing choice (`chooseTimer.after`), each put in at the thing that answers it.
    SampleTimer {
        id: chosenRowTimer
        /// How far the drop has got: plain press, held press, then the pane's answer. Latched: each press goes in once.
        property int dropStep: 0
        /// The commit the presses land on, and what the row said about a press without the modifier.
        property string dropOid: ""
        property bool dropTakes: true
        /// Commits held after the plain press — unchanged means the words underneath kept their press.
        property int dropKept: -1
        /// Two presses at the row's own functions; only the held second one is the row's, the plain first the words'.
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
                // As the area delivers the pair: two clicks, then the double the second is also reported as.
                const at = graphPane.view.itemAtIndex(chooseTimer.rows[chooseTimer.rows.length - 1])
                if (!at)
                    return
                chosenRowTimer.stop()
                at.leftClick(Qt.ControlModifier)
                const led = at.doubleClick(Qt.ControlModifier)
                // `led=` is the row's own answer: a `switch` would land ticks later, so the branch on screen says
                // nothing yet. `movable=` rules out a row that turns the gesture down for leading nowhere.
                Harness.report("chosen_dbl led=" + led
                                  + " movable=" + at.movable
                                  + " naming=" + (graphPane.namingOid !== "")
                                  + " branch=" + workingTree.branch)
                driver.complete()
                return
            }
            if (chooseTimer.after === "diff") {
                // Through the page's own opener, the one a file row's press goes to; the stack is the shot, so its rows
                // are waited for.
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
            if (!row || !row.rowReady())
                return
            if (chooseTimer.after === "drop") {
                chosenRowTimer.dropOne(row)
                return
            }
            if (chooseTimer.after === "said") {
                // Entered where the row's rest would open its card (hover cannot be injected).
                if (!rowCard.opened) {
                    row.askCard()
                    return
                }
                chosenRowTimer.stop()
                // `held=` stays right after `lit=`: the wall's run is judged on the three before it, and `must_say`
                // reads one unbroken stretch of the line.
                Harness.report("chosen_said open=" + rowCard.opened
                                  + " door=" + rowCard.asksForMore
                                  + " lit=" + (row.cardOid === row.oid_hex)
                                  + " held=" + rowCard.messageCut
                                  + " cut=" + row.summaryCut)
                driver.complete()
                return
            }
            chosenRowTimer.stop()
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
        /// Which row of the list `ref-list-follow` presses the name under; -1 for the verbs that press none.
        property int followRow: -1
        onTriggered: {
            // -1 stands for HEAD's row, which is only known once the graph has loaded.
            if (refListOpenTimer.row < 0)
                refListOpenTimer.row = graphModel.loading || graphModel.rowTotal === 0 ? -1 : graphModel.headRow
            const stacked = refListOpenTimer.row < 0 ? null : graphPane.view.itemAtIndex(refListOpenTimer.row)
            if (!stacked)
                return
            refListOpenTimer.stop()
            if (refListOpenTimer.cards)
                graphPane.view.rowHoverRequested(stacked, true)
            graphPane.view.chipExpandRequested(
                stacked.oid_hex, stacked.index, stacked.chipItem.records, stacked.chipItem)
            if (refListOpenTimer.cards) {
                graphPane.view.rowHoverRequested(stacked, true)
                rowCardTimer.oidHex = stacked.oid_hex
                rowCardTimer.row = refListOpenTimer.row
                rowCardTimer.byNumber = false
                rowCardTimer.start()
            } else if (refListOpenTimer.menuRow >= 0)
                listMenuTimer.start()
            else if (refListOpenTimer.litRow >= 0)
                listLitTimer.start()
            else if (refListOpenTimer.chooseRow >= 0)
                listChooseTimer.start()
            else if (refListOpenTimer.followRow >= 0)
                listFollowTimer.start()
            else
                refListShownTimer.start()
        }
    }
    // The name under a row of that list, pressed or rested on once the list is up. The landing is read off the graph —
    // the selection is the press's own bookkeeping — as the graph's row for that commit lit and laid out (`nav-jump`).
    SampleTimer {
        id: listFollowTimer
        property bool lit: false
        property bool pressed: false
        property var to: null
        /// Whether the rows under the card were told the rest of the gesture is not theirs, read at the press: the
        /// hush lasts one double-click window, and a busy machine reads the landing later.
        property bool hushed: false
        onTriggered: {
            const at = refListOpenTimer.followRow
            if (!listFollowTimer.pressed) {
                if (!refList.opened)
                    return
                // A row the list has not built is not a row with nothing under it.
                if (!refList.rowAt(at))
                    return
                listFollowTimer.to = refList.mateTo(at)
                const said = " row=" + refListOpenTimer.row + " card=" + at
                if (listFollowTimer.to === null) {
                    // Nothing under that name to press: that is the answer.
                    listFollowTimer.stop()
                    Harness.report((listFollowTimer.lit ? "ref_list_follow_lit" : "ref_list_follow") + said
                                      + " followed=false list=" + refList.opened)
                    driver.complete()
                    return
                }
                if (listFollowTimer.lit) {
                    // A real hand on the name is on its row too and lights both; the band is judged against that wash.
                    refList.pointRow(at)
                    refList.pointMate(at)
                    listFollowTimer.stop()
                    Harness.report("ref_list_follow_lit" + said
                                      + " list=" + refList.opened
                                      + " aimed=" + refList.mateAimed(at)
                                      + " to=" + listFollowTimer.to.key)
                    renderedBarrier.begin()
                    return
                }
                refList.followRow(at)
                listFollowTimer.hushed = graphPane.rowClicksHushed
                listFollowTimer.pressed = true
                return
            }
            const want = listFollowTimer.to.oid
            if (refList.opened || page.selectedOid !== want || !driver.cardSettled)
                return
            const landedAt = graphModel.rowOf(want)
            const item = landedAt >= 0 ? graphPane.view.itemAtIndex(landedAt) : null
            if (landedAt >= 0 && !item)
                return
            listFollowTimer.stop()
            Harness.report("ref_list_follow row=" + refListOpenTimer.row + " card=" + at
                              + " followed=true list=" + refList.opened
                              + " landed=" + (!!item && item.oid_hex === want && item.selected)
                              + " hushed=" + listFollowTimer.hushed
                              + " to=" + listFollowTimer.to.key + " at=" + landedAt)
            driver.complete()
        }
    }
    // The held click on a row of that list once it is up; the rows draw the choice a tick behind the press.
    SampleTimer {
        id: listChooseTimer
        property bool pressed: false
        /// The commit read before the press, for the report's `read=`.
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
            // `selectionLoaded` for the reason `chooseTimer` gives.
            const lit = acts.litRowsTo(refListOpenTimer.row)
            if (page.chosenCount !== 2 || lit !== 2 || !detailsModel.selectionLoaded)
                return
            listChooseTimer.stop()
            Harness.report("ref_list_choose chosen=" + page.chosenCount
                              + " lit=" + lit
                              + " read=" + (page.selectedOid === listChooseTimer.readOid)
                              + " list=" + refList.opened
                              + " row=" + refListOpenTimer.row)
            driver.complete()
        }
    }
    // The right-click on a row of that list, once it is up (an unopened popup has no rows).
    SampleTimer {
        id: listMenuTimer
        onTriggered: {
            if (!refList.opened)
                return
            if (!refList.menuRow(refListOpenTimer.menuRow))
                return
            listMenuTimer.stop()
            // `list=` is read with the menu up: the card has to still stand under it (デザイン規約 §メニュー の例外).
            // `copy=` / `branch=` / `tag=`: which card the named row brought up is what this gesture decides. The
            // WORKTREE card stands on every commit (`Create worktree here…`), so `copy=` is whether it stands on a
            // copy.
            Harness.report("list_menu list=" + refList.opened
                              + " menu=" + commitMenu.opened
                              + " copy=" + (driver.commitCopyCard.path !== "")
                              + " branch=" + commitBranchCard.applies
                              + " tag=" + commitTagCard.applies
                              + " rows=" + commitMenu.offeredRows)
            driver.complete()
        }
    }
    SampleTimer {
        id: listLitTimer
        onTriggered: {
            if (!refList.opened)
                return
            if (!refList.pointRow(refListOpenTimer.litRow))
                return
            listLitTimer.stop()
            // `lit=` is read off the row: the wash is one shade over the card's ground. `nowhere=`: only a row with
            // nowhere to go has no name colour to stand in for its wash.
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
            const stacked = graphPane.view.itemAtIndex(refListOpenTimer.row)
            if (!stacked)
                return
            refListShownTimer.stop()
            Harness.report("ref_list " + acts.coverOf(stacked.chipItem.chipItem) + " row=" + refListOpenTimer.row)
            renderedBarrier.begin()
        }
    }
    // The view is sent to its end until the stand-in is up on the top edge (as `graph-head` does); then the pointer
    // goes onto its chip and the card is waited for on that chip, since a card on a row's chip is up too.
    SampleTimer {
        id: headListTimer
        property bool pointed: false
        property bool click: false
        property bool pressed: false
        property bool movedOff: false
        /// As `listFollowTimer.hushed`.
        property bool hushed: false
        property string want: ""
        onTriggered: {
            const pin = graphPane.headPin
            // Pressed: the landing is read off the graph — the stand-in's row laid out and lit, the card gone.
            if (headListTimer.pressed) {
                const at = graphModel.headRow
                const item = at >= 0 ? graphPane.view.itemAtIndex(at) : null
                if (refList.opened || page.selectedOid !== headListTimer.want || !driver.cardSettled || !item)
                    return
                headListTimer.stop()
                const listAfter = refList.opened
                // The row now where the stand-in stood is told the still pointer is on its chip and must open nothing
                // (`GraphPane.settleUnderHand`) — the chip's half, where a card opens at once, is the one that can go
                // wrong.
                // The first row in view: at the head of the list `contentY` sits in the top margin, above row 0.
                const underAt = graphPane.view.indexAt(1, Math.max(0, graphPane.view.contentY) + 1)
                const under = underAt >= 0 ? graphPane.view.itemAtIndex(underAt) : null
                if (under)
                    under.pointerRowX = under.labelsW / 2
                const part = under ? under.pointedPart : ""
                const opened = refList.opened || rowHost.refListWanted
                const held = part === "chip" && !opened
                if (under)
                    under.pointerRowX = -1
                Harness.report("graph_head_list_click list=" + listAfter
                                  // The row read, lit, and the one the keyboard walks from — all three are the press's.
                                  + " landed=" + (item.oid_hex === headListTimer.want && item.selected
                                                  && graphPane.view.currentIndex === at)
                                  + " pin=" + pin.visible
                                  + " hushed=" + headListTimer.hushed
                                  + " held=" + held
                                  + " at=" + at
                                  // What `held=` is made of, for a reader of a red run.
                                  + " under=" + underAt + " part=" + part + " opened=" + opened
                                  + " holding=" + graphPane.rowHandHeld)
                driver.complete()
                return
            }
            if (graphModel.loading || graphModel.rowTotal === 0 || !pin.wanted)
                return
            // Off HEAD first, once: the page opens on HEAD, so a press that picked nothing would still read green.
            if (headListTimer.click && !headListTimer.movedOff) {
                const off = graphModel.headRow + 1
                if (off >= graphModel.rowTotal)
                    return
                page.activateRow(graphModel.oidAt(off), off)
                headListTimer.movedOff = true
                return
            }
            if (!headListTimer.pointed) {
                if (!pin.visible || !pin.rowAbove) {
                    graphPane.view.positionViewAtEnd()
                    return
                }
                headListTimer.pointed = true
                // The middle of the front card, where a hand reaching for the name lands.
                pin.pointerX = pin.chipItem.x + pin.chipItem.chipItem.width / 2
                return
            }
            if (!refList.opened || rowHost.refListAnchor !== pin.chipItem)
                return
            if (headListTimer.click) {
                // In at the card's own row, the way a hand presses it (`RefListPopup.clickRow`).
                headListTimer.want = graphModel.oidAt(graphModel.headRow)
                if (!refList.clickRow(0))
                    return
                headListTimer.hushed = graphPane.rowClicksHushed
                headListTimer.pressed = true
                return
            }
            headListTimer.stop()
            // `on=` is the stand-in's own "is the card on me", which takes its sheets down; `covers=` is read after.
            Harness.report("graph_head_list shown=" + pin.visible
                              + " list=" + refList.opened
                              + " on=" + pin.listOnThisChip
                              + " " + acts.coverOf(pin.chipItem.chipItem)
                              + " above=" + pin.rowAbove
                              + " names=" + pin.records.length)
            renderedBarrier.begin()
        }
    }
    // Samples until the card is up (`tipDelayMs` after the rest), then makes the return in one turn.
    SampleTimer {
        id: cardReturnTimer
        /// The row the hand rests on, leaves and comes back to.
        property int row: 0
        /// Where the card stood the first time, and the pointer x that put it there. The return is at another x: the
        /// seat follows the pointer, so only then would a reopened card move.
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
            // Off the row into the card and back a little further along, read in the same turn: pointer to hold is one
            // synchronous stretch, so the hold is back before the beat can start or not at all
            // (rules-refs/app-ui.md「動詞の前提条件は入力を出す枝で読む」).
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
        /// The commit the card was asked of, so its row can be asked back whether it is lit — by commit, as the row
        /// answers (`GraphRowDelegate.cardOnThisRow`): another copy's rows land on a pass of their own and can shift
        /// the row number between the hover and this read.
        property string oidHex: ""
        /// The row number instead, for a row with no commit of its own (`byNumber`): the working tree's and every
        /// copy's row share the all-zero id.
        property int row: 0
        property bool byNumber: false
        onTriggered: {
            if (!rowCard.opened && !refList.opened)
                return
            // A pass that moved the row lays its delegates out again, so the row can be a beat from standing.
            const row = rowCardTimer.byNumber ? rowCardTimer.row : graphModel.rowOf(rowCardTimer.oidHex)
            const asked = row >= 0 ? graphPane.view.itemAtIndex(row) : null
            if (row >= 0 && !asked)
                return
            rowCardTimer.stop()
            Harness.report(
            // `lit=` next to `open=`: the pair is judged as one unbroken stretch (`verify::verbs::must_say`).
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
    // The press on that card's note, and where it leaves the reader; the shot is of the arrival.
    SampleTimer {
        id: cardMessageTimer
        property int row: 0
        property bool asked: false
        /// Whether the run goes on to put the mark away with Escape, entered at the key handler's body
        /// (`RepoPage.escapePressed`); the key reaching it is held by `tests/qml/tst_escape.qml`.
        property bool escapes: false
        /// Which commit the press was of, kept because the card takes its own copy down with it.
        property string oidHex: ""
        onTriggered: {
            if (!cardMessageTimer.asked) {
                const hovered = graphPane.view.itemAtIndex(cardMessageTimer.row)
                if (!hovered)
                    return
                graphPane.view.rowHoverRequested(hovered, true)
                // The note stands only under a cut message (規約 §hover のツールチップ), and a card fills its fields in
                // after it opens.
                if (!rowCard.opened || !rowCard.messageCut)
                    return
                cardMessageTimer.oidHex = hovered.oid_hex
                rowCard.askMessage()
                cardMessageTimer.asked = true
                return
            }
            // The arrival: the card gone and the pane holding that commit's message — a shot before it frames whatever
            // was open before.
            if (rowCard.opened || detailsPane.details.shaHex !== cardMessageTimer.oidHex)
                return
            cardMessageTimer.stop()
            // Its own sentence: the two runs are judged on opposite answers, each as one unbroken stretch of a line.
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
    // The hand resting on that note: put in once the note is there (as the press waits), read a sample later off the
    // paint.
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
                if (!rowCard.opened || !rowCard.messageCut)
                    return
                rowCard.notePointedAt = true
                cardNoteTimer.pointed = true
                return
            }
            cardNoteTimer.stop()
            // `lit=` is the rule (the line under the words is the words' colour); `word=` pins the step, which `lit=`
            // alone cannot (規約 §hover のツールチップ).
            Harness.report(
                "card_note lit=" + Qt.colorEqual(rowCard.noteRuleColor, rowCard.noteWordColor)
                + " word=" + rowCard.noteWordColor
                + " rule=" + rowCard.noteRuleColor)
            driver.complete()
        }
    }
    // The same card's words taken from the air around them (規約 §hover のツールチップ); the starts are the air itself,
    // sampled (`SweepPad.airPoints`).
    SampleTimer {
        id: cardSweepTimer
        property string lastGeom: ""
        onTriggered: {
            // The commit's words first: a card fills in after it opens, and one swept before hands back `1970-01-01`.
            if (!rowCard.opened || rowCard.subject === "")
                return
            // And stopped laying out: mid-layout its fields are at some other width (the wait `details-sweep` takes).
            const geom = Math.round(rowCard.width) + "x" + Math.round(rowCard.height)
            if (geom !== cardSweepTimer.lastGeom) {
                cardSweepTimer.lastGeom = geom
                return
            }
            cardSweepTimer.stop()
            Harness.report("card_sweep "
                + rowCard.background.pad.sweepAir(9, "open=" + rowCard.opened))
            driver.complete()
        }
    }
    // The point asked along the row and the card expected of it, for the report.
    QtObject {
        id: rowPartReport
        property int row: 0
        property string want: ""
        property real x: 0
        property bool probed: false
    }
    // Waits for either card: a moved boundary opens the other, and waiting for the right one would run to the watchdog.
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
    // PGG_AUTO_ACT=ref-list-leave. Each step goes in once the one before has landed, and the last waits out the beat
    // the leave started (`RowHoverHost.listSettling`) — a leave that starts none is read at once, which is the list
    // left standing with no hand near it.
    SampleTimer {
        id: leaveTimer
        /// Where the hand comes on (a row number or `head`), and where it goes after (`off` / `back` / `menu` / a row
        /// number).
        property string from: ""
        property string to: ""
        property int step: 0
        /// The hand goes on to another row's column (`to` is its number) rather than staying about this one.
        function crosses() {
            return leaveTimer.to !== "off" && leaveTimer.to !== "back" && leaveTimer.to !== "menu"
        }
        /// The point along the row the hand came on at, and whether the card that opened lies over it — read off
        /// the scene's two boxes, so a card that covers the point makes the run say so rather than pass for another.
        property real handX: 0
        property bool covered: false
        /// The card's box as it first stood: a hand come back holds the list rather than laying it out again, and the
        /// list laid out again under its unstacked chip is measured off a chip without its fan.
        property string firstBox: ""
        function boxOf() {
            const face = refList.background
            const at = face.mapToItem(null, 0, 0)
            return Math.round(at.x) + "," + Math.round(at.y)
                   + " " + Math.round(face.width) + "x" + Math.round(face.height)
        }
        /// The ask and the beat as they stood in the turn of the leave.
        property bool heldAfter: false
        property bool settlingAfter: false
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
        onTriggered: {
            if (graphModel.loading || graphModel.rowTotal === 0)
                return
            const item = leaveTimer.surface(leaveTimer.from)
            if (leaveTimer.step === 0) {
                if (leaveTimer.from === "head" && item.wanted && (!item.visible || !item.rowAbove)) {
                    graphPane.view.positionViewAtEnd()
                    return
                }
                if (leaveTimer.from !== "head" && item === null) {
                    graphPane.view.positionViewAtIndex(Number(leaveTimer.from), ListView.Contain)
                    return
                }
                const other = leaveTimer.crosses() ? leaveTimer.surface(leaveTimer.to) : null
                if (!Awaited.all("ref_list_leave", {
                        "from": item !== null && item.visible && item.chipItem.visible,
                        "to": !leaveTimer.crosses() || (other !== null && other.visible && other.chipItem.visible)
                    }))
                    return
                leaveTimer.handX = leaveTimer.bareX(item)
                if (leaveTimer.handX < 0 || (other !== null && leaveTimer.bareX(other) < 0)) {
                    leaveTimer.stop()
                    Harness.report("ref_list_leave to=" + leaveTimer.to + " bare=false")
                    driver.complete()
                    return
                }
                item.pointerCrossed(true, leaveTimer.handX)
                leaveTimer.step = 1
                return
            }
            if (leaveTimer.step === 1) {
                // A tick after the ask, so the card stands where it was placed.
                if (!Awaited.all("ref_list_leave", {
                        "list": refList.opened, "on": rowHost.refListAnchor === item.chipItem }))
                    return
                leaveTimer.covered = leaveTimer.coveredAt(item, leaveTimer.handX)
                leaveTimer.firstBox = leaveTimer.boxOf()
                if (leaveTimer.to === "off") {
                    item.pointerCrossed(false, 0)
                } else if (leaveTimer.to === "back") {
                    item.pointerCrossed(false, 0)
                    item.pointerCrossed(true, leaveTimer.handX)
                } else if (leaveTimer.to === "menu") {
                    // Into the card, and a right-click on its first row: that row's menu stands on the list
                    // (`RepoPage.menuRaisedOn` = `refList`). Not built yet: no press has gone in.
                    if (!refList.menuRow(0))
                        return
                    item.pointerCrossed(false, 0)
                    leaveTimer.step = 3
                    return
                } else {
                    // Qt tells the row the hand came onto before the row it left (`deliverHoverEvent`: the rows under
                    // the point, then the ones no longer under it).
                    const other = leaveTimer.surface(leaveTimer.to)
                    other.pointerCrossed(true, leaveTimer.bareX(other))
                    item.pointerCrossed(false, 0)
                }
                leaveTimer.heldAfter = rowHost.refListWanted
                leaveTimer.settlingAfter = rowHost.listSettling
                leaveTimer.step = 2
                return
            }
            if (leaveTimer.step === 3) {
                // The hand back on the chip's column while the menu stands, and the menu let go of as Escape does:
                // its close asks the list again (`RepoPage` → `settleRefList`).
                if (!Awaited.all("ref_list_leave", { "menu": commitMenu.opened }))
                    return
                item.pointerCrossed(true, leaveTimer.handX)
                leaveTimer.heldAfter = rowHost.refListWanted
                leaveTimer.settlingAfter = rowHost.listSettling
                commitMenu.dismiss()
                leaveTimer.step = 2
                return
            }
            if (!Awaited.all("ref_list_leave", { "menu": !commitMenu.visible, "beat": !rowHost.listSettling }))
                return
            leaveTimer.stop()
            const target = leaveTimer.to === "off" ? null
                         : leaveTimer.crosses() ? leaveTimer.surface(leaveTimer.to) : item
            Harness.report("ref_list_leave to=" + leaveTimer.to
                              + " covered=" + leaveTimer.covered
                              + " list=" + refList.opened
                              + " on=" + (target !== null && rowHost.refListAnchor === target.chipItem)
                              + " same=" + (leaveTimer.boxOf() === leaveTimer.firstBox)
                              // How the leave was taken, for a reader of a red run.
                              + " held=" + leaveTimer.heldAfter
                              + " settling=" + leaveTimer.settlingAfter
                              + " x=" + leaveTimer.handX
                              + " box=" + leaveTimer.firstBox + "->" + leaveTimer.boxOf())
            driver.complete()
        }
    }
    // PGG_AUTO_ACT=menu-list. The list is HEAD's — the one commit sure to have a chip — and a graph row's menu the row
    // under it's, as `menu-hover` raises it. Each step waits for the one before to land; the list goes, or stays out,
    // in the same turn as the ask, so the answer is read once the menu is up — for `own`, once the beat the hand's
    // leaving started has run out.
    SampleTimer {
        id: menuListTimer
        property string how: ""
        property int step: 0
        onTriggered: {
            const head = graphModel.loading || graphModel.rowTotal === 0 ? -1 : graphModel.headRow
            const stacked = head < 0 ? null : graphPane.view.itemAtIndex(head)
            const other = head < 0 ? "" : graphModel.oidAt(head + 1)
            if (menuListTimer.step === 0) {
                if (!Awaited.all("menu_list_rows", { "head": stacked !== null, "other": other !== "" }))
                    return
                if (menuListTimer.how === "behind")
                    page.openRowMenu(other)
                else
                    graphPane.view.chipExpandRequested(stacked.oid_hex, stacked.index, stacked.chipItem.records,
                                                       stacked.chipItem)
                menuListTimer.step = 1
            } else if (menuListTimer.step === 1) {
                if (menuListTimer.how === "behind") {
                    if (!Awaited.all("menu_list_menu", { "menu": commitMenu.opened, "head": stacked !== null }))
                        return
                    graphPane.view.chipExpandRequested(stacked.oid_hex, stacked.index, stacked.chipItem.records,
                                                       stacked.chipItem)
                } else {
                    if (!Awaited.all("menu_list_open", { "list": refList.opened }))
                        return
                    if (menuListTimer.how === "under") {
                        page.openRowMenu(other)
                    } else {
                        // Not built yet: no press has gone in.
                        if (!refList.menuRow(0))
                            return
                        // The hand off the chip, as its row reports it (`GraphRowDelegate.settlePointed`).
                        graphPane.view.chipCollapseRequested(stacked.chipItem)
                    }
                }
                menuListTimer.step = 2
            } else {
                if (!Awaited.all("menu_list_after", {
                        "menu": commitMenu.opened,
                        "beat": menuListTimer.how !== "own" || !rowHost.listSettling
                    }))
                    return
                menuListTimer.stop()
                Harness.report("menu_list how=" + menuListTimer.how + " menu=" + commitMenu.opened
                                  + " list=" + refList.opened + " raised=" + page.menuRaisedOn)
                driver.complete()
            }
        }
    }
    // The row under a standing menu, asked for its card where its delay timer would ask. Asked once the menu is up,
    // read a turn later: a card that was going to open opens synchronously.
    SampleTimer {
        id: menuHoverTimer
        property string oidHex: ""
        property bool asked: false
        onTriggered: {
            if (!commitMenu.opened)
                return
            if (!menuHoverTimer.asked) {
                const row = graphPane.view.itemAtIndex(graphModel.rowOf(menuHoverTimer.oidHex))
                // Not laid out yet: latching here would wait for an answer to a question nobody put.
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
}
