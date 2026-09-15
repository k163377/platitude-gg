pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

/// PGG_AUTO_ACT runs one operation — a write, or a surface left standing for the overlay shot — through exactly the code
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
    property CarriedPane carriedPane
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
    /// The pair's own keeper, for the one verb whose subject is what it is holding rather than what it drew
    /// (`row-card-return`): whether the hand coming back to a row re-holds the card that is already out.
    property RowHoverHost rowHost

    /// The details card has caught up with a real selection — the one readiness every card-reading sampler waits on.
    /// Named once so no copy can drop the empty-selection half: the bare `!==` comparison is vacuously satisfied while
    /// nothing is selected, and a sampler that copies it without a prior selection guard photographs a stale card.
    readonly property bool cardSettled: page.selectedOid !== "" && detailsModel.shaHex === page.selectedOid

    /// The graph row another working copy's uncommitted work stands on, or -1 while the graph holds none of it.
    /// **Asked of the model**: a row off screen has no delegate to ask, and every one of these rows answers to the
    /// same all-zero id, so the copy's own name is what tells them apart (`GraphModel.carriedName`).
    ///
    /// Named here rather than in one family: two of them stand on a copy before doing anything else, and a second
    /// copy of the walk is a second answer to "which row is that copy's".
    function rowOfCopy(name) {
        for (let row = 0; row < graphModel.rowTotal; row++) {
            if (graphModel.carriedName(row) === name)
                return row
        }
        return -1
    }

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
    /// The tab's count of answers as it stood when the run armed — **a floor, and the only thing it is**: an answer
    /// numbered above it came back after the arm, which is how a verb reading the answers one by one finds its own
    /// among them (`RepoTab.writeAnswerSeq`). Whether the run's write is *done* is not this count's to say; that is
    /// the watch's, by id.
    property int writeSeqBefore: 0
    /// Whether the breach has been said. **The contract's own memory is the tab's** (`RepoTab.writeContractBroken`),
    /// and this is not a second copy of it: the tab is already broken by the time the words come back from it, so a
    /// sampler still ticking needs something that was false when the first one said them.
    property bool saidBroken: false
    /// **Nothing else about this run's write is held here.** The id belongs to the ask that was given it and the
    /// run's own side of the contract belongs beside it, both on the tab (`repo_tab::write_watch`) — where
    /// `cargo test` reaches them, and where one thread means the arm, the ask and the read are one uninterrupted
    /// stretch.
    /// Whether this run's picture is of the page the status behind its write leaves rather than of the page its
    /// answer arrives on — the verb says so (`AutoActCompletion.owesStatus`), and that file is where the two
    /// pages are told apart.
    property bool statusOwed: false
    /// Which report of HEAD can carry that status (`RepoTab.writeAnswerHeadSeq`), 0 until this run's own answer
    /// names one. **A status counted before the write cannot reach that number**, which is what makes the wait
    /// below about this write — the reset landing waits on the same pair for the same reason
    /// (`AutoActHistoryVerbs.resetLandedTimer`). Not a count of statuses: the poll's own tick moves that too.
    property int statusOwedFrom: 0
    /// Arms that wait on the answer at `index` of the notify being drained — the run's own, where the run knows
    /// which one that is (`RepoTab.commitAnswer`). A notify that carried none (-1) leaves the arming to the
    /// connection below, which takes the first answer nobody could have pressed for.
    function owedStatusAt(index) {
        driver.statusOwed = true
        driver.statusOwedFrom = index < 0 ? 0 : repoTab.writeAnswerHeadSeq(index)
    }
    /// Whether everything a run that named [`statusOwed`] is waiting for is in: the status that write asked for,
    /// the graph holding where it put HEAD, and the page that pair leaves behind.
    ///
    /// **The graph is waited out on the row and not on a count of passes** — a pass finishes for work nobody here
    /// asked for, and the rebuild the refs ask for is skipped where nothing moved. Where the reader ends up on what
    /// the write made, the landing inside `PageSettled` asks for that row as well; where the tree keeps the reader
    /// on the working-tree face, nothing else does.
    function owedStatusLanded() {
        return driver.statusOwedFrom !== 0
            && workTree.statusSeq >= driver.statusOwedFrom
            && graphModel.rowOf(workTree.headOid) >= 0
            && PageSettled.settled(page)
    }
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
    /// The menu this run took down on the product's behalf, or null. A verb that fires a row's write past the item
    /// (`page.dropCommit` from the history verbs) closes nothing by itself, and a closing menu is still on screen for
    /// its exit transition — which the census walk counts (`WindowCensus` goes by `visible`). So the completion
    /// holds until this one has gone (`complete`), the way the item's own click would have had it down before the
    /// write's answer came.
    property AppMenu menuGoing: null
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
        driver.statusOwed = completion.owesStatus(act)
        driver.statusOwedFrom = 0
        // The dispatch is itself a press for every verb that writes on its way through `run()`, so the watch is armed
        // before it and catches whatever ask it makes. A verb that presses later from a sampler arms again there
        // ([`pressWrite`]) — re-arming over a watch that caught nothing is what that is for.
        driver.beginWrite(act)
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
        // **Both halves are the graph's own word** (`GraphModel.wipRow` / `carriedTop`): every working copy's row
        // wears the same all-zero id, so a row read by the id alone answers `wip` for a neighbour's as readily as
        // for this window's, and the verbs here all ask this to mean ours.
        if (graphModel.wipRow)
            return "wip"
        if (graphModel.carriedTop)
            return "copy"
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
        // **A run whose contract broke does not complete.** It has already been ended, by the one path that ends it
        // (`sayBroken`), and a barrier landing afterwards would otherwise hand the shot driver a run that finished —
        // green, over a page the broken write never reached.
        if (repoTab.writeContractBroken())
            return
        // The menu the verb took down is on screen for its exit transition, and the walk counts what is visible
        // (`WindowCensus`): whichever barrier brought the run here, it is not complete while that menu is on its
        // way out (`menuGoing`).
        if (driver.menuGoing !== null && driver.menuGoing.visible) {
            menuGoneBarrier.start()
            return
        }
        page.Window.window.finishAutoAct()
    }

    /// **The one way a verb makes a write.** `what` names the press for the lines a failure and a watchdog leave;
    /// `press` is the product's own input path and answers whether the input went in. Answers the same.
    ///
    /// **The arm and the press are one call on purpose.** Held apart, every verb had to spell the order itself — arm,
    /// press, take the id — and one that armed on the far side of its press swallowed its own write and waited out
    /// the watchdog in silence (measured, `stage-all` / `unstage-all` / `resolve-all`, both sides, 600s). There is no
    /// order left to spell: the tab is told to keep the id of the next ask before the input goes in, and the ask that
    /// input produces is the one it keeps (`repo_tab::write_watch`).
    function pressWrite(what, press) {
        if (!driver.beginWrite(what))
            return false
        return driver.inputWent(press())
    }

    /// **A write whose input takes time** — a hold to run out, a dialog to answer, a row to appear. Arms and says the
    /// input is on its way; the ask it produces is caught whenever it comes, and until then the barrier is waiting on
    /// the input rather than on a missing id. Answers whether the arm took.
    /// **The rule itself is `repo_tab::write_watch`**, where `cargo test` reaches it: arming over a write this run
    /// announced a press for and is still waiting out is the breach, and a run that broke arms no more. Nothing
    /// here reads whether any particular timer is running — a verb waiting from a sampler of its own
    /// (`line-back` stages three writes that way) is waiting exactly as much as one on the shared barrier.
    function beginWrite(what) {
        const breach = repoTab.watchNextWrite(what)
        if (breach !== "")
            return driver.sayBroken(breach)
        driver.writeSeqBefore = repoTab.writeSeq
        driver.noteWrite()
        return true
    }

    /// **The input this run put in has gone**: a press returned, a hold ran out, a dialog was answered. `went` is
    /// whether the input path took it — `false` is a row that was not there yet, and the verb tries again.
    function inputWent(went) {
        if (!went)
            return false
        repoTab.writeInputWent()
        driver.noteWrite()
        return true
    }

    /// **This run is moving on from the write it pressed for without waiting it out** — said out loud, because
    /// forgetting to wait looks exactly the same from here (`repo_tab::write_watch`).
    function letWriteGo() {
        repoTab.letWriteGo()
        driver.noteWrite()
    }

    /// Hands the window what this run's write is doing, for the line the ceiling leaves (`AutoShotDriver`). Said on
    /// every change rather than read at the ceiling, because the window cannot reach a page's own driver.
    function noteWrite() {
        page.Window.window.noteAutoActWrite("verb=" + Harness.autoAct + " press=" + repoTab.writeWanted()
                                            + " run=" + repoTab.writeRunStage()
                                            + " watch=" + repoTab.writeWatchStage()
                                            + " id=" + repoTab.watchedWriteId())
    }

    /// **The contract broke, and this is the end of the run.** Says it once with the verb's name on it, and ends the
    /// run there rather than leaving it to the ceiling — the words are the finding, and ten minutes of silence after
    /// them says nothing more. The line is what the parent fails on (`verify::outcome`), so the run cannot come back
    /// green by another road either: the contract refuses to arm again and refuses to complete
    /// (`repo_tab::write_watch`).
    function sayBroken(breach) {
        if (driver.saidBroken)
            return false
        driver.saidBroken = true
        Harness.report("write_contract verb=" + Harness.autoAct + " broke=" + breach)
        driver.noteWrite()
        // Every barrier at once: whichever one this run was waiting on, it is waiting for something that will not
        // come, and the shot driver's own ending is what tears the run down.
        writeBarrier.stop()
        treeBarrier.stop()
        graphBarrier.stop()
        page.Window.window.finishAutoAct()
        return false
    }

    /// A breach the run saw and the contract could not — an input that went in with nothing accepted for it, say.
    /// Ends the run the same way.
    function breakWrite(breach) {
        repoTab.breakWriteContract(breach)
        return driver.sayBroken(breach)
    }

    /// Whether the write this run asked for has been through the boundaries behind it (`RepoTab.wroteThrough` — the
    /// tab holds the id its own ask was given and matches it, so another write answering first cannot answer for
    /// this one) and nothing else this run started is still in flight — **the whole of the write barrier**, so a verb
    /// with a sub-barrier of its own asks this rather than spelling it again (it was spelled out in ten files, and
    /// the spellings drifted).
    function wroteAndSettled() {
        return repoTab.wroteThrough() && repoTab.busyCount === 0
    }
    /// Runs a hold to its end, with the write barrier armed on the end and not on the start. The end is
    /// `Metrics.holdMs` of ticks away, and a write this run never pressed can answer in between — the fetch a
    /// repository makes on the way open, or the one a verb asked for itself (`push-tag` on a drifted tag). A barrier
    /// armed at the start passes on that answer and photographs the card still up, the row half filled and nothing
    /// written; the census walked then holds the card, and the next run's — whose stray answer came sooner or later
    /// — does not (observed: `push-tag v1.5:drift` framed both cards up on one run and neither under the gate's load,
    /// which is the whole reason the moment the stray answer lands cannot be part of the barrier).
    ///
    /// The row's own handler runs inside `completeHold()` — the panes' ones too (`DiffPane.completeHold` presses a
    /// row of its list, `GraphPane.completeHold` the ask bar's pill) — so the ask it makes has gone in by the time
    /// the call returns, which is where the watch is read.
    function holdToEnd(held) {
        return driver.pressWrite("hold", () => {
            held.completeHold()
            return true
        })
    }
    /// Past the end of any line these fixtures carry. Both models cut a selection's ends to the line they fell on
    /// before reading anything off it, so a drag that means "to the end of the row" can say so without measuring
    /// the row.
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
    // The number this run's own write answer named, for a run with no index of its own to arm from
    // (`owedStatusAt`). **Not whatever answered first**: the fetch a page makes on its way open and the interval's
    // own travel the same queue and answer on the same notify, and a run armed on one of those waits for a report
    // a status read before the press already carries. Which answers those are is named on the other side of the
    // bridge (`RepoTab.writeAnswerAsked`), the same set the barrier below counts.
    Connections {
        target: driver.repoTab
        enabled: driver.statusOwed && driver.statusOwedFrom === 0
        function onWriteSeqChanged() {
            const tab = driver.repoTab
            for (let i = 0; i < tab.writeAnswerCount(); i++) {
                if (tab.writeAnswerSeq(i) > driver.writeSeqBefore && tab.writeAnswerAsked(i)) {
                    driver.statusOwedFrom = tab.writeAnswerHeadSeq(i)
                    return
                }
            }
        }
    }
    // **The run's own write, through every one of its boundaries** (`wroteAndSettled`): git answered *this* id, and
    // every reading it invalidated has been published — so what the page is about to be photographed showing is that
    // write's result and not the page its answer arrived on. `busyCount === 0` is the quiet condition on top: nothing
    // else this run started is still in flight. (Waiting for busy to *rise* would wedge: the answer can be absorbed
    // before this sampler ever sees the flag up.) A verb whose picture is of the page the write's status leaves waits
    // past all of it (`owedStatusLanded`).
    SampleTimer {
        id: writeBarrier
        onTriggered: {
            // A broken contract has been said once, by name, and the run ended there (`sayBroken`). Nothing here
            // can mend it: `wroteAndSettled` answers false for good, so this would tick to the ceiling.
            if (repoTab.writeContractBroken()) {
                writeBarrier.stop()
                return
            }
            if (!driver.wroteAndSettled()) {
                // The one sampler that can be the thing still ticking at the ceiling, so the line it would leave is
                // kept current from here rather than read at the ceiling (the window cannot reach this driver).
                driver.noteWrite()
                return
            }
            if (driver.statusOwed && !driver.owedStatusLanded())
                return
            writeBarrier.stop()
            // **What the barrier waited on, said out loud.** Read off the tab rather than off the condition above,
            // so a barrier that went back to counting answers says so: `mine=false` is a run that passed without
            // ever having had a write of its own, and `settled=` is the last boundary that write reached.
            Harness.report("write_barrier mine=" + (repoTab.watchedWriteId() !== 0)
                           + " settled=" + (repoTab.writeWatchStage() === "settled"))
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
            // A diff the verb opened under the bar (`notice-over-diff`) is a git subprocess away, and the picture is
            // of the whole window: a pane still reading frames like one that has, and a walk taken then records no
            // row of it. So the bar is not the whole of the wait while the middle is a diff on its way.
            if (page.diffShown && !diffPane.diffSettled())
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
    // The menu a verb took down, waited off the screen rather than off the clock: `visible` stays up for the exit
    // transition and drops when it ends, and that edge is what lets the completion through (`complete`).
    SampleTimer {
        id: menuGoneBarrier
        onTriggered: {
            if (driver.menuGoing.visible)
                return
            menuGoneBarrier.stop()
            driver.complete()
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
            // `Opened` only means the path was accepted. Where HEAD stands, and the page having stopped arriving
            // (`PageSettled` — refs, status, the graph's rows agreeing with that status, the selected commit's read)
            // are the baseline every page verb is allowed to act on. The rows agreeing with the status matters here:
            // the opening walks the log before the first status is read, and a verb that names its rows by number
            // (`3:5:6`) presses one row off if it presses before the working-tree row has landed.
            if (repoTab.state !== "open" || !workTree.headKnown || !PageSettled.settled(page))
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
        // **The dispatch does not announce a press.** Whatever it asked for on its way through is already in the
        // watch armed above, which is all the barrier needs; what `pressed` means is a verb saying "the input I was
        // waiting to put in has gone in", and a verb that presses from a sampler has not said it yet. Claiming it
        // here would make every later arm read as a second write over an unfinished first.
        Harness.report("auto_act ran=" + act)
        // The file-row acts have not acted yet — they are waiting on their rows (`fileRowsTimer`),
        // and finishing here would photograph the scene before the menu is up. Their sampler
        // finishes the dispatch after `runFileRowAct` has run.
        if (driver.fileRowActs.indexOf(act) < 0)
            driver.dispatchFinished()
    }
}
