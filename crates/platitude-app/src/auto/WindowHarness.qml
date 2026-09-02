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
    required property var openFailedDialog
    required property var identityDialog
    required property var cloneModel
    required property var cloneDialog
    required property var folderDialog
    required property var settingsDialog
    required property var quitWaitDialog

    /// The tab in front. The one seat that moves under the harness, so the window writes it rather than handing it
    /// over once (`Main.qml`'s `Binding`).
    property var page: null

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
    /// One run has one ending. The census is not asked for here: it reports on the picture the shot driver saves
    /// (`AutoShotDriver.appPictured`), which is the scene the completion edge finished building.
    function finish() {
        shotDriver.finish()
    }

    WindowCensus {
        id: census
        window: harness.window
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
        onAppPictured: census.report()
    }

    // Built only when a verb was given: a run that is only being measured or photographed carries none of the verbs.
    Loader {
        id: actsLoader
        active: AppBackend.autoAct !== ""
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
}
