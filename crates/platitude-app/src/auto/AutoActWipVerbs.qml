pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The working tree's own verbs: what a commit is made of and what it is called — the message boxes, the amend
/// row, stashing, the line-ending card, and the seat a stopped merge is finished from. Built by `AutoActDriver`
/// only when a verb was given.
// An `Item` only because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    /// `var`, because naming the driver's type would be a circle: it is the file that builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var repoTab: driver.repoTab
    readonly property var workingTree: driver.workingTree
    readonly property var graphModel: driver.graphModel
    readonly property var detailsModel: driver.detailsModel
    readonly property var branchesModel: driver.branchesModel
    readonly property var unstagedModel: driver.unstagedModel
    readonly property var stashesModel: driver.stashesModel
    readonly property var graphPane: driver.graphPane
    readonly property var wipPane: driver.wipPane
    readonly property var gitCorner: driver.gitCorner
    readonly property var diffPane: driver.diffPane
    readonly property var fileRowMenu: driver.fileRowMenu
    readonly property var commitMenuState: driver.commitMenuState
    readonly property var stashDeleteItem: driver.stashDeleteItem

    /// Runs `act` if it is this family's and says whether it was; each verb belongs to one family (`AutoActDriver`).
    function run(act, arg) {
        if (act === "commit") {
            // A default message, as git refuses an empty one (amend needs none: empty there means `--no-edit`).
            repoTab.stageAll()
            wipPane.setMessage(arg === "" ? "chore: commit from the headless run" : arg, "")
            commitAnswer.armed = true
            page.commitNow()
        } else if (act === "commit-refused" || act === "notice-over-diff") {
            // Against a `pre-commit` hook that says no (`--preset hooked`): its words come down as a report
            // (デザイン規約 §答えの要らない報せ). `notice-over-diff` has a diff open in the middle, the one
            // arrangement where a bar over the page differs from one inside the graph.
            repoTab.stageAll()
            wipPane.setMessage("chore: something the hook will not have", "")
            if (act === "notice-over-diff")
                page.openDiff("staged", arg, "")
            page.commitNow()
            driver.barrierNotice.start()
        } else if (act === "stale-part") {
            // A stale part: the diff marks' own press with a fingerprint the bytes cannot match, answered by a report
            // (デザイン規約 §答えの要らない報せ). The argument is `<バケツ>:<パス>`.
            const cut = arg.indexOf(":")
            stalePartTimer.bucket = arg.substring(0, cut)
            stalePartTimer.path = arg.substring(cut + 1)
            page.openDiff(stalePartTimer.bucket, stalePartTimer.path, "")
            stalePartTimer.start()
        } else if (act === "report-tone") {
            // Through the page's door (`showReport`) with only the kind: each kind's dress is proven in
            // `tests/qml/tst_reportdress.qml`, and this proves the door carries it through to the bar.
            page.showReport(arg, "origin", "main", "")
            driver.barrierNotice.start()
        } else if (act === "amend") {
            // The message is supplied, so skip the prefill request that would otherwise land on top of it.
            wipPane.setAmendChecked(true)
            page.amending = true
            wipPane.setMessage(arg, "")
            commitAnswer.armed = true
            page.commitNow()
        } else if (act === "amend-reset-author") {
            // The authorship offer needs HEAD read first, so amend is toggled and the timer waits.
            wipPane.setAmendChecked(true)
            page.amendToggled(true)
            resetAuthorTimer.start()
        } else if (act === "stash" || act === "stash-lands") {
            // Through the band's button with the WIP pane left alone — the button does not need it
            // (デザイン規約 §変更を退避する). `stash-lands` opens it: where the reader stands is its subject. The
            // working-tree row going is the rebuild's edge (`graphGoneOid`). `named` leaves a summary in the commit
            // box to name the entry; the words are the driver's, as the argument cannot carry spaces.
            if (arg === "named") {
                driver.stashWanted = "feat: write the summary"
                wipPane.setMessage(driver.stashWanted, "")
            }
            // Armed now, ticks before the press, so a fetch answering in between is not taken for it.
            driver.beginWrite(act)
            stashPressTimer.fromWip = act === "stash-lands"
            stashPressTimer.start()
        } else if (act === "stash-file") {
            wipPane.chooseOnly("unstaged", arg)
            page.openFileMenu("unstaged", arg)
            fileRowMenu.sendPaths([arg])
            repoTab.stashPaths(wipPane.stashName)
        } else if (act === "amend-author") {
            // The boxes are in the commit editor, on screen only while the uncommitted row is selected.
            page.showWip()
            wipPane.setAmendChecked(true)
            page.amendToggled(true)
            amendAuthorTimer.start()
        } else if (act === "stash-menu" || act === "delete-stash-row") {
            // The row menu on the first stash's row; the argument "go" holds the delete row down.
            const menuOid = stashesModel.oidOfName(stashesModel.nameAt(0))
            page.openRowMenu(menuOid)
            Harness.report("row_menu stash=" + commitMenuState.menuStashRef)
            if (act === "delete-stash-row" && arg === "go") {
                // The drop takes this row off the graph — the same edge stash-pop-row waits on.
                driver.graphGoneOid = menuOid
                stashDeleteItem.completeHold()
            }
        } else if (act === "stash-apply-row" || act === "stash-pop-row") {
            const stashOid = stashesModel.oidOfName(stashesModel.nameAt(0))
            page.openRowMenu(stashOid)
            if (act === "stash-apply-row") {
                repoTab.applyStash(commitMenuState.menuStashRef)
            } else {
                // A pop's row leaves the graph, the rebuild's edge; an apply keeps the plain write barrier.
                driver.graphGoneOid = stashOid
                // Through the page, the menu's road, which reads the entry's name before it goes (`popStash`);
                // `back=` checks the box at the barrier.
                driver.popWanted = GitFacts.stashLabel(stashesModel.nameAt(0))
                page.popStash(commitMenuState.menuStashRef)
            }
        } else if (act === "wip-embedded") {
            // An embedded repository's row, whose pane answers with a sentence (core `details::embedded`). The
            // argument is its `<path>/`, trailing slash included.
            page.showWip()
            embeddedTimer.path = arg === "" ? "vendor/nest/" : arg
            embeddedTimer.begin()
        } else if (act === "wip") {
            page.showWip()
        } else if (act === "wip-lanes") {
            wipLanesTimer.start()
        } else if (act === "carried-read") {
            // The argument is the copy: in one an untracked file has no pieces to stage anyway, so only the staged
            // edit makes "no hunk puts a seat out" a claim about this pane.
            carriedReadTimer.start()
        } else if (act === "carried-stand") {
            // The same pane with nothing read yet — the argument is the copy, as above.
            carriedStandTimer.start()
        } else if (act === "wip-tally") {
            // The kinds must add up to `rows` (`status::Kinds` counts rows); a drift means two statuses were read.
            page.showWip()
            Harness.report("wip_tally added=" + graphPane.view.wipAdded
                              + " modified=" + graphPane.view.wipModified
                              + " deleted=" + graphPane.view.wipDeleted
                              + " renamed=" + graphPane.view.wipRenamed
                              + " copied=" + graphPane.view.wipCopied
                              + " conflicted=" + graphPane.view.wipConflicted
                              + " rows=" + unstagedModel.total)
        } else if (act === "wip-commit-half") {
            // Commits the staged half of `--preset dirty`, so the uncommitted row survives and is redrawn above the
            // new HEAD — staged as found, as staging everything would leave no row. `during` stops with the write out
            // and the picture not yet moved.
            page.showWip()
            wipPane.setMessage("feat: record the staged half", "")
            driver.headOidBefore = workingTree.headOid
            halfWatch.duringOnly = arg === "during"
            driver.pressWrite("commit", () => {
                page.commitNow()
                return true
            })
            if (halfWatch.duringOnly)
                halfDuringTimer.start()
            else
                halfSettledTimer.start()
        } else if (act === "wip-message" || act === "wip-message-focus") {
            // A body is typed because an empty box has no text to take a colour. Read the pair: only the caret differs.
            page.showWip()
            wipPane.setMessage("feat: write the summary",
                               arg === "" ? "And the description under it." : arg)
            if (act === "wip-message-focus")
                wipPane.focusDescription()
            Harness.report("message_focus pane=wip focused="
                              + wipPane.descriptionFocused
                              + " color=" + wipPane.descriptionColor)
        } else if (act === "merge-commit") {
            // Emptied first: the claim is that an empty box still records the merge's own message
            // (デザイン規約 §進行中の操作から出る).
            page.showWip()
            wipPane.clearMessage()
            mergeCommitTimer.begin()
            Harness.report("merge_commit pressed=" + wipPane.pressCommit())
        } else if (act === "op-exit" || act === "op-exit-go") {
            // "-go" runs the named row to its end (`OpExitRow.completeHold`). The argument is the bare word, as the
            // option parser turns away a positional starting with `--` (`verify::options`). Required: without it
            // nothing matches and the line says `op_exit_held false`.
            page.showWip()
            if (act === "op-exit-go")
                Harness.report("op_exit_held " + wipPane.completeOpExit("--" + arg))
            else
                opExitTimer.start()
        } else if (act === "op-exit-lands") {
            // "-go" followed to where it puts the reader, a status later than the write's own answer
            // (`AutoActDriver.awaitOpExitLanding`). The bare word as above, defaulting to `continue`.
            page.showWip()
            Harness.report("op_exit_held " + driver.pressWrite("op-exit", () => wipPane.completeOpExit(
                "--" + (arg === "" ? "continue" : arg))))
            driver.awaitOpExitLanding()
        } else if (act === "eol-commit") {
            // The shot is the state before anyone decides.
            page.showWip()
            repoTab.stageAll()
            // `amend` photographs the button's other wording.
            if (arg === "amend") {
                wipPane.setAmendChecked(true)
                page.amending = true
            }
            wipPane.setMessage(arg === "" || arg === "amend" ? "feat: something" : arg, "")
            // The pointer goes on before the tree settles, as a real hand's does; pointing after the warning would
            // leave that ordering untested.
            wipPane.pointAtCommit = true
            eolCommitTimer.start()
        } else if (act === "commit-face") {
            // The signing tick's tip on the commit button's face; writes what a real pointer writes (`AvatarButton`).
            page.showWip()
            wipPane.setMessage("feat: something", "")
            wipPane.signingPointedAt = true
            commitFaceTimer.start()
        } else if (act === "eol-hover") {
            // Writes what a real pointer writes (hover cannot be injected); the tip lands in overlay.png.
            page.showWip()
            wipPane.pointEol(arg)
            eolHoverTimer.start()
        } else {
            return false
        }
        return true
    }
    /// The editor's own answer, latched off its edge (`RepoPage.commitAnswered`). Not the write barrier: the press is
    /// two writes, staging then commit, and the barrier takes whichever answers first. `empty=` / `amend=` are read
    /// off the pane — the boxes let go and the amend row with them.
    Connections {
        target: acts.page
        enabled: commitAnswer.armed
        function onCommitAnswered(landed) {
            commitAnswer.armed = false
            Harness.report("commit_answered landed=" + landed
                              + " empty=" + (wipPane.subjectText === "" && wipPane.bodyText === "")
                              + " amend=" + page.amending)
            // Completed behind the status the commit's own answer asks for (`RepoTab.commitAnswer`) — the staging
            // answers first, with a status that predates the commit.
            driver.owedStatusAt(repoTab.commitAnswer)
            driver.barrierWrite.start()
        }
    }
    QtObject {
        id: commitAnswer
        property bool armed: false
    }
    // The row is chosen through its own signal, where a click lands — it is not there until the status arrives. Then
    // the pane's answer about this path is waited for (`DiffPane.diffSettled`).
    SampleTimer {
        id: embeddedTimer
        property string path: ""
        property bool clicked: false
        function begin() {
            embeddedTimer.clicked = false
            embeddedTimer.start()
        }
        onTriggered: {
            if (!embeddedTimer.clicked) {
                const row = wipPane.filesWalk.rowFor("untracked", embeddedTimer.path)
                if (!row)
                    return
                embeddedTimer.clicked = true
                row.fileClicked("untracked", embeddedTimer.path, "", Qt.NoModifier)
                return
            }
            if (!page.diffShown || page.diffPath !== embeddedTimer.path || !diffPane.diffSettled())
                return
            embeddedTimer.stop()
            // `sha8=`: the commit a stage would point at, empty where that repository has none — the pane's two
            // sentences for this row.
            Harness.report("wip_embedded path=" + page.diffPath
                              + " embedded=" + diffPane.diffModel.embedded
                              + " rows=" + diffPane.view.count
                              + " sha8=" + diffPane.diffModel.embeddedSha8)
            driver.complete()
        }
    }
    // The press has to land on rows that are on screen, so the diff is read first.
    SampleTimer {
        id: stalePartTimer
        property string bucket: ""
        property string path: ""
        onTriggered: {
            if (!page.diffShown || driver.diffPane.diffModel.loading)
                return
            stalePartTimer.stop()
            driver.pressWrite("stage-selection", () => {
                repoTab.stageSelection(stalePartTimer.bucket, stalePartTimer.path, "", 0, -1, 1)
                return true
            })
            driver.barrierNotice.start()
        }
    }
    // Another working copy's uncommitted row read in this window, with every writing control down
    // (P3-確認事項 §別 worktree の未コミット行). Every field is read off the output side — built from `writable`,
    // each would go green with the disabling unwired (app-ui.md §UI 自動化).
    SampleTimer {
        id: carriedReadTimer
        property bool asked: false
        property bool opened: false
        /// The step to the next file: made, whether it moved, and from which file. A one-file copy steps and stays,
        /// so no move is waited for there.
        property bool walked: false
        property bool moved: false
        property string leftPath: ""
        onTriggered: {
            if (graphModel.finishCount === 0 || !workingTree.loaded)
                return
            if (!carriedReadTimer.asked) {
                const row = driver.rowOfCopy(Harness.autoActArg)
                if (row < 0)
                    return
                // Through the row itself, so the row's own decision is the one taken (verify-ui §壊れない動詞の実装と反復).
                const item = graphPane.view.itemAtIndex(row)
                if (item === null)
                    return
                item.leftClick(Qt.NoModifier)
                carriedReadTimer.asked = true
                return
            }
            // Until the copy's own files arrive the lists hold this window's.
            const files = page.wipUnstaged
            if (page.carriedPath === "" || files.carriedAt !== page.carriedPath || files.total === 0)
                return
            if (!carriedReadTimer.opened) {
                // The pane's own numbering — one run, and a folder row in the tree view answers with no key at all.
                let key = ""
                for (let i = 0; key === "" && i < files.shownRows; i++)
                    key = files.fileKeyAt(i)
                if (key === "")
                    return
                const cut = key.indexOf(":")
                // Pressed on the row, so what is read is what the row makes of the model — handing the model's answer
                // to `openDiff` would skip that (verify-ui §壊れない動詞の実装と反復). Null until the list builds the row.
                const row = driver.carriedPane.filesWalk.rowFor(key.substring(0, cut), key.substring(cut + 1))
                if (row === null)
                    return
                row.press()
                carriedReadTimer.opened = true
                return
            }
            if (!driver.diffPane.diffSettled())
                return
            // A cut name is whole only in its hover tip, which waits out `tipDelayMs`; a name that fits has none.
            driver.carriedPane.namePointedAt = true
            if (driver.carriedPane.nameCut && !driver.carriedPane.nameTipShown)
                return
            if (carriedReadTimer.walked) {
                // The run ends on the file the step asked for — a pane still reading it photographs empty. A step
                // that moved nothing has no second read to wait for.
                if (carriedReadTimer.moved
                        && (!page.diffShown || page.diffPath === carriedReadTimer.leftPath))
                    return
                carriedReadTimer.stop()
                driver.complete()
                return
            }
            // Read before the step below, which opens another file.
            const lines = driver.diffPane.view.count
            // `lit=` is only the first lit row (as `file_step` reads it), so a list lighting every row still passes
            // it; `litRows=` counts them (`FileRowWalk.litRows`).
            const lit = driver.carriedPane.filesWalk.litPath() === page.diffPath
            const litRows = driver.carriedPane.filesWalk.litRows()
            // One arrow step through the pane's own walk. Rows are found by the side their bytes are on even in this
            // one list — handed the run's name for the bucket, the walk starts from nowhere and stays put.
            carriedReadTimer.leftPath = page.diffPath
            const stepped = driver.carriedPane.filesWalk.stepFile(1, false)
            carriedReadTimer.walked = true
            carriedReadTimer.moved = stepped
            // The preset's copy holds one path staged and written again, so `files=1` and a `tally=` of one are the
            // fold (per side both would be two). `cut=`/`tip=`: the band cuts the long name, its hover carries it.
            // `lines=` goes last, past what the table pins (`verify::outcome` matches a substring). The tally is in
            // the order the row draws it.
            const tally = graphModel.carriedTally(page.selectedRow)
            const counts = tally ? [tally.added, tally.modified, tally.deleted,
                                    tally.renamed, tally.copied, tally.conflicted] : []
            Harness.report("carried_read copy=" + files.carriedName
                              + " files=" + files.total
                              + " tally=" + counts.join(",")
                              + " lit=" + lit
                              + " litRows=" + litRows
                              + " cut=" + driver.carriedPane.nameCut
                              + " tip=" + driver.carriedPane.nameTipShown
                              + " stepped=" + stepped
                              + " stageFile=" + driver.diffPane.stageOffered
                              + " pieces=" + driver.diffPane.piecesOffered
                              + " lines=" + lines)
        }
    }
    // The same pane at rest, nothing read: `litRows=` must be 0 here, which `carried-read` cannot show. The corner is
    // claimed here too — `corner` cannot reach another copy's pane.
    SampleTimer {
        id: carriedStandTimer
        property bool asked: false
        /// The first folder row's path, empty where every path is at the root. This model folds by `<run>:<path>`, so
        /// a row handed the fold key as its path says `whole:src` (`FileRowDelegate.foldKey`).
        function folderPath() {
            const view = driver.carriedPane.view
            for (let i = 0; i < view.count; i++) {
                const row = view.itemAtIndex(i)
                if (row && row.isFolder === true)
                    return row.pathText
            }
            return ""
        }
        onTriggered: {
            if (graphModel.finishCount === 0 || !workingTree.loaded)
                return
            if (!carriedStandTimer.asked) {
                // The same road in as `carried-read`.
                const row = driver.rowOfCopy(Harness.autoActArg)
                if (row < 0)
                    return
                const item = graphPane.view.itemAtIndex(row)
                if (item === null)
                    return
                item.leftClick(Qt.NoModifier)
                carriedStandTimer.asked = true
                return
            }
            // The copy's own files arrived and their rows built: until then nothing is lit or bare.
            const files = page.wipUnstaged
            if (page.carriedPath === "" || files.carriedAt !== page.carriedPath || files.total === 0)
                return
            if (driver.carriedPane.view.count === 0 || gitCorner.width <= 0)
                return
            carriedStandTimer.stop()
            // `corner=` is the label's own visibility; the room it was handed would go green with the binding cut.
            Harness.report("carried_stand copy=" + files.carriedName
                              + " files=" + files.total
                              + " litRows=" + driver.carriedPane.filesWalk.litRows()
                              + " reading=" + page.diffShown
                              + " corner=" + gitCorner.visible
                              + " folderPath=" + carriedStandTimer.folderPath()
                              + " room=" + Math.round(gitCorner.roomLeft))
            driver.complete()
        }
    }
    SampleTimer {
        id: wipLanesTimer
        onTriggered: {
            if (driver.graphTopKind() !== "wip")
                return
            wipLanesTimer.stop()
            Harness.report("wip_lanes geometry=" + graphModel.geometryAt(0)
                              + " lanes=" + graphModel.maxLanes)
            driver.complete()
        }
    }
    // Waits for HEAD's author; until it arrives the offer to take it over is not in the picture.
    SampleTimer {
        id: amendAuthorTimer
        onTriggered: {
            if (repoTab.headAuthorName === "")
                return
            amendAuthorTimer.stop()
            Harness.report("amend_author differs=" + repoTab.headAuthorDiffers
                              + " name=" + repoTab.headAuthorName)
            driver.complete()
        }
    }
    QtObject {
        id: halfWatch
        property bool duringOnly: false
        /// The pair below and what the page held, latched at the edge — a later read gives the settled page's numbers.
        property bool caught: false
        property bool sawWipRow: false
        property int sawRows: -1
    }
    // The write out and nothing moved: busy up and HEAD unmoved, as busy alone outlasts the write's answer. Read on
    // the tab's notify (rules-refs/app-ui.md「一瞬だけ立つ状態は…」): the whole window can pass within one turn of
    // the loop, which a sampler beat would miss.
    Connections {
        target: acts.repoTab
        enabled: halfWatch.duringOnly && !halfWatch.caught
        function onBusyCountChanged() {
            if (repoTab.busyCount === 0 || workingTree.headOid !== driver.headOidBefore)
                return
            halfWatch.sawWipRow = graphModel.wipRow
            halfWatch.sawRows = unstagedModel.total
            halfWatch.caught = true
        }
    }
    // Completed a turn after the notify that carried the edge. The run ends either way: a commit that landed with no
    // notify raising busy had no `during` to stop at, and says so on a line outside what the verb is judged on.
    SampleTimer {
        id: halfDuringTimer
        onTriggered: {
            if (!halfWatch.caught) {
                if (workingTree.headOid === driver.headOidBefore)
                    return
                halfDuringTimer.stop()
                Harness.report("wip_half_missed the commit landed before any notify carried a raised busy, so this "
                                  + "run had no moment with the write out and the picture still to stop at")
                driver.complete()
                return
            }
            halfDuringTimer.stop()
            // What the latch found; the page's numbers are the latched ones, not the settled page's.
            Harness.report("wip_half stage=during busy=true moved=false"
                              + " wipRow=" + halfWatch.sawWipRow
                              + " rows=" + halfWatch.sawRows)
            driver.complete()
        }
    }
    // The far side: the write settled, the graph holding the new HEAD, and the row drawn again above it.
    SampleTimer {
        id: halfSettledTimer
        onTriggered: {
            if (!driver.wroteAndSettled()
                    || workingTree.headOid === driver.headOidBefore
                    || graphModel.rowOf(workingTree.headOid) < 0
                    || !graphModel.wipRow)
                return
            halfSettledTimer.stop()
            Harness.report("wip_half stage=after busy=false"
                              + " moved=" + (workingTree.headOid !== driver.headOidBefore)
                              + " wipRow=" + graphModel.wipRow
                              + " rows=" + unstagedModel.total)
            driver.complete()
        }
    }
    // Waits for HEAD's author, before which there is no offer to tick.
    SampleTimer {
        id: resetAuthorTimer
        onTriggered: {
            if (repoTab.headAuthorName === "")
                return
            resetAuthorTimer.stop()
            Harness.report("head_author differs="
                              + repoTab.headAuthorDiffers
                              + " name=" + repoTab.headAuthorName)
            wipPane.setResetAuthorChecked(true)
            wipPane.setMessage(Harness.autoActArg, "")
            // A deferred run: nothing raises its barrier, so the commit is sent and waited out here. HEAD is re-read
            // now — the page's opening fetch can have answered since `prepareCompletion` read it.
            driver.headOidBefore = workingTree.headOid
            driver.pressWrite("commit", () => {
                page.commitNow()
                return true
            })
            resetAuthorLandedTimer.start()
        }
    }
    // Past the write barrier: the amend answers before the rebuild, so the barrier frames the replaced commit. The
    // edge is the graph holding the new HEAD — not the old one going, which a stash on top keeps drawn. Where the
    // selection lands differs by repository, so the pane is waited for and `shown=` says whose author is pictured.
    SampleTimer {
        id: resetAuthorLandedTimer
        onTriggered: {
            if (!driver.wroteAndSettled()
                    || workingTree.headOid === driver.headOidBefore
                    || graphModel.rowOf(workingTree.headOid) < 0
                    || !driver.cardSettled)
                return
            resetAuthorLandedTimer.stop()
            Harness.report("reset_author was=" + driver.headOidBefore.substring(0, 8)
                              + " head=" + workingTree.headOid.substring(0, 8)
                              + " shown=" + detailsModel.shaHex.substring(0, 8)
                              + " author=" + detailsModel.authorName
                              + " committer=" + detailsModel.committerName)
            driver.complete()
        }
    }
    // What stands beside the exit card (デザイン規約 §進行中の操作から出る): a stopped rebase, pick or revert hides the
    // message boxes and the commit button, a stopped merge keeps them — its `--continue` is that button. Read off the
    // drawn items (an item's `visible` is false under a hidden parent), once the card stands on the working-tree face.
    SampleTimer {
        id: opExitTimer
        onTriggered: {
            if (!page.wipShown || !wipPane.offersOpExit("--abort"))
                return
            opExitTimer.stop()
            Harness.report("op_exit card=" + wipPane.offersOpExit("--abort")
                              + " box=" + wipPane.commitBlock.messageSeat.visible
                              + " button=" + wipPane.commitBlock.commitSeat.visible
                              + " op=" + workingTree.opText)
            driver.complete()
        }
    }
    // Staging has to land before the button can know what it carries.
    SampleTimer {
        id: eolCommitTimer
        onTriggered: {
            // Only the card is waited for — asking after the count first would be reading the input side.
            if (!wipPane.eolCardOpen)
                return
            eolCommitTimer.stop()
            Harness.report("eol_commit staged=" + workingTree.stagedCount
                              + " warned=" + workingTree.eolStagedCount
                              + " card=" + wipPane.eolCardOpen)
            driver.complete()
        }
    }
    // The tick's tip comes out on the shared delay, so the setting being on is not yet the tip being up.
    SampleTimer {
        id: commitFaceTimer
        onTriggered: {
            if (!wipPane.signingTipShown)
                return
            commitFaceTimer.stop()
            Harness.report("commit_face signs=" + repoTab.signsCommits
                              + " tip=" + wipPane.signingTipShown)
            driver.complete()
        }
    }
    // The marks arrive with the status read, so the row named for its sentence has to be named again once they are in.
    SampleTimer {
        id: eolHoverTimer
        onTriggered: {
            wipPane.pointEol(Harness.autoActArg)
            if (!wipPane.eolCardOpen)
                return
            eolHoverTimer.stop()
            Harness.report("eol_hover path=" + wipPane.pointedEolPath
                              + " card=" + wipPane.eolCardOpen
                              + " text=" + wipPane.pointedEolText)
            driver.complete()
        }
    }
    // The band's Stash button, pressed until it takes: it is down while the opening fetch keeps the tab busy. The row
    // that has to go is read here, once the working-tree row leads — it can arrive after the graph's first pass (a
    // stopped merge), and read earlier the verb would wait on the newest commit.
    SampleTimer {
        id: stashPressTimer
        /// `stash-lands`: pressed from the working-tree row with its pane open, taken here so the row is there.
        property bool fromWip: false
        onTriggered: {
            if (driver.graphTopKind() !== "wip" || page.pageBand === null)
                return
            const going = graphModel.oidAt(0)
            if (stashPressTimer.fromWip) {
                graphPane.setCurrentRow(0)
                page.showWip()
            }
            if (!driver.inputWent(page.pageBand.stashNow()))
                return
            driver.graphGoneOid = going
            stashPressTimer.stop()
        }
    }
    // Two waits: the write lands, then HEAD's own message comes back — only the new commit can say the empty box
    // committed the merge's words.
    SampleTimer {
        id: mergeCommitTimer
        property string wanted: ""
        property bool typed: false
        property int seenHead: -1
        property string headWas: ""
        function begin() {
            mergeCommitTimer.wanted = workingTree.opSubject
            mergeCommitTimer.headWas = workingTree.headOid
            // Read now: the landing clears the editor, so afterwards every run would say the boxes were empty.
            mergeCommitTimer.typed = wipPane.subjectText !== "" || wipPane.bodyText !== ""
            mergeCommitTimer.seenHead = -1
            mergeCommitTimer.start()
        }
        onTriggered: {
            if (!driver.wroteAndSettled())
                return
            // The graph holding the commit, checked against the id HEAD moved from: refs and the walk arrive behind
            // the write and each other, so the old tip would pass.
            if (!workingTree.headKnown || workingTree.headOid === mergeCommitTimer.headWas
                    || graphModel.rowOf(workingTree.headOid) < 0
                    || driver.graphTopKind() === "wip")
                return
            if (mergeCommitTimer.seenHead < 0) {
                mergeCommitTimer.seenHead = repoTab.headCommitSeq
                repoTab.requestHeadCommit()
                return
            }
            if (repoTab.headCommitSeq === mergeCommitTimer.seenHead)
                return
            mergeCommitTimer.stop()
            Harness.report(
                "merge_committed merging=" + (workingTree.opText !== "")
                + " kept=" + (mergeCommitTimer.wanted !== ""
                              && repoTab.headSubject === mergeCommitTimer.wanted)
                + " typed=" + mergeCommitTimer.typed
                + " head=" + repoTab.headSubject)
            driver.complete()
        }
    }
}
