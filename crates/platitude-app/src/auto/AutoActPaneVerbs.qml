pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The furniture around the panes: the bar between columns, the lane bar, the head pin, the tail footer, and the
/// box a name is typed into.
// `Item` because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    /// `var`: typing it `AutoActDriver` would be circular — that file builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var worktree: driver.worktree
    readonly property var graphModel: driver.graphModel
    readonly property var detailsModel: driver.detailsModel
    readonly property var graphPane: driver.graphPane
    readonly property var detailsPane: driver.detailsPane

    /// The name verbs' argument, `<行>[:<…>]`: the row the box opens on, and what follows the `:` ("" for none).
    function nameRow() {
        const arg = Harness.autoActArg
        const cut = arg.indexOf(":")
        return Number(cut < 0 ? arg : arg.substring(0, cut))
    }
    function nameRest() {
        const arg = Harness.autoActArg
        const cut = arg.indexOf(":")
        return cut < 0 ? "" : arg.substring(cut + 1)
    }

    /// Runs `act` if it is this family's and says whether it was; `AutoActDriver` asks each family in turn.
    function run(act, arg) {
        if (act === "name-box") {
            // The argument is `<行>[:<列幅>]`: the chip column dragged to that width (`0` = its floor, since the
            // metrics clamp a drag), or left at its default.
            if (arg.indexOf(":") >= 0)
                page.setGraphColumns(Number(acts.nameRest()), graphPane.graphColWManual)
            graphPane.startNaming(graphModel.oidAt(acts.nameRow()))
            driver.reportAtCompletion(() => {
                const boxItem = graphPane.view.itemAtIndex(acts.nameRow())
                return "name_box row=" + acts.nameRow()
                       + " label_w=" + Math.round(graphPane.labelW)
                       + " box_w=" + (boxItem ? Math.round(boxItem.nameBoxWidth) : -1)
            })
        } else if (act === "name-box-drop") {
            // The same box, and a press landing elsewhere while it stands; argument `<行>[:<打つ名前>]`. Empty, the
            // box goes with the press; with a name in it, it stays. The name goes in the way a recycled delegate puts
            // it back (`GraphRowDelegate.takeNamingFocus`), so the box on screen holds what the run says.
            graphPane.startNaming(graphModel.oidAt(acts.nameRow()))
            if (acts.nameRest() !== "") {
                graphPane.view.namingText = acts.nameRest()
                const nameItem = graphPane.view.itemAtIndex(acts.nameRow())
                if (nameItem)
                    nameItem.takeNamingFocus()
            }
            page.releasePressedAway(null)
            driver.reportAtCompletion(() => "name_drop box=" + (graphPane.view.namingOid !== "")
                                             + " row=" + acts.nameRow()
                                             + " typed=" + acts.nameRest())
        } else if (act === "graph-tail") {
            graphTailTimer.start()
        } else if (act === "graph-tail-more") {
            tailMoreTimer.start()
        } else if (act === "graph-head" || act === "graph-head-below"
                   || act === "graph-head-back" || act === "graph-head-go"
                   || act === "graph-head-lit") {
            // The runs about a reader elsewhere stand aside from HEAD's row first; then the view goes to the run's
            // edge; the press, the scroll back and the hand are the move after the stand-in is up.
            graphHeadTimer.order = (graphHeadTimer.standsAside ? ["stood_aside"] : []).concat(["edged"])
                .concat(act === "graph-head" || act === "graph-head-below" ? [] : ["moved"])
            graphHeadTimer.start()
        } else if (act === "middle-scroll-exit") {
            middleExitTimer.start()
        } else if (act === "graph-bar" || act === "graph-bar-away"
                   || act === "middle-scroll") {
            // Both want lanes overflowing their column, and no demo repository has that many — so the divider is
            // pulled in.
            page.setGraphColumns(graphPane.labelWManual,
                                 Metrics.laneInset + 2 * Metrics.laneW)
            // `middle-scroll` hands the run on once the drift is in; `-away` takes the pointer off after the send.
            graphPanTimer.order = act === "middle-scroll" ? ["scrolled"]
                                : act === "graph-bar-away" ? ["sent", "left"] : ["sent"]
            graphPanTimer.start()
        } else if (act === "pane-bar" || act === "pane-bar-away") {
            // The details pane's bar, which a page opened on a dirty tree is not showing (`RepoPage.trySelectDefault`);
            // HEAD is the commit `--preset long` gives its eighty files.
            page.activateRow(worktree.headOid)
            paneBarTimer.order = act === "pane-bar-away" ? ["sent", "lit", "left"] : ["sent", "lit"]
            paneBarTimer.start()
        } else if (act === "bar-track") {
            // The file list `--preset long` gives HEAD eighty files for, as `bar-arrow`.
            page.activateRow(worktree.headOid)
            barTrackTimer.start()
        } else if (act === "bar-arrow") {
            // The file list `--preset long` gives HEAD eighty files for, as `pane-bar`.
            page.activateRow(worktree.headOid)
            barArrowTimer.start()
        } else if (act === "text-bar" || act === "text-bar-away") {
            // The argument picks a commit whose body runs past the box — one that fits never moves, and the run waits
            // out its watchdog.
            page.activateRow(graphModel.oidAt(Number(arg)))
            textBarTimer.order = act === "text-bar-away" ? ["rolled", "lit", "left"] : ["rolled", "lit"]
            textBarTimer.start()
        } else if (act === "graph-min") {
            // Pulled past the floor so the clamp answers (the floor is lane 0's co-author badge kept whole).
            page.setGraphColumns(graphPane.labelWManual, 0)
            driver.reportAtCompletion(() => "graph_min w=" + graphPane.graphColW
                                             + " min=" + graphPane.graphColWMin)
        } else if (act === "graph-divider") {
            // `pin-lit` selects the newest commit before the stand-in can light under the hand; both `pin` runs put
            // the stand-in up before the rest.
            graphDividerTimer.order = arg === "pin-lit" ? ["stood_aside", "pinned", "rested"]
                                    : arg === "pin" ? ["pinned", "rested"] : ["rested"]
            graphDividerTimer.start()
        } else {
            return false
        }
        return true
    }
    /// Whether the graph's far-end footer stands on screen whole: `positionViewAtEnd` puts the last *row* at the edge,
    /// and the pane keeps a run-out below.
    function footerShown() {
        const tail = graphPane.view.footerItem
        return {
            "footer": tail !== null && tail.height > 0,
            "whole": tail !== null && tail.y + tail.height <= graphPane.view.contentY + graphPane.view.height + 0.5 }
    }
    /// A ListView builds only near its viewport, so the footer is asked for until it is there (`footerShown`): the
    /// press of the stage that brings it, answering whether it stands.
    function showFooter() {
        const shown = acts.footerShown()
        if (shown.footer && shown.whole)
            return true
        graphPane.view.positionViewAtEnd()
        return false
    }
    // The window cut: the walk stops at a round number of commits and only the footer says so. Waits: the walk having
    // answered (`truncated` starts false = "not asked yet" — app-ui.md §UI 自動化), the footer having a height, and the
    // view having arrived — read off the footer's own place, since `positionViewAtEnd()` is only the ask.
    PageStage {
        id: graphTailTimer
        name: "graph_tail"
        onTriggered: {
            const op = graphTailTimer
            if (!Awaited.all("graph_tail_ready", { "ready": !(graphModel.loading || graphModel.rowTotal === 0) }))
                return
            // The far-end footer asked for, then looked for — on screen whole.
            if (op.at("start") && !op.input("shown", () => acts.showFooter()))
                return
            // The verdict leads with its two halves adjacent: a graph that never cut and a footer that failed to draw
            // frame alike.
            op.complete({
                "name": "graph_tail_shown",
                "when": () => acts.footerShown(),
                "report": () => {
                    const footer = graphPane.view.footerItem
                    return "graph_tail truncated=" + graphModel.truncated
                           + " shown=" + (footer !== null && footer.visible)
                           + " walked=" + graphModel.walkedTotal
                           + " rows=" + graphPane.view.count
                           + " height=" + (footer !== null ? Math.round(footer.height) : -1)
                           + " lanes=" + graphModel.tailGeometry.length
                } })
        }
    }
    // The press on that cut (`GraphTailFooter.loadMore`, the MouseArea handler's one line — verify-ui スキル
    // 「注入はハンドラ本体そのものへ入れる」). The new rows must arrive under the ones being read: a window regrown by
    // restarting the stream lands the same count, so `restarted=` / `held=` compare with the state before the press,
    // taken on the press's own frame.
    PageStage {
        id: tailMoreTimer
        name: "graph_tail_more"
        order: ["shown", "pressed"]
        onTriggered: {
            const op = tailMoreTimer
            if (op.at("start")) {
                // The footer is read only up to the press: this preset's step reaches the end of history, so it goes.
                if (!Awaited.all("graph_tail_more_ready", {
                        "ready": !(graphModel.loading || graphModel.rowTotal === 0) }))
                    return
                // Asked for, then looked for (as in `graph-tail`).
                if (!op.input("shown", () => acts.showFooter()))
                    return
            }
            if (op.at("shown")) {
                const tail = graphPane.view.footerItem
                if (!op.input("pressed", () => {
                        op.witness("walked", graphModel.walkedTotal)
                        op.witness("resets", graphModel.resetCount)
                        op.witness("row", graphPane.view.firstVisibleRow())
                        const taken = op.witness("taken", tail.loadMore())
                        // The ring, read on the press's frame: it and the footer are gone by the time of the picture.
                        op.witness("waiting", taken && tail.waiting)
                    }))
                    return
            }
            if (!op.witnessed("taken")) {
                op.complete({ "when": () => ({}), "report": () => "graph_tail_more taken=false" })
                return
            }
            // The wider walk lands as one replacement: rows and the footer's number arrive as `growing` comes off.
            op.complete({
                "name": "graph_tail_more_grown",
                "when": () => ({ "grown": !graphModel.growing,
                                 "walked": graphModel.walkedTotal !== op.witnessed("walked") }),
                "report": () => "graph_tail_more taken=true"
                                + " waiting=" + op.witnessed("waiting")
                                + " restarted=" + (graphModel.resetCount !== op.witnessed("resets"))
                                + " held=" + (graphPane.view.firstVisibleRow() === op.witnessed("row"))
                                + " truncated=" + graphModel.truncated
                                + " step=" + graphModel.windowStep
                                + " walked=" + graphModel.walkedTotal
                                + " was=" + op.witnessed("walked")
                                + " rows=" + graphPane.view.count })
        }
    }
    // The stand-in for a HEAD scrolled off the graph (`GraphHeadPin`): each run sends the view to an edge, then reads
    // the stand-in itself. `headRow` is -1 until the walk answers, and the chips arrive a pass behind the rows.
    PageStage {
        id: graphHeadTimer
        name: "graph_head"
        /// Whether this run is moved off HEAD's row first. While HEAD is selected the stand-in wears the selection's
        /// ground (`picked=`) and does not light under the pointer, so the runs about a reader elsewhere select the
        /// newest commit first — the page opens on HEAD's (`RepoPage.trySelectDefault`).
        readonly property bool standsAside: Harness.autoAct === "graph-head-below"
                                            || Harness.autoAct === "graph-head-go"
                                            || Harness.autoAct === "graph-head-lit"
        readonly property bool below: Harness.autoAct === "graph-head-below"
        function line() {
            const row = graphModel.headRow
            // The judged answers lead, adjacent and in the longest `must_say`'s order, so shorter ones are its
            // prefixes (a `must_say` is one substring — `verify/verbs.rs`).
            return "graph_head shown=" + graphPane.headPin.visible
                   + " above=" + graphPane.headPin.rowAbove
                   + " onScreen=" + graphPane.view.rowOnScreen(row)
                   + " lit=" + graphPane.headPin.lit
                   + " landed=" + (graphPane.view.currentIndex === row)
                   // The stand-in carrying the selection, read off its ground; with `landed=` it checks the wiring.
                   + " picked=" + graphPane.headPin.picked
                   // The list's top sliver it keeps above itself — same-coloured ground a picture barely shows.
                   + " room=" + Math.round(graphPane.headPin.topRoom)
                   + " row=" + row
                   + " at=" + graphPane.view.currentIndex
                   // What it leaves the list's scroll bar (a picture cannot say); zero would make the trough answer
                   // with a jump to HEAD.
                   + " bar=" + Math.round(graphPane.headPin.barRoom)
        }
        /// Whether the stand-in is up on this run's edge.
        function edge() {
            return graphPane.headPin.visible && graphPane.headPin.rowAbove !== graphHeadTimer.below
        }
        /// The view sent to this run's edge, asked again each beat until the stand-in is up there: one told to go to
        /// its end before laying its rows out stops at the end it knows.
        function sendToEdge() {
            if (graphHeadTimer.edge())
                return true
            if (graphHeadTimer.below)
                graphPane.view.positionViewAtBeginning()
            else
                graphPane.view.positionViewAtEnd()
            return false
        }
        /// The move of `graph-head-back` / `graph-head-go`, made again while it has not taken: asked once and lost,
        /// nothing else brings HEAD's row back. Answers whether the stand-in stepped aside for the row.
        function moveBack() {
            if (Harness.autoAct === "graph-head-go") {
                if (graphPane.headPin.visible)
                    graphPane.headPin.activated(graphPane.headPin.headRow, Qt.NoModifier)
                return !graphPane.headPin.visible
            }
            if (!graphPane.view.rowOnScreen(graphModel.headRow))
                graphPane.view.positionViewAtBeginning()
            return graphPane.view.rowOnScreen(graphModel.headRow)
        }
        onTriggered: {
            const op = graphHeadTimer
            const act = Harness.autoAct
            if (!Awaited.all("graph_head_rows", { "rows": !(graphModel.loading || graphModel.rowTotal === 0),
                                                  "head": graphModel.headRow >= 0,
                                                  "chips": graphModel.headLabels.length > 0 }))
                return
            if (op.at("start") && op.standsAside) {
                if (!op.input("stood_aside", () => {
                        const top = graphModel.newestCommitRow()
                        if (top >= 0 && top !== graphModel.headRow)
                            page.activateRow(graphModel.oidAt(top), top)
                    }))
                    return
            }
            // The stand-in has to be up on this run's edge first: the runs about it going would call an empty band a
            // success, and a preset whose HEAD starts off the bottom hands the others one nobody scrolled for.
            if ((op.at("start") || op.at("stood_aside")) && !op.input("edged", () => op.sendToEdge()))
                return
            if (op.at("edged")) {
                // The row stood aside to and the view's move are the page's reads: waited for before the run moves
                // on or answers, since the forms that end here (`below`, `lit`) have no later wait of their own.
                if (!Awaited.all("graph_head_page", { "page": PageSettled.settled(page) }))
                    return
                if (act === "graph-head-lit") {
                    // Hover cannot be injected, so the hand is put where the pointer's own arrival would put it: on
                    // the pane's hand (`HandWatch.stir`), over the stand-in's words, which the stand-in reads it off.
                    if (!op.input("moved", () => {
                            const pin = graphPane.headPin
                            const words = pin.mapToItem(null, pin.labelWidth + pin.graphColWidth + Theme.spaceLg,
                                                        pin.rowMidY)
                            graphPane.hand.stir(words)
                        }))
                        return
                } else if (act === "graph-head-back" || act === "graph-head-go") {
                    if (!op.input("moved", () => op.moveBack()))
                        return
                }
            }
            if (act === "graph-head-back" || act === "graph-head-go") {
                // Both are judged on the row being on screen (the stand-in stepped aside); a press also on the
                // selection, whose read is the page's (`PageSettled`).
                op.complete({
                    "name": "graph_head_landed",
                    "when": () => ({
                        "pin": !graphPane.headPin.visible,
                        "row": graphPane.view.rowOnScreen(graphModel.headRow),
                        "picked": Harness.autoAct !== "graph-head-go"
                                  || graphPane.view.currentIndex === graphModel.headRow,
                        "page": PageSettled.settled(page) }),
                    "report": () => op.line() })
                return
            }
            op.complete({
                "name": "graph_head_page",
                "when": () => ({ "edge": op.edge(), "page": PageSettled.settled(page) }),
                "report": () => op.line() })
        }
    }
    // The pointer resting on the graph divider, and the pane's hand with it. The hand rests on the row under the
    // selection, so a picked ground and a lit one both stand under the line; with `pin` it rests on HEAD's
    // stand-in instead, which a view sent to its end puts up at the top edge (`--preset deep`). `pin-lit` selects the
    // newest commit first, so the stand-in lights under the hand rather than wearing the selection — HEAD has to be
    // another commit (`--preset deep-detached`). The divider's pointer rests over its row once: the row's seat decides
    // on its card after the rest, and the run ends on that decision (`PageSettled`), not inside it.
    PageStage {
        id: graphDividerTimer
        name: "graph_divider"
        /// The rows on screen wearing a highlight ground, and how many of them draw the graph divider's line again over
        /// it (`GraphRowDelegate.groundLines`) — the line as drawn, not its conditions.
        function ruledRows() {
            const view = graphPane.view
            let grounds = 0
            let ruled = 0
            for (let row = view.firstVisibleRow(); row < graphModel.rowTotal; row++) {
                const item = view.itemAtIndex(row)
                if (item === null || item.y >= view.contentY + view.height)
                    break
                if (!item.selected && !item.lit && !item.marked)
                    continue
                grounds++
                if (item.groundLines !== null && item.groundLines.graphShown)
                    ruled++
            }
            return " grounds=" + grounds + " ruled=" + ruled
        }
        /// Whether HEAD's stand-in is up at the top edge.
        function pinUp() {
            return graphPane.headPin.visible && graphPane.headPin.rowAbove
        }
        onTriggered: {
            const op = graphDividerTimer
            if (!Awaited.all("graph_divider_rows", { "rows": !(graphModel.loading || graphModel.rowTotal < 2) }))
                return
            const pin = graphPane.headPin
            const arg = Harness.autoActArg
            if (arg === "pin" || arg === "pin-lit") {
                if (arg === "pin-lit" && op.at("start")) {
                    // HEAD's row is known before the stand-in can stand, so the newest commit is told apart from it —
                    // once, or the view would never stay at its end.
                    if (!Awaited.all("graph_divider_head", { "head": graphModel.headRow >= 0 }))
                        return
                    if (!op.input("stood_aside", () => {
                            const top = graphModel.newestCommitRow()
                            if (top >= 0 && top !== graphModel.headRow)
                                page.activateRow(graphModel.oidAt(top), top)
                        }))
                        return
                }
                // The view sent to its end, asked again each beat until the stand-in is up at the top edge.
                if ((op.at("start") || op.at("stood_aside")) && !op.input("pinned", () => {
                        if (op.pinUp())
                            return true
                        graphPane.view.positionViewAtEnd()
                        return false
                    }))
                    return
                // The row `pin-lit` selected has its read and the view its place (`PageSettled`).
                if (op.at("pinned")) {
                    if (!Awaited.all("graph_divider_pin_page", { "page": PageSettled.settled(page) }))
                        return
                    op.input("rested", () => {
                        graphPane.restDividerPointer(true, pin.y + pin.rowMidY)
                    })
                    return
                }
                op.complete({
                    "name": "graph_divider_pin_landed",
                    "when": () => ({ "pin": op.pinUp(), "page": PageSettled.settled(page) }),
                    "report": () => "graph_divider_pin shown=" + graphPane.headPin.visible
                                    + " ruled=" + graphPane.headPin.lines.graphShown
                                    + " line=" + graphPane.graphDividerLineShown
                                    + " picked=" + graphPane.headPin.picked
                                    + " lit=" + graphPane.headPin.lit })
                return
            }
            const below = graphPane.view.itemAtIndex(graphPane.view.currentIndex + 1)
            if (!Awaited.all("graph_divider_row", { "row": below !== null }))
                return
            if (!op.at("rested")) {
                if (!Awaited.all("graph_divider_row_page", { "page": PageSettled.settled(page) }))
                    return
                op.input("rested", () => {
                    graphPane.restDividerPointer(true, below.mapToItem(graphPane, 0, below.height / 2).y)
                })
                return
            }
            op.complete({
                "name": "graph_divider_row_landed",
                "when": () => ({ "page": PageSettled.settled(page) }),
                "report": () => "graph_divider shown=" + graphPane.graphDividerShown
                                + " line=" + graphPane.graphDividerLineShown
                                + " refuses=" + graphPane.graphDividerRefuses
                                + " lanes=" + graphModel.maxLanes
                                + " max=" + Math.round(graphPane.graphColWMax)
                                + " min=" + Math.round(graphPane.graphColWMin)
                                + op.ruledRows() })
        }
    }
    // The lane column has to have taken its narrower width before there is anywhere to pan to, or a bar worth wanting.
    PageStage {
        id: graphPanTimer
        name: "graph_pan"
        onTriggered: {
            const op = graphPanTimer
            if (!Awaited.all("graph_pan_ready", { "ready": !(graphPane.graphXMax <= 0) }))
                return
            // The pointer is what puts this bar on screen (デザイン規約 §グラフを横へ送る), rested on the pane with the
            // first press; `-away` checks it goes too.
            if (Harness.autoAct === "middle-scroll") {
                // The middle click, then a sideways drift. The argument is the column clicked — only the lanes take the
                // drift (デザイン規約 §グラフを横へ送る).
                if (!op.input("scrolled", () => {
                        graphPane.restPointer(true)
                        const y = graphPane.height / 2
                        const x = Harness.autoActArg === "message"
                                ? graphPane.labelW + graphPane.graphColW + Theme.spaceXl
                                : graphPane.labelW + Theme.spaceSm
                        graphPane.startAutoScroll(Qt.point(x, y))
                        graphPane.driftPointer(Qt.point(x + graphPane.width, y))
                    }))
                    return
                op.stop()
                middleScrollTimer.start()
                return
            }
            // Sending the lanes brings the bar to full ink (デザイン規約 §QML 実装ルール のバーの明るさ).
            if (op.at("start")) {
                op.input("sent", () => {
                    graphPane.restPointer(true)
                    graphPane.graphX = graphPane.graphXMax / 2
                })
                return
            }
            if (Harness.autoAct === "graph-bar-away" && op.at("sent")) {
                if (!op.input("left", () => {
                        graphPane.restPointer(false)
                    }))
                    return
            }
            // The rise is animated, so the tick the send landed on would report the way there.
            op.complete({
                "name": "graph_pan_lit",
                "when": () => ({ "ink": Harness.autoAct !== "graph-bar" || graphPane.laneBarInk >= 1 }),
                "report": () => "graph_bar shown=" + graphPane.laneBarShown
                                + " ink=" + Math.round(graphPane.laneBarInk * 100) / 100
                                + " overflow=" + Math.round(graphPane.graphXMax) })
        }
    }
    // The panels' own bar (`PaneScrollBar`): bright while the reader sends the list, back to the divider ink once they
    // leave (デザイン規約 §QML 実装ルール のバーの明るさ) — the pair is the claim, in order: sent, then seen at full ink —
    // a slab that dims without having been bright proves nothing. The send is a plain `contentY` write (`changes-step`
    // owns the arrows).
    PageStage {
        id: paneBarTimer
        name: "pane_bar"
        onTriggered: {
            const op = paneBarTimer
            const view = detailsPane.filesWalk.view
            const bar = detailsPane.filesBar
            // Laid out and with somewhere to go, or the run would go green on no bar at all.
            if (!Awaited.all("pane_bar_ready", {
                "rows": !!view && view.count > 0,
                "sized": !!view && view.height > 0,
                "overflows": !!view && view.contentHeight > view.height + 1 }))
                return
            if (op.at("start")) {
                // Brightness wants the reader inside and the range sent; hover cannot be injected, so the property a
                // real pointer writes is set (`AutoScrollBar.inArea`).
                op.input("sent", () => {
                    bar.inArea = true
                    view.contentY = (view.contentHeight - view.height) / 2
                })
                return
            }
            if (op.at("sent")) {
                if (!Awaited.all("pane_bar_lit", { "lit": Qt.colorEqual(bar.slabColor, Theme.borderDefault) }))
                    return
                op.advance("lit")
                // Only the reader leaving puts the slab back down.
                if (Harness.autoAct === "pane-bar-away" && !op.input("left", () => {
                        bar.inArea = false
                    }))
                    return
            }
            // The slab as the claim has it: still at full ink, or for `-away` back down.
            const away = Harness.autoAct === "pane-bar-away"
            op.complete({
                "name": away ? "pane_bar_dim" : "pane_bar_lit",
                "when": () => Harness.autoAct === "pane-bar-away"
                              ? ({ "dim": Qt.colorEqual(detailsPane.filesBar.slabColor, Theme.bgElevated) })
                              : ({ "lit": Qt.colorEqual(detailsPane.filesBar.slabColor, Theme.borderDefault) }),
                "report": () => "pane_bar ink=" + detailsPane.filesBar.slabColor
                                + " at=" + Math.round(detailsPane.filesWalk.view.contentY)
                                + " rows=" + detailsPane.filesWalk.view.count })
        }
    }
    // The description box's see-through bar (デザイン規約 §色 スクロールバー), read as a pair: the two ink steps are a
    // quarter apart, which a picture answers badly. The box is a `ScrollView` whose flickable never calls itself
    // moving, so the notch goes in the way the wheel's does (`DescriptionBox.rollBy`).
    PageStage {
        id: textBarTimer
        name: "text_bar"
        onTriggered: {
            const op = textBarTimer
            // The commit has to have arrived, or the box holds no text.
            if (!Awaited.all("text_bar_ready", {
                "ready": !(detailsModel.loading || detailsModel.shaHex !== page.selectedOid) }))
                return
            if (op.at("start")) {
                // The reader in the box, then one notch down, repeated until the box has moved.
                op.input("rolled", () => {
                    detailsPane.holdDescriptionBar(true)
                    detailsPane.rollDescription(-120)
                    return detailsPane.descriptionAt > 0
                })
                return
            }
            if (op.at("rolled")) {
                if (!Awaited.all("text_bar_lit", { "lit": detailsPane.descriptionBarInk >= 1 }))
                    return
                op.advance("lit")
                // Only the reader leaving puts the ink back down.
                if (Harness.autoAct === "text-bar-away" && !op.input("left", () => {
                        detailsPane.holdDescriptionBar(false)
                    }))
                    return
            }
            // All the way down to the idle step (0.3 — デザイン規約 §QML 実装ルール のバーの明るさ); a tick inside the
            // fall reports the descent. Without `-away`, the ink still at full.
            const away = Harness.autoAct === "text-bar-away"
            op.complete({
                "name": away ? "text_bar_dim" : "text_bar_lit",
                "when": () => Harness.autoAct === "text-bar-away"
                              ? ({ "dim": detailsPane.descriptionBarInk <= 0.305 })
                              : ({ "lit": detailsPane.descriptionBarInk >= 1 }),
                "report": () => "text_bar ink=" + Math.round(detailsPane.descriptionBarInk * 100) / 100
                                + " at=" + Math.round(detailsPane.descriptionAt) })
        }
    }
    // PGG_AUTO_ACT=middle-scroll-exit: the two ways a middle-click gesture ends (デザイン規約 §グラフを横へ送る), put in at
    // the pane's own press / move / release. `held` leaves the dead zone before letting go (the drift ends with the
    // hand); `click` never leaves it (the drift stays for the next press). The ticker has to have run before either
    // is read — no movement cannot say whether anything asked.
    PageStage {
        id: middleExitTimer
        name: "middle_exit"
        order: ["started", "let_go"]
        onTriggered: {
            const op = middleExitTimer
            if (op.at("start")) {
                if (!Awaited.all("middle_exit_rows", { "rows": graphPane.view.count > 0 }))
                    return
                op.input("started", () => {
                    const y = graphPane.height / 2
                    // The subject column: a press on the lanes would mix the sideways drift into the answer.
                    const x = graphPane.labelW + graphPane.graphColW + Theme.spaceXl
                    op.witness("fromY", graphPane.view.contentY)
                    graphPane.startAutoScroll(Qt.point(x, y))
                    // Past the pad for `held`, well inside it for `click`.
                    graphPane.driftPointer(Qt.point(x, y + (Harness.autoActArg === "click"
                                                            ? Metrics.middleScrollDeadZone - 5 : 200)))
                })
                return
            }
            if (op.at("started")) {
                if (!Awaited.all("middle_exit_ticked", { "ticked": graphPane.autoTicks >= 2 }))
                    return
                if (!op.input("let_go", () => {
                        // Whether the view moved under the drift, read as the hand lets go.
                        op.witness("sent", Math.abs(graphPane.view.contentY - op.witnessed("fromY")) > 0.5)
                        graphPane.letGoAutoScroll()
                    }))
                    return
            }
            op.complete({
                "when": () => ({}),
                "report": () => "middle_scroll_exit case=" + (Harness.autoActArg === "click" ? "click" : "held")
                                + " travelled=" + graphPane.autoTravelled
                                + " scrolling=" + graphPane.autoScrolling
                                + " sent=" + op.witnessed("sent")
                                + " ticks=" + graphPane.autoTicks })
        }
    }
    // Waits on where the lanes ended up: a carrying gesture runs `graphX` to its maximum (the pointer is a pane's width
    // out), one that does not carry has `autoPanning` false from `MiddleAutoScroll.start`. The flag holds for the whole
    // gesture (デザイン規約 §グラフを横へ送る), so a wait for it to fall never ends.
    PageStage {
        id: middleScrollTimer
        name: "middle_scroll"
        onTriggered: {
            middleScrollTimer.complete({
                "name": "middle_scroll_ready",
                "when": () => ({ "ready": !(graphPane.autoPanning && graphPane.graphX < graphPane.graphXMax - 0.5) }),
                "report": () => "middle_scroll lanes=" + graphPane.autoPanning
                                + " x=" + Math.round(graphPane.graphX)
                                + " max=" + Math.round(graphPane.graphXMax) })
        }
    }
    // PGG_AUTO_ACT=bar-track: the track of the right panel's file list (`PaneScrollBar`; デザイン規約 §スクロールバーの
    // 矢印), pressed through the functions its handler calls. A click below the thumb sends one page; a hold runs on and
    // stops with the thumb's far end on the hand; a hand slid off before the pause ends keeps the run from starting;
    // letting go gives the bar back. Each stage waits on the view or the bar's own state, never on a clock; what each
    // says is read at the press that ends it.
    PageStage {
        id: barTrackTimer
        name: "bar_track"
        order: ["clicked", "holding", "slid_off", "released"]
        onTriggered: {
            const op = barTrackTimer
            const view = detailsPane.filesWalk.view
            const bar = detailsPane.filesBar
            if (op.at("start")) {
                if (!Awaited.all("bar_track_laid", {
                        "rows": !!view && view.count > 0,
                        "room": !!view && view.contentHeight > view.height + 1,
                        "arrows": bar.arrowEnd > 0
                    }))
                    return
                op.input("clicked", () => {
                    view.contentY = bar.limitY(view.originY - view.topMargin)
                    op.witness("from", view.contentY)
                    // The page of the view as pressed: the rows and the message come in one answer, and the block
                    // above the list takes its height on the next polish, so the view the step lands in may be
                    // another height.
                    op.witness("sent", bar.trackPage)
                    bar.pressTrack(bar.height - bar.arrowEnd - 2)
                    bar.releaseTrack()
                })
            } else if (op.at("clicked")) {
                if (!Awaited.all("bar_track_click", { "stepped": !bar.stepping }))
                    return
                op.input("holding", () => {
                    op.witness("moved", Math.round(view.contentY - op.witnessed("from")))
                    view.contentY = bar.limitY(view.originY - view.topMargin)
                    // Well below the thumb, short of the end: the run has pages to go before it reaches the hand.
                    bar.pressTrack(op.witness("hand", bar.arrowEnd + (bar.height - 2 * bar.arrowEnd) * 0.8))
                    op.witness("held", Hand.heldBar === bar)
                })
            } else if (op.at("holding")) {
                if (!Awaited.all("bar_track_hold", {
                        "run_over": !(bar.trackWaiting || bar.trackRunning || bar.stepping) }))
                    return
                op.input("slid_off", () => {
                    op.witness("reached", Math.abs(bar.thumbBottom() - op.witnessed("hand")) <= 1)
                    bar.releaseTrack()
                    view.contentY = bar.limitY(view.originY - view.topMargin)
                    bar.pressTrack(bar.height - bar.arrowEnd - 2)
                    bar.pointerOnTrack(false, 0)
                })
            } else if (op.at("slid_off")) {
                if (!Awaited.all("bar_track_off", { "stepped": !bar.stepping }))
                    return
                if (!op.input("released", () => {
                        op.witness("left", bar.trackHeld === 1 && !bar.trackWaiting && !bar.trackRunning)
                        bar.releaseTrack()
                        op.witness("stopped", Hand.heldBar !== bar)
                    }))
                    return
            }
            if (!op.at("released"))
                return
            op.complete({
                "when": () => ({}),
                "report": () => "bar_track page=" + (op.witnessed("moved") === op.witnessed("sent"))
                                + " reached=" + op.witnessed("reached")
                                + " held=" + op.witnessed("held") + " left=" + op.witnessed("left")
                                + " stopped=" + op.witnessed("stopped") + " sent=" + op.witnessed("sent")
                                + " moved=" + op.witnessed("moved")
                                + " at=" + Math.round(detailsPane.filesWalk.view.contentY) })
        }
    }
    // PGG_AUTO_ACT=bar-arrow: the arrows of the right panel's file list (`PaneScrollBar`; デザイン規約 §スクロールバーの矢印),
    // pressed through the functions their handlers call. A click sends one step; a hold runs on past it; sliding off
    // stops the run with the hand still down; letting go stops it and gives the bar back. No clock is read: a run's
    // start is waited for on the view having gone past the press's step, its stop read off the bar's own state (only a
    // running arrow moves the view), so a slow machine takes longer and says the same.
    PageStage {
        id: barArrowTimer
        name: "bar_arrow"
        order: ["clicked", "holding", "again", "released"]
        onTriggered: {
            const op = barArrowTimer
            const view = detailsPane.filesWalk.view
            const bar = detailsPane.filesBar
            // Where the view stood as each press went in: the click's, then each hold's.
            if (op.at("start")) {
                // Laid out, with somewhere to go and arrows to press.
                if (!Awaited.all("bar_arrow_laid", {
                        "rows": !!view && view.count > 0,
                        "room": !!view && view.contentHeight > view.height + 1,
                        "arrows": bar.arrowEnd > 0
                    }))
                    return
                op.input("clicked", () => {
                    view.contentY = bar.limitY(view.originY - view.topMargin)
                    op.witness("clickFrom", view.contentY)
                    bar.pressArrow(1)
                    bar.releaseArrow()
                })
            } else if (op.at("clicked")) {
                if (!Awaited.all("bar_arrow_click", { "stepped": !bar.stepping }))
                    return
                op.input("holding", () => {
                    op.witness("click", Math.round(view.contentY - op.witnessed("clickFrom")))
                    op.witness("holdFrom", view.contentY)
                    bar.pressArrow(1)
                    op.witness("held", Hand.heldBar === bar)
                })
            } else if (op.at("holding")) {
                // Past the press's own step: the run has started.
                if (!Awaited.all("bar_arrow_hold", {
                        "running": view.contentY > op.witnessed("holdFrom") + Metrics.arrowStep + 1 }))
                    return
                op.input("again", () => {
                    bar.pointerOnArrow(false)
                    op.witness("left", bar.arrowHeld === 1 && !bar.arrowRunning)
                    bar.releaseArrow()
                    // Back to the start: the second hold has room to run, wherever the first one got to.
                    view.contentY = bar.limitY(view.originY - view.topMargin)
                    op.witness("againFrom", view.contentY)
                    bar.pressArrow(1)
                })
            } else if (op.at("again")) {
                if (!Awaited.all("bar_arrow_again", {
                        "running": view.contentY > op.witnessed("againFrom") + Metrics.arrowStep + 1 }))
                    return
                if (!op.input("released", () => {
                        bar.releaseArrow()
                        op.witness("stopped", !bar.arrowRunning && Hand.heldBar !== bar)
                    }))
                    return
            }
            if (!op.at("released"))
                return
            op.complete({
                "when": () => ({}),
                "report": () => "bar_arrow click=" + op.witnessed("click") + " held=" + op.witnessed("held")
                                + " left=" + op.witnessed("left") + " stopped=" + op.witnessed("stopped")
                                + " end=" + detailsPane.filesBar.arrowEnd
                                + " at=" + Math.round(detailsPane.filesWalk.view.contentY) })
        }
    }
}
