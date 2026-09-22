pragma ComponentBehavior: Bound

import QtQuick
// For the attached `ToolTip` alone (`tab-name`; rules-refs/app-ui.md carries what an unimported attached type answers).
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

/// The tab strip's half of the window's PGG_AUTO_ACT harness: opening a path that is already open, closing a
/// tab from the middle button, and every way a tab is carried along the strip or measured on it.
///
/// Built by `WindowAutoActDriver`, which is what `Main` builds when a verb was given; what these verbs act
/// on is handed down below, one property per part of the window they reach into.
// An `Item` only because `QtObject` has no default property to hold the timers below; it is a
// sizeless holder.
Item {
    id: acts

    required property var window
    required property TabsModel tabsModel
    required property Repeater pageRepeater
    required property TopBar topBar
    /// The harness's own window onto the strip's laid-out tabs (`auto/TabProbe.qml`).
    required property TabProbe tabProbe
    required property Item mainUi
    required property Item gate

    /// Whether `page` is reading the working copy at `path` — asked of the session's own answer for where it
    /// opened (`RepoTab.repoPath`), which is the witness outside the strip's bookkeeping.
    ///
    /// **Folded the way the application folds it** (`nav/drain.rs`): one folder reaches the two sides spelled
    /// differently — `git worktree list` prints it one way and `rev-parse --show-toplevel` another — and on
    /// Windows the difference is the letter case the filesystem keeps but does not tell names apart by.
    function standsIn(page, path) {
        return path !== "" && page.pageTab.repoPath.toLowerCase() === path.toLowerCase()
    }

    /// The page the switch is about to be asked of, and what its graph stood at — **the half no picture holds**.
    /// A window showing one repository frames the same whether the page was taken down and built again or stayed
    /// where it was, and standing the tab elsewhere is supposed to keep it (デザイン規約 §タブの所作).
    property var standPage: null
    property int standFinished: -1
    property int standReset: -1
    /// The command log's rows as the page was left, which is the other thing standing a tab elsewhere used to take
    /// away: the log is the record of what this window ran, and the window has not changed (`CommandMsg`).
    property int standLogged: -1
    function holdStand(page) {
        acts.standPage = page
        acts.standFinished = page.pageGraph.finishCount
        acts.standReset = page.pageGraph.resetCount
        acts.standLogged = page.pageCommands.rowsHeld()
    }
    /// Whether the graph was drawn again on the way: a pass landed after the press.
    ///
    /// **Not a wait** — the session taking over is handed the record of the graph on screen
    /// (`platitude_core::session::DrawnGraph`), so a pass that arrives at the picture already there sends nothing
    /// at all, and two copies whose rows agree are a switch with no redraw in it. What the run waits for is the
    /// copy it stood in having answered (`PageSettled`), and this says which of the two it was.
    function standDrew(page) {
        return page.pageGraph.finishCount > acts.standFinished
    }
    /// …and it landed **over** that graph rather than in place of it: the same page object throughout, and a
    /// stream that never started over (`GraphModel.resetCount`, which only a restart moves).
    function standKept(page) {
        return page === acts.standPage && page.pageGraph.resetCount === acts.standReset
    }
    /// …and the log with it: every row it held before the switch is still there, whatever the copy arrived at has
    /// run since. `> 0` because a log with nothing in it would answer this with any behaviour at all.
    function standLogKept(page) {
        return acts.standLogged > 0 && page.pageCommands.rowsHeld() >= acts.standLogged
    }

    // What remains after a middle-click is the output under test. Wait for the tab-model count edge.
    // The two tabs this verb needs are a precondition of the press and
    // nothing else: read again after it, they turn the verb's own answer — one tab fewer — into a wait nothing can end.
    SampleTimer {
        id: middleCloseTimer
        running: Harness.autoAct === "middle-close"
        property bool requested: false
        property int beforeCount: -1
        property string closedPath: ""
        onTriggered: {
            if (!middleCloseTimer.requested) {
                if (pageRepeater.count < 2)
                    return
                middleCloseTimer.beforeCount = pageRepeater.count
                const at = Number(Harness.autoActArg)
                middleCloseTimer.closedPath = tabProbe.tabPathAt(at)
                // Latched on the strip's answer: the press has to land on an item, and the
                // row the model has just gained gets one with the layout.
                middleCloseTimer.requested = topBar.middleClickTab(at)
                return
            }
            // The strip is read back below, so it has to have caught up with the model before there is anything true to
            // say about which repository went and which is still standing — and caught up means laid
            // out: until the strip has responded to the row leaving, its items still answer by their old indices,
            // and index 0 is the closed tab's own item (`TabProbe.settleStrip`).
            if (pageRepeater.count >= middleCloseTimer.beforeCount)
                return
            tabProbe.settleStrip()
            if (tabProbe.tabItemCount() !== pageRepeater.count)
                return
            // And the tab that stayed has to be showing its repository: "the neighbour is still there" is the half of
            // this verb the picture carries, and a page still opening photographs the same whether it survived the
            // close or was never opened at all. The page driver's own baseline (AutoActDriver), asked of whichever page
            // the close left in front.
            const kept = window.curPage
            if (kept === null || kept.pageTab.state !== "open"
                    || !kept.pageWt.loaded || kept.pageGraph.finishCount === 0)
                return
            stop()
            // The verdict leads, and it is about the tab the press landed on: a count that merely fell would pass with
            // the wrong tab closed, and the titles cannot tell them apart.
            Harness.report("middle_close gone="
                              + !tabProbe.hasTabPath(middleCloseTimer.closedPath)
                              + " tabs=" + pageRepeater.count
                              + " active=" + tabsModel.currentIndex
                              + " open=" + tabProbe.tabPaths())
            window.finishAutoAct()
        }
    }

    // What the strip is holding after a tab was carried across it. The order is the output, and it is read off the
    // items: the drag settles itself against where the tabs actually sit, so the walk is what
    // says the two agree. The tabs it needs are a precondition of the carry and are read in that branch alone —
    // afterwards, "the order is not the one it started as" is this verb's own answer.
    SampleTimer {
        id: tabDragTimer
        running: Harness.autoAct === "tab-drag"
        property bool requested: false
        property string before: ""
        property int from: 0
        property int to: 0
        onTriggered: {
            if (!tabDragTimer.requested) {
                // Every row standing in the strip: the carry measures against the tabs' own places,
                // and a row the model has only just gained has none until the next layout.
                if (pageRepeater.count < 2 || tabProbe.tabItemCount() !== pageRepeater.count)
                    return
                // And the page under the strip settled, so that what is photographed underneath is a settled
                // repository. A precondition of the carry and read nowhere else: the carry moves to the tab
                // it takes up, and a page that then has to open would turn this into a wait for something the verb
                // itself caused.
                const front = window.curPage
                if (front === null || front.pageTab.state !== "open"
                        || !front.pageWt.loaded || front.pageGraph.finishCount === 0)
                    return
                const asked = (Harness.autoActArg || "3:1").split(":")
                tabDragTimer.from = Number(asked[0])
                tabDragTimer.to = Number(asked[1])
                tabDragTimer.before = tabProbe.tabPaths()
                // Latched on the strip's answer, the way the middle click is: a carry that found no tab to take up
                // never happened, and reporting it as one would leave the wait to the watchdog.
                tabDragTimer.requested = topBar.dragTabTo(tabDragTimer.from, tabDragTimer.to)
                return
            }
            const paths = tabProbe.tabPaths()
            if (paths === tabDragTimer.before || tabProbe.tabItemCount() !== pageRepeater.count)
                return
            stop()
            // The verdict leads, and it is about the tab that was carried: an order that merely changed would pass
            // with any two tabs swapped, and every demo working tree is called the same thing in the picture.
            const was = tabDragTimer.before.split(",")
            Harness.report("tab_drag moved="
                              + (paths.split(",")[tabDragTimer.to] === was[tabDragTimer.from])
                              + " from=" + tabDragTimer.from
                              + " to=" + tabDragTimer.to
                              + " tabs=" + pageRepeater.count
                              + " active=" + tabsModel.currentIndex
                              + " open=" + paths)
            window.finishAutoAct()
        }
    }

    // And the half of the carry that ends in no order at all: a tab drawn away from its own row while the hand is
    // still on it. The settled strip photographs the same whether it was ever drawn under the hand or only ever
    // jumped between rows, so the offset the transform is carrying says itself.
    SampleTimer {
        id: tabHoldTimer
        running: Harness.autoAct === "tab-hold"
        property bool requested: false
        onTriggered: {
            if (!tabHoldTimer.requested) {
                if (pageRepeater.count < 2 || tabProbe.tabItemCount() !== pageRepeater.count)
                    return
                const front = window.curPage
                if (front === null || front.pageTab.state !== "open"
                        || !front.pageWt.loaded || front.pageGraph.finishCount === 0)
                    return
                tabHoldTimer.requested = topBar.holdTabAt(Number(Harness.autoActArg || 0))
                return
            }
            // The tab has to be drawn off its row before there is a picture worth taking; nothing is waited for
            // afterwards, because a hand that has not let go is the whole state.
            const shift = topBar.heldTabShift()
            if (shift === 0)
                return
            stop()
            Harness.report("tab_hold lifted=true at=" + (Harness.autoActArg || 0)
                              + " shift=" + shift
                              + " active=" + tabsModel.currentIndex
                              + " open=" + tabProbe.tabPaths())
            window.finishAutoAct()
        }
    }

    // The strip travelling under a tab held past the end of it — the half of the carry that reaches a place which was
    // not on screen when the hand took hold. The window goes down on its floor first: whether a given number of tabs
    // overflows at all is a question about the installed fonts and the band's own furniture (`tab-widths` answers it
    // differently on each OS), and this verb needs a strip that overflows on every machine.
    SampleTimer {
        id: tabEdgeTimer
        running: Harness.autoAct === "tab-edge"
        property bool sized: false
        property bool settling: false
        property bool told: false
        property bool requested: false
        property int from: 0
        onTriggered: {
            if (!tabEdgeTimer.requested) {
                if (pageRepeater.count < 2 || tabProbe.tabItemCount() !== pageRepeater.count)
                    return
                const front = window.curPage
                if (front === null || front.pageTab.state !== "open"
                        || !front.pageWt.loaded || front.pageGraph.finishCount === 0)
                    return
                if (!tabEdgeTimer.sized) {
                    window.width = Math.ceil(window.floorWidth)
                    tabEdgeTimer.sized = true
                    return
                }
                // A strip that fits has no end to travel to. The resize is what makes one, and what is waited on
                // is the strip's own word, because which width crowds a strip is the
                // thing this cannot assume.
                if (!topBar.bandTabScrolls) {
                    // A strip still uncrowded a tick after the floor took is one this staging cannot carry: the wait
                    // above has nothing left to wait for, and the watchdog that ends such a run names the verb.
                    // Said once, and a tick late so the resize has laid out — the numbers
                    // the count has to be chosen against are the band's own, and they differ per OS (`tab-widths`).
                    if (tabEdgeTimer.settling && !tabEdgeTimer.told) {
                        tabEdgeTimer.told = true
                        Harness.report("tab_edge crowded=false tabs=" + pageRepeater.count
                                          + " content=" + Math.round(topBar.bandTabContent)
                                          + " view=" + Math.round(topBar.bandTabsWidth)
                                          + " windowW=" + Math.round(window.width)
                                          + " floorW=" + Math.ceil(window.floorWidth))
                    }
                    tabEdgeTimer.settling = true
                    return
                }
                tabEdgeTimer.from = Number(Harness.autoActArg || 0)
                tabEdgeTimer.requested = topBar.carryTabPastEnd(tabEdgeTimer.from)
                return
            }
            // Travelled to its far end, carrying the tab to the end of the order: the strip stops on its own bound,
            // and the tab passes every neighbour that slides under it on the way there.
            if (!topBar.runAtEnd() || topBar.heldTabIndex() !== pageRepeater.count - 1)
                return
            stop()
            topBar.dropCarriedTab()
            Harness.report("tab_edge landed=true from=" + tabEdgeTimer.from
                              + " run=" + topBar.runOffset()
                              + " content=" + Math.round(topBar.bandTabContent)
                              + " view=" + Math.round(topBar.bandTabsWidth)
                              + " tabs=" + pageRepeater.count
                              + " active=" + tabsModel.currentIndex
                              + " open=" + tabProbe.tabPaths())
            window.finishAutoAct()
        }
    }

    // Another working copy's uncommitted row stands this tab in that copy — the door to the changes it is about,
    // since this window's panes read the tree this tab is standing in. Window-level because the strip is what it
    // lands in: the page stays, but what it stands in is the strip's answer (デザイン規約 §タブの所作).
    SampleTimer {
        id: carriedOpenTimer
        running: Harness.autoAct === "carried-open"
        property bool asked: false
        property int beforeCount: -1
        property string wanted: ""
        /// The first row of somebody else's uncommitted work, or -1 while the graph has none. Off the model, which is
        /// what says whose a row is — a row off screen has no delegate to ask.
        function carriedRow(graph) {
            for (let row = 0; row < graph.rowTotal; row++) {
                if (graph.carriedName(row) !== "")
                    return row
            }
            return -1
        }
        onTriggered: {
            const page = window.curPage
            if (page === null || page.pageTab.state !== "open" || page.pageGraph.finishCount === 0)
                return
            if (!carriedOpenTimer.asked) {
                const row = carriedOpenTimer.carriedRow(page.pageGraph)
                if (row < 0)
                    return
                // Through the row itself: the item is the real handler's own, and a run that called the page
                // would go green with the row's own decision never taken (verify-ui §壊れない動詞).
                const item = page.pageGraphPane.view.itemAtIndex(row)
                if (item === null)
                    return
                carriedOpenTimer.wanted = page.pageGraph.carriedPath(row)
                carriedOpenTimer.beforeCount = pageRepeater.count
                acts.holdStand(page)
                if (item.doubleClick(Qt.NoModifier) !== true)
                    return
                carriedOpenTimer.asked = true
                return
            }
            // The strip did not grow and the page in front is reading the copy that was asked for: a page still
            // opening looks the same whichever copy it is, and so does a second tab in a picture of one.
            if (pageRepeater.count !== carriedOpenTimer.beforeCount)
                return
            const front = window.curPage
            if (front === null || front.pageTab.state !== "open"
                    || !PageSettled.settled(front)
                    || !acts.standsIn(front, carriedOpenTimer.wanted)
                    // …and the graph has stopped calling that copy somebody else's: the rows this verb is about
                    // are the synthetic ones, and the copy arrived at is drawn as another copy until the pass that
                    // lays them again lands (`GraphModel.carriedRowOf`). The settled rule above cannot see it —
                    // both copies are dirty in this preset, so the working-tree row it reads stands either way.
                    || front.pageGraph.carriedRowOf(carriedOpenTimer.wanted) >= 0)
                return
            stop()
            Harness.report("carried_open tabs=" + pageRepeater.count
                              + " grew=" + (pageRepeater.count !== carriedOpenTimer.beforeCount)
                              + " stood=" + acts.standsIn(front, carriedOpenTimer.wanted)
                              + " drew=" + acts.standDrew(front)
                              + " kept=" + acts.standKept(front)
                              + " wanted=" + carriedOpenTimer.wanted
                              + " where=" + front.pageTab.repoPath)
            window.finishAutoAct()
        }
    }

    // A WORKTREES row stands this tab in that working copy — the same landing the graph's carried row reaches, by
    // the door the left menu offers (デザイン規約 §左メニューの所作). Window-level for the reason above.
    SampleTimer {
        id: worktreeStandTimer
        running: Harness.autoAct === "worktree-stand"
        property bool asked: false
        /// Whether this run has already asked for a read to be written down (see the branch below) — asked once,
        /// not once per tick.
        property bool logging: false
        property int beforeCount: -1
        property string wanted: ""
        onTriggered: {
            const page = window.curPage
            if (page === null || page.pageTab.state !== "open" || page.pageGraph.finishCount === 0)
                return
            if (!worktreeStandTimer.asked) {
                // Rows in the log before the switch, which is what `log=` is read against — an empty log would
                // answer it whatever the switch did to one. The reads an opening makes are not the reader's and
                // are not written down (`Recording::UserOnly`), so this run asks for them to be, and then asks
                // for a read (`refreshQuick`, the same one the window coming back makes).
                if (page.pageCommands.rowsHeld() === 0) {
                    if (!worktreeStandTimer.logging) {
                        worktreeStandTimer.logging = true
                        page.pageCommands.setBackgroundReads(true)
                        page.pageTab.refreshQuick()
                    }
                    return
                }
                // The row, off the section's own model: which copies there are is what it is listing, and the
                // argument names one of its rows (the first linked copy by default — row 0 is the copy this window
                // is already standing in, whose row leads nowhere).
                const trees = page.pageSidebar.worktreesModel
                const row = Harness.autoActArg === "" ? 1 : Number(Harness.autoActArg)
                if (trees.shown() <= row)
                    return
                const full = trees.fullAt(row)
                if (full === "" || acts.standsIn(page, full))
                    return
                worktreeStandTimer.wanted = full
                worktreeStandTimer.beforeCount = pageRepeater.count
                acts.holdStand(page)
                // The pane's own door — the one the row's double-click calls (`SidebarRowGestures.activateRow`).
                page.pageSidebar.activateRow("worktree", trees.nameAt(row), full, trees.headOfCopy(full))
                worktreeStandTimer.asked = true
                return
            }
            if (pageRepeater.count !== worktreeStandTimer.beforeCount)
                return
            const front = window.curPage
            if (front === null || front.pageTab.state !== "open"
                    || !PageSettled.settled(front)
                    || !acts.standsIn(front, worktreeStandTimer.wanted))
                return
            stop()
            Harness.report("worktree_stand tabs=" + pageRepeater.count
                              + " grew=" + (pageRepeater.count !== worktreeStandTimer.beforeCount)
                              + " stood=" + acts.standsIn(front, worktreeStandTimer.wanted)
                              + " drew=" + acts.standDrew(front)
                              + " kept=" + acts.standKept(front)
                              + " log=" + acts.standLogKept(front)
                              + " wanted=" + worktreeStandTimer.wanted
                              + " where=" + front.pageTab.repoPath)
            window.finishAutoAct()
        }
    }

    // …and away and back again, which is the one claim about a copy switch no single landing can make: the unsent
    // words are filed under the copy they were written in, so leaving takes them off the screen and coming back puts
    // them there (デザイン規約 §タブの所作「未コミットのコミットメッセージは立ち位置ごとに憶える」). One page
    // throughout — that is what `kept=` says, and it is why the words can be read as having been put back rather
    // than never taken away.
    SampleTimer {
        id: copyDraftTimer
        running: Harness.autoAct === "copy-draft"
        /// 0 = write and leave, 1 = read the other copy's empty box and come back, 2 = read the words again.
        property int step: 0
        /// Words no repository can produce, so finding them again cannot be anything but this page having kept them.
        readonly property string typed: "chore: words written in one working copy"
        /// The copy they were written in, and the one stood in between.
        property string home: ""
        property string away: ""
        property bool awayEmpty: false
        /// Whether the tab has answered for where it now stands: the strip asks git before it moves, and the page
        /// holds its doors until the session opens (`RepoTab.standing`).
        function settled(page, copy) {
            return page !== null && page.pageTab.state === "open" && !page.pageTab.standing
                   && page.pageWt.loaded && acts.standsIn(page, copy)
        }
        /// The WORKTREES row naming `copy`, activated the way a double-click does. Answers whether it was there.
        function standIn(page, copy) {
            const trees = page.pageSidebar.worktreesModel
            for (let row = 0; row < trees.shown(); row++) {
                const full = trees.fullAt(row)
                if (full !== "" && full.toLowerCase() === copy.toLowerCase()) {
                    page.pageSidebar.activateRow("worktree", trees.nameAt(row), full, trees.headOfCopy(full))
                    return true
                }
            }
            return false
        }
        onTriggered: {
            const page = window.curPage
            if (page === null || page.pageTab.state !== "open" || page.pageTab.standing
                    || !page.pageWt.loaded || page.pageGraph.finishCount === 0)
                return
            if (copyDraftTimer.step === 0) {
                const trees = page.pageSidebar.worktreesModel
                if (trees.shown() < 2)
                    return
                const full = trees.fullAt(1)
                if (full === "" || acts.standsIn(page, full))
                    return
                copyDraftTimer.home = page.pageTab.repoPath
                copyDraftTimer.away = full
                page.pageWip.setMessage(copyDraftTimer.typed, "")
                acts.holdStand(page)
                if (!copyDraftTimer.standIn(page, full))
                    return
                copyDraftTimer.step = 1
                return
            }
            if (copyDraftTimer.step === 1) {
                if (!copyDraftTimer.settled(page, copyDraftTimer.away))
                    return
                copyDraftTimer.awayEmpty = page.pageWip.subjectText === "" && page.pageWip.bodyText === ""
                if (!copyDraftTimer.standIn(page, copyDraftTimer.home))
                    return
                copyDraftTimer.step = 2
                return
            }
            if (!copyDraftTimer.settled(page, copyDraftTimer.home))
                return
            stop()
            Harness.report("copy_draft empty=" + copyDraftTimer.awayEmpty
                              + " back=" + (page.pageWip.subjectText === copyDraftTimer.typed)
                              // …and the pane holding them is the one on screen, the same half `tab-carry` reads.
                              + " wip=" + page.wipShown
                              + " kept=" + acts.standKept(page)
                              + " away=" + copyDraftTimer.away)
            window.finishAutoAct()
        }
    }

    // Reopening an existing path selects the tab it already has.
    SampleTimer {
        id: openAgainTimer
        running: Harness.autoAct === "open-again"
        property bool requested: false
        property string asked: ""
        property int beforeCount: -1
        /// Which tab the ask is meant to bring to the front, where the run left that to the strip. -1 when the run
        /// named a path of its own: which tab holds a folder is the question there, and the whole point is that the
        /// spelling does not say.
        property int wantIndex: -1
        onTriggered: {
            if (!openAgainTimer.requested) {
                if (pageRepeater.count === 0)
                    return
                // The spelling to ask with, when the run named none, is the strip's own — and the strip is a view: the
                // item for a row the model has just gained arrives with the next layout. Asking with the "" it answers
                // until then opens nothing, and nothing opened is what this verb's completion looks like — it went
                // green having asked for nothing at all (measured).
                const path = Harness.autoActArg !== "" ? Harness.autoActArg : tabProbe.tabPathAt(0)
                if (path === "")
                    return
                openAgainTimer.asked = path
                openAgainTimer.wantIndex = Harness.autoActArg !== "" ? -1 : 0
                openAgainTimer.beforeCount = pageRepeater.count
                openAgainTimer.requested = true
                tabsModel.openRepositoryPath(path)
                return
            }
            // Reopening the first tab from a strip standing on another one has somewhere to arrive: without that, "went
            // to the tab it already had" and "did nothing whatever" are the same report.
            if (pageRepeater.count !== openAgainTimer.beforeCount
                    || tabsModel.currentIndex < 0
                    || (openAgainTimer.wantIndex >= 0
                        && tabsModel.currentIndex !== openAgainTimer.wantIndex)
                    // The strip is read back below (`middle-close` above).
                    || tabProbe.tabItemCount() !== pageRepeater.count)
                return
            // And the tab it arrived at has to be showing its repository: which tab came to the front is what the
            // picture carries here, and a page still opening looks the same whichever one it is (`middle-close` above,
            // same baseline).
            const front = window.curPage
            if (front === null || front.pageTab.state !== "open"
                    || !front.pageWt.loaded || front.pageGraph.finishCount === 0)
                return
            stop()
            Harness.report("open_again tabs=" + pageRepeater.count
                              + " active=" + tabsModel.currentIndex
                              + " asked=" + openAgainTimer.asked
                              + " open=" + tabProbe.tabPaths())
            window.finishAutoAct()
        }
    }

    // What a tab switch carries and what it drops — the two halves of one answer, so one verb reports both.
    //
    // The strip is walked in three landings: leave a mark on the tab in front, go to the other one and read what
    // arrived there, come back and read what was kept. Each landing waits for the page it is about to read to be
    // whole, because a page still opening answers every question here the same way an emptied one does.
    //
    // **The tab is what is waited on.** Rows renumber; `currentTabId` is the tab that is actually in
    // front (`TabsModel::current_tab_id`), and comparing against it is what makes "the switch has happened" a fact.
    // And the page read at each landing is a *different object* every time — the page in front is
    // built for the tab in front and taken down with it — so nothing here may be held across a landing but the words
    // themselves.
    SampleTimer {
        id: tabCarryTimer
        running: Harness.autoAct === "tab-carry"
        /// 0 = mark the first tab, 1 = read the second, 2 = read the first again.
        property int step: 0
        /// The tab the mark was left on, so the walk knows which landing it is at without counting rows.
        property int firstTab: -1
        /// Words no repository can produce, so finding them again cannot be anything but this page having kept them.
        readonly property string typed: "chore: words that outlived a tab switch"
        /// Read at the second landing and reported at the third: the layout followed the reader, the words did not.
        property bool folded: false
        property bool log: false
        property bool otherEmpty: false
        function whole(page) {
            return page !== null && page.pageTab.state === "open"
                   && page.pageWt.loaded && page.pageGraph.finishCount > 0
        }
        onTriggered: {
            const page = window.curPage
            if (!tabCarryTimer.whole(page))
                return
            if (tabCarryTimer.step === 0) {
                // Two tabs are what this verb is about, and they are a precondition of the first landing alone: read
                // again afterwards they would be read against a strip this verb has already moved.
                if (pageRepeater.count < 2)
                    return
                // A layout nobody starts in, so "it followed" cannot be read off a page that was already like this.
                page.foldByHand(true)
                page.commandsOpen = true
                page.pageWip.setMessage(tabCarryTimer.typed, "")
                tabCarryTimer.firstTab = tabsModel.currentTabId
                tabCarryTimer.step = 1
                tabsModel.setCurrentIndex(tabsModel.currentIndex === 0 ? 1 : 0)
                return
            }
            if (tabCarryTimer.step === 1) {
                if (tabsModel.currentTabId === tabCarryTimer.firstTab)
                    return
                tabCarryTimer.folded = page.sidebarCollapsed
                tabCarryTimer.log = page.commandsOpen
                tabCarryTimer.otherEmpty = page.pageWip.subjectText === ""
                                           && page.pageWip.bodyText === ""
                tabCarryTimer.step = 2
                tabsModel.setCurrentIndex(tabsModel.currentIndex === 0 ? 1 : 0)
                return
            }
            if (tabsModel.currentTabId !== tabCarryTimer.firstTab)
                return
            stop()
            // `sessions=` is the release itself, and the only thing here a picture cannot say: two tabs in the strip,
            // one repository in memory. `finishCount` being above zero on a page built after the switch is the other
            // side of the same coin — the graph read itself again from nothing.
            Harness.report("tab_carry tabs=" + pageRepeater.count
                              + " sessions=" + Harness.openSessionCount()
                              + " folded=" + tabCarryTimer.folded
                              + " log=" + tabCarryTimer.log
                              + " empty=" + tabCarryTimer.otherEmpty
                              + " back=" + (page.pageWip.subjectText === tabCarryTimer.typed)
                              // …and the pane holding them is the one on screen: words put back behind the commit
                              // details are only half of them being kept.
                              + " wip=" + page.wipShown)
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=tab-widths: numbers for the band's reason — a strip that narrowed the wrong tabs comes out looking
    // like one that got it right. `widths=` is the answer, and `room=` says which of the two things the strip gives up
    // it is living on (the mark's room first, the names after).
    //
    // `crushed=` is what the widths cannot say: a tab is as wide as its name, so a name drawn away to the mark alone
    // leaves the strip's own numbers looking exactly as they should (`TabProbe.tabNamesCrushed`). Said first, where
    // the whole of what is judged here stands together (`verify/verbs.rs`).
    SampleTimer {
        id: tabWidthActTimer
        running: Harness.autoAct === "tab-widths"
        onTriggered: {
            // Every row standing in the strip: both readings below walk the items the view built,
            // and a row the model has only just gained has none until the next layout — `widths=` would carry a 0
            // for it, and `crushed=` would answer for a strip it had not seen (規約 §UI 自動化の因果性).
            if (topBar.bandTabCount <= 0 || topBar.bandTabRun <= 0
                    || tabProbe.tabItemCount() !== topBar.bandTabCount)
                return
            stop()
            Harness.report(
                "tab_widths crushed=" + tabProbe.tabNamesCrushed()
                // Which state the run actually reached, beside the crush count and judged with it: how many tabs it
                // takes to crowd this band is the platform's answer (`TabStrip.tabNamesCut`), so a run that named a
                // count would be claiming a state nobody checked.
                + " cut=" + topBar.tabNamesCut
                + " folded=" + topBar.tabMarksFolded
                + " tabs=" + topBar.bandTabCount
                + " run=" + Math.round(topBar.bandTabRun)
                + " cap=" + Math.round(topBar.tabTitleCap)
                + " room=" + Math.round(topBar.tabMarkRoom)
                + " floor=" + topBar.tabTitleMinW
                + " ease=" + Math.round(topBar.tabTitleEaseW)
                + " max=" + topBar.tabTitleMaxW
                + " content=" + Math.round(topBar.bandTabContent)
                + " view=" + Math.round(topBar.bandTabsWidth)
                + " scrolls=" + topBar.bandTabScrolls
                + " widths=" + tabProbe.tabWidths())
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=tab-mark: the argument is which tab the hand is on — one of the others, or the run says
    // nothing the picture of any other verb does not already say.
    SampleTimer {
        id: tabMarkActTimer
        running: Harness.autoAct === "tab-mark"
        property bool requested: false
        property string beforeMarks: ""
        onTriggered: {
            const pointed = Number(Harness.autoActArg || 1)
            if (topBar.bandTabCount < pointed)
                return
            if (!tabMarkActTimer.requested) {
                tabMarkActTimer.requested = true
                tabMarkActTimer.beforeMarks = tabProbe.tabMarks()
                tabProbe.pointAtTab(pointed)
                return
            }
            if (tabProbe.tabMarks() === tabMarkActTimer.beforeMarks)
                return
            stop()
            Harness.report(
                "tab_marks tabs=" + topBar.bandTabCount
                + " current=" + tabsModel.currentIndex
                + " pointed=" + pointed
                + " marks=" + tabProbe.tabMarks())
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=tab-name: the names a strip of namesakes settled on, and the whole path the hand asks for on top of
    // them. The argument is which tab to point at. Both halves in one run because they are one question — what this tab
    // stands on — asked of the strip and then of the hover (デザイン規約 §タブの所作 / §hover のツールチップ).
    SampleTimer {
        id: tabNameActTimer
        running: Harness.autoAct === "tab-name"
        property bool requested: false
        readonly property int pointed: Number(Harness.autoActArg || 0)
        onTriggered: {
            // Every row has to have an item before the names are read: the strip's answer for a row the layout has not
            // reached yet is the empty string, which reads exactly like a name that came out blank.
            if (topBar.bandTabCount === 0 || tabProbe.tabItemCount() < topBar.bandTabCount)
                return
            if (!tabNameActTimer.requested) {
                tabNameActTimer.requested = true
                tabProbe.pointAtTab(tabNameActTimer.pointed)
                return
            }
            const tip = mainUi.ToolTip.toolTip
            // The tip is on a delay, so this waits it out.
            if (!tip.visible)
                return
            stop()
            // `unique=` is the rule the strip is run against — no two tabs reading alike (`verify/verbs.rs`) — and
            // `native=` is the spelling: a path on screen is punctuated with `/` on every platform
            // (規約 §パスの区切り), so a backslash in a name **or in the hover beside it** is Windows having
            // spelled one its own way. The two are read as one claim because they are the same path twice.
            const titles = tabProbe.tabTitles()
            const names = titles.split(",")
            Harness.report(
                "tab_names tabs=" + topBar.bandTabCount
                + " pointed=" + tabNameActTimer.pointed
                + " unique=" + names.every((name, at) => names.indexOf(name) === at)
                + " native=" + (names.some(name => name.includes("\\")) || tip.text.includes("\\"))
                + " tip=" + tip.visible
                + " titles=" + titles
                + " said=" + tip.text)
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=solo: the harness holds the real lock on the config directory before starting this process, so the
    // picture is of the mechanism.
    SampleTimer {
        id: soloActTimer
        running: Harness.autoAct === "solo"
        onTriggered: {
            if (!AppBackend.alreadyRunning || !gate.visible)
                return
            stop()
            Harness.report(
                "solo blocked=" + AppBackend.alreadyRunning
                + " held=" + (AppBackend.heldElsewhere !== "")
                + " gate=" + gate.visible
                + " main=" + mainUi.visible)
            window.finishAutoAct()
        }
    }
}
