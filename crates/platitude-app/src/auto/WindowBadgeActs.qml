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
        /// What the band shows, read now. `armed`: the stage arming the graph waits for the graph whole and the page;
        /// otherwise the badges the verb raises, the strip's tabs and the window at its width (`sized`).
        function terms(armed) {
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
            if (armed) {
                // Armed over a page that has settled, as `graph-stale` arms it (`staleActTimer` says why).
                terms["graph"] = page !== null && !page.pageGraph.loading && page.pageGraph.finishCount > 0
                terms["page"] = PageSettled.settled(page)
                return terms
            }
            if (badgesActTimer.wantsAll) {
                terms["stale"] = topBar.staleBadgeShown
                terms["lfs"] = topBar.lfsBadgeShown
                terms["oldGit"] = topBar.oldGitBadgeShown
                terms["graph"] = page !== null && !page.pageGraph.loading
            }
            // The strip shares the band's shortfall, so a run that named a tab count waits until the strip holds
            // every one — read off the strip's own count, since the pages come up in their own time.
            const wantedTabs = parseInt(Harness.autoActArg.split(":")[1])
            terms["tabs"] = isNaN(wantedTabs) || topBar.bandTabCount === wantedTabs
            return terms
        }
        /// The window at the width the run asked for (`requestedWidth`, kept as it was asked), the band across it.
        function sized() {
            return !badgesActTimer.sizes
                || (topBar.width === mainUi.width
                    && (badgesActTimer.widthArg === "floor"
                        ? window.width >= badgesActTimer.witnessed("requestedWidth")
                        : Math.round(window.width) === badgesActTimer.witnessed("requestedWidth")))
        }
        /// …and the card open and laid out where one is wanted, and the page: what the completion stands on.
        function landed() {
            const terms = badgesActTimer.terms(false)
            terms["sized"] = badgesActTimer.sized()
            terms["card"] = !badgesActTimer.wantsCard || (topBar.stateCardOpen && topBar.stateCardLaidOut)
            terms["page"] = PageSettled.windowSettled(window)
            return terms
        }
        onTriggered: {
            const op = badgesActTimer
            const page = window.curPage
            if (op.wantsAll && op.at("start")) {
                if (!Awaited.all("badges_arm", op.terms(true)))
                    return
                op.input("armed", () => {
                    page.pageGraph.failGraphPass("swapping")
                })
                return
            }
            const arg = op.widthArg
            const pointed = !op.points || !(op.at("start") || op.at("armed"))
            if (!pointed || (op.sizes && !op.at("sized"))) {
                if (!Awaited.all("badges_band", op.terms(false)))
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
                    window.width = op.witness("requestedWidth",
                                              arg === "floor" ? Math.ceil(window.floorWidth) : parseInt(arg))
                    if (arg === "floor")
                        window.height = Math.ceil(window.floorHeight)
                })
                return
            }
            // The card open and laid out where one is wanted: a `ColumnLayout` settles on polish with no
            // `forceLayout()`, so a report written in the frame a row arrives reads the card from before it
            // (`BandStateCard.laidOut`). The band's shape is read every beat (`acts.bandStood`).
            const terms = op.landed()
            const before = Object.keys(terms).every(term => term === "card" || term === "page" || terms[term])
            terms["laid"] = acts.bandStood(op, before ? acts.badgesLine() : "") && before
            if (!Awaited.all("badges_ready", terms))
                return
            op.complete({ "name": "badges_shaped", "when": () => op.landed(), "report": () => acts.badgesLine() })
        }
    }
    // The band's two rows (`TopBar`'s `bandRow` and its shadow `bandAsked`) lay out on polish, so in the beat the last
    // badge arrives, or the window reaches its width, the group still has the width the row gave it before — on macOS
    // the identity badge's alone, read as a shadow that disagrees. Every report of the band's shape waits for the
    // shape to stand across a polish pass once what it waits for is there, as the left menu's does
    // (`WindowFrameActs.menuStood`) — asked the whole time: a beat that turned back (a badge gone again, the card not
    // laid out) reads as another value and starts the wait over, or the report would go out in the very beat the
    // shape came back.
    /// Whether the band's shape — `line`, the report's reading of it, "" while what it waits for is not all there —
    /// has stood across a polish pass (`StageTimer.still`, under `op`'s stage). Asked every beat.
    function bandStood(op, line) {
        return op.still("band_laid", line)
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
        /// The fault put in, the badge up, then the pointer on it until its card stands.
        order: ["armed", "badge", "pointed"]
        running: Harness.autoAct === "graph-stale" || Harness.autoAct === "graph-stopped"
        /// The settled page's graph out of date and its badge up, read now.
        function badgeTerms() {
            const settled = PageSettled.settledUnder(window)
            const graph = settled ? window.curPage.pageGraph : null
            return { "page": settled, "badge": graph !== null && topBar.staleBadgeShown && !graph.loading }
        }
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
            if (op.at("armed")) {
                if (!Awaited.all("stale_act_badge", op.badgeTerms()))
                    return
                op.advance("badge", () => "graph_stale step=badge")
            }
            // The card, opened the way `badges-hover` opens it (`staleLine` says why): the pointer put down again each
            // beat the badge stands and the card is not up.
            if (op.at("badge")) {
                if (!op.badgeTerms().badge || !op.input("pointed", () => {
                        topBar.statePointedAt = true
                        return topBar.stateCardOpen
                    }))
                    return
            }
            op.complete({
                "name": "stale_act_card",
                "when": () => {
                    const terms = op.badgeTerms()
                    terms["card"] = topBar.stateCardOpen
                    return terms
                },
                "report": () => acts.staleLine() })
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
                // Where the page stood before the hold came down, and whether the hold answered the letting through
                // (`GraphModel.letTheWorktreeRowThrough`), kept.
                if (!op.input("let_through", () => {
                        op.witness("early", page.wipShown)
                        op.witness("earlyWorktree", page.carriedPath !== "")
                        op.witness("held", graph.letTheWorktreeRowThrough())
                    }))
                    return
            }
            // `held=` is the hold answering for itself: otherwise a run could pass on the ordinary order.
            if (!op.witnessed("held")) {
                op.complete({ "name": "wip_landing_unheld", "when": () => ({}),
                              "report": () => "wip_landing held=false" })
                return
            }
            op.complete({
                "name": "wip_landing_landed",
                "when": () => {
                    const page = acts.window.curPage
                    return { "ready": page !== null,
                             "landed": page !== null && page.pageGraph.wipRow,
                             "page": PageSettled.settled(page) }
                },
                "report": () => {
                    const landed = acts.window.curPage
                    return "wip_landing held=true otherTop=true early=" + op.witnessed("early")
                        + " earlyWorktree=" + op.witnessed("earlyWorktree")
                        + " wip=" + landed.wipShown
                        + " worktree=" + (landed.carriedPath !== "")
                        + " row=" + landed.selectedRow
                } })
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
        /// What the band shows, read now: loaded and settled; `badged`: with the git answered, its badge up and the
        /// window at the width the run asked for (`requestedWidth`, kept as it was asked); `carded`: and the card
        /// where one is wanted.
        function terms(badged, carded) {
            const terms = {
                "band": topBar.bandTabsWidth > 0,
                "page": PageSettled.settledUnder(window)
            }
            if (!badged)
                return terms
            const arg = oldGitActTimer.widthArg
            terms["version"] = AppBackend.gitVersion !== ""
            terms["badge"] = topBar.oldGitBadgeShown
            terms["sized"] = !oldGitActTimer.sizes
                || (topBar.width === mainUi.width
                    && (arg === "floor" ? window.width >= oldGitActTimer.witnessed("requestedWidth")
                                        : Math.round(window.width) === oldGitActTimer.witnessed("requestedWidth")))
            if (carded)
                terms["card"] = !oldGitActTimer.points || topBar.stateCardOpen
            return terms
        }
        // `version=` says which git answered: a run whose shim never got onto PATH photographs an ordinary window.
        function line() {
            return "old-git badge=" + topBar.oldGitBadgeShown
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
                + " w=" + window.width
        }
        onTriggered: {
            const op = oldGitActTimer
            const arg = op.widthArg
            if (op.sizes && op.at("start")) {
                if (!Awaited.all("old_git_ready", op.terms(false, false)))
                    return
                op.input("sized", () => {
                    window.width = op.witness("requestedWidth",
                                              arg === "floor" ? Math.ceil(window.floorWidth) : parseInt(arg))
                    if (arg === "floor")
                        window.height = Math.ceil(window.floorHeight)
                })
                return
            }
            // The pointer, through the one property the real hover writes, goes down on a tick of its own and the card
            // is read on the next. A pointer down before the badge is answered again when the badge arrives and when
            // the row places the group (`BandStateGroup.asking`).
            if (op.points && !op.at("pointed")) {
                if (!Awaited.all("old_git_ready", op.terms(true, false)))
                    return
                op.input("pointed", () => {
                    topBar.statePointedAt = true
                })
                return
            }
            const terms = op.terms(true, true)
            const before = Object.keys(terms).every(term => terms[term])
            terms["laid"] = acts.bandStood(op, before ? op.line() : "") && before
            if (!Awaited.all("old_git_ready", terms))
                return
            op.complete({ "name": "old_git_shaped", "when": () => op.terms(true, true), "report": () => op.line() })
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
        /// What the band shows, read now: loaded with a page; `badged`: with the worktree read, its badge up and the
        /// window at the width the run asked for (`requestedWidth`, kept as it was asked).
        function terms(badged) {
            const page = window.curPage
            const arg = Harness.autoActArg
            const terms = {
                "band": topBar.bandTabsWidth > 0,
                "ready": page !== null
            }
            if (!badged)
                return terms
            terms["loaded"] = page !== null && page.pageWorktree.loaded
            terms["badge"] = topBar.lfsBadgeShown
            terms["sized"] = !noLfsActTimer.sizes
                || (topBar.width === mainUi.width
                    && (arg === "floor" ? window.width >= noLfsActTimer.witnessed("requestedWidth")
                                        : Math.round(window.width) === noLfsActTimer.witnessed("requestedWidth")))
            return terms
        }
        /// …and the card open and laid out where one is wanted, and the page: what the completion stands on.
        function landed() {
            const terms = noLfsActTimer.terms(true)
            terms["card"] = !noLfsActTimer.points || (topBar.stateCardOpen && topBar.stateCardLaidOut)
            terms["page"] = PageSettled.windowSettled(window)
            return terms
        }
        // `count=` is the model's, which the card's sentence says: the picture cannot be read for a number. The
        // judged fields run together, `rows=` last among them (a closed card stands no rows).
        function line() {
            return "no-lfs badge=" + topBar.lfsBadgeShown
                + " card=" + topBar.stateCardOpen
                + " count=" + window.curPage.pageWorktree.lfsNeeded
                + " words=" + topBar.stateWordsShown
                + " mark=" + topBar.stateMarkShown
                + " tint=" + acts.stateTint
                + " fitted=" + topBar.stateMarkFitted
                + " deep=" + topBar.stateMarkDeep
                + " rows=" + topBar.stateCardRows
                + " cap=" + topBar.stateCapW
                + " w=" + window.width
        }
        onTriggered: {
            const op = noLfsActTimer
            const arg = Harness.autoActArg
            if (op.sizes && op.at("start")) {
                if (!Awaited.all("no_lfs_band", op.terms(false)))
                    return
                op.input("sized", () => {
                    window.width = op.witness("requestedWidth",
                                              arg === "floor" ? Math.ceil(window.floorWidth) : parseInt(arg))
                    if (arg === "floor")
                        window.height = Math.ceil(window.floorHeight)
                })
                return
            }
            // Down on a tick of its own and read on a later one, as `old-git-card`'s pointer is.
            if (op.points && !op.at("pointed")) {
                if (!Awaited.all("no_lfs_band", op.terms(true)))
                    return
                op.input("pointed", () => {
                    topBar.statePointedAt = true
                })
                return
            }
            const terms = op.landed()
            const before = Object.keys(terms).every(term => term === "card" || term === "page" || terms[term])
            terms["laid"] = acts.bandStood(op, before ? op.line() : "") && before
            if (!Awaited.all("no_lfs_ready", terms))
                return
            op.complete({ "name": "no_lfs_shaped", "when": () => op.landed(), "report": () => op.line() })
        }
    }
}
