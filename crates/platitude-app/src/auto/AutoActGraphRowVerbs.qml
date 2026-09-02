pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The graph's rows: the card a row puts out, the chips' own list, and the parts a row is made of. What a
/// second click on a lit row does is the family this one hands over to (`AutoActGraphReclickVerbs`).
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
    readonly property var workTree: driver.workTree
    readonly property var graphModel: driver.graphModel
    readonly property var graphPane: driver.graphPane
    readonly property var commitMenu: driver.commitMenu
    readonly property var commitBranchCard: driver.commitBranchCard
    readonly property var commitTagCard: driver.commitTagCard
    readonly property var refList: driver.refList
    readonly property var rowCard: driver.rowCard

    // The gesture that names a row, which this dispatcher hands over to rather than branching on
    // (`AutoActGraphReclickVerbs`). It has no name here: nothing in this file starts one of its timers.
    AutoActGraphReclickVerbs {
        id: reclick
        driver: acts.driver
    }

    /// Runs `act` if it is one of this family's, and says whether it was. The families are asked in turn
    /// and the first to know a verb runs it — no verb is named by two of them (`AutoActDriver`).
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
            // that was pressed rather than at the one the chip draws (デザイン規約 §グラフ行の右クリック). The
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
        } else if (act === "row-card" || act === "card-sweep") {
            // Hover cannot be injected, so this enters where the row's delay timer would. **The sweep's own default is
            // row 1, not row 0**: the presets it runs on carry a dirty working tree, whose row stands at the top and
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
            return reclick.run(act, arg)
        }
        return true
    }
    // The chip expansion, entered once the row's delegate exists; `-card` hands over to `rowCardTimer`.
    SampleTimer {
        id: refListOpenTimer
        property int row: 0
        property bool cards: false
        /// Which row of the list `list-menu` presses; -1 for the verbs that only open it.
        property int menuRow: -1
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
            else
                refListShownTimer.start()
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
            // The whole of the sweep is the pad's own sentence now (`SweepPad.sweepAir`) — seven surfaces carry this
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
    // Waits for either card, not the expected one: a boundary that moved opens the other, and waiting for the right
    // answer would spend the watchdog finding that out. The verb is judged on `agrees`, not on how long it took.
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
}
