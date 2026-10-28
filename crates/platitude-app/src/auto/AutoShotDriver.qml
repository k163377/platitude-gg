pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import platitude
import platitude.ui

/// Owns one automated run from completion through its pictures; idle unless PGG_AUTO_* was supplied.
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
    /// The overlay mirror stood when the pictures were called for, so its picture is a part.
    property bool overlayWanted: false
    /// The overlay held something when the pictures were called for (its children, as `popups=` counts them).
    property bool sceneWanted: false
    /// Pictures still owed, saved or failed (`partDone`): the run ends on the last.
    property int shotParts: 0
    /// Marks the picture waited for (0 if the scene was already drawn); reported, so the wait cannot go unwired.
    property int inkWaited: 0
    // Every PGG_AUTO_ACT run has one explicit completion edge; a verb without one meets the watchdog.
    readonly property bool causal: Harness.autoAct !== ""

    /// A picture is on disk — `app` (the window, whose scene is the settled one: the completion edge often builds
    /// that scene), `overlay` or `scene` — with the page as its grab answered (`owed`). Whatever reads the tree for
    /// what this run shows hears it here (`WindowHarness` → `WindowCensus`, `Awaited`), in the picture's own turn: a
    /// frame later is the stretch a read could land in.
    signal pictured(string artifact, var owed)

    /// Where the run's own write had got to, for the watchdog's line — written by the page that claimed the run
    /// (`Main.noteAutoActWrite` ← `AutoActDriver`), so a run that never reached a page keeps the dash.
    property string writeState: "-"
    /// When the last frame swapped (-1 for none), for the watchdog's line: with the frames counted (`Awaited.frames`)
    /// it separates a term stuck over a scene still drawing from one over a scene that stopped.
    property real lastFrameAt: -1
    /// The page's reads still on their way (`PageSettled.missing`), asked of whoever holds the census
    /// (`WindowHarness`) with the picture the reading is for: at each attempt's start and as its grab answers
    /// (`Attempt`). The answer's one reading, before the save and the walk — each takes its time under load, and a
    /// read that began meanwhile would be in a second reading — is both what holds the picture and what its line
    /// says (`pictured`). Null owes nothing.
    property var pageOwed: null
    function owedNow(artifact) {
        return driver.pageOwed !== null ? driver.pageOwed(artifact) : []
    }
    /// Whether a picture waits for the page it owes (`WindowHarness.holdsPicture`). Null holds nothing.
    property var pictureHeld: null
    function held(owed) {
        return driver.pictureHeld !== null && driver.pictureHeld(owed) === true
    }

    /// One picture's attempts. An attempt is one claim — the texture saved is of the page settled, and the page as
    /// the grab answered is the page its line says — run in stages that each wait on a cause of their own:
    /// `settling`, the page owing nothing a picture waits for (`pageOwed` / `pictureHeld`), asked again at every
    /// frame while it does; `refreshing`, the picture's source asked for a texture of the scene as it now stands and
    /// its completion awaited (a mirror's `scheduleUpdate` / `refresh`; the window's own is the grab, which draws
    /// the item afresh in the next frame); `grabbing`, `grabToImage` asked and its answer awaited; then the state as
    /// the grab answered is held against the picture. Ink still owed (`Ink`, the window's), or the page owing a read
    /// again, discards the attempt whole — texture and answer — and the next begins from the page (`start`): nothing
    /// asks a second grab off a texture an earlier attempt refreshed, and no completion or answer of an earlier
    /// attempt is read into a later one (`number`). An attempt that ends short says why (`picture … attempt=
    /// waiting= / held= / ink=`); the one that saves says its number (`picture … attempt= saved=`).
    component Attempt: QtObject {
        required property string artifact
        /// Attempts begun; the one under way.
        property int number: 0
        /// `idle`, `settling`, `refreshing`, `grabbing`, `inking` (discarded for the ink; the next begins as it
        /// lands) or `done`.
        property string stage: "idle"
        /// The wait for the page was said for this attempt (once).
        property bool waitingSaid: false
    }
    readonly property Attempt app: Attempt { artifact: "app" }
    readonly property Attempt overlay: Attempt { artifact: "overlay" }
    readonly property Attempt scene: Attempt { artifact: "scene" }

    /// A line of the attempt's own, under the run's id (`verify::contract` reads the `saved=` ones: the set asked
    /// for (`pictures run=`) is the set saved, and a save that failed is no picture, whatever came after it).
    function say(attempt, what) {
        Harness.report("picture run=" + Awaited.runId + " artifact=" + attempt.artifact
                       + " attempt=" + attempt.number + " " + what)
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

    function takeShot() {
        if (driver.shotTaken)
            return
        driver.shotTaken = true
        const holding = overlayMirror.item && overlayMirror.item.sourceItem
                        ? overlayMirror.item.sourceItem.children.length : 0
        driver.sceneWanted = holding > 0 && driver.sceneMirror !== null && driver.sceneMirror.item !== null
        driver.overlayWanted = overlayMirror.item !== null
        driver.shotParts = 1 + (driver.overlayWanted ? 1 : 0) + (driver.sceneWanted ? 1 : 0)
        // The pictures this run owes, fixed here: each is said saved or not (`say`), and the parent passes the run
        // only on all of them.
        const wanted = ["app"].concat(driver.overlayWanted ? ["overlay"] : [], driver.sceneWanted ? ["scene"] : [])
        Harness.report("pictures run=" + Awaited.runId + " wanted=" + wanted.join(","))
        driver.start(driver.app)
    }

    /// The next attempt at a picture, from the page.
    function start(attempt) {
        attempt.number++
        attempt.waitingSaid = false
        attempt.stage = "settling"
        driver.settle(attempt)
    }
    /// The settling stage, taken as the attempt starts and at every frame it waits through (`onFrameSwapped`).
    function settle(attempt) {
        if (attempt.stage !== "settling")
            return
        const owed = driver.owedNow(attempt.artifact)
        if (driver.held(owed)) {
            if (!attempt.waitingSaid) {
                attempt.waitingSaid = true
                driver.say(attempt, "waiting=" + owed.join(","))
            }
            driver.window.update()
            return
        }
        attempt.stage = "refreshing"
        if (attempt === driver.app)
            driver.refreshed(attempt)
        else if (attempt === driver.overlay)
            overlayMirror.item.scheduleUpdate()
        else
            driver.sceneMirror.item.refresh(attempt.number)
    }
    /// The source's texture stands — a mirror's completion (`onScheduledUpdateCompleted` / `onReady`), the window's
    /// at once — and the grab is asked. A completion of no attempt under way (the mirror's own on being built) is
    /// nobody's.
    function refreshed(attempt) {
        if (attempt.stage !== "refreshing")
            return
        attempt.stage = "grabbing"
        const number = attempt.number
        const source = attempt === driver.app ? (gate.visible ? gate : mainUi)
                     : attempt === driver.overlay ? overlayMirror.item : driver.sceneMirror.item
        const asked = source.grabToImage(res => driver.answered(attempt, number, res))
        if (!asked) {
            console.warn(attempt.artifact + " grabToImage returned false")
            driver.conclude(attempt, false, driver.owedNow(attempt.artifact))
        }
    }
    /// The grab's answer, held against the state it answered in. An answer of an attempt already discarded is
    /// nobody's.
    function answered(attempt, number, res) {
        if (number !== attempt.number || attempt.stage !== "grabbing")
            return
        // The page first, so an attempt's second reading is its answer's whatever the ink says (`shot-check`
        // counts on that); then the ink — the completion edge often builds the scene, and a `Canvas` has no ink
        // in its first frame (rules-refs/app-ui.md「`Canvas` は生まれたフレームにインクを持たない」).
        const owed = driver.owedNow(attempt.artifact)
        if (driver.held(owed)) {
            driver.say(attempt, "held=" + owed.join(","))
            driver.start(attempt)
            return
        }
        if (attempt === driver.app && Ink.owed > 0) {
            driver.inkWaited = Math.max(driver.inkWaited, Ink.owed)
            driver.say(attempt, "ink=" + Ink.owed)
            attempt.stage = "inking"
            return
        }
        driver.conclude(attempt, res.saveToFile(Harness.shotDir + "/" + attempt.artifact + ".png"), owed)
    }
    /// The picture is on disk, or failed: said, claimed (`pictured`), and the window's followed by the mirrors' —
    /// refreshed earlier, a popup's marks could be owed like the window's (`Ink`), and a texture refreshed before
    /// they paint holds their rooms with nothing in them.
    function conclude(attempt, saved, owed) {
        attempt.stage = "done"
        if (attempt === driver.app) {
            console.warn("screenshot saved=" + saved + " ink=" + driver.inkWaited
                         + " path=" + Harness.shotDir + "/app.png")
        } else if (attempt === driver.overlay) {
            // `popups=` counts what the overlay held, so a blank overlay.png means nothing was open; `yield=` is
            // whether those popups had the window's frame handed to the scene, so a press on the band's grab runs or
            // the edges would reach them — true for a menu or list a press outside closes.
            const mirror = overlayMirror.item
            const popups = mirror.sourceItem ? mirror.sourceItem.children.length : 0
            const yielded = driver.chrome !== null && driver.chrome.frameYielded
            console.warn("overlay saved=" + saved + " popups=" + popups + " yield=" + yielded)
        } else {
            console.warn("scene saved=" + saved)
        }
        driver.say(attempt, "saved=" + saved)
        driver.pictured(attempt.artifact, owed)
        if (attempt === driver.app) {
            if (driver.overlayWanted)
                driver.start(driver.overlay)
            if (driver.sceneWanted)
                driver.start(driver.scene)
        }
        driver.partDone()
    }
    function partDone() {
        driver.shotParts--
        if (driver.shotParts === 0)
            Qt.quit()
    }

    Connections {
        target: driver.window
        function onFrameSwapped() {
            driver.lastFrameAt = PerfProbe.clockMs()
            driver.scheduleShot()
            driver.settle(driver.app)
            driver.settle(driver.overlay)
            driver.settle(driver.scene)
        }
    }

    // The overlay's render boundary, and why `WindowShotMirrors` builds the mirror `live: false`: a live one leaves
    // nothing to wait on, and a popup opened in the completing turn is often not in its texture yet.
    Connections {
        target: driver.overlayMirror ? driver.overlayMirror.item : null
        ignoreUnknownSignals: true
        function onScheduledUpdateCompleted() {
            driver.refreshed(driver.overlay)
        }
    }

    // The composite's textures stand, for the attempt they were asked for (`WindowShotMirrors.refresh`).
    Connections {
        target: driver.sceneMirror ? driver.sceneMirror.item : null
        ignoreUnknownSignals: true
        function onReady(attempt) {
            if (attempt === driver.scene.number)
                driver.refreshed(driver.scene)
        }
    }

    // The ink landed: the window's attempt discarded for it begins again, from the page.
    Connections {
        target: Ink
        function onOwedChanged() {
            if (driver.app.stage === "inking" && Ink.owed === 0)
                driver.start(driver.app)
        }
    }

    // The only clock here. A run that reaches it has failed; it reports what the machine was holding.
    // waits(ceiling): a run that stopped answering ends here, and every verb's own completion is causal
    Timer {
        id: watchdog
        interval: Math.max(Harness.autoWatchdogMs, 1)
        onTriggered: {
            console.warn("auto-act watchdog expired verb=" + Harness.autoAct)
            console.warn("shot state app=" + driver.app.stage + "/" + driver.app.number
                         + " overlay=" + driver.overlay.stage + "/" + driver.overlay.number
                         + " scene=" + driver.scene.stage + "/" + driver.scene.number
                         + " parts=" + driver.shotParts + " ink=" + Ink.owed)
            // What the write barrier was holding, if the run wanted a write
            // (rules-refs/app-ui.md「天井まで行った run は…」).
            console.warn("write state " + driver.writeState)
            // What the verb was still waiting on and for how long (`Awaited`); `unsaid` for a verb that says none.
            const now = PerfProbe.clockMs()
            console.warn("awaited " + (Awaited.said === "" ? "unsaid"
                                       : Awaited.said + " for " + Math.round(now - Awaited.since) + "ms"))
            // Where each operation still in a generation stands (`StageTimer`): the stalled one is among them.
            console.warn("stages " + Awaited.standing())
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
}
