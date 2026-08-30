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
    readonly property bool identityReady: AppBackend.identityState === "ready"
    // ScreenInfo names can be friendly labels, not OS device IDs. The runner
    // separately records the native window's monitor; never infer its Hz from fps.
    readonly property string displayInfo: !AppBackend.autoPerf || !window || !window.screen ? "{}" : JSON.stringify({
        screen: window.screen.name, model: window.screen.model, manufacturer: window.screen.manufacturer,
        screenX: window.screen.virtualX, screenY: window.screen.virtualY,
        screenWidth: window.screen.width, screenHeight: window.screen.height,
        dpr: window.screen.devicePixelRatio, logicalDpi: window.screen.logicalPixelDensity * 25.4,
        x: window.x, y: window.y, width: window.width, height: window.height,
        visibility: window.visibility
    })

    function reportDisplay(phase) {
        if (AppBackend.autoPerf)
            AppBackend.report("perf_display clock_ms=" + PerfProbe.clockMs() + " phase=" + phase
                              + " data=" + driver.displayInfo)
    }

    function begin() {
        if (!AppBackend.autoPerf || driver.finished)
            return
        if (AppBackend.identityState === "missing" || AppBackend.identityState === "error") {
            driver.finished = true
            AppBackend.report("perf_failed reason=identity-gate")
            if (PerfProbe.verifying)
                Qt.quit()
            return
        }
        if (!driver.identityReady)
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
        driver.reportDisplay("complete")
        AppBackend.noteMemory("perf-done")
        AppBackend.report("perf_done open=" + (driver.page !== null))
        if (PerfProbe.verifying)
            window.finishAutoAct()
    }

    Connections {
        target: driver.window
        enabled: AppBackend.autoPerf && !driver.expectsPage && driver.page === null && !driver.finished
        function onFrameSwapped() {
            if (driver.identityReady && driver.window.frameCounter > driver.frameBefore)
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
        running: AppBackend.memReport
        triggeredOnStart: true
        onTriggered: AppBackend.noteMemory(driver.page === null ? "idle" : "open")
    }

    Component.onCompleted: {
        driver.reportDisplay("begin")
        driver.begin()
    }
    onPageChanged: driver.begin()
    onDisplayInfoChanged: driver.reportDisplay("changed")
    Connections {
        target: AppBackend
        function onIdentityChanged() { driver.begin() }
    }
}
