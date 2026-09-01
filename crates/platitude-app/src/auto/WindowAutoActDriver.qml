pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import platitude
import platitude.ui

/// The window's half of the PG_AUTO_ACT harness: the verbs that answer for the window itself — its frame, its band,
/// its tab strip, the state file it comes back to — and for the picker, which is the platform's own window and can
/// only be entered where its answer lands.
///
/// `Main` builds this only when a verb was given, so an ordinary run carries none of it. What the verbs act on is
/// handed in below: a file of its own cannot see the window's ids, and naming them in one list is what says how far the
/// harness reaches into the window.
///
/// The verbs themselves are grouped by the part of the window they answer for and live in the five items at the foot of
/// this file. Each of those gates itself on `AppBackend.autoAct`, so the grouping is only ever about where a verb is
/// read: nothing here hands one its turn. The two that stay are the two that cannot — they read the change signals
/// latched just below, which have to be connected before the verb they belong to starts.
// An `Item` only because `QtObject` has no default property to hold the timers below; it draws nothing and is never
// given a size.
Item {
    id: driver

    /// The window these verbs act on, and the parts of it they read back or leave standing for the shot. An
    /// automation-only exposure, the same one `GraphPane.view` is (app-ui.md). `var` because `Main` is the file the
    /// engine loads rather than a type anything can name.
    property var window

    property TabsModel tabsModel
    property Repeater pageRepeater
    property TopBar topBar
    property WindowChrome chrome
    property Item mainUi
    property Item gate
    property OpenFailedDialog openFailedDialog
    property IdentityDialog identityDialog
    property CloneModel cloneModel
    property CloneDialog cloneDialog
    property var folderDialog
    property var settingsDialog
    property QuitWaitDialog quitWaitDialog
    // Negative/error states can be shorter than a polling cadence when a later background command also finishes.
    // Observe their change signals synchronously, then carry that proof into the recovery report.
    property bool commandsWrongSeen: false
    property bool errorLineSeen: false
    Connections {
        target: window.curPage
        function onCommandsWrongChanged() {
            if (window.curPage.commandsWrong)
                driver.commandsWrongSeen = true
        }
    }
    Connections {
        target: window.curPage ? window.curPage.pageTab : null
        function onChanged() {
            if (window.curPage && window.curPage.pageTab.lastError !== "")
                driver.errorLineSeen = true
        }
    }

    /// Kicked off by the window once its tabs are open: a verb that ran before them would answer for a window holding
    /// nothing. The verbs missing from this list start themselves — theirs is a `running:` that is true from the moment
    /// this is built.
    function begin() {
        // All timers below are state polls. They never decide that a state is ready because a duration elapsed; each
        // one stops only after the property/event it reports is observable. The parent watchdog is the sole hang
        // ceiling.
    }

    // PG_AUTO_ACT=commands-clear. The band is where the answer is — the rows and the line both feed one mark, and
    // clearing only the rows left it red over an empty panel, which is why this verb lives up here. The
    // same mark is read on both sides of the press: `was=` is the half the picture cannot hold.
    //
    // The panel the press takes down with them is the other half. Which is why the standing-panel half of the
    // preconditions is read in the branch that presses and nowhere else (規約 §UI 自動化の因果性): every tick, it would be
    // this verb's own answer — the panel gone — barring the way to the report.
    SampleTimer {
        id: commandsClearActTimer
        running: AppBackend.autoAct === "commands-clear"
        property bool clearRequested: false
        property bool was: false
        onTriggered: {
            if (window.curPage === null)
                return
            if (!commandsClearActTimer.clearRequested) {
                if (!driver.commandsWrongSeen
                        || window.curPage.pageCommands.running
                        || !window.curPage.commandsShown)
                    return
                commandsClearActTimer.was = driver.commandsWrongSeen
                commandsClearActTimer.clearRequested = true
                window.curPage.clearCommandLog()
                return
            }
            if (window.curPage.commandsWrong || window.curPage.commandsShown)
                return
            stop()
            AppBackend.report(
                "commands_clear was=" + commandsClearActTimer.was
                + " wrong=" + window.curPage.commandsWrong
                + " open=" + window.curPage.commandsShown
                + " mark=" + window.curPage.commandsMarkColor)
            window.finishAutoAct()
        }
    }

    // PG_AUTO_ACT=fetch-recover: recovery, not the reader, is what retires fetch news. This waits for the refusal,
    // reads the mark, fires the fetch that can land, and reads the same mark again — the picture can only hold the
    // quiet half.
    SampleTimer {
        id: fetchRecoverActTimer
        running: AppBackend.autoAct === "fetch-recover"
        property bool fetchRequested: false
        property bool was: false
        property bool hadLine: false
        onTriggered: {
            if (window.curPage === null)
                return
            if (!fetchRecoverActTimer.fetchRequested) {
                if (!driver.commandsWrongSeen && !driver.errorLineSeen)
                    return
                fetchRecoverActTimer.was = driver.commandsWrongSeen
                fetchRecoverActTimer.hadLine = driver.errorLineSeen
                fetchRecoverActTimer.fetchRequested = true
                window.curPage.pageTab.fetch("")
                return
            }
            if (window.curPage.commandsWrong
                    || window.curPage.pageTab.lastError !== ""
                    || window.curPage.pageTab.busyCount !== 0
                    || window.curPage.pageTab.fetchFailures !== 0)
                return
            stop()
            AppBackend.report(
                "fetch_recover was=" + fetchRecoverActTimer.was
                + " hadline=" + fetchRecoverActTimer.hadLine
                + " wrong=" + window.curPage.commandsWrong
                + " line=" + (window.curPage.pageTab.lastError !== "")
                + " failures=" + window.curPage.pageTab.fetchFailures
                + " open=" + window.curPage.commandsShown)
            window.finishAutoAct()
        }
    }

    WindowTabActs {
        window: driver.window
        tabsModel: driver.tabsModel
        pageRepeater: driver.pageRepeater
        topBar: driver.topBar
        tabProbe: tabProbe
        mainUi: driver.mainUi
        gate: driver.gate
    }
    WindowTabPinActs {
        window: driver.window
        tabsModel: driver.tabsModel
        pageRepeater: driver.pageRepeater
        topBar: driver.topBar
        tabProbe: tabProbe
    }
    /// What the strip's own items came out as. Here rather than in the strip: reading tabs back is the harness's
    /// business, and the strip's part in it is the one name it hands over (`TabStrip.tabsView`).
    TabProbe {
        id: tabProbe
        view: driver.topBar.tabsView
    }
    WindowBandActs {
        window: driver.window
        topBar: driver.topBar
        chrome: driver.chrome
        mainUi: driver.mainUi
        identityDialog: driver.identityDialog
    }
    WindowBadgeActs {
        window: driver.window
        topBar: driver.topBar
        mainUi: driver.mainUi
        identityDialog: driver.identityDialog
    }
    WindowFrameActs {
        window: driver.window
        tabsModel: driver.tabsModel
        pageRepeater: driver.pageRepeater
        topBar: driver.topBar
        mainUi: driver.mainUi
    }
    WindowDialogActs {
        window: driver.window
        tabsModel: driver.tabsModel
        pageRepeater: driver.pageRepeater
        topBar: driver.topBar
        gate: driver.gate
        openFailedDialog: driver.openFailedDialog
        identityDialog: driver.identityDialog
        cloneModel: driver.cloneModel
        cloneDialog: driver.cloneDialog
        folderDialog: driver.folderDialog
        settingsDialog: driver.settingsDialog
        quitWaitDialog: driver.quitWaitDialog
    }
    WindowSettingsActs {
        window: driver.window
        settingsDialog: driver.settingsDialog
    }
}
