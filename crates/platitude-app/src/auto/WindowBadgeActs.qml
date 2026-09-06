pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The state badges' half of the band's PG_AUTO_ACT harness: the three the band raises at once, the one a graph
/// that stopped reading raises, and the one an old git raises.
///
/// Built by `WindowAutoActDriver` beside `WindowBandActs`, and split from it for length alone. Each verb stands
/// itself up on its own `running:`, so nothing dispatches into this file — it answers for the verbs it names and
/// no others (rules-refs/structure.md 窓側の自動化).
// An `Item` only because `QtObject` has no default property to hold the timers below; it draws nothing and is
// never given a size.
Item {
    id: acts

    required property var window
    required property TopBar topBar
    required property Item mainUi
    required property IdentityDialog identityDialog

    // PG_AUTO_ACT=badges-hover-early: the stand-in pointer put down before the band has ever placed the group. A hand
    // cannot be made to take that order — it arrives on a mark that is already somewhere — and it is the only order
    // `BandStateGroup.standInAsking` is there for: a pointer that cannot move raises the beat once, so the card has to
    // be asked for again when the row answers. `badges-hover` puts the pointer down on a group the band has long since
    // placed and passes whether that second asking exists or not.
    //
    // Written in the completion handler rather than on a tick because that is what puts it early: this harness is
    // built inside the window's own completion (`WindowHarness.screensUp`), and the row that places the group lays out
    // after it. Nothing else about the run differs — the guards, the report and the judgement below are shared.
    Component.onCompleted: {
        if (Harness.autoAct === "badges-hover-early")
            topBar.statePointedAt = true
    }

    // PG_AUTO_ACT=badges: all three of the band's state badges at once — the widest the band ever asks for, and the
    // floor is the only thing between that and a `>_` pushed off the end (デザイン規約 §ウィンドウの縁). The argument is
    // `<width>[:<tabs>]`: the window width, `floor` for the floor the three badges leave, and how many tabs the run
    // built the strip out of (`verify::repos::band_tab_count`).
    SampleTimer {
        id: badgesActTimer
        running: Harness.autoAct === "badges"
                 || Harness.autoAct === "badges-hover"
                 || Harness.autoAct === "badges-hover-early"
        /// Whether this run's report is of the card rather than of the band behind it. The two hover verbs differ in
        /// when the pointer goes down and in nothing after that.
        readonly property bool wantsCard: Harness.autoAct === "badges-hover"
                                          || Harness.autoAct === "badges-hover-early"
        property bool stateRequested: false
        property bool sizeRequested: false
        property int requestedWidth: -1
        onTriggered: {
            if (identityDialog.opened || topBar.bandTabsWidth <= 0
                    || window.curPage === null || !window.curPage.pageWt.loaded
                    || !topBar.opBadgeShown || !topBar.conflictBadgeShown
                    || !topBar.identityBadgeShown)
                return
            // The strip is the other half of what the band is short of (デザイン規約 §ウィンドウの縁), so a run that named a
            // count is not standing on its own fixture until the strip has every one of them: a group measured beside
            // three of the six tabs asked for is a measurement of a band nobody ran. Read off the strip's own count
            // rather than off the tabs having loaded — the pages come up in their own time, and the run the band
            // shares out is settled by the rows.
            const words = Harness.autoActArg.split(":")
            const wantedTabs = parseInt(words[1])
            if (!isNaN(wantedTabs) && topBar.bandTabCount !== wantedTabs)
                return
            const arg = words[0]
            const wantedW = parseInt(arg)
            const sized = arg === "floor" || (!isNaN(wantedW) && wantedW > 0)
            // The pointer, where headless cannot put one. Written to the same one property the real hover writes, so
            // the card cannot be opened by a road the hand does not have (app-ui.md).
            if (Harness.autoAct === "badges-hover" && !stateRequested) {
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
                // Which width brings on which of the group's three shapes is a question about the installed fonts, so
                // the run names the number and the report says the shape. Not held at the floor: the third shape sits
                // below what a hand can drag to today, and a shape nothing can photograph is a shape nobody can check.
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
                // Open, and holding what it says it holds. The card is measured after the fact — a `ColumnLayout`
                // settles on polish and has no `forceLayout()` of its own — so a report written in the frame a row
                // arrives in reads the card from before it (`BandStateCard.laidOut`).
                if (!topBar.stateCardOpen || !topBar.stateCardLaidOut)
                    return
            }
            stop()
            acts.reportBadges()
        }
    }
    /// Which rule painted the folded group's mark (規約 §状態: 色は最も 重い状態が決める). Read off the band's own colour, not off the
    /// conditions — recomputing the rule here would agree with itself whatever the band did.
    readonly property string stateTint: Qt.colorEqual(topBar.stateMarkColor, Theme.danger) ? "danger" : "warning"
    /// `fits=` leads, and the three badges are judged with it: a run where one never stood photographs a band that was
    /// never crowded.
    function reportBadges() {
        const floorW = Math.ceil(window.floorWidth)
        Harness.report(
            "badges fits=" + (window.width >= floorW)
            + " op=" + topBar.opBadgeShown
            + " conflicts=" + topBar.conflictBadgeShown
            + " identity=" + topBar.identityBadgeShown
            + " oldGit=" + topBar.oldGitBadgeShown
            // Which of the group's three shapes landed is `words=` / `mark=`; `cap=` is the width the badges were
            // narrowed to (-1 = none was).
            + " words=" + topBar.stateWordsShown
            + " mark=" + topBar.stateMarkShown
            + " tint=" + acts.stateTint
            + " cap=" + topBar.stateCapW
            + " groupW=" + topBar.stateGroupW
            + " badgeMin=" + topBar.stateBadgeMinW
            // What the strip was carrying while the group settled: the two of them share the band's shortfall, so a
            // cap read without the count beside it says nothing that can be repeated.
            + " tabs=" + topBar.bandTabCount
            + " tabCap=" + Math.round(topBar.tabTitleCap)
            + " tabMin=" + topBar.tabTitleMinW
            + " card=" + topBar.stateCardOpen
            + " rows=" + topBar.stateCardRows
            + " cardSize=" + topBar.stateCardSize
            // The band's own floor beside the window's: reading only the window's would not say whether it was this row
            // that set it.
            + " bandW=" + Math.ceil(topBar.floorWidth)
            + " floorW=" + floorW + " w=" + window.width
            + " tabsW=" + Math.round(topBar.bandTabsWidth)
            + " grabRun=" + Math.round(topBar.bandGrabRun))
        window.finishAutoAct()
    }

    // PG_AUTO_ACT=graph-stale / graph-stopped: the badge that says what is drawn is not this repository's history, and
    // the card line that says which of the two ways it came to be so. `graph-stale` fails the off-screen rebuild, so a
    // whole graph is left standing and goes out of date where it is; `graph-stopped` fails the stream, so the column
    // empties and the walk gives up part-way. Both go in at the walk itself (`GraphModel.failGraphPass`) — the state
    // needs a git that fails, and a demo repository has none in it.
    SampleTimer {
        id: staleActTimer
        running: Harness.autoAct === "graph-stale" || Harness.autoAct === "graph-stopped"
        property bool faultArmed: false
        onTriggered: {
            const page = window.curPage
            if (page === null)
                return
            const graph = page.pageGraph
            if (!staleActTimer.faultArmed) {
                // Not before a pass has landed: the fault has to be raised over a graph that was whole, or what the
                // badge is standing for is the opening rather than the failure (規約 §前提条件を完了判定に混ぜない).
                if (graph.loading || graph.finishCount <= 0 || graph.rowTotal <= 0)
                    return
                // **And whole means nothing is on its way to rebuild it.** The fault stands in time but not across
                // pass kinds, so a status landing after the arm flips the working-tree row and asks for a rebuild —
                // an off-screen *swap* pass — which undoes each verb in its own direction: the stream fault does
                // not touch a swap, so `graph-stopped`'s emptied column is walked full again, and the swap fault
                // fails that rebuild, so `graph-stale` freezes the pass with no working-tree row on it. The picture
                // is taken before either, so both verbs pass on a window the census then walks in the other state
                // (rules-refs/app-ui.md の同項). The rows agreeing with the status is what says the rebuild has
                // landed, off the one rule both sides ask (`graph::wip_row_stands`, as `WindowCensus.pageSettled`
                // reads it).
                if (!page.pageWt.loaded || !page.pageRefsLoaded
                        || graph.wipRow !== page.pageWt.wipRowStands)
                    return
                staleActTimer.faultArmed = true
                graph.failGraphPass(Harness.autoAct === "graph-stale" ? "swapping" : "streaming")
                return
            }
            if (!topBar.staleBadgeShown || graph.loading)
                return
            // The card, opened the one way the hand opens it (`badges-hover` の同じ 1 本): the line under the badge is
            // half of what this verb is for, and it is the only place the two states are told apart.
            topBar.statePointedAt = true
            if (!topBar.stateCardOpen)
                return
            stop()
            acts.reportStale()
        }
    }
    /// The five that are judged come first and in one run, because `must_say` matches them as one string. `badge=`
    /// leads; `stopped=` and `stale=` are the model's own two, which the badge folds into one — a run where they
    /// disagreed with it would be a badge standing for nothing. `tint=` is the rule reaching the paint (規約 §状態), and
    /// `card=` is there because the line under the badge is the only place the two states are told apart.
    function reportStale() {
        const graph = window.curPage.pageGraph
        Harness.report(
            "graph_stale badge=" + topBar.staleBadgeShown
            + " stopped=" + graph.failed
            + " stale=" + graph.stale
            + " tint=" + acts.stateTint
            + " card=" + topBar.stateCardOpen
            // Along for the read: whether the card had git's words to put on its line, how much of the graph was left
            // standing, and what the card actually laid out.
            + " words=" + (graph.error !== "")
            + " rows=" + graph.rowTotal
            + " cardRows=" + topBar.stateCardRows)
        window.finishAutoAct()
    }

    // PG_AUTO_ACT=old-git / old-git-card / old-git-fold. Nothing here stages the state — the run is handed a git that
    // answers `--version` with an older number (`verify-ui --old-git`), so the badge is answering a real reading of a
    // real program.
    SampleTimer {
        id: oldGitActTimer
        running: Harness.autoAct === "old-git"
                 || Harness.autoAct === "old-git-card"
                 || Harness.autoAct === "old-git-fold"
        property bool stateRequested: false
        property bool sizeRequested: false
        property int requestedWidth: -1
        onTriggered: {
            // The pointer, where headless cannot put one — the same one property the real hover writes (app-ui.md).
            if (topBar.bandTabsWidth <= 0)
                return
            // `-fold` brings its own width: the shape it is for is a folded group with nothing red in it — the only
            // place the mark's colour is the mark's whole meaning (規約 §状態).
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
            // The pointer goes down on a tick of its own, and whether the card came up is read on the next one — this
            // verb's whole report is that reading. A pointer that went down before the version badge did is not a card
            // that never opens: a stand-in pointer is answered again when the badge arrives and when the row places
            // the group (`BandStateGroup.standInAsking`).
            if (Harness.autoAct === "old-git-card" && !stateRequested) {
                topBar.statePointedAt = true
                stateRequested = true
                return
            }
            if (Harness.autoAct === "old-git-card" && !topBar.stateCardOpen)
                return
            stop()
            // `version=` says which git answered — a run whose shim never got onto PATH photographs an ordinary window,
            // and an ordinary window photographs well.
            Harness.report(
                "old-git badge=" + topBar.oldGitBadgeShown
                + " card=" + topBar.stateCardOpen
                + " rows=" + topBar.stateCardRows
                + " words=" + topBar.stateWordsShown
                + " mark=" + topBar.stateMarkShown
                + " tint=" + acts.stateTint
                + " cap=" + topBar.stateCapW
                + " version=" + AppBackend.gitVersion
                + " min=" + AppBackend.minimumGit
                + " w=" + window.width)
            window.finishAutoAct()
        }
    }
}
