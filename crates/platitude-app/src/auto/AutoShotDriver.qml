pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import platitude
import platitude.ui

/// Owns one automated run from completion through a rendered screenshot; idle unless PGG_AUTO_* was supplied.
Item {
    id: driver

    property var window
    property Loader overlayMirror
    /// The overlay laid over the content (`WindowShotMirrors`), asked for only when the overlay held something.
    property Loader sceneMirror
    property Item mainUi
    property Item gate

    property bool pageActClaimed: false
    property bool shotPending: false
    property bool shotTaken: false
    // A non-live mirror also refreshes once when built, with the same signal; only the asked-for one is the shot.
    property bool overlayAsked: false
    property bool overlayGrabbed: false
    property int shotParts: 0
    property bool appSaved: false
    /// The overlay held something when the picture was called for (its children, as `popups=` counts them).
    property bool sceneWanted: false
    property bool sceneAsked: false
    /// A grab in flight. One at a time: a second answer would count the part twice and quit before the overlay.
    property bool appGrabbing: false
    /// Marks the picture waited for (0 if the scene was already drawn); reported, so the wait cannot go unwired.
    property int inkWaited: 0
    // Every PGG_AUTO_ACT run has one explicit completion edge; a verb without one meets the watchdog.
    readonly property bool causal: Harness.autoAct !== ""

    /// The window's picture is on disk, so its scene is the settled one — the completion edge often builds that scene
    /// (`grabApp`). Whatever reads the tree for what this run showed hears it here (`WindowCensus`), still ahead of the
    /// quit.
    signal appPictured()

    /// The census walk as a part like the other two: it waits for the window to stop arriving, often later than the
    /// verb's own edge (`WindowCensus`), and without the part the run would quit out from under it.
    readonly property bool waitsForCensus: driver.causal
    function censusDone() {
        driver.partDone()
    }
    /// What the census is doing, for the watchdog's line alone. Bound by whoever holds one (`WindowHarness`).
    property string censusState: "-"
    /// Where the run's own write had got to, for the watchdog's line — written by the page that claimed the run
    /// (`Main.noteAutoActWrite` ← `AutoActDriver`), so a run that never reached a page keeps the dash.
    property string writeState: "-"
    /// Frames swapped and when the last one was (-1 for none), for the watchdog's line: it separates a term stuck over
    /// a scene still drawing from one over a scene that stopped (`Awaited` names the term).
    property int framesSwapped: 0
    property real lastFrameAt: -1

    function claimPageAct() {
        if (driver.pageActClaimed)
            return false
        driver.pageActClaimed = true
        return true
    }

    function begin() {
        if (Harness.autoWatchdogMs > 0)
            watchdog.start()
    }

    function finish() {
        if (!driver.causal || driver.shotTaken || driver.shotPending)
            return
        if (Harness.shotDir === "") {
            Qt.quit()
            return
        }
        Harness.report("auto_act complete=" + Harness.autoAct)
        driver.shotPending = true
        window.requestUpdate()
        // A quiet scene may not swap again even after an update request, so the next turn schedules the shot;
        // grabToImage owns the render callback.
        Qt.callLater(driver.scheduleShot)
    }

    function scheduleShot() {
        if (!driver.shotPending)
            return
        driver.shotPending = false
        Qt.callLater(driver.takeShot)
    }

    Connections {
        target: driver.window
        function onFrameSwapped() {
            driver.framesSwapped++
            driver.lastFrameAt = PerfProbe.clockMs()
            driver.scheduleShot()
        }
    }

    // The overlay's render boundary, and why `WindowShotMirrors` builds the mirror `live: false`: a live one leaves
    // nothing to wait on, and a popup opened in the completing turn is often not in its texture yet.
    Connections {
        target: driver.overlayMirror ? driver.overlayMirror.item : null
        ignoreUnknownSignals: true
        function onScheduledUpdateCompleted() {
            driver.grabOverlay()
        }
    }

    // The only clock here. A run that reaches it has failed; it reports what the machine was holding.
    // waits(ceiling): a run that stopped answering ends here, and every verb's own completion is causal
    Timer {
        id: watchdog
        interval: Math.max(Harness.autoWatchdogMs, 1)
        onTriggered: {
            console.warn("auto-act watchdog expired verb=" + Harness.autoAct)
            // `census=waiting`: the window still had a read on its way (`WindowCensus.pageSettled`).
            console.warn("shot state grabbing=" + driver.appGrabbing + " saved=" + driver.appSaved
                         + " parts=" + driver.shotParts + " ink=" + Ink.owed
                         + " census=" + driver.censusState)
            // What the write barrier was holding, if the run wanted a write
            // (rules-refs/app-ui.md「天井まで行った run は…」).
            console.warn("write state " + driver.writeState)
            // What the verb was still waiting on and for how long (`Awaited`); `unsaid` for a verb that says none.
            const now = PerfProbe.clockMs()
            console.warn("awaited " + (Awaited.said === "" ? "unsaid"
                                       : Awaited.said + " for " + Math.round(now - Awaited.since) + "ms"))
            console.warn("frames swapped=" + driver.framesSwapped + " last="
                         + (driver.lastFrameAt < 0 ? "never" : Math.round(now - driver.lastFrameAt) + "ms ago"))
            // Which of the page's reads never came (`PageSettled`) — from the verb's side it looks like its own term.
            const page = driver.window ? driver.window.curPage : null
            const owed = PageSettled.missing(page)
            console.warn("page " + (page === null ? "none"
                                    : owed.length === 0 ? "settled" : "missing=" + owed.join(",")))
            Qt.quit()
        }
    }
    function partDone() {
        driver.shotParts--
        if (driver.shotParts === 0)
            Qt.quit()
    }

    /// The popups' picture from the refreshed texture. `popups=` counts what the overlay held, so a blank overlay.png
    /// means nothing was open.
    function grabOverlay() {
        if (!driver.overlayAsked || driver.overlayGrabbed)
            return
        driver.overlayGrabbed = true
        const mirror = overlayMirror.item
        const popups = mirror.sourceItem ? mirror.sourceItem.children.length : 0
        const overlayOk = mirror.grabToImage(function (res) {
            const saved = res.saveToFile(Harness.shotDir + "/overlay.png")
            console.warn("overlay saved=" + saved + " popups=" + popups)
            driver.partDone()
        })
        if (!overlayOk) {
            console.warn("overlay grabToImage returned false")
            driver.partDone()
        }
    }

    function takeShot() {
        if (driver.shotTaken)
            return
        driver.shotTaken = true
        const holding = overlayMirror.item && overlayMirror.item.sourceItem
                        ? overlayMirror.item.sourceItem.children.length : 0
        driver.sceneWanted = holding > 0 && driver.sceneMirror !== null && driver.sceneMirror.item !== null
        driver.shotParts = 1 + (overlayMirror.item ? 1 : 0) + (driver.sceneWanted ? 1 : 0)
                           + (driver.waitsForCensus ? 1 : 0)
        if (overlayMirror.item) {
            driver.overlayAsked = true
            overlayMirror.item.scheduleUpdate()
        }
        driver.grabApp()
    }

    /// The composite, asked for only once app.png is saved — refreshed earlier, it could miss the ink (`grabApp`).
    Connections {
        target: driver.sceneWanted && driver.sceneMirror ? driver.sceneMirror.item : null
        ignoreUnknownSignals: true
        function onReady() {
            driver.grabScene()
        }
    }
    function grabScene() {
        const scene = driver.sceneMirror.item
        const ok = scene.grabToImage(function (res) {
            const saved = res.saveToFile(Harness.shotDir + "/scene.png")
            console.warn("scene saved=" + saved)
            driver.partDone()
        })
        if (!ok) {
            console.warn("scene grabToImage returned false")
            driver.partDone()
        }
    }

    /// The window, once the frame this asks for has rendered. A grab taken while `Ink` still owes marks is thrown away
    /// and asked for again at `Ink.owed === 0` — the completion edge often builds the scene, and a `Canvas` has no ink
    /// in its first frame (rules-refs/app-ui.md「`Canvas` は生まれたフレームにインクを持たない」).
    function grabApp() {
        if (driver.appSaved || driver.appGrabbing)
            return
        driver.appGrabbing = true
        const path = Harness.shotDir + "/app.png"
        const shown = gate.visible ? gate : mainUi
        const ok = shown.grabToImage(function (res) {
            driver.appGrabbing = false
            if (Ink.owed > 0) {
                driver.inkWaited = Math.max(driver.inkWaited, Ink.owed)
                return
            }
            driver.appSaved = true
            const saved = res.saveToFile(path)
            console.warn("screenshot saved=" + saved + " ink=" + driver.inkWaited + " path=" + path)
            driver.appPictured()
            if (driver.sceneWanted && !driver.sceneAsked) {
                driver.sceneAsked = true
                driver.sceneMirror.item.refresh()
            }
            driver.partDone()
        })
        if (!ok) {
            console.warn("grabToImage returned false")
            driver.appGrabbing = false
            driver.appSaved = true
            driver.appPictured()
            driver.partDone()
        }
    }

    Connections {
        target: Ink
        function onOwedChanged() {
            if (driver.shotTaken && Ink.owed === 0)
                driver.grabApp()
        }
    }
}
