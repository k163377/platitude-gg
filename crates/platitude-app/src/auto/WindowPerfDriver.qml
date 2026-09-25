import QtQuick
import platitude
import platitude.ui

/// Owns the process-level end of a PGG_AUTO_PERF run. The page supplies the repository half; without a repository, the
/// next rendered frame is the measurement's only observable completion point.
Item {
    id: driver

    property var window
    property var page
    property bool finished: false
    // The font walk below: begun, then over.
    property bool walking: false
    property bool walked: false
    property int frameBefore: 0
    // Waiting for the frame a run with no page ends on.
    property bool framing: false
    readonly property bool expectsPage: Harness.autoOpen !== ""
    readonly property bool identityReady: AppBackend.identityState === "ready"
    // `screen.name` can be a friendly label; the runner takes Hz from the native window's monitor instead.
    readonly property string displayInfo: !Harness.autoPerf || !window || !window.screen ? "{}" : JSON.stringify({
        screen: window.screen.name, model: window.screen.model, manufacturer: window.screen.manufacturer,
        screenX: window.screen.virtualX, screenY: window.screen.virtualY,
        screenWidth: window.screen.width, screenHeight: window.screen.height,
        dpr: window.screen.devicePixelRatio, logicalDpi: window.screen.logicalPixelDensity * 25.4,
        x: window.x, y: window.y, width: window.width, height: window.height,
        visibility: window.visibility
    })

    function reportDisplay(phase) {
        if (Harness.autoPerf)
            Harness.report("perf_display clock_ms=" + PerfProbe.clockMs() + " phase=" + phase
                              + " data=" + driver.displayInfo)
    }

    function begin() {
        if (!Harness.autoPerf || driver.finished)
            return
        if (AppBackend.identityState === "missing" || AppBackend.identityState === "error") {
            driver.finished = true
            Harness.report("perf_failed reason=identity-gate")
            if (PerfProbe.verifying)
                Qt.quit()
            return
        }
        if (!driver.identityReady)
            return
        if (driver.expectsPage || driver.page !== null)
            return
        // Named once, on entry.
        if (!driver.framing)
            Harness.report("perf_stage window-frame")
        driver.frameBefore = window.frameCounter
        driver.framing = true
        // `update()`: a bare `requestUpdate()` swaps nothing over a still offscreen scene (`PagePerfDriver.waitFrame`).
        window.update()
    }

    function finish() {
        if (driver.finished)
            return
        // Each frame during the walk asks again; the answer stays the same until the walk has settled.
        if (Harness.perfFontWalk && !driver.walked) {
            if (!driver.walking) {
                driver.walking = true
                fontWalk.begin()
            }
            return
        }
        driver.finished = true
        driver.reportDisplay("complete")
        Harness.noteMemory("perf-done")
        Harness.report("perf_done open=" + (driver.page !== null))
        if (PerfProbe.verifying)
            window.finishAutoAct()
    }

    Connections {
        target: driver.window
        enabled: Harness.autoPerf && !driver.expectsPage && driver.page === null && !driver.finished
        function onFrameSwapped() {
            if (driver.identityReady && driver.window.frameCounter > driver.frameBefore) {
                driver.framing = false
                driver.finish()
            }
        }
    }

    // Re-asked on the beat until the first frame (a request taken while not visible is spent without a swap), and not
    // after: the font walk is read over an idle window.
    SampleTimer {
        running: driver.framing && !driver.finished
        onTriggered: driver.window.update()
    }

    Connections {
        target: driver.page
        enabled: Harness.autoPerf && driver.page !== null && !driver.finished
        function onPerfFinished() { driver.finish() }
    }

    /// The font database's population, paid at a moment the memory sampler can see (`PGG_PERF_FONT_WALK=1`, the
    /// calibration run of `cargo xtask perf`; rules-refs/app-ui.md「perf の判定行は net で読む」). The walk is the
    /// product's own — the glyph question a gitmoji subject asks — only the moment is chosen: before `perf_done`,
    /// idle either side, so the parent's sampler weighs it between the first mark and the last.
    Item {
        id: fontWalk

        // Long enough for the sampler's 100ms ticks to see the process idle either side of the walk.
        readonly property int settleMs: 2000

        function begin() {
            settleBefore.start()
        }

        Text {
            id: probe
            font.family: Theme.uiFamily
            font.pixelSize: Theme.fontMd
        }

        // waits(measured): the idle either side is the calibration's input — the parent reads its sampler at both marks
        Timer {
            id: settleBefore
            interval: fontWalk.settleMs
            onTriggered: {
                Harness.report("perf_font_walk_begin clock_ms=" + PerfProbe.clockMs())
                // U+1F352 CHERRIES: emoji presentation asks for a colour font, which no family the product names has.
                // Reading the width is what forces the layout.
                probe.text = String.fromCodePoint(0x1F352)
                Harness.report("perf_font_walk_done clock_ms=" + PerfProbe.clockMs()
                                  + " width=" + probe.implicitWidth)
                settleAfter.start()
            }
        }

        // waits(measured): the second half of that same window, and `walked` is what ends the run
        Timer {
            id: settleAfter
            interval: fontWalk.settleMs
            onTriggered: {
                Harness.report("perf_font_walk_settled clock_ms=" + PerfProbe.clockMs())
                driver.walked = true
                driver.finish()
            }
        }
    }

    // Beside the process-level owner so it covers the whole run; `perf-done` gives the last causal sample.
    // waits(paced): the cadence a sample is taken on; nothing about the run ends on one of these
    Timer {
        interval: 500
        repeat: true
        running: Harness.memReport
        triggeredOnStart: true
        onTriggered: Harness.noteMemory(driver.page === null ? "idle" : "open")
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
