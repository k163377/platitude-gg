pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// Moving HEAD, and what a branch is measured against: switching to a ref, the questions a switch can stop on,
/// and setting an upstream.
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

    /// Runs `act` if it is one of this family's, and says whether it was. The families are asked in turn
    /// and the first to know a verb runs it — no verb is named by two of them (`AutoActDriver`).
    function run(act, arg) {
        if (act === "switch") {
            page.switchTo("branch", arg, arg)
        } else if (act === "move-ask") {
            // The other question a move can raise: landing on a remote branch whose local one holds commits of its
            // own. `move-branch` is the same road past this bar; this one stops on it. Waited on at `AskBar.settled`
            // for the reason `switch-stopped` is — `dbl-remote` completes before the bar has finished opening and
            // photographs a marked row under no bar at all (observed).
            page.switchToRef("R", arg)
            moveAskTimer.start()
        } else if (act === "ask-over-notice") {
            // **The two bars standing at the same time**, which is the one arrangement where Escape has to belong to
            // one of them rather than to both (デザイン規約 §答えの要らない報せ). The report goes up first because that is the
            // order a hand reaches it in: raising a question leaves a standing report exactly where it is
            // (`RepoPage.startRowAsk`). Raised through the page's own door the way `report-tone` is — **which** report
            // it is proves nothing here and the thirteen report verbs prove it already; the pair is the subject.
            page.showReport("update", "origin", "main", "")
            askOverNoticeTimer.ref = arg === "" ? "origin/main" : arg
            askOverNoticeTimer.start()
        } else if (act === "ask-sweep") {
            // The same question `move-ask` raises, swept instead of photographed: what a bar names is a branch, a
            // remote or the folder another working copy is holding, and while it stands over the list it is the only
            // place any of those is written (規約 §右のペインの字は掴める). Raised down the road a hand takes for the
            // reason that verb gives, and waited on at `AskBar.settled` for the same one — a bar still on its way down
            // has its words at some other width, and the air a run samples is the air of a frame nobody sees.
            page.switchToRef("R", arg === "" ? "origin/main" : arg)
            askSweepTimer.start()
        } else if (act === "switch-lands") {
            // A move photographed where it comes to rest. **`switch` cannot do this** — it ends on the write barrier,
            // and core answers a write before the rebuild it asks for (`AfterWrite::Graph`), so that verb's picture is
            // of the branch being left and of the stash count before the carry touched it.
            //
            // The argument is `<branch>[:<stashes>]`, and the count is there because **the branch is not the last
            // thing to arrive**: the stash list is read on its own after the move, so a run that stopped at the branch
            // photographed a carry whose entry was not in the list yet (observed — the row reached the graph after
            // the shot had been taken).
            const landing = arg.split(":")
            switchLandsTimer.branch = landing[0]
            switchLandsTimer.stashes = landing.length > 1 ? Number(landing[1]) : -1
            page.switchToRef("L", landing[0])
            switchLandsTimer.start()
        } else if (act === "switch-stopped" || act === "switch-stopped-go"
                   || act === "switch-conflicted" || act === "switch-conflicted-go"
                   || act === "switch-held") {
            // The question a move raises when something is in its way, and the gesture that answers it. **One road for
            // all three shapes** — an operation standing, an unmerged index with none, and a branch another working
            // copy has out — because the press is the same press; only the bar differs, and the verbs are separate so
            // each shape can be claimed on its own. Entered by the ref row's own road (`switchToRef`) rather than by
            // `switchTo`, so the run proves the gate sits where a hand arrives and not only on the last call before
            // the write. The argument is `<branch>[:<stashes>]`, the count meaning what it does for `switch-lands`.
            const leave = arg.split(":")
            switchStoppedTimer.go = act.endsWith("-go")
            switchStoppedLandedTimer.stashes = leave.length > 1 ? Number(leave[1]) : -1
            page.switchToRef("L", leave[0])
            switchStoppedTimer.start()
        } else if (act === "switch-remote") {
            page.switchToRef("R", arg)
        } else if (act === "switch-remote-twice") {
            // The same chip pressed twice, which is what the report was: both `switch --create` left in the same
            // second and git refused the second one, because the first had already made the branch (observed). **The
            // two presses go out in one turn** — `busyCount` only rises when the queue starts the write, so a run
            // that waited even a tick between them would be answered by the gate the old build had.
            switchTwiceTimer.branch = repoTab.localNameFor(arg)
            switchTwiceTimer.writesBefore = repoTab.writeSeq
            page.switchToRef("R", arg)
            // The road's own answer to the second press, not a copy of its condition: `switchToRef` says whether the
            // press did anything, and a build with no gate says it did.
            switchTwiceTimer.held = page.switchToRef("R", arg) === false
            switchTwiceTimer.start()
        } else if (act === "rename-local-upstream") {
            // The question about carrying the name over comes back only when git says the local rename landed — the
            // write's own answer is what raises the bar, so the completion is the bar settling, not the write barrier
            // (a shot taken there catches a bar whose words are written and whose height is still nothing).
            const local = workTree.branch
            sidebarPane.beginRename("branch", local, local)
            sidebarPane.submitEdit(arg)
            localUpstreamAskTimer.start()
        } else if (act === "set-upstream" || act === "set-upstream-go") {
            // `<branch>[:<name to answer with>]` — `:` cannot be in a ref name (`check-ref-format`), so it separates
            // the two without ambiguity. Without the second half the question stands as it opened, on whatever the
            // branch already speaks for.
            const want = arg.split(":")
            const on = want[0]
            upstreamAskTimer.wantName = want.length > 1 ? want[1] : ""
            upstreamAskTimer.answers = act === "set-upstream-go"
            upstreamAskTimer.typed = false
            // Through the row itself rather than the page's function, so a build where that row stopped reaching the
            // question waits here instead of passing.
            page.openRefMenu("branch", on, on, branchesModel.oidOfName(on))
            refMenu.openSub(refBranchCard)
            refUpstreamItem.triggered()
            upstreamAskTimer.start()
        } else if (act === "switch-mark") {
            // The mark the `switch` row wears when the press ahead of it raises a question rather than moving. The
            // argument is `<branch>:asks` or `<branch>:plain` — **the row is the same row either way**, and a 16px
            // mark in a full window is not something the picture answers (verify-ui §目視).
            const want = arg.split(":")
            switchMarkTimer.want = want.length > 1 ? want[1] : ""
            // The sidebar's row, which is where this menu lives now — the graph's own `switch` row is a row of the
            // commit menu, and `chip-menu` says whether it was offered there.
            page.openRefMenu("branch", want[0], want[0], branchesModel.oidOfName(want[0]))
            switchMarkTimer.start()
        } else if (act === "dbl-local" || act === "dbl-remote") {
            // The record is the chip as drawn (kind letter, four flags, name — see encode.rs).
            page.activateRecord(
                (act === "dbl-local" ? "L00010" : "R00000") + arg)
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
            // No chip on this one — git has no single word for moving a branch onto a ref, so the pill answers in the
            // ordinary voice (規約 §git 用語のコード表記). `code=` being empty is part of the claim.
            Harness.report("move_ask hold=" + graphPane.askHold
                              + " code=" + graphPane.askCode
                              + " branch=" + workTree.branch)
            driver.complete()
        }
    }
    /// PGG_AUTO_ACT=ask-over-notice: the report and the question standing together, and which of the two Escape is
    /// then handed to. **Three beats, and every one of them is a bar that has stopped moving**: the report all the
    /// way down before the question is raised over it (a bar on its way has no shortcut behind it yet — `open` is
    /// what enables one, so the pair would be read a frame before it exists), the question all the way down for the
    /// reason `move-ask` waits, and both of them stopped again after the press.
    ///
    /// **The picture answers none of it.** Two bars with two live Escapes frame exactly like two bars with one, and
    /// a window where Escape does nothing frames like a window where it does the right thing — so all four claims
    /// are in the line, and what the shot is left showing is only the outcome (`ask_over_notice`).
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
        /// Which of the two a pair of answers names — said once so all four halves of the line are spelt the same.
        function naming(ask, notice) {
            return ask && notice ? "both" : ask ? "question" : notice ? "notice" : "none"
        }
        /// A bar between its two ends: neither all the way down nor all the way back up. Nothing about a bar may be
        /// read there — a run that read one mid-flight would be reading a frame nobody sees.
        function moving(card) {
            return !card.settled && !card.shut
        }
        onTriggered: {
            // **Each bar is waited for inside the beat that needs it standing, and nowhere else** (規約 §UI 自動化の因果性):
            // the press below is what takes one of the two away, so a `settled` read every tick would be broken by
            // this verb's own answer and the run would wait out the watchdog with the work already done (measured).
            if (!askOverNoticeTimer.asked) {
                if (!page.noticeCard.settled)
                    return
                askOverNoticeTimer.asked = true
                page.switchToRef("R", askOverNoticeTimer.ref)
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
                // Through the body Escape itself runs, and through **whichever bar is holding it** rather than the
                // one this run expects to: a build that handed Escape to the other bar has to be judged on what that
                // bar then did. A build that handed it to neither presses nothing, and the two below say so.
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
    // The carry-the-upstream question, waited on the way every ask is: the bar has to have finished coming down
    // before its words — or its height — mean anything.
    SampleTimer {
        id: localUpstreamAskTimer
        onTriggered: {
            if (!graphPane.askCard.settled)
                return
            localUpstreamAskTimer.stop()
            Harness.report("rename_upstream_ask hold=" + graphPane.askHold
                              + " branch=" + workTree.branch)
            driver.complete()
        }
    }
    // ...and the same bar's words taken from the air around them: the band inside the bar's own inset, the step
    // between the heading and the line under it, the room beside a short one (規約 §右のペインの字は掴める). Waited on
    // at `settled` for the reason `move-ask` waits — the bar spends 200ms coming down and a run that sampled the air
    // of a half-open one would be reporting on a frame nobody sees. `words=` is what the bar is saying while it is
    // swept, so a green run on an empty bar cannot pass for a green run on a question.
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
            if (repoTab.busyCount !== 0 || workTree.branch !== switchLandsTimer.branch)
                return
            if (switchLandsTimer.stashes >= 0 && stashesModel.total !== switchLandsTimer.stashes)
                return
            switchLandsTimer.stop()
            // `log=` on all three of these: a move that git refused would raise the command log
            // (§git が言ったことを読む場所), and a red panel under a press that had a way out on screen is the thing
            // this whole road exists to stop. A shut panel and a panel that was never
            // raised are the same picture, which is why it is said rather than shown.
            Harness.report("switch_landed branch=" + workTree.branch
                              + " stashes=" + stashesModel.total
                              + " wanted=" + switchLandsTimer.stashes
                              + " conflicts=" + workTree.conflictCount
                              + " log=" + page.commandsOpen)
            driver.complete()
        }
    }
    // The same landing, reached by two presses instead of one. **The claim is `held=`** — the second press turned
    // away — because the two builds frame alike: the branch is the branch either way, and what the ungated one adds
    // is a refused `switch --create` in a panel nobody opened.
    SampleTimer {
        id: switchTwiceTimer
        property string branch: ""
        property bool held: false
        property int writesBefore: 0
        onTriggered: {
            if (repoTab.busyCount !== 0 || workTree.branch !== switchTwiceTimer.branch)
                return
            switchTwiceTimer.stop()
            // `writes=` is the same claim counted from the other side: one press, one answer. It is read after the
            // tree has settled, so a second write would have been counted by now.
            Harness.report("switch_twice held=" + switchTwiceTimer.held
                              + " writes=" + (repoTab.writeSeq - switchTwiceTimer.writesBefore)
                              + " branch=" + workTree.branch
                              + " log=" + page.commandsOpen)
            driver.complete()
        }
    }
    // The bar that comes down instead of the move — waited on all the way down (`AskBar.settled`), not at the label
    // that starts it: the 200ms opening is 200ms of red line with no words in it, and that is what the first run of
    // this verb photographed.
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
                              + " op=" + workTree.opCommand
                              + " branch=" + workTree.branch
                              + " log=" + page.commandsOpen)
            if (!switchStoppedTimer.go) {
                driver.complete()
                return
            }
            // Whichever gesture this shape of the question takes — a held pill reports no click, and a click pill has
            // no hold to run to its end. The bar itself says which it is.
            driver.writeSeqBefore = repoTab.writeSeq
            if (graphPane.askHold)
                graphPane.completeHold()
            else
                page.answerRowAsk()
            switchStoppedLandedTimer.start()
        }
    }
    // Where the answer put the reader, and what it left in the stash list. **The write's own answer is not the edge**
    // — it lands before the rebuild it asks for (core `AfterWrite::Graph`), so a run that read the branch there would
    // photograph the one it was leaving; and the stash list is read after that again (`switch-lands`), which is why
    // the count comes from the argument and is waited for.
    SampleTimer {
        id: switchStoppedLandedTimer
        property int stashes: -1
        onTriggered: {
            if (repoTab.busyCount !== 0 || repoTab.writeSeq <= driver.writeSeqBefore
                    || workTree.opCommand !== "" || !graphPane.askCard.shut)
                return
            if (switchStoppedLandedTimer.stashes >= 0
                    && stashesModel.total !== switchStoppedLandedTimer.stashes)
                return
            switchStoppedLandedTimer.stop()
            Harness.report("switch_stopped_landed branch=" + workTree.branch
                              + " op=" + workTree.opCommand
                              + " conflicts=" + workTree.conflictCount
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
            // `indent=` is the other half: the mark is drawn inside the padding the whole menu carries for it, so a
            // menu that forgot to open that column would draw the `!` over its own edge (`AppMenu.holdIndent`).
            Harness.report("switch_mark offered=" + refSwitchItem.offered
                              + " asks=" + refSwitchItem.asks
                              + " want=" + switchMarkTimer.want
                              + " indent=" + (refMenu.holdIndent > 0))
            driver.complete()
        }
    }
    /// PGG_AUTO_ACT=set-upstream…: the question about what a branch is measured against (`UpstreamFlow`). The bar has
    /// to be all the way down before there is a box to answer into — the form is loaded as the bar opens — and that
    /// is the picture's own moment as well (`AskBar.settled`).
    SampleTimer {
        id: upstreamAskTimer
        /// Whether this run answers the question or only photographs it.
        property bool answers: false
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
            if (upstreamAskTimer.answers && !graphPane.askAnswerable)
                return
            upstreamAskTimer.stop()
            // `there=` is whether this repository actually holds what was answered, which is what decides the pill —
            // and the refused form is the frame and the line, neither of which a full-window picture settles.
            Harness.report("upstream branch=" + upstreamFlow.branch
                              + " remote=" + upstreamFlow.remote
                              + " name=" + upstreamFlow.branchName
                              + " there=" + upstreamFlow.targetIsThere
                              + " answerable=" + graphPane.askAnswerable)
            if (!upstreamAskTimer.answers) {
                driver.complete()
                return
            }
            driver.writeSeqBefore = repoTab.writeSeq
            page.answerRowAsk()
            writeBarrier.start()
        }
    }
}
