pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import platitude

/// Owns one automated run from completion through a rendered screenshot.
/// The ordinary application constructs it too, but every timer stays idle
/// unless the PG_AUTO_* harness variables were supplied.
Item {
    id: driver

    property var window
    property Loader overlayMirror
    property ColumnLayout mainUi
    property Item gate

    property bool pageActClaimed: false
    property bool shotPending: false
    property bool shotTaken: false
    property int shotParts: 0
    readonly property bool causal:
        AppBackend.autoAct === "diff-file"
        || AppBackend.autoAct === "signature-tip"
        || AppBackend.autoAct === "open-fail-tab"
        || AppBackend.autoAct === "open-fail-tab-bare"
        || AppBackend.autoAct === "open-fail-tab-log"

    function claimPageAct() {
        if (driver.pageActClaimed)
            return false
        driver.pageActClaimed = true
        return true
    }

    function begin() {
        if (AppBackend.autoWatchdogMs > 0 || AppBackend.autoQuitMs > 0)
            watchdog.start()
        if (AppBackend.shotDir !== "" && !driver.causal)
            legacyShot.start()
    }

    function finish() {
        if (!driver.causal || driver.shotTaken || driver.shotPending)
            return
        AppBackend.report("auto_act complete=" + AppBackend.autoAct)
        driver.shotPending = true
        window.requestUpdate()
        // A quiet scene may not emit another frameSwapped even after an
        // update request. The next event-loop turn is still causal to the
        // completed driver state, and grabToImage owns the render callback.
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

    Timer {
        id: watchdog
        interval: Math.max(AppBackend.autoWatchdogMs > 0
                           ? AppBackend.autoWatchdogMs
                           : AppBackend.autoQuitMs, 1)
        onTriggered: {
            console.warn("auto-act watchdog expired verb=" + AppBackend.autoAct)
            Qt.quit()
        }
    }
    Timer {
        id: legacyShot
        interval: AppBackend.autoQuitMs > 800 ? AppBackend.autoQuitMs - 800 : 3500
        onTriggered: driver.takeShot()
    }
    function partDone() {
        driver.shotParts--
        if (driver.shotParts === 0)
            Qt.quit()
    }

    function takeShot() {
        if (driver.shotTaken)
            return
        driver.shotTaken = true
        const path = AppBackend.shotDir + "/app.png"
        driver.shotParts = overlayMirror.item ? 2 : 1
        if (overlayMirror.item) {
            const overlayOk = overlayMirror.item.grabToImage(function (res) {
                const saved = res.saveToFile(AppBackend.shotDir + "/overlay.png")
                console.warn("overlay saved=" + saved)
                driver.partDone()
            })
            if (!overlayOk) {
                console.warn("overlay grabToImage returned false")
                driver.partDone()
            }
        }
        const shown = gate.visible ? gate : mainUi
        const ok = shown.grabToImage(function (res) {
            const saved = res.saveToFile(path)
            console.warn("screenshot saved=" + saved + " path=" + path)
            driver.partDone()
        })
        if (!ok) {
            console.warn("grabToImage returned false")
            driver.partDone()
        }
    }
}
