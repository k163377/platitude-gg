pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// Moving HEAD, and what a branch is measured against: switching to a ref, the questions a switch can stop on,
/// and setting an upstream. Built by `AutoActDriver` only when a verb was given.
// An `Item` only because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    /// `var`, because naming the driver's type would be a circle: it is the file that builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var repoTab: driver.repoTab
    readonly property var worktree: driver.worktree
    readonly property var branchesModel: driver.branchesModel
    readonly property var stashesModel: driver.stashesModel
    readonly property var graphPane: driver.graphPane
    readonly property var sidebarPane: driver.sidebarPane
    readonly property var refMenu: driver.refMenu
    readonly property var refBranchCard: driver.refBranchCard
    readonly property var refUpstreamItem: driver.refUpstreamItem
    readonly property var refSwitchItem: driver.refSwitchItem
    readonly property var upstreamFlow: driver.upstreamFlow
    readonly property var writeBarrier: driver.barrierWrite

    /// Runs `act` if it is this family's and says whether it was; each verb belongs to one family (`AutoActDriver`).
    function run(act, arg) {
        if (act === "switch") {
            page.switchTo("branch", arg, arg)
        } else if (act === "move-ask") {
            // The question a move raises landing on a remote branch whose local one holds commits of its own
            // (`move-branch` is the same road past the bar). Waited on at `AskBar.settled` like `switch-stopped`:
            // `dbl-remote` completes before the bar has opened.
            page.switchToRef("remote", arg)
            moveAskTimer.start()
        } else if (act === "ask-over-notice") {
            // Both bars standing at once, the one arrangement where Escape has to pick one
            // (デザイン規約 §答えの要らない報せ). The report goes up first, the order a hand meets them in
            // (`RepoPage.startRowAsk` leaves a standing report where it is); which report it is does not matter.
            page.showReport("update", "origin", "main", "")
            askOverNoticeTimer.ref = arg === "" ? "origin/main" : arg
            askOverNoticeTimer.start()
        } else if (act === "ask-sweep") {
            // The `move-ask` question, swept: while the bar stands over the list it is the only place its branch,
            // remote or folder is written (規約 §右のペインの字は掴める).
            page.switchToRef("remote", arg === "" ? "origin/main" : arg)
            askSweepTimer.start()
        } else if (act === "switch-lands") {
            // A move photographed where it comes to rest. `switch` ends on the write barrier, which core answers
            // before the rebuild it asks for (`AfterWrite::Graph`), so its picture is of the branch being left.
            // The argument is `<branch>[:<stashes>]`: the stash list is read on its own after the move and arrives
            // last, so its count is waited for too.
            const landing = arg.split(":")
            switchLandsTimer.branch = landing[0]
            switchLandsTimer.stashes = landing.length > 1 ? Number(landing[1]) : -1
            page.switchToRef("branch", landing[0])
            switchLandsTimer.start()
        } else if (act === "switch-held") {
            // A branch another worktree holds: the press stands the tab in that worktree and raises nothing
            // (`offers::SwitchAction::OpenHolder`). The worktree is read before the press — after it this page is
            // that worktree and would answer with itself. Entered by `switchToRef`: the claim is that this road ends
            // in no bar.
            switchHeldTimer.wanted = sidebarPane.worktreesModel.worktreeHolding(arg)
            page.switchToRef("branch", arg)
            switchHeldTimer.start()
        } else if (act === "switch-stopped" || act === "switch-stopped-go"
                   || act === "switch-conflicted" || act === "switch-conflicted-go") {
            // The question a move raises when something is in its way (an operation standing, or an unmerged index),
            // and with "-go" the gesture that answers it; only the bar differs. Entered by `switchToRef`, where a
            // hand arrives. The argument is `<branch>[:<stashes>]`, as for `switch-lands`.
            const leave = arg.split(":")
            switchStoppedTimer.go = act.endsWith("-go")
            switchStoppedLandedTimer.stashes = leave.length > 1 ? Number(leave[1]) : -1
            page.switchToRef("branch", leave[0])
            switchStoppedTimer.start()
        } else if (act === "switch-remote") {
            page.switchToRef("remote", arg)
        } else if (act === "switch-remote-twice") {
            // The same chip pressed twice: ungated, the second `switch --create` is refused (the branch exists).
            // Both presses go out in one turn — `busyCount` only rises once the queue starts the write, so a tick
            // between them would be turned away by the busy gate and prove nothing.
            switchTwiceTimer.branch = repoTab.localNameFor(arg)
            switchTwiceTimer.writesBefore = repoTab.writeSeq
            page.switchToRef("remote", arg)
            // `switchToRef` answers whether the press did anything; an ungated build says it did.
            switchTwiceTimer.held = page.switchToRef("remote", arg) === false
            switchTwiceTimer.start()
        } else if (act === "rename-local-upstream" || act === "rename-local-upstream-go"
                   || act === "rename-local-upstream-tip") {
            // What the remote does with the new name. The bar comes down once git says the local rename landed, so
            // the run completes on the bar settling — at the write barrier its height is still nothing.
            // Argument `<新しい名前>[:<選ぶ答え>]` (`replace` / `add` / `leave`; none = as it opens). "-go" answers it.
            const want = arg.split(":")
            const goes = act.endsWith("-go")
            // "-go" with no answer named takes `add`: `replace` would delete the remote's HEAD branch, which git
            // refuses (verbs.md `rename-local-upstream`). "-tip" points at the pill instead; only `replace` has a tip.
            const points = act.endsWith("-tip")
            localUpstreamAskTimer.pick = points ? "replace" : want.length > 1 ? want[1] : (goes ? "add" : "")
            localUpstreamAskTimer.answers = goes
            localUpstreamAskTimer.points = points
            localUpstreamAskTimer.pointed = false
            const local = worktree.branch
            sidebarPane.beginRename("branch", local, local)
            sidebarPane.submitEdit(want[0])
            localUpstreamAskTimer.start()
        } else if (act === "set-upstream" || act === "set-upstream-go" || act === "set-upstream-list"
                   || act === "set-upstream-enter") {
            // `<branch>[:<name to answer with>]` (`:` cannot be in a ref name); without a name the question stands
            // as it opened.
            const want = arg.split(":")
            const on = want[0]
            upstreamAskTimer.wantName = want.length > 1 ? want[1] : ""
            upstreamAskTimer.answers = act === "set-upstream-go" || act === "set-upstream-enter"
            upstreamAskTimer.byKey = act === "set-upstream-enter"
            upstreamAskTimer.lists = act === "set-upstream-list"
            upstreamAskTimer.typed = false
            upstreamAskTimer.dropped = false
            // Through the row itself, so a build whose row stopped reaching the question stalls here.
            page.openRefMenu("branch", on, on, branchesModel.oidOfName(on))
            refMenu.openSub(refBranchCard)
            refUpstreamItem.triggered()
            upstreamAskTimer.start()
        } else if (act === "switch-mark") {
            // The mark the `switch` row wears when its press would raise a question. Argument `<branch>:asks|plain`;
            // the report judges it — a 16px mark in a full window is not for the picture (verify-ui「目視は等倍以上で」).
            const want = arg.split(":")
            switchMarkTimer.want = want.length > 1 ? want[1] : ""
            // The sidebar's menu; the graph's `switch` row belongs to the commit menu (`chip-menu`).
            page.openRefMenu("branch", want[0], want[0], branchesModel.oidOfName(want[0]))
            switchMarkTimer.start()
        } else if (act === "dbl-local" || act === "dbl-remote") {
            // The chip as drawn (`encode::Chip`); only its kind and name are read on this road.
            page.activateChip({ "kind": act === "dbl-local" ? "branch" : "remote", "name": arg })
        } else {
            return false
        }
        return true
    }
    SampleTimer {
        id: moveAskTimer
        onTriggered: {
            if (!graphPane.askCard.settled)
                return
            moveAskTimer.stop()
            // `code=` empty is part of the claim: git has no single word for this move (規約 §git 用語のコード表記).
            Harness.report("move_ask hold=" + graphPane.askHold
                              + " code=" + graphPane.askCode
                              + " branch=" + worktree.branch)
            driver.complete()
        }
    }
    /// Three beats, each on bars that have stopped moving: the report settled before the question is raised (a bar
    /// on its way has no Escape shortcut yet — `open` enables it), the question settled, and both after the press.
    /// The picture cannot tell one live Escape from two, so all four claims are in the line.
    SampleTimer {
        id: askOverNoticeTimer
        /// The remote ref the question is raised on, as `move-ask` takes it.
        property string ref: ""
        property bool asked: false
        property bool pressed: false
        /// What was read at the press, kept because the press is what takes one of the two away.
        property string stood: ""
        property string holds: ""
        property bool askStood: false
        property bool noticeStood: false
        function naming(ask, notice) {
            return ask && notice ? "both" : ask ? "question" : notice ? "notice" : "none"
        }
        /// A bar is read only at one of its ends; mid-flight is a frame nobody sees.
        function moving(card) {
            return !card.settled && !card.shut
        }
        onTriggered: {
            // Each bar is waited for only inside the beat that needs it (app-ui.md §UI 自動化): the press takes one
            // away, so a `settled` read on every tick would wait out the watchdog.
            if (!askOverNoticeTimer.asked) {
                if (!page.noticeCard.settled)
                    return
                askOverNoticeTimer.asked = true
                page.switchToRef("remote", askOverNoticeTimer.ref)
                return
            }
            if (!askOverNoticeTimer.pressed) {
                if (!graphPane.askCard.settled)
                    return
                askOverNoticeTimer.pressed = true
                askOverNoticeTimer.askStood = graphPane.askCard.open
                askOverNoticeTimer.noticeStood = page.noticeCard.open
                askOverNoticeTimer.stood = askOverNoticeTimer.naming(askOverNoticeTimer.askStood,
                                                                     askOverNoticeTimer.noticeStood)
                const askEsc = graphPane.askCard.escapes
                const noticeEsc = page.noticeCard.escapes
                askOverNoticeTimer.holds = askOverNoticeTimer.naming(askEsc, noticeEsc)
                // What Escape itself runs, on whichever bar holds it: a build that handed it to the wrong bar is
                // judged on what that bar did, and one that handed it to neither presses nothing.
                if (askEsc)
                    graphPane.askCard.dismiss()
                else if (noticeEsc)
                    page.noticeCard.dismiss()
                return
            }
            if (askOverNoticeTimer.moving(graphPane.askCard) || askOverNoticeTimer.moving(page.noticeCard))
                return
            askOverNoticeTimer.stop()
            Harness.report("ask_over_notice stood=" + askOverNoticeTimer.stood
                              + " holds=" + askOverNoticeTimer.holds
                              + " went=" + askOverNoticeTimer.naming(
                                  askOverNoticeTimer.askStood && !graphPane.askCard.open,
                                  askOverNoticeTimer.noticeStood && !page.noticeCard.open)
                              + " left=" + askOverNoticeTimer.naming(graphPane.askCard.open, page.noticeCard.open))
            driver.complete()
        }
    }
    // The carry-the-upstream question, read once the bar has settled.
    SampleTimer {
        id: localUpstreamAskTimer
        /// Which of the three answers the run picks, empty for the bar as it comes down.
        property string pick: ""
        /// Whether the picked answer is then given to the bar.
        property bool answers: false
        /// The chooser is answered once; the beats after it are the bar re-dressing around the choice.
        property bool picked: false
        /// Whether the report line has gone out.
        property bool said: false
        /// Whether a hand goes on the pill (`-tip`) — a beat after the pick, so the tip sits against the picked word.
        property bool points: false
        /// Whether it has gone on — said as it is taken.
        property bool pointed: false
        onTriggered: {
            if (!localUpstreamAskTimer.said) {
                if (!graphPane.askCard.settled)
                    return
                if (localUpstreamAskTimer.pick !== "" && !localUpstreamAskTimer.picked) {
                    // Through the field's own door (`RenameCarryFlow.pickChoice` — no injected click opens a popup
                    // offscreen), re-applied until it takes: the form's `Loader` is a frame behind the bar.
                    if (!page.pickCarryChoice(driver.carryChoiceIndex(localUpstreamAskTimer.pick)))
                        return
                    localUpstreamAskTimer.picked = true
                    return
                }
                if (localUpstreamAskTimer.points && !graphPane.askCard.tipStanding) {
                    // Put back on every beat until the tip stands: it is the hand the tip opens for.
                    graphPane.askCard.pointedAt = true
                    if (!localUpstreamAskTimer.pointed) {
                        localUpstreamAskTimer.pointed = true
                        // For a run that stalls at the tip: `pill=` is the answer worn, `words=` whether it has a tip.
                        Harness.report("rename_carry step=pointed pill=" + graphPane.askCard.accept
                                       + " words=" + (graphPane.askCard.tip !== ""))
                    }
                    return
                }
                localUpstreamAskTimer.said = true
                Harness.report(driver.carryWords("branch", localUpstreamAskTimer.pick)
                               + (localUpstreamAskTimer.points ? " tip=" + graphPane.askCard.tipStanding : ""))
                if (!localUpstreamAskTimer.answers) {
                    localUpstreamAskTimer.stop()
                    driver.complete()
                    return
                }
                // Held or clicked, as the bar asks.
                if (graphPane.askHold) {
                    driver.holdToEnd(graphPane)
                } else if (localUpstreamAskTimer.pick === "leave") {
                    // `leave` writes nothing: the beats below wait for the bar to go back up, not for a barrier.
                    page.answerRowAsk()
                    return
                } else {
                    driver.pressWrite("answer-ask", () => { page.answerRowAsk(); return true })
                }
                localUpstreamAskTimer.stop()
                writeBarrier.start()
                return
            }
            if (!graphPane.askCard.shut)
                return
            localUpstreamAskTimer.stop()
            driver.complete()
        }
    }
    // Swept only once settled — on its way down the bar's words are at another width. `words=` keeps a green run
    // on an empty bar from passing for one on a question.
    SampleTimer {
        id: askSweepTimer
        onTriggered: {
            if (!graphPane.askCard.settled)
                return
            askSweepTimer.stop()
            Harness.report("ask_sweep "
                + graphPane.askCard.pad.sweepAir(7, "words=" + (graphPane.askCard.label !== "")))
            driver.complete()
        }
    }
    SampleTimer {
        id: switchLandsTimer
        property string branch: ""
        property int stashes: -1
        onTriggered: {
            if (repoTab.busyCount !== 0 || worktree.branch !== switchLandsTimer.branch)
                return
            if (switchLandsTimer.stashes >= 0 && stashesModel.total !== switchLandsTimer.stashes)
                return
            switchLandsTimer.stop()
            // `log=`: a refused move raises the command log (デザイン規約 §git が言ったことを読む場所), and a shut
            // panel frames like one never raised.
            Harness.report("switch_landed branch=" + worktree.branch
                              + " stashes=" + stashesModel.total
                              + " wanted=" + switchLandsTimer.stashes
                              + " conflicts=" + worktree.conflictCount
                              + " log=" + page.commandsOpen)
            driver.complete()
        }
    }
    // The claim is `held=` (the second press turned away): gated and ungated builds frame alike.
    SampleTimer {
        id: switchTwiceTimer
        property string branch: ""
        property bool held: false
        property int writesBefore: 0
        onTriggered: {
            if (repoTab.busyCount !== 0 || worktree.branch !== switchTwiceTimer.branch)
                return
            switchTwiceTimer.stop()
            // `writes=` counts the same claim from the write side, read once the tree has settled.
            Harness.report("switch_twice held=" + switchTwiceTimer.held
                              + " writes=" + (repoTab.writeSeq - switchTwiceTimer.writesBefore)
                              + " branch=" + worktree.branch
                              + " log=" + page.commandsOpen)
            driver.complete()
        }
    }
    // The page stays through the restand (`Hub::restand_tab`), so this driver reads the answer. Either end ends the
    // run: a build that raised a bar instead never moves the tab, and `bar=` is what it fails on.
    SampleTimer {
        id: switchHeldTimer
        property string wanted: ""
        onTriggered: {
            const stood = switchHeldTimer.wanted !== ""
                && repoTab.repoPath.toLowerCase() === switchHeldTimer.wanted.toLowerCase()
            const barUp = graphPane.askCard.settled
            if (!barUp && !(stood && PageSettled.settled(page)))
                return
            switchHeldTimer.stop()
            Harness.report("switch_held stood=" + stood
                              + " bar=" + barUp
                              + " wanted=" + switchHeldTimer.wanted
                              + " where=" + repoTab.repoPath)
            driver.complete()
        }
    }
    // Waited on all the way down (`AskBar.settled`): while it opens, the bar is a red line with no words in it.
    SampleTimer {
        id: switchStoppedTimer
        property bool go: false
        onTriggered: {
            if (!graphPane.askCard.settled)
                return
            switchStoppedTimer.stop()
            Harness.report("switch_stopped code=" + graphPane.askCode
                              + " accept=" + graphPane.askCard.accept
                              + " hold=" + graphPane.askHold
                              + " bang=" + graphPane.askAlert
                              + " op=" + worktree.opCommand
                              + " branch=" + worktree.branch
                              + " log=" + page.commandsOpen)
            if (!switchStoppedTimer.go) {
                driver.complete()
                return
            }
            // Held or clicked, as the bar asks — a held pill ignores a click, a click pill has no hold. Each way in
            // arms its own watch.
            if (graphPane.askHold)
                driver.holdToEnd(graphPane)
            else
                driver.pressWrite("answer-ask", () => {
                    page.answerRowAsk()
                    return true
                })
            switchStoppedLandedTimer.start()
        }
    }
    // Where the answer put the reader: past the write's answer, and the stash count waited for (`switch-lands`).
    SampleTimer {
        id: switchStoppedLandedTimer
        property int stashes: -1
        onTriggered: {
            if (!driver.wroteAndSettled()
                    || worktree.opCommand !== "" || !graphPane.askCard.shut)
                return
            if (switchStoppedLandedTimer.stashes >= 0
                    && stashesModel.total !== switchStoppedLandedTimer.stashes)
                return
            switchStoppedLandedTimer.stop()
            Harness.report("switch_stopped_landed branch=" + worktree.branch
                              + " op=" + worktree.opCommand
                              + " conflicts=" + worktree.conflictCount
                              + " stashes=" + stashesModel.total
                              + " wanted=" + switchStoppedLandedTimer.stashes
                              + " log=" + page.commandsOpen)
            driver.complete()
        }
    }
    SampleTimer {
        id: switchMarkTimer
        property string want: ""
        onTriggered: {
            if (!refMenu.opened)
                return
            switchMarkTimer.stop()
            // `indent=`: without the menu's hold column the `!` draws over the menu's edge (`AppMenu.holdIndent`).
            Harness.report("switch_mark offered=" + refSwitchItem.offered
                              + " asks=" + refSwitchItem.asks
                              + " want=" + switchMarkTimer.want
                              + " indent=" + (refMenu.holdIndent > 0))
            driver.complete()
        }
    }
    /// `set-upstream…` (`UpstreamFlow`): the form loads as the bar opens, so nothing is answered before
    /// `AskBar.settled`.
    SampleTimer {
        id: upstreamAskTimer
        property bool answers: false
        /// Answers with Enter in the name box instead of the pill (`UpstreamFlow.enterBranch`).
        property bool byKey: false
        /// Drops the name box's list first, through the press's own door (`UpstreamFlow.openBranches`) — no
        /// injected click reaches a popup offscreen.
        property bool lists: false
        property bool dropped: false
        property string wantName: ""
        property bool typed: false
        onTriggered: {
            if (!graphPane.askCard.settled)
                return
            if (upstreamAskTimer.wantName !== "" && !upstreamAskTimer.typed) {
                upstreamFlow.setBranchName(upstreamAskTimer.wantName)
                upstreamAskTimer.typed = true
                return
            }
            if (upstreamAskTimer.lists) {
                if (!upstreamAskTimer.dropped) {
                    // Marked only if the door was there; a form without its name box yet is asked again next tick.
                    upstreamAskTimer.dropped = upstreamFlow.openBranches()
                    return
                }
                // The popup opens a turn after the press (`AppCombo.pressField`), so the popup itself is read.
                if (!upstreamFlow.branchesOpen())
                    return
            }
            if (upstreamAskTimer.answers && !graphPane.askAnswerable)
                return
            upstreamAskTimer.stop()
            // `there=` is whether the answered name exists here — it drives the bar's line, not the pill (a name not
            // here is an answer: デザイン規約 §ブランチが測られる相手を決める). `rows=` / `open=`: an empty list and
            // an unplumbed one frame alike.
            Harness.report("upstream branch=" + upstreamFlow.branch
                              + " remote=" + upstreamFlow.remote
                              + " name=" + upstreamFlow.branchName
                              + " there=" + upstreamFlow.targetIsThere
                              + " answerable=" + graphPane.askAnswerable
                              + " rows=" + upstreamFlow.branches.length
                              + " open=" + upstreamFlow.branchesOpen())
            if (!upstreamAskTimer.answers) {
                driver.complete()
                return
            }
            driver.pressWrite("answer-ask", () => {
                if (upstreamAskTimer.byKey)
                    return upstreamFlow.enterBranch()
                page.answerRowAsk()
                return true
            })
            writeBarrier.start()
        }
    }
}
