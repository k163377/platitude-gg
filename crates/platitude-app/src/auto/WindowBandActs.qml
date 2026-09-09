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
// An `Item` only because `QtObject` has no default property to hold the timers below; it draws nothing and is
// never given a size.
Item {
    id: acts

    required property var window
    required property TopBar topBar
    required property WindowChrome chrome
    required property Item mainUi
    required property IdentityDialog identityDialog

    // Capture the communication ring from a real busy edge: the tab has to have entered the operation, so a fast
    // child cannot clear it before the image callback runs. Latched here rather than read back — the edge is often
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
                              + " fails=" + topBar.fetchFails + " framed=" + topBar.fetchFramed)
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
            // The graph as well as the refs, for the picture rather than for the answer: the band settles first, and a
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
            // The graph as well, for the picture rather than for the answer — the same reason `fetch-tip` waits on it.
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

    // PGG_AUTO_ACT=band: numbers rather than a screenshot — the headless platform draws no window buttons of its own, so
    // a band that lost the grab run or pushed its buttons off the end looks fine in the picture.
    SampleTimer {
        id: bandActTimer
        running: Harness.autoAct === "band"
        onTriggered: {
            // No tabs is a valid laid-out band, not an unanswered one. Read readiness from the window and bar
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
                + " rightMargin=" + topBar.bandRightMargin)
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

    // PGG_AUTO_ACT=band-actions / band-actions-fold: the band's three actions giving their words up as the window
    // narrows (規約 §ウィンドウの縁). The width the run asks for is a *shape* rather than a number, because which pixel
    // brings on which shape is a question about the installed fonts; the band's own arithmetic names the width.
    SampleTimer {
        id: actionsActTimer
        running: Harness.autoAct === "band-actions"
                 || Harness.autoAct === "band-actions-none"
                 || Harness.autoAct === "band-actions-fold"
                 || Harness.autoAct === "band-actions-alert"
        property bool pushRequested: false
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
            // plain push is the road to hearing so (`push-retry` の仕込み). Sent through the page rather than through
            // the button, because in this state the button is a **hold** — its plain press is not wired to anything,
            // and force is the go that lands rather than the one that is refused.
            if (Harness.autoAct === "band-actions-alert") {
                if (!actionsActTimer.pushRequested) {
                    window.curPage.pushNow()
                    actionsActTimer.pushRequested = true
                    return
                }
                if (!topBar.actionAlertShown)
                    return
            }
            // Asked for again on every tick rather than once. **What the shape is worth is measured off the wording
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
        const floor = Math.ceil(window.openFloorWidth)
        if (arg === "fold")
            return floor
        // The other end: the narrowest window at which nothing has given yet. A band that folded early would
        // photograph as a band that is merely narrow, and no other width can tell the two apart.
        if (arg === "whole")
            return Math.ceil(floor + 3 * (topBar.actionNaturalW - Theme.railWidth))
        // The narrowest cell that still has a word in it — the last step before the marks.
        //
        // Not the middle of the stretch where the wordings are being cut: **how long that stretch is depends on the
        // installed fonts, and for a four-letter command it can be nothing at all** (measured, Linux: `push` and
        // `…` + two characters measure the same 27px, so `push -f` goes from whole to given up with no cut in
        // between). Landing on the last labelled cell is a shape that exists on every machine, and `cut=` rides along
        // to say whether this one had a cut in it.
        //
        // Rounded **up**, so the cap lands at or above the fold rather than a third of a pixel under it.
        return Math.ceil(floor + 3 * (topBar.actionFoldW - Theme.railWidth))
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
