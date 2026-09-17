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
// An `Item` only because `QtObject` has no default property to hold the timers below; it is a
// sizeless holder.
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
    readonly property var tagsModel: driver.tagsModel
    readonly property var stashesModel: driver.stashesModel
    readonly property var refRebaseItem: driver.refRebaseItem
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
    /// and the first to know a verb runs it — each verb is named by one (`AutoActDriver`).
    function run(act, arg) {
        if (act === "squash" || act === "fold-across-merge" || act === "fold-off-branch"
                || act === "fold-first-commit" || act === "fold-unfetched-base") {
            // One press, five landings: the fold itself, and the four shapes a history is turned down for
            // (`report::rewrite_across_merge` 他). Each refusal is its own verb because each has a different line
            // to be caught saying — the picture alone cannot tell a bar that came down from a log that came up.
            // The tip without an argument, and any row with one: the refusal about a commit the branch cannot
            // see needs a row that is not on it (`row:` / an oid).
            const foldOid = driver.autoActOid(arg)
            page.openRowMenu(foldOid)
            page.squashCommit(foldOid)
            // The refusals are waited on all the way down: the answer raises the bar, and a picture taken on the
            // answer catches one whose words are written and whose height is still nothing (`AutoActDriver`).
            if (act !== "squash")
                driver.barrierNotice.start()
        } else if (act === "reword" || act === "edit-message"
                   || act === "edit-message-leave"
                   || act === "edit-message-focus") {
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
            const backTo = acts.resetTarget(arg)
            acts.readCommit(backTo)
            page.openRowMenu(backTo)
            const mode = act === "reset-soft" ? "soft" : "mixed"
            resetLandedTimer.begin(backTo, mode)
            page.moveBranchHere(mode)
        } else if (act === "reset-hard" || act === "reset-hard-confirm") {
            // "-confirm" stops with the held row on screen; "reset-hard" runs the hold to its end. The row resolves as
            // reset-soft's.
            const wipeTo = acts.resetTarget(arg)
            acts.readCommit(wipeTo)
            page.openRowMenu(wipeTo)
            resetMenu.offer()
            // What the held row is offering to take besides the commits. `tagged=` is the row's own tag read against
            // the count it is drawn from — **the tag is there exactly when there is something to lose** — so it holds
            // on a clean fixture as well as a dirty one, and a build that drew the tag off nothing, or dropped it
            // over a dirty tree, says so. `note=` carries spaces, so it goes last.
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
            Harness.report("commit_menu rows=" + commitMenu.offeredRows
                              + " can_move=" + commitMenuState.menuCanMoveBranch)
        } else if (act === "drop-commit" || act === "drop-commit-go" || act === "drop-stops"
                   || act === "drop-last-commit") {
            // The plan is built by object name, the way a graph row hands one
            // over.
            page.openRowMenu(driver.autoActOid(arg))
            Harness.report("drop_row " + dropCommitItem.code
                              + " " + dropCommitItem.text
                              + " oid=" + commitMenuState.menuOid.substring(0, 8)
                              + " hold=" + (dropCommitItem.holdMs > 0)
                              + " reached=" + workTree.headReachedElsewhere)
            if (act !== "drop-commit") {
                // "drop-stops" is the replay that walks into a hole and stops with the work still in the stash it
                // took: the landing is the working tree, and the count and the stash's own row are what say where
                // that work went (規約 §未コミット変更がある状態で履歴を書き換える の着地表).
                if (act === "drop-stops")
                    opStoppedTimer.begin(true)
                if (dropCommitItem.holdMs > 0) {
                    dropCommitItem.completeHold()
                } else {
                    // The item's own click takes the menu down before the write goes out (`CommitRowMenu`,
                    // `dropCommitItem.onHeld` has the order). Fired past the item, the menu stands until something
                    // else closes it, and whether that beats the census walk is the scheduler's — so it is taken
                    // down here, and the completion waits for it to be gone (`AutoActDriver.menuGoing`).
                    commitMenu.dismiss()
                    driver.menuGoing = commitMenu
                    page.dropCommit(commitMenuState.menuOid)
                }
                // The drop with nowhere to land comes back as a report, and the bar it comes down in is the shot.
                if (act === "drop-last-commit")
                    driver.barrierNotice.start()
            }
        } else if (act === "wip-landing-stopped") {
            // The landing a stopped operation owes the working tree, taken in a pass that carries every other
            // copy's row and none of this window's — the arrangement the run is started into
            // (`xtask::verify::child`, `harness::faults`), since which of the walk and the status gets there first
            // is the scheduler's. **The press comes from a clean tree**: a replay is refused over uncommitted work,
            // and the row this landing goes to is the one the stop itself leaves behind.
            const clash = arg === "" ? "side/clash" : arg
            page.openRefMenu("branch", clash, clash, branchesModel.oidOfName(clash))
            stoppedLandingTimer.start()
            repoTab.rebase(clash, "", true)
        } else if (act === "integrate-menu") {
            acts.openIntegrateMenu(arg)
        } else if (act === "merge-branch" || act === "merge-stops" || act === "rebase-onto"
                   || act === "rebase-stops" || act === "replay-running" || act === "revert-commit"
                   || act === "revert-stops") {
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
                    // subject is the screen *while* git is out, so it is caught on the way.
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
    /// The ref menu left standing on what a merge or a rebase would bring in — a branch by bare name, a tag with
    /// `:tag` after it.
    ///
    /// The report is the `rebase` row's note, which the picture holds but cannot date: the note has to be on the row
    /// as the card is measured, and one arriving a frame later reads the same in a photograph (規約 §メニュー).
    /// **Both halves are runs of their own** — a `pushed=false` alone passes an implementation that never asks, and a
    /// `pushed=true` alone passes one that always says so.
    function openIntegrateMenu(arg) {
        const onTag = arg.endsWith(":tag")
        const name = onTag ? arg.substring(0, arg.length - 4) : arg
        const oid = onTag ? tagsModel.oidOfName(name) : branchesModel.oidOfName(name)
        page.openRefMenu(onTag ? "tag" : "branch", name, name, oid)
        Harness.report("integrate_menu ref=" + name
                          + " offered=" + refRebaseItem.offered
                          + " pushed=" + (refRebaseItem.note !== ""))
    }

    /// Which commit the four reset verbs take the branch back to. With nothing given, the row under HEAD's: a reset
    /// to where the branch already stands moves nothing, so the landing below would never come — and an empty name
    /// would reach git as `reset ''`.
    function resetTarget(arg) {
        return arg === "" ? graphModel.oidAt(graphModel.rowOf(workTree.headOid) + 1)
                          : driver.autoActOid(arg)
    }

    /// Opens the commit these verbs are about, the way a reader who is about to right-click a row opens it. The page
    /// itself opens on the working tree's own row wherever the tree has one (`RepoPage.trySelectDefault`), so a
    /// sampler that waits on the details card (`AutoActDriver.cardSettled`) has nothing to wait for until a run says
    /// which commit it is reading.
    function readCommit(oidHex) {
        const row = graphModel.rowOf(oidHex)
        if (row >= 0)
            page.activateRow(oidHex, row)
    }

    // ---- the picture of a replay that is still replaying -----------------
    // What only exists while git is out: the badge counting the steps out of git's own file, the doors the page holds
    // down, and the ring beside the hand. Every other rebase verb photographs a landing.
    //
    // **This only watches.** The page runs the count's own tick for as long as a replay is out
    // (`Metrics.opProgressMs`), so this waits for the number the way a reader does — and a build whose tick never
    // started, or never reached the badge, waits out the watchdog.
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
            // window's origin, which is a corner.
            const seat = graphPane.mapToItem(null, graphPane.width / 6, graphPane.height / 3)
            win.holdWaitHand(seat.x, seat.y)
            // `counted` alone: the step is whatever git had reached, and what is being claimed is
            // that the badge is counting a range out at all.
            const counted = workTree.opStep > 0 && workTree.opStep <= workTree.opSteps
                            && workTree.opSteps > 1
            Harness.report("replay_running op=" + workTree.opText + " counted=" + counted
                              + " ring=" + win.waitRingShown
                              + " held=" + page.replayRunning
                              + " step=" + workTree.opStep + " steps=" + workTree.opSteps)
            renderedBarrier.begin()
        }
    }
    // The two moments of the landing a stopped operation owes, in one line because neither means anything alone:
    // `owed=` and `early=` are the page **after a pass that beat the status was offered to the landing and turned
    // down** — the move decided and still held, with nothing opened and nothing standing on a copy — and `wip=` is
    // the pass that carries ours landing on this window's own tree. Every copy's row wears the same all-zero id,
    // so a reader that took the id at row 0 for ours resolves the landing onto a copy and the press ends somewhere
    // nobody asked for.
    SampleTimer {
        id: stoppedLandingTimer
        property bool read: false
        /// The passes this page had settled when the one this run asks for was asked for. **The landing is only
        /// ever offered a pass that finishes after it was armed** (`RepoPage.onStatsChanged` →
        /// `tryPendingWipSelect`), so a reading taken before one has is a reading of a question nobody asked yet —
        /// and `owed=` would say the landing is still held however it reads row 0.
        property int owedAt: -1
        property bool owed: false
        property bool early: false
        property bool earlyCopy: false
        /// A run that was never started into the arrangement says so and stops. Both doors into it answer whether
        /// the hold was up, so neither can read the ordinary order as this.
        function refuse() {
            stoppedLandingTimer.stop()
            Harness.report("wip_stop_landing held=false")
            driver.complete()
        }
        onTriggered: {
            // The stop, named by what git left: the press has answered, an operation is standing and the status
            // that carries its conflicted rows has arrived, and the landing it owes is armed.
            if (stoppedLandingTimer.owedAt < 0) {
                if (repoTab.busyCount !== 0 || workTree.opText === "" || !workTree.wipRowStands
                        || graphModel.loading || !page.pageLanding)
                    return
                // **The pass is asked for.** What ordinarily brings the next one is this
                // window's own row appearing, and the hold is what takes that away — a stopped replay moves no
                // branch, so nothing else asks either (measured: the run sat at the pass it opened with while the
                // press answered, the operation stood and the landing waited). Asked here, it lands with every
                // other copy's row and none of ours, which is the pass the landing has to turn down.
                if (!graphModel.walkAgainWhileHeld()) {
                    stoppedLandingTimer.refuse()
                    return
                }
                stoppedLandingTimer.owedAt = graphModel.finishCount
                return
            }
            // That pass, named by what it left standing: it leads with **a neighbour copy's** row
            // (`GraphModel.carriedTop` — the copy the preset stands where this replay stops, so the row it draws is
            // the one the landing would take). A pass with no all-zero row on top puts nothing in front of the
            // misreading, so this reads past it.
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
                              // The row the landing lit: this landing moves the highlight without
                              // activating a row (`tryPendingWipSelect`).
                              + " lit=" + graphPane.view.currentIndex)
            driver.complete()
        }
    }

    // Where an operation that answers at the tip left the reader — one report for the three of them. The write, its
    // refresh and the beat the viewport waits out all have to be behind it, and the picture cannot answer the second
    // half: a row can be selected and still be somewhere nobody can see.
    SampleTimer {
        id: tipLandedTimer
        /// The name this run's own write answered by, taken from the answer that carried
        /// it — **every** answer rewrites the group it comes from (`RepoTab::settle_write`), so a fetch
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
                    || !graphPane.rowOnScreen(row) || page.selectedOid !== workTree.headOid
                    || !driver.cardSettled)
                return
            tipLandedTimer.stop()
            Harness.report(
                "tip_landed follows="
                + (page.selectedOid !== "" && page.selectedOid === workTree.headOid)
                + " onscreen=" + (row >= 0 && graphPane.rowOnScreen(row))
                // Next to the pair above because that is where the harness reads it: the name is what tells the
                // three verbs' own writes from anything else that could have moved the counter (`must_say`).
                + " op=" + tipLandedTimer.answeredOp
                + " head=" + workTree.headOid.substring(0, 8)
                + " selected=" + page.selectedOid.substring(0, 8)
                + " row=" + row)
            driver.complete()
        }
    }
    /// The answer `tipLandedTimer` waits on, taken on the notify: two answers inside
    /// one beat would leave only the later one to be read, and it is the earlier one that says the write was this
    /// run's (app-ui.md §UI 自動化の因果性 — 一瞬だけ立つ状態は signal で観測して latch する). The chain waits on
    /// `writeSeqBefore`: a write that begins and ends between two looks never shows a *rise* of
    /// `busyCount`, and requiring it wedges the run.
    ///
    /// **And out of the answers that notify carried**: one drain empties
    /// the whole queue and notifies once (`RepoTab::write_answers`), so an answer arriving behind this one — the
    /// interval's own fetch, most often — leaves the group describing itself, with the landing nowhere on it.
    ///
    /// `writeAnswerAtTip` is the bridge's own word for "landed, did not stop part-way, and answers at the tip" — the
    /// op names are turned into meanings on that side of it (`RepoTab::settle_write`). A
    /// fetch's answer, a refusal and a stop all leave the arm down, and the run walks into its
    /// watchdog.
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
    // Where taking the branch back leaves the reader. A reset writes no commit, so there is no
    // answer "at the tip" to arm on — what it does is move the name, and **the name arriving on the commit that was
    // asked for is the whole claim**. Waited for by identity: the write answers before the
    // refs it invalidated are published (`session::write::run_write`), so a run that stopped at the write barrier
    // photographs the branch where it stood — which is the picture a build that never reset takes too.
    //
    // Then the graph holding a row for it, and the pane on the right caught up, the way every landing here waits.
    //
    // `files=` is the working tree the mode chose: `--soft` and `--mixed` put the commits' own changes back into it,
    // `--hard` writes over it — so the one report tells the three modes apart, and a `--hard` that quietly kept the
    // tree cannot pass as one that cleared it.
    SampleTimer {
        id: resetLandedTimer
        /// The commit the branch was sent back to, and which flag sent it. Both are the run's own
        /// words: what is being checked is that git did what this verb asked.
        property string target: ""
        property string mode: ""
        /// The number the reset's own answer named for the first report of HEAD after it (`writeAnswerHeadSeq`),
        /// taken on the notify the way `tipLandedTimer` takes its answer; 0 until the answer lands.
        property int armed: 0
        function begin(oidHex, flag) {
            resetLandedTimer.target = oidHex
            resetLandedTimer.mode = flag
            resetLandedTimer.armed = 0
            resetLandedTimer.start()
        }
        onTriggered: {
            // **The write's own status, by the number its answer named.** HEAD is one record, and the refs read moves
            // it ahead of the status that follows (`session::write::run_write` joins the two) — while `--hard` is
            // judged on the tree it left, and a reset to the very commit HEAD is on moves no name at all. So the
            // wait is on the status's own word for which report it stands beside (`WorkTreeModel.statusSeq`), at or
            // above what the reset's answer named: a status counted before the write cannot reach that number.
            if (repoTab.busyCount !== 0 || resetLandedTimer.armed === 0
                    || workTree.headOid !== resetLandedTimer.target
                    || workTree.statusSeq < resetLandedTimer.armed)
                return
            const row = graphModel.rowOf(resetLandedTimer.target)
            if (row < 0 || !driver.cardSettled)
                return
            resetLandedTimer.stop()
            // The wait above is that claim, and a build whose reset never landed
            // never reaches this line at all (it walks into the watchdog). A `moved=true` read back off the same
            // condition could not be false, and a field that cannot be false is one nobody can judge on.
            Harness.report(
                "reset_landed mode=" + resetLandedTimer.mode
                + " files=" + workTree.hardResetTakes
                + " untracked=" + workTree.untrackedCount
                + " head=" + workTree.headOid.substring(0, 8)
                + " row=" + row + " rows=" + graphModel.rowTotal)
            driver.complete()
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
            Harness.report(
                "merge_stopped wip=" + page.wipShown
                + " conflicts=" + (workTree.conflictCount > 0)
                + " error=" + (repoTab.lastError !== "")
                + " log=" + page.commandsOpen
                // The box opened holding what the merge is about to record, and the button under it
                // keeps its one door.
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
    // them it is a step onward (規約 §進行中の操作から出る).
    SampleTimer {
        id: opStoppedTimer
        // Whether a carry is part of this landing. The stash section is refreshed *after* the graph, so reading it
        // at the write barrier answers 0 for a tree whose work is sitting in an entry — and where the entry is the
        // whole claim, that is the answer arriving too early.
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
            Harness.report(
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
            if (Harness.autoAct === "edit-message-focus") {
                detailsPane.focusDescription()
                Harness.report("message_focus pane=details focused="
                                  + detailsPane.descriptionFocused
                                  + " color=" + detailsPane.descriptionColor)
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
            // "edit-message-leave" walks away from the unsaved text. Nothing asks any more — the draft goes and the
            // next commit's own message arrives, which is what the shot is of.
            else if (Harness.autoAct === "edit-message-leave")
                page.activateRow(graphModel.oidAt(graphModel.rowOf(page.selectedOid) + 1))
            if (Harness.autoAct === "reword")
                writeBarrier.start()
            else
                renderedBarrier.begin()
        }
    }
}
