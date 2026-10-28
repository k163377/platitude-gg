pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The worktree's own verbs: what a commit is made of and what it is called — the message boxes, the amend
/// row, stashing, the line-ending card, and the seat a stopped merge is finished from. Built by `AutoActDriver`
/// only when a verb was given.
// An `Item` only because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    /// `var`, because naming the driver's type would be a circle: it is the file that builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var repoTab: driver.repoTab
    readonly property var worktree: driver.worktree
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

    /// The commit's landing, read at every picture (`landed_on_head`, `verify::verbs::CLAIMS`): the tree emptied,
    /// the WIP face left and HEAD's row selected — HEAD as the write left it, not the row the report in hand named
    /// (`RepoPage.leaveWipWhenDone`); a tree with files left keeps the face.
    function claimLanding() {
        Awaited.claim("landed_on_head", () => page.treeClean
            ? (!page.wipShown && page.selectedOid === worktree.headOid)
            : page.wipShown)
    }

    /// Runs `act` if it is this family's and says whether it was; each verb belongs to one family (`AutoActDriver`).
    function run(act, arg) {
        if (act === "commit") {
            // A default message, as git refuses an empty one (amend needs none: empty there means `--no-edit`).
            repoTab.stageAll()
            wipPane.setMessage(arg === "" ? "chore: commit from the headless run" : arg, "")
            acts.claimLanding()
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
        } else if (act === "notice-link") {
            // The one report whose own line names the command log (`Words.reportPlace`), raised through the page's door
            // as `report-tone` raises one, its place word pressed once the bar is down.
            page.showReport("carried-refused", "", "topic", "")
            noticeLinkTimer.start()
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
            acts.claimLanding()
            page.commitNow()
        } else if (act === "amend-reset-author") {
            // The authorship offer needs HEAD read first, so amend is toggled and the timer waits.
            wipPane.setAmendChecked(true)
            page.amendToggled(true)
            resetAuthorTimer.start()
        } else if (act === "stash" || act === "stash-lands") {
            // Through the band's button with the WIP pane left alone — the button does not need it
            // (デザイン規約 §変更を退避する). `stash-lands` opens it: where the reader stands is its subject. The
            // worktree row going is the rebuild's edge (`graphGoneOid`). `named` leaves a summary in the commit
            // box to name the entry; the words are the driver's, as the argument cannot carry spaces.
            if (arg === "named") {
                driver.expectStashNamed("feat: write the summary")
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
            // Read as the menu opens: under `go` the drop takes the entry, and the menu with it, before the completion.
            driver.act.witness("menuStash", commitMenuState.menuStashRef)
            driver.reportAtCompletion(() => "row_menu stash=" + driver.act.witnessed("menuStash"))
            if (act === "delete-stash-row" && arg === "go") {
                // The drop takes this row off the graph — the same edge stash-pop-row waits on.
                driver.awaitGraphGone(menuOid)
                stashDeleteItem.completeHold()
            }
        } else if (act === "stash-apply-row" || act === "stash-pop-row") {
            const stashOid = stashesModel.oidOfName(stashesModel.nameAt(0))
            page.openRowMenu(stashOid)
            if (act === "stash-apply-row") {
                repoTab.applyStash(commitMenuState.menuStashRef)
            } else {
                // A pop's row leaves the graph, the rebuild's edge; an apply keeps the plain write barrier.
                driver.awaitGraphGone(stashOid)
                // Through the page, the menu's road, which reads the entry's name before it goes (`popStash`);
                // `back=` checks the box at the barrier.
                driver.expectPopBack(GitFacts.stashLabel(stashesModel.nameAt(0)))
                page.popStash(commitMenuState.menuStashRef)
            }
        } else if (act === "wip-embedded") {
            // An embedded repository's row, whose pane answers with a sentence (core `details::embedded`). The
            // argument is its `<path>/`, trailing slash included.
            page.showWip()
            embeddedTimer.path = arg === "" ? "vendor/nest/" : arg
            embeddedTimer.start()
        } else if (act === "wip") {
            page.showWip()
        } else if (act === "wip-lanes") {
            wipLanesTimer.start()
        } else if (act === "carried-read") {
            // The argument is the worktree: in one an untracked file has no pieces to stage anyway, so only the staged
            // edit makes "no hunk puts a seat out" a claim about this pane.
            carriedReadTimer.start()
        } else if (act === "carried-stand") {
            // The same pane with nothing read yet — the argument is the worktree, as above.
            carriedStandTimer.start()
        } else if (act === "carried-menu" || act === "carried-apply" || act === "carried-apply-refused") {
            // The menu on that worktree's uncommitted row — the argument is the worktree, as above — and, for the
            // two that apply, its work brought here.
            carriedMenuTimer.applying = act !== "carried-menu"
            // The apply arms its write and presses the row; the bare menu is read where it stands.
            carriedMenuTimer.order = carriedMenuTimer.applying ? ["up", "armed", "pressed"] : ["up"]
            carriedMenuTimer.start()
        } else if (act === "wip-tally") {
            // The kinds must add up to `rows` (`status::Kinds` counts rows); a drift means two statuses were read.
            page.showWip()
            driver.reportAtCompletion(() => "wip_tally added=" + graphPane.view.wipAdded
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
            // HEAD as the write finds it: `during` stops with it unmoved, the far side waits for it to move.
            driver.act.witness("headBefore", worktree.headOid)
            // Started ahead of the press, so the latch (`halfWatch`) listens from the press on.
            if (halfWatch.duringOnly)
                halfDuringTimer.start()
            else
                halfSettledTimer.start()
            driver.pressWrite("commit", () => {
                page.commitNow()
                return true
            })
        } else if (act === "wip-message" || act === "wip-message-focus") {
            // A body is typed because an empty box has no text to take a colour. Read the pair: only the caret differs.
            page.showWip()
            wipPane.setMessage("feat: write the summary",
                               arg === "" ? "And the description under it." : arg)
            if (act === "wip-message-focus")
                wipPane.focusDescription()
            driver.reportAtCompletion(() => "message_focus pane=wip focused="
                                            + wipPane.descriptionFocused
                                            + " color=" + wipPane.descriptionColor)
        } else if (act === "merge-commit") {
            // Emptied first: the claim is that an empty box still records the merge's own message
            // (デザイン規約 §進行中の操作から出る).
            page.showWip()
            wipPane.clearMessage()
            mergeCommitTimer.wanted = worktree.opSubject
            mergeCommitTimer.headWas = worktree.headOid
            // Read now: the landing clears the editor, so afterwards every run would say the boxes were empty.
            mergeCommitTimer.typed = wipPane.subjectText !== "" || wipPane.bodyText !== ""
            mergeCommitTimer.start()
            // The press's own answer, read by the completion `mergeCommitTimer` makes.
            driver.act.witness("commitPressed", wipPane.pressCommit())
            driver.reportAtCompletion(() => "merge_commit pressed=" + driver.act.witnessed("commitPressed"))
        } else if (act === "op-exit" || act === "op-exit-go") {
            // "-go" runs the named row to its end (`OpExitRow.completeHold`). The argument is the bare word, as the
            // option parser turns away a positional starting with `--` (`verify::options`). Required: without it
            // nothing matches and the line says `op_exit_held false`.
            page.showWip()
            if (act === "op-exit-go") {
                driver.act.witness("held", wipPane.completeOpExit("--" + arg))
                driver.reportAtCompletion(() => "op_exit_held " + driver.act.witnessed("held"))
            } else {
                opExitTimer.start()
            }
        } else if (act === "op-exit-lands") {
            // "-go" followed to where it puts the reader, a status later than the write's own answer
            // (`AutoActDriver.awaitOpExitLanding`). The bare word as above, defaulting to `continue`.
            page.showWip()
            driver.act.witness("held", driver.pressWrite("op-exit", () => wipPane.completeOpExit(
                "--" + (arg === "" ? "continue" : arg))))
            // Handed to the completion the landing makes (`AutoActLandings.opExitLandTimer`).
            driver.reportAtCompletion(() => "op_exit_held " + driver.act.witnessed("held"))
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
        } else if (act === "amend-row") {
            // Whether the amend row stands over this tree — a stopped merge takes it away (`WipCommitBlock.amendBarred`,
            // `--preset conflict-staged`). With a branch for the argument (`--preset clashing`), the box is ticked as a
            // hand ticks it and then that branch is merged, to stop on its conflict: the stop takes the tick away with
            // the row, or the button would still send `--amend` (`WipCommitBlock.onAmendBarredChanged`).
            page.showWip()
            if (arg !== "") {
                wipPane.setAmendChecked(true)
                page.amendToggled(true)
                amendRowTimer.merged = arg
                repoTab.merge(arg, false, false, "")
            }
            amendRowTimer.start()
        } else if (act === "amend-tip") {
            // The commit button's refusal with the amend box ticked over a stage that takes back all HEAD changed
            // (`--preset undone`): the tip comes up once git has answered whether the amend would leave HEAD empty
            // and HEAD's message has filled the box. Ticked as a hand ticks it, so the message is asked for too.
            page.showWip()
            wipPane.setAmendChecked(true)
            page.amendToggled(true)
            amendTipTimer.start()
        } else if (act === "commit-face") {
            // The signing tick's tip on the commit button's face; writes what a real pointer writes (`AvatarButton`).
            page.showWip()
            wipPane.setMessage("feat: something", "")
            wipPane.signingPointedAt = true
            commitFaceTimer.start()
        } else if (act === "eol-hover") {
            // Writes what a real pointer writes (hover cannot be injected); the tip lands in overlay.png. The argument
            // is the path, or `<bucket>:<path>` for the same path shown in two buckets.
            page.showWip()
            wipPane.pointEol(arg)
            eolHoverTimer.start()
        } else if (act === "eol-gone") {
            // The card out with the hand inside it, and the warning gone with the status read: the row's file staged
            // away from under it, or (`button`) the staged count fallen to nothing. `inside` stops with the card out
            // and the hand in it, before the write (`button:inside` for the button's).
            page.showWip()
            const goneParts = ("" + arg).split(":")
            eolGoneTimer.button = goneParts.indexOf("button") >= 0
            eolGoneTimer.inside = goneParts.indexOf("inside") >= 0
            eolGoneTimer.order = [eolGoneTimer.button ? "staged" : "pointed"]
                .concat(eolGoneTimer.inside ? ["held"] : ["away", "lifted"])
            eolGoneTimer.start()
        } else if (act === "eol-walk" || act === "eol-stay") {
            // The rows moved under a still hand with the card out. `eol-walk` sends the list a whole row, so another
            // row arrives under the hand; `eol-stay` sends it a pixel, so the hand stays on its row. The argument is
            // the stage to stop at — `gone` / `back` (`back` when none).
            page.showWip()
            eolWalkTimer.stay = act === "eol-stay"
            eolWalkTimer.stopAt = arg === "" ? "back" : arg
            eolWalkTimer.order = eolWalkTimer.stopAt === "gone" ? ["pointed", "scrolled"]
                                                                : ["pointed", "scrolled", "tremor"]
            eolWalkTimer.start()
        } else {
            return false
        }
        return true
    }
    /// The editor's own answer, latched off its edge (`RepoPage.commitAnswered`). Not the write barrier: the press is
    /// two writes, staging then commit, and the barrier takes whichever answers first. `empty=` / `amend=` are read
    /// off the pane at the answer — the boxes let go and the amend row with them — kept as witnesses of the write
    /// barrier this starts, and said by the completion it makes. Heard once, on the run of a verb that presses the
    /// editor's commit.
    Connections {
        target: acts.page
        enabled: commitAnswer.armed
        function onCommitAnswered(landed) {
            commitAnswer.answered = true
            // Completed behind the status the commit's own answer asks for (`RepoTab.commitAnswer`) — the staging
            // answers first, with a status that predates the commit.
            driver.owedStatusAt(repoTab.commitAnswer)
            const barrier = driver.barrierWrite
            barrier.start()
            barrier.witness("landed", landed)
            barrier.witness("emptied", wipPane.subjectText === "" && wipPane.bodyText === "")
            barrier.witness("amend", page.amending)
            driver.reportAtCompletion(() => "commit_answered landed=" + driver.barrierWrite.witnessed("landed")
                                      + " empty=" + driver.barrierWrite.witnessed("emptied")
                                      + " amend=" + driver.barrierWrite.witnessed("amend"))
        }
    }
    readonly property QtObject commitAnswer: QtObject {
        id: commitAnswer
        readonly property bool armed: (Harness.autoAct === "commit" || Harness.autoAct === "amend")
                                      && !commitAnswer.answered
        property bool answered: false
    }
    // The row is chosen through its own signal, where a click lands — it is not there until the status arrives. Then
    // the pane's answer about this path is waited for (`DiffPane.diffSettled`).
    PageStage {
        id: embeddedTimer
        name: "wip_embedded"
        order: ["clicked"]
        property string path: ""
        onTriggered: {
            if (embeddedTimer.at("start")) {
                // No row yet is no click: the beat after asks again.
                embeddedTimer.input("clicked", () => {
                    const row = wipPane.filesWalk.rowFor("untracked", embeddedTimer.path)
                    if (!row)
                        return false
                    row.fileClicked("untracked", embeddedTimer.path, "", Qt.NoModifier)
                    return true
                })
                return
            }
            // `sha8=`: the commit a stage would point at, empty where that repository has none — the pane's two
            // sentences for this row.
            embeddedTimer.complete({
                "name": "wip_embedded_read",
                "when": () => ({ "shown": page.diffShown,
                                 "path": page.diffPath === embeddedTimer.path,
                                 "settled": diffPane.diffSettled() }),
                "report": () => "wip_embedded path=" + page.diffPath
                                + " embedded=" + diffPane.diffModel.embedded
                                + " rows=" + diffPane.view.count
                                + " sha8=" + diffPane.diffModel.embeddedSha8 })
        }
    }
    // The press has to land on rows that are on screen, so the diff is read first.
    PageStage {
        id: stalePartTimer
        name: "stale_part"
        order: ["pressed"]
        property string bucket: ""
        property string path: ""
        onTriggered: {
            if (!Awaited.all("stale_part_ready", { "shown": page.diffShown,
                                                   "loaded": !driver.diffPane.diffModel.loading }))
                return
            const pressed = stalePartTimer.input("pressed", () => {
                driver.pressWrite("stage-selection", () => {
                    repoTab.stageSelection(stalePartTimer.bucket, stalePartTimer.path, "", 0, -1, 1)
                    return true
                })
            })
            if (!pressed)
                return
            stalePartTimer.stop()
            driver.barrierNotice.start()
        }
    }
    // Another worktree's uncommitted row read in this window, with every writing control down
    // (P3-確認事項 §別の worktree の未コミット行). Every field is read off the output side — built from `writable`,
    // each would go green with the disabling unwired (app-ui.md §UI 自動化).
    PageStage {
        id: carriedReadTimer
        name: "carried_read"
        order: ["asked", "opened", "pointed", "walked"]
        /// The run's line, read once the step has landed. What the pane showed before the step, which opens another
        /// file — its lines, whether the first lit row is the file read, how many rows were lit — and whether the step
        /// moved are the walk's witnesses; the rest is the pane as it stands.
        function line() {
            const op = carriedReadTimer
            const files = page.wipUnstaged
            // The preset's worktree holds one path staged and written again, so `files=1` and a `tally=` of one
            // are the fold (per side both would be two). `cut=`/`tip=`: the band cuts the long name, its hover
            // carries it. `lines=` goes last, past what the table pins (`verify::outcome` matches a substring).
            // The tally is in the order the row draws it.
            const tally = graphModel.carriedTally(page.selectedRow)
            const counts = tally ? [tally.added, tally.modified, tally.deleted,
                                    tally.renamed, tally.copied, tally.conflicted] : []
            return "carried_read worktree=" + files.carriedName
                   + " files=" + files.total
                   + " tally=" + counts.join(",")
                   + " lit=" + op.witnessed("lit")
                   + " litRows=" + op.witnessed("litRows")
                   + " cut=" + driver.carriedPane.nameCut
                   + " tip=" + driver.carriedPane.nameTipShown
                   + " stepped=" + op.witnessed("moved")
                   + " stageFile=" + driver.diffPane.stageOffered
                   + " pieces=" + driver.diffPane.piecesOffered
                   + " lines=" + op.witnessed("lines")
        }
        onTriggered: {
            const op = carriedReadTimer
            if (!Awaited.all("carried_read_ready", { "ready": !(graphModel.finishCount === 0 || !worktree.loaded) }))
                return
            if (op.at("start")) {
                // Through the row itself, so the row's own decision is the one taken (verify-ui §壊れない動詞の実装と反復).
                op.input("asked", () => {
                    const row = driver.rowOfWorktree(Harness.autoActArg)
                    const item = row < 0 ? null : graphPane.view.itemAtIndex(row)
                    if (item === null)
                        return false
                    item.leftClick(Qt.NoModifier)
                    return true
                })
                return
            }
            // Until the worktree's own files arrive the lists hold this window's.
            const files = page.wipUnstaged
            if (!Awaited.all("carried_read_files", {
                    "files": !(page.carriedPath === "" || files.carriedAt !== page.carriedPath || files.total === 0) }))
                return
            if (op.at("asked")) {
                op.input("opened", () => {
                    // The pane's own numbering — one run, and a folder row in the tree view answers with no key at all.
                    let key = ""
                    for (let i = 0; key === "" && i < files.shownRows; i++)
                        key = files.fileKeyAt(i)
                    if (key === "")
                        return false
                    const cut = key.indexOf(":")
                    // Pressed on the row, so what is read is what the row makes of the model — handing the model's
                    // answer to `openDiff` would skip that (verify-ui §壊れない動詞の実装と反復). Null until the list
                    // builds the row.
                    const row = driver.carriedPane.filesWalk.rowFor(key.substring(0, cut), key.substring(cut + 1))
                    if (row === null)
                        return false
                    row.press()
                    return true
                })
                return
            }
            if (!Awaited.all("carried_read_diff", { "diff": driver.diffPane.diffSettled() }))
                return
            // A cut name is whole only in its hover tip, which waits out `tipDelayMs`; a name that fits has none.
            if (op.at("opened"))
                op.input("pointed", () => {
                    driver.carriedPane.namePointedAt = true
                })
            if (!Awaited.all("carried_read_tip", {
                    "tip": !(driver.carriedPane.nameCut && !driver.carriedPane.nameTipShown) }))
                return
            if (op.at("walked")) {
                // The run ends on the file the step asked for — a pane still reading it photographs empty, and its
                // rows are built a polish after the read (`PageSettled`). A step that moved nothing — a one-file
                // worktree steps and stays — has no second read to wait for.
                op.complete({
                    "name": "carried_read_landed",
                    "when": () => ({
                        "moved": !op.witnessed("moved")
                                 || (page.diffShown && page.diffPath !== op.witnessed("leftPath")),
                        "page": PageSettled.settled(page) }),
                    "report": () => op.line() })
                return
            }
            op.input("walked", () => {
                // Read before the step below, which opens another file.
                op.witness("lines", driver.diffPane.view.count)
                // `lit=` is only the first lit row (as `file_step` reads it), so a list lighting every row still
                // passes it; `litRows=` counts them (`FileRowWalk.litRows`).
                op.witness("lit", driver.carriedPane.filesWalk.litPath() === page.diffPath)
                op.witness("litRows", driver.carriedPane.filesWalk.litRows())
                // One arrow step through the pane's own walk. Rows are found by the side their bytes are on even in
                // this one list — handed the run's name for the bucket, the walk starts from nowhere and stays put.
                op.witness("leftPath", page.diffPath)
                op.witness("moved", driver.carriedPane.filesWalk.stepFile(1, false))
            })
        }
    }
    // The same pane at rest, nothing read: `litRows=` must be 0 here, which `carried-read` cannot show. The corner is
    // claimed here too — `corner` cannot reach another worktree's pane.
    PageStage {
        id: carriedStandTimer
        name: "carried_stand"
        order: ["asked"]
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
        /// `corner=` is the label's own visibility; the room it was handed would go green with the binding cut.
        function line() {
            return "carried_stand worktree=" + page.wipUnstaged.carriedName
                   + " files=" + page.wipUnstaged.total
                   + " litRows=" + driver.carriedPane.filesWalk.litRows()
                   + " reading=" + page.diffShown
                   + " corner=" + gitCorner.visible
                   + " folderPath=" + carriedStandTimer.folderPath()
                   + " room=" + Math.round(gitCorner.roomLeft)
        }
        onTriggered: {
            if (!Awaited.all("carried_stand_ready", { "ready": !(graphModel.finishCount === 0 || !worktree.loaded) }))
                return
            if (carriedStandTimer.at("start")) {
                // The same road in as `carried-read`.
                carriedStandTimer.input("asked", () => {
                    const row = driver.rowOfWorktree(Harness.autoActArg)
                    const item = row < 0 ? null : graphPane.view.itemAtIndex(row)
                    if (item === null)
                        return false
                    item.leftClick(Qt.NoModifier)
                    return true
                })
                return
            }
            // The worktree's own files arrived and their rows built: until then nothing is lit or bare.
            const files = page.wipUnstaged
            if (!Awaited.all("carried_stand_files", {
                    "files": !(page.carriedPath === "" || files.carriedAt !== page.carriedPath || files.total === 0) }))
                return
            // `folderPath=` and `litRows=` are read off the rows as built (`Awaited.built`).
            carriedStandTimer.complete({
                "name": "carried_stand_rows",
                "when": () => ({
                    "rows": driver.carriedPane.view.count > 0 && Awaited.built(driver.carriedPane.view),
                    "corner": gitCorner.width > 0 }),
                "report": () => carriedStandTimer.line() })
        }
    }
    // PGG_AUTO_ACT=notice-link: the notice bar's place word pressed, and the panel it names coming up marked
    // (デザイン規約 §hover のツールチップ「送った先は名乗る」) — as `fetch-tip-link` presses a tip's. Nothing ran, so
    // the panel is down before the press, and `open=` is the press's own doing.
    PageStage {
        id: noticeLinkTimer
        name: "notice_link"
        order: ["pressed"]
        onTriggered: {
            const op = noticeLinkTimer
            if (op.at("start")) {
                if (!Awaited.all("notice_link_bar", { "bar": page.noticeCard.settled }))
                    return
                // Whether the panel stood before the press — kept at the press, so `open=` is shown to be the press's
                // doing — and whether the place word took it, the press's own answer.
                const pressed = op.input("pressed", () => {
                    op.witness("openBefore", page.commandsOpen)
                    op.witness("took", page.noticeCard.pressDetailLink())
                })
                // A word that took no press ends the run on it.
                if (pressed && !op.witnessed("took"))
                    op.complete({ "when": () => ({}), "report": () => "notice_link pressed=false" })
                return
            }
            // The panel is built as it is raised (`RepoPage.commandsSeat`), so wait for it to exist.
            op.complete({
                "name": "notice_link_log",
                "when": () => ({ "log": page.commandsOpen, "pane": page.commandsPane !== null }),
                "report": () => "notice_link pressed=true before=" + op.witnessed("openBefore")
                                + " open=" + page.commandsShown
                                + " lit=" + page.commandsPane.attention
                                + " bar=" + page.noticeCard.open })
        }
    }
    // Another worktree's uncommitted row right-clicked through the row itself (`GraphRowDelegate.askMenu`), its menu
    // waited for up; the apply verbs then press the menu's row as a click does (`AppMenuItem.triggered`) and wait for
    // what that leaves — this tree read again, or the bar a refusal comes down in (`barrierNotice`).
    PageStage {
        id: carriedMenuTimer
        name: "carried_menu"
        order: ["up", "armed", "pressed"]
        property bool applying: false
        /// The six tallies a row draws, in the order it draws them (`GraphModel::carried_tally`).
        function tally(counts) {
            return counts ? [counts.added, counts.modified, counts.deleted,
                             counts.renamed, counts.copied, counts.conflicted].join(",") : ""
        }
        onTriggered: {
            const op = carriedMenuTimer
            if (!Awaited.all("carried_menu_graph", { "graph": !(graphModel.finishCount === 0 || !worktree.loaded) }))
                return
            if (!op.at("pressed")) {
                // Asked again every beat until it stands: the row can arrive, or be rebuilt, after the first ask.
                if (op.at("start") && !op.input("up", () => {
                        if (!driver.carriedMenu.visible) {
                            const row = driver.rowOfWorktree(Harness.autoActArg)
                            const item = row < 0 ? null : graphPane.view.itemAtIndex(row)
                            // Through the menu key's door (`GraphRowWalk.menuFromKeys`), which stands the menu at the
                            // row: a run has no pointer, and `popup()` would put it in the window's corner.
                            if (item !== null)
                                KeyMenu.ask(item.mapToItem(null, 0, Theme.graphRowHeight), () => item.askMenu())
                        }
                        return driver.carriedMenu.visible
                    }))
                    return
                if (!Awaited.all("carried_menu_up", { "row": driver.carriedMenu.visible,
                                                      "menu": driver.carriedMenu.opened }))
                    return
                if (!op.applying) {
                    // The pane behind the menu has its rows (`PageSettled`'s carried and rows terms). Read off the
                    // rows as drawn: `apply=false` is the row taken off, `held=true` greyed, `says=` why.
                    op.complete({
                        "name": "carried_menu_pane",
                        "when": () => ({ "page": PageSettled.settled(page) }),
                        "report": () => "carried_menu open=" + driver.carriedOpenItem.visible
                                        + " apply=" + driver.carriedApplyItem.visible
                                        + " held=" + driver.carriedApplyItem.blocked
                                        + " name=" + driver.carriedOpenItem.codeName
                                        + " says=" + driver.carriedApplyItem.blockedWhy })
                    return
                }
                if (op.at("up") && !op.input("armed", () => driver.beginWrite(Harness.autoAct)))
                    return
                // Pressed only where a hand could: the row there and not greyed (`AppMenuItem.blocked`).
                const pressed = op.input("pressed", () => {
                    const row = driver.carriedApplyItem
                    return driver.inputWent(row.visible && !row.blocked && (row.triggered(), true))
                })
                if (pressed && Harness.autoAct === "carried-apply-refused") {
                    op.stop()
                    driver.barrierNotice.start()
                }
                return
            }
            // `here=` is this tree's own row after the apply, `there=` the row the work came from — still drawn, with
            // what it had.
            op.complete({
                "name": "carried_menu_landing",
                "when": () => ({
                    "written": driver.wroteAndSettled(),
                    "notice": !(page.noticeCard.open && !page.noticeCard.settled) }),
                "report": () => "carried_apply notice=" + page.noticeCard.open
                                + " here=" + [graphPane.view.wipAdded, graphPane.view.wipModified,
                                              graphPane.view.wipDeleted, graphPane.view.wipRenamed,
                                              graphPane.view.wipCopied, graphPane.view.wipConflicted].join(",")
                                + " there=" + op.tally(
                                    graphModel.carriedTally(driver.rowOfWorktree(Harness.autoActArg)))
                                + " conflicts=" + worktree.conflictCount })
        }
    }
    PageStage {
        id: wipLanesTimer
        name: "wip_lanes"
        onTriggered: wipLanesTimer.complete({
            "name": "wip_lanes_ready",
            "when": () => ({ "ready": !(driver.graphTopKind() !== "wip") }),
            "report": () => "wip_lanes geometry=" + graphModel.geometryAt(0) + " lanes=" + graphModel.maxLanes })
    }
    // Waits for HEAD's author; until it arrives the offer to take it over is not in the picture.
    PageStage {
        id: amendAuthorTimer
        name: "amend_author"
        onTriggered: amendAuthorTimer.complete({
            "name": "amend_author_ready",
            "when": () => ({ "ready": !(repoTab.headAuthorName === "") }),
            "report": () => "amend_author differs=" + repoTab.headAuthorDiffers + " name=" + repoTab.headAuthorName })
    }
    readonly property QtObject halfWatch: QtObject {
        id: halfWatch
        readonly property bool duringOnly: Harness.autoAct === "wip-commit-half" && Harness.autoActArg === "during"
        /// The pair below and what the page held, latched at the edge — a later read gives the settled page's numbers.
        property bool caught: false
        property bool sawWipRow: false
        property int sawRows: -1
    }
    // The write out and nothing moved: busy up and HEAD unmoved, as busy alone outlasts the write's answer. Read on
    // the tab's notify (rules-refs/app-ui.md「一瞬だけ立つ状態は…」): the whole window can pass within one turn of
    // the loop, which a sampler beat would miss. Heard while `halfDuringTimer` runs: from before the press, which
    // witnessed HEAD as the write found it, to the completion, whose report reads the latch.
    Connections {
        target: acts.repoTab
        enabled: halfWatch.duringOnly && !halfWatch.caught && halfDuringTimer.running
        function onBusyCountChanged() {
            if (repoTab.busyCount === 0 || worktree.headOid !== driver.act.witnessed("headBefore"))
                return
            halfWatch.sawWipRow = graphModel.wipRow
            halfWatch.sawRows = unstagedModel.total
            halfWatch.caught = true
        }
    }
    // Completed a turn after the notify that carried the edge. The run ends either way: a commit that landed with no
    // notify raising busy had no `during` to stop at, and says so on a line outside what the verb is judged on.
    PageStage {
        id: halfDuringTimer
        name: "half_during"
        /// What the latch found; the page's numbers are the latched ones, not the settled page's.
        function line() {
            if (!halfWatch.caught)
                return "wip_half_missed the commit landed before any notify carried a raised busy, so this run had no "
                       + "moment with the write out and the picture still to stop at"
            return "wip_half stage=during busy=true moved=false"
                   + " wipRow=" + halfWatch.sawWipRow
                   + " rows=" + halfWatch.sawRows
        }
        onTriggered: halfDuringTimer.complete({
            "name": "half_during_ready",
            "when": () => ({
                "edge": halfWatch.caught || worktree.headOid !== driver.act.witnessed("headBefore") }),
            "report": () => halfDuringTimer.line() })
    }
    // The far side: the write settled, the graph holding the new HEAD, and the row drawn again above it.
    PageStage {
        id: halfSettledTimer
        name: "half_settled"
        onTriggered: halfSettledTimer.complete({
            "name": "half_settled_ready",
            "when": () => ({
                "ready": !(!driver.wroteAndSettled() || worktree.headOid === driver.act.witnessed("headBefore")
                    || graphModel.rowOf(worktree.headOid) < 0 || !graphModel.wipRow) }),
            "report": () => "wip_half stage=after busy=false"
                            + " moved=" + (worktree.headOid !== driver.act.witnessed("headBefore"))
                            + " wipRow=" + graphModel.wipRow
                            + " rows=" + unstagedModel.total })
    }
    // Waits for HEAD's author, before which there is no offer to tick.
    PageStage {
        id: resetAuthorTimer
        name: "reset_author"
        order: ["committed"]
        onTriggered: {
            if (!Awaited.all("reset_author_ready", { "ready": !(repoTab.headAuthorName === "") }))
                return
            // HEAD's author as the commit below finds it.
            resetAuthorTimer.say(() => "head_author differs="
                                        + repoTab.headAuthorDiffers
                                        + " name=" + repoTab.headAuthorName)
            const committed = resetAuthorTimer.input("committed", () => {
                wipPane.setResetAuthorChecked(true)
                wipPane.setMessage(Harness.autoActArg, "")
                // A deferred run: nothing raises its barrier, so the commit is sent and waited out here — off HEAD as
                // the commit finds it, read at the press itself (`resetAuthorLandedTimer`).
                resetAuthorTimer.witness("headBefore", worktree.headOid)
                driver.pressWrite("commit", () => {
                    page.commitNow()
                    return true
                })
            })
            if (!committed)
                return
            resetAuthorTimer.stop()
            resetAuthorLandedTimer.start()
        }
    }
    // Past the write barrier: the amend answers before the rebuild, so the barrier frames the replaced commit. The
    // edge is the graph holding the new HEAD — not the old one going, which a stash on top keeps drawn. Where the
    // selection lands differs by repository, so the pane is waited for and `shown=` says whose author is pictured.
    PageStage {
        id: resetAuthorLandedTimer
        name: "reset_author_landed"
        onTriggered: resetAuthorLandedTimer.complete({
            "name": "reset_author_landed_ready",
            "when": () => ({
                "ready": !(!driver.wroteAndSettled() || worktree.headOid === resetAuthorTimer.witnessed("headBefore")
                    || graphModel.rowOf(worktree.headOid) < 0 || !driver.cardSettled) }),
            "report": () => "reset_author was=" + resetAuthorTimer.witnessed("headBefore").substring(0, 8)
                            + " head=" + worktree.headOid.substring(0, 8)
                            + " shown=" + detailsModel.shaHex.substring(0, 8)
                            + " author=" + detailsModel.authorName
                            + " committer=" + detailsModel.committerName })
    }
    // What stands beside the exit card (デザイン規約 §進行中の操作から出る): a stopped rebase, pick or revert hides the
    // message boxes and the commit button, a stopped merge keeps them — its `--continue` is that button. Read off the
    // drawn items (an item's `visible` is false under a hidden parent), once the card stands on the worktree face.
    PageStage {
        id: opExitTimer
        name: "op_exit"
        onTriggered: opExitTimer.complete({
            "name": "op_exit_ready",
            "when": () => ({ "ready": !(!page.wipShown || !wipPane.offersOpExit("--abort")) }),
            "report": () => "op_exit card=" + wipPane.offersOpExit("--abort")
                            + " box=" + wipPane.commitBlock.messageSeat.visible
                            + " button=" + wipPane.commitBlock.commitSeat.visible
                            + " op=" + worktree.opText })
    }
    // Staging has to land before the button can know what it carries.
    PageStage {
        id: eolCommitTimer
        name: "eol_commit"
        // Only the card is waited for — asking after the count first would be reading the input side.
        onTriggered: eolCommitTimer.complete({
            "name": "eol_commit_ready",
            "when": () => ({ "ready": wipPane.eolCardOpen }),
            "report": () => "eol_commit staged=" + worktree.stagedCount
                            + " warned=" + worktree.eolStagedCount
                            + " card=" + wipPane.eolCardOpen })
    }
    // The row is read once the status has landed and HEAD is known (the row stands only over a commit), and — after a
    // merge this verb pressed — once that merge's stop has reached the pane: the tick has to have met it.
    PageStage {
        id: amendRowTimer
        name: "amend_row"
        /// The branch this run merged to stop on, `""` for a run that reads the tree as it opened.
        property string merged: ""
        onTriggered: amendRowTimer.complete({
            "name": "amend_row_ready",
            "when": () => ({ "ready": !(!worktree.loaded || !worktree.headKnown),
                             "page": PageSettled.settled(page),
                             "merged": !(amendRowTimer.merged !== "" && !worktree.opMerging) }),
            "report": () => "amend_row shown=" + wipPane.commitBlock.amendRowShown
                            + " barred=" + wipPane.commitBlock.amendBarred
                            + " op=" + worktree.opText
                            + " ticked=" + wipPane.commitBlock.amendChecked
                            + " amending=" + page.amending })
    }
    // Two things land apart: HEAD's message into the box and git's answer about the amend. The pointer goes on once
    // the message is in, and the tip comes up only on a button that refuses — git's answer is what greys it, and the
    // run waits on the tip, never on the answer it reports (`empties=` is read, not waited for). The tip's own words
    // are read too: a refusal for anything else names that instead.
    PageStage {
        id: amendTipTimer
        name: "amend_tip"
        onTriggered: {
            if (!Awaited.all("amend_tip_ready", { "ready": !(wipPane.subjectText === "") }))
                return
            if (amendTipTimer.at("start"))
                amendTipTimer.input("pointed", () => {
                    wipPane.pointAtCommit = true
                })
            amendTipTimer.complete({
                "name": "amend_tip_shown",
                "when": () => ({ "tip": Tips.box.visible }),
                "report": () => "amend_tip empties=" + repoTab.amendEmpties
                                + " enabled=" + wipPane.commitBlock.commitEnabled
                                + " tip=" + Tips.box.visible
                                + " why=" + Tips.box.text })
        }
    }
    // The tick's tip comes out on the shared delay, so the setting being on is not yet the tip being up.
    PageStage {
        id: commitFaceTimer
        name: "commit_face"
        onTriggered: commitFaceTimer.complete({
            "name": "commit_face_ready",
            "when": () => ({ "ready": wipPane.signingTipShown }),
            "report": () => "commit_face signs=" + repoTab.signsCommits + " tip=" + wipPane.signingTipShown })
    }
    // The marks arrive with the status read, so the row named for its sentence is named again, each beat, until its
    // card is up.
    PageStage {
        id: eolHoverTimer
        name: "eol_hover"
        onTriggered: {
            if (eolHoverTimer.at("start") && !eolHoverTimer.input("pointed", () => {
                    wipPane.pointEol(Harness.autoActArg)
                    return wipPane.eolCardOpen
                }))
                return
            eolHoverTimer.complete({
                "name": "eol_hover_card",
                "when": () => ({ "card": wipPane.eolCardOpen }),
                "report": () => "eol_hover bucket=" + wipPane.pointedEolBucket
                                + " path=" + wipPane.pointedEolPath
                                + " card=" + wipPane.eolCardOpen
                                + " text=" + wipPane.pointedEolText })
        }
    }
    // PGG_AUTO_ACT=eol-gone. The card is asked for as the hover asks (the row's, or the button's once something is
    // staged), the hand is put in it as the card reads a hand (`AppCard.contentPointed`), and the warning is taken
    // away with one write: the card goes, held hand or not (`WipPane.weighEol`).
    PageStage {
        id: eolGoneTimer
        name: "eol_gone"
        order: ["pointed", "away", "lifted"]
        property bool button: false
        /// Stop with the card out and the hand in it, before the write that takes the warning away.
        property bool inside: false
        /// Everything staged (`stage-all`) or unstaged (`unstage-all`), through the driver's door for a write.
        function pressAll(what) {
            return driver.pressWrite(what, () => {
                if (what === "stage-all")
                    repoTab.stageAll()
                else
                    repoTab.unstageAll()
                return true
            })
        }
        /// The card out on a settled page — and, for the button's, the staging before it waited out before the next
        /// write is armed.
        function cardOut() {
            return {
                "card": wipPane.eolCardOpen,
                "wrote": !eolGoneTimer.button || driver.wroteAndSettled(),
                "page": PageSettled.settled(page)
            }
        }
        onTriggered: {
            const op = eolGoneTimer
            if (op.at("start")) {
                if (op.button) {
                    // The button's form stages everything first, pressed until it takes. The button warns only while
                    // it can be pressed (`WipCommitBlock.eolWarned`): a message first, then the pointer on it.
                    op.input("staged", () => {
                        if (!op.pressAll("stage-all"))
                            return false
                        wipPane.setMessage("feat: something", "")
                        wipPane.pointAtCommit = true
                        return true
                    })
                    return
                }
                // The row's form starts on the rows as they are: its card asked for each beat until it is up.
                if (!op.input("pointed", () => {
                        wipPane.pointEol("src/flipped.kt")
                        return wipPane.eolCardOpen
                    }))
                    return
            }
            if (op.at("staged") || op.at("pointed")) {
                if (!Awaited.all("eol_gone_card", op.cardOut()))
                    return
                if (!op.inside) {
                    const away = op.button ? "unstage-all" : "stage-all"
                    // The hand into the card, then the write: pressed until it takes.
                    op.input("away", () => {
                        wipPane.eolHoverCard.contentPointed = true
                        return op.pressAll(away)
                    })
                    return
                }
                op.input("held", () => {
                    wipPane.eolHoverCard.contentPointed = true
                })
            }
            if (op.at("held")) {
                op.complete({
                    "name": "eol_gone_card",
                    "when": () => op.cardOut(),
                    "report": () => "eol_gone button=" + op.button + " stage=inside card="
                                    + wipPane.eolCardOpen + " warned=" + worktree.eolStagedCount })
                return
            }
            if (op.at("away")) {
                if (!Awaited.all("eol_gone", { "gone": !wipPane.eolCardOpen, "page": PageSettled.settled(page) }))
                    return
                // The hand off the button and the card's place before the picture: a hand resting on the commit
                // button asks for its tip after the usual rest, and a tip arriving between the completion and a
                // picture is a window that moved. Lifted, the ask is withdrawn (`TipAsk.asked`), and `Tips.current`
                // is null once no tip is shown or waiting.
                op.input("lifted", () => {
                    wipPane.pointAtCommit = false
                    wipPane.eolHoverCard.contentPointed = false
                })
            }
            op.complete({
                "name": "eol_gone_hand",
                "when": () => ({ "gone": !wipPane.eolCardOpen, "page": PageSettled.settled(page),
                                 "tip": Tips.current === null }),
                "report": () => "eol_gone button=" + op.button + " card=" + wipPane.eolCardOpen
                                + " gone=" + !wipPane.eolCardOpen + " warned=" + worktree.eolStagedCount })
        }
    }
    // PGG_AUTO_ACT=eol-walk / eol-stay. The hand on the row's words as the pane hears it (`HandWatch.stir`), the card
    // out, then the unstaged list sent down under the still hand: the card goes and the hand is held. A whole row
    // (`eol-walk`): the light goes to the row that arrived under the hand at once, and its own card comes after a
    // tremor and the rest (デザイン規約 §左メニューの所作「手が動いたかの見切り」). A
    // pixel (`eol-stay`): the row the hand stayed on keeps its light, and the tremor brings its card back after the
    // rest (`HoverSeat.displace`). The report reads the state at the stage it stops at — `gone=` the card down,
    // `slid_lit=` the first row's light, `under_lit=` the light of the row under the hand, `back=` the card up — but
    // for the two read in the tremor's own turn: `tremor=` that row's light, `tremor_card=` the card put out
    // (`visible`, up from the open call on, where `opened` waits out the enter transition). No stage stops at the
    // tremor: the rest it starts runs on under the picture and the census, so what they show would be the machine's
    // speed.
    PageStage {
        id: eolWalkTimer
        name: "eol_walk"
        order: ["pointed", "scrolled", "tremor"]
        property bool stay: false
        /// The stage the run stops at: `gone` (the card down) or `back` (the card up again).
        property string stopAt: "back"
        function unstagedList() {
            const pane = wipPane.bucketPanes.find(pane => pane.section === "unstaged")
            return pane === undefined ? null : pane.list
        }
        /// The row of `src/flipped.kt` in the unstaged list — the one the hand comes onto and the list slides away.
        function slidRow() {
            return wipPane.rowIn("unstaged", "src/flipped.kt")
        }
        /// The row under the hand after the scroll — the one that arrived, or the one the hand stayed on — where the
        /// hand rests (`handAt`, witnessed in the window's frame as the hand came onto the row).
        function rowUnderHand() {
            const list = eolWalkTimer.unstagedList()
            if (list === null)
                return null
            const hand = eolWalkTimer.witnessed("handAt")
            const p = list.mapFromItem(null, hand.x, hand.y)
            return list.itemAt(p.x, p.y + list.contentY)
        }
        /// The run's line at the stage it stopped at, read off the rows as they stand; `tremor=` / `tremor_card=` are
        /// the tremor's witnesses, false for a run stopped before it.
        function line(stopped) {
            const op = eolWalkTimer
            const row = op.slidRow()
            const under = op.rowUnderHand()
            return (op.stay ? "eol_stay" : "eol_walk") + " stage=" + stopped
                   + " gone=" + !wipPane.eolCardOpen + " slid_lit=" + (row !== null && row.washLit)
                   + " under_lit=" + (under !== null && under.washLit)
                   + " tremor=" + (stopped !== "gone" && op.witnessed("tremorLit"))
                   + " tremor_card=" + (stopped !== "gone" && op.witnessed("tremorCard"))
                   + " back=" + wipPane.eolCardOpen
                   + " path=" + wipPane.pointedEolPath
        }
        onTriggered: {
            const op = eolWalkTimer
            // Asked again until the card is up: the marks arrive with the status read (`WipPane.pointEol`).
            if (op.at("start") && !op.input("pointed", () => {
                    wipPane.pointEol("src/flipped.kt")
                    return wipPane.eolCardOpen
                }))
                return
            if (op.at("pointed")) {
                const row = op.slidRow()
                if (!Awaited.all("eol_walk_card", { "card": wipPane.eolCardOpen, "row": row !== null }))
                    return
                op.input("scrolled", () => {
                    wipPane.hand.stir(op.witness("handAt", row.mapToItem(null, row.width / 2, row.height / 2)))
                    op.unstagedList().contentY += op.stay ? 1 : Theme.rowHeight
                })
                return
            }
            if (op.at("scrolled")) {
                if (op.stopAt === "gone") {
                    op.complete({ "name": "eol_walk_gone", "when": () => ({ "gone": !wipPane.eolCardOpen }),
                                  "report": () => op.line("gone") })
                    return
                }
                if (!Awaited.all("eol_walk_gone", { "gone": !wipPane.eolCardOpen }))
                    return
                op.input("tremor", () => {
                    const under = op.rowUnderHand()
                    const hand = op.witnessed("handAt")
                    wipPane.hand.stir(Qt.point(hand.x + 2, hand.y))
                    op.witness("tremorLit", under !== null && under.washLit)
                    op.witness("tremorCard", wipPane.eolHoverCard.visible)
                })
                return
            }
            // The row under the hand asks for its card — its own, or the one kept through the hold: it comes after
            // the rest the tremor started.
            op.complete({ "name": "eol_walk_back", "when": () => ({ "card": wipPane.eolCardOpen }),
                          "report": () => op.line("back") })
        }
    }
    // The band's Stash button, pressed until it takes: it is down while the opening fetch keeps the tab busy. The row
    // that has to go is read here, once the worktree row leads — it can arrive after the graph's first pass (a
    // stopped merge), and read earlier the verb would wait on the newest commit.
    PageStage {
        id: stashPressTimer
        name: "stash_press"
        order: ["pressed"]
        /// `stash-lands`: pressed from the worktree row with its pane open, taken here so the row is there.
        property bool fromWip: false
        onTriggered: {
            if (!Awaited.all("stash_press_ready", { "wip": driver.graphTopKind() === "wip",
                                                    "band": page.pageBand !== null }))
                return
            const pressed = stashPressTimer.input("pressed", () => {
                const going = graphModel.oidAt(0)
                if (stashPressTimer.fromWip) {
                    graphPane.setCurrentRow(0)
                    page.showWip()
                }
                if (!driver.inputWent(page.pageBand.stashNow()))
                    return false
                driver.awaitGraphGone(going)
                return true
            })
            if (pressed)
                stashPressTimer.stop()
        }
    }
    // Two waits: the write lands, then HEAD's own message comes back — only the new commit can say the empty box
    // committed the merge's words.
    PageStage {
        id: mergeCommitTimer
        name: "merge_commit"
        order: ["asked"]
        property string wanted: ""
        property bool typed: false
        property string headWas: ""
        onTriggered: {
            const op = mergeCommitTimer
            // The graph holding the commit, checked against the id HEAD moved from: refs and the walk arrive behind
            // the write and each other, so the old tip would pass.
            if (!Awaited.all("merge_commit_ready", { "ready": driver.wroteAndSettled(),
                                                     "page": PageSettled.settled(page),
                                                     "head": !(!worktree.headKnown || worktree.headOid === op.headWas
                                                               || graphModel.rowOf(worktree.headOid) < 0
                                                               || driver.graphTopKind() === "wip") }))
                return
            if (op.at("start")) {
                op.input("asked", () => {
                    // HEAD's commit answer count as its message is asked for again.
                    op.witness("seenHead", repoTab.headCommitSeq)
                    repoTab.requestHeadCommit()
                })
                return
            }
            op.complete({
                "name": "merge_commit_answered",
                "when": () => ({ "answered": repoTab.headCommitSeq !== op.witnessed("seenHead") }),
                "report": () => driver.verbReport().concat([
                    "merge_committed merging=" + (worktree.opText !== "")
                    + " kept=" + (op.wanted !== "" && repoTab.headSubject === op.wanted)
                    + " typed=" + op.typed
                    + " head=" + repoTab.headSubject]) })
        }
    }
}
