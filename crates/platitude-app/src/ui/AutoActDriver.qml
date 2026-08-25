pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

/// PG_AUTO_ACT runs one operation — a write, or a surface left standing for the overlay shot — through exactly the code
/// path a click takes, so the wiring can be proven headlessly. The dispatch is equality on a bare verb; the argument
/// passes through as whatever the verb needs (a name, an oid, a row number).
///
/// `RepoPage` builds this only when a verb was given, so an ordinary run carries none of it. What the verbs act on is
/// handed in below: a file of its own cannot see the page's ids, and naming them in one list is what says how far the
/// harness reaches into the page.
// An `Item` only because `QtObject` has no default property to hold the timers below; it draws nothing and is never
// given a size.
Item {
    id: driver

    /// The page these verbs act on, and the parts of it they read back or leave standing for the shot. An
    /// automation-only exposure, the same one `GraphPane.view` is (app-ui.md).
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
    /// The two cards the ref menu's rows hang behind. A verb that photographs one of those rows opens its card first
    /// (`driver.showRefCard`) — the row is reachable either way, but the picture is of the card the reader would see.
    property AppMenu refBranchCard
    property AppMenu refTagCard
    property AppMenuItem refDeleteItem
    property AppMenuItem refUpstreamItem
    property AppMenuItem refStashDropItem
    property AppMenuItem refSwitchItem
    property AppMenuItem refPushTagItem
    property AppMenuItem refTagHereItem
    property AppMenuItem refTagDeleteItem
    property AppMenuItem refRemoteTagDeleteItem
    property AppMenuItem refTagBothDeleteItem
    property FileRowMenu fileRowMenu
    property AppMenu fileMenu
    property AppMenuItem fileDiscardItem
    /// What the commit menu is standing on, read where a verb has to say which row it opened on and what was offered
    /// there.
    property CommitMenuState commitMenuState
    property AppMenu commitMenu
    property AppMenuItem dropCommitItem
    property AppMenuItem tagHereCommitItem
    property AppMenuItem stashDeleteItem
    property AppMenu resetMenu
    property AppMenu commitBranchCard
    property AppMenu commitTagCard
    property AppMenuItem hardResetItem
    property PublishFlow publishFlow
    property UpstreamFlow upstreamFlow
    property RemoteDialog remoteDialog
    property AppMenu remoteMenu
    /// The remote the push-default verbs act on, and the mark they wait for before they photograph anything — empty
    /// where the run is not asking for one to move.
    property string remoteTarget: ""
    property string markWanted: ""
    property RefListPopup refList
    property CommitHoverCard rowCard

    /// The details card has caught up with a real selection — the one readiness every card-reading sampler waits on.
    /// Named once so no copy can drop the empty-selection half: the bare `!==` comparison is vacuously satisfied while
    /// nothing is selected, and a sampler that copies it without a prior selection guard photographs a stale card.
    readonly property bool cardSettled: page.selectedOid !== "" && detailsModel.shaHex === page.selectedOid

    /// Kicked off by the page once its models are attached: a verb that ran before them would act on a repository
    /// nothing has read yet.
    property bool claimed: false
    function begin() {
        autoActTimer.start()
    }

    // Completion belongs to the page that claimed the run. A rendered surface is enough for a read-only, synchronous
    // verb. A write is different: seeing its request leave this item says nothing about the repository, so retain the
    // busy edge and the write answer as a causal barrier before handing the shot driver a completed scene.
    property bool completionDeferred: false
    property bool writeExpected: false
    /// The write counter as it stood immediately before the request went out, so that its moving is proof this run's
    /// own write answered.
    ///
    /// **That is the whole of the proof.** Waiting to *see* `busyCount` rise as well wedges on a write that begins and
    /// ends between two looks at it — which the container did and the host did not, and which taking work out of the
    /// post-write refresh made likelier still (2026-08-17 実測: `line-back`, then `keep-place`).
    property int writeSeqBefore: 0
    /// The working-tree row this run's write takes out of its bucket, as `<bucket>:<path>` — or "" for the verbs the
    /// write barrier alone answers for.
    ///
    /// A write answers before the status it invalidated has been read again: core reports `WriteFinished` and *then*
    /// publishes status and refs (`session::write::run_write`), and `writeSeq` is counted off that report
    /// (`drain::settle_write`). So a shot taken at the write barrier is a shot of the file list as it was — the same
    /// shape `graphGoneOid` below answers for on the graph. That is how a run whose merge editor resolved a file and a
    /// run whose editor never started came to save the same picture, down to the sha256 (2026-08-23 ユーザー報告).
    ///
    /// Waited out on the row itself rather than on a status counter, for the reason the graph gives: a counter also
    /// moves for statuses nobody here asked for, while this row moves only for this write.
    property string treeGoneRow: ""
    /// The graph row this run's write takes off the graph, or "" for the verbs the write barrier alone answers for.
    ///
    /// A write answers before the rebuild it asks for is even started (core's `AfterWrite::Graph`), so a shot taken at
    /// the write barrier is a shot of the graph as it was. That is how two rows wearing the wrong mark went out under
    /// green runs: a stash popped off the top left its archive box on the working-tree row, and a stash just made wore
    /// the working tree's dashed ring (2026-08-21 ユーザー報告). Waiting on the row itself — gone from the model —
    /// rather than on a pass counter keeps the wait about this write: the counter also moves for passes nobody here
    /// asked for.
    property string graphGoneOid: ""
    /// How many entries the stash list held before that write. The list is read after the rebuild rather than with it,
    /// so a shot taken the moment the graph settles frames a sidebar still counting the old entries — which is not a
    /// state the application ever rests in, and the run is judged by eye.
    property int stashTotalBefore: -1
    /// The summary this run put in the commit box before the write, or "" — what the entry the write makes has to be
    /// carrying when the list settles (`named=`). Held here rather than read back off the box, because the box is not
    /// what the claim is about: the name has to have reached git.
    property string stashWanted: ""
    /// The name the entry this run pops was carrying, or "" — what the commit box has to be holding once the pop has
    /// landed (`back=`). Read before the press, because the entry is gone by the time the answer is.
    property string popWanted: ""
    /// What HEAD was before that write, for the run whose picture is of the commit that replaces it. The same
    /// `AfterWrite::Graph` ordering applies: at the write barrier the panes still frame the commit that was replaced,
    /// wearing the author it was replaced for — which is the whole subject of `amend-reset-author`.
    property string headOidBefore: ""
    /// The acts that name rows of the WIP lists, and so run only once those rows are walkable
    /// (`fileRowsTimer` holds them until they are).
    readonly property var fileRowActs: [
        "stage-many", "stage-many-go", "discard-many", "discard-many-go",
        "file-menu", "file-menu-untracked", "file-menu-staged", "file-menu-conflict",
        "take-side-ours", "take-side-theirs", "open-mergetool",
        "discard-file", "discard-file-go", "delete-file", "delete-file-go",
        "discard-staged", "discard-staged-go"
    ]

    function isWriteAct(act) {
        return ["publish", "publish-taken", "publish-add", "publish-go",
                "publish-new-go", "commit", "amend", "amend-reset-author",
                "stash", "stash-lands", "stash-file", "stage-many-go",
                "discard-many-go", "take-side-ours", "take-side-theirs",
                "open-mergetool", "discard-file-go", "delete-file-go",
                "discard-staged-go", "switch", "switch-remote", "nav-dbl",
                "rename-branch", "rename-tag", "rename-stash",
                "rename-remote-go", "rename-local-upstream", "delete-branch",
                "delete-branch-go", "delete-tag-go", "delete-stash-go",
                "delete-remote-go", "delete-force", "delete-branch-refused",
                "set-upstream-go",
                "delete-stash-row", "stash-apply-row", "stash-pop-row",
                "branch-at-tag", "dbl-local", "dbl-remote", "ref-list-pick", "graph-rename", "move-branch",
                "name-branch", "squash", "reword", "cherry-pick", "reset-soft",
                "reset-mixed", "reset-hard", "drop-commit-go", "merge-branch",
                "merge-stops", "cherry-pick-stops", "revert-stops", "rebase-stops", "drop-stops",
                "rebase-onto", "revert-commit", "op-exit-go", "stage-hunk",
                "stage-line", "keep-place", "discard-hunk-go", "line-back", "diff-follow",
                "line-run",
                "stage-all", "unstage-all", "resolve-all",
                "push", "force-push", "push-retry", "fetch", "fetch-ref-list",
                "commands", "commands-fail", "commands-clear", "fetch-recover",
                "fetch-fail", "fetch-resume"].indexOf(act) >= 0
            // A double-click on a tag writes nothing: it opens the box for a name instead (デザイン規約 §左メニューの所作),
            // and a run held at the write barrier for one waits out the watchdog in silence.
            && !(act === "nav-dbl" && AppBackend.autoActArg.startsWith("tag:"))
    }

    function defersCompletion(act) {
        return ["publish", "publish-taken", "publish-remotes", "publish-add",
                "publish-go", "publish-new-go", "amend-reset-author", "amend-author",
                "eol-commit", "eol-hover", "commit-face",
                "stage-hunk", "stage-line", "discard-hunk", "discard-hunk-go",
                "diff-file", "conflict-sides", "diff-tick", "line-tools", "hunk-tools",
                "preview", "preview-unstaged", "preview-staged",
                // The write barrier is behind these, not in front of them: five land on the working tree's own
                // row, which the graph pass after the write is what puts there, and the last has to read the
                // commit it just made.
                "merge-stops", "cherry-pick-stops", "revert-stops", "rebase-stops", "drop-stops",
                "merge-commit",
                "code-send", "line-back", "diff-follow", "line-run",
                "stage-all", "unstage-all", "resolve-all",
                "keep-place", "colour-place", "delete-branch-go", "nav-fold",
                "nav-peek", "nav-unfold", "nav-peek-rename", "nav-peek-away",
                "nav-peek-into", "nav-peek-out", "nav-peek-shut", "nav-close",
                "nav-filter", "nav-tip", "nav-reclick", "nav-reclick-away", "nav-rename-far",
                "nav-branch-box", "nav-rename-box", "nav-tag-box",
                // The write barrier is behind this one: the row it makes is put in the sidebar by the read that
                // follows the write, and the write answers first.
                "create-tag",
                // Both wait for the readings that decide the push row's shape, and the second runs its press from
                // there — so the barrier is behind the wait rather than in front of it.
                "tag-menu", "push-tag", "delete-remote-tag", "delete-tag-both",
                "nav-add-remote", "push-default", "remote-menu", "remote-url",
                "publish-remotes-marked", "tags-eye",
                "delete-branch-refused", "chip-menu", "chip-menu-current",
                "delete-blocked-tip", "switch-stopped", "switch-lands",
                // Stops at its question, so the ask bar settling is the completion — a write
                // never comes (the write half is "-go", which stays a write act above).
                "rename-remote", "set-upstream",
                // The write barrier is behind this one: the question is answered from the timer, not before it.
                "set-upstream-go",
                "move-ask", "switch-conflicted", "switch-held", "switch-mark",
                // The write barrier is behind these, not in front of them: the commands that clear the way and the
                // move they carry only start once the question standing in the graph has been answered.
                "switch-stopped-go", "switch-conflicted-go", "delete-branch-early",
                // Deliberately not a write act: what it photographs is the moment before the answer.
                "delete-gone",
                "ref-list-card", "row-part", "graph-reclick", "graph-reclick-list",
                "graph-reclick-scrolled", "graph-reclick-across", "graph-reclick-mark",
                "graph-reclick-still", "rename-box-out",
                "signature", "signature-tip", "stash-tip", "path-tip", "row-card", "menu-hover",
                "author-card", "author-card-open", "co-authors", "co-authors-open",
                "details-grow", "details-grow-squeeze", "wip-grow", "wip-grow-squeeze",
                "details-fit", "corner", "graph-step", "graph-step-edge", "graph-step-far",
                "graph-step-named", "graph-step-dirty", "graph-step-diff", "graph-step-hold",
                "diff-step",
                "diff-step-edge", "changes-step", "changes-step-edge", "wip-step",
                "changes-fold", "changes-unfold",
                "graph-bar", "graph-bar-away", "middle-scroll",
                "graph-tail", "graph-head", "graph-head-below", "graph-head-back",
                "graph-head-go", "graph-head-lit", "wip-lanes",
                "divider-refuse", "commands-fail-shut",
                "cherry-pick", "merge-branch", "revert-commit", "reword", "edit-message",
                "edit-message-leave", "edit-message-focus",
                "push-retry", "fetch-ref-list", "avatar-assign", "avatar-badge",
                "find", "find-next", "find-prev", "find-drop",
                // These flows are completed by Main/WindowAutoActDriver. Some still begin here (picker, command
                // failure, recovery), but the page must never photograph their intermediate state before the
                // window-level predicate has answered.
                "open-fetches",
                "open-picker", "commands-clear", "fetch-recover",
                "open-not-a-repo", "open-bare", "open-not-a-repo-retry",
                "open-not-a-repo-cancel", "open-fail-tab",
                "open-fail-tab-bare", "open-fail-tab-log", "identity",
                "identity-half", "identity-tip", "band", "app-menu", "app-menu-reclick", "tab-widths",
                "tab-mark", "tab-drag", "tab-hold", "tab-edge",
                "window-fill", "solo", "window-floor",
                "badges", "badges-hover", "band-actions", "band-actions-none",
                "band-actions-fold", "band-actions-alert", "old-git", "old-git-card",
                "old-git-fold", "state", "middle-close", "open-again",
                "force-push-hold", "fetch-busy", "fetch-fail", "fetch-resume", "fetch-tip",
                "stash-state",
                "settings-tools", "settings-tools-loading",
                "avatar-settings", "avatar-combo", "avatar-row-lit", "avatar-remove"].indexOf(act) >= 0
    }

    function prepareCompletion(act) {
        driver.completionDeferred = driver.defersCompletion(act)
        driver.writeExpected = driver.isWriteAct(act)
        driver.writeSeqBefore = repoTab.writeSeq
        driver.treeGoneRow = ""
        driver.graphGoneOid = ""
        driver.stashTotalBefore = stashesModel.total
        driver.stashWanted = ""
        driver.popWanted = ""
    }

    /// Whether the entry now at the top of the list is wearing the summary this run typed. Read off the sidebar's own
    /// model — the reflog subject git wrote — so a name that never left the box answers `false`. git puts its own
    /// `On <branch>: ` in front of a named entry (実測), which is why this is a tail and not an equality.
    function stashNamed() {
        return driver.stashWanted !== "" && stashesModel.nameAt(0).endsWith(driver.stashWanted)
    }

    /// Whether the entry this run popped left its name in the commit box. The box is read, not the property that was
    /// put there: the claim is about what a reader would find typed in front of them.
    function stashCameBack() {
        return driver.popWanted !== "" && wipPane.subjectText === driver.popWanted
    }

    /// What the graph's leading row is, for the runs that are about the mark it wears.
    function graphTopKind() {
        const oid = graphModel.oidAt(0)
        if (oid === "")
            return "none"
        // The same reading the row delegate makes of the synthetic working-tree row: an oid of nothing but zeroes.
        if (!/[^0]/.test(oid))
            return "wip"
        return graphModel.stashRefOf(oid) !== "" ? "stash" : "commit"
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

    /// How far down a diff the reader is taken before the thing that could cost them their place happens — the rebuild
    /// a partial write asks for (`keep-place`), and the swap the colours arrive in (`colour-place`). One number for
    /// both, because both are judged on getting exactly it back, and a place nobody can name is not one either of them
    /// can be caught losing.
    readonly property real readY: 400
    function reportPlace(at, room) {
        AppBackend.report("diff_place at=" + Math.round(at)
                          + " want=" + Math.round(driver.readY)
                          + " room=" + Math.round(room))
    }

    // AutoShotDriver owns the final render boundary: it requests an update, advances the event loop, and waits for
    // grabToImage callbacks. Do not wait for frameSwapped here. A quiet scene is allowed not to emit one (the pilot
    // reproduced that hang twice under concurrent load).
    QtObject {
        id: renderedBarrier
        function begin() {
            driver.complete()
        }
    }
    // A write has two separate causal edges. `busyCount` proves the process was actually admitted, and `writeSeq`
    // proves its answer was absorbed. Both must precede the final rendered state.
    SampleTimer {
        id: writeBarrier
        onTriggered: {
            if (repoTab.busyCount !== 0 || repoTab.writeSeq <= driver.writeSeqBefore)
                return
            writeBarrier.stop()
            if (driver.treeGoneRow === "")
                driver.afterTreeSettled()
            else
                treeBarrier.start()
        }
    }
    /// What the chain does once the working tree has answered — or straight away, for the verbs that name no row of
    /// it: the graph rebuild for the verbs whose write takes a row off the graph, and the shot for everyone else.
    function afterTreeSettled() {
        if (driver.graphGoneOid === "")
            renderedBarrier.begin()
        else
            graphBarrier.start()
    }
    // The status that follows a write, read off the file list rather than off the clock: the row the write moves is
    // still in the model until the new status lands, so its leaving the bucket it was named in is the edge (see
    // `treeGoneRow`). A write git refused leaves the row where it was and the run to the watchdog, which is the
    // diagnosis — the same bargain every barrier here makes.
    SampleTimer {
        id: treeBarrier
        onTriggered: {
            const cut = driver.treeGoneRow.indexOf(":")
            const from = driver.treeGoneRow.substring(0, cut)
            const path = driver.treeGoneRow.substring(cut + 1)
            if (worktreeModel.holdsPath(from, path))
                return
            treeBarrier.stop()
            // Where the row went is the other half of the claim: a file resolved into the index and a file discarded
            // out of the tree both leave the bucket they were named in, and the two are told apart by what holds them
            // afterwards ("" being nothing at all).
            AppBackend.report("tree_settled from=" + from
                              + " landed=" + worktreeModel.bucketOf(path)
                              + " conflicts=" + workTree.conflictCount
                              + " staged=" + workTree.stagedCount
                              + " unstaged=" + (workTree.unstagedCount + workTree.untrackedCount))
            driver.afterTreeSettled()
        }
    }
    // The rebuild that follows a write, read off the graph rather than off the clock: the row the write took away is
    // still in the model until the rebuilt one lands, so its absence is the edge — and the marks the rows wear are
    // only right once that has happened (see `graphGoneOid`).
    SampleTimer {
        id: graphBarrier
        onTriggered: {
            if (graphModel.rowOf(driver.graphGoneOid) >= 0
                    || stashesModel.total === driver.stashTotalBefore)
                return
            graphBarrier.stop()
            AppBackend.report("graph_settled gone=true top=" + driver.graphTopKind()
                              + " named=" + driver.stashNamed()
                              + " back=" + driver.stashCameBack()
                              + " rows=" + graphModel.rowTotal
                              + " stashes=" + stashesModel.total)
            if (AppBackend.autoAct === "stash-lands")
                stashLandTimer.start()
            else
                renderedBarrier.begin()
        }
    }
    // Where the press left the reader, once the graph the row went out of has settled. The selection is the whole
    // subject, so it is waited for on the far side of the rebuild and read the way the reader would: the pane that was
    // describing the working tree is gone, the commit under it is the one the branch points at, the details pane is
    // showing that commit rather than the one before it, and the row is on screen.
    //
    // **The row being lit is not enough** — a highlight left on an index the working-tree row vacated lights whatever
    // slid into it, and in this run that is the entry the press just made. `follows=` is the identity the picture
    // cannot hold: two rows a couple of lines apart look alike at this width.
    //
    // **Waited out on the pane, not on the landing**, so a build that never lands still answers: the tree is empty and
    // whatever is on the right has caught up with the page — the working tree's own pane, which needs nothing fetched,
    // or a commit whose details have arrived. Both are states the application rests in, and the run says which one it
    // reached rather than waiting out its watchdog on the wrong one.
    SampleTimer {
        id: stashLandTimer
        onTriggered: {
            if (worktreeModel.total !== 0)
                return
            if (!page.wipShown && !driver.cardSettled)
                return
            stashLandTimer.stop()
            const row = graphModel.rowOf(branchesModel.headOid)
            AppBackend.report("stash_landed wip=" + page.wipShown
                              + " follows=" + (page.selectedOid === branchesModel.headOid)
                              + " onscreen=" + graphPane.rowOnScreen(row)
                              + " lit=" + (graphPane.view.currentIndex === row)
                              + " head=" + branchesModel.headOid.substring(0, 8)
                              + " selected=" + page.selectedOid.substring(0, 8)
                              + " row=" + row + " rows=" + graphModel.rowTotal
                              // What the entry ended up called, last because it is the one field with spaces in it.
                              // The other half of the same press: a box filled by the merge that was standing
                              // (`absorbOpMessage`) is not a name anybody gave these changes, and an entry wearing it
                              // would be promising a merge it does not hold (`WipPane.stashName`).
                              + " entry=" + stashesModel.nameAt(0))
            renderedBarrier.begin()
        }
    }
    // The lanes of the uncommitted row, in the tokens its delegate paints from. A lane is a stroke a couple of pixels
    // wide and its dashes are one pixel each, so which of them are dotted is not a question the photograph answers.
    // The row is put there by the pass behind the status read — the same read that says what a standing merge is
    // bringing in — so the wait is for the row itself to lead the graph, not for the tree to be loaded.
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

    SampleTimer {
        id: autoActTimer
        onTriggered: {
            // Tabs are constructed before their active index settles. The page that becomes current claims the one
            // process-wide verb; pages opened by that verb can never replay it.
            if (!driver.claimed) {
                if (!page.pageCurrent || !page.Window.window.claimAutoPageAct())
                    return
                driver.claimed = true
            }
            // `Opened` only means the path was accepted. Refs and the graph are the baseline every page verb is allowed
            // to act on.
            if (repoTab.state !== "open" || !workTree.loaded
                    || !branchesModel.refsLoaded || graphModel.finishCount === 0)
                return
            autoActTimer.stop()
            driver.runAutoAct()
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
    // amend was sent to take over (2026-08-21: `--preset authorship` photographed "Yuki Tanaka" on a green run).
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
    // The diff has to arrive before a row of it can be staged. Asked for rather than waited out: a fixed wait
    // photographs an empty pane the same as a late one (2026-08-13 実測: a verb fired against this repository named no
    // row and passed). The asking has no ceiling: a row that never lands leaves the run without a report line at all,
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
        // Whether what the verb is about to name is on screen. They all act on the first hunk, so a changed line in it
        // is the one answer they share — "diff-file" alone reads the model instead of a row, and the pictures and
        // binary files it also opens have no rows to find.
        function ready() {
            // "diff-file" alone reads the model instead of a row, and "conflict-sides" reads every row there is — the
            // one it is about (a side's own line, once it has been typed over) is a removal, which is not a changed
            // line of the first hunk. The previews take the settled form too: what they open can be all picture or
            // binary notice and no rows, and no row of it is theirs to name.
            if (["diff-file", "conflict-sides", "diff-tick",
                 "preview", "preview-unstaged", "preview-staged"].indexOf(AppBackend.autoAct) >= 0)
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
            // Which line the line-level verbs mean. Not 0: a hunk numbers its lines through the context it carries, and
            // the context is not part of the change (see `firstChangedLine`).
            const line = diffPane.firstChangedLine(0)
            // Said before the acting, so a verb that goes on to fail its write says both. `waited=` is ticks, not a
            // clock.
            AppBackend.report("diff_row act=" + act + " ready=" + arrived
                              + " rows=" + diffPane.view.count
                              + " line=" + line
                              + " waited=" + stageRowTimer.waited)
            // The diff is the shot; the line endings get a report line of their own (a picture cannot say which of the
            // four kinds the pane decided on).
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
            // Which rows the two sides are named on. The bands themselves are in the picture, but "how many rows
            // should have carried one" is not — and a resolved conflict is exactly where none of them did.
            if (act === "conflict-sides") {
                AppBackend.report("conflict_sides " + diffPane.sideTally())
                driver.complete()
                return
            }
            // The page's tick, fired here rather than waited for. What it asks about is a file this run has not
            // touched, so core answers it with nothing (`RepoSession::refresh_diff`) and there is no arrival to
            // observe — the ask is the edge, and it is read in the same beat it is made. `loading=` is the other half
            // of the claim: a tick must not put the pane back into the state a click does.
            if (act === "diff-tick") {
                const asked = page.pollDiff()
                AppBackend.report("diff_tick asked=" + asked
                                  + " loading=" + diffPane.diffModel.loading
                                  + " rows=" + diffPane.view.count)
                driver.complete()
                return
            }
            // A preview's settled form — rows, a picture, or a binary notice — is the shot; `kind=` is said because
            // the picture cannot say it (a pane the read never reached photographs as the same black under the same
            // DIFF header).
            if (act === "preview" || act === "preview-unstaged" || act === "preview-staged") {
                AppBackend.report("preview_pane kind=" + diffPane.diffModel.previewKind
                                  + " binary=" + diffPane.diffModel.isBinary)
                driver.complete()
                return
            }
            // The squares a line only puts out under the pointer, named rather than hovered (hover cannot be injected
            // on Windows).
            if (act === "line-tools") {
                diffPane.showLineTools(0, line)
                renderedBarrier.begin()
                return
            }
            // The heading's two words carry their colours only under the pointer, and hover cannot be injected, so the
            // row is named instead. A heading's own row is line -1 (`flatten_patches`).
            if (act === "hunk-tools") {
                diffPane.showLineTools(0, -1)
                renderedBarrier.begin()
                return
            }
            // Reading part way down a long diff and then writing: the rebuild has to come back to the same place.
            if (act === "keep-place") {
                keepPlaceTimer.begin(line)
                return
            }
            if (act === "stage-hunk" || act === "stage-line") {
                driver.writeSeqBefore = repoTab.writeSeq
                // One line goes through its own mark — the press writes, there and then — and a hunk through its
                // heading's word.
                if (act === "stage-line")
                    diffPane.stageLine(0, line)
                else
                    page.stageSelection(0, -1)
                writeBarrier.start()
                return
            }
            // Sending the code sideways, and the hand that sends it and the rows at once. Both read what moved rather
            // than what was asked for: a bar bound to nothing still takes a press.
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
            // No line-level discard exists — a hunk is the smallest piece that can be thrown away.
            if (act === "discard-hunk-go") {
                driver.writeSeqBefore = repoTab.writeSeq
                diffPane.completeHold()
                writeBarrier.start()
            } else {
                renderedBarrier.begin()
            }
        }
    }
    // One line staged from the diff, then the same file moved from the file list — the diff has to follow both, and
    // following only the first is the observed failure this verb pins (2026-08-17 ユーザー報告: the line was gone from
    // the unstaged side and never came back when the file was unstaged).
    //
    // Three answers in one run, because they are one story: the line goes (the rows shrink), the line comes back (the
    // rows are as they were), and staging the rest empties the side being read — where the pane follows the file to the
    // side it went to rather than closing on the reader (`RepoPage.followEmptySide`). Each step waits for its own write
    // to land *and* for the pane to say so — the rows and the key are the output, the write is only the cause.
    SampleTimer {
        id: lineBackTimer
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
        /// Whether the write this step asked for has landed. The sequence is read immediately before asking, so its
        /// moving is the whole of the evidence — waiting to *see* `busyCount` rise as well wedges on a write that
        /// begins and ends inside one tick, which is what the container did while the host did not (2026-08-17 実測:
        /// `line-back` PASS on Windows, watchdog on Linux).
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
                // The file list's own `−`, which is the half that was never reaching the pane.
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
            if (!lineBackTimer.wroteAndSettled() || page.diffKind !== "staged" || rows === 0)
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
    // Line after line, the way a hand does it. The pane refuses a press while the rows it would be written against are
    // still coming (`RepoPage.diffSettling`), so this waits for exactly that and no clock — which is also the thing
    // that broke: held on a signal the file list only sends when its rows differ, the pane went quiet for good at the
    // second line of a file already on both sides, and no `+` anywhere would go in again (2026-08-17 ユーザー報告).
    SampleTimer {
        id: lineRunTimer
        readonly property int want: 3
        property int done: 0
        /// How long the list has gone without a row to name, which is not the same as having none (see below).
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
            // A row is named by walking the list's own items, and the list builds them a frame after the model hands
            // the rows over: read too early it names nothing, which is not the same as there being nothing
            // (`keep-place` learned it too). So an empty answer is waited on — but not for ever, since a fixture with
            // fewer changed lines than this asks for is the run's own fault and has to show as one rather than as a
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
    // Where the reader lands when the file under the open diff is moved whole from the file list. Two answers, and
    // which one is right depends on what is left behind (デザイン規約 §diff の中のステージ): with other files still on that side the
    // pane takes the next of them, and with none left it stays on the same file and reads it from the side it went to.
    //
    // The argument names the file to open; the verb decides its own second step from what the tree holds afterwards,
    // and reports both.
    SampleTimer {
        id: followTimer
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
                // Whether this side has anything else on it, read before the write takes the file off it.
                followTimer.alone =
                    worktreeModel.besidePath(page.diffKind, page.diffPath) === ""
                // The sequence read here is the whole test below: seeing `busyCount` rise as well wedges on a write
                // that begins and ends inside one tick (see `lineBackTimer`).
                driver.writeSeqBefore = repoTab.writeSeq
                // The file list's own `+` / `−`, whole file at a time.
                if (page.diffKind === "staged")
                    repoTab.unstagePath(page.diffPath)
                else
                    repoTab.stagePath(page.diffPath)
                followTimer.step = 1
                return
            }
            // The landing is the output: the pane has to have moved off the key it was on and settled somewhere with
            // rows.
            const now = page.diffShown ? page.diffKind + ":" + page.diffPath : ""
            if (repoTab.busyCount !== 0 || repoTab.writeSeq <= driver.writeSeqBefore
                    || now === followTimer.was || (page.diffShown && diffPane.view.count === 0))
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
    // The band's Stash button, offered until it takes. The button is down while the tab is busy, and the fetch a
    // repository does on the way open outlives the baseline the verb starts on — so a single shot lands on nothing and
    // the run waits out its watchdog on a graph nobody asked to change. What follows the press is the graph barrier
    // (`graphGoneOid`), which is why nothing else is read here.
    //
    // **The row that has to go is read here rather than at the dispatch.** The walk prepends the working tree's row
    // only once the status says the tree is stacked on HEAD, and that status can arrive after the graph's first pass —
    // a repository opened onto a stopped merge is the case where it does. Read too early, the verb waits out its
    // watchdog on the newest *commit*, which was never going anywhere (2026-08-22 実測, `--preset conflict-staged`).
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
            driver.graphGoneOid = going
            stashPressTimer.stop()
        }
    }
    // Emptying one whole bucket from its own heading, and reading back which headings the list is left with. The two
    // directions are one verb because the claim is that they are symmetrical: a bucket that has just been emptied keeps
    // its heading, whichever bucket it was (デザイン規約 §その他の操作).
    //
    // The heading is pressed rather than the slot behind it called, and what is read back is the list's own children —
    // a band bound to nothing would still be counted by the condition that asks for it.
    SampleTimer {
        id: bucketAllTimer
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
                // The heading exists once the list has laid its sections out, which is a frame after the rows arrive.
                if (wipPane.rowAt(0) === null || !wipPane.moveBucket(bucketAllTimer.from))
                    return
                driver.writeSeqBefore = repoTab.writeSeq
                bucketAllTimer.pressed = true
                return
            }
            if (repoTab.busyCount !== 0 || repoTab.writeSeq <= driver.writeSeqBefore)
                return
            // The bucket that was emptied has to be empty before its heading means anything: the counts are the model's
            // answer and the headings are the list's, and reading the second before the first would report the state
            // that was.
            const emptied = bucketAllTimer.from === "staged"
                          ? workTree.stagedCount
                          : bucketAllTimer.from === "conflicts"
                          ? workTree.conflictCount
                          : workTree.unstagedCount + workTree.untrackedCount
            if (emptied !== 0)
                return
            bucketAllTimer.stop()
            // The three headings stand together at the front of the line, ahead of the counts: what a press claims is
            // about the headings side by side, and the judgement reads one unbroken stretch of the line
            // (`Outcome::must_say`) — a count in between would split the claim in two. `conflicts=` is the one that
            // answers the opposite way: that bucket comes and goes with git's own state, so marking the whole of it
            // resolved has to take its heading off the screen (規約 §その他の操作).
            AppBackend.report("wip_heads from=" + bucketAllTimer.from
                              + " unstaged=" + wipPane.bucketHeaded("unstaged")
                              + " staged=" + wipPane.bucketHeaded("staged")
                              + " conflicts=" + wipPane.bucketHeaded("conflicts")
                              + " unstaged_count="
                              + (workTree.unstagedCount + workTree.untrackedCount)
                              + " staged_count=" + workTree.stagedCount
                              + " conflict_count=" + workTree.conflictCount)
            renderedBarrier.begin()
        }
    }
    /// The file-row acts, run once the rows they name are walkable (`fileRowsTimer` holds them
    /// until then; `runAutoAct` has already put the WIP pane up).
    function runFileRowAct(act, arg) {
        if (act === "stage-many" || act === "stage-many-go") {
            const head = wipPane.rowAt(0)
            if (head)
                wipPane.chooseOnly(head.bucket, head.fullName)
            const mate = wipPane.rowFor(arg)
            // Named off the row while it is still in hand: the barrier waits on a bucket and a path, and a delegate
            // read back after the press is one the list has had a chance to take away.
            const moved = mate ? mate.bucket + ":" + mate.fullName : ""
            if (mate)
                wipPane.applyClick(mate.bucket, mate.fullName, Qt.ControlModifier)
            AppBackend.report("chosen count=" + wipPane.chosenCount)
            if (head) {
                wipPane.showStageTools(head.bucket, head.fullName)
                if (act.endsWith("-go")) {
                    const row = wipPane.rowAt(0)
                    if (row)
                        row.stageClicked(head.bucket, head.fullName)
                    driver.treeGoneRow = moved
                }
            }
        } else if (act === "discard-many" || act === "discard-many-go") {
            const first = wipPane.rowAt(0)
            if (first)
                wipPane.chooseOnly(first.bucket, first.fullName)
            const other = wipPane.rowFor(arg)
            // Read while the row is in hand, for the reason `stage-many` gives above.
            const dropped = other ? other.bucket + ":" + other.fullName : ""
            if (other)
                wipPane.applyClick(other.bucket, other.fullName, Qt.ControlModifier)
            AppBackend.report("chosen count=" + wipPane.chosenCount)
            page.openFileMenu(other ? other.bucket : "unstaged", arg, "")
            AppBackend.report("discard_row " + fileDiscardItem.text)
            if (act.endsWith("-go")) {
                fileDiscardItem.completeHold()
                driver.treeGoneRow = dropped
            }
        } else if (act === "file-menu" || act === "file-menu-untracked"
                   || act === "file-menu-staged" || act === "file-menu-conflict") {
            const menuBucket = act === "file-menu" ? "unstaged" : act === "file-menu-staged" ? "staged"
                             : act === "file-menu-conflict" ? "conflicts" : "untracked"
            wipPane.chooseOnly(menuBucket, arg)
            page.openFileMenu(menuBucket, arg, "")
            if (menuBucket === "conflicts") {
                const row = wipPane.rowFor(arg)
                AppBackend.report("conflict_kind " + (row ? row.conflictWords() : "-"))
            } else {
                AppBackend.report("discard_row " + fileDiscardItem.text)
            }
        } else if (act === "take-side-ours" || act === "take-side-theirs") {
            wipPane.chooseOnly("conflicts", arg)
            page.openFileMenu("conflicts", arg, "")
            fileMenu.close()
            // Read before the press, for the reason the merge editor below gives: a row that is not conflicted takes
            // no side, and a barrier armed anyway waits on a bucket this run never wrote to.
            const taken = fileRowMenu.chosenConflicts().length
            fileRowMenu.takeSideNow(act === "take-side-ours" ? "ours" : "theirs")
            if (taken > 0)
                driver.treeGoneRow = "conflicts:" + arg
        } else if (act === "open-mergetool") {
            // With a tool configured this holds the write queue until it exits, so a demo tool that blocks leaves the
            // wait on screen.
            wipPane.chooseOnly("conflicts", arg)
            page.openFileMenu("conflicts", arg, "")
            fileMenu.close()
            // The name comes from config and the paths from the choice, so the name alone cannot say whether
            // anything was handed over. A file that is already resolved — a fixture a previous run consumed —
            // chooses nothing, and `openInMergeTool` then returns without queueing a write, leaving the run to
            // the watchdog with the tool's name reported all the same. The count is what tells the two apart.
            const handed = fileRowMenu.chosenConflicts().length
            fileRowMenu.openInMergeTool()
            AppBackend.report("merge_tool " + wipPane.workTree.mergeTool + " paths=" + handed)
            // Named only where something was actually handed over: a fixture a previous run consumed queues no write
            // at all, and a barrier waiting for a row that left the bucket before this run began would report a
            // landing nothing here caused.
            if (handed > 0)
                driver.treeGoneRow = "conflicts:" + arg
        } else if (act === "discard-file" || act === "discard-file-go"
                   || act === "delete-file" || act === "delete-file-go"
                   || act === "discard-staged" || act === "discard-staged-go") {
            // Which row follows the verb: "delete-file" an untracked one, "discard-staged" the staged side, otherwise
            // the unstaged one. The plain verb leaves the menu standing for the shot; "-go" runs the hold to its end.
            const bucket = act.startsWith("delete-file") ? "untracked"
                         : act.startsWith("discard-staged") ? "staged" : "unstaged"
            wipPane.chooseOnly(bucket, arg)
            page.openFileMenu(bucket, arg)
            AppBackend.report("discard_row " + fileDiscardItem.text)
            if (act.endsWith("-go")) {
                fileDiscardItem.completeHold()
                driver.treeGoneRow = bucket + ":" + arg
            }
        }
    }
    // Every file-row act resolves the rows it names through the pane's walk — `rowAt` / `rowFor` /
    // `chosenRows` — and the walk reads delegates, which are born a layout after the model has the
    // rows. Fired on arrival the walk answers nothing: the choice stays empty, the menu opens over
    // it with an empty discard row, and a "-go" with nothing to write leaves the run to the
    // watchdog (2026-08-23 実測: Windows wedged this way while the same build walked on Linux).
    // So the acting waits for the row it is about to name, the way `bucketAllTimer` waits for the
    // headings; a row that never lands leaves the run to the watchdog, which is the diagnosis.
    SampleTimer {
        id: fileRowsTimer
        onTriggered: {
            const act = AppBackend.autoAct
            // The named row has to be walkable — and for the pairs that start from the head row,
            // that row too. One walk answering is every walk answering: they read the same
            // delegates.
            if (wipPane.rowFor(AppBackend.autoActArg) === null)
                return
            if ((act === "stage-many" || act === "stage-many-go"
                 || act === "discard-many" || act === "discard-many-go")
                && wipPane.rowAt(0) === null)
                return
            fileRowsTimer.stop()
            driver.runFileRowAct(act, AppBackend.autoActArg)
            driver.dispatchFinished()
        }
    }
    // Sending the diff's code sideways, by the bar's own path and then by the hand that carries the rows with it. What
    // is read back is where the code and the rows ended up, never what was asked for: a bar bound to nothing still
    // takes a press, and a hand wired to nothing still starts.
    //
    // The wait is for the view (`keep-place` learned the same lesson): rows that have arrived are not rows the list has
    // laid out, and until it has, `codeMax` is measured against a width of nothing. A diff with nowhere sideways to go
    // says so and stops there rather than at the watchdog — a run over one photographs a pane that proves nothing
    // (app-ui.md §UI 自動化の因果性).
    SampleTimer {
        id: codeSendTimer
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
                // Half the way by the bar's own path, and the rest — with the rows — by the hand, started from the
                // middle of the view and drifted down and to the right. The anchor's ring is part of the picture, so
                // the hand is left running for the shot.
                diffPane.sendCode(diffPane.codeMax / 2)
                diffPane.startCodeHand(diffPane.view.width / 2,
                                       diffPane.view.height / 2)
                diffPane.driftCodeHand(diffPane.view.width / 2 + 120,
                                       diffPane.view.height / 2 + 120)
                codeSendTimer.sent = true
                return
            }
            // The hand ticks on its own clock, and both of its axes have to be seen moving: the rows have come down,
            // and the code has gone further than the bar's half left it.
            if (diffPane.view.contentY <= 0 || diffPane.codeAt <= diffPane.codeMax / 2)
                return
            codeSendTimer.stop()
            codeSendTimer.report()
            renderedBarrier.begin()
        }
    }
    // Reading part way down a long diff and then writing: the rebuild has to come back to the same place.
    //
    // The wait is for the view, not for the model (`diff-step` learned the same lesson): rows that have arrived are not
    // rows the list has laid out, and until it has there is no place to lose — the scroll goes nowhere and the restore
    // has nothing to undo. So what is waited for is the room the reading consumes, and a diff that is laid out and
    // still too short says so and stops there rather than at the watchdog: nothing that short can hold a place, and a
    // run over it photographs a pane that proves nothing (app-ui.md §UI 自動化の因果性).
    SampleTimer {
        id: keepPlaceTimer
        /// The line of the first hunk that gets staged, which is what rebuilds the diff under the reader.
        property int line: -1
        property bool wrote: false
        function begin(atLine) {
            keepPlaceTimer.line = atLine
            keepPlaceTimer.wrote = false
            keepPlaceTimer.start()
        }
        /// Rows the list has actually put down, as against rows it has been handed: `contentHeight` is still zero for
        /// the first of those and `maxY` cannot be read before it.
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
            // Both edges of the write, and then the one output the whole verb is about: the rebuilt list put back on
            // the place. A write that emptied this side never gets a row back and so never lands anywhere — which is a
            // fixture with no place in it, and the run waits rather than passing on the silence.
            if (repoTab.busyCount !== 0 || repoTab.writeSeq <= driver.writeSeqBefore
                    || diffPane.placeLandedY < 0)
                return
            keepPlaceTimer.stop()
            driver.reportPlace(diffPane.placeLandedY, diffPane.view.maxY)
            renderedBarrier.begin()
        }
    }
    // A reader who scrolled before the colours landed: the whole list is swapped again when the colours turn up
    // (`DiffModel::lay_out_rows`), and that swap must not cost the place being read.
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
                // Read down the file the moment the rows are there, which is well before the colours are.
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
    // git's refusal has to come back before the row it turns into a held one can be held — or photographed.
    SampleTimer {
        id: forceDeleteTimer
        onTriggered: {
            if (repoTab.writeSeq <= driver.writeSeqBefore || repoTab.busyCount !== 0 || refDeleteItem.holdMs <= 0)
                return
            forceDeleteTimer.stop()
            AppBackend.report("ref_menu delete=" + refDeleteItem.text
                              + " note=" + refDeleteItem.note)
            driver.writeSeqBefore = repoTab.writeSeq
            refDeleteItem.completeHold()
            writeBarrier.start()
        }
    }
    // The splitter has to have handed the pane its new width before the width can be reported — the fold sets it, the
    // layout takes it.
    SampleTimer {
        id: navRailTimer
        onTriggered: {
            if (sidebarPane.width <= 0 || sidebarPane.height <= 0)
                return
            navRailTimer.stop()
            AppBackend.report(
            // The three that are read together: what the centre holds, and what is being typed into. A box opens where
            // its row is, and folded that is the section beside the rail — putting the list back would take an open
            // file down with it (デザイン規約 §左メニューを畳む), so the three are neighbours or they cannot be judged in one
            // substring.
            "nav_rail collapsed=" + page.sidebarCollapsed
            + " diff=" + page.diffShown
            + " editing=" + sidebarPane.editKey
            + " width=" + Math.round(sidebarPane.width)
            + " peek=" + sidebarPane.peekKind
            // Where the open section stands. `cell` is the top edge of the mark that opened it and `top` where the
            // panel begins — they are the same number or the list has walked away from its own cell — the failure a
            // section too tall for the pane invites. `end` against `pane` is the other half: it grows down into the
            // pane and stops at the foot of it.
            + " top=" + Math.round(sidebarPane.peekY)
            + " cell=" + Math.round(sidebarPane.peekTop)
            + " end=" + Math.round(sidebarPane.peekBottom)
            + " pane=" + Math.round(sidebarPane.height))
            driver.complete()
        }
    }
    // PG_AUTO_ACT=tags-eye: the eye at the end of the TAGS band, and the graph on the other side of it. What the
    // switch moves is the walk, so the commit the named tag stands on is the one thing that answers it — a tag on a
    // commit a branch also reaches keeps both its row and its chip, and a picture of that frames exactly like a
    // picture of a switch that did nothing.
    //
    // **The graph is emptied before it is rebuilt**, so neither the row's absence nor the row count is an edge on its
    // own — a reset shows both a beat after the press, with the walk still to run. `finishCount` is what says a pass
    // landed, and the row is read only after it moves. Turning the tags back on lands twice (a tag-less pass paints
    // first — core.md §2 段ストリーミング), and the row being back is what tells the second pass from the first.
    //
    // The press waits for that same row too, and reads it in the arming step alone: a page is current as soon as the
    // *first* pass finishes, and the first pass is the tag-less one, so a run that pressed at dispatch would be
    // taking the tags out of a graph that never had them in (規約 §前提条件を完了判定に混ぜない).
    property string tagEyeOid: ""
    property int tagEyeFinish: 0
    property bool tagEyeBack: false
    /// 0 = waiting for the tags to be in the graph to take out, 1 = for the row to go, 2 = for it to come back.
    property int tagEyeStep: 0
    SampleTimer {
        id: tagEyeTimer
        onTriggered: {
            const there = graphModel.rowOf(driver.tagEyeOid) >= 0
            if (driver.tagEyeStep === 0) {
                if (!there)
                    return
                driver.tagEyeStep = 1
                driver.tagEyeFinish = graphModel.finishCount
                sidebarPane.tapTagEye()
                return
            }
            if (graphModel.finishCount === driver.tagEyeFinish)
                return
            if (driver.tagEyeStep === 1) {
                if (there)
                    return
                if (driver.tagEyeBack) {
                    driver.tagEyeStep = 2
                    driver.tagEyeFinish = graphModel.finishCount
                    sidebarPane.tapTagEye()
                    return
                }
            } else if (!there)
                return
            tagEyeTimer.stop()
            // `there` is the half a picture cannot carry on its own, and `shown=` is the switch's own answer: a run
            // whose press never reached the band photographs the state it started in, which is a real state.
            AppBackend.report("tags_eye shown=" + repoTab.tagsShown
                              + " there=" + there
                              + " tag=" + AppBackend.autoActArg
                              + " rows=" + graphModel.rowTotal
                              + " tags=" + tagsModel.total)
            driver.complete()
        }
    }
    // PG_AUTO_ACT=nav-reclick / nav-reclick-away: the two clicks of the rename gesture, put in at the rows of the
    // section the folded rail has open, and what those rows made of them. Every step waits for its own answer — the
    // row has to exist before it can be clicked, the double-click window the first click opened has to have passed
    // before a second one counts as a second, and "-away" waits for the section to have actually gone (the pointer
    // leaving is answered a beat later — SectionPeekPopup.settle).
    property bool reclickAway: false
    property bool reclickArmed: false
    property int reclickStep: 0
    SampleTimer {
        id: reclickTimer
        /// Which section, and which of its rows. A section whose names fold into folders has no row to rename at the
        /// top of it, so the row travels with the argument (`branch:1`) and defaults to the first.
        readonly property string kind: {
            const arg = AppBackend.autoActArg
            const cut = arg.indexOf(":")
            return cut < 0 ? arg : arg.substring(0, cut)
        }
        readonly property int row: {
            const arg = AppBackend.autoActArg
            const cut = arg.indexOf(":")
            return cut < 0 ? 0 : Number(arg.substring(cut + 1))
        }
        // The pointer comes to rest on the cell. A section whose model has no rows yet opens nothing (NavRail.enterAt),
        // so the arrival is made again until one stands.
        function peeked() {
            if (sidebarPane.peekKind !== "")
                return true
            sidebarPane.peekAt(reclickTimer.kind)
            return false
        }
        onTriggered: {
            const section = sidebarPane.peekSection
            if (driver.reclickStep === 0) {
                if (!reclickTimer.peeked() || !section.clickRow(reclickTimer.row))
                    return
                driver.reclickStep = driver.reclickAway ? 1 : 3
            } else if (driver.reclickStep === 1) {
                sidebarPane.peekAway(reclickTimer.kind)
                driver.reclickStep = 2
            } else if (driver.reclickStep === 2) {
                // The pointer leaving is answered a beat later, so the section is gone when it says so and not before.
                if (sidebarPane.peekKind !== "")
                    return
                driver.reclickStep = 3
            } else if (driver.reclickStep === 3) {
                if (!reclickTimer.peeked() || section.rowGuarded(reclickTimer.row)
                        || !section.clickRow(reclickTimer.row))
                    return
                // Read where it is set, not where it lapses: the wait is short and the box is what it turns into.
                driver.reclickArmed = section.rowArmed(reclickTimer.row)
                driver.reclickStep = 4
            } else if (driver.reclickStep === 4) {
                // Armed, the box opens once that wait runs out — on the row where it stands, which is in the section
                // beside the rail (SidebarPane.startEdit). Unarmed there is nothing further to wait for.
                if (driver.reclickArmed && sidebarPane.editKey === "")
                    return
                reclickTimer.stop()
                AppBackend.report(
                "nav_reclick section=" + reclickTimer.kind
                + " row=" + reclickTimer.row
                + " marked=" + sidebarPane.activeKey
                + " away=" + driver.reclickAway
                + " armed=" + driver.reclickArmed
                // The fold is not undone for a box, the box is there, and it has the keyboard — the three that say the
                // gesture landed where the hand was (デザイン規約 §左メニューを畳む).
                + " collapsed=" + page.sidebarCollapsed
                + " box=" + (sidebarPane.editKey !== "")
                + " focused=" + section.rowFocused(reclickTimer.row)
                + " peek=" + sidebarPane.peekKind
                + " editing=" + sidebarPane.editKey)
                driver.complete()
            }
        }
    }
    // PG_AUTO_ACT=nav-rename-far: the section is opened, scrolled until its first row is out of sight, and only then
    // asked for a box on that row. Each step waits for the one before to have landed — a list still building has no
    // height to scroll by, and a run that named the row before the scroll took would be watching the list stay put.
    property int farStep: 0
    SampleTimer {
        id: farTimer
        onTriggered: {
            const section = sidebarPane.peekSection
            if (driver.farStep === 0) {
                if (sidebarPane.peekKind === "") {
                    sidebarPane.peekAt("tag")
                    return
                }
                section.scrollToEnd()
                driver.farStep = 1
            } else if (driver.farStep === 1) {
                // The scroll is what puts the row out of sight; until it has, there is nothing to bring back.
                if (section.rowInView(0))
                    return
                sidebarPane.beginRename("tag", tagsModel.nameAt(0),
                                        tagsModel.nameAt(0))
                driver.farStep = 2
            } else if (driver.farStep === 2) {
                if (sidebarPane.editKey === "")
                    return
                farTimer.stop()
                AppBackend.report(
                "nav_far row=" + tagsModel.nameAt(0)
                + " shown=" + section.rowInView(0)
                + " box=" + (sidebarPane.editKey !== "")
                + " collapsed=" + page.sidebarCollapsed
                + " editing=" + sidebarPane.editKey)
                driver.complete()
            }
        }
    }
    /// PG_AUTO_ACT=nav-add-remote: the run stops with the real dialog on screen. `remotes=` is the section the `+` was
    /// pressed on — at 0 the band around it is unavailable, and that the form still came up is the half of this a
    /// picture of an empty section cannot hold either way. `collapsed=` says which of the two doors it came through,
    /// and that the folded one did not put the list back on its way (デザイン規約 §左メニューを畳む).
    SampleTimer {
        id: navAddRemoteTimer
        onTriggered: {
            if (!remoteDialog.visible)
                return
            navAddRemoteTimer.stop()
            AppBackend.report("nav_add_remote dialog=" + remoteDialog.visible
                              + " collapsed=" + page.sidebarCollapsed
                              + " remotes=" + remotesModel.total
                              + " name=" + remoteDialog.wantedName)
            driver.complete()
        }
    }
    /// PG_AUTO_ACT=push-default: the mark lands on a remote and the run stops with the sidebar showing it. The write
    /// is the barrier — `pushDefault` only says the name once `git config` has run and the refresh behind it has
    /// republished the snapshot, so a picture taken here is of a repository that really is marked.
    SampleTimer {
        id: pushDefaultTimer
        onTriggered: {
            if (repoTab.pushDefault !== driver.markWanted || repoTab.busyCount > 0)
                return
            pushDefaultTimer.stop()
            // The judged fields lead, and together: `must_say` reads one run of the line, not a set of words in it.
            AppBackend.report("push_default local=" + repoTab.pushDefaultLocal
                              + " marked=" + repoTab.pushDefault
                              + " target=" + repoTab.defaultRemote
                              + " remotes=" + repoTab.remoteCount)
            driver.complete()
        }
    }
    /// PG_AUTO_ACT=push-target: where the toolbar says this branch's push is going, and the standing beside it. Both
    /// come off the page the button reads (`RepoPage.pushTargetLabel` / `pushState`), so what answers is the binding
    /// the window uses rather than a second reading written for the run. The marks that decided it ride the same line:
    /// the label alone cannot say **which** of them git would have followed.
    ///
    /// `workTree.loaded` is the barrier — before the first status lands the branch is empty and every mark reads as
    /// unset, which is a destination of nothing rather than an answer.
    SampleTimer {
        id: pushTargetTimer
        onTriggered: {
            if (!workTree.loaded || repoTab.state !== "open" || repoTab.busyCount > 0)
                return
            pushTargetTimer.stop()
            AppBackend.report("push_target label=" + page.pushTargetLabel
                              + " state=" + page.pushState
                              + " branch_mark=" + workTree.pushRemote
                              + " repo_mark=" + repoTab.pushDefault
                              + " tracks=" + workTree.upstream)
            driver.complete()
        }
    }
    /// PG_AUTO_ACT=publish-remotes-marked: the first push's destination list with the mark in it. The mark is put on
    /// first and waited for — the question reads the marked remote as it opens, so a list opened before the write
    /// landed would be the one from before.
    SampleTimer {
        id: publishMarkedTimer
        onTriggered: {
            if (repoTab.pushDefault !== driver.markWanted || repoTab.busyCount > 0)
                return
            if (!publishFlow.publishAsking) {
                page.pushNow()
                return
            }
            if (!publishFlow.publishRemotesOpen()) {
                publishFlow.openPublishRemotes()
                return
            }
            publishMarkedTimer.stop()
            AppBackend.report("publish_remotes open=" + publishFlow.publishRemotesOpen()
                              + " marked=" + publishFlow.publishRemotesMarked()
                              + " name=" + repoTab.pushDefault)
            driver.complete()
        }
    }
    /// PG_AUTO_ACT=remote-menu: the menu a remote's own row raises, left standing (overlay.png). `rows=` is what it is
    /// offering — two on a remote that is not the destination, one on the remote that already is, since a row with
    /// nothing to do is gone rather than greyed (デザイン規約 §メニュー).
    SampleTimer {
        id: remoteMenuTimer
        onTriggered: {
            if (repoTab.pushDefault !== driver.markWanted || repoTab.busyCount > 0)
                return
            if (!driver.remoteMenu.opened && !page.openRemoteMenu(driver.remoteTarget))
                return
            if (!driver.remoteMenu.opened)
                return
            remoteMenuTimer.stop()
            AppBackend.report("remote_menu open=" + driver.remoteMenu.opened
                              + " rows=" + driver.remoteMenu.offeredRows
                              + " remote=" + driver.remoteTarget
                              + " marked=" + repoTab.pushDefault)
            driver.complete()
        }
    }
    /// PG_AUTO_ACT=remote-url: the form that holds a remote's URL, left standing (overlay.png) — the other way to the
    /// mark. `box=` is whether the line is checked, which is the half a picture of a form cannot be trusted for.
    SampleTimer {
        id: remoteUrlTimer
        onTriggered: {
            if (repoTab.pushDefault !== driver.markWanted || repoTab.busyCount > 0)
                return
            if (!driver.remoteDialog.visible) {
                driver.publishFlow.startEditRemote(driver.remoteTarget)
                return
            }
            remoteUrlTimer.stop()
            AppBackend.report("remote_url dialog=" + driver.remoteDialog.visible
                              + " box=" + driver.remoteDialog.marked
                              + " remote=" + driver.remoteDialog.editing
                              + " local=" + driver.remoteDialog.markLocal)
            driver.complete()
        }
    }
    // The column has to be laid out again before the header that was closed can say where it ended up.
    SampleTimer {
        id: navSectionTimer
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
    /// What each section kept of what it holds. The rows a filter leaves are the ones the sections work out for
    /// themselves, so the counts are read off the models and the picture says what they drew.
    SampleTimer {
        id: navFilterTimer
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
    // The blocked row's line, worn where the pointer would put it. Past `tipDelayMs`, like the other forced tooltips:
    // read any sooner and the attached ToolTip has not opened yet, so the line reports false while the picture taken at
    // quit holds it.
    SampleTimer {
        id: blockedTipTimer
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
    // Where the move came to rest, and what the carry left in the stash list. The branch itself is the edge — the
    // status pass after the move is what writes it — so a run that never landed waits out the watchdog rather than
    // photographing the tree it started in.
    // The rename's own question, waited on for the same settle: a bar photographed before its words arrive is a red
    // line with nothing on it. The plain verb ends here — the write is "-go"'s half.
    SampleTimer {
        id: renameAskTimer
        onTriggered: {
            if (!graphPane.askSettled)
                return
            renameAskTimer.stop()
            // `code=` being empty is part of the claim: a push and a delete make no one command, so the pill answers
            // in the ordinary voice (規約 §git 用語のコード表記 の 1:1 規則 — the same reading `move_ask` makes).
            AppBackend.report("rename_ask hold=" + graphPane.askHold
                              + " code=" + graphPane.askCode)
            driver.complete()
        }
    }
    SampleTimer {
        id: moveAskTimer
        onTriggered: {
            if (!graphPane.askSettled)
                return
            moveAskTimer.stop()
            // No chip on this one — git has no single word for moving a branch onto a ref, so the pill answers in the
            // ordinary voice (規約 §git 用語のコード表記). `code=` being empty is part of the claim.
            AppBackend.report("move_ask hold=" + graphPane.askHold
                              + " code=" + graphPane.askCode
                              + " branch=" + workTree.branch)
            driver.complete()
        }
    }
    SampleTimer {
        id: switchLandsTimer
        property string branch: ""
        property int stashes: -1
        onTriggered: {
            if (repoTab.busyCount !== 0 || workTree.branch !== switchLandsTimer.branch)
                return
            if (switchLandsTimer.stashes >= 0 && stashesModel.total !== switchLandsTimer.stashes)
                return
            switchLandsTimer.stop()
            // `log=` on all three of these: a move that git refused would raise the command log
            // (§git が言ったことを読む場所), and a red panel under a press that had a way out on screen is the thing
            // this whole road exists to stop (2026-08-22 ユーザー判断). A shut panel and a panel that was never
            // raised are the same picture, which is why it is said rather than shown.
            AppBackend.report("switch_landed branch=" + workTree.branch
                              + " stashes=" + stashesModel.total
                              + " wanted=" + switchLandsTimer.stashes
                              + " conflicts=" + workTree.conflictCount
                              + " log=" + page.commandsOpen)
            driver.complete()
        }
    }
    // The bar that comes down instead of the move — waited on all the way down (`AskBar.settled`), not at the label
    // that starts it: the 200ms opening is 200ms of red line with no words in it, and that is what the first run of
    // this verb photographed.
    SampleTimer {
        id: switchStoppedTimer
        property bool go: false
        onTriggered: {
            if (!graphPane.askSettled)
                return
            switchStoppedTimer.stop()
            AppBackend.report("switch_stopped code=" + graphPane.askCode
                              + " accept=" + graphPane.askAccept
                              + " hold=" + graphPane.askHold
                              + " bang=" + graphPane.askAlert
                              + " op=" + workTree.opCommand
                              + " branch=" + workTree.branch
                              + " log=" + page.commandsOpen)
            if (!switchStoppedTimer.go) {
                driver.complete()
                return
            }
            // Whichever gesture this shape of the question takes — a held pill reports no click, and a click pill has
            // no hold to run to its end. The bar itself says which it is.
            driver.writeSeqBefore = repoTab.writeSeq
            if (graphPane.askHold)
                graphPane.completeHold()
            else
                page.answerRowAsk()
            switchStoppedLandedTimer.start()
        }
    }
    // Where the answer put the reader, and what it left in the stash list. **The write's own answer is not the edge**
    // — it lands before the rebuild it asks for (core `AfterWrite::Graph`), so a run that read the branch there would
    // photograph the one it was leaving; and the stash list is read after that again (`switch-lands`), which is why
    // the count comes from the argument and is waited for.
    SampleTimer {
        id: switchStoppedLandedTimer
        property int stashes: -1
        onTriggered: {
            if (repoTab.busyCount !== 0 || repoTab.writeSeq <= driver.writeSeqBefore
                    || workTree.opCommand !== "" || !graphPane.askShut)
                return
            if (switchStoppedLandedTimer.stashes >= 0
                    && stashesModel.total !== switchStoppedLandedTimer.stashes)
                return
            switchStoppedLandedTimer.stop()
            AppBackend.report("switch_stopped_landed branch=" + workTree.branch
                              + " op=" + workTree.opCommand
                              + " conflicts=" + workTree.conflictCount
                              + " stashes=" + stashesModel.total
                              + " wanted=" + switchStoppedLandedTimer.stashes
                              + " log=" + page.commandsOpen)
            driver.complete()
        }
    }
    SampleTimer {
        id: switchMarkTimer
        property string want: ""
        onTriggered: {
            if (!refMenu.opened)
                return
            switchMarkTimer.stop()
            // `indent=` is the other half: the mark is drawn inside the padding the whole menu carries for it, so a
            // menu that forgot to open that column would draw the `!` over its own edge (`AppMenu.holdIndent`).
            AppBackend.report("switch_mark offered=" + refSwitchItem.offered
                              + " asks=" + refSwitchItem.asks
                              + " want=" + switchMarkTimer.want
                              + " indent=" + (refMenu.holdIndent > 0))
            driver.complete()
        }
    }
    SampleTimer {
        id: chipMenuTimer
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
    // A tag that has been made is a row in TAGS, and **the write answers before the read that puts it there** (core
    // `AfterWrite::Graph` — verify-ui §壊れない動詞). Stopping at the write barrier photographs the sidebar as it was a
    // moment before, which is a picture of nothing having happened; what this waits for is the name itself.
    SampleTimer {
        id: createTagTimer
        onTriggered: {
            const oid = tagsModel.oidOfName(AppBackend.autoActArg)
            if (oid === "" || repoTab.busyCount !== 0)
                return
            createTagTimer.stop()
            AppBackend.report("create_tag tag=" + AppBackend.autoActArg
                              + " row=" + tagsModel.rowOfName(AppBackend.autoActArg)
                              + " total=" + tagsModel.total
                              + " at=" + (oid === driver.createTagOid))
            renderedBarrier.begin()
        }
    }
    /// The commit the run asked for the tag on, so the report can say the tag landed on that one rather than on
    /// wherever HEAD happened to be.
    property string createTagOid: ""

    // The tag menu's push row, which is the one row in this application whose whole shape — chip, hold, colour — is
    // decided by what a remote was last heard to carry (`RefRowMenu`). Two states to photograph and they are told
    // apart by nothing but that reading, so the report spells it out: a picture of `push` and a picture of
    // `push --force` differ by five glyphs in a card that is otherwise identical.
    //
    // **`want` is what the run is waiting for, not what it asserts.** The drifted side needs the remotes read
    // (`ls-remote --tags` is the only carrier — core.md タグのリモート状態), and that read lands well after the fetch it
    // rides out with; waiting on the fetch alone photographs the plain row and calls it the forced one.
    SampleTimer {
        id: tagMenuTimer
        /// The tag the menu is to stand on, and what the run needs known about it before the card is worth
        /// photographing: `drift` (a remote has the name on another commit) or `remote` (a remote has it at all).
        /// Both are answers only `ls-remote --tags` carries, so either one fetches first.
        property string tag: ""
        property string wants: ""
        /// Which row this run presses, empty for the ones that only stand the card up.
        property string press: ""
        function begin(arg, pressing) {
            const parts = arg.split(":")
            tagMenuTimer.tag = parts[0]
            tagMenuTimer.wants = parts.length > 1 ? parts[1] : ""
            tagMenuTimer.press = pressing
            if (tagMenuTimer.wants !== "")
                repoTab.fetch("")
            tagMenuTimer.start()
        }
        /// Whether what this run is waiting on has arrived, asked of the same lookups the menu asks
        /// (`NavSectionModel`): the readings are in or they are not, and no count of fetches says which.
        function ready() {
            if (tagMenuTimer.wants === "drift")
                return tagsModel.remoteTagDrift(tagMenuTimer.tag, repoTab.defaultRemote) !== ""
            if (tagMenuTimer.wants === "remote") {
                const sides = tagsModel.tagSides(tagMenuTimer.tag)
                return sides === "remote" || sides === "both"
            }
            return true
        }
        onTriggered: {
            if (!tagMenuTimer.ready() || repoTab.busyCount !== 0)
                return
            tagMenuTimer.stop()
            page.openRefMenu("tag", tagMenuTimer.tag, tagMenuTimer.tag,
                             tagsModel.oidOfName(tagMenuTimer.tag), true)
            // Every row this verb is about is one card in (デザイン規約 §メニュー の入れ子), so the run opens it: the
            // report reads the rows either way, but a picture of the outer card proves nothing about them.
            refMenu.openSub(refTagCard)
            // Every row this menu grew, in one line. The delete rows are told apart by nothing but which of them is
            // drawn, and a card missing one frames exactly like a card that never offered it.
            // **The order is the judging order.** `must_say` matches a run of this line, so what one run has to
            // assert together has to sit together: the sides and the four rows they decide first, the push row's
            // own shape after them (`verify/verbs/remote.rs`).
            AppBackend.report("tag_menu tag=" + tagMenuTimer.tag
                              + " sides=" + tagsModel.tagSides(tagMenuTimer.tag)
                              + " local_del=" + refTagDeleteItem.offered
                              + " remote_del=" + refRemoteTagDeleteItem.offered
                              + " both_del=" + refTagBothDeleteItem.offered
                              + " tag_here=" + refTagHereItem.offered
                              + " push=" + refPushTagItem.offered
                              + " code=" + refPushTagItem.code
                              + " held=" + (refPushTagItem.holdMs > 0)
                              + " lease=" + refRowMenu.tagDriftOid
                              + " text=" + refPushTagItem.text)
            if (tagMenuTimer.press === "") {
                driver.complete()
                return
            }
            driver.writeSeqBefore = repoTab.writeSeq
            if (tagMenuTimer.press === "remote-delete" || tagMenuTimer.press === "both-delete") {
                // What a delete is judged on is the sidebar afterwards, and **the write answers before the read that
                // rebuilds it** (core `AfterWrite::Graph`) — stopping at the write barrier photographs the list as it
                // was and calls it the list as it is. What the run waits for is this name's own reading changing.
                tagGoneTimer.was = tagsModel.tagSides(tagMenuTimer.tag)
                tagGoneTimer.tag = tagMenuTimer.tag
                if (tagMenuTimer.press === "remote-delete")
                    refRemoteTagDeleteItem.completeHold()
                else
                    refTagBothDeleteItem.completeHold()
                tagGoneTimer.start()
                return
            }
            if (refPushTagItem.holdMs > 0)
                refPushTagItem.completeHold()
            else
                refPushTagItem.triggered()
            writeBarrier.start()
        }
    }
    // A tag delete, judged on the sidebar it leaves rather than on the write that made it. The name's own reading is
    // the edge: gone from both sides it answers nothing at all, gone from the remote alone it drops back to `here`.
    // **A count would not do** — the remote half of a name held on both sides takes no row away.
    SampleTimer {
        id: tagGoneTimer
        property string tag: ""
        property string was: ""
        onTriggered: {
            if (repoTab.busyCount !== 0 || repoTab.writeSeq <= driver.writeSeqBefore)
                return
            const now = tagsModel.tagSides(tagGoneTimer.tag)
            if (now === tagGoneTimer.was)
                return
            tagGoneTimer.stop()
            AppBackend.report("tag_gone tag=" + tagGoneTimer.tag
                              + " was=" + tagGoneTimer.was
                              + " sides=" + now
                              + " row=" + tagsModel.rowOfName(tagGoneTimer.tag)
                              + " total=" + tagsModel.total)
            renderedBarrier.begin()
        }
    }
    // Waits on the early answer, not on a refusal: nothing here writes.
    SampleTimer {
        id: earlyDeleteTimer
        onTriggered: {
            // The row's `code` is never empty on a branch, so it cannot tell "git has not answered yet" from "answered
            // merged" — both wear `branch --delete`. What readiness there is comes from the echo of the branch asked
            // about, which the asking clears before the question goes out (app-ui.md §UI 自動化の因果性).
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
    SampleTimer {
        id: refusedRowTimer
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
    // The row taken away ahead of git's answer. **Waited on the list, not on the write** — being ahead of the write
    // is the whole of what this photographs, so a barrier here would wait out the very state it is about.
    SampleTimer {
        id: goneRowTimer
        onTriggered: {
            if (tagsModel.rowOfName(AppBackend.autoActArg) >= 0)
                return
            goneRowTimer.stop()
            AppBackend.report("gone_row tag=" + AppBackend.autoActArg
                              + " row=" + tagsModel.rowOfName(AppBackend.autoActArg)
                              + " total=" + tagsModel.total
                              + " chips=" + (graphModel.goneChips !== ""))
            driver.complete()
        }
    }
    // The window cut: the walk stops at a round number of commits and the footer is the only thing that says so — its
    // lanes carry on for one more commit's worth and its line names the count.
    //
    // Nothing here is waited out. The walk has to have answered before `truncated` means anything (the initial false is
    // "not asked yet", not "the whole history is loaded" — app-ui.md §UI 自動化の因果性), the footer has to have been given a
    // height, and the view has to have actually arrived at the end rather than merely been told to go: `atYEnd` is the
    // output, `positionViewAtEnd()` only the ask.
    SampleTimer {
        id: graphTailTimer
        onTriggered: {
            if (graphModel.loading || graphModel.rowTotal === 0)
                return
            // The footer lives at the far end of two thousand rows, and a ListView builds what is near its viewport —
            // so it is asked for and then looked for, rather than looked for first.
            const tail = graphPane.view.footerItem
            if (tail === null || tail.height <= 0) {
                graphPane.view.positionViewAtEnd()
                return
            }
            // On screen whole, read off where it sits rather than off the call having been made: `positionViewAtEnd`
            // puts the last *row* against the edge, and this pane keeps a run-out below it, so being told to go is not
            // the same as having arrived.
            const bottom = graphPane.view.contentY + graphPane.view.height
            if (tail.y + tail.height > bottom + 0.5) {
                graphPane.view.positionViewAtEnd()
                return
            }
            graphTailTimer.stop()
            // The verdict leads, and its two halves are neighbours: a graph that never cut and one whose footer failed
            // to draw frame the same way — the end of a history and the end of what was loaded are the same picture
            // without the line.
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
    // The stand-in for a HEAD scrolled off the graph (`GraphHeadPin`). It only exists where the row does not, so each
    // of these sends the view to an edge first — and then reads the stand-in itself rather than the ask, because
    // "told to go" and "arrived" are not the same thing (the same reason `graph-tail` reads `atYEnd`).
    //
    // Nothing here is waited out: the walk has to have answered (`headRow` is -1 until it has), and the chips arrive a
    // pass behind the rows, so the row is found before it can say its own name.
    SampleTimer {
        id: graphHeadTimer
        /// Whether the second move — the press, or the scroll back — has been made. The first one is not latched: a
        /// view told to go to its end before it has laid two thousand rows out goes to the end it knows about and stays
        /// there, so the ask is repeated until the stand-in itself says it arrived (2026-08-22 実測 — one ask, and the
        /// run waited out its watchdog at the top of the graph).
        property bool answered: false
        readonly property bool below: AppBackend.autoAct === "graph-head-below"
        function report() {
            const row = graphModel.headRow
            // The five judged answers first and in one run, because a
            // `must_say` catches neighbours only (`verify/verbs.rs`).
            AppBackend.report(
                "graph_head shown=" + graphPane.headPin.visible
                + " above=" + graphPane.headPin.rowAbove
                + " onScreen=" + graphPane.view.rowOnScreen(row)
                + " lit=" + graphPane.headPin.lit
                + " landed=" + (graphPane.view.currentIndex === row)
                + " row=" + row
                + " at=" + graphPane.view.currentIndex
                // What it leaves the list's own scroll bar. A picture cannot answer it — the band is drawn over the
                // trough either way — and a zero would mean the trough behind it answers with a jump to HEAD.
                + " bar=" + Math.round(graphPane.headPin.barRoom))
            driver.complete()
        }
        onTriggered: {
            if (graphModel.loading || graphModel.rowTotal === 0
                    || graphModel.headRow < 0 || graphModel.headLabels === "")
                return
            const act = AppBackend.autoAct
            if (!graphHeadTimer.answered) {
                // The stand-in has to have come up before anything is asked of it: the two runs below are about what
                // takes it away again, and a run that never saw it would call an empty band a success.
                if (!graphPane.headPin.visible) {
                    if (graphHeadTimer.below)
                        graphPane.view.positionViewAtBeginning()
                    else
                        graphPane.view.positionViewAtEnd()
                    return
                }
                graphHeadTimer.answered = true
                if (act === "graph-head-lit") {
                    // Hover cannot be injected, so the rest goes to the one property the pointer's own arrival writes.
                    graphPane.headPin.pointed = true
                } else if (act === "graph-head-back") {
                    graphPane.view.positionViewAtBeginning()
                    return
                } else if (act === "graph-head-go") {
                    graphPane.headPin.activated(graphPane.headPin.headRow)
                    return
                }
                graphHeadTimer.stop()
                graphHeadTimer.report()
                return
            }
            // What the press and the scroll back are both judged on: the row is on screen, so the stand-in has stepped
            // aside. A press is judged on where the selection went as well.
            if (graphPane.headPin.visible || !graphPane.view.rowOnScreen(graphModel.headRow))
                return
            if (act === "graph-head-go" && graphPane.view.currentIndex !== graphModel.headRow)
                return
            graphHeadTimer.stop()
            graphHeadTimer.report()
        }
    }
    // The lane column has to have taken its narrower width before there is anywhere to pan to, or a bar worth wanting.
    SampleTimer {
        id: graphPanTimer
        onTriggered: {
            if (graphPane.graphXMax <= 0)
                return
            graphPanTimer.stop()
            // Where the pointer is, which is the whole of what puts the bar on screen. `-away` walks it back out again:
            // a bar that comes when the pointer does proves nothing on its own unless it also goes when the pointer
            // goes.
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
            // The middle click, then the pointer drifting sideways off it. The argument says which column the click
            // landed in, which is the whole question — only the lanes take the sideways drift (デザイン規約 §グラフを横へ送る).
            const y = graphPane.height / 2
            const x = AppBackend.autoActArg === "message"
                    ? graphPane.labelW + graphPane.graphColW + Theme.spaceXl : graphPane.labelW + Theme.spaceSm
            graphPane.startAutoScroll(x, y)
            graphPane.driftPointer(x + graphPane.width, y)
            middleScrollTimer.start()
        }
    }
    // Where the lanes ended up is the whole question, so that is what is waited for — a pan that ran and a pan that was
    // refused must not read alike in the report.
    //
    // A gesture that carries the lanes runs until they have nowhere left to go: the pointer was put a whole pane's
    // width out, so the ticker saturates the clamp and `graphX` stops at its own maximum. One that does not carry them
    // has already answered by starting without the carry — `panning` is settled in `start()` by where the click landed
    // — and no tick will ever move them.
    //
    // Not "wait for `autoPanning` to go false": the flag is kept for the whole gesture (デザイン規約 §グラフを横へ送る), and nothing
    // here ends the gesture, so the lane column's own case never completed (2026-08-16 実測: watchdog on both systems,
    // `message` passing beside it because that one never pans).
    SampleTimer {
        id: middleScrollTimer
        onTriggered: {
            if (graphPane.autoPanning && graphPane.graphX < graphPane.graphXMax - 0.5)
                return
            middleScrollTimer.stop()
            AppBackend.report(
            "middle_scroll lanes=" + graphPane.autoPanning
            + " x=" + Math.round(graphPane.graphX)
            + " max=" + Math.round(graphPane.graphXMax))
            driver.complete()
        }
    }
    // The arrow keys, which no headless run can press: the walk enters where `Keys.onDownPressed` enters
    // (`GraphPane.stepRow`) after taking the keyboard the way a row click takes it. The selected commit's message has
    // to have arrived before it can be typed over, which is what the wait is for — the same one the reword verbs keep.
    SampleTimer {
        id: graphStepTimer
        /// How many rows, and which way. The refusing runs fix their own.
        property int steps: 1
        /// The one ground a step is refused on that a run can stand up: a name box open on the row. (The other — a
        /// question standing on the bar — is refused by the same expression, and its pill holds the keyboard anyway.)
        property bool named: false
        /// A half-written message in the details pane, which refuses **nothing** — the arrows walk off it and the
        /// draft goes, the same as a click (デザイン規約 §コミットメッセージの 2 つの枠). Here so a hold cannot come
        /// back unnoticed.
        property bool dirty: false
        /// The view sent away from the selection before the step, so the row stepped onto has no reading position to
        /// preserve.
        property bool away: false
        /// The third refusing ground, and the one that was reported: a diff opened over the graph from CHANGES. The
        /// path is the argument — the file has to be one the selected commit touched.
        property string diffPath: ""
        onTriggered: {
            if (!driver.cardSettled)
                return
            graphStepTimer.stop()
            if (graphStepTimer.dirty)
                detailsPane.setMessageText("wip: half of a subject", "")
            if (graphStepTimer.named)
                graphPane.startNaming(
                    graphModel.oidAt(graphPane.view.currentIndex))
            // The press that says the keyboard works here comes first, because the diff below is what has to take it
            // away again: a run that opened the diff and only then reached for the keyboard would be proving nothing
            // (it would be pressing on a pane that is no longer on the screen).
            graphPane.view.takeKeyboard()
            if (graphStepTimer.diffPath !== "")
                page.toggleDiff("commit", graphStepTimer.diffPath, "")
            graphStepWalk.start()
        }
    }
    // A beat between the setup and the walk: the layout swaps the graph away in its own pass, so a step taken in the
    // same tick as the diff opened would still find the pane on screen.
    SampleTimer {
        id: graphStepWalk
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
                // Where the view stood before each step, so what is read is how the last one landed: a walk that runs
                // off the bottom moves the view once per row from there on.
                graphStepReport.wasY = graphPane.view.contentY
                if (!graphPane.stepRow(way))
                    graphStepReport.refused++
            }
            graphStepReport.start()
        }
    }
    // Longer than the settle behind the walk (`keyStepSettleMs`), so what is read is the reading a hand coming off the
    // key would get: a run that moved the highlight and never landed the selection has to be told apart from one that
    // did, and both frame alike from the waist down.
    //
    // Two edges, because a walk asks twice: a held arrow is read where it set off and again where it stopped
    // (`GraphRowWalk.noteStep`), so the first row's details are still in flight when the last row's request goes out.
    // `selected=` is the selection reaching the lit row, `card=` the pane on the right reaching the selection —
    // waiting the first out alone photographs the highlight on the row the walk stopped on beside a card still
    // holding one it passed through (observed 2026-08-23 on Windows), the wait `file_step` keeps on the diff side.
    SampleTimer {
        id: graphStepReport
        property int from: -1
        property real wasY: 0
        property int refused: 0
        onTriggered: {
            const row = graphPane.view.currentIndex
            if (row < 0 || (graphStepTimer.diffPath === "" && page.selectedOid
                            !== graphModel.oidAt(row))
                    || !driver.cardSettled)
                return
            graphStepReport.stop()
            AppBackend.report(
                "graph_step from=" + graphStepReport.from
                + " row=" + row
                + " steps=" + graphStepTimer.steps
                + " landing=" + graphPane.stepLanding(row, graphStepReport.wasY)
                + " back=" + (row === graphStepReport.from)
                + " refused=" + graphStepReport.refused
                + " focused=" + graphPane.view.activeFocus
                + " diff=" + page.diffShown
                + " onscreen=" + graphPane.rowOnScreen(row)
                + " selected=" + (page.selectedOid === graphModel.oidAt(row))
                + " card=" + (detailsModel.shaHex === page.selectedOid))
            driver.complete()
        }
    }
    // The arrow held down, which a run of steps taken inside one tick cannot be: the settle behind the opening press
    // expires long before any OS sends its first repeat, so the run has to wait it out before the rest of the steps
    // arrive with the key still down (`GraphRowWalk.noteStep`). `reads=` is the whole of the report — a walk asks for
    // a commit twice, at the row it set off from and at the row it stopped on — and no picture holds it: a run that
    // read every row it passed through frames exactly like one that read two.
    SampleTimer {
        id: graphHoldTimer
        /// How many rows the run walks, the opening press included.
        property int steps: 6
        onTriggered: {
            if (!driver.cardSettled)
                return
            graphHoldTimer.stop()
            graphPane.view.takeKeyboard()
            driver.holdReads = 0
            graphHoldReport.from = graphPane.view.currentIndex
            graphHoldReport.refused = 0
            if (!graphPane.stepRow(1))
                graphHoldReport.refused++
            graphHoldRepeat.start()
        }
    }
    // The repeats. Waited on the settle being gone rather than on a count of beats (app-ui.md §UI 自動化の因果性) —
    // that is the edge a real keyboard's first repeat always arrives behind. They go in one tick once it has: what is
    // being proven is that a repeat is not read, not how fast one arrives.
    SampleTimer {
        id: graphHoldRepeat
        onTriggered: {
            if (graphPane.stepSettling)
                return
            graphHoldRepeat.stop()
            for (let n = 1; n < graphHoldTimer.steps; n++) {
                if (!graphPane.stepRow(1, true))
                    graphHoldReport.refused++
            }
            graphHoldReport.start()
        }
    }
    // The hand off the key: the settle behind the last repeat has landed the selection and the card has caught up to
    // it — the same pair `graph_step` waits out, and here also what says the run is over.
    SampleTimer {
        id: graphHoldReport
        property int from: -1
        property int refused: 0
        onTriggered: {
            const row = graphPane.view.currentIndex
            if (row < 0 || graphPane.stepSettling
                    || page.selectedOid !== graphModel.oidAt(row) || !driver.cardSettled)
                return
            graphHoldReport.stop()
            AppBackend.report(
                "graph_hold from=" + graphHoldReport.from
                + " row=" + row
                + " steps=" + graphHoldTimer.steps
                + " reads=" + driver.holdReads
                + " refused=" + graphHoldReport.refused
                + " selected=" + (page.selectedOid === graphModel.oidAt(row))
                + " card=" + (detailsModel.shaHex === page.selectedOid))
            driver.complete()
        }
    }
    /// How many commits the walk has asked for since the hold verb set off. Taken on the signal rather than sampled:
    /// the asks this verb is about are the ones that come and go inside a beat (app-ui.md §UI 自動化の因果性).
    property int holdReads: 0
    Connections {
        target: driver.graphPane
        function onRowActivated(oidHex) { driver.holdReads++ }
    }
    // The file list's arrows: the light and the diff move together, one file per press (規約 §diff のファイル一覧). Two things
    // have to be real for this to say anything, so both go through the door a hand goes through:
    //
    // - the click. The row's own signal is raised by name, not the pane's handler — the handler is where the keyboard
    // is handed to the list, and calling past it would leave `focused=` proving nothing (the same reason `nav-peek`
    // strikes the cell and not `SidebarPane`). - the step, which enters at `stepFile` where `Keys.onDownPressed`
    // enters. A keystroke cannot be injected (verify-ui).
    //
    // Nothing here reaches for the keyboard, and that is the point: the diff opened without taking it, so an arrow
    // still belongs to the list.
    SampleTimer {
        id: fileStepTimer
        /// Which list, `changes` or `wip`, and how far to walk. `overrun` asks for more files than the list holds,
        /// which is how the end it stops at is reached — the count is only known once the commit's details have
        /// arrived, so it cannot be a number set up here.
        property string pane: "changes"
        property int steps: 1
        property bool overrun: false
        /// The file clicked, and the bucket its row sits in (empty for the commit's list, whose files sit in none).
        property string bucket: ""
        property string path: ""
        /// Whether the click has gone out, so the tick that follows is waiting for the diff rather than for the row.
        property bool clicked: false
        property bool stopped: false
        function begin() {
            fileStepTimer.clicked = false
            fileStepTimer.stopped = false
            fileStepTimer.steps = 1
            fileStepTimer.start()
        }
        readonly property var walk:
            fileStepTimer.pane === "wip" ? wipPane.filesWalk : detailsPane.filesWalk
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
            // The click has to have landed before a step means anything: a walk with nothing being read is refused, and
            // reading that as "the end" would go green on a click that never arrived.
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
    // Longer than the settle behind the walk (`keyStepSettleMs`), because what is read is the reading a hand coming off
    // the key gets: the light runs at the key's rate and the diff catches up after it, so a run that moved the light
    // and never moved the diff has to be told apart from one that did (規約 §diff のファイル一覧).
    SampleTimer {
        id: fileStepReport
        onTriggered: {
            const walk = fileStepTimer.walk
            // The diff the walk landed on has been asked for, has arrived, and the row that says which file it is has
            // been built. All three are the output; the step was the cause. The middle one is what keeps the picture
            // worth looking at — a pane still waiting on its read photographs empty.
            if (!page.diffShown || page.diffPath === fileStepTimer.path || !diffPane.diffSettled()
                    || walk.litPath() === "")
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
    // A folder row of the commit's CHANGES tree struck shut, and struck open again (`-unfold`). The strike is the row's
    // own signal, where a click lands — the pane's handler is what carries it to the model, and calling past it would
    // leave the report proving nothing.
    //
    // The two lists that draw a fold arrow keep the answer in different fields (`NameCell.folded`), so `turn=` is read
    // off the icon rather than off either flag: a run that read the flag back would go green with the arrow unwired,
    // which is exactly the shape this verb was cut for.
    SampleTimer {
        id: changesFoldTimer
        /// The directory row struck, and whether the run leaves it shut or strikes it a second time back open. **Read
        /// as a pair** — one arrow on its own says nothing about which way it turned.
        property string path: ""
        property bool reopen: false
        /// How many strikes have gone out. After `n` of them the row is shut exactly when `n` is odd, which is the
        /// wait between one strike and the next: the toggle rebuilds the list, so the row answering a later tick is a
        /// later row.
        property int struck: 0
        function begin() {
            changesFoldTimer.struck = 0
            changesFoldTimer.start()
        }
        /// The folder row for this directory, once the list has built it. Not `FileRowWalk.rowFor`, which answers by
        /// `walkKey` — the name a folder deliberately has none of.
        function folderRow() {
            const view = detailsPane.filesWalk.view
            for (let i = 0; i < view.count; i++) {
                const row = view.itemAtIndex(i)
                if (row && row.isFolder && row.pathText === changesFoldTimer.path)
                    return row
            }
            return null
        }
        onTriggered: {
            const row = changesFoldTimer.folderRow()
            if (!row)
                return
            const strikes = changesFoldTimer.reopen ? 2 : 1
            if (row.isFolded !== (changesFoldTimer.struck % 2 === 1))
                return
            if (changesFoldTimer.struck < strikes) {
                // The commit's own files first, and **read only here**: a read landing after a strike puts the rows
                // back with every fold choice cleared (`DetailsModel::set_files`), so the row swings open under a wait
                // that then never ends (observed 2026-08-23 — 1 run in a handful reached the watchdog in silence).
                // Read every tick instead, and the verb's own answer would break its own precondition.
                if (detailsModel.loading || detailsModel.shaHex !== page.selectedOid)
                    return
                changesFoldTimer.struck++
                row.folderToggled(row.pathText)
                return
            }
            changesFoldTimer.stop()
            AppBackend.report(
                "changes_fold path=" + changesFoldTimer.path
                + " strikes=" + changesFoldTimer.struck
                + " shut=" + row.isFolded
                + " turn=" + row.foldTurn
                + " rows=" + detailsPane.filesWalk.view.count
                + " tree=" + detailsModel.treeView)
            driver.complete()
        }
    }
    // The diff's own arrows, which no headless run can press either: the walk enters where `Keys.onDownPressed` enters
    // (`DiffPane.stepRows`). The hand is walked into the pane first, through the same door the wheel comes in by
    // (`DiffPane.handArrived`) — the diff does not take the keyboard by appearing, so without that the arrows are still
    // the file list's and `focused=` would be false for the right reason (規約 §diff を上下に送る).
    //
    // The wait is for the view, not for the model. `diffSettled()` says the rows arrived; it says nothing about the
    // list having laid them out, and a list whose `contentHeight` is still zero clamps every step to where it already
    // was — the walk then reads exactly like a diff with nothing to scroll (2026-08-16 実測: 1 run in 3 came through with
    // `contentHeight` 0 at the step and 216 by the time it was reported). So what is waited for is the output the step
    // consumes: a view with room to be sent, which is `atEnd` answering false over a laid-out height (app-ui.md §UI
    // 自動化の因果性「まだ答えが無い」と値を分ける).
    SampleTimer {
        id: diffStepTimer
        /// How many rows, and which way.
        property int steps: 1
        onTriggered: {
            if (!page.diffShown || !diffPane.diffSettled() || diffPane.view.height <= 0 || diffPane.atEnd)
                return
            diffStepTimer.stop()
            diffStepReport.from = driver.diffRow()
            diffStepReport.stopped = false
            const way = diffStepTimer.steps < 0 ? -1 : 1
            for (let n = 0; n < Math.abs(diffStepTimer.steps); n++) {
                // A step that moved nothing is the end answering. Read beside `atEnd=`: a walk that was refused every
                // step because the pane was never on screen leaves the view at row 0, which is also where an
                // unscrollable diff sits.
                if (!diffPane.stepRows(way))
                    diffStepReport.stopped = true
            }
            diffStepReport.start()
        }
    }
    SampleTimer {
        id: diffStepReport
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
    /// Where the diff's view stands, in rows — what the walk is counted in, and steadier than a pixel count to read off
    /// a report line.
    function diffRow() {
        return Math.round(diffPane.view.contentY / Theme.rowHeight)
    }
    // The fetch has to land, and its answer reach the chips, before the stacked ones are worth unstacking.
    SampleTimer {
        id: fetchedRefListTimer
        onTriggered: {
            if (repoTab.busyCount !== 0 || repoTab.writeSeq <= driver.writeSeqBefore)
                return
            const stacked = graphPane.view.itemAtIndex(
                Number(AppBackend.autoActArg))
            if (!stacked)
                return
            fetchedRefListTimer.stop()
            graphPane.view.chipExpandRequested(
                stacked.oid_hex, stacked.chipItem.records, stacked.chipItem)
            renderedBarrier.begin()
        }
    }
    // The refusal has to be back and on the button before the second go is sent, and the report is what says it ever
    // got there — the mark is gone again by the time the screenshot is taken.
    SampleTimer {
        id: pushRetryTimer
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
    // Where an operation that answers at the tip left the reader — one report for the three of them. The write, its
    // refresh and the beat the viewport waits out all have to be behind it, and the picture cannot answer the second
    // half: a row can be selected and still be somewhere nobody can see.
    SampleTimer {
        id: tipLandedTimer
        /// The name this run's own write answered by, latched off the answer that carried it rather than read back
        /// at the report — **every** answer rewrites the group it comes from (`RepoTab::settle_write`), so a fetch
        /// settling while the landing is still being waited out takes it away again. The counter having moved says
        /// only that *an* answer arrived; a fetch's answer moves it too.
        ///
        /// Empty until that answer, and the emptiness is the arm. A press made with the selection already sitting at
        /// the tip — a merge from a ref row, a revert of HEAD — satisfies every other reading below before git has
        /// done anything, and the run would quit over an untouched repository (2026-08-22 実測: a copy nobody could
        /// see in the picture, `op=` empty in this very report, green).
        property string answeredOp: ""
        function begin() {
            tipLandedTimer.answeredOp = ""
            tipLandedTimer.start()
        }
        onTriggered: {
            const row = graphModel.rowOf(page.selectedOid)
            // Then the landing that answer armed: at the barrier the refs are still the old ones, so
            // `selected === headOid` holds vacuously until the page's own `pendingHeadSelect` has resolved onto the
            // refreshed pair.
            //
            // And last the pane the landing sends for. The details of the commit that was selected *before* the press
            // are still on the right until its own round trip comes back, and for a merge from a ref row that commit
            // is the old tip — so the half of this the picture does hold, whose commit fills the right-hand pane,
            // frames as the repository before the write (2026-08-23 実測: the pane's second round trip landed after
            // `screenshot saved=true`). Waited out the way `stashLandTimer` waits for it.
            if (tipLandedTimer.answeredOp === "" || page.pendingHeadSelect
                    || repoTab.busyCount !== 0 || row < 0
                    || !graphPane.rowOnScreen(row) || page.selectedOid !== branchesModel.headOid
                    || !driver.cardSettled)
                return
            tipLandedTimer.stop()
            AppBackend.report(
                "tip_landed follows="
                + (page.selectedOid !== "" && page.selectedOid === branchesModel.headOid)
                + " onscreen=" + (row >= 0 && graphPane.rowOnScreen(row))
                // Next to the pair above because that is where the harness reads it: the name is what tells the
                // three verbs' own writes from anything else that could have moved the counter (`must_say`).
                + " op=" + tipLandedTimer.answeredOp
                + " head=" + branchesModel.headOid.substring(0, 8)
                + " selected=" + page.selectedOid.substring(0, 8)
                + " row=" + row)
            driver.complete()
        }
    }
    /// The answer `tipLandedTimer` waits on, taken on the notify rather than on the sampling beat: two answers inside
    /// one beat would leave only the later one to be read, and it is the earlier one that says the write was this
    /// run's (app-ui.md §UI 自動化の因果性 — 一瞬だけ立つ状態は signal で観測して latch する). The *rise* of
    /// `busyCount` is deliberately not waited for anywhere in this chain: a write that begins and ends between two
    /// looks never shows one, and requiring it wedges the run instead (`writeSeqBefore`).
    ///
    /// `writeAtTip` is the bridge's own word for "landed, did not stop part-way, and answers at the tip" — the op
    /// names are turned into meanings on that side of it (`RepoTab::settle_write`), not branched on here. A fetch's
    /// answer, a refusal and a stop all leave the arm down, and the run walks into its watchdog rather than
    /// photographing a repository nothing happened to.
    Connections {
        target: driver.repoTab
        function onWriteSeqChanged() {
            if (!tipLandedTimer.running || tipLandedTimer.answeredOp !== ""
                    || driver.repoTab.writeSeq <= driver.writeSeqBefore || !driver.repoTab.writeAtTip)
                return
            tipLandedTimer.answeredOp = driver.repoTab.lastWriteOp
        }
    }
    // Where a merge that stopped on conflicts left the reader. The other half of `tipLandedTimer`: there is no commit
    // at the tip to land on, and what the press is answered with is the working tree — so this waits for the rows the
    // stop wrote to be on screen, and says in the same breath that nothing called it a failure.
    SampleTimer {
        id: mergeStoppedTimer
        onTriggered: {
            if (repoTab.busyCount !== 0 || !page.wipShown || workTree.conflictCount === 0)
                return
            mergeStoppedTimer.stop()
            AppBackend.report(
                "merge_stopped wip=" + page.wipShown
                + " conflicts=" + (workTree.conflictCount > 0)
                + " error=" + (repoTab.lastError !== "")
                + " log=" + page.commandsOpen
                // The box opened holding what the merge is about to record, and the card is not offering a second
                // door onto the button under it.
                + " msg=" + (workTree.opSubject !== "" && wipPane.subjectText === workTree.opSubject)
                + " cont=" + wipPane.offersOpExit("--continue")
                + " op=" + workTree.opText
                + " files=" + workTree.conflictCount)
            driver.complete()
        }
    }
    // Where a cherry-pick, a revert or a rebase that stopped on conflicts left the reader. `mergeStoppedTimer`'s twin,
    // for the four that step: the same press with no new commit at the tip to land on, answered by the working tree.
    // What the picture cannot hold is the same pair — that nothing wrote a red line over an ordinary conflict, and
    // that the command log stayed down — plus the row the merge does not have: these keep `--continue`, because for
    // them it is a step onward and not the commit somebody is writing (規約 §進行中の操作から出る).
    SampleTimer {
        id: opStoppedTimer
        // Whether a carry is part of this landing. The stash section is refreshed *after* the graph, so reading it
        // at the write barrier answers 0 for a tree whose work is sitting in an entry — and where the entry is the
        // whole claim, that is the answer arriving too early rather than the truth.
        property bool carried: false
        function begin(withStash) {
            opStoppedTimer.carried = withStash
            opStoppedTimer.start()
        }
        onTriggered: {
            if (repoTab.busyCount !== 0 || !page.wipShown || workTree.conflictCount === 0
                    || (opStoppedTimer.carried && stashesModel.total === 0))
                return
            opStoppedTimer.stop()
            AppBackend.report(
                "write_stopped wip=" + page.wipShown
                + " conflicts=" + (workTree.conflictCount > 0)
                + " error=" + (repoTab.lastError !== "")
                + " log=" + page.commandsOpen
                + " cont=" + wipPane.offersOpExit("--continue")
                // What the carry left behind, for a rewrite that took a stash out of its own way: git's words
                // are not raised over the stop, so the count and the graph's own row are the only things saying
                // where the work went (規約 §未コミット変更がある状態で履歴を書き換える).
                + " stashes=" + stashesModel.total
                + " op=" + workTree.opText
                + " files=" + workTree.conflictCount)
            driver.complete()
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
            // still on it with a working-tree row above (2026-08-22 実測).
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
    // The message has to arrive before it can be typed over, and the "is this commit ours to rewrite?" answer before it
    // may be saved.
    SampleTimer {
        id: rewordTimer
        onTriggered: {
            if (!driver.cardSettled)
                return
            rewordTimer.stop()
            // "edit-message-focus" types nothing: the commit's own body is what the caret has to be photographed on top
            // of, and an empty box would only show the placeholder.
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
            // "edit-message-leave" walks away from the unsaved text. Nothing asks any more — the draft goes and the
            // next commit's own message arrives, which is what the shot is of.
            else if (AppBackend.autoAct === "edit-message-leave")
                page.activateRow(graphModel.oidAt(graphModel.rowOf(page.selectedOid) + 1))
            if (AppBackend.autoAct === "reword")
                writeBarrier.start()
            else
                renderedBarrier.begin()
        }
    }
    // gpg / ssh-keygen have to finish before the mark they decide can be on screen, so the shot and the report both
    // wait for them.
    SampleTimer {
        id: signatureTimer
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
    // The tooltip halves of signature-tip / stash-tip: the state has to land (gpg's verdict, the stash's details)
    // before the target is pointed at, and the report then waits out Metrics.tipDelayMs so what it reads is the tip on
    // screen.
    SampleTimer {
        id: signatureTipTimer
        onTriggered: {
            if (!page.signatureIsForSelection)
                return
            signatureTipTimer.stop()
            detailsPane.signaturePointedAt = true
            signatureTipReport.start()
        }
    }
    SampleTimer {
        id: signatureTipReport
        onTriggered: {
            if (!detailsPane.signatureTipShown)
                return
            signatureTipReport.stop()
            AppBackend.report("signature_tip code=" + page.selectedSignatureCode
                              + " tip=" + detailsPane.signatureTipShown)
            driver.complete()
        }
    }
    SampleTimer {
        id: stashTipTimer
        onTriggered: {
            if (!driver.cardSettled)
                return
            stashTipTimer.stop()
            detailsPane.summaryPointedAt = true
            stashTipReport.start()
        }
    }
    SampleTimer {
        id: stashTipReport
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
    // The tooltip half of path-tip: the list has to land before a row can be pointed at, and the report then waits out
    // tipDelayMs so what it reads is the tip on screen. It reads the shared instance itself — the one thing that can
    // also say the words on it.
    SampleTimer {
        id: pathTipTimer
        property bool wipSide: true
        onTriggered: {
            if (pathTipTimer.wipSide && worktreeModel.total === 0)
                return
            if (!pathTipTimer.wipSide && !driver.cardSettled)
                return
            pathTipTimer.stop()
            if (pathTipTimer.wipSide)
                wipPane.pointedTipRow = 0
            else
                detailsPane.pointedTipRow = 0
            pathTipReport.start()
        }
    }
    SampleTimer {
        id: pathTipReport
        onTriggered: {
            const tip = page.ToolTip.toolTip
            if (!tip.visible)
                return
            pathTipReport.stop()
            AppBackend.report("path_tip pane="
                + (pathTipTimer.wipSide ? "wip" : "details")
                + " tree=" + (pathTipTimer.wipSide ? worktreeModel.treeView : detailsModel.treeView)
                + " tip=" + tip.visible
                + " text=" + tip.text)
            driver.complete()
        }
    }
    // The sidebar's row tooltips, and the rows that answer with none. Every run lights a control row first — the
    // WORKTREES row always says where it leads — so a run that photographs an empty overlay has said in the same line
    // that the pointer and the shared instance were both working. Without that, "nothing came out" and "nothing was
    // pointed at" are one picture.
    SampleTimer {
        id: navTipTimer
        property string kind: "branch"
        property int row: 0
        property bool head: false
        /// What the stand-in is put on screen by: a filter its own branch does not answer to. No section overflows its
        /// rows in the sidebar proper (`NavList.Layout.maximumHeight`), so scrolling cannot take the current branch's
        /// row off — the two ways it has no row are a filter and a folded folder (`HeadPinRow`).
        property string hide: ""
        property bool lit: false
        function begin(arg) {
            const parts = ("" + arg).split(":")
            navTipTimer.head = parts[0] === "head"
            navTipTimer.kind = navTipTimer.head || parts[0] === "" ? "branch" : parts[0]
            // A filter, typed once the control has answered: the way the stand-in's own row is taken out of the list,
            // and the way a leaf under a folded folder is brought into it (a filtered row leaves the tree and stands
            // under its full name — `SidebarFilterRow`).
            navTipTimer.hide = navTipTimer.head ? (parts.length > 1 ? parts[1] : "")
                             : (parts.length > 2 ? parts[2] : "")
            navTipTimer.row = !navTipTimer.head && parts.length > 1 ? Number(parts[1]) : 0
            navTipTimer.lit = false
            navTipTimer.start()
        }
        onTriggered: {
            const tip = page.ToolTip.toolTip
            if (!navTipTimer.lit) {
                // Re-applied every beat: the delegate arrives on a later layout than the rows the model got, and a
                // miss reads exactly like a row that wants no tooltip (`NavList.clickRow`).
                sidebarPane.pointTipAt("worktree", 0)
                if (!tip.visible)
                    return
                navTipTimer.lit = true
                sidebarPane.pointTipAt("worktree", -1)
                // The stand-in stands while its own branch has no row of its own — and the filter that takes that row
                // away would take the control row with it, so it goes in only once the control has answered.
                if (navTipTimer.hide !== "")
                    sidebarPane.typeFilter(navTipTimer.hide)
                if (navTipTimer.head)
                    sidebarPane.headPinPointed = true
                return
            }
            const target = navTipTimer.head ? branchesModel.headRow : navTipTimer.row
            if (!navTipTimer.head)
                sidebarPane.pointTipAt(navTipTimer.kind, target)
            // Nothing is attached to the stand-in, so what is read there is that the pointer is on it and the
            // instance went back down.
            const name = navTipTimer.head ? branchesModel.headName
                       : sidebarPane.tipNameAt(navTipTimer.kind, target)
            if (name === "" || (navTipTimer.head && !sidebarPane.headPinLit))
                return
            const words = navTipTimer.head ? sidebarPane.headPinWords
                        : sidebarPane.tipWordsAt(navTipTimer.kind, target)
            // A row with something to say is not photographed until the instance is up; one with nothing to say is not
            // photographed until the control's own tip has left the screen.
            if ((words !== "") !== tip.visible)
                return
            navTipTimer.stop()
            AppBackend.report("nav_tip section=" + (navTipTimer.head ? "head" : navTipTimer.kind)
                + " row=" + target + " name=" + name
                + " lit=" + navTipTimer.lit + " wants=" + (words !== "")
                + " tip=" + tip.visible + " text=" + (tip.visible ? tip.text : "")
                // Last, and after a text that may carry anything: the judged trio above has to stay one substring.
                + (navTipTimer.head ? " pin=" + sidebarPane.headPinLit : ""))
            driver.complete()
        }
    }
    // PG_AUTO_ACT=nav-branch-box / nav-rename-box: the pane is set to the width the argument names, the ref it names
    // is given a box, and the box is left standing for the shot. One timer for both, because what the shot is about —
    // how wide the box comes out on a row of that depth in a pane of that width — is one question asked of the two
    // things the box is opened for.
    SampleTimer {
        id: navNameBoxTimer
        property string mode: "branch"
        property string kind: "branch"
        property string ref: ""
        /// The width the splitter settles at: the ask held to the pane's own floor, since a run asking for less than
        /// the floor is asking what the floor looks like and the answer to that is the floor.
        property real want: 0
        property int step: 0
        readonly property NavSectionModel section:
            navNameBoxTimer.kind === "tag" ? tagsModel
            : navNameBoxTimer.kind === "remote" ? remotesModel : branchesModel
        /// The run walks the list on past the row once the box is standing, to read the one thing a box drawn outside
        /// the list has to answer for: that it goes when its row does.
        property bool away: false
        function begin(mode, arg) {
            const parts = ("" + arg).split(":")
            navNameBoxTimer.mode = mode
            navNameBoxTimer.away = parts[parts.length - 1] === "away"
            navNameBoxTimer.kind = parts[0]
            navNameBoxTimer.ref = parts[1]
            // Left at whatever the session opened with when no width is named.
            const asked = parts.length > 2 && parts[2] !== "away" ? Number(parts[2]) : NaN
            navNameBoxTimer.want = isNaN(asked) ? sidebarPane.width
                                                  : Math.max(asked, sidebarPane.minOpenWidth)
            if (!isNaN(asked))
                page.setSidebarWidth(asked)
            // A remote's root folder starts closed (`models::nav::tree`), so the rows under it are in no list for a
            // menu to be raised on.
            if (navNameBoxTimer.kind === "remote")
                remotesModel.toggleFolder(
                    GitFacts.remoteOfRef(navNameBoxTimer.ref, repoTab.remoteNames))
            navNameBoxTimer.step = 0
            navNameBoxTimer.start()
        }
        onTriggered: {
            // What is on show: a row behind a closed folder answers -1, and so does one whose section has not been
            // read yet.
            const row = navNameBoxTimer.section.rowOfName(navNameBoxTimer.ref)
            if (navNameBoxTimer.step === 0) {
                // The width first. The box is laid out in what the row has left over, so one opened before the pane
                // has been given its width would settle into a width nobody asked about.
                if (row < 0 || Math.round(sidebarPane.width) !== Math.round(navNameBoxTimer.want))
                    return
                if (navNameBoxTimer.mode === "rename") {
                    // What a second click puts in the box, by the same rule the row itself follows (`NavList`): a
                    // remote branch is typed without the remote it lives on, everything else answers to what it shows.
                    const shown = navNameBoxTimer.kind === "remote"
                        ? GitFacts.branchOfRef(navNameBoxTimer.ref, repoTab.remoteNames)
                        : navNameBoxTimer.ref
                    sidebarPane.beginRename(navNameBoxTimer.kind, navNameBoxTimer.ref, shown)
                    navNameBoxTimer.step = 1
                    return
                }
                const oid = navNameBoxTimer.section.oidOfName(navNameBoxTimer.ref)
                // Through the menu, which is the box's only door on this side — and the door that decides between
                // this box and the graph's (`RepoPage.refMenuInSidebar`). The row shows its last segment and answers
                // to the whole name; the menu wants both, and only the second is what git was given.
                page.openRefMenu(navNameBoxTimer.kind, navNameBoxTimer.ref,
                                 navNameBoxTimer.ref, oid, true)
                if (navNameBoxTimer.mode === "tag")
                    page.startTagAt(oid)
                else
                    page.startBranchAt(oid)
                // Dismissed the way choosing a row dismisses it: the box it leaves behind is the subject, and a menu
                // still standing is drawn over the rows beside it.
                refMenu.close()
                navNameBoxTimer.step = 1
                return
            }
            const key = navNameBoxTimer.kind + ":" + navNameBoxTimer.ref
            const list = sidebarPane.listOf(navNameBoxTimer.kind)
            const drawn = list.rowBoxWidth(row)
            if (navNameBoxTimer.step === 1) {
                // On the row, and built: a row the view has not laid out yet answers 0 for its box, the same as a row
                // with no box on it. Whether the box then took the keyboard is reported rather than waited on — a box
                // drawn where nothing can be typed is a real state, and one this picture would not tell from the
                // other.
                if (sidebarPane.editKey !== key || drawn <= 0)
                    return
                if (navNameBoxTimer.away) {
                    // Far enough that the row is out of the list, near enough that the view still holds its delegate —
                    // otherwise what went is the row and not the box.
                    list.scrollRows(4)
                    navNameBoxTimer.step = 2
                    return
                }
            } else if (list.rowBoxShown(row)) {
                // Walked past: what is waited for now is the box going. **Not the box's width** — a delegate the view
                // did let go of answers 0 for everything, and waiting on that number again would wait for ever.
                return
            }
            navNameBoxTimer.stop()
            const whole = list.rowBoxWhole(row)
            AppBackend.report("nav_name_box mode=" + navNameBoxTimer.mode
                              + " ref=" + navNameBoxTimer.ref
                              + " row=" + row
                              + " pane=" + Math.round(sidebarPane.width)
                              // What the row had for it, what it came out at, and what it would take to read whole:
                              // the seat is where the box used to stop, so the three together say whether this box
                              // needed the room outside the pane and whether it got it.
                              + " seat=" + Math.round(list.rowBoxSeat(row))
                              + " box=" + Math.round(drawn)
                              + " whole=" + Math.ceil(whole)
                              // What the picture is being taken for: the placeholder is all there is to say what the
                              // box is for, and a cut one asks nothing (デザイン規約 §グラフ行のダブルクリック).
                              + " cut=" + (Math.ceil(whole) > Math.round(drawn))
                              + " open=" + (sidebarPane.editKey === key)
                              + " focused=" + list.rowFocused(row)
                              + " shown=" + list.rowBoxShown(row)
                              + " at=" + list.rowBoxAt(row))
            driver.complete()
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
                                    AppBackend.autoActArg)
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
            AppBackend.report(
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
            AppBackend.report("find_drop open=" + graphPane.findCard.open
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
                                    AppBackend.autoActArg)
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
            AppBackend.report(
            "avatar email=" + detailsModel.authorEmail
            + " details=" + (detailsModel.avatarUrl !== "")
            + " rows=" + graphModel.avatarRowCount()
            + " error=" + AppBackend.avatarError)
            driver.complete()
        }
    }
    // The card is opened synchronously; this just lets the layout settle before it is measured and photographed.
    SampleTimer {
        id: rowCardTimer
        /// The row the card was asked of, so the report can ask it back whether it is still lit. Read off the row
        /// rather than off the host that wrote it — the whole point is that the row got the answer.
        property int row: 0
        onTriggered: {
            if (!rowCard.opened && !refList.opened)
                return
            rowCardTimer.stop()
            const asked = graphPane.view.itemAtIndex(rowCardTimer.row)
            AppBackend.report(
            // `lit=` sits next to `open=`: the pair is what the run is judged on, and the judge reads one unbroken
            // stretch of the line (`verify::verbs::must_say`).
            "row_card open=" + rowCard.opened
            + " lit=" + (asked ? asked.lit : false)
            + " credit=" + Math.round(rowCard.creditWidth)
            + " cut=" + rowCard.creditCut
            + " list=" + refList.opened
            + " subject=" + (rowCard.subject !== "")
            + " body=" + (rowCard.body !== ""))
            driver.complete()
        }
    }
    // What the row was asked and what it answered, kept for the report — the ask is a point along the row and the
    // answer is which of the two cards came out of it.
    QtObject {
        id: rowPartReport
        property string want: ""
        property real x: 0
    }
    // Waits for either card rather than for the one that was expected: a boundary that moved opens the other one, and
    // waiting for the right answer would spend the whole watchdog finding that out. The rest is a real `tipDelayMs`,
    // which this samples through — the verb is judged on `agrees`, not on how long it took.
    SampleTimer {
        id: rowPartTimer
        onTriggered: {
            if (!refList.opened && !rowCard.opened)
                return
            rowPartTimer.stop()
            const got = refList.opened ? "chip" : "row"
            AppBackend.report(
            "row_part x=" + rowPartReport.x
            + " want=" + rowPartReport.want
            + " got=" + got
            + " list=" + refList.opened
            + " card=" + rowCard.opened
            + " agrees=" + (got === rowPartReport.want
                            && refList.opened !== rowCard.opened))
            driver.complete()
        }
    }
    // PG_AUTO_ACT=graph-reclick / graph-reclick-list / ref-list-pick: the two clicks of the rename gesture put in at a
    // graph row, and at a row of the card its chip unfolds into — and, on the same card, the double-click that is the
    // way to move. Every step waits for its own answer: the row has to exist before it can be clicked, the card has to
    // be up before one of its rows can be, and the double-click window the first click opened has to have passed
    // before a second one counts as a second (app-ui.md §UI 自動化の因果性).
    property bool reclickGraphArmed: false
    property int reclickGraphStep: 0
    /// When the second click went in, so the report can say how long the box made the reader wait. **Not a success
    /// condition** — the wait is the double-click window and the sampler reads through it (app-ui.md §UI 自動化の因果性);
    /// it is here because a lag is the one thing about this gesture a picture cannot show (2026-08-26 ユーザー報告).
    property real reclickGraphAt: 0
    SampleTimer {
        id: reclickGraphTimer
        property int row: 0
        /// The name to put in the box once it is open, for the run that carries the gesture through to git
        /// (`graph-rename`); empty for the one that stops at the box.
        property string name: ""
        /// Whether the run scrolls the history away between the second click and the box. **The delegate is pooled by
        /// that scroll**, which is what the gesture must not be carried by (`ReclickGesture`) — and the box that
        /// opens has to be sent back into view (2026-08-26 ユーザー報告「入力モードに切り替わらないケースも有った」).
        property bool scrolls: false
        /// Whether the run ends inside the wait, on the mark the chip wears while it runs, instead of at the box.
        property bool marks: false
        /// Whether the pointer is rested on the chip first, so the card it opens has a rest running under the wait —
        /// what the gesture has to hold still (2026-08-26 ユーザー指示).
        property bool points: false
        onTriggered: {
            const item = graphPane.view.itemAtIndex(reclickGraphTimer.row)
            // A row the view has not laid out yet is not a row that was clicked: latching here would wait for an
            // answer to a question nobody put.
            if (!item)
                return
            if (driver.reclickGraphStep === 0) {
                // The pointer comes to rest on the chip first, which starts the rest that opens the card
                // (`GraphRowDelegate.restDelay`). Written where a real pointer writes it, so the row makes every
                // decision after that for itself (`row-part`).
                if (reclickGraphTimer.points)
                    item.pointerRowX = item.labelsW - 2
                item.leftClick(0)
                driver.reclickGraphStep = 1
            } else if (driver.reclickGraphStep === 1) {
                // Inside the window the first click opened, a second click is the other half of a double-click and not
                // a second click at all.
                if (item.clickGuarded)
                    return
                driver.reclickGraphAt = Date.now()
                item.leftClick(0)
                // Read where it is set, not where it lapses: the wait is short and the box is what it turns into.
                driver.reclickGraphArmed = item.renameArmed
                // The history walks away under the wait: the row that was clicked is pooled, and what was armed on it
                // has to survive that. Far enough that the row is well outside the view's own buffer.
                if (reclickGraphTimer.scrolls)
                    graphPane.view.positionViewAtIndex(reclickGraphTimer.row + 200, ListView.Beginning)
                if (reclickGraphTimer.marks) {
                    // **The mark is what the reader has to see during the wait**, so this run ends inside it rather
                    // than at the box. Read off the chip itself (`RefChip.waiting`), because a picture taken a beat
                    // late frames the box instead and would say nothing either way.
                    reclickGraphTimer.stop()
                    AppBackend.report(
                    "graph_reclick_mark row=" + reclickGraphTimer.row
                    + " armed=" + driver.reclickGraphArmed
                    + " mark=" + item.chipWaiting
                    + " box=" + (graphPane.namingOid !== "")
                    + " window=" + Application.styleHints.mouseDoubleClickInterval)
                    driver.complete()
                    return
                }
                driver.reclickGraphStep = 2
            } else if (driver.reclickGraphStep === 2) {
                if (driver.reclickGraphArmed && graphPane.namingOid === "")
                    return
                // The box is open: the row it is on has to have been sent back into view before the shot
                // (`RepoPage.startRename`), which the walk does a beat after the box opens.
                if (reclickGraphTimer.scrolls && !graphPane.rowOnScreen(reclickGraphTimer.row))
                    return
                reclickGraphTimer.stop()
                AppBackend.report(
                "graph_reclick row=" + reclickGraphTimer.row
                + " armed=" + driver.reclickGraphArmed
                + " box=" + (graphPane.namingOid !== "")
                // Nothing hover opened or closed under the wait — the rest that was running when the second click
                // landed is part of the same beat (`GraphList.renameWaiting`).
                + " list=" + refList.opened
                + " card=" + rowCard.opened
                + " mode=" + graphPane.namingMode
                + " kind=" + graphPane.namingKind
                + " typed=" + graphPane.namingText
                // The tree has not moved: the gesture the reader made was not the double-click, and this is the half
                // of the report a picture of an open box cannot make (2026-08-26 ユーザー報告).
                + " branch=" + workTree.branch
                // Whether the row the box is on is in sight — the whole of the scrolled run's claim, and true of the
                // plain one for nothing having moved it.
                + " shown=" + graphPane.rowOnScreen(reclickGraphTimer.row)
                // How long the box took, against the window it is waiting out. The sampler's own beat is in the
                // difference, so this is read as "about the window", not to the millisecond.
                + " wait=" + (Date.now() - driver.reclickGraphAt)
                + " window=" + Application.styleHints.mouseDoubleClickInterval)
                if (reclickGraphTimer.name === "") {
                    driver.complete()
                    return
                }
                // Through to git, by the path the field's own Enter takes. The write barrier finishes this one.
                graphPane.view.namingSubmitted(graphModel.oidAt(reclickGraphTimer.row),
                                               reclickGraphTimer.name, graphPane.namingMode)
            }
        }
    }
    // PG_AUTO_ACT=rename-box-out: the ways out of the name box, one route per run. **The claim is what the box and the
    // gesture are left holding**, not how long nothing happened for: a wait still running is the whole of how a box
    // comes back by itself, so `armed=false` is what says it will not (2026-08-26 ユーザー報告 — the box on a row clicked
    // again closed and reopened a window later).
    property int boxOutStep: 0
    SampleTimer {
        id: boxOutTimer
        /// Which way out this run takes: `same-row` / `other-row` / `escape` / `away` / `typed-away`.
        property string route: ""
        onTriggered: {
            const item = graphPane.view.itemAtIndex(0)
            if (!item)
                return
            if (driver.boxOutStep === 0) {
                // The box is opened by the same call the gesture ends in — what this run is about is the way out.
                page.startRename(item.oid_hex, item.renameRecord)
                driver.boxOutStep = 1
            } else if (driver.boxOutStep === 1) {
                if (graphPane.namingOid === "")
                    return
                const route = boxOutTimer.route
                if (route === "same-row") {
                    item.leftClick(0)
                } else if (route === "other-row") {
                    const other = graphPane.view.itemAtIndex(1)
                    if (!other)
                        return
                    other.leftClick(0)
                } else if (route === "escape") {
                    graphPane.view.namingCancelled()
                } else {
                    if (route === "typed-away") {
                        graphPane.view.namingText = "half-written"
                        item.takeNamingFocus()
                    }
                    page.releasePressedAway(null)
                }
                driver.boxOutStep = 2
            } else if (driver.boxOutStep === 2) {
                boxOutTimer.stop()
                AppBackend.report(
                "rename_box_out route=" + boxOutTimer.route
                + " box=" + (graphPane.namingOid !== "")
                // A wait left running is how a box that has just been walked away from comes back on its own.
                + " armed=" + graphPane.view.renameWaiting
                + " guarded=" + graphPane.view.clickGuarded
                + " typed=" + graphPane.namingText)
                driver.complete()
            }
        }
    }
    property bool reclickListArmed: false
    property int reclickListStep: 0
    SampleTimer {
        id: reclickListTimer
        property int row: 0
        property int card: 0
        /// Whether this run is the double-click that moves rather than the two clicks that name.
        property bool picks: false
        /// Whether the two clicks are put in at **different surfaces** — the first at the row, the second at the card
        /// its chip opens into. That is what a reader does without knowing it: the card comes up on the chip's own
        /// seat after a rest, so the second click at one spot lands somewhere else (2026-08-26 ユーザー報告).
        property bool across: false
        onTriggered: {
            if (driver.reclickListStep === 0) {
                const item = graphPane.view.itemAtIndex(reclickListTimer.row)
                if (!item)
                    return
                if (reclickListTimer.across)
                    item.leftClick(0)
                // Hover cannot be injected, so this enters where the row's own rest timer would (`ref-list`).
                graphPane.view.chipExpandRequested(item.oid_hex, item.chipItem.records, item.chipItem)
                driver.reclickListStep = 1
            } else if (driver.reclickListStep === 1) {
                // The card lays its rows out as it is shown; until it is up there is no row to click.
                if (!refList.opened)
                    return
                if (reclickListTimer.picks) {
                    if (!refList.doubleClickRow(reclickListTimer.card))
                        return
                    reclickListTimer.stop()
                    AppBackend.report("ref_list_pick row=" + reclickListTimer.row
                                      + " card=" + reclickListTimer.card
                                      + " list=" + refList.opened)
                    // The write barrier is what finishes this one: the move is the whole of it.
                    return
                }
                // Inside the window the row's click opened, a click here is the other half of a double-click.
                if (reclickListTimer.across && refList.rowGuarded(reclickListTimer.card))
                    return
                if (!refList.clickRow(reclickListTimer.card))
                    return
                if (reclickListTimer.across) {
                    // The row's click was the first: one click on the card is already the second.
                    driver.reclickListArmed = refList.rowArmed(reclickListTimer.card)
                    driver.reclickListStep = 3
                    return
                }
                driver.reclickListStep = 2
            } else if (driver.reclickListStep === 2) {
                if (refList.rowGuarded(reclickListTimer.card))
                    return
                if (!refList.clickRow(reclickListTimer.card))
                    return
                driver.reclickListArmed = refList.rowArmed(reclickListTimer.card)
                driver.reclickListStep = 3
            } else if (driver.reclickListStep === 3) {
                if (driver.reclickListArmed && graphPane.namingOid === "")
                    return
                reclickListTimer.stop()
                AppBackend.report(
                "graph_reclick_list row=" + reclickListTimer.row
                + " card=" + reclickListTimer.card
                + " armed=" + driver.reclickListArmed
                + " box=" + (graphPane.namingOid !== "")
                // The card came down for the box: it was standing on the column the box opens in, and one left up
                // would be covering what the run is about.
                + " list=" + refList.opened
                + " mode=" + graphPane.namingMode
                + " kind=" + graphPane.namingKind
                + " typed=" + graphPane.namingText
                + " branch=" + workTree.branch)
                driver.complete()
            }
        }
    }
    // The row under a standing menu, asked for its card the way its own delay timer would ask (hover cannot be
    // injected — verify-ui スキル §hover の絵の撮り方). Read as a pair with `row-card`, which proves that same input does
    // open the card: on its own, a card that stayed shut says nothing about why.
    //
    // The request goes in only once the menu is actually up — before that there is nothing for the card to be behind —
    // and the answer is read a sampler turn later, since a card that was going to open opens synchronously
    // (`rowCardTimer`).
    SampleTimer {
        id: menuHoverTimer
        property string oidHex: ""
        property bool asked: false
        onTriggered: {
            if (!commitMenu.opened)
                return
            if (!menuHoverTimer.asked) {
                const row = graphPane.view.itemAtIndex(graphModel.rowOf(menuHoverTimer.oidHex))
                // A row the view has not laid out yet is not a row that was asked: latching here would wait for an
                // answer to a question nobody put (app-ui.md §UI 自動化の因果性).
                if (!row)
                    return
                menuHoverTimer.asked = true
                graphPane.view.rowHoverRequested(row, true)
                return
            }
            menuHoverTimer.stop()
            AppBackend.report("menu_hover menu=" + commitMenu.opened + " card=" + rowCard.opened
                              + " list=" + refList.opened)
            driver.complete()
        }
    }
    // The details have to arrive before the name can name anybody. `cut=` leads: judgement is a run (verbs.md).
    SampleTimer {
        id: authorCardTimer
        onTriggered: {
            if (!driver.cardSettled)
                return
            if (AppBackend.autoAct === "author-card-open")
                detailsPane.showAuthor(true)
            if (AppBackend.autoAct === "author-card-open" && !detailsPane.authorCardOpen)
                return
            authorCardTimer.stop()
            AppBackend.report(
                "author_card cut=" + detailsPane.authorNameClipped + " open=" + detailsPane.authorCardOpen
                + " author=" + detailsPane.details.authorEmail
                + " committer=" + detailsPane.details.committerEmail
                + " other=" + detailsPane.details.committerDiffers
                + " later=" + detailsPane.details.commitTimeDiffers)
            driver.complete()
        }
    }
    // The details have to arrive before the credit line they carry can be opened or counted.
    SampleTimer {
        id: coAuthorTimer
        onTriggered: {
            if (!driver.cardSettled)
                return
            if (AppBackend.autoAct === "co-authors-open")
                detailsPane.showCoAuthors(true)
            if (AppBackend.autoAct === "co-authors-open" && !detailsPane.matesCardOpen)
                return
            coAuthorTimer.stop()
            // `open` is the card's own visibility, not the input that asked for it: reporting the input would go green
            // with the binding cut. `cut=` is the credit line's own eliding, read rather than judged (verbs.md).
            AppBackend.report(
                "co_authors count=" + detailsPane.coAuthorRecords.length + " cut=" + detailsPane.coAuthorsClipped
                + " first=" + detailsPane.coAuthorName(0)
                + " open=" + detailsPane.matesCardOpen)
            driver.complete()
        }
    }
    // The details have to arrive, and the column has to be laid out with them, before there is anything to measure.
    SampleTimer {
        id: detailsFitTimer
        // A pane width the splitter left on a fraction can put a fraction in the answer; what this verb is about is
        // tens of pixels.
        onTriggered: {
            if (!driver.cardSettled || detailsPane.width <= 0 || detailsPane.height <= 0)
                return
            detailsFitTimer.stop()
            AppBackend.report(
            "details_fit fits=" + (detailsPane.contentOverflow < 1)
            + " over=" + Math.round(detailsPane.contentOverflow)
            + " pane=" + Math.round(detailsPane.width)
            // The other axis rides along unjudged, the way `edge=` does in `window_fill`: how far the column runs past
            // the pane's own bottom is what says whether this pane needs a scroll of its own, and the answer depends on
            // the window, not on this verb.
            + " overH=" + Math.round(detailsPane.contentOverHeight)
            + " paneH=" + Math.round(detailsPane.height))
            driver.complete()
        }
    }
    // The rows have to arrive, and the list be laid out with them, before what they leave bare is worth measuring.
    SampleTimer {
        id: cornerTimer
        // `shown=` is the label's own visibility, not the room that decided it: reporting what was asked for would go
        // green with the binding cut.
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
    // Same wait as details-fit, for the same reason: the message has to be in the box, and the box laid out with it,
    // before there is a ceiling to pull on.
    SampleTimer {
        id: descGrowTimer
        // Pulled past everything, so where it stops is the bound itself rather than a number this verb chose.
        readonly property int pull: 1000
        /// Which pane's box to pull. The two carry the same box and hooks under the same names, so this verb is written
        /// once.
        property var pane: detailsPane
        property string paneName: "details"
        /// Whether to take the pane's room away again afterwards, by raising the command log under it — the one way a
        /// headless run can make the pane shorter than the box it is already holding.
        property bool squeeze: false
        property int frameBefore: 0
        /// Which end this run is carrying the grip past, or empty for the ordinary pull. Same wait and same box — the
        /// difference is that the grip is in hand, so the box answers instead of just stopping (規約 §掴める境界は答える).
        property string refuse: ""
        onTriggered: {
            if (descGrowTimer.pane.width <= 0 || descGrowTimer.pane.height <= 0 || descGrowTimer.pane.descCap <= 0)
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
    // The layout runs after that handler, so what the pull left behind is read a beat later: asked in the same breath,
    // the list still reports the height it had before it gave any of it up.
    //
    // `grip=` says the corner was offered at all, `keeps=` that what the box borrowed room from is still on screen —
    // the author card in the details pane, the commit button in the editor — which the picture cannot answer, because
    // the overflow draws over the window's own footer.
    SampleTimer {
        id: descGrowSettle
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
    /// Automation: how many rows the graph holds once the fetch the opening fired has landed. Its commit is one only
    /// the remote had (`--preset behind`), so a graph that reaches this many rows without anything being pressed is
    /// the fetch itself, said in the only place a headless run can read it.
    property int openFetchRows: 0
    SampleTimer {
        id: openFetchTimer
        onTriggered: {
            // A state to sample, not a length of time to wait: rows only reach the count after the fetch has landed and
            // the graph has been rebuilt over it, and a run where that never happens has nothing to report.
            //
            // The tab's own word for "the fetch is over" is waited for as well, so the count read below is the settled
            // one and the picture holds a button at rest rather than one caught mid-absorption. Never having seen it
            // turn is allowed: the fetch can be over before this page exists (§通信中(リング)と起動直後の狙い方).
            if (graphModel.rowTotal < driver.openFetchRows || repoTab.autoFetchRunning)
                return
            openFetchTimer.stop()
            AppBackend.report("open_fetch fails=" + repoTab.fetchFailures
                              + " rows=" + graphModel.rowTotal
                              + " wanted=" + driver.openFetchRows)
            driver.complete()
        }
    }
    /// Automation: how long a run of failed fetches the verb asked for, and whether to hold the button that resumes
    /// once it is there.
    property int fetchFailRuns: 0
    property bool fetchResumeAfter: false
    /// The run reached the length it was asked for. Kept apart from the length itself because resuming clears the
    /// count the tab keeps, and a run that read the length again would start a second one over the resumed button.
    property bool fetchRunDone: false
    /// The write this verb's last fetch was asked at, and the run of failures standing when it was asked.
    ///
    /// **A fetch is not admitted the moment it is asked for**: the process is entered a drain later. So between the
    /// drain that finishes one fetch and the drain that admits the next, `busyCount` is 0 and `writeSeq` has already
    /// moved — which reads exactly like a run that has come to rest. `writeBarrier` completed inside that window and
    /// photographed a single failure for every length asked for (実測 2026-08-19: `fetch-fail 3`, `fetch-fail 4` and
    /// `fetch-resume` all came back with the warning shape). What is waited for is the answer to the ask, never the
    /// process alone.
    property int fetchAskSeq: -1
    property int fetchAskFails: -1
    /// The stopped button, read at the moment the hold's slot is fired. Resuming takes it down, and what
    /// `fetch-resume` ends on is the button that came back — so the red half is kept here or it is lost.
    property bool fetchStopped: false
    /// The whole of a run of failed fetches and the resume on the end of it. Asking for the next fetch and judging
    /// that the run is over are the same owner, so the two cannot disagree about whether it is.
    SampleTimer {
        id: fetchFailTimer
        onTriggered: {
            // Nothing is read while git is out — the fetch an opening fires comes through `autoFetchRunning`, the
            // rest through `busyCount` — and nothing is read off an ask still waiting for its answer (`fetchAskSeq`).
            if (repoTab.busyCount !== 0 || repoTab.autoFetchRunning)
                return
            if (driver.fetchAskSeq >= 0 && repoTab.writeSeq <= driver.fetchAskSeq)
                return
            if (!driver.fetchRunDone && repoTab.fetchFailures < driver.fetchFailRuns) {
                // A fetch that came back clean is this verb's premise falling over — the remote is reachable — and
                // asking again cannot make it fail. Say so once and let the watchdog end the run: completing here
                // would hand back a picture of a button that never failed (verbs.md §ヘッドレスで色を確かめる時は
                // デモリモートの URL を疑う).
                if (driver.fetchAskFails >= 0 && repoTab.fetchFailures <= driver.fetchAskFails) {
                    fetchFailTimer.stop()
                    AppBackend.report("fetch_fail reachable=true fails=" + repoTab.fetchFailures)
                    return
                }
                driver.fetchAskFails = repoTab.fetchFailures
                driver.fetchAskSeq = repoTab.writeSeq
                repoTab.fetch("")
                return
            }
            driver.fetchRunDone = true
            if (driver.fetchResumeAfter) {
                driver.fetchResumeAfter = false
                driver.fetchStopped = repoTab.autoFetchSuspended
                driver.fetchAskSeq = repoTab.writeSeq
                // The slot a hold on the stopped button fires (`TopBar` `onHeld`). It clears the run and fetches
                // again by itself, so the ticks after this one wait for that fetch the way they waited for the rest.
                repoTab.resumeAutoFetch()
                return
            }
            fetchFailTimer.stop()
            if (AppBackend.autoAct === "fetch-resume")
                // What the picture cannot hold: the button was stopped when the hold came down, and a fetch ran
                // again after it. The one it ends on is a button back at work, which is the warning shape — the same
                // picture `fetch-fail 1` takes.
                AppBackend.report("fetch_resume stopped=" + driver.fetchStopped
                                  + " fetched=" + (repoTab.fetchFailures > 0)
                                  + " fails=" + repoTab.fetchFailures
                                  + " suspended=" + repoTab.autoFetchSuspended)
            else
                AppBackend.report("fetch_fail stopped=" + repoTab.autoFetchSuspended
                                  + " fails=" + repoTab.fetchFailures
                                  + " wanted=" + driver.fetchFailRuns)
            driver.complete()
        }
    }
    // The commit an automation argument names: an object name as it stands, "row:<n>" read off the graph the way the
    // other row verbs are addressed, and the branch tip when nothing is given. A headless run cannot spell an object
    // name it has not been told, and a demo repository is built fresh every time.
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
            // The button's own path, so the state machine in front of the question is exercised too, not just the
            // question.
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
            // `dialog=` / `name=` say whether the remote dialog stands and what its name box holds — the no-remote push
            // opens it by itself, and only this line can say so headless.
            AppBackend.report("publish state=" + page.pushState
                              + " remote=" + publishFlow.publishRemote
                              + " branch=" + publishFlow.publishBranch
                              + " dialog=" + remoteDialog.visible
                              + " name=" + remoteDialog.wantedName)
        } else if (act === "commit") {
            // A message of its own when none was named: git refuses an empty one outright, and a run that asked for a
            // commit and got a refusal is a picture of the history it did not write. Amend is the one below and needs
            // no such fallback — there an empty message means "keep HEAD's" (`--no-edit`).
            repoTab.stageAll()
            wipPane.setMessage(arg === "" ? "chore: commit from the headless run" : arg, "")
            page.commitNow()
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
            // the highlight still on it (2026-08-22 ユーザー報告). Pressed from the same button all the same.
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
            stashPressTimer.fromWip = act === "stash-lands"
            stashPressTimer.start()
        } else if (act === "stash-file") {
            wipPane.chooseOnly("unstaged", arg)
            page.openFileMenu("unstaged", arg, "")
            fileRowMenu.sendPaths([arg])
            repoTab.stashPaths(wipPane.stashName)
        } else if (act === "stage-all" || act === "unstage-all" || act === "resolve-all") {
            page.showWip()
            bucketAllTimer.begin(act === "stage-all" ? "unstaged"
                                 : act === "unstage-all" ? "staged" : "conflicts")
        } else if (driver.fileRowActs.indexOf(act) >= 0) {
            // Rows first: every one of these names a row of the WIP lists, and the walk that
            // resolves a name reads delegates (`fileRowsTimer`, which then runs `runFileRowAct`).
            page.showWip()
            fileRowsTimer.start()
        } else if (act === "amend-author") {
            // The boxes live in the commit editor, which is only on screen while the uncommitted row is the selected
            // one — without this the run photographs the details pane and says nothing about the amend row.
            page.showWip()
            wipPane.setAmendChecked(true)
            page.amendToggled(true)
            amendAuthorTimer.start()
        } else if (act === "switch") {
            page.switchTo("branch", arg, arg)
        } else if (act === "move-ask") {
            // The other question a move can raise: landing on a remote branch whose local one holds commits of its
            // own. `move-branch` is the same road past this bar; this one stops on it. Waited on at `AskBar.settled`
            // for the reason `switch-stopped` is — `dbl-remote` completes 18ms after the question opens and
            // photographs a marked row under no bar at all (2026-08-22 実測).
            page.switchToRef("R", arg)
            moveAskTimer.start()
        } else if (act === "switch-lands") {
            // A move photographed where it comes to rest. **`switch` cannot do this** — it ends on the write barrier,
            // and core answers a write before the rebuild it asks for (`AfterWrite::Graph`), so that verb's picture is
            // of the branch being left and of the stash count before the carry touched it.
            //
            // The argument is `<branch>[:<stashes>]`, and the count is there because **the branch is not the last
            // thing to arrive**: the stash list is read on its own after the move, so a run that stopped at the branch
            // photographed a carry whose entry was not in the list yet (2026-08-22 実測 — the row reached the graph
            // 70ms after the shot).
            const landing = arg.split(":")
            switchLandsTimer.branch = landing[0]
            switchLandsTimer.stashes = landing.length > 1 ? Number(landing[1]) : -1
            page.switchToRef("L", landing[0])
            switchLandsTimer.start()
        } else if (act === "switch-stopped" || act === "switch-stopped-go"
                   || act === "switch-conflicted" || act === "switch-conflicted-go"
                   || act === "switch-held") {
            // The question a move raises when something is in its way, and the gesture that answers it. **One road for
            // all three shapes** — an operation standing, an unmerged index with none, and a branch another working
            // copy has out — because the press is the same press; only the bar differs, and the verbs are separate so
            // each shape can be claimed on its own. Entered by the ref row's own road (`switchToRef`) rather than by
            // `switchTo`, so the run proves the gate sits where a hand arrives and not only on the last call before
            // the write. The argument is `<branch>[:<stashes>]`, the count meaning what it does for `switch-lands`.
            const leave = arg.split(":")
            switchStoppedTimer.go = act.endsWith("-go")
            switchStoppedLandedTimer.stashes = leave.length > 1 ? Number(leave[1]) : -1
            page.switchToRef("L", leave[0])
            switchStoppedTimer.start()
        } else if (act === "switch-remote") {
            page.switchToRef("R", arg)
        } else if (act === "nav-dbl") {
            // A double-click in the left menu, entered where the row enters it. The argument is `<section>:<name>`.
            const cut = arg.indexOf(":")
            const section = arg.substring(0, cut)
            const rowName = arg.substring(cut + 1)
            const model = section === "tag" ? tagsModel : section === "remote" ? remotesModel : branchesModel
            sidebarPane.activateRow(section, rowName, rowName,
                                    model.oidOfName(rowName))
        } else if (act === "nav-fold" || act === "nav-peek"
                   || act === "nav-unfold" || act === "nav-peek-rename"
                   || act === "nav-peek-away" || act === "nav-peek-into"
                   || act === "nav-peek-out" || act === "nav-peek-shut") {
            // Hover cannot be injected, so the rail cell is named. "-away" walks the pointer off the cell, "-into" down
            // into the opened list, "-out" on out the far side (the exit no cell can see), "-shut" clicks the cell;
            // only "-into" leaves the section standing. "nav-peek" on an empty section must not open at all
            // (NavRail.enterAt decides — `--preset empty` reads that side).
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
                // Typing a name into a peeked row: the box lands on that row, in the section standing beside the rail,
                // and holds it open — the list is not put back for it (SidebarPane.startEdit).
                sidebarPane.peekAt("branch")
                sidebarPane.beginRename("branch", workTree.branch,
                                        workTree.branch)
            }
            navRailTimer.start()
        } else if (act === "tags-eye") {
            // The eye pressed at the band itself, with the list left standing beside the graph: what the switch is
            // about is the graph, and TAGS keeping its count while the graph loses a row is half of what the picture
            // says. The argument names the tag whose commit is the subject — `<tag>` presses once, `<tag>:back`
            // presses again afterwards so the round trip is read rather than one half of it.
            const backing = arg.endsWith(":back")
            const tagName = backing ? arg.substring(0, arg.length - ":back".length) : arg
            driver.tagEyeOid = tagsModel.oidOfName(tagName)
            driver.tagEyeBack = backing
            driver.tagEyeStep = 0
            tagEyeTimer.start()
        } else if (act === "nav-reclick" || act === "nav-reclick-away") {
            // The rename gesture, on the section the folded rail has open. The plain verb clicks the same row twice
            // with that section standing; "-away" lets the pointer leave in between, so the two clicks land in a list
            // that went and came back — and that one must not read as a second click (デザイン規約 §左メニューの所作).
            // The argument is `<section>[:<row>]`.
            page.foldByHand(true)
            driver.reclickAway = act === "nav-reclick-away"
            driver.reclickArmed = false
            driver.reclickStep = 0
            reclickTimer.start()
        } else if (act === "nav-rename-far") {
            // A box on a row the list had scrolled away from. The list has to bring it back (デザイン規約 §左メニューの所作)
            // — a name changing itself off screen is a name nobody agreed to. Entered on the folded rail's section
            // because that is the one list a run can scroll and read back through a single handle, and the row is
            // named the way the menu names it (`beginRename`), not by clicking it.
            page.foldByHand(true)
            farTimer.start()
        } else if (act === "nav-rename-drop") {
            // The box, and the two ways it is walked away from without a word being typed: "fold" takes the list down
            // to the rail, "away" is the press that landed anywhere else (Main's `FocusRelease` enters here).
            sidebarPane.beginRename("branch", workTree.branch,
                                    workTree.branch)
            if (arg === "fold")
                page.foldByHand(true)
            else
                page.releasePressedAway(null)
            AppBackend.report("nav_drop how=" + arg
                              + " collapsed=" + page.sidebarCollapsed
                              + " box=" + (sidebarPane.editKey !== "")
                              + " editing=" + sidebarPane.editKey)
        } else if (act === "nav-add-remote") {
            // The `+` at the end of the REMOTES band, which stays live while the band itself has gone unavailable
            // (デザイン規約 §左メニューの所作). Put in at the band's own signal — clicks cannot be injected (verify-ui スキル) —
            // so what answers is the page's own wiring. `--preset noremote` is the half worth reading: a section of no
            // rows is where this is worth pressing.
            //
            // "folded" is the other door onto the same dialog: the rail's own REMOTES cell, which at zero has no
            // section to open and answers the click with this instead. It goes in at `NavRail.tapAt`, the one answer
            // the cells give (`peekTap`), so the routing being tested is the cell's.
            if (arg === "folded") {
                page.foldByHand(true)
                sidebarPane.peekTap("remote")
            } else {
                sidebarPane.tapAddRemote()
            }
            navAddRemoteTimer.start()
        } else if (act === "push-target") {
            // Nothing to press: the destination is a binding, and this run is about what it says once the repository
            // it is about has finished arriving.
            pushTargetTimer.start()
        } else if (act === "push-default" || act === "remote-menu" || act === "remote-url"
                   || act === "publish-remotes-marked") {
            // `<remote>`, or `<remote>:marked` to put the mark on it first. All three go in at the same doors a hand
            // uses — the slot the menu row calls, the page's one way into the menu, the flow's one way into the form —
            // so what answers is the wiring rather than a second route written for the run.
            const marked = arg.endsWith(":marked")
            // The destination list has no remote of its own to name, so it takes whichever one this repository would
            // send to — the only name a preset-agnostic run can be sure exists.
            driver.remoteTarget = act === "publish-remotes-marked" ? repoTab.defaultRemote
                                : marked ? arg.substring(0, arg.length - ":marked".length) : arg
            driver.markWanted = marked || act !== "remote-menu" && act !== "remote-url"
                                ? driver.remoteTarget : repoTab.pushDefault
            if (repoTab.pushDefault !== driver.markWanted)
                repoTab.setPushDefault(driver.markWanted)
            if (act === "push-default")
                pushDefaultTimer.start()
            else if (act === "remote-menu")
                remoteMenuTimer.start()
            else if (act === "publish-remotes-marked")
                publishMarkedTimer.start()
            else
                remoteUrlTimer.start()
        } else if (act === "nav-close") {
            // The pane keeps sections packed against the top; what is read is where the closed header came to rest — at
            // the foot of the pane is the failure this watches for.
            sidebarPane.closeSection(arg)
            navSectionTimer.start()
        } else if (act === "nav-filter") {
            sidebarPane.typeFilter(arg)
            navFilterTimer.start()
        } else if (act === "nav-tip") {
            // `<section>:<row>`, or `head` for the current branch's sticky stand-in. The pointer goes in at the row's
            // own `pointedTipRow`, the same one the file lists carry.
            navTipTimer.begin(arg)
        } else if (act === "nav-rename" || act === "rename-branch"
                   || act === "rename-tag" || act === "rename-stash") {
            // Which row: the current branch, the first tag, the first stash. "nav-rename" leaves the box standing for
            // the shot.
            const kind = act === "rename-tag" ? "tag" : act === "rename-stash" ? "stash" : "branch"
            const id = kind === "branch" ? workTree.branch
                     : kind === "tag" ? tagsModel.nameAt(0) : stashesModel.fullAt(0)
            const shown = kind === "stash" ? stashesModel.nameAt(0) : id
            sidebarPane.beginRename(kind, id, shown)
            if (act !== "nav-rename")
                sidebarPane.submitEdit(arg)
        } else if (act === "rename-remote" || act === "rename-remote-box"
                   || act === "rename-remote-go") {
            // Named outright (`origin/billing:billing-v2`) because the remote's rows are behind a fold. "-box" leaves
            // the box standing, the plain act stops at the question, "-go" holds the pill to the end.
            const parts = arg.split(":")
            const ref = parts[0]
            const was = GitFacts.branchOfRef(ref, repoTab.remoteNames)
            // "-box" opens with the argument already in it, so a name the remote already carries can be photographed
            // being refused — and the remote's fold has to come open for the row to be there at all (a remote root
            // starts closed).
            if (act === "rename-remote-box")
                remotesModel.toggleFolder(GitFacts.remoteOfRef(ref, repoTab.remoteNames))
            sidebarPane.beginRename("remote", ref,
                                    act === "rename-remote-box" ? parts[1] : was)
            if (act === "rename-remote-box") {
                renderedBarrier.begin()
                return
            }
            sidebarPane.submitEdit(parts[1])
            if (act === "rename-remote-go")
                graphPane.completeHold()
            else
                renameAskTimer.start()
        } else if (act === "rename-local-upstream") {
            // The question about carrying the name over comes back only when git says the local rename landed (so the
            // shot is late).
            const local = workTree.branch
            sidebarPane.beginRename("branch", local, local)
            sidebarPane.submitEdit(arg)
        } else if (act === "delete-branch" || act === "delete-branch-go") {
            // On a branch git refuses, the row turns into the held force-delete, which "-go" then runs to its end.
            page.openRefMenu("branch", arg, arg, branchesModel.oidOfName(arg))
            page.deleteRow("branch", arg, arg, branchesModel.oidOfName(arg))
            refMenu.openSub(refBranchCard)
            if (act === "delete-branch-go")
                forceDeleteTimer.start()
        } else if (act === "set-upstream" || act === "set-upstream-go") {
            // `<branch>[:<name to answer with>]` — `:` cannot be in a ref name (`check-ref-format`), so it separates
            // the two without ambiguity. Without the second half the question stands as it opened, on whatever the
            // branch already speaks for.
            const want = arg.split(":")
            const on = want[0]
            upstreamAskTimer.wantName = want.length > 1 ? want[1] : ""
            upstreamAskTimer.answers = act === "set-upstream-go"
            upstreamAskTimer.typed = false
            // Through the row itself rather than the page's function, so a build where that row stopped reaching the
            // question waits here instead of passing.
            page.openRefMenu("branch", on, on, branchesModel.oidOfName(on))
            refMenu.openSub(refBranchCard)
            refUpstreamItem.triggered()
            upstreamAskTimer.start()
        } else if (act === "delete-gone") {
            // The row and its chip leave at the press, and git is asked behind them (デザイン規約 §消す操作は先に画面から
            // 消す). **A tag, because git refuses no tag delete** — the branch's own half of the rule is the row coming
            // *back* from a refusal, which `delete-branch-refused` photographs. The page holds the in-between open
            // for the shot (`RepoPage.goneHeldForShot`); a demo repository answers before a picture can be grabbed.
            page.openRefMenu("tag", arg, arg, tagsModel.oidOfName(arg))
            refMenu.openSub(refTagCard)
            refTagDeleteItem.completeHold()
            goneRowTimer.start()
        } else if (act === "delete-tag" || act === "delete-tag-go") {
            page.openRefMenu("tag", arg, arg, tagsModel.oidOfName(arg))
            refMenu.openSub(refTagCard)
            if (act === "delete-tag-go")
                refTagDeleteItem.completeHold()
        } else if (act === "delete-stash" || act === "delete-stash-go") {
            page.openRefMenu("stash", stashesModel.nameAt(0),
                             stashesModel.fullAt(0),
                             stashesModel.oidOfName(stashesModel.nameAt(0)))
            if (act === "delete-stash-go")
                refStashDropItem.completeHold()
        } else if (act === "delete-remote" || act === "delete-remote-go") {
            // Named outright (`origin/feature/x`) because those rows sit behind a fold — opened here so the row is
            // under the menu. The remote's own name may hold `/`, so the cut is the configured one
            // (`GitFacts.remoteOfRef`), the same as the rows the verbs drive.
            remotesModel.toggleFolder(GitFacts.remoteOfRef(arg, repoTab.remoteNames))
            page.openRefMenu("remote", arg, arg, remotesModel.oidOfName(arg))
            refMenu.openSub(refBranchCard)
            AppBackend.report("ref_menu kind=remote delete=" + refDeleteItem.code
                              + " " + refDeleteItem.text)
            if (act === "delete-remote-go")
                refDeleteItem.completeHold()
        } else if (act === "delete-force") {
            repoTab.deleteBranch(arg, true)
        } else if (act === "delete-branch-refused") {
            // Same entry as delete-branch; this one waits for git's answer rather than acting on it.
            page.openRefMenu("branch", arg, arg, branchesModel.oidOfName(arg))
            page.deleteRow("branch", arg, arg, branchesModel.oidOfName(arg))
            refMenu.openSub(refBranchCard)
            refusedRowTimer.start()
        } else if (act === "chip-menu") {
            // Only the kind letter and the name of the record are read.
            page.openRecordMenu("L00000" + arg, branchesModel.oidOfName(arg))
            chipMenuTimer.start()
        } else if (act === "chip-menu-current") {
            page.openRecordMenu("L10010" + workTree.branch,
                                branchesModel.oidOfName(workTree.branch))
            chipMenuTimer.start()
        } else if (act === "delete-blocked-tip") {
            // Forced rather than hovered: the pointer cannot be put on a row from here, and this writes to the property
            // the real hover writes to. The argument names the branch, because the delete row is out for more than one
            // reason: without one it is the branch you are standing on, with one it is a branch another working copy
            // has checked out (the flags say which, and the current branch is the only chip that carries them).
            const blockedOn = arg === "" ? workTree.branch : arg
            page.openRecordMenu((arg === "" ? "L10010" : "L00000") + blockedOn,
                                branchesModel.oidOfName(blockedOn))
            refMenu.openSub(refBranchCard)
            refDeleteItem.tipForced = true
            blockedTipTimer.start()
        } else if (act === "switch-mark") {
            // The mark the `switch` row wears when the press ahead of it raises a question rather than moving. The
            // argument is `<branch>:asks` or `<branch>:plain` — **the row is the same row either way**, and a 16px
            // mark in a full window is not something the picture answers (verify-ui §目視).
            const want = arg.split(":")
            switchMarkTimer.want = want.length > 1 ? want[1] : ""
            page.openRecordMenu("L00000" + want[0], branchesModel.oidOfName(want[0]))
            switchMarkTimer.start()
        } else if (act === "menu-highlight") {
            // The keyboard's road to `highlighted` — the only one that can be driven from here.
            page.openRecordMenu("L00000" + (arg === "" ? workTree.branch : arg),
                                branchesModel.oidOfName(
                                    arg === "" ? workTree.branch : arg))
            refMenu.currentIndex = 1
            AppBackend.report("menu_highlight index=" + refMenu.currentIndex)
        } else if (act === "delete-branch-early") {
            // The early answer dresses the delete row before any click; the argument picks which half is on show.
            page.openRecordMenu("L00000" + arg, branchesModel.oidOfName(arg))
            refMenu.openSub(refBranchCard)
            earlyDeleteTimer.start()
        } else if (act === "stash-menu" || act === "delete-stash-row") {
            // The row menu on the first stash's row; the argument "go" holds the delete row down.
            page.openRowMenu(stashesModel.oidOfName(stashesModel.nameAt(0)))
            AppBackend.report("row_menu stash=" + commitMenuState.menuStashRef)
            if (act === "delete-stash-row" && arg === "go")
                stashDeleteItem.completeHold()
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
        } else if (act === "branch-at-tag") {
            sidebarPane.beginBranchAt("tag", tagsModel.nameAt(0),
                                      tagsModel.oidOfName(tagsModel.nameAt(0)))
            sidebarPane.submitEdit(arg)
        } else if (act === "create-tag") {
            // The graph's road, all the way through: the commit menu's row opens the box in the chip column, and what
            // is typed there is what git is finally spawned with. Nothing about it is a shortcut — the row is the one
            // the pointer would press, and the submit is the field's own.
            //
            // The row under HEAD's, counted the way `commit-menu` counts it: the rows above HEAD are whatever else the
            // graph is showing (the working tree, a stash), and neither of them opens this menu at all.
            driver.createTagOid = graphModel.oidAt(graphModel.rowOf(workTree.headOid) + 1)
            page.openRowMenu(driver.createTagOid)
            commitMenu.openSub(commitTagCard)
            tagHereCommitItem.triggered()
            // `<name>:box` stops at the box the row opened, which is the other half of what this verb wires up: the
            // chip column asking the second of its two questions (`GraphRowChips`).
            if (arg.endsWith(":box")) {
                graphPane.view.namingText = arg.slice(0, -4)
                AppBackend.report("create_tag box=" + (graphPane.view.namingOid !== "")
                                  + " mode=" + graphPane.view.namingMode)
                renderedBarrier.begin()
            } else {
                graphPane.view.namingSubmitted(driver.createTagOid, arg, "tag")
                createTagTimer.start()
            }
        } else if (act === "tag-menu" || act === "push-tag"
                   || act === "delete-remote-tag" || act === "delete-tag-both") {
            // The rows a tag's menu grew, and the press that runs one of them. The suffix on the argument says what
            // has to be known before the card is worth reading — `:drift` for the forced push, `:remote` for the
            // delete rows — and both of those are answers only a fetch brings.
            tagMenuTimer.begin(arg,
                               act === "push-tag" ? "push"
                             : act === "delete-remote-tag" ? "remote-delete"
                             : act === "delete-tag-both" ? "both-delete" : "")
        } else if (act === "nav-branch-box" || act === "nav-rename-box" || act === "nav-tag-box") {
            // The two boxes the left menu opens on a row, left standing instead of submitted — the copy of the chip
            // column's box on the side with no lanes to grow into, and the rename box that shares the field with it.
            // The argument is `<section>:<ref>[:<幅>][:away]`: the width is what a hand would drag the pane's own bar
            // to, since what the box is drawn at is the row's share of it and the indent under a folder comes out of
            // that share; `away` walks the list on past the row afterwards.
            navNameBoxTimer.begin(act === "nav-rename-box" ? "rename"
                                : act === "nav-tag-box" ? "tag" : "branch", arg)
        } else if (act === "dbl-local" || act === "dbl-remote") {
            // The record is the chip as drawn (kind letter, four flags, name — see encode.rs).
            page.activateRecord(
                (act === "dbl-local" ? "L00010" : "R00000") + arg)
        } else if (act === "move-branch") {
            // Past the question, for the write it guards.
            page.switchTo("force", repoTab.localNameFor(arg),
                          repoTab.localNameFor(arg), arg)
        } else if (act === "name-branch") {
            graphPane.view.namingSubmitted(graphModel.oidAt(0), arg, "branch")
        } else if (act === "row-part") {
            // Where the row divides, asked at a point along it. Hover cannot be injected, so this writes the one
            // property a real pointer writes (`GraphRowDelegate.pointerRowX`) and leaves every decision after that to
            // the row — **the point of the verb is the decision**, so reaching past it to `chipExpandRequested` (which
            // is what `ref-list` does) would prove nothing about the boundary.
            const parts = arg.split(":")
            const probed = graphPane.view.itemAtIndex(Number(parts[0]))
            if (probed) {
                rowPartReport.want = parts[2]
                rowPartReport.x = Number(parts[1])
                probed.pointerRowX = rowPartReport.x
                rowPartTimer.start()
            }
        } else if (act === "rename-box-out") {
            // The argument is the way out; the row is the first one, which every preset with a chip on it can answer.
            boxOutTimer.route = arg
            boxOutTimer.start()
        } else if (act === "graph-reclick" || act === "graph-rename"
                   || act === "graph-reclick-scrolled" || act === "graph-reclick-mark"
                   || act === "graph-reclick-still") {
            // The gesture at the row itself. The argument is `<行>[:<付ける名前>]` — the first row is the default, and a
            // run that wants a chip on it says which (`--preset tags`). `graph-rename` carries the same gesture
            // through to git; without a name there is nothing to carry.
            const renameCut = arg.indexOf(":")
            reclickGraphTimer.row = Number(renameCut < 0 ? (arg === "" ? "0" : arg) : arg.substring(0, renameCut))
            reclickGraphTimer.name = renameCut < 0 ? "" : arg.substring(renameCut + 1)
            reclickGraphTimer.scrolls = act === "graph-reclick-scrolled"
            reclickGraphTimer.marks = act === "graph-reclick-mark"
            reclickGraphTimer.points = act === "graph-reclick-still"
            reclickGraphTimer.start()
        } else if (act === "graph-reclick-list" || act === "graph-reclick-across" || act === "ref-list-pick") {
            // The same gesture, and the double-click beside it, put in at the card the chip unfolds into. The argument
            // is `<行>[:<カードの行>]` — the card's first row is the one sitting on the chip's own seat.
            const listParts = arg.split(":")
            reclickListTimer.row = listParts[0] === "" ? 0 : Number(listParts[0])
            reclickListTimer.card = listParts.length > 1 ? Number(listParts[1]) : 0
            reclickListTimer.picks = act === "ref-list-pick"
            reclickListTimer.across = act === "graph-reclick-across"
            reclickListTimer.start()
        } else if (act === "ref-list" || act === "ref-list-card") {
            // Hover cannot be injected, so this enters where the hover timer would. `-card` walks row → card → chip →
            // asked again from under the list: both card closes have to hold, and either failing leaves `open=true`.
            const stacked = graphPane.view.itemAtIndex(Number(arg))
            if (stacked) {
                if (act === "ref-list-card")
                    graphPane.view.rowHoverRequested(stacked, true)
                graphPane.view.chipExpandRequested(
                    stacked.oid_hex, stacked.chipItem.records, stacked.chipItem)
                if (act === "ref-list-card") {
                    graphPane.view.rowHoverRequested(stacked, true)
                    rowCardTimer.row = Number(arg)
                    rowCardTimer.start()
                }
            }
        } else if (act === "signature") {
            // The mark appears when the verify comes back, so the report waits for it.
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
            // Row 0 is the elided leaf in the flattened view, the folder chain in the tree (`-tree`). The argument
            // picks the pane the way `corner` does.
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
            // Hover cannot be injected, so this enters where the row's delay timer would.
            const hovered = graphPane.view.itemAtIndex(Number(arg))
            if (hovered)
                graphPane.view.rowHoverRequested(hovered, true)
            rowCardTimer.row = Number(arg)
            rowCardTimer.start()
        } else if (act === "author-card" || act === "author-card-open") {
            // Hover cannot be injected, so `-open` writes the same property the handler writes; read the two as a pair
            // — "stayed shut" only means something next to a run where it opened.
            page.activateRow(graphModel.oidAt(Number(arg)))
            authorCardTimer.start()
        } else if (act === "co-authors" || act === "co-authors-open") {
            // Same pairing as author-card.
            page.activateRow(graphModel.oidAt(Number(arg)))
            coAuthorTimer.start()
        } else if (act === "details-grow" || act === "details-grow-squeeze") {
            // The corner grip pulled past what the pane can spare; `-squeeze` then takes the pane's room back with the
            // log.
            page.activateRow(graphModel.oidAt(Number(arg)))
            descGrowTimer.pane = detailsPane
            descGrowTimer.paneName = "details"
            descGrowTimer.squeeze = act === "details-grow-squeeze"
            descGrowTimer.start()
        } else if (act === "wip-grow" || act === "wip-grow-squeeze") {
            // The argument is the description itself: the box starts empty, so a run that types nothing has nothing to
            // open.
            page.showWip()
            wipPane.setMessage("feat: write the summary", arg)
            descGrowTimer.pane = wipPane
            descGrowTimer.paneName = "wip"
            descGrowTimer.squeeze = act === "wip-grow-squeeze"
            descGrowTimer.start()
        } else if (act === "details-fit") {
            // Overflow shows as glyphs cut at the window's edge, which headless cannot see, so the pane reports the
            // number. `--preset edges` holds the wall.
            page.activateRow(graphModel.oidAt(Number(arg)))
            detailsFitTimer.start()
        } else if (act === "corner") {
            // Both sides of the corner's one rule: preset `basic` leaves the corner bare, `long` runs rows into it.
            // Read as a pair — one half alone frames like a label always on, or always off.
            if (arg === "" || arg === "wip")
                page.showWip()
            else
                page.activateRow(graphModel.oidAt(Number(arg)))
            cornerTimer.start()
        } else if (act === "graph-step" || act === "graph-step-edge"
                   || act === "graph-step-far" || act === "graph-step-named"
                   || act === "graph-step-dirty" || act === "graph-step-diff") {
            // Keystrokes cannot be injected, so the run enters at the same `stepRow` the key handler enters — and takes
            // the keyboard first through the same call a row click makes, since a graph nobody has pressed hears no
            // arrows at all (規約 §矢印で履歴を辿る). `-dirty` writes a half-written message and then walks anyway —
            // nothing holds the selection for a draft. `-edge` walks off the bottom; `-far` sends the view away first
            // so the stepped-off row is off screen. `-diff` opens a file over the graph: the pane swapped off screen
            // has to let the keyboard go, or the arrows walk the selection behind the diff.
            page.activateRow(branchesModel.headOid !== "" ? branchesModel.headOid : graphModel.oidAt(0))
            graphStepTimer.named = act === "graph-step-named"
            graphStepTimer.dirty = act === "graph-step-dirty"
            graphStepTimer.away = act === "graph-step-far"
            graphStepTimer.diffPath = act === "graph-step-diff" ? arg : ""
            graphStepTimer.steps = act === "graph-step-named" ? 1 : act === "graph-step-dirty" ? 2
                                 : act === "graph-step-edge" ? 10 : act === "graph-step-far" ? 1
                                 : act === "graph-step-diff" ? 1 : arg === "" ? 1 : Number(arg)
            graphStepTimer.start()
        } else if (act === "graph-step-hold") {
            // The same door with the key held down: every step behind the first says the key was already down. What it
            // proves is `reads=` — the settle cannot tell a repeat from a press on its own (`GraphRowWalk.noteStep`),
            // and this is the run where it would get it wrong.
            page.activateRow(branchesModel.headOid !== "" ? branchesModel.headOid : graphModel.oidAt(0))
            graphHoldTimer.steps = arg === "" ? 6 : Number(arg)
            graphHoldTimer.start()
        } else if (act === "changes-step" || act === "changes-step-edge"
                   || act === "wip-step") {
            // The file list's arrows: one file per press, the light and the diff moving together (規約 §diff のファイル一覧).
            // `-edge` walks further than the list is long, so the last presses are refused and it stops rather than
            // wrapping. The argument is the file to start on — `<bucket>:<path>` for the working tree's list, where a
            // file changed on both sides has a row under each.
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
                page.activateRow(branchesModel.headOid !== "" ? branchesModel.headOid : graphModel.oidAt(0))
                fileStepTimer.pane = "changes"
                fileStepTimer.bucket = ""
                fileStepTimer.path = arg
            }
            fileStepTimer.overrun = act === "changes-step-edge"
            fileStepTimer.begin()
        } else if (act === "changes-fold" || act === "changes-unfold") {
            // The commit's CHANGES tree opened and shut by its folder rows. The argument is the directory, written the
            // way the row is keyed — a chain with nothing beside it is one row and one key (`a/b/c`) — and it has to
            // name a row the list has actually built, since `itemAtIndex` answers for no other. The default is the
            // first row of the list, which is also the one the picture can hold: a folder struck shut below the fold
            // frames exactly like one left open.
            page.activateRow(branchesModel.headOid !== "" ? branchesModel.headOid : graphModel.oidAt(0))
            // Said rather than assumed: the tree is the list's resting look, but a run that inherited the paths view
            // would wait out the watchdog looking for a folder row that flat paths never put there.
            detailsModel.setTreeView(true)
            changesFoldTimer.path = arg === "" ? "assets/icons" : arg
            changesFoldTimer.reopen = act === "changes-unfold"
            changesFoldTimer.begin()
        } else if (act === "diff-step" || act === "diff-step-edge") {
            // Moves the view, not a selection (規約 §diff を上下に送る). Rides the 320x240 seed: no demo file's diff is longer
            // than a default window, and even there the room below the fold is two rows (実測) — which is why the plain
            // walk is one row.
            page.showWip()
            page.toggleDiff("untracked", arg, "")
            // The hand walks into the pane, through the same door the wheel comes in by: the diff does not take the
            // keyboard by appearing (規約 §diff のファイル一覧), so without this the arrows are still the file list's.
            diffPane.handArrived()
            diffStepTimer.steps = act === "diff-step-edge" ? 20 : 1
            diffStepTimer.start()
        } else if (act === "name-box") {
            // The argument is `<行>[:<列幅>]`. The width is what a hand would drag the chip column's divider to, and
            // the box is drawn against it — `0` asks for the column's own floor, since the metrics clamp what a drag
            // asks for. Without one the column is left wherever it was, which is the default width.
            const boxCut = arg.indexOf(":")
            const boxRow = Number(boxCut < 0 ? arg : arg.substring(0, boxCut))
            if (boxCut >= 0)
                page.setGraphColumns(Number(arg.substring(boxCut + 1)), graphPane.graphColWManual)
            graphPane.startNaming(graphModel.oidAt(boxRow))
            const boxItem = graphPane.view.itemAtIndex(boxRow)
            AppBackend.report("name_box row=" + boxRow
                              + " label_w=" + Math.round(graphPane.labelW)
                              + " box_w=" + (boxItem ? Math.round(boxItem.nameBoxWidth) : -1))
        } else if (act === "name-box-drop") {
            // The same box, and the press that lands somewhere else while it stands. The argument is `<行>[:<打つ名前>]`
            // — with nothing typed the box goes with the press, with a name in it it stays. What is typed goes in the
            // way a recycled delegate puts it back (`GraphRowDelegate.takeNamingFocus`), so the box on screen holds
            // what the run says it holds.
            const nameCut = arg.indexOf(":")
            const nameRow = Number(nameCut < 0 ? arg : arg.substring(0, nameCut))
            const nameTyped = nameCut < 0 ? "" : arg.substring(nameCut + 1)
            graphPane.startNaming(graphModel.oidAt(nameRow))
            if (nameTyped !== "") {
                graphPane.view.namingText = nameTyped
                const nameItem = graphPane.view.itemAtIndex(nameRow)
                if (nameItem)
                    nameItem.takeNamingFocus()
            }
            page.releasePressedAway(null)
            AppBackend.report("name_drop box=" + (graphPane.view.namingOid !== "")
                              + " row=" + nameRow
                              + " typed=" + nameTyped)
        } else if (act === "graph-tail") {
            graphTailTimer.start()
        } else if (act === "graph-head" || act === "graph-head-below"
                   || act === "graph-head-back" || act === "graph-head-go"
                   || act === "graph-head-lit") {
            graphHeadTimer.start()
        } else if (act === "graph-bar" || act === "graph-bar-away"
                   || act === "middle-scroll") {
            // Both want lanes that do not fit their column, and no demo repository has that many — the divider is
            // pulled in the way a person would.
            page.setGraphColumns(graphPane.labelWManual,
                                 Metrics.laneInset + 2 * Metrics.laneW)
            graphPanTimer.start()
        } else if (act === "graph-min") {
            // Pulled past the floor so the clamp answers (the floor is lane 0's co-author badge kept whole).
            page.setGraphColumns(graphPane.labelWManual, 0)
            AppBackend.report("graph_min w=" + graphPane.graphColW
                              + " min=" + graphPane.graphColWMin)
        } else if (act === "divider-refuse") {
            // A drag carried past one of a divider's bounds, named by the argument. The log's bar is not on screen
            // while the log is shut — open it and let it lay out before measuring against a bar that has no geometry
            // yet.
            if (arg === "log-min" && !page.commandsOpen) {
                page.commandsOpen = true
                splitRefuseTimer.start()
            } else if (arg === "desc-max" || arg === "desc-min") {
                // Row 1, not row 0: row 0 of every preset is the uncommitted row, and landing on it puts the working
                // tree in the right-hand pane — the box this pulls on would be off screen.
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
            // Read against two repositories: a line withheld on a linear history is only an answer next to a run where
            // it is drawn.
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
                   || act === "edit-message-focus") {
            // Detached there is no branch tip to name, so the newest row stands in — a commit other than HEAD.
            page.jumpToRef(branchesModel.headOid !== "" ? branchesModel.headOid : graphModel.oidAt(0))
            rewordTimer.start()
        } else if (act === "cherry-pick" || act === "cherry-pick-stops") {
            const pickOid = driver.autoActOid(arg)
            const pickRow = graphModel.rowOf(pickOid)
            if (pickRow >= 0) {
                graphPane.jumpToRow(pickRow)
                page.activateRow(pickOid)
            }
            // Two landings, one press: a copy that goes through answers at the tip, one that stops answers in the
            // working tree (規約 §履歴を合流させる / §進行中の操作から出る).
            if (act === "cherry-pick-stops")
                opStoppedTimer.begin(false)
            else
                tipLandedTimer.begin()
            repoTab.cherryPick(pickOid)
        } else if (act === "reset-soft" || act === "reset-mixed") {
            // With nothing given, the row under HEAD's: a reset to where the branch already stands moves nothing, and
            // an empty name would reach git as `reset ''`.
            page.openRowMenu(arg === "" ? graphModel.oidAt(graphModel.rowOf(workTree.headOid) + 1)
                             : driver.autoActOid(arg))
            page.moveBranchHere(act === "reset-soft" ? "soft" : "mixed")
        } else if (act === "reset-hard" || act === "reset-hard-confirm") {
            // "-confirm" stops with the held row on screen; "reset-hard" runs the hold to its end. The row resolves as
            // reset-soft's.
            page.openRowMenu(arg === "" ? graphModel.oidAt(graphModel.rowOf(workTree.headOid) + 1)
                             : driver.autoActOid(arg))
            resetMenu.offer()
            if (act === "reset-hard")
                hardResetItem.completeHold()
        } else if (act === "commit-menu" || act === "reset-menu"
                   || act === "branch-card" || act === "tag-card") {
            // With no row named, the row under HEAD's: most of this menu is about a commit the branch is *not* already
            // standing on, and it is counted from where HEAD actually sits — the rows above belong to whatever else the
            // graph is showing.
            let menuOid = arg
            if (menuOid === "")
                menuOid = graphModel.oidAt(
                    graphModel.rowOf(workTree.headOid) + 1)
            page.openRowMenu(menuOid)
            if (act === "reset-menu")
                resetMenu.offer()
            else if (act === "branch-card")
                commitMenu.openSub(commitBranchCard)
            else if (act === "tag-card")
                commitMenu.openSub(commitTagCard)
            AppBackend.report("commit_menu rows=" + commitMenu.offeredRows
                              + " can_move=" + commitMenuState.menuCanMoveBranch)
        } else if (act === "menu-hover") {
            // Same row and same default as `commit-menu`: the menu goes up, and then the row it is standing on is
            // asked for its hover card.
            let hoverOid = arg
            if (hoverOid === "")
                hoverOid = graphModel.oidAt(graphModel.rowOf(workTree.headOid) + 1)
            page.openRowMenu(hoverOid)
            menuHoverTimer.oidHex = hoverOid
            menuHoverTimer.asked = false
            menuHoverTimer.start()
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
        } else if (act === "drop-commit" || act === "drop-commit-go" || act === "drop-stops") {
            // The plan is built by object name, the way a graph row hands one over — a symbolic name is not what this
            // takes.
            page.openRowMenu(driver.autoActOid(arg))
            AppBackend.report("drop_row " + dropCommitItem.code
                              + " " + dropCommitItem.text
                              + " oid=" + commitMenuState.menuOid.substring(0, 8)
                              + " hold=" + (dropCommitItem.holdMs > 0)
                              + " reached=" + repoTab.headReachedElsewhere)
            if (act !== "drop-commit") {
                // "drop-stops" is the replay that walks into a hole and stops with the work still in the stash it
                // took: the landing is the working tree, and the count and the stash's own row are what say where
                // that work went (規約 §未コミット変更がある状態で履歴を書き換える の着地表).
                if (act === "drop-stops")
                    opStoppedTimer.begin(true)
                if (dropCommitItem.holdMs > 0)
                    dropCommitItem.completeHold()
                else
                    page.dropCommit(commitMenuState.menuOid)
            }
        } else if (act === "merge-branch" || act === "merge-stops" || act === "rebase-onto"
                   || act === "rebase-stops" || act === "revert-commit" || act === "revert-stops"
                   || act === "integrate-menu") {
            // Through the menus a right-click opens, so the rows' own gating decides whether anything runs.
            if (act === "revert-commit" || act === "revert-stops") {
                // The click that opens this menu selects the row too (GraphRowDelegate), so the hook takes both steps a
                // right-click takes.
                const oidHex = driver.autoActOid(arg)
                graphPane.jumpToRow(graphModel.rowOf(oidHex))
                page.activateRow(oidHex)
                page.openRowMenu(oidHex)
                // The undo's two landings, read like the copy's above.
                if (act === "revert-stops")
                    opStoppedTimer.begin(false)
                else
                    tipLandedTimer.begin()
                repoTab.revert(oidHex)
            } else {
                page.openRefMenu("branch", arg, arg,
                                 branchesModel.oidOfName(arg))
                if (act === "merge-branch" || act === "merge-stops") {
                    // Two landings, one press: a merge that goes through answers at the tip, and one that stops
                    // answers in the working tree (規約 §履歴を合流させる / §進行中の操作から出る).
                    if (act === "merge-stops")
                        mergeStoppedTimer.start()
                    else
                        tipLandedTimer.begin()
                    repoTab.merge(arg, false, false, "")
                } else if (act === "rebase-onto" || act === "rebase-stops") {
                    // A replay that stopped part-way answers in the working tree like the other three: no commit
                    // was written, and the badge, the exit card and the conflicted rows are where the press ends
                    // (規約 §未コミット変更がある状態で履歴を書き換える の着地表).
                    if (act === "rebase-stops")
                        opStoppedTimer.begin(false)
                    repoTab.rebase(arg, "", true)
                }
            }
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
            // photograph the same card while leaving the ordering that actually broke it untested (2026-08-18).
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
        } else if (act === "stage-hunk" || act === "stage-line"
                   || act === "discard-hunk" || act === "discard-hunk-go"
                   || act === "diff-file" || act === "conflict-sides" || act === "line-tools"
                   || act === "hunk-tools" || act === "keep-place" || act === "diff-tick"
                   || act === "code-send" || act === "line-back"
                   || act === "diff-follow" || act === "line-run") {
            // All enter through one file's diff and act on its first hunk. The bucket rides in front of the path
            // (`<bucket>:<path>`) when it is not the usual unstaged one: an untracked file has no unstaged diff at all,
            // a conflicted one is read from `conflicts`. Only the bucket names count as one, so a path carrying a colon
            // still opens.
            const cut = arg.indexOf(":")
            const head = cut > 0 ? arg.substring(0, cut) : ""
            const named = head === "staged" || head === "unstaged"
                          || head === "untracked" || head === "conflicts"
            page.showWip()
            // The source of a rename comes off the model rather than out of the argument: a row hands it over when it
            // is clicked, and a run that opened the destination alone would photograph a file git thinks appeared out
            // of nowhere.
            const wtPath = named ? arg.substring(cut + 1) : arg
            // A diff is of a file, and every way to open one on screen carries its name — so an argument that carries
            // none is the run's own mistake, and it has to read as one. **git will not call it an error**: an empty
            // pathspec matches the whole tree, so the pane fills with a diff that reads exactly like the file's, and a
            // fixture with one changed file in it renders down to the same rows. It parts company at the first write:
            // the tree moves, the list is asked about a path it never held, and the pane closes — correctly — on a
            // reader who was never on a file. What that leaves is a verb reporting rows it never owned, hours after
            // the argument it wanted was left off (2026-08-18 実測: `line-run` staged one line of nothing and read
            // `rows=0`, `line-back` waited out the watchdog).
            if (wtPath === "") {
                AppBackend.report("diff_arg act=" + act + " named=false")
                renderedBarrier.begin()
                return
            }
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
            // "-by-hand" brings the list back and so takes the diff down; "-by-rename" is the other side of that rule
            // — a box opened from a peeked row stands in the peek, so neither the fold nor the file goes anywhere.
            // "-keep-folded" had it folded before the diff arrived, so closing the diff leaves it folded.
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
            // A mark coming off is the absence of a thing, so the timer reports the refused state before sending the go
            // that clears it.
            page.pushNow()
            pushRetryTimer.start()
        } else if (act === "fetch" || act === "fetch-busy") {
            // `-busy` is the same fetch; what differs is who says the run is over — the band, once it has latched the
            // ring (`WindowAutoActDriver`).
            repoTab.fetch("")
        } else if (act === "fetch-ref-list") {
            // What a tag says about the remote only exists after a fetch (`ls-remote --tags` carries it), so the two
            // steps are one verb.
            repoTab.fetch("")
            fetchedRefListTimer.start()
        } else if (act === "preview" || act === "preview-unstaged" || act === "preview-staged") {
            // The toggle only asks; the read is a git subprocess away, so completion is the pane settling
            // (`stageRowTimer`), not the ask. No path is a run with nothing to open: said and stopped on the rendered
            // surface, rather than holding a wait no read will answer — the wanted line is what fails it.
            if (arg === "") {
                AppBackend.report("diff_arg act=" + act + " named=false")
                renderedBarrier.begin()
            } else {
                page.toggleDiff(act === "preview" ? "untracked"
                                : act === "preview-unstaged" ? "unstaged" : "staged", arg, "")
                stageRowTimer.begin()
            }
        } else if (act === "open-picker") {
            page.openRepositoryPicker()
        } else if (act === "settings" || act === "settings-tools"
                   || act === "settings-tools-loading") {
            // `-tools` goes on to open the candidate list from inside the dialog, and leaves the fetch interval where
            // it was.
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
            AppBackend.report("find open=" + graphPane.findCard.open
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
        } else if (act === "commands") {
            // Stage and unstage so the log has something in it.
            repoTab.stageAll()
            repoTab.unstageAll()
            page.toggleCommands()
        } else if (act === "commands-fail" || act === "commands-clear" || act === "commands-fail-shut") {
            // A real refusal in git's own words, raising the panel by itself. The clearing verb starts from the same
            // failure (`Main` waits for it, presses Clear, and reads the band); the shutting one takes the panel back
            // down with the `>_` instead, which leaves the error line standing and the mark red.
            if (act === "commands-fail-shut" && arg === "fold")
                page.foldByHand(true)
            repoTab.checkoutBranch("pg-no-such-branch", false)
            if (act === "commands-fail-shut")
                commandsShutTimer.start()
        } else if (act === "fetch-recover") {
            // A fetch that cannot land leaves a failure standing; `Main` then fires one that can and reads what the
            // success takes down by itself.
            repoTab.fetch("pg-no-such-remote")
        } else if (act === "fetch-fail") {
            // The argument is how many failed fetches to run, so one verb reaches the warning shape and the stopped one
            // alike. The fetches are asked for by `fetchFailTimer`, which is also what ends the run.
            driver.fetchFailRuns = Math.max(1, Number(arg))
            AppBackend.setAutoFetchMinutes(0)
            AppBackend.setAutoFetchMinutes(5)
            fetchFailTimer.start()
        } else if (act === "fetch-resume") {
            // Long enough a run to stop the timer, so the hold has a stopped button to come down on.
            driver.fetchFailRuns = 3
            driver.fetchResumeAfter = true
            AppBackend.setAutoFetchMinutes(0)
            AppBackend.setAutoFetchMinutes(5)
            fetchFailTimer.start()
        } else if (act === "open-fetches") {
            // Nothing is pressed here: the fetch the opening fires is the whole verb, and the argument is how many rows
            // the graph holds once it has landed.
            driver.openFetchRows = Math.max(1, Number(arg))
            openFetchTimer.start()
        }
        AppBackend.report("auto_act ran=" + act)
        // The file-row acts have not acted yet — they are waiting on their rows (`fileRowsTimer`),
        // and finishing here would photograph the scene before the menu is up. Their sampler
        // finishes the dispatch after `runFileRowAct` has run.
        if (driver.fileRowActs.indexOf(act) < 0)
            driver.dispatchFinished()
    }

    /// The first-push surface is either the standing question (a remote exists) or the add-remote dialog (none does).
    /// Do not wait for a remote check in the latter case: there is no target to check yet.
    SampleTimer {
        id: publishSurfaceTimer
        onTriggered: {
            if (!remoteDialog.visible && !publishFlow.publishChecked)
                return
            publishSurfaceTimer.stop()
            driver.complete()
        }
    }
    /// `publish-remotes` is about the popup, not merely the call which requested it. The form is created asynchronously
    /// with the ask bar.
    SampleTimer {
        id: publishRemotesTimer
        onTriggered: {
            if (!publishFlow.publishRemotesOpen()) {
                publishFlow.openPublishRemotes()
                return
            }
            publishRemotesTimer.stop()
            driver.complete()
        }
    }
    /// `publish-add` stops with the real dialog on screen. A check that happens to finish behind it is unrelated and
    /// must not end the run.
    SampleTimer {
        id: publishDialogTimer
        onTriggered: {
            if (!remoteDialog.visible)
                return
            publishDialogTimer.stop()
            driver.complete()
        }
    }
    /// Automation: the dialog's own button, once it is both visible and valid. This is the `-go` path; an empty URL
    /// cannot be submitted.
    SampleTimer {
        id: publishNewTimer
        onTriggered: {
            if (!remoteDialog.visible || remoteDialog.wantedName === "" || remoteDialog.wantedUrl === "")
                return
            publishNewTimer.stop()
            driver.writeSeqBefore = repoTab.writeSeq
            remoteDialog.submit()
            publishAnswerTimer.start()
        }
    }
    /// Automation: what the far side turned out to hold, once the remote has had time to answer.
    SampleTimer {
        id: publishSettleTimer
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
    /// Automation: the answer, given after the remote has had time to say what it has — the pill is dead until it has.
    SampleTimer {
        id: publishAnswerTimer
        onTriggered: {
            if (!publishFlow.publishChecked || !graphPane.askAnswerable)
                return
            publishAnswerTimer.stop()
            // `far` is what the far side turned out to hold — the other line's `state` is this end's own push state,
            // and the two answer different questions.
            AppBackend.report("publish answering far="
                              + publishFlow.publishState
                              + " unsure=" + publishFlow.publishUnsure
                              + " answerable=" + graphPane.askAnswerable)
            // The same gesture a person is given: a hold cannot be answered by a click here either.
            driver.writeSeqBefore = repoTab.writeSeq
            if (publishFlow.publishRefused)
                graphPane.completeHold()
            else
                page.answerRowAsk()
            writeBarrier.start()
        }
    }
    /// PG_AUTO_ACT=set-upstream…: the question about what a branch is measured against (`UpstreamFlow`). The bar has
    /// to be all the way down before there is a box to answer into — the form is loaded as the bar opens — and that
    /// is the picture's own moment as well (`AskBar.settled`).
    SampleTimer {
        id: upstreamAskTimer
        /// Whether this run answers the question or only photographs it.
        property bool answers: false
        property string wantName: ""
        property bool typed: false
        onTriggered: {
            if (!graphPane.askSettled)
                return
            if (upstreamAskTimer.wantName !== "" && !upstreamAskTimer.typed) {
                upstreamFlow.setBranchName(upstreamAskTimer.wantName)
                upstreamAskTimer.typed = true
                return
            }
            if (upstreamAskTimer.answers && !graphPane.askAnswerable)
                return
            upstreamAskTimer.stop()
            // `there=` is whether this repository actually holds what was answered, which is what decides the pill —
            // and the refused form is the frame and the line, neither of which a full-window picture settles.
            AppBackend.report("upstream branch=" + upstreamFlow.branch
                              + " remote=" + upstreamFlow.remote
                              + " name=" + upstreamFlow.branchName
                              + " there=" + upstreamFlow.targetIsThere
                              + " answerable=" + graphPane.askAnswerable)
            if (!upstreamAskTimer.answers) {
                driver.complete()
                return
            }
            driver.writeSeqBefore = repoTab.writeSeq
            page.answerRowAsk()
            writeBarrier.start()
        }
    }
    // PG_AUTO_ACT=commands-fail-shut: the mark's red with the panel out of the way, which is the state no other verb
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
            AppBackend.report("commands_shut wrong=" + page.commandsWrong
                              + " open=" + page.commandsShown
                              + " folded=" + page.sidebarCollapsed
                              + " mark=" + page.commandsMarkColor)
            renderedBarrier.begin()
        }
    }
    // The log has to be on screen and laid out before the bar above it has a place to be measured from.
    SampleTimer {
        id: splitRefuseTimer
        onTriggered: {
            if (!page.commandsOpen || !page.commandsShown)
                return
            splitRefuseTimer.stop()
            page.reportDividerRefusal(AppBackend.autoActArg)
            renderedBarrier.begin()
        }
    }
}
