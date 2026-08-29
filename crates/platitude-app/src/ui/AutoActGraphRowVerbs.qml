pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The graph's rows: what a second click on a lit row does, the card a row puts out, the chips' own list, and
/// the parts a row is made of.
///
/// Built by `AutoActDriver`, which `RepoPage` builds only when a verb was given. What these verbs act on
/// hangs off that driver; the names it owns are read back once below, so the code under them reads as it
/// did when it was all one file.
// An `Item` only because `QtObject` has no default property to hold the timers below; it draws nothing
// and is never given a size.
Item {
    id: acts

    /// The driver these verbs belong to. `var` because naming its type here would be a circle: it is
    /// the file that builds this one.
    required property var driver

    // The driver's own names, read once so what is written under them reads as it did.
    readonly property var page: driver.page
    readonly property var workTree: driver.workTree
    readonly property var graphModel: driver.graphModel
    readonly property var graphPane: driver.graphPane
    readonly property var commitMenu: driver.commitMenu
    readonly property var refList: driver.refList
    readonly property var rowCard: driver.rowCard

    /// Runs `act` if it is one of this family's, and says whether it was. The families are asked in turn
    /// and the first to know a verb runs it — no verb is named by two of them (`AutoActDriver`).
    function run(act, arg) {
        if (act === "row-part") {
            // Where the row divides, asked at a point along it. Hover cannot be injected, so this writes the one
            // property a real pointer writes (`GraphRowDelegate.pointerRowX`) and leaves every decision after that to
            // the row — **the point of the verb is the decision**, so reaching past it to `chipExpandRequested` (which
            // is what `ref-list` does) would prove nothing about the boundary. The probe itself runs from the sampler:
            // the delegate is born a layout after the model row, and an early miss must retry, not go silent.
            const parts = arg.split(":")
            rowPartReport.row = Number(parts[0])
            rowPartReport.want = parts[2]
            rowPartReport.x = Number(parts[1])
            rowPartTimer.start()
        } else if (act === "graph-reclick-lanes") {
            // The argument is the row the presses land on.
            laneClickTimer.row = Number(arg === "" ? "0" : arg)
            laneClickTimer.start()
        } else if (act === "graph-reclick" || act === "graph-rename"
                   || act === "graph-reclick-scrolled" || act === "graph-reclick-mark"
                   || act === "graph-reclick-still") {
            // The gesture at the row itself. The argument is `<行>[:<付ける名前>]` — the first row is the default, and a
            // run that wants a chip on it says which (`--preset tags`). `graph-rename` carries the same gesture
            // through to git; without a name there is nothing to carry.
            const renameCut = arg.indexOf(":")
            reclickGraphTimer.row = Number(renameCut < 0 ? (arg === "" ? "0" : arg) : arg.substring(0, renameCut))
            reclickGraphTimer.name = renameCut < 0 ? "" : arg.substring(renameCut + 1)
            reclickGraphTimer.scrolls = act === "graph-reclick-scrolled"
            reclickGraphTimer.marks = act === "graph-reclick-mark"
            reclickGraphTimer.points = act === "graph-reclick-still"
            if (act === "graph-rename")
                // The submit is ticks away; the barrier must not pass on a
                // fetch that answered in between (`expectWriteAtPress`).
                driver.expectWriteAtPress()
            reclickGraphTimer.start()
        } else if (act === "graph-reclick-list" || act === "graph-reclick-across" || act === "ref-list-pick") {
            // The same gesture, and the double-click beside it, put in at the card the chip unfolds into. The argument
            // is `<行>[:<カードの行>]` — the card's first row is the one sitting on the chip's own seat.
            const listParts = arg.split(":")
            reclickListTimer.row = listParts[0] === "" ? 0 : Number(listParts[0])
            reclickListTimer.card = listParts.length > 1 ? Number(listParts[1]) : 0
            reclickListTimer.picks = act === "ref-list-pick"
            reclickListTimer.across = act === "graph-reclick-across"
            if (reclickListTimer.picks)
                driver.expectWriteAtPress()
            reclickListTimer.start()
        } else if (act === "ref-list" || act === "ref-list-card") {
            // Hover cannot be injected, so this enters where the hover timer would — from the sampler, because the
            // row's delegate is born a layout after the model row and an early miss must retry, not pass on the
            // plain screen. `-card` walks row → card → chip → asked again from under the list: both card closes have
            // to hold, and either failing leaves `open=true`.
            refListOpenTimer.row = Number(arg)
            refListOpenTimer.cards = act === "ref-list-card"
            refListOpenTimer.start()
        } else if (act === "row-card" || act === "card-sweep") {
            // Hover cannot be injected, so this enters where the row's delay timer would. **The sweep's own default is
            // row 1, not row 0**: the presets it runs on carry a dirty working tree, whose row stands at the top and
            // opens a card with no commit in it (2026-08-28 実測 — `subject` empty, the stamp `1970-01-01`).
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
        } else {
            return false
        }
        return true
    }
    // The chip expansion, entered once the row's delegate exists. The bare verb then ends on the opened
    // list; `-card` hands over to `rowCardTimer`, which owns its report.
    SampleTimer {
        id: refListOpenTimer
        property int row: 0
        property bool cards: false
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
            } else {
                refListShownTimer.start()
            }
        }
    }
    // The bare verb's own end: the list is up. Waited on `opened` — an unopened popup frames as the
    // plain screen, which is exactly the miss this family retries against.
    SampleTimer {
        id: refListShownTimer
        onTriggered: {
            if (!refList.opened)
                return
            refListShownTimer.stop()
            renderedBarrier.begin()
        }
    }
    // The card is opened synchronously; this just lets the layout settle before it is measured and photographed.
    SampleTimer {
        id: rowCardTimer
        /// The row the card was asked of, so the report can ask it back whether it is still lit. Read off the row
        /// rather than off the host that wrote it — the whole point is that the row got the answer.
        property int row: 0
        onTriggered: {
            if (!rowCard.opened && !refList.opened)
                return
            rowCardTimer.stop()
            const asked = graphPane.view.itemAtIndex(rowCardTimer.row)
            AppBackend.report(
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
            // (2026-08-28 実測).
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
            // The whole of the sweep is the pad's own sentence now (`SweepPad.sweepAir`) — seven surfaces carry this
            // hand and were each asking it the same four things. `open=` is this verb's own half and goes in where it
            // always stood.
            AppBackend.report("card_sweep "
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
    // Waits for either card rather than for the one that was expected: a boundary that moved opens the other one, and
    // waiting for the right answer would spend the whole watchdog finding that out. The rest is a real `tipDelayMs`,
    // which this samples through — the verb is judged on `agrees`, not on how long it took.
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
            AppBackend.report(
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
    // PG_AUTO_ACT=graph-reclick / graph-reclick-list / ref-list-pick: the two clicks of the rename gesture put in at a
    // graph row, and at a row of the card its chip unfolds into — and, on the same card, the double-click that is the
    // way to move. Every step waits for its own answer: the row has to exist before it can be clicked, the card has to
    // be up before one of its rows can be, and the double-click window the first click opened has to have passed
    // before a second one counts as a second (app-ui.md §UI 自動化の因果性).
    property bool reclickGraphArmed: false
    property int reclickGraphStep: 0
    /// When the second click went in, so the report can say how long the box made the reader wait. **Not a success
    /// condition** — the wait is the double-click window and the sampler reads through it (app-ui.md §UI 自動化の因果性);
    /// it is here because a lag is the one thing about this gesture a picture cannot show (2026-08-26 ユーザー報告).
    property real reclickGraphAt: 0
    SampleTimer {
        id: reclickGraphTimer
        property int row: 0
        /// The name to put in the box once it is open, for the run that carries the gesture through to git
        /// (`graph-rename`); empty for the one that stops at the box.
        property string name: ""
        /// Whether the run scrolls the history away between the second click and the box. **The delegate is pooled by
        /// that scroll**, which is what the gesture must not be carried by (`ReclickGesture`) — and the box that
        /// opens has to be sent back into view (2026-08-26 ユーザー報告「入力モードに切り替わらないケースも有った」).
        property bool scrolls: false
        /// Whether the run ends inside the wait, on the mark the chip wears while it runs, instead of at the box.
        property bool marks: false
        /// Whether the pointer is rested on the chip first, so the card it opens has a rest running under the wait —
        /// what the gesture has to hold still (2026-08-26 ユーザー指示).
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
                acts.reclickGraphAt = Date.now()
                item.leftClick(0)
                // Read where it is set, not where it lapses: the wait is short and the box is what it turns into.
                acts.reclickGraphArmed = item.renameArmed
                // The history walks away under the wait: the row that was clicked is pooled, and what was armed on it
                // has to survive that. Far enough that the row is well outside the view's own buffer.
                if (reclickGraphTimer.scrolls)
                    graphPane.view.positionViewAtIndex(reclickGraphTimer.row + 200, ListView.Beginning)
                if (reclickGraphTimer.marks) {
                    // **The mark is what the reader has to see during the wait**, so this run ends inside it rather
                    // than at the box. Read off the chip itself (`RefChip.waiting`), because a picture taken a beat
                    // late frames the box instead and would say nothing either way.
                    reclickGraphTimer.stop()
                    AppBackend.report(
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
                AppBackend.report(
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
                // of the report a picture of an open box cannot make (2026-08-26 ユーザー報告).
                + " branch=" + workTree.branch
                // Whether the row the box is on is in sight — the whole of the scrolled run's claim, and true of the
                // plain one for nothing having moved it.
                + " shown=" + graphPane.rowOnScreen(reclickGraphTimer.row)
                // How long the box took, against the window it is waiting out. The sampler's own beat is in the
                // difference, so this is read as "about the window", not to the millisecond.
                + " wait=" + (Date.now() - acts.reclickGraphAt)
                + " window=" + Application.styleHints.mouseDoubleClickInterval)
                if (reclickGraphTimer.name === "") {
                    driver.complete()
                    return
                }
                // Through to git, by the path the field's own Enter takes. The write barrier finishes this one.
                graphPane.view.namingSubmitted(graphModel.oidAt(reclickGraphTimer.row),
                                               reclickGraphTimer.name, graphPane.namingMode)
                driver.pressedWrite()
            }
        }
    }
    // PG_AUTO_ACT=graph-reclick-lanes: the same gesture put in through **the strip over the lane column** — the half
    // of the row between the two dividers, which has a press-taking layer of its own wherever the lanes overflow their
    // column (`GraphLanePan`). A reader aiming at the middle of a wide graph is aiming at that strip, and a strip that
    // answered with a copy of half of what a click does left the gesture doing nothing there (2026-08-26 ユーザー報告).
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
                AppBackend.report(
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
        /// Whether this run is the double-click that moves rather than the two clicks that name.
        property bool picks: false
        /// Whether the two clicks are put in at **different surfaces** — the first at the row, the second at the card
        /// its chip opens into. That is what a reader does without knowing it: the card comes up on the chip's own
        /// seat after a rest, so the second click at one spot lands somewhere else (2026-08-26 ユーザー報告).
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
                    if (!refList.doubleClickRow(reclickListTimer.card))
                        return
                    driver.pressedWrite()
                    reclickListTimer.stop()
                    AppBackend.report("ref_list_pick row=" + reclickListTimer.row
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
                AppBackend.report(
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
            AppBackend.report("menu_hover menu=" + commitMenu.opened + " card=" + rowCard.opened
                              + " list=" + refList.opened)
            driver.complete()
        }
    }
}
