pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import platitude
import platitude.ui

/// The band's half of the window's PGG_AUTO_ACT harness: the row itself, the ☰ menu, and the three actions giving
/// their words up as the window narrows. The state badges are the other half (`WindowBadgeActs`).
///
/// Built by `WindowAutoActDriver`, which is what `Main` builds when a verb was given; what these verbs act
/// on is handed down below, one property per part of the window they reach into.
// An `Item` only because `QtObject` has no default property to hold the timers below; it is a
// sizeless holder.
Item {
    id: acts

    required property var window
    required property TopBar topBar
    required property WindowChrome chrome
    required property Item mainUi
    required property IdentityDialog identityDialog

    // Capture the communication ring from a real busy edge: the tab has to have entered the operation, so a fast
    // child cannot clear it before the image callback runs. Latched here — the edge is often
    // shorter than a sampler's beat, and a run that missed it would wait out its watchdog on a band that had already
    // been through what it was there to photograph.
    Connections {
        target: acts.topBar.curPage ? acts.topBar.curPage.pageTab : null
        function onBusyOpChanged() {
            const op = acts.topBar.curPage.pageTab.busyOp
            if (Harness.autoAct === "force-push-hold" && op === "push")
                acts.topBar.holdPushBusy = true
            if (Harness.autoAct === "fetch-busy" && op === "fetch")
                acts.topBar.holdFetchBusy = true
        }
    }

    SampleTimer {
        running: Harness.autoAct === "force-push-hold"
        property bool requested: false
        onTriggered: {
            if (topBar.curPage === null || topBar.pushMode !== "diverged")
                return
            if (!requested) {
                requested = true
                topBar.completePushHold()
                return
            }
            if (!topBar.holdPushBusy)
                return
            stop()
            Harness.report("push_hold mode=" + topBar.pushMode + " busy=" + topBar.holdPushBusy)
            window.finishAutoAct()
        }
    }

    // The same edge on the button the wait is drawn for: bare, unframed, and the one a timer can start on its own. The
    // fetch itself is fired by the page's driver; all this waits for is the edge above.
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

    // PGG_AUTO_ACT=fetch-tip: what the fetch button offers a pointer, on each side of the one thing that decides it.
    // The argument names which side this run is — `off` is the repository with no remote (`--preset noremote`), where
    // the button is dim and says nothing, and `on` is any repository that has one. Neither run proves anything alone.
    //
    // The two wait for different things because "not pressable" has two reasons and only one of them is the subject:
    // the live side waits for the button itself, the dim side for a repository that has finished landing with nothing
    // running on it, so a band read before the remotes arrived cannot pass for either.
    SampleTimer {
        running: Harness.autoAct === "fetch-tip"
        onTriggered: {
            // The graph as well as the refs, for the picture: the band settles first, and a
            // half-drawn page under a settled band is a worse photograph of it.
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

    // PGG_AUTO_ACT=stash-state: which of the working tree's four answers the Stash button settled on. The argument names
    // the one this repository is meant to give (`ready` / `clean` / `conflicts` / `unborn`), and the run waits for the
    // band to say it — the reading is built out of a HEAD and four counts that land over several drains, so a band read
    // too early would answer `unborn` for every repository on its way open.
    //
    // Four runs, because a dim button frames the same whichever refusal put it there: only the set says that the
    // conditions are told apart at all (デザイン規約 §変更を退避する).
    SampleTimer {
        running: Harness.autoAct === "stash-state"
        onTriggered: {
            // The graph as well, for the picture — the same reason `fetch-tip` waits on it.
            // `empty` has no rows at all, so that repository is judged settled on its working tree alone.
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

    // PGG_AUTO_ACT=band: numbers say it — the headless platform draws no window buttons of its own, so
    // a band that lost the grab run or pushed its buttons off the end looks fine in the picture.
    SampleTimer {
        id: bandActTimer
        running: Harness.autoAct === "band"
        onTriggered: {
            // No tabs is a valid laid-out band. Read readiness from the window and bar
            // themselves, then let `tabsW=0` describe the empty output.
            if (!window.visible || mainUi.width <= 0 || topBar.width <= 0)
                return
            stop()
            Harness.report(
                "band merged=" + window.captionMerged
                + " systemTitleBar=" + AppBackend.systemTitleBar
                + " grabRun=" + topBar.bandGrabRun + " dividerRun=" + topBar.bandDividerRun
                + " buttonsX=" + topBar.bandButtonsX
                + " width=" + topBar.width
                + " tabsW=" + topBar.bandTabsWidth
                + " rightMargin=" + topBar.bandRightMargin
                // The panel's three names and the upstream in brackets after the branch. A photograph cannot be read
                // for either: a name cut at both ends looks like a short name, and an empty bracket looks like no
                // bracket.
                + " names=" + topBar.opsNames + " upstream=" + topBar.branchUpstream)
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=ops-panel: the panel standing, once the repository it names has landed — the verb the state
    // matrix is photographed with, run once per repository shape and once per width. An argument that parses as a
    // number is a width to stand the window at; the window's own floor may refuse it, which is why the width the
    // run came out at rides back in the line.
    //
    // **Everything a picture cannot be read for comes back in the line**: a name cut at both ends looks like a short
    // name, a button that gave its word up looks like a narrow band, and a badge that never stood looks like a
    // window with nothing the matter.
    SampleTimer {
        id: opsPanelTimer
        running: Harness.autoAct === "ops-panel"
        onTriggered: {
            if (!window.visible || topBar.width <= 0 || window.curPage === null
                    || !window.curPage.pageRefsLoaded || !window.curPage.pageWt.loaded)
                return
            const tab = window.curPage.pageTab
            if (tab.busyCount !== 0 || tab.autoFetchRunning)
                return
            // Asked for again until it takes or is refused: the width is the window's to allow, and the panel is
            // laid out again after it moves.
            const wanted = parseInt(Harness.autoActArg)
            if (!isNaN(wanted) && wanted > 0 && Math.round(window.width) !== wanted
                    && wanted >= Math.ceil(window.floorWidth)) {
                window.width = wanted
                return
            }
            if (topBar.width !== mainUi.width)
                return
            stop()
            Harness.report(
                "ops_panel settled=true width=" + Math.round(window.width)
                + " floor=" + Math.ceil(window.floorWidth)
                + " names=" + topBar.opsNames + " upstream=" + topBar.branchUpstream
                + " namesCut=" + topBar.opsNameCut
                + " folded=" + topBar.actionsFolded + " cut=" + topBar.actionWordCut
                + " alert=" + topBar.actionAlertShown
                + " push=" + topBar.pushMode + " stash=" + topBar.stashMode
                + " badges=" + [topBar.opBadgeShown, topBar.conflictBadgeShown,
                                topBar.identityBadgeShown, topBar.oldGitBadgeShown,
                                topBar.staleBadgeShown].join(","))
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=ops-stand / ops-stand-repos / ops-stand-copies / ops-branch: the panel's own two doors — the one
    // the repository's name opens, either tier of it, and the one the branch's name opens.
    //
    // **The rows are counted as well as photographed.** Each card is assembled from a listing that arrives after the
    // tab does, and a card holding nothing looks in a picture exactly like a card holding rows that were all left
    // out (`AppMenu.offeredRows`). A run waits for the listing rather than for a beat: the refs and the copies come
    // back on separate reads, and which of them is last is the machine's business, not the verb's.
    SampleTimer {
        id: opsDoorTimer
        running: Harness.autoAct === "ops-stand" || Harness.autoAct === "ops-stand-repos"
                 || Harness.autoAct === "ops-stand-copies" || Harness.autoAct === "ops-branch"
        /// Which step this run is on: the card, then the tier inside it.
        property int opened: 0
        onTriggered: {
            // **Both listings, not a beat**: the refs and the copies come back on separate reads, and a card asked
            // for between them is a card with rows still missing. Every repository has its own copy in the second
            // of them, so a zero there is a listing that has not landed rather than a repository without one.
            if (!window.visible || topBar.width <= 0 || window.curPage === null
                    || !window.curPage.pageRefsLoaded
                    || window.curPage.pageWorktrees.total <= 0)
                return
            const branchDoor = Harness.autoAct === "ops-branch"
            if (opsDoorTimer.opened === 0) {
                opsDoorTimer.opened = 1
                // **Asked for once, and reported however it went.** `offerHere` turns away a card with nothing in
                // it, which is the right answer for a repository that has nowhere else to stand — and a run that
                // kept asking would sit out its watchdog on a window that was never going to open one (observed).
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
            stop()
            Harness.report(
                "ops_door open=" + (branchDoor ? topBar.branchMenuOpen : topBar.standMenuOpen)
                + " tier=" + topBar.standDoor
                + " repos=" + topBar.standRepoRows
                + " copies=" + topBar.standCopyRows
                + " branches=" + topBar.branchMenuRows)
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=app-menu / app-menu-reclick: the ☰ pressed once and left standing, or pressed twice.
    //
    // Numbers as well as the card's picture, because the two things that were reported broken are both invisible to
    // it: a second press that reopens the card frames exactly like one that never closed it, and which side of the
    // band owns the grab run is not drawn at all. `strip=none` is the run handed back to the scene, which is what
    // makes a press on the band's empty stretch reach the card (`WindowChrome.captionYielded`) — on a build where the
    // band is not the window's title bar there is no strip either way, and `merged=` says which run this was.
    SampleTimer {
        id: appMenuActTimer
        running: Harness.autoAct === "app-menu" || Harness.autoAct === "app-menu-reclick"
        /// How many presses have gone in. The second one has to land on a card that was observed standing, or the
        /// gesture being reported is not the one a hand makes.
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

    /// Whether the run of failures that stops the timer is over, asking for one more if it is not.
    ///
    /// **One at a time, and each waited out.** A fetch is answered a drain after it is asked for, so three fired in a
    /// row are counted as one — the seq the timer holds is the write this verb's last fetch was asked at. The remote
    /// has to be one git cannot reach (`--preset unreachable`), or this never comes true.
    function stoppedYet(tab, timer) {
        if (tab.autoFetchSuspended)
            return true
        // **Not before the repository is open.** A fetch asked of a tab still opening is refused by core before git
        // is reached ("no repository is open"), and that refusal suspends the fetches like any other — so the panel
        // the verb then opens holds no row at all, which is a different picture from the one it is about (measured
        // under a 16-wide gate: `fetch-tip-link` photographed an empty panel and moved its census line). The same
        // reading the page makes before it asks anything of the tab (`repoTab.state`).
        if (tab.state !== "open")
            return false
        if (timer.askSeq >= 0 && tab.writeSeq <= timer.askSeq)
            return false
        timer.askSeq = tab.writeSeq
        tab.fetch("")
        return false
    }

    // PGG_AUTO_ACT=fetch-tip-link: the word inside the fetch button's tip that is a place to go, pressed — and the
    // panel it names coming up marked (デザイン規約 §hover のツールチップ — 送った先は名乗る).
    //
    // The failures come first; then the hand goes on the button and the tip is waited out, because a tip that is not
    // standing has no word in it to press.
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
            // The failures raised the panel on the way here (§git が言ったことを読む場所), and a press that finds it
            // already up proves only half of what this verb is about. So it goes down first, by the reader's own
            // hand — which is the state the link is worth having: somebody who put it away and was then sent back.
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
            // The panel is built into its seat as it is raised (`RepoPage.commandsSeat`), so what says the press
            // arrived is the panel standing and wearing the mark.
            if (!page.commandsOpen || page.commandsPane === null)
                return
            tipLinkTimer.stop()
            Harness.report("fetch_link open=" + page.commandsShown
                              + " lit=" + page.commandsPane.attention
                              + " suspended=" + tab.autoFetchSuspended)
            acts.window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=band-actions / band-actions-fold: the band's three actions giving their words up as the window
    // narrows (規約 §ウィンドウの縁). The width the run asks for is a *shape*, because which pixel
    // brings on which shape is a question about the installed fonts; the band's own arithmetic names the width.
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
            // The box the set shares is settled once the band has loaded (`TopBar.widestAction`), and every width
            // here is measured off it — asked for before that, the run would size the window against a box of zero.
            //
            // The working tree as well: what push says is read off it (`publish` before the first status is in, and
            // `push -f` after, on a branch that has diverged), and the wordings are what is being cut here. A band
            // shot before that photographs the shape of a wording nobody will see.
            if (topBar.bandTabsWidth <= 0 || topBar.actionNaturalW <= Theme.railWidth
                    || window.curPage === null || !window.curPage.pageWt.loaded)
                return
            // The `!` before the window is sized: it stands on a go that came back refused, and a band shot before
            // that answer landed frames a band with nothing the matter — which is what an ordinary band looks like.
            //
            // The refusal is a real one, from git: `--preset diverged` is a branch git will not fast-forward, and the
            // plain push is the road to hearing so (`push-retry` の仕込み). Sent through the page, because in this
            // state the button is a **hold** — its plain press is not wired to anything, and force is the go that
            // lands.
            if (Harness.autoAct === "band-actions-alert") {
                if (!actionsActTimer.pushRequested) {
                    window.curPage.pushNow()
                    actionsActTimer.pushRequested = true
                    return
                }
                if (!topBar.actionAlertShown)
                    return
            }
            // The same corner on the other button. Fetch has no go to be refused — what puts a mark there is a run of
            // failures, and the third of them stops the timer (`stoppedYet`).
            if (Harness.autoAct === "band-actions-stopped") {
                const tab = window.curPage.pageTab
                if (tab.busyCount !== 0 || tab.autoFetchRunning)
                    return
                if (!acts.stoppedYet(tab, actionsActTimer))
                    return
            }
            // Asked for again on every tick. **What the shape is worth is measured off the wording
            // the button is saying**, and push's wording arrives with the readings that decide it — a width settled
            // on the tick the working tree loaded is one measured for `push`, and the band it lands on is saying
            // `push -f` (measured, the run came out a whole step further down than it asked for). Re-asking
            // costs nothing and closes no ring: the target is read off the wordings and the floor, neither of which
            // the window's width moves.
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
            // Which button the corner mark belongs to. The band's own `alert=` is the pair or-ed together
            // (`TopBar.actionAlertShown`), so on its own it cannot tell this run from the one that photographs the
            // refused push — and the two put their mark on different corners of different marks.
            if (Harness.autoAct === "band-actions-stopped")
                Harness.report("band_stopped suspended=" + window.curPage.pageTab.autoFetchSuspended
                                  + " turned=" + topBar.fetchFrameTurned
                                  + " alert=" + topBar.actionAlertShown)
            acts.reportBandActions()
        }
    }
    /// The width a shape sits at. `fold` is the floor the words are finished by — the window's own, measured with the
    /// left list open — and the default is the middle of the cap's travel, which is the one
    /// stretch where every wording is cut and none is given up. A number passed through is somebody naming a pixel.
    function actionsWidthFor(arg) {
        const wanted = parseInt(arg)
        if (!isNaN(wanted) && wanted > 0)
            return wanted
        // The two ends, and the step between them, are the panel's own arithmetic — asked for rather than worked out
        // again here, so a run cannot pass a build whose schedule has moved (`TopBar.actionsWholeAt`).
        //
        // `fold` is the widest panel that has given every wording up; `whole` the narrowest that still says all three
        // in full. A band that folded early photographs as a band that is merely narrow, and no other width can tell
        // the two apart.
        if (arg === "fold")
            return topBar.actionsFoldAt
        // Never under the window's own floor: with a panel this wide the wordings are whole there already, and a run
        // that asked for less would photograph a window no hand can make.
        if (arg === "whole")
            return Math.max(topBar.actionsWholeAt, Math.ceil(window.floorWidth))
        // The default: the narrowest cell that still has a word in it — the last step before the marks.
        //
        // **How long the stretch where the wordings are cut is depends on the installed
        // fonts, and for a four-letter command it can be nothing at all** (measured, Linux: `push` and
        // `…` + two characters measure the same 27px, so `push -f` goes from whole to given up with no cut in
        // between). Landing on the last labelled cell is a shape that exists on every machine, and `cut=` rides along
        // to say whether this one had a cut in it.
        return topBar.actionsFoldAt + 1
    }
    /// What the three came out as. `cut=` and `folded=` are the two the picture cannot answer on its own: a wording
    /// that ends in `…` of its own reads like an elided one, and a band photographed at one width says nothing about
    /// the width the shape was supposed to change at.
    function reportBandActions() {
        Harness.report(
            "band_actions fits=" + (window.width >= Math.ceil(window.floorWidth))
            // The four that are judged lead and stand together: only neighbours can be caught in one substring.
            + " folded=" + topBar.actionsFolded
            + " cut=" + topBar.actionWordCut
            + " alert=" + topBar.actionAlertShown
            + " cap=" + topBar.actionCapW
            + " natural=" + topBar.actionNaturalW
            + " foldAt=" + topBar.actionFoldW
            // How long the stretch where the wordings are cut is on this machine: the cell at which the widest
            // wording on the band starts being cut, against the one at which the set gives up. They can be the same
            // number (規約 §ウィンドウの縁「省略の段は短い」), and no picture says so.
            + " cutAt=" + topBar.actionCutW
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
