pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The things that stand over a page rather than in it: the find bar, the command log, the settings dialog and
/// the avatar store.
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
    readonly property var graphModel: driver.graphModel
    readonly property var detailsModel: driver.detailsModel
    readonly property var graphPane: driver.graphPane
    readonly property var detailsPane: driver.detailsPane
    readonly property var renderedBarrier: driver.barrierRendered

    /// Runs `act` if it is one of this family's, and says whether it was. The families are asked in turn
    /// and the first to know a verb runs it — no verb is named by two of them (`AutoActDriver`).
    function run(act, arg) {
        if (act === "open-picker") {
            page.openRepositoryPicker()
        } else if (act === "settings") {
            page.settingsDialogRequested()
            AppBackend.setAutoFetchMinutes(Number(arg))
        } else if (act === "settings-git") {
            // The screen's other category, entered by the door the conflict menu and the app menu both use.
            page.gitSettingsRequested()
        } else if (act === "avatar-rest" || act === "avatar-hover"
                   || act === "avatar-assign" || act === "avatar-badge") {
            // The first ordinary commit — row 0 is WIP.
            page.activateRow(graphModel.oidAt(1))
            if (act === "avatar-assign") {
                detailsPane.avatarPointedAt = true
                avatarAssignTimer.start()
            } else if (act === "avatar-badge") {
                avatarBadgeTimer.start()
            } else {
                // The picture is of the details pane, which arrives a git
                // subprocess later — the same wait avatar-badge takes; a
                // shot at the ask frames a loading pane.
                avatarShownTimer.hovers = act === "avatar-hover"
                avatarShownTimer.start()
            }
        } else if (act === "avatar-settings" || act === "avatar-combo"
                   || act === "avatar-row-lit" || act === "avatar-remove") {
            // Each run starts with an empty store, so a picture to look at has to be filed first — the argument is the
            // one to file. The card the four of them are about is the window's, and so is their completion
            // (`WindowAutoActDriver`); all that happens here is the filing and the asking.
            page.activateRow(graphModel.oidAt(1))
            if (arg !== "")
                avatarSeedTimer.start()
            else
                page.settingsDialogRequested()
        } else if (act === "find" || act === "find-next" || act === "find-prev") {
            // The key cannot be pressed from here; assigning the text runs the same search a keystroke runs.
            page.startFind()
            if (arg !== "")
                graphPane.findCard.query = arg
            if (act === "find-next")
                graphPane.findNext()
            else if (act === "find-prev")
                graphPane.findPrevious()
            // `width` and `cap` are the two halves of the rule the long queries are here to check: the card may grow,
            // and it may not reach past a subject's first character.
            Harness.report("find open=" + graphPane.findCard.open
                              + " query=" + graphPane.findCard.query
                              + " matches=" + graphPane.findCard.matches
                              + " at=" + graphPane.findCard.atMatch
                              + " row=" + graphPane.view.currentIndex
                              + " selected=" + page.selectedOid.substring(0, 7)
                              + " width=" + Math.round(graphPane.findCard.width)
                              + " cap=" + Math.round(graphPane.width - graphPane.subjectTextX)
                              + " clears=" + graphPane.findCard.findClears)
            findSettled.restart()
        } else if (act === "find-drop") {
            // The card standing while a press lands somewhere else. Presses cannot be injected (verify-ui スキル), so
            // this enters where `FocusRelease.pressedAway` enters and gives the press no place of its own — which is
            // "it landed on none of ours", the answer that matters here. The argument is what is typed in first:
            // nothing, and the card goes with the press; a query, and the card stays because the query is what there
            // would be to lose (規約 §コミットを探す).
            page.startFind()
            if (arg !== "")
                graphPane.findCard.query = arg
            page.releasePressedAway(null)
            findDropSettled.restart()
        } else if (act === "commands" || act === "commands-select" || act === "commands-copy"
                   || act === "commands-sweep") {
            // Stage and unstage so the log has something in it.
            repoTab.stageAll()
            repoTab.unstageAll()
            page.toggleCommands()
            if (act === "commands-sweep")
                commandsSweepTimer.start()
            else if (act !== "commands")
                commandsPickTimer.start()
        } else if (act === "commands-fail" || act === "commands-clear" || act === "commands-fail-shut") {
            // A real refusal in git's own words, raising the panel by itself. The clearing verb starts from the same
            // failure (`Main` waits for it, presses Clear, and reads the band); the shutting one takes the panel back
            // down with the `>_` instead, which leaves the error line standing and the mark red.
            if (act === "commands-fail-shut" && arg === "fold")
                page.foldByHand(true)
            repoTab.checkoutBranch("pg-no-such-branch", false)
            if (act === "commands-fail-shut")
                commandsShutTimer.start()
        } else {
            return false
        }
        return true
    }
    // The details pane the resting/hovered face sits in, waited to settle before the shot; the hover mark
    // goes on once the face it marks is there.
    SampleTimer {
        id: avatarShownTimer
        property bool hovers: false
        onTriggered: {
            if (!driver.cardSettled)
                return
            avatarShownTimer.stop()
            if (avatarShownTimer.hovers)
                detailsPane.avatarPointedAt = true
            renderedBarrier.begin()
        }
    }
    // Automation: the details have to land before the author card can be worked, since it is that author the picture is
    // filed against.
    SampleTimer {
        id: avatarAssignTimer
        onTriggered: {
            if (detailsModel.authorEmail === "")
                return
            avatarAssignTimer.stop()
            AppBackend.assignAvatar(detailsModel.authorEmail,
                                    detailsModel.authorName,
                                    Harness.autoActArg)
            avatarReportTimer.start()
        }
    }
    SampleTimer {
        id: avatarBadgeTimer
        onTriggered: {
            if (!driver.cardSettled)
                return
            avatarBadgeTimer.stop()
            detailsPane.avatarClicked()
            renderedBarrier.begin()
        }
    }
    // What the graph did about the find bar, read after it finished doing it. The step down out from under the card is
    // animated, so the value in the same call stack as the verb is always the one before it moved — reporting that
    // would be reporting the intent, which the line above already carries as `clears=`.
    SampleTimer {
        id: findSettled
        onTriggered: {
            if (!graphPane.findCard.open)
                return
            findSettled.stop()
            Harness.report(
            "find_settled shift=" + Math.round(graphPane.findShift))
            driver.complete()
        }
    }
    // The card fades in and out, so both halves of `find-drop` are photographed at one end of that fade or the other:
    // caught in between, the card that stayed and the card that went away frame the same.
    SampleTimer {
        id: findDropSettled
        onTriggered: {
            if (graphPane.findCard.opacity > 0 && graphPane.findCard.opacity < 1)
                return
            findDropSettled.stop()
            Harness.report("find_drop open=" + graphPane.findCard.open
                              + " shown=" + (graphPane.findCard.opacity > 0)
                              + " query=" + graphPane.findCard.query)
            driver.complete()
        }
    }
    // The store starts empty in every run, so the card's own verbs put a picture in it before opening on it.
    SampleTimer {
        id: avatarSeedTimer
        onTriggered: {
            if (detailsModel.authorEmail === "")
                return
            avatarSeedTimer.stop()
            AppBackend.assignAvatar(detailsModel.authorEmail,
                                    detailsModel.authorName,
                                    Harness.autoActArg)
            page.settingsDialogRequested()
        }
    }
    // The picture is read off disk asynchronously, so what the shot wants is a beat after the write rather than the
    // instant it returns.
    SampleTimer {
        id: avatarReportTimer
        onTriggered: {
            if (detailsModel.avatarUrl === "" && AppBackend.avatarError === "")
                return
            avatarReportTimer.stop()
            Harness.report(
            "avatar email=" + detailsModel.authorEmail
            + " details=" + (detailsModel.avatarUrl !== "")
            + " rows=" + graphModel.avatarRowCount()
            + " error=" + AppBackend.avatarError)
            driver.complete()
        }
    }
    // PGG_AUTO_ACT=commands-fail-shut: the mark's red with the panel out of the way, which is the state no other verb
    // can photograph — `commands-fail` leaves the panel standing over it and `commands-clear` takes the red away with
    // the rows. The press goes in at the `>_`'s own function rather than at `commandsOpen`, so a build where that
    // press stopped reaching the page waits here instead of passing.
    SampleTimer {
        id: commandsShutTimer
        property bool pressed: false
        onTriggered: {
            // Both halves are the refusal landing: red mark, panel raised by it. Read again after the press, the
            // panel's going away would bar the way to the report (規約 §UI 自動化の因果性).
            if (!commandsShutTimer.pressed) {
                if (!page.commandsWrong || !page.commandsShown)
                    return
                commandsShutTimer.pressed = true
                page.toggleCommands()
                return
            }
            if (page.commandsShown)
                return
            commandsShutTimer.stop()
            Harness.report("commands_shut wrong=" + page.commandsWrong
                              + " open=" + page.commandsShown
                              + " folded=" + page.sidebarCollapsed
                              + " mark=" + page.commandsMarkColor)
            renderedBarrier.begin()
        }
    }
    // PGG_AUTO_ACT=commands-select / commands-copy: a drag over the log, and the key that takes what it picked. The
    // drag runs from the head of the first row to the end of the last, which is the stretch that reaches all three of
    // a row's columns and more than one row — a drag inside one column would prove a fraction of the rule and read as
    // a pass.
    //
    // `commands-select` stops with the wash standing (the picture is the deliverable); `commands-copy` presses Ctrl+C
    // and reads back what went out, since the clipboard will not answer a headless run
    // (`ClipboardHelper.lastCopied`). **`perRow=` is the claim**: one line on the clipboard for every row in the
    // panel. Lines that begin with a tab are not counted — a failure brings git's own words down under it, and no run
    // owns which of its commands fail (reading a repository from inside a container answers `not a git repository` for
    // the working copy the tree really lives in, and that one refusal is three more lines: measured).
    //
    // The write barrier is in front of these rather than behind them (the same wait `dispatchFinished` makes for a
    // plain write act): the two writes that give the log something to hold are still queued when this starts, and a
    // tick that arrives before them drags over whatever a background read happened to leave. All of it is read in the
    // branch that acts and nowhere else (規約 §UI 自動化の因果性).
    SampleTimer {
        id: commandsPickTimer
        onTriggered: {
            if (repoTab.busyCount !== 0 || repoTab.writeSeq <= driver.writeSeqBefore)
                return
            // Two rows, because the fixture makes two writes and the drag is about crossing from one row to another:
            // the write barrier alone lets a tick through while an opening read is the only thing in the log, and a
            // drag inside one row proves the smaller half of the rule (measured — `rows=1` on both OS).
            if (!page.commandsShown || page.pageCommands.running || page.pageCommands.rowsHeld() < 2)
                return
            commandsPickTimer.stop()
            // The model's own count, not the panel's `(N)` — that one is the view's, and the view is a frame behind
            // the rows in the tick a press lands in (measured, 1 against 4 commands on the clipboard).
            const rows = page.pageCommands.rowsHeld()
            page.pickCommandText(0, 0, rows - 1, driver.pastLineEnd)
            if (Harness.autoAct === "commands-select") {
                // Two halves, because either one alone passes a broken run: `holds=` is the selection the model is
                // keeping, `worn=` is the rectangle every row is drawing of it. A log that holds one and wears none
                // photographs exactly like a log nobody dragged over (`CommandsPane.washTally`).
                Harness.report("commands_pick holds=" + (page.pageCommands.selectionText() !== "")
                                  + " " + page.commandWashTally() + " held=" + rows)
                renderedBarrier.begin()
                return
            }
            page.copyCommandText()
            const text = driver.clipboard.lastCopied
            const lines = text === "" ? [] : text.split("\n")
            const said = lines.filter(line => !line.startsWith("\t")).length
            Harness.report("commands_copy perRow=" + (rows > 0 && said === rows)
                              + " rows=" + rows + " said=" + said + " lines=" + lines.length)
            renderedBarrier.begin()
        }
    }
    // PGG_AUTO_ACT=commands-sweep: the same text, started on the ground under the last row instead of on a row — the
    // one place inside the panel's own frame where a press used to reach nothing
    // (規約 §git が言ったことを読む場所). The waits are `commands-select`'s, and the sweep is `details-sweep`'s:
    //
    // **Nine starts, not one.** A reach that worked from a single place in the ground is exactly the fault the right
    // pane's values shipped with, and the middle is the one place that hides it. Each start
    // is judged on its own, over a board cleared first — a run that read the selection once at the end would report
    // the last try and call the other eight green.
    SampleTimer {
        id: commandsSweepTimer
        /// Where the ground began at the previous sample, for the settle below.
        property string lastGeom: ""
        onTriggered: {
            if (repoTab.busyCount !== 0 || repoTab.writeSeq <= driver.writeSeqBefore)
                return
            if (!page.commandsShown || page.pageCommands.running || page.pageCommands.rowsHeld() < 2)
                return
            // **And the panel has to have stopped laying out.** The rows arrive a frame ahead of the view that draws
            // them, and a sweep aimed at ground that is about to be a row lands on neither.
            const geom = Math.round(page.commandsGroundTop) + "," + page.pageCommands.rowsHeld()
            if (geom !== commandsSweepTimer.lastGeom) {
                commandsSweepTimer.lastGeom = geom
                return
            }
            commandsSweepTimer.stop()
            let reach = 0
            let tries = 0
            const across = [0.05, 0.5, 0.95]
            const down = [0.05, 0.5, 0.95]
            for (let i = 0; i < across.length; i++) {
                for (let j = 0; j < down.length; j++) {
                    tries++
                    page.pageCommands.clearSelect()
                    if (page.sweepCommandGround(across[i], down[j])
                        && page.pageCommands.selectionText() !== "")
                        reach++
                }
            }
            // `ground=` is the run's own honesty: a panel whose log fills it has nowhere to sweep from, and a
            // `reach=0/9` off one is a fixture that stopped saying anything rather than a hand that stopped working.
            Harness.report("commands_sweep reach=" + reach + "/" + tries
                              + " ground=" + page.commandsHasGround
                              + " rows=" + page.pageCommands.rowsHeld())
            renderedBarrier.begin()
        }
    }
}
