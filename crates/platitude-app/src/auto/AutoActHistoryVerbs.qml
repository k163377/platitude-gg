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
    readonly property var renderedBarrier: driver.barrierRendered
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
                tipLandedTimer.begin()
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
            Harness.report("reset_row tagged="
                              + ((hardResetItem.note !== "") === (worktree.hardResetTakes > 0))
                              + " files=" + worktree.hardResetTakes
                              + " untracked=" + worktree.untrackedCount
                              + " note=" + hardResetItem.note)
            if (act === "reset-hard") {
                resetLandedTimer.begin(wipeTo, "hard")
                hardResetItem.completeHold()
            }
        } else if (act === "menu-wait") {
            menuWaitTimer.begin(arg)
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
            Harness.report("commit_menu rows=" + commitMenu.offeredRows
                              + " can_move=" + commitMenuState.menuCanMoveBranch)
        } else if (act === "drop-commit" || act === "drop-commit-go" || act === "drop-stops"
                   || act === "drop-last-commit") {
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
            pullPressTimer.after = act
            pullPressTimer.branch = ""
            pullPressTimer.start()
        } else if (act === "pull-branch-go") {
            // The same row on a branch the worktree is not on (`--preset others-behind`'s `release`): the press
            // fast-forwards that branch and nothing this worktree stands on.
            pullPressTimer.after = act
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
                    tipLandedTimer.begin()
                repoTab.revert(oidHex)
            } else {
                page.openRefMenu("branch", arg, arg,
                                 branchesModel.oidOfName(arg))
                if (act === "merge-branch" || act === "merge-stops") {
                    if (act === "merge-stops")
                        mergeStoppedTimer.start()
                    else
                        tipLandedTimer.begin()
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
    /// alone passes an implementation that always answers the same.
    function openIntegrateMenu(arg) {
        const onTag = arg.endsWith(":tag")
        const name = onTag ? arg.substring(0, arg.length - 4) : arg
        const oid = onTag ? tagsModel.oidOfName(name) : branchesModel.oidOfName(name)
        page.openRefMenu(onTag ? "tag" : "branch", name, name, oid)
        Harness.report("integrate_menu ref=" + name
                          + " offered=" + refRebaseItem.offered
                          + " pushed=" + (refRebaseItem.note !== ""))
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
    /// Asked for each beat until the status has the row (`RepoTab.pollNow`).
    function replayRow() {
        if (!worktree.wipRowStands)
            repoTab.pollNow()
        return { "status": worktree.wipRowStands, "row": graphModel.wipRow }
    }

    // The drop row is a count of presses or a click by whether anything else holds the branch tip, read once as the
    // menu opens (`CommitMenuState.standRow`). Before that answer is in, its `false` is no answer, and which of the two
    // controls a run pressed would be the scheduler's — so the menu goes up once it is in (`WorktreeModel`'s
    // `headReachKnown`) and nothing is out that would raise the menu greyed. The presses come past the dispatch, so the
    // write barrier starts behind them: armed earlier, a write asked meanwhile would open it.
    SampleTimer {
        id: dropTimer
        property string act: ""
        property string oid: ""
        function begin(act, oid) {
            dropTimer.act = act
            dropTimer.oid = oid
            dropTimer.start()
        }
        onTriggered: {
            if (!Awaited.all("drop_row", { "reach": worktree.headReachKnown, "live": driver.menusLive() }))
                return
            dropTimer.stop()
            const act = dropTimer.act
            // Opened first where a replay follows, for the fold's reason. The bare menu stays where the page opened:
            // nothing after it moves the reader, and a read started here would race the picture.
            if (act !== "drop-commit")
                acts.readCommit(dropTimer.oid)
            page.openRowMenu(dropTimer.oid)
            Harness.report("drop_row " + dropCommitItem.code
                              + " " + dropCommitItem.text
                              + " oid=" + commitMenuState.menuOid.substring(0, 8)
                              + " hold=" + (dropCommitItem.holdPresses > 0)
                              + " reached=" + worktree.headReachedElsewhere)
            if (act === "drop-commit") {
                driver.complete()
                return
            }
            // "drop-stops" stops with the work still in the stash it took (`write_stopped stashes=`).
            if (act === "drop-stops")
                opStoppedTimer.start()
            if (dropCommitItem.holdPresses > 0) {
                driver.holdToEnd(dropCommitItem)
            } else {
                // The item's own click takes the menu down before the write (`CommitRowMenu`
                // `dropCommitItem.onHeld`); fired past it, whether the menu goes before the census walk is the
                // scheduler's. So it goes here, and completion waits until it is gone (`AutoActDriver.menuGoing`).
                commitMenu.dismiss()
                driver.menuGoing = commitMenu
                driver.pressWrite("drop", () => {
                    page.dropCommit(commitMenuState.menuOid)
                    return true
                })
            }
            if (act === "drop-commit-go")
                writeBarrier.start()
            // Nowhere to land: the refusal's bar is the shot.
            if (act === "drop-last-commit")
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
    SampleTimer {
        id: menuWaitTimer
        /// `""`, `after`, `stash`, `stash-ref` or `replay`, as above.
        property string mode: ""
        /// `idle` (the count with nothing running), `out` (the write pressed, the menu not raised yet), `row`
        /// (`replay`: raised, the rewrite's worktree row not in yet), `up` (raised under the write; `after` and the
        /// stash forms wait there for the menu to stand again).
        property string stage: ""
        property string oid: ""
        property int idleRows: -1
        property string raisedOn: ""
        /// The level the menu opens as: a stash's graph row has its own, and the left menu is another.
        readonly property var level: menuWaitTimer.mode === "stash" ? acts.stashMenu
                                   : menuWaitTimer.mode === "stash-ref" ? refMenu : commitMenu
        /// The selector the stash's menu acts on.
        readonly property string selector: menuWaitTimer.mode === "stash-ref" ? refRowMenu.refId
                                                                              : commitMenuState.menuStashRef
        function begin(mode) {
            menuWaitTimer.mode = mode
            menuWaitTimer.stage = "idle"
            menuWaitTimer.start()
        }
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
        function report(stage) {
            const level = menuWaitTimer.level
            Harness.report("menu_wait stage=" + stage
                              + " same=" + (level.offeredRows === menuWaitTimer.idleRows)
                              + " held=" + menuWaitTimer.held()
                              + " open=" + level.visible
                              + (menuWaitTimer.raisedOn !== ""
                                 ? " was=" + menuWaitTimer.raisedOn + " selector=" + menuWaitTimer.selector : "")
                              + " rows=" + level.offeredRows + " busy=" + repoTab.busyCount)
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
        onTriggered: {
            // `row` says its own terms (`replayRow`): said in turn with this line, each would read as a change.
            if (menuWaitTimer.stage !== "row")
                Awaited.at("menu_wait", menuWaitTimer.stage)
            if (menuWaitTimer.stage === "idle") {
                if (repoTab.state !== "open" || repoTab.busyCount !== 0 || !PageSettled.settled(page))
                    return
                if (menuWaitTimer.mode === "stash" || menuWaitTimer.mode === "stash-ref") {
                    if (menuWaitTimer.popUnder())
                        menuWaitTimer.stage = "up"
                    return
                }
                // The row under HEAD's where there is one, as `commit-menu` takes; HEAD's own over a single commit.
                const below = graphModel.oidAt(graphModel.rowOf(worktree.headOid) + 1)
                menuWaitTimer.oid = below !== "" ? below : worktree.headOid
                if (!menuWaitTimer.raise())
                    return
                menuWaitTimer.idleRows = commitMenu.offeredRows
                commitMenu.dismiss()
                if (menuWaitTimer.mode === "replay") {
                    driver.pressWrite("rebase", () => {
                        // `--preset replay`'s other line: three hundred steps, replayed for seconds.
                        repoTab.rebase("base", "", true)
                        return true
                    })
                } else {
                    page.showWip()
                    wipPane.setMessage("chore: held by a sleeping hook", "")
                    driver.pressWrite("commit", () => {
                        page.commitNow()
                        return true
                    })
                }
                menuWaitTimer.stage = "out"
            } else if (menuWaitTimer.stage === "out") {
                // The hook sleeps for seconds, so the commit is still out when the menu goes up. One that has landed
                // already left no moment to raise it in, and says so rather than wait out the watchdog.
                if (driver.wroteAndSettled()) {
                    menuWaitTimer.stop()
                    Harness.report("menu_wait_missed the write landed before the menu could go up under it")
                    driver.complete()
                    return
                }
                if (repoTab.busyCount === 0 || commitMenu.visible)
                    return
                // A replay's menu goes up once the rewrite shows mid-way — HEAD detached, `REBASING` counted — which
                // is what a menu standing on the worktree as it is would lose its rows to (`RepoPage.treeAtAsk`).
                if (menuWaitTimer.mode === "replay" && worktree.opText === "")
                    return
                if (!menuWaitTimer.raise())
                    return
                menuWaitTimer.stage = menuWaitTimer.mode === "replay" ? "row" : "up"
                if (menuWaitTimer.mode === "after" || menuWaitTimer.mode === "replay")
                    return
                menuWaitTimer.stop()
                menuWaitTimer.report("during")
                driver.complete()
            } else if (menuWaitTimer.stage === "row") {
                // Raised before the row, not after: the rebuild that brings the row puts the commits replayed so far
                // above the one the menu was counted on, and enough of them scroll it out of the view.
                if (driver.wroteAndSettled()) {
                    menuWaitTimer.stop()
                    Harness.report("menu_wait_missed the write landed before its worktree row stood")
                    driver.complete()
                    return
                }
                if (!Awaited.all("menu_wait_row", acts.replayRow()))
                    return
                menuWaitTimer.stop()
                menuWaitTimer.report("during")
                driver.complete()
            } else if (menuWaitTimer.stage === "up") {
                // The menu's own word that it stood again: the write's settle reaches the tab a drain before the
                // models it was read into are through (`RepoTab::settling`).
                if (!driver.wroteAndSettled() || menuWaitTimer.level.waiting)
                    return
                menuWaitTimer.stop()
                menuWaitTimer.report("after")
                driver.complete()
            }
        }
    }

    // `replay-running`: the screen while git is out — the step badge, the doors held down, the ring beside the hand,
    // and the worktree's row the replay stands up (`replayRow`, the one read this asks for). The count ticks on the
    // page's own clock (`Metrics.opProgressMs`), so a build whose tick never reaches the badge waits out the watchdog.
    // Then the face is held (`RepoPage.autoReplayHeld`, the latch `doors-held` takes): the write answers before the
    // reads it invalidated, so a picture taken at the edge could show a screen the replay had already left.
    SampleTimer {
        id: replayRunningTimer
        /// The replay has been seen counting: one gone before its row stood says so rather than wait out the watchdog.
        property bool counting: false
        onTriggered: {
            // Queued is not out: git writes nothing to count until it is actually replaying.
            if (!repoTab.replaying || worktree.opSteps === 0 || worktree.opStep === 0) {
                if (replayRunningTimer.counting && !repoTab.replaying) {
                    replayRunningTimer.stop()
                    Harness.report("replay_running_missed the replay ended before its worktree row stood")
                    driver.complete()
                }
                return
            }
            replayRunningTimer.counting = true
            if (!Awaited.all("replay_running_row", acts.replayRow()))
                return
            replayRunningTimer.stop()
            page.autoReplayHeld = true
            const win = page.Window.window
            // Hover cannot be injected (offscreen's hand sits at the window's corner), so the run places the hand the
            // ring reads (`Main.holdWaitHand`): over the rewritten pane, clear of row text a ring would hide.
            const seat = graphPane.mapToItem(null, graphPane.width / 6, graphPane.height / 3)
            win.holdWaitHand(seat.x, seat.y)
            // That a range is being counted at all, not which step: the step is whatever git had reached.
            const counted = worktree.opStep > 0 && worktree.opStep <= worktree.opSteps
                            && worktree.opSteps > 1
            Harness.report("replay_running op=" + worktree.opText + " counted=" + counted
                              + " ring=" + win.waitRingShown
                              + " held=" + page.replayRunning
                              + " step=" + worktree.opStep + " steps=" + worktree.opSteps)
            renderedBarrier.begin()
        }
    }
    // Both moments of the stop's landing in one line, since neither means anything alone: `owed=` / `early=` are the
    // page after a pass that beat the status was offered the landing and turned it down; `wip=` is the pass carrying
    // ours landing on this window's tree. Every worktree's row wears the same all-zero id, so taking row 0's id for
    // ours lands the press on another worktree's.
    SampleTimer {
        id: stoppedLandingTimer
        property bool read: false
        /// `finishCount` when the pass was asked for: the landing is only offered passes that finish after it was
        /// armed (`RepoPage.onStatsChanged` → `tryPendingWipSelect`), so an earlier reading answers nothing.
        property int owedAt: -1
        property bool owed: false
        property bool early: false
        property bool earlyWorktree: false
        /// For a run not started with the hold up (both harness calls answer whether it was): says so and stops.
        function refuse() {
            stoppedLandingTimer.stop()
            Harness.report("wip_stop_landing held=false")
            driver.complete()
        }
        onTriggered: {
            // First the stop: answered, an operation standing with its status in, the landing armed.
            if (stoppedLandingTimer.owedAt < 0) {
                if (repoTab.busyCount !== 0 || worktree.opText === "" || !worktree.wipRowStands
                        || graphModel.loading || !page.pageLanding)
                    return
                // Ask for the pass: under the hold nothing else brings one (our row appearing normally does, and a
                // stopped replay moves no branch). It lands with every other worktree's row and none of ours — the
                // pass the landing has to turn down.
                if (!graphModel.walkAgainWhileHeld()) {
                    stoppedLandingTimer.refuse()
                    return
                }
                stoppedLandingTimer.owedAt = graphModel.finishCount
                return
            }
            // Then that pass, led by a neighbour worktree's row (`GraphModel.carriedTop`); a pass without one on top
            // cannot catch the misreading, so it is read past.
            if (!stoppedLandingTimer.read) {
                if (graphModel.loading || graphModel.finishCount <= stoppedLandingTimer.owedAt
                        || graphModel.wipRow || !graphModel.carriedTop)
                    return
                stoppedLandingTimer.read = true
                stoppedLandingTimer.owed = page.pageLanding
                stoppedLandingTimer.early = page.wipShown
                stoppedLandingTimer.earlyWorktree = page.carriedPath !== ""
                if (!graphModel.letTheWorktreeRowThrough())
                    stoppedLandingTimer.refuse()
                return
            }
            if (!graphModel.wipRow || !PageSettled.settled(page))
                return
            stoppedLandingTimer.stop()
            Harness.report("wip_stop_landing held=true otherTop=true owed=" + stoppedLandingTimer.owed
                              + " early=" + stoppedLandingTimer.early
                              + " earlyWorktree=" + stoppedLandingTimer.earlyWorktree
                              + " wip=" + page.wipShown
                              + " worktree=" + (page.carriedPath !== "")
                              + " op=" + worktree.opText
                              // This landing moves the highlight without activating a row (`tryPendingWipSelect`).
                              + " lit=" + graphPane.view.currentIndex)
            driver.complete()
        }
    }

    /// The press behind `pull-go` / `pull-ahead` / `pull-branch-go`. A pull answers at the tip
    /// (`RepoTab::settle_write`), so `pull-go` finishes on `tipLandedTimer`; the other two on what they read back.
    SampleTimer {
        id: pullPressTimer
        property string after: "pull-go"
        /// The row's branch: the worktree's own where empty.
        property string branch: ""
        onTriggered: {
            if (!driver.menusLive())
                return
            const branch = pullPressTimer.branch === "" ? worktree.branch : pullPressTimer.branch
            if (!refMenu.opened)
                page.openRefMenu("branch", branch, branch, branchesModel.oidOfName(branch))
            if (!refMenu.opened || !refPullItem.offered)
                return
            pullPressTimer.stop()
            // Read back as `moved=` / `head=`: a picture cannot say whether a graph was redrawn or never touched.
            driver.headOidBefore = worktree.headOid
            if (pullPressTimer.after === "pull-go")
                tipLandedTimer.begin()
            driver.pressWrite(pullPressTimer.after, () => {
                refPullItem.triggered()
                return true
            })
            if (pullPressTimer.after === "pull-ahead")
                pullAheadTimer.start()
            if (pullPressTimer.after === "pull-branch-go")
                pullBranchTimer.start()
            // A hand's press takes the card down; this one must be told, and completion waits out its exit
            // (`menuGoing` — the census walks what is visible).
            driver.menuGoing = refMenu
            refMenu.close()
        }
    }
    /// `pull-branch-go`: the branch level with the commit its upstream stands on, read off one listing — before the
    /// press the two differ, and nothing but the fast-forward makes them meet — with the write answered first, so a
    /// refusal reads as one.
    SampleTimer {
        id: pullBranchTimer
        onTriggered: {
            if (repoTab.busyCount !== 0 || repoTab.writeSeq === driver.writeSeqBefore)
                return
            const branch = pullPressTimer.branch
            // Both slots answer "" for a name the listing lost, which must not read as level.
            const tip = branchesModel.oidOfName(branch)
            const level = tip !== "" && tip === branchesModel.upstreamOidOf(branch)
            if (!level && !repoTab.writeRefused)
                return
            pullBranchTimer.stop()
            Harness.report("pull_branch refused=" + repoTab.writeRefused
                              + " level=" + level
                              + " behind=" + branchesModel.behindOf(branch)
                              + " head=" + (worktree.headOid !== driver.headOidBefore))
            driver.complete()
        }
    }
    /// `pull-ahead`: git brings nothing (`Already up to date.`) and nothing on screen changes, so the command panel is
    /// opened — its row is the only sign the press went anywhere.
    SampleTimer {
        id: pullAheadTimer
        onTriggered: {
            if (repoTab.busyCount !== 0 || repoTab.writeSeq === driver.writeSeqBefore)
                return
            if (!page.commandsShown) {
                page.toggleCommands()
                return
            }
            if (page.pageCommands.running)
                return
            pullAheadTimer.stop()
            Harness.report("pull_ahead refused=" + repoTab.writeRefused
                              + " log=" + page.commandsShown
                              + " moved=" + (worktree.headOid !== driver.headOidBefore))
            driver.complete()
        }
    }
    SampleTimer {
        id: tipLandedTimer
        /// The op name this run's own write answered by, latched by the `Connections` below: a fetch's answer moves
        /// the counter too. Empty until then, and that is the arm: with the selection already at the tip (a merge from
        /// a ref row, a revert of HEAD) every other reading below holds before git has done anything.
        property string answeredOp: ""
        function begin() {
            tipLandedTimer.answeredOp = ""
            tipLandedTimer.start()
        }
        onTriggered: {
            const row = graphModel.rowOf(page.selectedOid)
            // `pendingHeadSelect`: right after the answer the refs are still old, so `selected === headOid` holds
            // vacuously until it resolves. `cardSettled`: until its round trip returns, the right pane still shows the
            // commit selected before the press — for a merge from a ref row, the old tip. `settling`: the refs
            // publish ahead of the status, the rebuilt graph and the sidebar, and a fast-forward lands on a row the
            // graph already has (the upstream's), so every other gate here opens while the WIP row still leashes the
            // old tip and the counts still say behind.
            if (tipLandedTimer.answeredOp === "" || page.pendingHeadSelect
                    || repoTab.busyCount !== 0 || repoTab.settling || row < 0
                    || !graphPane.rowOnScreen(row) || page.selectedOid !== worktree.headOid
                    || !driver.cardSettled)
                return
            tipLandedTimer.stop()
            Harness.report(
                "tip_landed follows="
                + (page.selectedOid !== "" && page.selectedOid === worktree.headOid)
                + " onscreen=" + (row >= 0 && graphPane.rowOnScreen(row))
                // Right after the pair above: `must_say` reads the three as one string.
                + " op=" + tipLandedTimer.answeredOp
                + " head=" + worktree.headOid.substring(0, 8)
                + " selected=" + page.selectedOid.substring(0, 8)
                + " row=" + row)
            driver.complete()
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
    SampleTimer {
        id: resetLandedTimer
        /// What this verb asked for, not read back from git — that git did it is what is being checked.
        property string target: ""
        property string mode: ""
        /// The number the reset's answer named for the first report of HEAD after it (`writeAnswerHeadSeq`),
        /// latched on the notify; 0 until the answer lands.
        property int armed: 0
        function begin(oidHex, flag) {
            resetLandedTimer.target = oidHex
            resetLandedTimer.mode = flag
            resetLandedTimer.armed = 0
            resetLandedTimer.start()
        }
        onTriggered: {
            // The write's own status, by the number its answer named (`WorktreeModel.statusSeq`): the refs read
            // moves HEAD ahead of the status that follows (`session::write::run_write`), and `--hard` is judged on the
            // tree.
            // A status counted before the write cannot reach that number.
            if (repoTab.busyCount !== 0 || resetLandedTimer.armed === 0
                    || worktree.headOid !== resetLandedTimer.target
                    || worktree.statusSeq < resetLandedTimer.armed)
                return
            const row = graphModel.rowOf(resetLandedTimer.target)
            if (row < 0 || !driver.cardSettled)
                return
            resetLandedTimer.stop()
            // No `moved=`: the wait above is that claim, and a field read off the same condition could never be false.
            Harness.report(
                "reset_landed mode=" + resetLandedTimer.mode
                + " files=" + worktree.hardResetTakes
                + " untracked=" + worktree.untrackedCount
                + " head=" + worktree.headOid.substring(0, 8)
                + " row=" + row + " rows=" + graphModel.rowTotal)
            driver.complete()
        }
    }
    // Where a merge that stopped on conflicts left the reader: the worktree, its conflicted rows on screen, and
    // nothing calling it a failure.
    SampleTimer {
        id: mergeStoppedTimer
        onTriggered: {
            if (repoTab.busyCount !== 0 || !page.wipShown || worktree.conflictCount === 0)
                return
            mergeStoppedTimer.stop()
            Harness.report(
                "merge_stopped wip=" + page.wipShown
                + " conflicts=" + (worktree.conflictCount > 0)
                + " error=" + (repoTab.lastError !== "")
                + " log=" + page.commandsOpen
                // The box holds what the merge will record; the button under it stays the only door (`cont=false`).
                + " msg=" + (worktree.opSubject !== "" && wipPane.subjectText === worktree.opSubject)
                + " cont=" + wipPane.offersOpExit("--continue")
                + " op=" + worktree.opText
                + " files=" + worktree.conflictCount)
            driver.complete()
        }
    }
    // `mergeStoppedTimer`'s twin for the operations that step (cherry-pick, revert, rebase, a stopped drop): the same
    // checks, plus the `--continue` row the merge does not have — for these it is a step onward
    // (規約 §進行中の操作から出る).
    SampleTimer {
        id: opStoppedTimer
        onTriggered: {
            // The whole write barrier: the stash and worktree listings are reads of their own, landing apart from
            // the status that says the stop, and only the write's settle says both are in
            // (`session::write::settle_after`).
            if (!driver.wroteAndSettled() || !page.wipShown || worktree.conflictCount === 0)
                return
            opStoppedTimer.stop()
            Harness.report(
                "write_stopped wip=" + page.wipShown
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
            driver.complete()
        }
    }
    // The message has to arrive before it can be typed over; the boxes are read-only until then (`cardSettled`).
    SampleTimer {
        id: rewordTimer
        onTriggered: {
            if (!driver.cardSettled)
                return
            rewordTimer.stop()
            // "edit-message-focus" types nothing: the caret is photographed over the commit's own body.
            if (Harness.autoAct === "edit-message-focus") {
                detailsPane.focusDescription()
                Harness.report("message_focus pane=details focused="
                                  + detailsPane.descriptionFocused
                                  + " color=" + detailsPane.descriptionColor)
                driver.complete()
                return
            }
            // "edit-message-away" puts the caret in the same way, then presses on the pane's band through the window's
            // own watcher: the save row that stood for the caret comes down with it (デザイン規約 §コミットメッセージの
            // 2 つの枠). `stood=` says the caret was in first; `page=` that the page itself holds the keyboard after.
            if (Harness.autoAct === "edit-message-away") {
                detailsPane.focusDescription()
                const stood = detailsPane.saveRowShown
                const win = page.Window.window
                win.caretHand.pressedAt(detailsPane.mapToItem(null, detailsPane.width / 2, 2))
                Harness.report("message_away stood=" + stood
                                  + " row=" + detailsPane.saveRowShown
                                  + " caret=" + detailsPane.descriptionFocused
                                  + " page=" + (win.activeFocusItem === page))
                driver.complete()
                return
            }
            detailsPane.setMessageText(Harness.autoActArg, "")
            // "edit-message-refused" waits for the save to turn the amend down (`--preset undone`) and reads its tip.
            if (Harness.autoAct === "edit-message-refused") {
                saveRefusedTimer.start()
                return
            }
            // "edit-message" stops here, with the save row on screen.
            if (Harness.autoAct === "reword")
                driver.pressWrite("reword", () => {
                    detailsPane.submitMessage()
                    return true
                })
            // "edit-message-leave" walks away from the unsaved text: nothing asks, and the shot is the next commit's
            // own message.
            else if (Harness.autoAct === "edit-message-leave")
                page.activateRow(graphModel.oidAt(graphModel.rowOf(page.selectedOid) + 1))
            if (Harness.autoAct === "reword")
                writeBarrier.start()
            else
                renderedBarrier.begin()
        }
    }
    // The save of HEAD's message over a stage that takes back all HEAD changed: git's answer about the amend is what
    // greys it, and the run waits on the button's tip alone — pointed at once the save row stands, the tip comes up
    // only on a button that refuses, with the words the reader gets.
    SampleTimer {
        id: saveRefusedTimer
        onTriggered: {
            if (!detailsPane.saveRowShown)
                return
            detailsPane.savePointedAt = true
            const tip = detailsPane.saveTip
            if (!tip.shown)
                return
            saveRefusedTimer.stop()
            Harness.report("save_refused enabled=" + detailsPane.saveEnabled
                              + " tip=" + tip.shown
                              + " why=" + tip.words)
            driver.complete()
        }
    }
}
