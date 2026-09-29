pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The state badges' half of the band's PGG_AUTO_ACT harness: the three the band raises at once (and all six), the one
/// a graph that stopped reading raises, the one an old git raises, and the one files for a missing Git LFS raise.
/// Split from `WindowBandActs` for length alone; each verb stands itself up on its own `running:` (rules-refs/structure.md「窓側の自動化は動詞が自分で `running:` に立つ」).
// An `Item` only because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    required property var window
    required property TopBar topBar
    required property Item mainUi
    required property IdentityDialog identityDialog

    // PGG_AUTO_ACT=badges-hover-early: the stand-in pointer put down before the band has placed the group — an order a
    // hand cannot take, and the one `BandStateGroup.standInAsking` is for (a pointer that cannot move raises the beat
    // once, so the card is asked for again when the row answers); `badges-hover` passes with or without it. Written
    // in the completion handler, the first moment this verb exists (`WindowHarness.begun`); the rest is shared.
    Component.onCompleted: {
        if (Harness.autoAct === "badges-hover-early")
            topBar.statePointedAt = true
    }

    // PGG_AUTO_ACT=badges: all three of the band's state badges at once — the widest the band asks for, which only the
    // floor keeps from pushing `>_` off the end (デザイン規約 §ウィンドウの縁). The argument is `<width>[:<tabs>]`: the
    // window width (or `floor`), and how many tabs the run built the strip out of (`verify::repos::band_tab_count`).
    // `badges-all` / `badges-all-hover`: every one of the six — the run's git is old and has no Git LFS
    // (`verify-ui` stages both by the verb's name), and the graph is made stale here the way `graph-stale` makes it.
    SampleTimer {
        id: badgesActTimer
        running: Harness.autoAct === "badges"
                 || Harness.autoAct === "badges-hover"
                 || Harness.autoAct === "badges-hover-early"
                 || badgesActTimer.wantsAll
        /// Whether the report is of the card or of the band behind it; the two hover verbs differ only in when the
        /// pointer goes down.
        readonly property bool wantsCard: Harness.autoAct === "badges-hover"
                                          || Harness.autoAct === "badges-hover-early"
                                          || Harness.autoAct === "badges-all-hover"
        readonly property bool wantsAll: Harness.autoAct === "badges-all" || Harness.autoAct === "badges-all-hover"
        property bool stateRequested: false
        property bool sizeRequested: false
        property bool staleArmed: false
        property int requestedWidth: -1
        onTriggered: {
            if (identityDialog.opened || topBar.bandTabsWidth <= 0
                    || window.curPage === null || !window.curPage.pageWt.loaded
                    || !topBar.opBadgeShown || !topBar.conflictBadgeShown
                    || !topBar.identityBadgeShown)
                return
            if (badgesActTimer.wantsAll) {
                const page = window.curPage
                // Armed over a page that has settled, as `graph-stale` arms it (`staleActTimer` says why).
                if (!badgesActTimer.staleArmed) {
                    if (page.pageGraph.loading || page.pageGraph.finishCount <= 0 || !PageSettled.settled(page))
                        return
                    badgesActTimer.staleArmed = true
                    page.pageGraph.failGraphPass("swapping")
                    return
                }
                if (!topBar.staleBadgeShown || !topBar.lfsBadgeShown || !topBar.oldGitBadgeShown
                        || page.pageGraph.loading)
                    return
            }
            // The strip shares the band's shortfall, so a run that named a tab count waits until the strip holds
            // every one — read off the strip's own count, since the pages come up in their own time.
            const words = Harness.autoActArg.split(":")
            const wantedTabs = parseInt(words[1])
            if (!isNaN(wantedTabs) && topBar.bandTabCount !== wantedTabs)
                return
            const arg = words[0]
            const wantedW = parseInt(arg)
            const sized = arg === "floor" || (!isNaN(wantedW) && wantedW > 0)
            // The pointer, through the one property the real hover writes.
            if ((Harness.autoAct === "badges-hover" || Harness.autoAct === "badges-all-hover") && !stateRequested) {
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
                // Which shape a width brings on depends on the installed fonts, so the run names the width and the
                // report says the shape. Allowed below the floor: the third shape sits below what a hand can drag to.
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
            if (badgesActTimer.wantsCard) {
                // Open and laid out: a `ColumnLayout` settles on polish with no `forceLayout()`, so a report written
                // in the frame a row arrives reads the card from before it (`BandStateCard.laidOut`).
                if (!topBar.stateCardOpen || !topBar.stateCardLaidOut)
                    return
            }
            stop()
            acts.reportBadges()
        }
    }
    /// Which rule painted the folded group's mark (デザイン規約 §ウィンドウの縁「色は最も重い状態が決める」), read off
    /// the band's own colour — recomputing the rule here would agree with itself whatever the band did.
    readonly property string stateTint: Qt.colorEqual(topBar.stateMarkColor, Theme.danger) ? "danger" : "warning"
    /// `fits=` leads, and the three badges are judged with it: a run where one never stood photographs an uncrowded
    /// band. `stale=` / `lfs=` ride between, so the three's line stays one string for `badges` to be judged by.
    function reportBadges() {
        const floorW = Math.ceil(window.floorWidth)
        Harness.report(
            "badges fits=" + (window.width >= floorW)
            + " stale=" + topBar.staleBadgeShown
            + " lfs=" + topBar.lfsBadgeShown
            + " op=" + topBar.opBadgeShown
            + " conflicts=" + topBar.conflictBadgeShown
            + " identity=" + topBar.identityBadgeShown
            + " oldGit=" + topBar.oldGitBadgeShown
            // Whether the band was short at all (the arithmetic is `BandStateShare`'s, asked without a window).
            // Judged: a window with room for every word photographs like a band that gave the crowd room.
            + " narrowed=" + (topBar.stateCapW >= 0)
            // Which of the group's three shapes landed is `words=` / `mark=`; `cap=` is the width the badges were
            // narrowed to (-1 = none was).
            + " words=" + topBar.stateWordsShown
            + " mark=" + topBar.stateMarkShown
            // The shape above was decided on a shadow band's room (`TopBar.bandShadowAgrees`); a cell it does not
            // mirror is room the words do not have, and they run past the group's own cell.
            + " shadow=" + topBar.bandShadowAgrees
            + " tint=" + acts.stateTint
            // A folded group as wide as its mark and no wider (`BandStateGroup.markFitted`).
            + " fitted=" + topBar.stateMarkFitted
            + " cap=" + topBar.stateCapW
            + " groupW=" + topBar.stateGroupW
            + " badgeMin=" + topBar.stateBadgeMinW
            // The strip shares the band's shortfall, so a cap read without the tab count cannot be repeated.
            + " tabs=" + topBar.bandTabCount
            + " tabCap=" + Math.round(topBar.tabTitleCap)
            + " tabMin=" + topBar.tabTitleMinW
            + " card=" + topBar.stateCardOpen
            + " rows=" + topBar.stateCardRows
            + " cardSize=" + topBar.stateCardSize
            // The band's own floor beside the window's, which alone would not say whether this row set it.
            + " bandW=" + Math.ceil(topBar.floorWidth)
            + " floorW=" + floorW + " w=" + window.width
            + " tabsW=" + Math.round(topBar.bandTabsWidth)
            + " grabRun=" + Math.round(topBar.bandGrabRun)
            // The count the card's sentence says, from the model the badge reads.
            + " lfsCount=" + window.curPage.pageWt.lfsNeeded)
        window.finishAutoAct()
    }

    // PGG_AUTO_ACT=graph-stale / graph-stopped: the badge that says what is drawn is not this repository's history, and
    // the card line that says which way. `graph-stale` fails the off-screen rebuild (a whole graph goes out of date);
    // `graph-stopped` fails the stream (the column empties). Both go in at the walk (`GraphModel.failGraphPass`).
    SampleTimer {
        id: staleActTimer
        running: Harness.autoAct === "graph-stale" || Harness.autoAct === "graph-stopped"
        property bool faultArmed: false
        property bool badgeSaid: false
        onTriggered: {
            const page = window.curPage
            if (page === null)
                return
            const graph = page.pageGraph
            if (!staleActTimer.faultArmed) {
                // Only over a graph that was whole, or the badge stands for the opening
                // (rules-refs/app-ui.md「動詞の前提条件は入力を出す枝で読む」).
                if (graph.loading || graph.finishCount <= 0 || graph.rowTotal <= 0)
                    return
                // And only once nothing is on its way to rebuild it, asked of the census's own rule (`PageSettled`):
                // a status landing after the arm asks for a swap rebuild that undoes either fault, and the census then
                // walks the other state (rules-refs/app-ui.md「`STALE GRAPH` の動確は `graph-stale` / `graph-stopped`」).
                if (!PageSettled.settled(page))
                    return
                staleActTimer.faultArmed = true
                // Asked once; the harness keeps it up, since what undoes it is invisible from here
                // (`harness::fail_graph_pass`).
                graph.failGraphPass(Harness.autoAct === "graph-stale" ? "swapping" : "streaming")
                Harness.report("graph_stale step=armed")
                return
            }
            if (!topBar.staleBadgeShown || graph.loading)
                return
            if (!staleActTimer.badgeSaid) {
                staleActTimer.badgeSaid = true
                Harness.report("graph_stale step=badge")
            }
            // The card, opened the way `badges-hover` opens it (`reportStale` says why).
            topBar.statePointedAt = true
            if (!topBar.stateCardOpen)
                return
            stop()
            acts.reportStale()
        }
    }
    // PGG_AUTO_ACT=wip-landing: where the page lands when the pass it opened on carries every other working copy's row
    // and none of its own. The run starts with the hold raised (`--preset carried`, `xtask::verify::child`): which of
    // the walk and the first status comes first is the scheduler's.
    //
    // A window verb, because the page never settles while the hold is up (`PageSettled` refuses rows that disagree
    // with the status), so a page verb would wait out its watchdog (`AutoActDriver`'s baseline).
    //
    // Every copy's row wears the same all-zero id, so a reader that took row 0 for ours stands the page on a copy
    // nobody asked for — `early=` / `earlyCopy=`, read before the hold comes down.
    SampleTimer {
        id: wipLandingTimer
        running: Harness.autoAct === "wip-landing"
        property bool read: false
        property bool early: false
        property bool earlyCopy: false
        onTriggered: {
            const page = acts.window.curPage
            if (!page)
                return
            const graph = page.pageGraph
            if (!wipLandingTimer.read) {
                // The raced pass: the tree is dirty, a pass has finished, and its top row is a neighbour copy's
                // (`GraphModel.carriedTop`). Over a pass with no all-zero row, row 0 reads correctly and the verb
                // would pass without the misreading ever in front of the page.
                if (!page.pageWt.loaded || !page.pageWt.wipRowStands
                        || graph.loading || graph.finishCount <= 0 || graph.wipRow || !graph.carriedTop)
                    return
                wipLandingTimer.read = true
                wipLandingTimer.early = page.wipShown
                wipLandingTimer.earlyCopy = page.carriedPath !== ""
                // `held=` is the hold answering for itself: otherwise a run could pass on the ordinary order.
                if (!graph.letTheWorkingTreeRowThrough()) {
                    stop()
                    Harness.report("wip_landing held=false")
                    window.finishAutoAct()
                }
                return
            }
            if (!graph.wipRow || !PageSettled.settled(page))
                return
            stop()
            Harness.report("wip_landing held=true otherTop=true early=" + wipLandingTimer.early
                              + " earlyCopy=" + wipLandingTimer.earlyCopy
                              + " wip=" + page.wipShown
                              + " copy=" + (page.carriedPath !== "")
                              + " row=" + page.selectedRow)
            window.finishAutoAct()
        }
    }

    /// The five judged fields come first and together: `must_say` matches them as one string. `stopped=` / `stale=`
    /// are the model's two, which the badge folds into one; `card=` because the card's line is the only place the two
    /// states are told apart.
    function reportStale() {
        const graph = window.curPage.pageGraph
        Harness.report(
            "graph_stale badge=" + topBar.staleBadgeShown
            + " stopped=" + graph.failed
            + " stale=" + graph.stale
            + " tint=" + acts.stateTint
            + " card=" + topBar.stateCardOpen
            // Not judged, along for the read.
            + " words=" + (graph.error !== "")
            + " rows=" + graph.rowTotal
            + " cardRows=" + topBar.stateCardRows)
        window.finishAutoAct()
    }

    // PGG_AUTO_ACT=old-git / old-git-card / old-git-fold: the run is handed a git that answers `--version` with an
    // older number (`verify-ui --old-git`).
    SampleTimer {
        id: oldGitActTimer
        running: Harness.autoAct === "old-git"
                 || Harness.autoAct === "old-git-card"
                 || Harness.autoAct === "old-git-fold"
        property bool stateRequested: false
        property bool sizeRequested: false
        property int requestedWidth: -1
        onTriggered: {
            if (topBar.bandTabsWidth <= 0)
                return
            // `-fold` brings its own width: a folded group with nothing red in it, where the mark's colour is its whole
            // meaning (デザイン規約 §状態).
            const arg = Harness.autoAct === "old-git-fold"
                        ? "floor" : Harness.autoActArg
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
            // The pointer, through the one property the real hover writes, goes down on a tick of its own and the card
            // is read on the next. A pointer down before the badge is answered again when the badge arrives and when
            // the row places the group (`BandStateGroup.standInAsking`).
            if (Harness.autoAct === "old-git-card" && !stateRequested) {
                topBar.statePointedAt = true
                stateRequested = true
                return
            }
            if (Harness.autoAct === "old-git-card" && !topBar.stateCardOpen)
                return
            stop()
            // `version=` says which git answered: a run whose shim never got onto PATH photographs an ordinary window.
            Harness.report(
                "old-git badge=" + topBar.oldGitBadgeShown
                + " card=" + topBar.stateCardOpen
                + " rows=" + topBar.stateCardRows
                + " words=" + topBar.stateWordsShown
                + " mark=" + topBar.stateMarkShown
                + " tint=" + acts.stateTint
                // Whether the folded group is its mark's width: kept at its words' room it leaves empty band beside
                // the mark, which a picture reads as a fat grab run.
                + " fitted=" + topBar.stateMarkFitted
                + " cap=" + topBar.stateCapW
                + " version=" + AppBackend.gitVersion
                + " min=" + AppBackend.minimumGit
                + " w=" + window.width)
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=no-lfs / no-lfs-card: the run's git answers `git lfs` as a git without Git LFS does
    // (`verify-ui --no-lfs`, staged by the verb's name), over a preset with files for LFS pending (`--preset lfs`).
    // The argument is a width, or `floor` for the folded mark, as `old-git`'s is.
    SampleTimer {
        id: noLfsActTimer
        running: Harness.autoAct === "no-lfs" || Harness.autoAct === "no-lfs-card"
        property bool stateRequested: false
        property bool sizeRequested: false
        property int requestedWidth: -1
        onTriggered: {
            const page = window.curPage
            if (topBar.bandTabsWidth <= 0 || page === null)
                return
            const arg = Harness.autoActArg
            const wantedW = parseInt(arg)
            if (!noLfsActTimer.sizeRequested && arg === "floor") {
                noLfsActTimer.requestedWidth = Math.ceil(window.floorWidth)
                window.width = noLfsActTimer.requestedWidth
                window.height = Math.ceil(window.floorHeight)
                noLfsActTimer.sizeRequested = true
                return
            } else if (!noLfsActTimer.sizeRequested && !isNaN(wantedW) && wantedW > 0) {
                noLfsActTimer.requestedWidth = wantedW
                window.width = noLfsActTimer.requestedWidth
                noLfsActTimer.sizeRequested = true
                return
            }
            if (!page.pageWt.loaded || !topBar.lfsBadgeShown)
                return
            if (noLfsActTimer.sizeRequested
                    && (topBar.width !== mainUi.width
                        || (arg === "floor"
                            ? window.width < noLfsActTimer.requestedWidth
                            : Math.round(window.width) !== noLfsActTimer.requestedWidth)))
                return
            // Down on a tick of its own and read on a later one, as `old-git-card`'s pointer is.
            if (Harness.autoAct === "no-lfs-card" && !stateRequested) {
                topBar.statePointedAt = true
                stateRequested = true
                return
            }
            if (Harness.autoAct === "no-lfs-card" && (!topBar.stateCardOpen || !topBar.stateCardLaidOut))
                return
            stop()
            // `count=` is the model's, which the card's sentence says: the picture cannot be read for a number. The
            // judged fields run together, `rows=` last among them (a closed card stands no rows).
            Harness.report(
                "no-lfs badge=" + topBar.lfsBadgeShown
                + " card=" + topBar.stateCardOpen
                + " count=" + page.pageWt.lfsNeeded
                + " words=" + topBar.stateWordsShown
                + " mark=" + topBar.stateMarkShown
                + " tint=" + acts.stateTint
                + " fitted=" + topBar.stateMarkFitted
                + " rows=" + topBar.stateCardRows
                + " cap=" + topBar.stateCapW
                + " w=" + window.width)
            window.finishAutoAct()
        }
    }
}
