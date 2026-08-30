import QtQuick
import platitude
import platitude.ui

/// Each model change must reach a frame before the next action.
/// The animation duration is a sampling window, never a readiness delay.
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

    function fail(reason) {
        driver.stage = "failed"
        AppBackend.report("perf_failed reason=" + reason)
        if (PerfProbe.verifying)
            Qt.quit()
    }

    function waitFrame(next) {
        graphPane.view.forceLayout()
        driver.frameBefore = driver.frames
        driver.stage = next
        page.Window.window.requestUpdate()
    }

    function tick() {
        if (driver.stage === "opening" && (graphModel.rowTotal > 0 || driver.ready) && driver.graphVisible)
            driver.waitFrame("graph-frame")
        else if (driver.stage === "ready" && driver.ready)
            driver.choose()
        else if (driver.stage === "details" && !detailsModel.loading && detailsModel.shaHex === driver.wantedOid)
            driver.waitFrame("details-frame")
        else if (driver.stage === "diff" && !diffModel.loading && diffModel.title !== "" && page.diffShown)
            driver.waitFrame("diff-frame")
    }

    function choose() {
        if (PerfProbe.selection === "none") {
            if (page.selectedOid !== "" || detailsModel.shaHex !== "" || detailsModel.loading || page.diffShown) {
                driver.fail("none-selected-a-commit")
                return
            }
            // ListView gives its first row a current index even though the
            // page never activated a commit. Clear that visual selection too.
            const hadCurrent = graphPane.view.currentIndex >= 0
            graphPane.setCurrentRow(-1)
            AppBackend.report("perf_selection mode=none oid=none")
            if (hadCurrent)
                driver.waitFrame("none-frame")
            else
                driver.afterInteraction()
            return
        }
        let row = -1
        const oid = PerfProbe.oid !== "" ? PerfProbe.oid
                    : PerfProbe.selection === "head" ? branchesModel.headOid : ""
        if (oid !== "")
            row = graphModel.rowOf(oid)
        else if (PerfProbe.selection === "first") {
            for (let i = 0; i < graphModel.rowTotal; i++) {
                if (!GitFacts.wipOid(graphModel.oidAt(i))) {
                    row = i
                    break
                }
            }
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
        driver.stage = "details"
        driver.actionStart = PerfProbe.clockMs()
        page.releasePressedAway(null)
        item.leftClick(0)
        AppBackend.report("perf_selection mode=" + PerfProbe.selection + " oid=" + driver.wantedOid)
        driver.tick()
    }

    function afterDetails() {
        driver.sawDetails = true
        AppBackend.report("perf_details_frame elapsed_ms=" + (PerfProbe.clockMs() - driver.actionStart)
                          + " oid=" + detailsModel.shaHex + " boundary=handler-to-frame")
        if (!PerfProbe.withDiff) {
            driver.afterInteraction()
            return
        }
        let file = 0
        if (PerfProbe.filePath !== "") {
            file = -1
            for (let i = 0; i < detailsModel.fileTotal; i++) {
                if (detailsModel.filePathAt(i) === PerfProbe.filePath) {
                    file = i
                    break
                }
            }
        }
        if (file < 0 || detailsModel.fileTotal === 0) {
            driver.fail("diff-file-missing")
            return
        }
        driver.stage = "diff"
        driver.actionStart = PerfProbe.clockMs()
        AppBackend.report("perf_file path=" + detailsModel.filePathAt(file))
        page.openDiff("commit", detailsModel.filePathAt(file), detailsModel.fileOrigPathAt(file))
        driver.tick()
    }

    function afterInteraction() {
        if (AppBackend.autoScroll) {
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

    function finish() {
        driver.stage = "finished"
        AppBackend.report("perf_complete selection=" + PerfProbe.selection + " details=" + driver.sawDetails
                          + " diff=" + driver.sawDiff + " graph=" + driver.graphVisible
                          + " scrolled=" + AppBackend.autoScroll + " rows=" + graphModel.rowTotal)
        page.perfFinished()
    }

    function beginScroll() {
        graphPane.view.forceLayout()
        driver.reportViewport("start")
        if (!driver.graphVisible || scrollBench.to <= scrollBench.from) {
            driver.fail("scroll-hidden-or-no-overflow")
            return
        }
        driver.stage = "scrolling"
        driver.scrollStart = graphPane.view.contentY
        PerfProbe.beginScroll()
        scrollBench.start()
    }

    function reportViewport(at) {
        const view = graphPane.view
        AppBackend.report("perf_viewport clock_ms=" + PerfProbe.clockMs() + " at=" + at
                          + " y=" + view.contentY + " origin=" + view.originY
                          + " height=" + view.contentHeight + " viewport=" + view.height
                          + " row=" + view.indexAt(view.width / 2, view.contentY + view.height / 2))
    }

    function frame() {
        if (driver.stage === "scrolling") {
            driver.scrollValid = driver.scrollValid && driver.graphVisible
            PerfProbe.frame()
            return
        }
        if (!driver.stage.endsWith("-frame"))
            return
        if (driver.frames <= driver.frameBefore) {
            page.Window.window.requestUpdate()
            return
        }
        if (driver.stage === "graph-frame") {
            if (!driver.graphVisible) {
                driver.fail("first-graph-hidden")
                return
            }
            AppBackend.report("perf_graph_frame clock_ms=" + PerfProbe.clockMs() + " visible=true")
            driver.stage = "ready"
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
            AppBackend.report("perf_diff_frame elapsed_ms=" + (PerfProbe.clockMs() - driver.actionStart))
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
            AppBackend.report("perf_scroll_frame visible=true row=" + row)
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
        id: scrollBench
        target: driver.graphPane.view
        property: "contentY"
        from: driver.graphPane.view.originY
        to: from + Math.max(0, Math.min(3000 * Theme.graphRowHeight,
                                      driver.graphPane.view.contentHeight - driver.graphPane.view.height))
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

    Component.onCompleted: driver.tick()
    onReadyChanged: driver.tick()
}
