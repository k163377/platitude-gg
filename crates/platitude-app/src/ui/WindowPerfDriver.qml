import QtQuick
import platitude

/// Owns the process-level end of a PG_AUTO_PERF run. The page supplies the repository half; without a repository, the
/// next rendered frame is the measurement's only observable completion point.
Item {
    id: driver

    property var window
    property var page
    property bool finished: false
    property int frameBefore: 0
    readonly property bool expectsPage: AppBackend.autoOpen !== ""

    function begin() {
        if (!AppBackend.autoPerf || driver.finished)
            return
        if (driver.expectsPage || driver.page !== null)
            return
        driver.frameBefore = window.frameCounter
        window.requestUpdate()
    }

    function finish() {
        if (driver.finished)
            return
        driver.finished = true
        AppBackend.noteMemory("perf-done")
        AppBackend.report("perf_done open=" + (driver.page !== null))
    }

    Connections {
        target: driver.window
        enabled: AppBackend.autoPerf && !driver.expectsPage && driver.page === null && !driver.finished
        function onFrameSwapped() {
            if (driver.window.frameCounter > driver.frameBefore)
                driver.finish()
        }
    }

    Connections {
        target: driver.page
        enabled: AppBackend.autoPerf && driver.page !== null && !driver.finished
        function onPerfFinished() { driver.finish() }
    }

    // The memory sampler belongs beside the process-level owner: it keeps covering the full run, while `perf-done`
    // gives the last causal sample.
    Timer {
        interval: 500
        repeat: true
        running: AppBackend.memReport && !driver.finished
        triggeredOnStart: true
        onTriggered: AppBackend.noteMemory(driver.page === null ? "idle" : "open")
    }

    Component.onCompleted: driver.begin()
    onPageChanged: driver.begin()
}
