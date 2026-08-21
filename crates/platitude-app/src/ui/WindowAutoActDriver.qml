pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Window
import platitude
import platitude.ui

/// The window's half of the PG_AUTO_ACT harness: the verbs that answer for the window itself — its floor, its band, the
/// state file it comes back to — and for the picker, which is the platform's own window and can only be entered where
/// its answer lands.
///
/// `Main` builds this only when a verb was given, so an ordinary run carries none of it. What the verbs act on is
/// handed in below: a file of its own cannot see the window's ids, and naming them in one list is what says how far the
/// harness reaches into the window.
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
    property ColumnLayout mainUi
    property Item gate
    property OpenFailedDialog openFailedDialog
    property IdentityDialog identityDialog
    property var folderDialog
    property var settingsDialog
    // Negative/error states can be shorter than a polling cadence when a later background command also finishes.
    // Observe their change signals synchronously, then carry that proof into the recovery report.
    property bool commandsWrongSeen: false
    property bool errorLineSeen: false
    Connections {
        target: topBar
        function onCommandsWrongChanged() {
            if (topBar.commandsWrong)
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

    // The platform picker completes only once its own window is open.
    Timer {
        interval: 25
        repeat: true
        running: AppBackend.autoAct === "open-picker"
        onTriggered: {
            if (!folderDialog.opened)
                return
            stop()
            AppBackend.report("picker folder=" + folderDialog.currentFolder)
            window.finishAutoAct()
        }
    }

    // The tools popup has two separately latched output states: a real loading edge and the populated, settled choices.
    Timer {
        interval: 25
        repeat: true
        running: AppBackend.autoAct === "settings-tools"
                 || AppBackend.autoAct === "settings-tools-loading"
        onTriggered: {
            const ready = AppBackend.autoAct === "settings-tools-loading"
                        ? settingsDialog.autoToolsLoadingReady
                        : settingsDialog.autoToolsSettledReady
            if (!ready)
                return
            stop()
            settingsDialog.reportTool()
            window.finishAutoAct()
        }
    }

    // What remains after a middle-click is the output under test. Wait for the tab-model count edge rather than
    // allowing a fixed delay to stand in for it. The two tabs this verb needs are a precondition of the press and
    // nothing else: read again after it, they turn the verb's own answer — one tab fewer — into a wait nothing can end.
    Timer {
        id: middleCloseTimer
        interval: 25
        repeat: true
        running: AppBackend.autoAct === "middle-close"
        property bool requested: false
        property int beforeCount: -1
        property string closedPath: ""
        onTriggered: {
            if (!middleCloseTimer.requested) {
                if (pageRepeater.count < 2)
                    return
                middleCloseTimer.beforeCount = pageRepeater.count
                const at = Number(AppBackend.autoActArg)
                middleCloseTimer.closedPath = topBar.tabPathAt(at)
                // Latched on the strip's answer rather than on the asking: the press has to land on an item, and the
                // row the model has just gained gets one with the layout.
                middleCloseTimer.requested = topBar.middleClickTab(at)
                return
            }
            // The strip is read back below, so it has to have caught up with the model before there is anything true to
            // say about which repository went and which is still standing.
            if (pageRepeater.count >= middleCloseTimer.beforeCount || topBar.tabItemCount() !== pageRepeater.count)
                return
            // And the tab that stayed has to be showing its repository: "the neighbour is still there" is the half of
            // this verb the picture carries, and a page still opening photographs the same whether it survived the
            // close or was never opened at all. The page driver's own baseline (AutoActDriver), asked of whichever page
            // the close left in front.
            const kept = window.curPage
            if (kept === null || kept.pageTab.state !== "open"
                    || !kept.pageWt.loaded || kept.pageGraph.finishCount === 0)
                return
            stop()
            // The verdict leads, and it is about the tab the press landed on: a count that merely fell would pass with
            // the wrong tab closed, and the titles cannot tell them apart.
            AppBackend.report("middle_close gone="
                              + !topBar.hasTabPath(middleCloseTimer.closedPath)
                              + " tabs=" + pageRepeater.count
                              + " active=" + tabsModel.currentIndex
                              + " open=" + topBar.tabPaths())
            window.finishAutoAct()
        }
    }

    // What the strip is holding after a tab was carried across it. The order is the output, and it is read off the
    // items rather than the model: the drag settles itself against where the tabs actually sit, so the walk is what
    // says the two agree. The tabs it needs are a precondition of the carry and are read in that branch alone —
    // afterwards, "the order is not the one it started as" is this verb's own answer.
    Timer {
        id: tabDragTimer
        interval: 25
        repeat: true
        running: AppBackend.autoAct === "tab-drag"
        property bool requested: false
        property string before: ""
        property int from: 0
        property int to: 0
        onTriggered: {
            if (!tabDragTimer.requested) {
                // Every row standing in the strip, not merely open: the carry measures against the tabs' own places,
                // and a row the model has only just gained has none until the next layout.
                if (pageRepeater.count < 2 || topBar.tabItemCount() !== pageRepeater.count)
                    return
                // And the page under the strip settled, so that what is photographed underneath is a repository rather
                // than one still opening. A precondition of the carry and read nowhere else: the carry moves to the tab
                // it takes up, and a page that then has to open would turn this into a wait for something the verb
                // itself caused.
                const front = window.curPage
                if (front === null || front.pageTab.state !== "open"
                        || !front.pageWt.loaded || front.pageGraph.finishCount === 0)
                    return
                const asked = (AppBackend.autoActArg || "3:1").split(":")
                tabDragTimer.from = Number(asked[0])
                tabDragTimer.to = Number(asked[1])
                tabDragTimer.before = topBar.tabPaths()
                // Latched on the strip's answer, the way the middle click is: a carry that found no tab to take up
                // never happened, and reporting it as one would leave the wait to the watchdog.
                tabDragTimer.requested = topBar.dragTabTo(tabDragTimer.from, tabDragTimer.to)
                return
            }
            const paths = topBar.tabPaths()
            if (paths === tabDragTimer.before || topBar.tabItemCount() !== pageRepeater.count)
                return
            stop()
            // The verdict leads, and it is about the tab that was carried: an order that merely changed would pass
            // with any two tabs swapped, and every demo working tree is called the same thing in the picture.
            const was = tabDragTimer.before.split(",")
            AppBackend.report("tab_drag moved="
                              + (paths.split(",")[tabDragTimer.to] === was[tabDragTimer.from])
                              + " from=" + tabDragTimer.from
                              + " to=" + tabDragTimer.to
                              + " tabs=" + pageRepeater.count
                              + " active=" + tabsModel.currentIndex
                              + " open=" + paths)
            window.finishAutoAct()
        }
    }

    // And the half of the carry that ends in no order at all: a tab drawn away from its own row while the hand is
    // still on it. The settled strip photographs the same whether it was ever drawn under the hand or only ever
    // jumped between rows, so the offset the transform is carrying says itself.
    Timer {
        id: tabHoldTimer
        interval: 25
        repeat: true
        running: AppBackend.autoAct === "tab-hold"
        property bool requested: false
        onTriggered: {
            if (!tabHoldTimer.requested) {
                if (pageRepeater.count < 2 || topBar.tabItemCount() !== pageRepeater.count)
                    return
                const front = window.curPage
                if (front === null || front.pageTab.state !== "open"
                        || !front.pageWt.loaded || front.pageGraph.finishCount === 0)
                    return
                tabHoldTimer.requested = topBar.holdTabAt(Number(AppBackend.autoActArg || 0))
                return
            }
            // The tab has to be drawn off its row before there is a picture worth taking; nothing is waited for
            // afterwards, because a hand that has not let go is the whole state.
            const shift = topBar.heldTabShift()
            if (shift === 0)
                return
            stop()
            AppBackend.report("tab_hold lifted=true at=" + (AppBackend.autoActArg || 0)
                              + " shift=" + shift
                              + " active=" + tabsModel.currentIndex
                              + " open=" + topBar.tabPaths())
            window.finishAutoAct()
        }
    }

    // The strip travelling under a tab held past the end of it — the half of the carry that reaches a place which was
    // not on screen when the hand took hold. The window goes down on its floor first: whether a given number of tabs
    // overflows at all is a question about the installed fonts and the band's own furniture (`tab-widths` answers it
    // differently on each OS), and this verb needs a strip that overflows on every machine.
    Timer {
        id: tabEdgeTimer
        interval: 25
        repeat: true
        running: AppBackend.autoAct === "tab-edge"
        property bool sized: false
        property bool requested: false
        property int from: 0
        onTriggered: {
            if (!tabEdgeTimer.requested) {
                if (pageRepeater.count < 2 || topBar.tabItemCount() !== pageRepeater.count)
                    return
                const front = window.curPage
                if (front === null || front.pageTab.state !== "open"
                        || !front.pageWt.loaded || front.pageGraph.finishCount === 0)
                    return
                if (!tabEdgeTimer.sized) {
                    window.width = Math.ceil(window.floorWidth)
                    tabEdgeTimer.sized = true
                    return
                }
                // A strip that fits has no end to travel to. The resize is what makes one, and the strip itself says
                // when that has taken — no width of its own is waited on, because which width crowds a strip is the
                // thing this cannot assume.
                if (!topBar.bandTabScrolls)
                    return
                tabEdgeTimer.from = Number(AppBackend.autoActArg || 0)
                tabEdgeTimer.requested = topBar.carryTabPastEnd(tabEdgeTimer.from)
                return
            }
            // Travelled to its far end, carrying the tab to the end of the order: the strip stops on its own bound,
            // and the tab passes every neighbour that slides under it on the way there.
            if (!topBar.runAtEnd() || topBar.heldTabIndex() !== pageRepeater.count - 1)
                return
            stop()
            topBar.dropCarriedTab()
            AppBackend.report("tab_edge landed=true from=" + tabEdgeTimer.from
                              + " run=" + topBar.runOffset()
                              + " tabs=" + pageRepeater.count
                              + " active=" + tabsModel.currentIndex
                              + " open=" + topBar.tabPaths())
            window.finishAutoAct()
        }
    }

    // Reopening an existing path must select its tab without adding one.
    Timer {
        id: openAgainTimer
        interval: 25
        repeat: true
        running: AppBackend.autoAct === "open-again"
        property bool requested: false
        property string asked: ""
        property int beforeCount: -1
        /// Which tab the ask is meant to bring to the front, where the run left that to the strip. -1 when the run
        /// named a path of its own: which tab holds a folder is the question there, and the whole point is that the
        /// spelling does not say.
        property int wantIndex: -1
        onTriggered: {
            if (!openAgainTimer.requested) {
                if (pageRepeater.count === 0)
                    return
                // The spelling to ask with, when the run named none, is the strip's own — and the strip is a view: the
                // item for a row the model has just gained arrives with the next layout. Asking with the "" it answers
                // until then opens nothing, and nothing opened is what this verb's completion looks like — it went
                // green having asked for nothing at all (measured 2026-08-17).
                const path = AppBackend.autoActArg !== "" ? AppBackend.autoActArg : topBar.tabPathAt(0)
                if (path === "")
                    return
                openAgainTimer.asked = path
                openAgainTimer.wantIndex = AppBackend.autoActArg !== "" ? -1 : 0
                openAgainTimer.beforeCount = pageRepeater.count
                openAgainTimer.requested = true
                tabsModel.openRepositoryPath(path)
                return
            }
            // Reopening the first tab from a strip standing on another one has somewhere to arrive: without that, "went
            // to the tab it already had" and "did nothing whatever" are the same report.
            if (pageRepeater.count !== openAgainTimer.beforeCount
                    || tabsModel.currentIndex < 0
                    || (openAgainTimer.wantIndex >= 0
                        && tabsModel.currentIndex !== openAgainTimer.wantIndex)
                    // The strip is read back below (`middle-close` above).
                    || topBar.tabItemCount() !== pageRepeater.count)
                return
            // And the tab it arrived at has to be showing its repository: which tab came to the front is what the
            // picture carries here, and a page still opening looks the same whichever one it is (`middle-close` above,
            // same baseline).
            const front = window.curPage
            if (front === null || front.pageTab.state !== "open"
                    || !front.pageWt.loaded || front.pageGraph.finishCount === 0)
                return
            stop()
            AppBackend.report("open_again tabs=" + pageRepeater.count
                              + " active=" + tabsModel.currentIndex
                              + " asked=" + openAgainTimer.asked
                              + " open=" + topBar.tabPaths())
            window.finishAutoAct()
        }
    }

    // Capture the communication ring from a real busy edge. TopBar latches the visual only after RepoTab actually
    // enters `push`, so a fast child cannot clear it before the image callback runs.
    Timer {
        interval: 25
        repeat: true
        running: AppBackend.autoAct === "force-push-hold"
        property bool requested: false
        onTriggered: {
            if (topBar.curPage === null || topBar.pushMode !== "diverged")
                return
            if (!requested) {
                requested = true
                topBar.completePushHold()
                return
            }
            if (!topBar.autoPushBusyLatched)
                return
            stop()
            topBar.reportPushBusy()
            window.finishAutoAct()
        }
    }

    // The same edge on the button the wait is drawn for: bare, unframed, and the one a timer can start on its own. The
    // fetch itself is fired by the page's driver; all this waits for is the band's latch.
    Timer {
        interval: 25
        repeat: true
        running: AppBackend.autoAct === "fetch-busy"
        onTriggered: {
            if (!topBar.autoFetchBusyLatched)
                return
            stop()
            topBar.reportFetchBusy()
            window.finishAutoAct()
        }
    }

    // PG_AUTO_ACT=fetch-tip: what the fetch button offers a pointer, on each side of the one thing that decides it.
    // The argument names which side this run is — `off` is the repository with no remote (`--preset noremote`), where
    // the button is dim and says nothing, and `on` is any repository that has one. Neither run proves anything alone.
    //
    // The two wait for different things because "not pressable" has two reasons and only one of them is the subject:
    // the live side waits for the button itself, the dim side for a repository that has finished landing with nothing
    // running on it, so a band read before the remotes arrived cannot pass for either.
    Timer {
        interval: 25
        repeat: true
        running: AppBackend.autoAct === "fetch-tip"
        onTriggered: {
            // The graph as well as the refs, for the picture rather than for the answer: the band settles first, and a
            // half-drawn page under a settled band is a worse photograph of it.
            if (window.curPage === null
                    || !window.curPage.pageRefsLoaded
                    || window.curPage.pageGraph.rowTotal < 1)
                return
            const tab = window.curPage.pageTab
            if (tab.busyCount !== 0 || tab.autoFetchRunning)
                return
            if (AppBackend.autoActArg === "off") {
                if (tab.remoteCount !== 0)
                    return
            } else if (!topBar.fetchLive) {
                return
            }
            stop()
            topBar.reportFetchTip()
            window.finishAutoAct()
        }
    }

    // Smoke hooks (PG_AUTO_ACT=open-not-a-repo / open-bare and the two ways back out). The picker is the platform's own
    // window, so the run enters where its answer lands — the path it accepted.
    readonly property bool pickAct: AppBackend.autoAct === "open-not-a-repo"
                                    || AppBackend.autoAct === "open-bare"
                                    || AppBackend.autoAct === "open-not-a-repo-retry"
                                    || AppBackend.autoAct === "open-not-a-repo-cancel"
    property bool pickStarted: false
    Timer {
        interval: 25
        repeat: true
        running: driver.pickAct
        onTriggered: {
            if (driver.pickStarted || !window.visible)
                return
            driver.pickStarted = true
            tabsModel.openPickedPath(AppBackend.autoActArg)
            pickAnswerTimer.start()
        }
    }
    // Poll the dialog's observable answer. The 25ms cadence is sampling only; it is not a correctness deadline.
    Timer {
        id: pickAnswerTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (!openFailedDialog.opened)
                return
            if (AppBackend.autoAct === "open-not-a-repo-retry")
                openFailedDialog.retry()
            else if (AppBackend.autoAct === "open-not-a-repo-cancel")
                openFailedDialog.close()
            else {
                pickAnswerTimer.stop()
                driver.reportPick()
                window.finishAutoAct()
                return
            }
            pickAnswerTimer.stop()
            pickSettleTimer.start()
        }
    }
    Timer {
        id: pickSettleTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (openFailedDialog.opened)
                return
            pickSettleTimer.stop()
            driver.reportPick()
            window.finishAutoAct()
        }
    }

    // Smoke hooks (PG_AUTO_ACT=open-fail-tab / -bare / -log): the road that keeps its tab. Nothing checks the folder
    // first there, so the page itself is what says so (`kind=` reports which). The `-log` half goes on to open the
    // command log the way the toolbar's `>_` does.
    Timer {
        id: failTabActTimer
        interval: 25
        repeat: true
        running: AppBackend.autoAct === "open-fail-tab"
                 || AppBackend.autoAct === "open-fail-tab-bare"
                 || AppBackend.autoAct === "open-fail-tab-log"
        onTriggered: {
            if (window.curPage === null || window.curPage.pageTab.state !== "open")
                return
            failTabActTimer.stop()
            tabsModel.openRepositoryPath(AppBackend.autoActArg)
            failTabTimer.start()
        }
    }
    Timer {
        id: failTabTimer
        interval: 25
        repeat: true
        property bool commandsRequested: false
        onTriggered: {
            if (window.curPage === null || window.curPage.pageTab.state !== "error"
                    || window.curPage.pageTab.errorKind === "")
                return
            if (AppBackend.autoAct === "open-fail-tab-log" && window.curPage !== null)
                if (!failTabTimer.commandsRequested) {
                    failTabTimer.commandsRequested = true
                    window.curPage.toggleCommands()
                    return
                } else if (!window.curPage.commandsShown) {
                    return
                }
            failTabTimer.stop()
            AppBackend.report(
                "open_fail_tab tabs=" + pageRepeater.count
                + " state=" + (window.curPage !== null ? window.curPage.pageTab.state : "-")
                + " kind=" + (window.curPage !== null ? window.curPage.pageTab.errorKind : "-")
                + " commands=" + (window.curPage !== null ? window.curPage.commandsShown : "-"))
            window.finishAutoAct()
        }
    }
    /// What the run has to show for itself. `dialog=` is the dialog's own `opened` (reporting what was asked of it
    /// would go on passing with the binding cut), and `tabs=` says the refused folder never became one — which is the
    /// whole of what this verb is about.
    function reportPick() {
        AppBackend.report("open_failed kind=" + openFailedDialog.kind
                          + " dialog=" + openFailedDialog.opened
                          + " tabs=" + pageRepeater.count
                          + " active=" + tabsModel.currentIndex
                          + " near=" + openFailedDialog.near)
    }
    // PG_AUTO_ACT=identity / identity-half: "which half landed" is a pair of booleans, and a dialog that stayed open
    // because the save did not take looks exactly like one nobody has answered yet. Read the two verbs as a pair.
    Timer {
        interval: 25
        repeat: true
        running: AppBackend.autoAct === "identity" || AppBackend.autoAct === "identity-half"
        onTriggered: {
            const wholeReady = AppBackend.autoAct === "identity"
                               && AppBackend.identityState === "missing"
                               && identityDialog.opened
            const halfReady = AppBackend.autoAct === "identity-half"
                              && AppBackend.identityState === "ready"
                              && AppBackend.identityUnsaved
                              && identityDialog.opened
            if (!wholeReady && !halfReady)
                return
            stop()
            AppBackend.report(
                "identity state=" + AppBackend.identityState
                + " dialog=" + identityDialog.opened
                + " nameSaved=" + AppBackend.identityNameSaved
                + " emailSaved=" + AppBackend.identityEmailSaved
                + " unsaved=" + AppBackend.identityUnsaved
                + " badge=" + topBar.identityBadgeShown
                + " said=" + (AppBackend.identityError !== ""))
            window.finishAutoAct()
        }
    }

    // PG_AUTO_ACT=identity-tip: the mark's reason, read where the pointer cannot go. `tip=` is the card's own `opened`;
    // `badge=` is the group in whichever shape the width left it — reading the mark alone would fail a band that is
    // saying exactly what it should.
    Timer {
        interval: 25
        repeat: true
        running: AppBackend.autoAct === "identity-tip"
        onTriggered: {
            if (!identityDialog.opened && !AppBackend.identityUnsaved)
                return
            window.dismissIdentity()
            stop()
            identityTipTimer.start()
        }
    }
    Timer {
        id: identityTipTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (identityDialog.opened)
                return
            topBar.statePointedAt = true
            stop()
            identityTipReport.start()
        }
    }
    // The attached card intentionally has a visual tip delay. Completion is still gated by its opened property, never
    // by that duration.
    Timer {
        id: identityTipReport
        interval: 25
        repeat: true
        onTriggered: {
            if (!topBar.stateCardOpen)
                return
            stop()
            AppBackend.report(
                "identity_tip unsaved=" + AppBackend.identityUnsaved
                + " badge=" + (topBar.stateWordsShown || topBar.stateMarkShown)
                + " tip=" + topBar.stateCardOpen
                + " rows=" + topBar.stateCardRows)
            window.finishAutoAct()
        }
    }

    // PG_AUTO_ACT=commands-clear. The band is where the answer is — the rows and the line both feed one mark, and
    // clearing only the rows left it red over an empty panel (2026-08-10 報告), which is why this verb lives up here. The
    // same mark is read on both sides of the press: `was=` is the half the picture cannot hold.
    //
    // The panel the press takes down with them is the other half. Which is why the standing-panel half of the
    // preconditions is read in the branch that presses and nowhere else (規約 §UI 自動化の因果性): every tick, it would be
    // this verb's own answer — the panel gone — barring the way to the report.
    Timer {
        id: commandsClearActTimer
        interval: 25
        repeat: true
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
            if (topBar.commandsWrong || window.curPage.commandsShown)
                return
            stop()
            AppBackend.report(
                "commands_clear was=" + commandsClearActTimer.was
                + " wrong=" + topBar.commandsWrong
                + " open=" + window.curPage.commandsShown
                + " mark=" + topBar.commandsMarkColor)
            window.finishAutoAct()
        }
    }

    // PG_AUTO_ACT=fetch-recover: recovery, not the reader, is what retires fetch news. This waits for the refusal,
    // reads the mark, fires the fetch that can land, and reads the same mark again — the picture can only hold the
    // quiet half.
    Timer {
        id: fetchRecoverActTimer
        interval: 25
        repeat: true
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
            if (topBar.commandsWrong
                    || window.curPage.pageTab.lastError !== ""
                    || window.curPage.pageTab.busyCount !== 0
                    || window.curPage.pageTab.fetchFailures !== 0)
                return
            stop()
            AppBackend.report(
                "fetch_recover was=" + fetchRecoverActTimer.was
                + " hadline=" + fetchRecoverActTimer.hadLine
                + " wrong=" + topBar.commandsWrong
                + " line=" + (window.curPage.pageTab.lastError !== "")
                + " failures=" + window.curPage.pageTab.fetchFailures
                + " open=" + window.curPage.commandsShown)
            window.finishAutoAct()
        }
    }

    // PG_AUTO_ACT=band: numbers rather than a screenshot — the headless platform draws no window buttons of its own, so
    // a band that lost the grab run or pushed its buttons off the end looks fine in the picture.
    Timer {
        id: bandActTimer
        interval: 25
        repeat: true
        running: AppBackend.autoAct === "band"
        onTriggered: {
            // No tabs is a valid laid-out band, not an unanswered one. Read readiness from the window and bar
            // themselves, then let `tabsW=0` describe the empty output.
            if (!window.visible || mainUi.width <= 0 || topBar.width <= 0)
                return
            stop()
            AppBackend.report(
                "band merged=" + window.captionMerged
                + " plain=" + AppBackend.plainChrome
                + " grabRun=" + topBar.bandGrabRun
                + " buttonsX=" + topBar.bandButtonsX
                + " width=" + topBar.width
                + " tabsW=" + topBar.bandTabsWidth
                + " rightMargin=" + topBar.bandRightMargin)
            window.finishAutoAct()
        }
    }

    // PG_AUTO_ACT=tab-widths: numbers for the band's reason — a strip that narrowed the wrong tabs comes out looking
    // like one that got it right. `widths=` is the answer.
    Timer {
        id: tabWidthActTimer
        interval: 25
        repeat: true
        running: AppBackend.autoAct === "tab-widths"
        onTriggered: {
            if (topBar.bandTabCount <= 0 || topBar.bandTabRun <= 0)
                return
            stop()
            AppBackend.report(
                "tab_widths tabs=" + topBar.bandTabCount
                + " run=" + Math.round(topBar.bandTabRun)
                + " cap=" + Math.round(topBar.tabTitleCap)
                + " floor=" + topBar.tabTitleMinW
                + " max=" + topBar.tabTitleMaxW
                + " content=" + Math.round(topBar.bandTabContent)
                + " view=" + Math.round(topBar.bandTabsWidth)
                + " scrolls=" + topBar.bandTabScrolls
                + " widths=" + topBar.tabWidths())
            window.finishAutoAct()
        }
    }

    // PG_AUTO_ACT=tab-mark: the argument is which tab the hand is on — one that is not in front, or the run says
    // nothing the picture of any other verb does not already say.
    Timer {
        id: tabMarkActTimer
        interval: 25
        repeat: true
        running: AppBackend.autoAct === "tab-mark"
        property bool requested: false
        property string beforeMarks: ""
        onTriggered: {
            const pointed = Number(AppBackend.autoActArg || 1)
            if (topBar.bandTabCount < pointed)
                return
            if (!tabMarkActTimer.requested) {
                tabMarkActTimer.requested = true
                tabMarkActTimer.beforeMarks = topBar.tabMarks()
                topBar.pointAtTab(pointed)
                return
            }
            if (topBar.tabMarks() === tabMarkActTimer.beforeMarks)
                return
            stop()
            AppBackend.report(
                "tab_marks tabs=" + topBar.bandTabCount
                + " current=" + tabsModel.currentIndex
                + " pointed=" + pointed
                + " marks=" + topBar.tabMarks())
            window.finishAutoAct()
        }
    }

    // PG_AUTO_ACT=window-fill: whether the window's contents reach all four edges while maximised. Numbers rather than
    // a picture: the app is the whole screen, so there is no desktop left beside it to show a gap against.
    Timer {
        id: fillActTimer
        interval: 25
        repeat: true
        running: AppBackend.autoAct === "window-fill"
        property bool maximizeRequested: false
        onTriggered: {
            if (!fillActTimer.maximizeRequested) {
                fillActTimer.maximizeRequested = true
                window.visibility = Window.Maximized
                return
            }
            if (window.visibility !== Window.Maximized
                    || mainUi.width !== window.width
                    || mainUi.height !== window.height)
                return
            stop()
            fillReportTimer.start()
        }
    }
    // The window has to have taken the state, and the layout to have run inside the new size, before either can be read
    // back.
    Timer {
        id: fillReportTimer
        interval: 25
        repeat: true
        // Measured in scene coordinates rather than from the margins that were asked for, because a margin that misses
        // is exactly what this is looking for.
        onTriggered: {
            const at = mainUi.mapToItem(null, 0, 0)
            if (at.x !== 0 || at.y !== 0 || mainUi.width !== window.width || mainUi.height !== window.height)
                return
            stop()
            AppBackend.report(
                "window_fill fills=" + (at.x === 0 && at.y === 0
                                        && mainUi.width === window.width
                                        && mainUi.height === window.height)
                + " maximized=" + (window.visibility === Window.Maximized)
                + " at=" + at.x + "," + at.y
                + " size=" + mainUi.width + "x" + mainUi.height
                + " window=" + window.width + "x" + window.height)
            // Windowed for the shot, as `state minimize` ends: a picture the shape of the offscreen screen is a picture
            // of the harness.
            window.visibility = Window.Windowed
            window.finishAutoAct()
        }
    }

    // PG_AUTO_ACT=solo: the harness holds the real lock on the config directory before starting this process, so the
    // picture is of the mechanism and not of a flag that imitates it.
    Timer {
        id: soloActTimer
        interval: 25
        repeat: true
        running: AppBackend.autoAct === "solo"
        onTriggered: {
            if (!AppBackend.alreadyRunning || !gate.visible)
                return
            stop()
            AppBackend.report(
                "solo blocked=" + AppBackend.alreadyRunning
                + " held=" + (AppBackend.heldElsewhere !== "")
                + " gate=" + gate.visible
                + " main=" + mainUi.visible)
            window.finishAutoAct()
        }
    }

    // PG_AUTO_ACT=window-floor: (no argument) the shape remembered in the configuration directory, which xtask writes
    // at 320x240 — under every floor there is, so what comes up says whether the way in lifts it fold folded, put down
    // exactly on that floor, then the list put back: the floor rises under a window already standing on it log the same
    // rise the other way — put down on the floor without the log, then the log opened
    Timer {
        id: floorActTimer
        interval: 25
        repeat: true
        running: AppBackend.autoAct === "window-floor"
        property bool shapeRequested: false
        onTriggered: {
            if (window.floorPage === null)
                return
            if (AppBackend.autoActArg === "") {
                if (window.width < window.floorWidth || window.height < window.floorHeight)
                    return
                stop()
                driver.reportFloor()
                return
            }
            if (!floorActTimer.shapeRequested) {
                floorActTimer.shapeRequested = true
                if (AppBackend.autoActArg === "fold")
                    window.floorPage.sidebarCollapsed = true
                else if (AppBackend.autoActArg === "wip")
                    window.floorPage.showWip()
                else if (AppBackend.autoActArg !== "log")
                    return
                return
            }
            if (AppBackend.autoActArg === "fold"
                    && !window.floorPage.sidebarCollapsed)
                return
            if (AppBackend.autoActArg === "wip" && !window.floorPage.wipShown)
                return
            floorShrinkTimer.start()
            stop()
        }
    }
    // Poll until the requested size has actually reached the window floor.
    Timer {
        id: floorShrinkTimer
        interval: 25
        repeat: true
        property bool resized: false
        onTriggered: {
            if (!floorShrinkTimer.resized
                    && (AppBackend.autoActArg === "wip"
                        || window.floorPage.sidebarCollapsed
                        || AppBackend.autoActArg === "log")) {
                window.width = Math.ceil(window.floorWidth)
                window.height = Math.ceil(window.floorHeight)
                driver.floorStoodAt = window.width + "x" + window.height
                floorShrinkTimer.resized = true
                return
            }
            if (!floorShrinkTimer.resized)
                return
            if (window.width < window.floorWidth
                    || window.height < window.floorHeight)
                return
            stop()
            floorRaiseTimer.start()
        }
    }
    /// Automation: where the window was standing before the floor moved under it — a window that came back up cannot
    /// otherwise be told from one that was never let down.
    property string floorStoodAt: ""
    Timer {
        id: floorRaiseTimer
        interval: 25
        repeat: true
        property bool raised: false
        onTriggered: {
            if (!floorRaiseTimer.raised && AppBackend.autoActArg === "fold") {
                window.floorPage.sidebarCollapsed = false
                floorRaiseTimer.raised = true
                return
            }
            if (!floorRaiseTimer.raised && AppBackend.autoActArg === "log") {
                window.floorPage.commandsOpen = true
                floorRaiseTimer.raised = true
                return
            }
            if (AppBackend.autoActArg === "fold"
                    && window.floorPage.sidebarCollapsed)
                return
            if (AppBackend.autoActArg === "log"
                    && !window.floorPage.commandsOpen)
                return
            stop()
            floorReportTimer.start()
        }
    }
    Timer {
        id: floorReportTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (window.width < Math.ceil(window.floorWidth)
                    || window.height < Math.ceil(window.floorHeight))
                return
            if (AppBackend.autoActArg === "fold"
                    && window.floorPage.sidebarCollapsed)
                return
            if (AppBackend.autoActArg === "log"
                    && !window.floorPage.commandsOpen)
                return
            if (AppBackend.autoActArg === "wip"
                    && !window.floorPage.wipBlockScrolls)
                return
            stop()
            driver.reportFloor()
        }
    }
    /// `fits=` is the whole verdict: reporting the floor alone would pass with the window nowhere near it.
    function reportFloor() {
        const floorW = Math.ceil(window.floorWidth)
        const floorH = Math.ceil(window.floorHeight)
        AppBackend.report(
            // The verdict leads: the pair "this verb" and "it held" has to be caught in one substring, and only
            // neighbours can be.
            "window_floor fits="
            + (window.width >= floorW && window.height >= floorH)
            + " floorW=" + floorW + " floorH=" + floorH
            // The two `floorW` is the larger of (`Main.floorWidth`), so which of them set it is read here rather than
            // worked back out of the number: folding the list lowers `pageW` and leaves `bandW` where it is, and on an
            // OS whose band carries the window buttons and the grab runs that is where the two change places.
            + " bandW=" + Math.ceil(topBar.floorWidth)
            + " pageW=" + (window.floorPage !== null
                           ? Math.ceil(window.floorPage.floorWidth) : 0)
            + " w=" + window.width + " h=" + window.height
            + " from=" + (driver.floorStoodAt === "" ? "-" : driver.floorStoodAt)
            // The page the floor was read off, which with no tab open is the blank one — `tabs=0` is the run that
            // proves it counts.
            + " tabs=" + pageRepeater.count
            + " folded=" + (window.floorPage !== null
                            && window.floorPage.sidebarCollapsed)
            + " log=" + (window.floorPage !== null && window.floorPage.commandsOpen)
            // What the right pane made of a height that cannot hold it: scrolling is the answer, and a scroll bar is
            // not something a headless run can see (`wip` shape).
            + " wipScrolls=" + (window.floorPage !== null
                                && window.floorPage.wipBlockScrolls)
            + " detailsOver=" + (window.floorPage !== null
                                 ? window.floorPage.detailsOverHeight : 0))
        window.finishAutoAct()
    }

    // PG_AUTO_ACT=badges: all three of the band's state badges at once — the widest the band ever asks for, and the
    // floor is the only thing between that and a `>_` pushed off the end (デザイン規約 §ウィンドウの縁). The argument is the window
    // width; `floor` puts it down on the floor the three badges leave.
    Timer {
        id: badgesActTimer
        interval: 25
        repeat: true
        running: AppBackend.autoAct === "badges"
                 || AppBackend.autoAct === "badges-hover"
        property bool stateRequested: false
        property bool sizeRequested: false
        property int requestedWidth: -1
        onTriggered: {
            if (identityDialog.opened || topBar.bandTabsWidth <= 0
                    || window.curPage === null || !window.curPage.pageWt.loaded
                    || !topBar.opBadgeShown || !topBar.conflictBadgeShown
                    || !topBar.identityBadgeShown)
                return
            const arg = AppBackend.autoActArg
            const wantedW = parseInt(arg)
            const sized = arg === "floor" || (!isNaN(wantedW) && wantedW > 0)
            // The pointer, where headless cannot put one. Written to the same one property the real hover writes, so
            // the card cannot be opened by a road the hand does not have (app-ui.md).
            if (AppBackend.autoAct === "badges-hover" && !stateRequested) {
                topBar.statePointedAt = true
                stateRequested = true
            }
            if (!badgesActTimer.sizeRequested && arg === "floor") {
                badgesActTimer.requestedWidth = Math.ceil(window.floorWidth)
                window.width = badgesActTimer.requestedWidth
                window.height = Math.ceil(window.floorHeight)
                badgesActTimer.sizeRequested = true
                return
            } else if (!badgesActTimer.sizeRequested && !isNaN(wantedW) && wantedW > 0) {
                // Which width brings on which of the group's three shapes is a question about the installed fonts, so
                // the run names the number and the report says the shape. Not held at the floor: the third shape sits
                // below what a hand can drag to today, and a shape nothing can photograph is a shape nobody can check.
                badgesActTimer.requestedWidth = wantedW
                window.width = badgesActTimer.requestedWidth
                badgesActTimer.sizeRequested = true
                return
            }
            if (sized && (topBar.width !== mainUi.width
                    || (arg === "floor"
                        ? window.width < badgesActTimer.requestedWidth
                        : Math.round(window.width) !== badgesActTimer.requestedWidth)))
                return
            if (AppBackend.autoAct === "badges-hover") {
                if (!topBar.stateCardOpen)
                    return
            }
            stop()
            driver.reportBadges()
        }
    }
    /// Which rule painted the folded group's mark (規約 §状態: 色は最も 重い状態が決める). Read off the band's own colour, not off the
    /// conditions — recomputing the rule here would agree with itself whatever the band did.
    readonly property string stateTint: Qt.colorEqual(topBar.stateMarkColor, Theme.danger) ? "danger" : "warning"
    /// `fits=` leads, and the three badges are judged with it: a run where one never stood photographs a band that was
    /// never crowded.
    function reportBadges() {
        const floorW = Math.ceil(window.floorWidth)
        AppBackend.report(
            "badges fits=" + (window.width >= floorW)
            + " op=" + topBar.opBadgeShown
            + " conflicts=" + topBar.conflictBadgeShown
            + " identity=" + topBar.identityBadgeShown
            + " oldGit=" + topBar.oldGitBadgeShown
            // Which of the group's three shapes landed is `words=` / `mark=`; `cap=` is the width the badges were
            // narrowed to (-1 = none was).
            + " words=" + topBar.stateWordsShown
            + " mark=" + topBar.stateMarkShown
            + " tint=" + driver.stateTint
            + " cap=" + topBar.stateCapW
            + " groupW=" + topBar.stateGroupW
            + " badgeMin=" + topBar.stateBadgeMinW
            + " tabCap=" + Math.round(topBar.tabTitleCap)
            + " tabMin=" + topBar.tabTitleMinW
            + " card=" + topBar.stateCardOpen
            + " rows=" + topBar.stateCardRows
            + " cardSize=" + topBar.stateCardSize
            // The band's own floor beside the window's: reading only the window's would not say whether it was this row
            // that set it.
            + " bandW=" + Math.ceil(topBar.floorWidth)
            + " floorW=" + floorW + " w=" + window.width
            + " tabsW=" + Math.round(topBar.bandTabsWidth)
            + " grabRun=" + Math.round(topBar.bandGrabRun))
        window.finishAutoAct()
    }

    // PG_AUTO_ACT=old-git / old-git-card / old-git-fold. Nothing here stages the state — the run is handed a git that
    // answers `--version` with an older number (`verify-ui --old-git`), so the badge is answering a real reading of a
    // real program.
    Timer {
        id: oldGitActTimer
        interval: 25
        repeat: true
        running: AppBackend.autoAct === "old-git"
                 || AppBackend.autoAct === "old-git-card"
                 || AppBackend.autoAct === "old-git-fold"
        property bool stateRequested: false
        property bool sizeRequested: false
        property int requestedWidth: -1
        onTriggered: {
            // The pointer, where headless cannot put one — the same one property the real hover writes (app-ui.md).
            if (topBar.bandTabsWidth <= 0)
                return
            // `-fold` brings its own width: the shape it is for is a folded group with nothing red in it — the only
            // place the mark's colour is the mark's whole meaning (規約 §状態).
            const arg = AppBackend.autoAct === "old-git-fold"
                        ? "floor" : AppBackend.autoActArg
            const wantedW = parseInt(arg)
            if (!oldGitActTimer.sizeRequested && arg === "floor") {
                oldGitActTimer.requestedWidth = Math.ceil(window.floorWidth)
                window.width = oldGitActTimer.requestedWidth
                window.height = Math.ceil(window.floorHeight)
                oldGitActTimer.sizeRequested = true
                return
            } else if (!oldGitActTimer.sizeRequested && !isNaN(wantedW) && wantedW > 0) {
                oldGitActTimer.requestedWidth = wantedW
                window.width = oldGitActTimer.requestedWidth
                oldGitActTimer.sizeRequested = true
                return
            }
            if (AppBackend.gitVersion === "" || !topBar.oldGitBadgeShown)
                return
            if (oldGitActTimer.sizeRequested
                    && (topBar.width !== mainUi.width
                        || (arg === "floor"
                            ? window.width < oldGitActTimer.requestedWidth
                            : Math.round(window.width)
                              !== oldGitActTimer.requestedWidth)))
                return
            // The card decides whether it has any rows on the pointer edge. Pointing before the version badge exists
            // would ask an empty group once and leave `pointedAt` true, so no later edge could reopen it when the badge
            // arrives.
            if (AppBackend.autoAct === "old-git-card" && !stateRequested) {
                topBar.statePointedAt = true
                stateRequested = true
                return
            }
            if (AppBackend.autoAct === "old-git-card" && !topBar.stateCardOpen)
                return
            stop()
            // `version=` says which git answered — a run whose shim never got onto PATH photographs an ordinary window,
            // and an ordinary window photographs well.
            AppBackend.report(
                "old-git badge=" + topBar.oldGitBadgeShown
                + " card=" + topBar.stateCardOpen
                + " rows=" + topBar.stateCardRows
                + " words=" + topBar.stateWordsShown
                + " mark=" + topBar.stateMarkShown
                + " tint=" + driver.stateTint
                + " cap=" + topBar.stateCapW
                + " version=" + AppBackend.gitVersion
                + " min=" + AppBackend.minimumGit
                + " w=" + window.width)
            window.finishAutoAct()
        }
    }

    // PG_AUTO_ACT=state: two runs sharing one --config-dir are what actually tests this — a single run can only ever
    // agree with itself.
    Timer {
        id: stateActTimer
        interval: 25
        repeat: true
        running: AppBackend.autoAct === "state"
        property bool stateRequested: false
        onTriggered: {
            if (window.curPage === null || window.curPage.pageTab.state !== "open")
                return
            if (!stateActTimer.stateRequested) {
                stateActTimer.stateRequested = true
                if (AppBackend.autoActArg === "change" && window.curPage !== null) {
                    window.curPage.sidebarCollapsed = true
                    window.curPage.commandsOpen = true
                    window.curPage.setDetailsWidth(520)
                    window.curPage.setGraphColumns(190, 300)
                    AppBackend.setAutoFetchMinutes(7)
                }
                if (AppBackend.autoActArg === "minimize")
                    window.visibility = Window.Maximized
                return
            }
            if (AppBackend.autoActArg === "minimize" && window.visibility !== Window.Maximized)
                return
            stop()
            stateReportTimer.start()
        }
    }
    // The splitters have to have taken the new sizes before they can be read back off the panes.
    Timer {
        id: stateReportTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (AppBackend.autoActArg === "minimize" && window.visibility !== Window.Maximized)
                return
            if (AppBackend.autoActArg === "change"
                    && (window.curPage === null
                        || !window.curPage.sidebarCollapsed
                        || !window.curPage.commandsOpen
                        || !window.curPage.commandsShown
                        || Math.abs(window.curPage.stateDetailsWidth - 520) >= 1))
                return
            // Up, then down, with a report from each: what the file holds once the window is down has to be what it
            // held while it was up. Without the first report there is nothing for the second one to leave alone, and
            // the verb passes either way.
            if (AppBackend.autoActArg === "minimize") {
                window.reportState()
                window.visibility = Window.Minimized
            }
            window.reportState()
            AppBackend.report(
                "state tabs=" + pageRepeater.count
                + " active=" + tabsModel.currentIndex
                + " opened=" + (window.curPage !== null ? window.curPage.pageTab.state : "-")
                + " collapsed=" + (window.curPage !== null ? window.curPage.sidebarCollapsed : "-")
                + " sidebar=" + AppBackend.startSidebarWidth()
                + " details=" + AppBackend.startDetailsWidth()
                + " graphLabels=" + AppBackend.startGraphLabelsWidth()
                + " graphLanes=" + AppBackend.startGraphLanesWidth()
                + " commands=" + AppBackend.startCommandsShown()
                + " maximized=" + (window.visibility === Window.Maximized)
                // What the report above left in the store, which is what the next launch comes back to. Read back
                // rather than repeated from the window, so a run that had nothing to say (minimised) is told apart from
                // one that said this.
                + " windowW=" + AppBackend.startWindowWidth()
                + " windowH=" + AppBackend.startWindowHeight()
                + " windowMax=" + AppBackend.startWindowMaximized()
                + " autoFetch=" + AppBackend.autoFetchMinutes)
            // Back up for the shot: `grabToImage` has nothing to hand back from a window that is down. Windowed rather
            // than maximised, so the picture is the size every other verb's is — the offscreen platform maximises to
            // its own 800x800 screen, and a shot that shape is a shot of the harness.
            if (AppBackend.autoActArg === "minimize")
                window.visibility = Window.Windowed
            stop()
            window.finishAutoAct()
        }
    }
}
