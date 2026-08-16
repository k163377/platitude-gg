pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Window
import platitude
import platitude.ui

/// The window's half of the PG_AUTO_ACT harness: the verbs that answer
/// for the window itself — its floor, its band, the state file it comes
/// back to — and for the picker, which is the platform's own window and
/// can only be entered where its answer lands.
///
/// `Main` builds this only when a verb was given, so an ordinary run
/// carries none of it. What the verbs act on is handed in below: a file
/// of its own cannot see the window's ids, and naming them in one list is
/// what says how far the harness reaches into the window.
// An `Item` only because `QtObject` has no default property to hold the
// timers below; it draws nothing and is never given a size.
Item {
    id: driver

    /// The window these verbs act on, and the parts of it they read back
    /// or leave standing for the shot. An automation-only exposure, the
    /// same one `GraphPane.view` is (app-ui.md). `var` because `Main` is
    /// the file the engine loads rather than a type anything can name.
    property var window

    property TabsModel tabsModel
    property Repeater pageRepeater
    property TopBar topBar
    property ColumnLayout mainUi
    property Item gate
    property OpenFailedDialog openFailedDialog
    property IdentityDialog identityDialog

    /// Kicked off by the window once its tabs are open: a verb that ran
    /// before them would answer for a window holding nothing. The verbs
    /// missing from this list start themselves — theirs is a `running:`
    /// that is true from the moment this is built.
    function begin() {
        if (AppBackend.autoAct === "state")
            stateActTimer.start()
        if (AppBackend.autoAct === "commands-clear")
            commandsClearActTimer.start()
        if (AppBackend.autoAct === "fetch-recover")
            fetchRecoverActTimer.start()
        if (AppBackend.autoAct === "band")
            bandActTimer.start()
        if (AppBackend.autoAct === "tab-widths")
            tabWidthActTimer.start()
        if (AppBackend.autoAct === "tab-mark")
            tabMarkActTimer.start()
        if (AppBackend.autoAct === "window-fill")
            fillActTimer.start()
        if (AppBackend.autoAct === "solo")
            soloActTimer.start()
    }

    // Smoke hooks (PG_AUTO_ACT=open-not-a-repo / open-bare and the two
    // ways back out). The picker is the platform's own window, so the run
    // enters where its answer lands — the path it accepted.
    readonly property bool pickAct: AppBackend.autoAct === "open-not-a-repo"
                                    || AppBackend.autoAct === "open-bare"
                                    || AppBackend.autoAct === "open-not-a-repo-retry"
                                    || AppBackend.autoAct === "open-not-a-repo-cancel"
    Timer {
        interval: 1200
        running: driver.pickAct
        onTriggered: {
            tabsModel.openPickedPath(AppBackend.autoActArg)
            pickAnswerTimer.start()
        }
    }
    // The answer is one `git rev-parse` away (実測 30–36ms on Windows,
    // repository or not), so this is a beat rather than a wait.
    Timer {
        id: pickAnswerTimer
        interval: 400
        onTriggered: {
            if (AppBackend.autoAct === "open-not-a-repo-retry")
                openFailedDialog.retry()
            else if (AppBackend.autoAct === "open-not-a-repo-cancel")
                openFailedDialog.close()
            else {
                driver.reportPick()
                return
            }
            // Both ways out end the dialog, and a closing popup is still
            // `opened` for a frame or two — the answer this reads is
            // whether it went, so it is read after it has had the time.
            pickSettleTimer.start()
        }
    }
    Timer {
        id: pickSettleTimer
        interval: 300
        onTriggered: driver.reportPick()
    }

    // Smoke hooks (PG_AUTO_ACT=open-fail-tab / -bare / -log): the road
    // that keeps its tab. Nothing checks the folder first there, so the
    // page itself is what says so (`kind=` reports which). The `-log`
    // half goes on to open the command log the way the toolbar's `>_`
    // does.
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
        onTriggered: {
            if (window.curPage === null || window.curPage.pageTab.state !== "error"
                    || window.curPage.pageTab.errorKind === "")
                return
            failTabTimer.stop()
            if (AppBackend.autoAct === "open-fail-tab-log" && window.curPage !== null)
                window.curPage.toggleCommands()
            AppBackend.report(
                "open_fail_tab tabs=" + pageRepeater.count
                + " state=" + (window.curPage !== null ? window.curPage.pageTab.state : "-")
                + " kind=" + (window.curPage !== null ? window.curPage.pageTab.errorKind : "-")
                + " commands=" + (window.curPage !== null ? window.curPage.commandsShown : "-"))
            window.finishAutoAct()
        }
    }
    /// What the run has to show for itself. `dialog=` is the dialog's own
    /// `opened` (reporting what was asked of it would go on passing with
    /// the binding cut), and `tabs=` says the refused folder never became
    /// one — which is the whole of what this verb is about.
    function reportPick() {
        AppBackend.report("open_failed kind=" + openFailedDialog.kind
                          + " dialog=" + openFailedDialog.opened
                          + " tabs=" + pageRepeater.count
                          + " active=" + tabsModel.currentIndex
                          + " near=" + openFailedDialog.near)
    }
    // PG_AUTO_ACT=identity / identity-half: "which half landed" is a pair
    // of booleans, and a dialog that stayed open because the save did not
    // take looks exactly like one nobody has answered yet. Read the two
    // verbs as a pair.
    Timer {
        interval: 1200
        running: AppBackend.autoAct === "identity"
                 || AppBackend.autoAct === "identity-half"
        onTriggered: AppBackend.report(
            "identity state=" + AppBackend.identityState
            + " dialog=" + identityDialog.opened
            + " nameSaved=" + AppBackend.identityNameSaved
            + " emailSaved=" + AppBackend.identityEmailSaved
            + " unsaved=" + AppBackend.identityUnsaved
            + " badge=" + topBar.identityBadgeShown
            + " said=" + (AppBackend.identityError !== ""))
    }

    // PG_AUTO_ACT=identity-tip: the mark's reason, read where the pointer
    // cannot go. `tip=` is the card's own `opened`; `badge=` is the group
    // in whichever shape the width left it — reading the mark alone would
    // fail a band that is saying exactly what it should.
    Timer {
        interval: 1600
        running: AppBackend.autoAct === "identity-tip"
        onTriggered: {
            window.dismissIdentity()
            identityTipTimer.start()
        }
    }
    Timer {
        id: identityTipTimer
        interval: 400
        onTriggered: {
            topBar.statePointedAt = true
            identityTipReport.start()
        }
    }
    // Past Metrics.tipDelayMs, so what is reported is what is on screen.
    Timer {
        id: identityTipReport
        interval: 800
        onTriggered: AppBackend.report(
            "identity_tip unsaved=" + AppBackend.identityUnsaved
            + " badge=" + (topBar.stateWordsShown || topBar.stateMarkShown)
            + " tip=" + topBar.stateCardOpen
            + " rows=" + topBar.stateCardRows)
    }

    // PG_AUTO_ACT=commands-clear. The band is where the answer is — the
    // rows and the line both feed one mark, and clearing only the rows
    // left it red over an empty panel (2026-08-10 報告), which is why
    // this verb lives up here. The same mark is read on both sides of the
    // press: `was=` is the half the picture cannot hold.
    Timer {
        id: commandsClearActTimer
        interval: 2400
        onTriggered: {
            const was = topBar.commandsWrong
            if (window.curPage !== null)
                window.curPage.clearCommandLog()
            AppBackend.report(
                "commands_clear was=" + was
                + " wrong=" + topBar.commandsWrong
                + " mark=" + topBar.commandsMarkColor
                + " open=" + (window.curPage !== null
                              && window.curPage.commandsShown))
        }
    }

    // PG_AUTO_ACT=fetch-recover: recovery, not the reader, is what
    // retires fetch news. This waits for the refusal, reads the mark,
    // fires the fetch that can land, and reads the same mark again — the
    // picture can only hold the quiet half.
    Timer {
        id: fetchRecoverActTimer
        interval: 3000
        onTriggered: {
            fetchRecoverReport.was = topBar.commandsWrong
            fetchRecoverReport.hadLine = window.curPage !== null
                && window.curPage.pageTab.lastError !== ""
            if (window.curPage !== null)
                window.curPage.pageTab.fetch("")
            fetchRecoverReport.start()
        }
    }
    Timer {
        id: fetchRecoverReport
        interval: 1600
        property bool was: false
        property bool hadLine: false
        onTriggered: AppBackend.report(
            "fetch_recover was=" + was
            + " hadline=" + hadLine
            + " wrong=" + topBar.commandsWrong
            + " line=" + (window.curPage !== null
                          && window.curPage.pageTab.lastError !== "")
            + " failures=" + (window.curPage !== null
                              ? window.curPage.pageTab.fetchFailures : -1)
            + " open=" + (window.curPage !== null
                          && window.curPage.commandsShown))
    }

    // PG_AUTO_ACT=band: numbers rather than a screenshot — the headless
    // platform draws no window buttons of its own, so a band that lost
    // the grab run or pushed its buttons off the end looks fine in the
    // picture.
    Timer {
        id: bandActTimer
        interval: 1200
        onTriggered: AppBackend.report(
            "band merged=" + window.captionMerged
            + " plain=" + AppBackend.plainChrome
            + " grabRun=" + topBar.bandGrabRun
            + " buttonsX=" + topBar.bandButtonsX
            + " width=" + topBar.width
            + " tabsW=" + topBar.bandTabsWidth
            + " rightMargin=" + topBar.bandRightMargin)
    }

    // PG_AUTO_ACT=tab-widths: numbers for the band's reason — a strip
    // that narrowed the wrong tabs comes out looking like one that got it
    // right. `widths=` is the answer.
    Timer {
        id: tabWidthActTimer
        interval: 1200
        onTriggered: AppBackend.report(
            "tab_widths tabs=" + topBar.bandTabCount
            + " run=" + Math.round(topBar.bandTabRun)
            + " cap=" + Math.round(topBar.tabTitleCap)
            + " floor=" + topBar.tabTitleMinW
            + " max=" + topBar.tabTitleMaxW
            + " content=" + Math.round(topBar.bandTabContent)
            + " view=" + Math.round(topBar.bandTabsWidth)
            + " scrolls=" + topBar.bandTabScrolls
            + " widths=" + topBar.tabWidths())
    }

    // PG_AUTO_ACT=tab-mark: the argument is which tab the hand is on —
    // one that is not in front, or the run says nothing the picture of
    // any other verb does not already say.
    Timer {
        id: tabMarkActTimer
        interval: 1200
        onTriggered: {
            topBar.pointAtTab(Number(AppBackend.autoActArg || 1))
            AppBackend.report(
                "tab_marks tabs=" + topBar.bandTabCount
                + " current=" + tabsModel.currentIndex
                + " pointed=" + Number(AppBackend.autoActArg || 1)
                + " marks=" + topBar.tabMarks())
        }
    }

    // PG_AUTO_ACT=window-fill: whether the window's contents reach all
    // four edges while maximised. Numbers rather than a picture: the app
    // is the whole screen, so there is no desktop left beside it to show
    // a gap against.
    Timer {
        id: fillActTimer
        interval: 1200
        onTriggered: {
            window.visibility = Window.Maximized
            fillReportTimer.start()
        }
    }
    // The window has to have taken the state, and the layout to have run
    // inside the new size, before either can be read back.
    Timer {
        id: fillReportTimer
        interval: 400
        // Measured in scene coordinates rather than from the margins that
        // were asked for, because a margin that misses is exactly what
        // this is looking for.
        onTriggered: {
            const at = mainUi.mapToItem(null, 0, 0)
            AppBackend.report(
                "window_fill fills=" + (at.x === 0 && at.y === 0
                                        && mainUi.width === window.width
                                        && mainUi.height === window.height)
                + " maximized=" + (window.visibility === Window.Maximized)
                + " at=" + at.x + "," + at.y
                + " size=" + mainUi.width + "x" + mainUi.height
                + " window=" + window.width + "x" + window.height)
            // Windowed for the shot, as `state minimize` ends: a picture
            // the shape of the offscreen screen is a picture of the
            // harness.
            window.visibility = Window.Windowed
        }
    }

    // PG_AUTO_ACT=solo: the harness holds the real lock on the config
    // directory before starting this process, so the picture is of the
    // mechanism and not of a flag that imitates it.
    Timer {
        id: soloActTimer
        interval: 1200
        onTriggered: AppBackend.report(
            "solo blocked=" + AppBackend.alreadyRunning
            + " held=" + (AppBackend.heldElsewhere !== "")
            + " gate=" + gate.visible
            + " main=" + mainUi.visible)
    }

    // PG_AUTO_ACT=window-floor:
    //   (no argument)  the shape remembered in the configuration
    //                  directory, which xtask writes at 320x240 — under
    //                  every floor there is, so what comes up says
    //                  whether the way in lifts it
    //   fold           folded, put down exactly on that floor, then the
    //                  list put back: the floor rises under a window
    //                  already standing on it
    //   log            the same rise the other way — put down on the
    //                  floor without the log, then the log opened
    Timer {
        id: floorActTimer
        interval: 1200
        running: AppBackend.autoAct === "window-floor"
        onTriggered: {
            if (window.floorPage === null || AppBackend.autoActArg === "") {
                driver.reportFloor()
                return
            }
            if (AppBackend.autoActArg === "fold")
                window.floorPage.sidebarCollapsed = true
            else if (AppBackend.autoActArg === "wip")
                window.floorPage.showWip()
            floorShrinkTimer.start()
        }
    }
    // A beat apart from the fold above, which has to have reached the
    // layout before the floor it leaves can be read off.
    Timer {
        id: floorShrinkTimer
        interval: Metrics.anchorDelayMs
        onTriggered: {
            window.width = Math.ceil(window.floorWidth)
            window.height = Math.ceil(window.floorHeight)
            driver.floorStoodAt = window.width + "x" + window.height
            floorRaiseTimer.start()
        }
    }
    /// Automation: where the window was standing before the floor moved
    /// under it — a window that came back up cannot otherwise be told
    /// from one that was never let down.
    property string floorStoodAt: ""
    Timer {
        id: floorRaiseTimer
        interval: Metrics.anchorDelayMs
        onTriggered: {
            if (AppBackend.autoActArg === "fold")
                window.floorPage.sidebarCollapsed = false
            else if (AppBackend.autoActArg === "log")
                window.floorPage.commandsOpen = true
            floorReportTimer.start()
        }
    }
    Timer {
        id: floorReportTimer
        interval: Metrics.anchorDelayMs
        onTriggered: driver.reportFloor()
    }
    /// `fits=` is the whole verdict: reporting the floor alone would pass
    /// with the window nowhere near it.
    function reportFloor() {
        const floorW = Math.ceil(window.floorWidth)
        const floorH = Math.ceil(window.floorHeight)
        AppBackend.report(
            // The verdict leads: the pair "this verb" and "it held" has
            // to be caught in one substring, and only neighbours can be.
            "window_floor fits="
            + (window.width >= floorW && window.height >= floorH)
            + " floorW=" + floorW + " floorH=" + floorH
            + " w=" + window.width + " h=" + window.height
            + " from=" + (driver.floorStoodAt === "" ? "-" : driver.floorStoodAt)
            // The page the floor was read off, which with no tab open is
            // the blank one — `tabs=0` is the run that proves it counts.
            + " tabs=" + pageRepeater.count
            + " folded=" + (window.floorPage !== null
                            && window.floorPage.sidebarCollapsed)
            + " log=" + (window.floorPage !== null && window.floorPage.commandsOpen)
            // What the right pane made of a height that cannot hold it:
            // scrolling is the answer, and a scroll bar is not something
            // a headless run can see (`wip` shape).
            + " wipScrolls=" + (window.floorPage !== null
                                && window.floorPage.wipBlockScrolls)
            + " detailsOver=" + (window.floorPage !== null
                                 ? window.floorPage.detailsOverHeight : 0))
    }

    // PG_AUTO_ACT=badges: all three of the band's state badges at once —
    // the widest the band ever asks for, and the floor is the only thing
    // between that and a `>_` pushed off the end (デザイン規約
    // §ウィンドウの縁). The argument is the window width; `floor` puts it
    // down on the floor the three badges leave.
    Timer {
        id: badgesActTimer
        interval: 1200
        running: AppBackend.autoAct === "badges"
                 || AppBackend.autoAct === "badges-hover"
        onTriggered: {
            const arg = AppBackend.autoActArg
            const wantedW = parseInt(arg)
            const sized = arg === "floor" || (!isNaN(wantedW) && wantedW > 0)
            // The pointer, where headless cannot put one. Written to the
            // same one property the real hover writes, so the card cannot
            // be opened by a road the hand does not have (app-ui.md).
            if (AppBackend.autoAct === "badges-hover")
                topBar.statePointedAt = true
            if (arg === "floor") {
                window.width = Math.ceil(window.floorWidth)
                window.height = Math.ceil(window.floorHeight)
            } else if (!isNaN(wantedW) && wantedW > 0) {
                // Which width brings on which of the group's three shapes
                // is a question about the installed fonts, so the run
                // names the number and the report says the shape. Not
                // held at the floor: the third shape sits below what a
                // hand can drag to today, and a shape nothing can
                // photograph is a shape nobody can check.
                window.width = wantedW
            }
            // A beat later if anything was moved or opened; straight away
            // if this run is only reading the band as it stands.
            if (sized || AppBackend.autoAct === "badges-hover") {
                badgesReportTimer.start()
                return
            }
            driver.reportBadges()
        }
    }
    // A beat after the shrink, for the same reason the floor verb waits:
    // what is being read is where the layout came to rest, not what it
    // was asked for.
    Timer {
        id: badgesReportTimer
        interval: Metrics.anchorDelayMs
        onTriggered: driver.reportBadges()
    }
    /// Which rule painted the folded group's mark (規約 §状態: 色は最も
    /// 重い状態が決める). Read off the band's own colour, not off the
    /// conditions — recomputing the rule here would agree with itself
    /// whatever the band did.
    readonly property string stateTint:
        Qt.colorEqual(topBar.stateMarkColor, Theme.danger)
        ? "danger" : "warning"
    /// `fits=` leads, and the three badges are judged with it: a run
    /// where one never stood photographs a band that was never crowded.
    function reportBadges() {
        const floorW = Math.ceil(window.floorWidth)
        AppBackend.report(
            "badges fits=" + (window.width >= floorW)
            + " op=" + topBar.opBadgeShown
            + " conflicts=" + topBar.conflictBadgeShown
            + " identity=" + topBar.identityBadgeShown
            + " oldGit=" + topBar.oldGitBadgeShown
            // Which of the group's three shapes landed is `words=` /
            // `mark=`; `cap=` is the width the badges were narrowed to
            // (-1 = none was).
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
            // The band's own floor beside the window's: reading only the
            // window's would not say whether it was this row that set it.
            + " bandW=" + Math.ceil(topBar.floorWidth)
            + " floorW=" + floorW + " w=" + window.width
            + " tabsW=" + Math.round(topBar.bandTabsWidth)
            + " grabRun=" + Math.round(topBar.bandGrabRun))
    }

    // PG_AUTO_ACT=old-git / old-git-card / old-git-fold. Nothing here
    // stages the state — the run is handed a git that answers
    // `--version` with an older number (`verify-ui --old-git`), so the
    // badge is answering a real reading of a real program.
    Timer {
        id: oldGitActTimer
        interval: 1200
        running: AppBackend.autoAct === "old-git"
                 || AppBackend.autoAct === "old-git-card"
                 || AppBackend.autoAct === "old-git-fold"
        onTriggered: {
            // The pointer, where headless cannot put one — the same one
            // property the real hover writes (app-ui.md).
            if (AppBackend.autoAct === "old-git-card")
                topBar.statePointedAt = true
            // `-fold` brings its own width: the shape it is for is a
            // folded group with nothing red in it — the only place the
            // mark's colour is the mark's whole meaning (規約 §状態).
            const arg = AppBackend.autoAct === "old-git-fold"
                        ? "floor" : AppBackend.autoActArg
            const wantedW = parseInt(arg)
            if (arg === "floor") {
                window.width = Math.ceil(window.floorWidth)
                window.height = Math.ceil(window.floorHeight)
            } else if (!isNaN(wantedW) && wantedW > 0) {
                window.width = wantedW
            }
            oldGitReportTimer.start()
        }
    }
    Timer {
        id: oldGitReportTimer
        interval: Metrics.anchorDelayMs
        onTriggered: AppBackend.report(
            // `version=` says which git answered — a run whose shim never
            // got onto PATH photographs an ordinary window, and an
            // ordinary window photographs well.
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
    }

    // PG_AUTO_ACT=state: two runs sharing one --config-dir are what
    // actually tests this — a single run can only ever agree with itself.
    Timer {
        id: stateActTimer
        interval: 1200
        onTriggered: {
            if (AppBackend.autoActArg === "change" && window.curPage !== null) {
                window.curPage.sidebarCollapsed = true
                window.curPage.commandsOpen = true
                window.curPage.setDetailsWidth(520)
                window.curPage.setGraphColumns(190, 300)
                // The other file: a decision, written out at once rather
                // than on the state timer.
                AppBackend.setAutoFetchMinutes(7)
            }
            // A run that ends with the window down. Up first, because the
            // window whose numbers go wrong while it is minimised is the
            // one that was maximised.
            if (AppBackend.autoActArg === "minimize")
                window.visibility = Window.Maximized
            stateReportTimer.start()
        }
    }
    // The splitters have to have taken the new sizes before they can be
    // read back off the panes.
    Timer {
        id: stateReportTimer
        interval: 400
        onTriggered: {
            // Up, then down, with a report from each: what the file holds
            // once the window is down has to be what it held while it was
            // up. Without the first report there is nothing for the
            // second one to leave alone, and the verb passes either way.
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
                // What the report above left in the store, which is what
                // the next launch comes back to. Read back rather than
                // repeated from the window, so a run that had nothing to
                // say (minimised) is told apart from one that said this.
                + " windowW=" + AppBackend.startWindowWidth()
                + " windowH=" + AppBackend.startWindowHeight()
                + " windowMax=" + AppBackend.startWindowMaximized()
                + " autoFetch=" + AppBackend.autoFetchMinutes)
            // Back up for the shot: `grabToImage` has nothing to hand
            // back from a window that is down. Windowed rather than
            // maximised, so the picture is the size every other verb's
            // is — the offscreen platform maximises to its own 800x800
            // screen, and a shot that shape is a shot of the harness.
            if (AppBackend.autoActArg === "minimize")
                window.visibility = Window.Windowed
        }
    }
}
