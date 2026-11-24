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
    readonly property var remotesModel: driver.remotesModel
    readonly property var stashesModel: driver.stashesModel
    readonly property var graphPane: driver.graphPane
    readonly property var sidebarPane: driver.sidebarPane
    readonly property var refMenu: driver.refMenu
    readonly property var refBranchCard: driver.refBranchCard
    readonly property var refUpstreamItem: driver.refUpstreamItem
    readonly property var refSwitchItem: driver.refSwitchItem
    readonly property var upstreamFlow: driver.upstreamFlow
    readonly property var writeBarrier: driver.barrierWrite
    readonly property var wipPane: driver.wipPane

    /// Runs `act` if it is this family's and says whether it was; each verb belongs to one family (`AutoActDriver`).
    function run(act, arg) {
        if (act === "switch") {
            // Over a stopped operation or a tree with conflicts nothing is written, and the run waits out the
            // watchdog: those two trees are `switch-stopped` / `switch-conflicted`'s.
            page.switchTo(GitFacts.switchMove(page.refFacts.leadOf("branch", arg, ""), worktree.standing, true).checkout)
        } else if (act === "move-ask") {
            // The question a move raises landing on a remote branch whose local one holds commits of its own
            // (`move-branch` is the same road past the bar). Waited on at `AskBar.settled` like `switch-stopped`:
            // `dbl-remote` completes before the bar has opened.
            page.switchToRef("remote", arg)
            moveAskTimer.start()
        } else if (act === "ask-over-notice") {
            // A question asked while a report stands (デザイン規約 §答えの要らない報せ): it waits its turn behind the report, which
            // keeps Escape, and takes its place once the report is gone (`AskBar.held`, `BarTravel`). The report goes
            // up first, the order a hand meets them in (`RepoPage.startRowAsk` leaves a standing report where it is);
            // which report it is does not matter.
            page.showReport({ "kind": "update-rule", "remote": "origin", "name": "main", "fact": "" })
            askOverNoticeTimer.ref = arg === "" ? "origin/main" : arg
            askOverNoticeTimer.start()
        } else if (act === "ask-sweep") {
            // The `move-ask` question, swept: while the bar stands over the list it is the only place its branch,
            // remote or folder is written (規約 §右のペインの字は掴める). Needs `--preset diverged`, where the question
            // rises as it does for `move-ask`.
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
            // in no bar. Needs `--preset worktrees`: on a tree where no worktree holds the branch it simply moves. No
            // `-go` — nothing moves; the row's words (`Open <mark><worktree>`) are `switch-mark`'s
            // (`<branch>:plain --preset worktrees`).
            switchHeldTimer.wanted = sidebarPane.worktreesModel.worktreeHolding(arg)
            page.switchToRef("branch", arg)
            switchHeldTimer.start()
        } else if (act === "switch-stopped" || act === "switch-stopped-go"
                   || act === "switch-conflicted" || act === "switch-conflicted-go") {
            // The question a move raises when something is in its way (an operation standing, or an unmerged index),
            // and with "-go" the gesture that answers it; only the bar differs. Entered by `switchToRef`, where a
            // hand arrives — a build that gates only `switchTo` fails here. The argument is `<branch>[:<stashes>]`, as
            // for `switch-lands`, and required. Read as a pair, the bar's two shapes: `feature/clash:1 --preset
            // cherry-pick-conflict` (`--quit` and a stash: warning, a click, chip `stash`, landing with one stash) and
            // `main --preset rebase-conflict` (`--abort`: danger, a hold, chip `rebase --abort`). On a preset with no
            // operation standing the switch just moves. `switch-conflicted[-go]` is the third shape:
            // `feature/clash[:1] --preset cherry-pick-quit`, no operation, the index unmerged. The bar's line says
            // only that it came down; whether the operation went and the move followed is the landing's line, and
            // `log=false` (no error raised) is only in the lines. `switch-conflicted main` presses the branch already
            // out: a move never sent asks nothing, so the press is turned away (`pressed=false`) and no bar comes.
            switchStoppedTimer.go = act.endsWith("-go")
            // "-go" says the bar as it stopped, then answers it.
            switchStoppedTimer.order = switchStoppedTimer.go ? ["stopped", "answered"] : []
            switchStoppedTimer.pressed = page.switchToRef("branch", arg.split(":")[0]) !== false
            switchStoppedTimer.start()
        } else if (act === "switch-cleared") {
            // The question a blocked move raised, the way cleared under it by a press of this window that does not walk
            // away from it, and the same bar asking the plain switch (`LeaveFlow`). `<branch>:resolve --preset
            // cherry-pick-quit` marks the unmerged index resolved and the uncommitted row stays; `<branch>:abort
            // --preset rebase-conflict` holds the exit card's `--abort` to its end, the tree empties and the page lands
            // on HEAD by itself (`RepoPage.activateRow`'s `followsTree`) — the bar has to stand past that landing.
            const want = arg.split(":")
            switchClearedTimer.how = want.length > 1 ? want[1] : ""
            switchClearedTimer.order = ["asked", "cleared"]
            // The exit card is the uncommitted face's: the run reads from it, as a hand reaches it.
            page.showWip()
            switchClearedTimer.pressed = page.switchToRef("branch", want[0]) !== false
            switchClearedTimer.start()
        } else if (act === "switch-remote") {
            page.switchToRef("remote", arg)
        } else if (act === "switch-remote-twice") {
            // The same chip pressed twice: ungated, the second `switch --create` is refused (the branch exists).
            // Both presses go out in one turn — `busyCount` only rises once the queue starts the write, so a tick
            // between them would be turned away by the busy gate and prove nothing. The argument is a remote branch
            // with no local one (`origin/feature/remote-only` on `basic`).
            switchTwiceTimer.branch = remotesModel.landsOn("remote", arg)
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
            // `gone` picks nothing: once the bar stands, the old upstream's remote-tracking ref is taken away from
            // outside, as a prune takes it, and the bar is read as it turned (`RenameCarryFlow.gone`) — "-go" then
            // answers with what it offers first, the upstream's setting alone.
            const want = arg.split(":")
            const goes = act.endsWith("-go")
            const gone = want.length > 1 && want[1] === "gone"
            // "-go" with no answer named takes `add`: `replace` would delete the remote's HEAD branch, which git
            // refuses (that write is `replace-remote-go`'s). "-tip" points at the pill; only `replace` has a tip.
            const points = act.endsWith("-tip")
            localUpstreamAskTimer.pick = points ? "replace" : want.length > 1 ? want[1] : (goes ? "add" : "")
            localUpstreamAskTimer.answers = goes
            localUpstreamAskTimer.points = points
            localUpstreamAskTimer.gone = gone
            // The pick, then the hand on the pill and its tip standing, then the bar said as it stands and answered —
            // each where asked.
            localUpstreamAskTimer.order = (gone ? ["removed", "reread", "turned"]
                                           : localUpstreamAskTimer.pick !== "" ? ["picked"] : [])
                .concat(points ? ["pointed", "tipped"] : [])
                .concat(goes ? ["said", "answered"] : [])
            const local = worktree.branch
            // Read before the rename, as `RepoPage.renameRow` reads its own: afterwards the row answers to the new
            // name.
            localUpstreamAskTimer.spokenFor = branchesModel.upstreamOf(local)
            localUpstreamAskTimer.lost = branchesModel.upstreamGoneOf(local) !== ""
            localUpstreamAskTimer.comes = !gone ? ""
                : GitFacts.remoteRefBeside(localUpstreamAskTimer.spokenFor, want[0], repoTab.remoteNames)
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
            // The mark the `switch` row wears when its press would raise a question. Argument `<branch>:asks|plain`
            // (required: it names the side expected); the report judges it — a 16px mark in a full window is not for
            // the picture (verify-ui「目視は等倍以上で」). `asks` needs an operation standing: `feature/clash:asks
            // --preset cherry-pick-conflict` (a branch held by a worktree wears no mark, so not `worktrees`);
            // `plain` is `feature/topic-a:plain --preset basic`, and on `--preset worktrees` the third look — the row
            // says `Open <mark><worktree>`, `indent=false`.
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
    /// on its way has no Escape shortcut yet — `open` enables it), the question standing behind it all the way up,
    /// and both after the press — the report gone and the question all the way down. The picture cannot tell one
    /// live Escape from two, nor a question waiting from none, nor one standing in the report's place at its height
    /// (`in_place`), so the claims are in the line.
    PageStage {
        id: askOverNoticeTimer
        name: "ask_over_notice"
        order: ["asked", "escaped"]
        /// The remote ref the question is raised on, as `move-ask` takes it.
        property string ref: ""
        // What was read at the press is its witnesses — which bars were on screen (`askShown`, `noticeStood`), whether
        // the question stood off screen (`askHeld`), and which held Escape (`askEsc`, `noticeEsc`) — kept because the
        // press is what takes the report away.
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
            const askStood = op.witnessed("askShown") || op.witnessed("askHeld")
            const noticeStood = op.witnessed("noticeStood")
            return "ask_over_notice shown=" + op.naming(op.witnessed("askShown"), noticeStood)
                   + " waiting=" + op.naming(op.witnessed("askHeld"), false)
                   + " holds=" + op.naming(op.witnessed("askEsc"), op.witnessed("noticeEsc"))
                   + " went=" + op.naming(askStood && !graphPane.askCard.open, noticeStood && !page.noticeCard.open)
                   + " then=" + op.naming(graphPane.askCard.settled, page.noticeCard.open)
                   + " in_place=" + op.witnessed("inPlace")
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
                // Standing, and still all the way up: a build that let it down over the report is caught at the press.
                if (!Awaited.all("ask_over_notice_question", { "question": graphPane.askCard.open
                                                                           && !op.moving(graphPane.askCard) }))
                    return
                op.input("escaped", () => {
                    op.witness("askShown", graphPane.askCard.shown)
                    op.witness("askHeld", graphPane.askCard.open && graphPane.askCard.held)
                    op.witness("noticeStood", page.noticeCard.open)
                    const askEsc = op.witness("askEsc", graphPane.askCard.escapes)
                    const noticeEsc = op.witness("noticeEsc", page.noticeCard.escapes)
                    const noticeHeight = page.noticeCard.height
                    // What Escape itself runs, on whichever bar holds it: a build that handed it to the wrong bar is
                    // judged on what that bar did, and one that handed it to neither presses nothing.
                    if (askEsc)
                        graphPane.askCard.dismiss()
                    else if (noticeEsc)
                        page.noticeCard.dismiss()
                    // Read in the press's own turn, as the frame it draws will stand: the question at the report's
                    // height, and the graph's place kept through the swap (`RepoPage.hideNotice`, `PlaceKeep`).
                    op.witness("inPlace", graphPane.askCard.implicitHeight === noticeHeight
                                          && graphPane.view.placeKeep.keeping)
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
        /// Which of the three answers the run picks — or `gone`, which picks none —, empty for the bar as it comes
        /// down.
        property string pick: ""
        /// Whether the picked answer is then given to the bar.
        property bool answers: false
        /// Whether a hand goes on the pill (`-tip`) — a beat after the pick, so the tip sits against the picked word.
        property bool points: false
        /// Whether the old upstream's row is taken away while the bar stands (`:gone`), and that row's name.
        property bool gone: false
        property string spokenFor: ""
        /// Whether that upstream was gone from the list before the rename (`--preset gone-out`): the bar comes down
        /// on the two answers.
        property bool lost: false
        /// …and the remote branch the new name would be (`:gone`).
        property string comes: ""
        /// What the answer to the turned bar wrote, read off the listing behind it: the upstream the branch names
        /// now — no row answers to it, so it is the gone one's seat —, and whether anything went up.
        function written() {
            return !localUpstreamAskTimer.gone ? ""
                 : " upstream=" + branchesModel.upstreamGoneOf(worktree.branch)
                   + " sent=" + (acts.remotesModel.oidOfName(localUpstreamAskTimer.comes) !== "")
        }
        function line() {
            return driver.carryWords("branch", localUpstreamAskTimer.pick)
                   + (localUpstreamAskTimer.points ? " tip=" + graphPane.askCard.tipStanding : "")
                   // The line under the heading is the gone pair's own: `rows=` alone could be a list not yet in.
                   + (localUpstreamAskTimer.gone || localUpstreamAskTimer.lost
                      ? " line=" + (graphPane.askDetail !== "") : "")
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
                if (op.gone && op.at("start")) {
                    // As a terminal would, with nothing of the window's asked: the ref the old upstream's row is.
                    op.input("removed", () => Harness.runGit(
                        repoTab.repoPath,
                        ["update-ref", "-d", "refs/remotes/" + op.spokenFor].join("\n")).startsWith("0 "))
                    return
                }
                if (op.gone && op.at("removed")) {
                    // The read that notices it, asked for once: nothing of the window's wrote.
                    op.input("reread", () => {
                        repoTab.refreshQuick()
                        return true
                    })
                    return
                }
                if (op.gone && op.at("reread")) {
                    // The row out of the list, and the bar turned with it and at rest again.
                    if (!Awaited.all("local_upstream_gone", {
                        "row": acts.remotesModel.oidOfName(op.spokenFor) === "",
                        "line": graphPane.askDetail !== "",
                        "bar": graphPane.askCard.settled })
                        || !op.advance("turned", () => "rename_carry step=turned gone=" + op.spokenFor))
                        return
                }
                if (op.pick !== "" && !op.gone && op.at("start")) {
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
                driver.reportAtCompletion(() => localUpstreamAskTimer.witnessed("bar")
                                                + localUpstreamAskTimer.written())
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
                && GitFacts.samePath(repoTab.repoPath, switchHeldTimer.wanted)
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
        /// The press's own answer (`switchToRef`): false is a press turned away, which raised no bar to wait for.
        property bool pressed: false
        function line() {
            return "switch_stopped code=" + graphPane.askCode
                   + " accept=" + graphPane.askCard.accept
                   + " hold=" + graphPane.askHold
                   + " bang=" + graphPane.askAlert
                   + " op=" + worktree.opCommand
                   + " branch=" + worktree.branch
                   + " log=" + page.commandsOpen
                   + " pressed=" + switchStoppedTimer.pressed
                   // `open`, raised as the question is asked (`GraphPane.startAsking`), not `settled`: a bar on its way
                   // down still reads unsettled.
                   + " bar=" + graphPane.askCard.open
        }
        onTriggered: {
            const op = switchStoppedTimer
            if (op.at("start")) {
                if (!op.go) {
                    op.complete({ "name": "switch_stopped_ready",
                                  "when": () => ({ "ready": !op.pressed || graphPane.askCard.settled }),
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
    // The way cleared under the standing question from this window (`switch-cleared`): the bar as it stands once
    // nothing is in the way, read past the write and past the landing an emptied tree makes.
    PageStage {
        id: switchClearedTimer
        name: "switch_cleared"
        /// `resolve` (the bucket's `Mark all resolved`) or `abort` (the exit card's `--abort`).
        property string how: ""
        /// The press's own answer (`switchToRef`): false is a press turned away, which raised no bar.
        property bool pressed: false
        function line() {
            return "switch_cleared bar=" + graphPane.askCard.open
                   + " code=" + graphPane.askCode
                   + " neutral=" + graphPane.askNeutral
                   + " hold=" + graphPane.askHold
                   + " op=" + worktree.opCommand
                   + " detail=" + graphPane.askDetail
        }
        onTriggered: {
            const op = switchClearedTimer
            if (op.at("start")) {
                if (!Awaited.all("switch_cleared_asked", { "asked": op.pressed && graphPane.askCard.settled }))
                    return
                // The bar as the move raised it, before the way clears.
                if (!op.advance("asked", () => op.line()))
                    return
            }
            if (op.at("asked")) {
                if (!op.input("cleared", () => driver.pressWrite("switch-cleared", () => {
                        if (op.how === "abort")
                            return wipPane.completeOpExit("--abort")
                        repoTab.stageConflicted()
                        return true
                    })))
                    return
            }
            op.complete({
                "name": "switch_cleared_ready",
                "when": () => ({
                    "written": driver.wroteAndSettled(),
                    "clear": !worktree.movesBlocked,
                    "landed": !page.pageLanding && (op.how !== "abort" || !worktree.wipRowStands),
                    "bar": !graphPane.askCard.open || graphPane.askCard.settled,
                    "page": PageSettled.settled(page) }),
                "report": () => op.line() })
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
    ///
    /// The forms. Bare `set-upstream <branch>` is the question as it opens; its claim (`there=true`) needs a branch
    /// whose counterpart is here (`feature/topic-a`) — `feature/new-thing` opens on the default remote and its own
    /// name, `there=false`. `:missing` types a name not here, answerable all the same: only the line changes, frame and
    /// pill stay plain. `-list` drops the box's list (what it holds is overlay.png's); the remote the question opens on
    /// needs fetched branches, or `open=true` never comes and the run waits out its watchdog. `-go` answers once
    /// answerable (`set-upstream-go 'feature/new-thing:carried' --preset unpublished`); `-enter` gives the same answer
    /// with Enter in the box (デザイン規約 §立っている質問は 1 か所で聞く). A bare trailing `:` empties the box: on a
    /// branch with an upstream (`feature/mine: --preset others-behind`) the question turns into `Unset`, which `-go`
    /// holds through its two presses (the picture: the row's ↑ and cloud gone); on one without (`scratch` / `scratch:`)
    /// it stays the `Set` question, unpressable once emptied — read the pair.
    PageStage {
        id: upstreamAskTimer
        name: "upstream_ask"
        property bool answers: false
        /// Answers with Enter in the name box instead of the pill (`UpstreamFlow.enterBranch` raises the box's
        /// `accepted`; that a real Enter raises it is `tst_appcombo`'s), so the write barrier is the whole claim.
        property bool byKey: false
        /// Drops the name box's list first, through the press's own door (`UpstreamFlow.openBranches`) — no
        /// injected click reaches a popup offscreen.
        property bool lists: false
        /// Whether the run writes a name into the box at all — `wantName` may be the empty one.
        property bool types: false
        property string wantName: ""
        // `there=` is whether the answered name exists here — it drives the bar's line, not the pill (a name not here
        // is an answer: `UpstreamFlow.targetIsThere`). `unsetting=` / `pill=`: an emptied name turns the
        // question into taking the upstream off, the pill read off the bar itself; `presses=` its count (0 = a click).
        // `rows=` (the chosen remote's branches held here) / `open=`: an empty list and an unplumbed one frame alike.
        // `says=`: the bar's line, read off the bar too — last, being words.
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
    /// row is read off it, and `-go` presses it through to the listing the write's refresh publishes. A gone upstream
    /// (`[gone]`, `release-1.2 --preset gone`) is taken off the same way; the picture is the row's warning cloud gone.
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
        /// The upstream the listing names for the branch now, listed or gone; empty for none.
        function upstream() {
            return branchesModel.branchUpstream("branch", unsetUpstreamTimer.branch).upstream
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
    // names no upstream for it, listed or gone (`NavSectionModel.branchUpstream`'s `upstream`).
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
                    "unnamed": unsetUpstreamTimer.upstream() === "" || repoTab.writeRefused }),
                "report": () => "unset_upstream_landed refused=" + repoTab.writeRefused
                                + " upstream=" + unsetUpstreamTimer.upstream() })
        }
    }
}
