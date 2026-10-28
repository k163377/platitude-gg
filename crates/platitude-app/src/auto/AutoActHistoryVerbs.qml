pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// Rewriting what is already committed: squash, reword, drop, cherry-pick, revert, merge, rebase and reset —
/// and the menus that offer them. Built by `AutoActDriver`.
// An `Item` only because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    /// `var` because naming its type would be a circle: the driver is the file that builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var repoTab: driver.repoTab
    readonly property var worktree: driver.worktree
    readonly property var graphModel: driver.graphModel
    readonly property var branchesModel: driver.branchesModel
    readonly property var tagsModel: driver.tagsModel
    readonly property var stashesModel: driver.stashesModel
    readonly property var refRebaseItem: driver.refRebaseItem
    readonly property var refPullItem: driver.refPullItem
    readonly property var refMenu: driver.refMenu
    readonly property var refRowMenu: driver.refRowMenu
    readonly property var graphPane: driver.graphPane
    readonly property var detailsPane: driver.detailsPane
    readonly property var wipPane: driver.wipPane
    readonly property var commitMenuState: driver.commitMenuState
    readonly property var commitMenu: driver.commitMenu
    readonly property var stashMenu: driver.stashMenu
    readonly property var dropCommitItem: driver.dropCommitItem
    readonly property var resetMenu: driver.resetMenu
    readonly property var commitBranchCard: driver.commitBranchCard
    readonly property var commitTagCard: driver.commitTagCard
    readonly property var hardResetItem: driver.hardResetItem
    readonly property var writeBarrier: driver.barrierWrite

    /// Runs `act` if it is one of this family's, and says whether it was (`AutoActDriver` asks each family in turn).
    function run(act, arg) {
        if (act === "squash" || act === "fold-first-commit") {
            // The fold, and the one refusal a window is still asked for (`report::fold_first_commit`); the other
            // refusal shapes take the same road and are held by `repo_tab::drain_report_tests` and
            // `tst_reportdress.qml`.
            // Opened first: the fold is a replay that stashes the work while it runs, and whether a graph pass
            // catches it half-way (reader lands on a commit) or not (reader stays on the worktree row) is the
            // scheduler's. Started from the commit, both end on a commit's page (`followVanishedCommit`).
            const foldOid = driver.autoActOid(arg)
            acts.readCommit(foldOid)
            page.openRowMenu(foldOid)
            page.squashCommit(foldOid)
            if (act !== "squash")
                driver.barrierNotice.start()
        } else if (act === "reword" || act === "edit-message"
                   || act === "edit-message-leave" || act === "edit-message-refused"
                   || act === "edit-message-focus" || act === "edit-message-away") {
            // HEAD's own: the only row that takes typing (`offers::message_edit`), branch or not.
            page.jumpToRef(worktree.headOid)
            rewordTimer.order = act === "edit-message-focus" ? ["focused"]
                              : act === "edit-message-away" ? ["focused", "away"]
                              : act === "reword" ? ["typed", "saved"]
                              : act === "edit-message-leave" ? ["typed", "left"]
                              : ["typed"]
            rewordTimer.start()
        } else if (act === "cherry-pick" || act === "cherry-pick-stops") {
            const pickOid = driver.autoActOid(arg)
            const pickRow = graphModel.rowOf(pickOid)
            if (pickRow >= 0) {
                graphPane.jumpToRow(pickRow)
                page.activateRow(pickOid)
            }
            // Through → answers at the tip; stopped → answers in the worktree
            // (規約 §履歴を合流させる / §進行中の操作から出る). Revert and merge land the same way.
            if (act === "cherry-pick-stops")
                opStoppedTimer.start()
            else
                tipLandedTimer.start()
            repoTab.cherryPick(pickOid)
        } else if (act === "reset-soft" || act === "reset-mixed") {
            const backTo = acts.resetTarget(arg)
            acts.readCommit(backTo)
            page.openRowMenu(backTo)
            const mode = act === "reset-soft" ? "soft" : "mixed"
            resetLandedTimer.begin(backTo, mode)
            page.moveBranchHere(mode)
        } else if (act === "reset-hard" || act === "reset-hard-confirm") {
            // "-confirm" stops with the held row on screen; "reset-hard" runs the hold to its end.
            const wipeTo = acts.resetTarget(arg)
            acts.readCommit(wipeTo)
            page.openRowMenu(wipeTo)
            resetMenu.offer()
            // `tagged=`: the row's tag is there exactly when there is something to lose, so it holds on a clean
            // fixture and a dirty one alike. `note=` carries spaces, so it goes last.
            const resetRow = () => "reset_row tagged="
                                   + ((hardResetItem.note !== "") === (worktree.hardResetTakes > 0))
                                   + " files=" + worktree.hardResetTakes
                                   + " untracked=" + worktree.untrackedCount
                                   + " note=" + hardResetItem.note
            if (act === "reset-hard") {
                // Read before the hold runs out: the reset takes the files the row counts.
                driver.act.say(resetRow)
                resetLandedTimer.begin(wipeTo, "hard")
                hardResetItem.completeHold()
            } else {
                driver.reportAtCompletion(resetRow)
            }
        } else if (act === "menu-wait") {
            menuWaitTimer.mode = arg
            // The stash forms put their one input in and wait for the menu to stand again. The others can meet the
            // write landed first, and end on saying so from whichever stage it caught.
            menuWaitTimer.order = arg === "stash" || arg === "stash-ref" ? ["popped"] : []
            menuWaitTimer.start()
        } else if (act === "commit-menu" || act === "reset-menu"
                   || act === "branch-card" || act === "tag-card") {
            // With no row named, the row under HEAD's: most of this menu is about a commit the branch is not on, and
            // the rows above HEAD belong to whatever else the graph is showing. The BRANCH card's own case is HEAD's
            // row aimed at the branch the tree is on: the card holds a local branch's upstream rows, and a
            // remote-tracking chip — what the row under HEAD often draws — has no card (`RefBranchMenu.applies`).
            let menuOid = arg
            if (menuOid === "" && act === "branch-card") {
                page.openRowMenu(worktree.headOid, { "kind": "branch", "name": worktree.branch })
            } else {
                if (menuOid === "")
                    menuOid = graphModel.oidAt(graphModel.rowOf(worktree.headOid) + 1)
                page.openRowMenu(menuOid)
            }
            if (act === "reset-menu")
                resetMenu.offer()
            else if (act === "branch-card")
                commitMenu.openSub(commitBranchCard)
            else if (act === "tag-card")
                commitMenu.openSub(commitTagCard)
            // `greyed=` the first level's rows out for a reason of their own — a stopped operation's, all of them.
            driver.reportAtCompletion(() => {
                let greyed = 0
                for (let i = 0; i < commitMenu.count; i++) {
                    const row = commitMenu.itemAt(i)
                    if (row && row.codeColSeat !== undefined && row.offered && !row.subMenu
                            && row.blockedReason !== "")
                        greyed++
                }
                return "commit_menu rows=" + commitMenu.offeredRows
                       + " can_move=" + commitMenuState.menuCanMoveBranch
                       + " standing=" + commitMenuState.menuOpStanding
                       + " greyed=" + greyed
            })
        } else if (act === "drop-commit" || act === "drop-commit-go" || act === "drop-stops"
                   || act === "drop-last-commit") {
            // The bare menu ends on its row; the others press it and hand on.
            dropTimer.order = act === "drop-commit" ? ["opened"] : ["opened", "pressed"]
            dropTimer.begin(act, driver.autoActOid(arg))
        } else if (act === "wip-landing-stopped") {
            // The stop's worktree landing, read in a pass that carries every other worktree's row and none of
            // this window's. The run is started into that arrangement (`xtask::verify::child`, `harness::faults`)
            // because which of the walk and the status arrives first is otherwise the scheduler's. The press needs a
            // clean tree: a replay is refused over uncommitted work.
            const clash = arg === "" ? "side/clash" : arg
            page.openRefMenu("branch", clash, clash, branchesModel.oidOfName(clash))
            stoppedLandingTimer.start()
            repoTab.rebase(clash, "", true)
        } else if (act === "integrate-menu") {
            acts.openIntegrateMenu(arg)
        } else if (act === "pull-go" || act === "pull-ahead") {
            // Through the row's own `triggered`, the handler a click runs (verify-ui §壊れない動詞の実装と反復).
            // `pull-go` brings the far side in (`--preset behind`); `pull-ahead` gets nothing (`--preset basic`). A
            // divergence is not this row's: the row is out before the press (`pull-blocked`).
            // The press waits for an idle tick: a pull queued behind the opening fetch would be judged on its answer.
            pullPressTimer.verb = act
            pullPressTimer.branch = ""
            pullPressTimer.start()
        } else if (act === "pull-branch-go") {
            // The same row on a branch the worktree is not on (`--preset others-behind`'s `release`): the press
            // fast-forwards that branch and nothing this worktree stands on.
            pullPressTimer.verb = act
            pullPressTimer.branch = arg
            pullPressTimer.start()
        } else if (act === "merge-branch" || act === "merge-stops" || act === "rebase-onto"
                   || act === "rebase-stops" || act === "replay-running" || act === "revert-commit"
                   || act === "revert-stops") {
            if (act === "revert-commit" || act === "revert-stops") {
                // A right-click selects the row too (GraphRowDelegate), so both steps are taken.
                const oidHex = driver.autoActOid(arg)
                graphPane.jumpToRow(graphModel.rowOf(oidHex))
                page.activateRow(oidHex)
                page.openRowMenu(oidHex)
                if (act === "revert-stops")
                    opStoppedTimer.start()
                else
                    tipLandedTimer.start()
                repoTab.revert(oidHex)
            } else {
                page.openRefMenu("branch", arg, arg,
                                 branchesModel.oidOfName(arg))
                if (act === "merge-branch" || act === "merge-stops") {
                    if (act === "merge-stops")
                        mergeStoppedTimer.start()
                    else
                        tipLandedTimer.start()
                    repoTab.merge(arg, false, false, "")
                } else if (act === "rebase-onto" || act === "rebase-stops" || act === "replay-running") {
                    // `rebase-stops` answers in the worktree (規約 §未コミット変更がある状態で履歴を書き換える
                    // の着地表); `replay-running` answers nowhere — its subject is the screen while git is out.
                    if (act === "rebase-stops") {
                        opStoppedTimer.start()
                    } else if (act === "replay-running") {
                        replayRunningTimer.start()
                        // What a click does next; the other two land long after the menu is gone, this one does not.
                        refMenu.close()
                    }
                    repoTab.rebase(arg, "", true)
                }
            }
        } else {
            return false
        }
        return true
    }
    /// The ref menu left standing on what a merge or a rebase would bring in — a branch by bare name, a tag with
    /// `:tag` after it. `pushed=` is the `rebase` row's note, which must be on the row as the card is measured; a
    /// photograph cannot tell it from one a frame late (規約 §メニュー). Both values are runs of their own: either
    /// alone passes an implementation that always answers the same. `greyed=` counts the rows that would start
    /// something of their own — `Create branch here…`, merge, rebase, `Create tag here…` — standing greyed: all four
    /// under a stopped operation, none otherwise; `says=` is the rebase row's reason. Read as the card is measured, in
    /// the turn it opens, and handed to the completion as read.
    function openIntegrateMenu(arg) {
        const onTag = arg.endsWith(":tag")
        const name = onTag ? arg.substring(0, arg.length - 4) : arg
        const oid = onTag ? tagsModel.oidOfName(name) : branchesModel.oidOfName(name)
        page.openRefMenu(onTag ? "tag" : "branch", name, name, oid)
        const starts = [refRowMenu.branchHereItem, refRowMenu.mergeItem, refRebaseItem, refRowMenu.tagHereItem]
        const measured = "integrate_menu ref=" + name
                         + " offered=" + refRebaseItem.offered
                         + " pushed=" + (refRebaseItem.note !== "")
                         + " greyed=" + starts.filter(item => item.offered && item.blocked).length
                         + " says=" + refRebaseItem.blockedReason
        driver.reportAtCompletion(() => measured)
    }

    /// Which commit the reset verbs take the branch back to. With nothing given, the row under HEAD's: a reset to
    /// where the branch already stands moves nothing, so its landing would never come.
    function resetTarget(arg) {
        return arg === "" ? graphModel.oidAt(graphModel.rowOf(worktree.headOid) + 1)
                          : driver.autoActOid(arg)
    }

    /// Opens the commit these verbs are about (rules-refs/app-ui.md「コミットの詳細を読む動詞は自分でその行を開く」).
    function readCommit(oidHex) {
        const row = graphModel.rowOf(oidHex)
        if (row >= 0)
            page.activateRow(oidHex, row)
    }

    /// What a picture of a replay under way waits on: the worktree's row standing for it — the status saying so
    /// (`status`), and the graph's rows carrying it (`row`). The reads go on under a replay
    /// (`RepoSession::begin_poll`) and the first one stands the row up for the operation at whatever moment the pace
    /// gives it, so a picture of the rewrite, and the census walked after it, wait for that read rather than race it.
    /// Asked for each beat until the status has the row (`RepoTab.pollNow`). With it the selected row's own read
    /// (`details`) and the rows the lists show (`PageSettled.rowsBuilt`): a replay keeps the page's reads coming, so
    /// the picture is held to these, not to the page settling.
    function replayRow() {
        if (!worktree.wipRowStands)
            repoTab.pollNow()
        return { "status": worktree.wipRowStands, "row": graphModel.wipRow,
                 "details": page.pageDetailsSettled, "rows": PageSettled.rowsBuilt(page) }
    }

    // The drop row is a count of presses or a click by whether anything else holds the branch tip, read once as the
    // menu opens (`CommitMenuState.standRow`). Before that answer is in, its `false` is no answer, and which of the two
    // controls a run pressed would be the scheduler's — so the menu goes up once it is in (`WorktreeModel`'s
    // `headReachKnown`) and nothing is out that would raise the menu greyed. The presses come past the dispatch, so the
    // write barrier starts behind them: armed earlier, a write asked meanwhile would open it.
    PageStage {
        id: dropTimer
        name: "drop_row"
        property string act: ""
        property string oid: ""
        function begin(act, oid) {
            dropTimer.act = act
            dropTimer.oid = oid
            dropTimer.start()
        }
        function line() {
            return "drop_row " + dropCommitItem.code
                   + " " + dropCommitItem.text
                   + " oid=" + commitMenuState.menuOid.substring(0, 8)
                   + " hold=" + (dropCommitItem.holdPresses > 0)
                   + " reached=" + worktree.headReachedElsewhere
        }
        /// The row's press, held to its end or clicked. Answers whether it went in.
        function press() {
            if (dropCommitItem.holdPresses > 0)
                return driver.holdToEnd(dropCommitItem)
            // The item's own click takes the menu down before the write (`CommitRowMenu` `dropCommitItem.onHeld`);
            // fired past it, whether the menu goes before the census walk is the scheduler's. So it goes here, and
            // completion waits until it is gone (`AutoActDriver.menuGoing`).
            commitMenu.dismiss()
            driver.menuGoing = commitMenu
            return driver.pressWrite("drop", () => {
                page.dropCommit(commitMenuState.menuOid)
                return true
            })
        }
        onTriggered: {
            const op = dropTimer
            if (op.at("start")) {
                if (!Awaited.all("drop_row", { "reach": worktree.headReachKnown, "live": driver.menusLive() }))
                    return
                // Opened first where a replay follows, for the fold's reason. The bare menu stays where the page
                // opened: nothing after it moves the reader, and a read started here would race the picture.
                const open = () => {
                    if (op.act !== "drop-commit")
                        acts.readCommit(op.oid)
                    page.openRowMenu(op.oid)
                }
                if (op.act === "drop-commit") {
                    op.input("opened", open)
                    op.complete(() => op.line())
                    return
                }
                // Read as the menu opens, before the press takes it down.
                op.input("opened", open, () => op.line())
            }
            if (!op.input("pressed", () => op.press()))
                return
            op.stop()
            // "drop-stops" stops with the work still in the stash it took (`write_stopped stashes=`).
            if (op.act === "drop-stops")
                opStoppedTimer.start()
            else if (op.act === "drop-commit-go")
                writeBarrier.start()
            // Nowhere to land: the refusal's bar is the shot.
            else
                driver.barrierNotice.start()
        }
    }

    // `menu-wait` / `menu-wait after`, over `--preset slowhook`'s sleeping pre-commit hook: the graph row's menu raised
    // while that commit is out keeps the rows the same row offers with nothing running, every one greyed for it, and
    // gives them back once what the commit left is read — the same menu, still up, stood again on that reading
    // (デザイン規約 §メニュー). `same=` holds the rows to a count taken off the menu raised and put down on that row
    // before the commit; `held=` is how many of them grey, which a picture of grey rows next to a picture of missing
    // ones cannot say.
    // `menu-wait stash`, over `--preset stashes`: the second entry's graph row menu, raised in the turn the first is
    // popped — the pop asked for and not started — waits for it all the same, and stands again on the selector the
    // entry answers to once the pop is read (`CommitMenuState.standAgain`): `was=` is the one it went up on,
    // `selector=` the one a press now acts on. `menu-wait stash-ref` is the same over the left menu's STASHES row
    // (`RefRowMenu.standAgain`).
    // `menu-wait replay`, over `--preset replay`: the plain form's line with a rebase out instead of the commit, the
    // menu raised once the rewrite shows mid-way, and photographed once its worktree row stands too (`replayRow`).
    PageStage {
        id: menuWaitTimer
        name: "menu_wait"
        /// `""`, `after`, `stash`, `stash-ref` or `replay`, as above.
        property string mode: ""
        property string oid: ""
        property int idleRows: -1
        property string raisedOn: ""
        /// The level the menu opens as: a stash's graph row has its own, and the left menu is another.
        readonly property var level: menuWaitTimer.mode === "stash" ? acts.stashMenu
                                   : menuWaitTimer.mode === "stash-ref" ? refMenu : commitMenu
        /// The selector the stash's menu acts on.
        readonly property string selector: menuWaitTimer.mode === "stash-ref" ? refRowMenu.refId
                                                                              : commitMenuState.menuStashRef
        /// The row's own right-click (`GraphRowDelegate.askMenu`), or the left menu's door. False while the row has no
        /// delegate to answer.
        function raise() {
            if (menuWaitTimer.mode === "stash-ref")
                return page.openRefMenu("stash", stashesModel.nameAt(1), stashesModel.fullAt(1), menuWaitTimer.oid,
                                        true)
            const item = graphPane.view.itemAtIndex(graphModel.rowOf(menuWaitTimer.oid))
            if (item === null)
                return false
            item.askMenu()
            return menuWaitTimer.level.visible
        }
        /// The rows that start git — every row on show but the cards' titles, which stay live while the rows inside
        /// say why — and how many of them grey. A row greyed for a reason of its own (`blockedReason`: the delete
        /// table's row on the branch the tree is on) greys whether or not anything runs, so it is not one the wait
        /// holds.
        function held() {
            const level = menuWaitTimer.level
            let rows = 0
            let greyed = 0
            for (let i = 0; i < level.count; i++) {
                const row = level.itemAt(i)
                if (!row || !row.present || row.codeColSeat === undefined || row.subMenu)
                    continue
                if (row.blockedReason !== "" && !row.withdrawn)
                    continue
                rows++
                if (row.blocked)
                    greyed++
            }
            return greyed === rows ? "all" : greyed === 0 ? "none" : greyed + "/" + rows
        }
        function line(stage) {
            const level = menuWaitTimer.level
            return "menu_wait stage=" + stage
                   + " same=" + (level.offeredRows === menuWaitTimer.idleRows)
                   + " held=" + menuWaitTimer.held()
                   + " open=" + level.visible
                   + (menuWaitTimer.raisedOn !== ""
                      ? " was=" + menuWaitTimer.raisedOn + " selector=" + menuWaitTimer.selector : "")
                   + " rows=" + level.offeredRows + " busy=" + repoTab.busyCount
        }
        /// A write that landed before `what` left no moment to photograph, and says so rather than wait out the
        /// watchdog.
        function missed(what) {
            return "menu_wait_missed the write landed before " + what
        }
        /// The run's completion on `report`. Photographed during the write it presses: the page's reads are on their
        /// way by design.
        function completeOn(report) {
            menuWaitTimer.complete(report, ["*"])
        }
        /// The second stash's menu, counted and put down; then the first popped and the menu raised again in the same
        /// turn. False while the listing is not in.
        function popUnder() {
            if (stashesModel.fullAt(1) === "")
                return false
            menuWaitTimer.oid = stashesModel.oidOfName(stashesModel.nameAt(1))
            if (!menuWaitTimer.raise())
                return false
            menuWaitTimer.idleRows = menuWaitTimer.level.offeredRows
            menuWaitTimer.level.dismiss()
            const popped = stashesModel.fullAt(0)
            driver.pressWrite("stash-pop", () => {
                page.popStash(popped)
                return true
            })
            menuWaitTimer.raise()
            menuWaitTimer.raisedOn = menuWaitTimer.selector
            return true
        }
        /// The row's menu counted with nothing running and put down, then the write pressed under it: `replay`'s
        /// rebase, or the commit. False while the row has no delegate to answer.
        function pressUnder() {
            // The row under HEAD's where there is one, as `commit-menu` takes; HEAD's own over a single commit.
            const below = graphModel.oidAt(graphModel.rowOf(worktree.headOid) + 1)
            menuWaitTimer.oid = below !== "" ? below : worktree.headOid
            if (!menuWaitTimer.raise())
                return false
            menuWaitTimer.idleRows = commitMenu.offeredRows
            commitMenu.dismiss()
            if (menuWaitTimer.mode === "replay") {
                return driver.pressWrite("rebase", () => {
                    // `--preset replay`'s other line: three hundred steps, replayed for seconds.
                    repoTab.rebase("base", "", true)
                    return true
                })
            }
            page.showWip()
            wipPane.setMessage("chore: held by a sleeping hook", "")
            return driver.pressWrite("commit", () => {
                page.commitNow()
                return true
            })
        }
        onTriggered: {
            const op = menuWaitTimer
            if (op.at("start")) {
                if (!Awaited.all("menu_wait_idle", { "open": repoTab.state === "open",
                                                     "idle": repoTab.busyCount === 0,
                                                     "page": PageSettled.settled(page) }))
                    return
                if (op.mode === "stash" || op.mode === "stash-ref")
                    op.input("popped", () => op.popUnder())
                else
                    op.input("pressed", () => op.pressUnder())
                return
            }
            // Read first at every stage under the write: a write that has landed ends the run on saying so.
            const landed = driver.wroteAndSettled()
            if (op.at("pressed")) {
                // The hook sleeps for seconds, so the commit is still out when the menu goes up. One that has landed
                // already left no moment to raise it in. A replay's menu goes up once the rewrite shows mid-way — HEAD
                // detached, `REBASING` counted — which is what a menu standing on the worktree as it is would lose its
                // rows to (`RepoPage.treeAtAsk`).
                if (!Awaited.all("menu_wait_out", landed ? { "landed": landed } : {
                    "out": repoTab.busyCount !== 0,
                    "down": !commitMenu.visible,
                    "rewriting": op.mode !== "replay" || worktree.opText !== "" }))
                    return
                if (landed) {
                    op.completeOn(() => op.missed("the menu could go up under it"))
                    return
                }
                op.input("raised", () => op.raise())
                return
            }
            if (op.mode === "") {
                // The menu stands over the write still out; the reads the press invalidated (the selected commit's
                // own, the lists' rows) land under the hook's sleep, and the picture is of them landed.
                if (!Awaited.all("menu_wait_during", landed ? { "landed": landed } : {
                    "menu": commitMenu.visible,
                    "details": page.pageDetailsSettled,
                    "rows": PageSettled.rowsBuilt(page) }))
                    return
                op.completeOn(landed ? () => op.missed("its reads could settle under the menu")
                                     : () => op.line("during"))
                return
            }
            if (op.mode === "replay") {
                // Raised before the row, not after: the rebuild that brings the row puts the commits replayed so far
                // above the one the menu was counted on, and enough of them scroll it out of the view.
                if (!Awaited.all("menu_wait_row", landed ? { "landed": landed } : acts.replayRow()))
                    return
                op.completeOn(landed ? () => op.missed("its worktree row stood") : () => op.line("during"))
                return
            }
            // `after` and the stash forms: the menu's own word that it stood again. The write's settle reaches the tab
            // a drain before the models it was read into are through (`RepoTab::settling`).
            if (!Awaited.all("menu_wait_up", { "written": landed, "stood": !op.level.waiting }))
                return
            op.completeOn(() => op.line("after"))
        }
    }

    // `replay-running`: the screen while git is out — the step badge, the doors held down, the ring beside the hand,
    // and the worktree's row the replay stands up (`replayRow`, the one read this asks for). The count ticks on the
    // page's own clock (`Metrics.opProgressMs`), so a build whose tick never reaches the badge waits out the watchdog.
    // Then the face is held (`RepoPage.autoReplayHeld`, the latch `doors-held` takes): the write answers before the
    // reads it invalidated, so a picture taken at the edge could show a screen the replay had already left. The line is
    // read at that edge, the face held: the step badge and the operation are git's, which the replay moves on from.
    // No `order`: a replay seen counting and gone before its row stood ends the run on saying so, rather than wait out
    // the watchdog.
    PageStage {
        id: replayRunningTimer
        name: "replay_running"
        onTriggered: {
            const op = replayRunningTimer
            if (op.at("held")) {
                // The face held; the picture waits for the page under it.
                if (!Awaited.all("replay_running_settled", { "page": PageSettled.settled(page) }))
                    return
                op.complete(() => [])
                return
            }
            // Queued is not out: git writes nothing to count until it is actually replaying.
            const counting = repoTab.replaying && worktree.opSteps !== 0 && worktree.opStep !== 0
            if (op.at("start")) {
                if (!Awaited.all("replay_running_counting", { "counting": counting }))
                    return
                op.advance("counting")
            }
            const ended = !repoTab.replaying
            if (!Awaited.all("replay_running_row", ended ? { "ended": ended }
                                                         : Object.assign({ "counting": counting }, acts.replayRow())))
                return
            if (ended) {
                op.complete(() => "replay_running_missed the replay ended before its worktree row stood")
                return
            }
            op.input("held", () => {
                page.autoReplayHeld = true
                // Hover cannot be injected (offscreen's hand sits at the window's corner), so the run places the hand
                // the ring reads (`Main.holdWaitHand`): over the rewritten pane, clear of row text a ring would hide.
                const seat = graphPane.mapToItem(null, graphPane.width / 6, graphPane.height / 3)
                page.Window.window.holdWaitHand(seat.x, seat.y)
            }, () => {
                // That a range is being counted at all, not which step: the step is whatever git had reached.
                const counted = worktree.opStep > 0 && worktree.opStep <= worktree.opSteps
                                && worktree.opSteps > 1
                return "replay_running op=" + worktree.opText + " counted=" + counted
                       + " ring=" + page.Window.window.waitRingShown
                       + " held=" + page.replayRunning
                       + " step=" + worktree.opStep + " steps=" + worktree.opSteps
            })
        }
    }
    // Both moments of the stop's landing in one line, since neither means anything alone: `owed=` / `early=` are the
    // page after a pass that beat the status was offered the landing and turned it down; `wip=` is the pass carrying
    // ours landing on this window's tree. Every worktree's row wears the same all-zero id, so taking row 0's id for
    // ours lands the press on another worktree's. No `order`: a run not started with the hold up (both harness calls
    // answer whether it was) ends on whichever call said so.
    PageStage {
        id: stoppedLandingTimer
        name: "wip_stop_landing"
        /// `finishCount` when the pass was asked for: the landing is only offered passes that finish after it was
        /// armed (`RepoPage.onStatsChanged` → `tryPendingWipSelect`), so an earlier reading answers nothing.
        property int owedAt: -1
        property bool owed: false
        property bool early: false
        property bool earlyWorktree: false
        /// Whether the hold was up, as the harness call just made answered.
        property bool held: false
        onTriggered: {
            const op = stoppedLandingTimer
            if (op.at("start")) {
                // First the stop: answered, an operation standing with its status in, the landing armed.
                if (!Awaited.all("wip_stop_landing_stopped", { "idle": repoTab.busyCount === 0,
                                                               "op": worktree.opText !== "",
                                                               "row": worktree.wipRowStands,
                                                               "walked": !graphModel.loading,
                                                               "landing": page.pageLanding }))
                    return
                // Ask for the pass: under the hold nothing else brings one (our row appearing normally does, and a
                // stopped replay moves no branch). It lands with every other worktree's row and none of ours — the
                // pass the landing has to turn down.
                op.input("asked", () => {
                    op.held = graphModel.walkAgainWhileHeld()
                    op.owedAt = graphModel.finishCount
                })
                if (!op.held)
                    op.complete(() => "wip_stop_landing held=false")
                return
            }
            if (op.at("asked")) {
                // Then that pass, led by a neighbour worktree's row (`GraphModel.carriedTop`); a pass without one on
                // top cannot catch the misreading, so it is read past.
                if (!Awaited.all("wip_stop_landing_pass", { "walked": !graphModel.loading,
                                                            "after": graphModel.finishCount > op.owedAt,
                                                            "carried": !graphModel.wipRow && graphModel.carriedTop }))
                    return
                op.input("let_through", () => {
                    op.owed = page.pageLanding
                    op.early = page.wipShown
                    op.earlyWorktree = page.carriedPath !== ""
                    op.held = graphModel.letTheWorktreeRowThrough()
                })
                if (!op.held)
                    op.complete(() => "wip_stop_landing held=false")
                return
            }
            if (!Awaited.all("wip_stop_landing_landed", { "row": graphModel.wipRow,
                                                          "page": PageSettled.settled(page) }))
                return
            op.complete(() => "wip_stop_landing held=true otherTop=true owed=" + op.owed
                              + " early=" + op.early
                              + " earlyWorktree=" + op.earlyWorktree
                              + " wip=" + page.wipShown
                              + " worktree=" + (page.carriedPath !== "")
                              + " op=" + worktree.opText
                              // This landing moves the highlight without activating a row (`tryPendingWipSelect`).
                              + " lit=" + graphPane.view.currentIndex)
        }
    }

    /// The press behind `pull-go` / `pull-ahead` / `pull-branch-go`. A pull answers at the tip
    /// (`RepoTab::settle_write`), so `pull-go` finishes on `tipLandedTimer`; the other two on what they read back.
    PageStage {
        id: pullPressTimer
        name: "pull_press"
        order: ["opened", "pressed"]
        property string verb: "pull-go"
        /// The row's branch: the worktree's own where empty.
        property string branch: ""
        onTriggered: {
            const op = pullPressTimer
            const branch = op.branch === "" ? worktree.branch : op.branch
            if (op.at("start")) {
                if (!Awaited.all("pull_press_live", { "live": driver.menusLive() }))
                    return
                // Raised each beat until it is up.
                if (!op.input("opened", () => {
                    page.openRefMenu("branch", branch, branch, branchesModel.oidOfName(branch))
                    return refMenu.opened
                }))
                    return
            }
            if (!Awaited.all("pull_press_ready", { "live": driver.menusLive(),
                                                   "opened": refMenu.opened,
                                                   "offered": refPullItem.offered }))
                return
            op.input("pressed", () => {
                // Read back as `moved=` / `head=`: a picture cannot say whether a graph was redrawn or never touched.
                driver.headOidBefore = worktree.headOid
                if (op.verb === "pull-go")
                    tipLandedTimer.start()
                driver.pressWrite(op.verb, () => {
                    refPullItem.triggered()
                    return true
                })
                // A hand's press takes the card down; this one must be told, and completion waits out its exit
                // (`menuGoing` — the census walks what is visible).
                driver.menuGoing = refMenu
                refMenu.close()
            })
            op.stop()
            if (op.verb === "pull-ahead")
                pullAheadTimer.start()
            if (op.verb === "pull-branch-go")
                pullBranchTimer.start()
        }
    }
    /// `pull-branch-go`: the branch level with the commit its upstream stands on, read off one listing — before the
    /// press the two differ, and nothing but the fast-forward makes them meet — with the write answered first, so a
    /// refusal reads as one.
    PageStage {
        id: pullBranchTimer
        name: "pull_branch"
        /// Whether the branch stands level with its upstream. Both slots answer "" for a name the listing lost, which
        /// must not read as level.
        function level() {
            const tip = branchesModel.oidOfName(pullPressTimer.branch)
            return tip !== "" && tip === branchesModel.upstreamOidOf(pullPressTimer.branch)
        }
        onTriggered: {
            if (!Awaited.all("pull_branch_ready", {
                "ready": !(repoTab.busyCount !== 0 || repoTab.writeSeq === driver.writeSeqBefore),
                "level": pullBranchTimer.level() || repoTab.writeRefused }))
                return
            pullBranchTimer.complete(() => "pull_branch refused=" + repoTab.writeRefused
                                            + " level=" + pullBranchTimer.level()
                                            + " behind=" + branchesModel.behindOf(pullPressTimer.branch)
                                            + " head=" + (worktree.headOid !== driver.headOidBefore))
        }
    }
    /// `pull-ahead`: git brings nothing (`Already up to date.`) and nothing on screen changes, so the command panel is
    /// opened — its row is the only sign the press went anywhere.
    PageStage {
        id: pullAheadTimer
        name: "pull_ahead"
        order: ["logged"]
        onTriggered: {
            const op = pullAheadTimer
            const ready = !(repoTab.busyCount !== 0 || repoTab.writeSeq === driver.writeSeqBefore)
            if (op.at("start")) {
                if (!Awaited.all("pull_ahead_ready", { "ready": ready, "page": PageSettled.settled(page) }))
                    return
                op.input("logged", () => {
                    if (!page.commandsShown)
                        page.toggleCommands()
                })
                return
            }
            if (!Awaited.all("pull_ahead_logged", { "ready": ready,
                                                    "page": PageSettled.settled(page),
                                                    "log": page.commandsShown,
                                                    "idle": !page.pageCommands.running }))
                return
            op.complete(() => "pull_ahead refused=" + repoTab.writeRefused
                              + " log=" + page.commandsShown
                              + " moved=" + (worktree.headOid !== driver.headOidBefore))
        }
    }
    PageStage {
        id: tipLandedTimer
        name: "tip_landed"
        /// The op name this run's own write answered by, latched by the `Connections` below: a fetch's answer moves
        /// the counter too. Empty until then, and that is the arm: with the selection already at the tip (a merge from
        /// a ref row, a revert of HEAD) every other reading below holds before git has done anything.
        property string answeredOp: ""
        onTriggered: {
            const row = graphModel.rowOf(page.selectedOid)
            // `pendingHeadSelect`: right after the answer the refs are still old, so `selected === headOid` holds
            // vacuously until it resolves. `cardSettled`: until its round trip returns, the right pane still shows the
            // commit selected before the press — for a merge from a ref row, the old tip. `settling`: the refs
            // publish ahead of the status, the rebuilt graph and the sidebar, and a fast-forward lands on a row the
            // graph already has (the upstream's), so every other gate here opens while the WIP row still leashes the
            // old tip and the counts still say behind.
            if (!Awaited.all("tip_landed_ready", {
                "answered": tipLandedTimer.answeredOp !== "",
                "landed": !page.pendingHeadSelect,
                "idle": repoTab.busyCount === 0 && !repoTab.settling,
                "row": row >= 0 && graphPane.rowOnScreen(row),
                "head": page.selectedOid === worktree.headOid,
                "card": driver.cardSettled }))
                return
            tipLandedTimer.complete(() => {
                const landed = graphModel.rowOf(page.selectedOid)
                return "tip_landed follows="
                       + (page.selectedOid !== "" && page.selectedOid === worktree.headOid)
                       + " onscreen=" + (landed >= 0 && graphPane.rowOnScreen(landed))
                       // Right after the pair above: `must_say` reads the three as one string.
                       + " op=" + tipLandedTimer.answeredOp
                       + " head=" + worktree.headOid.substring(0, 8)
                       + " selected=" + page.selectedOid.substring(0, 8)
                       + " row=" + landed
            })
        }
    }
    /// Latches the answers `tipLandedTimer` and `resetLandedTimer` wait on, on the notify and out of every answer it
    /// carried: one drain notifies once for the whole queue (`RepoTab::write_answers`), and a later answer (a fetch,
    /// most often) rewrites the group (rules-refs/app-ui.md「一瞬だけ立つ状態は signal で観測して latch する」). The
    /// floor is `writeSeqBefore`, not a rise of `busyCount`: a write that begins and ends between two looks shows
    /// none. `writeAnswerAtTip` is the bridge's "landed at the tip, did not stop" (`RepoTab::settle_write`).
    Connections {
        target: driver.repoTab
        function onWriteSeqChanged() {
            const tab = driver.repoTab
            if (resetLandedTimer.running && resetLandedTimer.armed === 0) {
                for (let i = 0; i < tab.writeAnswerCount(); i++) {
                    if (tab.writeAnswerSeq(i) > driver.writeSeqBefore && tab.writeAnswerOp(i) === "reset") {
                        resetLandedTimer.armed = tab.writeAnswerHeadSeq(i)
                        break
                    }
                }
            }
            if (!tipLandedTimer.running || tipLandedTimer.answeredOp !== "")
                return
            for (let i = 0; i < tab.writeAnswerCount(); i++) {
                if (tab.writeAnswerSeq(i) <= driver.writeSeqBefore || !tab.writeAnswerAtTip(i))
                    continue
                tipLandedTimer.answeredOp = tab.writeAnswerOp(i)
                return
            }
        }
    }
    // Where a reset leaves the reader. No commit is written, so the claim is the name arriving on the commit asked
    // for, waited for by identity: the write answers before its refs are published (`session::write::run_write`), so
    // the write barrier would photograph the branch where it stood. `files=` tells the modes apart: `--soft` /
    // `--mixed` put the commits' changes back, `--hard` writes over them.
    PageStage {
        id: resetLandedTimer
        name: "reset_landed"
        /// What this verb asked for, not read back from git — that git did it is what is being checked.
        property string target: ""
        property string mode: ""
        /// The number the reset's answer named for the first report of HEAD after it (`writeAnswerHeadSeq`),
        /// latched on the notify; 0 until the answer lands.
        property int armed: 0
        function begin(oidHex, flag) {
            resetLandedTimer.target = oidHex
            resetLandedTimer.mode = flag
            resetLandedTimer.start()
        }
        onTriggered: {
            // The write's own status, by the number its answer named (`WorktreeModel.statusSeq`): the refs read
            // moves HEAD ahead of the status that follows (`session::write::run_write`), and `--hard` is judged on the
            // tree.
            // A status counted before the write cannot reach that number.
            if (!Awaited.all("reset_landed_ready", {
                "ready": !(repoTab.busyCount !== 0 || resetLandedTimer.armed === 0
                    || worktree.headOid !== resetLandedTimer.target || worktree.statusSeq < resetLandedTimer.armed),
                "page": PageSettled.settled(page),
                "row": graphModel.rowOf(resetLandedTimer.target) >= 0,
                "card": driver.cardSettled }))
                return
            // No `moved=`: the wait above is that claim, and a field read off the same condition could never be false.
            resetLandedTimer.complete(() => "reset_landed mode=" + resetLandedTimer.mode
                                             + " files=" + worktree.hardResetTakes
                                             + " untracked=" + worktree.untrackedCount
                                             + " head=" + worktree.headOid.substring(0, 8)
                                             + " row=" + graphModel.rowOf(resetLandedTimer.target)
                                             + " rows=" + graphModel.rowTotal)
        }
    }
    // Where a merge that stopped on conflicts left the reader: the worktree, its conflicted rows on screen, and
    // nothing calling it a failure.
    PageStage {
        id: mergeStoppedTimer
        name: "merge_stopped"
        onTriggered: {
            if (!Awaited.all("merge_stopped_ready", {
                "idle": repoTab.busyCount === 0,
                "wip": page.wipShown,
                "conflicts": worktree.conflictCount > 0 }))
                return
            mergeStoppedTimer.complete(() => "merge_stopped wip=" + page.wipShown
                + " conflicts=" + (worktree.conflictCount > 0)
                + " error=" + (repoTab.lastError !== "")
                + " log=" + page.commandsOpen
                // The box holds what the merge will record; the button under it stays the only door (`cont=false`).
                + " msg=" + (worktree.opSubject !== "" && wipPane.subjectText === worktree.opSubject)
                + " cont=" + wipPane.offersOpExit("--continue")
                + " op=" + worktree.opText
                + " files=" + worktree.conflictCount)
        }
    }
    // `mergeStoppedTimer`'s twin for the operations that step (cherry-pick, revert, rebase, a stopped drop): the same
    // checks, plus the `--continue` row the merge does not have — for these it is a step onward
    // (規約 §進行中の操作から出る).
    PageStage {
        id: opStoppedTimer
        name: "op_stopped"
        onTriggered: {
            // The whole write barrier: the stash and worktree listings are reads of their own, landing apart from
            // the status that says the stop, and only the write's settle says both are in
            // (`session::write::settle_after`).
            if (!Awaited.all("op_stopped_ready", {
                "written": driver.wroteAndSettled(),
                "wip": page.wipShown,
                "conflicts": worktree.conflictCount > 0 }))
                return
            opStoppedTimer.complete(() => "write_stopped wip=" + page.wipShown
                + " conflicts=" + (worktree.conflictCount > 0)
                + " error=" + (repoTab.lastError !== "")
                + " log=" + page.commandsOpen
                + " cont=" + wipPane.offersOpExit("--continue")
                // A replay stops on no branch, and the main worktree's WORKTREES row is named by its folder
                // (デザイン規約 §左メニューの所作); a pick or a revert stops on the branch.
                + " home=" + driver.navProbe.homeWorktreeName()
                // Where carried work went: git's words are not raised over the stop, so only the count and the
                // stash row say it (規約 §未コミット変更がある状態で履歴を書き換える).
                + " stashes=" + stashesModel.total
                + " op=" + worktree.opText
                + " files=" + worktree.conflictCount)
        }
    }
    // The message has to arrive before it can be typed over; the boxes are read-only until then (`cardSettled`). The
    // inputs of each form go in in one beat.
    PageStage {
        id: rewordTimer
        name: "reword"
        /// Whether the save row stood for the caret before the press away (`edit-message-away`), read there.
        property bool stood: false
        onTriggered: {
            const op = rewordTimer
            if (!Awaited.all("reword_ready", { "ready": driver.cardSettled, "page": PageSettled.settled(page) }))
                return
            // "edit-message-focus" types nothing: the caret is photographed over the commit's own body.
            if (Harness.autoAct === "edit-message-focus") {
                op.input("focused", () => {
                    detailsPane.focusDescription()
                })
                op.complete(() => "message_focus pane=details focused=" + detailsPane.descriptionFocused
                                  + " color=" + detailsPane.descriptionColor)
                return
            }
            // "edit-message-away" puts the caret in the same way, then presses on the pane's band through the window's
            // own watcher: the save row that stood for the caret comes down with it (デザイン規約 §コミットメッセージの
            // 2 つの枠). `stood=` says the caret was in first; `page=` that the page itself holds the keyboard after.
            if (Harness.autoAct === "edit-message-away") {
                op.input("focused", () => {
                    detailsPane.focusDescription()
                })
                op.input("away", () => {
                    op.stood = detailsPane.saveRowShown
                    page.Window.window.caretHand.pressedAt(detailsPane.mapToItem(null, detailsPane.width / 2, 2))
                })
                op.complete(() => "message_away stood=" + op.stood
                                  + " row=" + detailsPane.saveRowShown
                                  + " caret=" + detailsPane.descriptionFocused
                                  + " page=" + (page.Window.window.activeFocusItem === page))
                return
            }
            op.input("typed", () => {
                detailsPane.setMessageText(Harness.autoActArg, "")
            })
            // "edit-message-refused" waits for the save to turn the amend down (`--preset undone`) and reads its tip.
            if (Harness.autoAct === "edit-message-refused") {
                op.stop()
                saveRefusedTimer.start()
                return
            }
            if (Harness.autoAct === "reword") {
                if (!op.input("saved", () => driver.pressWrite("reword", () => {
                    detailsPane.submitMessage()
                    return true
                })))
                    return
                op.stop()
                writeBarrier.start()
                return
            }
            // "edit-message-leave" walks away from the unsaved text: nothing asks, and the shot is the next commit's
            // own message. "edit-message" stops here, with the save row on screen.
            if (Harness.autoAct === "edit-message-leave") {
                op.input("left", () => {
                    page.activateRow(graphModel.oidAt(graphModel.rowOf(page.selectedOid) + 1))
                })
            }
            op.complete(() => "")
        }
    }
    // The save of HEAD's message over a stage that takes back all HEAD changed: git's answer about the amend is what
    // greys it, and the run waits on the button's tip alone — pointed at once the save row stands, the tip comes up
    // only on a button that refuses, with the words the reader gets.
    PageStage {
        id: saveRefusedTimer
        name: "save_refused"
        order: ["pointed"]
        onTriggered: {
            const op = saveRefusedTimer
            if (op.at("start")) {
                if (!Awaited.all("save_refused_ready", { "ready": detailsPane.saveRowShown }))
                    return
                op.input("pointed", () => {
                    detailsPane.savePointedAt = true
                })
            }
            if (!Awaited.all("save_refused_tip", { "ready": detailsPane.saveRowShown,
                                                   "tip": detailsPane.saveTip.shown }))
                return
            op.complete(() => "save_refused enabled=" + detailsPane.saveEnabled
                              + " tip=" + detailsPane.saveTip.shown
                              + " why=" + detailsPane.saveTip.words)
        }
    }
}
