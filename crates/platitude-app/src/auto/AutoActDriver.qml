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
    property RebasePlanPane planPane
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
    /// The `rebase` row, read where a verb has to say what its note says about the range (`integrate-menu`).
    property AppMenuItem refRebaseItem
    property AppMenuItem refPushTagItem
    property AppMenuItem refTagHereItem
    property AppMenuItem refTagDeleteItem
    property AppMenuItem refRemoteTagDeleteItem
    property AppMenuItem refTagBothDeleteItem
    property FileRowMenu fileRowMenu
    property AppMenu fileMenu
    property AppMenuItem fileDiscardItem
    property DiffRowMenu diffRowMenu
    /// What reached the clipboard, which the clipboard itself will not say. The one place a copy verb can read its
    /// own answer back (`ClipboardHelper.lastCopied`).
    property ClipboardHelper clipboard
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
    /// The delete row inside that branch card — the same row the sidebar's menu carries, read where a verb has to say
    /// what the graph row's own entrance offered (`chip-menu`).
    property AppMenuItem commitDeleteItem
    property AppMenuItem switchCommitItem
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
    /// post-write refresh made likelier still (measured, `line-back`, then `keep-place`).
    property int writeSeqBefore: 0
    /// The working-tree row this run's write takes out of its bucket, as `<bucket>:<path>` — or "" for the verbs the
    /// write barrier alone answers for.
    ///
    /// A write answers before the status it invalidated has been read again: core reports `WriteFinished` and *then*
    /// publishes status and refs (`session::write::run_write`), and `writeSeq` is counted off that report
    /// (`drain::settle_write`). So a shot taken at the write barrier is a shot of the file list as it was — the same
    /// shape `graphGoneOid` below answers for on the graph. That is how a run whose merge editor resolved a file and a
    /// run whose editor never started came to save the same picture, down to the sha256.
    ///
    /// Waited out on the row itself rather than on a status counter, for the reason the graph gives: a counter also
    /// moves for statuses nobody here asked for, while this row moves only for this write.
    property string treeGoneRow: ""
    /// The graph row this run's write takes off the graph, or "" for the verbs the write barrier alone answers for.
    ///
    /// A write answers before the rebuild it asks for is even started (core's `AfterWrite::Graph`), so a shot taken at
    /// the write barrier is a shot of the graph as it was — two rows can wear each other's marks in it (a popped
    /// stash's archive box on the working-tree row, the dashed ring on the entry just made). Waiting on the row itself
    /// — gone from the model — rather than on a pass counter keeps the wait about this write: the counter also moves
    /// for passes nobody here asked for.
    ///
    /// **Stash verbs only as it stands**: `graphBarrier` also holds for the stash total moving, so a non-stash write
    /// that set this would wait out the watchdog. Widen the barrier before pointing a new verb at it.
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

    function prepareCompletion(act) {
        driver.completionDeferred = completion.defersCompletion(act)
        driver.writeExpected = completion.isWriteAct(act)
        driver.writeSeqBefore = repoTab.writeSeq
        driver.treeGoneRow = ""
        driver.graphGoneOid = ""
        driver.stashTotalBefore = stashesModel.total
        driver.stashWanted = ""
        driver.popWanted = ""
    }

    /// Whether the entry now at the top of the list is wearing the summary this run typed. Read off the sidebar's own
    /// model — the reflog subject git wrote — so a name that never left the box answers `false`. git puts its own
    /// `On <branch>: ` in front of a named entry (measured), which is why this is a tail and not an equality.
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

    /// Holds the write barrier for a verb whose press goes in from a
    /// sampler, tick(s) after the dispatch: the fetch a repository does on
    /// the way open moves `writeSeq` on its own, so a barrier armed with
    /// the dispatch-time sequence can pass — and photograph — before
    /// anything was pressed. Until [`pressedWrite`] re-arms it, no
    /// sequence reads past this.
    function expectWriteAtPress() {
        // 2^31−1: the property is a QML int, and a wider sentinel wraps
        // negative — which opens the barrier instead of holding it.
        driver.writeSeqBefore = 2147483647
    }
    /// The press went in (call it right after the successful press: the
    /// answer that moves `writeSeq` cannot land inside the same tick).
    function pressedWrite() {
        driver.writeSeqBefore = repoTab.writeSeq
    }
    /// Past the end of any line these fixtures carry: `hit_byte` clamps, so a drag that means "to the end of the row"
    /// can say so without measuring the row.
    readonly property int pastLineEnd: 9999

    // AutoShotDriver owns the final render boundary: it requests an update, advances the event loop, and waits for
    // grabToImage callbacks. Do not wait for frameSwapped here. A quiet scene is allowed not to emit one (the pilot
    // reproduced that hang twice under concurrent load).
    QtObject {
        id: renderedBarrier
        function begin() {
            driver.complete()
        }
    }
    readonly property alias barrierRendered: renderedBarrier
    // `writeSeq` moving past the armed sequence proves the write's answer was absorbed; `busyCount === 0` is the
    // quiet condition on top — nothing else this run started is still in flight. (Waiting for busy to *rise* would
    // wedge: the answer can be absorbed before this sampler ever sees the flag up.)
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
    readonly property alias barrierWrite: writeBarrier
    /// The report a write that did not happen comes down in — **waited on all the way down**
    /// (`NoticeBar.settled`), not at the write barrier: the answer to the write is what raises the bar, so a picture
    /// taken on the answer catches a bar whose words are written and whose height is still nothing (the edge
    /// `AskBar.settled` names, for the same reason).
    ///
    /// **One barrier for every kind of report** (`Words.writeReported`) — a tag the far side keeps, a commit a hook
    /// declined, a push that is only out of date — because what each of those runs claims is the same three things:
    /// the bar came down, it was written out of what core classified, and no error was raised beside it.
    ///
    /// The words themselves are reported rather than photographed for the second half: whoever said no writes
    /// sentences that wrap, and a report line cannot hold what the picture holds.
    SampleTimer {
        id: noticeBarrier
        onTriggered: {
            if (!page.noticeCard.settled)
                return
            noticeBarrier.stop()
            // `log=` and `wrong=` are the other half of the claim, and the half no picture can make on its own: the
            // panel did not raise itself over the same news, and the mark in the corner is not calling it an error
            // (デザイン規約 §答えの要らない報せ). A window that never opened the log frames exactly like one that opened and
            // closed it.
            Harness.report("write_notice open=" + page.noticeCard.open
                              + " clears=" + page.noticeClears
                              + " why=" + (page.noticeCard.detail !== "")
                              + " tone=" + (page.noticeCard.tone === "" ? "none" : page.noticeCard.tone)
                              + " log=" + page.commandsOpen
                              + " wrong=" + page.commandsWrong
                              + " said=" + page.noticeCard.label)
            driver.complete()
        }
    }
    readonly property alias barrierNotice: noticeBarrier
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
            Harness.report("tree_settled from=" + from
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
            Harness.report("graph_settled gone=true top=" + driver.graphTopKind()
                              + " named=" + driver.stashNamed()
                              + " back=" + driver.stashCameBack()
                              + " rows=" + graphModel.rowTotal
                              + " stashes=" + stashesModel.total)
            if (Harness.autoAct === "stash-lands")
                driver.awaitStashLanding()
            else
                renderedBarrier.begin()
        }
    }
    /// The two landings a press can have. A file of their own: both wait on the far side of the write barrier — core
    /// answers a write before it publishes what that write invalidated — and each has a report of its own to make about
    /// where the reader was put (`AutoActLandings`).
    AutoActLandings { id: landings; driver: driver }
    /// The edges the page's own wiring puts out, written as report lines. A file of their own for the reason the
    /// landings are one: none of them is a state a later reading could recover (`PageReports`).
    PageReports { driver: driver }
    /// Arms the stash landing — called once the graph the row went out of has settled.
    function awaitStashLanding() {
        landings.awaitStash()
    }
    /// Arms the stopped-operation landing — called right after the press that puts the operation down.
    function awaitOpExitLanding() {
        landings.awaitOpExit()
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
            // `Opened` only means the path was accepted. Refs, the graph and where HEAD stands are the baseline every
            // page verb is allowed to act on.
            if (repoTab.state !== "open" || !workTree.loaded || !workTree.headKnown
                    || !branchesModel.refsLoaded || graphModel.finishCount === 0)
                return
            autoActTimer.stop()
            driver.runAutoAct()
        }
    }
    // The commit an automation argument names: an object name as it stands, "row:<n>" read off the graph the way the
    // other row verbs are addressed, and the branch tip when nothing is given. A headless run cannot spell an object
    // name it has not been told, and a demo repository is built fresh every time.
    function autoActOid(arg) {
        if (arg === "")
            return workTree.headOid
        if (arg.indexOf("row:") === 0)
            return graphModel.oidAt(Number(arg.substring(4)))
        return arg
    }

    // Which verbs write and which defer their completion: two lists, read only from `prepareCompletion`.
    AutoActCompletion { id: completion }


    /// The verbs, grouped by what they act on. Each family gates nothing of its own: the dispatch below asks
    /// them in turn and the first to know the verb runs it (`run`).
    // Handed the driver and nothing else — what a verb reaches into, it reads back off this one property.
    AutoActWipVerbs { id: wipVerbs; driver: driver }
    AutoActFileRowVerbs { id: fileRowVerbs; driver: driver }
    /// What a run does to the sidebar and reads back off it, composed from the three children the pane hands over.
    /// One for the two nav families, so a peek one of them opened is the peek the other reads.
    NavProbe {
        id: navigation
        sidebar: driver.sidebarPane
    }
    readonly property alias navProbe: navigation

    AutoActNavVerbs { id: navVerbs; driver: driver }
    AutoActNavBoxVerbs { id: navBoxVerbs; driver: driver }
    AutoActRefVerbs { id: refVerbs; driver: driver }
    AutoActSwitchVerbs { id: switchVerbs; driver: driver }
    AutoActPublishVerbs { id: publishVerbs; driver: driver }
    AutoActGraphRowVerbs { id: graphRowVerbs; driver: driver }
    AutoActStepVerbs { id: stepVerbs; driver: driver }
    AutoActPaneVerbs { id: paneVerbs; driver: driver }
    AutoActTipVerbs { id: tipVerbs; driver: driver }
    AutoActDetailsVerbs { id: detailsVerbs; driver: driver }
    AutoActHistoryVerbs { id: historyVerbs; driver: driver }
    AutoActPlanVerbs { id: planVerbs; driver: driver }
    AutoActFindVerbs { id: findVerbs; driver: driver }
    AutoActDiffVerbs { id: diffVerbs; driver: driver }

    function runAutoAct() {
        const act = Harness.autoAct
        const arg = Harness.autoActArg
        if (act === "perf") return // WindowPerfDriver owns this verb's causal completion.
        driver.prepareCompletion(act)
        // First family to know the verb runs it — the same first match the one chain had, and no verb is
        // named by two of them.
        const known =
            wipVerbs.run(act, arg)
            || fileRowVerbs.run(act, arg)
            || navVerbs.run(act, arg)
            || navBoxVerbs.run(act, arg)
            || refVerbs.run(act, arg)
            || switchVerbs.run(act, arg)
            || publishVerbs.run(act, arg)
            || graphRowVerbs.run(act, arg)
            || stepVerbs.run(act, arg)
            || paneVerbs.run(act, arg)
            || tipVerbs.run(act, arg)
            || detailsVerbs.run(act, arg)
            || historyVerbs.run(act, arg)
            || planVerbs.run(act, arg)
            || findVerbs.run(act, arg)
            || diffVerbs.run(act, arg)
        if (!known && !driver.completionDeferred && !driver.writeExpected) {
            // Not a page verb, and not in the completion ledger either —
            // window verbs are (they defer to Main's own driver), so this
            // is a misspelling. It must not pass as a green run of the
            // plain screen: say so and leave the run to the watchdog.
            Harness.report("auto_act unknown=" + act)
            return
        }
        Harness.report("auto_act ran=" + act)
        // The file-row acts have not acted yet — they are waiting on their rows (`fileRowsTimer`),
        // and finishing here would photograph the scene before the menu is up. Their sampler
        // finishes the dispatch after `runFileRowAct` has run.
        if (driver.fileRowActs.indexOf(act) < 0)
            driver.dispatchFinished()
    }
}
