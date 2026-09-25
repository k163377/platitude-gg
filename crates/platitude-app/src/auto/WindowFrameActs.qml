pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Window
import platitude
import platitude.ui

/// The window frame's half of the PGG_AUTO_ACT harness: the width and height it can be taken to, the floor it
/// refuses to go under, and the state file it comes back to.
// An `Item` only because `QtObject` has no default property to hold the timers; it is sizeless.
Item {
    id: acts

    required property var window
    required property TabsModel tabsModel
    required property Repeater pageRepeater
    required property TopBar topBar
    required property Item mainUi

    // PGG_AUTO_ACT=window-fill: whether the contents reach all four edges while maximised. Numbers, because a
    // maximised app leaves no desktop beside it to show a gap against.
    SampleTimer {
        id: fillActTimer
        running: Harness.autoAct === "window-fill"
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
    // Read back once the state and the layout inside the new size have both landed.
    SampleTimer {
        id: fillReportTimer
        // In scene coordinates: a margin that misses is what this looks for.
        onTriggered: {
            const at = mainUi.mapToItem(null, 0, 0)
            if (at.x !== 0 || at.y !== 0 || mainUi.width !== window.width || mainUi.height !== window.height)
                return
            stop()
            Harness.report(
                "window_fill fills=" + (at.x === 0 && at.y === 0
                                        && mainUi.width === window.width
                                        && mainUi.height === window.height)
                + " maximized=" + (window.visibility === Window.Maximized)
                + " at=" + at.x + "," + at.y
                + " size=" + mainUi.width + "x" + mainUi.height
                + " window=" + window.width + "x" + window.height)
            // Windowed for the shot, as `state minimize` ends.
            window.visibility = Window.Windowed
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=window-floor [fold|log|wip] (the four forms: verbs.md `window-floor`). With no argument the saved
    // shape is xtask's 320x240, under every floor; `fold` / `log` stand the window on its floor, then put the list
    // back / open the log, so the floor rises under a window already standing on it.
    SampleTimer {
        id: floorActTimer
        running: Harness.autoAct === "window-floor"
        property bool shapeRequested: false
        onTriggered: {
            if (window.floorPage === null)
                return
            if (Harness.autoActArg === "") {
                // The band's marks come off the page's tree (`BandStateGroup`), so a floor read before it is an
                // empty band's. With no repository to open (`--restore`) the floor is the blank page's.
                if (Harness.autoOpen !== ""
                        && (window.curPage === null || !window.curPage.pageWt.loaded))
                    return
                if (window.width < window.floorWidth || window.height < window.floorHeight)
                    return
                // Through the report stage like the other forms: the band's floor is `bandRow.Layout.minimumWidth`,
                // written in the layout's own pass, so a mark just put up is not in it yet.
                stop()
                floorReportTimer.start()
                return
            }
            // Asked of a real page: until a tab opens, `window.floorPage` is the blank one, lost when the real page
            // arrives (`WindowBody.floorPage`). Its tree read too, or the face stands at four zeroes and `wipScrolls=`
            // answers for an empty pane.
            if (window.curPage === null || !window.curPage.pageWt.loaded)
                return
            if (!floorActTimer.shapeRequested) {
                floorActTimer.shapeRequested = true
                if (Harness.autoActArg === "fold")
                    window.floorPage.sidebarCollapsed = true
                else if (Harness.autoActArg === "wip")
                    window.floorPage.showWip()
                else if (Harness.autoActArg !== "log")
                    return
                return
            }
            if (Harness.autoActArg === "fold"
                    && !window.floorPage.sidebarCollapsed)
                return
            if (Harness.autoActArg === "wip" && !window.floorPage.wipShown)
                return
            floorShrinkTimer.start()
            stop()
        }
    }
    SampleTimer {
        id: floorShrinkTimer
        property bool resized: false
        onTriggered: {
            if (!floorShrinkTimer.resized
                    && (Harness.autoActArg === "wip"
                        || window.floorPage.sidebarCollapsed
                        || Harness.autoActArg === "log")) {
                window.width = Math.ceil(window.floorWidth)
                window.height = Math.ceil(window.floorHeight)
                acts.floorStoodAt = window.width + "x" + window.height
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
    /// Where the window stood before the floor moved: one that came back up otherwise looks never let down.
    property string floorStoodAt: ""
    SampleTimer {
        id: floorRaiseTimer
        property bool raised: false
        onTriggered: {
            if (!floorRaiseTimer.raised && Harness.autoActArg === "fold") {
                window.floorPage.sidebarCollapsed = false
                floorRaiseTimer.raised = true
                return
            }
            if (!floorRaiseTimer.raised && Harness.autoActArg === "log") {
                window.floorPage.commandsOpen = true
                floorRaiseTimer.raised = true
                return
            }
            if (Harness.autoActArg === "fold"
                    && window.floorPage.sidebarCollapsed)
                return
            if (Harness.autoActArg === "log"
                    && !window.floorPage.commandsOpen)
                return
            stop()
            floorReportTimer.start()
        }
    }
    SampleTimer {
        id: floorReportTimer
        onTriggered: {
            if (window.width < Math.ceil(window.floorWidth)
                    || window.height < Math.ceil(window.floorHeight))
                return
            if (Harness.autoActArg === "fold"
                    && window.floorPage.sidebarCollapsed)
                return
            if (Harness.autoActArg === "log"
                    && !window.floorPage.commandsOpen)
                return
            // The block scrolls even when not shown, so only `wipShown` catches a face taken away behind the run
            // (`RepoPage.leaveWipWhenDone`).
            if (Harness.autoActArg === "wip"
                    && (!window.floorPage.wipShown
                        || !window.floorPage.wipBlockScrolls))
                return
            stop()
            acts.reportFloor()
        }
    }
    /// `fits=` is the whole verdict: reporting the floor alone would pass with the window nowhere near it.
    function reportFloor() {
        const floorW = Math.ceil(window.floorWidth)
        const floorH = Math.ceil(window.floorHeight)
        Harness.report(
            // The verdict leads: `must_say` catches only neighbours in one substring.
            "window_floor fits="
            + (window.width >= floorW && window.height >= floorH)
            + " floorW=" + floorW + " floorH=" + floorH
            // `floorW` is the larger of these two (`Main.floorWidth`). Folding lowers `pageW` only — where, on an OS
            // whose band carries the window buttons, the two change places.
            + " bandW=" + Math.ceil(topBar.floorWidth)
            + " pageW=" + (window.floorPage !== null
                           ? Math.ceil(window.floorPage.floorWidth) : 0)
            + " w=" + window.width + " h=" + window.height
            + " from=" + (acts.floorStoodAt === "" ? "-" : acts.floorStoodAt)
            // `tabs=0`: the floor was read off the blank page.
            + " tabs=" + pageRepeater.count
            + " folded=" + (window.floorPage !== null
                            && window.floorPage.sidebarCollapsed)
            + " log=" + (window.floorPage !== null && window.floorPage.commandsOpen)
            // Which face the right pane shows, for the two numbers after it.
            + " wip=" + (window.floorPage !== null && window.floorPage.wipShown)
            // A height the pane cannot hold should scroll, and a headless run cannot see a scroll bar.
            + " wipScrolls=" + (window.floorPage !== null
                                && window.floorPage.wipBlockScrolls)
            + " detailsOver=" + (window.floorPage !== null
                                 ? window.floorPage.detailsOverHeight : 0))
        window.finishAutoAct()
    }

    // PGG_AUTO_ACT=state: two runs sharing one --config-dir are what actually tests this — a single run can only ever
    // agree with itself.
    SampleTimer {
        id: stateActTimer
        running: Harness.autoAct === "state"
        property bool stateRequested: false
        onTriggered: {
            if (window.curPage === null || window.curPage.pageTab.state !== "open")
                return
            if (!stateActTimer.stateRequested) {
                stateActTimer.stateRequested = true
                if (Harness.autoActArg === "change" && window.curPage !== null) {
                    window.curPage.sidebarCollapsed = true
                    window.curPage.commandsOpen = true
                    window.curPage.setDetailsWidth(520)
                    window.curPage.setGraphColumns(190, 300)
                    AppBackend.setAutoFetchMinutes(7)
                    AppBackend.setInitialCommits(3000)
                }
                if (Harness.autoActArg === "minimize")
                    window.visibility = Window.Maximized
                return
            }
            if (Harness.autoActArg === "minimize" && window.visibility !== Window.Maximized)
                return
            stop()
            stateReportTimer.start()
        }
    }
    // Read back once the splitters have taken the new sizes.
    SampleTimer {
        id: stateReportTimer
        onTriggered: {
            if (Harness.autoActArg === "minimize" && window.visibility !== Window.Maximized)
                return
            if (Harness.autoActArg === "change"
                    && (window.curPage === null
                        || !window.curPage.sidebarCollapsed
                        || !window.curPage.commandsOpen
                        || !window.curPage.commandsShown
                        || Math.abs(window.curPage.stateDetailsWidth - 520) >= 1))
                return
            // Reported up, then down: the file must hold the same once the window is down, which one report cannot
            // show.
            if (Harness.autoActArg === "minimize") {
                window.reportState()
                window.visibility = Window.Minimized
            }
            window.reportState()
            Harness.report(
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
                // Read back from the store the next launch comes back to, so a minimised run that said nothing is
                // told apart.
                + " windowW=" + AppBackend.startWindowWidth()
                + " windowH=" + AppBackend.startWindowHeight()
                + " windowMax=" + AppBackend.startWindowMaximized()
                + " autoFetch=" + AppBackend.autoFetchMinutes
                + " initialCommits=" + AppBackend.initialCommits)
            // Back up for the shot (`grabToImage` gets nothing from a window that is down), and windowed: the
            // offscreen platform maximises to its own 800x800 screen.
            if (Harness.autoActArg === "minimize")
                window.visibility = Window.Windowed
            stop()
            window.finishAutoAct()
        }
    }
}
