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
    WindowStage {
        id: middleCloseTimer
        name: "middle_close"
        order: ["clicked"]
        running: Harness.autoAct === "middle-close"
        property int beforeCount: -1
        property string closedPath: ""
        onTriggered: {
            const op = middleCloseTimer
            if (op.at("start")) {
                if (pageRepeater.count < 2)
                    return
                // The strip's answer is the press's: a row the model just gained has no item until the next layout.
                op.input("clicked", () => {
                    op.beforeCount = pageRepeater.count
                    const at = Number(Harness.autoActArg)
                    op.closedPath = tabProbe.tabPathAt(at)
                    return topBar.middleClickTab(at)
                })
                return
            }
            // The strip must be laid out before it is read back: until then its items answer by their old indices,
            // and index 0 is the closed tab's own item (`TabProbe.settleStrip`).
            if (pageRepeater.count >= op.beforeCount)
                return
            tabProbe.settleStrip()
            if (tabProbe.tabItemCount() !== pageRepeater.count)
                return
            // And the tab left in front showing its repository: a page still opening frames the same whether it
            // survived the close or never opened (`AutoActDriver`'s baseline).
            const kept = window.curPage
            if (!Awaited.all("middle_close_landed", {
                "open": kept !== null && kept.pageTab.state === "open",
                "page": kept !== null && PageSettled.settled(kept) }))
                return
            // `gone=` leads: a count that merely fell would pass with the wrong tab closed.
            op.complete(() => "middle_close gone=" + !tabProbe.hasTabPath(op.closedPath)
                               + " tabs=" + pageRepeater.count
                               + " active=" + tabsModel.currentIndex
                               + " open=" + tabProbe.tabPaths())
        }
    }

    // The strip's order after a tab was carried across it, read off the items (the drag settles against where the
    // tabs actually sit). The tabs it needs are a precondition of the carry, read in that stage alone.
    WindowStage {
        id: tabDragTimer
        name: "tab_drag"
        order: ["carried"]
        running: Harness.autoAct === "tab-drag"
        property string before: ""
        property int from: 0
        property int to: 0
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
                    const asked = (Harness.autoActArg || "3:1").split(":")
                    op.from = Number(asked[0])
                    op.to = Number(asked[1])
                    op.before = tabProbe.tabPaths()
                    return topBar.dragTabTo(op.from, op.to)
                })
                return
            }
            if (!Awaited.all("tab_drag_landed", {
                "moved": tabProbe.tabPaths() !== op.before,
                "laid": tabProbe.tabItemCount() === pageRepeater.count,
                "page": PageSettled.settledUnder(window) }))
                return
            // `moved=` leads: an order that merely changed would pass with any two tabs swapped.
            op.complete(() => {
                const paths = tabProbe.tabPaths()
                const was = op.before.split(",")
                return "tab_drag moved=" + (paths.split(",")[op.to] === was[op.from])
                       + " from=" + op.from
                       + " to=" + op.to
                       + " tabs=" + pageRepeater.count
                       + " active=" + tabsModel.currentIndex
                       + " open=" + paths
            })
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
            if (!Awaited.all("tab_hold_lifted", {
                "shift": topBar.heldTabShift() !== 0,
                "page": PageSettled.settledUnder(window) }))
                return
            tabHoldTimer.complete(() => "tab_hold lifted=true at=" + (Harness.autoActArg || 0)
                                         + " shift=" + topBar.heldTabShift()
                                         + " active=" + tabsModel.currentIndex
                                         + " open=" + tabProbe.tabPaths())
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
        property int from: 0
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
                    // Still uncrowded once the floor has laid out (the band's numbers standing across a frame): this
                    // staging cannot carry it. Said with the band's numbers to choose the tab count against.
                    const band = "tab_edge crowded=false tabs=" + pageRepeater.count
                                 + " content=" + Math.round(topBar.bandTabContent)
                                 + " view=" + Math.round(topBar.bandTabsWidth)
                                 + " windowW=" + Math.round(window.width)
                                 + " floorW=" + Math.ceil(window.floorWidth)
                    if (op.still("band", band))
                        Awaited.note(band)
                    return
                }
                op.input("carried", () => {
                    op.from = Number(Harness.autoActArg || 0)
                    return topBar.carryTabPastEnd(op.from)
                })
                return
            }
            if (!Awaited.all("tab_edge_landed", { "end": topBar.runAtEnd(),
                                                  "held": topBar.heldTabIndex() === pageRepeater.count - 1,
                                                  "page": PageSettled.windowSettled(window) }))
                return
            op.input("dropped", () => topBar.dropCarriedTab())
            op.complete(() => "tab_edge landed=true from=" + op.from
                               + " run=" + topBar.runOffset()
                               + " content=" + Math.round(topBar.bandTabContent)
                               + " view=" + Math.round(topBar.bandTabsWidth)
                               + " tabs=" + pageRepeater.count
                               + " active=" + tabsModel.currentIndex
                               + " open=" + tabProbe.tabPaths())
        }
    }

    // Another worktree's uncommitted row, double-clicked, stands this tab in that worktree (デザイン規約 §タブの所作).
    // Window-level because the landing is the strip's answer: the page stays.
    WindowStage {
        id: carriedOpenTimer
        name: "carried_open"
        order: ["asked"]
        running: Harness.autoAct === "carried-open"
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
                op.input("asked", () => {
                    op.wanted = page.pageGraph.carriedPath(row)
                    op.beforeCount = pageRepeater.count
                    acts.holdStand(page)
                    return item.doubleClick(Qt.NoModifier) === true
                })
                return
            }
            // The strip did not grow, and the page in front reads the asked worktree — neither shows in a picture.
            // `rows`: the graph no longer draws the arrived worktree as another's (`GraphModel.carriedRowOf`) —
            // settled cannot see it, as both worktrees are dirty in this preset.
            const front = window.curPage
            if (!Awaited.all("carried_open_landed", {
                "tabs": pageRepeater.count === op.beforeCount,
                "open": front !== null && front.pageTab.state === "open",
                "page": front !== null && PageSettled.settled(front),
                "stood": front !== null && acts.standsIn(front, op.wanted),
                "rows": front !== null && front.pageGraph.carriedRowOf(op.wanted) < 0 }))
                return
            op.complete(() => {
                const front = window.curPage
                return "carried_open tabs=" + pageRepeater.count
                       + " grew=" + (pageRepeater.count !== op.beforeCount)
                       + " stood=" + acts.standsIn(front, op.wanted)
                       + " drew=" + acts.standDrew(front)
                       + " kept=" + acts.standKept(front)
                       + " wanted=" + op.wanted
                       + " where=" + front.pageTab.repoPath
            })
        }
    }

    // A WORKTREES row stands this tab in that worktree — the carried row's landing, by the left menu's door
    // (デザイン規約 §左メニューの所作). Window-level for the reason above.
    WindowStage {
        id: worktreeStandTimer
        name: "worktree_stand"
        order: ["log_asked", "asked"]
        running: Harness.autoAct === "worktree-stand"
        property int beforeCount: -1
        property string wanted: ""
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
                    op.wanted = full
                    op.beforeCount = pageRepeater.count
                    acts.holdStand(page)
                    page.pageSidebar.activateRow("worktree", trees.nameAt(row), full, trees.headOfWorktree(full))
                })
                return
            }
            if (!Awaited.all("worktree_stand_landed", {
                "tabs": pageRepeater.count === op.beforeCount,
                "open": page !== null && page.pageTab.state === "open",
                "page": page !== null && PageSettled.settled(page),
                "stood": page !== null && acts.standsIn(page, op.wanted) }))
                return
            op.complete(() => {
                const front = window.curPage
                return "worktree_stand tabs=" + pageRepeater.count
                       + " grew=" + (pageRepeater.count !== op.beforeCount)
                       + " stood=" + acts.standsIn(front, op.wanted)
                       + " drew=" + acts.standDrew(front)
                       + " kept=" + acts.standKept(front)
                       + " log=" + acts.standLogKept(front)
                       + " wanted=" + op.wanted
                       + " where=" + front.pageTab.repoPath
            })
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
        property string from: ""
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
                op.from = page.pageTab.repoPath
                acts.holdStand(page)
                op.advance("stand_held")
                return
            }
            // Answered with a report instead (a preset whose git says no after the press): the tab stays, and the bar
            // is the run's subject once it is down.
            if (page === null || page.pageTab.state !== "open")
                return
            const refused = op.reported !== "" && op.reported !== "worktree-hook"
            if (refused && acts.standsIn(page, op.from)) {
                if (!Awaited.all("worktree_refused_ready", { "bar": page.noticeCard.settled,
                                                             "page": PageSettled.settled(page) }))
                    return
                op.complete(() => {
                    const page = window.curPage
                    return "worktree_refused stood=false said=" + page.noticeCard.label
                           + " place=" + page.noticeCard.detailPlace
                           + " says=" + page.noticeCard.detail
                })
                return
            }
            if (!Awaited.all("worktree_made_landed", {
                "stood": !acts.standsIn(page, op.from),
                "page": PageSettled.settled(page),
                "branch": page.pageWorktree.branch !== "",
                "notice": op.reported === "" || page.noticeCard.settled }))
                return
            op.complete(() => {
                const page = window.curPage
                const where = page.pageTab.repoPath
                // A made worktree's report (`postcheckout`) is the page's, which stood anew: the bar came along.
                const said = op.reported === "" ? ""
                    : " said=" + page.noticeCard.label + " place=" + page.noticeCard.detailPlace
                      + " says=" + page.noticeCard.detail
                return "worktree_made stood=" + !acts.standsIn(page, op.from)
                       + " worktree=" + GitFacts.pathLeaf(where)
                       + " branch=" + page.pageWorktree.branch
                       + " tabs=" + pageRepeater.count
                       + " kept=" + acts.standKept(page)
                       + " log=" + page.commandsOpen
                       + said
                       + " where=" + where
            })
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
        /// The worktree they were written in, and the one stood in between.
        property string home: ""
        property string away: ""
        property bool awayEmpty: false
        property string waits: ""
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
                // The row's answer is the press's: a row not listed yet stands nothing, and is asked again.
                op.input("left", () => {
                    op.home = page.pageTab.repoPath
                    op.away = full
                    page.pageWip.setMessage(op.typed, "")
                    // A branch's and a tag's rename still asking when the tab moves, set as the ask leaves them: git
                    // answers a local rename well inside the beat a move takes, so only a held-up git opens the
                    // window.
                    page.pendingRenameRemote = "origin"
                    page.pendingRenameTo = "renamed"
                    page.pendingRenameTagRemote = "origin"
                    page.pendingRenameTagFrom = "v1"
                    page.pendingRenameTagTo = "v2"
                    acts.holdStand(page)
                    return op.standIn(page, full)
                })
                return
            }
            if (op.at("left")) {
                if (!Awaited.all("worktree_draft_away", {
                    "stood": op.settled(page, op.away),
                    "page": page !== null && PageSettled.settled(page) }))
                    return
                op.input("returned", () => {
                    op.awayEmpty = page.pageWip.subjectText === "" && page.pageWip.bodyText === ""
                    const held = []
                    if (page.pendingRenameRemote !== "")
                        held.push("branch")
                    if (page.pendingRenameTagRemote !== "")
                        held.push("tag")
                    op.waits = held.length > 0 ? held.join("+") : "none"
                    return op.standIn(page, op.home)
                })
                return
            }
            if (!Awaited.all("worktree_draft_home", {
                "stood": op.settled(page, op.home),
                "page": page !== null && PageSettled.settled(page) }))
                return
            op.complete(() => {
                const page = window.curPage
                return "worktree_draft empty=" + op.awayEmpty
                       + " back=" + (page.pageWip.subjectText === op.typed)
                       // …and the pane holding them is the one on screen, the same half `tab-carry` reads.
                       + " wip=" + page.wipShown
                       + " kept=" + acts.standKept(page)
                       + " waits=" + op.waits
                       + " away=" + op.away
            })
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
        property string from: ""
        property string to: ""
        property int beforeCount: -1
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
                op.input("stood_in", () => {
                    let found = false
                    for (let row = 0; row < trees.shown(); row++) {
                        if (trees.nameAt(row) !== op.folder)
                            continue
                        op.from = trees.fullAt(row)
                        if (!acts.standsIn(page, op.from))
                            worktreeDraftTimer.standIn(page, op.from)
                        found = true
                    }
                    return found
                })
                return
            }
            if (op.at("stood_in")) {
                if (!Awaited.all("worktree_follow_stood", {
                    "stood": worktreeDraftTimer.settled(page, op.from),
                    "page": PageSettled.settled(page) }))
                    return
                const place = trees.renamedWorktreeFor(op.from, op.renamed)
                if (place === undefined)
                    return
                op.input("renamed", () => {
                    op.to = place.path
                    op.beforeCount = pageRepeater.count
                    page.pageWip.setMessage(op.typed, "")
                    acts.holdStand(page)
                    page.pageSidebar.beginRename("worktreeFolder", op.from, op.folder)
                    page.pageSidebar.submitEdit(op.renamed)
                })
                return
            }
            if (!Awaited.all("worktree_follow_landed", {
                "stood": worktreeDraftTimer.settled(page, op.to),
                "page": PageSettled.settled(page) }))
                return
            op.complete(() => {
                const page = window.curPage
                return "worktree_follow stood=" + acts.standsIn(page, op.to)
                       + " tabs=" + pageRepeater.count
                       + " grew=" + (pageRepeater.count !== op.beforeCount)
                       + " kept=" + acts.standKept(page)
                       + " draft=" + (page.pageWip.subjectText === op.typed)
                       + " box=" + (page.pageSidebar.editKey !== "")
                       + " gone=" + (page.pageSidebar.worktreesModel.worktreeFacts(op.from) === undefined)
                       + " where=" + page.pageTab.repoPath
            })
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
        property string wanted: ""
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
                    op.wanted = full
                    page.pageSidebar.activateRow("worktree", trees.nameAt(row), full, trees.headOfWorktree(full))
                })
                return
            }
            if (op.at("stood_in")) {
                // The tab item as well as the page: it arrives with the next layout, and a hand on nothing waits
                // forever.
                if (!Awaited.all("worktree_tip_stood", {
                    "stood": page !== null && acts.standsIn(page, op.wanted),
                    "tabs": tabProbe.tabItemCount() > 0,
                    "page": page !== null && PageSettled.settled(page) }))
                    return
                op.input("pointed", () => tabProbe.pointAtTab(0))
                return
            }
            // The tip is on a delay, so this waits it out.
            if (!Awaited.all("worktree_tip_shown", { "tip": Tips.box.visible,
                                                     "page": PageSettled.windowSettled(window) }))
                return
            // `worktree=` is the claim; `native=` is the spelling, read as `tab-name` reads it.
            op.complete(() => {
                const tip = Tips.box
                return "worktree_tip tip=" + tip.visible
                       + " worktree=" + (tip.text.toLowerCase() === op.wanted.toLowerCase())
                       + " native=" + tip.text.includes("\\")
                       + " said=" + tip.text
            })
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
        /// Stand this tab in a linked worktree, then leave it, then (unless the run stops away) come back to it.
        order: (Harness.autoActArg || "").indexOf("away") < 0 ? ["stood_in", "left", "returned"]
                                                               : ["stood_in", "left"]
        running: Harness.autoAct === "worktree-kept"
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
        /// The run's line, read as it completes.
        function line() {
            const page = window.curPage
            tabProbe.settleStrip()
            const trees = tabProbe.tabTrees().split(",")
            return "worktree_kept tabs=" + pageRepeater.count
                   + " kept=" + (trees[worktreeKeptTimer.seat] !== undefined && trees[worktreeKeptTimer.seat] !== "")
                   + " front=" + (tabsModel.currentIndex === worktreeKeptTimer.seat)
                   + " stood=" + acts.standsIn(page, worktreeKeptTimer.wanted)
                   // `kept=` is off the model, which names the worktree even after the strip gave the run up for
                   // room; 0 here is that (デザイン規約 §ウィンドウの縁 の譲る順).
                   + " wide=" + tabProbe.tabTreeWidths()
                   + " says=" + tabProbe.tabTrees()
                   + " wanted=" + worktreeKeptTimer.wanted
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
                    op.wanted = full
                    op.seat = tabsModel.currentIndex
                    page.pageSidebar.activateRow("worktree", trees.nameAt(row), full, trees.headOfWorktree(full))
                })
                return
            }
            if (op.at("stood_in")) {
                if (!Awaited.all("worktree_kept_stood", { "stood": acts.standsIn(page, op.wanted),
                                                          "page": PageSettled.settled(page) }))
                    return
                op.input("left", () => tabsModel.setCurrentIndex(op.seat === 0 ? 1 : 0))
                return
            }
            if (op.at("left")) {
                if (!Awaited.all("worktree_kept_away", { "away": tabsModel.currentIndex !== op.seat,
                                                         "page": PageSettled.settled(page) }))
                    return
                if (!op.comesBack) {
                    op.complete(() => op.line())
                    return
                }
                op.input("returned", () => tabsModel.setCurrentIndex(op.seat))
                return
            }
            if (!Awaited.all("worktree_kept_back", { "front": tabsModel.currentIndex === op.seat,
                                                     "stood": acts.standsIn(page, op.wanted),
                                                     "page": PageSettled.settled(page) }))
                return
            op.complete(() => op.line())
        }
    }

    // Reopening an existing path selects the tab it already has.
    WindowStage {
        id: openAgainTimer
        name: "open_again"
        order: ["asked"]
        running: Harness.autoAct === "open-again"
        property string asked: ""
        property int beforeCount: -1
        /// The tab the ask should bring to the front; -1 when the run named its own path, whose spelling does not say
        /// which tab holds it.
        property int wantIndex: -1
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
                    op.asked = path
                    op.wantIndex = Harness.autoActArg !== "" ? -1 : 0
                    op.beforeCount = pageRepeater.count
                    tabsModel.openRepositoryPath(path)
                })
                return
            }
            // Tab 0 asked from a strip standing on another has somewhere to arrive; else "went to its tab" and "did
            // nothing" report the same.
            if (pageRepeater.count !== op.beforeCount
                    || tabsModel.currentIndex < 0
                    || (op.wantIndex >= 0
                        && tabsModel.currentIndex !== op.wantIndex)
                    // The strip is read back below (`middle-close` above).
                    || tabProbe.tabItemCount() !== pageRepeater.count)
                return
            // And the front tab showing its repository (`middle-close` above, same baseline).
            const front = window.curPage
            if (!Awaited.all("open_again_landed", {
                "open": front !== null && front.pageTab.state === "open",
                "page": front !== null && PageSettled.settled(front) }))
                return
            op.complete(() => "open_again tabs=" + pageRepeater.count
                               + " active=" + tabsModel.currentIndex
                               + " asked=" + op.asked
                               + " open=" + tabProbe.tabPaths())
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
                op.input("away", () => {
                    page.foldByHand(true)
                    page.commandsOpen = true
                    page.pageWip.setMessage(op.typed, "")
                    op.firstTab = tabsModel.currentTabId
                    tabsModel.setCurrentIndex(tabsModel.currentIndex === 0 ? 1 : 0)
                })
                return
            }
            if (op.at("away")) {
                if (!Awaited.all("tab_carry_moved", { "moved": tabsModel.currentTabId !== op.firstTab }))
                    return
                op.input("back", () => {
                    op.folded = page.sidebarCollapsed
                    op.log = page.commandsOpen
                    op.otherEmpty = page.pageWip.subjectText === "" && page.pageWip.bodyText === ""
                    tabsModel.setCurrentIndex(tabsModel.currentIndex === 0 ? 1 : 0)
                })
                return
            }
            if (!Awaited.all("tab_carry_back", { "back": tabsModel.currentTabId === op.firstTab }))
                return
            // `sessions=` is the release: two tabs in the strip, one repository in memory.
            op.complete(() => {
                const page = window.curPage
                return "tab_carry tabs=" + pageRepeater.count
                       + " sessions=" + Harness.openSessionCount()
                       + " folded=" + op.folded
                       + " log=" + op.log
                       + " empty=" + op.otherEmpty
                       + " back=" + (page.pageWip.subjectText === op.typed)
                       // …and the pane holding them is the one on screen: words put back behind the commit
                       // details are only half of them being kept.
                       + " wip=" + page.wipShown
            })
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
            if (!Awaited.all("tab_width_act_ready", {
                "ready": !(topBar.bandTabCount <= 0 || topBar.bandTabRun <= 0
                    || tabProbe.tabItemCount() !== topBar.bandTabCount),
                "page": PageSettled.settledUnder(window) }))
                return
            tabWidthActTimer.complete(() => "tab_widths crushed=" + tabProbe.tabNamesCrushed()
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
        }
    }

    // PGG_AUTO_ACT=tab-mark <index>: the tab the hand is on — one of the others, or the picture says nothing new.
    WindowStage {
        id: tabMarkActTimer
        name: "tab_mark_act"
        order: ["pointed"]
        running: Harness.autoAct === "tab-mark"
        property string beforeMarks: ""
        onTriggered: {
            const op = tabMarkActTimer
            const pointed = Number(Harness.autoActArg || 1)
            if (topBar.bandTabCount < pointed)
                return
            if (op.at("start")) {
                op.input("pointed", () => {
                    op.beforeMarks = tabProbe.tabMarks()
                    tabProbe.pointAtTab(pointed)
                })
                return
            }
            // The marks, and the pointed tab's path, which comes up a rest after the hand (`TabItemDelegate`): ended
            // before it, the picture holds the box or not by the machine's pace. The box is the shared one, so it is
            // read as that tab's by its target.
            const tip = Tips.box
            if (!Awaited.all("tab_marks", {
                    "marks": tabProbe.tabMarks() !== op.beforeMarks,
                    "tip": tip.visible && tip.parent === tabProbe.view.itemAtIndex(pointed),
                    "page": PageSettled.windowSettled(window)
                }))
                return
            op.complete(() => "tab_marks tabs=" + topBar.bandTabCount
                               + " current=" + tabsModel.currentIndex
                               + " pointed=" + pointed
                               + " marks=" + tabProbe.tabMarks())
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
            if (!Tips.box.visible)
                return
            // `unique=`: no two tabs read alike. `native=`: a backslash in a name or its hover is Windows' own
            // spelling leaking through (規約 §パスの区切り).
            op.complete(() => {
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
            })
        }
    }

    // PGG_AUTO_ACT=solo: the harness holds the real lock on the config directory before starting this process, so the
    // picture is of the mechanism.
    WindowStage {
        id: soloActTimer
        name: "solo_act"
        running: Harness.autoAct === "solo"
        onTriggered: {
            if (!Awaited.all("solo_act_ready", { "ready": !(!AppBackend.alreadyRunning || !gate.visible),
                                                 "page": PageSettled.windowSettled(window) }))
                return
            soloActTimer.complete(() => "solo blocked=" + AppBackend.alreadyRunning
                                        + " held=" + (AppBackend.heldElsewhere !== "")
                                        + " gate=" + gate.visible
                                        + " main=" + mainUi.visible)
        }
    }
}
