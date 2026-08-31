pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// Rewriting what is already committed: squash, reword, drop, cherry-pick, revert, merge, rebase and reset —
/// and the menus that offer them.
///
/// Built by `AutoActDriver`, which `RepoPage` builds only when a verb was given. What these verbs act on
/// hangs off that driver; the names it owns are
/// read back once below so the verbs can name them bare.
// An `Item` only because `QtObject` has no default property to hold the timers below; it draws nothing
// and is never given a size.
Item {
    id: acts

    /// The driver these verbs belong to. `var` because naming its type here would be a circle: it is
    /// the file that builds this one.
    required property var driver

    // The driver's own names, read once so the verbs can name them bare.
    readonly property var page: driver.page
    readonly property var repoTab: driver.repoTab
    readonly property var workTree: driver.workTree
    readonly property var graphModel: driver.graphModel
    readonly property var branchesModel: driver.branchesModel
    readonly property var stashesModel: driver.stashesModel
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

    /// Runs `act` if it is one of this family's, and says whether it was. The families are asked in turn
    /// and the first to know a verb runs it — no verb is named by two of them (`AutoActDriver`).
    function run(act, arg) {
        if (act === "squash") {
            page.openRowMenu(branchesModel.headOid)
            page.squashCommit(branchesModel.headOid)
        } else if (act === "reword" || act === "edit-message"
                   || act === "edit-message-leave"
                   || act === "edit-message-focus") {
            // HEAD's own, not the branch tip: the only row that takes typing (`offers::message_edit`), branch or not.
            page.jumpToRef(workTree.headOid !== "" ? workTree.headOid : branchesModel.headOid)
            rewordTimer.start()
        } else if (act === "cherry-pick" || act === "cherry-pick-stops") {
            const pickOid = driver.autoActOid(arg)
            const pickRow = graphModel.rowOf(pickOid)
            if (pickRow >= 0) {
                graphPane.jumpToRow(pickRow)
                page.activateRow(pickOid)
            }
            // Two landings, one press: a copy that goes through answers at the tip, one that stops answers in the
            // working tree (規約 §履歴を合流させる / §進行中の操作から出る).
            if (act === "cherry-pick-stops")
                opStoppedTimer.begin(false)
            else
                tipLandedTimer.begin()
            repoTab.cherryPick(pickOid)
        } else if (act === "reset-soft" || act === "reset-mixed") {
            // With nothing given, the row under HEAD's: a reset to where the branch already stands moves nothing, and
            // an empty name would reach git as `reset ''`.
            page.openRowMenu(arg === "" ? graphModel.oidAt(graphModel.rowOf(workTree.headOid) + 1)
                             : driver.autoActOid(arg))
            page.moveBranchHere(act === "reset-soft" ? "soft" : "mixed")
        } else if (act === "reset-hard" || act === "reset-hard-confirm") {
            // "-confirm" stops with the held row on screen; "reset-hard" runs the hold to its end. The row resolves as
            // reset-soft's.
            page.openRowMenu(arg === "" ? graphModel.oidAt(graphModel.rowOf(workTree.headOid) + 1)
                             : driver.autoActOid(arg))
            resetMenu.offer()
            if (act === "reset-hard")
                hardResetItem.completeHold()
        } else if (act === "commit-menu" || act === "reset-menu"
                   || act === "branch-card" || act === "tag-card") {
            // With no row named, the row under HEAD's: most of this menu is about a commit the branch is *not* already
            // standing on, and it is counted from where HEAD actually sits — the rows above belong to whatever else the
            // graph is showing.
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
            AppBackend.report("commit_menu rows=" + commitMenu.offeredRows
                              + " can_move=" + commitMenuState.menuCanMoveBranch)
        } else if (act === "drop-commit" || act === "drop-commit-go" || act === "drop-stops") {
            // The plan is built by object name, the way a graph row hands one over — a symbolic name is not what this
            // takes.
            page.openRowMenu(driver.autoActOid(arg))
            AppBackend.report("drop_row " + dropCommitItem.code
                              + " " + dropCommitItem.text
                              + " oid=" + commitMenuState.menuOid.substring(0, 8)
                              + " hold=" + (dropCommitItem.holdMs > 0)
                              + " reached=" + repoTab.headReachedElsewhere)
            if (act !== "drop-commit") {
                // "drop-stops" is the replay that walks into a hole and stops with the work still in the stash it
                // took: the landing is the working tree, and the count and the stash's own row are what say where
                // that work went (規約 §未コミット変更がある状態で履歴を書き換える の着地表).
                if (act === "drop-stops")
                    opStoppedTimer.begin(true)
                if (dropCommitItem.holdMs > 0)
                    dropCommitItem.completeHold()
                else
                    page.dropCommit(commitMenuState.menuOid)
            }
        } else if (act === "merge-branch" || act === "merge-stops" || act === "rebase-onto"
                   || act === "rebase-stops" || act === "replay-running" || act === "revert-commit"
                   || act === "revert-stops" || act === "integrate-menu") {
            // Through the menus a right-click opens, so the rows' own gating decides whether anything runs.
            if (act === "revert-commit" || act === "revert-stops") {
                // The click that opens this menu selects the row too (GraphRowDelegate), so the hook takes both steps a
                // right-click takes.
                const oidHex = driver.autoActOid(arg)
                graphPane.jumpToRow(graphModel.rowOf(oidHex))
                page.activateRow(oidHex)
                page.openRowMenu(oidHex)
                // The undo's two landings, read like the copy's above.
                if (act === "revert-stops")
                    opStoppedTimer.begin(false)
                else
                    tipLandedTimer.begin()
                repoTab.revert(oidHex)
            } else {
                page.openRefMenu("branch", arg, arg,
                                 branchesModel.oidOfName(arg))
                if (act === "merge-branch" || act === "merge-stops") {
                    // Two landings, one press: a merge that goes through answers at the tip, and one that stops
                    // answers in the working tree (規約 §履歴を合流させる / §進行中の操作から出る).
                    if (act === "merge-stops")
                        mergeStoppedTimer.start()
                    else
                        tipLandedTimer.begin()
                    repoTab.merge(arg, false, false, "")
                } else if (act === "rebase-onto" || act === "rebase-stops" || act === "replay-running") {
                    // A replay that stopped part-way answers in the working tree like the other three: no commit
                    // was written, and the badge, the exit card and the conflicted rows are where the press ends
                    // (規約 §未コミット変更がある状態で履歴を書き換える の着地表). The third of them answers nowhere: its
                    // subject is the screen *while* git is out, so it is caught on the way rather than at a landing.
                    if (act === "rebase-stops") {
                        opStoppedTimer.begin(false)
                    } else if (act === "replay-running") {
                        replayRunningTimer.start()
                        // What a click on the row does next, which the other two never need: their picture is a
                        // landing the menu is long gone from, and this one is of the screen the press left behind.
                        driver.refMenu.close()
                    }
                    repoTab.rebase(arg, "", true)
                }
            }
        } else {
            return false
        }
        return true
    }
    // ---- the picture of a replay that is still replaying -----------------
    // What only exists while git is out: the badge counting the steps out of git's own file, the doors the page holds
    // down, and the ring beside the hand. Every other rebase verb photographs a landing.
    //
    // **Nothing is pressed here.** The page runs the count's own tick for as long as a replay is out
    // (`Metrics.opProgressMs`), so this waits for the number the way a reader does — and a build whose tick never
    // started, or never reached the badge, waits out the watchdog rather than being carried by the run.
    //
    // Then the face is **held** — `RepoPage.autoReplayHeld`, the same latch `doors-held` takes, because the two runs
    // photograph one state from two sides. The write answers before the reads it invalidated, so a run that only
    // reported at the edge could photograph a screen the replay had already left (verify-ui スキル).
    SampleTimer {
        id: replayRunningTimer
        onTriggered: {
            // Queued is not out: the write travels a queue that can be carrying something else, and git writes
            // nothing to count until it is actually replaying.
            if (!repoTab.replaying || workTree.opSteps === 0 || workTree.opStep === 0)
                return
            replayRunningTimer.stop()
            page.autoReplayHeld = true
            const win = page.Window.window
            // Where the hand would have been: over the pane the replay is rewriting, and clear of the rows' own ink
            // — a sixteen-pixel ring laid over a subject line cannot be judged at all. Hover cannot be injected, so
            // the run writes the one answer the mark reads (`Main.holdWaitHand`); offscreen's own hand sits at the
            // window's origin, which is a corner rather than a place worth photographing a mark in.
            const seat = graphPane.mapToItem(null, graphPane.width / 6, graphPane.height / 3)
            win.holdWaitHand(seat.x, seat.y)
            // `counted` rather than the numbers: the step is whatever git had reached, and what is being claimed is
            // that the badge is counting a range out at all.
            const counted = workTree.opStep > 0 && workTree.opStep <= workTree.opSteps
                            && workTree.opSteps > 1
            AppBackend.report("replay_running op=" + workTree.opText + " counted=" + counted
                              + " ring=" + win.waitRingShown
                              + " held=" + page.replayRunning
                              + " step=" + workTree.opStep + " steps=" + workTree.opSteps)
            renderedBarrier.begin()
        }
    }
    // Where an operation that answers at the tip left the reader — one report for the three of them. The write, its
    // refresh and the beat the viewport waits out all have to be behind it, and the picture cannot answer the second
    // half: a row can be selected and still be somewhere nobody can see.
    SampleTimer {
        id: tipLandedTimer
        /// The name this run's own write answered by, taken from the answer that carried it rather than read back
        /// at the report — **every** answer rewrites the group it comes from (`RepoTab::settle_write`), so a fetch
        /// settling while the landing is still being waited out takes it away again. The counter having moved says
        /// only that *an* answer arrived; a fetch's answer moves it too.
        ///
        /// Empty until that answer, and the emptiness is the arm. A press made with the selection already sitting at
        /// the tip — a merge from a ref row, a revert of HEAD — satisfies every other reading below before git has
        /// done anything, and the run would quit over an untouched repository (measured, a copy nobody could
        /// see in the picture, `op=` empty in this very report, green).
        property string answeredOp: ""
        function begin() {
            tipLandedTimer.answeredOp = ""
            tipLandedTimer.start()
        }
        onTriggered: {
            const row = graphModel.rowOf(page.selectedOid)
            // Then the landing that answer armed: at the barrier the refs are still the old ones, so
            // `selected === headOid` holds vacuously until the page's own `pendingHeadSelect` has resolved onto the
            // refreshed pair.
            //
            // And last the pane the landing sends for. The details of the commit that was selected *before* the press
            // are still on the right until its own round trip comes back, and for a merge from a ref row that commit
            // is the old tip — so the half of this the picture does hold, whose commit fills the right-hand pane,
            // frames as the repository before the write (measured, the pane's second round trip landed after
            // `screenshot saved=true`). Waited out the way `stashLandTimer` waits for it.
            if (tipLandedTimer.answeredOp === "" || page.pendingHeadSelect
                    || repoTab.busyCount !== 0 || row < 0
                    || !graphPane.rowOnScreen(row) || page.selectedOid !== branchesModel.headOid
                    || !driver.cardSettled)
                return
            tipLandedTimer.stop()
            AppBackend.report(
                "tip_landed follows="
                + (page.selectedOid !== "" && page.selectedOid === branchesModel.headOid)
                + " onscreen=" + (row >= 0 && graphPane.rowOnScreen(row))
                // Next to the pair above because that is where the harness reads it: the name is what tells the
                // three verbs' own writes from anything else that could have moved the counter (`must_say`).
                + " op=" + tipLandedTimer.answeredOp
                + " head=" + branchesModel.headOid.substring(0, 8)
                + " selected=" + page.selectedOid.substring(0, 8)
                + " row=" + row)
            driver.complete()
        }
    }
    /// The answer `tipLandedTimer` waits on, taken on the notify rather than on the sampling beat: two answers inside
    /// one beat would leave only the later one to be read, and it is the earlier one that says the write was this
    /// run's (app-ui.md §UI 自動化の因果性 — 一瞬だけ立つ状態は signal で観測して latch する). The *rise* of
    /// `busyCount` is deliberately not waited for anywhere in this chain: a write that begins and ends between two
    /// looks never shows one, and requiring it wedges the run instead (`writeSeqBefore`).
    ///
    /// **And out of the answers that notify carried rather than off the group they leave behind**: one drain empties
    /// the whole queue and notifies once (`RepoTab::write_answers`), so an answer arriving behind this one — the
    /// interval's own fetch, most often — leaves the group describing itself, with the landing nowhere on it.
    ///
    /// `writeAnswerAtTip` is the bridge's own word for "landed, did not stop part-way, and answers at the tip" — the
    /// op names are turned into meanings on that side of it (`RepoTab::settle_write`), not branched on here. A
    /// fetch's answer, a refusal and a stop all leave the arm down, and the run walks into its watchdog rather than
    /// photographing a repository nothing happened to.
    Connections {
        target: driver.repoTab
        function onWriteSeqChanged() {
            if (!tipLandedTimer.running || tipLandedTimer.answeredOp !== "")
                return
            const tab = driver.repoTab
            for (let i = 0; i < tab.writeAnswerCount(); i++) {
                if (tab.writeAnswerSeq(i) <= driver.writeSeqBefore || !tab.writeAnswerAtTip(i))
                    continue
                tipLandedTimer.answeredOp = tab.writeAnswerOp(i)
                return
            }
        }
    }
    // Where a merge that stopped on conflicts left the reader. The other half of `tipLandedTimer`: there is no commit
    // at the tip to land on, and what the press is answered with is the working tree — so this waits for the rows the
    // stop wrote to be on screen, and says in the same breath that nothing called it a failure.
    SampleTimer {
        id: mergeStoppedTimer
        onTriggered: {
            if (repoTab.busyCount !== 0 || !page.wipShown || workTree.conflictCount === 0)
                return
            mergeStoppedTimer.stop()
            AppBackend.report(
                "merge_stopped wip=" + page.wipShown
                + " conflicts=" + (workTree.conflictCount > 0)
                + " error=" + (repoTab.lastError !== "")
                + " log=" + page.commandsOpen
                // The box opened holding what the merge is about to record, and the card is not offering a second
                // door onto the button under it.
                + " msg=" + (workTree.opSubject !== "" && wipPane.subjectText === workTree.opSubject)
                + " cont=" + wipPane.offersOpExit("--continue")
                + " op=" + workTree.opText
                + " files=" + workTree.conflictCount)
            driver.complete()
        }
    }
    // Where a cherry-pick, a revert or a rebase that stopped on conflicts left the reader. `mergeStoppedTimer`'s twin,
    // for the four that step: the same press with no new commit at the tip to land on, answered by the working tree.
    // What the picture cannot hold is the same pair — that nothing wrote a red line over an ordinary conflict, and
    // that the command log stayed down — plus the row the merge does not have: these keep `--continue`, because for
    // them it is a step onward and not the commit somebody is writing (規約 §進行中の操作から出る).
    SampleTimer {
        id: opStoppedTimer
        // Whether a carry is part of this landing. The stash section is refreshed *after* the graph, so reading it
        // at the write barrier answers 0 for a tree whose work is sitting in an entry — and where the entry is the
        // whole claim, that is the answer arriving too early rather than the truth.
        property bool carried: false
        function begin(withStash) {
            opStoppedTimer.carried = withStash
            opStoppedTimer.start()
        }
        onTriggered: {
            if (repoTab.busyCount !== 0 || !page.wipShown || workTree.conflictCount === 0
                    || (opStoppedTimer.carried && stashesModel.total === 0))
                return
            opStoppedTimer.stop()
            AppBackend.report(
                "write_stopped wip=" + page.wipShown
                + " conflicts=" + (workTree.conflictCount > 0)
                + " error=" + (repoTab.lastError !== "")
                + " log=" + page.commandsOpen
                + " cont=" + wipPane.offersOpExit("--continue")
                // What the carry left behind, for a rewrite that took a stash out of its own way: git's words
                // are not raised over the stop, so the count and the graph's own row are the only things saying
                // where the work went (規約 §未コミット変更がある状態で履歴を書き換える).
                + " stashes=" + stashesModel.total
                + " op=" + workTree.opText
                + " files=" + workTree.conflictCount)
            driver.complete()
        }
    }
    // The message has to arrive before it can be typed over: `cardSettled` is that wait, and the boxes read-only until
    // the details of the row jumped to are the ones on screen.
    SampleTimer {
        id: rewordTimer
        onTriggered: {
            if (!driver.cardSettled)
                return
            rewordTimer.stop()
            // "edit-message-focus" types nothing: the commit's own body is what the caret has to be photographed on top
            // of, and an empty box would only show the placeholder.
            if (AppBackend.autoAct === "edit-message-focus") {
                detailsPane.focusDescription()
                AppBackend.report("message_focus pane=details focused="
                                  + detailsPane.descriptionFocused
                                  + " color=" + detailsPane.descriptionColor)
                driver.complete()
                return
            }
            detailsPane.setMessageText(AppBackend.autoActArg, "")
            // "edit-message" stops here, with the save row on screen.
            if (AppBackend.autoAct === "reword") {
                driver.writeSeqBefore = repoTab.writeSeq
            }
            if (AppBackend.autoAct === "reword")
                detailsPane.submitMessage()
            // "edit-message-leave" walks away from the unsaved text. Nothing asks any more — the draft goes and the
            // next commit's own message arrives, which is what the shot is of.
            else if (AppBackend.autoAct === "edit-message-leave")
                page.activateRow(graphModel.oidAt(graphModel.rowOf(page.selectedOid) + 1))
            if (AppBackend.autoAct === "reword")
                writeBarrier.start()
            else
                renderedBarrier.begin()
        }
    }
}
