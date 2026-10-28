pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import platitude
import platitude.ui

/// The window's half of the PGG_AUTO_ACT harness: the verbs that answer for the window itself — its frame, its band,
/// its tab strip, the state file it comes back to — and for the picker, which is the platform's own window and can
/// only be entered where its answer lands.
///
/// Built by `WindowHarness` only when a verb was given. What the verbs act on is handed in below: a file of its own
/// cannot see the window's ids, and the one list says how far the harness reaches into the window.
///
/// The verbs live in the items at the foot, grouped by the part of the window they answer for; each gates itself on
/// `Harness.autoAct` (rules-refs/structure.md「窓側の自動化は動詞が自分で `running:` に立つ」). The two that stay
/// here read the change signals latched just below, which have to be connected before their verb starts.
// An `Item` only because `QtObject` has no default property to hold the timers below.
Item {
    id: driver

    /// `var` because `Main` is the file the engine loads.
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
    // Latched off the change signals: a failure can clear within one poll when a later command finishes.
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

    /// Called once the tabs are open, which is also when this is built (`WindowHarness.begun`) — so every verb here
    /// starts itself off its own `running:`, and this has nothing to do.
    function begin() {
    }

    // PGG_AUTO_ACT=commands-clear. Answered at the band: the rows and the line both feed one mark, read on both sides
    // of the press (`was=` is the half the picture cannot hold). The standing panel is a precondition read only in
    // the stage that presses (rules-refs/app-ui.md「動詞の前提条件は入力を出す枝で読む」): the press takes it down.
    // The failing command is the page's dispatch (`AutoActFindVerbs`), so the press joins it: a failure latched before
    // it — the opening fetch's — is not the one this run clears.
    WindowStage {
        id: commandsClearActTimer
        name: "commands_clear"
        order: ["cleared"]
        after: ["page/act/dispatched"]
        running: Harness.autoAct === "commands-clear"
        property bool was: false
        onTriggered: {
            if (commandsClearActTimer.at("start")) {
                if (!Awaited.all("commands_clear_ready", {
                        "joined": commandsClearActTimer.joined(),
                        "wrong": driver.commandsWrongSeen,
                        "idle": window.curPage !== null && !window.curPage.pageCommands.running,
                        "shown": window.curPage !== null && window.curPage.commandsShown }))
                    return
                commandsClearActTimer.input("cleared", () => {
                    commandsClearActTimer.was = driver.commandsWrongSeen
                    window.curPage.clearCommandLog()
                })
                return
            }
            if (!Awaited.all("commands_clear_cleared", {
                    "cleared": !window.curPage.commandsWrong && !window.curPage.commandsShown }))
                return
            commandsClearActTimer.complete(() => "commands_clear was=" + commandsClearActTimer.was
                                                 + " wrong=" + window.curPage.commandsWrong
                                                 + " open=" + window.curPage.commandsShown
                                                 + " mark=" + window.curPage.commandsMarkColor)
        }
    }

    // PGG_AUTO_ACT=fetch-recover: recovery is what retires fetch news. Waits for the refusal to raise the panel, reads
    // the mark, fires a fetch that can land, and reads both again — the picture only holds the quiet half.
    //
    // `-held` is the same run over a panel the reader put up first; the two part only on the last `open=` (down for
    // one, up for the other). The close happens inside the drain that zeroes the count, so it needs no wait of its own.
    //
    // The standing panel is read only in the stage that fetches, as in `commands-clear`. Both halves of the failure
    // — the mark and the line, which arrive in no fixed order — are latched before the fetch goes, since the report
    // claims both (rules-refs/app-ui.md「報告行が主張する到着は、前提条件でも全部待つ」). The refusal is the page's
    // dispatch (`AutoActPublishVerbs`), which the fetch joins.
    WindowStage {
        id: fetchRecoverActTimer
        name: "fetch_recover"
        order: ["fetched"]
        after: ["page/act/dispatched"]
        running: Harness.autoAct === "fetch-recover" || Harness.autoAct === "fetch-recover-held"
        property bool was: false
        property bool hadLine: false
        property bool wasOpen: false
        onTriggered: {
            if (fetchRecoverActTimer.at("start")) {
                if (!Awaited.all("fetch_recover_ready", {
                        "joined": fetchRecoverActTimer.joined(),
                        "wrong": driver.commandsWrongSeen,
                        "line": driver.errorLineSeen,
                        "shown": window.curPage !== null && window.curPage.commandsShown }))
                    return
                // A run that ends at the ceiling after this stage was waiting for the recovery to take the news down.
                fetchRecoverActTimer.input("fetched", () => {
                    fetchRecoverActTimer.was = driver.commandsWrongSeen
                    fetchRecoverActTimer.hadLine = driver.errorLineSeen
                    fetchRecoverActTimer.wasOpen = window.curPage.commandsShown
                    window.curPage.pageTab.fetch("")
                })
                return
            }
            if (!Awaited.all("fetch_recover_recovered", {
                    "quiet": !(window.curPage.commandsWrong
                               || window.curPage.pageTab.lastError !== ""
                               || window.curPage.pageTab.busyCount !== 0
                               || window.curPage.pageTab.fetchFailures !== 0) }))
                return
            fetchRecoverActTimer.complete(() => "fetch_recover was=" + fetchRecoverActTimer.was
                                                + " hadline=" + fetchRecoverActTimer.hadLine
                                                + " wasopen=" + fetchRecoverActTimer.wasOpen
                                                + " wrong=" + window.curPage.commandsWrong
                                                + " line=" + (window.curPage.pageTab.lastError !== "")
                                                + " failures=" + window.curPage.pageTab.fetchFailures
                                                + " open=" + window.curPage.commandsShown)
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
    /// Reads the strip's items through the one name the strip hands over (`TabStrip.tabsView`).
    readonly property TabProbe tabProbe: TabProbe {
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
    WindowOpsActs {
        window: driver.window
        tabsModel: driver.tabsModel
        pageRepeater: driver.pageRepeater
        topBar: driver.topBar
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
