pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The verbs that talk to a remote: the first push and the question in front of it, the push defaults, and every
/// way a fetch lands or fails.
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
    readonly property var graphPane: driver.graphPane
    readonly property var publishFlow: driver.publishFlow
    readonly property var remoteDialog: driver.remoteDialog
    readonly property var renderedBarrier: driver.barrierRendered
    readonly property var writeBarrier: driver.barrierWrite

    /// Runs `act` if it is one of this family's, and says whether it was. The families are asked in turn
    /// and the first to know a verb runs it — no verb is named by two of them (`AutoActDriver`).
    function run(act, arg) {
        if (act === "publish" || act === "publish-taken"
                || act === "publish-add" || act === "publish-go"
                || act === "publish-new-go" || act === "publish-remotes"
                || act === "publish-dismiss") {
            // The button's own path, so the state machine in front of the question is exercised too, not just the
            // question.
            page.pushNow()
            if (act === "publish-taken")
                publishFlow.setPublishBranch(arg === "" ? "taken" : arg)
            else if (act === "publish-add" || act === "publish-new-go")
                publishFlow.startPublishAddRemote(arg)
            else if (arg !== "")
                publishFlow.setPublishBranch(arg)
            if (act === "publish-new-go")
                publishNewTimer.start()
            else if (act === "publish-go")
                publishAnswerTimer.start()
            else if (act === "publish-remotes")
                publishRemotesTimer.start()
            else if (act === "publish-dismiss")
                publishDismissTimer.start()
            else if (act === "publish-add")
                publishDialogTimer.start()
            else if (act === "publish")
                publishSurfaceTimer.start()
            else
                publishSettleTimer.start()
            // `dialog=` / `name=` say whether the remote dialog stands and what its name box holds — the no-remote push
            // opens it by itself, and only this line can say so headless.
            Harness.report("publish state=" + page.pushState
                              + " remote=" + publishFlow.publishRemote
                              + " branch=" + publishFlow.publishBranch
                              + " dialog=" + remoteDialog.visible
                              + " name=" + remoteDialog.wantedName)
        } else if (act === "push-target") {
            // Nothing to press: the destination is a binding, and this run is about what it says once the repository
            // it is about has finished arriving.
            pushTargetTimer.start()
        } else if (act === "push-default" || act === "remote-menu" || act === "remote-url"
                   || act === "publish-remotes-marked") {
            // `<remote>`, or `<remote>:marked` to put the mark on it first. All three go in at the same doors a hand
            // uses — the slot the menu row calls, the page's one way into the menu, the flow's one way into the form —
            // so what answers is the wiring rather than a second route written for the run.
            const marked = arg.endsWith(":marked")
            // The destination list has no remote of its own to name, so it takes whichever one this repository would
            // send to — the only name a preset-agnostic run can be sure exists.
            driver.remoteTarget = act === "publish-remotes-marked" ? repoTab.defaultRemote
                                : marked ? arg.substring(0, arg.length - ":marked".length) : arg
            driver.markWanted = marked || act !== "remote-menu" && act !== "remote-url"
                                ? driver.remoteTarget : repoTab.pushDefault
            if (repoTab.pushDefault !== driver.markWanted)
                repoTab.setPushDefault(driver.markWanted)
            if (act === "push-default")
                pushDefaultTimer.start()
            else if (act === "remote-menu")
                remoteMenuTimer.start()
            else if (act === "publish-remotes-marked")
                publishMarkedTimer.start()
            else
                remoteUrlTimer.start()
        } else if (act === "push") {
            page.pushNow()
        } else if (act === "push-outdated") {
            // The same press, against a remote that has moved on since this end last looked (`--preset outrun`).
            // git will not send, and **the next move is already being made** — the session fetches on this refusal —
            // so what comes back is a report rather than an error (デザイン規約 §答えの要らない報せ).
            page.pushNow()
            driver.barrierNotice.start()
        } else if (act === "force-push") {
            page.forcePush()
        } else if (act === "push-retry") {
            // A mark coming off is the absence of a thing, so the timer reports the refused state before sending the go
            // that clears it.
            page.pushNow()
            pushRetryTimer.start()
        } else if (act === "fetch" || act === "fetch-busy") {
            // `-busy` is the same fetch; what differs is who says the run is over — the band, once it has latched the
            // ring (`WindowAutoActDriver`).
            repoTab.fetch("")
        } else if (act === "fetch-ref-list") {
            // What a tag says about the remote only exists after a fetch (`ls-remote --tags` carries it), so the two
            // steps are one verb.
            repoTab.fetch("")
            fetchedRefListTimer.start()
        } else if (act === "fetch-recover") {
            // A fetch that cannot land leaves a failure standing; `Main` then fires one that can and reads what the
            // success takes down by itself.
            repoTab.fetch("pgg-no-such-remote")
        } else if (act === "fetch-recover-held") {
            // The same recovery over a panel the reader put up first, which the fetch may not take down again
            // (デザイン規約 §git が言ったことを読む場所). The press goes in at the seat's own function — the `>_` calls this
            // and nothing else — so a build where the press stopped reaching the rule ends with the panel gone.
            page.toggleCommands()
            repoTab.fetch("pgg-no-such-remote")
        } else if (act === "fetch-fail") {
            // The argument is how many failed fetches to run, so one verb reaches the warning shape and the stopped one
            // alike. The fetches are asked for by `fetchFailTimer`, which is also what ends the run.
            acts.fetchFailRuns = Math.max(1, Number(arg))
            AppBackend.setAutoFetchMinutes(0)
            AppBackend.setAutoFetchMinutes(5)
            fetchFailTimer.start()
        } else if (act === "fetch-resume") {
            // Long enough a run to stop the timer, so the hold has a stopped button to come down on.
            acts.fetchFailRuns = 3
            acts.fetchResumeAfter = true
            AppBackend.setAutoFetchMinutes(0)
            AppBackend.setAutoFetchMinutes(5)
            fetchFailTimer.start()
        } else {
            return false
        }
        return true
    }
    /// PGG_AUTO_ACT=push-default: the mark lands on a remote and the run stops with the sidebar showing it. The write
    /// is the barrier — `pushDefault` only says the name once `git config` has run and the refresh behind it has
    /// republished the snapshot, so a picture taken here is of a repository that really is marked.
    SampleTimer {
        id: pushDefaultTimer
        onTriggered: {
            if (repoTab.pushDefault !== driver.markWanted || repoTab.busyCount > 0)
                return
            pushDefaultTimer.stop()
            // The judged fields lead, and together: `must_say` reads one run of the line, not a set of words in it.
            Harness.report("push_default local=" + repoTab.pushDefaultLocal
                              + " marked=" + repoTab.pushDefault
                              + " target=" + repoTab.defaultRemote
                              + " remotes=" + repoTab.remoteCount)
            driver.complete()
        }
    }
    /// PGG_AUTO_ACT=push-target: where the toolbar says this branch's push is going, and the standing beside it. Both
    /// come off the page the button reads (`RepoPage.pushTargetLabel` / `pushState`), so what answers is the binding
    /// the window uses rather than a second reading written for the run. The marks that decided it ride the same line:
    /// the label alone cannot say **which** of them git would have followed.
    ///
    /// `workTree.loaded` is the barrier — before the first status lands the branch is empty and every mark reads as
    /// unset, which is a destination of nothing rather than an answer.
    SampleTimer {
        id: pushTargetTimer
        onTriggered: {
            if (!workTree.loaded || repoTab.state !== "open" || repoTab.busyCount > 0)
                return
            pushTargetTimer.stop()
            Harness.report("push_target label=" + page.pushTargetLabel
                              + " state=" + page.pushState
                              + " branch_mark=" + workTree.pushRemote
                              + " repo_mark=" + repoTab.pushDefault
                              + " tracks=" + workTree.upstream)
            driver.complete()
        }
    }
    /// PGG_AUTO_ACT=publish-remotes-marked: the first push's destination list with the mark in it. The mark is put on
    /// first and waited for — the question reads the marked remote as it opens, so a list opened before the write
    /// landed would be the one from before.
    SampleTimer {
        id: publishMarkedTimer
        onTriggered: {
            if (repoTab.pushDefault !== driver.markWanted || repoTab.busyCount > 0)
                return
            if (!publishFlow.publishAsking) {
                page.pushNow()
                return
            }
            if (!publishFlow.publishRemotesOpen()) {
                publishFlow.openPublishRemotes()
                return
            }
            publishMarkedTimer.stop()
            Harness.report("publish_remotes open=" + publishFlow.publishRemotesOpen()
                              + " marked=" + publishFlow.publishRemotesMarked()
                              + " name=" + repoTab.pushDefault)
            driver.complete()
        }
    }
    /// PGG_AUTO_ACT=remote-menu: the menu a remote's own row raises, left standing (overlay.png). `rows=` is what it is
    /// offering — two on a remote that is not the destination, one on the remote that already is, since a row with
    /// nothing to do is gone rather than greyed (デザイン規約 §メニュー).
    SampleTimer {
        id: remoteMenuTimer
        onTriggered: {
            if (repoTab.pushDefault !== driver.markWanted || repoTab.busyCount > 0)
                return
            if (!driver.remoteMenu.opened && !page.openRemoteMenu(driver.remoteTarget))
                return
            if (!driver.remoteMenu.opened)
                return
            remoteMenuTimer.stop()
            Harness.report("remote_menu open=" + driver.remoteMenu.opened
                              + " rows=" + driver.remoteMenu.offeredRows
                              + " remote=" + driver.remoteTarget
                              + " marked=" + repoTab.pushDefault)
            driver.complete()
        }
    }
    /// PGG_AUTO_ACT=remote-url: the form that holds a remote's URL, left standing (overlay.png) — the other way to the
    /// mark. `box=` is whether the line is checked, which is the half a picture of a form cannot be trusted for.
    SampleTimer {
        id: remoteUrlTimer
        onTriggered: {
            if (repoTab.pushDefault !== driver.markWanted || repoTab.busyCount > 0)
                return
            if (!driver.remoteDialog.visible) {
                driver.publishFlow.startEditRemote(driver.remoteTarget)
                return
            }
            remoteUrlTimer.stop()
            Harness.report("remote_url dialog=" + driver.remoteDialog.visible
                              + " box=" + driver.remoteDialog.marked
                              + " remote=" + driver.remoteDialog.editing
                              + " local=" + driver.remoteDialog.markLocal)
            driver.complete()
        }
    }
    // The fetch has to land, and its answer reach the chips, before the stacked ones are worth unstacking.
    SampleTimer {
        id: fetchedRefListTimer
        onTriggered: {
            if (repoTab.busyCount !== 0 || repoTab.writeSeq <= driver.writeSeqBefore)
                return
            const stacked = graphPane.view.itemAtIndex(
                Number(Harness.autoActArg))
            if (!stacked)
                return
            fetchedRefListTimer.stop()
            graphPane.view.chipExpandRequested(
                stacked.oid_hex, stacked.index, stacked.chipItem.records, stacked.chipItem)
            renderedBarrier.begin()
        }
    }
    // The refusal has to be back and on the button before the second go is sent, and the report is what says it ever
    // got there — the mark is gone again by the time the screenshot is taken.
    SampleTimer {
        id: pushRetryTimer
        onTriggered: {
            if (!page.pushFailed || repoTab.busyCount !== 0)
                return
            pushRetryTimer.stop()
            Harness.report("push_retry refused=" + page.pushFailed
                              + " branch=" + publishFlow.pushFailBranch)
            driver.writeSeqBefore = repoTab.writeSeq
            page.forcePush()
            writeBarrier.start()
        }
    }
    /// Automation: how long a run of failed fetches the verb asked for, and whether to hold the button that resumes
    /// once it is there.
    property int fetchFailRuns: 0
    property bool fetchResumeAfter: false
    /// The run reached the length it was asked for. Kept apart from the length itself because resuming clears the
    /// count the tab keeps, and a run that read the length again would start a second one over the resumed button.
    property bool fetchRunDone: false
    /// The write this verb's last fetch was asked at, and the run of failures standing when it was asked.
    ///
    /// **A fetch is not admitted the moment it is asked for**: the process is entered a drain later. So between the
    /// drain that finishes one fetch and the drain that admits the next, `busyCount` is 0 and `writeSeq` has already
    /// moved — which reads exactly like a run that has come to rest. `writeBarrier` completed inside that window and
    /// photographed a single failure for every length asked for (measured, `fetch-fail 3`, `fetch-fail 4` and
    /// `fetch-resume` all came back with the warning shape). What is waited for is the answer to the ask, never the
    /// process alone.
    property int fetchAskSeq: -1
    property int fetchAskFails: -1
    /// The stopped button, read at the moment the hold's slot is fired. Resuming takes it down, and what
    /// `fetch-resume` ends on is the button that came back — so the red half is kept here or it is lost.
    property bool fetchStopped: false
    /// The whole of a run of failed fetches and the resume on the end of it. Asking for the next fetch and judging
    /// that the run is over are the same owner, so the two cannot disagree about whether it is.
    SampleTimer {
        id: fetchFailTimer
        onTriggered: {
            // Nothing is read while git is out — the fetch an opening fires comes through `autoFetchRunning`, the
            // rest through `busyCount` — and nothing is read off an ask still waiting for its answer (`fetchAskSeq`).
            if (repoTab.busyCount !== 0 || repoTab.autoFetchRunning)
                return
            if (acts.fetchAskSeq >= 0 && repoTab.writeSeq <= acts.fetchAskSeq)
                return
            if (!acts.fetchRunDone && repoTab.fetchFailures < acts.fetchFailRuns) {
                // A fetch that came back clean is this verb's premise falling over — the remote is reachable — and
                // asking again cannot make it fail. Say so once and let the watchdog end the run: completing here
                // would hand back a picture of a button that never failed (verbs.md §ヘッドレスで色を確かめる時は
                // デモリモートの URL を疑う).
                if (acts.fetchAskFails >= 0 && repoTab.fetchFailures <= acts.fetchAskFails) {
                    fetchFailTimer.stop()
                    Harness.report("fetch_fail reachable=true fails=" + repoTab.fetchFailures)
                    return
                }
                acts.fetchAskFails = repoTab.fetchFailures
                acts.fetchAskSeq = repoTab.writeSeq
                repoTab.fetch("")
                return
            }
            acts.fetchRunDone = true
            if (acts.fetchResumeAfter) {
                acts.fetchResumeAfter = false
                acts.fetchStopped = repoTab.autoFetchSuspended
                acts.fetchAskSeq = repoTab.writeSeq
                // The slot a hold on the stopped button fires (`TopBar` `onHeld`). It clears the run and fetches
                // again by itself, so the ticks after this one wait for that fetch the way they waited for the rest.
                repoTab.resumeAutoFetch()
                return
            }
            fetchFailTimer.stop()
            if (Harness.autoAct === "fetch-resume")
                // What the picture cannot hold: the button was stopped when the hold came down, and a fetch ran
                // again after it. The one it ends on is a button back at work, which is the warning shape — the same
                // picture `fetch-fail 1` takes.
                Harness.report("fetch_resume stopped=" + acts.fetchStopped
                                  + " fetched=" + (repoTab.fetchFailures > 0)
                                  + " fails=" + repoTab.fetchFailures
                                  + " suspended=" + repoTab.autoFetchSuspended)
            else
                Harness.report("fetch_fail stopped=" + repoTab.autoFetchSuspended
                                  + " fails=" + repoTab.fetchFailures
                                  + " wanted=" + acts.fetchFailRuns)
            driver.complete()
        }
    }

    /// The first-push surface is either the standing question (a remote exists) or the add-remote dialog (none does).
    /// Do not wait for a remote check in the latter case: there is no target to check yet.
    SampleTimer {
        id: publishSurfaceTimer
        onTriggered: {
            if (!remoteDialog.visible && !publishFlow.publishChecked)
                return
            publishSurfaceTimer.stop()
            driver.complete()
        }
    }
    /// `publish-remotes` is about the popup, not merely the call which requested it. The form is created asynchronously
    /// with the ask bar.
    SampleTimer {
        id: publishRemotesTimer
        onTriggered: {
            if (!publishFlow.publishRemotesOpen()) {
                publishFlow.openPublishRemotes()
                return
            }
            publishRemotesTimer.stop()
            driver.complete()
        }
    }
    /// `publish-dismiss` is about what the bar wears on the way back up, which no picture of this run can hold: the
    /// 200ms it spends going is over long before the shot, and a bar that turned `warning` and empty for the whole of
    /// it frames exactly like one that kept its question.
    ///
    /// The ✕ is pressed through the bar's own handler once the question is both dressed (`publishChecked` — the
    /// remote has answered, so the frame and the pill's word are settled) and all the way down, and the line is read
    /// in the same turn: that is the frame a reader is looking at. `shut=false` is what says the reading was taken
    /// while the bar was still on screen — a line read after it had gone would be about nothing.
    SampleTimer {
        id: publishDismissTimer
        onTriggered: {
            if (!publishFlow.publishChecked || !graphPane.askCard.settled)
                return
            publishDismissTimer.stop()
            graphPane.askCard.dismiss()
            Harness.report("ask_dismissed shut=" + graphPane.askCard.shut
                              + " words=" + (graphPane.askCard.label !== "")
                              + " detail=" + (graphPane.askDetail !== "")
                              + " code=" + graphPane.askCode
                              + " neutral=" + graphPane.askNeutral)
            publishGoneTimer.start()
        }
    }
    /// …and the picture is taken once it really has gone, so the run is not photographing a bar caught half way.
    SampleTimer {
        id: publishGoneTimer
        onTriggered: {
            if (!graphPane.askCard.shut)
                return
            publishGoneTimer.stop()
            driver.complete()
        }
    }
    /// `publish-add` stops with the real dialog on screen. A check that happens to finish behind it is unrelated and
    /// must not end the run.
    SampleTimer {
        id: publishDialogTimer
        onTriggered: {
            if (!remoteDialog.visible)
                return
            publishDialogTimer.stop()
            driver.complete()
        }
    }
    /// Automation: the dialog's own button, once it is both visible and valid. This is the `-go` path; an empty URL
    /// cannot be submitted.
    SampleTimer {
        id: publishNewTimer
        onTriggered: {
            if (!remoteDialog.visible || remoteDialog.wantedName === "" || remoteDialog.wantedUrl === "")
                return
            publishNewTimer.stop()
            driver.writeSeqBefore = repoTab.writeSeq
            remoteDialog.submit()
            publishAnswerTimer.start()
        }
    }
    /// Automation: what the far side turned out to hold, once the remote has had time to answer.
    SampleTimer {
        id: publishSettleTimer
        onTriggered: {
            if (!publishFlow.publishChecked)
                return
            publishSettleTimer.stop()
            Harness.report("publish settled far="
                                       + publishFlow.publishState
                                       + " code=" + graphPane.askCode
                                       + " hold=" + graphPane.askHold
                                       + " alert=" + graphPane.askAlert
                                       + " lease=" + (publishFlow.publishLease !== "")
                                       + " theirs=" + repoTab.remoteBranchTheirs)
            driver.complete()
        }
    }
    /// Automation: the answer, given after the remote has had time to say what it has — the pill is dead until it has.
    SampleTimer {
        id: publishAnswerTimer
        onTriggered: {
            if (!publishFlow.publishChecked || !graphPane.askAnswerable)
                return
            publishAnswerTimer.stop()
            // `far` is what the far side turned out to hold — the other line's `state` is this end's own push state,
            // and the two answer different questions.
            Harness.report("publish answering far="
                              + publishFlow.publishState
                              + " unsure=" + publishFlow.publishUnsure
                              + " answerable=" + graphPane.askAnswerable)
            // The same gesture a person is given: a hold cannot be answered by a click here either. The hold's end
            // is the press the write barrier is armed on (`holdToEnd`), said by the pane when the pill confirms.
            driver.writeSeqBefore = repoTab.writeSeq
            if (publishFlow.publishRefused)
                driver.holdToEnd(graphPane, graphPane.askConfirmed)
            else
                page.answerRowAsk()
            writeBarrier.start()
        }
    }
}
