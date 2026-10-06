pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The verbs that talk to a remote: the first push and the question in front of it, the push defaults, and every
/// way a fetch lands or fails.
// `Item` because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    /// `var`: naming its type would be a cycle — the driver is the file that builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var repoTab: driver.repoTab
    readonly property var workTree: driver.workTree
    readonly property var graphPane: driver.graphPane
    readonly property var publishFlow: driver.publishFlow
    readonly property var remoteDialog: driver.remoteDialog
    readonly property var renderedBarrier: driver.barrierRendered
    readonly property var writeBarrier: driver.barrierWrite
    readonly property var refList: driver.refList

    /// The card's tags that opened on the line saying they stand apart (`RowHoverHost.mateOf`), and the words of the
    /// first — what the colour in the picture cannot say: which reading it was weighed against.
    function apartWords() {
        const names = []
        let says = ""
        for (let i = 0; i < acts.refList.records.length; i++) {
            const mate = i < acts.refList.mates.length ? acts.refList.mates[i] : null
            if (acts.refList.records[i].kind !== "tag" || mate === null)
                continue
            names.push(acts.refList.records[i].name)
            if (says === "")
                says = mate.text
        }
        return "apart=" + names.join(",") + " says=" + says
    }
    /// The same for one tag of the card (`<row>:<tag>`): whether the card holds it at all, and its line's words.
    function tagWords(tag) {
        for (let i = 0; i < acts.refList.records.length; i++) {
            const rec = acts.refList.records[i]
            if (rec.kind !== "tag" || rec.name !== tag)
                continue
            const mate = i < acts.refList.mates.length ? acts.refList.mates[i] : null
            return "tag=" + tag + " held=true says=" + (mate === null ? "" : mate.text)
        }
        return "tag=" + tag + " held=false says="
    }

    /// Runs `act` if it is one of this family's, and says whether it was.
    function run(act, arg) {
        if (act === "publish" || act === "publish-taken" || act === "publish-tip"
                || act === "publish-add" || act === "publish-go" || act === "publish-enter"
                || act === "publish-new-go" || act === "publish-remotes"
                || act === "publish-dismiss") {
            // The button's own path, so the state machine in front of the question is exercised too.
            page.pushNow()
            if (act === "publish-taken" || act === "publish-tip")
                publishFlow.setPublishBranch(arg === "" ? "taken" : arg)
            else if (act === "publish-add" || act === "publish-new-go")
                publishFlow.startPublishAddRemote(arg)
            else if (arg !== "")
                publishFlow.setPublishBranch(arg)
            if (act === "publish-new-go")
                publishNewTimer.start()
            else if (act === "publish-go" || act === "publish-enter") {
                publishAnswerTimer.byKey = act === "publish-enter"
                publishAnswerTimer.start()
            }
            else if (act === "publish-remotes")
                publishRemotesTimer.start()
            else if (act === "publish-dismiss")
                publishDismissTimer.start()
            else if (act === "publish-add")
                publishDialogTimer.start()
            else if (act === "publish")
                publishSurfaceTimer.start()
            else if (act === "publish-tip")
                publishTipTimer.start()
            else
                publishSettleTimer.start()
            // `dialog=` / `name=`: the no-remote push opens the remote dialog by itself, and only this line can say so
            // headless.
            Harness.report("publish state=" + page.pushState
                              + " remote=" + publishFlow.publishRemote
                              + " branch=" + publishFlow.publishBranch
                              + " dialog=" + remoteDialog.visible
                              + " name=" + remoteDialog.wantedName)
        } else if (act === "publish-upstream") {
            // A push with an upstream the far side does not have yet (デザイン規約 §ブランチが測られる相手を決める):
            // the question the toolbar's press raises must open on that name, or the answer just given is asked again.
            // The upstream goes in at the slot the question's pill calls — the question is `set-upstream-go`'s subject.
            publishUpstreamTimer.want = arg === "" ? "brand-new" : arg
            repoTab.setUpstream(workTree.branch, repoTab.defaultRemote, publishUpstreamTimer.want)
            publishUpstreamTimer.start()
        } else if (act === "push-target") {
            pushTargetTimer.start()
        } else if (act === "push-hover") {
            pushHoverTimer.start()
        } else if (act === "push-default" || act === "remote-menu" || act === "remote-url"
                   || act === "publish-remotes-marked") {
            // `<remote>`, or `<remote>:marked` to put the mark on it first. Each goes in at the door a hand uses (the
            // menu row's slot, the page's way into the menu, the flow's way into the form).
            const marked = arg.endsWith(":marked")
            // The destination list names no remote, so it takes the one this repository sends to — the only name a
            // preset-agnostic run can be sure exists.
            driver.remoteTarget = act === "publish-remotes-marked" ? repoTab.defaultRemote
                                : marked ? arg.substring(0, arg.length - ":marked".length) : arg
            driver.markWanted = marked || act !== "remote-menu" && act !== "remote-url"
                                ? driver.remoteTarget : repoTab.markedOrigin
            if (repoTab.markedOrigin !== driver.markWanted)
                repoTab.markOrigin(driver.markWanted)
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
            // Against a remote that has moved on (`--preset outrun`): git refuses and the session fetches on the
            // refusal, so what comes back is a report (デザイン規約 §答えの要らない報せ).
            page.pushNow()
            driver.barrierNotice.start()
        } else if (act === "force-push") {
            page.forcePush()
        } else if (act === "push-retry") {
            page.pushNow()
            pushRetryTimer.start()
        } else if (act === "fetch" || act === "fetch-busy") {
            // `-busy` is the same fetch; what differs is who says the run is over — the band, once it has latched the
            // ring (`WindowAutoActDriver`).
            repoTab.fetch("")
        } else if (act === "fetch-ref-list") {
            // What a tag says about the remote exists only after a fetch (`ls-remote --tags`), so the two are one verb.
            repoTab.fetch("")
            fetchedRefListTimer.start()
        } else if (act === "fetch-recover") {
            // A fetch that cannot land leaves a failure standing; `Main` then fires one that can and reads what the
            // success takes down by itself.
            repoTab.fetch("pgg-no-such-remote")
        } else if (act === "fetch-recover-held") {
            // The same recovery over a log panel the reader opened, which the fetch leaves standing
            // (デザイン規約 §git が言ったことを読む場所). Opened by the `>_`'s own function, so a build where the press
            // stopped reaching the rule ends with the panel gone.
            page.toggleCommands()
            repoTab.fetch("pgg-no-such-remote")
        } else if (act === "fetch-fail") {
            // The argument is how many fetches to fail, so one verb reaches the warning shape and the stopped one.
            acts.fetchFailRuns = Math.max(1, Number(arg))
            AppBackend.setAutoFetchMinutes(0)
            AppBackend.setAutoFetchMinutes(5)
            fetchFailTimer.start()
        } else if (act === "fetch-hover") {
            // A hand on the button's three live shapes: the argument is how many fetches to fail first — none, one,
            // or enough to stop the timer.
            acts.fetchFailRuns = Math.max(0, Number(arg))
            acts.fetchPointAfter = true
            if (acts.fetchFailRuns > 0) {
                AppBackend.setAutoFetchMinutes(0)
                AppBackend.setAutoFetchMinutes(5)
            }
            fetchFailTimer.start()
        } else if (act === "fetch-resume") {
            // Long enough a run to stop the timer, so the press has a stopped button to land on.
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
    /// PGG_AUTO_ACT=push-default: stops with the sidebar showing the mark. `markedOrigin` names it only once both
    /// `git config` writes ran and the refresh behind them republished, so the picture is of a marked repository.
    SampleTimer {
        id: pushDefaultTimer
        onTriggered: {
            if (repoTab.markedOrigin !== driver.markWanted || repoTab.busyCount > 0)
                return
            pushDefaultTimer.stop()
            // The judged fields lead, and together: `must_say` reads one run of the line.
            Harness.report("push_default local=" + repoTab.pushDefaultLocal
                              + " origin=" + repoTab.markedOrigin
                              + " marked=" + repoTab.pushDefault
                              + " target=" + repoTab.defaultRemote
                              + " remotes=" + repoTab.remoteCount)
            driver.complete()
        }
    }
    /// PGG_AUTO_ACT=publish-upstream. The barrier is the status read after the write (`branch.upstream`): only then
    /// does the button have the destination to open on. `tracked=` says the far side lacks it; `remote=` / `branch=`
    /// are what the question opened on, which a picture of two boxes cannot say.
    SampleTimer {
        id: publishUpstreamTimer
        property string want: ""
        property bool pressed: false
        onTriggered: {
            if (!publishUpstreamTimer.pressed) {
                const target = repoTab.defaultRemote + "/" + publishUpstreamTimer.want
                if (!workTree.countsSettled || workTree.upstream !== target || repoTab.busyCount > 0)
                    return
                page.pushNow()
                publishUpstreamTimer.pressed = true
                return
            }
            // The bar all the way down, so the boxes are on screen (`AskBar.settled`).
            if (!graphPane.askCard.settled)
                return
            publishUpstreamTimer.stop()
            Harness.report("publish upstream=" + workTree.upstream
                              + " tracked=" + workTree.upstreamTracked
                              + " state=" + page.pushState
                              + " remote=" + publishFlow.publishRemote
                              + " branch=" + publishFlow.publishBranch)
            driver.complete()
        }
    }
    /// PGG_AUTO_ACT=push-target: the toolbar's destination and standing, off the bindings the button reads
    /// (`RepoPage.pushTargetLabel` / `pushState`), with the marks that decided it — the label cannot say which one git
    /// followed. `workTree.loaded` is the barrier: before the first status every mark reads unset.
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
    /// PGG_AUTO_ACT=push-hover: the tip on the band's push button, the one place the count a push sends (or an
    /// overwrite drops) is said. The opening's fetch is waited out first: the shared tooltip takes the words once as
    /// it opens, so a count that moved later would be photographed stale.
    SampleTimer {
        id: pushHoverTimer
        property bool pointed: false
        onTriggered: {
            if (!pushHoverTimer.pointed) {
                if (!workTree.loaded || repoTab.state !== "open" || repoTab.busyCount > 0
                        || repoTab.autoFetchRunning)
                    return
                pushHoverTimer.pointed = true
                // The hand goes on where a real one is read (`HoverToolButton.pointedAt`).
                page.pageBand.pushPointedAt = true
                return
            }
            // The tip waits out `tipDelayMs` before it stands.
            if (!page.pageBand.pushTipStanding)
                return
            pushHoverTimer.stop()
            // The counts the tip says: the destination's own where the push goes elsewhere (`RepoPage.pushAhead`).
            Harness.report("push_hover tip=" + page.pageBand.pushTipStanding
                              + " mode=" + page.pageBand.pushMode
                              + " ahead=" + page.pushAhead
                              + " behind=" + page.pushBehind)
            driver.complete()
        }
    }
    /// PGG_AUTO_ACT=publish-remotes-marked: the first push's destination list with the mark in it. The mark's write is
    /// waited out first — the question reads the marked remote as it opens — and the list opened, as `publish-remotes`
    /// opens it, only once the question is dressed and the bar is all the way down.
    SampleTimer {
        id: publishMarkedTimer
        onTriggered: {
            if (repoTab.markedOrigin !== driver.markWanted || repoTab.busyCount > 0)
                return
            if (!publishFlow.publishAsking) {
                page.pushNow()
                return
            }
            if (!publishFlow.publishChecked || !graphPane.askCard.settled)
                return
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
    /// PGG_AUTO_ACT=remote-menu: the menu a remote's own row raises, left standing (overlay.png). `rows=`: two on a
    /// remote that is not origin, one on the remote both keys already name (デザイン規約 §メニュー).
    SampleTimer {
        id: remoteMenuTimer
        onTriggered: {
            if (repoTab.markedOrigin !== driver.markWanted || repoTab.busyCount > 0)
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
    /// mark. `box=` is whether the line is checked, which a picture of a form cannot be trusted for.
    SampleTimer {
        id: remoteUrlTimer
        onTriggered: {
            if (repoTab.markedOrigin !== driver.markWanted || repoTab.busyCount > 0)
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
            if (!driver.wroteAndSettled())
                return
            // `<row>` or `<row>:<tag>` — the second reports that one tag's line.
            const asked = ("" + Harness.autoActArg).split(":")
            const stacked = graphPane.view.itemAtIndex(Number(asked[0]))
            if (!stacked)
                return
            fetchedRefListTimer.stop()
            graphPane.view.chipExpandRequested(
                stacked.oid_hex, stacked.index, stacked.chipItem.records, stacked.chipItem)
            // The card's lines are handed over as it opens (`RowHoverHost.openRefList`), so they are read here. The
            // free words last: a claim is one substring.
            Harness.report("fetch_ref_list row=" + stacked.index + " "
                           + (asked.length > 1 ? acts.tagWords(asked[1]) : acts.apartWords()))
            renderedBarrier.begin()
        }
    }
    // The refusal must be on the button before the second go is sent, and the report is the only proof it got there —
    // the go clears the mark before the shot.
    SampleTimer {
        id: pushRetryTimer
        onTriggered: {
            if (!page.pushFailed || repoTab.busyCount !== 0)
                return
            pushRetryTimer.stop()
            Harness.report("push_retry refused=" + page.pushFailed
                              + " branch=" + publishFlow.pushFailBranch)
            driver.pressWrite("force-push", () => {
                page.forcePush()
                return true
            })
            writeBarrier.start()
        }
    }
    /// The run of failed fetches asked for, and what to do to the button at its end: press resume, or point at it.
    property int fetchFailRuns: 0
    property bool fetchResumeAfter: false
    property bool fetchPointAfter: false
    /// Whether the word was at full before the hand went on — what a picture with a hand on the button cannot hold.
    property bool fetchRestFull: false
    /// The run reached its length. Kept apart because resuming clears the tab's count, and re-reading the length would
    /// start a second run over the resumed button.
    property bool fetchRunDone: false
    /// The `writeSeq` this verb's last fetch was asked at, and the failures standing then. A fetch is admitted a drain
    /// after it is asked, so between one finishing and the next being admitted `busyCount` is 0 and `writeSeq` has
    /// moved — which reads like a run at rest, and `writeBarrier` completes there. So the wait is for the answer to
    /// the ask.
    property int fetchAskSeq: -1
    property int fetchAskFails: -1
    /// The stopped button, read as the press goes in: resuming takes it down, so it is kept here or lost.
    property bool fetchStopped: false
    /// A run of failed fetches and the resume at its end. Asking for the next fetch and judging the run over are one
    /// owner, so the two cannot disagree.
    SampleTimer {
        id: fetchFailTimer
        onTriggered: {
            // Waits while git is out (the opening's fetch shows in `autoFetchRunning`, the rest in `busyCount`) and on
            // an ask still waiting for its answer.
            if (repoTab.busyCount !== 0 || repoTab.autoFetchRunning)
                return
            if (acts.fetchAskSeq >= 0 && repoTab.writeSeq <= acts.fetchAskSeq)
                return
            if (!acts.fetchRunDone && repoTab.fetchFailures < acts.fetchFailRuns) {
                // A clean fetch means the remote is reachable, and asking again cannot make it fail. Say so once and
                // let the watchdog end the run — completing would photograph a button that never failed
                // (verbs.md §ヘッドレスで色を確かめる時はデモリモートの URL を疑う).
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
                // Read before the press, because the press is what takes them down.
                const wasStopped = repoTab.autoFetchSuspended
                const askedAt = repoTab.writeSeq
                // The band's own button: calling `resumeAutoFetch` directly would pass a build where the press
                // stopped reaching it. It clears the run and fetches again, so later ticks wait for that fetch too.
                if (!page.pageBand.fetchNow())
                    return
                acts.fetchResumeAfter = false
                acts.fetchStopped = wasStopped
                acts.fetchAskSeq = askedAt
                return
            }
            if (Harness.autoAct === "fetch-hover") {
                if (acts.fetchPointAfter) {
                    acts.fetchPointAfter = false
                    acts.fetchRestFull = page.pageBand.fetchWordFull
                    page.pageBand.fetchPointedAt = true
                    return
                }
                // A button with nothing to say never gets its tip — `fetch-tip`'s side.
                if (!page.pageBand.fetchTipStanding)
                    return
                fetchFailTimer.stop()
                Harness.report("fetch_hover tip=" + page.pageBand.fetchTipStanding
                                  + " word=" + page.pageBand.fetchWordFull
                                  + " rest=" + acts.fetchRestFull
                                  + " stopped=" + repoTab.autoFetchSuspended
                                  + " fails=" + repoTab.fetchFailures)
                driver.complete()
                return
            }
            fetchFailTimer.stop()
            if (Harness.autoAct === "fetch-resume")
                // What the picture cannot hold: stopped at the press, and a fetch ran after it. The picture is the
                // warning shape, the same as `fetch-fail 1`.
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
    /// The dialog alone ends the run: there is no target to check yet.
    SampleTimer {
        id: publishSurfaceTimer
        onTriggered: {
            if (!remoteDialog.visible && !publishFlow.publishChecked)
                return
            publishSurfaceTimer.stop()
            driver.complete()
        }
    }
    /// `publish-remotes`: the popup, asked for each tick — the form is created asynchronously with the ask bar. Only
    /// once the question is dressed and the bar is all the way down: a list opened mid-slide is photographed over a bar
    /// still cut short at its foot.
    SampleTimer {
        id: publishRemotesTimer
        onTriggered: {
            if (!publishFlow.publishChecked || !graphPane.askCard.settled)
                return
            if (!publishFlow.publishRemotesOpen()) {
                publishFlow.openPublishRemotes()
                return
            }
            publishRemotesTimer.stop()
            driver.complete()
        }
    }
    /// `publish-dismiss`: what the bar wears on the way back up, which no picture holds — the exit is over long before
    /// the shot. The ✕ goes through the bar's own handler once the question is dressed (`publishChecked`) and all the
    /// way down, and the line is read in the same turn; `shut=false` says it was read with the bar still on screen.
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
    /// …and the picture once it has really gone, not a bar caught half way.
    SampleTimer {
        id: publishGoneTimer
        onTriggered: {
            if (!graphPane.askCard.shut)
                return
            publishGoneTimer.stop()
            driver.complete()
        }
    }
    /// `publish-add` stops with the real dialog on screen; a check finishing behind it is unrelated.
    SampleTimer {
        id: publishDialogTimer
        onTriggered: {
            if (!remoteDialog.visible)
                return
            publishDialogTimer.stop()
            driver.complete()
        }
    }
    /// `publish-new-go`: the dialog's own button, once visible and valid (an empty URL cannot be submitted).
    SampleTimer {
        id: publishNewTimer
        onTriggered: {
            if (!remoteDialog.visible || remoteDialog.wantedName === "" || remoteDialog.wantedUrl === "")
                return
            publishNewTimer.stop()
            // The dialog is answered here, which is the input; the ask it makes is inside the same call.
            driver.pressWrite("remote-dialog", () => {
                remoteDialog.submit()
                return true
            })
            publishAnswerTimer.start()
        }
    }
    /// What the far side turned out to hold, once the remote has answered.
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
    /// PGG_AUTO_ACT=publish-tip: the question `publish-taken` stops at, with a hand on its pill — the tip is the only
    /// place that says what an overwrite would drop. The hand goes on once the bar has stopped moving: the tooltip
    /// places itself against the pill as it opens, and would stand where a travelling pill was.
    SampleTimer {
        id: publishTipTimer
        property bool pointed: false
        onTriggered: {
            if (!publishTipTimer.pointed) {
                if (!publishFlow.publishChecked || !graphPane.askCard.settled)
                    return
                publishTipTimer.pointed = true
                graphPane.askCard.pointedAt = true
                return
            }
            if (!graphPane.askCard.tipStanding)
                return
            publishTipTimer.stop()
            Harness.report("publish_tip tip=" + graphPane.askCard.tipStanding
                              + " far=" + publishFlow.publishState
                              + " theirs=" + repoTab.remoteBranchTheirs)
            driver.complete()
        }
    }
    /// The answer, once the remote has said what it has — the pill is dead until then.
    SampleTimer {
        id: publishAnswerTimer
        /// Whether the answer comes from the name box instead of the pill (`PublishFlow.enterBranch`). A held question
        /// is never answered this way — the key is not a gesture (`AskBar.answerFromForm`) — so such a run waits out
        /// its watchdog.
        property bool byKey: false
        onTriggered: {
            if (!publishFlow.publishChecked || !graphPane.askAnswerable)
                return
            publishAnswerTimer.stop()
            // `far` is what the far side holds — not the other line's `state`, which is this end's push state.
            Harness.report("publish answering far="
                              + publishFlow.publishState
                              + " unsure=" + publishFlow.publishUnsure
                              + " answerable=" + graphPane.askAnswerable)
            // The push is the write this run waits out — `publish-new-go` presses two. Said out loud: arming over a
            // write still being waited out is a breach (`repo_tab::write_watch`).
            driver.letWriteGo()
            // The gesture a person is given: a hold is not answered by a click. Each way in arms its own watch
            // (`holdToEnd` / `pressWrite`); the hold's is at its end, ticks after this.
            if (publishFlow.publishRefused)
                driver.holdToEnd(graphPane)
            else
                driver.pressWrite("answer-ask", () => {
                    if (publishAnswerTimer.byKey)
                        return publishFlow.enterBranch()
                    page.answerRowAsk()
                    return true
                })
            writeBarrier.start()
        }
    }
}
