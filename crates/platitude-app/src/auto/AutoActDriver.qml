pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

/// PGG_AUTO_ACT runs one operation — a write, or a surface left standing for the overlay shot — through the code
/// path a click takes, so the wiring can be proven headlessly. `PageHarness` builds this only when a verb was given.
// `Item` because `QtObject` has no default property to hold the timers below.
Item {
    id: driver

    property Item page

    property RepoTab repoTab
    property WorkingTreeModel workingTree
    property GraphModel graphModel
    property DetailsModel detailsModel
    property NavSectionModel branchesModel
    property NavSectionModel remotesModel
    property NavSectionModel unstagedModel
    /// The working copies (WORKTREES).
    property NavSectionModel worktreesModel
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
    /// The cards the ref menu's rows hang behind. A verb photographing a row opens its card first
    /// (`refMenu.openSub`): the row answers either way, but the picture should show the card a reader sees.
    property AppMenu refBranchCard
    property AppMenu refTagCard
    property AppMenuItem refDeleteItem
    property AppMenuItem refUpstreamItem
    property AppMenuItem refStashDropItem
    property AppMenuItem refSwitchItem
    property AppMenuItem refPullItem
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
    /// What reached the clipboard, which the clipboard itself will not say (`ClipboardHelper.lastCopied`).
    property ClipboardHelper clipboard
    property CommitMenuState commitMenuState
    property AppMenu commitMenu
    property AppMenuItem dropCommitItem
    property AppMenuItem tagHereCommitItem
    property AppMenuItem stashDeleteItem
    property AppMenu resetMenu
    property AppMenu commitBranchCard
    property AppMenu commitTagCard
    property AppMenuItem commitDeleteItem
    property AppMenuItem switchCommitItem
    property AppMenuItem pullCommitItem
    property AppMenuItem hardResetItem
    property PublishFlow publishFlow
    property UpstreamFlow upstreamFlow
    property RemoteDialog remoteDialog
    property AppMenu remoteMenu
    /// The WORKTREE card in each of the two menus that carry it: the ref menu and the graph row's, on every row with a
    /// commit.
    property RefWorktreeMenu refCopyCard
    property RefWorktreeMenu commitCopyCard
    /// The two rows that make a working copy, on each of those two cards (`AutoActCopyVerbs`).
    property AppMenuItem refCopyHereItem
    property AppMenuItem refCopyAddItem
    property AppMenuItem commitCopyHereItem
    property AppMenuItem commitCopyAddItem
    /// The remote the push-default verbs act on, and the mark they wait for before the shot — empty where no mark
    /// is asked to move.
    property string remoteTarget: ""
    property string markWanted: ""
    property RefListPopup refList
    property CommitHoverCard rowCard
    property RowHoverHost rowHost

    /// The details card has caught up with a real selection. The sha comparison alone is vacuously true while
    /// nothing is selected — a sampler copying it without the first half photographs a stale card.
    readonly property bool cardSettled: page.selectedOid !== "" && detailsModel.shaHex === page.selectedOid

    /// The graph row another working copy's uncommitted work stands on, or -1 while the graph holds none of it.
    /// Asked of the model by the copy's name (`GraphModel.carriedName`): a row off screen has no delegate, and all
    /// these rows answer to the same all-zero id.
    function rowOfCopy(name) {
        for (let row = 0; row < graphModel.rowTotal; row++) {
            if (graphModel.carriedName(row) === name)
                return row
        }
        return -1
    }
    /// Whether the shared tooltip stands clear of its target sideways — beside the row rather than over the rows
    /// above and below it (規約 §hover のツールチップ「行の的は、行の横に立つ」). Read off the instance's drawn seat, not the rule.
    function tipAside(tip) {
        const at = tip.parent
        if (at === null || !tip.visible)
            return false
        return tip.x + tip.width <= 0 || tip.x >= at.width
    }
    /// Forces a menu row's tooltip through the property the real hover writes (`AppMenuItem.tipForced`), and says
    /// whether it stands where a hand's would: up, beside the row. One that came out before the menu stood where the
    /// hand would have raised it keeps the seat read off the row then (`SharedToolTip.targetAt`), so it is taken down
    /// to come out again. True at once for a row that `wants` none.
    function rowTipStood(row, wants) {
        const tip = row.ToolTip.toolTip
        if (row.tipForced && tip.visible && !driver.tipAside(tip)) {
            row.tipForced = false
            return false
        }
        row.tipForced = wants
        return !wants || (row.ToolTip.visible && driver.tipAside(tip))
    }
    /// Where a TAG card's rows reach, in the one report line both entrances' verbs say it with
    /// (`AutoActRefVerbs.reachWords`).
    function tagReachWords(card) {
        return refVerbs.reachWords(card)
    }

    property bool claimed: false
    /// The page starts this once its models are attached: a verb run before them would act on a repository nothing
    /// has read yet.
    function begin() {
        autoActTimer.start()
    }

    // A read-only verb completes on a rendered surface. A write also waits out its own answer: its request leaving
    // this item says nothing about the repository.
    property bool completionDeferred: false
    property bool writeExpected: false
    /// The tab's answer count when the run armed — only a floor: an answer numbered above it came back after the arm
    /// (`RepoTab.writeAnswerSeq`). Whether the run's write is done is the watch's to say, by id.
    property int writeSeqBefore: 0
    /// Whether the breach has been said. Not `RepoTab.writeContractBroken`: that is already true when the words are
    /// said, so a sampler still ticking needs something that was false before.
    property bool saidBroken: false
    /// Whether this run's picture is of the page the status behind its write leaves
    /// (`AutoActCompletion.owesStatus`).
    property bool statusOwed: false
    /// Which report of HEAD can carry that status (`RepoTab.writeAnswerHeadSeq`), 0 until this run's own answer
    /// names one. A report number, not a count: a status counted before the write cannot reach it, and the poll's
    /// own tick moves a count too.
    property int statusOwedFrom: 0
    /// Arms on the answer at `index` of the notify being drained — the run's own, where it knows which that is
    /// (`RepoTab.commitAnswer`). -1 leaves the arming to the connection below.
    function owedStatusAt(index) {
        driver.statusOwed = true
        driver.statusOwedFrom = index < 0 ? 0 : repoTab.writeAnswerHeadSeq(index)
    }
    /// Whether everything a [`statusOwed`] run waits for is in: the status its write asked for, the graph holding
    /// where it put HEAD, and the page that pair leaves behind.
    ///
    /// The graph is waited on the row, not a pass: a pass finishes for work nobody here asked for, and the rebuild
    /// is skipped where nothing moved. `PageSettled` asks for that row only where the reader lands on what the write
    /// made.
    function owedStatusLanded() {
        return driver.statusOwedFrom !== 0
            && workingTree.statusSeq >= driver.statusOwedFrom
            && graphModel.rowOf(workingTree.headOid) >= 0
            && PageSettled.settled(page)
    }
    /// The working-tree row this run's write takes out of its bucket, as `<bucket>:<path>` — or "" for the verbs the
    /// write barrier alone answers for. The write barrier already has the model re-read (each feed's drain is queued
    /// ahead of the tab's settle), but a settle also comes for a write git refused and for one whose re-read failed
    /// (`WriteSettled.failed` never reaches the tab): the row leaving is what says this write landed — a refusal
    /// leaves it for the watchdog to diagnose, a failed re-read until the next read. Read off the row (verify-ui
    /// verbs.md「は着地まで待って撮る」), not a counter: a counter also moves for statuses nobody here asked for.
    property string treeGoneRow: ""
    /// The graph row this run's write takes off the graph, or "" for the verbs the write barrier alone answers for.
    /// Until the row is gone from the model, rows can wear each other's marks (a popped stash's box on the
    /// working-tree row); a pass counter would also move for passes nobody here asked for. The write barrier's settle
    /// is no proof the row went: it also comes for a refused write and a failed re-read (see `treeGoneRow`).
    ///
    /// **Stash verbs only as it stands**: `graphBarrier` also holds for the stash total moving, so a non-stash write
    /// that set this would wait out the watchdog. Widen the barrier before pointing a new verb at it.
    property string graphGoneOid: ""
    /// The menu this run took down on the product's behalf, or null. A verb firing a row's write past the item closes
    /// nothing by itself, and a closing menu stays `visible` through its exit transition, which the census counts
    /// (`WindowCensus`) — so `complete` holds until it has gone.
    property AppMenu menuGoing: null
    /// How many entries the stash list held before that write. The list is read after the rebuild, so a shot taken
    /// when the graph settles can frame a sidebar still counting the old entries.
    property int stashTotalBefore: -1
    /// The summary this run put in the commit box before the write, or "" — what the entry the write makes has to
    /// carry when the list settles (`named=`).
    property string stashWanted: ""
    /// The name the entry this run pops was carrying, or "" — what the commit box has to be holding once the pop has
    /// landed (`back=`). Read before the press, because the entry is gone by the time the answer is.
    property string popWanted: ""
    /// What HEAD was before that write, for the runs that wait for HEAD to move off it (`amend-reset-author`).
    property string headOidBefore: ""
    /// The acts that name rows of the WIP lists — run only once those rows are walkable (`fileRowsTimer`).
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
        // Armed before the dispatch, which is itself the press for a verb that writes inside `run()`. A verb that
        // presses later from a sampler re-arms there (`pressWrite`) — allowed over a watch that caught nothing.
        driver.beginWrite(act)
        driver.treeGoneRow = ""
        driver.graphGoneOid = ""
        driver.stashTotalBefore = stashesModel.total
        driver.stashWanted = ""
        driver.popWanted = ""
    }

    /// Whether the top stash entry wears the summary this run typed, read off the reflog subject git wrote. A tail
    /// match: git puts `On <branch>: ` in front of a named entry.
    function stashNamed() {
        return driver.stashWanted !== "" && stashesModel.nameAt(0).endsWith(driver.stashWanted)
    }

    function stashCameBack() {
        return driver.popWanted !== "" && wipPane.subjectText === driver.popWanted
    }

    /// What the graph's leading row is, for the runs that are about the mark it wears.
    function graphTopKind() {
        const oid = graphModel.oidAt(0)
        if (oid === "")
            return "none"
        // The graph's own word, not the id: every working copy's row wears the all-zero id
        // (rules-refs/app-ui.md「作業コピーの行は全部 git の all-zero id を着ている」).
        if (graphModel.wipRow)
            return "wip"
        if (graphModel.carriedTop)
            return "copy"
        return graphModel.stashRefOf(oid) !== "" ? "stash" : "commit"
    }

    /// The row a word names in the rename's remote chooser (`RenameCarryFlow.choices`): `replace` makes one name and
    /// takes the other away, `add` leaves the other, `leave` writes nothing.
    function carryChoiceIndex(word) {
        return word === "replace" ? 0 : word === "add" ? 1 : word === "leave" ? 2 : -1
    }
    /// What that bar is saying, for the two runs that raise it. Read off the control and the bar, not the flow that
    /// dressed them, so a pick that never reached the form answers `shown=false`. `pick=` says which of the three
    /// dressings was expected.
    function carryWords(kind, pick) {
        const field = driver.graphPane.askForm ? driver.graphPane.askForm.pick : null
        return "rename_carry kind=" + kind
            + " rows=" + (field ? field.count : -1)
            + " pick=" + (pick === "" ? "none" : pick)
            + " shown=" + Boolean(field && field.wanted !== "")
            + " answerable=" + driver.graphPane.askAnswerable
            + " hold=" + driver.graphPane.askHold
            + " neutral=" + driver.graphPane.askNeutral
            // Read off the bar: the pill's word follows the chooser, and a binding that stopped would show here.
            + " pill=" + driver.graphPane.askCard.accept
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
        // A broken run was already ended by `sayBroken`; a barrier landing afterwards would finish it green.
        if (repoTab.writeContractBroken())
            return
        // Whichever barrier brought the run here, not while that menu is still on its way out (`menuGoing`).
        if (driver.menuGoing !== null && driver.menuGoing.visible) {
            menuGoneBarrier.start()
            return
        }
        page.Window.window.finishAutoAct()
    }

    /// **The one way a verb makes a write.** `what` names the press for the lines a failure and a watchdog leave;
    /// `press` is the product's own input path and answers whether the input went in. Answers the same.
    /// Arm and press are one call so no verb arms after its own press
    /// (rules-refs/app-ui.md「書き込みを頼む口は動詞側に 1 つだけ」). Only for a write that leaves by the tab's door
    /// (`RepoTab::ask_session`): the plan's run leaves by its own with no id to hold, and pressed through here would
    /// leave the watch pressed for good (`AutoActPlanVerbs`).
    function pressWrite(what, press) {
        if (!driver.beginWrite(what))
            return false
        return driver.inputWent(press())
    }

    /// **A write whose input takes time** — a hold to run out, a dialog to answer, a row to appear. Arms; the ask the
    /// input produces is caught whenever it comes. Answers whether the arm took: arming over a pressed write not yet
    /// waited out is a breach (`repo_tab::write_watch`).
    function beginWrite(what) {
        const breach = repoTab.watchNextWrite(what)
        if (breach !== "")
            return driver.sayBroken(breach)
        driver.writeSeqBefore = repoTab.writeSeq
        driver.noteWrite()
        return true
    }

    /// The input this run put in has gone (a press returned, a hold ran out, a dialog was answered). `went` is false
    /// where the input path did not take it — a row not there yet — and the verb tries again.
    function inputWent(went) {
        if (!went)
            return false
        repoTab.writeInputWent()
        driver.noteWrite()
        return true
    }

    /// This run moves on from its pressed write without waiting it out — said out loud, since forgetting to wait
    /// looks the same.
    function letWriteGo() {
        repoTab.letWriteGo()
        driver.noteWrite()
    }

    /// Hands the window this run's write state for the ceiling's line (`AutoShotDriver`) — on every change, as the
    /// window cannot reach a page's driver.
    function noteWrite() {
        page.Window.window.noteAutoActWrite("verb=" + Harness.autoAct + " press=" + repoTab.writeWanted()
                                            + " run=" + repoTab.writeRunStage()
                                            + " watch=" + repoTab.writeWatchStage()
                                            + " id=" + repoTab.watchedWriteId())
    }

    /// **The contract broke: the run ends here**, said once with the verb's name on it. The parent fails on that line
    /// (rules-refs/app-ui.md「違反は取り消せない」).
    function sayBroken(breach) {
        if (driver.saidBroken)
            return false
        driver.saidBroken = true
        Harness.report("write_contract verb=" + Harness.autoAct + " broke=" + breach)
        driver.noteWrite()
        // Whichever barrier the run was on waits for something that will not come.
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

    /// Whether this run's own write has been through its boundaries (`RepoTab.wroteThrough`, matched by id) and
    /// nothing else it started is in flight — **the whole write barrier**; a verb with a sub-barrier asks this.
    function wroteAndSettled() {
        return repoTab.wroteThrough() && repoTab.busyCount === 0
    }
    /// Runs a hold to its end, armed on the end: a write this run never pressed can answer during `Metrics.holdMs`
    /// (the opening fetch, or `push-tag`'s own on a drifted tag), and a barrier armed at the start would pass on it
    /// with the card still up. The row's handler runs inside `completeHold()`, so its ask has gone in when the call
    /// returns.
    function holdToEnd(held) {
        // A button waiting on git answers false (`ActionButton.completeHold`) — no press. A hold with no such gate
        // answers nothing, which is a press that went in.
        return driver.pressWrite("hold", () => held.completeHold() !== false)
    }
    /// A hold on a button that may be waiting on git: pressed again each tick until it takes (`holdToEnd`), then
    /// `then` runs.
    function holdWhenLive(held, then) {
        holdRetry.held = held
        holdRetry.then = then
        holdRetry.start()
    }
    SampleTimer {
        id: holdRetry
        property var held: null
        property var then: null
        onTriggered: {
            const went = driver.holdToEnd(holdRetry.held)
            Awaited.at(Harness.autoAct, went ? "pressed" : "live")
            if (!went)
                return
            holdRetry.stop()
            holdRetry.then()
        }
    }
    /// Past the end of any fixture line: both models clamp a selection's ends to their line, so a drag "to the end of
    /// the row" need not measure it.
    readonly property int pastLineEnd: 9999

    // The render boundary is `AutoShotDriver`'s (rules/app-ui.md §UI 自動化), so completing is all this does.
    QtObject {
        id: renderedBarrier
        function begin() {
            driver.complete()
        }
    }
    readonly property alias barrierRendered: renderedBarrier
    // Arms `statusOwedFrom` for a run with no index of its own (`owedStatusAt`) on the first answer after the arm
    // that was asked for (`RepoTab.writeAnswerAsked`): the opening and interval fetches answer on the same notify,
    // and arming on one waits for a report a status read before the press already carries.
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
    // The run's own write through all its boundaries (`wroteAndSettled`). Never wait for busy to *rise*: the answer
    // can be absorbed before this sampler sees the flag up. A [`statusOwed`] verb also waits `owedStatusLanded`.
    SampleTimer {
        id: writeBarrier
        onTriggered: {
            // Ended by `sayBroken`; `wroteAndSettled` stays false for good, so this would tick to the ceiling.
            if (repoTab.writeContractBroken()) {
                writeBarrier.stop()
                return
            }
            if (!driver.wroteAndSettled()) {
                // This sampler can be what is still ticking at the ceiling, so it keeps that line current.
                driver.noteWrite()
                return
            }
            if (driver.statusOwed && !driver.owedStatusLanded())
                return
            // A write refused with a report brings its bar down on the answer, and the bar grows into place
            // (`NoticeBar` の `Behavior`): shot on the answer, it is half open.
            if (page.noticeCard.open && !page.noticeCard.settled)
                return
            writeBarrier.stop()
            // `mine=false`: passed without a write of its own; `settled=`: the last boundary that write reached.
            Harness.report("write_barrier mine=" + (repoTab.watchedWriteId() !== 0)
                           + " settled=" + (repoTab.writeWatchStage() === "settled"))
            if (driver.treeGoneRow === "")
                driver.afterTreeSettled()
            else
                treeBarrier.start()
        }
    }
    readonly property alias barrierWrite: writeBarrier
    /// The bar a write that did not happen comes down in, waited on until fully down (`NoticeBar.settled`): a picture
    /// taken on the answer catches the words written and the height still nothing. One barrier for every kind of
    /// report (`Words.writeReported`) — each run claims the same things (verify-ui verbs.md「報せバーの 10 本」).
    SampleTimer {
        id: noticeBarrier
        /// Whether the line also says `plan=` — only for the verbs whose bar comes up instead of a plan
        /// (`AutoActPlanVerbs`); on the others it would always read the same.
        property bool saysPlan: false
        onTriggered: {
            if (!page.noticeCard.settled)
                return
            // A diff the verb opened under the bar (`notice-over-diff`) is a git subprocess away: waited out too.
            if (page.diffShown && !diffPane.diffSettled())
                return
            noticeBarrier.stop()
            // `log=` / `wrong=`: no error raised beside the bar, which no picture can say (デザイン規約 §答えの要らない報せ).
            Harness.report("write_notice open=" + page.noticeCard.open
                              + " clears=" + page.noticeClears
                              + " why=" + (page.noticeCard.detail !== "")
                              + " tone=" + (page.noticeCard.tone === "" ? "none" : page.noticeCard.tone)
                              + " log=" + page.commandsOpen
                              + " wrong=" + page.commandsWrong
                              + (noticeBarrier.saysPlan ? " plan=" + page.planActive : "")
                              // Last, because it is a sentence.
                              + " said=" + page.noticeCard.label
                              // …but for the one report whose heading names a working copy: whether the tree mark
                              // found the name (`NoticeBar.markWord`). Absent elsewhere, so no other line moves.
                              + (page.noticeCard.markWord !== "" ? " mark=" + page.noticeCard.markShown : ""))
            driver.complete()
        }
    }
    readonly property alias barrierNotice: noticeBarrier
    /// The diff's first heading lit as under a pointer (`hunk-tools`), waited on until its row is built. `level=` is
    /// its two words on one line, and `caption=` the band's caption and path on one. Each is a label beside something
    /// built otherwise (a button deeper by its hold mark, a field), and whether the pair shares a baseline is what the
    /// face answers for a line — a few pixels apart reads in a picture as the words' own setting. The distances ride
    /// along for a run that came out false.
    SampleTimer {
        id: headingBarrier
        onTriggered: {
            const words = diffPane.hunkWordsApart()
            // -1: the heading's row is not built yet.
            if (words < 0)
                return
            headingBarrier.stop()
            const caption = diffPane.captionApart()
            Harness.report("hunk_tools level=" + (words < 1) + " caption=" + (caption < 1)
                              + " words=" + words + " head=" + caption)
            driver.complete()
        }
    }
    readonly property alias barrierHeading: headingBarrier
    // Holds `complete` until `menuGoing` has finished its exit transition.
    SampleTimer {
        id: menuGoneBarrier
        onTriggered: {
            if (driver.menuGoing.visible)
                return
            menuGoneBarrier.stop()
            driver.complete()
        }
    }
    function afterTreeSettled() {
        if (driver.graphGoneOid === "")
            renderedBarrier.begin()
        else
            graphBarrier.start()
    }
    // Waits `treeGoneRow` out of the bucket it was named in. A write git refused leaves the row, and the run to the
    // watchdog — which is the diagnosis.
    SampleTimer {
        id: treeBarrier
        onTriggered: {
            const cut = driver.treeGoneRow.indexOf(":")
            const from = driver.treeGoneRow.substring(0, cut)
            const path = driver.treeGoneRow.substring(cut + 1)
            if (unstagedModel.holdsPath(from, path))
                return
            treeBarrier.stop()
            // `landed=` tells a file resolved into the index from one discarded out of the tree ("" = nowhere).
            Harness.report("tree_settled from=" + from
                              + " landed=" + unstagedModel.bucketOf(path)
                              + " conflicts=" + workingTree.conflictCount
                              + " staged=" + workingTree.stagedCount
                              + " unstaged=" + (workingTree.unstagedCount + workingTree.untrackedCount))
            driver.afterTreeSettled()
        }
    }
    // Waits `graphGoneOid` off the graph and the stash total off its old value (see `graphGoneOid`).
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
    /// The two landings a press can have — where the write put the reader (`AutoActLandings`).
    AutoActLandings { id: landings; driver: driver }
    /// The page wiring's edges as report lines — none is a state a later reading could recover (`PageReports`).
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
            // `open` only means the path was accepted; the baseline is HEAD known and `PageSettled` — without the rows
            // agreeing with the status, a verb naming rows by number (`3:5:6`) presses one row off.
            if (repoTab.state !== "open" || !workingTree.headKnown || !PageSettled.settled(page))
                return
            autoActTimer.stop()
            driver.runAutoAct()
        }
    }
    // The commit an argument names: an object name, "row:<n>" off the graph, or HEAD when empty — the demo
    // repository is built fresh every run, so its object names cannot be spelled ahead.
    function autoActOid(arg) {
        if (arg === "")
            return workingTree.headOid
        if (arg.indexOf("row:") === 0)
            return graphModel.oidAt(Number(arg.substring(4)))
        return arg
    }

    // Which verbs write and which defer their completion: two lists, read only from `prepareCompletion`.
    AutoActCompletion { id: completion }

    /// The verbs, grouped by what they act on.
    AutoActWipVerbs { id: wipVerbs; driver: driver }
    AutoActFileRowVerbs { id: fileRowVerbs; driver: driver }
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
    AutoActGraphReclickVerbs { id: graphReclickVerbs; driver: driver }
    AutoActStepVerbs { id: stepVerbs; driver: driver }
    AutoActKeyMenuVerbs { id: keyMenuVerbs; driver: driver }
    AutoActPaneVerbs { id: paneVerbs; driver: driver }
    AutoActTipVerbs { id: tipVerbs; driver: driver }
    AutoActDetailsVerbs { id: detailsVerbs; driver: driver }
    AutoActHistoryVerbs { id: historyVerbs; driver: driver }
    AutoActPlanVerbs { id: planVerbs; driver: driver }
    AutoActFindVerbs { id: findVerbs; driver: driver }
    AutoActDiffVerbs { id: diffVerbs; driver: driver }
    AutoActSplitVerbs { id: splitVerbs; driver: driver }
    AutoActHandVerbs { id: handVerbs; driver: driver }
    AutoActCopyVerbs { id: copyVerbs; driver: driver }
    AutoActRecoverVerbs { id: recoverVerbs; driver: driver }

    function runAutoAct() {
        const act = Harness.autoAct
        const arg = Harness.autoActArg
        if (act === "perf") return // WindowPerfDriver owns this verb's causal completion.
        driver.prepareCompletion(act)
        // First family to know the verb runs it; each verb is named by exactly one.
        const known =
            wipVerbs.run(act, arg)
            || fileRowVerbs.run(act, arg)
            || navVerbs.run(act, arg)
            || navBoxVerbs.run(act, arg)
            || refVerbs.run(act, arg)
            || switchVerbs.run(act, arg)
            || publishVerbs.run(act, arg)
            || graphRowVerbs.run(act, arg)
            || graphReclickVerbs.run(act, arg)
            || stepVerbs.run(act, arg)
            || keyMenuVerbs.run(act, arg)
            || paneVerbs.run(act, arg)
            || tipVerbs.run(act, arg)
            || detailsVerbs.run(act, arg)
            || historyVerbs.run(act, arg)
            || planVerbs.run(act, arg)
            || findVerbs.run(act, arg)
            || diffVerbs.run(act, arg)
            || splitVerbs.run(act, arg)
            || handVerbs.run(act, arg)
            || copyVerbs.run(act, arg)
            || recoverVerbs.run(act, arg)
        if (!known && !driver.completionDeferred && !driver.writeExpected) {
            // A misspelling: window verbs are in the completion ledger, so a verb known to neither is
            // nobody's. Left to the watchdog.
            Harness.report("auto_act unknown=" + act)
            return
        }
        // No `inputWent` here: a verb that presses from a sampler has not put its input in yet, and claiming it
        // would make its later arm read as a second write over an unfinished first.
        Harness.report("auto_act ran=" + act)
        // The file-row acts are still waiting on their rows; `fileRowsTimer` finishes their dispatch.
        if (driver.fileRowActs.indexOf(act) < 0)
            driver.dispatchFinished()
    }
}
