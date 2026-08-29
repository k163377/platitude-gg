pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The furniture around the panes: the bar between columns, the lane bar, the head pin, the tail footer, and the
/// box a name is typed into.
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
    readonly property var graphModel: driver.graphModel
    readonly property var detailsModel: driver.detailsModel
    readonly property var graphPane: driver.graphPane
    readonly property var detailsPane: driver.detailsPane

    /// Runs `act` if it is one of this family's, and says whether it was. The families are asked in turn
    /// and the first to know a verb runs it — no verb is named by two of them (`AutoActDriver`).
    function run(act, arg) {
        if (act === "name-box") {
            // The argument is `<行>[:<列幅>]`. The width is what a hand would drag the chip column's divider to, and
            // the box is drawn against it — `0` asks for the column's own floor, since the metrics clamp what a drag
            // asks for. Without one the column is left wherever it was, which is the default width.
            const boxCut = arg.indexOf(":")
            const boxRow = Number(boxCut < 0 ? arg : arg.substring(0, boxCut))
            if (boxCut >= 0)
                page.setGraphColumns(Number(arg.substring(boxCut + 1)), graphPane.graphColWManual)
            graphPane.startNaming(graphModel.oidAt(boxRow))
            const boxItem = graphPane.view.itemAtIndex(boxRow)
            AppBackend.report("name_box row=" + boxRow
                              + " label_w=" + Math.round(graphPane.labelW)
                              + " box_w=" + (boxItem ? Math.round(boxItem.nameBoxWidth) : -1))
        } else if (act === "name-box-drop") {
            // The same box, and the press that lands somewhere else while it stands. The argument is `<行>[:<打つ名前>]`
            // — with nothing typed the box goes with the press, with a name in it it stays. What is typed goes in the
            // way a recycled delegate puts it back (`GraphRowDelegate.takeNamingFocus`), so the box on screen holds
            // what the run says it holds.
            const nameCut = arg.indexOf(":")
            const nameRow = Number(nameCut < 0 ? arg : arg.substring(0, nameCut))
            const nameTyped = nameCut < 0 ? "" : arg.substring(nameCut + 1)
            graphPane.startNaming(graphModel.oidAt(nameRow))
            if (nameTyped !== "") {
                graphPane.view.namingText = nameTyped
                const nameItem = graphPane.view.itemAtIndex(nameRow)
                if (nameItem)
                    nameItem.takeNamingFocus()
            }
            page.releasePressedAway(null)
            AppBackend.report("name_drop box=" + (graphPane.view.namingOid !== "")
                              + " row=" + nameRow
                              + " typed=" + nameTyped)
        } else if (act === "graph-tail") {
            graphTailTimer.start()
        } else if (act === "graph-tail-more") {
            tailMoreTimer.start()
        } else if (act === "graph-head" || act === "graph-head-below"
                   || act === "graph-head-back" || act === "graph-head-go"
                   || act === "graph-head-lit") {
            graphHeadTimer.start()
        } else if (act === "graph-bar" || act === "graph-bar-away"
                   || act === "middle-scroll") {
            // Both want lanes that do not fit their column, and no demo repository has that many — the divider is
            // pulled in the way a person would.
            page.setGraphColumns(graphPane.labelWManual,
                                 Metrics.laneInset + 2 * Metrics.laneW)
            graphPanTimer.begin()
        } else if (act === "pane-bar" || act === "pane-bar-away") {
            paneBarTimer.begin()
        } else if (act === "text-bar" || act === "text-bar-away") {
            // The argument picks a commit whose body runs past the box — one that fits has no bar to raise, which the
            // report says rather than the watchdog.
            page.activateRow(graphModel.oidAt(Number(arg)))
            textBarTimer.begin()
        } else if (act === "graph-min") {
            // Pulled past the floor so the clamp answers (the floor is lane 0's co-author badge kept whole).
            page.setGraphColumns(graphPane.labelWManual, 0)
            AppBackend.report("graph_min w=" + graphPane.graphColW
                              + " min=" + graphPane.graphColWMin)
        } else if (act === "graph-divider") {
            // Read against two repositories: a line withheld on a linear history is only an answer next to a run where
            // it is drawn.
            graphPane.restDividerPointer(true)
            AppBackend.report("graph_divider shown=" + graphPane.graphDividerShown
                              + " line=" + graphPane.graphDividerLineShown
                              + " refuses=" + graphPane.graphDividerRefuses
                              + " lanes=" + graphModel.maxLanes
                              + " max=" + Math.round(graphPane.graphColWMax)
                              + " min=" + Math.round(graphPane.graphColWMin))
        } else {
            return false
        }
        return true
    }
    // The window cut: the walk stops at a round number of commits and the footer is the only thing that says so — its
    // lanes carry on for one more commit's worth and its line names the count.
    //
    // Nothing here is waited out. The walk has to have answered before `truncated` means anything (the initial false is
    // "not asked yet", not "the whole history is loaded" — app-ui.md §UI 自動化の因果性), the footer has to have been given a
    // height, and the view has to have actually arrived at the end rather than merely been told to go: `atYEnd` is the
    // output, `positionViewAtEnd()` only the ask.
    SampleTimer {
        id: graphTailTimer
        onTriggered: {
            if (graphModel.loading || graphModel.rowTotal === 0)
                return
            // The footer lives at the far end of two thousand rows, and a ListView builds what is near its viewport —
            // so it is asked for and then looked for, rather than looked for first.
            const tail = graphPane.view.footerItem
            if (tail === null || tail.height <= 0) {
                graphPane.view.positionViewAtEnd()
                return
            }
            // On screen whole, read off where it sits rather than off the call having been made: `positionViewAtEnd`
            // puts the last *row* against the edge, and this pane keeps a run-out below it, so being told to go is not
            // the same as having arrived.
            const bottom = graphPane.view.contentY + graphPane.view.height
            if (tail.y + tail.height > bottom + 0.5) {
                graphPane.view.positionViewAtEnd()
                return
            }
            graphTailTimer.stop()
            // The verdict leads, and its two halves are neighbours: a graph that never cut and one whose footer failed
            // to draw frame the same way — the end of a history and the end of what was loaded are the same picture
            // without the line.
            AppBackend.report(
                "graph_tail truncated=" + graphModel.truncated
                + " shown=" + tail.visible
                + " walked=" + graphModel.walkedTotal
                + " rows=" + graphPane.view.count
                + " height=" + Math.round(tail.height)
                + " lanes=" + (graphModel.tailGeometry === ""
                               ? 0 : graphModel.tailGeometry.split(";").length))
            driver.complete()
        }
    }
    // The press on that cut: the footer loads the next step of history (`GraphTailFooter.loadMore`).
    //
    // What it has to prove is not that more rows arrived — it is that they arrived **under the ones being read**. So
    // the run holds on to where the view was and how many times the model had been reset before the press, and the
    // report leads with the pair: a window that grew by starting the stream over would land the same row count with
    // `restarted=true`, and it is the same picture.
    //
    // The press goes in at the footer's own function — the one line the MouseArea's handler is (verify-ui スキル
    // 「注入はハンドラ本体そのものへ入れる」). Nothing is waited out here either: the walk answers by taking `growing`
    // back off, and only then is the count worth reading.
    property int tailWalkedBefore: -1
    property int tailResetsBefore: -1
    property int tailRowBefore: -1
    property bool tailWaiting: false
    SampleTimer {
        id: tailMoreTimer
        onTriggered: {
            // **Nothing about the footer is read after the press.** The step this preset loads reaches the end of the
            // history, so the footer answers by going — and a wait that kept asking for its height would sit out the
            // watchdog on the very run that worked (2026-08-29 実測).
            if (acts.tailWalkedBefore < 0) {
                if (!tailMoreTimer.press())
                    return
            }
            // The wider walk lands as one replacement, so both halves of it — the rows and the footer's own number —
            // are here on the same frame the wait comes off.
            if (graphModel.growing || graphModel.walkedTotal === acts.tailWalkedBefore)
                return
            tailMoreTimer.stop()
            AppBackend.report(
                "graph_tail_more taken=true"
                + " waiting=" + acts.tailWaiting
                + " restarted=" + (graphModel.resetCount !== acts.tailResetsBefore)
                + " held=" + (graphPane.view.firstVisibleRow() === acts.tailRowBefore)
                + " truncated=" + graphModel.truncated
                + " step=" + graphModel.windowStep
                + " walked=" + graphModel.walkedTotal
                + " was=" + acts.tailWalkedBefore
                + " rows=" + graphPane.view.count)
            driver.complete()
        }
        /// Gets the cut on screen and presses it, and says whether the press is now out. Everything the press needs
        /// to be compared against is taken here, on the frame it goes in.
        function press() {
            if (graphModel.loading || graphModel.rowTotal === 0)
                return false
            // The footer lives at the far end of two thousand rows, and a ListView builds what is near its viewport —
            // so it is asked for and then looked for, rather than looked for first (`graph-tail`).
            const tail = graphPane.view.footerItem
            if (tail === null || tail.height <= 0) {
                graphPane.view.positionViewAtEnd()
                return false
            }
            const bottom = graphPane.view.contentY + graphPane.view.height
            if (tail.y + tail.height > bottom + 0.5) {
                graphPane.view.positionViewAtEnd()
                return false
            }
            acts.tailWalkedBefore = graphModel.walkedTotal
            acts.tailResetsBefore = graphModel.resetCount
            acts.tailRowBefore = graphPane.view.firstVisibleRow()
            if (tail.loadMore()) {
                // The ring, read off the ring itself on the frame the press went in — the only place it can be read.
                // It is gone by the time this run is photographed, and on this preset so is the footer under it.
                acts.tailWaiting = tail.waiting
                return true
            }
            tailMoreTimer.stop()
            AppBackend.report("graph_tail_more taken=false")
            driver.complete()
            return false
        }
    }
    // The stand-in for a HEAD scrolled off the graph (`GraphHeadPin`). It only exists where the row does not, so each
    // of these sends the view to an edge first — and then reads the stand-in itself rather than the ask, because
    // "told to go" and "arrived" are not the same thing (the same reason `graph-tail` reads `atYEnd`).
    //
    // Nothing here is waited out: the walk has to have answered (`headRow` is -1 until it has), and the chips arrive a
    // pass behind the rows, so the row is found before it can say its own name.
    SampleTimer {
        id: graphHeadTimer
        /// Whether the second move — the press, or the scroll back — has been made. The first one is not latched: a
        /// view told to go to its end before it has laid two thousand rows out goes to the end it knows about and stays
        /// there, so the ask is repeated until the stand-in itself says it arrived (2026-08-22 実測 — one ask, and the
        /// run waited out its watchdog at the top of the graph).
        property bool answered: false
        readonly property bool below: AppBackend.autoAct === "graph-head-below"
        function report() {
            const row = graphModel.headRow
            // The five judged answers first and in one run, because a
            // `must_say` catches neighbours only (`verify/verbs.rs`).
            AppBackend.report(
                "graph_head shown=" + graphPane.headPin.visible
                + " above=" + graphPane.headPin.rowAbove
                + " onScreen=" + graphPane.view.rowOnScreen(row)
                + " lit=" + graphPane.headPin.lit
                + " landed=" + (graphPane.view.currentIndex === row)
                + " row=" + row
                + " at=" + graphPane.view.currentIndex
                // What it leaves the list's own scroll bar. A picture cannot answer it — the band is drawn over the
                // trough either way — and a zero would mean the trough behind it answers with a jump to HEAD.
                + " bar=" + Math.round(graphPane.headPin.barRoom))
            driver.complete()
        }
        onTriggered: {
            if (graphModel.loading || graphModel.rowTotal === 0
                    || graphModel.headRow < 0 || graphModel.headLabels === "")
                return
            const act = AppBackend.autoAct
            if (!graphHeadTimer.answered) {
                // The stand-in has to have come up before anything is asked of it: the two runs below are about what
                // takes it away again, and a run that never saw it would call an empty band a success.
                if (!graphPane.headPin.visible) {
                    if (graphHeadTimer.below)
                        graphPane.view.positionViewAtBeginning()
                    else
                        graphPane.view.positionViewAtEnd()
                    return
                }
                graphHeadTimer.answered = true
                if (act === "graph-head-lit") {
                    // Hover cannot be injected, so the rest goes to the one property the pointer's own arrival writes.
                    graphPane.headPin.pointed = true
                } else if (act === "graph-head-back") {
                    graphPane.view.positionViewAtBeginning()
                    return
                } else if (act === "graph-head-go") {
                    graphPane.headPin.activated(graphPane.headPin.headRow)
                    return
                }
                graphHeadTimer.stop()
                graphHeadTimer.report()
                return
            }
            // What the press and the scroll back are both judged on: the row is on screen, so the stand-in has stepped
            // aside. A press is judged on where the selection went as well.
            if (graphPane.headPin.visible || !graphPane.view.rowOnScreen(graphModel.headRow))
                return
            if (act === "graph-head-go" && graphPane.view.currentIndex !== graphModel.headRow)
                return
            graphHeadTimer.stop()
            graphHeadTimer.report()
        }
    }
    // The lane column has to have taken its narrower width before there is anywhere to pan to, or a bar worth wanting.
    SampleTimer {
        id: graphPanTimer
        /// The lanes have been sent. Latched, so the tick after it can read the ink rather than the send
        /// (app-ui.md §UI 自動化の因果性).
        property bool sent: false
        function begin() {
            graphPanTimer.sent = false
            graphPanTimer.start()
        }
        onTriggered: {
            if (graphPane.graphXMax <= 0)
                return
            // Where the pointer is, which is the whole of what puts this bar on screen (デザイン規約 §グラフを横へ送る).
            // `-away` walks it back out again: a bar that comes when the pointer does proves nothing on its own unless
            // it also goes when the pointer goes.
            graphPane.restPointer(true)
            if (AppBackend.autoAct === "middle-scroll") {
                graphPanTimer.stop()
                // The middle click, then the pointer drifting sideways off it. The argument says which column the click
                // landed in, which is the whole question — only the lanes take the sideways drift
                // (デザイン規約 §グラフを横へ送る).
                const y = graphPane.height / 2
                const x = AppBackend.autoActArg === "message"
                        ? graphPane.labelW + graphPane.graphColW + Theme.spaceXl : graphPane.labelW + Theme.spaceSm
                graphPane.startAutoScroll(x, y)
                graphPane.driftPointer(x + graphPane.width, y)
                middleScrollTimer.start()
                return
            }
            // The pointer puts the bar on screen; sending the lanes is what brings it up to full ink
            // (§QML 実装ルール のバーの明るさ). Both halves are wanted on the side that photographs it.
            if (!graphPanTimer.sent) {
                graphPanTimer.sent = true
                graphPane.graphX = graphPane.graphXMax / 2
                return
            }
            // The rise takes 200ms, so reading on the tick the send landed would report the way there rather than the
            // arrival.
            if (AppBackend.autoAct === "graph-bar" && graphPane.laneBarInk < 1)
                return
            if (AppBackend.autoAct === "graph-bar-away")
                graphPane.restPointer(false)
            graphPanTimer.stop()
            AppBackend.report(
                "graph_bar shown=" + graphPane.laneBarShown
                + " ink=" + Math.round(graphPane.laneBarInk * 100) / 100
                + " overflow=" + Math.round(graphPane.graphXMax))
            driver.complete()
        }
    }
    // The panels' own bar (`PaneScrollBar`): the pane's ink while the reader is sending the list, and the pane's own
    // divider ink once they have left it (デザイン規約 §QML 実装ルール のバーの明るさ). **The pair is the whole claim** —
    // either state alone reads as "it always looks like that", and the two are what changed.
    //
    // The list is sent by assigning `contentY` rather than by pressing the arrows: what is being proven here is the
    // bar, not the walk (`changes-step` owns the arrows), and a bar lit by a send of any kind is the claim.
    SampleTimer {
        id: paneBarTimer
        /// The list has been sent, and the slab has been seen at full ink because of it. Latched in order: a slab that
        /// dims without ever having been bright proves nothing (app-ui.md §UI 自動化の因果性).
        property bool sent: false
        property bool wasLit: false
        function begin() {
            paneBarTimer.sent = false
            paneBarTimer.wasLit = false
            paneBarTimer.start()
        }
        onTriggered: {
            const view = detailsPane.filesWalk.view
            const bar = detailsPane.filesBar
            // Laid out, and with somewhere to go. A list still measuring itself has no bar to light, and one that fits
            // has none to light either — both would go green on a run that photographs nothing.
            if (!view || view.count <= 0 || view.height <= 0
                    || view.contentHeight <= view.height + 1)
                return
            if (!paneBarTimer.sent) {
                paneBarTimer.sent = true
                // Brightness wants both halves: the reader inside the range, and the range being sent. Hover cannot be
                // injected, so the first goes to the property a real pointer writes (`AutoScrollBar.inArea`).
                bar.inArea = true
                view.contentY = (view.contentHeight - view.height) / 2
                return
            }
            if (!paneBarTimer.wasLit) {
                if (!Qt.colorEqual(bar.slabColor, Theme.borderDefault))
                    return
                paneBarTimer.wasLit = true
                // The reader leaves, which is the only thing that puts the slab back down (there is no timer — the
                // bar answers the reader, not the clock).
                if (AppBackend.autoAct === "pane-bar-away")
                    bar.inArea = false
            }
            if (AppBackend.autoAct === "pane-bar-away" && !Qt.colorEqual(bar.slabColor, Theme.bgElevated))
                return
            paneBarTimer.stop()
            AppBackend.report("pane_bar ink=" + bar.slabColor
                              + " at=" + Math.round(view.contentY)
                              + " rows=" + view.count)
            driver.complete()
        }
    }
    // The bar inside the description box, which is the style's see-through one rather than the panel's slab: a box is
    // the content's own place, not a pane's edge (デザイン規約 §色 スクロールバー). **Read as a pair** — the two steps of
    // ink are a quarter apart, which a picture answers badly on a 6px thumb.
    //
    // This is the one bar the window hands a `ScrollView`, whose flickable is not interactive and never calls itself
    // moving — a box scrolled by the wheel was showing no bar at all until it was told to watch the text instead
    // (2026-08-27 ユーザー報告). So the notch goes in the way a notch does (`DescriptionBox.rollBy`), and the bar
    // answers for itself.
    SampleTimer {
        id: textBarTimer
        property bool rolled: false
        property bool wasLit: false
        function begin() {
            textBarTimer.rolled = false
            textBarTimer.wasLit = false
            textBarTimer.start()
        }
        onTriggered: {
            // The commit has to have arrived, or the box holds no text and there is nothing to send.
            if (detailsModel.loading || detailsModel.shaHex !== page.selectedOid)
                return
            if (!textBarTimer.rolled) {
                // The reader in the box, then one notch down. A box that fits swallows the notch and stands where it
                // was, which the report says rather than the watchdog.
                detailsPane.holdDescriptionBar(true)
                detailsPane.rollDescription(-120)
                if (detailsPane.descriptionAt <= 0)
                    return
                textBarTimer.rolled = true
                return
            }
            if (!textBarTimer.wasLit) {
                if (detailsPane.descriptionBarInk < 1)
                    return
                textBarTimer.wasLit = true
                // The reader leaves the box, which is the only thing that puts the ink back down.
                if (AppBackend.autoAct === "text-bar-away")
                    detailsPane.holdDescriptionBar(false)
            }
            // All the way down to the idle step (three tenths — 規約 §QML 実装ルール のバーの明るさ), not merely on the
            // way there: the fall takes 400ms, and a tick inside it reports the descent rather than where it lands.
            if (AppBackend.autoAct === "text-bar-away" && detailsPane.descriptionBarInk > 0.305)
                return
            textBarTimer.stop()
            AppBackend.report("text_bar ink=" + Math.round(detailsPane.descriptionBarInk * 100) / 100
                              + " at=" + Math.round(detailsPane.descriptionAt))
            driver.complete()
        }
    }
    // Where the lanes ended up is the whole question, so that is what is waited for — a pan that ran and a pan that was
    // refused must not read alike in the report.
    //
    // A gesture that carries the lanes runs until they have nowhere left to go: the pointer was put a whole pane's
    // width out, so the ticker saturates the clamp and `graphX` stops at its own maximum. One that does not carry them
    // has already answered by starting without the carry — `panning` is settled in `start()` by where the click landed
    // — and no tick will ever move them.
    //
    // Not "wait for `autoPanning` to go false": the flag is kept for the whole gesture (デザイン規約 §グラフを横へ送る), and nothing
    // here ends the gesture, so the lane column's own case never completed (2026-08-16 実測: watchdog on both systems,
    // `message` passing beside it because that one never pans).
    SampleTimer {
        id: middleScrollTimer
        onTriggered: {
            if (graphPane.autoPanning && graphPane.graphX < graphPane.graphXMax - 0.5)
                return
            middleScrollTimer.stop()
            AppBackend.report(
            "middle_scroll lanes=" + graphPane.autoPanning
            + " x=" + Math.round(graphPane.graphX)
            + " max=" + Math.round(graphPane.graphXMax))
            driver.complete()
        }
    }
}
