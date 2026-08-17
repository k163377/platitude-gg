import QtQuick
import platitude
import platitude.ui

/// The repository half of PG_AUTO_PERF.  Readiness, a real frame, and the requested output all form one causal chain;
/// the twelve-second animation is deliberately the FPS sample window, never a readiness delay.
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
    property bool frameRequested: false
    property bool frameSeen: false
    property bool scrollDone: !AppBackend.autoScroll
    property bool finished: false
    property real startedAt: 0
    property int frameBefore: 0
    property int startedFrames: 0

    readonly property bool needsRows: AppBackend.autoSelect || AppBackend.autoScroll
    readonly property bool ready: repoTab.state === "open"
                                  && !graphModel.loading
                                  && (!driver.needsRows || graphModel.rowTotal > 0)
                                  && graphModel.finishCount > 0
                                  && branchesModel.refsLoaded
                                  && worktreeModel.loaded
    readonly property bool selected: !AppBackend.autoSelect
                                     || (detailsModel.shaHex !== ""
                                         && (detailsModel.fileTotal === 0
                                             || (diffModel.title !== ""
                                                 && !diffModel.loading)))

    function requestStart() {
        if (!AppBackend.autoPerf || driver.finished || driver.frameRequested || !driver.ready || !driver.selected)
            return
        driver.frameRequested = true
        driver.frameBefore = page.Window.window.frameCounter
        page.Window.window.requestUpdate()
    }

    function finishWhenReady() {
        if (driver.finished || !driver.ready || !driver.frameSeen || !driver.selected || !driver.scrollDone)
            return
        driver.finished = true
        page.perfFinished()
    }

    Connections {
        target: driver.graphModel
        enabled: AppBackend.autoPerf
        function onStatsChanged() { driver.requestStart() }
    }
    Connections {
        target: driver.branchesModel
        enabled: AppBackend.autoPerf
        function onChanged() { driver.requestStart() }
    }
    Connections {
        target: driver.worktreeModel
        enabled: AppBackend.autoPerf
        function onChanged() { driver.requestStart() }
    }
    Connections {
        target: driver.repoTab
        enabled: AppBackend.autoPerf
        function onChanged() { driver.requestStart() }
    }
    Connections {
        target: driver.page.Window.window
        enabled: AppBackend.autoPerf && driver.frameRequested && !driver.frameSeen
        function onFrameSwapped() {
            if (driver.page.Window.window.frameCounter <= driver.frameBefore)
                return
            if (!driver.ready || !driver.selected) {
                driver.frameRequested = false
                driver.requestStart()
                return
            }
            driver.frameSeen = true
            if (AppBackend.autoScroll) {
                driver.startedAt = Date.now()
                driver.startedFrames = driver.page.Window.window.frameCounter
                scrollBench.start()
            }
            driver.finishWhenReady()
        }
    }
    Connections {
        target: driver.detailsModel
        enabled: AppBackend.autoPerf && AppBackend.autoSelect
        function onChanged() { driver.requestStart() }
    }
    Connections {
        target: driver.diffModel
        enabled: AppBackend.autoPerf && AppBackend.autoSelect
        function onChanged() { driver.requestStart() }
    }

    NumberAnimation {
        id: scrollBench
        target: driver.graphPane.view
        property: "contentY"
        from: 0
        to: Math.max(0, Math.min(3000, driver.graphModel.rowTotal - 40)) * Theme.graphRowHeight
        duration: 12000
        onStopped: {
            const seconds = (Date.now() - driver.startedAt) / 1000
            const frames = driver.page.Window.window.frameCounter - driver.startedFrames
            AppBackend.report("scroll_bench fps=" + (frames / seconds).toFixed(1)
                              + " rows=" + driver.graphModel.rowTotal)
            driver.scrollDone = true
            driver.finishWhenReady()
        }
    }

    Component.onCompleted: driver.requestStart()
}
