pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import platitude
import platitude.ui

/// The band's half of the window's PGG_AUTO_ACT harness: the row itself, the ☰ menu, and the three actions giving
/// their words up as the window narrows. The state badges are the other half (`WindowBadgeActs`).
// An `Item` only because `QtObject` has no default property to hold the timers; it is sizeless.
Item {
    id: acts

    required property var window
    required property TopBar topBar
    required property WindowChrome chrome
    required property Item mainUi
    required property IdentityDialog identityDialog

    // The ring is latched on the real busy edge, which is often shorter than a sampler's beat; the hold keeps a fast
    // child from clearing it before the image callback runs.
    Connections {
        target: acts.topBar.curPage ? acts.topBar.curPage.pageTab : null
        function onBusyOpChanged() {
            const op = acts.topBar.curPage.pageTab.busyOp
            if (Harness.autoAct === "force-push-hold" && op === "push" && !acts.topBar.holdPushBusy) {
                acts.pushModeAtBusy = acts.topBar.pushMode
                acts.topBar.holdPushBusy = true
            }
            if (Harness.autoAct === "fetch-busy" && op === "fetch")
                acts.topBar.holdFetchBusy = true
        }
    }
    /// The push mode on the busy edge. Read at report time it is already the push's answer: a fast push has levelled
    /// the branch with its remote.
    property string pushModeAtBusy: ""

    // PGG_AUTO_ACT=force-push-hold: the push's hold run out on a diverged branch, and the ring latched on its edge.
    WindowStage {
        id: pushHoldTimer
        name: "force_push_hold"
        order: ["pressed"]
        running: Harness.autoAct === "force-push-hold"
        onTriggered: {
            const op = pushHoldTimer
            if (op.at("start")) {
                if (!Awaited.all("force_push_hold_ready", {
                        "diverged": topBar.curPage !== null && topBar.pushMode === "diverged",
                        "idle": !topBar.pushHolding }))
                    return
                // Answers whether it went in: a button met deaf is pressed a beat later.
                op.input("pressed", () => topBar.completePushHold())
                return
            }
            // A press the button blanked: the tab's opening fetch took git, the button went deaf and the fill blanked
            // with nothing sent (`ActionButton.onLiveChanged`) — no gesture and no edge. Pressed again, in a
            // generation of its own. A completed press stands until its edge above, so nothing is pressed twice.
            if (!topBar.holdPushBusy && !topBar.pushHolding) {
                op.again()
                return
            }
            // The ring is held, so the page can be waited for: the row the opening selected is still reading its
            // details, and the mode was latched on the edge (`pushModeAtBusy`).
            op.complete({
                "name": "force_push_hold_landed",
                "when": () => ({ "busy": topBar.holdPushBusy, "page": PageSettled.windowSettled(window) }),
                "report": () => "push_hold mode=" + acts.pushModeAtBusy + " busy=" + topBar.holdPushBusy })
        }
    }

    // PGG_AUTO_ACT=fetch-busy: the same latch on fetch. The page's driver fires the fetch; this waits for the edge.
    WindowStage {
        id: fetchBusyTimer
        name: "fetch_busy"
        running: Harness.autoAct === "fetch-busy"
        onTriggered: fetchBusyTimer.complete({
            "name": "fetch_busy_ready",
            "when": () => ({ "ready": topBar.holdFetchBusy, "page": PageSettled.windowSettled(window) }),
            "report": () => "fetch_busy busy=" + topBar.holdFetchBusy
                            + " fails=" + topBar.fetchFails + " turned=" + topBar.fetchFrameTurned })
    }

    // PGG_AUTO_ACT=fetch-tip on|off: the fetch button's tip with and without a remote (`off` = `--preset noremote`).
    // The dim side waits for a landed, idle repository rather than the button, so a band read before the remotes
    // arrived cannot pass for either.
    WindowStage {
        id: fetchTipTimer
        name: "fetch_tip"
        running: Harness.autoAct === "fetch-tip"
        // The graph too: the band settles first, and the picture is not to have a half-drawn page under it.
        onTriggered: fetchTipTimer.complete({
            "name": "fetch_tip_ready",
            "when": () => {
                const tab = window.curPage !== null ? window.curPage.pageTab : null
                return { "ready": !(window.curPage === null || !window.curPage.pageRefsLoaded
                                    || window.curPage.pageGraph.rowTotal < 1),
                         "page": PageSettled.windowSettled(window),
                         "idle": tab !== null && tab.busyCount === 0 && !tab.autoFetchRunning,
                         "side": Harness.autoActArg === "off" ? tab !== null && tab.remoteCount === 0
                                                              : topBar.fetchLive }
            },
            "report": () => "fetch_tip enabled=" + topBar.fetchLive
                            + " tip=" + topBar.fetchTipShown
                            + " remotes=" + (topBar.curPage !== null ? topBar.curPage.pageTab.remoteCount : -1) })
    }

    // PGG_AUTO_ACT=stash-state ready|clean|conflicts|unborn: waits for the Stash button to say the argument — its
    // reading lands over several drains, and read early it is `unborn` everywhere (デザイン規約 §変更を退避する).
    WindowStage {
        id: stashStateTimer
        name: "stash_state"
        running: Harness.autoAct === "stash-state"
        // Not the graph, unlike `fetch-tip`: `empty` has no rows, so it is judged on its worktree alone.
        onTriggered: stashStateTimer.complete({
            "name": "stash_state_ready",
            "when": () => {
                const tab = window.curPage !== null ? window.curPage.pageTab : null
                return { "ready": !(window.curPage === null || !window.curPage.pageWorktree.loaded),
                         "page": PageSettled.windowSettled(window),
                         "idle": tab !== null && tab.busyCount === 0 && !tab.autoFetchRunning,
                         "mode": topBar.stashMode === Harness.autoActArg }
            },
            "report": () => "stash_state mode=" + topBar.stashMode
                            + " enabled=" + topBar.stashLive
                            + " tip=" + topBar.stashTipShown })
    }

    // PGG_AUTO_ACT=band: numbers, because the headless platform draws no window buttons — a band that lost its grab
    // run or pushed its buttons off the end looks fine in the picture.
    WindowStage {
        id: bandActTimer
        name: "band"
        running: Harness.autoAct === "band"
        // No tabs is a valid band: readiness is the window and the bar, and `tabsW=0` says the rest.
        onTriggered: bandActTimer.complete({
            "name": "band_act_ready",
            "when": () => ({ "shown": window.visible,
                             "sized": mainUi.width > 0 && topBar.width > 0,
                             "page": PageSettled.windowSettled(window) }),
            "report": () => "band merged=" + window.captionMerged
                + " systemTitleBar=" + AppBackend.systemTitleBar
                + " grabRun=" + topBar.bandGrabRun + " seamRun=" + topBar.bandSeamRun
                + " buttonsX=" + topBar.bandButtonsX
                + " width=" + topBar.width
                + " tabsW=" + topBar.bandTabsWidth
                + " rightMargin=" + topBar.bandRightMargin
                // A picture cannot tell a name cut at both ends from a short one, or an empty bracket from none.
                + " names=" + topBar.opsNames + " upstream=" + topBar.branchUpstream })
    }

    // PGG_AUTO_ACT=ops-panel [<width>]: the panel once its repository has landed (the state matrix's verb). The floor
    // may refuse the width, so the width the run came out at rides back in the line, with everything else a picture
    // cannot be read for.
    WindowStage {
        id: opsPanelTimer
        name: "ops_panel"
        running: Harness.autoAct === "ops-panel"
        /// The width the argument asks for, or NaN for none.
        readonly property real wanted: parseInt(Harness.autoActArg)
        /// Whether the window is still to be brought to that width: the floor may refuse it.
        function resizes() {
            return !isNaN(opsPanelTimer.wanted) && opsPanelTimer.wanted > 0
                && Math.round(window.width) !== opsPanelTimer.wanted
                && opsPanelTimer.wanted >= Math.ceil(window.floorWidth)
        }
        /// What the panel is read on, read now: its repository landed, the page, the width the run asked for and the
        /// band across the window.
        function terms() {
            return { "loaded": acts.opsPanelLoaded(),
                     "page": PageSettled.settledUnder(window),
                     "width": !opsPanelTimer.resizes(),
                     "band": topBar.width === mainUi.width }
        }
        onTriggered: {
            const op = opsPanelTimer
            if (op.at("start") && op.resizes()) {
                if (!Awaited.all("ops_panel_loaded", { "loaded": acts.opsPanelLoaded(),
                                                       "page": PageSettled.settledUnder(window) }))
                    return
                // Asked again each beat until the window has taken it; a width lost after that is the `width` term's.
                op.input("sized", () => {
                    window.width = op.wanted
                    return !op.resizes()
                })
                return
            }
            if (!Awaited.all("ops_panel_ready", op.terms()))
                return
            // Read off a drawn frame: the layout places the counts on the way to a frame, so on the tick they still
            // sit where the previous reading put them — the panel's readings (`opsPanelKey`) standing across a
            // swapped frame (`still`). What a frame was drawn for is kept, and the completion and the pictures stand
            // on the panel still reading it.
            const key = acts.opsPanelKey()
            if (!op.still("ops_panel_drawn", key))
                return
            op.witness("drawn", key)
            Awaited.claim("ops_panel", () => acts.opsPanelLoaded() && acts.opsPanelKey() === op.witnessed("drawn"))
            op.complete({
                "name": "ops_panel_drawn",
                "when": () => {
                    const terms = op.terms()
                    terms["drawn"] = acts.opsPanelKey() === op.witnessed("drawn")
                    return terms
                },
                "report": () => acts.opsPanelLine() })
        }
    }
    function opsPanelKey() {
        const worktree = window.curPage === null ? null : window.curPage.pageWorktree
        return topBar.opsNames + "|" + topBar.branchUpstream + "|"
            + (worktree === null ? "" : worktree.ahead + "/" + worktree.behind)
            + "|" + topBar.width
    }
    function opsPanelLoaded() {
        if (!window.visible || topBar.width <= 0 || window.curPage === null
                || !window.curPage.pageRefsLoaded || !window.curPage.pageWorktree.loaded)
            return false
        return acts.fetchSettled(window.curPage.pageTab)
    }
    /// Whether the fetch the page makes on its own at the opening has answered, and nothing is running now: a
    /// `busyCount` of 0 read before that fetch began is the moment before it, not after it, and a picture taken
    /// then is of counts the answer rewrites. Read off the count of answers (`RepoTab.autoFetchSeq`), since the
    /// running flag falls between two beats and the answer rides one notify. A repository with no remote, or a
    /// page whose fetching is suspended, makes no such fetch.
    function fetchSettled(tab) {
        if (tab.busyCount !== 0 || tab.autoFetchRunning)
            return false
        if (tab.remoteNames.length === 0 || tab.autoFetchSuspended)
            return true
        return tab.autoFetchSeq > 0
    }
    /// The panel as the completion reads it. Claimed again at the pictures (`ops_panel`): a fetch answering between
    /// the report and the picture rewrites the counts the report named.
    function opsPanelLine() {
        return "ops_panel settled=true"
            // The next four cannot be judged off the picture: each frames like something else.
            + " lit=" + (topBar.repoNameLit || topBar.branchNameLit)
            + " boxed=" + topBar.actionsBoxed
            + " rule=" + topBar.findRuled
            + " track=" + topBar.branchTrackPlace
            + " width=" + Math.round(window.width)
            + " floor=" + Math.ceil(window.floorWidth)
            + " names=" + topBar.opsNames + " upstream=" + topBar.branchUpstream
            + " namesCut=" + topBar.opsNameCut
            + " folded=" + topBar.actionsFolded
            + " alert=" + topBar.actionAlertShown
            + " push=" + topBar.pushMode + " stash=" + topBar.stashMode
            + " badges=" + [topBar.opBadgeShown, topBar.conflictBadgeShown,
                            topBar.identityBadgeShown, topBar.oldGitBadgeShown,
                            topBar.staleBadgeShown, topBar.lfsBadgeShown].join(",")
    }

    // PGG_AUTO_ACT=ops-stand / ops-stand-repos / ops-stand-worktrees [<filter>] / ops-branch /
    // ops-branch-folder <path>: the cards the repository's and the branch's names open. Rows are counted, since an
    // empty card looks like one whose rows were all left out (`AppMenu.offeredRows`), and the opening name's lit /
    // turned is read, since under the hand that pressed it the name looks the same whether or not the card keeps it
    // lit.
    //
    // The card asked for (`opened`), then the tier inside it (`tier`) or the branch card's folder (`folder`).
    WindowStage {
        id: opsDoorTimer
        name: "ops_door"
        order: Harness.autoAct === "ops-branch-folder" ? ["opened", "folder"]
             : acts.opsDoorTier ? ["opened", "tier"] : ["opened"]
        running: Harness.autoAct === "ops-stand" || Harness.autoAct === "ops-stand-repos"
                 || Harness.autoAct === "ops-stand-worktrees" || Harness.autoAct === "ops-branch"
                 || Harness.autoAct === "ops-branch-folder"
        onTriggered: {
            const op = opsDoorTimer
            const branchDoor = Harness.autoAct === "ops-branch" || Harness.autoAct === "ops-branch-folder"
            if (op.at("start")) {
                // Both listings, not a beat: they land on separate reads. Every repository lists its own worktree,
                // so zero worktrees is a listing not yet landed.
                if (!Awaited.all("ops_door_ready", {
                    "shown": window.visible && topBar.width > 0,
                    "refs": window.curPage !== null && window.curPage.pageRefsLoaded,
                    "worktrees": window.curPage !== null && window.curPage.pageWorktrees.total > 0,
                    "page": PageSettled.settledUnder(window) }))
                    return
                // Asked for once, and reported however it went: `offerHere` turns away an empty card, so re-asking
                // would sit out the watchdog.
                op.input("opened", () => {
                    // The left menu filtered first (as `NavProbe.typeFilter` does): a worktree filtered out of sight
                    // there is still one the card offers.
                    if (Harness.autoAct === "ops-stand-worktrees" && Harness.autoActArg !== "")
                        window.curPage.pageSidebar.autoSections.filterText = Harness.autoActArg
                    if (branchDoor)
                        topBar.openBranchMenu()
                    else
                        topBar.openStandMenu()
                })
                return
            }
            if (op.at("opened") && Harness.autoAct === "ops-branch-folder") {
                if (!Awaited.all("ops_door_branch_menu", { "menu": topBar.branchMenuOpen }))
                    return
                // Whether a folder of the branch card was there to be opened, kept.
                op.input("folder", () => {
                    op.witness("folderAsked", topBar.openBranchFolder(Harness.autoActArg))
                }, () => "ops_door step=folder asked=" + op.witnessed("folderAsked"))
                return
            }
            // The tier asked for in the beat that reads the card, as far as the card stands to hold it.
            if (op.at("opened") && acts.opsDoorTier
                    && !op.input("tier", () => {
                        if (!topBar.standMenuOpen)
                            return
                        if (Harness.autoAct === "ops-stand-repos")
                            topBar.openStandRepos()
                        else
                            topBar.openStandWorktrees()
                    }))
                return
            // The folder's card is up a turn behind the ask (`OpsBranchMenu.folderStanding`); a folder the card did
            // not have is reported now.
            op.complete({
                "name": "ops_door_landed",
                "when": () => ({ "folder": Harness.autoAct !== "ops-branch-folder" || !op.witnessed("folderAsked")
                                           || topBar.branchFolderOpen !== "" }),
                "report": () => {
                    const branchDoor = Harness.autoAct === "ops-branch" || Harness.autoAct === "ops-branch-folder"
                    return "ops_door open=" + (branchDoor ? topBar.branchMenuOpen : topBar.standMenuOpen)
                        + " tier=" + topBar.standDoor
                        + " lit=" + (branchDoor ? topBar.branchNameLit : topBar.repoNameLit)
                        + " turned=" + (branchDoor ? topBar.branchNameTurned : topBar.repoNameTurned)
                        // The frame handed back, so a press on the grab runs closes the card
                        // (`WindowChrome.frameYielded`).
                        + " yield=" + chrome.frameYielded
                        + " folder=" + topBar.branchFolderOpen
                        + " repos=" + topBar.standRepoRows
                        // The worktrees the left menu shows — the other half of `worktrees=` under a filter.
                        + " listed=" + window.curPage.pageWorktrees.shown()
                        + " worktrees=" + topBar.standWorktreeRows
                        + " branches=" + topBar.branchMenuRows
                } })
        }
    }
    /// Whether the verb goes on into a tier of the repository's card.
    readonly property bool opsDoorTier: Harness.autoAct === "ops-stand-repos"
                                        || Harness.autoAct === "ops-stand-worktrees"

    // PGG_AUTO_ACT=app-menu / app-menu-reclick: the ☰ pressed once and left standing, or pressed twice. Numbers too:
    // a reopened card frames like one never closed, and who owns the grab run is not drawn. `yield=true` is the frame
    // handed back to the scene (`WindowChrome.frameYielded`); `strip=` is where the run stands for when it is not,
    // `none` with no merged title bar. The second press lands only on a card observed standing, as a hand's would.
    WindowStage {
        id: appMenuActTimer
        name: "app_menu"
        order: Harness.autoAct === "app-menu-reclick" ? ["pressed", "repressed"] : ["pressed"]
        running: Harness.autoAct === "app-menu" || Harness.autoAct === "app-menu-reclick"
        onTriggered: {
            const op = appMenuActTimer
            const twice = Harness.autoAct === "app-menu-reclick"
            if (op.at("start")) {
                if (!Awaited.all("app_menu_act_ready", { "shown": window.visible && topBar.width > 0,
                                                         "page": PageSettled.windowSettled(window) }))
                    return
                op.input("pressed", () => {
                    topBar.clickAppMenu()
                })
                return
            }
            if (op.at("pressed") && twice) {
                if (!Awaited.all("app_menu_open", { "menu": topBar.appMenuOpen }))
                    return
                op.input("repressed", () => {
                    topBar.clickAppMenu()
                })
                return
            }
            op.complete({
                "name": "app_menu_landed",
                "when": () => {
                    const twice = Harness.autoAct === "app-menu-reclick"
                    return { "menu": twice || topBar.appMenuOpen,
                             "shut": !twice || !topBar.appMenuOpen,
                             "page": PageSettled.windowSettled(window) }
                },
                "report": () => "app_menu open=" + topBar.appMenuOpen
                    + " yield=" + chrome.frameYielded
                    + " strip=" + chrome.sentStrip
                    + " merged=" + window.captionMerged })
        }
    }

    /// Whether the run of failures that stops the fetch timer is over (`fetchStopTimer` asks for it). Needs a remote
    /// git cannot reach (`--preset unreachable`).
    readonly property bool fetchStopped: acts.window.curPage !== null && acts.window.curPage.pageTab.autoFetchSuspended

    // The run of failures that stops the fetch timer, for the two verbs that read the timer stopped: one fetch at a
    // time, each waited out — fetches fired in a row are answered as one — a generation each (`again`), until the
    // timer has stopped (`fetchStopped`).
    WindowStage {
        id: fetchStopTimer
        name: "fetch_stop"
        /// The fetch asked for, at the write the tab had answered before it.
        order: ["asked"]
        running: (Harness.autoAct === "fetch-tip-link" || Harness.autoAct === "band-actions-stopped")
                 && !acts.fetchStopped
        onTriggered: {
            const op = fetchStopTimer
            const page = acts.window.curPage
            if (op.at("start")) {
                // Not before the repository is open: core refuses the fetch without reaching git, that refusal
                // suspends the fetches too, and the panel the verb then opens holds no row. Nor before the opening
                // fetch has answered (`fetchSettled`), which the next one would be answered with.
                if (!Awaited.all("fetch_stop_idle", {
                        "open": page !== null && page.pageTab.state === "open",
                        "page": page !== null && PageSettled.windowSettled(window),
                        "idle": page !== null && acts.fetchSettled(page.pageTab) }))
                    return
                op.input("asked", () => {
                    op.witness("seq", page.pageTab.writeSeq)
                    page.pageTab.fetch("")
                })
                return
            }
            if (Awaited.all("fetch_stop_answered", {
                    "answered": page !== null && page.pageTab.writeSeq > op.witnessed("seq") }))
                op.again()
        }
    }

    // PGG_AUTO_ACT=fetch-tip-link: the link in the fetch button's tip pressed, and the panel it names coming up marked
    // (デザイン規約 §hover のツールチップ「送った先は名乗る」). The failures raised the panel (デザイン規約 §git が言った
    // ことを読む場所), and a press that finds it up proves half the verb, so it is shut first (`shut`); then the
    // stand-in pointer put on the button, once the failures are over, until its tip stands (`pointed`); then the link
    // pressed (`pressed`).
    WindowStage {
        id: tipLinkTimer
        name: "fetch_tip_link"
        order: ["shut", "pointed", "pressed"]
        running: Harness.autoAct === "fetch-tip-link"
        /// What every stage moves on, read now: the page settled and idle, and the run of failures over
        /// (`fetchStopTimer`). `pointed`: with the tip standing.
        function terms(pointed) {
            const page = acts.window.curPage
            const ready = page !== null && PageSettled.windowSettled(window)
            const idle = ready && page.pageTab.busyCount === 0 && !page.pageTab.autoFetchRunning
            const terms = { "ready": page !== null,
                            "page": PageSettled.windowSettled(window),
                            "idle": idle,
                            "stopped": idle && acts.fetchStopped }
            if (pointed)
                terms["standing"] = acts.topBar.fetchTipStanding
            return terms
        }
        onTriggered: {
            const op = tipLinkTimer
            const page = acts.window.curPage
            if (op.at("start")) {
                if (!Awaited.all("tip_link_ready", op.terms(false)))
                    return
                op.input("shut", () => {
                    if (page.commandsOpen)
                        page.shutCommands()
                })
                return
            }
            if (op.at("shut")) {
                if (!Awaited.all("tip_link_ready", op.terms(false)))
                    return
                op.input("pointed", () => {
                    acts.topBar.fetchPointedAt = true
                    return acts.topBar.fetchTipStanding
                })
                return
            }
            if (op.at("pointed")) {
                if (!Awaited.all("tip_link_ready", op.terms(true)))
                    return
                op.input("pressed", () => acts.window.pressTipLink())
                return
            }
            // The panel is built as it is raised (`RepoPage.commandsSeat`), so wait for it to exist.
            op.complete({
                "name": "tip_link_ready",
                "when": () => {
                    const shown = acts.window.curPage
                    const terms = op.terms(false)
                    terms["panel"] = shown !== null && shown.commandsOpen && shown.commandsPane !== null
                    return terms
                },
                "report": () => {
                    const shown = acts.window.curPage
                    return "fetch_link open=" + shown.commandsShown
                        + " lit=" + shown.commandsPane.attention
                        + " suspended=" + shown.pageTab.autoFetchSuspended
                } })
        }
    }

    // PGG_AUTO_ACT=band-actions*: the panel's four buttons giving their words up as the window narrows (規約 §操作パネル).
    // The run asks for a *shape* and the panel's own arithmetic names its width — which pixel is which shape depends on
    // the installed fonts.
    WindowStage {
        id: actionsActTimer
        name: "band_actions"
        /// The `!` pushed for (`-alert`), then the window brought to the shape's width.
        order: Harness.autoAct === "band-actions-alert" ? ["pushed", "sized"] : ["sized"]
        running: Harness.autoAct === "band-actions"
                 || Harness.autoAct === "band-actions-none"
                 || Harness.autoAct === "band-actions-fold"
                 || Harness.autoAct === "band-actions-alert"
                 || Harness.autoAct === "band-actions-stopped"
        /// The window's width for the shape asked for, off the panel's own arithmetic (`actionsWidthFor`).
        function wanted() {
            return acts.actionsWidthFor(Harness.autoAct === "band-actions" ? Harness.autoActArg
                                        : Harness.autoAct === "band-actions-none" ? "whole" : "fold")
        }
        /// What the panel is measured on, read now. `corner`: with what the verb's corner adds; `sized`: and the
        /// window at the shape's width, the band spanning it.
        function terms(corner, sized) {
            const page = window.curPage
            // Every width is measured off the set's shared box (`TopBar.widestAction`), zero until the band has
            // loaded — and off push's wording, which the worktree decides (`publish` until the first status,
            // `push -f` after on a diverged branch).
            const terms = {
                "band": topBar.bandTabsWidth > 0 && topBar.actionNaturalW > Theme.railWidth,
                "page": PageSettled.settledUnder(window),
                // The opening fetch answered: push's wording and the alert are the worktree's after it.
                "fetch": page !== null && acts.fetchSettled(page.pageTab)
            }
            const ready = terms["band"] && terms["page"] && terms["fetch"]
            // The refusal also brings its report down, and the bar grows into place (`NoticeBar.settled`):
            // photographed on the mark alone, it is half open. A refusal that raised no report would stand here to
            // the ceiling, named.
            if (corner && Harness.autoAct === "band-actions-alert") {
                terms["alert"] = topBar.actionAlertShown
                terms["report"] = page !== null && page.noticeCard.settled
            }
            // The same corner on fetch, which is marked by the run of failures that stops its timer
            // (`fetchStopTimer`).
            if (corner && Harness.autoAct === "band-actions-stopped") {
                const idle = page !== null && page.pageTab.busyCount === 0 && !page.pageTab.autoFetchRunning
                terms["idle"] = idle
                terms["stopped"] = ready && idle && acts.fetchStopped
            }
            if (sized) {
                terms["width"] = Math.round(window.width) === actionsActTimer.wanted()
                terms["spans"] = topBar.width === mainUi.width
            }
            return terms
        }
        onTriggered: {
            const op = actionsActTimer
            const page = window.curPage
            // The `!` before the window is sized: a real refusal from git (`--preset longnames` will not fast-forward,
            // as `push-retry` sets up). Sent through the page, because in this state the button is a hold and its
            // plain press is wired to nothing.
            if (Harness.autoAct === "band-actions-alert" && op.at("start")) {
                if (!Awaited.all("band_actions_ready", op.terms(false, false)))
                    return
                op.input("pushed", () => {
                    page.pushNow()
                })
                return
            }
            // Sized once the rest holds — push's wording, which the width is measured off, is the worktree's once the
            // opening fetch has answered — and asked again each beat until the window takes it. The target does not
            // move with the window's width, so this closes no ring.
            if (!op.at("sized")) {
                if (!Awaited.all("band_actions_ready", op.terms(true, false)))
                    return
                const wanted = op.wanted()
                if (!op.input("sized", () => {
                        if (Math.round(window.width) !== wanted) {
                            window.width = wanted
                            window.height = Math.ceil(window.floorHeight)
                        }
                        return Math.round(window.width) === wanted
                    }))
                    return
            }
            if (!Awaited.all("band_actions_ready", op.terms(true, true)))
                return
            // The shape the run completes on, kept: the completion and the pictures stand on the panel still in it.
            op.witness("shape", acts.bandActionsKey())
            Awaited.claim("band_actions", () => acts.bandActionsKey() === op.witnessed("shape")
                          && acts.fetchSettled(window.curPage.pageTab))
            // Which button the mark is on: `alert=` is the pair or-ed (`TopBar.actionAlertShown`).
            op.complete({
                "name": "band_actions_shaped",
                "when": () => {
                    const terms = op.terms(true, true)
                    terms["shape"] = acts.bandActionsKey() === op.witnessed("shape")
                    return terms
                },
                "report": () => Harness.autoAct === "band-actions-stopped"
                    ? ["band_stopped suspended=" + window.curPage.pageTab.autoFetchSuspended
                       + " turned=" + topBar.fetchFrameTurned
                       + " alert=" + topBar.actionAlertShown,
                       acts.bandActionsLine()]
                    : acts.bandActionsLine() })
        }
    }
    /// The width a shape sits at: a number is a pixel, `fold` the widest panel that has given every wording up,
    /// `whole` the narrowest that says all three in full, and the default the narrowest cell still holding a word.
    function actionsWidthFor(arg) {
        const wanted = parseInt(arg)
        if (!isNaN(wanted) && wanted > 0)
            return wanted
        // The panel's own arithmetic, not worked out again here, so a build whose schedule moved cannot pass
        // (`TopBar.actionsWholeAt`). Those are the panel's widths; the window is wider by its edge on each side.
        const edges = 2 * window.edgeInset
        if (arg === "fold")
            return topBar.actionsFoldAt + edges
        // Never under the window's floor: less would photograph a window no hand can make.
        if (arg === "whole")
            return Math.max(topBar.actionsWholeAt + edges, Math.ceil(window.floorWidth))
        // The last labelled cell: a pixel past it, the three give their words up.
        return topBar.actionsFoldAt + 1 + edges
    }
    /// `folded=`, `even=` and `find=` are what the picture cannot answer: one width says nothing of where the shape
    /// changes, and cells a pixel apart look alike.
    /// The panel's shape as the report names it, for the claim read again at the pictures.
    function bandActionsKey() {
        return [topBar.actionsFolded, topBar.actionsDeep, topBar.actionsBoxed, topBar.actionsEven, topBar.findFolded,
                topBar.actionAlertShown, topBar.stateMarkShown, window.width].join("/")
    }
    function bandActionsLine() {
        return "band_actions fits=" + (window.width >= Math.ceil(window.floorWidth))
            // The seven judged lead together: `must_say` catches only neighbours in one substring. `deep=` is the
            // frames' depth, which a fold keeps (`TopBar.actionsDeep`); `boxed=` that depth holding what each button
            // draws, which the face's line height decides (`actionsBoxed`); `even=` the four cells' one width
            // (`actionsEven`); `find=` whether the find has given its word up, which it does before the three
            // (`findFolded`).
            + " folded=" + topBar.actionsFolded
            + " deep=" + topBar.actionsDeep
            + " boxed=" + topBar.actionsBoxed
            + " even=" + topBar.actionsEven
            + " find=" + topBar.findFolded
            + " alert=" + topBar.actionAlertShown
            + " cap=" + topBar.actionCapW
            + " natural=" + topBar.actionNaturalW
            + " foldAt=" + topBar.actionFoldW
            + " want=" + topBar.actionWantW
            + " cellW=" + topBar.actionCellW + " cellH=" + topBar.actionCellH
            + " wordW=" + topBar.actionWordW + " inkW=" + topBar.actionInkW
            + " mark=" + topBar.stateMarkShown
            + " openW=" + Math.ceil(window.openFloorWidth)
            + " floorW=" + Math.ceil(window.floorWidth)
            + " bandW=" + Math.ceil(topBar.floorWidth)
            + " w=" + window.width
    }
}
