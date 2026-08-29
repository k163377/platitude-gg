pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The working tree's own verbs: what a commit is made of and what it is called — the message boxes, the amend
/// row, stashing, the line-ending card, and the seat a stopped merge is finished from.
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
    readonly property var graphModel: driver.graphModel
    readonly property var detailsModel: driver.detailsModel
    readonly property var branchesModel: driver.branchesModel
    readonly property var worktreeModel: driver.worktreeModel
    readonly property var stashesModel: driver.stashesModel
    readonly property var graphPane: driver.graphPane
    readonly property var wipPane: driver.wipPane
    readonly property var fileRowMenu: driver.fileRowMenu
    readonly property var commitMenuState: driver.commitMenuState
    readonly property var stashDeleteItem: driver.stashDeleteItem

    /// Runs `act` if it is one of this family's, and says whether it was. The families are asked in turn
    /// and the first to know a verb runs it — no verb is named by two of them (`AutoActDriver`).
    function run(act, arg) {
        if (act === "commit") {
            // A message of its own when none was named: git refuses an empty one outright, and a run that asked for a
            // commit and got a refusal is a picture of the history it did not write. Amend is the one below and needs
            // no such fallback — there an empty message means "keep HEAD's" (`--no-edit`).
            repoTab.stageAll()
            wipPane.setMessage(arg === "" ? "chore: commit from the headless run" : arg, "")
            page.commitNow()
        } else if (act === "commit-refused" || act === "notice-over-diff") {
            // The same press against a repository whose `pre-commit` hook says no (`--preset hooked`). Nothing is
            // half written and `--no-verify` is never passed, so the hook's own words come down as a report
            // (デザイン規約 §答えの要らない報せ).
            //
            // `notice-over-diff` runs it **with a file open in the middle**, which is the one arrangement that tells
            // a bar standing over the page apart from one living inside the graph: the second says nothing at all
            // here, and the picture of a window whose middle is a diff is the same either way.
            repoTab.stageAll()
            wipPane.setMessage("chore: something the hook will not have", "")
            if (act === "notice-over-diff")
                page.openDiff("staged", arg, "")
            page.commitNow()
            driver.barrierNotice.start()
        } else if (act === "stale-part") {
            // A part of a file that is not the file any more: the same press the diff's own marks make, carrying a
            // fingerprint the bytes cannot match. Nothing is written and the pane reads the file again, so what comes
            // back is a report (デザイン規約 §答えの要らない報せ). The argument is `<バケツ>:<パス>`.
            const cut = arg.indexOf(":")
            stalePartTimer.bucket = arg.substring(0, cut)
            stalePartTimer.path = arg.substring(cut + 1)
            page.openDiff(stalePartTimer.bucket, stalePartTimer.path, "")
            stalePartTimer.start()
        } else if (act === "report-tone") {
            // The dress a kind of report wears, entered through the page's own door (`showReport`) so the run reads
            // the same `Words` the answer does. **Only the kind is handed in** — which report arrived is what the
            // other verbs prove.
            page.showReport(arg, "origin", "main", "")
            driver.barrierNotice.start()
        } else if (act === "amend") {
            // The message is supplied, so skip the prefill request that would otherwise land on top of it.
            wipPane.setAmendChecked(true)
            page.amending = true
            wipPane.setMessage(arg, "")
            page.commitNow()
        } else if (act === "amend-reset-author") {
            // Whether authorship is HEAD's to take over is only known once HEAD has been read, so this one goes the
            // long way round: turn amend on and wait for the answer.
            wipPane.setAmendChecked(true)
            page.amendToggled(true)
            resetAuthorTimer.start()
        } else if (act === "stash" || act === "stash-lands") {
            // Through the band's button, which is the whole of it: nothing is asked before the write. The WIP pane is
            // left where it is — the button stands on the window's band now, so a run that opened that pane first
            // would be proving the reach of a pane the button no longer needs (デザイン規約 §変更を退避する).
            //
            // **`stash-lands` is the one that opens it**, because where the reader is standing is its whole subject:
            // the press empties the tree the pane is describing, and the row it is standing on leaves the graph with
            // the highlight still on it. Pressed from the same button all the same.
            //
            // Everything goes, so the working-tree row goes with it and the new stash takes the lead — the row whose
            // absence says the rebuild has landed. The one-path verb leaves the row where it is and keeps the plain
            // write barrier.
            //
            // `named` leaves a summary in the commit box first, which the entry is then called after (デザイン規約
            // §変更を退避する). The words are the driver's own, the way `wip-message` supplies its own line: a summary
            // written for a commit has spaces and a colon in it, and the argument cannot carry either (`check --verb`
            // splits its line on whitespace). The pane still is not opened — the button does not need it, and the name
            // is read back off the sidebar rather than off the box.
            if (arg === "named") {
                driver.stashWanted = "feat: write the summary"
                wipPane.setMessage(driver.stashWanted, "")
            }
            // The press is ticks away; the barrier must not pass on a
            // fetch that answered in between (`expectWriteAtPress`).
            driver.expectWriteAtPress()
            stashPressTimer.fromWip = act === "stash-lands"
            stashPressTimer.start()
        } else if (act === "stash-file") {
            wipPane.chooseOnly("unstaged", arg)
            page.openFileMenu("unstaged", arg)
            fileRowMenu.sendPaths([arg])
            repoTab.stashPaths(wipPane.stashName)
        } else if (act === "amend-author") {
            // The boxes live in the commit editor, which is only on screen while the uncommitted row is the selected
            // one — without this the run photographs the details pane and says nothing about the amend row.
            page.showWip()
            wipPane.setAmendChecked(true)
            page.amendToggled(true)
            amendAuthorTimer.start()
        } else if (act === "stash-menu" || act === "delete-stash-row") {
            // The row menu on the first stash's row; the argument "go" holds the delete row down.
            const menuOid = stashesModel.oidOfName(stashesModel.nameAt(0))
            page.openRowMenu(menuOid)
            AppBackend.report("row_menu stash=" + commitMenuState.menuStashRef)
            if (act === "delete-stash-row" && arg === "go") {
                // The drop takes this row off the graph — the same second
                // barrier stash-pop-row waits on, for the same reason.
                driver.graphGoneOid = menuOid
                stashDeleteItem.completeHold()
            }
        } else if (act === "stash-apply-row" || act === "stash-pop-row") {
            const stashOid = stashesModel.oidOfName(stashesModel.nameAt(0))
            page.openRowMenu(stashOid)
            if (act === "stash-apply-row") {
                repoTab.applyStash(commitMenuState.menuStashRef)
            } else {
                // A pop drops the entry, so its row leaves the graph — the edge that says the rebuild has landed.
                // An apply keeps it, and keeps the plain write barrier.
                driver.graphGoneOid = stashOid
                // Through the page, which is where the entry's name is read before it goes (`popStash`) — the menu's
                // own road. What the box has to be holding afterwards is read back at the barrier (`back=`).
                driver.popWanted = GitFacts.stashLabel(stashesModel.nameAt(0))
                page.popStash(commitMenuState.menuStashRef)
            }
        } else if (act === "wip") {
            page.showWip()
        } else if (act === "wip-lanes") {
            wipLanesTimer.start()
        } else if (act === "wip-tally") {
            // Read the two together: `status::Kinds` counts rows, so the kinds have to add up to `rows` — a drift means
            // one of the two stopped reading the same status.
            page.showWip()
            AppBackend.report("wip_tally added=" + graphPane.view.wipAdded
                              + " modified=" + graphPane.view.wipModified
                              + " deleted=" + graphPane.view.wipDeleted
                              + " renamed=" + graphPane.view.wipRenamed
                              + " copied=" + graphPane.view.wipCopied
                              + " conflicted=" + graphPane.view.wipConflicted
                              + " rows=" + worktreeModel.total)
        } else if (act === "wip-message" || act === "wip-message-focus") {
            // A body is typed first because this editor starts empty, and an empty box has no text to take a colour.
            // Read as a pair: the caret is the only difference between the two verbs.
            page.showWip()
            wipPane.setMessage("feat: write the summary",
                               arg === "" ? "And the description under it." : arg)
            if (act === "wip-message-focus")
                wipPane.focusDescription()
            AppBackend.report("message_focus pane=wip focused="
                              + wipPane.descriptionFocused
                              + " color=" + wipPane.descriptionColor)
        } else if (act === "merge-commit") {
            // The box opens holding the merge's own message; this empties it first, because the state worth proving
            // is the one where nothing is typed and the press still records those words
            // (デザイン規約 §進行中の操作から出る).
            page.showWip()
            wipPane.clearMessage()
            mergeCommitTimer.begin()
            AppBackend.report("merge_commit pressed=" + wipPane.pressCommit())
        } else if (act === "op-exit" || act === "op-exit-go") {
            // "-go" runs the held row the argument names to its end.
            page.showWip()
            if (act === "op-exit-go")
                AppBackend.report("op_exit_held " + wipPane.completeOpExit(arg))
        } else if (act === "eol-commit") {
            // Nothing is committed — the shot is the state before anyone decides.
            page.showWip()
            repoTab.stageAll()
            // `amend` asks for the other wording rather than for a message: the two forms of this button differ in what
            // they say, and both have to be photographable.
            if (arg === "amend") {
                wipPane.setAmendChecked(true)
                page.amending = true
            }
            wipPane.setMessage(arg === "" || arg === "amend" ? "feat: something" : arg, "")
            // **The pointer goes on before the tree has settled**, which is what a real one does — the hand reaches the
            // button while the index is still being written. Waiting for the warning first and pointing after would
            // photograph the same card while leaving the ordering that actually broke it untested.
            wipPane.pointAtCommit = true
            eolCommitTimer.start()
        } else if (act === "commit-face") {
            // The signing tick on the face at the end of the commit button, with its one line out. Hover cannot be
            // injected, so this writes the one property a real pointer writes (`AvatarButton`).
            page.showWip()
            wipPane.setMessage("feat: something", "")
            wipPane.signingPointedAt = true
            commitFaceTimer.start()
        } else if (act === "eol-hover") {
            // Hover cannot be injected; this writes the one property a real pointer writes. The tip lands in
            // overlay.png.
            page.showWip()
            wipPane.pointEol(arg)
            eolHoverTimer.start()
        } else {
            return false
        }
        return true
    }
    // The lanes of the uncommitted row, in the tokens its delegate paints from. A lane is a stroke a couple of pixels
    // wide and its dashes are one pixel each, so which of them are dotted is not a question the photograph answers.
    // The row is put there by the pass behind the status read — the same read that says what a standing merge is
    // bringing in — so the wait is for the row itself to lead the graph, not for the tree to be loaded.
    // The press has to land on rows that are actually on screen, so the diff has to have been read first — and the
    // fingerprint handed in is one the bytes cannot have, which is the whole of the arrangement.
    SampleTimer {
        id: stalePartTimer
        property string bucket: ""
        property string path: ""
        onTriggered: {
            if (!page.diffShown || driver.diffPane.diffModel.loading)
                return
            stalePartTimer.stop()
            driver.writeSeqBefore = repoTab.writeSeq
            repoTab.stageSelection(stalePartTimer.bucket, stalePartTimer.path, "", 0, -1, 1)
            driver.barrierNotice.start()
        }
    }
    SampleTimer {
        id: wipLanesTimer
        onTriggered: {
            if (driver.graphTopKind() !== "wip")
                return
            wipLanesTimer.stop()
            AppBackend.report("wip_lanes geometry=" + graphModel.geometryAt(0)
                              + " lanes=" + graphModel.maxLanes)
            driver.complete()
        }
    }
    // The offer to take HEAD's authorship over is only in the picture once HEAD's author has arrived, so the shot
    // waits for it rather than for a stretch of time.
    SampleTimer {
        id: amendAuthorTimer
        onTriggered: {
            if (repoTab.headAuthorName === "")
                return
            amendAuthorTimer.stop()
            AppBackend.report("amend_author differs=" + repoTab.headAuthorDiffers
                              + " name=" + repoTab.headAuthorName)
            driver.complete()
        }
    }
    // HEAD's author has to arrive before the offer to take it over can be there to tick.
    SampleTimer {
        id: resetAuthorTimer
        onTriggered: {
            if (repoTab.headAuthorName === "")
                return
            resetAuthorTimer.stop()
            AppBackend.report("head_author differs="
                              + repoTab.headAuthorDiffers
                              + " name=" + repoTab.headAuthorName)
            wipPane.setResetAuthorChecked(true)
            wipPane.setMessage(AppBackend.autoActArg, "")
            // The run is deferred, so nothing raises a barrier for it on the way out: the commit is sent from here and
            // waited out from here. Both marks are re-read on the spot rather than carried over from
            // `prepareCompletion` — the page's own opening fetch can have answered in between.
            driver.writeSeqBefore = repoTab.writeSeq
            driver.headOidBefore = branchesModel.headOid
            page.commitNow()
            resetAuthorLandedTimer.start()
        }
    }
    // The write barrier's two edges, and then the one this run is actually about: the amend answers before the rebuild
    // it asks for is started, so stopping at the write frames the commit that was replaced — still under the name the
    // amend was sent to take over (observed: `--preset authorship` photographed "Yuki Tanaka" on a green run).
    //
    // Refs answer ahead of the rebuild, so the new tip being named is not the graph holding it — the graph taking the
    // commit in is the edge, and it is read the positive way round. The replaced one going is not the same statement
    // and is not always true: a stash made on top of it keeps it drawn as its own parent, so `--preset basic` comes
    // back from an amend one row *longer* than it went in, with the commit that was amended still on screen.
    //
    // That is also why the pane is waited for by the selection rather than by the tip. Where the page puts the reader
    // afterwards is its own business and it differs by repository — onto the new tip where the amended row led, back
    // onto the same commit where the graph still holds it — but either way the shot must not be taken while the pane
    // is still fetching, and `shown=` is then what says whose name the author line in the picture is.
    SampleTimer {
        id: resetAuthorLandedTimer
        onTriggered: {
            if (repoTab.busyCount !== 0 || repoTab.writeSeq <= driver.writeSeqBefore
                    || branchesModel.headOid === driver.headOidBefore
                    || graphModel.rowOf(branchesModel.headOid) < 0
                    || !driver.cardSettled)
                return
            resetAuthorLandedTimer.stop()
            AppBackend.report("reset_author was=" + driver.headOidBefore.substring(0, 8)
                              + " head=" + branchesModel.headOid.substring(0, 8)
                              + " shown=" + detailsModel.shaHex.substring(0, 8)
                              + " author=" + detailsModel.authorName
                              + " committer=" + detailsModel.committerName)
            driver.complete()
        }
    }
    // Staging has to land before the button can know what it carries.
    SampleTimer {
        id: eolCommitTimer
        onTriggered: {
            // Only the card is waited for. The pointer is already on the button, so the tree settling is what is being
            // watched, and the card coming out is that — asking after the count first would be reading the input side.
            if (!wipPane.eolCardOpen)
                return
            eolCommitTimer.stop()
            AppBackend.report("eol_commit staged=" + workTree.stagedCount
                              + " warned=" + workTree.eolStagedCount
                              + " card=" + wipPane.eolCardOpen)
            driver.complete()
        }
    }
    // The tick's line comes out on the shared delay, so the setting being on is not yet the line being up.
    SampleTimer {
        id: commitFaceTimer
        onTriggered: {
            if (!wipPane.signingTipShown)
                return
            commitFaceTimer.stop()
            AppBackend.report("commit_face signs=" + repoTab.signsCommits
                              + " tip=" + wipPane.signingTipShown)
            driver.complete()
        }
    }
    // The marks arrive with the status read, so the row named for its sentence has to be named again once they are in.
    SampleTimer {
        id: eolHoverTimer
        onTriggered: {
            wipPane.pointEol(AppBackend.autoActArg)
            if (!wipPane.eolCardOpen)
                return
            eolHoverTimer.stop()
            AppBackend.report("eol_hover path=" + wipPane.pointedEolPath
                              + " card=" + wipPane.eolCardOpen
                              + " text=" + wipPane.pointedEolText)
            driver.complete()
        }
    }
    // The band's Stash button, offered until it takes. The button is down while the tab is busy, and the fetch a
    // repository does on the way open outlives the baseline the verb starts on — so a single shot lands on nothing and
    // the run waits out its watchdog on a graph nobody asked to change. What follows the press is the graph barrier
    // (`graphGoneOid`), which is why nothing else is read here.
    //
    // **The row that has to go is read here rather than at the dispatch.** The walk prepends the working tree's row
    // only once the status says the tree is stacked on HEAD, and that status can arrive after the graph's first pass —
    // a repository opened onto a stopped merge is the case where it does. Read too early, the verb waits out its
    // watchdog on the newest *commit*, which was never going anywhere (measured, `--preset conflict-staged`).
    SampleTimer {
        id: stashPressTimer
        /// Whether the press is made from the working tree's own row with the pane that describes it open — the seat
        /// `stash-lands` is about, taken here rather than at the dispatch so the row is there to sit on.
        property bool fromWip: false
        onTriggered: {
            if (driver.graphTopKind() !== "wip" || page.pageBand === null)
                return
            const going = graphModel.oidAt(0)
            if (stashPressTimer.fromWip) {
                graphPane.setCurrentRow(0)
                page.showWip()
            }
            if (!page.pageBand.stashNow())
                return
            driver.pressedWrite()
            driver.graphGoneOid = going
            stashPressTimer.stop()
        }
    }
    // The merge finished from the button under the exit card, with nothing typed in the box. Two waits in one timer:
    // the write has to land, and then HEAD's own message has to come back — the claim is that the empty box committed
    // the merge's words, and only the commit that now exists can say so.
    SampleTimer {
        id: mergeCommitTimer
        property string wanted: ""
        property bool typed: false
        property int seenHead: -1
        property string headWas: ""
        function begin() {
            mergeCommitTimer.wanted = workTree.opSubject
            mergeCommitTimer.headWas = branchesModel.headOid
            // Read now rather than at the report: the editor is cleared by the landing, so afterwards every run says
            // the boxes were empty.
            mergeCommitTimer.typed = wipPane.subjectText !== "" || wipPane.bodyText !== ""
            mergeCommitTimer.seenHead = -1
            mergeCommitTimer.start()
        }
        onTriggered: {
            if (repoTab.busyCount !== 0 || repoTab.writeSeq <= driver.writeSeqBefore)
                return
            // The commit exists; the picture is of the graph holding it. **Against the id it moved from**, not
            // merely "HEAD is somewhere in the graph": refs and the walk arrive behind the write and behind each
            // other, so the old tip answers that question perfectly well, and the shot came back framing the branch
            // still on it with a working-tree row above (measured).
            if (!branchesModel.refsLoaded || branchesModel.headOid === mergeCommitTimer.headWas
                    || graphModel.rowOf(branchesModel.headOid) < 0
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
            AppBackend.report(
                "merge_committed merging=" + (workTree.opText !== "")
                + " kept=" + (mergeCommitTimer.wanted !== ""
                              && repoTab.headSubject === mergeCommitTimer.wanted)
                + " typed=" + mergeCommitTimer.typed
                + " head=" + repoTab.headSubject)
            driver.complete()
        }
    }
}
