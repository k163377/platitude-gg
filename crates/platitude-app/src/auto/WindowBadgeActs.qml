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
    // hand cannot take, and the one `BandStateGroup.asking` is for (a pointer that cannot move raises the beat
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
    // The group's card is raised only by a hand on the marks or `badges-hover`'s stand-in: a card up in a run that
    // put no hand there is Qt's own hover, and the census moves with it — so every change is noted, with what asked.
    Connections {
        target: topBar
        function onStateCardOpenChanged() {
            Awaited.note("state_card open=" + topBar.stateCardOpen + " pointedAt=" + topBar.statePointedAt)
        }
    }
    // The graph made stale (`armed`, the six only), the pointer down (`pointed`, the two hover verbs that put it there)
    // and the window sized (`sized`, a width or `floor` in the argument), in that order; each stage reads the band
    // again.
    WindowStage {
        id: badgesActTimer
        name: "badges"
        order: {
            const out = []
            if (badgesActTimer.wantsAll)
                out.push("armed")
            if (badgesActTimer.points)
                out.push("pointed")
            if (badgesActTimer.sizes)
                out.push("sized")
            return out
        }
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
        /// Whether this run puts the pointer down itself (`badges-hover-early` put it down as the verb was built).
        readonly property bool points: Harness.autoAct === "badges-hover" || Harness.autoAct === "badges-all-hover"
        /// The width the argument names, `floor` for the window's own floor.
        readonly property string widthArg: Harness.autoActArg.split(":")[0]
        readonly property bool sizes: badgesActTimer.widthArg === "floor"
                                      || (!isNaN(parseInt(badgesActTimer.widthArg))
                                          && parseInt(badgesActTimer.widthArg) > 0)
        property int requestedWidth: -1
        onTriggered: {
            const op = badgesActTimer
            acts.bandBeat()
            const page = window.curPage
            // The page whole (`PageSettled`): the worktree row's tally comes with the rows agreeing with the status.
            const terms = {
                "dialog": !identityDialog.opened,
                "band": topBar.bandTabsWidth > 0,
                "settled": PageSettled.settledUnder(window),
                "op": topBar.opBadgeShown,
                "conflicts": topBar.conflictBadgeShown,
                "identity": topBar.identityBadgeShown
            }
            if (op.wantsAll && op.at("start")) {
                // Armed over a page that has settled, as `graph-stale` arms it (`staleActTimer` says why).
                terms["graph"] = page !== null && !page.pageGraph.loading && page.pageGraph.finishCount > 0
                terms["page"] = PageSettled.settled(page)
                if (!Awaited.all("badges_arm", terms))
                    return
                op.input("armed", () => {
                    page.pageGraph.failGraphPass("swapping")
                })
                return
            }
            if (op.wantsAll) {
                terms["stale"] = topBar.staleBadgeShown
                terms["lfs"] = topBar.lfsBadgeShown
                terms["oldGit"] = topBar.oldGitBadgeShown
                terms["graph"] = page !== null && !page.pageGraph.loading
            }
            // The strip shares the band's shortfall, so a run that named a tab count waits until the strip holds
            // every one — read off the strip's own count, since the pages come up in their own time.
            const wantedTabs = parseInt(Harness.autoActArg.split(":")[1])
            terms["tabs"] = isNaN(wantedTabs) || topBar.bandTabCount === wantedTabs
            const arg = op.widthArg
            const pointed = !op.points || !(op.at("start") || op.at("armed"))
            if (!pointed || (op.sizes && !op.at("sized"))) {
                if (!Awaited.all("badges_band", terms))
                    return
                // The pointer, through the one property the real hover writes.
                if (!pointed && !op.input("pointed", () => {
                        topBar.statePointedAt = true
                    }))
                    return
                if (!op.sizes)
                    return
                // Which shape a width brings on depends on the installed fonts, so the run names the width and the
                // report says the shape. Allowed below the floor: the third shape sits below what a hand can drag to.
                op.input("sized", () => {
                    op.requestedWidth = arg === "floor" ? Math.ceil(window.floorWidth) : parseInt(arg)
                    window.width = op.requestedWidth
                    if (arg === "floor")
                        window.height = Math.ceil(window.floorHeight)
                })
                return
            }
            terms["sized"] = !op.sizes
                || (topBar.width === mainUi.width
                    && (arg === "floor" ? window.width >= op.requestedWidth
                                        : Math.round(window.width) === op.requestedWidth))
            const before = Object.keys(terms).every(term => terms[term])
            // The card open and laid out where one is wanted: a `ColumnLayout` settles on polish with no
            // `forceLayout()`, so a report written in the frame a row arrives reads the card from before it
            // (`BandStateCard.laidOut`). The band's shape is read every beat (`bandLaidOutAfterFrames`).
            terms["card"] = !op.wantsCard || (topBar.stateCardOpen && topBar.stateCardLaidOut)
            terms["laid"] = before && acts.bandLaidOutAfterFrames(op)
            terms["page"] = PageSettled.windowSettled(window)
            if (!Awaited.all("badges_ready", terms))
                return
            op.complete(() => acts.badgesLine())
        }
    }
    // The band's two rows (`TopBar`'s `bandRow` and its shadow `bandAsked`) lay out on polish, so in the beat the last
    // badge arrives, or the window reaches its width, the group still has the width the row gave it before — on macOS
    // the identity badge's alone, read as a shadow that disagrees. Every report of the band's shape waits two swapped
    // frames once what it waits for is there, as the left menu's does (`WindowFrameActs.menuReadAfterFrames`) — there
    // the whole time: a beat that turned back before asking (a badge gone again, the card not laid out) starts the
    // count over, or the report would go out in the very beat the shape came back.
    /// Whether the beat before this one asked about the band's shape, and whether this one has.
    property bool bandAskedBefore: false
    property bool bandAskedNow: false
    /// The run of asking beats the frames are counted over: a new one begins at an ask the beat before did not make.
    property int bandRun: 0
    /// Called at the top of each beat of a verb that reads the band's shape.
    function bandBeat() {
        acts.bandAskedBefore = acts.bandAskedNow
        acts.bandAskedNow = false
    }
    /// Whether two frames have swapped since the band's shape was first asked about on this run of beats
    /// (`StageTimer.drawnSince`, under `op`'s stage).
    function bandLaidOutAfterFrames(op) {
        if (!acts.bandAskedBefore && !acts.bandAskedNow)
            acts.bandRun++
        acts.bandAskedNow = true
        return op.drawnSince("band_laid_out#" + acts.bandRun)
    }
    /// Which rule painted the folded group's mark (デザイン規約 §ウィンドウの縁「色は最も重い状態が決める」), read off
    /// the band's own colour — recomputing the rule here would agree with itself whatever the band did.
    readonly property string stateTint: Qt.colorEqual(topBar.stateMarkColor, Theme.danger) ? "danger" : "warning"
    /// `fits=` leads, and the three badges are judged with it: a run where one never stood photographs an uncrowded
    /// band. `stale=` / `lfs=` ride between, so the three's line stays one string for `badges` to be judged by.
    function badgesLine() {
        const floorW = Math.ceil(window.floorWidth)
        const wordsOff = topBar.stateCardWordsOff()
        return "badges fits=" + (window.width >= floorW)
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
            // A folded group as wide as its mark and no wider (`BandStateGroup.markFitted`), its frame a badge's depth
            // (`markDeep`).
            + " fitted=" + topBar.stateMarkFitted
            + " deep=" + topBar.stateMarkDeep
            + " cap=" + topBar.stateCapW
            + " groupW=" + topBar.stateGroupW
            + " badgeMin=" + topBar.stateBadgeMinW
            // The strip shares the band's shortfall, so a cap read without the tab count cannot be repeated.
            + " tabs=" + topBar.bandTabCount
            + " tabCap=" + Math.round(topBar.tabTitleCap)
            + " tabMin=" + topBar.tabTitleMinW
            + " card=" + topBar.stateCardOpen
            + " rows=" + topBar.stateCardRows
            // Each row's sentence on its badge's words' baseline (`BandStateCard.wordsOffLevel`), to the half pixel its
            // whole-pixel seat may round by, with how far off the worst one stood.
            + " level=" + (wordsOff <= 0.5)
            + " cardSize=" + topBar.stateCardSize
            + " levelOff=" + wordsOff.toFixed(2)
            // The band's own floor beside the window's, which alone would not say whether this row set it.
            + " bandW=" + Math.ceil(topBar.floorWidth)
            + " floorW=" + floorW + " w=" + window.width
            + " tabsW=" + Math.round(topBar.bandTabsWidth)
            + " grabRun=" + Math.round(topBar.bandGrabRun)
            // The count the card's sentence says, from the model the badge reads.
            + " lfsCount=" + window.curPage.pageWorktree.lfsNeeded
    }

    // PGG_AUTO_ACT=graph-stale / graph-stopped: the badge that says what is drawn is not this repository's history, and
    // the card line that says which way. `graph-stale` fails the off-screen rebuild (a whole graph goes out of date);
    // `graph-stopped` fails the stream (the column empties). Both go in at the walk (`GraphModel.failGraphPass`).
    WindowStage {
        id: staleActTimer
        name: "graph_stale"
        order: ["armed", "badge"]
        running: Harness.autoAct === "graph-stale" || Harness.autoAct === "graph-stopped"
        onTriggered: {
            const op = staleActTimer
            const page = window.curPage
            // Over a settled page (`PageSettled`): a status landing after the arm asks for a swap rebuild that undoes
            // either fault, and the census then walks the other state
            // (rules-refs/app-ui.md「`STALE GRAPH` の動確は `graph-stale` / `graph-stopped`」).
            const settled = PageSettled.settledUnder(window)
            const graph = settled ? page.pageGraph : null
            if (op.at("start")) {
                // Only over a graph that was whole, or the badge stands for the opening
                // (rules-refs/app-ui.md「動詞の前提条件は入力を出す枝で読む」).
                if (!Awaited.all("stale_act_ready", {
                        "page": settled,
                        "whole": graph !== null && !graph.loading && graph.finishCount > 0 && graph.rowTotal > 0 }))
                    return
                // Asked once; the harness keeps it up, since what undoes it is invisible from here
                // (`harness::fail_graph_pass`).
                op.input("armed", () => {
                    graph.failGraphPass(Harness.autoAct === "graph-stale" ? "swapping" : "streaming")
                }, () => "graph_stale step=armed")
                return
            }
            const badge = graph !== null && topBar.staleBadgeShown && !graph.loading
            if (op.at("armed")) {
                if (!Awaited.all("stale_act_badge", { "page": settled, "badge": badge }))
                    return
                op.advance("badge", () => "graph_stale step=badge")
                // The card, opened the way `badges-hover` opens it (`staleLine` says why).
                topBar.statePointedAt = true
                return
            }
            if (badge)
                topBar.statePointedAt = true
            if (!Awaited.all("stale_act_card", { "page": settled, "badge": badge, "card": topBar.stateCardOpen }))
                return
            op.complete(() => acts.staleLine())
        }
    }
    // PGG_AUTO_ACT=wip-landing: where the page lands when the pass it opened on carries every other worktree's row
    // and none of its own. The run starts with the hold raised (`--preset carried`, `xtask::verify::child`): which of
    // the walk and the first status comes first is the scheduler's.
    //
    // A window verb, because the page never settles while the hold is up (`PageSettled` refuses rows that disagree
    // with the status), so a page verb would wait out its watchdog (`AutoActDriver`'s baseline).
    //
    // Every worktree's row wears the same all-zero id, so a reader that took row 0 for ours stands the page on a
    // worktree nobody asked for — `early=` / `earlyWorktree=`, read before the hold comes down.
    WindowStage {
        id: wipLandingTimer
        name: "wip_landing"
        order: ["let_through"]
        running: Harness.autoAct === "wip-landing"
        property bool early: false
        property bool earlyWorktree: false
        /// Whether the hold answered the letting through (`GraphModel.letTheWorktreeRowThrough`).
        property bool held: false
        onTriggered: {
            const op = wipLandingTimer
            const page = acts.window.curPage
            const graph = page !== null ? page.pageGraph : null
            if (op.at("start")) {
                // Not the settled page: `early=` is read while the hold keeps the page arriving, by design. The raced
                // pass: the tree is dirty, a pass has finished, and its top row is a neighbour worktree's
                // (`GraphModel.carriedTop`). Over a pass with no all-zero row, row 0 reads correctly and the verb
                // would pass without the misreading ever in front of the page.
                if (!Awaited.all("wip_landing_ready", {
                        "ready": page,
                        "raced": page !== null && page.pageWorktree.loaded && page.pageWorktree.wipRowStands
                            && !graph.loading && graph.finishCount > 0 && !graph.wipRow && graph.carriedTop }))
                    return
                if (!op.input("let_through", () => {
                        op.early = page.wipShown
                        op.earlyWorktree = page.carriedPath !== ""
                        op.held = graph.letTheWorktreeRowThrough()
                    }))
                    return
                // `held=` is the hold answering for itself: otherwise a run could pass on the ordinary order.
                if (!op.held)
                    op.complete(() => "wip_landing held=false")
                return
            }
            if (!Awaited.all("wip_landing_landed", { "ready": page,
                                                     "landed": graph !== null && graph.wipRow,
                                                     "page": PageSettled.settled(page) }))
                return
            op.complete(() => {
                const landed = acts.window.curPage
                return "wip_landing held=true otherTop=true early=" + op.early
                    + " earlyWorktree=" + op.earlyWorktree
                    + " wip=" + landed.wipShown
                    + " worktree=" + (landed.carriedPath !== "")
                    + " row=" + landed.selectedRow
            })
        }
    }

    /// The five judged fields come first and together: `must_say` matches them as one string. `stopped=` / `stale=`
    /// are the model's two, which the badge folds into one; `card=` because the card's line is the only place the two
    /// states are told apart.
    function staleLine() {
        const graph = window.curPage.pageGraph
        return "graph_stale badge=" + topBar.staleBadgeShown
            + " stopped=" + graph.failed
            + " stale=" + graph.stale
            + " tint=" + acts.stateTint
            + " card=" + topBar.stateCardOpen
            // Not judged, along for the read.
            + " words=" + (graph.error !== "")
            + " rows=" + graph.rowTotal
            + " cardRows=" + topBar.stateCardRows
    }

    // PGG_AUTO_ACT=old-git / old-git-card / old-git-fold: the run is handed a git that answers `--version` with an
    // older number (`verify-ui --old-git`). The window sized (`sized`, a width or `floor` in the argument), then the
    // pointer down (`pointed`, `-card`).
    WindowStage {
        id: oldGitActTimer
        name: "old_git"
        order: (oldGitActTimer.sizes ? ["sized"] : []).concat(oldGitActTimer.points ? ["pointed"] : [])
        running: Harness.autoAct === "old-git"
                 || Harness.autoAct === "old-git-card"
                 || Harness.autoAct === "old-git-fold"
        /// `-fold` brings its own width: a folded group with nothing red in it, where the mark's colour is its whole
        /// meaning (デザイン規約 §状態).
        readonly property string widthArg: Harness.autoAct === "old-git-fold" ? "floor" : Harness.autoActArg
        readonly property bool sizes: oldGitActTimer.widthArg === "floor"
                                      || (!isNaN(parseInt(oldGitActTimer.widthArg))
                                          && parseInt(oldGitActTimer.widthArg) > 0)
        readonly property bool points: Harness.autoAct === "old-git-card"
        property int requestedWidth: -1
        onTriggered: {
            const op = oldGitActTimer
            acts.bandBeat()
            const arg = op.widthArg
            const terms = {
                "band": topBar.bandTabsWidth > 0,
                "page": PageSettled.settledUnder(window)
            }
            if (op.sizes && op.at("start")) {
                if (!Awaited.all("old_git_ready", terms))
                    return
                op.input("sized", () => {
                    op.requestedWidth = arg === "floor" ? Math.ceil(window.floorWidth) : parseInt(arg)
                    window.width = op.requestedWidth
                    if (arg === "floor")
                        window.height = Math.ceil(window.floorHeight)
                })
                return
            }
            terms["version"] = AppBackend.gitVersion !== ""
            terms["badge"] = topBar.oldGitBadgeShown
            terms["sized"] = !op.sizes
                || (topBar.width === mainUi.width
                    && (arg === "floor" ? window.width >= op.requestedWidth
                                        : Math.round(window.width) === op.requestedWidth))
            // The pointer, through the one property the real hover writes, goes down on a tick of its own and the card
            // is read on the next. A pointer down before the badge is answered again when the badge arrives and when
            // the row places the group (`BandStateGroup.asking`).
            if (op.points && !op.at("pointed")) {
                if (!Awaited.all("old_git_ready", terms))
                    return
                op.input("pointed", () => {
                    topBar.statePointedAt = true
                })
                return
            }
            terms["card"] = !op.points || topBar.stateCardOpen
            terms["laid"] = Object.keys(terms).every(term => terms[term]) && acts.bandLaidOutAfterFrames(op)
            if (!Awaited.all("old_git_ready", terms))
                return
            // `version=` says which git answered: a run whose shim never got onto PATH photographs an ordinary window.
            op.complete(() => "old-git badge=" + topBar.oldGitBadgeShown
                + " card=" + topBar.stateCardOpen
                + " rows=" + topBar.stateCardRows
                + " words=" + topBar.stateWordsShown
                + " mark=" + topBar.stateMarkShown
                + " tint=" + acts.stateTint
                // Whether the folded group is its mark's width: kept at its words' room it leaves empty band beside
                // the mark, which a picture reads as a fat grab run. Whether its frame is a badge's depth: one the
                // band's depth lies along the band's edges, and a cropped picture reads that as the band's own line.
                + " fitted=" + topBar.stateMarkFitted
                + " deep=" + topBar.stateMarkDeep
                + " cap=" + topBar.stateCapW
                + " version=" + AppBackend.gitVersion
                + " min=" + AppBackend.minimumGit
                + " w=" + window.width)
        }
    }

    // PGG_AUTO_ACT=no-lfs / no-lfs-card: the run's git answers `git lfs` as a git without Git LFS does
    // (`verify-ui --no-lfs`, staged by the verb's name), over a preset with files for LFS pending (`--preset lfs`).
    // The argument is a width, or `floor` for the folded mark, as `old-git`'s is. The window sized (`sized`), then
    // the pointer down (`pointed`, `-card`), as `old-git` does.
    WindowStage {
        id: noLfsActTimer
        name: "no_lfs"
        order: (noLfsActTimer.sizes ? ["sized"] : []).concat(noLfsActTimer.points ? ["pointed"] : [])
        running: Harness.autoAct === "no-lfs" || Harness.autoAct === "no-lfs-card"
        readonly property bool sizes: Harness.autoActArg === "floor"
                                      || (!isNaN(parseInt(Harness.autoActArg)) && parseInt(Harness.autoActArg) > 0)
        readonly property bool points: Harness.autoAct === "no-lfs-card"
        property int requestedWidth: -1
        onTriggered: {
            const op = noLfsActTimer
            acts.bandBeat()
            const page = window.curPage
            const arg = Harness.autoActArg
            const terms = {
                "band": topBar.bandTabsWidth > 0,
                "ready": page !== null
            }
            if (op.sizes && op.at("start")) {
                if (!Awaited.all("no_lfs_band", terms))
                    return
                op.input("sized", () => {
                    op.requestedWidth = arg === "floor" ? Math.ceil(window.floorWidth) : parseInt(arg)
                    window.width = op.requestedWidth
                    if (arg === "floor")
                        window.height = Math.ceil(window.floorHeight)
                })
                return
            }
            terms["loaded"] = page !== null && page.pageWorktree.loaded
            terms["badge"] = topBar.lfsBadgeShown
            terms["sized"] = !op.sizes
                || (topBar.width === mainUi.width
                    && (arg === "floor" ? window.width >= op.requestedWidth
                                        : Math.round(window.width) === op.requestedWidth))
            // Down on a tick of its own and read on a later one, as `old-git-card`'s pointer is.
            if (op.points && !op.at("pointed")) {
                if (!Awaited.all("no_lfs_band", terms))
                    return
                op.input("pointed", () => {
                    topBar.statePointedAt = true
                })
                return
            }
            const before = Object.keys(terms).every(term => terms[term])
            terms["card"] = !op.points || (topBar.stateCardOpen && topBar.stateCardLaidOut)
            terms["laid"] = before && acts.bandLaidOutAfterFrames(op)
            terms["page"] = PageSettled.windowSettled(window)
            if (!Awaited.all("no_lfs_ready", terms))
                return
            // `count=` is the model's, which the card's sentence says: the picture cannot be read for a number. The
            // judged fields run together, `rows=` last among them (a closed card stands no rows).
            op.complete(() => "no-lfs badge=" + topBar.lfsBadgeShown
                + " card=" + topBar.stateCardOpen
                + " count=" + window.curPage.pageWorktree.lfsNeeded
                + " words=" + topBar.stateWordsShown
                + " mark=" + topBar.stateMarkShown
                + " tint=" + acts.stateTint
                + " fitted=" + topBar.stateMarkFitted
                + " deep=" + topBar.stateMarkDeep
                + " rows=" + topBar.stateCardRows
                + " cap=" + topBar.stateCapW
                + " w=" + window.width)
        }
    }
}
