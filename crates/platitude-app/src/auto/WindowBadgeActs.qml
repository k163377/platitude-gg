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

    // PG_AUTO_ACT=badges: all three of the band's state badges at once — the widest the band ever asks for, and the
    // floor is the only thing between that and a `>_` pushed off the end (デザイン規約 §ウィンドウの縁). The argument is the window
    // width; `floor` puts it down on the floor the three badges leave.
    SampleTimer {
        id: badgesActTimer
        running: AppBackend.autoAct === "badges"
                 || AppBackend.autoAct === "badges-hover"
        property bool stateRequested: false
        property bool sizeRequested: false
        property int requestedWidth: -1
        onTriggered: {
            if (identityDialog.opened || topBar.bandTabsWidth <= 0
                    || window.curPage === null || !window.curPage.pageWt.loaded
                    || !topBar.opBadgeShown || !topBar.conflictBadgeShown
                    || !topBar.identityBadgeShown)
                return
            const arg = AppBackend.autoActArg
            const wantedW = parseInt(arg)
            const sized = arg === "floor" || (!isNaN(wantedW) && wantedW > 0)
            // The pointer, where headless cannot put one. Written to the same one property the real hover writes, so
            // the card cannot be opened by a road the hand does not have (app-ui.md).
            if (AppBackend.autoAct === "badges-hover" && !stateRequested) {
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
            if (AppBackend.autoAct === "badges-hover") {
                if (!topBar.stateCardOpen)
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
        AppBackend.report(
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
        running: AppBackend.autoAct === "graph-stale" || AppBackend.autoAct === "graph-stopped"
        property bool faultArmed: false
        onTriggered: {
            if (window.curPage === null)
                return
            const graph = window.curPage.pageGraph
            if (!staleActTimer.faultArmed) {
                // Not before a pass has landed: the fault has to be raised over a graph that was whole, or what the
                // badge is standing for is the opening rather than the failure (規約 §前提条件を完了判定に混ぜない).
                if (graph.loading || graph.finishCount <= 0 || graph.rowTotal <= 0)
                    return
                staleActTimer.faultArmed = true
                graph.failGraphPass(AppBackend.autoAct === "graph-stale" ? "swapping" : "streaming")
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
        AppBackend.report(
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
        running: AppBackend.autoAct === "old-git"
                 || AppBackend.autoAct === "old-git-card"
                 || AppBackend.autoAct === "old-git-fold"
        property bool stateRequested: false
        property bool sizeRequested: false
        property int requestedWidth: -1
        onTriggered: {
            // The pointer, where headless cannot put one — the same one property the real hover writes (app-ui.md).
            if (topBar.bandTabsWidth <= 0)
                return
            // `-fold` brings its own width: the shape it is for is a folded group with nothing red in it — the only
            // place the mark's colour is the mark's whole meaning (規約 §状態).
            const arg = AppBackend.autoAct === "old-git-fold"
                        ? "floor" : AppBackend.autoActArg
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
            // The card decides whether it has any rows on the pointer edge. Pointing before the version badge exists
            // would ask an empty group once and leave `pointedAt` true, so no later edge could reopen it when the badge
            // arrives.
            if (AppBackend.autoAct === "old-git-card" && !stateRequested) {
                topBar.statePointedAt = true
                stateRequested = true
                return
            }
            if (AppBackend.autoAct === "old-git-card" && !topBar.stateCardOpen)
                return
            stop()
            // `version=` says which git answered — a run whose shim never got onto PATH photographs an ordinary window,
            // and an ordinary window photographs well.
            AppBackend.report(
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
