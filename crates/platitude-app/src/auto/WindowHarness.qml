pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The whole of the window's harness in one part, which the window's own QML loads by name and nothing else names
/// (`HarnessSeat`). Everything the verbs and the measurements act on is handed over here: a file of its own cannot see
/// `Main.qml`'s ids, and one list of them is what says how far the harness reaches into the window.
///
/// The three ways in the window itself calls — `begin`, `claimPageAct`, `finish` — are forwarded from here, so the
/// window holds one null check apiece instead of one per part.
Item {
    id: harness

    anchors.fill: parent

    /// The window, and the parts of it the verbs read back or leave standing for the shot. An automation-only
    /// exposure, the same one `GraphPane.view` is (app-ui.md).
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
    /// The seat the two biggest screens are built in, rather than the screens: neither exists until somebody asks
    /// for it, and asking is this file's ([`screensUp`]).
    required property var dialogSeat
    required property var folderDialog
    required property var quitWaitDialog

    /// The tab in front. The one seat that moves under the harness, so the window writes it rather than handing it
    /// over once (`Main.qml`'s `Binding`).
    property var page: null

    /// The screens have been asked for, so the verbs that read them may be built. **Two phases rather than one
    /// binding**: the order two siblings finish being built in is not something to stand a null check on, and this
    /// whole part is built inside the window's own completion handler, so both phases are over before the window
    /// reaches its next line (`HarnessSeat`).
    property bool screensUp: false
    readonly property var cloneModel: harness.dialogSeat.cloneModel
    readonly property var cloneDialog: harness.dialogSeat.cloneDialog
    readonly property var settingsDialog: harness.dialogSeat.settingsDialog

    Component.onCompleted: {
        // A verb reads a screen's properties before opening it, and a null there is a dead run rather than a
        // refusal. Only where a verb or the identity hook is going to want one: holding the two ready costs the bare
        // window ~8MB of working set (`WindowDialogSeat`).
        harness.dialogSeat.keepBuilt = Harness.autoAct !== "" || Harness.autoIdentity !== ""
        // The offscreen platform reports an 800x800 screen, which would cut every picture down to fit.
        harness.windowShape.keepSavedSize = Harness.automated
        // A turning ring photographs differently every time.
        Motion.stilled = Harness.shotDir !== ""
        harness.screensUp = true
    }

    // A run has no pointer to rest anywhere, and a tip that opens beside the hand has to be told where one would have
    // been. A quarter across the target, so the picture says which of the two the seat was read from.
    Binding {
        target: harness.sharedToolTip
        property: "handAcross"
        value: Harness.autoAct !== "" ? 0.25 : -1
    }
    // The photograph is of the wait itself, and a seeded write can land while the shot pipeline is still grabbing —
    // the close the dialog would fire takes the window, and the PNGs, down with it (`finishAutoAct` ends a run, never
    // a window).
    Binding {
        target: harness.quitWaitDialog
        property: "selfCloses"
        value: Harness.autoAct === ""
    }
    // PGG_AUTO_OPEN: ';'-separated repositories open as tabs in order, instead of the ones that were left.
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

    /// The window is up: the verbs may start, and the shot clock with them.
    function begin() {
        if (actsLoader.item)
            actsLoader.item.begin()
        shotDriver.begin()
    }
    /// One run has one owner of the page-level act (app-ui.md §UI 自動化の因果性).
    function claimPageAct() {
        return shotDriver.claimPageAct()
    }
    /// One run has one ending. The census is not asked for here: the picture calls for it
    /// (`AutoShotDriver.appPictured`) and it walks once the window has stopped arriving, which the ending waits for.
    function finish() {
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

    // The two grabbable stand-ins a headless shot is taken from, behind everything the window draws — the seat this
    // part is built into carries the `z` that puts them there.
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
        // `armed` is a frame asked for and not yet polished — a run that ends there had the window's word withheld.
        censusState: census.armed ? "armed" : census.waiting ? "waiting" : census.settled ? "settled" : "arriving"
        onAppPictured: census.report()
    }

    // Built only when a verb was given: a run that is only being measured or photographed carries none of the verbs.
    Loader {
        id: actsLoader
        active: harness.screensUp && Harness.autoAct !== ""
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

    // The identity hooks are not verbs and run without one (`PGG_AUTO_IDENTITY`), so they are built off their own
    // knob rather than beside the verbs.
    Loader {
        active: harness.screensUp && Harness.autoIdentity !== ""
        sourceComponent: WindowIdentityActs {
            identityGate: harness.identityGate
            identityDialog: harness.identityDialog
        }
    }
}
