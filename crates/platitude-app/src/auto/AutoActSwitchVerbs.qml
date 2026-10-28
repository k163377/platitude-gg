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
            page.showReport("update-rule", "origin", "main", "")
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
            // Through the write barrier: the landing is the status the write's settle brings.
            driver.pressWrite("switch-lands", () => page.switchToRef("branch", landing[0]) !== false)
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
            switchStoppedTimer.go = act.endsWith("-go")
            // "-go" says the bar as it stopped, then answers it.
            switchStoppedTimer.order = switchStoppedTimer.go ? ["stopped", "answered"] : []
            page.switchToRef("branch", arg.split(":")[0])
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
            // The pick, then the hand on the pill and its tip standing, then the bar said as it stands and answered —
            // each where asked.
            localUpstreamAskTimer.order = (localUpstreamAskTimer.pick !== "" ? ["picked"] : [])
                .concat(points ? ["pointed", "tipped"] : [])
                .concat(goes ? ["said", "answered"] : [])
            const local = worktree.branch
            sidebarPane.beginRename("branch", local, local)
            sidebarPane.submitEdit(want[0])
            localUpstreamAskTimer.start()
        } else if (act === "set-upstream" || act === "set-upstream-go" || act === "set-upstream-list"
                   || act === "set-upstream-enter") {
            // `<branch>[:<name to answer with>]` (`:` cannot be in a ref name); without a name the question stands
            // as it opened. A `:` with nothing after it empties the name box.
            const want = arg.split(":")
            const on = want[0]
            upstreamAskTimer.types = want.length > 1
            upstreamAskTimer.wantName = want.length > 1 ? want[1] : ""
            upstreamAskTimer.answers = act === "set-upstream-go" || act === "set-upstream-enter"
            upstreamAskTimer.byKey = act === "set-upstream-enter"
            upstreamAskTimer.lists = act === "set-upstream-list"
            // The name typed, then the list dropped, then the bar said as it stands and answered — each where asked.
            upstreamAskTimer.order = (upstreamAskTimer.types ? ["typed"] : [])
                .concat(upstreamAskTimer.lists ? ["dropped"] : [])
                .concat(upstreamAskTimer.answers ? ["said", "answered"] : [])
            // Through the row itself, so a build whose row stopped reaching the question stalls here.
            page.openRefMenu("branch", on, on, branchesModel.oidOfName(on))
            refMenu.openSub(refBranchCard)
            refUpstreamItem.triggered()
            upstreamAskTimer.start()
        } else if (act === "unset-upstream" || act === "unset-upstream-go") {
            // The BRANCH card's row that takes the upstream off. `[chip:]<branch>`: bare is the sidebar's entrance,
            // `chip:` the graph row's — one card behind two doors, so only a run through each says both are wired.
            unsetUpstreamTimer.onChip = arg.startsWith("chip:")
            unsetUpstreamTimer.branch = unsetUpstreamTimer.onChip ? arg.slice("chip:".length) : arg
            unsetUpstreamTimer.goes = act === "unset-upstream-go"
            unsetUpstreamTimer.start()
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
    PageStage {
        id: moveAskTimer
        name: "move_ask"
        // `code=` empty is part of the claim: git has no single word for this move (規約 §git 用語のコード表記).
        // `says=`: the bar's line as it stands — the count core read, judged where it lands; last, being words.
        onTriggered: moveAskTimer.complete({
            "name": "move_ask_ready",
            "when": () => ({ "ready": graphPane.askCard.settled }),
            "report": () => "move_ask hold=" + graphPane.askHold
                            + " code=" + graphPane.askCode
                            + " branch=" + worktree.branch
                            + " says=" + graphPane.askDetail })
    }
    /// Three beats, each on bars that have stopped moving: the report settled before the question is raised (a bar
    /// on its way has no Escape shortcut yet — `open` enables it), the question settled, and both after the press.
    /// The picture cannot tell one live Escape from two, so all four claims are in the line.
    PageStage {
        id: askOverNoticeTimer
        name: "ask_over_notice"
        order: ["asked", "escaped"]
        /// The remote ref the question is raised on, as `move-ask` takes it.
        property string ref: ""
        // What was read at the press is its witnesses — which bars stood (`askStood`, `noticeStood`) and which held
        // Escape (`askEsc`, `noticeEsc`) — kept because the press is what takes one of the two away.
        function naming(ask, notice) {
            return ask && notice ? "both" : ask ? "question" : notice ? "notice" : "none"
        }
        /// A bar is read only at one of its ends; mid-flight is a frame nobody sees.
        function moving(card) {
            return !card.settled && !card.shut
        }
        /// The bars as the press found them, and as they stand.
        function line() {
            const op = askOverNoticeTimer
            const askStood = op.witnessed("askStood")
            const noticeStood = op.witnessed("noticeStood")
            return "ask_over_notice stood=" + op.naming(askStood, noticeStood)
                   + " holds=" + op.naming(op.witnessed("askEsc"), op.witnessed("noticeEsc"))
                   + " went=" + op.naming(askStood && !graphPane.askCard.open, noticeStood && !page.noticeCard.open)
                   + " left=" + op.naming(graphPane.askCard.open, page.noticeCard.open)
        }
        onTriggered: {
            const op = askOverNoticeTimer
            // Each bar is waited for only inside the beat that needs it (app-ui.md §UI 自動化): the press takes one
            // away, so a `settled` read on every tick would wait out the watchdog.
            if (op.at("start")) {
                if (!Awaited.all("ask_over_notice_report", { "notice": page.noticeCard.settled }))
                    return
                op.input("asked", () => {
                    page.switchToRef("remote", op.ref)
                })
                return
            }
            if (op.at("asked")) {
                if (!Awaited.all("ask_over_notice_question", { "question": graphPane.askCard.settled }))
                    return
                op.input("escaped", () => {
                    op.witness("askStood", graphPane.askCard.open)
                    op.witness("noticeStood", page.noticeCard.open)
                    const askEsc = op.witness("askEsc", graphPane.askCard.escapes)
                    const noticeEsc = op.witness("noticeEsc", page.noticeCard.escapes)
                    // What Escape itself runs, on whichever bar holds it: a build that handed it to the wrong bar is
                    // judged on what that bar did, and one that handed it to neither presses nothing.
                    if (askEsc)
                        graphPane.askCard.dismiss()
                    else if (noticeEsc)
                        page.noticeCard.dismiss()
                })
                return
            }
            op.complete({
                "name": "ask_over_notice_still",
                "when": () => ({ "question": !op.moving(graphPane.askCard), "notice": !op.moving(page.noticeCard) }),
                "report": () => op.line() })
        }
    }
    // The carry-the-upstream question, read once the bar has settled. A run that answers it claims the bar as it stood
    // when answered (`bar`, kept before the answer takes it down), said by whichever completes the run: the write
    // barrier, or this operation where `leave` writes nothing.
    PageStage {
        id: localUpstreamAskTimer
        name: "local_upstream_ask"
        /// Which of the three answers the run picks, empty for the bar as it comes down.
        property string pick: ""
        /// Whether the picked answer is then given to the bar.
        property bool answers: false
        /// Whether a hand goes on the pill (`-tip`) — a beat after the pick, so the tip sits against the picked word.
        property bool points: false
        function line() {
            return driver.carryWords("branch", localUpstreamAskTimer.pick)
                   + (localUpstreamAskTimer.points ? " tip=" + graphPane.askCard.tipStanding : "")
        }
        /// The bar settled, and the tip up for a run that points at the pill.
        function standing() {
            return { "bar": graphPane.askCard.settled,
                     "tip": !localUpstreamAskTimer.points || graphPane.askCard.tipStanding }
        }
        onTriggered: {
            const op = localUpstreamAskTimer
            if (op.at("answered")) {
                // `leave` writes nothing: the end is the bar going back up, not a barrier.
                op.complete({ "name": "local_upstream_shut", "when": () => ({ "shut": graphPane.askCard.shut }),
                              "report": () => driver.verbReport() })
                return
            }
            if (!op.at("said")) {
                if (!Awaited.all("local_upstream_ask_ready", { "bar": graphPane.askCard.settled }))
                    return
                if (op.pick !== "" && op.at("start")) {
                    // Through the field's own door (`RenameCarryFlow.pickChoice` — no injected click opens a popup
                    // offscreen), re-applied until it takes: the form's `Loader` is a frame behind the bar. The beats
                    // after it are the bar re-dressing around the choice.
                    op.input("picked", () => page.pickCarryChoice(driver.carryChoiceIndex(op.pick)))
                    return
                }
                if (op.points && (op.at("start") || op.at("picked"))) {
                    // For a run that stalls at the tip: `pill=` is the answer worn, `words=` whether it has a tip.
                    op.input("pointed", () => {
                        graphPane.askCard.pointedAt = true
                    }, () => "rename_carry step=pointed pill=" + graphPane.askCard.accept
                             + " words=" + (graphPane.askCard.tip !== ""))
                    return
                }
                // Put back on every beat until the tip stands: it is the hand the tip opens for.
                if (op.points && op.at("pointed") && !op.input("tipped", () => {
                    graphPane.askCard.pointedAt = true
                    return graphPane.askCard.tipStanding
                }))
                    return
                if (!op.answers) {
                    op.complete({ "name": "local_upstream_ask_ready", "when": () => op.standing(),
                                  "report": () => op.line() })
                    return
                }
                // The bar as it stands, before the answer takes it down.
                if (!Awaited.all("local_upstream_ask_ready", op.standing()) || !op.advance("said", () => op.line()))
                    return
                op.witness("bar", op.line())
                driver.reportAtCompletion(() => localUpstreamAskTimer.witnessed("bar"))
            }
            // Held or clicked, as the bar asks; a press that did not take is pressed again on the next beat.
            const held = graphPane.askHold
            if (!op.input("answered", () => {
                    if (held)
                        return driver.holdToEnd(graphPane)
                    if (op.pick === "leave") {
                        page.answerRowAsk()
                        return true
                    }
                    return driver.pressWrite("answer-ask", () => { page.answerRowAsk(); return true })
                }))
                return
            // `leave` writes nothing: the beats above wait for the bar to go back up, not for a barrier.
            if (!held && op.pick === "leave")
                return
            op.stop()
            writeBarrier.start()
        }
    }
    // Swept only once settled — on its way down the bar's words are at another width. `words=` keeps a green run
    // on an empty bar from passing for one on a question.
    PageStage {
        id: askSweepTimer
        name: "ask_sweep"
        onTriggered: {
            const op = askSweepTimer
            if (op.at("start")) {
                if (!Awaited.all("ask_sweep_ready", { "ready": graphPane.askCard.settled }))
                    return
                // The sweep is the run's input — a hand's gesture from every point of the bar's air — and what it came
                // away with is its witness (`swept`).
                op.input("swept", () => {
                    op.witness("swept", graphPane.askCard.pad.sweepAir(7, "words=" + (graphPane.askCard.label !== "")))
                })
            }
            op.complete({ "name": "ask_sweep_ready", "when": () => ({ "ready": graphPane.askCard.settled }),
                          "report": () => "ask_sweep " + op.witnessed("swept") })
        }
    }
    PageStage {
        id: switchLandsTimer
        name: "switch_lands"
        property string branch: ""
        property int stashes: -1
        // `conflicts=` is the status the write's settle brings (`session::write::settle_after`): a pop that conflicted
        // leaves its entry, and the entry lands with the refs before the status does. `log=`: a refused move raises
        // the command log (デザイン規約 §git が言ったことを読む場所), and a shut panel frames like one never raised.
        onTriggered: switchLandsTimer.complete({
            "name": "switch_lands_ready",
            "when": () => ({
                "written": driver.wroteAndSettled(),
                "branch": worktree.branch === switchLandsTimer.branch,
                "stashes": switchLandsTimer.stashes < 0 || stashesModel.total === switchLandsTimer.stashes,
                "page": PageSettled.settled(page) }),
            "report": () => "switch_landed branch=" + worktree.branch
                            + " stashes=" + stashesModel.total
                            + " wanted=" + switchLandsTimer.stashes
                            + " conflicts=" + worktree.conflictCount
                            + " log=" + page.commandsOpen })
    }
    // The claim is `held=` (the second press turned away): gated and ungated builds frame alike. `writes=` counts the
    // same claim from the write side, read once the tree has settled.
    PageStage {
        id: switchTwiceTimer
        name: "switch_twice"
        property string branch: ""
        property bool held: false
        property int writesBefore: 0
        onTriggered: switchTwiceTimer.complete({
            "name": "switch_twice_ready",
            "when": () => ({
                "ready": !(repoTab.busyCount !== 0 || worktree.branch !== switchTwiceTimer.branch),
                "page": PageSettled.settled(page) }),
            "report": () => "switch_twice held=" + switchTwiceTimer.held
                            + " writes=" + (repoTab.writeSeq - switchTwiceTimer.writesBefore)
                            + " branch=" + worktree.branch
                            + " log=" + page.commandsOpen })
    }
    // The page stays through the restand (`Hub::restand_tab`), so this driver reads the answer. Either end ends the
    // run: a build that raised a bar instead never moves the tab, and `bar=` is what it fails on.
    PageStage {
        id: switchHeldTimer
        name: "switch_held"
        property string wanted: ""
        function stood() {
            return switchHeldTimer.wanted !== ""
                && repoTab.repoPath.toLowerCase() === switchHeldTimer.wanted.toLowerCase()
        }
        onTriggered: switchHeldTimer.complete({
            "name": "switch_held_ended",
            "when": () => ({
                "ended": graphPane.askCard.settled || (switchHeldTimer.stood() && PageSettled.settled(page)) }),
            "report": () => "switch_held stood=" + switchHeldTimer.stood()
                            + " bar=" + graphPane.askCard.settled
                            + " wanted=" + switchHeldTimer.wanted
                            + " where=" + repoTab.repoPath })
    }
    // Waited on all the way down (`AskBar.settled`): while it opens, the bar is a red line with no words in it.
    PageStage {
        id: switchStoppedTimer
        name: "switch_stopped"
        property bool go: false
        function line() {
            return "switch_stopped code=" + graphPane.askCode
                   + " accept=" + graphPane.askCard.accept
                   + " hold=" + graphPane.askHold
                   + " bang=" + graphPane.askAlert
                   + " op=" + worktree.opCommand
                   + " branch=" + worktree.branch
                   + " log=" + page.commandsOpen
        }
        onTriggered: {
            const op = switchStoppedTimer
            if (op.at("start")) {
                if (!op.go) {
                    op.complete({ "name": "switch_stopped_ready",
                                  "when": () => ({ "ready": graphPane.askCard.settled }),
                                  "report": () => op.line() })
                    return
                }
                if (!Awaited.all("switch_stopped_ready", { "ready": graphPane.askCard.settled }))
                    return
                // The bar as it stopped, before the answer takes it down.
                if (!op.advance("stopped", () => op.line()))
                    return
            }
            // Held or clicked, as the bar asks — a held pill ignores a click, a click pill has no hold. Each way in
            // arms its own watch; a press that did not take is pressed again on the next beat.
            if (!op.input("answered", () => {
                    if (graphPane.askHold)
                        return driver.holdToEnd(graphPane)
                    return driver.pressWrite("answer-ask", () => {
                        page.answerRowAsk()
                        return true
                    })
                }))
                return
            op.stop()
            switchStoppedLandedTimer.start()
        }
    }
    // Where the answer put the reader: past the write's answer, and the stash count waited for (`switch-lands`).
    PageStage {
        id: switchStoppedLandedTimer
        name: "switch_stopped_landed"
        /// The stash count waited for, off the run's `<branch>[:<stashes>]`; -1 where none is named.
        readonly property int stashes: {
            const leave = Harness.autoActArg.split(":")
            const ours = ["switch-stopped-go", "switch-conflicted-go"].indexOf(Harness.autoAct) >= 0
            return ours && leave.length > 1 ? Number(leave[1]) : -1
        }
        onTriggered: switchStoppedLandedTimer.complete({
            "name": "switch_stopped_landed_ready",
            "when": () => ({
                "written": driver.wroteAndSettled(),
                "done": worktree.opCommand === "",
                "shut": graphPane.askCard.shut,
                "stashes": switchStoppedLandedTimer.stashes < 0
                    || stashesModel.total === switchStoppedLandedTimer.stashes,
                "page": PageSettled.settled(page) }),
            "report": () => "switch_stopped_landed branch=" + worktree.branch
                            + " op=" + worktree.opCommand
                            + " conflicts=" + worktree.conflictCount
                            + " stashes=" + stashesModel.total
                            + " wanted=" + switchStoppedLandedTimer.stashes
                            + " log=" + page.commandsOpen })
    }
    PageStage {
        id: switchMarkTimer
        name: "switch_mark"
        property string want: ""
        // `indent=`: without the menu's hold column the `!` draws over the menu's edge (`AppMenu.holdIndent`).
        onTriggered: switchMarkTimer.complete({
            "name": "switch_mark_ready",
            "when": () => ({ "ready": refMenu.opened }),
            "report": () => "switch_mark offered=" + refSwitchItem.offered
                            + " asks=" + refSwitchItem.asks
                            + " want=" + switchMarkTimer.want
                            + " indent=" + (refMenu.holdIndent > 0) })
    }
    /// `set-upstream…` (`UpstreamFlow`): the form loads as the bar opens, so nothing is answered before
    /// `AskBar.settled`.
    PageStage {
        id: upstreamAskTimer
        name: "upstream_ask"
        property bool answers: false
        /// Answers with Enter in the name box instead of the pill (`UpstreamFlow.enterBranch`).
        property bool byKey: false
        /// Drops the name box's list first, through the press's own door (`UpstreamFlow.openBranches`) — no
        /// injected click reaches a popup offscreen.
        property bool lists: false
        /// Whether the run writes a name into the box at all — `wantName` may be the empty one.
        property bool types: false
        property string wantName: ""
        // `there=` is whether the answered name exists here — it drives the bar's line, not the pill (a name not here
        // is an answer: デザイン規約 §ブランチが測られる相手を決める). `unsetting=` / `pill=`: an emptied name turns the
        // question into taking the upstream off, the pill read off the bar itself. `rows=` / `open=`: an empty list and
        // an unplumbed one frame alike. `says=`: the bar's line, read off the bar too — last, being words.
        function line() {
            return "upstream branch=" + upstreamFlow.branch
                   + " remote=" + upstreamFlow.remote
                   + " rows=" + upstreamFlow.branches.length
                   + " open=" + upstreamFlow.branchesOpen()
                   + " name=" + upstreamFlow.branchName
                   + " there=" + upstreamFlow.targetIsThere
                   + " answerable=" + graphPane.askAnswerable
                   + " unsetting=" + upstreamFlow.unsetting
                   + " pill=" + graphPane.askCard.accept
                   + " presses=" + graphPane.askCard.holdPresses
                   + " says=" + graphPane.askCard.detail
        }
        /// The bar settled with what the run asked of it. The popup opens a turn after the press
        /// (`AppCombo.pressField`), so the popup itself is read.
        function asked() {
            return {
                "ready": graphPane.askCard.settled,
                "open": !upstreamAskTimer.lists || upstreamFlow.branchesOpen(),
                "answerable": !upstreamAskTimer.answers || graphPane.askAnswerable }
        }
        onTriggered: {
            const op = upstreamAskTimer
            if (!op.at("said")) {
                if (!Awaited.all("upstream_ask_ready", { "ready": graphPane.askCard.settled }))
                    return
                if (op.types && op.at("start")) {
                    op.input("typed", () => {
                        upstreamFlow.setBranchName(op.wantName)
                    })
                    return
                }
                if (op.lists && !op.at("dropped")) {
                    // Gone in only if the door was there; a form without its name box yet is asked again next tick.
                    op.input("dropped", () => upstreamFlow.openBranches())
                    return
                }
                if (!op.answers) {
                    op.complete({ "name": "upstream_ask_set", "when": () => op.asked(), "report": () => op.line() })
                    return
                }
                if (!Awaited.all("upstream_ask_set", op.asked()))
                    return
                // The bar as it stands, before the answer takes it down: the run's claim, kept for the write barrier
                // that completes it (judged before the press — whether the question was answerable when answered).
                if (!op.advance("said", () => op.line()))
                    return
                op.witness("bar", op.line())
                driver.reportAtCompletion(() => upstreamAskTimer.witnessed("bar"))
            }
            // The gesture a person is given: a held pill is not answered by a click, nor by a key (a run of `-enter`
            // on one waits out the watchdog by rule). A press that did not take is pressed again on the next beat.
            if (!op.input("answered", () => {
                    if (graphPane.askHold && !op.byKey)
                        return driver.holdToEnd(graphPane)
                    return driver.pressWrite("answer-ask", () => {
                        if (op.byKey)
                            return upstreamFlow.enterBranch()
                        page.answerRowAsk()
                        return true
                    })
                }))
                return
            op.stop()
            upstreamShutTimer.start()
        }
    }
    // The bar goes back up as the question is answered: the write barrier is waited from there, so the picture is not
    // of the bar half way up (`AskBar.shut`).
    PageStage {
        id: upstreamShutTimer
        name: "upstream_shut"
        order: ["shut"]
        onTriggered: {
            if (!Awaited.all("upstream_shut_ready", { "shut": graphPane.askCard.shut }))
                return
            if (!upstreamShutTimer.advance("shut"))
                return
            upstreamShutTimer.stop()
            writeBarrier.start()
        }
    }
    /// `unset-upstream[-go]`: the card goes up once nothing is out (a card raised under a write greys its rows), the
    /// row is read off it, and `-go` presses it through to the listing the write's refresh publishes.
    PageStage {
        id: unsetUpstreamTimer
        name: "unset_upstream"
        order: ["opened"]
        property string branch: ""
        property bool onChip: false
        property bool goes: false
        /// The card the run stands up: the entrance's own BRANCH card.
        function card() {
            return unsetUpstreamTimer.onChip ? driver.commitBranchCard : refBranchCard
        }
        // The shape is fixed as the card opens: seat, chip, what it names, the plain delete's presses and ink.
        function line() {
            const row = unsetUpstreamTimer.card().unsetUpstreamItem
            return "unset_upstream where=" + (unsetUpstreamTimer.onChip ? "chip" : "row")
                   + " offered=" + row.offered
                   + " code=" + row.code
                   + " presses=" + row.holdPresses
                   + " plain=" + (row.holdTone === Theme.textPrimary)
                   + " text=" + row.text
        }
        onTriggered: {
            const op = unsetUpstreamTimer
            if (op.at("start")) {
                // The precondition is read here and nowhere else (app-ui.md §UI 自動化).
                if (!Awaited.all("unset_upstream_live", { "live": driver.menusLive() }))
                    return
                op.input("opened", () => {
                    const oid = branchesModel.oidOfName(op.branch)
                    if (op.onChip) {
                        page.openRowMenu(oid, { "kind": "branch", "name": op.branch })
                        driver.commitMenu.openSub(driver.commitBranchCard)
                    } else {
                        page.openRefMenu("branch", op.branch, op.branch, oid)
                        refMenu.openSub(refBranchCard)
                    }
                })
                return
            }
            // Read a beat after the card went up.
            if (!op.goes || !op.card().unsetUpstreamItem.offered) {
                op.complete({ "name": "unset_upstream_card", "when": () => ({}), "report": () => op.line() })
                return
            }
            // Said before the press, which closes the card (`RefBranchMenu`'s `onHeld`); the landing presses it.
            op.say(() => op.line())
            op.stop()
            unsetLandedTimer.start()
        }
    }
    // The row's press, then git's answer, judged on the branch's own row: the listing the write's refresh publishes
    // names no upstream for it, listed or gone (`NavSectionModel.upstreamNamed`).
    PageStage {
        id: unsetLandedTimer
        name: "unset_landed"
        order: ["pressed"]
        onTriggered: {
            const op = unsetLandedTimer
            if (op.at("start")) {
                // The row closes the card itself (`RefBranchMenu`'s `onHeld`). A row out of reach takes no press and
                // answers false (`holdToEnd`): pressed again on the next beat.
                op.input("pressed", () => driver.holdToEnd(unsetUpstreamTimer.card().unsetUpstreamItem))
                return
            }
            op.complete({
                "name": "unset_landed_ready",
                "when": () => ({
                    "ready": driver.wroteAndSettled(),
                    "page": PageSettled.settled(page),
                    "unnamed": branchesModel.upstreamNamed(unsetUpstreamTimer.branch) === "" || repoTab.writeRefused }),
                "report": () => "unset_upstream_landed refused=" + repoTab.writeRefused
                                + " upstream=" + branchesModel.upstreamNamed(unsetUpstreamTimer.branch) })
        }
    }
}
