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
    // maximised app leaves no desktop beside it to show a gap against. Windowed again for the shot, as `state minimize`
    // ends: the maximised window is read in the press that puts it back, the one moment it still stands.
    WindowStage {
        id: fillActTimer
        name: "window_fill"
        order: ["maximized", "filled", "windowed"]
        running: Harness.autoAct === "window-fill"
        /// The maximised window as it was read before it was put back (`fillLine`).
        property string filledLine: ""
        onTriggered: {
            const op = fillActTimer
            if (op.at("start")) {
                op.input("maximized", () => {
                    window.visibility = Window.Maximized
                })
                return
            }
            if (op.at("maximized")) {
                if (!Awaited.all("window_fill_maximized", {
                        "maximized": window.visibility === Window.Maximized,
                        "filled": mainUi.width === window.width && mainUi.height === window.height }))
                    return
                op.advance("filled")
                return
            }
            // Read back once the state and the layout inside the new size have both landed. In scene coordinates: a
            // margin that misses is what this looks for.
            const at = mainUi.mapToItem(null, 0, 0)
            if (!Awaited.all("window_fill_report_ready", {
                    "placed": at.x === 0 && at.y === 0,
                    "filled": mainUi.width === window.width && mainUi.height === window.height,
                    "page": PageSettled.windowSettled(window) }))
                return
            if (!op.input("windowed", () => {
                    op.filledLine = acts.fillLine()
                    window.visibility = Window.Windowed
                }))
                return
            op.complete(() => op.filledLine)
        }
    }
    /// The maximised window: whether its contents reach the four edges, in scene coordinates.
    function fillLine() {
        const at = mainUi.mapToItem(null, 0, 0)
        return "window_fill fills=" + (at.x === 0 && at.y === 0
                                       && mainUi.width === window.width
                                       && mainUi.height === window.height)
            + " maximized=" + (window.visibility === Window.Maximized)
            + " at=" + at.x + "," + at.y
            + " size=" + mainUi.width + "x" + mainUi.height
            + " window=" + window.width + "x" + window.height
    }

    // PGG_AUTO_ACT=window-floor [fold|log|wip|rail] (the five forms: verbs.md `window-floor`). With no argument the
    // saved shape is xtask's 320x240, under every floor; `fold` / `log` stand the window on its floor, then put the
    // list back / open the log — the width's floor rises under a window already standing on it, the height's holds
    // still (デザイン規約 §窓の床). `rail` folds, stands on the floor and opens the log: the log comes up as far as the
    // panes let it, over the rail's last cell unless they hold it.
    //
    // The stages: the face asked for (`shaped`), the window stood on its floor (`shrunk`) and the menu read there
    // (`floored`), then the floor moved under it (`raised`, landed as `risen`); `wip` moves nothing after the menu's
    // reading. With no argument the floor is read where it stands (`blank`).
    WindowStage {
        id: floorActTimer
        name: "window_floor"
        order: Harness.autoActArg === "" ? ["blank"]
             : Harness.autoActArg === "wip" ? ["shaped", "shrunk", "floored"]
             : ["shaped", "shrunk", "floored", "raised", "risen"]
        running: Harness.autoAct === "window-floor"
        onTriggered: {
            const op = floorActTimer
            const arg = Harness.autoActArg
            const folds = arg === "fold" || arg === "rail"
            const logs = arg === "log" || arg === "rail"
            const floorPage = window.floorPage
            if (op.at("start") && arg === "") {
                // The band's marks come off the page's tree (`BandStateGroup`), so a floor read before it is an
                // empty band's. With no repository to open (`--restore`) the floor is the blank page's.
                if (!Awaited.all("window_floor_blank", {
                        "floorPage": floorPage !== null,
                        "page": Harness.autoOpen === ""
                            || PageSettled.settledUnder(window),
                        "floor": window.width >= window.floorWidth && window.height >= window.floorHeight }))
                    return
                // Through the report stage like the other forms: the band's floor is `bandRow.Layout.minimumWidth`,
                // written in the layout's own pass, so a mark just put up is not in it yet.
                op.advance("blank")
                return
            }
            if (op.at("start") || op.at("shaped")) {
                // Asked of a real page: until a tab opens, `window.floorPage` is the blank one, lost when the real page
                // arrives (`WindowBody.floorPage`). Its tree read too, or the face stands at four zeroes and
                // `wipScrolls=` answers for an empty pane.
                const shaped = op.at("shaped")
                if (!Awaited.all("window_floor_page", {
                        "floorPage": floorPage !== null,
                        "page": PageSettled.settledUnder(window),
                        "folded": !shaped || !folds || (floorPage !== null && floorPage.sidebarCollapsed),
                        "wip": !shaped || arg !== "wip" || (floorPage !== null && floorPage.wipShown) }))
                    return
                if (!shaped) {
                    op.input("shaped", () => {
                        if (folds)
                            floorPage.sidebarCollapsed = true
                        else if (arg === "wip")
                            floorPage.showWip()
                    })
                    return
                }
                // The face asked for stands: the window goes down to its floor, as far as a hand could drag it.
                op.input("shrunk", () => {
                    if (arg !== "wip" && !floorPage.sidebarCollapsed && arg !== "log")
                        return false
                    window.width = Math.ceil(window.floorWidth)
                    window.height = Math.ceil(window.floorHeight)
                    acts.floorStoodAt = window.width + "x" + window.height
                    return true
                })
                return
            }
            if (op.at("shrunk")) {
                // On the floor, before it moves: a folded window's rail is only ever read here in `fold`.
                const standing = window.width >= window.floorWidth && window.height >= window.floorHeight
                if (!Awaited.all("window_floor_shrunk", {
                        "floor": standing,
                        "laid": standing && acts.menuReadAfterFrames(op) }))
                    return
                op.advance("floored")
                return
            }
            if (op.at("floored") && arg !== "wip") {
                op.input("raised", () => {
                    if (arg === "fold")
                        window.floorPage.sidebarCollapsed = false
                    else if (logs)
                        window.floorPage.commandsOpen = true
                })
                return
            }
            if (op.at("raised")) {
                if (!Awaited.all("window_floor_raised", {
                        "unfolded": arg !== "fold" || !window.floorPage.sidebarCollapsed,
                        "log": !logs || window.floorPage.commandsOpen }))
                    return
                op.advance("risen")
                return
            }
            // The report's reading: `blank`, `risen`, or `floored` for `wip`. The block scrolls even when not shown,
            // so only `wipShown` catches a face taken away behind the run (`RepoPage.leaveWipWhenDone`).
            const stands = window.width >= Math.ceil(window.floorWidth)
                && window.height >= Math.ceil(window.floorHeight)
                && !(arg === "fold" && window.floorPage.sidebarCollapsed)
            if (!Awaited.all("window_floor_landed", {
                    "floor": stands,
                    "log": !logs || window.floorPage.commandsOpen,
                    "wip": arg !== "wip"
                        || (window.floorPage.wipShown && window.floorPage.wipBlockScrolls),
                    "laid": stands && acts.menuReadAfterFrames(op),
                    "page": PageSettled.windowSettled(window) }))
                return
            op.complete(() => acts.floorLine())
        }
    }
    /// Where the window stood before the floor moved: one that came back up otherwise looks never let down.
    property string floorStoodAt: ""

    // ---- whether the left menu stands whole in the room the page gives it ----
    // Read off the layout itself (デザイン規約 §窓の床「左メニューが全部入る」): the floor's own sum would agree with
    // itself whatever the rail drew. A `SplitView` and a `Column` lay out on polish, so a reading waits two swapped
    // frames after it is asked for — one whose polish ran before the ask answers for the shape before it.
    /// Every reading held, and the last one's two numbers.
    property bool menuHeld: true
    property real menuEnd: 0
    property real menuRoom: 0
    /// The readings taken so far: each is waited for under a name of its own, so the next waits frames of its own.
    property int menuReads: 0
    /// Whether the menu has been read for `op`'s stage: false while the frames it waits for are still to come
    /// (`StageTimer.drawnSince`).
    function menuReadAfterFrames(op) {
        if (!op.drawnSince("menu_read#" + acts.menuReads))
            return false
        acts.menuReads++
        const pane = window.floorPage.pageSidebar
        if (pane.collapsed) {
            const rail = pane.autoRail
            // The rail's one child is the column of the fold block and the cells. It fills the rail, and what it laid
            // out runs past it when the rail is shorter — the rail does not clip.
            const column = rail.children[0]
            acts.menuEnd = column.childrenRect.y + column.childrenRect.height
            acts.menuRoom = rail.height
        } else {
            // The list's last band, at its own height: the layout gives a band no minimum, so a short list squeezes
            // the bands rather than pushing the last one out.
            const last = pane.autoSections.headOf("tag")
            acts.menuEnd = last.y + last.implicitHeight
            acts.menuRoom = pane.autoSections.height
        }
        acts.menuHeld = acts.menuHeld && acts.menuEnd <= acts.menuRoom
        return true
    }
    /// `fits=` is the whole verdict: reporting the floor alone would pass with the window nowhere near it.
    function floorLine() {
        const floorW = Math.ceil(window.floorWidth)
        const floorH = Math.ceil(window.floorHeight)
        return (
            // The verdict leads: `must_say` catches only neighbours in one substring.
            "window_floor fits="
            + (window.width >= floorW && window.height >= floorH)
            // Beside `fits=`: a floor summed short passes that one, the window standing on it.
            + " menu=" + acts.menuHeld + " menuEnd=" + acts.menuEnd + " menuRoom=" + acts.menuRoom
            + " floorW=" + floorW + " floorH=" + floorH
            // `floorW` is the larger of these two plus the edge's two columns (`Main.floorWidth`). Folding lowers
            // `pageW` only.
            + " bandW=" + Math.ceil(topBar.floorWidth)
            + " pageW=" + (window.floorPage !== null
                           ? Math.ceil(window.floorPage.floorWidth) : 0)
            + " edge=" + window.edgeInset
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
    }

    // PGG_AUTO_ACT=state: two runs sharing one --config-dir are what actually tests this — a single run can only ever
    // agree with itself.
    WindowStage {
        id: stateActTimer
        name: "state"
        order: ["asked", "landed", "stored"]
        running: Harness.autoAct === "state"
        /// The line, read in the press that stores the window — for `minimize`, while it is still down.
        property string storedLine: ""
        onTriggered: {
            const op = stateActTimer
            const minimize = Harness.autoActArg === "minimize"
            if (op.at("start") || op.at("asked")) {
                if (!Awaited.all("state_open", {
                        "open": window.curPage !== null && window.curPage.pageTab.state === "open",
                        "maximized": op.at("start") || !minimize || window.visibility === Window.Maximized }))
                    return
                if (op.at("asked")) {
                    op.advance("landed")
                    return
                }
                op.input("asked", () => {
                    if (Harness.autoActArg === "change") {
                        window.curPage.sidebarCollapsed = true
                        window.curPage.commandsOpen = true
                        window.curPage.setDetailsWidth(520)
                        window.curPage.setGraphColumns(190, 300)
                        AppBackend.setAutoFetchMinutes(7)
                        AppBackend.setInitialCommits(3000)
                    }
                    if (minimize)
                        window.visibility = Window.Maximized
                })
                return
            }
            // Read back once the splitters have taken the new sizes.
            const page = window.curPage
            if (!Awaited.all("state_report_ready", {
                    "ready": !(minimize && window.visibility !== Window.Maximized),
                    "page": PageSettled.windowSettled(window),
                    "changed": Harness.autoActArg !== "change"
                        || (page !== null && page.sidebarCollapsed && page.commandsOpen && page.commandsShown
                            && Math.abs(page.stateDetailsWidth - 520) < 1) }))
                return
            // Reported up, then down: the file must hold the same once the window is down, which one report cannot
            // show. Then back up for the shot (`grabToImage` gets nothing from a window that is down), and windowed:
            // the offscreen platform maximises to its own 800x800 screen.
            if (!op.input("stored", () => {
                    if (minimize) {
                        window.reportState()
                        window.visibility = Window.Minimized
                    }
                    window.reportState()
                    op.storedLine = acts.stateLine()
                    if (minimize)
                        window.visibility = Window.Windowed
                }))
                return
            op.complete(() => op.storedLine)
        }
    }
    function stateLine() {
        return "state tabs=" + pageRepeater.count
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
            + " initialCommits=" + AppBackend.initialCommits
    }
}
