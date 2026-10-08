pragma ComponentBehavior: Bound

import QtQuick
// For the attached `ToolTip` alone (rules-refs/app-ui.md: an unimported attached type reads `undefined`).
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

/// The tab strip's half of the window's PGG_AUTO_ACT harness: opening a path that is already open, closing a
/// tab from the middle button, and every way a tab is carried along the strip or measured on it.
// `Item`, not `QtObject`: rules-refs/app-ui.md「ドライバの root は `Item`」.
Item {
    id: acts

    required property var window
    required property TabsModel tabsModel
    required property Repeater pageRepeater
    required property TopBar topBar
    required property TabProbe tabProbe
    required property Item mainUi
    required property Item gate

    /// Whether `page` is reading the worktree at `path`, asked of the session's own answer (`RepoTab.repoPath`).
    /// Case-folded as `nav/drain.rs` does: `git worktree list` and `rev-parse --show-toplevel` can spell one folder
    /// in different letter case on Windows.
    function standsIn(page, path) {
        return path !== "" && page.pageTab.repoPath.toLowerCase() === path.toLowerCase()
    }

    /// The page before the switch and its graph's counters — the half no picture holds: standing the tab elsewhere
    /// must keep the page (デザイン規約 §タブの所作), and a rebuilt one frames the same.
    property var standPage: null
    property int standFinished: -1
    property int standReset: -1
    /// The command log's rows as the page was left: a switch must not take the window's log away (`CommandMsg`).
    property int standLogged: -1
    function holdStand(page) {
        acts.standPage = page
        acts.standFinished = page.pageGraph.finishCount
        acts.standReset = page.pageGraph.resetCount
        acts.standLogged = page.pageCommands.rowsHeld()
    }
    /// Whether a graph pass landed after the press. Reported, not waited on: two worktrees whose rows agree send no
    /// pass (`platitude_core::session::DrawnGraph`); the run waits on `PageSettled`.
    function standDrew(page) {
        return page.pageGraph.finishCount > acts.standFinished
    }
    /// …and over that graph, not in place of it: the same page, and a stream that never restarted
    /// (`GraphModel.resetCount`).
    function standKept(page) {
        return page === acts.standPage && page.pageGraph.resetCount === acts.standReset
    }
    /// …and the log with it. `> 0`: an empty log would pass whatever the switch did.
    function standLogKept(page) {
        return acts.standLogged > 0 && page.pageCommands.rowsHeld() >= acts.standLogged
    }

    // What remains after a middle-click. The two tabs are a precondition of the press only: read again after it, the
    // verb's own answer (one tab fewer) becomes a wait nothing can end.
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
                // Latched on the strip's answer: a row the model just gained has no item until the next layout.
                middleCloseTimer.requested = topBar.middleClickTab(at)
                return
            }
            // The strip must be laid out before it is read back: until then its items answer by their old indices,
            // and index 0 is the closed tab's own item (`TabProbe.settleStrip`).
            if (pageRepeater.count >= middleCloseTimer.beforeCount)
                return
            tabProbe.settleStrip()
            if (tabProbe.tabItemCount() !== pageRepeater.count)
                return
            // And the tab left in front showing its repository: a page still opening frames the same whether it
            // survived the close or never opened (`AutoActDriver`'s baseline).
            const kept = window.curPage
            if (kept === null || kept.pageTab.state !== "open"
                    || !kept.pageWorktree.loaded || kept.pageGraph.finishCount === 0)
                return
            stop()
            // `gone=` leads: a count that merely fell would pass with the wrong tab closed.
            Harness.report("middle_close gone="
                              + !tabProbe.hasTabPath(middleCloseTimer.closedPath)
                              + " tabs=" + pageRepeater.count
                              + " active=" + tabsModel.currentIndex
                              + " open=" + tabProbe.tabPaths())
            window.finishAutoAct()
        }
    }

    // The strip's order after a tab was carried across it, read off the items (the drag settles against where the
    // tabs actually sit). The tabs it needs are a precondition of the carry, read in that branch alone.
    SampleTimer {
        id: tabDragTimer
        running: Harness.autoAct === "tab-drag"
        property bool requested: false
        property string before: ""
        property int from: 0
        property int to: 0
        onTriggered: {
            if (!tabDragTimer.requested) {
                // Every row laid out: the carry measures against the tabs' own places.
                if (pageRepeater.count < 2 || tabProbe.tabItemCount() !== pageRepeater.count)
                    return
                // And the page under the strip settled — here only: the carry moves to the tab it takes up, whose
                // page then has to open.
                const front = window.curPage
                if (front === null || front.pageTab.state !== "open"
                        || !front.pageWorktree.loaded || front.pageGraph.finishCount === 0)
                    return
                const asked = (Harness.autoActArg || "3:1").split(":")
                tabDragTimer.from = Number(asked[0])
                tabDragTimer.to = Number(asked[1])
                tabDragTimer.before = tabProbe.tabPaths()
                // Latched on the strip's answer, as the middle click is: a carry that found no tab never happened.
                tabDragTimer.requested = topBar.dragTabTo(tabDragTimer.from, tabDragTimer.to)
                return
            }
            const paths = tabProbe.tabPaths()
            if (paths === tabDragTimer.before || tabProbe.tabItemCount() !== pageRepeater.count)
                return
            stop()
            // `moved=` leads: an order that merely changed would pass with any two tabs swapped.
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

    // A tab drawn off its row while the hand is still on it. A settled strip frames the same whether the tab was drawn
    // under the hand or only jumped, so the transform's offset is reported.
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
                        || !front.pageWorktree.loaded || front.pageGraph.finishCount === 0)
                    return
                tabHoldTimer.requested = topBar.holdTabAt(Number(Harness.autoActArg || 0))
                return
            }
            // Nothing after the lift is waited for: a hand that has not let go is the whole state.
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

    // The strip travelling under a tab held past its end. The window goes down to its floor first: whether N tabs
    // overflow depends on the installed fonts (`tab-widths` differs per OS), and this needs overflow on every machine.
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
                        || !front.pageWorktree.loaded || front.pageGraph.finishCount === 0)
                    return
                if (!tabEdgeTimer.sized) {
                    window.width = Math.ceil(window.floorWidth)
                    tabEdgeTimer.sized = true
                    return
                }
                // Waited on the strip's own word that it scrolls: which width crowds it cannot be assumed.
                if (!topBar.bandTabScrolls) {
                    // Still uncrowded a tick after the floor took: this staging cannot carry it. Said once, a tick
                    // late so the resize has laid out, with the band's numbers to choose the tab count against.
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

    // Another worktree's uncommitted row, double-clicked, stands this tab in that worktree (デザイン規約 §タブの所作).
    // Window-level because the landing is the strip's answer: the page stays.
    SampleTimer {
        id: carriedOpenTimer
        running: Harness.autoAct === "carried-open"
        property bool asked: false
        property int beforeCount: -1
        property string wanted: ""
        /// The first row of another worktree's uncommitted work, or -1. Off the model: a row off screen has no
        /// delegate.
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
                // Through the row's own handler: calling the page would go green with the row's decision never
                // taken (verify-ui implement.md §壊れない動詞の実装と反復).
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
            // The strip did not grow, and the page in front reads the asked worktree — neither shows in a picture.
            if (pageRepeater.count !== carriedOpenTimer.beforeCount)
                return
            const front = window.curPage
            if (front === null || front.pageTab.state !== "open"
                    || !PageSettled.settled(front)
                    || !acts.standsIn(front, carriedOpenTimer.wanted)
                    // …and the graph no longer draws the arrived worktree as another's (`GraphModel.carriedRowOf`):
                    // settled cannot see it, as both worktrees are dirty in this preset.
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

    // A WORKTREES row stands this tab in that worktree — the carried row's landing, by the left menu's door
    // (デザイン規約 §左メニューの所作). Window-level for the reason above.
    SampleTimer {
        id: worktreeStandTimer
        running: Harness.autoAct === "worktree-stand"
        property bool asked: false
        /// The logged read below has been asked for (once, not per tick).
        property bool logging: false
        property int beforeCount: -1
        property string wanted: ""
        onTriggered: {
            const page = window.curPage
            if (page === null || page.pageTab.state !== "open" || page.pageGraph.finishCount === 0)
                return
            if (!worktreeStandTimer.asked) {
                // `log=` needs rows before the switch. An opening's reads are not logged (`Recording::UserOnly`), so
                // the run turns background reads on and asks for one (`refreshQuick`).
                if (page.pageCommands.rowsHeld() === 0) {
                    if (!worktreeStandTimer.logging) {
                        worktreeStandTimer.logging = true
                        page.pageCommands.setBackgroundReads(true)
                        page.pageTab.refreshQuick()
                    }
                    return
                }
                // The argument is a WORKTREES row, default 1 = the first linked worktree (row 0 is the worktree
                // stood in).
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
                page.pageSidebar.activateRow("worktree", trees.nameAt(row), full, trees.headOfWorktree(full))
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

    // A worktree made from a menu stands this tab in it (デザイン規約 §worktree を作る): the page presses
    // (`AutoActWorktreeVerbs`) and the landing is read here, the page being stood elsewhere by it. Taken as the tab
    // first settles — the press comes several beats later, and the move behind it takes git's answer and two more
    // processes.
    SampleTimer {
        id: worktreeMadeTimer
        running: Harness.autoAct === "worktree-new" || Harness.autoAct === "worktree-add"
        property string from: ""
        onTriggered: {
            const page = window.curPage
            if (page === null || page.pageTab.state !== "open")
                return
            if (worktreeMadeTimer.from === "") {
                // Settled, as the page's own verb waits to be before it presses: the opening's graph still landing
                // would count as the graph thrown away (`standKept`).
                if (!PageSettled.settled(page))
                    return
                worktreeMadeTimer.from = page.pageTab.repoPath
                acts.holdStand(page)
                return
            }
            if (acts.standsIn(page, worktreeMadeTimer.from) || !PageSettled.settled(page)
                    || page.pageWorktree.branch === "")
                return
            stop()
            const where = page.pageTab.repoPath
            Harness.report("worktree_made stood=" + !acts.standsIn(page, worktreeMadeTimer.from)
                              + " worktree=" + GitFacts.pathLeaf(where)
                              + " branch=" + page.pageWorktree.branch
                              + " tabs=" + pageRepeater.count
                              + " kept=" + acts.standKept(page)
                              + " log=" + page.commandsOpen
                              + " where=" + where)
            window.finishAutoAct()
        }
    }

    // …and away and back: unsent words are filed under the worktree they were written in
    // (デザイン規約 §タブの所作「未コミットのコミットメッセージは立ち位置ごとに憶える」). `kept=` (one page
    // throughout) is what makes `back=` a put-back rather than never taken away.
    SampleTimer {
        id: worktreeDraftTimer
        running: Harness.autoAct === "worktree-draft"
        /// 0 = write and leave, 1 = read the other worktree's empty box and come back, 2 = read the words again.
        property int step: 0
        /// Words no repository can produce.
        readonly property string typed: "chore: words written in one worktree"
        /// The worktree they were written in, and the one stood in between.
        property string home: ""
        property string away: ""
        property bool awayEmpty: false
        /// Whether the tab has answered for where it now stands (`RepoTab.standing`).
        function settled(page, worktree) {
            return page !== null && page.pageTab.state === "open" && !page.pageTab.standing
                   && page.pageWorktree.loaded && acts.standsIn(page, worktree)
        }
        /// The WORKTREES row naming `worktree`, activated the way a double-click does. Answers whether it was there.
        function standIn(page, worktree) {
            const trees = page.pageSidebar.worktreesModel
            for (let row = 0; row < trees.shown(); row++) {
                const full = trees.fullAt(row)
                if (full !== "" && full.toLowerCase() === worktree.toLowerCase()) {
                    page.pageSidebar.activateRow("worktree", trees.nameAt(row), full, trees.headOfWorktree(full))
                    return true
                }
            }
            return false
        }
        onTriggered: {
            const page = window.curPage
            if (page === null || page.pageTab.state !== "open" || page.pageTab.standing
                    || !page.pageWorktree.loaded || page.pageGraph.finishCount === 0)
                return
            if (worktreeDraftTimer.step === 0) {
                const trees = page.pageSidebar.worktreesModel
                if (trees.shown() < 2)
                    return
                const full = trees.fullAt(1)
                if (full === "" || acts.standsIn(page, full))
                    return
                worktreeDraftTimer.home = page.pageTab.repoPath
                worktreeDraftTimer.away = full
                page.pageWip.setMessage(worktreeDraftTimer.typed, "")
                acts.holdStand(page)
                if (!worktreeDraftTimer.standIn(page, full))
                    return
                worktreeDraftTimer.step = 1
                return
            }
            if (worktreeDraftTimer.step === 1) {
                if (!worktreeDraftTimer.settled(page, worktreeDraftTimer.away))
                    return
                worktreeDraftTimer.awayEmpty = page.pageWip.subjectText === "" && page.pageWip.bodyText === ""
                if (!worktreeDraftTimer.standIn(page, worktreeDraftTimer.home))
                    return
                worktreeDraftTimer.step = 2
                return
            }
            if (!worktreeDraftTimer.settled(page, worktreeDraftTimer.home))
                return
            stop()
            Harness.report("worktree_draft empty=" + worktreeDraftTimer.awayEmpty
                              + " back=" + (page.pageWip.subjectText === worktreeDraftTimer.typed)
                              // …and the pane holding them is the one on screen, the same half `tab-carry` reads.
                              + " wip=" + page.wipShown
                              + " kept=" + acts.standKept(page)
                              + " away=" + worktreeDraftTimer.away)
            window.finishAutoAct()
        }
    }

    // The hover over a tab standing in a linked worktree says that worktree's path, not the repository's
    // (デザイン規約 §hover のツールチップ).
    SampleTimer {
        id: worktreeTipTimer
        running: Harness.autoAct === "worktree-tip"
        /// 0 = stand this tab in a linked worktree, 1 = put the hand on it, 2 = read what came out.
        property int step: 0
        property string wanted: ""
        onTriggered: {
            const page = window.curPage
            if (page === null || page.pageTab.state !== "open" || page.pageGraph.finishCount === 0)
                return
            if (worktreeTipTimer.step === 0) {
                const trees = page.pageSidebar.worktreesModel
                const row = Harness.autoActArg === "" ? 1 : Number(Harness.autoActArg)
                if (trees.shown() <= row)
                    return
                const full = trees.fullAt(row)
                if (full === "" || acts.standsIn(page, full))
                    return
                worktreeTipTimer.wanted = full
                worktreeTipTimer.step = 1
                page.pageSidebar.activateRow("worktree", trees.nameAt(row), full, trees.headOfWorktree(full))
                return
            }
            if (worktreeTipTimer.step === 1) {
                // The tab item as well as the page: it arrives with the next layout, and a hand on nothing waits
                // forever.
                if (!acts.standsIn(page, worktreeTipTimer.wanted) || tabProbe.tabItemCount() === 0)
                    return
                worktreeTipTimer.step = 2
                tabProbe.pointAtTab(0)
                return
            }
            const tip = mainUi.ToolTip.toolTip
            // The tip is on a delay, so this waits it out.
            if (!tip.visible)
                return
            stop()
            // `worktree=` is the claim; `native=` is the spelling, read as `tab-name` reads it.
            Harness.report("worktree_tip tip=" + tip.visible
                              + " worktree=" + (tip.text.toLowerCase() === worktreeTipTimer.wanted.toLowerCase())
                              + " native=" + tip.text.includes("\\")
                              + " said=" + tip.text)
            window.finishAutoAct()
        }
    }

    // The worktree-name run on a tab stays with it, and coming back stands the reader in that worktree again
    // (デザイン規約 §タブの所作). The argument picks the landing: `away` (the tab not in front) or `back` (default).
    // At `away` the tab left behind has no page (`Hub::release_tab`), so the strip's row answers (`TabProbe.tabTrees`).
    //
    // Two repositories, the linked worktrees in front: `--preset basic --preset worktrees`.
    SampleTimer {
        id: worktreeKeptTimer
        running: Harness.autoAct === "worktree-kept"
        /// 0 = stand this tab in a linked worktree, 1 = wait for it, 2 = leave it, 3 = come back to it.
        property int step: 0
        /// The strip index of the tab that was stood, and the worktree it was stood in.
        property int seat: -1
        property string wanted: ""
        /// `[<row>][:away]` — the WORKTREES row to stand in (default the first linked worktree), and whether to stop
        /// away. A long-named worktree (`8`) makes the strip cut the run.
        readonly property int row: {
            const named = Number((Harness.autoActArg || "").split(":")[0])
            return Number.isInteger(named) && named > 0 ? named : 1
        }
        readonly property bool comesBack: (Harness.autoActArg || "").indexOf("away") < 0
        function whole(page) {
            return page !== null && page.pageTab.state === "open"
                   && page.pageWorktree.loaded && page.pageGraph.finishCount > 0
        }
        function tell(page) {
            tabProbe.settleStrip()
            const trees = tabProbe.tabTrees().split(",")
            Harness.report("worktree_kept tabs=" + pageRepeater.count
                              + " kept=" + (trees[worktreeKeptTimer.seat] !== undefined
                                            && trees[worktreeKeptTimer.seat] !== "")
                              + " front=" + (tabsModel.currentIndex === worktreeKeptTimer.seat)
                              + " stood=" + acts.standsIn(page, worktreeKeptTimer.wanted)
                              // `kept=` is off the model, which names the worktree even after the strip gave the run up
                              // for room; 0 here is that (デザイン規約 §ウィンドウの縁 の譲る順).
                              + " wide=" + tabProbe.tabTreeWidths()
                              + " says=" + tabProbe.tabTrees()
                              + " wanted=" + worktreeKeptTimer.wanted)
            window.finishAutoAct()
        }
        onTriggered: {
            const page = window.curPage
            if (!worktreeKeptTimer.whole(page))
                return
            if (worktreeKeptTimer.step === 0) {
                // Two tabs — read in this branch alone, as the verb then moves the strip.
                if (pageRepeater.count < 2)
                    return
                const trees = page.pageSidebar.worktreesModel
                const row = worktreeKeptTimer.row
                if (trees.shown() <= row)
                    return
                const full = trees.fullAt(row)
                if (full === "" || acts.standsIn(page, full))
                    return
                worktreeKeptTimer.wanted = full
                worktreeKeptTimer.seat = tabsModel.currentIndex
                worktreeKeptTimer.step = 1
                page.pageSidebar.activateRow("worktree", trees.nameAt(row), full, trees.headOfWorktree(full))
                return
            }
            if (worktreeKeptTimer.step === 1) {
                if (!acts.standsIn(page, worktreeKeptTimer.wanted))
                    return
                worktreeKeptTimer.step = 2
                tabsModel.setCurrentIndex(worktreeKeptTimer.seat === 0 ? 1 : 0)
                return
            }
            if (worktreeKeptTimer.step === 2) {
                if (tabsModel.currentIndex === worktreeKeptTimer.seat)
                    return
                if (!worktreeKeptTimer.comesBack) {
                    stop()
                    worktreeKeptTimer.tell(page)
                    return
                }
                worktreeKeptTimer.step = 3
                tabsModel.setCurrentIndex(worktreeKeptTimer.seat)
                return
            }
            if (tabsModel.currentIndex !== worktreeKeptTimer.seat
                    || !acts.standsIn(page, worktreeKeptTimer.wanted))
                return
            stop()
            worktreeKeptTimer.tell(page)
        }
    }

    // Reopening an existing path selects the tab it already has.
    SampleTimer {
        id: openAgainTimer
        running: Harness.autoAct === "open-again"
        property bool requested: false
        property string asked: ""
        property int beforeCount: -1
        /// The tab the ask should bring to the front; -1 when the run named its own path, whose spelling does not say
        /// which tab holds it.
        property int wantIndex: -1
        onTriggered: {
            if (!openAgainTimer.requested) {
                if (pageRepeater.count === 0)
                    return
                // With no argument, the strip's own spelling of tab 0 — "" until the next layout, and asking with ""
                // opens nothing, which passes for this verb's completion.
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
            // Tab 0 asked from a strip standing on another has somewhere to arrive; else "went to its tab" and "did
            // nothing" report the same.
            if (pageRepeater.count !== openAgainTimer.beforeCount
                    || tabsModel.currentIndex < 0
                    || (openAgainTimer.wantIndex >= 0
                        && tabsModel.currentIndex !== openAgainTimer.wantIndex)
                    // The strip is read back below (`middle-close` above).
                    || tabProbe.tabItemCount() !== pageRepeater.count)
                return
            // And the front tab showing its repository (`middle-close` above, same baseline).
            const front = window.curPage
            if (front === null || front.pageTab.state !== "open"
                    || !front.pageWorktree.loaded || front.pageGraph.finishCount === 0)
                return
            stop()
            Harness.report("open_again tabs=" + pageRepeater.count
                              + " active=" + tabsModel.currentIndex
                              + " asked=" + openAgainTimer.asked
                              + " open=" + tabProbe.tabPaths())
            window.finishAutoAct()
        }
    }

    // What a tab switch carries and what it drops, in three landings: mark the front tab, read the other, come back.
    // Each landing waits for a whole page — a page still opening answers like an emptied one. Waited on
    // `currentTabId`, not the row (rows renumber). The page is a different object at each landing (built and taken
    // down with its tab), so nothing but the words may be held across one.
    SampleTimer {
        id: tabCarryTimer
        running: Harness.autoAct === "tab-carry"
        /// 0 = mark the first tab, 1 = read the second, 2 = read the first again.
        property int step: 0
        /// The tab the mark was left on.
        property int firstTab: -1
        /// Words no repository can produce.
        readonly property string typed: "chore: words that outlived a tab switch"
        /// Read at the second landing and reported at the third: the layout followed the reader, the words did not.
        property bool folded: false
        property bool log: false
        property bool otherEmpty: false
        function whole(page) {
            return page !== null && page.pageTab.state === "open"
                   && page.pageWorktree.loaded && page.pageGraph.finishCount > 0
        }
        onTriggered: {
            const page = window.curPage
            if (!tabCarryTimer.whole(page))
                return
            if (tabCarryTimer.step === 0) {
                // Two tabs — read at the first landing alone, as the verb then moves the strip.
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
            // `sessions=` is the release: two tabs in the strip, one repository in memory.
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

    // PGG_AUTO_ACT=tab-widths: numbers, since a strip that narrowed the wrong tabs frames like one that got it right.
    // `room=` says which give-up the strip is living on (the mark's room first, the names after). `crushed=` is what
    // the widths cannot say — a name drawn away to the mark alone (`TabProbe.tabNamesCrushed`).
    SampleTimer {
        id: tabWidthActTimer
        running: Harness.autoAct === "tab-widths"
        onTriggered: {
            // Every row laid out: both readings walk the view's items, and a just-gained row has none until the next
            // layout.
            if (topBar.bandTabCount <= 0 || topBar.bandTabRun <= 0
                    || tabProbe.tabItemCount() !== topBar.bandTabCount)
                return
            stop()
            Harness.report(
                "tab_widths crushed=" + tabProbe.tabNamesCrushed()
                // The state reached, judged with the crush count: how many tabs crowd the band is the platform's
                // answer (`TabStrip.tabNamesCut`).
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

    // PGG_AUTO_ACT=tab-mark <index>: the tab the hand is on — one of the others, or the picture says nothing new.
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

    // PGG_AUTO_ACT=tab-name <index>: the names a strip of namesakes settled on, and the hover's whole path for the
    // pointed tab — one question, asked of the strip and of the hover (デザイン規約 §タブの所作 / §hover のツールチップ).
    SampleTimer {
        id: tabNameActTimer
        running: Harness.autoAct === "tab-name"
        property bool requested: false
        readonly property int pointed: Number(Harness.autoActArg || 0)
        onTriggered: {
            // Every row laid out first: an unreached row answers "", which reads like a blank name.
            if (topBar.bandTabCount === 0 || tabProbe.tabItemCount() < topBar.bandTabCount)
                return
            if (!tabNameActTimer.requested) {
                tabNameActTimer.requested = true
                tabProbe.pointAtTab(tabNameActTimer.pointed)
                return
            }
            const tip = mainUi.ToolTip.toolTip
            if (!tip.visible)
                return
            stop()
            // `unique=`: no two tabs read alike. `native=`: a backslash in a name or its hover is Windows' own
            // spelling leaking through (規約 §パスの区切り).
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
