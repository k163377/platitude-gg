pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import platitude
import platitude.ui

/// Owns one automated run from completion through a rendered screenshot. The ordinary application constructs it too,
/// but every timer stays idle unless the PG_AUTO_* harness variables were supplied.
Item {
    id: driver

    property var window
    property Loader overlayMirror
    /// The two textures laid over each other, for the picture that holds the hover and the thing it hangs off at once
    /// (`Main.qml`). Only asked for when the overlay was holding something.
    property Loader sceneMirror
    property Item mainUi
    property Item gate

    property bool pageActClaimed: false
    property bool shotPending: false
    property bool shotTaken: false
    // A mirror that is not live still renders itself once when it is built, and announces that one the same way it
    // announces an asked-for refresh. Only the refresh this driver asked for is the shot.
    property bool overlayAsked: false
    property bool overlayGrabbed: false
    property int shotParts: 0
    property bool appSaved: false
    /// The overlay was holding something when the picture was called for, so the two are worth laying over each other.
    /// Counted off the overlay's own children at that moment, the same number `popups=` reports.
    property bool sceneWanted: false
    property bool sceneAsked: false
    /// A grab already asked for and not yet answered. **One at a time**: two in flight both answer, and the second
    /// saving the same file again would take the run's part count past zero and quit it out from under the overlay.
    property bool appGrabbing: false
    /// How many marks the picture had to wait for, 0 where the scene was already drawn when it was called for. Said in
    /// the report line, so a run cannot go green with this wait unwired.
    property int inkWaited: 0
    // Every PG_AUTO_ACT run has one explicit completion edge. A verb that still relies on the old shot clock is a
    // harness bug: the watchdog must expose it instead of taking a plausible picture of an intermediate state.
    readonly property bool causal: Harness.autoAct !== ""

    /// The window's picture is on disk, so the scene it came out of is the settled one: **the completion edge is often
    /// what builds that scene** (see `grabApp`), and a list whose rows stand up in the frame the grab is fulfilled in
    /// has no delegates at all before it. Anything reading the tree for what this run showed hears about it from here
    /// (`WindowCensus`), and this is still ahead of the quit, which waits on the parts.
    signal appPictured()

    /// Whether the ending waits for the census to have walked. **A part like the other two**, because the walk is not
    /// taken at the picture: it waits for the window to have stopped arriving, which is later than the verb's own edge
    /// as often as not (`WindowCensus`). Without the part the run quits out from under the walk on exactly the runs
    /// whose census would have been worth having.
    readonly property bool waitsForCensus: driver.causal
    function censusDone() {
        driver.partDone()
    }
    /// What the census is doing, for the watchdog's line alone. Bound by whoever holds one (`WindowHarness`).
    property string censusState: "-"

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
        // A quiet scene may not emit another frameSwapped even after an update request. The next event-loop turn is
        // still causal to the completed driver state, and grabToImage owns the render callback.
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
            driver.scheduleShot()
        }
    }

    // The overlay's own render boundary, and the reason `Main.qml` builds the mirror with `live: false`. A live mirror
    // leaves the shot nothing to wait on -- the grab reads whatever frame happened to have reached the texture, which
    // for a popup opened in the turn that completed the verb is often none at all (observed: two of four concurrent
    // `commit-menu` runs photographed a blank overlay and passed, while the same verb run one at a time never did).
    // Asked for one refresh instead, this is the edge that says the refresh landed: from here the texture holds what
    // the overlay held when the shot was called for.
    Connections {
        target: driver.overlayMirror ? driver.overlayMirror.item : null
        ignoreUnknownSignals: true
        function onScheduledUpdateCompleted() {
            driver.grabOverlay()
        }
    }

    Timer {
        id: watchdog
        interval: Math.max(Harness.autoWatchdogMs, 1)
        onTriggered: {
            console.warn("auto-act watchdog expired verb=" + Harness.autoAct)
            // `census=waiting` is the one part that can be out long after the picture: the window had a read still on
            // its way when the run ended (`WindowCensus.pageSettled`), which is what to go and look at.
            console.warn("shot state grabbing=" + driver.appGrabbing + " saved=" + driver.appSaved
                         + " parts=" + driver.shotParts + " ink=" + Ink.owed
                         + " census=" + driver.censusState)
            Qt.quit()
        }
    }
    function partDone() {
        driver.shotParts--
        if (driver.shotParts === 0)
            Qt.quit()
    }

    /// Refreshed texture in hand, the picture of the popups. `popups=` is the overlay's own count of what it was
    /// holding: a blank overlay.png now means nothing was open, never a shot that outran the frame.
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
        // Asked for here, taken in `grabOverlay` when the mirror answers.
        if (overlayMirror.item) {
            driver.overlayAsked = true
            overlayMirror.item.scheduleUpdate()
        }
        driver.grabApp()
    }

    /// The picture that holds both. Asked for only once the app's own is saved, so the two textures are taken from the
    /// scene that picture came out of — a mirror refreshed before the ink landed would draw the rows without their
    /// marks (`grabApp`).
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

    /// The window, once the frame this asks for has been rendered.
    ///
    /// **The scene is still being built in that frame**, and the completion edge is often what builds it: a verb that
    /// finishes when the status arrives finishes in the very turn the working tree's buckets stand up, so their rows
    /// are laid out and their names drawn in the frame the grab is fulfilled in — and their marks are not, because a
    /// `Canvas` cannot draw before the turn after the one that made it (`Ink`). The picture that came out was a list of
    /// names with an empty seat at the head of every row.
    ///
    /// So the frame is read for what it is worth and thrown away if the scene still owed ink: `Ink` says when the last
    /// mark has been drawn, and the grab is asked for again from there. A scene that was already drawn when the picture
    /// was called for — which is every verb that waits for something of its own after the edge — owes nothing and is
    /// saved from the first grab, exactly as before.
    function grabApp() {
        if (driver.appSaved || driver.appGrabbing)
            return
        driver.appGrabbing = true
        const path = Harness.shotDir + "/app.png"
        const shown = gate.visible ? gate : mainUi
        const ok = shown.grabToImage(function (res) {
            driver.appGrabbing = false
            if (Ink.owed > 0) {
                // Asked for again by `onOwedChanged` below, once every one of them has been drawn.
                driver.inkWaited = Math.max(driver.inkWaited, Ink.owed)
                return
            }
            driver.appSaved = true
            const saved = res.saveToFile(path)
            console.warn("screenshot saved=" + saved + " ink=" + driver.inkWaited + " path=" + path)
            driver.appPictured()
            // The scene this picture came out of is what the composite mirrors, so it is asked for from here.
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
