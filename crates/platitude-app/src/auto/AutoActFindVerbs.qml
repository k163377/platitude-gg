pragma ComponentBehavior: Bound

import QtQuick
// For the attached `ToolTip.toolTip` alone (rules-refs/app-ui.md「attached 型は宣言元モジュールを import しないと」).
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

/// The things that stand over a page: the find bar, the command log, the settings dialog and
/// the avatar store.
// `Item` because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    /// `var`: typing it `AutoActDriver` would be circular — that file builds this one.
    required property var driver

    // The driver's own names, read once so the verbs can name them bare.
    readonly property var page: driver.page
    readonly property var repoTab: driver.repoTab
    readonly property var graphModel: driver.graphModel
    readonly property var detailsModel: driver.detailsModel
    readonly property var graphPane: driver.graphPane
    readonly property var detailsPane: driver.detailsPane
    readonly property var renderedBarrier: driver.barrierRendered

    /// Runs `act` if it is one of this family's, and says whether it was.
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
                   || act === "avatar-assign" || act === "avatar-badge"
                   || act === "avatar-tip") {
            // The first ordinary commit — row 0 is WIP.
            page.activateRow(graphModel.oidAt(1))
            if (act === "avatar-assign") {
                detailsPane.avatarPointedAt = true
                avatarAssignTimer.start()
            } else if (act === "avatar-badge") {
                avatarBadgeTimer.start()
            } else if (act === "avatar-tip") {
                avatarTipTimer.start()
            } else {
                // The details pane arrives a git subprocess later; a shot at the ask frames it loading.
                avatarShownTimer.hovers = act === "avatar-hover"
                avatarShownTimer.start()
            }
        } else if (act === "avatar-settings" || act === "avatar-combo"
                   || act === "avatar-row-lit" || act === "avatar-remove"
                   || act === "avatar-enter") {
            // The store starts empty each run, so the argument is filed first. The card and its completion are the
            // window's (`WindowAutoActDriver`).
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
            // `width` / `cap`: the card may grow, but stops at a subject's first character.
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
        } else if (act === "find-hint" || act === "find-hint-key") {
            // The note under the card waits for the typing to stop; `-key` holds the line's last character back and
            // presses it after the stop. `head:<n>` is the first n characters of HEAD's id — a fixture's ids differ
            // between the two OSes.
            const line = arg.startsWith("head:")
                         ? graphModel.oidAt(graphModel.headRow).substring(0, Number(arg.slice(5)))
                         : arg
            page.startFind()
            findHinted.key = act === "find-hint-key" ? line.slice(-1) : ""
            graphPane.findCard.query = line.slice(0, line.length - findHinted.key.length)
            findHinted.start()
        } else if (act === "find-scroll") {
            // The search drawn, then the list scrolled: rows come in on delegates handed on from rows just drawn
            // dimmed or lit, where one row's ink can carry into the next. `find` draws each row only once.
            page.startFind()
            graphPane.findCard.query = arg
            findScrolled.start()
        } else if (act === "band-find") {
            // Pressed at the mark itself (`TopBar.findNow`), not `RepoPage.startFind`, so a mark wired to nothing
            // fails. The mark is down while a plan stands over the graph: no press, nothing to latch.
            if (!driver.inputWent(page.pageBand.findNow()))
                return
            Harness.report("band_find open=" + graphPane.findCard.open)
            findSettled.restart()
        } else if (act === "find-drop") {
            // A press landing elsewhere, entered where `FocusRelease.pressedAway` enters, with no place of its own
            // (= on none of ours). The argument is typed first: empty, the card goes with the press; a query, it
            // stays (規約 §コミットを探す).
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
        } else if (act === "commands-fail" || act === "commands-clear" || act === "commands-fail-shut"
                   || act === "commands-escape") {
            // A real refusal in git's own words, which raises the panel by itself.
            if (act === "commands-fail-shut" && arg === "fold")
                page.foldByHand(true)
            repoTab.checkoutBranch("pgg-no-such-branch", false)
            if (act === "commands-fail-shut")
                commandsShutTimer.start()
            else if (act === "commands-escape")
                commandsEscapeTimer.start()
        } else {
            return false
        }
        return true
    }
    // The hover mark goes on once the face it marks is there.
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
    // The badge's sentence, up only after `Metrics.tipDelayMs`. Read off the shared instance: the address it names
    // arrives late (`%1`), and a picture cannot tell a filled sentence from an empty one.
    SampleTimer {
        id: avatarTipTimer
        onTriggered: {
            if (!driver.cardSettled)
                return
            avatarTipTimer.stop()
            detailsPane.avatarPointedAt = true
            avatarTipReport.start()
        }
    }
    SampleTimer {
        id: avatarTipReport
        onTriggered: {
            const tip = page.ToolTip.toolTip
            if (!tip.visible)
                return
            avatarTipReport.stop()
            Harness.report("avatar_tip tip=" + tip.visible
                + " named=" + (detailsModel.authorEmail !== ""
                               && tip.text.indexOf(detailsModel.authorEmail) >= 0)
                + " text=" + tip.text)
            driver.complete()
        }
    }
    // The picture is filed against the details' author, so the details land first.
    SampleTimer {
        id: avatarAssignTimer
        onTriggered: {
            if (detailsModel.authorEmail === "")
                return
            avatarAssignTimer.stop()
            AppBackend.assignAvatar(detailsModel.authorEmail,
                                    detailsModel.authorName,
                                    Harness.fileUrl(Harness.autoActArg))
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
    // Read once the card has faded in and the graph's step out from under it has landed: both are animated, and a shot
    // before the fade ends shows the row beneath through the card. `pin=` is HEAD's stand-in once the jump sent HEAD's
    // row off — `none`, `lit` or `dim`; one left lit photographs exactly like a match.
    SampleTimer {
        id: findSettled
        onTriggered: {
            const card = graphPane.findCard
            if (!card.open || card.opacity < 1 || graphPane.findShift !== (card.findClears ? card.height : 0))
                return
            findSettled.stop()
            const pin = graphPane.headPin
            Harness.report("find_settled shift=" + Math.round(graphPane.findShift)
                              + " pin=" + (!pin.visible ? "none" : pin.wordsOpacity < 1 ? "dim" : "lit"))
            driver.complete()
        }
    }
    // PGG_AUTO_ACT=find-scroll: the scroll waits for the grab callback of the frame that drew the search
    // (rules/app-ui.md §UI 自動化). `grabbed=` says it did — otherwise the scroll lands in the same turn as the query
    // and the run is `find` again. Two screens: a jump builds the incoming rows before letting go of the outgoing
    // ones, so drawn delegates are handed on only on the second.
    SampleTimer {
        id: findScrolled
        property string stage: "wait"
        property bool grabbed: false
        property bool moved: false
        function scroll() {
            const view = graphPane.view
            const before = view.contentY
            view.contentY = before + view.height
            view.contentY = before + 2 * view.height
            findScrolled.moved = view.contentY > before
            findScrolled.stage = "done"
        }
        onTriggered: {
            if (findScrolled.stage === "wait") {
                if (!graphPane.findCard.open || !graphModel.searching)
                    return
                findScrolled.stage = "drawing"
                findScrolled.grabbed = graphPane.view.grabToImage(() => findScrolled.scroll())
                if (!findScrolled.grabbed)
                    findScrolled.scroll()
                return
            }
            if (findScrolled.stage !== "done")
                return
            findScrolled.stop()
            const pin = graphPane.headPin
            Harness.report("find_scroll grabbed=" + findScrolled.grabbed + " moved=" + findScrolled.moved
                              + " pin=" + (!pin.visible ? "none" : pin.wordsOpacity < 1 ? "dim" : "lit")
                              + " matches=" + graphPane.findCard.matches)
            renderedBarrier.begin()
        }
    }
    // PGG_AUTO_ACT=find-hint / find-hint-key: waits on the card's own word that the typing stopped (`typingStopped`),
    // which a line with no note reaches all the same. `-key` then presses the held-back character and is shot on the
    // next frame — the one that keystroke took the note down in (規約 §コミットを探す).
    SampleTimer {
        id: findHinted
        property string key: ""
        onTriggered: {
            const card = graphPane.findCard
            if (!card.typingStopped)
                return
            findHinted.stop()
            if (findHinted.key === "") {
                Harness.report("find_hint stopped=true shown=" + card.hintShown + " matches=" + card.matches
                                  + " query=" + card.query + " hint=" + card.hint)
            } else {
                const before = card.hintShown
                card.query = card.query + findHinted.key
                Harness.report("find_hint_key before=" + before + " after=" + card.hintShown
                                  + " stopped=" + card.typingStopped + " query=" + card.query)
            }
            renderedBarrier.begin()
        }
    }
    // Shot at one end of the card's fade: caught in between, the card that stayed and the one leaving frame the same.
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
    SampleTimer {
        id: avatarSeedTimer
        onTriggered: {
            if (detailsModel.authorEmail === "")
                return
            avatarSeedTimer.stop()
            AppBackend.assignAvatar(detailsModel.authorEmail,
                                    detailsModel.authorName,
                                    Harness.fileUrl(Harness.autoActArg))
            page.settingsDialogRequested()
        }
    }
    // The picture is read off disk asynchronously, a beat after the write.
    SampleTimer {
        id: avatarReportTimer
        onTriggered: {
            if (detailsModel.avatarUrl === "" && AppBackend.avatarErrorKind === "")
                return
            avatarReportTimer.stop()
            Harness.report(
            "avatar email=" + detailsModel.authorEmail
            + " details=" + (detailsModel.avatarUrl !== "")
            + " rows=" + graphModel.avatarRowCount()
            + " error=" + AppBackend.avatarErrorKind)
            driver.complete()
        }
    }
    // PGG_AUTO_ACT=commands-fail-shut: the red mark with the panel out of the way, which no other verb photographs.
    // Pressed at the `>_`'s own function, so a build where that press stopped reaching the page waits here.
    SampleTimer {
        id: commandsShutTimer
        property bool pressed: false
        onTriggered: {
            // Read only before the press: the panel going away is this verb's own answer (rules/app-ui.md §UI 自動化).
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
    // PGG_AUTO_ACT=commands-escape: Esc on the panel a refusal raised (デザイン規約 §git が言ったことを読む場所),
    // entered at `escapePressed` where `Keys.onEscapePressed` enters. **`took=` is the claim**: a key that fell
    // through to nobody would frame exactly like this.
    SampleTimer {
        id: commandsEscapeTimer
        property bool took: false
        property bool pressed: false
        onTriggered: {
            if (!commandsEscapeTimer.pressed) {
                if (!page.commandsWrong || !page.commandsShown)
                    return
                commandsEscapeTimer.pressed = true
                commandsEscapeTimer.took = page.escapePressed()
                return
            }
            if (page.commandsShown)
                return
            commandsEscapeTimer.stop()
            // The mark goes with its panel; a red left behind is what a picture of the shut window cannot show.
            Harness.report("commands_escape took=" + commandsEscapeTimer.took
                              + " open=" + page.commandsShown
                              + " wrong=" + page.commandsWrong
                              + " mark=" + page.commandsMarkColor)
            renderedBarrier.begin()
        }
    }
    // PGG_AUTO_ACT=commands-select / commands-copy: a drag from the head of the first row to the end of the last —
    // all three columns and more than one row — then, for `-copy`, Ctrl+C. **`perRow=` is the claim**: one clipboard
    // line per row. Tab-led lines are not counted: they are git's words under a failure, and which commands fail is
    // not the run's (a container reading the tree's real worktree gets `not a git repository`).
    //
    // Behind the write barrier: the two writes that fill the log are still queued when this starts. Everything is
    // read in the branch that acts (rules/app-ui.md §UI 自動化).
    SampleTimer {
        id: commandsPickTimer
        onTriggered: {
            if (!driver.wroteAndSettled())
                return
            // Two rows: the write barrier alone can let a tick through while an opening read is the only row.
            if (!page.commandsShown || page.pageCommands.running || page.pageCommands.rowsHeld() < 2)
                return
            commandsPickTimer.stop()
            // The model's count: the panel's `(N)` is the view's, a frame behind in the tick a press lands in.
            const rows = page.pageCommands.rowsHeld()
            page.pickCommandText(0, 0, rows - 1, driver.pastLineEnd)
            if (Harness.autoAct === "commands-select") {
                // `holds=` is the model's selection, `worn=` what the rows draw of it (`CommandsPane.washTally`) —
                // either alone passes a broken run.
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
    // PGG_AUTO_ACT=commands-sweep: the same drag, started on the ground under the last row
    // (規約 §git が言ったことを読む場所). Nine starts, each judged over a cleared selection: a reach that works from one
    // spot hides at the middle, and one read at the end would report only the last try.
    SampleTimer {
        id: commandsSweepTimer
        /// Where the ground began at the previous sample, for the settle below.
        property string lastGeom: ""
        onTriggered: {
            if (!driver.wroteAndSettled())
                return
            if (!page.commandsShown || page.pageCommands.running || page.pageCommands.rowsHeld() < 2)
                return
            // The layout has to be still: rows arrive a frame ahead of the view, and a sweep aimed at ground about
            // to be a row lands on neither.
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
            // `ground=false`: the log fills the panel, and `reach=0/9` then says nothing.
            Harness.report("commands_sweep reach=" + reach + "/" + tries
                              + " ground=" + page.commandsHasGround
                              + " rows=" + page.pageCommands.rowsHeld())
            renderedBarrier.begin()
        }
    }
}
