pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

/// PG_AUTO_ACT runs one operation — a write, or a surface left standing
/// for the overlay shot — through exactly the code path a click takes, so
/// the wiring can be proven headlessly. The dispatch is equality on a bare
/// verb; the argument passes through as whatever the verb needs (a name,
/// an oid, a row number).
///
/// `RepoPage` builds this only when a verb was given, so an ordinary run
/// carries none of it. What the verbs act on is handed in below: a file of
/// its own cannot see the page's ids, and naming them in one list is what
/// says how far the harness reaches into the page.
// An `Item` only because `QtObject` has no default property to hold the
// timers below; it draws nothing and is never given a size.
Item {
    id: driver

    /// The page these verbs act on, and the parts of it they read back or
    /// leave standing for the shot. An automation-only exposure, the same
    /// one `GraphPane.view` is (app-ui.md).
    property Item page

    property RepoTab repoTab
    property WorkTreeModel workTree
    property GraphModel graphModel
    property DetailsModel detailsModel
    property NavSectionModel branchesModel
    property NavSectionModel remotesModel
    property NavSectionModel worktreeModel
    property NavSectionModel stashesModel
    property NavSectionModel tagsModel

    property GraphPane graphPane
    property SidebarPane sidebarPane
    property DetailsPane detailsPane
    property DiffPane diffPane
    property WipPane wipPane
    property RowLayout gitCorner

    property AppMenu refMenu
    property AppMenuItem refDeleteItem
    property FileRowMenu fileRowMenu
    property AppMenu fileMenu
    property AppMenuItem fileDiscardItem
    property AppMenu commitMenu
    property AppMenuItem dropCommitItem
    property AppMenuItem stashDeleteItem
    property AppMenu resetMenu
    property AppMenuItem hardResetItem
    property RemoteDialog remoteDialog
    property RefListPopup refList
    property CommitHoverCard rowCard

    /// Kicked off by the page once its models are attached: a verb that
    /// ran before them would act on a repository nothing has read yet.
    function begin() {
        autoActTimer.start()
    }

    Timer {
        id: autoActTimer
        interval: 1200
        onTriggered: driver.runAutoAct()
    }
    // HEAD's author has to arrive before the offer to take it over can
    // be there to tick.
    Timer {
        id: resetAuthorTimer
        interval: 800
        onTriggered: {
            AppBackend.report("head_author differs="
                              + repoTab.headAuthorDiffers
                              + " name=" + repoTab.headAuthorName)
            wipPane.setResetAuthorChecked(true)
            wipPane.setMessage(AppBackend.autoActArg, "")
            page.commitNow()
        }
    }
    // Staging has to land before the button can know what it carries.
    Timer {
        id: eolCommitTimer
        interval: 1200
        onTriggered: {
            wipPane.pointAtCommit = true
            AppBackend.report("eol_commit staged=" + workTree.stagedCount
                              + " warned=" + workTree.eolStagedCount
                              + " card=" + wipPane.eolCardOpen)
        }
    }
    // The marks arrive with the status read, so the row named for its
    // sentence has to be named again once they are in.
    Timer {
        id: eolHoverTimer
        interval: 800
        onTriggered: {
            wipPane.pointEol(AppBackend.autoActArg)
            AppBackend.report("eol_hover path=" + wipPane.pointedEolPath
                              + " card=" + wipPane.eolCardOpen
                              + " text=" + wipPane.pointedEolText)
        }
    }
    // The diff has to arrive before a row of it can be staged. Asked for
    // rather than waited out: a fixed wait photographs an empty pane the
    // same as a late one (2026-08-13 実測: `pick-lines` against this
    // repository picked no lines and passed). When the asking runs out it
    // acts anyway — the report carries `ready=`, which the run is failed
    // on.
    Timer {
        id: stageRowTimer
        interval: 50
        repeat: true
        // Where the asking stops. Long enough for a repository still
        // reading tens of thousands of refs beside the diff, and short
        // enough to leave the write some of these end with room to land
        // before `PG_AUTO_QUIT_MS` takes the picture.
        readonly property int waitMs: 5000
        property int waited: 0
        function begin() {
            stageRowTimer.waited = 0
            stageRowTimer.start()
        }
        // Whether what the verb is about to name is on screen. They all
        // act on the first hunk, so a changed line in it is the one
        // answer they share — "diff-file" alone reads the model instead
        // of a row, and the pictures and binary files it also opens have
        // no rows to find.
        function ready() {
            if (AppBackend.autoAct === "diff-file")
                return diffPane.diffSettled()
            return diffPane.firstChangedLine(0) >= 0
        }
        onTriggered: {
            stageRowTimer.waited += stageRowTimer.interval
            const arrived = stageRowTimer.ready()
            if (!arrived && stageRowTimer.waited < stageRowTimer.waitMs)
                return
            stageRowTimer.stop()
            const act = AppBackend.autoAct
            // Which line the line-level verbs mean. Not 0: a hunk numbers
            // its lines through the context it carries, and the context is
            // not part of the change (see `firstChangedLine`).
            const line = diffPane.firstChangedLine(0)
            // Said before the acting, so a verb that goes on to fail its
            // write says both. `waited=` is ticks, not a clock.
            AppBackend.report("diff_row act=" + act + " ready=" + arrived
                              + " rows=" + diffPane.view.count
                              + " line=" + line
                              + " waited=" + stageRowTimer.waited)
            // The diff is the shot; the line endings get a report line of
            // their own (a picture cannot say which of the four kinds the
            // pane decided on).
            if (act === "diff-file") {
                const d = diffPane.diffModel
                AppBackend.report("line_endings kind=" + d.endingKind
                                  + " scope=" + d.endingScope
                                  + " lines=" + d.endingLines
                                  + " text=" + Words.lineEndings(
                                      d.endingKind, d.endingFrom, d.endingTo,
                                      d.endingLines, d.endingScope, d.endingExt))
                return
            }
            // The squares a line only puts out under the pointer, named
            // rather than hovered (hover cannot be injected on Windows).
            if (act === "line-tools") {
                diffPane.showLineTools(0, line)
                return
            }
            // The heading's two words carry their colours only under the
            // pointer, and hover cannot be injected, so the row is named
            // instead. A heading's own row is line -1 (`flatten_patches`).
            if (act === "hunk-tools") {
                diffPane.showLineTools(0, -1)
                return
            }
            // Reading part way down a long diff and then writing: the
            // rebuild has to come back to the same place. `diff_place` is
            // reported by the restore.
            if (act === "keep-place") {
                diffPane.scrollTo(400)
                page.stageSelection(0, line)
                return
            }
            if (act === "pick-lines" || act === "stage-lines") {
                // Two lines, so the heading names a count rather than a
                // hunk. `any=` is the one judged.
                const want = 2
                const got = diffPane.chooseLines(0, want)
                AppBackend.report("picked_lines any=" + (got > 0)
                                  + " got=" + got + " want=" + want)
                if (act === "stage-lines")
                    page.stageChosenLines()
                return
            }
            if (act === "stage-hunk" || act === "stage-line") {
                page.stageSelection(0, act === "stage-line" ? line : -1)
                return
            }
            // No line-level discard exists — a hunk is the smallest piece
            // that can be thrown away.
            if (act === "discard-hunk-go")
                diffPane.completeHold()
        }
    }
    // A reader who scrolled before the colours landed: the whole list is
    // swapped again when the colours turn up (`DiffModel::lay_out_rows`),
    // and that swap must not cost the place being read.
    Timer {
        id: colourPlaceTimer
        interval: 50
        repeat: true
        readonly property int waitMs: 8000
        property int waited: 0
        property bool scrolled: false
        function begin() {
            colourPlaceTimer.waited = 0
            colourPlaceTimer.scrolled = false
            colourPlaceTimer.start()
        }
        onTriggered: {
            colourPlaceTimer.waited += colourPlaceTimer.interval
            const late = colourPlaceTimer.waited >= colourPlaceTimer.waitMs
            if (!colourPlaceTimer.scrolled) {
                // Read down the file the moment the rows are there, which
                // is well before the colours are.
                if (diffPane.firstChangedLine(0) < 0 && !late)
                    return
                diffPane.scrollTo(400)
                colourPlaceTimer.scrolled = true
                return
            }
            if (!diffPane.diffModel.coloured && !late)
                return
            colourPlaceTimer.stop()
            AppBackend.report("colour_place coloured="
                              + diffPane.diffModel.coloured
                              + " at=" + Math.round(diffPane.view.contentY)
                              + " rows=" + diffPane.view.count
                              + " waited=" + colourPlaceTimer.waited)
        }
    }
    // git's refusal has to come back before the row it turns into a held
    // one can be held — or photographed.
    Timer {
        id: forceDeleteTimer
        interval: 800
        onTriggered: {
            AppBackend.report("ref_menu delete=" + refDeleteItem.text
                              + " note=" + refDeleteItem.note)
            refDeleteItem.completeHold()
        }
    }
    // The splitter has to have handed the pane its new width before the
    // width can be reported — the fold sets it, the layout takes it.
    Timer {
        id: navRailTimer
        interval: 400
        onTriggered: AppBackend.report(
            "nav_rail collapsed=" + page.sidebarCollapsed
            + " width=" + Math.round(sidebarPane.width)
            + " peek=" + sidebarPane.peekKind
            // Where the open section stands. `cell` is the top edge of the
            // mark that opened it and `top` where the panel begins — they
            // are the same number or the list has walked away from its own
            // cell — the failure a section too tall for the pane invites.
            // `end` against `pane` is the other half: it grows down
            // into the pane and stops at the foot of it.
            + " top=" + Math.round(sidebarPane.peekY)
            + " cell=" + Math.round(sidebarPane.peekTop)
            + " end=" + Math.round(sidebarPane.peekBottom)
            + " pane=" + Math.round(sidebarPane.height)
            + " editing=" + sidebarPane.editKey
            // What the centre holds: the list coming back closes a file,
            // so the two are read together or not at all.
            + " diff=" + page.diffShown)
    }
    // The column has to be laid out again before the header that was
    // closed can say where it ended up.
    Timer {
        id: navSectionTimer
        interval: 400
        onTriggered: AppBackend.report(
            "nav_section closed=" + AppBackend.autoActArg
            + " header=" + Math.round(
                sidebarPane.headerTopOf(AppBackend.autoActArg))
            + " ground=" + Math.round(sidebarPane.groundTop)
            + " pane=" + Math.round(sidebarPane.height))
    }
    /// What each section kept of what it holds. The rows a filter leaves
    /// are the ones the sections work out for themselves, so the counts
    /// are read off the models and the picture says what they drew.
    Timer {
        id: navFilterTimer
        interval: 400
        onTriggered: AppBackend.report(
            "nav_filter typed=" + AppBackend.autoActArg
            + " branches=" + branchesModel.shown() + "/" + branchesModel.total
            + " remotes=" + remotesModel.shown() + "/" + remotesModel.total
            + " tags=" + tagsModel.shown() + "/" + tagsModel.total
            + " stashes=" + stashesModel.shown() + "/" + stashesModel.total)
    }
    // The blocked row's line, worn where the pointer would put it.
    // Past `tipDelayMs`, like the other forced tooltips: read any sooner
    // and the attached ToolTip has not opened yet, so the line reports
    // false while the picture taken at quit holds it.
    Timer {
        id: blockedTipTimer
        interval: 800
        onTriggered: AppBackend.report(
            "delete_blocked code=" + refDeleteItem.code
            + " tip=" + refDeleteItem.ToolTip.visible
            + " reason=" + refDeleteItem.blockedReason)
    }
    Timer {
        id: chipMenuTimer
        interval: 200
        onTriggered: AppBackend.report("chip_menu ref=" + refMenu.opened
                                       + " commit=" + commitMenu.opened
                                       + " delete=" + refDeleteItem.code
                                       + " " + refDeleteItem.text)
    }
    // Waits on the early answer, not on a refusal: nothing here writes.
    Timer {
        id: earlyDeleteTimer
        interval: 800
        onTriggered: AppBackend.report("delete_early asked="
                                       + (repoTab.branchDeleteAsked !== "")
                                       + " merged=" + repoTab.branchDeleteMerged
                                       + " code=" + refDeleteItem.code
                                       + " held=" + (refDeleteItem.holdMs > 0)
                                       + " note=" + refDeleteItem.note)
    }
    Timer {
        id: refusedRowTimer
        interval: 800
        onTriggered: AppBackend.report("ref_menu delete=" + refDeleteItem.code
                                       + " " + refDeleteItem.text
                                       + " note=" + refDeleteItem.note)
    }
    // The lane column has to have taken its narrower width before there
    // is anywhere to pan to, or a bar worth wanting.
    Timer {
        id: graphPanTimer
        interval: 400
        onTriggered: {
            // Where the pointer is, which is the whole of what puts the
            // bar on screen. `-away` walks it back out again: a bar that
            // comes when the pointer does proves nothing on its own
            // unless it also goes when the pointer goes.
            graphPane.restPointer(true)
            if (AppBackend.autoAct !== "middle-scroll") {
                if (AppBackend.autoAct === "graph-bar-away")
                    graphPane.restPointer(false)
                AppBackend.report(
                    "graph_bar shown=" + graphPane.laneBarShown
                    + " overflow=" + Math.round(graphPane.graphXMax))
                return
            }
            // The middle click, then the pointer drifting sideways off
            // it. The argument says which column the click landed in,
            // which is the whole question — only the lanes take the
            // sideways drift (デザイン規約 §グラフを横へ送る).
            const y = graphPane.height / 2
            const x = AppBackend.autoActArg === "message"
                    ? graphPane.labelW + graphPane.graphColW + Theme.spaceXl
                    : graphPane.labelW + Theme.spaceSm
            graphPane.startAutoScroll(x, y)
            graphPane.driftPointer(x + graphPane.width, y)
            middleScrollTimer.start()
        }
    }
    // Long enough for the 16ms ticker to have taken the lanes as far as
    // they go: a pan that ran and a pan that was refused must not read
    // alike in the report.
    Timer {
        id: middleScrollTimer
        interval: 400
        onTriggered: AppBackend.report(
            "middle_scroll lanes=" + graphPane.autoPanning
            + " x=" + Math.round(graphPane.graphX)
            + " max=" + Math.round(graphPane.graphXMax))
    }
    // The arrow keys, which no headless run can press: the walk enters
    // where `Keys.onDownPressed` enters (`GraphPane.stepRow`) after
    // taking the keyboard the way a row click takes it. The selected
    // commit's message has to have arrived before it can be typed over,
    // which is what the wait is for — the same one the reword verbs keep.
    Timer {
        id: graphStepTimer
        interval: 800
        /// How many rows, and which way. The refusing runs fix their own.
        property int steps: 1
        /// The two grounds a step is refused on that a run can stand up:
        /// a name box open on the row, and a half-written message the
        /// move is already being asked about. (The third — a question
        /// standing on the bar — is refused by the same expression, and
        /// its pill holds the keyboard anyway.)
        property bool named: false
        property bool dirty: false
        /// The view sent away from the selection before the step, so the
        /// row stepped onto has no reading position to preserve.
        property bool away: false
        /// The third refusing ground, and the one that was reported: a
        /// diff opened over the graph from CHANGES. The path is the
        /// argument — the file has to be one the selected commit touched.
        property string diffPath: ""
        onTriggered: {
            if (graphStepTimer.dirty)
                detailsPane.setMessageText("wip: half of a subject", "")
            if (graphStepTimer.named)
                graphPane.startNaming(
                    graphModel.oidAt(graphPane.view.currentIndex))
            // The press that says the keyboard works here comes first,
            // because the diff below is what has to take it away again:
            // a run that opened the diff and only then reached for the
            // keyboard would be proving nothing (it would be pressing on
            // a pane that is no longer on the screen).
            graphPane.view.takeKeyboard()
            if (graphStepTimer.diffPath !== "")
                page.toggleDiff("commit", graphStepTimer.diffPath, "")
            graphStepWalk.start()
        }
    }
    // A beat between the setup and the walk: the layout swaps the graph
    // away in its own pass, so a step taken in the same tick as the diff
    // opened would still find the pane on screen.
    Timer {
        id: graphStepWalk
        interval: 200
        onTriggered: {
            if (graphStepTimer.away)
                graphPane.view.contentY = graphPane.view.clampY(Infinity)
            graphStepReport.from = graphPane.view.currentIndex
            graphStepReport.refused = 0
            const way = graphStepTimer.steps < 0 ? -1 : 1
            for (let n = 0; n < Math.abs(graphStepTimer.steps); n++) {
                // Where the view stood before each step, so what is read
                // is how the last one landed: a walk that runs off the
                // bottom moves the view once per row from there on.
                graphStepReport.wasY = graphPane.view.contentY
                if (!graphPane.stepRow(way))
                    graphStepReport.refused++
            }
            graphStepReport.start()
        }
    }
    // Longer than the settle behind the walk (`keyStepSettleMs`), so what
    // is read is the reading a hand coming off the key would get: a run
    // that moved the highlight and never landed the selection has to be
    // told apart from one that did, and both frame alike from the waist
    // down — the picture holds the lit row, not which commit the panes
    // on the right ended up on.
    Timer {
        id: graphStepReport
        interval: 400
        property int from: -1
        property real wasY: 0
        property int refused: 0
        onTriggered: {
            const row = graphPane.view.currentIndex
            AppBackend.report(
                "graph_step from=" + graphStepReport.from
                + " row=" + row
                + " steps=" + graphStepTimer.steps
                + " landing=" + graphPane.stepLanding(row, graphStepReport.wasY)
                + " held=" + (page.pendingMove !== null)
                + " back=" + (row === graphStepReport.from)
                + " refused=" + graphStepReport.refused
                + " focused=" + graphPane.view.activeFocus
                + " diff=" + page.diffShown
                + " onscreen=" + graphPane.rowOnScreen(row)
                + " selected=" + (page.selectedOid === graphModel.oidAt(row)))
        }
    }
    // The diff's own arrows, which no headless run can press either: the
    // walk enters where `Keys.onDownPressed` enters (`DiffPane.stepRows`).
    // Nothing reaches for the keyboard first, and that is half of what
    // this reads — the diff takes it by coming on screen, so `focused=`
    // is a claim about the arrival and not about a press this made
    // (規約 §diff を上下に送る). The wait is for the file's rows to land:
    // a list still empty has nothing to send and refuses every step.
    Timer {
        id: diffStepTimer
        interval: 800
        /// How many rows, and which way.
        property int steps: 1
        onTriggered: {
            diffStepReport.from = driver.diffRow()
            diffStepReport.stopped = false
            const way = diffStepTimer.steps < 0 ? -1 : 1
            for (let n = 0; n < Math.abs(diffStepTimer.steps); n++) {
                // A step that moved nothing is the end answering. Read
                // beside `atEnd=`: a walk that was refused every step
                // because the pane was never on screen leaves the view at
                // row 0, which is also where an unscrollable diff sits.
                if (!diffPane.stepRows(way))
                    diffStepReport.stopped = true
            }
            diffStepReport.start()
        }
    }
    Timer {
        id: diffStepReport
        interval: 300
        property int from: -1
        property bool stopped: false
        onTriggered: AppBackend.report(
            "diff_step from=" + diffStepReport.from
            + " rows=" + driver.diffRow()
            + " steps=" + diffStepTimer.steps
            + " moved=" + (driver.diffRow() !== diffStepReport.from)
            + " atEnd=" + diffPane.atEnd
            + " stopped=" + diffStepReport.stopped
            + " focused=" + diffPane.view.activeFocus)
    }
    /// Where the diff's view stands, in rows — what the walk is counted
    /// in, and steadier than a pixel count to read off a report line.
    function diffRow() {
        return Math.round(diffPane.view.contentY / Theme.rowHeight)
    }
    // The fetch has to land, and its answer reach the chips, before the
    // stacked ones are worth unstacking.
    Timer {
        id: fetchedRefListTimer
        interval: 1500
        onTriggered: {
            const stacked = graphPane.view.itemAtIndex(
                Number(AppBackend.autoActArg))
            if (stacked)
                graphPane.view.chipExpandRequested(
                    stacked.chipItem.records, stacked.chipItem)
        }
    }
    // The refusal has to be back and on the button before the second go
    // is sent, and the report is what says it ever got there — the mark
    // is gone again by the time the screenshot is taken.
    Timer {
        id: pushRetryTimer
        interval: 1800
        onTriggered: {
            AppBackend.report("push_retry refused=" + page.pushFailed
                              + " branch=" + page.pushFailBranch)
            page.forcePush()
        }
    }
    // Where an operation that answers at the tip left the reader — one
    // report for the three of them. The write, its refresh and the beat
    // the viewport waits out all have to be behind it, and the picture
    // cannot answer the second half: a row can be selected and still be
    // somewhere nobody can see.
    Timer {
        id: tipLandedTimer
        interval: 1800
        onTriggered: {
            const row = graphModel.rowOf(page.selectedOid)
            AppBackend.report(
                "tip_landed follows="
                + (page.selectedOid !== "" && page.selectedOid === branchesModel.headOid)
                + " onscreen=" + (row >= 0 && graphPane.rowOnScreen(row))
                + " op=" + repoTab.lastWriteOp
                + " head=" + branchesModel.headOid.substring(0, 8)
                + " selected=" + page.selectedOid.substring(0, 8)
                + " row=" + row)
        }
    }
    // The message has to arrive before it can be typed over, and the
    // "is this commit ours to rewrite?" answer before it may be saved.
    Timer {
        id: rewordTimer
        interval: 800
        onTriggered: {
            // "edit-message-focus" types nothing: the commit's own body
            // is what the caret has to be photographed on top of, and
            // an empty box would only show the placeholder.
            if (AppBackend.autoAct === "edit-message-focus") {
                detailsPane.focusDescription()
                AppBackend.report("message_focus pane=details focused="
                                  + detailsPane.descriptionFocused
                                  + " color=" + detailsPane.descriptionColor)
                return
            }
            detailsPane.setMessageText(AppBackend.autoActArg, "")
            // "edit-message" stops here, with the save row on screen.
            if (AppBackend.autoAct === "reword")
                detailsPane.submitMessage()
            // "edit-message-leave" walks away from the unsaved text,
            // which is what raises the question about dropping it;
            // "-discard" then answers it, which lets the move through.
            else if (AppBackend.autoAct === "edit-message-leave"
                     || AppBackend.autoAct === "edit-message-discard") {
                page.activateRow(graphModel.oidAt(graphModel.rowOf(page.selectedOid) + 1))
                if (AppBackend.autoAct === "edit-message-discard")
                    detailsPane.leaveResolved(true)
            }
        }
    }
    // gpg / ssh-keygen have to finish before the mark they decide can be
    // on screen, so the shot and the report both wait for them.
    Timer {
        id: signatureTimer
        interval: 800
        onTriggered: AppBackend.report(
            "signature kind=" + page.selectedSignatureKind
            + " code=" + page.selectedSignatureCode
            + " signer=" + page.selectedSignatureSigner)
    }
    // The tooltip halves of signature-tip / stash-tip: the state has to
    // land (gpg's verdict, the stash's details) before the target is
    // pointed at, and the report then waits out Metrics.tipDelayMs so
    // what it reads is the tip on screen.
    Timer {
        id: signatureTipTimer
        interval: 800
        onTriggered: {
            detailsPane.signaturePointedAt = true
            signatureTipReport.start()
        }
    }
    Timer {
        id: signatureTipReport
        interval: 800
        onTriggered: AppBackend.report(
            "signature_tip code=" + page.selectedSignatureCode
            + " tip=" + detailsPane.signatureTipShown)
    }
    Timer {
        id: stashTipTimer
        interval: 800
        onTriggered: {
            detailsPane.summaryPointedAt = true
            stashTipReport.start()
        }
    }
    Timer {
        id: stashTipReport
        interval: 800
        onTriggered: AppBackend.report(
            "stash_tip blocked=" + (detailsPane.editBlocked !== "")
            + " tip=" + detailsPane.summaryTipShown)
    }
    // The tooltip half of path-tip: the list has to land before a row
    // can be pointed at, and the report then waits out tipDelayMs so
    // what it reads is the tip on screen. It reads the shared instance
    // itself — the one thing that can also say the words on it.
    Timer {
        id: pathTipTimer
        property bool wipSide: true
        interval: 800
        onTriggered: {
            if (pathTipTimer.wipSide)
                wipPane.pointedTipRow = 0
            else
                detailsPane.pointedTipRow = 0
            pathTipReport.start()
        }
    }
    Timer {
        id: pathTipReport
        interval: 800
        onTriggered: {
            const tip = page.ToolTip.toolTip
            AppBackend.report("path_tip pane="
                + (pathTipTimer.wipSide ? "wip" : "details")
                + " tree=" + (pathTipTimer.wipSide
                              ? worktreeModel.treeView
                              : detailsModel.treeView)
                + " tip=" + tip.visible
                + " text=" + tip.text)
        }
    }
    // Automation: the details have to land before the author card can be
    // worked, since it is that author the picture is filed against.
    Timer {
        id: avatarAssignTimer
        interval: 400
        onTriggered: {
            AppBackend.assignAvatar(detailsModel.authorEmail,
                                    detailsModel.authorName,
                                    AppBackend.autoActArg)
            avatarReportTimer.start()
        }
    }
    Timer {
        id: avatarBadgeTimer
        interval: 400
        onTriggered: detailsPane.avatarClicked()
    }
    // What the graph did about the find bar, read after it finished doing
    // it. The step down out from under the card is animated, so the value
    // in the same call stack as the verb is always the one before it
    // moved — reporting that would be reporting the intent, which the
    // line above already carries as `clears=`.
    Timer {
        id: findSettled
        interval: 300
        onTriggered: AppBackend.report(
            "find_settled shift=" + Math.round(graphPane.findShift))
    }
    // The store starts empty in every run, so the card's own verbs put a
    // picture in it before opening on it.
    Timer {
        id: avatarSeedTimer
        interval: 400
        onTriggered: {
            AppBackend.assignAvatar(detailsModel.authorEmail,
                                    detailsModel.authorName,
                                    AppBackend.autoActArg)
            page.settingsDialogRequested()
        }
    }
    // The picture is read off disk asynchronously, so what the shot wants
    // is a beat after the write rather than the instant it returns.
    Timer {
        id: avatarReportTimer
        interval: 600
        onTriggered: AppBackend.report(
            "avatar email=" + detailsModel.authorEmail
            + " details=" + (detailsModel.avatarUrl !== "")
            + " rows=" + graphModel.avatarRowCount()
            + " error=" + AppBackend.avatarError)
    }
    // The card is opened synchronously; this just lets the layout settle
    // before it is measured and photographed.
    Timer {
        id: rowCardTimer
        interval: 400
        onTriggered: AppBackend.report(
            "row_card open=" + rowCard.opened
            + " credit=" + Math.round(rowCard.creditWidth)
            + " cut=" + rowCard.creditCut
            + " list=" + refList.opened
            + " subject=" + (rowCard.subject !== "")
            + " body=" + (rowCard.body !== ""))
    }
    // The details have to arrive before the name can name anybody.
    Timer {
        id: authorCardTimer
        interval: 800
        onTriggered: {
            if (AppBackend.autoAct === "author-card-open")
                detailsPane.showAuthor(true)
            AppBackend.report(
                "author_card open=" + detailsPane.authorCardOpen
                + " author=" + detailsPane.details.authorEmail
                + " committer=" + detailsPane.details.committerEmail
                + " other=" + detailsPane.details.committerDiffers
                + " later=" + detailsPane.details.commitTimeDiffers)
        }
    }
    // The details have to arrive before the credit line they carry can
    // be opened or counted.
    Timer {
        id: coAuthorTimer
        interval: 800
        onTriggered: {
            if (AppBackend.autoAct === "co-authors-open")
                detailsPane.showCoAuthors(true)
            // `open` is the card's own visibility, not the input that
            // asked for it: reporting the input would go green with the
            // binding cut.
            AppBackend.report(
                "co_authors count=" + detailsPane.coAuthorRecords.length
                + " first=" + detailsPane.coAuthorName(0)
                + " open=" + detailsPane.matesCardOpen)
        }
    }
    // The details have to arrive, and the column has to be laid out with
    // them, before there is anything to measure.
    Timer {
        id: detailsFitTimer
        interval: 800
        // A pane width the splitter left on a fraction can put a fraction
        // in the answer; what this verb is about is tens of pixels.
        onTriggered: AppBackend.report(
            "details_fit fits=" + (detailsPane.contentOverflow < 1)
            + " over=" + Math.round(detailsPane.contentOverflow)
            + " pane=" + Math.round(detailsPane.width)
            // The other axis rides along unjudged, the way `edge=` does in
            // `window_fill`: how far the column runs past the pane's own
            // bottom is what says whether this pane needs a scroll of its
            // own, and the answer depends on the window, not on this verb.
            + " overH=" + Math.round(detailsPane.contentOverHeight)
            + " paneH=" + Math.round(detailsPane.height))
    }
    // The rows have to arrive, and the list be laid out with them, before
    // what they leave bare is worth measuring.
    Timer {
        id: cornerTimer
        interval: 800
        // `shown=` is the label's own visibility, not the room that
        // decided it: reporting what was asked for would go green with
        // the binding cut.
        onTriggered: AppBackend.report(
            "git_corner pane=" + (page.wipShown ? "wip" : "details")
            + " shown=" + gitCorner.visible
            + " room=" + Math.round(gitCorner.roomLeft)
            + " needs=" + Math.round(gitCorner.roomNeeded))
    }
    // Same wait as details-fit, for the same reason: the message has to
    // be in the box, and the box laid out with it, before there is a
    // ceiling to pull on.
    Timer {
        id: descGrowTimer
        interval: 800
        // Pulled past everything, so where it stops is the bound itself
        // rather than a number this verb chose.
        readonly property int pull: 1000
        /// Which pane's box to pull. The two carry the same box and hooks
        /// under the same names, so this verb is written once.
        property var pane: detailsPane
        property string paneName: "details"
        /// Whether to take the pane's room away again afterwards, by
        /// raising the command log under it — the one way a headless run
        /// can make the pane shorter than the box it is already holding.
        property bool squeeze: false
        /// Which end this run is carrying the grip past, or empty for the
        /// ordinary pull. Same wait and same box — the difference is that
        /// the grip is in hand, so the box answers instead of just
        /// stopping (規約 §掴める境界は答える).
        property string refuse: ""
        onTriggered: {
            if (descGrowTimer.refuse !== "") {
                descGrowTimer.pane.pullDescriptionPast(
                    descGrowTimer.refuse === "desc-max")
                AppBackend.report(
                    "divider_refuse refuses=" + page.refusalShown
                    // The grip itself, still offered: the box moves the
                    // other way, and a corner that withdrew its mark would
                    // be answering a different question.
                    + " line=" + descGrowTimer.pane.descGrips
                    + " case=" + descGrowTimer.refuse
                    + " box=" + Math.round(descGrowTimer.pane.descHeight)
                    + " wants=" + Math.round(descGrowTimer.pane.descWants))
                return
            }
            descGrowTimer.pane.growDescription(descGrowTimer.pull)
            if (descGrowTimer.squeeze)
                page.toggleCommands()
            descGrowSettle.start()
        }
    }
    // The layout runs after that handler, so what the pull left behind is
    // read a beat later: asked in the same breath, the list still reports
    // the height it had before it gave any of it up.
    //
    // `grip=` says the corner was offered at all, `keeps=` that what the
    // box borrowed room from is still on screen — the author card in the
    // details pane, the commit button in the editor — which the picture
    // cannot answer, because the overflow draws over the window's own
    // footer.
    Timer {
        id: descGrowSettle
        interval: 200
        onTriggered: AppBackend.report(
            "description_grow keeps=" + descGrowTimer.pane.descKeeps
            + " pane=" + descGrowTimer.paneName
            + " grip=" + descGrowTimer.pane.descGrips
            + " box=" + Math.round(descGrowTimer.pane.descHeight)
            + " wants=" + Math.round(descGrowTimer.pane.descWants)
            + " cap=" + Math.round(descGrowTimer.pane.descCap)
            + " rows=" + descGrowTimer.pane.descListRows)
    }
    /// Automation: how long a run of failed fetches the verb asked for,
    /// and whether to hold the button that resumes once it is there.
    property int fetchFailRuns: 0
    property bool fetchResumeAfter: false
    /// The count this already answered. `changed` fires on every message
    /// the tab drains, and without this every one of them would queue
    /// another fetch behind the one still running.
    property int fetchFailSeen: -1
    function runFetchFailures() {
        if (driver.fetchFailRuns <= 0 || repoTab.fetchFailures === driver.fetchFailSeen)
            return
        driver.fetchFailSeen = repoTab.fetchFailures
        if (repoTab.fetchFailures < driver.fetchFailRuns) {
            repoTab.fetch("")
            return
        }
        driver.fetchFailRuns = 0
        if (driver.fetchResumeAfter) {
            driver.fetchResumeAfter = false
            repoTab.resumeAutoFetch()
        }
    }
    // The commit an automation argument names: an object name as it
    // stands, "row:<n>" read off the graph the way the other row verbs
    // are addressed, and the branch tip when nothing is given. A headless
    // run cannot spell an object name it has not been told, and a demo
    // repository is built fresh every time.
    function autoActOid(arg) {
        if (arg === "")
            return branchesModel.headOid
        if (arg.indexOf("row:") === 0)
            return graphModel.oidAt(Number(arg.substring(4)))
        return arg
    }
    function runAutoAct() {
        const act = AppBackend.autoAct
        const arg = AppBackend.autoActArg
        if (act === "publish" || act === "publish-taken"
                || act === "publish-add" || act === "publish-go"
                || act === "publish-new-go" || act === "publish-remotes") {
            // The button's own path, so the state machine in front of the
            // question is exercised too, not just the question.
            page.pushNow()
            if (act === "publish-taken")
                page.setPublishBranch(arg === "" ? "taken" : arg)
            else if (act === "publish-add" || act === "publish-new-go")
                page.startPublishAddRemote(arg)
            else if (arg !== "")
                page.setPublishBranch(arg)
            if (act === "publish-new-go")
                publishAddTimer.start()
            else if (act === "publish-go")
                publishAnswerTimer.start()
            else if (act === "publish-remotes")
                page.openPublishRemotes()
            else
                publishSettleTimer.start()
            // `dialog=` / `name=` say whether the remote dialog stands and
            // what its name box holds — the no-remote push opens it by
            // itself, and only this line can say so headless.
            AppBackend.report("publish state=" + page.pushState
                              + " remote=" + page.publishRemote
                              + " branch=" + page.publishBranch
                              + " dialog=" + remoteDialog.visible
                              + " name=" + remoteDialog.wantedName)
        } else if (act === "commit") {
            // A message of its own when none was named: git refuses an
            // empty one outright, and a run that asked for a commit and
            // got a refusal is a picture of the history it did not write.
            // Amend is the one below and needs no such fallback — there an
            // empty message means "keep HEAD's" (`--no-edit`).
            repoTab.stageAll()
            wipPane.setMessage(arg === "" ? "chore: commit from the headless run" : arg, "")
            page.commitNow()
        } else if (act === "amend") {
            // The message is supplied, so skip the prefill request
            // that would otherwise land on top of it.
            wipPane.setAmendChecked(true)
            page.amending = true
            wipPane.setMessage(arg, "")
            page.commitNow()
        } else if (act === "amend-reset-author") {
            // Whether authorship is HEAD's to take over is only known
            // once HEAD has been read, so this one goes the long way
            // round: turn amend on and wait for the answer.
            wipPane.setAmendChecked(true)
            page.amendToggled(true)
            resetAuthorTimer.start()
        } else if (act === "stash" || act === "stash-staged") {
            // Through the pane's card, like the button: it decides which
            // options the stash is made with.
            page.showWip()
            wipPane.openStashPanel()
            if (act === "stash-staged")
                wipPane.stashClickStagedOnly()
            wipPane.stashApply()
        } else if (act === "stash-file") {
            wipPane.chooseOnly("unstaged", arg)
            page.openFileMenu("unstaged", arg, "")
            fileRowMenu.sendPaths([arg])
            repoTab.stashPaths("")
        } else if (act === "stage-many" || act === "stage-many-go") {
            page.showWip()
            const head = wipPane.rowAt(0)
            if (head)
                wipPane.chooseOnly(head.bucket, head.fullName)
            const mate = wipPane.rowFor(arg)
            if (mate)
                wipPane.applyClick(mate.bucket, mate.fullName, Qt.ControlModifier)
            AppBackend.report("chosen count=" + wipPane.chosenCount)
            if (head) {
                wipPane.showStageTools(head.bucket, head.fullName)
                if (act.endsWith("-go")) {
                    const row = wipPane.rowAt(0)
                    if (row)
                        row.stageClicked(head.bucket, head.fullName)
                }
            }
        } else if (act === "discard-many" || act === "discard-many-go") {
            page.showWip()
            const first = wipPane.rowAt(0)
            if (first)
                wipPane.chooseOnly(first.bucket, first.fullName)
            const other = wipPane.rowFor(arg)
            if (other)
                wipPane.applyClick(other.bucket, other.fullName, Qt.ControlModifier)
            AppBackend.report("chosen count=" + wipPane.chosenCount)
            page.openFileMenu(other ? other.bucket : "unstaged", arg, "")
            AppBackend.report("discard_row " + fileDiscardItem.text)
            if (act.endsWith("-go"))
                fileDiscardItem.completeHold()
        } else if (act === "stash-dialog") {
            page.showWip()
            wipPane.openStashPanel()
            if (arg === "staged-only")
                wipPane.stashClickStagedOnly()
        } else if (act === "file-menu" || act === "file-menu-untracked"
                   || act === "file-menu-staged" || act === "file-menu-conflict") {
            const menuBucket = act === "file-menu" ? "unstaged"
                             : act === "file-menu-staged" ? "staged"
                             : act === "file-menu-conflict" ? "conflicts"
                             : "untracked"
            page.showWip()
            wipPane.chooseOnly(menuBucket, arg)
            page.openFileMenu(menuBucket, arg, "")
            if (menuBucket === "conflicts") {
                const row = wipPane.rowFor(arg)
                AppBackend.report("conflict_kind " + (row ? row.conflictWords() : "-"))
            } else {
                AppBackend.report("discard_row " + fileDiscardItem.text)
            }
        } else if (act === "take-side-ours" || act === "take-side-theirs") {
            page.showWip()
            wipPane.chooseOnly("conflicts", arg)
            page.openFileMenu("conflicts", arg, "")
            fileMenu.close()
            fileRowMenu.takeSideNow(act === "take-side-ours" ? "ours" : "theirs")
        } else if (act === "open-mergetool") {
            // With a tool configured this holds the write queue until it
            // exits, so a demo tool that blocks leaves the wait on screen.
            page.showWip()
            wipPane.chooseOnly("conflicts", arg)
            page.openFileMenu("conflicts", arg, "")
            fileMenu.close()
            fileRowMenu.openInMergeTool()
            AppBackend.report("merge_tool " + wipPane.workTree.mergeTool)
        } else if (act === "discard-file" || act === "discard-file-go"
                   || act === "delete-file" || act === "delete-file-go"
                   || act === "discard-staged" || act === "discard-staged-go") {
            // Which row follows the verb: "delete-file" an untracked one,
            // "discard-staged" the staged side, otherwise the unstaged
            // one. The plain verb leaves the menu standing for the shot;
            // "-go" runs the hold to its end.
            page.showWip()
            const bucket = act.startsWith("delete-file") ? "untracked"
                         : act.startsWith("discard-staged") ? "staged"
                         : "unstaged"
            wipPane.chooseOnly(bucket, arg)
            page.openFileMenu(bucket, arg)
            AppBackend.report("discard_row " + fileDiscardItem.text)
            if (act.endsWith("-go"))
                fileDiscardItem.completeHold()
        } else if (act === "amend-author") {
            wipPane.setAmendChecked(true)
            page.amendToggled(true)
        } else if (act === "switch") {
            page.switchTo("branch", arg, arg)
        } else if (act === "switch-remote") {
            page.switchTo("remote", arg, arg, "")
        } else if (act === "nav-dbl") {
            // A double-click in the left menu, entered where the row
            // enters it. The argument is `<section>:<name>`.
            const cut = arg.indexOf(":")
            const section = arg.substring(0, cut)
            const rowName = arg.substring(cut + 1)
            const model = section === "tag" ? tagsModel
                        : section === "remote" ? remotesModel : branchesModel
            sidebarPane.activateRow(section, rowName, rowName,
                                    model.oidOfName(rowName))
        } else if (act === "nav-fold" || act === "nav-peek"
                   || act === "nav-unfold" || act === "nav-peek-rename"
                   || act === "nav-peek-away" || act === "nav-peek-into"
                   || act === "nav-peek-out" || act === "nav-peek-shut") {
            // Hover cannot be injected, so the rail cell is named.
            // "-away" walks the pointer off the cell, "-into" down into
            // the opened list, "-out" on out the far side (the exit no
            // cell can see), "-shut" clicks the cell; only "-into" leaves
            // the section standing. "nav-peek" on an empty section must
            // not open at all (NavRail.enterAt decides — `--preset empty`
            // reads that side).
            if (arg === "no-tags")
                repoTab.setTagsShown(false)
            page.foldByHand(true)
            if (act === "nav-peek")
                sidebarPane.peekAt(arg)
            else if (act === "nav-peek-away") {
                sidebarPane.peekAt(arg)
                sidebarPane.peekAway(arg)
            } else if (act === "nav-peek-into" || act === "nav-peek-out") {
                sidebarPane.peekAt(arg)
                sidebarPane.peekInto(arg)
                if (act === "nav-peek-out")
                    sidebarPane.peekOut()
            } else if (act === "nav-peek-shut") {
                sidebarPane.peekAt(arg)
                sidebarPane.peekTap(arg)
            } else if (act === "nav-unfold")
                page.foldByHand(false)
            else if (act === "nav-peek-rename") {
                // Typing a name into a peeked row: the list has to come
                // back on its own and the box land on the same row in it
                // with the keyboard (SidebarPane.startEdit).
                sidebarPane.peekAt("branch")
                sidebarPane.beginRename("branch", workTree.branch,
                                        workTree.branch)
            }
            navRailTimer.start()
        } else if (act === "nav-close") {
            // The pane keeps sections packed against the top; what is
            // read is where the closed header came to rest — at the foot
            // of the pane is the failure this watches for.
            sidebarPane.closeSection(arg)
            navSectionTimer.start()
        } else if (act === "nav-filter") {
            sidebarPane.typeFilter(arg)
            navFilterTimer.start()
        } else if (act === "nav-rename" || act === "rename-branch"
                   || act === "rename-tag" || act === "rename-stash") {
            // Which row: the current branch, the first tag, the first
            // stash. "nav-rename" leaves the box standing for the shot.
            const kind = act === "rename-tag" ? "tag"
                       : act === "rename-stash" ? "stash" : "branch"
            const id = kind === "branch" ? workTree.branch
                     : kind === "tag" ? tagsModel.nameAt(0) : stashesModel.fullAt(0)
            const shown = kind === "stash" ? stashesModel.nameAt(0) : id
            sidebarPane.beginRename(kind, id, shown)
            if (act !== "nav-rename")
                sidebarPane.submitEdit(arg)
        } else if (act === "rename-remote" || act === "rename-remote-box"
                   || act === "rename-remote-go") {
            // Named outright (`origin/billing:billing-v2`) because the
            // remote's rows are behind a fold. "-box" leaves the box
            // standing, the plain act stops at the question, "-go" holds
            // the pill to the end.
            const parts = arg.split(":")
            const ref = parts[0]
            const was = ref.substring(ref.indexOf("/") + 1)
            // "-box" opens with the argument already in it, so a name the
            // remote already carries can be photographed being refused —
            // and the remote's fold has to come open for the row to be
            // there at all (a remote root starts closed).
            if (act === "rename-remote-box")
                remotesModel.toggleFolder(ref.substring(0, ref.indexOf("/")))
            sidebarPane.beginRename("remote", ref,
                                    act === "rename-remote-box" ? parts[1] : was)
            if (act === "rename-remote-box")
                return
            sidebarPane.submitEdit(parts[1])
            if (act === "rename-remote-go")
                graphPane.completeHold()
        } else if (act === "rename-local-upstream") {
            // The question about carrying the name over comes back only
            // when git says the local rename landed (so the shot is late).
            const local = workTree.branch
            sidebarPane.beginRename("branch", local, local)
            sidebarPane.submitEdit(arg)
        } else if (act === "delete-branch" || act === "delete-branch-go") {
            // On a branch git refuses, the row turns into the held
            // force-delete, which "-go" then runs to its end.
            page.openRefMenu("branch", arg, arg, branchesModel.oidOfName(arg))
            page.deleteRow("branch", arg, arg, branchesModel.oidOfName(arg))
            if (act === "delete-branch-go")
                forceDeleteTimer.start()
        } else if (act === "delete-tag" || act === "delete-tag-go") {
            page.openRefMenu("tag", arg, arg, tagsModel.oidOfName(arg))
            if (act === "delete-tag-go")
                refDeleteItem.completeHold()
        } else if (act === "delete-stash" || act === "delete-stash-go") {
            page.openRefMenu("stash", stashesModel.nameAt(0),
                             stashesModel.fullAt(0),
                             stashesModel.oidOfName(stashesModel.nameAt(0)))
            if (act === "delete-stash-go")
                refDeleteItem.completeHold()
        } else if (act === "delete-remote" || act === "delete-remote-go") {
            // Named outright (`origin/feature/x`) because those rows sit
            // behind a fold — opened here so the row is under the menu.
            remotesModel.toggleFolder(arg.substring(0, arg.indexOf("/")))
            page.openRefMenu("remote", arg, arg, remotesModel.oidOfName(arg))
            AppBackend.report("ref_menu kind=remote delete=" + refDeleteItem.code
                              + " " + refDeleteItem.text)
            if (act === "delete-remote-go")
                refDeleteItem.completeHold()
        } else if (act === "delete-force") {
            repoTab.deleteBranch(arg, true)
        } else if (act === "delete-branch-refused") {
            // Same entry as delete-branch; this one waits for git's
            // answer rather than acting on it.
            page.openRefMenu("branch", arg, arg, branchesModel.oidOfName(arg))
            page.deleteRow("branch", arg, arg, branchesModel.oidOfName(arg))
            refusedRowTimer.start()
        } else if (act === "chip-menu") {
            // Only the kind letter and the name of the record are read.
            page.openRecordMenu("L0000" + arg, branchesModel.oidOfName(arg))
            chipMenuTimer.start()
        } else if (act === "chip-menu-current") {
            page.openRecordMenu("L1001" + workTree.branch,
                                branchesModel.oidOfName(workTree.branch))
            chipMenuTimer.start()
        } else if (act === "delete-blocked-tip") {
            // Forced rather than hovered: the pointer cannot be put on a
            // row from here, and this writes to the property the real
            // hover writes to.
            page.openRecordMenu("L1001" + workTree.branch,
                                branchesModel.oidOfName(workTree.branch))
            refDeleteItem.tipForced = true
            blockedTipTimer.start()
        } else if (act === "menu-highlight") {
            // The keyboard's road to `highlighted` — the only one that
            // can be driven from here.
            page.openRecordMenu("L0000" + (arg === "" ? workTree.branch : arg),
                                branchesModel.oidOfName(
                                    arg === "" ? workTree.branch : arg))
            refMenu.currentIndex = 1
            AppBackend.report("menu_highlight index=" + refMenu.currentIndex)
        } else if (act === "delete-branch-early") {
            // The early answer dresses the delete row before any click;
            // the argument picks which half is on show.
            page.openRecordMenu("L0000" + arg, branchesModel.oidOfName(arg))
            earlyDeleteTimer.start()
        } else if (act === "stash-menu" || act === "delete-stash-row") {
            // The row menu on the first stash's row; the argument "go"
            // holds the delete row down.
            page.openRowMenu(stashesModel.oidOfName(stashesModel.nameAt(0)))
            AppBackend.report("row_menu stash=" + page.menuStashRef)
            if (act === "delete-stash-row" && arg === "go")
                stashDeleteItem.completeHold()
        } else if (act === "stash-apply-row" || act === "stash-pop-row") {
            page.openRowMenu(stashesModel.oidOfName(stashesModel.nameAt(0)))
            if (act === "stash-apply-row")
                repoTab.applyStash(page.menuStashRef)
            else
                repoTab.popStash(page.menuStashRef)
        } else if (act === "branch-at-tag") {
            sidebarPane.beginBranchAt(tagsModel.nameAt(0),
                                      tagsModel.oidOfName(tagsModel.nameAt(0)))
            sidebarPane.submitEdit(arg)
        } else if (act === "dbl-local" || act === "dbl-remote") {
            // The record is the chip as drawn (kind letter, four flags,
            // name — see encode.rs).
            page.activateRecord(
                (act === "dbl-local" ? "L0001" : "R0000") + arg)
        } else if (act === "move-branch") {
            // Past the question, for the write it guards.
            page.switchTo("force", repoTab.localNameFor(arg),
                          repoTab.localNameFor(arg), arg)
        } else if (act === "name-branch") {
            graphPane.view.namingSubmitted(graphModel.oidAt(0), arg)
        } else if (act === "ref-list" || act === "ref-list-card") {
            // Hover cannot be injected, so this enters where the hover
            // timer would. `-card` walks row → card → chip → asked again
            // from under the list: both card closes have to hold, and
            // either failing leaves `open=true`.
            const stacked = graphPane.view.itemAtIndex(Number(arg))
            if (stacked) {
                if (act === "ref-list-card")
                    graphPane.view.rowHoverRequested(stacked, true)
                graphPane.view.chipExpandRequested(
                    stacked.chipItem.records, stacked.chipItem)
                if (act === "ref-list-card") {
                    graphPane.view.rowHoverRequested(stacked, true)
                    rowCardTimer.start()
                }
            }
        } else if (act === "signature") {
            // The mark appears when the verify comes back, so the report
            // waits for it.
            page.activateRow(graphModel.oidAt(Number(arg)))
            signatureTimer.start()
        } else if (act === "signature-tip") {
            // Once the verify is back the mark is asked to say why.
            page.activateRow(graphModel.oidAt(Number(arg)))
            signatureTipTimer.start()
        } else if (act === "stash-tip") {
            // The box is asked why it refuses the caret.
            page.activateRow(graphModel.oidAt(Number(arg)))
            stashTipTimer.start()
        } else if (act === "path-tip") {
            // Row 0 is the elided leaf in the flattened view, the folder
            // chain in the tree (`-tree`). The argument picks the pane
            // the way `corner` does.
            const wantsTree = ("" + arg).endsWith("-tree")
            const pane = wantsTree ? ("" + arg).slice(0, -5) : arg
            pathTipTimer.wipSide = pane === "" || pane === "wip"
            if (pathTipTimer.wipSide) {
                page.showWip()
                worktreeModel.setTreeView(wantsTree)
            } else {
                page.activateRow(graphModel.oidAt(Number(pane)))
                detailsModel.setTreeView(wantsTree)
            }
            pathTipTimer.start()
        } else if (act === "row-card") {
            // Hover cannot be injected, so this enters where the row's
            // delay timer would.
            const hovered = graphPane.view.itemAtIndex(Number(arg))
            if (hovered)
                graphPane.view.rowHoverRequested(hovered, true)
            rowCardTimer.start()
        } else if (act === "author-card" || act === "author-card-open") {
            // Hover cannot be injected, so `-open` writes the same
            // property the handler writes; read the two as a pair —
            // "stayed shut" only means something next to a run where it
            // opened.
            page.activateRow(graphModel.oidAt(Number(arg)))
            authorCardTimer.start()
        } else if (act === "co-authors" || act === "co-authors-open") {
            // Same pairing as author-card.
            page.activateRow(graphModel.oidAt(Number(arg)))
            coAuthorTimer.start()
        } else if (act === "details-grow" || act === "details-grow-squeeze") {
            // The corner grip pulled past what the pane can spare;
            // `-squeeze` then takes the pane's room back with the log.
            page.activateRow(graphModel.oidAt(Number(arg)))
            descGrowTimer.pane = detailsPane
            descGrowTimer.paneName = "details"
            descGrowTimer.squeeze = act === "details-grow-squeeze"
            descGrowTimer.start()
        } else if (act === "wip-grow" || act === "wip-grow-squeeze") {
            // The argument is the description itself: the box starts
            // empty, so a run that types nothing has nothing to open.
            page.showWip()
            wipPane.setMessage("feat: write the summary", arg)
            descGrowTimer.pane = wipPane
            descGrowTimer.paneName = "wip"
            descGrowTimer.squeeze = act === "wip-grow-squeeze"
            descGrowTimer.start()
        } else if (act === "details-fit") {
            // Overflow shows as glyphs cut at the window's edge, which
            // headless cannot see, so the pane reports the number.
            // `--preset edges` holds the wall.
            page.activateRow(graphModel.oidAt(Number(arg)))
            detailsFitTimer.start()
        } else if (act === "corner") {
            // Both sides of the corner's one rule: preset `basic` leaves
            // the corner bare, `long` runs rows into it. Read as a pair —
            // one half alone frames like a label always on, or always off.
            if (arg === "" || arg === "wip")
                page.showWip()
            else
                page.activateRow(graphModel.oidAt(Number(arg)))
            cornerTimer.start()
        } else if (act === "graph-step" || act === "graph-step-edge"
                   || act === "graph-step-far" || act === "graph-step-named"
                   || act === "graph-step-dirty" || act === "graph-step-diff") {
            // Keystrokes cannot be injected, so the run enters at the
            // same `stepRow` the key handler enters — and takes the
            // keyboard first through the same call a row click makes,
            // since a graph nobody has pressed hears no arrows at all
            // (規約 §矢印で履歴を辿る). `-dirty` takes two steps: the
            // first raises the half-written-message question, the second
            // must not tug against it. `-edge` walks off the bottom;
            // `-far` sends the view away first so the stepped-off row is
            // off screen. `-diff` opens a file over the graph: the pane
            // swapped off screen has to let the keyboard go, or the
            // arrows walk the selection behind the diff.
            page.activateRow(branchesModel.headOid !== ""
                             ? branchesModel.headOid : graphModel.oidAt(0))
            graphStepTimer.named = act === "graph-step-named"
            graphStepTimer.dirty = act === "graph-step-dirty"
            graphStepTimer.away = act === "graph-step-far"
            graphStepTimer.diffPath = act === "graph-step-diff" ? arg : ""
            graphStepTimer.steps = act === "graph-step-named" ? 1
                                 : act === "graph-step-dirty" ? 2
                                 : act === "graph-step-edge" ? 10
                                 : act === "graph-step-far" ? 1
                                 : act === "graph-step-diff" ? 1
                                 : arg === "" ? 1 : Number(arg)
            graphStepTimer.start()
        } else if (act === "diff-step" || act === "diff-step-edge") {
            // Moves the view, not a selection (規約 §diff を上下に送る).
            // Rides the 320x240 seed: no demo file's diff is longer than
            // a default window, and even there the room below the fold is
            // two rows (実測) — which is why the plain walk is one row.
            page.showWip()
            page.toggleDiff("untracked", arg, "")
            diffStepTimer.steps = act === "diff-step-edge" ? 20 : 1
            diffStepTimer.start()
        } else if (act === "name-box") {
            graphPane.startNaming(graphModel.oidAt(Number(arg)))
        } else if (act === "graph-bar" || act === "graph-bar-away"
                   || act === "middle-scroll") {
            // Both want lanes that do not fit their column, and no demo
            // repository has that many — the divider is pulled in the way
            // a person would.
            page.setGraphColumns(graphPane.labelWManual,
                                 Metrics.laneInset + 2 * Metrics.laneW)
            graphPanTimer.start()
        } else if (act === "graph-min") {
            // Pulled past the floor so the clamp answers (the floor is
            // lane 0's co-author badge kept whole).
            page.setGraphColumns(graphPane.labelWManual, 0)
            AppBackend.report("graph_min w=" + graphPane.graphColW
                              + " min=" + graphPane.graphColWMin)
        } else if (act === "divider-refuse") {
            // A drag carried past one of a divider's bounds, named by the
            // argument. The log's bar is not on screen while the log is
            // shut — open it and let it lay out before measuring against
            // a bar that has no geometry yet.
            if (arg === "log-min" && !page.commandsOpen) {
                page.commandsOpen = true
                splitRefuseTimer.start()
            } else if (arg === "desc-max" || arg === "desc-min") {
                // Row 1, not row 0: row 0 of every preset is the
                // uncommitted row, and landing on it puts the working
                // tree in the right-hand pane — the box this pulls on
                // would be off screen.
                page.activateRow(graphModel.oidAt(1))
                descGrowTimer.pane = detailsPane
                descGrowTimer.paneName = "details"
                descGrowTimer.squeeze = false
                descGrowTimer.refuse = arg
                descGrowTimer.start()
            } else {
                page.reportDividerRefusal(arg)
            }
        } else if (act === "graph-divider") {
            // Read against two repositories: a line withheld on a linear
            // history is only an answer next to a run where it is drawn.
            graphPane.restDividerPointer(true)
            AppBackend.report("graph_divider shown=" + graphPane.graphDividerShown
                              + " line=" + graphPane.graphDividerLineShown
                              + " refuses=" + graphPane.graphDividerRefuses
                              + " lanes=" + graphModel.maxLanes
                              + " max=" + Math.round(graphPane.graphColWMax)
                              + " min=" + Math.round(graphPane.graphColWMin))
        } else if (act === "squash") {
            page.openRowMenu(branchesModel.headOid)
            page.squashCommit(branchesModel.headOid)
        } else if (act === "reword" || act === "edit-message"
                   || act === "edit-message-leave"
                   || act === "edit-message-discard"
                   || act === "edit-message-focus") {
            // Detached there is no branch tip to name, so the newest row
            // stands in — a commit other than HEAD.
            page.jumpToRef(branchesModel.headOid !== ""
                           ? branchesModel.headOid : graphModel.oidAt(0))
            rewordTimer.start()
        } else if (act === "cherry-pick") {
            const pickOid = driver.autoActOid(arg)
            const pickRow = graphModel.rowOf(pickOid)
            if (pickRow >= 0) {
                graphPane.jumpToRow(pickRow)
                page.activateRow(pickOid)
            }
            tipLandedTimer.start()
            repoTab.cherryPick(pickOid)
        } else if (act === "reset-soft" || act === "reset-mixed") {
            // With nothing given, the row under HEAD's: a reset to where
            // the branch already stands moves nothing, and an empty name
            // would reach git as `reset ''`.
            page.openRowMenu(arg === ""
                             ? graphModel.oidAt(
                                   graphModel.rowOf(workTree.headOid) + 1)
                             : driver.autoActOid(arg))
            page.moveBranchHere(act === "reset-soft" ? "soft" : "mixed")
        } else if (act === "reset-hard" || act === "reset-hard-confirm") {
            // "-confirm" stops with the held row on screen; "reset-hard"
            // runs the hold to its end. The row resolves as reset-soft's.
            page.openRowMenu(arg === ""
                             ? graphModel.oidAt(
                                   graphModel.rowOf(workTree.headOid) + 1)
                             : driver.autoActOid(arg))
            resetMenu.offer()
            if (act === "reset-hard")
                hardResetItem.completeHold()
        } else if (act === "commit-menu" || act === "reset-menu") {
            // With no row named, the row under HEAD's: most of this menu
            // is about a commit the branch is *not* already standing on,
            // and it is counted from where HEAD actually sits — the rows
            // above belong to whatever else the graph is showing.
            let menuOid = arg
            if (menuOid === "")
                menuOid = graphModel.oidAt(
                    graphModel.rowOf(workTree.headOid) + 1)
            page.openRowMenu(menuOid)
            if (act === "reset-menu")
                resetMenu.offer()
            AppBackend.report("commit_menu rows=" + commitMenu.offeredRows
                              + " can_move=" + page.menuCanMoveBranch)
        } else if (act === "wip") {
            page.showWip()
        } else if (act === "wip-tally") {
            // Read the two together: `status::Kinds` counts rows, so the
            // kinds have to add up to `rows` — a drift means one of the
            // two stopped reading the same status.
            page.showWip()
            AppBackend.report("wip_tally added=" + graphPane.view.wipAdded
                              + " modified=" + graphPane.view.wipModified
                              + " deleted=" + graphPane.view.wipDeleted
                              + " renamed=" + graphPane.view.wipRenamed
                              + " copied=" + graphPane.view.wipCopied
                              + " conflicted=" + graphPane.view.wipConflicted
                              + " rows=" + worktreeModel.total)
        } else if (act === "wip-message" || act === "wip-message-focus") {
            // A body is typed first because this editor starts empty, and
            // an empty box has no text to take a colour. Read as a pair:
            // the caret is the only difference between the two verbs.
            page.showWip()
            wipPane.setMessage("feat: write the summary",
                               arg === "" ? "And the description under it." : arg)
            if (act === "wip-message-focus")
                wipPane.focusDescription()
            AppBackend.report("message_focus pane=wip focused="
                              + wipPane.descriptionFocused
                              + " color=" + wipPane.descriptionColor)
        } else if (act === "drop-commit" || act === "drop-commit-go") {
            // The plan is built by object name, the way a graph row hands
            // one over — a symbolic name is not what this takes.
            page.openRowMenu(driver.autoActOid(arg))
            AppBackend.report("drop_row " + dropCommitItem.code
                              + " " + dropCommitItem.text
                              + " oid=" + page.menuOid.substring(0, 8)
                              + " hold=" + (dropCommitItem.holdMs > 0)
                              + " reached=" + repoTab.headReachedElsewhere)
            if (act === "drop-commit-go") {
                if (dropCommitItem.holdMs > 0)
                    dropCommitItem.completeHold()
                else
                    page.dropCommit(page.menuOid)
            }
        } else if (act === "merge-branch" || act === "rebase-onto"
                   || act === "revert-commit" || act === "integrate-menu") {
            // Through the menus a right-click opens, so the rows' own
            // gating decides whether anything runs.
            if (act === "revert-commit") {
                // The click that opens this menu selects the row too
                // (GraphRowDelegate), so the hook takes both steps a
                // right-click takes.
                const oidHex = driver.autoActOid(arg)
                graphPane.jumpToRow(graphModel.rowOf(oidHex))
                page.activateRow(oidHex)
                page.openRowMenu(oidHex)
                tipLandedTimer.start()
                repoTab.revert(oidHex)
            } else {
                page.openRefMenu("branch", arg, arg,
                                 branchesModel.oidOfName(arg))
                if (act === "merge-branch") {
                    tipLandedTimer.start()
                    repoTab.merge(arg, false, false, "")
                } else if (act === "rebase-onto") {
                    repoTab.rebase(arg, "", true)
                }
            }
        } else if (act === "op-exit" || act === "op-exit-go") {
            // "-go" runs the held row the argument names to its end.
            page.showWip()
            if (act === "op-exit-go")
                AppBackend.report("op_exit_held " + wipPane.completeOpExit(arg))
        } else if (act === "eol-commit") {
            // Nothing is committed — the shot is the state before anyone
            // decides.
            page.showWip()
            repoTab.stageAll()
            // `amend` asks for the other wording rather than for a
            // message: the two forms of this button differ in what they
            // say, and both have to be photographable.
            if (arg === "amend") {
                wipPane.setAmendChecked(true)
                page.amending = true
            }
            wipPane.setMessage(arg === "" || arg === "amend"
                               ? "feat: something" : arg, "")
            eolCommitTimer.start()
        } else if (act === "eol-hover") {
            // Hover cannot be injected; this writes the one property a
            // real pointer writes. The tip lands in overlay.png.
            page.showWip()
            wipPane.pointEol(arg)
            eolHoverTimer.start()
        } else if (act === "stage-hunk" || act === "stage-line"
                   || act === "discard-hunk" || act === "discard-hunk-go"
                   || act === "diff-file" || act === "line-tools"
                   || act === "hunk-tools" || act === "pick-lines"
                   || act === "stage-lines" || act === "keep-place") {
            // All enter through one file's diff and act on its first
            // hunk. The bucket rides in front of the path
            // (`<bucket>:<path>`) when it is not the usual unstaged one:
            // an untracked file has no unstaged diff at all, a conflicted
            // one is read from `conflicts`. Only the bucket names count
            // as one, so a path carrying a colon still opens.
            const cut = arg.indexOf(":")
            const head = cut > 0 ? arg.substring(0, cut) : ""
            const named = head === "staged" || head === "unstaged"
                          || head === "untracked" || head === "conflicts"
            page.showWip()
            // The source of a rename comes off the model rather than out
            // of the argument: a row hands it over when it is clicked, and
            // a run that opened the destination alone would photograph a
            // file git thinks appeared out of nowhere.
            const wtPath = named ? arg.substring(cut + 1) : arg
            page.toggleDiff(named ? head : "unstaged", wtPath,
                            worktreeModel.origOf(wtPath))
            stageRowTimer.begin()
        } else if (act === "colour-place") {
            page.showWip()
            page.toggleDiff("unstaged", arg, "")
            colourPlaceTimer.begin()
        } else if (act === "diff-fold" || act === "diff-unfold"
                   || act === "diff-fold-by-hand"
                   || act === "diff-fold-by-rename"
                   || act === "diff-keep-folded") {
            // "-by-hand" and "-by-rename" both bring the list back and so
            // take the diff down; "-keep-folded" had it folded before the
            // diff arrived, so closing the diff leaves it folded.
            page.showWip()
            if (act === "diff-keep-folded")
                page.foldByHand(true)
            page.toggleDiff("unstaged", arg, "")
            if (act === "diff-fold-by-hand")
                page.foldByHand(false)
            else if (act === "diff-fold-by-rename") {
                sidebarPane.peekAt("branch")
                sidebarPane.beginRename("branch", workTree.branch,
                                        workTree.branch)
            } else if (act !== "diff-fold")
                page.closeDiff()
            navRailTimer.start()
        } else if (act === "push") {
            page.pushNow()
        } else if (act === "force-push") {
            page.forcePush()
        } else if (act === "push-retry") {
            // A mark coming off is the absence of a thing, so the timer
            // reports the refused state before sending the go that
            // clears it.
            page.pushNow()
            pushRetryTimer.start()
        } else if (act === "fetch") {
            repoTab.fetch("")
        } else if (act === "fetch-ref-list") {
            // What a tag says about the remote only exists after a fetch
            // (`ls-remote --tags` carries it), so the two steps are one
            // verb.
            repoTab.fetch("")
            fetchedRefListTimer.start()
        } else if (act === "preview") {
            page.toggleDiff("untracked", arg, "")
        } else if (act === "preview-unstaged") {
            page.toggleDiff("unstaged", arg, "")
        } else if (act === "preview-staged") {
            page.toggleDiff("staged", arg, "")
        } else if (act === "open-picker") {
            page.openRepositoryPicker()
        } else if (act === "settings" || act === "settings-tools") {
            // `-tools` goes on to open the candidate list from inside the
            // dialog, and leaves the fetch interval where it was.
            page.settingsDialogRequested()
            if (act === "settings")
                AppBackend.setAutoFetchMinutes(Number(arg))
        } else if (act === "avatar-rest" || act === "avatar-hover"
                   || act === "avatar-assign" || act === "avatar-badge") {
            // The first ordinary commit — row 0 is WIP.
            page.activateRow(graphModel.oidAt(1))
            if (act === "avatar-hover" || act === "avatar-assign")
                detailsPane.avatarPointedAt = true
            if (act === "avatar-assign")
                avatarAssignTimer.start()
            if (act === "avatar-badge")
                avatarBadgeTimer.start()
        } else if (act === "avatar-settings" || act === "avatar-combo"
                   || act === "avatar-row-lit" || act === "avatar-remove") {
            // Each run starts with an empty store, so a picture to look
            // at has to be filed first — the argument is the one to file.
            page.activateRow(graphModel.oidAt(1))
            if (arg !== "")
                avatarSeedTimer.start()
            else
                page.settingsDialogRequested()
        } else if (act === "find" || act === "find-next" || act === "find-prev") {
            // The key cannot be pressed from here; assigning the text
            // runs the same search a keystroke runs.
            page.startFind()
            if (arg !== "")
                graphPane.findQuery = arg
            if (act === "find-next")
                graphPane.findNext()
            else if (act === "find-prev")
                graphPane.findPrevious()
            // `width` and `cap` are the two halves of the rule the long
            // queries are here to check: the card may grow, and it may
            // not reach past a subject's first character.
            AppBackend.report("find open=" + graphPane.findOpen
                              + " query=" + graphPane.findQuery
                              + " matches=" + graphPane.findMatches
                              + " at=" + graphPane.findAt
                              + " row=" + graphPane.view.currentIndex
                              + " selected=" + page.selectedOid.substring(0, 7)
                              + " width=" + Math.round(graphPane.findWidth)
                              + " cap=" + Math.round(graphPane.width - graphPane.subjectTextX)
                              + " clears=" + graphPane.findClears)
            findSettled.restart()
        } else if (act === "commands") {
            // Stage and unstage so the log has something in it.
            repoTab.stageAll()
            repoTab.unstageAll()
            page.toggleCommands()
        } else if (act === "commands-fail" || act === "commands-clear") {
            // A real refusal in git's own words, raising the panel by
            // itself. The clearing verb starts from the same failure
            // (`Main` waits for it, presses Clear, and reads the band).
            repoTab.checkoutBranch("pg-no-such-branch")
        } else if (act === "fetch-recover") {
            // A fetch that cannot land leaves a failure standing; `Main`
            // then fires one that can and reads what the success takes
            // down by itself.
            repoTab.fetch("pg-no-such-remote")
        } else if (act === "fetch-fail") {
            // The argument is how many failed fetches to run, so one verb
            // reaches the warning shape and the stopped one alike.
            driver.fetchFailRuns = Math.max(1, Number(arg))
            AppBackend.setAutoFetchMinutes(0)
            AppBackend.setAutoFetchMinutes(5)
            repoTab.fetch("")
        } else if (act === "fetch-resume") {
            driver.fetchFailRuns = 3
            driver.fetchResumeAfter = true
            AppBackend.setAutoFetchMinutes(0)
            AppBackend.setAutoFetchMinutes(5)
            repoTab.fetch("")
        }
        AppBackend.report("auto_act ran=" + act)
    }

    /// Automation: the dialog's own button, once it is up.
    Timer {
        id: publishAddTimer
        interval: 600
        onTriggered: {
            remoteDialog.submit()
            publishAnswerTimer.start()
        }
    }
    /// Automation: what the far side turned out to hold, once the remote
    /// has had time to answer.
    Timer {
        id: publishSettleTimer
        interval: 1200
        onTriggered: AppBackend.report("publish settled far=" + page.publishState
                                       + " code=" + graphPane.askCode
                                       + " hold=" + graphPane.askHold
                                       + " alert=" + graphPane.askAlert
                                       + " lease=" + (page.publishLease !== "")
                                       + " theirs=" + repoTab.remoteBranchTheirs)
    }
    /// Automation: the answer, given after the remote has had time to say
    /// what it has — the pill is dead until it has.
    Timer {
        id: publishAnswerTimer
        interval: 1200
        onTriggered: {
            // `far` is what the far side turned out to hold — the other
            // line's `state` is this end's own push state, and the two
            // answer different questions.
            AppBackend.report("publish answering far=" + page.publishState
                              + " unsure=" + page.publishUnsure
                              + " answerable=" + graphPane.askAnswerable)
            // The same gesture a person is given: a hold cannot be
            // answered by a click here either.
            if (page.publishRefused)
                graphPane.completeHold()
            else
                page.answerRowAsk()
        }
    }
    // The log has to be on screen and laid out before the bar above it has
    // a place to be measured from.
    Timer {
        id: splitRefuseTimer
        interval: 400
        onTriggered: page.reportDividerRefusal(AppBackend.autoActArg)
    }
}
