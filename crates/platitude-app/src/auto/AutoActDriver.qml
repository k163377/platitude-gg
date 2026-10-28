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
    property WorktreeModel worktree
    property GraphModel graphModel
    property DetailsModel detailsModel
    property NavSectionModel branchesModel
    property NavSectionModel remotesModel
    property NavSectionModel unstagedModel
    /// The worktrees (WORKTREES).
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
    property RefRowMenu refRowMenu
    property FileRowMenu fileRowMenu
    property AppMenu fileMenu
    property AppMenuItem fileDiscardItem
    property DiffRowMenu diffRowMenu
    /// What reached the clipboard, which the clipboard itself will not say (`ClipboardHelper.lastCopied`).
    property ClipboardHelper clipboard
    property CommitMenuState commitMenuState
    /// The graph row's menus whole, for the cards the verbs below do not name one by one (`AutoActGithubVerbs`).
    property CommitRowMenu commitRowMenu
    property AppMenu commitMenu
    property AppMenu stashMenu
    property AppMenuItem dropCommitItem
    property AppMenuItem tagHereCommitItem
    property AppMenuItem stashDeleteItem
    /// The menu on another worktree's uncommitted row and its two rows (`CommitRowMenu.carriedRowMenu`).
    property AppMenu carriedMenu
    property AppMenuItem carriedOpenItem
    property AppMenuItem carriedApplyItem
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
    /// The REMOTES row's menu and the one on a commit's changed file whole, for their GITHUB cards
    /// (`AutoActGithubVerbs`).
    property RemoteRowMenu remoteRowMenu
    property CommitFileMenu commitFileMenu
    /// The WORKTREE card in each of the two menus that carry it: the ref menu and the graph row's, on every row with a
    /// commit.
    property RefWorktreeMenu refWorktreeCard
    property RefWorktreeMenu commitWorktreeCard
    /// The two rows that make a worktree, on each of those two cards (`AutoActWorktreeVerbs`).
    property AppMenuItem refWorktreeHereItem
    property AppMenuItem refWorktreeAddItem
    property AppMenuItem commitWorktreeHereItem
    property AppMenuItem commitWorktreeAddItem
    property RefListPopup refList
    property CommitHoverCard rowCard
    property RowHoverHost rowHost

    /// The details card has caught up with a real selection. The sha comparison alone is vacuously true while
    /// nothing is selected — a sampler copying it without the first half photographs a stale card.
    readonly property bool cardSettled: page.selectedOid !== "" && detailsModel.shaHex === page.selectedOid

    /// The graph row another worktree's uncommitted work stands on, or -1 while the graph holds none of it.
    /// Asked of the model by the worktree's name (`GraphModel.carriedName`): a row off screen has no delegate, and all
    /// these rows answer to the same all-zero id.
    function rowOfWorktree(name) {
        for (let row = 0; row < graphModel.rowTotal; row++) {
            if (graphModel.carriedName(row) === name)
                return row
        }
        return -1
    }
    /// Whether the window's tip stands clear of its target sideways — beside the row rather than over the rows
    /// above and below it (規約 §hover のツールチップ「行の的は、行の横に立つ」). Read off the instance's drawn seat, not the rule.
    function tipAside(tip) {
        const at = tip.parent
        if (at === null || !tip.visible)
            return false
        return tip.x + tip.width <= 0 || tip.x >= at.width
    }
    /// Forces a menu row's tooltip through the property the real hover writes (`AppMenuItem.tipForced`), and says
    /// whether it stands where a hand's would: up, beside the row. One that came out before the menu stood where the
    /// hand would have raised it keeps the seat read off the row then (`TipSeat.targetAt`), so it is taken down
    /// to come out again. True at once for a row that `wants` none.
    // reach(press)
    function rowTipStood(row, wants) {
        const tip = Tips.box
        if (row.tipForced && tip.visible && !driver.tipAside(tip)) {
            row.tipForced = false
            return false
        }
        row.tipForced = wants
        return !wants || (row.tip.shown && driver.tipAside(tip))
    }
    // A menu row's words take their widths from a `RowLayout` on polish, so a cut name (`AppMenuItem.nameCut`) read in
    // the beat its card opens answers for the row before that layout ran — whole, as if it fit. Under load the
    // sampler's beat lands there. A reading waits two swapped frames from the first time `op` asks for a card in its
    // stage (`StageTimer.drawnSince`): one whose polish ran before the ask answers for the shape before it.
    property var framesCard: null
    property int framesCardSerial: 0
    /// Whether `card`'s rows have been laid out since `op` first asked about it: false while the frames are still to
    /// come.
    // reach(beat)
    function cardLaidOut(op, card) {
        if (driver.framesCard !== card) {
            driver.framesCard = card
            driver.framesCardSerial++
        }
        return op.drawnSince("card_laid_out#" + driver.framesCardSerial)
    }
    /// Where a TAG card's rows reach, in the one report line both entrances' verbs say it with
    /// (`AutoActRefVerbs.reachWords`).
    function tagReachWords(card) {
        return refVerbs.reachWords(card)
    }

    /// The page starts this once its models are attached: a verb run before them would act on a repository nothing
    /// has read yet.
    function begin() {
        autoActTimer.start()
    }
    /// The page's operation that claims the run and dispatches the verb (`autoActTimer`): the one a verb completing in
    /// the dispatching turn completes from.
    readonly property alias act: autoActTimer

    /// The report of a verb whose completion one of the barriers below makes (`dispatchFinished`, `barrierWrite`,
    /// `barrierNotice`): handed over unread as the verb dispatches or presses, and read by whichever operation
    /// completes — once the completion's hold has passed (`CompletionRoute`). Handed once.
    property var owedReport: null
    // reach(beat, press; 0=read)
    function reportAtCompletion(report) {
        if (driver.owedReport !== null) {
            Awaited.breach(autoActTimer, "handed a second report for the completion")
            return
        }
        driver.owedReport = report
    }
    /// The lines of `owedReport`, for a completing operation's report to read with its own.
    // reach(read)
    function verbReport() {
        if (driver.owedReport === null)
            return []
        const answer = driver.owedReport()
        return Array.isArray(answer) ? answer : [answer]
    }
    /// What the page holds a completion for besides its reads (`Awaited.pageHolds`): a menu this run took down on the
    /// product's behalf still on its way out (`menuGoing`). The page that claimed the run can be gone by then — a
    /// window verb closing its tab, or standing it in another worktree — and a page gone holds nothing.
    function completionHolds() {
        if (driver === null)
            return ({})
        return { "menuGone": driver.menuGoing === null || !driver.menuGoing.visible }
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
            && worktree.statusSeq >= driver.statusOwedFrom
            && graphModel.rowOf(worktree.headOid) >= 0
            && PageSettled.settled(page)
    }
    /// The worktree row this run's write takes out of its bucket, as `<bucket>:<path>` — or "" for the verbs the
    /// write barrier alone answers for. The write barrier already has the model re-read (each feed's drain is queued
    /// ahead of the tab's settle), but a settle also comes for a write git refused and for one whose re-read failed
    /// (`WriteSettled.failed` never reaches the tab): the row leaving is what says this write landed — a refusal
    /// leaves it for the watchdog to diagnose, a failed re-read until the next read. Read off the row (verify-ui
    /// verbs.md「は着地まで待って撮る」), not a counter: a counter also moves for statuses nobody here asked for.
    property string treeGoneRow: ""
    /// The graph row this run's write takes off the graph, or "" for the verbs the write barrier alone answers for.
    /// Until the row is gone from the model, rows can wear each other's marks (a popped stash's box on the
    /// worktree row); a pass counter would also move for passes nobody here asked for. The write barrier's settle
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

    // What a verb hands the barriers that finish its run — each the condition a barrier starts on, named once
    // where the verb knows it (in its dispatch or its press).
    // reach(beat, press)
    function awaitTreeGone(row) {
        driver.treeGoneRow = row
    }
    // reach(beat, press)
    function awaitGraphGone(oid) {
        driver.graphGoneOid = oid
    }
    // reach(beat, press)
    function menuLeaving(menu) {
        driver.menuGoing = menu
    }
    // reach(beat, press)
    function expectStashNamed(summary) {
        driver.stashWanted = summary
    }
    // reach(beat, press)
    function expectPopBack(label) {
        driver.popWanted = label
    }
    /// The verb completes from an operation of its own: the dispatch hands it no completion.
    // reach(beat, press)
    function deferCompletion() {
        driver.completionDeferred = true
    }
    /// The notice barrier's line says `plan=` too.
    // reach(beat, press)
    function noticeSaysPlan() {
        noticeBarrier.saysPlan = true
    }
    /// The acts that name rows of the WIP lists — run only once those rows are walkable (`fileRowsTimer`).
    readonly property var fileRowActs: [
        "stage-many", "stage-many-go", "discard-many", "discard-many-go",
        "file-menu", "file-menu-untracked", "file-menu-staged", "file-menu-conflict",
        "take-side-ours", "take-side-theirs", "open-mergetool",
        "discard-file", "discard-file-go", "delete-file", "delete-file-go",
        "discard-staged", "discard-staged-go"
    ]

    // reach(beat)
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
        driver.owedReport = null
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
        // The graph's own word, not the id: every worktree's row wears the all-zero id
        // (rules-refs/app-ui.md「worktree の行は全部 git の all-zero id を着ている」).
        if (graphModel.wipRow)
            return "wip"
        if (graphModel.carriedTop)
            return "carried"
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

    /// The end of a verb's dispatch, in the beat of `op` — the operation that dispatched it (`act`, or the one that
    /// waited out its rows): a verb that owns no completion completes here on the report it handed over
    /// (`reportAtCompletion`) — what it set going (the row it selected being read) is the page's, and the completion's
    /// hold waits the page out before the report is read — or a write verb's barrier takes over.
    // reach(beat)
    function dispatchFinished(op) {
        // Completed in the dispatch itself (`act.complete`).
        if (driver.completionDeferred || op.at("complete"))
            return
        if (driver.writeExpected) {
            writeBarrier.start()
            return
        }
        // Nothing of the verb's own to stand on at the completion but the page, which the route holds for.
        op.complete({ "when": () => ({}), "report": () => driver.verbReport() })
    }

    /// **The one way a verb makes a write.** `what` names the press for the lines a failure and a watchdog leave;
    /// `press` is the product's own input path and answers whether the input went in. Answers the same.
    /// Arm and press are one call so no verb arms after its own press
    /// (rules-refs/app-ui.md「書き込みを頼む口は動詞側に 1 つだけ」). Only for a write that leaves by the tab's door
    /// (`RepoTab::ask_session`): the plan's run leaves by its own with no id to hold, and pressed through here would
    /// leave the watch pressed for good (`AutoActPlanVerbs`).
    // reach(press; 1=press)
    function pressWrite(what, press) {
        if (!driver.beginWrite(what))
            return false
        return driver.inputWent(press())
    }

    /// **A write whose input takes time** — a hold to run out, a dialog to answer, a row to appear. Arms; the ask the
    /// input produces is caught whenever it comes. Answers whether the arm took: arming over a pressed write not yet
    /// waited out is a breach (`repo_tab::write_watch`).
    // reach(press)
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
    // reach(press)
    function inputWent(went) {
        if (!went)
            return false
        repoTab.writeInputWent()
        driver.noteWrite()
        return true
    }

    /// This run moves on from its pressed write without waiting it out — said out loud, since forgetting to wait
    /// looks the same.
    // reach(beat, press)
    function letWriteGo() {
        repoTab.letWriteGo()
        driver.noteWrite()
    }

    /// Hands the window this run's write state for the ceiling's line (`AutoShotDriver`) — on every change, as the
    /// window cannot reach a page's driver.
    // reach(beat, press)
    function noteWrite() {
        page.Window.window.noteAutoActWrite("verb=" + Harness.autoAct + " press=" + repoTab.writeWanted()
                                            + " run=" + repoTab.writeRunStage()
                                            + " watch=" + repoTab.writeWatchStage()
                                            + " id=" + repoTab.watchedWriteId())
    }

    /// **The contract broke: the run ends here**, said once with the verb's name on it. The parent fails on that line
    /// (rules-refs/app-ui.md「違反は取り消せない」).
    // reach(beat, press)
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
    // reach(beat, press)
    function breakWrite(breach) {
        repoTab.breakWriteContract(breach)
        return driver.sayBroken(breach)
    }

    /// Whether this run's own write has been through its boundaries (`RepoTab.wroteThrough`, matched by id) and
    /// nothing else it started is in flight — **the whole write barrier**; a verb with a sub-barrier asks this.
    function wroteAndSettled() {
        return repoTab.wroteThrough() && repoTab.busyCount === 0
    }
    /// Whether a menu raised now opens live: nothing this window asked git for is out, its reading included. One
    /// raised before then waits with every row greyed (`RepoPage.runningWhy`, `AppMenu.runningWhy`) — the
    /// precondition of every run that raises a menu to read or press its rows; `busyCount` falls ahead of it.
    function menusLive() {
        return page.runningWhy === ""
    }
    /// Runs a control's presses to their end, armed on the end: a write this run never pressed can answer before then
    /// (the opening fetch, or `push-tag`'s own on a drifted tag), and a barrier armed at the start would pass on it
    /// with the card still up. Automated presses wait out no floor (`HoldDriver.runOut`), so the row's handler runs
    /// inside `completeHold()` and its ask has gone in when the call returns.
    // reach(press)
    function holdToEnd(held) {
        // A button waiting on git, or a menu or exit row out of reach, answers false (`ActionButton.completeHold`,
        // `AppMenuItem.completeHold`, `OpExitRow.completeHold`) — no press. A control with no such gate (the question
        // bar's pill) answers nothing, which is a press that went in.
        return driver.pressWrite("hold", () => held.completeHold() !== false)
    }
    /// A hold on a button that may be waiting on git: pressed again each tick until it takes (`holdToEnd`), then
    /// `then` runs.
    // reach(beat; 1=beat)
    function holdWhenLive(held, then) {
        holdRetry.held = held
        holdRetry.then = then
        holdRetry.start()
    }
    PageStage {
        id: holdRetry
        name: "hold_retry"
        property var held: null
        property var then: null
        onTriggered: {
            // The press is its own reading: a button waiting on git answers false, and nothing moved.
            if (!holdRetry.input("pressed", () => driver.holdToEnd(holdRetry.held)))
                return
            holdRetry.stop()
            holdRetry.then()
        }
    }
    /// Past the end of any fixture line: both models clamp a selection's ends to their line, so a drag "to the end of
    /// the row" need not measure it.
    readonly property int pastLineEnd: 9999

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
    /// What the write barrier says of the write: `mine=false` passed without a write of its own; `settled=` the last
    /// boundary that write reached. A write a verb pressed that came back as a report says so the way the notice verbs
    /// do, for a run of that verb over a preset that refuses it (`verify/verbs`'s `WithPreset` rows).
    function writeLines() {
        return ["write_barrier mine=" + (repoTab.watchedWriteId() !== 0)
                + " settled=" + (repoTab.writeWatchStage() === "settled"),
                page.noticeCard.open ? driver.noticeLine() : ""]
    }
    // The run's own write through all its boundaries (`wroteAndSettled`). Never wait for busy to *rise*: the answer
    // can be absorbed before this sampler sees the flag up. A [`statusOwed`] verb also waits `owedStatusLanded`.
    // Completes the run where no later barrier is owed; otherwise hands on to the one that is.
    PageStage {
        id: writeBarrier
        name: "write_barrier"
        /// Every write verb's: its write answered and read back, the status its answer named in, and the report a
        /// refusal raised standing still — a refused write brings its bar down on the answer, and the bar grows into
        /// place (`NoticeBar` の `Behavior`): shot on the answer, it is half open.
        function settled() {
            return {
                "written": driver.wroteAndSettled(),
                "status": !driver.statusOwed || driver.owedStatusLanded(),
                "notice": !(page.noticeCard.open && !page.noticeCard.settled),
                "page": PageSettled.settled(page) }
        }
        onTriggered: {
            // Ended by `sayBroken`; `wroteAndSettled` stays false for good, so this would tick to the ceiling.
            if (repoTab.writeContractBroken()) {
                writeBarrier.stop()
                return
            }
            // This sampler can be what is still ticking at the ceiling, so it keeps that line current.
            if (!driver.wroteAndSettled())
                driver.noteWrite()
            if (driver.treeGoneRow === "" && driver.graphGoneOid === "") {
                writeBarrier.complete({ "name": "write_settled", "when": () => writeBarrier.settled(),
                                        "report": () => driver.writeLines().concat(driver.verbReport()) })
                return
            }
            if (!Awaited.all("write_settled", writeBarrier.settled()))
                return
            writeBarrier.advance("settled", () => driver.writeLines())
            writeBarrier.stop()
            if (driver.treeGoneRow !== "")
                treeBarrier.start()
            else
                graphBarrier.start()
        }
    }
    readonly property alias barrierWrite: writeBarrier
    /// The bar a write that did not happen comes down in, waited on until fully down (`NoticeBar.settled`): a picture
    /// taken on the answer catches the words written and the height still nothing. One barrier for every kind of
    /// report (`Words.writeReported`) — each run claims the same things (verify-ui verbs.md「報せバーの 10 本」).
    PageStage {
        id: noticeBarrier
        name: "notice_barrier"
        /// Whether the line also says `plan=` — only for the verbs whose bar comes up instead of a plan
        /// (`AutoActPlanVerbs`); on the others it would always read the same.
        property bool saysPlan: false
        onTriggered: {
            // And the page: a diff the verb opened under the bar (`notice-over-diff`) is a git subprocess away, and a
            // refusal's reread lands after its bar.
            noticeBarrier.complete({
                "name": "notice_barrier_ready",
                "when": () => ({ "bar": page.noticeCard.settled, "page": PageSettled.settled(page) }),
                "report": () => [driver.noticeLine(noticeBarrier.saysPlan)].concat(driver.verbReport()) })
        }
    }
    readonly property alias barrierNotice: noticeBarrier
    /// What a standing report says, for a run to be judged on: `log=` / `wrong=` that no error was raised beside the
    /// bar, which no picture can say (デザイン規約 §答えの要らない報せ); `plan=` only where asked (`saysPlan`).
    function noticeLine(withPlan) {
        return "write_notice open=" + page.noticeCard.open
               + " clears=" + page.noticeClears
               + " why=" + (page.noticeCard.detail !== "")
               + " tone=" + (page.noticeCard.tone === "" ? "none" : page.noticeCard.tone)
               + " log=" + page.commandsOpen
               + " wrong=" + page.commandsWrong
               + (withPlan === true ? " plan=" + page.planActive : "")
               // The heading, a sentence: after it only what follows from it.
               + " said=" + page.noticeCard.label
               // …for a heading that names a worktree: whether its first mark found the name
               // (`NoticeBar.namesWorktree`). Absent elsewhere, so no other line moves.
               + (page.noticeCard.namesWorktree ? " mark=" + page.noticeCard.markShown : "")
               // The second line, ours always, and the word of it that goes to the log (`NoticeBar.detailPlace`).
               + " place=" + page.noticeCard.detailPlace
               + " says=" + page.noticeCard.detail
    }
    /// The diff's first heading lit as under a pointer (`hunk-tools`), waited on until its row is built. `level=` is
    /// its two words on one line, and `caption=` the band's caption and path on one. Each is a label beside something
    /// built otherwise (a button deeper by its hold mark, a field), and whether the pair shares a baseline is what the
    /// face answers for a line — a few pixels apart reads in a picture as the words' own setting. The distances ride
    /// along for a run that came out false.
    PageStage {
        id: headingBarrier
        name: "heading_barrier"
        onTriggered: {
            // -1: the heading's row is not built yet.
            headingBarrier.complete({
                "name": "heading_barrier_ready",
                "when": () => ({ "row": diffPane.hunkWordsApart() >= 0, "page": PageSettled.settled(page) }),
                "report": () => {
                    const words = diffPane.hunkWordsApart()
                    const caption = diffPane.captionApart()
                    return ["hunk_tools level=" + (words < 1) + " caption=" + (caption < 1)
                            + " words=" + words + " head=" + caption].concat(driver.verbReport())
                } })
        }
    }
    readonly property alias barrierHeading: headingBarrier
    /// Where a verb's own operation `op` goes once the tree row its write took is out: the graph's barrier if a graph
    /// row is owed too, else the completion from `op`.
    // reach(beat)
    function afterTreeSettled(op) {
        if (driver.graphGoneOid === "") {
            op.complete({ "when": () => ({}), "report": () => driver.verbReport() })
            return
        }
        op.stop()
        graphBarrier.start()
    }
    // Waits `treeGoneRow` out of the bucket it was named in. A write git refused leaves the row, and the run to the
    // watchdog — which is the diagnosis.
    PageStage {
        id: treeBarrier
        name: "tree_barrier"
        /// `landed=` tells a file resolved into the index from one discarded out of the tree ("" = nowhere).
        function settledLine() {
            const cut = driver.treeGoneRow.indexOf(":")
            const path = driver.treeGoneRow.substring(cut + 1)
            return "tree_settled from=" + driver.treeGoneRow.substring(0, cut)
                   + " landed=" + unstagedModel.bucketOf(path)
                   + " conflicts=" + worktree.conflictCount
                   + " staged=" + worktree.stagedCount
                   + " unstaged=" + (worktree.unstagedCount + worktree.untrackedCount)
        }
        /// The row gone from the bucket it was named in.
        function gone() {
            const cut = driver.treeGoneRow.indexOf(":")
            return { "ready": !unstagedModel.holdsPath(driver.treeGoneRow.substring(0, cut),
                                                       driver.treeGoneRow.substring(cut + 1)) }
        }
        onTriggered: {
            if (driver.graphGoneOid === "") {
                treeBarrier.complete({ "name": "tree_barrier_ready", "when": () => treeBarrier.gone(),
                                       "report": () => [treeBarrier.settledLine()].concat(driver.verbReport()) })
                return
            }
            if (!Awaited.all("tree_barrier_ready", treeBarrier.gone()))
                return
            treeBarrier.advance("settled", () => treeBarrier.settledLine())
            treeBarrier.stop()
            graphBarrier.start()
        }
    }
    // Waits `graphGoneOid` off the graph and the stash total off its old value (see `graphGoneOid`).
    PageStage {
        id: graphBarrier
        name: "graph_barrier"
        function settledLine() {
            return "graph_settled gone=true top=" + driver.graphTopKind()
                   + " named=" + driver.stashNamed()
                   + " back=" + driver.stashCameBack()
                   + " rows=" + graphModel.rowTotal
                   + " stashes=" + stashesModel.total
        }
        /// The row off the graph and the stash total off its old value.
        function gone() {
            return { "ready": !(graphModel.rowOf(driver.graphGoneOid) >= 0
                                || stashesModel.total === driver.stashTotalBefore) }
        }
        onTriggered: {
            if (Harness.autoAct !== "stash-lands") {
                graphBarrier.complete({ "name": "graph_barrier_ready", "when": () => graphBarrier.gone(),
                                        "report": () => [graphBarrier.settledLine()].concat(driver.verbReport()) })
                return
            }
            if (!Awaited.all("graph_barrier_ready", graphBarrier.gone()))
                return
            graphBarrier.advance("settled", () => graphBarrier.settledLine())
            graphBarrier.stop()
            driver.awaitStashLanding()
        }
    }
    /// The two landings a press can have — where the write put the reader (`AutoActLandings`).
    AutoActLandings { id: landings; driver: driver }
    /// The page wiring's edges as report lines — none is a state a later reading could recover (`PageReports`).
    PageReports { driver: driver }
    /// Arms the stash landing — called once the graph the row went out of has settled.
    // reach(beat, press)
    function awaitStashLanding() {
        landings.awaitStash()
    }
    /// Arms the stopped-operation landing — called right after the press that puts the operation down.
    // reach(beat, press)
    function awaitOpExitLanding() {
        landings.awaitOpExit()
    }

    /// Claims the run for this page and dispatches the verb once the page it acts on has arrived.
    PageStage {
        id: autoActTimer
        name: "act"
        order: ["claimed", "dispatched"]
        onTriggered: {
            // A window's act completed the run while this page was still arriving: nothing is left to dispatch, and a
            // stage reached now would be one after the completion.
            if (Awaited.done) {
                autoActTimer.stop()
                return
            }
            // Tabs are constructed before their active index settles. The page that becomes current claims the one
            // process-wide verb; pages opened by that verb can never replay it. The claim is its own reading: it
            // answers false for every page but the one that took it.
            if (autoActTimer.at("start")) {
                if (!autoActTimer.input("claimed", () => page.pageCurrent && page.Window.window.claimAutoPageAct()))
                    return
                Awaited.holdPage(driver.completionHolds)
            }
            // `open` only means the path was accepted; the baseline is HEAD known and `PageSettled` — without the rows
            // agreeing with the status, a verb naming rows by number (`3:5:6`) presses one row off. A run holding the
            // worktree row back on purpose (`:row-late`, `AutoActCardFlowVerbs`) starts on the page that hold keeps
            // from settling, and lets the row through itself.
            const holdsTheRow = Harness.autoActArg.endsWith(":row-late")
            if (!Awaited.all("page_ready", {
                "open": repoTab.state === "open",
                "head": worktree.headKnown,
                "settled": PageSettled.settled(page) || holdsTheRow }))
                return
            // The dispatch is the run's first input: the verb's family presses what it presses and starts the
            // operations that wait for it (`runAutoAct`). Whether a family took the verb is the press's own answer,
            // kept as this generation's.
            if (Harness.autoAct !== "perf")
                driver.prepareCompletion(Harness.autoAct)
            autoActTimer.input("dispatched", () => {
                autoActTimer.witness("taken", driver.runAutoAct())
            })
            // The file-row verbs are still waiting on their rows; `fileRowsTimer` finishes their dispatch.
            if (autoActTimer.witnessed("taken") && driver.fileRowActs.indexOf(Harness.autoAct) < 0)
                driver.dispatchFinished(autoActTimer)
            // A verb that completed stopped this already.
            autoActTimer.stop()
        }
    }

    // The commit an argument names: an object name, "row:<n>" off the graph, or HEAD when empty — the demo
    // repository is built fresh every run, so its object names cannot be spelled ahead.
    function autoActOid(arg) {
        if (arg === "")
            return worktree.headOid
        if (arg.indexOf("row:") === 0)
            return graphModel.oidAt(Number(arg.substring(4)))
        return arg
    }

    // Which verbs write, defer their completion or are owed a status: three lists, read only from `prepareCompletion`.
    readonly property AutoActCompletion completion: AutoActCompletion { id: completion }

    /// The verbs, grouped by what they act on.
    AutoActWipVerbs { id: wipVerbs; driver: driver }
    AutoActFileRowVerbs { id: fileRowVerbs; driver: driver }
    /// One for the two nav families, so a peek one of them opened is the peek the other reads.
    readonly property NavProbe navProbe: NavProbe {
        id: navProbe
        sidebar: driver.sidebarPane
    }

    AutoActNavVerbs { id: navVerbs; driver: driver }
    AutoActNavBoxVerbs { id: navBoxVerbs; driver: driver }
    AutoActRefVerbs { id: refVerbs; driver: driver }
    AutoActSwitchVerbs { id: switchVerbs; driver: driver }
    AutoActPublishVerbs { id: publishVerbs; driver: driver }
    AutoActGraphRowVerbs { id: graphRowVerbs; driver: driver }
    AutoActRowMenuVerbs { id: rowMenuVerbs; driver: driver }
    AutoActCardFlowVerbs { id: cardFlowVerbs; driver: driver }
    AutoActHoverHoldVerbs { id: hoverHoldVerbs; driver: driver }
    AutoActHoverGoneVerbs { id: hoverGoneVerbs; driver: driver }
    AutoActRereadVerbs { id: rereadVerbs; driver: driver }
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
    AutoActDiffFindVerbs { id: diffFindVerbs; driver: driver }
    AutoActHandVerbs { id: handVerbs; driver: driver }
    AutoActWorktreeVerbs { id: worktreeVerbs; driver: driver }
    AutoActRecoverVerbs { id: recoverVerbs; driver: driver }
    AutoActBranchPushVerbs { id: branchPushVerbs; driver: driver }
    AutoActGithubVerbs { id: githubVerbs; driver: driver }

    /// The dispatch, pressed as `act`'s input: the family that knows the verb presses it and starts what waits for
    /// it. Answers whether the run goes on from here — a verb no family and no ledger knows is nobody's, and is left
    /// to the watchdog. A verb whose report is read at once hands it over (`reportAtCompletion`) and completes from
    /// `act` when the dispatch is through (`dispatchFinished`).
    function runAutoAct() {
        const act = Harness.autoAct
        const arg = Harness.autoActArg
        if (act === "perf") return false // WindowPerfDriver owns this verb's causal completion.
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
            || rowMenuVerbs.run(act, arg)
            || cardFlowVerbs.run(act, arg)
            || hoverHoldVerbs.run(act, arg)
            || hoverGoneVerbs.run(act, arg)
            || rereadVerbs.run(act, arg)
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
            || diffFindVerbs.run(act, arg)
            || handVerbs.run(act, arg)
            || worktreeVerbs.run(act, arg)
            || recoverVerbs.run(act, arg)
            || branchPushVerbs.run(act, arg)
            || githubVerbs.run(act, arg)
        if (!known && !driver.completionDeferred && !driver.writeExpected) {
            // A misspelling: window verbs are in the completion ledger, so a verb known to neither is
            // nobody's. Left to the watchdog.
            Harness.report("auto_act unknown=" + act)
            return false
        }
        // No `inputWent` here: a verb that presses from a sampler has not put its input in yet, and claiming it
        // would make its later arm read as a second write over an unfinished first.
        Harness.report("auto_act ran=" + act)
        return true
    }
}
