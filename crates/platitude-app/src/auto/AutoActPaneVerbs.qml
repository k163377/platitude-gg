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

    /// Runs `act` if it is this family's and says whether it was; `AutoActDriver` asks each family in turn.
    function run(act, arg) {
        if (act === "name-box") {
            // The argument is `<行>[:<列幅>]`: the chip column dragged to that width (`0` = its floor, since the
            // metrics clamp a drag), or left at its default.
            const boxCut = arg.indexOf(":")
            const boxRow = Number(boxCut < 0 ? arg : arg.substring(0, boxCut))
            if (boxCut >= 0)
                page.setGraphColumns(Number(arg.substring(boxCut + 1)), graphPane.graphColWManual)
            graphPane.startNaming(graphModel.oidAt(boxRow))
            const boxItem = graphPane.view.itemAtIndex(boxRow)
            Harness.report("name_box row=" + boxRow
                              + " label_w=" + Math.round(graphPane.labelW)
                              + " box_w=" + (boxItem ? Math.round(boxItem.nameBoxWidth) : -1))
        } else if (act === "name-box-drop") {
            // The same box, and a press landing elsewhere while it stands; argument `<行>[:<打つ名前>]`. Empty, the
            // box goes with the press; with a name in it, it stays. The name goes in the way a recycled delegate puts
            // it back (`GraphRowDelegate.takeNamingFocus`), so the box on screen holds what the run says.
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
            Harness.report("name_drop box=" + (graphPane.view.namingOid !== "")
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
        } else if (act === "middle-scroll-exit") {
            middleExitTimer.begin()
        } else if (act === "graph-bar" || act === "graph-bar-away"
                   || act === "middle-scroll") {
            // Both want lanes overflowing their column, and no demo repository has that many — so the divider is
            // pulled in.
            page.setGraphColumns(graphPane.labelWManual,
                                 Metrics.laneInset + 2 * Metrics.laneW)
            graphPanTimer.begin()
        } else if (act === "pane-bar" || act === "pane-bar-away") {
            // The details pane's bar, which a page opened on a dirty tree is not showing (`RepoPage.trySelectDefault`);
            // HEAD is the commit `--preset long` gives its eighty files.
            page.activateRow(worktree.headOid)
            paneBarTimer.begin()
        } else if (act === "bar-track") {
            // The file list `--preset long` gives HEAD eighty files for, as `bar-arrow`.
            page.activateRow(worktree.headOid)
            barTrackTimer.begin()
        } else if (act === "bar-arrow") {
            // The file list `--preset long` gives HEAD eighty files for, as `pane-bar`.
            page.activateRow(worktree.headOid)
            barArrowTimer.begin()
        } else if (act === "text-bar" || act === "text-bar-away") {
            // The argument picks a commit whose body runs past the box — one that fits never moves, and the run waits
            // out its watchdog.
            page.activateRow(graphModel.oidAt(Number(arg)))
            textBarTimer.begin()
        } else if (act === "graph-min") {
            // Pulled past the floor so the clamp answers (the floor is lane 0's co-author badge kept whole).
            page.setGraphColumns(graphPane.labelWManual, 0)
            Harness.report("graph_min w=" + graphPane.graphColW
                              + " min=" + graphPane.graphColWMin)
        } else if (act === "graph-divider") {
            // Read against two repositories: a line withheld on a linear history is only an answer next to a run where
            // it is drawn.
            graphPane.restDividerPointer(true)
            Harness.report("graph_divider shown=" + graphPane.graphDividerShown
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
    // The window cut: the walk stops at a round number of commits and only the footer says so. Waits: the walk having
    // answered (`truncated` starts false = "not asked yet" — app-ui.md §UI 自動化), the footer having a height, and the
    // view having arrived — read off the footer's own place, since `positionViewAtEnd()` is only the ask.
    SampleTimer {
        id: graphTailTimer
        onTriggered: {
            if (graphModel.loading || graphModel.rowTotal === 0)
                return
            // A ListView builds only near its viewport, so the far-end footer is asked for, then looked for.
            const tail = graphPane.view.footerItem
            if (tail === null || tail.height <= 0) {
                graphPane.view.positionViewAtEnd()
                return
            }
            // On screen whole: `positionViewAtEnd` puts the last *row* at the edge, and the pane keeps a run-out below.
            const bottom = graphPane.view.contentY + graphPane.view.height
            if (tail.y + tail.height > bottom + 0.5) {
                graphPane.view.positionViewAtEnd()
                return
            }
            graphTailTimer.stop()
            // The verdict leads with its two halves adjacent: a graph that never cut and a footer that failed to draw
            // frame alike.
            Harness.report(
                "graph_tail truncated=" + graphModel.truncated
                + " shown=" + tail.visible
                + " walked=" + graphModel.walkedTotal
                + " rows=" + graphPane.view.count
                + " height=" + Math.round(tail.height)
                + " lanes=" + graphModel.tailGeometry.length)
            driver.complete()
        }
    }
    // The press on that cut (`GraphTailFooter.loadMore`, the MouseArea handler's one line — verify-ui スキル
    // 「注入はハンドラ本体そのものへ入れる」). The new rows must arrive under the ones being read: a window regrown by
    // restarting the stream lands the same count, so `restarted=` / `held=` compare with the state before the press.
    property int tailWalkedBefore: -1
    property int tailResetsBefore: -1
    property int tailRowBefore: -1
    property bool tailWaiting: false
    SampleTimer {
        id: tailMoreTimer
        onTriggered: {
            // The footer is read only up to the press: this preset's step reaches the end of history, so it goes.
            if (acts.tailWalkedBefore < 0) {
                if (!tailMoreTimer.press())
                    return
            }
            // The wider walk lands as one replacement: rows and the footer's number arrive as `growing` comes off.
            if (graphModel.growing || graphModel.walkedTotal === acts.tailWalkedBefore)
                return
            tailMoreTimer.stop()
            Harness.report(
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
        /// Gets the cut on screen and presses it; says whether the press is out. The before-values are taken on the
        /// press's own frame.
        function press() {
            if (graphModel.loading || graphModel.rowTotal === 0)
                return false
            // Asked for, then looked for (as in `graph-tail`).
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
                // The ring, read on the press's frame: it and the footer are gone by the time of the picture.
                acts.tailWaiting = tail.waiting
                return true
            }
            tailMoreTimer.stop()
            Harness.report("graph_tail_more taken=false")
            driver.complete()
            return false
        }
    }
    // The stand-in for a HEAD scrolled off the graph (`GraphHeadPin`): each run sends the view to an edge, then reads
    // the stand-in itself. `headRow` is -1 until the walk answers, and the chips arrive a pass behind the rows.
    SampleTimer {
        id: graphHeadTimer
        /// Whether the second move (the press, or the scroll back) has been made. The first repeats: a view told to go
        /// to its end before laying its rows out stops at the end it knows.
        property bool answered: false
        /// Whether this run has been moved off HEAD's row. While HEAD is selected the stand-in wears the selection's
        /// ground (`picked=`) and does not light under the pointer, so the runs about a reader elsewhere select the
        /// newest commit first — the page opens on HEAD's (`RepoPage.trySelectDefault`).
        property bool stoodAside: false
        readonly property bool below: Harness.autoAct === "graph-head-below"
        function report() {
            const row = graphModel.headRow
            // The judged answers lead, adjacent and in the longest `must_say`'s order, so shorter ones are its
            // prefixes (a `must_say` is one substring — `verify/verbs.rs`).
            Harness.report(
                "graph_head shown=" + graphPane.headPin.visible
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
                + " bar=" + Math.round(graphPane.headPin.barRoom))
            driver.complete()
        }
        onTriggered: {
            if (graphModel.loading || graphModel.rowTotal === 0
                    || graphModel.headRow < 0 || graphModel.headLabels.length === 0) {
                Awaited.at("graph_head", "rows")
                return
            }
            const act = Harness.autoAct
            if (!graphHeadTimer.stoodAside) {
                graphHeadTimer.stoodAside = true
                if (act === "graph-head-below" || act === "graph-head-go"
                        || act === "graph-head-lit") {
                    const top = graphModel.newestCommitRow()
                    if (top >= 0 && top !== graphModel.headRow)
                        page.activateRow(graphModel.oidAt(top), top)
                }
            }
            if (!graphHeadTimer.answered) {
                // The stand-in has to be up on this run's edge first: the runs about it going would call an empty band
                // a success, and a preset whose HEAD starts off the bottom hands the others one nobody scrolled for.
                if (!graphPane.headPin.visible || graphPane.headPin.rowAbove === graphHeadTimer.below) {
                    Awaited.at("graph_head", "edge")
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
                    graphPane.headPin.activated(graphPane.headPin.headRow, Qt.NoModifier)
                    return
                }
                graphHeadTimer.stop()
                graphHeadTimer.report()
                return
            }
            // Both are judged on the row being on screen (the stand-in stepped aside); a press also on the selection.
            // The move again while it has not taken: asked once and lost, nothing else brings HEAD's row back.
            if (graphPane.headPin.visible || !graphPane.view.rowOnScreen(graphModel.headRow)
                    || (act === "graph-head-go" && graphPane.view.currentIndex !== graphModel.headRow)) {
                Awaited.at("graph_head", "landed")
                if (act === "graph-head-go" && graphPane.headPin.visible)
                    graphPane.headPin.activated(graphPane.headPin.headRow, Qt.NoModifier)
                else if (act === "graph-head-back" && !graphPane.view.rowOnScreen(graphModel.headRow))
                    graphPane.view.positionViewAtBeginning()
                return
            }
            graphHeadTimer.stop()
            graphHeadTimer.report()
        }
    }
    // The lane column has to have taken its narrower width before there is anywhere to pan to, or a bar worth wanting.
    SampleTimer {
        id: graphPanTimer
        /// The lanes have been sent; latched so a later tick reads the ink.
        property bool sent: false
        function begin() {
            graphPanTimer.sent = false
            graphPanTimer.start()
        }
        onTriggered: {
            if (graphPane.graphXMax <= 0)
                return
            // The pointer is what puts this bar on screen (デザイン規約 §グラフを横へ送る); `-away` checks it goes too.
            graphPane.restPointer(true)
            if (Harness.autoAct === "middle-scroll") {
                graphPanTimer.stop()
                // The middle click, then a sideways drift. The argument is the column clicked — only the lanes take the
                // drift (デザイン規約 §グラフを横へ送る).
                const y = graphPane.height / 2
                const x = Harness.autoActArg === "message"
                        ? graphPane.labelW + graphPane.graphColW + Theme.spaceXl : graphPane.labelW + Theme.spaceSm
                graphPane.startAutoScroll(x, y)
                graphPane.driftPointer(x + graphPane.width, y)
                middleScrollTimer.start()
                return
            }
            // Sending the lanes brings the bar to full ink (デザイン規約 §QML 実装ルール のバーの明るさ).
            if (!graphPanTimer.sent) {
                graphPanTimer.sent = true
                graphPane.graphX = graphPane.graphXMax / 2
                return
            }
            // The rise is animated, so the tick the send landed on would report the way there.
            if (Harness.autoAct === "graph-bar" && graphPane.laneBarInk < 1)
                return
            if (Harness.autoAct === "graph-bar-away")
                graphPane.restPointer(false)
            graphPanTimer.stop()
            Harness.report(
                "graph_bar shown=" + graphPane.laneBarShown
                + " ink=" + Math.round(graphPane.laneBarInk * 100) / 100
                + " overflow=" + Math.round(graphPane.graphXMax))
            driver.complete()
        }
    }
    // The panels' own bar (`PaneScrollBar`): bright while the reader sends the list, back to the divider ink once they
    // leave (デザイン規約 §QML 実装ルール のバーの明るさ) — the pair is the claim. The send is a plain `contentY` write
    // (`changes-step` owns the arrows).
    SampleTimer {
        id: paneBarTimer
        /// Latched in order — sent, then seen at full ink: a slab that dims without having been bright proves nothing.
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
            // Laid out and with somewhere to go, or the run would go green on no bar at all.
            if (!view || view.count <= 0 || view.height <= 0
                    || view.contentHeight <= view.height + 1)
                return
            if (!paneBarTimer.sent) {
                paneBarTimer.sent = true
                // Brightness wants the reader inside and the range sent; hover cannot be injected, so the property a
                // real pointer writes is set (`AutoScrollBar.inArea`).
                bar.inArea = true
                view.contentY = (view.contentHeight - view.height) / 2
                return
            }
            if (!paneBarTimer.wasLit) {
                if (!Qt.colorEqual(bar.slabColor, Theme.borderDefault))
                    return
                paneBarTimer.wasLit = true
                // Only the reader leaving puts the slab back down.
                if (Harness.autoAct === "pane-bar-away")
                    bar.inArea = false
            }
            if (Harness.autoAct === "pane-bar-away" && !Qt.colorEqual(bar.slabColor, Theme.bgElevated))
                return
            paneBarTimer.stop()
            Harness.report("pane_bar ink=" + bar.slabColor
                              + " at=" + Math.round(view.contentY)
                              + " rows=" + view.count)
            driver.complete()
        }
    }
    // The description box's see-through bar (デザイン規約 §色 スクロールバー), read as a pair: the two ink steps are a
    // quarter apart, which a picture answers badly. The box is a `ScrollView` whose flickable never calls itself
    // moving, so the notch goes in the way the wheel's does (`DescriptionBox.rollBy`).
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
            // The commit has to have arrived, or the box holds no text.
            if (detailsModel.loading || detailsModel.shaHex !== page.selectedOid)
                return
            if (!textBarTimer.rolled) {
                // The reader in the box, then one notch down, repeated until the box has moved.
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
                // Only the reader leaving puts the ink back down.
                if (Harness.autoAct === "text-bar-away")
                    detailsPane.holdDescriptionBar(false)
            }
            // All the way down to the idle step (0.3 — デザイン規約 §QML 実装ルール のバーの明るさ); a tick inside the
            // fall reports the descent.
            if (Harness.autoAct === "text-bar-away" && detailsPane.descriptionBarInk > 0.305)
                return
            textBarTimer.stop()
            Harness.report("text_bar ink=" + Math.round(detailsPane.descriptionBarInk * 100) / 100
                              + " at=" + Math.round(detailsPane.descriptionAt))
            driver.complete()
        }
    }
    // PGG_AUTO_ACT=middle-scroll-exit: the two ways a middle-click gesture ends (デザイン規約 §グラフを横へ送る), put in at
    // the pane's own press / move / release. `held` leaves the dead zone before letting go (the drift ends with the
    // hand); `click` never leaves it (the drift stays for the next press). The ticker has to have run before either
    // is read — no movement cannot say whether anything asked.
    SampleTimer {
        id: middleExitTimer
        property int step: 0
        property real fromY: 0
        property bool movedAway: false
        function begin() {
            middleExitTimer.step = 0
            middleExitTimer.start()
        }
        onTriggered: {
            if (middleExitTimer.step === 0) {
                if (graphPane.view.count === 0)
                    return
                const y = graphPane.height / 2
                // The subject column: a press on the lanes would mix the sideways drift into the answer.
                const x = graphPane.labelW + graphPane.graphColW + Theme.spaceXl
                middleExitTimer.fromY = graphPane.view.contentY
                graphPane.startAutoScroll(x, y)
                // Past the pad for `held`, well inside it for `click`.
                graphPane.driftPointer(x, y + (Harness.autoActArg === "click"
                                               ? Metrics.middleScrollDeadZone - 5 : 200))
                middleExitTimer.step = 1
                return
            }
            if (middleExitTimer.step === 1) {
                if (graphPane.autoTicks < 2)
                    return
                middleExitTimer.movedAway =
                    Math.abs(graphPane.view.contentY - middleExitTimer.fromY) > 0.5
                graphPane.letGoAutoScroll()
                middleExitTimer.step = 2
                return
            }
            middleExitTimer.stop()
            Harness.report("middle_scroll_exit case=" + (Harness.autoActArg === "click" ? "click" : "held")
                           + " travelled=" + graphPane.autoTravelled
                           + " scrolling=" + graphPane.autoScrolling
                           + " sent=" + middleExitTimer.movedAway
                           + " ticks=" + graphPane.autoTicks)
            driver.complete()
        }
    }
    // Waits on where the lanes ended up: a carrying gesture runs `graphX` to its maximum (the pointer is a pane's width
    // out), one that does not carry has `autoPanning` false from `MiddleAutoScroll.start`. The flag holds for the whole
    // gesture (デザイン規約 §グラフを横へ送る), so a wait for it to fall never ends.
    SampleTimer {
        id: middleScrollTimer
        onTriggered: {
            if (graphPane.autoPanning && graphPane.graphX < graphPane.graphXMax - 0.5)
                return
            middleScrollTimer.stop()
            Harness.report(
            "middle_scroll lanes=" + graphPane.autoPanning
            + " x=" + Math.round(graphPane.graphX)
            + " max=" + Math.round(graphPane.graphXMax))
            driver.complete()
        }
    }
    // PGG_AUTO_ACT=bar-track: the track of the right panel's file list (`PaneScrollBar`; デザイン規約 §スクロールバーの
    // 矢印), pressed through the functions its handler calls. A click below the thumb sends one page; a hold runs on and
    // stops with the thumb's far end on the hand; a hand slid off before the pause ends keeps the run from starting;
    // letting go gives the bar back. Each stage waits on the view or the bar's own state, never on a clock.
    SampleTimer {
        id: barTrackTimer
        property string stage: ""
        property real from: 0
        property real hand: 0
        property real sent: 0
        property real moved: 0
        property bool page: false
        property bool reached: false
        property bool held: false
        property bool left: false
        property bool stopped: false
        function begin() {
            barTrackTimer.stage = "laid"
            barTrackTimer.page = false
            barTrackTimer.reached = false
            barTrackTimer.held = false
            barTrackTimer.left = false
            barTrackTimer.stopped = false
            barTrackTimer.start()
        }
        onTriggered: {
            const view = detailsPane.filesWalk.view
            const bar = detailsPane.filesBar
            // The first stage names what it waits on itself; one line a tick each would take turns at `said`.
            if (barTrackTimer.stage !== "laid")
                Awaited.at("bar_track", barTrackTimer.stage)
            if (barTrackTimer.stage === "laid") {
                if (!Awaited.all("bar_track_laid", {
                        "rows": !!view && view.count > 0,
                        "room": !!view && view.contentHeight > view.height + 1,
                        "arrows": bar.arrowEnd > 0
                    }))
                    return
                view.contentY = bar.limitY(view.originY - view.topMargin)
                barTrackTimer.from = view.contentY
                // The page of the view as pressed: the rows and the message come in one answer, and the block above
                // the list takes its height on the next polish, so the view the step lands in may be another height.
                barTrackTimer.sent = bar.trackPage
                bar.pressTrack(bar.height - bar.arrowEnd - 2)
                bar.releaseTrack()
                barTrackTimer.stage = "click"
            } else if (barTrackTimer.stage === "click") {
                if (bar.stepping)
                    return
                barTrackTimer.moved = Math.round(view.contentY - barTrackTimer.from)
                barTrackTimer.page = barTrackTimer.moved === barTrackTimer.sent
                view.contentY = bar.limitY(view.originY - view.topMargin)
                // Well below the thumb, short of the end: the run has pages to go before it reaches the hand.
                barTrackTimer.hand = bar.arrowEnd + (bar.height - 2 * bar.arrowEnd) * 0.8
                bar.pressTrack(barTrackTimer.hand)
                barTrackTimer.held = Hand.heldBar === bar
                barTrackTimer.stage = "hold"
            } else if (barTrackTimer.stage === "hold") {
                if (bar.trackWaiting || bar.trackRunning || bar.stepping)
                    return
                barTrackTimer.reached = Math.abs(bar.thumbBottom() - barTrackTimer.hand) <= 1
                bar.releaseTrack()
                view.contentY = bar.limitY(view.originY - view.topMargin)
                bar.pressTrack(bar.height - bar.arrowEnd - 2)
                bar.pointerOnTrack(false, 0)
                barTrackTimer.stage = "off"
            } else if (barTrackTimer.stage === "off") {
                if (bar.stepping)
                    return
                barTrackTimer.left = bar.trackHeld === 1 && !bar.trackWaiting && !bar.trackRunning
                bar.releaseTrack()
                barTrackTimer.stopped = Hand.heldBar !== bar
                barTrackTimer.stop()
                Harness.report("bar_track page=" + barTrackTimer.page + " reached=" + barTrackTimer.reached
                                  + " held=" + barTrackTimer.held + " left=" + barTrackTimer.left
                                  + " stopped=" + barTrackTimer.stopped + " sent=" + barTrackTimer.sent
                                  + " moved=" + barTrackTimer.moved + " at=" + Math.round(view.contentY))
                driver.complete()
            }
        }
    }
    // PGG_AUTO_ACT=bar-arrow: the arrows of the right panel's file list (`PaneScrollBar`; デザイン規約 §スクロールバーの矢印),
    // pressed through the functions their handlers call. A click sends one step; a hold runs on past it; sliding off
    // stops the run with the hand still down; letting go stops it and gives the bar back. No clock is read: a run's
    // start is waited for on the view having gone past the press's step, its stop read off the bar's own state (only a
    // running arrow moves the view), so a slow machine takes longer and says the same.
    SampleTimer {
        id: barArrowTimer
        property string stage: ""
        property real from: 0
        property int click: -1
        property bool held: false
        property bool left: false
        property bool stopped: false
        function begin() {
            barArrowTimer.stage = "laid"
            barArrowTimer.click = -1
            barArrowTimer.held = false
            barArrowTimer.left = false
            barArrowTimer.stopped = false
            barArrowTimer.start()
        }
        onTriggered: {
            const view = detailsPane.filesWalk.view
            const bar = detailsPane.filesBar
            // The first stage names what it waits on itself; one line a tick each would take turns at `said`.
            if (barArrowTimer.stage !== "laid")
                Awaited.at("bar_arrow", barArrowTimer.stage)
            if (barArrowTimer.stage === "laid") {
                // Laid out, with somewhere to go and arrows to press.
                if (!Awaited.all("bar_arrow_laid", {
                        "rows": !!view && view.count > 0,
                        "room": !!view && view.contentHeight > view.height + 1,
                        "arrows": bar.arrowEnd > 0
                    }))
                    return
                view.contentY = bar.limitY(view.originY - view.topMargin)
                barArrowTimer.from = view.contentY
                bar.pressArrow(1)
                bar.releaseArrow()
                barArrowTimer.stage = "click"
            } else if (barArrowTimer.stage === "click") {
                if (bar.stepping)
                    return
                barArrowTimer.click = Math.round(view.contentY - barArrowTimer.from)
                barArrowTimer.from = view.contentY
                bar.pressArrow(1)
                barArrowTimer.held = Hand.heldBar === bar
                barArrowTimer.stage = "hold"
            } else if (barArrowTimer.stage === "hold") {
                // Past the press's own step: the run has started.
                if (view.contentY <= barArrowTimer.from + Metrics.arrowStep + 1)
                    return
                bar.pointerOnArrow(false)
                barArrowTimer.left = bar.arrowHeld === 1 && !bar.arrowRunning
                bar.releaseArrow()
                // Back to the start: the second hold has room to run, wherever the first one got to.
                view.contentY = bar.limitY(view.originY - view.topMargin)
                barArrowTimer.from = view.contentY
                bar.pressArrow(1)
                barArrowTimer.stage = "again"
            } else if (barArrowTimer.stage === "again") {
                if (view.contentY <= barArrowTimer.from + Metrics.arrowStep + 1)
                    return
                bar.releaseArrow()
                barArrowTimer.stopped = !bar.arrowRunning && Hand.heldBar !== bar
                barArrowTimer.stop()
                Harness.report("bar_arrow click=" + barArrowTimer.click + " held=" + barArrowTimer.held
                                  + " left=" + barArrowTimer.left + " stopped=" + barArrowTimer.stopped
                                  + " end=" + bar.arrowEnd + " at=" + Math.round(view.contentY))
                driver.complete()
            }
        }
    }
}
