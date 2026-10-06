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
    readonly property var workTree: driver.workTree
    readonly property var graphModel: driver.graphModel
    readonly property var branchesModel: driver.branchesModel
    readonly property var tagsModel: driver.tagsModel
    readonly property var stashesModel: driver.stashesModel
    readonly property var refRebaseItem: driver.refRebaseItem
    readonly property var refPullItem: driver.refPullItem
    readonly property var refMenu: driver.refMenu
    readonly property var graphPane: driver.graphPane
    readonly property var detailsPane: driver.detailsPane
    readonly property var wipPane: driver.wipPane
    readonly property var commitMenuState: driver.commitMenuState
    readonly property var commitMenu: driver.commitMenu
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
            // catches it half-way (reader lands on a commit) or not (reader stays on the working-tree row) is the
            // scheduler's. Started from the commit, both end on a commit's page (`followVanishedCommit`).
            const foldOid = driver.autoActOid(arg)
            acts.readCommit(foldOid)
            page.openRowMenu(foldOid)
            page.squashCommit(foldOid)
            if (act !== "squash")
                driver.barrierNotice.start()
        } else if (act === "reword" || act === "edit-message"
                   || act === "edit-message-leave"
                   || act === "edit-message-focus" || act === "edit-message-away") {
            // HEAD's own: the only row that takes typing (`offers::message_edit`), branch or not.
            page.jumpToRef(workTree.headOid)
            rewordTimer.start()
        } else if (act === "cherry-pick" || act === "cherry-pick-stops") {
            const pickOid = driver.autoActOid(arg)
            const pickRow = graphModel.rowOf(pickOid)
            if (pickRow >= 0) {
                graphPane.jumpToRow(pickRow)
                page.activateRow(pickOid)
            }
            // Through → answers at the tip; stopped → answers in the working tree
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
                              + ((hardResetItem.note !== "") === (workTree.hardResetTakes > 0))
                              + " files=" + workTree.hardResetTakes
                              + " untracked=" + workTree.untrackedCount
                              + " note=" + hardResetItem.note)
            if (act === "reset-hard") {
                resetLandedTimer.begin(wipeTo, "hard")
                hardResetItem.completeHold()
            }
        } else if (act === "commit-menu" || act === "reset-menu"
                   || act === "branch-card" || act === "tag-card") {
            // With no row named, the row under HEAD's: most of this menu is about a commit the branch is not on, and
            // the rows above HEAD belong to whatever else the graph is showing.
            let menuOid = arg
            if (menuOid === "")
                menuOid = graphModel.oidAt(
                    graphModel.rowOf(workTree.headOid) + 1)
            page.openRowMenu(menuOid)
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
            // Opened first where a replay follows, for the fold's reason. The bare menu stays where the page opened:
            // nothing after it moves the reader, and a read started here would race the picture.
            const dropOid = driver.autoActOid(arg)
            if (act !== "drop-commit")
                acts.readCommit(dropOid)
            page.openRowMenu(dropOid)
            Harness.report("drop_row " + dropCommitItem.code
                              + " " + dropCommitItem.text
                              + " oid=" + commitMenuState.menuOid.substring(0, 8)
                              + " hold=" + (dropCommitItem.holdMs > 0)
                              + " reached=" + workTree.headReachedElsewhere)
            if (act !== "drop-commit") {
                // "drop-stops" stops with the work still in the stash it took (`write_stopped stashes=`).
                if (act === "drop-stops")
                    opStoppedTimer.start()
                if (dropCommitItem.holdMs > 0) {
                    dropCommitItem.completeHold()
                } else {
                    // The item's own click takes the menu down before the write (`CommitRowMenu`
                    // `dropCommitItem.onHeld`); fired past it, whether the menu goes before the census walk is the
                    // scheduler's. So it goes here, and completion waits until it is gone (`AutoActDriver.menuGoing`).
                    commitMenu.dismiss()
                    driver.menuGoing = commitMenu
                    page.dropCommit(commitMenuState.menuOid)
                }
                // Nowhere to land: the refusal's bar is the shot.
                if (act === "drop-last-commit")
                    driver.barrierNotice.start()
            }
        } else if (act === "wip-landing-stopped") {
            // The stop's working-tree landing, read in a pass that carries every other copy's row and none of this
            // window's. The run is started into that arrangement (`xtask::verify::child`, `harness::faults`) because
            // which of the walk and the status arrives first is otherwise the scheduler's. The press needs a clean
            // tree: a replay is refused over uncommitted work.
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
                    // `rebase-stops` answers in the working tree (規約 §未コミット変更がある状態で履歴を書き換える
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
        return arg === "" ? graphModel.oidAt(graphModel.rowOf(workTree.headOid) + 1)
                          : driver.autoActOid(arg)
    }

    /// Opens the commit these verbs are about (rules-refs/app-ui.md「コミットの詳細を読む動詞は自分でその行を開く」).
    function readCommit(oidHex) {
        const row = graphModel.rowOf(oidHex)
        if (row >= 0)
            page.activateRow(oidHex, row)
    }

    // `replay-running`: the screen while git is out — the step badge, the doors held down, the ring beside the hand.
    // This only watches: the page ticks the count itself (`Metrics.opProgressMs`), so a build whose tick never
    // reaches the badge waits out the watchdog. Then the face is held (`RepoPage.autoReplayHeld`, the latch
    // `doors-held` takes): the write answers before the reads it invalidated, so a picture taken at the edge could
    // show a screen the replay had already left.
    SampleTimer {
        id: replayRunningTimer
        onTriggered: {
            // Queued is not out: git writes nothing to count until it is actually replaying.
            if (!repoTab.replaying || workTree.opSteps === 0 || workTree.opStep === 0)
                return
            replayRunningTimer.stop()
            page.autoReplayHeld = true
            const win = page.Window.window
            // Hover cannot be injected (offscreen's hand sits at the window's corner), so the run places the hand the
            // ring reads (`Main.holdWaitHand`): over the rewritten pane, clear of row text a ring would hide.
            const seat = graphPane.mapToItem(null, graphPane.width / 6, graphPane.height / 3)
            win.holdWaitHand(seat.x, seat.y)
            // That a range is being counted at all, not which step: the step is whatever git had reached.
            const counted = workTree.opStep > 0 && workTree.opStep <= workTree.opSteps
                            && workTree.opSteps > 1
            Harness.report("replay_running op=" + workTree.opText + " counted=" + counted
                              + " ring=" + win.waitRingShown
                              + " held=" + page.replayRunning
                              + " step=" + workTree.opStep + " steps=" + workTree.opSteps)
            renderedBarrier.begin()
        }
    }
    // Both moments of the stop's landing in one line, since neither means anything alone: `owed=` / `early=` are the
    // page after a pass that beat the status was offered the landing and turned it down; `wip=` is the pass carrying
    // ours landing on this window's tree. Every copy's row wears the same all-zero id, so taking row 0's id for ours
    // lands the press on a copy.
    SampleTimer {
        id: stoppedLandingTimer
        property bool read: false
        /// `finishCount` when the pass was asked for: the landing is only offered passes that finish after it was
        /// armed (`RepoPage.onStatsChanged` → `tryPendingWipSelect`), so an earlier reading answers nothing.
        property int owedAt: -1
        property bool owed: false
        property bool early: false
        property bool earlyCopy: false
        /// For a run not started with the hold up (both harness calls answer whether it was): says so and stops.
        function refuse() {
            stoppedLandingTimer.stop()
            Harness.report("wip_stop_landing held=false")
            driver.complete()
        }
        onTriggered: {
            // First the stop: answered, an operation standing with its status in, the landing armed.
            if (stoppedLandingTimer.owedAt < 0) {
                if (repoTab.busyCount !== 0 || workTree.opText === "" || !workTree.wipRowStands
                        || graphModel.loading || !page.pageLanding)
                    return
                // Ask for the pass: under the hold nothing else brings one (our row appearing normally does, and a
                // stopped replay moves no branch). It lands with every other copy's row and none of ours — the pass
                // the landing has to turn down.
                if (!graphModel.walkAgainWhileHeld()) {
                    stoppedLandingTimer.refuse()
                    return
                }
                stoppedLandingTimer.owedAt = graphModel.finishCount
                return
            }
            // Then that pass, led by a neighbour copy's row (`GraphModel.carriedTop`); a pass without one on top
            // cannot catch the misreading, so it is read past.
            if (!stoppedLandingTimer.read) {
                if (graphModel.loading || graphModel.finishCount <= stoppedLandingTimer.owedAt
                        || graphModel.wipRow || !graphModel.carriedTop)
                    return
                stoppedLandingTimer.read = true
                stoppedLandingTimer.owed = page.pageLanding
                stoppedLandingTimer.early = page.wipShown
                stoppedLandingTimer.earlyCopy = page.carriedPath !== ""
                if (!graphModel.letTheWorkingTreeRowThrough())
                    stoppedLandingTimer.refuse()
                return
            }
            if (!graphModel.wipRow || !PageSettled.settled(page))
                return
            stoppedLandingTimer.stop()
            Harness.report("wip_stop_landing held=true otherTop=true owed=" + stoppedLandingTimer.owed
                              + " early=" + stoppedLandingTimer.early
                              + " earlyCopy=" + stoppedLandingTimer.earlyCopy
                              + " wip=" + page.wipShown
                              + " copy=" + (page.carriedPath !== "")
                              + " op=" + workTree.opText
                              // This landing moves the highlight without activating a row (`tryPendingWipSelect`).
                              + " lit=" + graphPane.view.currentIndex)
            driver.complete()
        }
    }

    /// The press behind `pull-go` / `pull-ahead`. A pull answers at the tip (`RepoTab::settle_write`), so `pull-go`
    /// finishes on `tipLandedTimer`.
    SampleTimer {
        id: pullPressTimer
        property string after: "pull-go"
        onTriggered: {
            if (repoTab.busyCount !== 0)
                return
            const branch = workTree.branch
            if (!refMenu.opened)
                page.openRefMenu("branch", branch, branch, branchesModel.oidOfName(branch))
            if (!refMenu.opened || !refPullItem.offered)
                return
            pullPressTimer.stop()
            // Read back as `pull-ahead`'s `moved=`: a picture cannot say whether a graph was redrawn or never touched.
            driver.headOidBefore = workTree.headOid
            if (pullPressTimer.after === "pull-go")
                tipLandedTimer.begin()
            driver.pressWrite(pullPressTimer.after, () => {
                refPullItem.triggered()
                return true
            })
            if (pullPressTimer.after === "pull-ahead")
                pullAheadTimer.start()
            // A hand's press takes the card down; this one must be told, and completion waits out its exit
            // (`menuGoing` — the census walks what is visible).
            driver.menuGoing = refMenu
            refMenu.close()
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
                              + " moved=" + (workTree.headOid !== driver.headOidBefore))
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
            // commit selected before the press — for a merge from a ref row, the old tip.
            if (tipLandedTimer.answeredOp === "" || page.pendingHeadSelect
                    || repoTab.busyCount !== 0 || row < 0
                    || !graphPane.rowOnScreen(row) || page.selectedOid !== workTree.headOid
                    || !driver.cardSettled)
                return
            tipLandedTimer.stop()
            Harness.report(
                "tip_landed follows="
                + (page.selectedOid !== "" && page.selectedOid === workTree.headOid)
                + " onscreen=" + (row >= 0 && graphPane.rowOnScreen(row))
                // Right after the pair above: `must_say` reads the three as one string.
                + " op=" + tipLandedTimer.answeredOp
                + " head=" + workTree.headOid.substring(0, 8)
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
            // The write's own status, by the number its answer named (`WorkTreeModel.statusSeq`): the refs read moves
            // HEAD ahead of the status that follows (`session::write::run_write`), and `--hard` is judged on the tree.
            // A status counted before the write cannot reach that number.
            if (repoTab.busyCount !== 0 || resetLandedTimer.armed === 0
                    || workTree.headOid !== resetLandedTimer.target
                    || workTree.statusSeq < resetLandedTimer.armed)
                return
            const row = graphModel.rowOf(resetLandedTimer.target)
            if (row < 0 || !driver.cardSettled)
                return
            resetLandedTimer.stop()
            // No `moved=`: the wait above is that claim, and a field read off the same condition could never be false.
            Harness.report(
                "reset_landed mode=" + resetLandedTimer.mode
                + " files=" + workTree.hardResetTakes
                + " untracked=" + workTree.untrackedCount
                + " head=" + workTree.headOid.substring(0, 8)
                + " row=" + row + " rows=" + graphModel.rowTotal)
            driver.complete()
        }
    }
    // Where a merge that stopped on conflicts left the reader: the working tree, its conflicted rows on screen, and
    // nothing calling it a failure.
    SampleTimer {
        id: mergeStoppedTimer
        onTriggered: {
            if (repoTab.busyCount !== 0 || !page.wipShown || workTree.conflictCount === 0)
                return
            mergeStoppedTimer.stop()
            Harness.report(
                "merge_stopped wip=" + page.wipShown
                + " conflicts=" + (workTree.conflictCount > 0)
                + " error=" + (repoTab.lastError !== "")
                + " log=" + page.commandsOpen
                // The box holds what the merge will record; the button under it stays the only door (`cont=false`).
                + " msg=" + (workTree.opSubject !== "" && wipPane.subjectText === workTree.opSubject)
                + " cont=" + wipPane.offersOpExit("--continue")
                + " op=" + workTree.opText
                + " files=" + workTree.conflictCount)
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
            if (!driver.wroteAndSettled() || !page.wipShown || workTree.conflictCount === 0)
                return
            opStoppedTimer.stop()
            Harness.report(
                "write_stopped wip=" + page.wipShown
                + " conflicts=" + (workTree.conflictCount > 0)
                + " error=" + (repoTab.lastError !== "")
                + " log=" + page.commandsOpen
                + " cont=" + wipPane.offersOpExit("--continue")
                // A replay stops on no branch, and the main copy's WORKTREES row is named by its folder
                // (デザイン規約 §左メニューの所作); a pick or a revert stops on the branch.
                + " home=" + driver.navProbe.homeCopyName()
                // Where carried work went: git's words are not raised over the stop, so only the count and the
                // stash row say it (規約 §未コミット変更がある状態で履歴を書き換える).
                + " stashes=" + stashesModel.total
                + " op=" + workTree.opText
                + " files=" + workTree.conflictCount)
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
}
