pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The whole of the window's harness in one part, loaded by name through `HarnessSeat`. Everything the verbs and the
/// measurements act on is handed over here, since a file of its own cannot see `Main.qml`'s ids; the window's calls
/// in — `begin`, `claimPageAct`, `finish` — are forwarded from here.
Item {
    id: harness

    anchors.fill: parent

    /// Automation-only exposures, as `GraphPane.view` is (rules-refs/app-ui.md).
    required property var window
    required property var tabsModel
    required property var pageRepeater
    required property var topBar
    required property var chrome
    required property var mainUi
    required property var gate
    /// The window's edge, drawn outside `mainUi` — the scene picture lays it over the body (`WindowShotMirrors`).
    required property var windowEdge
    required property var windowShape
    required property var windowTip
    required property var openFailedDialog
    required property var identityGate
    required property var identityDialog
    /// The seat the two biggest screens are built in, only once asked for — this file asks ([`screensUp`]).
    required property var dialogSeat
    required property var folderDialog
    required property var quitWaitDialog

    /// The tab in front — the one seat that moves, so the window keeps writing it (`Main.qml`'s `Binding`).
    property var page: null

    /// The screens have been asked for, so the verbs that read them may be built. Two phases, because the order two
    /// siblings finish building in is Qt's; both end inside the window's own completion handler (`HarnessSeat`).
    property bool screensUp: false
    /// The window said the verbs may start, and whether they have (`begin`). One way: verbs open tabs of their own,
    /// and a gate that closed again would take the running verb down.
    property bool beginAsked: false
    property bool begun: false
    readonly property var cloneModel: harness.dialogSeat.cloneModel
    readonly property var cloneDialog: harness.dialogSeat.cloneDialog
    readonly property var settingsDialog: harness.dialogSeat.settingsDialog

    Component.onCompleted: {
        // Only for a verb or the identity hook, which read a screen before opening it (`WindowDialogSeat`).
        harness.dialogSeat.keepBuilt = Harness.autoAct !== "" || Harness.autoIdentity !== ""
        // The offscreen platform reports an 800x800 screen, which would cut every picture down to fit.
        harness.windowShape.keepSavedSize = Harness.automated
        // A run has no hand, yet the offscreen cursor rests on the window's top-left cell and cannot be taken back
        // (rules-refs/app-ui.md「ヘッドレスの窓には手が乗っている」).
        Hand.away = Harness.automated
        // A turning ring photographs differently every time.
        Motion.stilled = Harness.shotDir !== ""
        // The window whose frames the waits on a layout count (`Awaited.still` / `drawnSince`).
        Awaited.window = harness.window
        harness.screensUp = true
    }

    // A tip that opens beside the hand is told where a run's would be: a quarter across the target, so the picture
    // says which of the two the seat was read from.
    Binding {
        target: harness.windowTip
        property: "handAcross"
        value: Harness.autoAct !== "" ? 0.25 : -1
    }
    // A seeded write can land while the shot is still grabbing the wait, and the dialog's own close would take the
    // window and the PNGs down (`finishAutoAct` ends the run alone).
    Binding {
        target: harness.quitWaitDialog
        property: "selfCloses"
        value: Harness.autoAct === ""
    }
    // PGG_AUTO_OPEN: ';'-separated repositories are the tabs, opened in order.
    Connections {
        target: harness.window
        function onStartingTabs() {
            if (Harness.autoOpen === "")
                return
            harness.window.tabsClaimed = true
            const paths = Harness.autoOpen.split(";")
            for (let i = 0; i < paths.length; i++) {
                if (paths[i] !== "")
                    harness.tabsModel.openRepositoryPath(paths[i])
            }
        }
    }

    /// The window is up: the verbs may start, and the shot clock with them — once the strip holds every folder the
    /// run named, since the tabs arrive over later frames (デザイン規約 §タブの所作). One gate for every window verb,
    /// so none has to remember it. The verbs are built here, not told to start: a `SampleTimer` runs off its own
    /// `running`.
    function begin() {
        harness.beginAsked = true
        harness.beginOnceTheStripStands()
    }
    /// The half of [`begin`] that runs when nothing is left to place — now, or on the answer that empties the
    /// queue (`TabsModel.opening`).
    function beginOnceTheStripStands() {
        if (harness.begun || !harness.beginAsked || harness.tabsModel.opening)
            return
        // `actsLoader` builds inside this write, so its item exists by the line below.
        harness.begun = true
        if (actsLoader.item)
            actsLoader.item.begin()
        shotDriver.begin()
    }
    Connections {
        target: harness.tabsModel
        function onOpeningChanged() {
            harness.beginOnceTheStripStands()
        }
    }
    /// One run has one owner of the page-level act (rules/app-ui.md §UI 自動化).
    function claimPageAct() {
        return shotDriver.claimPageAct()
    }
    /// The page's write barrier, for the ceiling's line: the ceiling lives on the shot driver, out of the page's reach
    /// (`AutoActDriver.noteWrite`).
    function noteWriteState(state) {
        shotDriver.writeState = state
    }
    /// One run has one ending: the parts standing are walked in the completing turn (`WindowCensus.markCompletion`)
    /// and the completion said by name with what the page still owed (`Awaited.completed`); then the pictures wait
    /// for one swapped frame (`settle`) — the frame a view builds its rows in, and the frame a read landing in the
    /// completing turn would draw — so that what the completion left moving is in the picture, and the picture's walk
    /// against the completion's names it (`WindowCensus.reportAt`). The walk is not taken again: a run that
    /// completed on a moving scene fails, naming what moved, instead of settling into a state it never claimed.
    function finish() {
        // A run ordered to hold its act has none (`PGG_FAULT_HOLD_ACT`): only the ceiling ends it — the shape
        // `cargo xtask wedge-check` reads back (`xtask::verify::faults`).
        if (Harness.faultHoldAct)
            return
        if (Harness.autoAct === "") {
            shotDriver.finish()
            return
        }
        if (harness.settlingFrom >= 0 || completionWait.running)
            return
        if (census.atCompletion === null) {
            const owed = census.pageMissing()
            if (owed.length > 0 && harness.ambient(owed)) {
                Awaited.note("completion waiting=" + owed.join(","))
                completionWait.start()
                return
            }
            census.markCompletion()
            Awaited.completed(Harness.autoAct, owed)
        }
        harness.settlingFrom = Awaited.frames
        harness.window.update()
    }
    /// The page's reads a completion is held for here, never named by the verb: the two the product's own timers
    /// drive — `placing` (the view re-centring after `Metrics.anchorDelayMs`) and `readings` (the pacer's reads of
    /// other worktrees) — which no report reads. Everything else a run waits for itself before its report, a
    /// window's act included (`PageSettled.windowSettled` in the wait its report follows: this hold comes after the
    /// report, and would hold a report already read), or its completion is a breach (`Awaited.completed`). Once
    /// held, the hold lasts until the page owes nothing: what an ambient term's settling sets moving — a seat the
    /// re-centring displaced from under a stand-in hand (`RowHoverHost`), a walk the pacer's read asked for — is the
    /// hold's too, not the verb's, which completed over the ambient terms alone.
    readonly property var ambientTerms: ["placing", "readings"]
    function ambient(owed) {
        return owed.every(term => harness.ambientTerms.indexOf(term) >= 0)
    }
    SampleTimer {
        id: completionWait
        onTriggered: {
            const owed = census.pageMissing()
            if (owed.length > 0) {
                Awaited.note("completion waiting=" + owed.join(","))
                return
            }
            completionWait.stop()
            harness.finish()
        }
    }
    /// The frame the completion waits out, -1 while no completion is settling.
    property int settlingFrom: -1
    function settle() {
        if (Awaited.frames <= harness.settlingFrom)
            return
        harness.settlingFrom = -1
        shotDriver.finish()
    }

    WindowCensus {
        id: census
        window: harness.window
        harnessBranch: harness
    }

    // The render boundary the waits on a layout read (`Awaited.still` / `drawnSince`), and the completion's walk
    // (`settle`).
    Connections {
        target: harness.window
        function onFrameSwapped() {
            Awaited.frames++
            if (harness.settlingFrom >= 0)
                harness.settle()
        }
    }

    WindowPerfDriver {
        window: harness.window
        page: harness.page
    }

    // The stand-ins a headless shot is grabbed from, behind everything the window draws (the seat carries the `z`).
    WindowShotMirrors {
        id: shotMirrors
        anchors.fill: parent
        window: harness.window
        mainUi: harness.mainUi
        gate: harness.gate
        windowEdge: harness.windowEdge
    }

    AutoShotDriver {
        id: shotDriver
        window: harness.window
        overlayMirror: shotMirrors.overlayMirror
        sceneMirror: shotMirrors.sceneMirror
        mainUi: harness.mainUi
        gate: harness.gate
        chrome: harness.chrome
        // The claims and the walk in the picture's own turn: a frame later is the stretch a read could land in.
        onPictured: artifact => {
            Awaited.pictured(artifact)
            census.reportAt(artifact)
        }
    }

    Loader {
        id: actsLoader
        active: harness.screensUp && harness.begun && Harness.autoAct !== ""
        sourceComponent: WindowAutoActDriver {
            window: harness.window
            tabsModel: harness.tabsModel
            pageRepeater: harness.pageRepeater
            topBar: harness.topBar
            chrome: harness.chrome
            mainUi: harness.mainUi
            gate: harness.gate
            openFailedDialog: harness.openFailedDialog
            identityDialog: harness.identityDialog
            cloneModel: harness.cloneModel
            cloneDialog: harness.cloneDialog
            folderDialog: harness.folderDialog
            settingsDialog: harness.settingsDialog
            quitWaitDialog: harness.quitWaitDialog
        }
    }

    // Off its own knob: the identity hooks run with no verb (`WindowIdentityActs`).
    Loader {
        active: harness.screensUp && Harness.autoIdentity !== ""
        sourceComponent: WindowIdentityActs {
            identityGate: harness.identityGate
            identityDialog: harness.identityDialog
            page: harness.page
        }
    }
}
