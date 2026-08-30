pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Window
import platitude
import platitude.ui

/// The window frame's half of the PG_AUTO_ACT harness: the width and height it can be taken to, the floor it
/// refuses to go under, and the state file it comes back to.
///
/// Built by `WindowAutoActDriver`, which is what `Main` builds when a verb was given; what these verbs act
/// on is handed down below, one property per part of the window they reach into.
// An `Item` only because `QtObject` has no default property to hold the timers below; it draws nothing and is
// never given a size.
Item {
    id: acts

    required property var window
    required property TabsModel tabsModel
    required property Repeater pageRepeater
    required property TopBar topBar
    required property Item mainUi

    // PG_AUTO_ACT=window-fill: whether the window's contents reach all four edges while maximised. Numbers rather than
    // a picture: the app is the whole screen, so there is no desktop left beside it to show a gap against.
    SampleTimer {
        id: fillActTimer
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
    SampleTimer {
        id: fillReportTimer
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

    // PG_AUTO_ACT=window-floor: (no argument) the shape remembered in the configuration directory, which xtask writes
    // at 320x240 — under every floor there is, so what comes up says whether the way in lifts it fold folded, put down
    // exactly on that floor, then the list put back: the floor rises under a window already standing on it log the same
    // rise the other way — put down on the floor without the log, then the log opened
    SampleTimer {
        id: floorActTimer
        running: AppBackend.autoAct === "window-floor"
        property bool shapeRequested: false
        onTriggered: {
            if (window.floorPage === null)
                return
            if (AppBackend.autoActArg === "") {
                if (window.width < window.floorWidth || window.height < window.floorHeight)
                    return
                stop()
                acts.reportFloor()
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
    SampleTimer {
        id: floorShrinkTimer
        property bool resized: false
        onTriggered: {
            if (!floorShrinkTimer.resized
                    && (AppBackend.autoActArg === "wip"
                        || window.floorPage.sidebarCollapsed
                        || AppBackend.autoActArg === "log")) {
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
    /// Automation: where the window was standing before the floor moved under it — a window that came back up cannot
    /// otherwise be told from one that was never let down.
    property string floorStoodAt: ""
    SampleTimer {
        id: floorRaiseTimer
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
    SampleTimer {
        id: floorReportTimer
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
            acts.reportFloor()
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
            + " from=" + (acts.floorStoodAt === "" ? "-" : acts.floorStoodAt)
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

    // PG_AUTO_ACT=state: two runs sharing one --config-dir are what actually tests this — a single run can only ever
    // agree with itself.
    SampleTimer {
        id: stateActTimer
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
                    AppBackend.setInitialCommits(3000)
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
    SampleTimer {
        id: stateReportTimer
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
                + " autoFetch=" + AppBackend.autoFetchMinutes
                + " initialCommits=" + AppBackend.initialCommits)
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
