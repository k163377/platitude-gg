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
    required property var windowShape
    required property var sharedToolTip
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
        harness.screensUp = true
    }

    // A tip that opens beside the hand is told where a run's would be: a quarter across the target, so the picture
    // says which of the two the seat was read from.
    Binding {
        target: harness.sharedToolTip
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
    /// One run has one ending, and it waits for the census walk (`AutoShotDriver.appPictured`).
    function finish() {
        // A run ordered to hold its act has none (`PGG_FAULT_HOLD_ACT`): only the ceiling ends it — the shape
        // `cargo xtask wedge-check` reads back (`xtask::verify::faults`).
        if (Harness.faultHoldAct)
            return
        shotDriver.finish()
    }

    WindowCensus {
        id: census
        window: harness.window
        onWalked: shotDriver.censusDone()
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
    }

    AutoShotDriver {
        id: shotDriver
        window: harness.window
        overlayMirror: shotMirrors.overlayMirror
        sceneMirror: shotMirrors.sceneMirror
        mainUi: harness.mainUi
        gate: harness.gate
        // `armed`: a run that ends there had its frame withheld.
        censusState: census.armed ? "armed" : census.waiting ? "waiting" : census.settled ? "settled" : "arriving"
        onAppPictured: census.report()
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
        }
    }
}
