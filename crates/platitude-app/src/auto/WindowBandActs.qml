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

    // Pressed again when the button blanked the hold: the tab's opening fetch takes git, the button goes deaf and the
    // fill blanks with nothing sent (`ActionButton.onLiveChanged`). A completed press stands until its edge above, so
    // nothing is pressed twice.
    SampleTimer {
        running: Harness.autoAct === "force-push-hold"
        onTriggered: {
            const verb = "force_push_hold"
            if (topBar.holdPushBusy) {
                stop()
                Harness.report("push_hold mode=" + acts.pushModeAtBusy + " busy=" + topBar.holdPushBusy)
                window.finishAutoAct()
                return
            }
            if (topBar.curPage === null || topBar.pushMode !== "diverged") {
                Awaited.at(verb, "diverged")
                return
            }
            if (topBar.pushHolding) {
                Awaited.at(verb, "held")
                return
            }
            Awaited.at(verb, topBar.completePushHold() ? "pressed" : "live")
        }
    }

    // PGG_AUTO_ACT=fetch-busy: the same latch on fetch. The page's driver fires the fetch; this waits for the edge.
    SampleTimer {
        running: Harness.autoAct === "fetch-busy"
        onTriggered: {
            if (!topBar.holdFetchBusy)
                return
            stop()
            Harness.report("fetch_busy busy=" + topBar.holdFetchBusy
                              + " fails=" + topBar.fetchFails + " turned=" + topBar.fetchFrameTurned)
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=fetch-tip on|off: the fetch button's tip with and without a remote (`off` = `--preset noremote`).
    // The dim side waits for a landed, idle repository rather than the button, so a band read before the remotes
    // arrived cannot pass for either.
    SampleTimer {
        running: Harness.autoAct === "fetch-tip"
        onTriggered: {
            // The graph too: the band settles first, and the picture is not to have a half-drawn page under it.
            if (window.curPage === null
                    || !window.curPage.pageRefsLoaded
                    || window.curPage.pageGraph.rowTotal < 1)
                return
            const tab = window.curPage.pageTab
            if (tab.busyCount !== 0 || tab.autoFetchRunning)
                return
            if (Harness.autoActArg === "off") {
                if (tab.remoteCount !== 0)
                    return
            } else if (!topBar.fetchLive) {
                return
            }
            stop()
            Harness.report("fetch_tip enabled=" + topBar.fetchLive
                              + " tip=" + topBar.fetchTipShown
                              + " remotes=" + (topBar.curPage !== null
                                               ? topBar.curPage.pageTab.remoteCount : -1))
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=stash-state ready|clean|conflicts|unborn: waits for the Stash button to say the argument — its
    // reading lands over several drains, and read early it is `unborn` everywhere (デザイン規約 §変更を退避する).
    SampleTimer {
        running: Harness.autoAct === "stash-state"
        onTriggered: {
            // Not the graph, unlike `fetch-tip`: `empty` has no rows, so it is judged on its working tree alone.
            if (window.curPage === null || !window.curPage.pageWt.loaded)
                return
            const tab = window.curPage.pageTab
            if (tab.busyCount !== 0 || tab.autoFetchRunning)
                return
            if (topBar.stashMode !== Harness.autoActArg)
                return
            stop()
            Harness.report("stash_state mode=" + topBar.stashMode
                              + " enabled=" + topBar.stashLive
                              + " tip=" + topBar.stashTipShown)
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=band: numbers, because the headless platform draws no window buttons — a band that lost its grab
    // run or pushed its buttons off the end looks fine in the picture.
    SampleTimer {
        id: bandActTimer
        running: Harness.autoAct === "band"
        onTriggered: {
            // No tabs is a valid band: readiness is the window and the bar, and `tabsW=0` says the rest.
            if (!window.visible || mainUi.width <= 0 || topBar.width <= 0)
                return
            stop()
            Harness.report(
                "band merged=" + window.captionMerged
                + " systemTitleBar=" + AppBackend.systemTitleBar
                + " grabRun=" + topBar.bandGrabRun + " seamRun=" + topBar.bandSeamRun
                + " buttonsX=" + topBar.bandButtonsX
                + " width=" + topBar.width
                + " tabsW=" + topBar.bandTabsWidth
                + " rightMargin=" + topBar.bandRightMargin
                // A picture cannot tell a name cut at both ends from a short one, or an empty bracket from none.
                + " names=" + topBar.opsNames + " upstream=" + topBar.branchUpstream)
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=ops-panel [<width>]: the panel once its repository has landed (the state matrix's verb). The floor
    // may refuse the width, so the width the run came out at rides back in the line, with everything else a picture
    // cannot be read for.
    SampleTimer {
        id: opsPanelTimer
        running: Harness.autoAct === "ops-panel"
        /// A frame has been asked for and not yet drawn (`reportOpsPanel`).
        property bool drawing: false
        onTriggered: {
            if (!acts.opsPanelLoaded())
                return
            const wanted = parseInt(Harness.autoActArg)
            if (!isNaN(wanted) && wanted > 0 && Math.round(window.width) !== wanted
                    && wanted >= Math.ceil(window.floorWidth)) {
                window.width = wanted
                return
            }
            if (topBar.width !== mainUi.width || opsPanelTimer.drawing)
                return
            // Read off a drawn frame (the grab's callback, rules/app-ui.md §UI 自動化): the layout places the counts
            // on the way to a frame, so on the tick they still sit where the previous reading put them. The callback
            // is posted, so a frame whose readings moved before it ran is let go and the next one asked for.
            const drawnFor = acts.opsPanelKey()
            opsPanelTimer.drawing = topBar.grabToImage(() => {
                opsPanelTimer.drawing = false
                if (opsPanelTimer.running && acts.opsPanelLoaded() && topBar.width === mainUi.width
                        && acts.opsPanelKey() === drawnFor)
                    acts.reportOpsPanel()
            })
        }
    }
    function opsPanelKey() {
        const wt = window.curPage === null ? null : window.curPage.pageWt
        return topBar.opsNames + "|" + topBar.branchUpstream + "|" + (wt === null ? "" : wt.ahead + "/" + wt.behind)
            + "|" + topBar.width
    }
    function opsPanelLoaded() {
        if (!window.visible || topBar.width <= 0 || window.curPage === null
                || !window.curPage.pageRefsLoaded || !window.curPage.pageWt.loaded)
            return false
        const tab = window.curPage.pageTab
        return tab.busyCount === 0 && !tab.autoFetchRunning
    }
    function reportOpsPanel() {
        opsPanelTimer.stop()
        Harness.report(
            "ops_panel settled=true"
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
                            topBar.staleBadgeShown, topBar.lfsBadgeShown].join(","))
        window.finishAutoAct()
    }

    // PGG_AUTO_ACT=ops-stand / ops-stand-repos / ops-stand-copies [<filter>] / ops-branch / ops-branch-folder <path>:
    // the cards the repository's and the branch's names open. Rows are counted, since an empty card looks like one
    // whose rows were all left out (`AppMenu.offeredRows`), and the opening name's lit / turned is read, since under
    // the hand that pressed it the name looks the same whether or not the card keeps it lit.
    SampleTimer {
        id: opsDoorTimer
        running: Harness.autoAct === "ops-stand" || Harness.autoAct === "ops-stand-repos"
                 || Harness.autoAct === "ops-stand-copies" || Harness.autoAct === "ops-branch"
                 || Harness.autoAct === "ops-branch-folder"
        /// Which step this run is on: the card, then the tier inside it.
        property int opened: 0
        /// Whether a folder of the branch card was there to be opened (`ops-branch-folder`).
        property bool folderAsked: false
        onTriggered: {
            // Both listings, not a beat: they land on separate reads. Every repository lists its own copy, so zero
            // worktrees is a listing not yet landed.
            if (!window.visible || topBar.width <= 0 || window.curPage === null
                    || !window.curPage.pageRefsLoaded
                    || window.curPage.pageWorktrees.total <= 0)
                return
            const branchDoor = Harness.autoAct === "ops-branch" || Harness.autoAct === "ops-branch-folder"
            if (opsDoorTimer.opened === 0) {
                // The left menu filtered first (as `NavProbe.typeFilter` does): a copy filtered out of sight there is
                // still one the card offers.
                if (Harness.autoAct === "ops-stand-copies" && Harness.autoActArg !== "")
                    window.curPage.pageSidebar.autoSections.filterText = Harness.autoActArg
                opsDoorTimer.opened = 1
                // Asked for once, and reported however it went: `offerHere` turns away an empty card, so re-asking
                // would sit out the watchdog.
                if (branchDoor)
                    topBar.openBranchMenu()
                else
                    topBar.openStandMenu()
                return
            }
            if (opsDoorTimer.opened === 1 && !branchDoor && topBar.standMenuOpen) {
                if (Harness.autoAct === "ops-stand-repos")
                    topBar.openStandRepos()
                if (Harness.autoAct === "ops-stand-copies")
                    topBar.openStandCopies()
            }
            if (opsDoorTimer.opened === 1 && Harness.autoAct === "ops-branch-folder" && topBar.branchMenuOpen) {
                opsDoorTimer.opened = 2
                opsDoorTimer.folderAsked = topBar.openBranchFolder(Harness.autoActArg)
                Harness.report("ops_door step=folder asked=" + opsDoorTimer.folderAsked)
                return
            }
            // The folder's card is up a turn behind the ask (`OpsBranchMenu.folderStanding`); a folder the card did
            // not have is reported now.
            if (opsDoorTimer.opened === 2 && opsDoorTimer.folderAsked && topBar.branchFolderOpen === "")
                return
            stop()
            Harness.report(
                "ops_door open=" + (branchDoor ? topBar.branchMenuOpen : topBar.standMenuOpen)
                + " tier=" + topBar.standDoor
                + " lit=" + (branchDoor ? topBar.branchNameLit : topBar.repoNameLit)
                + " turned=" + (branchDoor ? topBar.branchNameTurned : topBar.repoNameTurned)
                // Grab runs handed back, so a press on them closes the card (`WindowChrome.captionYielded`).
                + " yield=" + chrome.captionYielded
                + " folder=" + topBar.branchFolderOpen
                + " repos=" + topBar.standRepoRows
                // The copies the left menu shows — the other half of `copies=` under a filter.
                + " listed=" + window.curPage.pageWorktrees.shown()
                + " copies=" + topBar.standCopyRows
                + " branches=" + topBar.branchMenuRows)
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=app-menu / app-menu-reclick: the ☰ pressed once and left standing, or pressed twice. Numbers too:
    // a reopened card frames like one never closed, and who owns the grab run is not drawn. `strip=none` is the run
    // handed back to the scene (`WindowChrome.captionYielded`); with no merged title bar there is no strip either way.
    SampleTimer {
        id: appMenuActTimer
        running: Harness.autoAct === "app-menu" || Harness.autoAct === "app-menu-reclick"
        /// Presses gone in. The second lands only on a card observed standing, as a hand's would.
        property int pressed: 0
        onTriggered: {
            if (!window.visible || topBar.width <= 0)
                return
            const twice = Harness.autoAct === "app-menu-reclick"
            if (appMenuActTimer.pressed === 0) {
                appMenuActTimer.pressed = 1
                topBar.clickAppMenu()
                return
            }
            if (appMenuActTimer.pressed === 1) {
                if (!topBar.appMenuOpen)
                    return
                if (twice) {
                    appMenuActTimer.pressed = 2
                    topBar.clickAppMenu()
                    return
                }
            }
            if (twice && topBar.appMenuOpen)
                return
            stop()
            Harness.report(
                "app_menu open=" + topBar.appMenuOpen
                + " yield=" + chrome.captionYielded
                + " strip=" + chrome.sentStrip
                + " merged=" + window.captionMerged)
            window.finishAutoAct()
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
    // (デザイン規約 §hover のツールチップ「送った先は名乗る」).
    SampleTimer {
        id: tipLinkTimer
        running: Harness.autoAct === "fetch-tip-link"
        property int askSeq: -1
        property bool shut: false
        property bool pressed: false
        onTriggered: {
            if (acts.window.curPage === null)
                return
            const page = acts.window.curPage
            const tab = page.pageTab
            if (tab.busyCount !== 0 || tab.autoFetchRunning)
                return
            if (!acts.stoppedYet(tab, tipLinkTimer))
                return
            // The failures raised the panel (デザイン規約 §git が言ったことを読む場所), and a press that finds it up
            // proves half the verb, so it is shut first.
            if (!tipLinkTimer.shut) {
                tipLinkTimer.shut = true
                if (page.commandsOpen)
                    page.shutCommands()
                return
            }
            if (!tipLinkTimer.pressed) {
                acts.topBar.fetchPointedAt = true
                if (!acts.topBar.fetchTipStanding)
                    return
                if (!acts.window.pressTipLink())
                    return
                tipLinkTimer.pressed = true
                return
            }
            // The panel is built as it is raised (`RepoPage.commandsSeat`), so wait for it to exist.
            if (!page.commandsOpen || page.commandsPane === null)
                return
            tipLinkTimer.stop()
            Harness.report("fetch_link open=" + page.commandsShown
                              + " lit=" + page.commandsPane.attention
                              + " suspended=" + tab.autoFetchSuspended)
            acts.window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=band-actions*: the band's three actions giving their words up as the window narrows (規約 §ウィンドウの縁).
    // The run asks for a *shape* and the band's own arithmetic names its width — which pixel is which shape depends on
    // the installed fonts.
    SampleTimer {
        id: actionsActTimer
        running: Harness.autoAct === "band-actions"
                 || Harness.autoAct === "band-actions-none"
                 || Harness.autoAct === "band-actions-fold"
                 || Harness.autoAct === "band-actions-alert"
                 || Harness.autoAct === "band-actions-stopped"
        property bool pushRequested: false
        property int askSeq: -1
        onTriggered: {
            // Every width is measured off the set's shared box (`TopBar.widestAction`), zero until the band has
            // loaded — and off push's wording, which the working tree decides (`publish` until the first status,
            // `push -f` after on a diverged branch).
            if (topBar.bandTabsWidth <= 0 || topBar.actionNaturalW <= Theme.railWidth
                    || window.curPage === null || !window.curPage.pageWt.loaded)
                return
            // The `!` before the window is sized: a real refusal from git (`--preset longnames` will not fast-forward,
            // as `push-retry` sets up). Sent through the page, because in this state the button is a hold and its
            // plain press is wired to nothing.
            if (Harness.autoAct === "band-actions-alert") {
                if (!actionsActTimer.pushRequested) {
                    window.curPage.pushNow()
                    actionsActTimer.pushRequested = true
                    return
                }
                if (!topBar.actionAlertShown) {
                    Awaited.at(Harness.autoAct, "alert")
                    return
                }
                // The refusal also brings its report down, and the bar grows into place (`NoticeBar.settled`):
                // photographed on the mark alone, it is half open. A refusal that raised no report would stand here to
                // the ceiling, named.
                if (!window.curPage.noticeCard.settled) {
                    Awaited.at(Harness.autoAct, "report")
                    return
                }
            }
            // The same corner on fetch, which is marked by the run of failures that stops its timer (`stoppedYet`).
            if (Harness.autoAct === "band-actions-stopped") {
                const tab = window.curPage.pageTab
                if (tab.busyCount !== 0 || tab.autoFetchRunning)
                    return
                if (!acts.stoppedYet(tab, actionsActTimer))
                    return
            }
            // Re-asked on every tick: push's wording can still change after the working tree loads, and the width is
            // measured off it. The target does not move with the window's width, so this closes no ring.
            const wanted = acts.actionsWidthFor(
                Harness.autoAct === "band-actions" ? Harness.autoActArg
                : Harness.autoAct === "band-actions-none" ? "whole" : "fold")
            if (Math.round(window.width) !== wanted) {
                window.width = wanted
                window.height = Math.ceil(window.floorHeight)
                return
            }
            if (topBar.width !== mainUi.width)
                return
            stop()
            // Which button the mark is on: `alert=` is the pair or-ed (`TopBar.actionAlertShown`).
            if (Harness.autoAct === "band-actions-stopped")
                Harness.report("band_stopped suspended=" + window.curPage.pageTab.autoFetchSuspended
                                  + " turned=" + topBar.fetchFrameTurned
                                  + " alert=" + topBar.actionAlertShown)
            acts.reportBandActions()
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
    function reportBandActions() {
        Harness.report(
            "band_actions fits=" + (window.width >= Math.ceil(window.floorWidth))
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
            + " w=" + window.width)
        window.finishAutoAct()
    }
}
