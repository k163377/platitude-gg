pragma ComponentBehavior: Bound

import QtQuick
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
    /// must keep the page (デザイン規約 §タブの所作), and a rebuilt one frames the same. With them the command log's
    /// rows as the page was left: a switch must not take the window's log away (`CommandMsg`). Read before the press
    /// that switches, and kept as the operation's witness `stand` once that press went in.
    function standOf(page) {
        return { "page": page, "finished": page.pageGraph.finishCount, "reset": page.pageGraph.resetCount,
                 "logged": page.pageCommands.rowsHeld() }
    }
    /// Whether a graph pass landed after the press. Reported, not waited on: two worktrees whose rows agree send no
    /// pass (`platitude_core::session::DrawnGraph`); the run waits on `PageSettled`.
    function standDrew(op, page) {
        return page.pageGraph.finishCount > op.witnessed("stand").finished
    }
    /// …and over that graph, not in place of it: the same page, and a stream that never restarted
    /// (`GraphModel.resetCount`).
    function standKept(op, page) {
        const stand = op.witnessed("stand")
        return page === stand.page && page.pageGraph.resetCount === stand.reset
    }
    /// …and the log with it. `> 0`: an empty log would pass whatever the switch did.
    function standLogKept(op, page) {
        const logged = op.witnessed("stand").logged
        return logged > 0 && page.pageCommands.rowsHeld() >= logged
    }
    /// Lays the strip out for the model as it stands (`TabProbe.settleStrip`), as a press: answers whether every row
    /// has an item standing for it, and is pressed again on the next beat while not.
    function laidStrip() {
        tabProbe.settleStrip()
        return tabProbe.tabItemCount() === pageRepeater.count
    }

    // What remains after a middle-click. The two tabs are a precondition of the press only: read again after it, the
    // verb's own answer (one tab fewer) becomes a wait nothing can end.
    WindowStage {
        id: middleCloseTimer
        name: "middle_close"
        /// The tab clicked, then the strip laid out for the model that lost it.
        order: ["clicked", "laid"]
        running: Harness.autoAct === "middle-close"
        onTriggered: {
            const op = middleCloseTimer
            if (op.at("start")) {
                if (pageRepeater.count < 2)
                    return
                // The strip's answer is the press's: a row the model just gained has no item until the next layout.
                op.input("clicked", () => {
                    op.witness("beforeCount", pageRepeater.count)
                    const at = Number(Harness.autoActArg)
                    op.witness("closedPath", tabProbe.tabPathAt(at))
                    return topBar.middleClickTab(at)
                })
                return
            }
            // The strip must be laid out before it is read back: until then its items answer by their old indices,
            // and index 0 is the closed tab's own item (`TabProbe.settleStrip`).
            if (op.at("clicked")) {
                if (pageRepeater.count >= op.witnessed("beforeCount"))
                    return
                if (!op.input("laid", () => acts.laidStrip()))
                    return
            }
            // And the tab left in front showing its repository: a page still opening frames the same whether it
            // survived the close or never opened (`AutoActDriver`'s baseline).
            // `gone=` leads: a count that merely fell would pass with the wrong tab closed.
            op.complete({
                "name": "middle_close_landed",
                "when": () => {
                    const kept = window.curPage
                    return { "laid": tabProbe.tabItemCount() === pageRepeater.count,
                             "open": kept !== null && kept.pageTab.state === "open",
                             "page": kept !== null && PageSettled.settled(kept) }
                },
                "report": () => "middle_close gone=" + !tabProbe.hasTabPath(op.witnessed("closedPath"))
                                + " tabs=" + pageRepeater.count
                                + " active=" + tabsModel.currentIndex
                                + " open=" + tabProbe.tabPaths() })
        }
    }

    // The strip's order after a tab was carried across it, read off the items (the drag settles against where the
    // tabs actually sit). The tabs it needs are a precondition of the carry, read in that stage alone.
    WindowStage {
        id: tabDragTimer
        name: "tab_drag"
        order: ["carried"]
        running: Harness.autoAct === "tab-drag"
        /// `<from>:<to>`, the strip indices the tab is carried between.
        readonly property int from: Number((Harness.autoActArg || "3:1").split(":")[0])
        readonly property int to: Number((Harness.autoActArg || "3:1").split(":")[1])
        onTriggered: {
            const op = tabDragTimer
            if (op.at("start")) {
                // Every row laid out: the carry measures against the tabs' own places.
                if (pageRepeater.count < 2 || tabProbe.tabItemCount() !== pageRepeater.count)
                    return
                // And the page under the strip settled — here only: the carry moves to the tab it takes up, whose
                // page then has to open.
                const front = window.curPage
                if (front === null || !PageSettled.settled(front))
                    return
                // The strip's answer is the press's, as the middle click's is: a carry that found no tab never
                // happened.
                op.input("carried", () => {
                    op.witness("before", tabProbe.tabPaths())
                    return topBar.dragTabTo(op.from, op.to)
                })
                return
            }
            // `moved=` leads: an order that merely changed would pass with any two tabs swapped.
            op.complete({
                "name": "tab_drag_landed",
                "when": () => ({ "moved": tabProbe.tabPaths() !== op.witnessed("before"),
                                 "laid": tabProbe.tabItemCount() === pageRepeater.count,
                                 "page": PageSettled.settledUnder(window) }),
                "report": () => {
                    const paths = tabProbe.tabPaths()
                    const was = op.witnessed("before").split(",")
                    return "tab_drag moved=" + (paths.split(",")[op.to] === was[op.from])
                           + " from=" + op.from
                           + " to=" + op.to
                           + " tabs=" + pageRepeater.count
                           + " active=" + tabsModel.currentIndex
                           + " open=" + paths
                } })
        }
    }

    // A tab drawn off its row while the hand is still on it. A settled strip frames the same whether the tab was drawn
    // under the hand or only jumped, so the transform's offset is reported.
    WindowStage {
        id: tabHoldTimer
        name: "tab_hold"
        order: ["lifted"]
        running: Harness.autoAct === "tab-hold"
        onTriggered: {
            if (tabHoldTimer.at("start")) {
                if (pageRepeater.count < 2 || tabProbe.tabItemCount() !== pageRepeater.count)
                    return
                const front = window.curPage
                if (front === null || !PageSettled.settled(front))
                    return
                tabHoldTimer.input("lifted", () => topBar.holdTabAt(Number(Harness.autoActArg || 0)))
                return
            }
            // Nothing after the lift is waited for but the page the hand is over: a hand that has not let go is
            // the whole state, and the page under it is in the picture.
            tabHoldTimer.complete({
                "name": "tab_hold_lifted",
                "when": () => ({ "shift": topBar.heldTabShift() !== 0,
                                 "page": PageSettled.settledUnder(window) }),
                "report": () => "tab_hold lifted=true at=" + (Harness.autoActArg || 0)
                                + " shift=" + topBar.heldTabShift()
                                + " active=" + tabsModel.currentIndex
                                + " open=" + tabProbe.tabPaths() })
        }
    }

    // The strip travelling under a tab held past its end. The window goes down to its floor first: whether N tabs
    // overflow depends on the installed fonts (`tab-widths` differs per OS), and this needs overflow on every machine.
    // The tab is let go once the run has reached the end, and the picture is of the strip it was dropped on.
    WindowStage {
        id: tabEdgeTimer
        name: "tab_edge"
        order: ["floored", "carried", "dropped"]
        running: Harness.autoAct === "tab-edge"
        /// The strip index of the tab carried.
        readonly property int from: Number(Harness.autoActArg || 0)
        onTriggered: {
            const op = tabEdgeTimer
            if (op.at("start") || op.at("floored")) {
                if (pageRepeater.count < 2 || tabProbe.tabItemCount() !== pageRepeater.count)
                    return
                const front = window.curPage
                if (front === null || !PageSettled.settled(front))
                    return
                if (op.at("start")) {
                    op.input("floored", () => {
                        window.width = Math.ceil(window.floorWidth)
                    })
                    return
                }
                // Waited on the strip's own word that it scrolls: which width crowds it cannot be assumed.
                if (!topBar.bandTabScrolls) {
                    // Still uncrowded once the floor has laid out (the band's numbers standing across a polish pass): this
                    // staging cannot carry it. Said with the band's numbers to choose the tab count against.
                    const band = "tab_edge crowded=false tabs=" + pageRepeater.count
                                 + " content=" + Math.round(topBar.bandTabContent)
                                 + " view=" + Math.round(topBar.bandTabsWidth)
                                 + " windowW=" + Math.round(window.width)
                                 + " floorW=" + Math.ceil(window.floorWidth)
                    if (op.still("band", band))
                        op.note(band)
                    return
                }
                op.input("carried", () => topBar.carryTabPastEnd(op.from))
                return
            }
            if (op.at("carried")) {
                if (!Awaited.all("tab_edge_landed", { "end": topBar.runAtEnd(),
                                                      "held": topBar.heldTabIndex() === pageRepeater.count - 1,
                                                      "page": PageSettled.windowSettled(window) }))
                    return
                if (!op.input("dropped", () => topBar.dropCarriedTab()))
                    return
            }
            // The picture is of the strip the tab was dropped on: what the run reached is read off it, the page
            // waited for by the completion's hold.
            op.complete({
                "name": "tab_edge_dropped",
                "when": () => ({}),
                "report": () => "tab_edge landed=true from=" + op.from
                                + " run=" + topBar.runOffset()
                                + " content=" + Math.round(topBar.bandTabContent)
                                + " view=" + Math.round(topBar.bandTabsWidth)
                                + " tabs=" + pageRepeater.count
                                + " active=" + tabsModel.currentIndex
                                + " open=" + tabProbe.tabPaths() })
        }
    }

    // Another worktree's uncommitted row, double-clicked, stands this tab in that worktree (デザイン規約 §タブの所作).
    // Window-level because the landing is the strip's answer: the page stays.
    WindowStage {
        id: carriedOpenTimer
        name: "carried_open"
        order: ["asked"]
        running: Harness.autoAct === "carried-open"
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
            const op = carriedOpenTimer
            if (op.at("start")) {
                const page = window.curPage
                if (!Awaited.all("carried_open_ready", {
                    "open": page !== null && page.pageTab.state === "open",
                    "walked": page !== null && page.pageGraph.finishCount > 0 }))
                    return
                const row = op.carriedRow(page.pageGraph)
                if (row < 0)
                    return
                // Through the row's own handler: calling the page would go green with the row's decision never
                // taken (verify-ui implement.md §壊れない動詞の実装と反復).
                const item = page.pageGraphPane.view.itemAtIndex(row)
                if (item === null)
                    return
                // The row's answer is the press's: a double-click it did not take is pressed again.
                // What stood before it is read first, and kept once the row took it.
                op.input("asked", () => {
                    const wanted = page.pageGraph.carriedPath(row)
                    const beforeCount = pageRepeater.count
                    const stand = acts.standOf(page)
                    if (item.doubleClick(Qt.NoModifier) !== true)
                        return false
                    op.witness("wanted", wanted)
                    op.witness("beforeCount", beforeCount)
                    op.witness("stand", stand)
                })
                return
            }
            // The strip did not grow, and the page in front reads the asked worktree — neither shows in a picture.
            // `rows`: the graph no longer draws the arrived worktree as another's (`GraphModel.carriedRowOf`) —
            // settled cannot see it, as both worktrees are dirty in this preset.
            op.complete({
                "name": "carried_open_landed",
                "when": () => {
                    const front = window.curPage
                    const wanted = op.witnessed("wanted")
                    return { "tabs": pageRepeater.count === op.witnessed("beforeCount"),
                             "open": front !== null && front.pageTab.state === "open",
                             "page": front !== null && PageSettled.settled(front),
                             "stood": front !== null && acts.standsIn(front, wanted),
                             "rows": front !== null && front.pageGraph.carriedRowOf(wanted) < 0 }
                },
                "report": () => {
                    const front = window.curPage
                    return "carried_open tabs=" + pageRepeater.count
                           + " grew=" + (pageRepeater.count !== op.witnessed("beforeCount"))
                           + " stood=" + acts.standsIn(front, op.witnessed("wanted"))
                           + " drew=" + acts.standDrew(op, front)
                           + " kept=" + acts.standKept(op, front)
                           + " wanted=" + op.witnessed("wanted")
                           + " where=" + front.pageTab.repoPath
                } })
        }
    }

    // A WORKTREES row stands this tab in that worktree — the carried row's landing, by the left menu's door
    // (デザイン規約 §左メニューの所作). Window-level for the reason above.
    WindowStage {
        id: worktreeStandTimer
        name: "worktree_stand"
        order: ["log_asked", "asked"]
        running: Harness.autoAct === "worktree-stand"
        onTriggered: {
            const op = worktreeStandTimer
            const page = window.curPage
            if (!op.at("asked")) {
                if (!Awaited.all("worktree_stand_ready", {
                    "open": page !== null && page.pageTab.state === "open",
                    "walked": page !== null && page.pageGraph.finishCount > 0 }))
                    return
                // `log=` needs rows before the switch. An opening's reads are not logged (`Recording::UserOnly`), so
                // a log with none turns background reads on and asks for one (`refreshQuick`), once.
                if (op.at("start")) {
                    op.input("log_asked", () => {
                        if (page.pageCommands.rowsHeld() !== 0)
                            return
                        page.pageCommands.setBackgroundReads(true)
                        page.pageTab.refreshQuick()
                    })
                }
                if (page.pageCommands.rowsHeld() === 0)
                    return
                // The argument is a WORKTREES row, default 1 = the first linked worktree (row 0 is the worktree
                // stood in).
                const trees = page.pageSidebar.worktreesModel
                const row = Harness.autoActArg === "" ? 1 : Number(Harness.autoActArg)
                if (trees.shown() <= row)
                    return
                const full = trees.fullAt(row)
                if (full === "" || acts.standsIn(page, full))
                    return
                // The pane's own door — the one the row's double-click calls (`SidebarRowGestures.activateRow`).
                op.input("asked", () => {
                    op.witness("wanted", full)
                    op.witness("beforeCount", pageRepeater.count)
                    op.witness("stand", acts.standOf(page))
                    page.pageSidebar.activateRow("worktree", trees.nameAt(row), full, trees.headOfWorktree(full))
                })
                return
            }
            op.complete({
                "name": "worktree_stand_landed",
                "when": () => {
                    const front = window.curPage
                    return { "tabs": pageRepeater.count === op.witnessed("beforeCount"),
                             "open": front !== null && front.pageTab.state === "open",
                             "page": front !== null && PageSettled.settled(front),
                             "stood": front !== null && acts.standsIn(front, op.witnessed("wanted")) }
                },
                "report": () => {
                    const front = window.curPage
                    return "worktree_stand tabs=" + pageRepeater.count
                           + " grew=" + (pageRepeater.count !== op.witnessed("beforeCount"))
                           + " stood=" + acts.standsIn(front, op.witnessed("wanted"))
                           + " drew=" + acts.standDrew(op, front)
                           + " kept=" + acts.standKept(op, front)
                           + " log=" + acts.standLogKept(op, front)
                           + " wanted=" + op.witnessed("wanted")
                           + " where=" + front.pageTab.repoPath
                } })
        }
    }

    // A worktree made from a menu stands this tab in it (デザイン規約 §worktree を作る): the page presses
    // (`AutoActWorktreeVerbs`) and the landing is read here, the page being stood elsewhere by it. The tab is taken as
    // it first settles — the page's press comes several beats after its dispatch, and the move behind it takes git's
    // answer and two more processes.
    WindowStage {
        id: worktreeMadeTimer
        name: "worktree_made"
        order: ["stand_held"]
        running: Harness.autoAct === "worktree-new" || Harness.autoAct === "worktree-add"
        /// The kind of the report the press's answer raised, latched off the page's signal (`Connections` below): a
        /// worktree made over a failed `post-checkout` reports and moves too, so the bar alone does not end the run.
        property string reported: ""
        onTriggered: {
            const op = worktreeMadeTimer
            const page = window.curPage
            if (op.at("start")) {
                // Settled, as the page's own verb waits to be before it presses: the opening's graph still landing
                // would count as the graph thrown away (`standKept`).
                if (!Awaited.all("worktree_made_ready", {
                    "open": page !== null && page.pageTab.state === "open",
                    "page": PageSettled.settledUnder(window) }))
                    return
                op.witness("from", page.pageTab.repoPath)
                op.witness("stand", acts.standOf(page))
                op.advance("stand_held")
                return
            }
            // Answered with a report instead (a preset whose git says no after the press): the tab stays, and the bar
            // is the run's subject once it is down.
            if (page === null || page.pageTab.state !== "open")
                return
            const refused = op.reported !== "" && op.reported !== "worktree-hook"
            if (refused && acts.standsIn(page, op.witnessed("from"))) {
                op.complete({
                    "name": "worktree_refused_ready",
                    "when": () => {
                        const page = window.curPage
                        return { "bar": page !== null && page.noticeCard.settled,
                                 "page": page !== null && PageSettled.settled(page) }
                    },
                    "report": () => {
                        const page = window.curPage
                        return "worktree_refused stood=false said=" + page.noticeCard.label
                               + " place=" + page.noticeCard.detailPlace
                               + " says=" + page.noticeCard.detail
                    } })
                return
            }
            op.complete({
                "name": "worktree_made_landed",
                "when": () => {
                    const page = window.curPage
                    return { "stood": page !== null && !acts.standsIn(page, op.witnessed("from")),
                             "page": page !== null && PageSettled.settled(page),
                             "branch": page !== null && page.pageWorktree.branch !== "",
                             "notice": op.reported === "" || (page !== null && page.noticeCard.settled) }
                },
                "report": () => {
                    const page = window.curPage
                    const where = page.pageTab.repoPath
                    // A made worktree's report (`postcheckout`) is the page's, which stood anew: the bar came along.
                    const said = op.reported === "" ? ""
                        : " said=" + page.noticeCard.label + " place=" + page.noticeCard.detailPlace
                          + " says=" + page.noticeCard.detail
                    return "worktree_made stood=" + !acts.standsIn(page, op.witnessed("from"))
                           + " worktree=" + GitFacts.pathLeaf(where)
                           + " branch=" + page.pageWorktree.branch
                           + " tabs=" + pageRepeater.count
                           + " kept=" + acts.standKept(op, page)
                           + " log=" + page.commandsOpen
                           + said
                           + " where=" + where
                } })
        }
    }
    Connections {
        target: worktreeMadeTimer.running ? window.curPage : null
        function onWriteReported(kind, remote, name) {
            worktreeMadeTimer.reported = kind
        }
    }

    // …and away and back: unsent words are filed under the worktree they were written in
    // (デザイン規約 §タブの所作「未コミットのコミットメッセージは立ち位置ごとに憶える」). `kept=` (one page
    // throughout) is what makes `back=` a put-back rather than never taken away. `waits=` is what the page still
    // waits on in the worktree arrived at, of the renames left asking in the one left (`RepoPage.leaveWorktree`):
    // their answers were that session's and do not come, so a wait left standing takes the next session's answer
    // for its own.
    WindowStage {
        id: worktreeDraftTimer
        name: "worktree_draft"
        /// Write and leave, then read the other worktree's empty box and come back, then read the words again.
        order: ["left", "returned"]
        running: Harness.autoAct === "worktree-draft"
        /// Words no repository can produce.
        readonly property string typed: "chore: words written in one worktree"
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
            const op = worktreeDraftTimer
            const page = window.curPage
            if (op.at("start")) {
                if (!Awaited.all("worktree_draft_ready", {
                    "open": page !== null && page.pageTab.state === "open" && !page.pageTab.standing,
                    "loaded": page !== null && page.pageWorktree.loaded,
                    "walked": page !== null && page.pageGraph.finishCount > 0 }))
                    return
                const trees = page.pageSidebar.worktreesModel
                if (trees.shown() < 2)
                    return
                const full = trees.fullAt(1)
                if (full === "" || acts.standsIn(page, full))
                    return
                // The row's answer is the press's: a row not listed yet stands nothing, and is asked again. The
                // worktree the words were written in, and the one stood in between, kept once it went in.
                op.input("left", () => {
                    const home = page.pageTab.repoPath
                    page.pageWip.setMessage(op.typed, "")
                    // A branch's and a tag's rename still asking when the tab moves, set as the ask leaves them: git
                    // answers a local rename well inside the beat a move takes, so only a held-up git opens the
                    // window.
                    page.pendingRenameRemote = "origin"
                    page.pendingRenameTo = "renamed"
                    page.pendingRenameTagRemote = "origin"
                    page.pendingRenameTagFrom = "v1"
                    page.pendingRenameTagTo = "v2"
                    const stand = acts.standOf(page)
                    if (!op.standIn(page, full))
                        return false
                    op.witness("home", home)
                    op.witness("away", full)
                    op.witness("stand", stand)
                })
                return
            }
            if (op.at("left")) {
                if (!Awaited.all("worktree_draft_away", {
                    "stood": op.settled(page, op.witnessed("away")),
                    "page": page !== null && PageSettled.settled(page) }))
                    return
                // The box in the worktree arrived at, and the renames still waiting there — read before the way back.
                op.input("returned", () => {
                    const awayEmpty = page.pageWip.subjectText === "" && page.pageWip.bodyText === ""
                    const held = []
                    if (page.pendingRenameRemote !== "")
                        held.push("branch")
                    if (page.pendingRenameTagRemote !== "")
                        held.push("tag")
                    if (!op.standIn(page, op.witnessed("home")))
                        return false
                    op.witness("awayEmpty", awayEmpty)
                    op.witness("waits", held.length > 0 ? held.join("+") : "none")
                })
                return
            }
            op.complete({
                "name": "worktree_draft_home",
                "when": () => {
                    const page = window.curPage
                    return { "stood": op.settled(page, op.witnessed("home")),
                             "page": page !== null && PageSettled.settled(page) }
                },
                "report": () => {
                    const page = window.curPage
                    return "worktree_draft empty=" + op.witnessed("awayEmpty")
                           + " back=" + (page.pageWip.subjectText === op.typed)
                           // …and the pane holding them is the one on screen, the same half `tab-carry` reads.
                           + " wip=" + page.wipShown
                           + " kept=" + acts.standKept(op, page)
                           + " waits=" + op.witnessed("waits")
                           + " away=" + op.witnessed("away")
                } })
        }
    }

    // The worktree this tab stands in, its folder renamed: the session stands down for git, and the tab stands in the
    // new folder on a session of its own, the page and the words written there kept (デザイン規約 §タブの所作).
    // The argument is `<folder>=<new name>`; the tab is stood in `<folder>` first, by its WORKTREES row. The rename goes
    // in through the left menu's box door (`SidebarPane.beginRename` — the gesture is `worktree-rename`'s to judge).
    WindowStage {
        id: worktreeFollowTimer
        name: "worktree_follow"
        /// Stand in the folder, then write the words and send the rename, then read where the tab stands.
        order: ["stood_in", "renamed"]
        running: Harness.autoAct === "worktree-rename-stand"
        readonly property string typed: "chore: words written before the rename"
        readonly property string folder: Harness.autoActArg.split("=")[0]
        readonly property string renamed: Harness.autoActArg.split("=")[1] || ""
        onTriggered: {
            const op = worktreeFollowTimer
            const page = window.curPage
            if (page === null)
                return
            const trees = page.pageSidebar.worktreesModel
            if (op.at("start")) {
                if (!Awaited.all("worktree_follow_ready", {
                    "ready": !(page.pageTab.state !== "open" || page.pageTab.standing
                        || !page.pageWorktree.loaded || page.pageGraph.finishCount === 0) }))
                    return
                // The folder's row is the press's answer: a folder not listed yet stands nothing, and is asked again.
                // The last row of that name is the one stood in, kept as `from`.
                op.input("stood_in", () => {
                    let found = false
                    let from = ""
                    for (let row = 0; row < trees.shown(); row++) {
                        if (trees.nameAt(row) !== op.folder)
                            continue
                        from = trees.fullAt(row)
                        if (!acts.standsIn(page, from))
                            worktreeDraftTimer.standIn(page, from)
                        found = true
                    }
                    if (found)
                        op.witness("from", from)
                    return found
                })
                return
            }
            if (op.at("stood_in")) {
                const from = op.witnessed("from")
                if (!Awaited.all("worktree_follow_stood", {
                    "stood": worktreeDraftTimer.settled(page, from),
                    "page": PageSettled.settled(page) }))
                    return
                const place = trees.renamedWorktreeFor(from, op.renamed)
                if (place === undefined)
                    return
                op.input("renamed", () => {
                    op.witness("to", place.path)
                    op.witness("beforeCount", pageRepeater.count)
                    page.pageWip.setMessage(op.typed, "")
                    op.witness("stand", acts.standOf(page))
                    page.pageSidebar.beginRename("worktreeFolder", from, op.folder)
                    page.pageSidebar.submitEdit(op.renamed)
                })
                return
            }
            op.complete({
                "name": "worktree_follow_landed",
                "when": () => {
                    const page = window.curPage
                    return { "stood": worktreeDraftTimer.settled(page, op.witnessed("to")),
                             "page": page !== null && PageSettled.settled(page) }
                },
                "report": () => {
                    const page = window.curPage
                    return "worktree_follow stood=" + acts.standsIn(page, op.witnessed("to"))
                           + " tabs=" + pageRepeater.count
                           + " grew=" + (pageRepeater.count !== op.witnessed("beforeCount"))
                           + " kept=" + acts.standKept(op, page)
                           + " draft=" + (page.pageWip.subjectText === op.typed)
                           + " box=" + (page.pageSidebar.editKey !== "")
                           + " gone=" + (page.pageSidebar.worktreesModel.worktreeFacts(op.witnessed("from"))
                                         === undefined)
                           + " where=" + page.pageTab.repoPath
                } })
        }
    }

    // The hover over a tab standing in a linked worktree says that worktree's path, not the repository's
    // (デザイン規約 §hover のツールチップ).
    WindowStage {
        id: worktreeTipTimer
        name: "worktree_tip"
        /// Stand this tab in a linked worktree, then put the hand on it, then read what came out.
        order: ["stood_in", "pointed"]
        running: Harness.autoAct === "worktree-tip"
        onTriggered: {
            const op = worktreeTipTimer
            const page = window.curPage
            if (op.at("start")) {
                if (!Awaited.all("worktree_tip_ready", {
                    "open": page !== null && page.pageTab.state === "open",
                    "walked": page !== null && page.pageGraph.finishCount > 0 }))
                    return
                const trees = page.pageSidebar.worktreesModel
                const row = Harness.autoActArg === "" ? 1 : Number(Harness.autoActArg)
                if (trees.shown() <= row)
                    return
                const full = trees.fullAt(row)
                if (full === "" || acts.standsIn(page, full))
                    return
                op.input("stood_in", () => {
                    op.witness("wanted", full)
                    page.pageSidebar.activateRow("worktree", trees.nameAt(row), full, trees.headOfWorktree(full))
                })
                return
            }
            if (op.at("stood_in")) {
                // The tab item as well as the page: it arrives with the next layout, and a hand on nothing waits
                // forever.
                if (!Awaited.all("worktree_tip_stood", {
                    "stood": page !== null && acts.standsIn(page, op.witnessed("wanted")),
                    "tabs": tabProbe.tabItemCount() > 0,
                    "page": page !== null && PageSettled.settled(page) }))
                    return
                op.input("pointed", () => tabProbe.pointAtTab(0))
                return
            }
            // The tip is on a delay, so this waits it out.
            // `worktree=` is the claim; `native=` is the spelling, read as `tab-name` reads it.
            op.complete({
                "name": "worktree_tip_shown",
                "when": () => ({ "tip": Tips.box.visible,
                                 "page": PageSettled.windowSettled(window) }),
                "report": () => {
                    const tip = Tips.box
                    return "worktree_tip tip=" + tip.visible
                           + " worktree=" + (tip.text.toLowerCase() === op.witnessed("wanted").toLowerCase())
                           + " native=" + tip.text.includes("\\")
                           + " said=" + tip.text
                } })
        }
    }

    // The worktree-name run on a tab stays with it, and coming back stands the reader in that worktree again
    // (デザイン規約 §タブの所作). The argument picks the landing: `away` (the tab not in front) or `back` (default).
    // At `away` the tab left behind has no page (`Hub::release_tab`), so the strip's row answers (`TabProbe.tabTrees`).
    //
    // Two repositories, the linked worktrees in front: `--preset basic --preset worktrees`.
    WindowStage {
        id: worktreeKeptTimer
        name: "worktree_kept"
        /// Stand this tab in a linked worktree, then leave it, then (unless the run stops away) come back to it; then
        /// the strip laid out for the landing.
        order: (Harness.autoActArg || "").indexOf("away") < 0 ? ["stood_in", "left", "returned", "laid"]
                                                               : ["stood_in", "left", "laid"]
        running: Harness.autoAct === "worktree-kept"
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
        /// The run's line, read as it completes.
        function line() {
            const page = window.curPage
            const seat = worktreeKeptTimer.witnessed("seat")
            const trees = tabProbe.tabTrees().split(",")
            return "worktree_kept tabs=" + pageRepeater.count
                   + " kept=" + (trees[seat] !== undefined && trees[seat] !== "")
                   + " front=" + (tabsModel.currentIndex === seat)
                   + " stood=" + acts.standsIn(page, worktreeKeptTimer.witnessed("wanted"))
                   // `kept=` is off the model, which names the worktree even after the strip gave the run up for
                   // room; 0 here is that (デザイン規約 §ウィンドウの縁 の譲る順).
                   + " wide=" + tabProbe.tabTreeWidths()
                   + " says=" + tabProbe.tabTrees()
                   + " wanted=" + worktreeKeptTimer.witnessed("wanted")
        }
        /// What the landing stands on, read now: away from the tab that was stood, or back on it and stood again.
        function landing() {
            const page = window.curPage
            const seat = worktreeKeptTimer.witnessed("seat")
            if (!worktreeKeptTimer.comesBack)
                return { "away": tabsModel.currentIndex !== seat,
                         "page": page !== null && PageSettled.settled(page) }
            return { "front": tabsModel.currentIndex === seat,
                     "stood": page !== null && acts.standsIn(page, worktreeKeptTimer.witnessed("wanted")),
                     "page": page !== null && PageSettled.settled(page) }
        }
        onTriggered: {
            const op = worktreeKeptTimer
            const page = window.curPage
            if (!op.whole(page))
                return
            if (op.at("start")) {
                // Two tabs — read in this stage alone, as the verb then moves the strip.
                if (!Awaited.all("worktree_kept_ready", { "tabs": pageRepeater.count >= 2,
                                                          "page": PageSettled.settled(page) }))
                    return
                const trees = page.pageSidebar.worktreesModel
                const row = op.row
                if (trees.shown() <= row)
                    return
                const full = trees.fullAt(row)
                if (full === "" || acts.standsIn(page, full))
                    return
                op.input("stood_in", () => {
                    op.witness("wanted", full)
                    op.witness("seat", tabsModel.currentIndex)
                    page.pageSidebar.activateRow("worktree", trees.nameAt(row), full, trees.headOfWorktree(full))
                })
                return
            }
            if (op.at("stood_in")) {
                if (!Awaited.all("worktree_kept_stood", { "stood": acts.standsIn(page, op.witnessed("wanted")),
                                                          "page": PageSettled.settled(page) }))
                    return
                op.input("left", () => tabsModel.setCurrentIndex(op.witnessed("seat") === 0 ? 1 : 0))
                return
            }
            if (op.at("left") && op.comesBack) {
                if (!Awaited.all("worktree_kept_away", { "away": tabsModel.currentIndex !== op.witnessed("seat"),
                                                         "page": PageSettled.settled(page) }))
                    return
                op.input("returned", () => tabsModel.setCurrentIndex(op.witnessed("seat")))
                return
            }
            // The strip answers for the model as it stands before its rows are read (`TabProbe.settleStrip`).
            if (op.at("left") || op.at("returned")) {
                if (!Awaited.all(op.comesBack ? "worktree_kept_back" : "worktree_kept_away", op.landing()))
                    return
                if (!op.input("laid", () => acts.laidStrip()))
                    return
            }
            op.complete({
                "name": "worktree_kept_landed",
                "when": () => op.landing(),
                "report": () => op.line() })
        }
    }

    // Reopening an existing path selects the tab it already has.
    WindowStage {
        id: openAgainTimer
        name: "open_again"
        order: ["asked"]
        running: Harness.autoAct === "open-again"
        /// The tab the ask should bring to the front; -1 when the run named its own path, whose spelling does not say
        /// which tab holds it.
        readonly property int wantIndex: Harness.autoActArg !== "" ? -1 : 0
        onTriggered: {
            const op = openAgainTimer
            if (op.at("start")) {
                if (pageRepeater.count === 0)
                    return
                // With no argument, the strip's own spelling of tab 0 — "" until the next layout, and asking with ""
                // opens nothing, which passes for this verb's completion.
                const path = Harness.autoActArg !== "" ? Harness.autoActArg : tabProbe.tabPathAt(0)
                if (path === "")
                    return
                op.input("asked", () => {
                    op.witness("asked", path)
                    op.witness("beforeCount", pageRepeater.count)
                    tabsModel.openRepositoryPath(path)
                })
                return
            }
            // Tab 0 asked from a strip standing on another has somewhere to arrive; else "went to its tab" and "did
            // nothing" report the same.
            if (pageRepeater.count !== op.witnessed("beforeCount")
                    || tabsModel.currentIndex < 0
                    || (op.wantIndex >= 0
                        && tabsModel.currentIndex !== op.wantIndex)
                    // The strip is read back below (`middle-close` above).
                    || tabProbe.tabItemCount() !== pageRepeater.count)
                return
            // And the front tab showing its repository (`middle-close` above, same baseline).
            op.complete({
                "name": "open_again_landed",
                "when": () => {
                    const front = window.curPage
                    return { "open": front !== null && front.pageTab.state === "open",
                             "page": front !== null && PageSettled.settled(front) }
                },
                "report": () => "open_again tabs=" + pageRepeater.count
                                + " active=" + tabsModel.currentIndex
                                + " asked=" + op.witnessed("asked")
                                + " open=" + tabProbe.tabPaths() })
        }
    }

    // What a tab switch carries and what it drops, in three landings: mark the front tab, read the other, come back.
    // Each landing waits for a whole page — a page still opening answers like an emptied one. Waited on
    // `currentTabId`, not the row (rows renumber). The page is a different object at each landing (built and taken
    // down with its tab), so nothing but the words may be held across one.
    WindowStage {
        id: tabCarryTimer
        name: "tab_carry"
        /// Mark the first tab and leave it, then read the second and come back, then read the first again.
        order: ["away", "back"]
        running: Harness.autoAct === "tab-carry"
        /// Words no repository can produce.
        readonly property string typed: "chore: words that outlived a tab switch"
        function whole(page) {
            return page !== null && page.pageTab.state === "open"
                   && page.pageWorktree.loaded && page.pageGraph.finishCount > 0
        }
        onTriggered: {
            const op = tabCarryTimer
            const page = window.curPage
            if (!Awaited.all("tab_carry_ready", { "ready": op.whole(page),
                                                  "page": PageSettled.windowSettled(window) }))
                return
            if (op.at("start")) {
                // Two tabs — read at the first landing alone, as the verb then moves the strip.
                if (!Awaited.all("tab_carry_tabs", { "tabs": pageRepeater.count >= 2 }))
                    return
                // A layout nobody starts in, so "it followed" cannot be read off a page that was already like this.
                // The tab the mark was left on is kept.
                op.input("away", () => {
                    page.foldByHand(true)
                    page.commandsOpen = true
                    page.pageWip.setMessage(op.typed, "")
                    op.witness("firstTab", tabsModel.currentTabId)
                    tabsModel.setCurrentIndex(tabsModel.currentIndex === 0 ? 1 : 0)
                })
                return
            }
            if (op.at("away")) {
                if (!Awaited.all("tab_carry_moved", { "moved": tabsModel.currentTabId !== op.witnessed("firstTab") }))
                    return
                // Read at the second landing and reported at the third: the layout followed the reader, the words did
                // not.
                op.input("back", () => {
                    op.witness("folded", page.sidebarCollapsed)
                    op.witness("log", page.commandsOpen)
                    op.witness("otherEmpty", page.pageWip.subjectText === "" && page.pageWip.bodyText === "")
                    tabsModel.setCurrentIndex(tabsModel.currentIndex === 0 ? 1 : 0)
                })
                return
            }
            // `sessions=` is the release: two tabs in the strip, one repository in memory.
            op.complete({
                "name": "tab_carry_back",
                "when": () => ({ "back": tabsModel.currentTabId === op.witnessed("firstTab"),
                                 "ready": op.whole(window.curPage) }),
                "report": () => {
                    const page = window.curPage
                    return "tab_carry tabs=" + pageRepeater.count
                           + " sessions=" + Harness.openSessionCount()
                           + " folded=" + op.witnessed("folded")
                           + " log=" + op.witnessed("log")
                           + " empty=" + op.witnessed("otherEmpty")
                           + " back=" + (page.pageWip.subjectText === op.typed)
                           // …and the pane holding them is the one on screen: words put back behind the commit
                           // details are only half of them being kept.
                           + " wip=" + page.wipShown
                } })
        }
    }

    // PGG_AUTO_ACT=tab-widths: numbers, since a strip that narrowed the wrong tabs frames like one that got it right.
    // `room=` says which give-up the strip is living on (the mark's room first, the names after). `crushed=` is what
    // the widths cannot say — a name drawn away to the mark alone (`TabProbe.tabNamesCrushed`).
    WindowStage {
        id: tabWidthActTimer
        name: "tab_width_act"
        running: Harness.autoAct === "tab-widths"
        onTriggered: {
            // Every row laid out: both readings walk the view's items, and a just-gained row has none until the next
            // layout.
            tabWidthActTimer.complete({
                "name": "tab_width_act_ready",
                "when": () => ({ "ready": !(topBar.bandTabCount <= 0 || topBar.bandTabRun <= 0
                                            || tabProbe.tabItemCount() !== topBar.bandTabCount),
                                 "page": PageSettled.settledUnder(window) }),
                "report": () => "tab_widths crushed=" + tabProbe.tabNamesCrushed()
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
                    + " widths=" + tabProbe.tabWidths() })
        }
    }

    // PGG_AUTO_ACT=tab-mark <index>: the tab the hand is on — one of the others, or the picture says nothing new.
    WindowStage {
        id: tabMarkActTimer
        name: "tab_mark_act"
        order: ["pointed"]
        running: Harness.autoAct === "tab-mark"
        /// The strip index of the tab the hand is put on.
        readonly property int pointed: Number(Harness.autoActArg || 1)
        onTriggered: {
            const op = tabMarkActTimer
            if (topBar.bandTabCount < op.pointed)
                return
            if (op.at("start")) {
                op.input("pointed", () => {
                    op.witness("beforeMarks", tabProbe.tabMarks())
                    tabProbe.pointAtTab(op.pointed)
                })
                return
            }
            // The marks, and the pointed tab's path, which comes up a rest after the hand (`TabItemDelegate`): ended
            // before it, the picture holds the box or not by the machine's pace. The box is the shared one, so it is
            // read as that tab's by its target.
            op.complete({
                "name": "tab_marks",
                "when": () => {
                    const tip = Tips.box
                    return { "marks": tabProbe.tabMarks() !== op.witnessed("beforeMarks"),
                             "tip": tip.visible && tip.parent === tabProbe.view.itemAtIndex(op.pointed),
                             "page": PageSettled.windowSettled(window) }
                },
                "report": () => "tab_marks tabs=" + topBar.bandTabCount
                                + " current=" + tabsModel.currentIndex
                                + " pointed=" + op.pointed
                                + " marks=" + tabProbe.tabMarks() })
        }
    }

    // PGG_AUTO_ACT=tab-name <index>: the names a strip of namesakes settled on, and the hover's whole path for the
    // pointed tab — one question, asked of the strip and of the hover (デザイン規約 §タブの所作 / §hover のツールチップ).
    WindowStage {
        id: tabNameActTimer
        name: "tab_name_act"
        order: ["pointed"]
        running: Harness.autoAct === "tab-name"
        readonly property int pointed: Number(Harness.autoActArg || 0)
        onTriggered: {
            const op = tabNameActTimer
            // Every row laid out first: an unreached row answers "", which reads like a blank name.
            if (!Awaited.all("tab_name_act_ready", {
                "ready": !(topBar.bandTabCount === 0 || tabProbe.tabItemCount() < topBar.bandTabCount),
                "page": PageSettled.windowSettled(window) }))
                return
            if (op.at("start")) {
                op.input("pointed", () => tabProbe.pointAtTab(op.pointed))
                return
            }
            // `unique=`: no two tabs read alike. `native=`: a backslash in a name or its hover is Windows' own
            // spelling leaking through (規約 §パスの区切り).
            op.complete({
                "name": "tab_name_tip",
                "when": () => ({ "tip": Tips.box.visible }),
                "report": () => {
                    const tip = Tips.box
                    const titles = tabProbe.tabTitles()
                    const names = titles.split(",")
                    return "tab_names tabs=" + topBar.bandTabCount
                           + " pointed=" + op.pointed
                           + " unique=" + names.every((name, at) => names.indexOf(name) === at)
                           + " native=" + (names.some(name => name.includes("\\")) || tip.text.includes("\\"))
                           + " tip=" + tip.visible
                           + " titles=" + titles
                           + " said=" + tip.text
                } })
        }
    }

    // PGG_AUTO_ACT=solo: the harness holds the real lock on the config directory before starting this process, so the
    // picture is of the mechanism.
    WindowStage {
        id: soloActTimer
        name: "solo_act"
        running: Harness.autoAct === "solo"
        onTriggered: {
            soloActTimer.complete({
                "name": "solo_act_ready",
                "when": () => ({ "ready": !(!AppBackend.alreadyRunning || !gate.visible),
                                 "page": PageSettled.windowSettled(window) }),
                "report": () => "solo blocked=" + AppBackend.alreadyRunning
                                + " held=" + (AppBackend.heldElsewhere !== "")
                                + " gate=" + gate.visible
                                + " main=" + mainUi.visible })
        }
    }
}
