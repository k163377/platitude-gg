import QtQuick
import platitude
import platitude.ui

/// Owns the process-level end of a PG_AUTO_PERF run. The page supplies the repository half; without a repository, the
/// next rendered frame is the measurement's only observable completion point.
Item {
    id: driver

    property var window
    property var page
    property bool finished: false
    // The font walk below, in the run that asked for one: begun, and then over.
    property bool walking: false
    property bool walked: false
    property int frameBefore: 0
    readonly property bool expectsPage: Harness.autoOpen !== ""
    readonly property bool identityReady: AppBackend.identityState === "ready"
    // ScreenInfo names can be friendly labels, not OS device IDs. The runner
    // separately records the native window's monitor; never infer its Hz from fps.
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
        driver.frameBefore = window.frameCounter
        window.requestUpdate()
    }

    function finish() {
        if (driver.finished)
            return
        // Frames keep arriving while the walk is under way, and each one asks again: the answer is the same until
        // the walk has said it settled.
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
            if (driver.identityReady && driver.window.frameCounter > driver.frameBefore)
                driver.finish()
        }
    }

    Connections {
        target: driver.page
        enabled: Harness.autoPerf && driver.page !== null && !driver.finished
        function onPerfFinished() { driver.finish() }
    }

    /// The font database's population, paid at a moment the memory sampler can see (`PG_PERF_FONT_WALK=1`: the
    /// calibration run of `cargo xtask perf`, whose budget line is read net of what this weighs — xtask
    /// `perf::fonts`).
    ///
    /// The first glyph the UI family lacks makes Qt build a fallback list, and building one populates every family
    /// the database knows — on Windows a DirectWrite face over each file, kept for the life of the process: tens of
    /// MB once, whichever glyph asked, and nothing the product can decline (the fallback-family and emoji-family
    /// APIs only order that list). The corpus asks during the scroll, where the walk's bytes and the bench's arrive
    /// in the same ticks, so this asks before `perf_done` instead, idle either side, and says when: the parent reads
    /// its sampler at the first mark and the last. The walk is the product's own — the same question a gitmoji
    /// subject asks, the same fonts, the same bytes — only the moment is chosen. Offscreen there is nothing to weigh
    /// (that platform's FreeType database holds no fonts), so `verify-ui perf font-walk` checks the three lines alone.
    Item {
        id: fontWalk

        // Long enough for the sampler's 100ms ticks to have seen the process idle on either side of the walk, and
        // the whole of what the calibration run costs beyond opening the repository.
        readonly property int settleMs: 2000

        function begin() {
            settleBefore.start()
        }

        Text {
            id: probe
            font.family: Theme.uiFamily
            font.pixelSize: Theme.fontMd
        }

        Timer {
            id: settleBefore
            interval: fontWalk.settleMs
            onTriggered: {
                Harness.report("perf_font_walk_begin clock_ms=" + PerfProbe.clockMs())
                // U+1F352 CHERRIES: emoji presentation, so the run is an emoji run asking for a colour font — the
                // question no family the product names can answer. Reading the width is what forces the layout.
                probe.text = String.fromCodePoint(0x1F352)
                Harness.report("perf_font_walk_done clock_ms=" + PerfProbe.clockMs()
                                  + " width=" + probe.implicitWidth)
                settleAfter.start()
            }
        }

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

    // The memory sampler belongs beside the process-level owner: it keeps covering the full run, while `perf-done`
    // gives the last causal sample.
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
