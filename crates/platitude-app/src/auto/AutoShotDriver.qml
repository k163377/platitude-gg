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
    /// Whose frame the popups in the picture stand over: `yield=` on the overlay's line (`WindowChrome.frameYielded`).
    property WindowChrome chrome

    property bool pageActClaimed: false
    property bool shotPending: false
    property bool shotTaken: false
    /// The overlay mirror stood when the picture was called for, so its picture is a part.
    property bool overlayWanted: false
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

    /// A picture is on disk — `app` (the window, whose scene is the settled one: the completion edge often builds
    /// that scene, `grabApp`), `overlay` or `scene`. Whatever reads the tree for what this run shows hears it here
    /// (`WindowHarness` → `WindowCensus`, `Awaited`), in the picture's own turn: a frame later is the stretch a read
    /// could land in.
    signal pictured(string artifact)

    /// Where the run's own write had got to, for the watchdog's line — written by the page that claimed the run
    /// (`Main.noteAutoActWrite` ← `AutoActDriver`), so a run that never reached a page keeps the dash.
    property string writeState: "-"
    /// When the last frame swapped (-1 for none), for the watchdog's line: with the frames counted (`Awaited.frames`)
    /// it separates a term stuck over a scene still drawing from one over a scene that stopped.
    property real lastFrameAt: -1
    /// Whether a picture waits for the page (`WindowHarness.holdsPicture`), asked before the overlay's and the
    /// scene's grabs: an ambient read owed again between two pictures is waited out as before the first. Null
    /// holds nothing.
    property var pictureHeld: null
    property bool overlayHeldBack: false
    property bool sceneHeldBack: false
    function held() {
        return driver.pictureHeld !== null && driver.pictureHeld() === true
    }
    /// Said for each picture the run asked for (`takeShot`), saved or not: the parent holds the set asked for
    /// against the set saved (`verify::contract`), and a save that failed is no picture, whatever came after it.
    function pictureSaved(artifact, saved) {
        Harness.report("picture run=" + Awaited.runId + " artifact=" + artifact + " saved=" + saved)
    }

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
            driver.lastFrameAt = PerfProbe.clockMs()
            driver.scheduleShot()
            if (driver.overlayHeldBack) {
                driver.overlayHeldBack = false
                driver.grabOverlay()
            }
            if (driver.sceneHeldBack) {
                driver.sceneHeldBack = false
                driver.grabScene()
            }
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
            console.warn("shot state grabbing=" + driver.appGrabbing + " saved=" + driver.appSaved
                         + " parts=" + driver.shotParts + " ink=" + Ink.owed)
            // What the write barrier was holding, if the run wanted a write
            // (rules-refs/app-ui.md「天井まで行った run は…」).
            console.warn("write state " + driver.writeState)
            // What the verb was still waiting on and for how long (`Awaited`); `unsaid` for a verb that says none.
            const now = PerfProbe.clockMs()
            console.warn("awaited " + (Awaited.said === "" ? "unsaid"
                                       : Awaited.said + " for " + Math.round(now - Awaited.since) + "ms"))
            console.warn("frames swapped=" + Awaited.frames + " last="
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
    /// means nothing was open; `yield=` is whether those popups had the window's frame handed to the scene, so a press
    /// on the band's grab runs or the edges would reach them — true for a menu or list a press outside closes.
    function grabOverlay() {
        if (!driver.overlayAsked || driver.overlayGrabbed)
            return
        if (driver.held()) {
            driver.overlayHeldBack = true
            driver.window.update()
            return
        }
        driver.overlayGrabbed = true
        const mirror = overlayMirror.item
        const popups = mirror.sourceItem ? mirror.sourceItem.children.length : 0
        const yielded = driver.chrome !== null && driver.chrome.frameYielded
        const overlayOk = mirror.grabToImage(function (res) {
            const saved = res.saveToFile(Harness.shotDir + "/overlay.png")
            console.warn("overlay saved=" + saved + " popups=" + popups + " yield=" + yielded)
            driver.pictureSaved("overlay", saved)
            driver.pictured("overlay")
            driver.partDone()
        })
        if (!overlayOk) {
            console.warn("overlay grabToImage returned false")
            driver.pictureSaved("overlay", false)
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
        driver.overlayWanted = overlayMirror.item !== null
        driver.shotParts = 1 + (driver.overlayWanted ? 1 : 0) + (driver.sceneWanted ? 1 : 0)
        // The pictures this run owes, fixed here: each is said saved or not (`pictureSaved`), and the parent passes
        // the run only on all of them.
        const wanted = ["app"].concat(driver.overlayWanted ? ["overlay"] : [], driver.sceneWanted ? ["scene"] : [])
        Harness.report("pictures run=" + Awaited.runId + " wanted=" + wanted.join(","))
        driver.grabApp()
    }
    /// The popups' texture, refreshed once app.png is saved, as the composite is: a popup's marks are owed like the
    /// window's (`Ink`), and a texture refreshed before they paint holds their rooms with nothing in them.
    function refreshOverlay() {
        if (driver.overlayWanted && !driver.overlayAsked) {
            driver.overlayAsked = true
            overlayMirror.item.scheduleUpdate()
        }
    }

    /// The composite, asked for only once app.png is saved — refreshed earlier, it could miss the ink (`grabApp`).
    Connections {
        target: driver.sceneWanted && driver.sceneMirror ? driver.sceneMirror.item : null
        ignoreUnknownSignals: true
        function onReady() {
            driver.grabScene()
        }
    }
    property bool sceneGrabbed: false
    function grabScene() {
        if (driver.sceneGrabbed)
            return
        if (driver.held()) {
            driver.sceneHeldBack = true
            driver.window.update()
            return
        }
        driver.sceneGrabbed = true
        const scene = driver.sceneMirror.item
        const ok = scene.grabToImage(function (res) {
            const saved = res.saveToFile(Harness.shotDir + "/scene.png")
            console.warn("scene saved=" + saved)
            driver.pictureSaved("scene", saved)
            driver.pictured("scene")
            driver.partDone()
        })
        if (!ok) {
            console.warn("scene grabToImage returned false")
            driver.pictureSaved("scene", false)
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
            driver.pictureSaved("app", saved)
            driver.pictured("app")
            driver.refreshOverlay()
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
            driver.pictureSaved("app", false)
            driver.pictured("app")
            driver.refreshOverlay()
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
