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
            if (!Awaited.all("force_push_hold_landed", { "busy": topBar.holdPushBusy,
                                                         "page": PageSettled.windowSettled(window) }))
                return
            op.complete(() => "push_hold mode=" + acts.pushModeAtBusy + " busy=" + topBar.holdPushBusy)
        }
    }

    // PGG_AUTO_ACT=fetch-busy: the same latch on fetch. The page's driver fires the fetch; this waits for the edge.
    WindowStage {
        id: fetchBusyTimer
        name: "fetch_busy"
        running: Harness.autoAct === "fetch-busy"
        onTriggered: {
            if (!Awaited.all("fetch_busy_ready", { "ready": topBar.holdFetchBusy,
                                                   "page": PageSettled.windowSettled(window) }))
                return
            fetchBusyTimer.complete(() => "fetch_busy busy=" + topBar.holdFetchBusy
                                          + " fails=" + topBar.fetchFails + " turned=" + topBar.fetchFrameTurned)
        }
    }

    // PGG_AUTO_ACT=fetch-tip on|off: the fetch button's tip with and without a remote (`off` = `--preset noremote`).
    // The dim side waits for a landed, idle repository rather than the button, so a band read before the remotes
    // arrived cannot pass for either.
    WindowStage {
        id: fetchTipTimer
        name: "fetch_tip"
        running: Harness.autoAct === "fetch-tip"
        onTriggered: {
            const tab = window.curPage !== null ? window.curPage.pageTab : null
            // The graph too: the band settles first, and the picture is not to have a half-drawn page under it.
            if (!Awaited.all("fetch_tip_ready", {
                "ready": !(window.curPage === null || !window.curPage.pageRefsLoaded
                    || window.curPage.pageGraph.rowTotal < 1),
                "page": PageSettled.windowSettled(window),
                "idle": tab !== null && tab.busyCount === 0 && !tab.autoFetchRunning,
                "side": Harness.autoActArg === "off" ? tab !== null && tab.remoteCount === 0 : topBar.fetchLive }))
                return
            fetchTipTimer.complete(() => "fetch_tip enabled=" + topBar.fetchLive
                                         + " tip=" + topBar.fetchTipShown
                                         + " remotes=" + (topBar.curPage !== null
                                                          ? topBar.curPage.pageTab.remoteCount : -1))
        }
    }

    // PGG_AUTO_ACT=stash-state ready|clean|conflicts|unborn: waits for the Stash button to say the argument — its
    // reading lands over several drains, and read early it is `unborn` everywhere (デザイン規約 §変更を退避する).
    WindowStage {
        id: stashStateTimer
        name: "stash_state"
        running: Harness.autoAct === "stash-state"
        onTriggered: {
            const tab = window.curPage !== null ? window.curPage.pageTab : null
            // Not the graph, unlike `fetch-tip`: `empty` has no rows, so it is judged on its worktree alone.
            if (!Awaited.all("stash_state_ready", {
                "ready": !(window.curPage === null || !window.curPage.pageWorktree.loaded),
                "page": PageSettled.windowSettled(window),
                "idle": tab !== null && tab.busyCount === 0 && !tab.autoFetchRunning,
                "mode": topBar.stashMode === Harness.autoActArg }))
                return
            stashStateTimer.complete(() => "stash_state mode=" + topBar.stashMode
                                           + " enabled=" + topBar.stashLive
                                           + " tip=" + topBar.stashTipShown)
        }
    }

    // PGG_AUTO_ACT=band: numbers, because the headless platform draws no window buttons — a band that lost its grab
    // run or pushed its buttons off the end looks fine in the picture.
    WindowStage {
        id: bandActTimer
        name: "band"
        running: Harness.autoAct === "band"
        onTriggered: {
            // No tabs is a valid band: readiness is the window and the bar, and `tabsW=0` says the rest.
            if (!Awaited.all("band_act_ready", {
                "shown": window.visible,
                "sized": mainUi.width > 0 && topBar.width > 0,
                "page": PageSettled.windowSettled(window) }))
                return
            bandActTimer.complete(() => "band merged=" + window.captionMerged
                + " systemTitleBar=" + AppBackend.systemTitleBar
                + " grabRun=" + topBar.bandGrabRun + " seamRun=" + topBar.bandSeamRun
                + " buttonsX=" + topBar.bandButtonsX
                + " width=" + topBar.width
                + " tabsW=" + topBar.bandTabsWidth
                + " rightMargin=" + topBar.bandRightMargin
                // A picture cannot tell a name cut at both ends from a short one, or an empty bracket from none.
                + " names=" + topBar.opsNames + " upstream=" + topBar.branchUpstream)
        }
    }

    // PGG_AUTO_ACT=ops-panel [<width>]: the panel once its repository has landed (the state matrix's verb). The floor
    // may refuse the width, so the width the run came out at rides back in the line, with everything else a picture
    // cannot be read for.
    WindowStage {
        id: opsPanelTimer
        name: "ops_panel"
        running: Harness.autoAct === "ops-panel"
        /// The grab out and the last one answered, each the ask it was made for: one grab is out at a time.
        property var asked: null
        property var answered: null
        /// The panel's readings a drawn frame was answered for (`opsPanelKey`), "" until one was.
        property string drawnFor: ""
        onTriggered: {
            const op = opsPanelTimer
            const loaded = acts.opsPanelLoaded()
            const settled = PageSettled.settledUnder(window)
            const wanted = parseInt(Harness.autoActArg)
            const resizes = !isNaN(wanted) && wanted > 0 && Math.round(window.width) !== wanted
                && wanted >= Math.ceil(window.floorWidth)
            const spans = topBar.width === mainUi.width
            const key = acts.opsPanelKey()
            if (!Awaited.all("ops_panel_ready", {
                    "loaded": loaded,
                    "page": settled,
                    "width": !resizes,
                    "band": spans,
                    "drawn": op.drawnFor === key })) {
                if (loaded && settled && resizes) {
                    window.width = wanted
                } else if (loaded && settled && spans && op.asked === op.answered) {
                    // Read off a drawn frame (the grab's callback, rules/app-ui.md §UI 自動化): the layout places the
                    // counts on the way to a frame, so on the tick they still sit where the previous reading put them.
                    // The callback is posted, so a frame whose readings moved before it ran answers nothing, and the
                    // next one is asked for.
                    const ask = { "key": key }
                    if (topBar.grabToImage(() => {
                            op.answered = ask
                            if (op.running && acts.opsPanelLoaded() && topBar.width === mainUi.width
                                    && acts.opsPanelKey() === ask.key)
                                op.drawnFor = ask.key
                        }))
                        op.asked = ask
                }
                return
            }
            op.complete(() => acts.opsPanelLine())
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
    /// The panel as the completion reads it. Read again at the pictures: a fetch answering between the report and the
    /// picture rewrites the counts the report named.
    function opsPanelLine() {
        const key = acts.opsPanelKey()
        Awaited.claim("ops_panel", () => acts.opsPanelLoaded() && acts.opsPanelKey() === key)
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
        /// Whether a folder of the branch card was there to be opened (`ops-branch-folder`).
        property bool folderAsked: false
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
                op.input("folder", () => {
                    op.folderAsked = topBar.openBranchFolder(Harness.autoActArg)
                }, () => "ops_door step=folder asked=" + op.folderAsked)
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
            if (!Awaited.all("ops_door_landed", {
                    "folder": !op.at("folder") || !op.folderAsked || topBar.branchFolderOpen !== "" }))
                return
            op.complete(() => "ops_door open=" + (branchDoor ? topBar.branchMenuOpen : topBar.standMenuOpen)
                + " tier=" + topBar.standDoor
                + " lit=" + (branchDoor ? topBar.branchNameLit : topBar.repoNameLit)
                + " turned=" + (branchDoor ? topBar.branchNameTurned : topBar.repoNameTurned)
                // The frame handed back, so a press on the grab runs closes the card (`WindowChrome.frameYielded`).
                + " yield=" + chrome.frameYielded
                + " folder=" + topBar.branchFolderOpen
                + " repos=" + topBar.standRepoRows
                // The worktrees the left menu shows — the other half of `worktrees=` under a filter.
                + " listed=" + window.curPage.pageWorktrees.shown()
                + " worktrees=" + topBar.standWorktreeRows
                + " branches=" + topBar.branchMenuRows)
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
            if (!Awaited.all("app_menu_landed", { "menu": twice || topBar.appMenuOpen,
                                                  "shut": !twice || !topBar.appMenuOpen,
                                                  "page": PageSettled.windowSettled(window) }))
                return
            op.complete(() => "app_menu open=" + topBar.appMenuOpen
                + " yield=" + chrome.frameYielded
                + " strip=" + chrome.sentStrip
                + " merged=" + window.captionMerged)
        }
    }

    /// Whether the run of failures that stops the timer is over, asking for one more if it is not. Needs a remote git
    /// cannot reach (`--preset unreachable`).
    ///
    /// One at a time, each waited out: fetches fired in a row are answered as one, so `timer.askSeq` holds the write
    /// the last one was asked at.
    function stoppedYet(tab, timer) {
        if (tab.autoFetchSuspended)
            return true
        // Not before the repository is open: core refuses the fetch without reaching git, that refusal suspends the
        // fetches too, and the panel the verb then opens holds no row.
        if (tab.state !== "open")
            return false
        if (timer.askSeq >= 0 && tab.writeSeq <= timer.askSeq)
            return false
        timer.askSeq = tab.writeSeq
        tab.fetch("")
        return false
    }

    // PGG_AUTO_ACT=fetch-tip-link: the link in the fetch button's tip pressed, and the panel it names coming up marked
    // (デザイン規約 §hover のツールチップ「送った先は名乗る」). The failures raised the panel (デザイン規約 §git が言った
    // ことを読む場所), and a press that finds it up proves half the verb, so it is shut first (`shut`); then the link
    // pressed (`pressed`).
    WindowStage {
        id: tipLinkTimer
        name: "fetch_tip_link"
        order: ["shut", "pressed"]
        running: Harness.autoAct === "fetch-tip-link"
        property int askSeq: -1
        onTriggered: {
            const op = tipLinkTimer
            const page = acts.window.curPage
            const tab = page !== null ? page.pageTab : null
            const ready = page !== null && PageSettled.windowSettled(window)
            const idle = ready && tab.busyCount === 0 && !tab.autoFetchRunning
            const stopped = idle && acts.stoppedYet(tab, op)
            // The stand-in pointer on the button, once the panel is shut and the failures are over.
            if (op.at("shut") && stopped)
                acts.topBar.fetchPointedAt = true
            if (!Awaited.all("tip_link_ready", {
                    "ready": !(page === null),
                    "page": PageSettled.windowSettled(window),
                    "idle": idle,
                    "stopped": stopped,
                    "standing": !op.at("shut") || acts.topBar.fetchTipStanding,
                    // The panel is built as it is raised (`RepoPage.commandsSeat`), so wait for it to exist.
                    "panel": !op.at("pressed")
                        || (page !== null && page.commandsOpen && page.commandsPane !== null) }))
                return
            if (op.at("start")) {
                op.input("shut", () => {
                    if (page.commandsOpen)
                        page.shutCommands()
                })
                return
            }
            if (op.at("shut")) {
                op.input("pressed", () => acts.window.pressTipLink())
                return
            }
            op.complete(() => {
                const shown = acts.window.curPage
                return "fetch_link open=" + shown.commandsShown
                    + " lit=" + shown.commandsPane.attention
                    + " suspended=" + shown.pageTab.autoFetchSuspended
            })
        }
    }

    // PGG_AUTO_ACT=band-actions*: the panel's four buttons giving their words up as the window narrows (規約 §操作パネル).
    // The run asks for a *shape* and the panel's own arithmetic names its width — which pixel is which shape depends on
    // the installed fonts.
    WindowStage {
        id: actionsActTimer
        name: "band_actions"
        order: Harness.autoAct === "band-actions-alert" ? ["pushed"] : []
        running: Harness.autoAct === "band-actions"
                 || Harness.autoAct === "band-actions-none"
                 || Harness.autoAct === "band-actions-fold"
                 || Harness.autoAct === "band-actions-alert"
                 || Harness.autoAct === "band-actions-stopped"
        property int askSeq: -1
        onTriggered: {
            const op = actionsActTimer
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
            // The `!` before the window is sized: a real refusal from git (`--preset longnames` will not fast-forward,
            // as `push-retry` sets up). Sent through the page, because in this state the button is a hold and its
            // plain press is wired to nothing.
            if (Harness.autoAct === "band-actions-alert") {
                if (op.at("start")) {
                    if (!Awaited.all("band_actions_ready", terms))
                        return
                    op.input("pushed", () => {
                        page.pushNow()
                    })
                    return
                }
                // The refusal also brings its report down, and the bar grows into place (`NoticeBar.settled`):
                // photographed on the mark alone, it is half open. A refusal that raised no report would stand here
                // to the ceiling, named.
                terms["alert"] = topBar.actionAlertShown
                terms["report"] = page !== null && page.noticeCard.settled
            }
            // The same corner on fetch, which is marked by the run of failures that stops its timer (`stoppedYet`).
            if (Harness.autoAct === "band-actions-stopped") {
                const idle = page !== null && page.pageTab.busyCount === 0 && !page.pageTab.autoFetchRunning
                terms["idle"] = idle
                terms["stopped"] = ready && idle && acts.stoppedYet(page.pageTab, op)
            }
            // Re-asked on every tick: push's wording can still change after the worktree loads, and the width is
            // measured off it. The target does not move with the window's width, so this closes no ring.
            const wanted = acts.actionsWidthFor(
                Harness.autoAct === "band-actions" ? Harness.autoActArg
                : Harness.autoAct === "band-actions-none" ? "whole" : "fold")
            const before = Object.keys(terms).every(term => terms[term])
            terms["width"] = Math.round(window.width) === wanted
            terms["spans"] = topBar.width === mainUi.width
            if (before && !terms["width"]) {
                window.width = wanted
                window.height = Math.ceil(window.floorHeight)
            }
            if (!Awaited.all("band_actions_ready", terms))
                return
            // Which button the mark is on: `alert=` is the pair or-ed (`TopBar.actionAlertShown`).
            op.complete(() => Harness.autoAct === "band-actions-stopped"
                        ? ["band_stopped suspended=" + window.curPage.pageTab.autoFetchSuspended
                           + " turned=" + topBar.fetchFrameTurned
                           + " alert=" + topBar.actionAlertShown,
                           acts.bandActionsLine()]
                        : acts.bandActionsLine())
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
        const key = acts.bandActionsKey()
        Awaited.claim("band_actions", () => acts.bandActionsKey() === key
                      && acts.fetchSettled(window.curPage.pageTab))
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
