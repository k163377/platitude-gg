pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import platitude

/// Owns one automated run from completion through a rendered screenshot. The ordinary application constructs it too,
/// but every timer stays idle unless the PG_AUTO_* harness variables were supplied.
Item {
    id: driver

    property var window
    property Loader overlayMirror
    property ColumnLayout mainUi
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
    /// A grab already asked for and not yet answered. **One at a time**: two in flight both answer, and the second
    /// saving the same file again would take the run's part count past zero and quit it out from under the overlay.
    property bool appGrabbing: false
    /// How many marks the picture had to wait for, 0 where the scene was already drawn when it was called for. Said in
    /// the report line, so a run cannot go green with this wait unwired.
    property int inkWaited: 0
    // Every PG_AUTO_ACT run has one explicit completion edge. A verb that still relies on the old shot clock is a
    // harness bug: the watchdog must expose it instead of taking a plausible picture of an intermediate state.
    readonly property bool causal: AppBackend.autoAct !== ""

    function claimPageAct() {
        if (driver.pageActClaimed)
            return false
        driver.pageActClaimed = true
        return true
    }

    function begin() {
        if (AppBackend.autoWatchdogMs > 0)
            watchdog.start()
    }

    function finish() {
        if (!driver.causal || driver.shotTaken || driver.shotPending)
            return
        if (AppBackend.shotDir === "") {
            Qt.quit()
            return
        }
        AppBackend.report("auto_act complete=" + AppBackend.autoAct)
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
    // for a popup opened in the turn that completed the verb is often none at all (2026-08-16: two of four concurrent
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
        interval: Math.max(AppBackend.autoWatchdogMs, 1)
        onTriggered: {
            console.warn("auto-act watchdog expired verb=" + AppBackend.autoAct)
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
            const saved = res.saveToFile(AppBackend.shotDir + "/overlay.png")
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
        driver.shotParts = overlayMirror.item ? 2 : 1
        // Asked for here, taken in `grabOverlay` when the mirror answers.
        if (overlayMirror.item) {
            driver.overlayAsked = true
            overlayMirror.item.scheduleUpdate()
        }
        driver.grabApp()
    }

    /// The window, once the frame this asks for has been rendered.
    ///
    /// **The scene is still being built in that frame**, and the completion edge is often what builds it: a verb that
    /// finishes when the status arrives finishes in the very turn the working tree's buckets stand up, so their rows
    /// are laid out and their names drawn in the frame the grab is fulfilled in — and their marks are not, because a
    /// `Canvas` cannot draw before the turn after the one that made it (`Ink`). The picture that came out was a list of
    /// names with an empty seat at the head of every row (2026-08-26 ユーザー報告).
    ///
    /// So the frame is read for what it is worth and thrown away if the scene still owed ink: `Ink` says when the last
    /// mark has been drawn, and the grab is asked for again from there. A scene that was already drawn when the picture
    /// was called for — which is every verb that waits for something of its own after the edge — owes nothing and is
    /// saved from the first grab, exactly as before.
    function grabApp() {
        if (driver.appSaved || driver.appGrabbing)
            return
        driver.appGrabbing = true
        const path = AppBackend.shotDir + "/app.png"
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
            driver.partDone()
        })
        if (!ok) {
            console.warn("grabToImage returned false")
            driver.appGrabbing = false
            driver.appSaved = true
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
