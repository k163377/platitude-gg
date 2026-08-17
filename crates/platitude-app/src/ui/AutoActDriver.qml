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
    /// What the commit menu is standing on, read where a verb has to say
    /// which row it opened on and what was offered there.
    property CommitMenuState commitMenuState
    property AppMenu commitMenu
    property AppMenuItem dropCommitItem
    property AppMenuItem stashDeleteItem
    property AppMenu resetMenu
    property AppMenuItem hardResetItem
    property PublishFlow publishFlow
    property RemoteDialog remoteDialog
    property RefListPopup refList
    property CommitHoverCard rowCard

    /// Kicked off by the page once its models are attached: a verb that
    /// ran before them would act on a repository nothing has read yet.
    property bool claimed: false
    function begin() {
        autoActTimer.start()
    }

    // Completion belongs to the page that claimed the run.  A rendered
    // surface is enough for a read-only, synchronous verb.  A write is
    // different: seeing its request leave this item says nothing about the
    // repository, so retain the busy edge and the write answer as a causal
    // barrier before handing the shot driver a completed scene.
    property bool completionDeferred: false
    property bool writeExpected: false
    /// The write counter as it stood immediately before the request went
    /// out, so that its moving is proof this run's own write answered.
    ///
    /// **That is the whole of the proof.** Waiting to *see* `busyCount`
    /// rise as well wedges on a write that begins and ends between two
    /// looks at it — which the container did and the host did not, and
    /// which taking work out of the post-write refresh made likelier still
    /// (2026-08-17 実測: `line-back`, then `keep-place`).
    property int writeSeqBefore: 0

    function isWriteAct(act) {
        return ["publish", "publish-taken", "publish-add", "publish-go",
                "publish-new-go", "commit", "amend", "amend-reset-author",
                "stash", "stash-staged", "stash-file", "stage-many-go",
                "discard-many-go", "take-side-ours", "take-side-theirs",
                "open-mergetool", "discard-file-go", "delete-file-go",
                "discard-staged-go", "switch", "switch-remote", "nav-dbl",
                "rename-branch", "rename-tag", "rename-stash", "rename-remote",
                "rename-remote-go", "rename-local-upstream", "delete-branch",
                "delete-branch-go", "delete-tag-go", "delete-stash-go",
                "delete-remote-go", "delete-force", "delete-branch-refused",
                "delete-stash-row", "stash-apply-row", "stash-pop-row",
                "branch-at-tag", "dbl-local", "dbl-remote", "move-branch",
                "name-branch", "squash", "reword", "cherry-pick", "reset-soft",
                "reset-mixed", "reset-hard", "drop-commit-go", "merge-branch",
                "rebase-onto", "revert-commit", "op-exit-go", "stage-hunk",
                "stage-line", "keep-place", "discard-hunk-go", "line-back", "diff-follow",
                "line-run",
                "stage-all", "unstage-all",
                "push", "force-push", "push-retry", "fetch", "fetch-ref-list",
                "commands", "commands-fail", "commands-clear", "fetch-recover",
                "fetch-fail", "fetch-resume"].indexOf(act) >= 0
    }

    function defersCompletion(act) {
        return ["publish", "publish-taken", "publish-remotes", "publish-add",
                "publish-go", "publish-new-go", "amend-reset-author",
                "eol-commit", "eol-hover",
                "stage-hunk", "stage-line", "discard-hunk", "discard-hunk-go",
                "diff-file", "line-tools", "hunk-tools",
                "code-send", "line-back", "diff-follow", "line-run",
                "stage-all", "unstage-all",
                "keep-place", "colour-place", "delete-branch-go", "nav-fold",
                "nav-peek", "nav-unfold", "nav-peek-rename", "nav-peek-away",
                "nav-peek-into", "nav-peek-out", "nav-peek-shut", "nav-close",
                "nav-filter", "delete-branch-refused", "chip-menu", "chip-menu-current",
                "delete-blocked-tip", "delete-branch-early", "ref-list-card",
                "signature", "signature-tip", "stash-tip", "path-tip", "row-card",
                "author-card", "author-card-open", "co-authors", "co-authors-open",
                "details-grow", "details-grow-squeeze", "wip-grow", "wip-grow-squeeze",
                "details-fit", "corner", "graph-step", "graph-step-edge", "graph-step-far",
                "graph-step-named", "graph-step-dirty", "graph-step-diff", "diff-step",
                "diff-step-edge", "changes-step", "changes-step-edge", "wip-step",
                "graph-bar", "graph-bar-away", "middle-scroll",
                "graph-tail", "divider-refuse", "cherry-pick", "reword", "edit-message",
                "edit-message-leave", "edit-message-discard", "edit-message-focus",
                "eol-commit", "eol-hover",
                "push-retry", "fetch-ref-list", "avatar-assign", "avatar-badge",
                "avatar-settings", "avatar-combo", "avatar-row-lit", "avatar-remove",
                "find", "find-next", "find-prev",
                // These flows are completed by Main/WindowAutoActDriver.
                // Some still begin here (picker, command failure, recovery),
                // but the page must never photograph their intermediate
                // state before the window-level predicate has answered.
                "open-picker", "commands-clear", "fetch-recover",
                "open-not-a-repo", "open-bare", "open-not-a-repo-retry",
                "open-not-a-repo-cancel", "open-fail-tab",
                "open-fail-tab-bare", "open-fail-tab-log", "identity",
                "identity-half", "identity-tip", "band", "tab-widths",
                "tab-mark", "window-fill", "solo", "window-floor",
                "badges", "badges-hover", "old-git", "old-git-card",
                "old-git-fold", "state", "middle-close", "open-again",
                "force-push-hold", "settings-tools",
                "settings-tools-loading"].indexOf(act) >= 0
    }

    function prepareCompletion(act) {
        driver.completionDeferred = driver.defersCompletion(act)
        driver.writeExpected = driver.isWriteAct(act)
        driver.writeSeqBefore = repoTab.writeSeq
    }

    function dispatchFinished() {
        if (driver.completionDeferred)
            return
        if (driver.writeExpected) {
            writeBarrier.start()
            return
        }
        renderedBarrier.begin()
    }

    function complete() {
        page.Window.window.finishAutoAct()
    }

    /// How far down a diff the reader is taken before the thing that could
    /// cost them their place happens — the rebuild a partial write asks
    /// for (`keep-place`), and the swap the colours arrive in
    /// (`colour-place`). One number for both, because both are judged on
    /// getting exactly it back, and a place nobody can name is not one
    /// either of them can be caught losing.
    readonly property real readY: 400
    function reportPlace(at, room) {
        AppBackend.report("diff_place at=" + Math.round(at)
                          + " want=" + Math.round(driver.readY)
                          + " room=" + Math.round(room))
    }

    // AutoShotDriver owns the final render boundary: it requests an update,
    // advances the event loop, and waits for grabToImage callbacks. Do not
    // wait for frameSwapped here. A quiet scene is allowed not to emit one
    // (the pilot reproduced that hang twice under concurrent load).
    QtObject {
        id: renderedBarrier
        function begin() {
            driver.complete()
        }
    }
    // A write has two separate causal edges.  `busyCount` proves the
    // process was actually admitted, and `writeSeq` proves its answer was
    // absorbed.  Both must precede the final rendered state.
    Timer {
        id: writeBarrier
        interval: 25
        repeat: true
        onTriggered: {
            if (repoTab.busyCount !== 0
                    || repoTab.writeSeq <= driver.writeSeqBefore)
                return
            writeBarrier.stop()
            renderedBarrier.begin()
        }
    }

    Timer {
        id: autoActTimer
        interval: 25
        repeat: true
        onTriggered: {
            // Tabs are constructed before their active index settles. The
            // page that becomes current claims the one process-wide verb;
            // pages opened by that verb can never replay it.
            if (!driver.claimed) {
                if (!page.pageCurrent || !page.Window.window.claimAutoPageAct())
                    return
                driver.claimed = true
            }
            // `Opened` only means the path was accepted. Refs and the graph
            // are the baseline every page verb is allowed to act on.
            if (repoTab.state !== "open" || !workTree.loaded
                    || !branchesModel.refsLoaded || graphModel.finishCount === 0)
                return
            autoActTimer.stop()
            driver.runAutoAct()
        }
    }
    // HEAD's author has to arrive before the offer to take it over can
    // be there to tick.
    Timer {
        id: resetAuthorTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (repoTab.headAuthorName === "")
                return
            resetAuthorTimer.stop()
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
        interval: 25
        repeat: true
        onTriggered: {
            if (workTree.stagedCount === 0)
                return
            wipPane.pointAtCommit = true
            if (!wipPane.eolCardOpen)
                return
            eolCommitTimer.stop()
            AppBackend.report("eol_commit staged=" + workTree.stagedCount
                              + " warned=" + workTree.eolStagedCount
                              + " card=" + wipPane.eolCardOpen)
            driver.complete()
        }
    }
    // The marks arrive with the status read, so the row named for its
    // sentence has to be named again once they are in.
    Timer {
        id: eolHoverTimer
        interval: 25
        repeat: true
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
    // The diff has to arrive before a row of it can be staged. Asked for
    // rather than waited out: a fixed wait photographs an empty pane the
    // same as a late one (2026-08-13 実測: a verb fired against this
    // repository named no row and passed). The asking has no ceiling:
    // a row that never lands leaves the run without a report line at all,
    // and the watchdog is what ends it.
    Timer {
        id: stageRowTimer
        interval: 50
        repeat: true
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
            if (!arrived)
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
                driver.complete()
                return
            }
            // The squares a line only puts out under the pointer, named
            // rather than hovered (hover cannot be injected on Windows).
            if (act === "line-tools") {
                diffPane.showLineTools(0, line)
                renderedBarrier.begin()
                return
            }
            // The heading's two words carry their colours only under the
            // pointer, and hover cannot be injected, so the row is named
            // instead. A heading's own row is line -1 (`flatten_patches`).
            if (act === "hunk-tools") {
                diffPane.showLineTools(0, -1)
                renderedBarrier.begin()
                return
            }
            // Reading part way down a long diff and then writing: the
            // rebuild has to come back to the same place.
            if (act === "keep-place") {
                keepPlaceTimer.begin(line)
                return
            }
            if (act === "stage-hunk" || act === "stage-line") {
                driver.writeSeqBefore = repoTab.writeSeq
                // One line goes through its own mark — the press writes,
                // there and then — and a hunk through its heading's word.
                if (act === "stage-line")
                    diffPane.stageLine(0, line)
                else
                    page.stageSelection(0, -1)
                writeBarrier.start()
                return
            }
            // Sending the code sideways, and the hand that sends it and
            // the rows at once. Both read what moved rather than what was
            // asked for: a bar bound to nothing still takes a press.
            if (act === "code-send") {
                codeSendTimer.begin()
                return
            }
            if (act === "line-run") {
                lineRunTimer.begin()
                return
            }
            if (act === "diff-follow") {
                followTimer.begin()
                return
            }
            if (act === "line-back") {
                lineBackTimer.begin()
                return
            }
            // No line-level discard exists — a hunk is the smallest piece
            // that can be thrown away.
            if (act === "discard-hunk-go") {
                driver.writeSeqBefore = repoTab.writeSeq
                diffPane.completeHold()
                writeBarrier.start()
            } else {
                renderedBarrier.begin()
            }
        }
    }
    // One line staged from the diff, then the same file moved from the
    // file list — the diff has to follow both, and it used to follow only
    // the first (2026-08-17 ユーザー報告: the line was gone from the
    // unstaged side and never came back when the file was unstaged).
    //
    // Three answers in one run, because they are one story: the line goes
    // (the rows shrink), the line comes back (the rows are as they were),
    // and staging the rest empties the side being read — where the pane
    // follows the file to the side it went to rather than closing on the
    // reader (`RepoPage.followEmptySide`). Each step waits for its own
    // write to land *and* for the pane to say so — the rows and the key
    // are the output, the write is only the cause.
    Timer {
        id: lineBackTimer
        interval: 25
        repeat: true
        property int step: 0
        property int rows0: 0
        property int rows1: 0
        property bool shrank: false
        property bool back: false
        function begin() {
            lineBackTimer.step = 0
            lineBackTimer.shrank = false
            lineBackTimer.back = false
            lineBackTimer.start()
        }
        /// Whether the write this step asked for has landed. The sequence
        /// is read immediately before asking, so its moving is the whole
        /// of the evidence — waiting to *see* `busyCount` rise as well
        /// wedges on a write that begins and ends inside one tick, which
        /// is what the container did while the host did not (2026-08-17
        /// 実測: `line-back` PASS on Windows, watchdog on Linux).
        function wroteAndSettled() {
            return repoTab.busyCount === 0
                    && repoTab.writeSeq > driver.writeSeqBefore
        }
        function expect() {
            driver.writeSeqBefore = repoTab.writeSeq
        }
        onTriggered: {
            const rows = diffPane.view.count
            if (lineBackTimer.step === 0) {
                const line = diffPane.firstChangedLine(0)
                if (line < 0)
                    return
                lineBackTimer.rows0 = rows
                lineBackTimer.expect()
                diffPane.stageLine(0, line)
                lineBackTimer.step = 1
                return
            }
            if (lineBackTimer.step === 1) {
                if (!lineBackTimer.wroteAndSettled() || rows >= lineBackTimer.rows0)
                    return
                lineBackTimer.rows1 = rows
                lineBackTimer.shrank = true
                lineBackTimer.expect()
                // The file list's own `−`, which is the half that was
                // never reaching the pane.
                repoTab.unstagePath(page.diffPath)
                lineBackTimer.step = 2
                return
            }
            if (lineBackTimer.step === 2) {
                if (!lineBackTimer.wroteAndSettled() || rows !== lineBackTimer.rows0)
                    return
                lineBackTimer.back = true
                lineBackTimer.expect()
                repoTab.stagePath(page.diffPath)
                lineBackTimer.step = 3
                return
            }
            if (!lineBackTimer.wroteAndSettled()
                    || page.diffKind !== "staged" || rows === 0)
                return
            lineBackTimer.stop()
            AppBackend.report("line_back back=" + lineBackTimer.back
                              + " shrank=" + lineBackTimer.shrank
                              + " followed=" + (page.diffKind + ":" + page.diffPath)
                              + " rows0=" + lineBackTimer.rows0
                              + " rows1=" + lineBackTimer.rows1)
            renderedBarrier.begin()
        }
    }
    // Line after line, the way a hand does it. The pane refuses a press
    // while the rows it would be written against are still coming
    // (`RepoPage.diffSettling`), so this waits for exactly that and no
    // clock — which is also the thing that broke: held on a signal the
    // file list only sends when its rows differ, the pane went quiet for
    // good at the second line of a file already on both sides, and no `+`
    // anywhere would go in again (2026-08-17 ユーザー報告).
    Timer {
        id: lineRunTimer
        interval: 25
        repeat: true
        readonly property int want: 3
        property int done: 0
        /// How long the list has gone without a row to name, which is not
        /// the same as having none (see below).
        property int waited: 0
        function begin() {
            lineRunTimer.done = 0
            lineRunTimer.waited = 0
            lineRunTimer.start()
        }
        function report() {
            AppBackend.report("line_run staged=" + lineRunTimer.done
                              + " want=" + lineRunTimer.want
                              + " rows=" + diffPane.view.count
                              + " waited=" + lineRunTimer.waited)
        }
        onTriggered: {
            // The pane is still catching up with the last press.
            if (page.diffSettling)
                return
            if (lineRunTimer.done === lineRunTimer.want) {
                lineRunTimer.stop()
                lineRunTimer.report()
                renderedBarrier.begin()
                return
            }
            // A row is named by walking the list's own items, and the
            // list builds them a frame after the model hands the rows
            // over: read too early it names nothing, which is not the
            // same as there being nothing (`keep-place` learned it too).
            // So an empty answer is waited on — but not for ever, since a
            // fixture with fewer changed lines than this asks for is the
            // run's own fault and has to show as one rather than as a
            // watchdog.
            const line = diffPane.firstChangedLine(0)
            if (line < 0) {
                lineRunTimer.waited += lineRunTimer.interval
                if (lineRunTimer.waited < 5000)
                    return
                lineRunTimer.stop()
                lineRunTimer.report()
                renderedBarrier.begin()
                return
            }
            lineRunTimer.waited = 0
            lineRunTimer.done++
            diffPane.stageLine(0, line)
        }
    }
    // Where the reader lands when the file under the open diff is moved
    // whole from the file list. Two answers, and which one is right
    // depends on what is left behind (デザイン規約 §diff の中のステージ):
    // with other files still on that side the pane takes the next of them,
    // and with none left it stays on the same file and reads it from the
    // side it went to.
    //
    // The argument names the file to open; the verb decides its own second
    // step from what the tree holds afterwards, and reports both.
    Timer {
        id: followTimer
        interval: 25
        repeat: true
        property int step: 0
        property string was: ""
        property string landed: ""
        property bool alone: false
        function begin() {
            followTimer.step = 0
            followTimer.landed = ""
            followTimer.start()
        }
        onTriggered: {
            if (followTimer.step === 0) {
                if (diffPane.view.count === 0)
                    return
                followTimer.was = page.diffKind + ":" + page.diffPath
                // Whether this side has anything else on it, read before
                // the write takes the file off it.
                followTimer.alone =
                    worktreeModel.besidePath(page.diffKind, page.diffPath) === ""
                // The sequence read here is the whole test below: seeing
                // `busyCount` rise as well wedges on a write that begins
                // and ends inside one tick (see `lineBackTimer`).
                driver.writeSeqBefore = repoTab.writeSeq
                // The file list's own `+` / `−`, whole file at a time.
                if (page.diffKind === "staged")
                    repoTab.unstagePath(page.diffPath)
                else
                    repoTab.stagePath(page.diffPath)
                followTimer.step = 1
                return
            }
            // The landing is the output: the pane has to have moved off
            // the key it was on and settled somewhere with rows.
            const now = page.diffShown ? page.diffKind + ":" + page.diffPath : ""
            if (repoTab.busyCount !== 0
                    || repoTab.writeSeq <= driver.writeSeqBefore
                    || now === followTimer.was
                    || (page.diffShown && diffPane.view.count === 0))
                return
            followTimer.stop()
            followTimer.landed = now
            AppBackend.report("diff_follow shown=" + page.diffShown
                              + " alone=" + followTimer.alone
                              + " was=" + followTimer.was
                              + " landed=" + followTimer.landed)
            renderedBarrier.begin()
        }
    }
    // Emptying one whole bucket from its own heading, and reading back
    // which headings the list is left with. The two directions are one
    // verb because the claim is that they are symmetrical: a bucket that
    // has just been emptied keeps its heading, whichever bucket it was
    // (デザイン規約 §その他の操作).
    //
    // The heading is pressed rather than the slot behind it called, and
    // what is read back is the list's own children — a band bound to
    // nothing would still be counted by the condition that asks for it.
    Timer {
        id: bucketAllTimer
        interval: 25
        repeat: true
        /// Which bucket is being emptied, and whether the press went in.
        property string from: ""
        property bool pressed: false
        function begin(bucket) {
            bucketAllTimer.from = bucket
            bucketAllTimer.pressed = false
            bucketAllTimer.start()
        }
        onTriggered: {
            if (!bucketAllTimer.pressed) {
                // The heading exists once the list has laid its sections
                // out, which is a frame after the rows arrive.
                if (wipPane.rowAt(0) === null
                        || !wipPane.moveBucket(bucketAllTimer.from))
                    return
                driver.writeSeqBefore = repoTab.writeSeq
                bucketAllTimer.pressed = true
                return
            }
            if (repoTab.busyCount !== 0
                    || repoTab.writeSeq <= driver.writeSeqBefore)
                return
            // The bucket that was emptied has to be empty before its
            // heading means anything: the counts are the model's answer
            // and the headings are the list's, and reading the second
            // before the first would report the state that was.
            const emptied = bucketAllTimer.from === "staged"
                          ? workTree.stagedCount
                          : workTree.unstagedCount + workTree.untrackedCount
            if (emptied !== 0)
                return
            bucketAllTimer.stop()
            AppBackend.report("wip_heads from=" + bucketAllTimer.from
                              + " unstaged=" + wipPane.bucketHeaded("unstaged")
                              + " staged=" + wipPane.bucketHeaded("staged")
                              + " unstaged_count="
                              + (workTree.unstagedCount + workTree.untrackedCount)
                              + " staged_count=" + workTree.stagedCount)
            renderedBarrier.begin()
        }
    }
    // Sending the diff's code sideways, by the bar's own path and then by
    // the hand that carries the rows with it. What is read back is where
    // the code and the rows ended up, never what was asked for: a bar
    // bound to nothing still takes a press, and a hand wired to nothing
    // still starts.
    //
    // The wait is for the view (`keep-place` learned the same lesson):
    // rows that have arrived are not rows the list has laid out, and until
    // it has, `codeMax` is measured against a width of nothing. A diff
    // with nowhere sideways to go says so and stops there rather than at
    // the watchdog — a run over one photographs a pane that proves
    // nothing (app-ui.md §UI 自動化の因果性).
    Timer {
        id: codeSendTimer
        interval: 25
        repeat: true
        property bool sent: false
        function begin() {
            codeSendTimer.sent = false
            codeSendTimer.start()
        }
        function laidOut() {
            return diffPane.view.count > 0 && diffPane.view.width > 0
                    && diffPane.view.contentHeight > 0
        }
        function report() {
            AppBackend.report("code_send bar=" + diffPane.codeBarShown
                              + " hand=" + diffPane.codeHandOn
                              + " at=" + Math.round(diffPane.codeAt)
                              + " max=" + Math.round(diffPane.codeMax)
                              + " down=" + Math.round(diffPane.view.contentY))
        }
        onTriggered: {
            if (!codeSendTimer.sent) {
                if (!codeSendTimer.laidOut())
                    return
                if (diffPane.codeMax <= 0) {
                    codeSendTimer.stop()
                    codeSendTimer.report()
                    renderedBarrier.begin()
                    return
                }
                // Half the way by the bar's own path, and the rest — with
                // the rows — by the hand, started from the middle of the
                // view and drifted down and to the right. The anchor's
                // ring is part of the picture, so the hand is left running
                // for the shot.
                diffPane.sendCode(diffPane.codeMax / 2)
                diffPane.startCodeHand(diffPane.view.width / 2,
                                       diffPane.view.height / 2)
                diffPane.driftCodeHand(diffPane.view.width / 2 + 120,
                                       diffPane.view.height / 2 + 120)
                codeSendTimer.sent = true
                return
            }
            // The hand ticks on its own clock, and both of its axes have
            // to be seen moving: the rows have come down, and the code has
            // gone further than the bar's half left it.
            if (diffPane.view.contentY <= 0
                    || diffPane.codeAt <= diffPane.codeMax / 2)
                return
            codeSendTimer.stop()
            codeSendTimer.report()
            renderedBarrier.begin()
        }
    }
    // Reading part way down a long diff and then writing: the rebuild has
    // to come back to the same place.
    //
    // The wait is for the view, not for the model (`diff-step` learned
    // the same lesson): rows that have arrived are not rows the list has
    // laid out, and until it has there is no place to lose — the scroll
    // goes nowhere and the restore has nothing to undo. So what is waited
    // for is the room the reading consumes, and a diff that is laid out
    // and still too short says so and stops there rather than at the
    // watchdog: nothing that short can hold a place, and a run over it
    // photographs a pane that proves nothing (app-ui.md §UI 自動化の
    // 因果性).
    Timer {
        id: keepPlaceTimer
        interval: 25
        repeat: true
        /// The line of the first hunk that gets staged, which is what
        /// rebuilds the diff under the reader.
        property int line: -1
        property bool wrote: false
        function begin(atLine) {
            keepPlaceTimer.line = atLine
            keepPlaceTimer.wrote = false
            keepPlaceTimer.start()
        }
        /// Rows the list has actually put down, as against rows it has
        /// been handed: `contentHeight` is still zero for the first of
        /// those and `maxY` cannot be read before it.
        function laidOut() {
            return diffPane.view.count > 0 && diffPane.view.height > 0
                    && diffPane.view.contentHeight > 0
        }
        onTriggered: {
            if (!keepPlaceTimer.wrote) {
                if (!keepPlaceTimer.laidOut())
                    return
                if (diffPane.view.maxY < driver.readY) {
                    keepPlaceTimer.stop()
                    driver.reportPlace(diffPane.view.contentY, diffPane.view.maxY)
                    renderedBarrier.begin()
                    return
                }
                diffPane.scrollTo(driver.readY)
                driver.writeSeqBefore = repoTab.writeSeq
                page.stageSelection(0, keepPlaceTimer.line)
                keepPlaceTimer.wrote = true
                return
            }
            // Both edges of the write, and then the one output the whole
            // verb is about: the rebuilt list put back on the place. A
            // write that emptied this side never gets a row back and so
            // never lands anywhere — which is a fixture with no place in
            // it, and the run waits rather than passing on the silence.
            if (repoTab.busyCount !== 0
                    || repoTab.writeSeq <= driver.writeSeqBefore
                    || diffPane.placeLandedY < 0)
                return
            keepPlaceTimer.stop()
            driver.reportPlace(diffPane.placeLandedY, diffPane.view.maxY)
            renderedBarrier.begin()
        }
    }
    // A reader who scrolled before the colours landed: the whole list is
    // swapped again when the colours turn up (`DiffModel::lay_out_rows`),
    // and that swap must not cost the place being read.
    Timer {
        id: colourPlaceTimer
        interval: 50
        repeat: true
        property int waited: 0
        property bool scrolled: false
        function begin() {
            colourPlaceTimer.waited = 0
            colourPlaceTimer.scrolled = false
            colourPlaceTimer.start()
        }
        onTriggered: {
            colourPlaceTimer.waited += colourPlaceTimer.interval
            if (!colourPlaceTimer.scrolled) {
                // Read down the file the moment the rows are there, which
                // is well before the colours are.
                if (diffPane.firstChangedLine(0) < 0)
                    return
                diffPane.scrollTo(driver.readY)
                colourPlaceTimer.scrolled = true
                return
            }
            if (!diffPane.diffModel.coloured)
                return
            colourPlaceTimer.stop()
            AppBackend.report("colour_place coloured="
                              + diffPane.diffModel.coloured
                              + " at=" + Math.round(diffPane.view.contentY)
                              + " rows=" + diffPane.view.count
                              + " waited=" + colourPlaceTimer.waited)
            driver.complete()
        }
    }
    // git's refusal has to come back before the row it turns into a held
    // one can be held — or photographed.
    Timer {
        id: forceDeleteTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (repoTab.writeSeq <= driver.writeSeqBefore
                    || repoTab.busyCount !== 0 || refDeleteItem.holdMs <= 0)
                return
            forceDeleteTimer.stop()
            AppBackend.report("ref_menu delete=" + refDeleteItem.text
                              + " note=" + refDeleteItem.note)
            driver.writeSeqBefore = repoTab.writeSeq
            refDeleteItem.completeHold()
            writeBarrier.start()
        }
    }
    // The splitter has to have handed the pane its new width before the
    // width can be reported — the fold sets it, the layout takes it.
    Timer {
        id: navRailTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (sidebarPane.width <= 0 || sidebarPane.height <= 0)
                return
            navRailTimer.stop()
            AppBackend.report(
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
            driver.complete()
        }
    }
    // The column has to be laid out again before the header that was
    // closed can say where it ended up.
    Timer {
        id: navSectionTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (sidebarPane.height <= 0)
                return
            navSectionTimer.stop()
            AppBackend.report(
            "nav_section closed=" + AppBackend.autoActArg
            + " header=" + Math.round(
                sidebarPane.headerTopOf(AppBackend.autoActArg))
            + " ground=" + Math.round(sidebarPane.groundTop)
            + " pane=" + Math.round(sidebarPane.height))
            driver.complete()
        }
    }
    /// What each section kept of what it holds. The rows a filter leaves
    /// are the ones the sections work out for themselves, so the counts
    /// are read off the models and the picture says what they drew.
    Timer {
        id: navFilterTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (sidebarPane.width <= 0)
                return
            navFilterTimer.stop()
            AppBackend.report(
            "nav_filter typed=" + AppBackend.autoActArg
            + " branches=" + branchesModel.shown() + "/" + branchesModel.total
            + " remotes=" + remotesModel.shown() + "/" + remotesModel.total
            + " tags=" + tagsModel.shown() + "/" + tagsModel.total
            + " stashes=" + stashesModel.shown() + "/" + stashesModel.total)
            driver.complete()
        }
    }
    // The blocked row's line, worn where the pointer would put it.
    // Past `tipDelayMs`, like the other forced tooltips: read any sooner
    // and the attached ToolTip has not opened yet, so the line reports
    // false while the picture taken at quit holds it.
    Timer {
        id: blockedTipTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (!refDeleteItem.ToolTip.visible)
                return
            blockedTipTimer.stop()
            AppBackend.report(
            "delete_blocked code=" + refDeleteItem.code
            + " tip=" + refDeleteItem.ToolTip.visible
            + " reason=" + refDeleteItem.blockedReason)
            driver.complete()
        }
    }
    Timer {
        id: chipMenuTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (!refMenu.opened && !commitMenu.opened)
                return
            chipMenuTimer.stop()
            AppBackend.report("chip_menu ref=" + refMenu.opened
                                       + " commit=" + commitMenu.opened
                                       + " delete=" + refDeleteItem.code
                                       + " " + refDeleteItem.text)
            driver.complete()
        }
    }
    // Waits on the early answer, not on a refusal: nothing here writes.
    Timer {
        id: earlyDeleteTimer
        interval: 25
        repeat: true
        onTriggered: {
            // The row's `code` is never empty on a branch, so it cannot
            // tell "git has not answered yet" from "answered merged" —
            // both wear `branch --delete`. What readiness there is comes
            // from the echo of the branch asked about, which the asking
            // clears before the question goes out (app-ui.md
            // §UI 自動化の因果性).
            if (repoTab.branchDeleteAsked !== AppBackend.autoActArg)
                return
            earlyDeleteTimer.stop()
            AppBackend.report("delete_early asked="
                                       + (repoTab.branchDeleteAsked !== "")
                                       + " merged=" + repoTab.branchDeleteMerged
                                       + " code=" + refDeleteItem.code
                                       + " held=" + (refDeleteItem.holdMs > 0)
                                       + " note=" + refDeleteItem.note)
            driver.complete()
        }
    }
    Timer {
        id: refusedRowTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (repoTab.writeSeq <= driver.writeSeqBefore || repoTab.busyCount !== 0)
                return
            refusedRowTimer.stop()
            AppBackend.report("ref_menu delete=" + refDeleteItem.code
                                       + " " + refDeleteItem.text
                                       + " note=" + refDeleteItem.note)
            driver.complete()
        }
    }
    // The window cut: the walk stops at a round number of commits and the
    // footer is the only thing that says so — its lanes carry on for one
    // more commit's worth and its line names the count.
    //
    // Nothing here is waited out. The walk has to have answered before
    // `truncated` means anything (the initial false is "not asked yet",
    // not "the whole history is loaded" — app-ui.md §UI 自動化の因果性),
    // the footer has to have been given a height, and the view has to
    // have actually arrived at the end rather than merely been told to
    // go: `atYEnd` is the output, `positionViewAtEnd()` only the ask.
    Timer {
        id: graphTailTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (graphModel.loading || graphModel.rowTotal === 0)
                return
            // The footer lives at the far end of two thousand rows, and a
            // ListView builds what is near its viewport — so it is asked
            // for and then looked for, rather than looked for first.
            const tail = graphPane.view.footerItem
            if (tail === null || tail.height <= 0) {
                graphPane.view.positionViewAtEnd()
                return
            }
            // On screen whole, read off where it sits rather than off the
            // call having been made: `positionViewAtEnd` puts the last
            // *row* against the edge, and this pane keeps a run-out below
            // it, so being told to go is not the same as having arrived.
            const bottom = graphPane.view.contentY + graphPane.view.height
            if (tail.y + tail.height > bottom + 0.5) {
                graphPane.view.positionViewAtEnd()
                return
            }
            graphTailTimer.stop()
            // The verdict leads, and its two halves are neighbours: a
            // graph that never cut and one whose footer failed to draw
            // frame the same way — the end of a history and the end of
            // what was loaded are the same picture without the line.
            AppBackend.report(
                "graph_tail truncated=" + graphModel.truncated
                + " shown=" + tail.visible
                + " walked=" + graphModel.walkedTotal
                + " rows=" + graphPane.view.count
                + " height=" + Math.round(tail.height)
                + " lanes=" + (graphModel.tailGeometry === ""
                               ? 0 : graphModel.tailGeometry.split(";").length))
            driver.complete()
        }
    }
    // The lane column has to have taken its narrower width before there
    // is anywhere to pan to, or a bar worth wanting.
    Timer {
        id: graphPanTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (graphPane.graphXMax <= 0)
                return
            graphPanTimer.stop()
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
                driver.complete()
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
    // Where the lanes ended up is the whole question, so that is what is
    // waited for — a pan that ran and a pan that was refused must not
    // read alike in the report.
    //
    // A gesture that carries the lanes runs until they have nowhere left
    // to go: the pointer was put a whole pane's width out, so the ticker
    // saturates the clamp and `graphX` stops at its own maximum. One that
    // does not carry them has already answered by starting without the
    // carry — `panning` is settled in `start()` by where the click landed
    // — and no tick will ever move them.
    //
    // Not "wait for `autoPanning` to go false": the flag is kept for the
    // whole gesture (デザイン規約 §グラフを横へ送る), and nothing here
    // ends the gesture, so the lane column's own case never completed
    // (2026-08-16 実測: watchdog on both systems, `message` passing beside
    // it because that one never pans).
    Timer {
        id: middleScrollTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (graphPane.autoPanning
                    && graphPane.graphX < graphPane.graphXMax - 0.5)
                return
            middleScrollTimer.stop()
            AppBackend.report(
            "middle_scroll lanes=" + graphPane.autoPanning
            + " x=" + Math.round(graphPane.graphX)
            + " max=" + Math.round(graphPane.graphXMax))
            driver.complete()
        }
    }
    // The arrow keys, which no headless run can press: the walk enters
    // where `Keys.onDownPressed` enters (`GraphPane.stepRow`) after
    // taking the keyboard the way a row click takes it. The selected
    // commit's message has to have arrived before it can be typed over,
    // which is what the wait is for — the same one the reword verbs keep.
    Timer {
        id: graphStepTimer
        interval: 25
        repeat: true
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
            if (detailsModel.shaHex !== page.selectedOid)
                return
            graphStepTimer.stop()
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
        interval: 25
        repeat: true
        onTriggered: {
            if (graphStepTimer.diffPath !== "" && !page.diffShown)
                return
            graphStepWalk.stop()
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
        interval: 25
        repeat: true
        property int from: -1
        property real wasY: 0
        property int refused: 0
        onTriggered: {
            const row = graphPane.view.currentIndex
            if (row < 0 || (graphStepTimer.diffPath === "" && page.selectedOid
                            !== graphModel.oidAt(row)))
                return
            graphStepReport.stop()
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
            driver.complete()
        }
    }
    // The file list's arrows: the light and the diff move together, one file
    // per press (規約 §diff のファイル一覧). Two things have to be real for
    // this to say anything, so both go through the door a hand goes through:
    //
    //  - the click. The row's own signal is raised by name, not the pane's
    //    handler — the handler is where the keyboard is handed to the list,
    //    and calling past it would leave `focused=` proving nothing (the same
    //    reason `nav-peek` strikes the cell and not `SidebarPane`).
    //  - the step, which enters at `stepFile` where `Keys.onDownPressed`
    //    enters. A keystroke cannot be injected (verify-ui).
    //
    // Nothing here reaches for the keyboard, and that is the point: the diff
    // opened without taking it, so an arrow still belongs to the list.
    Timer {
        id: fileStepTimer
        interval: 25
        repeat: true
        /// Which list, `changes` or `wip`, and how far to walk. `overrun`
        /// asks for more files than the list holds, which is how the end it
        /// stops at is reached — the count is only known once the commit's
        /// details have arrived, so it cannot be a number set up here.
        property string pane: "changes"
        property int steps: 1
        property bool overrun: false
        /// The file clicked, and the bucket its row sits in (empty for the
        /// commit's list, whose files sit in none).
        property string bucket: ""
        property string path: ""
        /// Whether the click has gone out, so the tick that follows is
        /// waiting for the diff rather than for the row.
        property bool clicked: false
        property bool stopped: false
        function begin() {
            fileStepTimer.clicked = false
            fileStepTimer.stopped = false
            fileStepTimer.steps = 1
            fileStepTimer.start()
        }
        readonly property var walk:
            fileStepTimer.pane === "wip" ? wipPane.filesWalk
                                         : detailsPane.filesWalk
        onTriggered: {
            if (!fileStepTimer.clicked) {
                const row = fileStepTimer.walk.rowFor(fileStepTimer.bucket,
                                                      fileStepTimer.path)
                if (!row)
                    return
                fileStepTimer.clicked = true
                if (fileStepTimer.pane === "wip")
                    row.fileClicked(fileStepTimer.bucket, fileStepTimer.path,
                                    worktreeModel.origOf(fileStepTimer.path),
                                    Qt.NoModifier)
                else
                    row.activated("", fileStepTimer.path,
                                  detailsModel.origOf(fileStepTimer.path))
                return
            }
            // The click has to have landed before a step means anything: a
            // walk with nothing being read is refused, and reading that as
            // "the end" would go green on a click that never arrived.
            if (!page.diffShown || page.diffPath !== fileStepTimer.path)
                return
            fileStepTimer.stop()
            if (fileStepTimer.overrun)
                fileStepTimer.steps = fileStepTimer.walk.view.count + 5
            const way = fileStepTimer.steps < 0 ? -1 : 1
            for (let n = 0; n < Math.abs(fileStepTimer.steps); n++) {
                if (!fileStepTimer.walk.stepFile(way))
                    fileStepTimer.stopped = true
            }
            fileStepReport.start()
        }
    }
    // Longer than the settle behind the walk (`keyStepSettleMs`), because
    // what is read is the reading a hand coming off the key gets: the light
    // runs at the key's rate and the diff catches up after it, so a run that
    // moved the light and never moved the diff has to be told apart from one
    // that did (規約 §diff のファイル一覧).
    Timer {
        id: fileStepReport
        interval: 25
        repeat: true
        onTriggered: {
            const walk = fileStepTimer.walk
            // The diff the walk landed on has been asked for, has arrived,
            // and the row that says which file it is has been built. All
            // three are the output; the step was the cause. The middle one is
            // what keeps the picture worth looking at — a pane still waiting
            // on its read photographs empty.
            if (!page.diffShown || page.diffPath === fileStepTimer.path
                    || !diffPane.diffSettled() || walk.litPath() === "")
                return
            fileStepReport.stop()
            AppBackend.report(
                "file_step pane=" + fileStepTimer.pane
                + " from=" + fileStepTimer.path
                + " steps=" + fileStepTimer.steps
                + " read=" + (page.diffKind + ":" + page.diffPath)
                + " moved=" + (page.diffPath !== fileStepTimer.path)
                + " stopped=" + fileStepTimer.stopped
                + " lit=" + (walk.litPath() === page.diffPath)
                + " focused=" + walk.view.activeFocus)
            driver.complete()
        }
    }
    // The diff's own arrows, which no headless run can press either: the
    // walk enters where `Keys.onDownPressed` enters (`DiffPane.stepRows`).
    // The hand is walked into the pane first, through the same door the
    // wheel comes in by (`DiffPane.handArrived`) — the diff does not take
    // the keyboard by appearing, so without that the arrows are still the
    // file list's and `focused=` would be false for the right reason
    // (規約 §diff を上下に送る).
    //
    // The wait is for the view, not for the model. `diffSettled()` says the
    // rows arrived; it says nothing about the list having laid them out,
    // and a list whose `contentHeight` is still zero clamps every step to
    // where it already was — the walk then reads exactly like a diff with
    // nothing to scroll (2026-08-16 実測: 1 run in 3 came through with
    // `contentHeight` 0 at the step and 216 by the time it was reported).
    // So what is waited for is the output the step consumes: a view with
    // room to be sent, which is `atEnd` answering false over a laid-out
    // height (app-ui.md §UI 自動化の因果性「まだ答えが無い」と値を分ける).
    Timer {
        id: diffStepTimer
        interval: 25
        repeat: true
        /// How many rows, and which way.
        property int steps: 1
        onTriggered: {
            if (!page.diffShown || !diffPane.diffSettled()
                    || diffPane.view.height <= 0 || diffPane.atEnd)
                return
            diffStepTimer.stop()
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
        interval: 25
        repeat: true
        property int from: -1
        property bool stopped: false
        onTriggered: {
            if (!diffPane.diffSettled())
                return
            diffStepReport.stop()
            AppBackend.report(
            "diff_step from=" + diffStepReport.from
            + " rows=" + driver.diffRow()
            + " steps=" + diffStepTimer.steps
            + " moved=" + (driver.diffRow() !== diffStepReport.from)
            + " atEnd=" + diffPane.atEnd
            + " stopped=" + diffStepReport.stopped
            + " focused=" + diffPane.view.activeFocus)
            driver.complete()
        }
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
        interval: 25
        repeat: true
        onTriggered: {
            if (repoTab.busyCount !== 0 || repoTab.writeSeq <= driver.writeSeqBefore)
                return
            const stacked = graphPane.view.itemAtIndex(
                Number(AppBackend.autoActArg))
            if (!stacked)
                return
            fetchedRefListTimer.stop()
            graphPane.view.chipExpandRequested(
                stacked.chipItem.records, stacked.chipItem)
            renderedBarrier.begin()
        }
    }
    // The refusal has to be back and on the button before the second go
    // is sent, and the report is what says it ever got there — the mark
    // is gone again by the time the screenshot is taken.
    Timer {
        id: pushRetryTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (!page.pushFailed || repoTab.busyCount !== 0)
                return
            pushRetryTimer.stop()
            AppBackend.report("push_retry refused=" + page.pushFailed
                              + " branch=" + publishFlow.pushFailBranch)
            driver.writeSeqBefore = repoTab.writeSeq
            page.forcePush()
            writeBarrier.start()
        }
    }
    // Where an operation that answers at the tip left the reader — one
    // report for the three of them. The write, its refresh and the beat
    // the viewport waits out all have to be behind it, and the picture
    // cannot answer the second half: a row can be selected and still be
    // somewhere nobody can see.
    Timer {
        id: tipLandedTimer
        interval: 25
        repeat: true
        onTriggered: {
            const row = graphModel.rowOf(page.selectedOid)
            if (repoTab.busyCount !== 0 || row < 0 || !graphPane.rowOnScreen(row)
                    || page.selectedOid !== branchesModel.headOid)
                return
            tipLandedTimer.stop()
            AppBackend.report(
                "tip_landed follows="
                + (page.selectedOid !== "" && page.selectedOid === branchesModel.headOid)
                + " onscreen=" + (row >= 0 && graphPane.rowOnScreen(row))
                + " op=" + repoTab.lastWriteOp
                + " head=" + branchesModel.headOid.substring(0, 8)
                + " selected=" + page.selectedOid.substring(0, 8)
                + " row=" + row)
            driver.complete()
        }
    }
    // The message has to arrive before it can be typed over, and the
    // "is this commit ours to rewrite?" answer before it may be saved.
    Timer {
        id: rewordTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (detailsModel.shaHex !== page.selectedOid)
                return
            rewordTimer.stop()
            // "edit-message-focus" types nothing: the commit's own body
            // is what the caret has to be photographed on top of, and
            // an empty box would only show the placeholder.
            if (AppBackend.autoAct === "edit-message-focus") {
                detailsPane.focusDescription()
                AppBackend.report("message_focus pane=details focused="
                                  + detailsPane.descriptionFocused
                                  + " color=" + detailsPane.descriptionColor)
                driver.complete()
                return
            }
            detailsPane.setMessageText(AppBackend.autoActArg, "")
            // "edit-message" stops here, with the save row on screen.
            if (AppBackend.autoAct === "reword") {
                driver.writeSeqBefore = repoTab.writeSeq
            }
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
            if (AppBackend.autoAct === "reword")
                writeBarrier.start()
            else
                renderedBarrier.begin()
        }
    }
    // gpg / ssh-keygen have to finish before the mark they decide can be
    // on screen, so the shot and the report both wait for them.
    Timer {
        id: signatureTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (!page.signatureIsForSelection)
                return
            signatureTimer.stop()
            AppBackend.report("signature kind=" + page.selectedSignatureKind
                              + " code=" + page.selectedSignatureCode
                              + " signer=" + page.selectedSignatureSigner)
            driver.complete()
        }
    }
    // The tooltip halves of signature-tip / stash-tip: the state has to
    // land (gpg's verdict, the stash's details) before the target is
    // pointed at, and the report then waits out Metrics.tipDelayMs so
    // what it reads is the tip on screen.
    Timer {
        id: signatureTipTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (!page.signatureIsForSelection)
                return
            signatureTipTimer.stop()
            detailsPane.signaturePointedAt = true
            signatureTipReport.start()
        }
    }
    Timer {
        id: signatureTipReport
        interval: 25
        repeat: true
        onTriggered: {
            if (!detailsPane.signatureTipShown)
                return
            signatureTipReport.stop()
            AppBackend.report("signature_tip code=" + page.selectedSignatureCode
                              + " tip=" + detailsPane.signatureTipShown)
            driver.complete()
        }
    }
    Timer {
        id: stashTipTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (detailsModel.shaHex !== page.selectedOid)
                return
            stashTipTimer.stop()
            detailsPane.summaryPointedAt = true
            stashTipReport.start()
        }
    }
    Timer {
        id: stashTipReport
        interval: 25
        repeat: true
        onTriggered: {
            if (!detailsPane.summaryTipShown)
                return
            stashTipReport.stop()
            AppBackend.report(
            "stash_tip blocked=" + (detailsPane.editBlocked !== "")
            + " tip=" + detailsPane.summaryTipShown)
            driver.complete()
        }
    }
    // The tooltip half of path-tip: the list has to land before a row
    // can be pointed at, and the report then waits out tipDelayMs so
    // what it reads is the tip on screen. It reads the shared instance
    // itself — the one thing that can also say the words on it.
    Timer {
        id: pathTipTimer
        property bool wipSide: true
        interval: 25
        repeat: true
        onTriggered: {
            if (pathTipTimer.wipSide && worktreeModel.total === 0)
                return
            if (!pathTipTimer.wipSide && detailsModel.shaHex !== page.selectedOid)
                return
            pathTipTimer.stop()
            if (pathTipTimer.wipSide)
                wipPane.pointedTipRow = 0
            else
                detailsPane.pointedTipRow = 0
            pathTipReport.start()
        }
    }
    Timer {
        id: pathTipReport
        interval: 25
        repeat: true
        onTriggered: {
            const tip = page.ToolTip.toolTip
            if (!tip.visible)
                return
            pathTipReport.stop()
            AppBackend.report("path_tip pane="
                + (pathTipTimer.wipSide ? "wip" : "details")
                + " tree=" + (pathTipTimer.wipSide
                              ? worktreeModel.treeView
                              : detailsModel.treeView)
                + " tip=" + tip.visible
                + " text=" + tip.text)
            driver.complete()
        }
    }
    // Automation: the details have to land before the author card can be
    // worked, since it is that author the picture is filed against.
    Timer {
        id: avatarAssignTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (detailsModel.authorEmail === "")
                return
            avatarAssignTimer.stop()
            AppBackend.assignAvatar(detailsModel.authorEmail,
                                    detailsModel.authorName,
                                    AppBackend.autoActArg)
            avatarReportTimer.start()
        }
    }
    Timer {
        id: avatarBadgeTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (detailsModel.shaHex !== page.selectedOid)
                return
            avatarBadgeTimer.stop()
            detailsPane.avatarClicked()
            renderedBarrier.begin()
        }
    }
    // What the graph did about the find bar, read after it finished doing
    // it. The step down out from under the card is animated, so the value
    // in the same call stack as the verb is always the one before it
    // moved — reporting that would be reporting the intent, which the
    // line above already carries as `clears=`.
    Timer {
        id: findSettled
        interval: 25
        repeat: true
        onTriggered: {
            if (!graphPane.findOpen)
                return
            findSettled.stop()
            AppBackend.report(
            "find_settled shift=" + Math.round(graphPane.findShift))
            driver.complete()
        }
    }
    // The store starts empty in every run, so the card's own verbs put a
    // picture in it before opening on it.
    Timer {
        id: avatarSeedTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (detailsModel.authorEmail === "")
                return
            avatarSeedTimer.stop()
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
        interval: 25
        repeat: true
        onTriggered: {
            if (detailsModel.avatarUrl === "" && AppBackend.avatarError === "")
                return
            avatarReportTimer.stop()
            AppBackend.report(
            "avatar email=" + detailsModel.authorEmail
            + " details=" + (detailsModel.avatarUrl !== "")
            + " rows=" + graphModel.avatarRowCount()
            + " error=" + AppBackend.avatarError)
            driver.complete()
        }
    }
    // The card is opened synchronously; this just lets the layout settle
    // before it is measured and photographed.
    Timer {
        id: rowCardTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (!rowCard.opened && !refList.opened)
                return
            rowCardTimer.stop()
            AppBackend.report(
            "row_card open=" + rowCard.opened
            + " credit=" + Math.round(rowCard.creditWidth)
            + " cut=" + rowCard.creditCut
            + " list=" + refList.opened
            + " subject=" + (rowCard.subject !== "")
            + " body=" + (rowCard.body !== ""))
            driver.complete()
        }
    }
    // The details have to arrive before the name can name anybody.
    Timer {
        id: authorCardTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (detailsModel.shaHex !== page.selectedOid)
                return
            if (AppBackend.autoAct === "author-card-open")
                detailsPane.showAuthor(true)
            if (AppBackend.autoAct === "author-card-open" && !detailsPane.authorCardOpen)
                return
            authorCardTimer.stop()
            AppBackend.report(
                "author_card open=" + detailsPane.authorCardOpen
                + " author=" + detailsPane.details.authorEmail
                + " committer=" + detailsPane.details.committerEmail
                + " other=" + detailsPane.details.committerDiffers
                + " later=" + detailsPane.details.commitTimeDiffers)
            driver.complete()
        }
    }
    // The details have to arrive before the credit line they carry can
    // be opened or counted.
    Timer {
        id: coAuthorTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (detailsModel.shaHex !== page.selectedOid)
                return
            if (AppBackend.autoAct === "co-authors-open")
                detailsPane.showCoAuthors(true)
            if (AppBackend.autoAct === "co-authors-open" && !detailsPane.matesCardOpen)
                return
            coAuthorTimer.stop()
            // `open` is the card's own visibility, not the input that
            // asked for it: reporting the input would go green with the
            // binding cut.
            AppBackend.report(
                "co_authors count=" + detailsPane.coAuthorRecords.length
                + " first=" + detailsPane.coAuthorName(0)
                + " open=" + detailsPane.matesCardOpen)
            driver.complete()
        }
    }
    // The details have to arrive, and the column has to be laid out with
    // them, before there is anything to measure.
    Timer {
        id: detailsFitTimer
        interval: 25
        repeat: true
        // A pane width the splitter left on a fraction can put a fraction
        // in the answer; what this verb is about is tens of pixels.
        onTriggered: {
            if (detailsModel.shaHex !== page.selectedOid || detailsPane.width <= 0
                    || detailsPane.height <= 0)
                return
            detailsFitTimer.stop()
            AppBackend.report(
            "details_fit fits=" + (detailsPane.contentOverflow < 1)
            + " over=" + Math.round(detailsPane.contentOverflow)
            + " pane=" + Math.round(detailsPane.width)
            // The other axis rides along unjudged, the way `edge=` does in
            // `window_fill`: how far the column runs past the pane's own
            // bottom is what says whether this pane needs a scroll of its
            // own, and the answer depends on the window, not on this verb.
            + " overH=" + Math.round(detailsPane.contentOverHeight)
            + " paneH=" + Math.round(detailsPane.height))
            driver.complete()
        }
    }
    // The rows have to arrive, and the list be laid out with them, before
    // what they leave bare is worth measuring.
    Timer {
        id: cornerTimer
        interval: 25
        repeat: true
        // `shown=` is the label's own visibility, not the room that
        // decided it: reporting what was asked for would go green with
        // the binding cut.
        onTriggered: {
            if (gitCorner.parent === null || gitCorner.width <= 0)
                return
            cornerTimer.stop()
            AppBackend.report(
            "git_corner pane=" + (page.wipShown ? "wip" : "details")
            + " shown=" + gitCorner.visible
            + " room=" + Math.round(gitCorner.roomLeft)
            + " needs=" + Math.round(gitCorner.roomNeeded))
            driver.complete()
        }
    }
    // Same wait as details-fit, for the same reason: the message has to
    // be in the box, and the box laid out with it, before there is a
    // ceiling to pull on.
    Timer {
        id: descGrowTimer
        interval: 25
        repeat: true
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
        property int frameBefore: 0
        /// Which end this run is carrying the grip past, or empty for the
        /// ordinary pull. Same wait and same box — the difference is that
        /// the grip is in hand, so the box answers instead of just
        /// stopping (規約 §掴める境界は答える).
        property string refuse: ""
        onTriggered: {
            if (descGrowTimer.pane.width <= 0 || descGrowTimer.pane.height <= 0
                    || descGrowTimer.pane.descCap <= 0)
                return
            descGrowTimer.stop()
            descGrowTimer.frameBefore = page.Window.window.frameCounter
            if (descGrowTimer.refuse !== "") {
                descGrowTimer.pane.pullDescriptionPast(
                    descGrowTimer.refuse === "desc-max")
                descGrowSettle.start()
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
        interval: 25
        repeat: true
        onTriggered: {
            if (page.Window.window.frameCounter <= descGrowTimer.frameBefore)
                return
            if (descGrowTimer.refuse !== "" && !page.refusalShown)
                return
            descGrowSettle.stop()
            if (descGrowTimer.refuse !== "") {
                AppBackend.report("divider_refuse refuses=" + page.refusalShown
                                  + " line=" + descGrowTimer.pane.descGrips
                                  + " case=" + descGrowTimer.refuse
                                  + " box=" + Math.round(descGrowTimer.pane.descHeight)
                                  + " wants=" + Math.round(descGrowTimer.pane.descWants))
                driver.complete()
                return
            }
            AppBackend.report(
            "description_grow keeps=" + descGrowTimer.pane.descKeeps
            + " pane=" + descGrowTimer.paneName
            + " grip=" + descGrowTimer.pane.descGrips
            + " box=" + Math.round(descGrowTimer.pane.descHeight)
            + " wants=" + Math.round(descGrowTimer.pane.descWants)
            + " cap=" + Math.round(descGrowTimer.pane.descCap)
            + " rows=" + descGrowTimer.pane.descListRows)
            driver.complete()
        }
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
        driver.prepareCompletion(act)
        if (act === "publish" || act === "publish-taken"
                || act === "publish-add" || act === "publish-go"
                || act === "publish-new-go" || act === "publish-remotes") {
            // The button's own path, so the state machine in front of the
            // question is exercised too, not just the question.
            page.pushNow()
            if (act === "publish-taken")
                publishFlow.setPublishBranch(arg === "" ? "taken" : arg)
            else if (act === "publish-add" || act === "publish-new-go")
                publishFlow.startPublishAddRemote(arg)
            else if (arg !== "")
                publishFlow.setPublishBranch(arg)
            if (act === "publish-new-go")
                publishNewTimer.start()
            else if (act === "publish-go")
                publishAnswerTimer.start()
            else if (act === "publish-remotes")
                publishRemotesTimer.start()
            else if (act === "publish-add")
                publishDialogTimer.start()
            else if (act === "publish")
                publishSurfaceTimer.start()
            else
                publishSettleTimer.start()
            // `dialog=` / `name=` say whether the remote dialog stands and
            // what its name box holds — the no-remote push opens it by
            // itself, and only this line can say so headless.
            AppBackend.report("publish state=" + page.pushState
                              + " remote=" + publishFlow.publishRemote
                              + " branch=" + publishFlow.publishBranch
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
        } else if (act === "stage-all" || act === "unstage-all") {
            page.showWip()
            bucketAllTimer.begin(act === "stage-all" ? "unstaged" : "staged")
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
            page.switchToRef("R", arg)
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
            if (act === "rename-remote-box") {
                renderedBarrier.begin()
                return
            }
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
            AppBackend.report("row_menu stash=" + commitMenuState.menuStashRef)
            if (act === "delete-stash-row" && arg === "go")
                stashDeleteItem.completeHold()
        } else if (act === "stash-apply-row" || act === "stash-pop-row") {
            page.openRowMenu(stashesModel.oidOfName(stashesModel.nameAt(0)))
            if (act === "stash-apply-row")
                repoTab.applyStash(commitMenuState.menuStashRef)
            else
                repoTab.popStash(commitMenuState.menuStashRef)
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
        } else if (act === "changes-step" || act === "changes-step-edge"
                   || act === "wip-step") {
            // The file list's arrows: one file per press, the light and the
            // diff moving together (規約 §diff のファイル一覧). `-edge` walks
            // further than the list is long, so the last presses are refused
            // and it stops rather than wrapping. The argument is the file to
            // start on — `<bucket>:<path>` for the working tree's list, where
            // a file changed on both sides has a row under each.
            if (act === "wip-step") {
                const cut = arg.indexOf(":")
                const head = cut > 0 ? arg.substring(0, cut) : ""
                const named = head === "staged" || head === "unstaged"
                              || head === "untracked" || head === "conflicts"
                page.showWip()
                fileStepTimer.pane = "wip"
                fileStepTimer.bucket = named ? head : "unstaged"
                fileStepTimer.path = named ? arg.substring(cut + 1) : arg
            } else {
                page.activateRow(branchesModel.headOid !== ""
                                 ? branchesModel.headOid : graphModel.oidAt(0))
                fileStepTimer.pane = "changes"
                fileStepTimer.bucket = ""
                fileStepTimer.path = arg
            }
            fileStepTimer.overrun = act === "changes-step-edge"
            fileStepTimer.begin()
        } else if (act === "diff-step" || act === "diff-step-edge") {
            // Moves the view, not a selection (規約 §diff を上下に送る).
            // Rides the 320x240 seed: no demo file's diff is longer than
            // a default window, and even there the room below the fold is
            // two rows (実測) — which is why the plain walk is one row.
            page.showWip()
            page.toggleDiff("untracked", arg, "")
            // The hand walks into the pane, through the same door the wheel
            // comes in by: the diff does not take the keyboard by appearing
            // (規約 §diff のファイル一覧), so without this the arrows are
            // still the file list's.
            diffPane.handArrived()
            diffStepTimer.steps = act === "diff-step-edge" ? 20 : 1
            diffStepTimer.start()
        } else if (act === "name-box") {
            graphPane.startNaming(graphModel.oidAt(Number(arg)))
        } else if (act === "graph-tail") {
            graphTailTimer.start()
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
                renderedBarrier.begin()
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
                              + " can_move=" + commitMenuState.menuCanMoveBranch)
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
                              + " oid=" + commitMenuState.menuOid.substring(0, 8)
                              + " hold=" + (dropCommitItem.holdMs > 0)
                              + " reached=" + repoTab.headReachedElsewhere)
            if (act === "drop-commit-go") {
                if (dropCommitItem.holdMs > 0)
                    dropCommitItem.completeHold()
                else
                    page.dropCommit(commitMenuState.menuOid)
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
                   || act === "hunk-tools" || act === "keep-place"
                   || act === "code-send" || act === "line-back"
                   || act === "diff-follow" || act === "line-run") {
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
        } else if (act === "settings" || act === "settings-tools"
                   || act === "settings-tools-loading") {
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
        driver.dispatchFinished()
    }

    /// The first-push surface is either the standing question (a remote
    /// exists) or the add-remote dialog (none does). Do not wait for a
    /// remote check in the latter case: there is no target to check yet.
    Timer {
        id: publishSurfaceTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (!remoteDialog.visible && !publishFlow.publishChecked)
                return
            publishSurfaceTimer.stop()
            driver.complete()
        }
    }
    /// `publish-remotes` is about the popup, not merely the call which
    /// requested it. The form is created asynchronously with the ask bar.
    Timer {
        id: publishRemotesTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (!publishFlow.publishRemotesOpen()) {
                publishFlow.openPublishRemotes()
                return
            }
            publishRemotesTimer.stop()
            driver.complete()
        }
    }
    /// `publish-add` stops with the real dialog on screen. A check that
    /// happens to finish behind it is unrelated and must not end the run.
    Timer {
        id: publishDialogTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (!remoteDialog.visible)
                return
            publishDialogTimer.stop()
            driver.complete()
        }
    }
    /// Automation: the dialog's own button, once it is both visible and
    /// valid. This is the `-go` path; an empty URL cannot be submitted.
    Timer {
        id: publishNewTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (!remoteDialog.visible || remoteDialog.wantedName === ""
                    || remoteDialog.wantedUrl === "")
                return
            publishNewTimer.stop()
            driver.writeSeqBefore = repoTab.writeSeq
            remoteDialog.submit()
            publishAnswerTimer.start()
        }
    }
    /// Automation: what the far side turned out to hold, once the remote
    /// has had time to answer.
    Timer {
        id: publishSettleTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (!publishFlow.publishChecked)
                return
            publishSettleTimer.stop()
            AppBackend.report("publish settled far="
                                       + publishFlow.publishState
                                       + " code=" + graphPane.askCode
                                       + " hold=" + graphPane.askHold
                                       + " alert=" + graphPane.askAlert
                                       + " lease=" + (publishFlow.publishLease !== "")
                                       + " theirs=" + repoTab.remoteBranchTheirs)
            driver.complete()
        }
    }
    /// Automation: the answer, given after the remote has had time to say
    /// what it has — the pill is dead until it has.
    Timer {
        id: publishAnswerTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (!publishFlow.publishChecked || !graphPane.askAnswerable)
                return
            publishAnswerTimer.stop()
            // `far` is what the far side turned out to hold — the other
            // line's `state` is this end's own push state, and the two
            // answer different questions.
            AppBackend.report("publish answering far="
                              + publishFlow.publishState
                              + " unsure=" + publishFlow.publishUnsure
                              + " answerable=" + graphPane.askAnswerable)
            // The same gesture a person is given: a hold cannot be
            // answered by a click here either.
            driver.writeSeqBefore = repoTab.writeSeq
            if (publishFlow.publishRefused)
                graphPane.completeHold()
            else
                page.answerRowAsk()
            writeBarrier.start()
        }
    }
    // The log has to be on screen and laid out before the bar above it has
    // a place to be measured from.
    Timer {
        id: splitRefuseTimer
        interval: 25
        repeat: true
        onTriggered: {
            if (!page.commandsOpen || !page.commandsShown)
                return
            splitRefuseTimer.stop()
            page.reportDividerRefusal(AppBackend.autoActArg)
            renderedBarrier.begin()
        }
    }
}
