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
    readonly property var worktree: driver.worktree
    readonly property var graphPane: driver.graphPane
    readonly property var publishFlow: driver.publishFlow
    readonly property var remoteDialog: driver.remoteDialog
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

    /// The remote a push-default verb's argument names (`<remote>[:marked]`).
    readonly property string remoteTarget: {
        const arg = Harness.autoActArg
        return arg.endsWith(":marked") ? arg.substring(0, arg.length - ":marked".length) : arg
    }
    /// The mark the push-default verbs wait for before the shot, kept as the dispatch read it (`markWanted`):
    /// where no mark is asked to move, the one already there.
    function markWanted() {
        return driver.act.witnessed("markWanted")
    }
    /// The first push's lines as the run claims them: the surface as the press left it, kept in the press's turn
    /// (`publish`), and for a run that answers the question, the question as it was answered (`answering`).
    function publishLines() {
        const lines = [driver.act.witnessed("publish")]
        if (["publish-go", "publish-enter", "publish-new-go"].indexOf(Harness.autoAct) >= 0)
            lines.push(publishAnswerTimer.witnessed("answering"))
        return lines
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
            // headless. Read in the turn of the press — none of these verbs completes in it, and a later reading meets
            // the push state the question's answer moved on — and kept as the dispatch's witness, which whichever
            // operation completes the run says (`publishLines`).
            driver.act.witness("publish", "publish state=" + page.pushState
                                          + " remote=" + publishFlow.publishRemote
                                          + " branch=" + publishFlow.publishBranch
                                          + " dialog=" + remoteDialog.visible
                                          + " name=" + remoteDialog.wantedName)
            driver.reportAtCompletion(() => acts.publishLines())
        } else if (act === "publish-upstream") {
            // A push with an upstream the far side does not have yet (デザイン規約 §ブランチが測られる相手を決める):
            // the question the toolbar's press raises must open on that name, or the answer just given is asked again.
            // The upstream goes in at the slot the question's pill calls — the question is `set-upstream-go`'s subject.
            publishUpstreamTimer.want = arg === "" ? "brand-new" : arg
            repoTab.setUpstream(worktree.branch, repoTab.defaultRemote, publishUpstreamTimer.want)
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
            const target = act === "publish-remotes-marked" ? repoTab.defaultRemote : acts.remoteTarget
            const wanted = driver.act.witness("markWanted", marked || act !== "remote-menu" && act !== "remote-url"
                                                            ? target : repoTab.markedOrigin)
            if (repoTab.markedOrigin !== wanted)
                repoTab.markOrigin(wanted)
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
            // The argument is how many fetches to fail (`fetchFailRuns`), so one verb reaches the warning shape and
            // the stopped one.
            AppBackend.setAutoFetchMinutes(0)
            AppBackend.setAutoFetchMinutes(5)
            fetchFailTimer.start()
        } else if (act === "fetch-hover") {
            // A hand on the button's three live shapes: the argument is how many fetches to fail first — none, one,
            // or enough to stop the timer. The hand goes on at the run's end.
            if (acts.fetchFailRuns > 0) {
                AppBackend.setAutoFetchMinutes(0)
                AppBackend.setAutoFetchMinutes(5)
            }
            fetchFailTimer.start()
        } else if (act === "fetch-resume") {
            // Long enough a run to stop the timer, so the press has a stopped button to land on; the resume is pressed
            // at the run's end.
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
    PageStage {
        id: pushDefaultTimer
        name: "push_default"
        // The judged fields lead, and together: `must_say` reads one run of the line.
        onTriggered: pushDefaultTimer.complete({
            "name": "push_default_ready",
            "when": () => ({ "ready": !(repoTab.markedOrigin !== acts.markWanted() || repoTab.busyCount > 0) }),
            "report": () => "push_default local=" + repoTab.pushDefaultLocal
                            + " origin=" + repoTab.markedOrigin
                            + " marked=" + repoTab.pushDefault
                            + " target=" + repoTab.defaultRemote
                            + " remotes=" + repoTab.remoteCount })
    }
    /// PGG_AUTO_ACT=publish-upstream. The barrier is the status read after the write (`branch.upstream`): only then
    /// does the button have the destination to open on. `tracked=` says the far side lacks it; `remote=` / `branch=`
    /// are what the question opened on, which a picture of two boxes cannot say.
    PageStage {
        id: publishUpstreamTimer
        name: "publish_upstream"
        order: ["pressed"]
        property string want: ""
        onTriggered: {
            const op = publishUpstreamTimer
            if (op.at("start")) {
                if (!Awaited.all("publish_upstream_ready", {
                    "counts": worktree.countsSettled,
                    "upstream": worktree.upstream === repoTab.defaultRemote + "/" + op.want,
                    "idle": repoTab.busyCount <= 0 }))
                    return
                op.input("pressed", () => {
                    page.pushNow()
                })
                return
            }
            // The bar all the way down, so the boxes are on screen (`AskBar.settled`).
            op.complete({
                "name": "publish_upstream_asked",
                "when": () => ({ "bar": graphPane.askCard.settled }),
                "report": () => "publish upstream=" + worktree.upstream
                                + " tracked=" + worktree.upstreamTracked
                                + " state=" + page.pushState
                                + " remote=" + publishFlow.publishRemote
                                + " branch=" + publishFlow.publishBranch })
        }
    }
    /// PGG_AUTO_ACT=push-target: the toolbar's destination and standing, off the bindings the button reads
    /// (`RepoPage.pushTargetLabel` / `pushState`), with the marks that decided it — the label cannot say which one git
    /// followed. `worktree.loaded` is the barrier: before the first status every mark reads unset.
    PageStage {
        id: pushTargetTimer
        name: "push_target"
        onTriggered: pushTargetTimer.complete({
            "name": "push_target_ready",
            "when": () => ({
                "loaded": worktree.loaded,
                "open": repoTab.state === "open",
                "idle": repoTab.busyCount === 0 }),
            "report": () => "push_target label=" + page.pushTargetLabel
                            + " state=" + page.pushState
                            + " branch_mark=" + worktree.pushRemote
                            + " repo_mark=" + repoTab.pushDefault
                            + " tracks=" + worktree.upstream })
    }
    /// PGG_AUTO_ACT=push-hover: the tip on the band's push button, the one place the count a push sends (or an
    /// overwrite drops) is said. The opening's fetch is waited out first: the shared tooltip takes the words once as
    /// it opens, so a count that moved later would be photographed stale.
    PageStage {
        id: pushHoverTimer
        name: "push_hover"
        order: ["pointed"]
        onTriggered: {
            const op = pushHoverTimer
            if (op.at("start")) {
                if (!Awaited.all("push_hover_ready", {
                    "loaded": worktree.loaded,
                    "open": repoTab.state === "open",
                    "idle": repoTab.busyCount <= 0,
                    "fetched": !repoTab.autoFetchRunning }))
                    return
                // The hand goes on where a real one is read (`HoverToolButton.pointedAt`).
                op.input("pointed", () => {
                    page.pageBand.pushPointedAt = true
                })
                return
            }
            // The tip waits out `tipDelayMs` before it stands. The counts the tip says: the destination's own where the
            // push goes elsewhere (`RepoPage.pushAhead`).
            op.complete({
                "name": "push_hover_tip",
                "when": () => ({ "tip": page.pageBand.pushTipStanding }),
                "report": () => "push_hover tip=" + page.pageBand.pushTipStanding
                                + " mode=" + page.pageBand.pushMode
                                + " ahead=" + page.pushAhead
                                + " behind=" + page.pushBehind })
        }
    }
    /// PGG_AUTO_ACT=publish-remotes-marked: the first push's destination list with the mark in it. The mark's write is
    /// waited out first — the question reads the marked remote as it opens — and the list opened, as `publish-remotes`
    /// opens it, only once the question is dressed and the bar is all the way down. The list is asked for each beat
    /// until it is up: the form is created asynchronously with the ask bar.
    PageStage {
        id: publishMarkedTimer
        name: "publish_marked"
        order: ["asked", "listed"]
        onTriggered: {
            const op = publishMarkedTimer
            const ready = !(repoTab.markedOrigin !== acts.markWanted() || repoTab.busyCount > 0)
            if (op.at("start")) {
                if (!Awaited.all("publish_marked_ready", { "ready": ready }))
                    return
                op.input("asked", () => {
                    page.pushNow()
                })
                return
            }
            if (op.at("asked")) {
                if (!Awaited.all("publish_marked_asked", {
                    "ready": ready,
                    "asking": publishFlow.publishAsking,
                    "checked": publishFlow.publishChecked,
                    "bar": graphPane.askCard.settled }))
                    return
                op.input("listed", () => publishFlow.openPublishRemotes())
                return
            }
            op.complete({
                "name": "publish_marked_listed",
                "when": () => ({ "ready": !(repoTab.markedOrigin !== acts.markWanted() || repoTab.busyCount > 0),
                                 "open": publishFlow.publishRemotesOpen() }),
                "report": () => "publish_remotes open=" + publishFlow.publishRemotesOpen()
                                + " marked=" + publishFlow.publishRemotesMarked()
                                + " name=" + repoTab.pushDefault })
        }
    }
    /// PGG_AUTO_ACT=remote-menu: the menu a remote's own row raises, left standing (overlay.png). `rows=`: two on a
    /// remote that is not origin, one on the remote both keys already name (デザイン規約 §メニュー). Raised each beat
    /// until it is up.
    PageStage {
        id: remoteMenuTimer
        name: "remote_menu"
        order: ["raised"]
        function ready() {
            return !(repoTab.markedOrigin !== acts.markWanted() || !driver.menusLive())
        }
        onTriggered: {
            const op = remoteMenuTimer
            if (op.at("start")) {
                if (!Awaited.all("remote_menu_ready", { "ready": op.ready() }))
                    return
                op.input("raised", () => page.openRemoteMenu(acts.remoteTarget) && driver.remoteMenu.opened)
                return
            }
            op.complete({
                "name": "remote_menu_open",
                "when": () => ({ "ready": op.ready(), "open": driver.remoteMenu.opened }),
                "report": () => "remote_menu open=" + driver.remoteMenu.opened
                                + " rows=" + driver.remoteMenu.offeredRows
                                + " remote=" + acts.remoteTarget
                                + " marked=" + repoTab.pushDefault })
        }
    }
    /// PGG_AUTO_ACT=remote-url: the form that holds a remote's URL, left standing (overlay.png) — the other way to the
    /// mark. `box=` is whether the line is checked, which a picture of a form cannot be trusted for. Asked for each
    /// beat until it is up.
    PageStage {
        id: remoteUrlTimer
        name: "remote_url"
        order: ["opened"]
        function ready() {
            return !(repoTab.markedOrigin !== acts.markWanted() || repoTab.busyCount > 0)
        }
        onTriggered: {
            const op = remoteUrlTimer
            if (op.at("start")) {
                if (!Awaited.all("remote_url_ready", { "ready": op.ready() }))
                    return
                op.input("opened", () => {
                    driver.publishFlow.startEditRemote(acts.remoteTarget)
                    return driver.remoteDialog.visible
                })
                return
            }
            op.complete({
                "name": "remote_url_open",
                "when": () => ({ "ready": op.ready(), "dialog": driver.remoteDialog.visible }),
                "report": () => "remote_url dialog=" + driver.remoteDialog.visible
                                + " box=" + driver.remoteDialog.marked
                                + " remote=" + driver.remoteDialog.editing
                                + " local=" + driver.remoteDialog.markLocal })
        }
    }
    // The fetch has to land, and its answer reach the chips, before the stacked ones are worth unstacking.
    PageStage {
        id: fetchedRefListTimer
        name: "fetched_ref_list"
        order: ["expanded"]
        onTriggered: {
            const op = fetchedRefListTimer
            if (op.at("expanded")) {
                // The card's line was kept as it opened; the picture waits for the page under it.
                op.complete({ "name": "fetched_ref_list_settled", "when": () => ({ "page": PageSettled.settled(page) }),
                              "report": () => op.witnessed("card") })
                return
            }
            // `<row>` or `<row>:<tag>` — the second reports that one tag's line.
            const asked = ("" + Harness.autoActArg).split(":")
            const stacked = graphPane.view.itemAtIndex(Number(asked[0]))
            if (!Awaited.all("fetched_ref_list_ready", { "ready": driver.wroteAndSettled(),
                                                         "page": PageSettled.settled(page),
                                                         "row": !!stacked }))
                return
            // The card's lines are handed over as it opens (`RowHoverHost.openRefList`), so they are read here, in the
            // turn it opened, and kept (`card`). The free words last: a claim is one substring.
            op.input("expanded", () => {
                graphPane.view.chipExpandRequested(
                    stacked.oid_hex, stacked.index, stacked.chipItem.records, stacked.chipItem)
                op.witness("card", "fetch_ref_list row=" + stacked.index + " "
                                   + (asked.length > 1 ? acts.tagWords(asked[1]) : acts.apartWords()))
            })
        }
    }
    // The refusal must be on the button before the second go is sent, and the report is the only proof it got there —
    // the go clears the mark before the shot. The write barrier completes this one.
    PageStage {
        id: pushRetryTimer
        name: "push_retry"
        order: ["refused", "forced"]
        onTriggered: {
            const op = pushRetryTimer
            if (op.at("start")) {
                if (!Awaited.all("push_retry_ready", { "ready": !(!page.pushFailed || repoTab.busyCount !== 0) }))
                    return
                // `branch=` is every branch a refusal is remembered against — the record itself, not the branch
                // `refused=` was asked about.
                op.advance("refused", () => "push_retry refused=" + page.pushFailed
                                            + " branch=" + Array.from(publishFlow.pushFails.keys()).join(","))
            }
            if (!op.input("forced", () => driver.pressWrite("force-push", () => {
                page.forcePush()
                return true
            })))
                return
            op.stop()
            writeBarrier.start()
        }
    }
    /// The run of failed fetches asked for, off the argument: `fetch-fail` fails at least one, `fetch-hover` as many
    /// as named, `fetch-resume` enough to stop the timer.
    readonly property int fetchFailRuns: Harness.autoAct === "fetch-fail" ? Math.max(1, Number(Harness.autoActArg))
                                       : Harness.autoAct === "fetch-hover" ? Math.max(0, Number(Harness.autoActArg))
                                       : Harness.autoAct === "fetch-resume" ? 3 : 0
    /// What every beat of the run waits on: git not out (the opening's fetch shows in `autoFetchRunning`, the rest in
    /// `busyCount`), and no ask still waiting for its answer. `askSeq` is the `writeSeq` the run's last fetch was
    /// asked at, -1 for none: a fetch is admitted a drain after it is asked, so between one finishing and the next
    /// being admitted `busyCount` is 0 and `writeSeq` has moved — which reads like a run at rest, and `writeBarrier`
    /// completes there. So the wait is for the answer to the ask.
    function fetchQuiet(askSeq) {
        return { "ready": !(repoTab.busyCount !== 0 || repoTab.autoFetchRunning),
                 "answered": !(askSeq >= 0 && repoTab.writeSeq <= askSeq) }
    }
    /// A run of failed fetches. Asking for the next fetch and judging the run long enough are one owner, so the two
    /// cannot disagree; what is done at its end is `fetchEndTimer`'s, so a resume that clears the tab's count cannot
    /// start a second run over the resumed button. Each generation's ask keeps the `writeSeq` it was asked at and the
    /// failures standing then (`askSeq`, `askFails`), read by the same generation once its answer is in.
    PageStage {
        id: fetchFailTimer
        name: "fetch_fail_run"
        order: ["fetched"]
        onTriggered: {
            const op = fetchFailTimer
            const asked = op.at("fetched")
            if (!Awaited.all("fetch_fail_ready", acts.fetchQuiet(asked ? op.witnessed("askSeq") : -1)))
                return
            if (repoTab.fetchFailures >= acts.fetchFailRuns) {
                op.stop()
                fetchEndTimer.start()
                return
            }
            // A clean fetch means the remote is reachable, and asking again cannot make it fail. Say so once and let
            // the watchdog end the run — completing would photograph a button that never failed
            // (verbs.md §ヘッドレスで色を確かめる時はデモリモートの URL を疑う).
            if (asked && repoTab.fetchFailures <= op.witnessed("askFails")) {
                op.say(() => "fetch_fail reachable=true fails=" + repoTab.fetchFailures)
                op.stop()
                return
            }
            // The answer to this generation's fetch is in. The run asks the same fetch again on purpose, each ask a
            // generation of its own.
            if (asked)
                op.again()
            op.input("fetched", () => {
                op.witness("askFails", repoTab.fetchFailures)
                op.witness("askSeq", repoTab.writeSeq)
                repoTab.fetch("")
            })
        }
    }
    /// The run's end: the band's resume pressed (`fetch-resume`), a hand put on the button (`fetch-hover`) or nothing
    /// (`fetch-fail`), then the button read. Its witnesses: the stopped button and the `writeSeq` the resume's fetch was
    /// asked at, read as the press goes in (`stopped`, `askSeq`) — resuming takes the first down, so it is kept there
    /// or lost; whether the word was at full before the hand went on (`restFull`) — what a picture with a hand on the
    /// button cannot hold.
    PageStage {
        id: fetchEndTimer
        name: "fetch_fail_end"
        order: Harness.autoAct === "fetch-hover" ? ["pointed"] : Harness.autoAct === "fetch-resume" ? ["resumed"] : []
        function line() {
            if (Harness.autoAct === "fetch-hover")
                return "fetch_hover tip=" + page.pageBand.fetchTipStanding
                       + " word=" + page.pageBand.fetchWordFull
                       + " rest=" + fetchEndTimer.witnessed("restFull")
                       + " stopped=" + repoTab.autoFetchSuspended
                       + " fails=" + repoTab.fetchFailures
            // What the picture cannot hold: stopped at the press, and a fetch ran after it. The picture is the
            // warning shape, the same as `fetch-fail 1`.
            if (Harness.autoAct === "fetch-resume")
                return "fetch_resume stopped=" + fetchEndTimer.witnessed("stopped")
                       + " fetched=" + (repoTab.fetchFailures > 0)
                       + " fails=" + repoTab.fetchFailures
                       + " suspended=" + repoTab.autoFetchSuspended
            return "fetch_fail stopped=" + repoTab.autoFetchSuspended
                   + " fails=" + repoTab.fetchFailures
                   + " wanted=" + acts.fetchFailRuns
        }
        /// The run at rest past the resume's fetch where one was asked, and a hand's tip up where one was put on — a
        /// button with nothing to say never gets its tip (`fetch-tip`'s side).
        function quiet(resumed, pointed) {
            const terms = acts.fetchQuiet(resumed ? fetchEndTimer.witnessed("askSeq") : -1)
            if (pointed)
                terms["tip"] = page.pageBand.fetchTipStanding
            return terms
        }
        onTriggered: {
            const op = fetchEndTimer
            if (op.at("start") && Harness.autoAct === "fetch-resume") {
                if (!Awaited.all("fetch_fail_end", op.quiet(false, false)))
                    return
                // The band's own button: calling `resumeAutoFetch` directly would pass a build where the press stopped
                // reaching it. It clears the run and fetches again, so later beats wait for that fetch too.
                op.input("resumed", () => {
                    // Read before the press, because the press is what takes them down.
                    const wasStopped = repoTab.autoFetchSuspended
                    const askedAt = repoTab.writeSeq
                    if (!page.pageBand.fetchNow())
                        return false
                    op.witness("stopped", wasStopped)
                    op.witness("askSeq", askedAt)
                    return true
                })
                return
            }
            if (op.at("start") && Harness.autoAct === "fetch-hover") {
                if (!Awaited.all("fetch_fail_end", op.quiet(false, false)))
                    return
                op.input("pointed", () => {
                    op.witness("restFull", page.pageBand.fetchWordFull)
                    page.pageBand.fetchPointedAt = true
                })
                return
            }
            op.complete({
                "name": "fetch_fail_end",
                "when": () => op.quiet(Harness.autoAct === "fetch-resume", Harness.autoAct === "fetch-hover"),
                "report": () => op.line() })
        }
    }

    /// The first-push surface is either the standing question (a remote exists) or the add-remote dialog (none does).
    /// The dialog alone ends the run: there is no target to check yet.
    PageStage {
        id: publishSurfaceTimer
        name: "publish_surface"
        // What the far side's answer made of the bar: the line under the question, the reader's only word of it.
        onTriggered: publishSurfaceTimer.complete({
            "name": "publish_surface_ready",
            "when": () => ({ "ready": !(!remoteDialog.visible && !publishFlow.publishChecked) }),
            "report": () => driver.verbReport().concat([publishFlow.publishChecked
                                                        ? "publish asked state=" + publishFlow.publishState
                                                          + " says=" + graphPane.askDetail
                                                        : ""]) })
    }
    /// `publish-remotes`: the popup, asked for each beat until it is up — the form is created asynchronously with the
    /// ask bar. Only once the question is dressed and the bar is all the way down: a list opened mid-slide is
    /// photographed over a bar still cut short at its foot.
    PageStage {
        id: publishRemotesTimer
        name: "publish_remotes"
        order: ["listed"]
        function ready() {
            return !(!publishFlow.publishChecked || !graphPane.askCard.settled)
        }
        onTriggered: {
            const op = publishRemotesTimer
            if (op.at("start")) {
                if (!Awaited.all("publish_remotes_ready", { "ready": op.ready() }))
                    return
                op.input("listed", () => publishFlow.openPublishRemotes())
                return
            }
            op.complete({
                "name": "publish_remotes_open",
                "when": () => ({ "ready": op.ready(), "open": publishFlow.publishRemotesOpen() }),
                "report": () => driver.verbReport() })
        }
    }
    /// `publish-dismiss`: what the bar wears on the way back up, which no picture holds — the exit is over long before
    /// the shot. The ✕ goes through the bar's own handler once the question is dressed (`publishChecked`) and all the
    /// way down, and the line is read in the same turn; `shut=false` says it was read with the bar still on screen.
    /// The picture is taken once it has really gone, not a bar caught half way.
    PageStage {
        id: publishDismissTimer
        name: "publish_dismiss"
        order: ["dismissed"]
        onTriggered: {
            const op = publishDismissTimer
            if (op.at("start")) {
                if (!Awaited.all("publish_dismiss_ready", {
                    "ready": !(!publishFlow.publishChecked || !graphPane.askCard.settled) }))
                    return
                // The bar as the press left it, read in the press's own turn and kept (`dismissed`).
                op.input("dismissed", () => {
                    graphPane.askCard.dismiss()
                    op.witness("dismissed", "ask_dismissed shut=" + graphPane.askCard.shut
                                            + " words=" + (graphPane.askCard.label !== "")
                                            + " detail=" + (graphPane.askDetail !== "")
                                            + " code=" + graphPane.askCode
                                            + " neutral=" + graphPane.askNeutral)
                })
                return
            }
            op.complete({
                "name": "publish_gone_ready",
                "when": () => ({ "ready": graphPane.askCard.shut }),
                "report": () => driver.verbReport().concat([op.witnessed("dismissed")]) })
        }
    }
    /// `publish-add` stops with the real dialog on screen; a check finishing behind it is unrelated.
    PageStage {
        id: publishDialogTimer
        name: "publish_dialog"
        onTriggered: publishDialogTimer.complete({
            "name": "publish_dialog_ready",
            "when": () => ({ "ready": remoteDialog.visible }),
            "report": () => driver.verbReport() })
    }
    /// `publish-new-go`: the dialog's own button, once visible and valid (an empty URL cannot be submitted).
    PageStage {
        id: publishNewTimer
        name: "publish_new"
        order: ["submitted"]
        onTriggered: {
            const op = publishNewTimer
            if (!Awaited.all("publish_new_ready", { "dialog": remoteDialog.visible,
                                                    "name": remoteDialog.wantedName !== "",
                                                    "url": remoteDialog.wantedUrl !== "" }))
                return
            // The dialog is answered here, which is the input; the ask it makes is inside the same call.
            if (!op.input("submitted", () => driver.pressWrite("remote-dialog", () => {
                remoteDialog.submit()
                return true
            })))
                return
            op.stop()
            publishAnswerTimer.start()
        }
    }
    /// What the far side turned out to hold, once the remote has answered.
    PageStage {
        id: publishSettleTimer
        name: "publish_settle"
        onTriggered: publishSettleTimer.complete({
            "name": "publish_settle_ready",
            "when": () => ({ "ready": publishFlow.publishChecked }),
            "report": () => driver.verbReport().concat(["publish settled far="
                                                        + publishFlow.publishState
                                                        + " code=" + graphPane.askCode
                                                        + " hold=" + graphPane.askHold
                                                        + " alert=" + graphPane.askAlert
                                                        + " lease=" + (publishFlow.publishLease !== "")
                                                        + " theirs=" + repoTab.remoteBranchTheirs]) })
    }
    /// PGG_AUTO_ACT=publish-tip: the question `publish-taken` stops at, with a hand on its pill — the tip is the only
    /// place that says what an overwrite would drop. The hand goes on once the bar has stopped moving: the tooltip
    /// places itself against the pill as it opens, and would stand where a travelling pill was.
    PageStage {
        id: publishTipTimer
        name: "publish_tip"
        order: ["pointed"]
        onTriggered: {
            const op = publishTipTimer
            if (op.at("start")) {
                if (!Awaited.all("publish_tip_ready", { "checked": publishFlow.publishChecked,
                                                        "bar": graphPane.askCard.settled }))
                    return
                op.input("pointed", () => {
                    graphPane.askCard.pointedAt = true
                })
                return
            }
            op.complete({
                "name": "publish_tip_standing",
                "when": () => ({ "tip": graphPane.askCard.tipStanding }),
                "report": () => driver.verbReport().concat(["publish_tip tip=" + graphPane.askCard.tipStanding
                                                            + " far=" + publishFlow.publishState
                                                            + " theirs=" + repoTab.remoteBranchTheirs]) })
        }
    }
    /// The answer, once the remote has said what it has — the pill is dead until then. The write barrier completes
    /// this one.
    PageStage {
        id: publishAnswerTimer
        name: "publish_answer"
        order: ["read", "answered"]
        /// Whether the answer comes from the name box instead of the pill (`PublishFlow.enterBranch`). A held question
        /// is never answered this way — the key is not a gesture (`AskBar.answerFromForm`) — so such a run waits out
        /// its watchdog.
        property bool byKey: false
        function answering() {
            return "publish answering far=" + publishFlow.publishState
                   + " unsure=" + publishFlow.publishUnsure
                   + " answerable=" + graphPane.askAnswerable
        }
        /// The gesture a person is given: a hold is not answered by a click. Each way in arms its own watch
        /// (`holdToEnd` / `pressWrite`); the hold's is at its end, ticks after this. Answers whether it went in.
        function answer() {
            if (publishFlow.publishRefused)
                return driver.holdToEnd(graphPane)
            return driver.pressWrite("answer-ask", () => {
                if (publishAnswerTimer.byKey)
                    return publishFlow.enterBranch()
                page.answerRowAsk()
                return true
            })
        }
        onTriggered: {
            const op = publishAnswerTimer
            if (op.at("start")) {
                if (!Awaited.all("publish_answer_ready", {
                    "ready": !(!publishFlow.publishChecked || !graphPane.askAnswerable) }))
                    return
                // `far` is what the far side holds — not the other line's `state`, which is this end's push state.
                // Read in the turn of the press, which takes the question down, and kept (`answering`): the line the
                // write barrier's completion says (`publishLines`).
                op.advance("read", () => op.answering())
                op.witness("answering", op.answering())
                // The push is the write this run waits out — `publish-new-go` presses two. Said out loud: arming over
                // a write still being waited out is a breach (`repo_tab::write_watch`).
                driver.letWriteGo()
            }
            if (!op.input("answered", () => op.answer()))
                return
            op.stop()
            writeBarrier.start()
        }
    }
}
