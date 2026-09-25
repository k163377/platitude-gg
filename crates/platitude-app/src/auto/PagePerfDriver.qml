import QtQuick
import platitude
import platitude.ui

/// Each model change must reach a frame before the next action.
/// The animation duration is a sampling window.
Item {
    id: driver

    property Item page
    property RepoTab repoTab
    property GraphModel graphModel
    property WorkTreeModel worktreeModel
    property NavSectionModel branchesModel
    property DetailsModel detailsModel
    property DiffModel diffModel
    property GraphPane graphPane
    property DiffPane diffPane
    property int operation: 0
    property string wantedPath: ""
    property string wantedFingerprint: ""
    property int wantedGeneration: -1
    readonly property string caseName: PerfProbe.caseField(driver.operation, 0)
    readonly property bool wantsColour: PerfProbe.caseField(driver.operation, 3) === "coloured"
    property string stage: "opening"
    property string wantedOid: ""
    property real actionStart: 0
    property int frameBefore: 0
    property int frames: 0
    property bool sawDetails: false
    property bool sawDiff: false
    property bool scrollValid: true
    property real scrollStart: 0

    readonly property bool graphVisible: graphPane.visible && !page.diffShown
                                        && graphPane.width > 0 && graphPane.height > 0
    readonly property bool ready: AppBackend.identityState === "ready" && repoTab.state === "open" && !graphModel.loading
                                  && graphModel.finishCount > 0 && branchesModel.refsLoaded
                                  && worktreeModel.loaded

    // Every stage change goes through here (rules/app-ui.md §UI 自動化「段を持つドライバは段が変わるたびに名乗る」).
    function enter(next) {
        driver.stage = next
        Harness.report("perf_stage " + next + " operation=" + driver.operation)
    }

    function fail(reason) {
        driver.enter("failed")
        Harness.report("perf_failed reason=" + reason)
        if (PerfProbe.verifying)
            Qt.quit()
    }

    function waitFrame(next) {
        graphPane.view.forceLayout()
        if (page.diffShown)
            diffPane.view.forceLayout()
        driver.frameBefore = driver.frames
        driver.enter(next)
        // `update()`, not `requestUpdate()`: a bare request over a still offscreen scene swaps no frame
        // (rules-refs/app-ui.md「`frameSwapped` を待つなら頼むのは `window.update()`」).
        page.Window.window.update()
    }

    function tick() {
        if (driver.stage === "opening" && (graphModel.rowTotal > 0 || driver.ready) && driver.graphVisible)
            driver.waitFrame("graph-frame")
        else if (driver.stage === "ready" && driver.ready)
            driver.choose()
        else if (driver.stage === "details" && !detailsModel.loading && detailsModel.shaHex === driver.wantedOid)
            driver.waitFrame("details-frame")
        else if (driver.stage === "diff" && !diffModel.loading && diffModel.title === driver.wantedPath
                 && diffModel.fingerprint !== "" && page.diffShown)
            driver.waitFrame("diff-frame")
        else if (driver.stage === "colour" && driver.sameDiff() && diffModel.coloured) {
            driver.wantedGeneration = diffModel.rowsGen
            driver.waitFrame("colour-frame")
        }
    }

    function sameDiff() {
        return page.diffShown && page.diffPath === driver.wantedPath && detailsModel.shaHex === driver.wantedOid
                && !diffModel.loading && diffModel.fingerprint === driver.wantedFingerprint
    }

    function note(point) {
        Harness.report(point + " elapsed_ms=" + (PerfProbe.clockMs() - driver.actionStart)
                       + " case=" + driver.caseName + " operation=" + driver.operation
                       + " oid=" + driver.wantedOid + " rows_gen=" + diffModel.rowsGen
                       + " fingerprint=" + diffModel.fingerprint)
    }

    function choose() {
        if (PerfProbe.selection === "none") {
            if (page.selectedOid !== "" || detailsModel.shaHex !== "" || detailsModel.loading || page.diffShown) {
                driver.fail("none-selected-a-commit")
                return
            }
            // ListView gives its first row a current index though the page never activated a commit; clear it too.
            const hadCurrent = graphPane.view.currentIndex >= 0
            graphPane.setCurrentRow(-1)
            Harness.report("perf_selection mode=none oid=none")
            if (hadCurrent)
                driver.waitFrame("none-frame")
            else
                driver.afterInteraction()
            return
        }
        let row = -1
        const caseOid = PerfProbe.caseField(driver.operation, 1)
        const oid = caseOid !== "" ? caseOid
                    : PerfProbe.selection === "head" ? workTree.headOid : ""
        if (oid !== "")
            row = graphModel.rowOf(oid)
        else if (PerfProbe.selection === "first") {
            // Past the WIP row and any stash over the tip (`GraphModel.newestCommitRow`).
            row = graphModel.newestCommitRow()
        }
        if (row < 0) {
            driver.fail("selection-not-in-loaded-graph")
            return
        }
        driver.wantedOid = graphModel.oidAt(row)
        graphPane.view.positionViewAtIndex(row, ListView.Contain)
        graphPane.view.forceLayout()
        const item = graphPane.view.itemAtIndex(row)
        if (!item) {
            driver.fail("selection-delegate-missing")
            return
        }
        driver.enter("details")
        driver.actionStart = PerfProbe.clockMs()
        page.releasePressedAway(null)
        item.leftClick(Qt.NoModifier)
        Harness.report("perf_selection mode=" + PerfProbe.selection + " oid=" + driver.wantedOid
                       + " case=" + driver.caseName + " operation=" + driver.operation)
        driver.tick()
    }

    function afterDetails() {
        driver.sawDetails = true
        driver.note("perf_details_frame")
        if (!PerfProbe.withDiff) {
            driver.afterInteraction()
            return
        }
        let file = 0
        const caseFile = PerfProbe.caseField(driver.operation, 2)
        if (caseFile !== "") {
            file = -1
            for (let i = 0; i < detailsModel.fileTotal; i++) {
                if (detailsModel.filePathAt(i) === caseFile) {
                    file = i
                    break
                }
            }
        }
        if (file < 0 || detailsModel.fileTotal === 0) {
            driver.fail("diff-file-missing")
            return
        }
        driver.enter("diff")
        driver.wantedPath = detailsModel.filePathAt(file)
        driver.actionStart = PerfProbe.clockMs()
        Harness.report("perf_file path=" + detailsModel.filePathAt(file))
        page.openDiff("commit", detailsModel.filePathAt(file), detailsModel.fileOrigPathAt(file))
        driver.tick()
    }

    function afterInteraction() {
        if (driver.operation + 1 < PerfProbe.operationCount()) {
            if (page.diffShown)
                page.closeDiff()
            driver.operation++
            driver.waitFrame("next-frame")
            return
        }
        if (Harness.autoScroll) {
            const returning = page.diffShown
            if (returning)
                page.closeDiff()
            graphPane.view.positionViewAtBeginning()
            if (returning)
                driver.waitFrame("scroll-frame")
            else
                driver.beginScroll()
        } else {
            driver.finish()
        }
    }

    function afterDiff() {
        if (PerfProbe.diffScroll) {
            diffPane.view.forceLayout()
            if (!diffPane.visible || diffBench.to <= diffBench.from) {
                driver.fail("diff-scroll-hidden-or-no-overflow")
                return
            }
            const before = diffPane.view.contentY
            diffPane.view.positionViewAtBeginning()
            // A still offscreen scene swaps no further frame, and this diff's frame was already seen: wait for one
            // only if the viewport moved.
            if (diffPane.view.contentY === before)
                driver.beginDiffScroll()
            else
                driver.waitFrame("diff-scroll-frame")
        } else {
            driver.afterInteraction()
        }
    }

    function beginDiffScroll() {
        driver.scrollStart = diffPane.view.contentY
        driver.scrollValid = true
        driver.enter("diff-scrolling")
        PerfProbe.scrollSurface("diff")
        PerfProbe.scrollContext(driver.caseName, driver.operation)
        PerfProbe.beginScroll()
        diffBench.start()
    }

    function finish() {
        driver.enter("finished")
        Harness.report("perf_complete selection=" + PerfProbe.selection + " details=" + driver.sawDetails
                          + " diff=" + driver.sawDiff + " graph=" + driver.graphVisible
                          + " scrolled=" + Harness.autoScroll + " rows=" + graphModel.rowTotal)
        Harness.report("perf_operations count=" + (driver.operation + 1))
        page.perfFinished()
    }

    function beginScroll() {
        graphPane.view.forceLayout()
        driver.reportViewport("start")
        if (!driver.graphVisible || scrollBench.to <= scrollBench.from) {
            driver.fail("scroll-hidden-or-no-overflow")
            return
        }
        driver.enter("scrolling")
        driver.scrollStart = graphPane.view.contentY
        PerfProbe.scrollSurface("graph")
        PerfProbe.beginScroll()
        scrollBench.start()
    }

    function reportViewport(at) {
        const view = graphPane.view
        Harness.report("perf_viewport clock_ms=" + PerfProbe.clockMs() + " at=" + at
                          + " y=" + view.contentY + " origin=" + view.originY
                          + " height=" + view.contentHeight + " viewport=" + view.height
                          + " row=" + view.indexAt(view.width / 2, view.contentY + view.height / 2))
    }

    function frame() {
        if (driver.stage === "diff-scrolling") {
            driver.scrollValid = driver.scrollValid && driver.sameDiff() && diffPane.visible
            PerfProbe.frame()
            return
        }
        if (driver.stage === "scrolling") {
            driver.scrollValid = driver.scrollValid && driver.graphVisible
            PerfProbe.frame()
            return
        }
        if (!driver.stage.endsWith("-frame"))
            return
        if (driver.frames <= driver.frameBefore) {
            page.Window.window.update()
            return
        }
        if (driver.stage === "graph-frame") {
            if (!driver.graphVisible) {
                driver.fail("first-graph-hidden")
                return
            }
            Harness.report("perf_graph_frame clock_ms=" + PerfProbe.clockMs() + " visible=true")
            driver.enter("ready")
            driver.tick()
        } else if (driver.stage === "none-frame") {
            if (graphPane.view.currentIndex >= 0)
                driver.fail("none-retained-current-row")
            else
                driver.afterInteraction()
        } else if (driver.stage === "details-frame") {
            driver.afterDetails()
        } else if (driver.stage === "diff-frame") {
            driver.sawDiff = true
            driver.wantedFingerprint = diffModel.fingerprint
            driver.note("perf_diff_frame")
            if (driver.wantsColour) {
                driver.enter("colour")
                driver.tick()
            } else {
                driver.afterDiff()
            }
        } else if (driver.stage === "colour-frame") {
            if (!driver.sameDiff() || !diffModel.coloured || diffModel.rowsGen !== driver.wantedGeneration) {
                driver.fail("colour-frame-changed-target")
                return
            }
            driver.note("perf_colour_frame")
            driver.afterDiff()
        } else if (driver.stage === "next-frame") {
            driver.choose()
        } else if (driver.stage === "diff-scroll-frame") {
            driver.beginDiffScroll()
        } else if (driver.stage === "diff-end-frame") {
            const view = diffPane.view
            const row = view.indexAt(view.width / 2, view.contentY + view.height / 2)
            if (!driver.sameDiff() || row < 0 || !view.itemAtIndex(row)) {
                driver.fail("diff-scroll-ended-without-visible-row")
                return
            }
            Harness.report("perf_diff_scroll_frame visible=true case=" + driver.caseName
                           + " operation=" + driver.operation + " row=" + row)
            driver.afterInteraction()
        } else if (driver.stage === "scroll-frame") {
            driver.beginScroll()
        } else if (driver.stage === "scroll-end-frame") {
            const view = graphPane.view
            const row = view.indexAt(view.width / 2, view.contentY + view.height / 2)
            const item = row >= 0 ? view.itemAtIndex(row) : null
            if (!driver.graphVisible || !item || !item.visible) {
                driver.fail("scroll-ended-without-visible-row")
                return
            }
            Harness.report("perf_scroll_frame visible=true row=" + row)
            driver.finish()
        }
    }

    Connections {
        target: driver.graphModel
        function onStatsChanged() { driver.tick() }
    }
    Connections {
        target: driver.branchesModel
        function onChanged() { driver.tick() }
    }
    Connections {
        target: driver.worktreeModel
        function onChanged() { driver.tick() }
    }
    Connections {
        target: driver.repoTab
        function onChanged() { driver.tick() }
    }
    Connections {
        target: driver.detailsModel
        function onChanged() { driver.tick() }
    }
    Connections {
        target: driver.diffModel
        function onChanged() { driver.tick() }
    }
    Connections {
        target: driver.page.Window.window
        function onFrameSwapped() { driver.frames++; driver.frame() }
    }

    NumberAnimation {
        id: diffBench
        target: driver.diffPane.view
        property: "contentY"
        from: driver.diffPane.view.originY
        to: from + Math.max(0, driver.diffPane.view.contentHeight - driver.diffPane.view.height)
        // waits(measured): sample visible diff frames over this animation; movement and a final row are required
        duration: 2000
        onFinished: {
            const moved = Math.abs(driver.diffPane.view.contentY - driver.scrollStart)
            PerfProbe.endScroll(driver.diffPane.view.count, moved, driver.scrollValid)
            if (!driver.scrollValid || moved <= 0)
                driver.fail("diff-scroll-not-observed")
            else
                driver.waitFrame("diff-end-frame")
        }
    }

    NumberAnimation {
        id: scrollBench
        target: driver.graphPane.view
        property: "contentY"
        from: driver.graphPane.view.originY
        to: from + Math.max(0, Math.min(3000 * Theme.graphRowHeight,
                                      driver.graphPane.view.contentHeight - driver.graphPane.view.height))
        // The window the frame rate is read over; `onFinished` fails the run unless the view moved while on screen.
        // waits(measured): the length of the sample, judged by what the scroll did
        duration: 12000
        onFinished: {
            driver.reportViewport("end")
            const moved = Math.abs(driver.graphPane.view.contentY - driver.scrollStart)
            const visible = driver.scrollValid && driver.graphVisible
            PerfProbe.endScroll(driver.graphModel.rowTotal, moved, visible)
            if (!visible || moved <= 0)
                driver.fail("scroll-not-observed")
            else
                driver.waitFrame("scroll-end-frame")
        }
    }

    // Re-reads the predicates on the beat — `tick()` reads inputs no signal here is wired to (the pane's size and
    // visibility, the page's diff) — and re-asks for a frame each beat, since a request taken while the window was
    // not visible swaps nothing. Off over a scroll: the animation's end moves that stage on.
    SampleTimer {
        running: driver.stage !== "finished" && driver.stage !== "failed" && !driver.stage.endsWith("scrolling")
        onTriggered: {
            driver.tick()
            if (driver.stage.endsWith("-frame") && driver.frames <= driver.frameBefore)
                driver.page.Window.window.update()
        }
    }

    Component.onCompleted: {
        driver.enter(driver.stage)
        driver.tick()
    }
    onReadyChanged: driver.tick()
}
