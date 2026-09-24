pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// **A focus scope**, and that is what keeps Escape answerable: a pane swapped off the
// screen lets the keyboard go (規約 §矢印で履歴を辿る), and inside a plain item it falls all the way out of the page to
// the window's own content item — where this page is a descendant rather than an ancestor, so nothing here is on the
// key's way any more and the one gesture that puts a standing thing away goes quiet until the next press
// (measured with qmltestrunner; the facts are held by `tests/qml/tst_escape.qml`). A scope catches that fall: it
// keeps active focus itself and `Keys.onEscapePressed` below goes on hearing the key.
FocusScope {
    id: page
    required property int index
    required property int tab_id
    readonly property bool blank: tab_id < 0

    /// The blank page's "Open repository…" button (folder picker).
    signal openRepositoryPicker()
    /// A worktree row was clicked: open that path as a new tab.
    signal openRepositoryPathRequested(string path)
    /// PGG_AUTO_ACT=settings wants the window's settings dialog open for the screenshot.
    signal settingsDialogRequested()
    /// A conflicted file has nowhere to be opened: the git settings screen is where the merge editor is named.
    signal gitSettingsRequested()
    /// The settings card, opened from an avatar and carrying whom it was opened on.
    signal avatarSettingsRequested(string name, string email)
    /// PGG_AUTO_PERF completion after every requested measurement output.
    signal perfFinished()
    /// The failed-open screen's "Close tab" button.
    signal closeTabRequested()
    /// The working copy this tab is standing in would not open, and the repository has its own one to stand in
    /// instead (`RepoTab.standHomeAsked`). The strip is what moves the tab.
    signal standHomeRequested()

    property string selectedOid: ""

    property bool sidebarCollapsed: false
    /// The fold the diff put on: closing the diff takes back only that; a fold by hand stays.
    property bool foldedByDiff: false
    function foldForDiff(open) {
        if (open) {
            if (page.sidebarCollapsed)
                return
            page.sidebarCollapsed = true
            page.foldedByDiff = true
        } else if (page.foldedByDiff) {
            page.sidebarCollapsed = false
            page.foldedByDiff = false
        }
    }
    /// The same for the plan: the mode takes the graph pane whole, and the list beside it is a column of doors into a
    /// history the plan is composed over — so it goes down to the rail with the rest of the window's other business
    /// (規約 §フル interactive rebase). Its own flag, so putting the plan away takes back only the fold the plan made.
    ///
    /// **The fold comes back when the plan does.** From the run onwards the list has nothing left to say about a
    /// screen that is no longer standing over it, and the reader is back on the graph watching the rewrite land —
    /// the hold on the doors carries on without it (`doorsHeldWhy`).
    ///
    /// Unlike the diff's, there is no way back by hand while the plan stands: the rail's own fold control is inside
    /// the pane the plan freezes (`sidebarFrozen`).
    property bool foldedByPlan: false
    function foldForPlan(open) {
        if (open) {
            if (page.sidebarCollapsed)
                return
            page.sidebarCollapsed = true
            page.foldedByPlan = true
        } else if (page.foldedByPlan) {
            page.sidebarCollapsed = false
            page.foldedByPlan = false
        }
    }
    /// A plan has been handed over and its replay has not started yet, so the hold can begin at the press:
    /// `replaying` rises when the queue *starts* the write.
    ///
    /// **The gap runs as long as the queue takes, and no sequence number spans it.** The run is queued, and the
    /// queue can carry something else — the timer's fetch travels it too — so the wait is however long that
    /// takes; and `writeSeq` counts *every* write's answer, so the fetch landing in between would move it past
    /// anything armed here and let the pane back to life with the rebase still to come. Let go of the moment a
    /// replay is under way, which is where `replayRunning` takes over.
    property bool planRunOut: false
    readonly property bool replayRunning: repoTab.replaying || page.autoReplayHeld
    onReplayRunningChanged: if (page.replayRunning) page.planRunOut = false
    /// Automation: a replay this run really started, kept standing until its picture has been taken
    /// (`AutoActNavVerbs`, PGG_AUTO_ACT=doors-held). The rise is caught at the signal — a demo repository's rebase is
    /// over inside one beat of the sampler — and what it holds up is the state the picture is
    /// of (app-ui.md §UI 自動化の因果性: 一瞬だけ立つ状態は signal で観測して latch する).
    property bool autoReplayHeld: false
    /// **The two ways the left pane is out are two different things, shown two different ways.**
    ///
    /// This one is the plan's: while a rebase is being composed the only way into a write is the run button, so the
    /// whole pane goes to the disabled step and stays there — the restriction belongs to the mode, and lasts as
    /// long as the mode does, so it is said plainly (デザイン規約 §フル interactive rebase). Only
    /// the `>_` band is left out of it (`SidebarPane.frozen`).
    ///
    /// **From the press**: the mode is entered when the face takes the graph's seat, and a pane that stays live
    /// under it for the length of the read would be saying the reader may still write — which is
    /// the one thing this mode is for taking away.
    readonly property bool sidebarFrozen: page.planShown
    /// The other one, which begins where the plan ends: a write that replays a range a commit at a time is running
    /// (`RepoTab.replaying` — the meaning core puts on the op name), or its run is out and has not started yet. A
    /// rebase is measured in seconds once the range is deep, and a switch or a delete let go into the middle of one is
    /// the exit nobody meant — but the answer here is "wait for it".
    ///
    /// **So nothing freezes and nothing leaves**: the doors are held one at a time and each says this line
    /// (`SidebarPane.doorsHeld` / `AppMenu.heldReason`), and the lock comes off the moment git answers. One line for
    /// both halves of that, so the pane and the menus cannot disagree about whether a rewrite is under way.
    readonly property string doorsHeldWhy:
        page.replayRunning || page.planRunOut ? Words.otherCommandRunning : ""
    /// A press landed away from whatever held the keyboard (Main's `FocusRelease`), at `scenePos` — `null` for a press
    /// with no place of its own (the headless run's door). The left menu's name box goes with it — nothing is asked,
    /// what it costs is the typing (デザイン規約 §左メニューの所作) — and so does anything over the graph that is standing on an
    /// empty box, where there is not even that to lose (§コミットを探す).
    function releasePressedAway(scenePos) {
        sidebarPane.stopEdit()
        graphPane.dropEmptyBoxes(scenePos)
    }
    /// A press landed anywhere in the window (Main's `FocusRelease` again, this one on every press). The right
    /// pane's mark says where a reader was just sent, and the first thing they do after arriving is the answer that
    /// they arrived (デザイン規約 §hover のツールチップ).
    function notePress() {
        detailsPane.dropAttention()
        // The same answer for the same question, on the other mark this window raises (`raiseCommands`). **A link is
        // answered on the release and this is read on the press**, so the press that sends a reader here takes the
        // mark down a moment before putting it up — and what stands afterwards is the one they were sent for.
        page.commandsAttention = false
    }
    /// And Escape, which every other standing thing in this window answers (デザイン規約 §hover のツールチップ).
    ///
    /// **A key handler**, and that is what keeps it out of everyone else's way: a shortcut is
    /// matched before the key is delivered at all, so a bar, a popup or a box that wants Escape takes it first and
    /// this is never reached — which is the rule itself, since the mark is the last thing left to dismiss. **Two
    /// enabled `StandardKey.Cancel` shortcuts in one window fire neither**, so a third one here would have taken
    /// whichever bar was standing down with it (`tests/qml/tst_escape.qml` holds all of this). The two bars keep one
    /// live between them by an order written where they meet (`noticeBar.yieldsEscape`); anything else that comes to
    /// want Escape is added the way this one is, as a key handler under all of them.
    ///
    /// **Topmost first, and the marks last** — the mark is what is left to dismiss when nothing else stands, so
    /// anything that took the centre answers before it does. Everything above this line has already had its chance:
    /// a shortcut, a popup or a box that wants Escape was matched before the key was delivered here at all.
    ///
    /// Accepted only when there was something to take: an Escape this page did nothing with is not this page's.
    function escapePressed() {
        // **First, because it has taken the pointer and the view.** A middle-click autoscroll is a mode: the cursor
        // is not the one the reader put there and the rows are going by on their own, and everything standing in this
        // window answers Escape (デザイン規約 §hover のツールチップ). Nothing behind it is reached while it is up, so it
        // is also the only door here that can never be the wrong one. **Whichever surface it is running on** — every
        // one that scrolls has a hand, and there is one gesture in the window (`MiddleHand`).
        if (MiddleHand.stop())
            return true
        // The plan has the centre and names its own way out, so Escape is that door — and
        // **only while that door is a press**. With something composed to lose the button is a hold (規約 §長押し —
        // `RebasePlanPane.discards`), and one key down is not a hold; a key that threw away typed rows because it
        // could not be held would be the very thing the hold is there to stop.
        if (page.planShown) {
            if (page.planDiscards)
                return false
            planModel.cancelPlan()
            return true
        }
        // The diff stands over the graph and costs nothing to put away — the same one press its own `✕` takes
        // (デザイン規約 §hover のツールチップ「この窓で立つものは全部 Esc に答える」). The reading position and the
        // lines picked out in it are the only things it holds, and the `✕` already spends both. The arrows go to the
        // graph with it, which is the same door the mark uses (`closeDiffToGraph`).
        if (page.diffShown) {
            page.closeDiffToGraph()
            return true
        }
        // The fourth door into the log, beside `>_`, `Clear` and the closing mark (デザイン規約 §git が言ったことを読む場所).
        // It stands over nothing — it takes a room of its own behind a splitter — which is why it was first counted
        // with the left menu, which Escape does not fold either. What tells the two apart is the one thing the menu
        // never does: **a failed command raises this panel, and nothing but a hand puts it back down**. Below the
        // centre and above the marks, since a reader with the diff up is not reaching past it for this. The reader's
        // own hand, so `shutCommands` — a panel Escape put away is not one the next landing fetch may take over.
        if (page.commandsOpen) {
            page.shutCommands()
            return true
        }
        if (!detailsPane.attention)
            return false
        detailsPane.dropAttention()
        return true
    }
    Keys.onEscapePressed: event => {
        event.accepted = page.escapePressed()
    }
    function foldByHand(collapse) {
        page.sidebarCollapsed = collapse
        page.foldedByDiff = false
        // Every way the list comes back arrives here — the band's block, a rail cell, the rename box
        // (SidebarPane.startEdit) — so "unfolding closes the diff" has no exceptions elsewhere.
        if (!collapse && page.diffShown)
            page.closeDiff()
    }

    property bool wipShown: false
    // The ask bar goes off screen with the pane, and nothing off screen may be answered.
    onWipShownChanged: if (!page.wipShown) page.stopRowAsk()
    // Selected stash row's reflog selector ("" = not a stash).
    property string selectedStashRef: ""
    function showWip() {
        // **This face is this window's tree unless something says otherwise**, and the one thing that says so is the
        // row being read (`openWipFor`, the only caller that puts a copy back afterwards). Every other road here — a
        // stopped operation landing, the default selection, the verbs — is about this tree, and any of them arriving
        // while a copy was on screen would otherwise leave the copy's files under this tree's selection.
        page.dropCarried()
        page.wipShown = true
        page.pendingHeadSelect = false
        page.pendingHeadAsked = false
        page.pendingWipSelect = false
        page.selectedOid = ""
        page.selectedStashRef = ""
        // The working tree's row is not a commit, so nothing is held while it is what is shown — and the rows fall
        // back to the current one for the highlight there (`GraphRowDelegate.selected`).
        page.chooseOnly("")
        page.closeDiff()
    }

    // ---- which working copy the WIP pane is about ---------------------
    //
    // **Every uncommitted row carries git's all-zero id**, this window's own included: that spelling is git saying
    // there is no object here, and it is as true of a copy's row as of ours (P3-確認事項 §別 worktree の未コミット行).
    // So which copy a row is about is the row's own — and it is the predicate the whole read-only pane hangs
    // off: **empty means this window's own tree, and only then can anything here be written.**
    property string carriedPath: ""
    property string carriedName: ""
    /// Whether the pane may write what it is showing. One property, read all the way down: the buckets' whole-bucket
    /// buttons, the rows' marks, the commit block, the band's stash, and the diff's own staging
    /// (デザイン規約 §無効).
    readonly property bool wipWritable: page.carriedPath === ""
    /// **The list the right pane is showing**, whichever pane that is: this window's own unstaged run, or the one
    /// folded list another copy's changes are shown as (`CarriedPane`). Everything this page asks about the files on
    /// screen is asked of it, so the copy being read is chosen in one place and the questions below do not each have
    /// to know. The two panes agree on what it can answer — which bucket holds a path, what is beside it, where a
    /// rename came from — because both lists are built from the whole of one status (`models::nav::Source::files`).
    readonly property var wipUnstaged: page.wipWritable ? worktreeModel : carriedModel
    /// Tree or flat paths, for both trios at once: the choice is the pane's, so stepping onto another copy and back
    /// keeps it — and what is saved at close is the one answer either trio would give.
    /// Reads the diff as two columns or as one — the band's toggle, and the machine's saved choice as the page
    /// opens (`PageLayout.applySavedLayout`). The model lays its rows out again; what the page owns is the choice
    /// (デザイン規約 §diff を 2 列で読む).
    function setDiffSplit(split) {
        diffModel.setSplit(split)
    }
    function setWipTreeView(tree) {
        conflictsModel.setTreeView(tree)
        worktreeModel.setTreeView(tree)
        stagedModel.setTreeView(tree)
        carriedModel.setTreeView(tree)
    }
    /// Opens the WIP pane on whichever working copy row `row` is — this window's own, or another copy's.
    function openWipFor(row) {
        const at = row >= 0 ? graphModel.carriedPath(row) : ""
        // Already the copy being read: a second click on that row has nothing new to ask for, and clearing the pane
        // would close the file opened from it under the hand that clicked.
        if (at !== "" && at === page.carriedPath) {
            page.wipShown = true
            return
        }
        // The light in the list names a file of the copy being left — a name the copy arriving may well have too.
        // Stepping between our own row and itself is not a step, and keeps the light where it was.
        if (at !== page.carriedPath)
            wipPane.clearChoice()
        page.showWip()
        if (at !== "")
            page.standOnCopy(at, graphModel.carriedName(row))
    }
    /// Points the pane at one copy and asks for its files — the read behind the pane, which the copies' tick then
    /// keeps current for as long as it stands open (`pollCarried`).
    function standOnCopy(at, name) {
        page.carriedPath = at
        page.carriedName = name
        repoTab.readCarriedStatus(at, name)
    }
    /// The pane is about this window's own tree again — every road to a commit and to our own row comes through here.
    function dropCarried() {
        page.carriedPath = ""
        page.carriedName = ""
    }
    /// Follows the copy being read across a graph pass, and lets go of it where the copy has gone clean and taken its
    /// row with it. **Addressed by the copy**: the rows move under a rebuild, and every one of them answers to the
    /// same all-zero id.
    function settleCarriedAfterPass() {
        if (page.carriedPath === "")
            return
        const row = graphModel.carriedRowOf(page.carriedPath)
        if (row >= 0) {
            page.selectedRow = row
            graphPane.setCurrentRow(row)
            return
        }
        // Nothing to show and nothing to stand on: the copy committed, or put its changes away.
        //
        // **The landing is where that copy now stands.** An uncommitted row names one working copy and nothing else
        // — a copy with no branch out has one all the same — so the copy is what the reader was reading, and the
        // commit it has just made is where it went. Asked of the worktree listing by the copy's path
        // (`NavSectionModel.headOfCopy`), which is one lookup over a handful of entries.
        //
        // **A listing a tick behind lands one commit behind**, on the copy's previous HEAD — still that copy's own
        // history, and the next pass leaves the reader there because that commit is still drawn.
        const head = worktreesModel.headOfCopy(page.carriedPath)
        const headRow = head === "" ? -1 : graphModel.rowOf(head)
        page.dropCarried()
        page.wipShown = false
        page.selectedOid = ""
        if (headRow >= 0) {
            graphPane.setCurrentRow(headRow)
            page.activateRow(graphModel.oidAt(headRow), headRow)
            return
        }
        // The copy is gone from the listing (removed, pruned, or bare): there is no copy left to follow, so the page
        // falls back to what it would have opened on.
        page.trySelectDefault()
    }

    // ---- commit editor -------------------------------------------
    property bool amending: false
    // Whether a remote already has the commit HEAD is on — the amend's `already pushed`. Answered by the session off
    // the walk's own marks and kept beside HEAD (`WorkTreeModel.headPublished`), so nothing here asks git for it and
    // a HEAD that moved wears no answer about the commit it left.
    readonly property bool headPublished: workTree.headPublished
    /// Whether the tab was standing in another working copy at the last drain (`RepoTab.standing`). The edge is
    /// read here rather than in a handler of its own: every property of that model shares one notify, so the only
    /// way to see one of them turn over is to have kept what it was (規約 §UI 自動化の因果性 —
    /// "まだ答えが無い" と値を分ける).
    property bool wasStanding: false
    Connections {
        target: repoTab
        function onChanged() {
            if (page.wasStanding !== repoTab.standing) {
                page.wasStanding = repoTab.standing
                if (!page.wasStanding)
                    page.standSettled()
            }
            page.absorbHeadMessage()
            page.absorbMoveAsk()
            page.absorbWriteResult()
            page.absorbFetchRecovery()
        }
        // Only the first failure of a run: an offline machine would otherwise re-raise the panel every interval (デザイン規約
        // §git が言ったことを読む場所).
        function onFetchFirstFailed() {
            // What this raises is what a fetch that reaches the remote again takes back down (`absorbFetchRecovery`).
            // Asked before the raise, because what the answer turns on is whether a panel was already standing.
            commandsOwner.fetchRaises(page.commandsOpen)
            page.commandsOpen = true
        }
    }

    // A tab whose repository would not open. The screen sits where the panes do, so the log's seat and the panel
    // both stay reachable. The log stays down: the failed command is a background read, so the panel would come up
    // empty (measured).
    readonly property bool openFailed: !page.blank && repoTab.state === "error"
    /// Automation: that screen's hand — the one its three lines are dragged over from the air around them
    /// (`PGG_AUTO_ACT=open-fail-sweep`). An automation-only exposure, the same one `GraphPane.view` is (app-ui.md).
    readonly property alias openFailedHand: failedScreen.pad

    property int seenHeadCommitSeq: 0
    property bool wantHeadMessage: false
    function amendToggled(on) {
        page.amending = on
        if (on) {
            page.wantHeadMessage = true
            repoTab.requestHeadCommit()
        } else {
            page.clearCommitEditor()
        }
    }
    function absorbHeadMessage() {
        if (repoTab.headCommitSeq === page.seenHeadCommitSeq)
            return
        page.seenHeadCommitSeq = repoTab.headCommitSeq
        if (!page.wantHeadMessage)
            return
        page.wantHeadMessage = false
        wipPane.setMessage(repoTab.headSubject, repoTab.headBody)
    }
    function clearCommitEditor() {
        wipPane.clearMessage()
        page.opFilledSubject = ""
        page.opFilledBody = ""
    }

    /// A stopped merge opens the box already holding what it is about to record.
    ///
    /// The button under it is what finishes a merge — `git commit` there writes the very commit `--continue` would,
    /// down to the tree, the two parents and the hooks it runs (measured, 2.55) — so the message it will use belongs
    /// on screen before the press (デザイン規約 §進行中の操作から出る).
    ///
    /// **Answered once per message, and only into empty boxes.** Text in the boxes is the one thing here that
    /// cannot be read back off disk, so a merge arriving under it leaves it where it is; the merge's own words are
    /// still the placeholder underneath, and emptying the boxes commits them.
    ///
    /// **Taken back out when the merge goes** — but only where it is still standing untouched, which is what the pair
    /// below is for: an abort leaves a box holding a message for a merge that no longer exists, and one word typed
    /// into it makes it the reader's.
    property string seenOpMessage: ""
    property string opFilledSubject: ""
    property string opFilledBody: ""
    function absorbOpMessage() {
        const subject = workTree.opMerging ? workTree.opSubject : ""
        const body = workTree.opMerging ? workTree.opBody : ""
        const message = subject === "" ? "" : subject + "\n" + body
        if (message === page.seenOpMessage)
            return
        page.seenOpMessage = message
        const ours = page.opFilledSubject !== ""
                     && wipPane.subjectText === page.opFilledSubject
                     && wipPane.bodyText === page.opFilledBody
        page.opFilledSubject = ""
        page.opFilledBody = ""
        if (subject === "") {
            if (ours)
                wipPane.clearMessage()
            return
        }
        if (!ours && (wipPane.subjectText !== "" || wipPane.bodyText !== ""))
            return
        page.opFilledSubject = subject
        page.opFilledBody = body
        wipPane.setMessage(subject, body)
    }

    /// The name the entry a pop is bringing back was carrying, held until git says the pop landed.
    ///
    /// **Read at the press**: by the time the answer comes the entry is off the list, and nothing else holds the
    /// words. Empty for one git named itself (`WIP on …`) — that line names the commit the work was standing on,
    /// not the work.
    property string pendingPopLabel: ""
    /// The id the queue gave the pop at the press (`RepoTab.popStash`), so its answer is found by name. Zero while
    /// no pop is out.
    ///
    /// **Every stash operation answers under the same word** (`writeStashed` cannot say whose), so the answer alone
    /// does not say it was the pop's: the details pane's band leaves its buttons live, and an `apply` pressed just
    /// before a pop answers first — counted by turn, its landing would be taken for the pop's, and the pop's own
    /// refusal for somebody else's. The id says which answer is this one's whatever answered in between, and nothing
    /// else disarms it: no other answer can be mistaken for the pop's, so there is nothing to disarm against.
    property int pendingPopId: 0
    /// Both ways in to a pop — the graph row's menu and the details pane's band — so the name comes back from one
    /// place (デザイン規約 §変更を退避する).
    function popStash(selector) {
        page.pendingPopLabel = GitFacts.stashLabel(stashesModel.nameOfFull(selector))
        page.pendingPopId = repoTab.popStash(selector)
        page.selectedStashRef = ""
    }
    /// The answer to that pop, whichever way it went. Read before the refusal branch below so both landings pass
    /// through here — and the words only go in where the pop is what answered, and it landed
    /// (デザイン規約 §変更を退避する. by design). Looked up by id in the answers this notify carried
    /// (`RepoTab.writeAnswerIndex`): a notify that did not carry it leaves the wait standing.
    ///
    /// **Only into empty boxes.** Text in these boxes is the one thing on this page that cannot be read
    /// back off disk (`absorbOpMessage`), and both are asked: a description with no summary is not an empty editor.
    function absorbPopLabel() {
        if (page.pendingPopId === 0)
            return
        const answer = repoTab.writeAnswerIndex(page.pendingPopId)
        if (answer < 0)
            return
        const carried = page.pendingPopLabel
        const landed = !repoTab.writeAnswerFailed(answer)
        page.pendingPopLabel = ""
        page.pendingPopId = 0
        if (landed && carried !== "" && wipPane.subjectText === "" && wipPane.bodyText === "")
            wipPane.setMessage(carried, "")
    }

    // Run straight, even when HEAD is already on a remote: amending rewrites nothing that a switch or a reset cannot
    // bring back, and the push that would spread it is asked about on its own.
    //
    // **The id the queue took it under is written down by the slot itself** (`ops::Press`), so nothing here holds
    // one and the answer below is found without reading anything into what else came back.
    function commitNow() {
        // The button is down while the pane is another copy's (`WipCommitBlock`); this is the belt under it.
        if (!page.wipWritable)
            return
        repoTab.commit(wipPane.outgoingSubject, wipPane.outgoingBody, page.amending, wipPane.resetAuthor)
    }
    /// The answer to that commit, whichever way it went — **its own answer**, out of the ones this notify carried
    /// (`RepoTab.commitAnswer`, -1 where it carried none). A drain empties the whole queue and notifies once, so the
    /// fetch running behind the press answers in the same batch: read off a property every answer rewrites, a commit
    /// that landed would leave the editor full of a message that is already a commit, and one a hook turned down
    /// would be refused with nobody told.
    ///
    /// Everything the page does with a commit is here, so the branch below — which reads what is left over for the
    /// answers nobody waited for — never reaches this one and cannot do any of it a second time.
    function absorbCommitAnswer() {
        const answer = repoTab.commitAnswer
        if (answer < 0)
            return
        const landed = !repoTab.writeAnswerFailed(answer)
        if (landed) {
            // The message is on its commit: the boxes have done their job, and the amend they may have been filled
            // from is over.
            page.clearCommitEditor()
            wipPane.setAmendChecked(false)
            page.amending = false
            // …and the commit moved what the two sides hold, so a diff left open on either is a picture of a file as
            // it was.
            page.readDiffForAnswer()
        } else {
            // A rejected commit keeps its text — the boxes are the one thing on this page that cannot be read back
            // off disk — and what git said has nowhere else to go.
            page.tellRefusal(answer)
        }
        page.commitAnswered(landed)
    }
    /// The answer to the stash press that took the working tree away — **its own answer**, out of the ones this
    /// notify carried (`RepoTab.stashAnswer`, -1 where it carried none). The tree it emptied is a second wait of its
    /// own, answered where the page acts on a tree (`leaveWipWhenDone`); this is the half that is over as soon as
    /// git speaks.
    function absorbStashAnswer() {
        const answer = repoTab.stashAnswer
        if (answer < 0)
            return
        if (repoTab.writeAnswerFailed(answer)) {
            page.tellRefusal(answer)
            return
        }
        // The entry took what was in the tree, so a diff left open on either side is a picture of a file as it was.
        page.readDiffForAnswer()
    }
    /// The answer to the toolbar's push — **its own answer**, out of the ones this notify carried
    /// (`RepoTab.pushAnswer`, -1 where it carried none). The button's mark is the flow's to work out
    /// (`PublishFlow.noteWriteAnswer`); what git said is the page's, said the way every press's refusal is said —
    /// a far side that explained itself comes down as a report, one that did not raises the log (`tellRefusal`).
    /// A push that landed asks nothing of the page: what it moved comes back as refs.
    function absorbPushAnswer() {
        const answer = repoTab.pushAnswer
        if (answer < 0 || !repoTab.writeAnswerFailed(answer))
            return
        page.tellRefusal(answer)
    }
    /// The answer to a push a **ref row** sent — the two `push --delete`s, the replace git has no command for, and the
    /// two pairs that reach over there after doing something here (`RepoTab.refPushAnswer`, -1 where this notify
    /// carried none).
    ///
    /// **Its own answer.** These all answer under the word `push`, like the fetch running behind them and like the
    /// toolbar's own button, so a fetch coming back in the same drain took the group over and the far side's refusal
    /// was never said at all (P3-確認事項, observed). The rows the delete took away come back either way — that is
    /// the delete's own owner — and what only this carries is why.
    function absorbRefPushAnswer() {
        const answer = repoTab.refPushAnswer
        if (answer < 0 || !repoTab.writeAnswerFailed(answer))
            return
        page.tellRefusal(answer)
    }
    /// What one answer that did not happen has to say for itself, whoever was waiting for it: somebody outside this
    /// application turned it down — a hook here or over there, a remote this end had only an older picture of.
    /// Nothing here could have known beforehand and nothing here can answer it, so what they said comes down as a
    /// report and the log stays where the reader left it (デザイン規約 §答えの要らない報せ).
    ///
    /// Says whether there was one, because what is left when there is not differs for every reader: the editor
    /// raises the log, and the card that stayed up for a delete turns its own row instead.
    ///
    /// **Read off the answer** — a report belongs to the answer that carried it, and one drain can bring several
    /// (`RepoTab.writeAnswerReportKind`).
    function reportRefusal(answer) {
        const kind = repoTab.writeAnswerReportKind(answer)
        if (kind === "")
            return false
        const remote = repoTab.writeAnswerReportRemote(answer)
        const name = repoTab.writeAnswerReportName(answer)
        page.showReport(kind, remote, name, repoTab.writeAnswerReportReason(answer))
        // …and the mark in the corner goes quiet with it, the way every answered report takes it down: the row it is
        // about may not have reached the log yet, and then this pays for it when it does.
        if (!commandsModel.failed)
            page.answeredFailures++
        commandsModel.noteAnswered()
        page.writeReported(kind, remote, name)
        return true
    }
    /// The whole of what a press does with a refusal it has nothing else to answer with: the words come down as a
    /// report where somebody outside wrote them, and where nobody did, the log comes up because nothing else on
    /// screen says what git said (デザイン規約 §git が言ったことを読む場所).
    ///
    /// **The card that stayed up for a delete is the one exception** and calls `reportRefusal` alone — its row is
    /// the answer, so there is nothing to raise over it.
    function tellRefusal(answer) {
        if (page.reportRefusal(answer))
            return
        commandsOwner.newsTakes()
        page.commandsOpen = true
    }
    /// Automation: the editor's own commit has been answered and the boxes have been dealt with — **the one thing no
    /// picture can make**, since git answers before the reading that redraws the pane and the window photographs the
    /// same either way. An automation-only exposure, the same one `GraphPane.view` is (app-ui.md).
    signal commitAnswered(bool landed)

    // ---- moving between branches and commits ----------------------
    // Terminology is deliberate: git runs `switch` / `restore`, and the UI says "Switch to" (デザイン規約 §用語).
    //
    // Uncommitted changes come along unasked; where git refuses, core goes round through a stash (デザイン規約
    // §未コミット変更がある状態での移動).
    //
    // kind is "branch" / "remote" / "force" (a local branch moved to `moveStart` before landing on it). Every one lands
    // on a branch — nothing here moves onto a bare commit (デザイン規約 §ブランチ・コミットへの移動).
    property string moveKind: ""
    property string moveTarget: ""
    property string moveLocal: ""
    property string moveStart: ""

    // ---- the same chip, pressed again ------------------------------
    /// The branch a move already sent is putting HEAD on; `""` while none is on its way.
    ///
    /// **A press is decided from what the screen holds, and the screen takes a move in long after the write that made
    /// it has answered** — core answers a write and only then publishes the refs it moved (`session::write`), and a
    /// listing of fifty thousand refs is the slow half of that. Pressed twice on a remote branch with no local one,
    /// the second press is decided from the very picture the first was and sends the same `switch --create`, which
    /// git refuses because the first one made the branch.
    ///
    /// So while a move is on its way, the road a press arrives by (`switchToRef`) sends nothing. **What ends the wait
    /// is the move's own landing**: HEAD on the branch and that branch in the listing
    /// (`offers::move_landed` — a seq would move for reads nobody asked for). Everything else that can become of a
    /// move puts it down where it happens, and each of those certainly comes: a refusal (nothing moved, so pressing
    /// again is the reader's to do) and the question a move can come back as (`absorbMoveAsk`).
    property string moveLanding: ""
    function absorbMoveLanding() {
        if (page.moveLanding !== ""
                && GitFacts.moveLanded(page.moveLanding, workTree.branch,
                                       branchesModel.oidOfName(page.moveLanding)))
            page.moveLanding = ""
    }

    function switchTo(kind, target, local, start, leaving) {
        page.moveKind = kind
        page.moveTarget = target
        page.moveLocal = local
        page.moveStart = start === undefined ? "" : start
        if (page.standsInTheWay(leaving)) {
            page.askLeaveOperation(function () { page.runSwitch(true) })
            return
        }
        page.runSwitch(leaving === true)
    }
    function runSwitch(leaving) {
        // Marked where the move goes out: the question a blocked move raises comes back through here, and a press
        // held behind a bar has sent nothing yet.
        page.moveLanding = page.moveLocal
        if (page.moveKind === "branch")
            repoTab.checkoutBranch(page.moveTarget, leaving === true)
        else if (page.moveKind === "remote")
            repoTab.checkoutRemote(page.moveTarget, page.moveLocal, leaving === true)
        else if (page.moveKind === "force")
            repoTab.checkoutForceCreate(page.moveTarget, page.moveStart, leaving === true)
    }

    // ---- moving out of what stands in a move's way --------------------
    // What blocks a move — an operation standing or unmerged paths — is core's measured rule
    // (offers::moves_blocked → `workTree.movesBlocked`). The move has to clear the way first, and that is a question
    // (デザイン規約 §進行中の操作から出る).
    //
    // **One question for both.** The second needs no operation put down, and everything after that is the same two
    // commands, so it is the same question with one clause fewer.
    //
    // **Asked before anything is sent.** The screen already knows what is standing, so letting git refuse would put a
    // red line in the command log where a question belongs — and leave the reader where they were with nothing to
    // press.
    function standsInTheWay(leaving) {
        return leaving !== true && workTree.movesBlocked
    }
    readonly property string leaveHeading:
        workTree.leaveUndoes ? qsTr("Undo it and go?") : qsTr("Put it aside and go?")
    readonly property string leaveDetail:
        workTree.leaveUndoes ? qsTr("Nothing it did since it started is kept.")
        // Nothing standing: the files are the whole of what is in the way, and there is no operation to name.
        : workTree.opCommand === ""
        ? qsTr("The files waiting on a decision go to the stash, markers and all.")
        //: %1 is the standing operation in git's own spelling, e.g. cherry-pick.
        : qsTr("The %1 stops; its files go to the stash, markers and all.").arg(workTree.opCommand)
    function askLeaveOperation(retry) {
        // Marked on the row HEAD stands on — a rebase runs detached, so that is the only name the tree has for where
        // it is (デザイン規約 §立っている質問は 1 か所で聞く). Whether leaving undoes anything — the rebase, whose abort throws away
        // work in hand: `danger` and a hold (§状態, §長押し) — and the command that opens the question's line are core's
        // answers (offers::leaving_undoes / leave_code → `workTree.leaveUndoes` / `leaveCode`).
        page.startRowAsk(workTree.headOid, page.leaveHeading, page.leaveDetail,
                         workTree.leaveUndoes, "", retry, workTree.leaveUndoes, "", null,
                         workTree.leaveCode)
    }

    // ---- what a chip leads to --------------------------------------
    // `chip` is the chip as it is drawn (`encode::Chip`); null is a row that draws none.
    function activateChip(chip) {
        if (chip)
            page.switchToRef(chip.kind, chip.name)
    }
    // A branch another working copy holds is the one refusal no stash gets past and no operation put down can clear
    // (offers::SwitchAction) — the branch is simply somewhere else, and the way to it is that copy. **So the press
    // goes there**: the tab stands in that copy, down the same road the WORKTREES row takes
    // (`openRepositoryPathRequested`). Nothing is asked in front of it — the press writes nothing to the repository
    // and is one press back — and the rows that carry words name the copy before they are pressed
    // (`RefRowMenu` の `Open`, デザイン規約 §進行中の操作から出る).
    function openHolder(local) {
        const held = worktreesModel.worktreeHolding(local)
        if (held !== "")
            page.openRepositoryPathRequested(held)
    }
    /// Answers whether the press did anything — a move sent, or a question raised in front of one. `false` is a press
    /// this road turned away, which is what the headless double press reads (動詞 `switch-remote-twice`): the second
    /// of two presses in one turn has to be turned away, and the gate that turns it away is not something the run can
    /// see from outside (both presses look alike, and the window after them differs only by a line in the log).
    function switchToRef(kind, name, leaving) {
        // `busyCount` alone is not the gate: it rises when the queue starts the write, and it is back down while the
        // screen is still catching up with what the write did (`moveLanding`).
        //
        // **The held doors are the third of them**, and the one that covers the beat between a plan's run being handed
        // over and the queue starting it: the count is still zero there, and a move let go into that gap lands ahead
        // of the rewrite it was made about. This road is where the graph's own doors end up — a row's double-click, a
        // chip's, a row of the list a chip had to stack — so holding it here holds all three (`doorsHeldWhy`).
        if (repoTab.state !== "open" || repoTab.busyCount > 0 || page.moveLanding !== ""
                || page.doorsHeldWhy !== "")
            return false
        // The models hold the lookups — the local branch a remote row lands on is the one another copy can be holding
        // — and which move they add up to is core's rule (offers::switch_action): a tag or the detached marker moves
        // nothing, a tag's row offering a branch at its commit instead (`startNaming`).
        const local = kind === "remote" ? remotesModel.localNameFor(name) : name
        const action = GitFacts.switchAction(kind, local, workTree.branch,
                                             worktreesModel.worktreeHolding(local),
                                             branchesModel.oidOfName(local))
        // Before the leave question: the holder road is the one no
        // operation put down can clear (offers::SwitchAction), so asking
        // to undo a rebase first would spend the undo on a move that was
        // never possible — and this road leaves the operation exactly
        // where it is, in the copy it is standing in.
        if (action === "holder") {
            page.openHolder(local)
            return true
        }
        // Ahead of the branches below: the one that lands on an existing local branch asks git what the move would
        // cost before it moves, and that read is worth nothing while an operation is standing.
        if (page.standsInTheWay(leaving)) {
            page.askLeaveOperation(function () { page.switchToRef(kind, name, true) })
            return true
        }
        if (action === "switch")
            page.switchTo("branch", name, name, "", leaving)
        else if (action === "materialize")
            page.switchTo("remote", name, local, "", leaving)
        else if (action === "move") {
            // Whether to ask is git's to answer — a branch that only fell behind loses nothing by moving. The question
            // comes back as `moveAskSeq` when something would be lost, and the operation is left standing for the
            // answer to that one to undo (core asks before it aborts, never the other way round).
            page.moveLanding = local
            repoTab.checkoutMovingBranch(local, name, leaving === true)
        }
        // A tag or the detached marker is the one press left: it moves nothing, and it raised nothing either.
        return action !== ""
    }

    // Landing on the remote branch moves the existing local one onto it — the one move here that can leave commits
    // unreachable.
    function askMoveBranchOnto(local, remoteRef) {
        page.startRowAsk(
            remotesModel.oidOfName(remoteRef),
            qsTr("Move %1 here?").arg(local),
            qsTr("Commits only %1 has stop being reachable.").arg(local),
            false,
            qsTr("Move"),
            function () { page.switchTo("force", local, local, remoteRef) },
            true)
        page.moveBranchAsked(local)
    }
    /// Automation: the question above was raised, about `local`. An automation-only exposure, the same one
    /// `GraphPane.view` is (app-ui.md) — the bar itself carries no name a run could read the branch back off.
    signal moveBranchAsked(string local)

    // ---- the standing question --------------------------------------
    // One bar over the graph (デザイン規約 §可否・警告の出し場所); Escape, another ask or a click anywhere else walks away from it.
    // Only questions about a ref reach it: everything that takes one named thing away is held down on the row or button
    // that names it (デザイン規約 §長押し).
    /// Ctrl+F: the find bar belongs to the graph — which the plan is standing over while one is open, so the press
    /// is quietly refused there, the way a standing question already refuses it (`GraphPane.startFind`).
    function startFind() {
        if (page.planShown)
            return
        graphPane.startFind()
    }

    // ---- the interactive-rebase plan ---------------------------------
    // Composing runs nothing; the run is the one queued write, pinned by the model to the tip the plan opened on.
    // While the plan stands, the page holds down what could move the history under it from inside this window (the
    // sidebar, find, the toolbar's own writes — TopBar reads `planActive`); what it cannot hold — a terminal, another
    // session — the model answers by putting the plan away when the tip moves (`noteHead` below).
    /// A plan is standing: its rows are here and they are a draft somebody can compose.
    ///
    /// **The one answer**, read by everything on this page that a standing plan changes — the freezes, the right
    /// pane's boxes, the run bar, the corner it takes the seat of. Written twice they could disagree, and the run
    /// that holds the opening face (`planLoadHeld`) is exactly the moment a second spelling would.
    readonly property bool planActive: planModel.active && !page.planLoadHeld
    /// The plan's face has the graph's seat: the read that opens it is out, or its rows have arrived.
    ///
    /// **The seat is taken at the press**. The read walks the whole range, so a shallow click is
    /// quick and the root of a real history is far past the moment a press has to be acknowledged in
    /// (ci/baseline/code-costs-windows-x64.md) — and a screen that does not move for that long says the press was
    /// not heard. What the pane can say before the rows land is in the pane (`RebasePlanPane.waiting`).
    ///
    /// **The mode's restrictions are on this too** — the left menu, `push`, `stash` and Ctrl+F all go out from the
    /// press, because entering the mode is what taking the seat *is*. What waits for the rows is only what needs a
    /// draft to mean anything, and that reads `planActive`.
    readonly property bool planShown: planModel.active || planModel.loading
    /// Automation (`PGG_AUTO_ACT=plan-loading`): the opening face, held from the real edge — the rows arrive through
    /// the feed and can land while the asynchronous grab is still out, and the picture would then be of the plan
    /// rather than of the wait for it (verify-ui スキル §中間状態は実 edge を latch する).
    ///
    /// It holds the *whole* face: a run that held only the pane would photograph the run bar standing at the right
    /// pane's foot and the left menu already frozen, under a centre that says the plan has not arrived. That is a
    /// screen this app never puts up.
    property bool planLoadHeld: false
    /// The plan model itself — an automation-only exposure, the same one `GraphPane.view` is (app-ui.md).
    readonly property var rebasePlan: planModel
    /// The rewrite warning's count for the plan's own range — the plan's own answer, counted by the read that opens
    /// it and again when the refs move under it (`RebasePlanModel.pushedCount`); 0 while no plan stands.
    readonly property int planPushed: planModel.pushedCount
    /// Whether the details pane's boxes are, right now, a plan row's reword input: the plan stands, the row the
    /// selection sits on carries the verb, **and the pane is showing that very commit** — anything else that moves
    /// the selection (a shortcut, a landing) leaves the typing where the message on screen is. The model owns which
    /// row it is, so a reorder cannot detach the two.
    readonly property bool planReword: page.planActive && planModel.selectedAction === "reword"
                                       && planModel.selectedOid !== ""
                                       && planModel.selectedOid === detailsModel.shaHex
    /// What `Discard` is about to take away with the plan: the plan's own edits, and a reword standing in the right
    /// pane's boxes that the plan has not been given yet. Both are composed here and nowhere else — the screen after
    /// the press holds no copy of either — so the button is held (デザイン規約 §長押し).
    ///
    /// **A half-written amend from before the plan stood is not in this.** The plan closing leaves that text exactly
    /// where it is (`DetailsPane.dropDraft`), so there is nothing for a hold to guard, and arming one there would
    /// say the press costs something it does not.
    ///
    /// **Read off `boxMoved`.** A row walked back out of `reword` locks the boxes with the typed text still
    /// standing in them, and the reader can neither save it nor be warned by a button that asks whether the boxes
    /// still take typing — but `Discard` takes that text all the same.
    readonly property bool planDiscards: page.planActive
        && (planModel.dirty || (detailsPane.boxFromPlan && detailsPane.boxMoved))
    function startRebasePlan(oidHex) {
        planModel.open(oidHex)
    }
    // What the pane's arrival and departure do, on the edge the *seat* changes: the graph is gone from the press, and
    // a diff opened over it goes with it. Either way out of the read — rows, a refusal, a failure, `Discard` pressed
    // on the empty face — comes back through here, so the fold has no exceptions.
    onPlanShownChanged: {
        if (page.planShown) {
            page.closeDiff()
            // Down to the rail with the rest of the window's other business, and back up when the plan goes —
            // whichever way it goes (`foldForPlan`).
            page.foldForPlan(true)
        } else {
            page.foldForPlan(false)
        }
    }
    onPlanActiveChanged: {
        if (page.planActive) {
            // The graph lands on the plan's own newest row, whatever face was up before — the right pane
            // becomes that commit's, and the WIP face (whose commit button would sit under the run bar, and whose
            // own writes the freeze is for) cannot stay up under an open plan. `activateRow` also puts away a
            // standing row question and a name box, the way any deliberate click does. The plan's *own* selection
            // is already on that row: the model opens it there, because writing it from here would fire the
            // model's one `changed()` inside the very binding delivering it (`RebasePlanModel::take`).
            page.activateRow(planModel.expectHead)
        } else {
            // Discarded, stale, run, or put away under a standing operation — the plan is gone either way, and so is
            // the row that was going to carry whatever a `reword` left in the right pane's boxes. Nothing moved the
            // commit on screen, so the pane's own guard would sit still (`DetailsPane.syncMessage`) and leave that
            // text where the plain amend this hands back can reach it: on the newest commit that is a one-press
            // rewrite of the history the plan never ran, and on any other row it is one commit's message shown under
            // another's. The commit's own message goes back in.
            detailsPane.dropDraft()
        }
    }
    // What the range has already been sent of moves with the remote-tracking refs, and fetch is the one write the
    // freeze leaves running — so the count is asked again whenever the refs actually move (`refsMoved` — the
    // every-tick `refsSettled` would spawn a rev-list at the status rate), and the run button's amber follows the
    // fetch (規約 §フル interactive rebase). The plan asks for itself and reads its own answer by range, so no other
    // question can take the answer's place.
    Connections {
        target: branchesModel
        function onRefsMoved() {
            planModel.refreshPushed()
        }
    }
    Connections {
        target: planModel
        // Armed by the answer that a run actually went out: `runPlan` turns away a plan that asks for nothing, and a
        // hold armed for a write that was never sent would never be let go of.
        function onPlanRan() {
            page.planRunOut = true
        }
        // Three ways a range cannot be replayed. Only the kind travels — the words are this end's, because git was
        // never run (app-ui.md「Rust に文言を置かない」). **The line under the heading is `Words`'**: the row menu
        // turns down the same three histories and says them the same way (`Words.rewriteRefusedWhy`). What stays
        // here is what the two surfaces do not share — a heading about the range, and `warning`, because the
        // plan is a gesture still going
        // (規約 §答えの要らない報せ).
        function onRefusedPlan(kind) {
            page.showNotice(Words.planRefused(kind), Words.rewriteRefusedWhy(kind), "warning")
        }
        function onStalePlan() {
            page.showNotice(qsTr("The branch tip moved"),
                            qsTr("The plan was put away; nothing has run."), "warning")
        }
        function onStandingOp() {
            page.showNotice(qsTr("An operation started here"),
                            qsTr("The plan was put away; nothing has run. Finish what is in progress first."),
                            "warning")
        }
    }
    // The tip and the standing operation as the status reads them, fed on every snapshot: the model compares the tip
    // against the one the plan opened on and puts a stale draft away itself, and an operation starting under the plan
    // puts it away outright — a merge stopped on a conflict leaves the tip where it was, so the tip alone cannot see
    // it. Only while one stands — fed to a shut model they would spend a borrow per tick saying nothing.
    Connections {
        target: workTree
        function onChanged() {
            if (!page.planActive)
                return
            planModel.noteOp(workTree.opText)
            if (page.planActive)
                planModel.noteHead(workTree.headOid)
        }
    }

    property var rowAskRun: null
    function startRowAsk(oidHex, label, detail, danger, acceptText, run,
                         hold = false, tip = "", form = null, code = "", refName = "") {
        // Whoever was dressing the bar lets go first: a question can be raised over one still standing (a ref clicked
        // in the left menu while the first push asks where it goes), and the flow left behind would go on calling
        // itself the one standing — which is what keeps its check timer firing round trips at a remote nobody is
        // asking about. The dress is safe: `startAsking`'s assignments beat a live `Binding` outright
        // (measured). The flow raising this one turns its own back on after this returns.
        publishFlow.publishAsking = false
        upstreamFlow.asking = false
        renameCarryFlow.asking = false
        page.rowAskRun = run
        graphPane.startAsking(oidHex, label, detail, acceptText, danger, hold, tip, form, code, refName)
    }
    function stopRowAsk() {
        page.rowAskRun = null
        publishFlow.publishAsking = false
        upstreamFlow.asking = false
        renameCarryFlow.asking = false
        graphPane.stopAsking()
    }
    function answerRowAsk() {
        const run = page.rowAskRun
        page.stopRowAsk()
        if (run)
            run()
    }

    // On its own counter: the same move can be asked about twice in a row, and only a fresh answer may raise the
    // question.
    property int seenMoveAskSeq: 0
    function absorbMoveAsk() {
        if (repoTab.moveAskSeq === page.seenMoveAskSeq)
            return
        page.seenMoveAskSeq = repoTab.moveAskSeq
        // The move came back as a question instead: nothing moved, so nothing is on its way to the screen and the next
        // press — the one that answers the bar, or the one that raises it again after it is walked away from — goes.
        page.moveLanding = ""
        page.askMoveBranchOnto(repoTab.moveAskLocal, repoTab.moveAskStart)
    }

    // ---- what a branch is measured against --------------------------
    UpstreamFlow {
        id: upstreamFlow
        repoTab: repoTab
        remotesModel: remotesModel
        graphPane: graphPane
        onAskRequested: (oidHex, label, accept, run, form) =>
            page.startRowAsk(oidHex, label, "", false, accept, run, false, "", form, "")
    }
    /// The branch card's row, from either menu it is carried by. Marked on the row that branch stands on, so the
    /// question is beside the thing it is about.
    function startUpstreamAsk(branch, counterpart) {
        upstreamFlow.startAsk(branch, branchesModel.oidOfName(branch), counterpart)
    }

    // ---- what a name changed here does to the remote ----------------
    RenameCarryFlow {
        id: renameCarryFlow
        repoTab: repoTab
        graphPane: graphPane
        onAskRequested: (oidHex, label, accept, run, form) =>
            page.startRowAsk(oidHex, label, "", false, accept, run, false, "", form, "")
        onCarryAsked: (kind, from, to) => page.renameCarryAsked(kind, from, to)
    }
    /// Automation: the question above was raised, and the two names it is between. An automation-only exposure, the
    /// same one `GraphPane.view` is (app-ui.md).
    signal renameCarryAsked(string kind, string from, string to)
    /// A branch took a new name here and the remote it was measured against still carries the old one. The cut is
    /// the configured remote name where one owns the ref (a remote's own name may contain `/`), the first slash
    /// otherwise — either way the question still fires (`GitFacts.remoteOfRef`).
    function carryBranchRenameOver(remoteRef, name) {
        const remote = GitFacts.remoteOfRef(remoteRef, repoTab.remoteNames)
        if (remote === "")
            return
        const from = GitFacts.branchOfRef(remoteRef, repoTab.remoteNames)
        // Nothing worth asking where the remote already carries the name that would be made: both answers that make
        // one are a plain push, which fast-forwards the branch already over there and reports success — so somebody
        // else's branch would move. The box refuses it too; this catches the way in that has no box.
        if (name === from || remotesModel.oidOfName(remote + "/" + name) !== "")
            return
        renameCarryFlow.startAsk("branch", remote, from, name, remotesModel.oidOfName(remoteRef))
    }
    /// Automation: the chooser inside it, which no injected click can open.
    function openCarryChoices() { return renameCarryFlow.openChoices() }
    function pickCarryChoice(index) { return renameCarryFlow.pickChoice(index) }

    // ---- sending the branch to its remote ---------------------------
    PublishFlow {
        id: publishFlow
        repoTab: repoTab
        workTree: workTree
        remotesModel: remotesModel
        graphPane: graphPane
        // The first push asks where the branch goes, and it asks in the one bar every other question stands in.
        onAskRequested: (label, run, form, code, refName) =>
            page.startRowAsk("", label, "", false, "", run, false, "", form, code, refName)
    }
    /// What the window's toolbar reads off the page it is showing: the button lives up there, and the state machine
    /// behind it down here (`TopBar`).
    readonly property alias pushTargetLabel: publishFlow.pushTargetLabel
    readonly property alias pushState: publishFlow.pushState
    readonly property alias canPush: publishFlow.canPush
    readonly property alias canForcePush: publishFlow.canForcePush
    readonly property alias pushFailed: publishFlow.pushFailed
    readonly property alias pushFailReason: publishFlow.pushFailReason
    function pushNow() {
        publishFlow.pushNow()
    }
    function forcePush() {
        publishFlow.forcePush()
    }

    /// One of this page's right-click menus is standing. What the pointer is over then is the menu; the row it was
    /// opened on is behind it, and nothing behind a menu is being hovered — a card or a tip that comes out now is
    /// drawn over the very rows the hand is reading. The menus are all the page's, so this is
    /// the one place that can see all of them — and a menu nobody has raised yet is not standing.
    readonly property bool menuStanding:
        page.menuShowing(refMenuSeat) || page.menuShowing(commitMenuSeat) || page.menuShowing(fileMenuSeat)
        || page.menuShowing(remoteMenuSeat) || page.menuShowing(diffMenuSeat)
    function menuShowing(seat) {
        return seat.item !== null && seat.item.showing
    }

    /// Every seat on this page that is built on being asked for — the five menus — stands built from the start
    /// instead. Written from outside: the harness asks for this before its verbs read a menu's rows, since a null
    /// there is nothing to read rather than a refusal (`PageHarness`), and false wherever nobody wrote it, which is
    /// every page a person opens. The plan's face and the command log are not in this: their verbs raise them the
    /// way a hand does, and read them only once they are up.
    property bool keepBuilt: false

    // ---- context menu on a sidebar row ------------------------------
    // Each menu is built the first time it is raised: five cards of rows, held ready, are working set a page pays
    // for whether or not a hand ever comes (rules-refs/app-ui.md — the same rule the window's two dialogs follow,
    // `WindowDialogSeat`). The door activates the seat and then calls into it, which is synchronous; the seat fills
    // the page so the menu inside measures the window the way it always did (`AppMenu.ownerItem`).
    Loader {
        id: refMenuSeat
        anchors.fill: parent
        active: page.keepBuilt
        sourceComponent: RefRowMenu {
            // Every row of this menu moves something — a switch, an integrate, a name taken away — so the whole card
            // is held while the doors are, wherever it was raised from. **The chip on a graph row is the same door**:
            // one menu answers for both entrances, and a `switch` that stayed live over there would be the very move
            // the left pane just closed (デザイン規約 §メニュー: 入口が違っても同じ操作は同じ文).
            heldReason: page.doorsHeldWhy
            repoTab: repoTab
            workTree: workTree
            graphModel: graphModel
            branchesModel: branchesModel
            worktreesModel: worktreesModel
            tagsModel: tagsModel
            onSwitchRequested: (kind, name) => page.switchToRef(kind, name)
            onBranchHereRequested: oidHex => page.startBranchAt(oidHex)
            onTagHereRequested: oidHex => page.startTagAt(oidHex)
            onDeleteRequested: (kind, id, name, oidHex) => page.deleteRow(kind, id, name, oidHex)
            onDropStashRequested: selector => page.dropStashNow(selector)
            onUpstreamRequested: (branch, counterpart) => page.startUpstreamAsk(branch, counterpart)
            // The settle re-run is for a menu that stood on the stacked list's row: the list stayed up under it, and
            // whether it stays now is the pointer's to answer again.
            onDismissed: rowHost.settleRefList()
        }
    }
    // What a remote itself offers. Its own menu: a remote is repository configuration, and the ref menu is about
    // refs (デザイン規約 §左メニューの所作).
    Loader {
        id: remoteMenuSeat
        anchors.fill: parent
        active: page.keepBuilt
        sourceComponent: RemoteRowMenu {
            heldReason: page.doorsHeldWhy
            repoTab: repoTab
            onUrlRequested: name => publishFlow.startEditRemote(name)
            onDismissed: rowHost.settleRefList()
        }
    }
    /// The one door into that menu. Says whether it opened.
    function openRemoteMenu(name) {
        remoteMenuSeat.active = true
        return remoteMenuSeat.item.offerOn(name)
    }

    /// Which surface raised the standing ref menu. Its branch row opens a box to type a name in, and that box belongs
    /// on the row the hand is already on — the same reason the box is in the chip column and not over the window
    /// (デザイン規約 §可否・警告の出し場所).
    property bool refMenuInSidebar: false
    /// The one door into that menu: the sidebar's rows, a chip, the stacked list and the automation all come through
    /// here. Says whether it opened.
    function openRefMenu(kind, name, full, oidHex, inSidebar) {
        page.refMenuInSidebar = inSidebar === true
        refMenuSeat.active = true
        return refMenuSeat.item.offerOn(kind, name, full, oidHex)
    }
    /// A new branch on a commit, asked for from a menu: the name box opens where that menu was raised. Nothing is
    /// created until it is submitted — walking away costs the typing and nothing else. The sidebar's half reads the
    /// row off the ref menu, which is standing whenever that half is taken: only its door sets `refMenuInSidebar`.
    function startBranchAt(oidHex) {
        if (oidHex === "")
            return
        if (page.refMenuInSidebar)
            sidebarPane.beginBranchAt(refMenuSeat.item.kind, refMenuSeat.item.refId, oidHex)
        else
            graphPane.startNaming(oidHex)
    }
    /// And a tag on that commit, through the same two doors and on the same terms.
    function startTagAt(oidHex) {
        if (oidHex === "")
            return
        if (page.refMenuInSidebar)
            sidebarPane.beginTagAt(refMenuSeat.item.kind, refMenuSeat.item.refId, oidHex)
        else
            graphPane.startTagging(oidHex)
    }

    // ---- what the sidebar's rows ask for ---------------------------
    /// Unconfirmed: a name is not history. A tag and a stash are re-made under the new name by core — the only rename
    /// git has for them.
    function renameRow(kind, id, name) {
        if (kind === "branch") {
            // Which remote this branch speaks for has to be read before the rename: afterwards the row answers to the
            // new name.
            page.pendingRenameRemote = branchesModel.upstreamOf(id)
            page.pendingRenameTo = name
            repoTab.renameBranch(id, name, false)
        } else if (kind === "tag") {
            // And which remote carries this name, read before the rename for the same reason: afterwards the row
            // answers to the new one (デザイン規約 §手元の改名の後のリモート).
            page.armRenameTagRemote(id, name)
            repoTab.renameTag(id, name)
        } else if (kind === "stash") {
            repoTab.renameStash(id, name)
        } else if (kind === "remote") {
            // A remote branch's box does not rename: what it leads to is the replace over there. **The question goes
            // out first**, carrying both names from now on (`askReplaceRemote`). So the box that sent it has nothing
            // to wait for and comes down, where every rename keeps it until git answers (デザイン規約 §答えの要らない報せ).
            // Left waiting, a dismissed question would leave it standing over the sidebar with nothing coming.
            page.noteRenameLanded()
            page.askReplaceRemote(id, name)
        }
    }

    /// The remote branch a just-renamed local one spoke for, and the name it took — the question about what the remote
    /// does with it waits until git says the local rename landed.
    property string pendingRenameRemote: ""
    property string pendingRenameTo: ""

    /// The same three for a tag: the remote that carries the name too, and the two names the rename is between. A tag
    /// has no upstream to read them off, so they are the tags section's answers about the name itself
    /// (`NavSectionModel.tagSides`) — and the section stops answering for the old name the moment the rename lands,
    /// which is why they are taken before it.
    property string pendingRenameTagRemote: ""
    property string pendingRenameTagFrom: ""
    property string pendingRenameTagTo: ""
    /// And the commit the bar marks. **Taken with them, before the write.** A rename moves the name and never the
    /// object ([`tag::rename`]), so the old name's commit is the new name's — while asking the model for the new name
    /// at the moment the answer lands is a race the bar loses: the rows are rebuilt by the read behind the write, and
    /// a question raised before that read arrives would stand over no row at all.
    property string pendingRenameTagOid: ""
    /// Reads them, and **only where the pair over there would change the name and nothing else** (デザイン規約
    /// §手元の改名の後のリモート): the remote's copy has to stand where this one does, since the push and the delete
    /// the answer runs would otherwise move the mark as well as the name; and the new name has to be free over
    /// there, since a push to one it already carries is refused outright. Nothing is armed where the answer would be
    /// a question nobody can say yes to.
    function armRenameTagRemote(from, to) {
        page.forgetRenameTagRemote()
        const remote = repoTab.defaultRemote
        if (remote === "" || from === to)
            return
        if (tagsModel.tagSides(from) !== "both" || tagsModel.remoteTagDrift(from, remote) !== "")
            return
        // A name standing anywhere already: held here as well, git's own rename refuses it first and this never
        // lands; held only over there, the push would be the refusal.
        if (tagsModel.tagSides(to) !== "")
            return
        page.pendingRenameTagRemote = remote
        page.pendingRenameTagFrom = from
        page.pendingRenameTagTo = to
        page.pendingRenameTagOid = tagsModel.oidOfName(from)
    }
    function forgetRenameTagRemote() {
        page.pendingRenameTagRemote = ""
        page.pendingRenameTagFrom = ""
        page.pendingRenameTagTo = ""
        page.pendingRenameTagOid = ""
    }

    /// A rename's box stays open until git answers, and these are the two words back to it. **Both boxes are asked**
    /// and only the one that was waiting acts: a rename comes from the left menu's row or from the chip on the graph,
    /// and neither knows about the other (デザイン規約 §答えの要らない報せ).
    function noteRenameLanded() {
        sidebarPane.noteRenameLanded()
        graphPane.renameLanded()
    }
    function noteRenameRefused(why) {
        sidebarPane.noteRenameRefused(why)
        graphPane.renameRefused(why)
    }

    /// Replacing a branch on a remote with one under a new name, which git has no command for: core pushes the new
    /// name and deletes the old, so the question is asked first and its answer is held down — this is the one write
    /// here that another machine keeps (デザイン規約 §長押し).
    function askReplaceRemote(remoteRef, name) {
        // The cut is the configured remote name where one owns the ref (a remote's own name may contain `/`), the
        // first slash otherwise — either way the question still fires (`GitFacts.remoteOfRef`).
        const remote = GitFacts.remoteOfRef(remoteRef, repoTab.remoteNames)
        if (remote === "")
            return
        const from = GitFacts.branchOfRef(remoteRef, repoTab.remoteNames)
        // The question stops at a name already over there: a plain push to one that exists fast-forwards it and
        // reports success, so somebody else's branch would move. The box refuses it too; this catches the way in
        // that has no box.
        if (name === from || remotesModel.oidOfName(remote + "/" + name) !== "")
            return
        page.startRowAsk(
            remotesModel.oidOfName(remoteRef),
            //: %1 and %2 are remote branches, e.g. origin/main. The old name goes and the new one is made.
            qsTr("Replace %1 with %2?").arg(remoteRef).arg(remote + "/" + name),
            qsTr("The old branch is deleted, not moved."),
            false,
            qsTr("Replace"),
            function () { repoTab.replaceRemoteBranch(remote, from, name) },
            true,
            qsTr("Hold to replace. %1 goes up first, then %2 comes off — so a push the far side turns down leaves the old name where it is. Anything it carried — an open pull request, a running check — does not follow the new name.").arg(remote + "/" + name).arg(remoteRef))
        page.replaceRemoteAsked(remoteRef, name)
    }
    /// Automation: the question above was raised, and the two names it is between. An automation-only exposure, the
    /// same one `GraphPane.view` is (app-ui.md).
    signal replaceRemoteAsked(string from, string to)

    /// The failures a report has already answered — armed by the report when the row it is about has not reached the
    /// log yet (`absorbWriteResult`), spent by that row's own arrival. Counted, because the refusal and the write
    /// result arrive on separate paths, in no fixed order.
    property int answeredFailures: 0
    /// The answer to the plain delete a card stayed up for — **its own answer**, out of the ones this notify
    /// carried (`RepoTab.branchDeleteAnswer`, -1 where it carried none). The name it was asked with is what the card
    /// acts on and that stands until the next delete is asked; this is how the page tells the answer in hand from
    /// the one standing above it, which no count of answers could.
    function absorbBranchDelete() {
        const answer = repoTab.branchDeleteAnswer
        if (answer < 0)
            return
        // Taken, and the card goes by itself off the name (`RefBranchMenu`); turned down by git on its own, and the
        // same card turns its row into the held `-D`. **The row is the answer** — a refusal does not mean the
        // commits stop being reachable (git measures the branch against its upstream when it has one, so a branch
        // merged into HEAD but not yet pushed is refused while nothing at all would be lost — measured), so there is
        // nothing here to raise over it.
        if (!repoTab.writeAnswerFailed(answer))
            return
        // What is left is a refusal somebody outside made — the far side keeping the branch. The row cannot answer
        // that and the name it asked with is not what was turned down, so the bar carries it instead.
        page.reportRefusal(answer)
    }

    // ---- what the window is already showing as gone ------------------
    //
    // **A delete takes the row away at the press, and git is asked behind it** (デザイン規約 §消す操作は先に画面から消す).
    // The write itself is the short half: `git branch -d` is one process, and everything the reader is actually
    // waiting on comes after it — the refs read, then the walk that rebuilds the graph, which on a real history is
    // the long one and which `busyCount` does not cover (ci/baseline/code-costs-windows-x64.md). Left to those, the
    // row sits there through all of it with nothing to say whether the press even landed.
    //
    // **Which rows those are, and how long they stay, is not decided here.** The press, the wait and the put-down are
    // one operation and belong to one owner (`ops::StandIn`, held per tab by the hub so it outlives this page); the
    // four names below are the picture it leaves, and all this page does with them is hand each one to the list that
    // draws it. The owner answers by the id the queue accepted the write under, so nothing here counts writes.
    //
    // One key per kind, because a delete touches at most one of each; `Delete both` is the one that touches two.
    // Empty means nothing is being shown as gone, which is also what a refusal goes back to.
    readonly property string goneBranch: repoTab.goneBranch
    readonly property string goneRemote: repoTab.goneRemote
    readonly property string goneTag: repoTab.goneTag
    readonly property string goneStash: repoTab.goneStash
    /// The chips that go with the rows are the graph model's to key and pack (`GraphModel.setGone` /
    /// `encode::gone_keys`): the names cross the bridge as they are. A dropped stash has no chip: it is a row of the
    /// graph rather than a name on one, and a row only leaves with the walk.
    function syncGone() {
        graphModel.setGone(page.goneBranch, page.goneRemote, page.goneTag)
    }
    onGoneBranchChanged: {
        branchesModel.setHidden(page.goneBranch)
        page.syncGone()
    }
    onGoneRemoteChanged: {
        remotesModel.setHidden(page.goneRemote)
        page.syncGone()
    }
    onGoneTagChanged: {
        tagsModel.setHidden(page.goneTag)
        page.syncGone()
    }
    onGoneStashChanged: stashesModel.setHidden(page.goneStash)

    /// Automation: the run that photographs a row already gone holds it there.
    ///
    /// What it photographs is the in-between — the row taken away, git not yet answered for it — and a demo
    /// repository answers in tens of milliseconds, which is over before the picture is grabbed. Asked for before the
    /// press, so what holds the row up is a standing decision
    /// (verify-ui スキル §壊れない動詞の実装と反復).
    ///
    /// **Held by withholding the news**: the two readings below are what the owner puts the rows down on, so a run
    /// that wants the in-between simply does not tell it they arrived.
    property bool holdGoneRows: false

    /// Whether the delete this page was last asked for went out to git, or was dropped before it did. **Both
    /// entrances end here** — the left row's card and the graph row's — and a signal carries no answer back to
    /// either, so the answer stands here. The automation reads it as "did the input land"
    /// (app-ui.md §UI 自動化の因果性), the same question `RefBranchMenu.deleteAsked` answers for the card: a request
    /// this page dropped leaves nothing for a write barrier to wait on, and waiting on it is the watchdog's whole
    /// ceiling in silence. Nothing on screen needs it.
    property bool deleteRowAsked: false
    function deleteRow(kind, id, name, oidHex) {
        page.deleteRowAsked = false
        if (kind !== "branch")
            return
        // The one row in any menu that outlives its own write (it stays open for git's answer), so it is also the one
        // that can be clicked twice — the second click is the same request again.
        if (repoTab.busyCount > 0)
            return
        // A branch is deleted with `-d`, and git's refusal is the question — asked when it arrives
        // (デザイン規約 §左メニューの所作). Which branch the answer is about is the tab's to keep, and the card that asked
        // reads it there (`RefBranchMenu`); the row leaves the screen at the press, which is the same slot's doing
        // (`ops_delete`).
        repoTab.deleteBranch(id, false)
        page.deleteRowAsked = true
    }
    /// Held (デザイン規約 §長押し).
    function dropStashNow(ref) {
        repoTab.dropStash(ref)
        if (page.selectedStashRef === ref)
            page.selectedStashRef = ""
    }

    // ---- context menu on a working-tree file row --------------------
    function openFileMenu(bucket, path) {
        // A right-click is a click: it walks away from a question that was standing, which may well be about another
        // row.
        page.stopRowAsk()
        fileMenuSeat.active = true
        fileMenuSeat.item.offer(bucket, path)
    }
    Loader {
        id: fileMenuSeat
        anchors.fill: parent
        active: page.keepBuilt
        sourceComponent: FileRowMenu {
            repoTab: repoTab
            workTree: workTree
            wipPane: wipPane
            onMergeToolWanted: page.gitSettingsRequested()
            onCopyRequested: text => clipboard.copy(text)
        }
    }

    // ---- context menu on the diff's own text ------------------------
    /// The one door into that menu. What it will act on is the selection, and the hand settled that before asking
    /// (`DiffTextSelect.askMenu`), so nothing about the row travels here.
    function openCodeMenu() {
        page.stopRowAsk()
        diffMenuSeat.active = true
        diffMenuSeat.item.offer()
    }
    Loader {
        id: diffMenuSeat
        anchors.fill: parent
        active: page.keepBuilt
        sourceComponent: DiffRowMenu {
            diffModel: diffModel
            onCopyRequested: text => clipboard.copy(text)
        }
    }

    // ---- context menu on a graph row -------------------------------
    /// The chip a graph row draws — its first one, whatever kind it is, which is what the row hands over when a hand
    /// presses it (`GraphRowDelegate.renameChip`); null where it draws no ref. Asked of the model for the callers
    /// that have no row in hand. The chips already taken off the screen ahead of git's answer are filtered out the
    /// same way the delegate filters them, so a name that is gone does not put a card up over a ref that is not
    /// there any more (デザイン規約 §消す操作は先に画面から消す).
    function rowChipAt(oidHex) {
        const row = graphModel.rowOf(oidHex)
        if (row < 0)
            return null
        const shown = GitFacts.chipsShown(graphModel.labelsAt(row), graphModel.goneChips)
        if (shown.length === 0)
            return null
        return GitFacts.refKind(shown[0].kind) === "" ? null : shown[0]
    }

    /// The one door into that menu: the graph's rows wherever they are pressed, the rows of the stacked list a chip
    /// unfolds into, and the automation all come through here. `record` is the name the menu is aimed at — the one
    /// the chip draws, or the one pressed in the list — and empty aims it at nothing, which is a row that draws no
    /// name at all. **The first level stands still**: it is the same commit either way, so what a naming in
    /// the list changes is which card comes up (デザイン規約 §グラフ行の右クリック). Left out, the row's own name is
    /// asked of the model, which is what the callers with no row in hand do.
    function openRowMenu(oidHex, chip) {
        const named = chip === undefined ? page.rowChipAt(oidHex) : chip
        commitMenuSeat.active = true
        const menu = commitMenuSeat.item
        menu.targetKind = named ? GitFacts.refKind(named.kind) : ""
        menu.targetName = menu.targetKind === "" ? "" : named.name
        commitMenuState.openRowMenu(oidHex)
    }

    CommitMenuState {
        id: commitMenuState
        repoTab: repoTab
        workTree: workTree
        graphModel: graphModel
        worktreesModel: worktreesModel
        branchesModel: branchesModel
        tagsModel: tagsModel
        // Null until the menu is first raised; the one function that reads it is the door that raises it.
        menu: commitMenuSeat.item
    }

    Loader {
        id: commitMenuSeat
        anchors.fill: parent
        active: page.keepBuilt
        sourceComponent: CommitRowMenu {
            // The graph's rows are doors onto the same history the left pane's are, so they are held on the same
            // answer: a reset or a drop let go into the middle of a replay is the same accident a switch would be.
            heldReason: page.doorsHeldWhy
            branch: workTree.branch
            oid: commitMenuState.menuOid
            stashRef: commitMenuState.menuStashRef
            published: commitMenuState.menuPublished
            canSwitch: commitMenuState.menuCanSwitch
            switchAsks: commitMenuState.menuSwitchAsks
            heldLeaf: commitMenuState.menuHeldLeaf
            canPull: commitMenuState.menuCanPull
            pullBlocked: commitMenuState.menuPullBlocked
            canSequence: commitMenuState.menuCanSequence
            canIntegrate: commitMenuState.menuCanIntegrate
            canEditHistory: commitMenuState.menuCanEditHistory
            canMoveBranch: commitMenuState.menuCanMoveBranch
            canBranchHere: commitMenuState.menuCanBranchHere
            stashCanWrite: commitMenuState.menuStashCanWrite
            hardResetTakes: commitMenuState.menuHardResetTakes
            tipHeldElsewhere: commitMenuState.menuTipHeldElsewhere
            // git's answers to the branch card's delete, live while the card stands (`RefBranchMenu`).
            refusedDelete: repoTab.branchDeleteRefused
            landedDelete: repoTab.branchDeleteLanded
            checkedBranch: repoTab.branchDeleteAsked
            checkedMerged: repoTab.branchDeleteMerged
            // The same road the row's double-click takes, held on the same answers (`switchToRef`).
            onSwitchRequested: (kind, name) => page.switchToRef(kind, name)
            // Straight to the graph row: this menu is only ever raised on one.
            onBranchHereRequested: oidHex => graphPane.startNaming(oidHex)
            onTagHereRequested: oidHex => graphPane.startTagging(oidHex)
            onSquashRequested: oidHex => page.squashCommit(oidHex)
            onDropRequested: oidHex => page.dropCommit(oidHex)
            onPlanRequested: oidHex => page.startRebasePlan(oidHex)
            onResetRequested: mode => page.moveBranchHere(mode)
            onApplyStashRequested: selector => repoTab.applyStash(selector)
            onPopStashRequested: selector => page.popStash(selector)
            onDropStashRequested: selector => page.dropStashNow(selector)
            // The branch card's own three, answered exactly where the ref menu's are.
            onDeleteRequested: (kind, id, name, oidHex) => page.deleteRow(kind, id, name, oidHex)
            onUpstreamRequested: (branch, counterpart) => page.startUpstreamAsk(branch, counterpart)
            // The writes its rows run. The tab is here, and the two cuts of a remote ref are kept beside the lookups
            // the same cards' answers came from (`CommitMenuState`).
            onCherryPickRequested: oidHex => repoTab.cherryPick(oidHex)
            onRevertRequested: oidHex => repoTab.revert(oidHex)
            onMergeRequested: ref => repoTab.merge(ref, false, false, "")
            onRebaseRequested: ref => repoTab.rebase(ref, "", true)
            onPullRequested: repoTab.pull()
            onCheckDeleteRequested: branch => repoTab.checkBranchDelete(branch)
            onForceDeleteRequested: branch => repoTab.deleteBranch(branch, true)
            onDeleteRemoteRequested: remoteRef => commitMenuState.deleteRemoteNow(remoteRef)
            onDeleteEverywhereRequested: (branch, remoteRef, forced) =>
                commitMenuState.deleteEverywhereNow(branch, remoteRef, forced)
            onPushTagRequested: (remote, tag, lease) => repoTab.pushTag(remote, tag, lease)
            onDeleteTagRequested: tag => repoTab.deleteTag(tag)
            onDeleteRemoteTagRequested: (remote, tag, onlyThere) =>
                repoTab.deleteRemoteTag(remote, tag, onlyThere)
            onDeleteTagEverywhereRequested: (tag, remote) => repoTab.deleteTagEverywhere(tag, remote)
            // The settle re-run is for a menu that stood on the stacked list's row: the list stayed up under it, and
            // whether it stays now is the pointer's to answer again.
            onDismissed: rowHost.settleRefList()
        }
    }

    ClipboardHelper {
        id: clipboard
    }

    // ---- the second click, spaced, on a graph row -------------------
    /// The name on the row's chip goes into a box where the chip is (デザイン規約 §グラフ行のダブルクリック / §左メニューの所作 — the
    /// gesture and the box are the sidebar's, and this is the other place a ref name is on screen). **One way in for
    /// both halves of the row**: the click that lands on the row itself, and the one that lands on the card the chip
    /// unfolds into, which is standing on that same chip.
    ///
    /// What is typed is the ref's own name — a remote branch without the remote it is on, the shape `renameRow` and
    /// the sidebar's box both take it in.
    function startRename(oidHex, chip) {
        if (repoTab.state !== "open" || !chip)
            return
        const kind = GitFacts.refKind(chip.kind)
        // The detached-HEAD marker names no ref, so there is nothing here to be renamed.
        if (kind === "")
            return
        const id = chip.name
        // The card the chip unfolds into is standing on the column the box opens in.
        rowHost.closeRefList()
        graphPane.startRenaming(oidHex, kind, id,
                                kind === "remote" ? GitFacts.branchOfRef(id, repoTab.remoteNames) : id)
        // **A box always opens where it can be seen** (デザイン規約 §左メニューの所作「開く行は必ず見える所へ送る」): the wait this
        // gesture opens is long enough to scroll away in, and a name changing itself off screen is a name nobody
        // agreed to. A row the walk no longer has is simply not sent anywhere.
        const row = graphModel.rowOf(oidHex)
        if (row >= 0)
            graphPane.showRowSoon(row)
    }
    /// Whether what is in the graph's name box can be accepted, and the one line that says why not. The rules are
    /// git's own, asked of core, and the models that answer "is that name taken" are here — the pane only draws it.
    /// **Only a rename is refused**: a box that opened empty to make a name has not been answered yet, and a frame
    /// that comes up already turned down is turning down the reader's arrival (§可否・警告の出し場所, `SidebarRowGestures`).
    readonly property string graphRenameRemote: graphPane.namingKind !== "remote" ? ""
        : GitFacts.remoteOfRef(graphPane.namingId, repoTab.remoteNames)
    readonly property bool graphNameTaken: graphPane.namingMode === "rename" && page.graphRenameRemote !== ""
        && graphPane.namingText.trim() !== ""
        && remotesModel.oidOfName(page.graphRenameRemote + "/" + graphPane.namingText.trim()) !== ""
    readonly property string graphNameRefusedWhy: {
        if (graphPane.namingOid === "" || graphPane.namingMode !== "rename")
            return ""
        // git was asked about this very name and answered; everything below is what this end works out before asking.
        if (graphPane.namingGitRefusal !== "")
            return graphPane.namingGitRefusal
        const typed = graphPane.namingText
        if (typed.trim() === "")
            return qsTr("A name is needed")
        // The same rule the left menu's box asks (`SidebarRowGestures.editCaseOnly`): a tag renamed to its own name in
        // other letters takes both names with it on a case-insensitive disk.
        if (graphPane.namingKind === "tag" && typed.trim() !== graphPane.namingId
            && typed.trim().toLowerCase() === graphPane.namingId.toLowerCase())
            return qsTr("Only the letter case differs — on this disk that deletes both names")
        if (page.graphNameTaken)
            return qsTr("%1 already has a branch called that").arg(page.graphRenameRemote)
        return GitFacts.validRefName(typed) ? "" : qsTr("git will not take this as a name")
    }

    // ---- double-click on a graph row -------------------------------
    // A row with no chip is offered one.
    function rowDoubleClicked(oidHex, chip) {
        if (repoTab.state !== "open")
            return
        if (chip) {
            page.activateChip(chip)
            return
        }
        // The other half of the same door, and held on the same answer as the switch above it (`switchToRef`): the box
        // is a branch about to be written, and the run of a plan is out before the count has anything to say.
        if (repoTab.busyCount === 0 && page.doorsHeldWhy === "")
            graphPane.startNaming(oidHex)
    }

    // What a resting pointer opens on a graph row: the row's own card, and the refs one chip had to stack. Owned here
    // because rows are recycled out from under both of them.
    RowHoverHost {
        id: rowHost
        graphPane: graphPane
        currentBranch: workTree.branch
        branchesModel: branchesModel
        remotesModel: remotesModel
        menuStanding: commitMenuSeat.item !== null && commitMenuSeat.item.opened
        hoverBlocked: page.menuStanding
        onRecordActivated: chip => page.activateChip(chip)
        // A plain click in the card is a click on the row it is standing on: every name in it is on that one commit
        // (a held one never comes this way — the card hands it to the graph's row itself).
        onRecordChosen: (oidHex, atRow) => page.activateRow(oidHex, atRow)
        // The note under a message the card had to cut: the row's own click, made from inside the card, and then the
        // pane that holds the whole of it says so — the reader was sent somewhere and the sentence they read is gone
        // by the time they arrive (デザイン規約 §hover のツールチップ).
        onMessageAsked: (oidHex, atRow) => {
            page.activateRow(oidHex, atRow)
            detailsPane.callAttention()
        }
        // A right-click on one of the card's rows raises the row's own menu, aimed at the name that was pressed.
        onRecordMenuAsked: (oidHex, chip) => page.openRowMenu(oidHex, chip)
    }

    // ---- rewriting one commit --------------------------------------
    // Run straight, even for a commit a remote already has: nothing here leaves the machine, and the push that would
    // spread it is asked about on its own.
    function squashCommit(oidHex) {
        repoTab.squashIntoParent(oidHex)
    }

    /// Leaves the commit out of the history. One place for both ways in: the row is a hold or a click depending on
    /// whether anything else still holds the branch tip, and what it runs is the same whichever of the two the
    /// reader got (デザイン規約 §履歴を合流させる).
    function dropCommit(oidHex) {
        repoTab.dropCommit(oidHex)
    }

    // ---- taking the branch back to an earlier commit ----------------
    function moveBranchHere(mode) {
        repoTab.resetTo(commitMenuState.menuOid, mode)
    }

    // ---- editing the selected commit's message ---------------------
    // Run straight, even for a commit a remote already has: this rewrites nothing that a switch or a reset cannot
    // bring back, and the push that would spread it is asked about on its own.
    function saveMessage(oidHex, subject, body) {
        // Where the row sits now. A reword leaves the shape of the history alone, so the rewritten commit lands on the
        // same row and the selection can follow it there.
        page.rewordRow = graphModel.rowOf(oidHex)
        repoTab.rewordCommit(oidHex, subject, body)
    }
    // Row to re-select once the rewritten graph arrives (-1 = none).
    property int rewordRow: -1

    // What the details pane's boxes do with the commit on screen, in core's own word (`offers::message_edit`):
    // `amend` where typing lands, `stash` / `not-head` / `standing` where it does not, `""` where there is no message.
    // **Only HEAD's own commit takes it** — everything else would be replayed, and this is a text box a click can land
    // a caret in (デザイン規約 §コミットメッセージの 2 つの枠). A binding: the boxes stand open while the repository moves
    // under them, so a commit that stops being HEAD's stops taking typing.
    readonly property string messageEdit: GitFacts.messageEdit(
        !page.blank && repoTab.state === "open", detailsModel.shaHex, workTree.headOid,
        page.selectedStashRef, workTree.opText, workTree.opEditing)

    // What git makes of the selected commit's signature. Asked on every selection, and read only when the answer names
    // the commit now on screen — verifying runs gpg or ssh-keygen, so the answer arrives well after the details do.
    // A signature only changes when the commit does, and a changed commit is a different hash, so nothing has to ask
    // twice.
    function askSignature(oidHex) {
        if (oidHex !== "" && repoTab.state === "open")
            repoTab.checkSignature(oidHex)
    }
    readonly property bool signatureIsForSelection:
        detailsModel.shaHex !== "" && repoTab.signatureOid === detailsModel.shaHex
    readonly property string selectedSignatureKind:
        page.signatureIsForSelection ? repoTab.signatureKind : ""
    readonly property string selectedSignatureCode:
        page.signatureIsForSelection ? repoTab.signatureCode : ""
    readonly property string selectedSignatureSigner:
        page.signatureIsForSelection ? repoTab.signatureSigner : ""

    // Whether a remote already has the selected commit — what the save row's warning rests on. Only HEAD's own commit
    // takes typing (`messageEdit`), so the answer is HEAD's own (`WorkTreeModel.headPublished`), read while the
    // selection stands on it; under a plan the reword chip carries the plan's own warning instead. Nothing is asked
    // on the keystroke: the answer rides beside HEAD, and a plan standing over the boxes cannot have spent it.
    readonly property bool selectedPublished: !page.planActive && page.selectedOid !== ""
                                              && page.selectedOid === workTree.headOid && workTree.headPublished

    // **Moving off a half-written message drops it, and nothing asks** (デザイン規約 §コミットメッセージの 2 つの枠).
    // A message that has not been saved is a draft of somebody else's commit; the way to keep it is to press the
    // button, and the two ways to drop it — Escape, and reading another commit — are both things the reader did on
    // purpose. So nothing stands between a click in the graph and the commit it lands on (`DetailsPane.syncMessage`).

    // An assignment is not something git knows about, so nothing here is waiting for a refresh to bring it: the rows
    // and the card re-read the store themselves.
    Connections {
        target: AppBackend
        function onAvatarsChanged() {
            graphModel.refreshAvatars()
            detailsModel.refreshAvatar()
            // The commit editor wears the identity's own face, which is reached by the same badge.
            repoTab.refreshAvatar()
            // And the list of what is held, whose faces are packed off the rows.
            page.refreshChosenRecords()
        }
    }

    // ---- the harness -----------------------------------------------
    // The whole of this tab's verification harness, which a shipped build does not carry (`HarnessSeat`). Everything
    // the verbs and the measurements act on is handed over here — a module of its own cannot see this one's ids — and
    // naming them is an automation-only exposure, the same one `GraphPane.view` is (app-ui.md). The menus and the
    // plan's face go over as the seats they are built in: none exists until it is asked for, and asking is the
    // harness's to do (`keepBuilt`).
    HarnessSeat {
        id: harness
        anchors.fill: parent
        part: "PageHarness.qml"
        wanted: AppBackend.harnessPresent
        seats: ({
            page: page,
            repoTab: repoTab,
            workTree: workTree,
            graphModel: graphModel,
            detailsModel: detailsModel,
            diffModel: diffModel,
            branchesModel: branchesModel,
            remotesModel: remotesModel,
            worktreeModel: worktreeModel,
            stashesModel: stashesModel,
            tagsModel: tagsModel,
            graphPane: graphPane,
            sidebarPane: sidebarPane,
            detailsPane: detailsPane,
            diffPane: diffPane,
            wipPane: wipPane,
            carriedPane: carriedPane,
            planSeat: planSeat,
            gitCorner: gitCorner,
            refMenuSeat: refMenuSeat,
            fileMenuSeat: fileMenuSeat,
            diffMenuSeat: diffMenuSeat,
            commitMenuSeat: commitMenuSeat,
            remoteMenuSeat: remoteMenuSeat,
            rowHost: rowHost,
            clipboard: clipboard,
            commitMenuState: commitMenuState,
            publishFlow: publishFlow,
            upstreamFlow: upstreamFlow
        })
    }

    // ---- a report with nothing to answer ----------------------------
    // A write that did not happen, and whoever said no. It comes down over the middle of the page — over whichever of
    // the graph and the diff is showing — and goes back up on the one word it offers (デザイン規約 §答えの要らない報せ).

    /// Raises the report for one write that did not happen — **the one door**, so a headless run that hands it a kind
    /// enters the same body the answer does (verify-ui). `reason` is what whoever said no wrote; empty where this end
    /// refused the write itself, and then the second line is ours (`Words.writeReportedWhy`).
    function showReport(kind, remote, name, reason) {
        page.showNotice(Words.writeReported(kind, remote, name),
                        reason !== "" ? reason : Words.writeReportedWhy(kind),
                        Words.reportTone(kind))
    }
    /// Automation: git's answer to a write has been taken all the way — the bar raised, the mark taken down, the
    /// standing questions cleared. An automation-only exposure, the same one `GraphPane.view` is (app-ui.md).
    ///
    /// **What it was about rides the signal.** A report is the answer's own and a drain can bring several
    /// (`RepoTab.writeAnswerReportKind`), so the tab holds no one report for a listener to read back afterwards.
    signal writeReported(string kind, string remote, string name)
    /// Raises the report. `label` is what did not happen, `detail` whoever said no in their own words, `tone` the
    /// state it is in if it is in one at all.
    function showNotice(label, detail, tone) {
        // Dressed, then raised — so nothing on a bar the reader can see is ever written (`NoticeBar.open`).
        noticeBar.label = label
        noticeBar.detail = detail
        noticeBar.tone = tone
        noticeBar.open = true
    }
    /// Only lowered: the words stay where they are for the 200ms it spends going up.
    function hideNotice() {
        noticeBar.open = false
    }
    /// The bar itself — automation-only exposure, like `GraphPane.askCard` (app-ui.md). A headless run reads
    /// `settled` / `shut` / `label` off it and presses its one control through `dismiss()`.
    readonly property alias noticeCard: noticeBar
    /// Automation: whether the middle of the page is standing **under** the report — the whole of what moving the bar
    /// out of the graph was for, and the one thing about it a picture cannot settle (a bar drawn over a pane frames
    /// exactly like a bar the pane was moved down for, since what it covers is the pane's own top edge either way).
    /// Read in page coordinates, so it holds whichever of the two panes is showing.
    readonly property bool noticeClears:
        centreStack.mapToItem(page, 0, 0).y >= noticeBar.mapToItem(page, 0, noticeBar.height).y

    // A finished write the editor asked for: clear it only once git says the commit landed, so a rejected one keeps its
    // text.
    property int seenWriteSeq: 0
    /// Whether the open file has already been read again for the answers this notify carried. **One read for the
    /// lot of them**: a drain can bring two answers that both moved what the two sides hold, and they leave the same
    /// one file to read — a second reading would cost a second git and bring back the same bytes.
    property bool diffReadForAnswers: false
    /// Reads the open file again because a write answered, and tells the tab it was read — which is what keeps the
    /// status that write publishes behind it from reading the very same file as news (`ops::DiffReread`).
    ///
    /// **The one door**, so that door is where the reading is counted.
    function readDiffForAnswer() {
        if (page.diffReadForAnswers)
            return
        page.diffReadForAnswers = true
        repoTab.noteDiffRead()
        page.reloadDiff()
    }
    // What an answer *means* is settled on the tab, where the op names are known (`drain::settle_write`); this
    // function only sequences the screen off those classified properties — reads, landings, menus.
    function absorbWriteResult() {
        if (repoTab.writeSeq === page.seenWriteSeq)
            return
        page.seenWriteSeq = repoTab.writeSeq
        page.diffReadForAnswers = false
        // The press has its answer. What is left of the wait is the read, which says so itself (`diffSettling`).
        page.diffAwaits = false
        // A push this button sent has come back; what it means for the toolbar's button is the flow's to work out.
        publishFlow.noteWriteAnswer()
        // …and what git said about it is the page's to say, off the same answer.
        page.absorbPushAnswer()
        // …and the same for the pushes a ref row sends, which answer under the same word.
        page.absorbRefPushAnswer()
        // A pop that did not happen leaves its entry, and its name, where they were — so this is read on both
        // landings, above the refusal branch and its early returns.
        page.absorbPopLabel()
        // The same for the editor's commit, which is answered by the write it sent and by no other. Above the branch
        // below for the same reason, and out of it altogether: the group that branch reads describes the answers
        // nobody was waiting for, and this one was waited for by name (`RepoTab.commitAnswer`).
        page.absorbCommitAnswer()
        // …and for the plain delete a card stayed up for, which is the same arrangement one door along.
        page.absorbBranchDelete()
        // …and for the stash that took the working tree away.
        page.absorbStashAnswer()
        // **The half of a press's pair that this notify completes.** The tree a stash emptied may already have been
        // read — the status and this answer are drained apart — and then nothing else is coming to ask on its behalf.
        // Asked with no edge of its own: the landing is the edge (`leaveWipWhenDone`), and where nothing is standing
        // this reads as the poll it already ignores.
        page.leaveWipWhenDone(false)
        // What is left over is the answers nobody named — **and its branches end it**, so no press's own answer can
        // be skipped by a refusal somebody else's write came back with.
        page.absorbLeftoverAnswer()
    }
    /// The one answer of this notify nobody was waiting for by name, sequenced off the group it left behind
    /// (`RepoTab::fold_into_group`): the timer's fetch, a write a page that has since gone away sent.
    ///
    /// **Every early return in here ends this and nothing else.** The presses that named their writes are answered
    /// above, where a refusal in this group cannot reach them.
    function absorbLeftoverAnswer() {
        if (repoTab.writeRefused) {
            // Nothing moved, so nothing is coming to the screen for a move to be recognised by: pressing again is the
            // reader's to do, and this is the one put-down `moveLanding` cannot wait for a landing for.
            page.moveLanding = ""
            // Whatever the window took away for this write is already back: the rows are the delete's own owner to
            // put down, and it did so on the answer itself — before this notify went out, so the row the question is
            // about is on screen when the question is (`ops_delete::delete_answered`).
            //
            // The rows in the diff are the other thing a refusal leaves standing, and they are not the file any more
            // — drifted bytes are the one thing the fingerprint refuses on, and the tally watch below cannot always
            // catch the drift that caused it (an outside change that moves no bucket count moves no tally), so left
            // alone the same press would be refused again for as long as the reader cared to try. The refusal's
            // answer is the fresh file.
            if (repoTab.writeStaleDiff)
                page.readDiffForAnswer()
            // The write did not happen and something outside this application said so — a protected branch, a hook
            // over there or here, a remote this end had only an older picture of. Nothing here could have known
            // beforehand and nothing here can answer it, so what it said comes down as a report and the log stays
            // where the reader left it (デザイン規約 §答えの要らない報せ).
            //
            // Anything but the name itself being turned down leaves the box nothing to answer, so it comes down the
            // way a landing takes it down — a half-finished rename most of all, where the row it was on is exactly
            // what could not be found.
            if (repoTab.writeReportKind !== "rename")
                page.noteRenameLanded()
            if (repoTab.writeReportKind !== "") {
                // A name git would not take goes back into the box it was typed into as well as into the bar: the
                // box is still open, holding it (デザイン規約 §答えの要らない報せ). Whichever of the two boxes was waiting
                // answers; the other is not open and says nothing.
                if (repoTab.writeReportKind === "rename")
                    page.noteRenameRefused(repoTab.writeReportReason)
                page.showReport(repoTab.writeReportKind,
                                repoTab.writeReportRemote,
                                repoTab.writeReportName,
                                repoTab.writeReportReason)
                // …and the mark in the corner goes quiet with it: it is there to fetch somebody to a failure nothing
                // else has said, and the bar has just said this one (デザイン規約 §git が言ったことを読む場所). The row keeps
                // git's words under its red edge — that is the record, and the record is what the panel is for.
                //
                // **The row it is about may not have reached the log yet.** The write's answer and the command's own
                // end travel separate feeds, in no fixed order: where the row has landed already, taking the mark
                // down is the whole of it; where it has not, it puts the mark back up when it does, and this is the
                // credit that takes it down again (measured, 1 Linux run in 5, and none of 5 on Windows — the
                // picture is identical either way).
                if (!commandsModel.failed)
                    page.answeredFailures++
                commandsModel.noteAnswered()
                page.pendingRenameRemote = ""
                page.pendingRenameTo = ""
                page.forgetRenameTagRemote()
                page.writeReported(repoTab.writeReportKind,
                                   repoTab.writeReportRemote,
                                   repoTab.writeReportName)
                return
            }
            // Nothing else on screen says what git said, so the log comes up (デザイン規約 §git が言ったことを読む場所).
            // Raised from the answer, because the ones that answer by their exit code do not raise it themselves —
            // and whether an operation built out of several of them failed is a question only its own answer can
            // settle.
            //
            // **A fetch answering here is not somebody else's news**: it is the same refusal the tab has already
            // counted, lined and raised the panel once for, and taking the panel over on it would leave the reader's
            // screen holding a failure that has since healed (`CommandsOwner`).
            if (!repoTab.writeFetched)
                commandsOwner.newsTakes()
            page.commandsOpen = true
            // A rename that did not happen has nothing to carry over.
            page.pendingRenameRemote = ""
            page.pendingRenameTo = ""
            page.forgetRenameTagRemote()
            return
        }
        // A plain delete that landed is not answered here: the card that stayed up for it goes by itself
        // (`RefBranchMenu` reads `RepoTab.branchDeleteLanded`).
        //
        // The name went in, so the box that was holding it has done its job and comes down (デザイン規約 §答えの要らない報せ:
        // a rename keeps its box until git answers).
        //
        // **Only once the queue has drained.** The writes are serialised but a fetch queued ahead of the rename
        // answers first, and this branch cannot tell whose answer it is holding — closing the box on that one would
        // take it down before its own refusal arrived, leaving git's words nowhere to go. Nothing else can be in
        // flight when the rename's own answer lands, so `busyCount` is the gate.
        if (repoTab.busyCount === 0)
            page.noteRenameLanded()
        // The branch took its new name here; the remote it was measured against is still under the old one. Asked
        // only now, and only because there is a remote to ask about (デザイン規約 §手元の改名の後のリモート).
        if (repoTab.writeBranchOp && page.pendingRenameRemote !== "") {
            const spokenFor = page.pendingRenameRemote
            const took = page.pendingRenameTo
            page.pendingRenameRemote = ""
            page.pendingRenameTo = ""
            page.carryBranchRenameOver(spokenFor, took)
        }
        // And the tag's own half, on the same terms and through the same bar: the name over there is still the old
        // one, and what to do about it is the same three answers.
        if (repoTab.writeTagOp && page.pendingRenameTagRemote !== "") {
            const on = page.pendingRenameTagRemote
            const was = page.pendingRenameTagFrom
            const now = page.pendingRenameTagTo
            const at = page.pendingRenameTagOid
            page.forgetRenameTagRemote()
            renameCarryFlow.startAsk("tag", on, was, now, at)
        }
        // Where the answer sends the reader — **read out of the answers this notify carried**. One drain empties the
        // whole queue and notifies once (`RepoTab::write_answers`), so a write *starting* in the same batch — the
        // fetch that follows a run — takes the stop's flag back down before this line is reached, and the reader is
        // left on the graph where the conflicted rows should have been.
        //
        // Walked in order with the later answer winning: the two landings are exclusive — a stop leaves no commit at
        // the tip to go to — so the one armed is the one the last answer asked for.
        for (let i = 0; i < repoTab.writeAnswerCount(); i++) {
            // git stopped part-way and left the operation standing, so there is no commit at the tip to land on and
            // the answer to the press is the working tree: the conflicted rows, and the way out under them
            // (デザイン規約 §進行中の操作から出る). Armed, for the reason the head landing below is: the status that will
            // carry those rows has not arrived yet (by design).
            if (repoTab.writeAnswerStopped(i)) {
                page.pendingWipSelect = true
                page.pendingHeadSelect = false
                page.pendingHeadAsked = false
            }
            // The answer is a commit at the tip, and that commit is what was asked for here. The selection goes to it
            // and the viewport follows: what was clicked can be anywhere in the history, while the answer is always
            // at the top.
            else if (repoTab.writeAnswerAtTip(i)) {
                page.pendingWipSelect = false
                page.pendingHeadSelect = true
                // The answer names the first report that may answer it (`RepoTab.writeAnswerHeadSeq`): the report
                // in hand may be the one before the write or already the one after it — the two feeds are drained
                // in no fixed order — and the number tells them apart where a count of reports could not.
                page.pendingHeadFromSeq = repoTab.writeAnswerHeadSeq(i)
                page.pendingHeadAsked = true
            }
        }
        // The report that answers the landing may already be in hand (above).
        page.tryPendingHeadSelect()
        // The write moved what the two sides hold, so a diff left open on either is a picture of a file as it was —
        // the same staleness a press on the file list's own `+` leaves, which `reloadDiff` answers.
        //
        // **Read here, where the answer is.** git has already moved the index by the time it answers, so the file's
        // diff is the new one — what has not caught up yet is the *file list*, and that is a different question
        // (`followEmptySide` asks it later). The status that follows would be the other place to read from, but it is
        // published only when it has rows to change, so a second line staged out of the same file would never be read
        // at all.
        //
        if (repoTab.writeStaleDiff)
            page.readDiffForAnswer()
        // The message landed: the editor stops offering to save it, and keeps what was written until the selection
        // catches up with the commit that now carries it.
        if (repoTab.writeReworded)
            detailsPane.noteMessageSaved()
        // Moving HEAD rewrites the working tree under the diff pane: the file it holds may not even exist where the
        // move landed, so the center goes back to the graph that was moved through. Taking the branch back does the
        // same to the file, and to which side of the index it sits on.
        if (repoTab.writeMovedHead)
            page.closeDiff()
    }

    // Center area switches between the graph and a file diff. The pieces are kept apart: a path may contain
    // anything, colons included.
    property bool diffShown: false
    onDiffShownChanged: page.foldForDiff(page.diffShown)
    property string diffKey: ""
    property string diffKind: ""
    property string diffPath: ""
    property string diffOrigPath: ""
    // Whether the shown diff is a working-tree file (stageable).
    property bool diffFromWt: false
    readonly property bool diffStaged: page.diffKind === "staged"
    /// The two stage letters git reports for the file being shown, read once when it is opened: the pane is handed a
    /// path, and on a conflict git prints no patch for those letters are all there is
    /// to say. A file stops being conflicted only by a write, and every write reads the diff again.
    property string diffChange: ""
    /// The colour each side of a conflict is drawn in: the lane colour the graph gives that branch where it has one, so
    /// the diff borrows an answer.
    ///
    /// A binding: the graph arrives in two passes (the chips land after the rows), and it is
    /// rebuilt whenever refs move. `finishCount` is read only to depend on it — every graph property shares one notify
    /// signal, so touching any of them is what makes this re-run when the rows change.
    readonly property int sideColorOurs:
        graphModel.finishCount >= 0 ? graphModel.conflictColorOurs(workTree.sideOurs, workTree.sideTheirs) : -1
    readonly property int sideColorTheirs:
        graphModel.finishCount >= 0 ? graphModel.conflictColorTheirs(workTree.sideOurs, workTree.sideTheirs) : -1
    function toggleDiff(kind, path, origPath) {
        if (page.diffShown && page.diffKey === kind + ":" + path) {
            page.closeDiff()
            return
        }
        page.openDiff(kind, path, origPath)
    }
    /// Reads one file, whoever asked — a row that was clicked, or the pane moving itself off a side that ran out
    /// (`followEmptySide`).
    function openDiff(kind, path, origPath) {
        // Held off while a plan stands. The plan has the centre for its whole stay (`centreStack`), so a file
        // read here goes into a pane nobody can see — and it is still open when the plan is put away, which lands the
        // reader who came back for the graph on a file instead. **This is the only place the diff can be raised**, so
        // the fold, the neighbour and the read are all held by the one line.
        //
        // The file rows themselves stay live, because the right pane does (規約 §フル interactive rebase「右の詳細
        // ペインがそのまま生きる」): what is held is the one thing a press on them reaches past the pane to do. Silent,
        // like the mode's other refusals — the arrows are already inert here for a reason of their own (`readPath` is
        // empty with no diff open, and `FileRowWalk.stepFile` will not step from nowhere).
        if (page.planShown)
            return
        // Whatever place a rebuild of the last diff was keeping is the last diff's: restored here it would put the new
        // rows at the old file's scroll (規約 §diff を横へ送る「別のファイルは左端から」).
        diffPane.dropScroll()
        page.diffKey = kind + ":" + path
        page.diffKind = kind
        page.diffPath = path
        page.diffOrigPath = origPath
        page.diffFromWt = kind !== "commit"
        page.diffChange = kind === "conflicts" ? page.wipUnstaged.changeOf(path) : ""
        // The list's light names the file being read. A click had already made this row the whole of the choice, so
        // what this catches is the pane moving itself — the commit's list needs no such line, its light *is* the path
        // being read (`DetailsPane.readPath`).
        //
        // **Only this window's own tree has such a choice.** The file being read may be another copy's, and that
        // pane's light is the reading alone; picking here would put this tree's own list on a path chosen in a tree
        // nobody asked about — and that choice is what a press acts on (`WipPane.readOne`).
        if (kind !== "commit" && page.wipWritable)
            wipPane.readOne(kind, path)
        if (kind === "commit")
            page.askCommitDiff(path, origPath)
        else if (page.carriedPath !== "")
            // The contents have to be readable even where nothing here can be written: the read is the ordinary one,
            // aimed at the copy the rows came from (`RepoSession::load_carried_diff`).
            diffModel.requestCarried(page.carriedPath, kind, path, origPath)
        else
            diffModel.requestWorkTree(kind, path, origPath)
        page.diffShown = true
        page.diffNeighbour = ""
        page.noteDiffNeighbour()
    }
    /// A pointer came to rest on a commit, or left one. **Two lists reach this**: the graph's rows and the ones the
    /// right pane lists under a choice (デザイン規約 §複数のコミットを選ぶ). One card between them — what it holds back
    /// depends on what the row it came off had already shown (`RowHoverHost.openRowCard`).
    function restOnCommit(row, inside) {
        rowHost.rowCardWanted = inside
        if (inside)
            rowHost.openRowCard(row)
        else
            rowHost.settleRowCard()
    }
    /// The patch behind a row of the commit pane's list, in the three shapes that list comes in
    /// (デザイン規約 §複数のコミットを選ぶ): one commit's own change to the file, what differs between exactly two, or —
    /// for a merged list — what **each** of the chosen commits did to it, one block after another.
    ///
    /// **The last is each commit's own block.** The list above it is what these commits did, so the patches
    /// behind a row of it are theirs: a diff of the two ends would carry whatever unchosen commits stand in between.
    function askCommitDiff(path, origPath) {
        if (detailsModel.selectionCount > 2) {
            diffModel.requestChoiceFile(page.chosenIds, path, origPath)
            return
        }
        if (detailsModel.comparing) {
            diffModel.requestRangeFile(detailsModel.compareFrom, detailsModel.compareTo, path, origPath)
            return
        }
        diffModel.requestCommitFile(detailsModel.shaHex, detailsModel.parentHex, path, origPath)
    }
    // Stages (or unstages) one hunk, or one line of it. The indices address the diff currently on screen, so the pane
    // is reloaded afterwards: once the patch is applied the rows have moved.
    function stageSelection(hunk, line) {
        // The seats these come from are not offered on another copy's file (`DiffPane.partial`); this is the belt.
        if (!page.wipWritable)
            return
        // The shown diff's fingerprint rides along: the write refuses to apply the indices to bytes that drifted since
        // this was read.
        //
        // Said here, because `busyCount` rises when the queue starts the write: two presses in a row both went out
        // before the first had begun.
        //
        // Armed by the answer: the slot says whether a write actually went out, and a request it turned away —
        // nothing selected, no fingerprint to address — has no answer coming, so a wait armed for it would hold the
        // marks for good (the same wedge the tree-read wait had, through the request's own door).
        page.diffAwaits = repoTab.stageSelection(page.diffKind, page.diffPath, page.diffOrigPath, hunk, line,
                                                 diffModel.fingerprint)
    }
    /// Throwing one hunk of the shown diff away, with no question in front of it: the button in that hunk's own heading
    /// was held down, which is the whole of the asking (デザイン規約 §その他の操作). A line cannot be thrown away on its own — the
    /// hunk is the smallest piece — though it can still be staged on its own, which loses nothing.
    function discardHunkNow(hunk) {
        if (!page.wipWritable)
            return
        // Armed by the answer, for the reason `stageSelection` gives.
        page.diffAwaits = repoTab.discardSelection(page.diffKind, page.diffPath, page.diffOrigPath, hunk, -1,
                                                   diffModel.fingerprint)
    }
    // ---- what the reader lands on when a side runs out ---------------
    /// The row beside the open diff's file under the same heading, as `<bucket>:<path>` — the file after it, or the one
    /// before it where it is the last (`NavSectionModel.besidePath`).
    ///
    /// Noted while the file is still there, because by the time the side has run out the file has already moved to the
    /// other one and the place it left is not in the list to be read.
    property string diffNeighbour: ""
    function noteDiffNeighbour() {
        if (!page.diffShown || page.diffKind === "commit" || !page.wipUnstaged.holdsPath(page.diffKind, page.diffPath))
            return
        page.diffNeighbour = page.wipUnstaged.besidePath(page.diffKind, page.diffPath)
    }
    /// Everything the open diff's file had on the side being read has gone over — staged, unstaged, thrown away,
    /// committed. The reader is left standing on it, so the pane moves (デザイン規約 §diff の中のステージ):
    ///
    ///  - the next file of the side that ran out, if it still has one;
    ///  - otherwise the same file, read from wherever it went — the whole
    ///    of it is on the other side now, which is the thing to look at;
    ///  - and only with nothing uncommitted left does the pane close.
    ///
    /// **The file list is what says so** — this runs on its `changed` and stands down while it still holds the file on
    /// the side being read. The re-read's own emptiness cannot say it: the read runs beside the status
    /// (`load_diff` / `publish_status`), so an empty answer could land first and ask a list that still held
    /// the pre-write rows for a neighbour — and some sides never read empty at all (an untracked file staged whole
    /// still renders as its whole content, a picture has no rows either way). Asked here, the answers below are read
    /// from the very change that said the file moved.
    function followEmptySide() {
        if (!page.diffShown || page.diffKind === "commit" || page.wipUnstaged.holdsPath(page.diffKind, page.diffPath))
            return
        const cut = page.diffNeighbour.indexOf(":")
        if (cut > 0) {
            const bucket = page.diffNeighbour.substring(0, cut)
            const path = page.diffNeighbour.substring(cut + 1)
            if (page.wipUnstaged.holdsPath(bucket, path)) {
                page.openDiff(bucket, path, page.wipUnstaged.origOf(path))
                return
            }
        }
        const moved = page.wipUnstaged.bucketOf(page.diffPath)
        if (moved !== "") {
            page.openDiff(moved, page.diffPath, page.wipUnstaged.origOf(page.diffPath))
            return
        }
        page.closeDiff()
    }
    /// Whether the diff on screen is still catching up with a write.
    ///
    /// A press addresses the rows it was made on, and carries the fingerprint of the bytes they were read from; git
    /// refuses it against anything else. So from the moment a press goes out until the rows it changed are back, the
    /// pane holds the next one back — pressed twice in a row, the second landed on the diff the first had already
    /// replaced and came back with a refusal in the log.
    ///
    /// Three parts, in the order they happen, and **every one of them ends by itself**: the press is out and no answer
    /// has come (`diffAwaits`, armed only when the tab says a write went out, put down by the write's own answer), git
    /// is running (`busyCount`, which the session balances), the file is being read again (`loading`, put down by the
    /// rows arriving).
    ///
    /// **Every wait here has an end of its own.** Held on "the tree has not been read yet" instead, it
    /// wedged for good the first time a write moved no rows — staging a second line of a file already on both sides —
    /// because the file list only says `changed` when its rows differ, and then no `+` anywhere would go in again.
    property bool diffAwaits: false
    readonly property bool diffSettling:
        page.diffAwaits || repoTab.busyCount > 0 || diffModel.loading
    /// A write on the working tree has landed, so the open diff is a picture of what the file used to be.
    ///
    /// **Whoever wrote it.** The file list's own `+` and `−` move the same file out from under the pane as the writes
    /// made inside the diff do: asked for by the diff's writes alone, a line staged here and then unstaged there is
    /// left missing from both sides on screen. The caller already knows the write
    /// was one that moves the tree (`absorbWriteResult`), so being open is the whole of the condition.
    function reloadDiff() {
        // Nothing this window writes moves another copy's file, so a copy's diff is never re-read from here — its own
        // tick is what keeps it current (`pollCarried`).
        if (!page.diffShown || page.diffKind === "commit" || !page.wipWritable)
            return
        diffModel.requestWorkTree(page.diffKind, page.diffPath, page.diffOrigPath)
    }
    /// Asks whether the file on screen still reads the way it did. Run on the page's tick, beside the repository's own
    /// re-read.
    ///
    /// **The file can move with the tree standing still.** The counts below say a stage or an unstage happened, and
    /// the file list says a file changed state; neither hears an edit that leaves both where they were — a conflict
    /// resolved in another window keeps its two stage letters until it is added, so the pane went on drawing the
    /// conflict it was opened on. Core answers this with silence unless the bytes moved
    /// (`RepoSession::refresh_diff`), so a quiet file costs one read and no repaint.
    ///
    /// Only a settled working-tree file is asked: a commit's diff is settled already, and one catching up with a
    /// write has its answer coming.
    ///
    /// Answers whether a read went out, for the automation to latch on (`diff-tick`): the read itself is answered with
    /// silence on a file nobody touched, so the ask is the only edge this side of it has.
    function pollDiff() {
        if (!page.diffShown || page.diffKind === "commit" || page.diffSettling)
            return false
        // A copy's file is re-read on the copies' tick, and aimed at the copy — sent through this one it would read
        // *this* window's file of that name and hand it to a pane showing somebody else's (`pollCarried`).
        if (!page.wipWritable)
            return false
        return diffModel.refreshWorkTree(page.diffKind, page.diffPath, page.diffOrigPath)
    }
    /// The copies' own tick over the copy being read: its file list, and the file open from it. **Off the page's tick**
    /// for the reason the rows' tallies are — a `status` of another tree is not a price the ten-second tick can pay
    /// (`RepoSession::refresh_carried`).
    ///
    /// Answers whether it asked for anything, for the automation to latch on: the file's re-read is answered with
    /// silence unless the bytes moved, the same way the page's own tick is.
    function pollCarried() {
        if (page.carriedPath === "")
            return false
        repoTab.readCarriedStatus(page.carriedPath, page.carriedName)
        if (page.diffShown && page.diffKind !== "commit" && !diffModel.loading)
            diffModel.refreshCarried(page.carriedPath, page.diffKind, page.diffPath, page.diffOrigPath)
        return true
    }

    /// Puts the diff away by a gesture aimed at the diff itself — Escape, or the header's own mark — and hands the
    /// arrows to the graph (デザイン規約 §diff のファイル一覧「diff を仕舞う所作は、矢印をグラフへ返す」).
    ///
    /// **The file row's second click keeps them where they are** (`toggleDiff`): it is a press, it landed in the
    /// list, still on screen with the row it lit — so the keyboard stays where the press fell and
    /// the arrows go on walking files (規約 §diff のファイル一覧「キーボードは press が落ちた所に残る」). These two doors
    /// have no such place to leave it in: Escape is not a press at all, and the mark's own pane is what goes. What
    /// is left in the middle is the graph, so the graph answers next.
    ///
    /// **This door is the reader putting the diff away.** The rest go their own way — a plan taking the centre, a
    /// write moving HEAD, the side under the reader running out, the list coming back — and the graph is not always
    /// even what they leave behind.
    function closeDiffToGraph() {
        page.closeDiff()
        graphPane.takeKeyboard()
    }

    function closeDiff() {
        page.diffShown = false
        page.diffKey = ""
        page.diffKind = ""
        page.diffPath = ""
        page.diffOrigPath = ""
        page.diffFromWt = false
        page.diffChange = ""
        diffModel.clear()
    }

    // Exposed for the window toolbar (acts on the active tab).
    readonly property var pageTab: repoTab
    readonly property var pageWt: workTree
    /// The branch list, for the one reading the band takes off it: whether what this branch is measured against has
    /// gone from the remote (`headUpstreamGone`, git's own `[gone]`).
    readonly property var pageBranches: branchesModel
    /// The copies this repository has, for the panel's own door into them (`TopBar`'s WORKTREE rows). The same
    /// listing the left menu's WORKTREES section draws, so the two name the same places in the same order.
    readonly property var pageWorktrees: worktreesModel
    /// What the band's Stash button would name the entry — the commit box is on this page, the button is not
    /// (デザイン規約 §変更を退避する). Decided by the pane that owns the box, so the file row's `stash` reads the same one.
    readonly property string pageStashName: wipPane.stashName
    readonly property var pageCommands: commandsModel
    /// The commit editor itself. Automation only, and only for the one verb that has to reach it from outside a page:
    /// `tab-carry` writes words into one tab's boxes and reads them out of another's, which is a question about two
    /// pages and so belongs to the window (`WindowAutoActDriver`). Everything a person does to these boxes goes
    /// through the pane's own signals.
    readonly property alias pageWip: wipPane
    /// For the settings card's avatar entry, which offers the authors of the repository being looked at.
    readonly property var pageGraph: graphModel
    /// The graph's rows themselves. Automation only, and for the same reason `pageWip` is exposed: a window-level
    /// verb asks one page for a row and reads the answer in the window (`carried-open` stands this tab in another
    /// working copy, and where the strip lands is the window's to say).
    readonly property alias pageGraphPane: graphPane
    /// The left menu, for the same one reason (`worktree-stand`): a WORKTREES row stands the tab in that copy, and
    /// the landing is the strip's — this page stays, so the verb reads the answer off the window.
    readonly property alias pageSidebar: sidebarPane
    /// Whether the refs listing has landed — the read that also settles how many remotes this repository has, and so
    /// what the band's fetch button is allowed to be (`fetch-tip`).
    readonly property bool pageRefsLoaded: branchesModel.refsLoaded
    /// Whether the right pane is still waiting on the selected commit's own read — the last of the page's reads, and
    /// the one the changed-file list is laid out from. **The graph's pass and the refs are both in before it is even
    /// asked for**: they are what decide the row this page opens on, and the request goes out from that landing
    /// (`trySelectDefault`). **Settled**: a page showing the working tree is waiting for no such answer, and neither
    /// is a repository with no commit to select, so both answer yes holding no details at all.
    readonly property bool pageDetailsSettled: page.wipShown || page.selectedOid === ""
                                               || !detailsModel.loading
    /// Whether this page still owes itself a landing — a move of the reader it has already decided on and is holding
    /// until the refreshed pair can carry it (`pendingHeadSelect` / `pendingWipSelect`). Read where a sampler asks
    /// whether the page has stopped arriving (`PageSettled`): between a write's answer and the landing behind it,
    /// every other reading of the page is of the repository as the write found it, and both ends of that hold still.
    readonly property bool pageLanding: page.pendingHeadSelect || page.pendingWipSelect
    /// The window's own band, handed back in by `Main`. The Stash button stands there (デザイン規約 §変更を退避する), and
    /// this page's verbs press the real one through here — a verb that called what the button calls would be
    /// answering for a second way in. An automation-only exposure, the same one `GraphPane.view` is (app-ui.md).
    /// `var` because `TopBar` is above this file, not beside it.
    property var pageBand: null

    /// Whether the command log is up. Closed is the resting state: the `>_` at the foot of the left menu opens it, and
    /// a failed command raises it.
    property bool commandsOpen: false
    // The mark is the panel's own, so it cannot outlive it (`commandsAttention`). Written here — the three ways down
    // the reader presses, Escape, and the recovery a landing fetch takes it down with all say the same thing —
    // because what it says is about the mark.
    onCommandsOpenChanged: if (!page.commandsOpen) page.commandsAttention = false
    // Who the panel standing belongs to — the one rule here that is about the **order** failures arrive in, which is
    // why it is a component of its own with every order walked (`CommandsOwner` / `tst_commandsowner.qml`).
    CommandsOwner {
        id: commandsOwner
    }
    function toggleCommands() {
        commandsOwner.readerTakes()
        page.commandsOpen = !page.commandsOpen
    }
    /// The panel was sent for from somewhere else in the window, and it says so once the reader gets there
    /// (デザイン規約 §hover のツールチップ — 送った先は名乗る). **The reader's own hand, so `readerTakes()`**: a panel
    /// asked for is a panel the next landing fetch may not take away, which is the same thing the `>_` says.
    ///
    /// Raised whether or not the panel was already up — what the mark answers is "you were sent here", and a reader
    /// who was looking at it already was still sent.
    property bool commandsAttention: false
    function raiseCommands() {
        commandsOwner.readerTakes()
        page.commandsOpen = true
        page.commandsAttention = true
    }
    /// The one link this window's tooltips carry (`Words.commandsHref`). Unknown hrefs are nobody's: a tip is not a
    /// browser, and the set of places it can send a reader is the set spelled here.
    function tipLinkAsked(href) {
        if (href === Words.commandsHref)
            page.raiseCommands()
    }
    /// Takes the panel down for its own two controls — `Clear` and the closing mark.
    function shutCommands() {
        commandsOwner.readerTakes()
        page.commandsOpen = false
    }
    /// A fetch has reached the remote again, and the panel its failure raised goes down with the news
    /// (デザイン規約 §git が言ったことを読む場所 — パネルを下ろす唯一の自動). **Read on every drain**: the run of failures
    /// going back to zero *is* the recovery, and the tab holds both halves of what the answer turns
    /// on.
    function absorbFetchRecovery() {
        if (commandsOwner.landingTakesItDown(repoTab.fetchFailures, repoTab.lastError !== ""))
            page.commandsOpen = false
    }
    /// One mark for a failed command and for this tab's error line both. The page's rule, since the mark moves seats.
    readonly property bool commandsWrong: commandsModel.failed || repoTab.lastError !== ""
    /// The panel itself, or null: it is built into its seat when the log is raised and taken down with it
    /// (`commandsSeat`), so everything below that reads the panel reads it through here and answers for a log that
    /// is down with nothing.
    readonly property var commandsPane: commandsSeat.item
    /// Automation: the colour the `>_` painted — the log's own band while it is up, the pane's foot while it is down.
    readonly property color commandsMarkColor:
        page.commandsPane !== null ? page.commandsPane.markColor : sidebarPane.commandsMarkColor
    /// What the panel is doing — automation reads this one, so a cut binding fails.
    readonly property bool commandsShown: page.commandsPane !== null && page.commandsPane.visible
    /// Automation reads the laid-out width before persisting a state round trip.
    readonly property real stateDetailsWidth: rightPane.width
    /// Automation only: the header's `Clear`, pressed from outside the panel (`PGG_AUTO_ACT=commands-clear`).
    function clearCommandLog() {
        if (page.commandsPane !== null)
            page.commandsPane.clearPanel()
    }
    /// Automation only: the hand that drags over the log, and the key that takes what it picked
    /// (`PGG_AUTO_ACT=commands-select` / `commands-copy`). Both enter the panel's own functions.
    function pickCommandText(fromRow, fromAt, toRow, toAt) {
        if (page.commandsPane !== null)
            page.commandsPane.pickText(fromRow, fromAt, toRow, toAt)
    }
    /// ...and the same hand started on the ground under the last row (`PGG_AUTO_ACT=commands-sweep`).
    function sweepCommandGround(fx, fy) {
        return page.commandsPane !== null && page.commandsPane.sweepGround(fx, fy)
    }
    /// Automation: how much of the log is actually wearing the wash the drag left (`CommandsPane.washTally`) — the
    /// half of `commands-select` the picture cannot be judged on.
    function commandWashTally() {
        return page.commandsPane === null ? "worn=false rows=0 washed=0" : page.commandsPane.washTally()
    }
    readonly property bool commandsHasGround: page.commandsPane !== null && page.commandsPane.hasGround
    readonly property real commandsGroundTop: page.commandsPane !== null ? page.commandsPane.groundTop : 0
    function copyCommandText() {
        return page.commandsPane !== null && page.commandsPane.copySelection()
    }

    RepoTab {
        id: repoTab
        // A turn later, always: this arrives from the middle of the drain filling this very page, and standing the
        // tab elsewhere takes the page down (規約 §Qt Bridges の要点 — 再入 borrow). `Qt.callLater` also folds a
        // repeat ask into one, which is what a second refusal on the way down would be.
        onStandHomeAsked: Qt.callLater(page.standHomeRequested)
    }
    CommandsModel {
        id: commandsModel
        // The machine's offset from UTC, said before the first row arrives: the panel says it again each time it is
        // raised (`CommandsPane.tellTheZone`), but the panel is built only when it is raised, and the rows that
        // arrive before that are stamped as they arrive.
        Component.onCompleted: commandsModel.setZoneMinutes(new Date().getTimezoneOffset())
    }
    GraphModel { id: graphModel }
    WorkTreeModel { id: workTree }
    DetailsModel { id: detailsModel }
    DiffModel { id: diffModel }
    RebasePlanModel { id: planModel }
    NavSectionModel { id: branchesModel }
    NavSectionModel { id: remotesModel }
    // One letter apart, two different things: the three `*Model`s below feed the WIP pane (the "worktree" nav section =
    // uncommitted files), while `worktreesModel` lists git worktrees (the sidebar's WORKTREES).
    //
    // **One per bucket**, because the pane is a list per bucket (`WipPane`). Each shows its own run and holds the whole
    // status, so a question about a file — which bucket has it, what is beside it, where it came from — is asked of
    // `worktreeModel` whichever bucket the file is in.
    NavSectionModel { id: conflictsModel }
    NavSectionModel { id: worktreeModel }
    NavSectionModel { id: stagedModel }
    // The same three again, for the copy the pane shows when the row being read is another working copy's
    // (`page.carriedPath`). **A second set**: this window's own status arrives on its own tick whether or not anybody
    // is reading another copy, and one set of lists would lose the copy's rows to it every ten seconds — and lose the
    // reader's place in them with it.
    // The list another working copy's changes are shown as — **one**, where this window's own tree is three: the
    // split those three stand for is the index's, and nothing here can move that copy's index (`CarriedPane`).
    NavSectionModel { id: carriedModel }
    NavSectionModel { id: worktreesModel }
    NavSectionModel { id: stashesModel }
    NavSectionModel { id: tagsModel }

    /// True on the one page the window is showing, which is the only page there is: the window builds one for the tab
    /// in front and takes it down when that tab stops being in front (`Main.qml`). The blank page is the exception —
    /// it stands in for no tab at all.
    ///
    /// So this is set once, at construction, and `Component.onCompleted` is what acts on it. Nothing turns it over
    /// afterwards; a page that would have to be told it is no longer current is a page that has already been
    /// destroyed.
    property bool pageCurrent: false

    /// Everything this page owes on its way off the front, in the order it is owed (`TabsModel::leaving_tab`).
    ///
    /// **Called while the page is still whole**, which is the only moment any of it can be read: the strip has not
    /// moved yet, so nothing here has been taken down.
    ///
    /// The layout goes first because it is what the *next* tab is laid out at — one set for the whole application, so
    /// leaving is the moment it is worth writing down (`PageLayout.reportLayout`). Then the words in the commit
    /// editor, which are the one thing on this page no repository can be asked for again. Then the repository itself,
    /// which can.
    function leaveFront() {
        pageLayout.reportLayout()
        if (page.blank)
            return
        repoTab.holdDraft(wipPane.subjectText, wipPane.bodyText, page.amending)
        repoTab.release()
    }

    /// Everything this page owes on its way into another working copy of the repository it is showing
    /// (`TabsModel::leaving_copy`), and **this page is staying**: the graph, the refs sections and the panes around
    /// them are the repository's, and linked copies share all of it (デザイン規約 §タブの所作
    /// 「同じリポジトリのタブは 1 枚」). What goes is what the copy being left owned.
    ///
    /// **Called while that copy is still the one behind the tab**, which is what puts the unsent words back where
    /// they were written: the hub files them under the copy the tab is standing in at the time (`Hub::hold_draft`).
    function leaveCopy() {
        // The blank page stands in for no tab, so there is no copy to leave (`leaveFront` keeps the same guard).
        if (page.blank)
            return
        repoTab.holdDraft(wipPane.subjectText, wipPane.bodyText, page.amending)
        // …and the boxes go empty behind them: the copy arrived at has words of its own or none at all, and words
        // left standing would be read as that copy's (`restoreDraft` puts back only what it finds).
        page.amending = false
        wipPane.setAmendChecked(false)
        wipPane.clearMessage()
        // A plan is composed against one working tree and would be replayed in it.
        if (page.planShown)
            planModel.cancelPlan()
        // The pane goes back to this tab's own tree — which is about to be the copy it was reading.
        page.dropCarried()
        // A diff of a file in the tree being left is that tree's and goes with it. A commit's is the repository's
        // and stays open — the rows are the same on both sides of a linked copy — but a read still out was asked
        // of the session being closed, so that one is written down and asked again (`standSettled`).
        //
        // **Written down before the close below**, which is what takes the answer to "what was being read" away.
        page.owedDiff = page.diffShown && page.diffKind === "commit" && diffModel.loading
        if (page.diffShown && page.diffKind !== "commit")
            page.closeDiff()
        // A read still out was asked of the session about to be closed, so no answer is coming for it — and the
        // details pane is the one place that would wait for it in silence. Written down here, where it can still be
        // seen, and asked again once there is a session to ask (`standSettled`).
        page.owedDetails = detailsModel.loading
        // **Every answer this page was still waiting for was that session's**, and none of them is coming: a write
        // it accepted answers to a retired sink (`Hub::let_go_of_session`). Left armed, each is a wait nothing ends
        // — and worse than that for the ones armed by *number*, because the next session counts its writes from the
        // start and somebody else's answer would be taken for this one's (`pendingPopId`). The landings go with
        // them: where a write put the reader is about the tree it was made in.
        page.pendingPopLabel = ""
        page.pendingPopId = 0
        page.pendingRenameRemote = ""
        page.pendingRenameTo = ""
        page.diffAwaits = false
        page.moveLanding = ""
        page.pendingHeadSelect = false
        page.pendingHeadAsked = false
        page.pendingHeadFromSeq = 0
        page.pendingWipSelect = false
        page.rewordRow = -1
        page.planRunOut = false
        // What the models hold of that copy, each by its own rule (see the `restand` slots): the working tree's
        // answers go back to "not read yet", and the graph keeps its rows.
        repoTab.restand()
        graphModel.restand()
        workTree.restand()
        conflictsModel.restand()
        worktreeModel.restand()
        stagedModel.restand()
        carriedModel.restand()
        // **The command log is not on this list.** It is the record of what this window ran, and the window is the
        // same one: its rows name the session that made them (`CommandMsg`), so the copy arrived at numbers its own
        // commands from one without touching them — and a write the copy being left is still running ends in its own
        // row, where it has been saying "running" all along (`BridgeSink::retire_reads`).
    }

    /// …and the other side of it: the tab is standing in the copy that was asked for, and the session reading it is
    /// opening (`TabsModel::stood_copy`).
    ///
    /// **The words only.** Everything else this page owes itself waits for the session to say where it is
    /// (`standSettled`): a read asked before that reaches a session with no repository open yet and is answered with
    /// nothing at all (`RepoSession::load_details`).
    function standInCopy() {
        if (page.blank)
            return
        page.restoreDraft()
        // The log's switch is a setting of the *session* (`Recording`), and this is a new one — which opens
        // keeping the reader's commands and nothing else. The panel is still on screen saying otherwise, so what
        // it says is said again; the rows it asks for are the only thing the reader would notice missing.
        if (commandsModel.backgroundReads)
            commandsModel.setBackgroundReads(true)
    }

    /// The reads the closed session was still owing this page when the tab was stood elsewhere (`leaveCopy`): the
    /// commit being read about, and the file of it open in the diff.
    property bool owedDetails: false
    property bool owedDiff: false

    /// The session reading the copy this tab now stands in has said where it is (`RepoTab.standing` going down), so
    /// the reads that died with the last one can be asked again.
    ///
    /// **Only the ones that died.** What the right pane is showing is a commit's, and a commit is the repository's —
    /// the same on both sides of a linked copy — so asking for it again would spend a `git show` and a gpg run on an
    /// answer already on screen (デザイン規約 §グラフ行のダブルクリック keeps the same rule for a second click).
    /// What has to be asked again is what was still out: the details, which this page wrote down as it left, and the
    /// signature, which says so itself by not being the selection's.
    function standSettled() {
        if (page.owedDetails) {
            page.owedDetails = false
            if (page.chosenCount > 1)
                detailsModel.requestSelection(page.chosenIds, page.chosenCount === 2)
            else if (page.selectedOid !== "")
                detailsModel.request(page.selectedOid)
        }
        if (page.selectedOid !== "" && repoTab.signatureOid !== page.selectedOid)
            page.askSignature(page.selectedOid)
        // The file the diff is holding open, for the same reason and in the same words it was asked in
        // (`askCommitDiff` reads the pane's own header, which has not moved).
        if (page.owedDiff) {
            page.owedDiff = false
            if (page.diffShown && page.diffKind === "commit")
                page.askCommitDiff(page.diffPath, page.diffOrigPath)
        }
    }

    /// …and the other half: what the last page on this tab was holding, put back into the boxes.
    ///
    /// **Before the repository says anything**, so that the two paths that fill these boxes on their own — a stopped
    /// merge's message (`absorbOpMessage`) and a popped stash's name (`absorbPopLabel`) — find them already written
    /// in and leave them alone, which is the rule those two keep anyway.
    function restoreDraft() {
        // The flag as well as the words: a message written for an amend, put back under a plain commit button, would
        // make a second commit instead of replacing the first.
        if (repoTab.draftAmending()) {
            page.amending = true
            wipPane.setAmendChecked(true)
        }
        const subject = repoTab.draftSubject()
        const body = repoTab.draftBody()
        if (subject === "" && body === "")
            return
        // Words put back into a pane nobody is looking at are only half of them being kept: the reader left this tab
        // mid-sentence, and what they come back to has to be the sentence. This is also what holds the selection off —
        // `trySelectDefault` leaves a page showing the working tree alone.
        page.showWip()
        wipPane.setMessage(subject, body)
        // **`pendingWipSelect` stays down.** A dirty tree already puts its own row at the top of a graph nobody has
        // picked a row on, so the highlight lands there without asking; a clean tree has no such row, and the flag
        // would then stand until the tree got dirty and jump the view to the top from wherever the reader was
        // (規約 §ListView.highlightFollowsCurrentItem — a background pass may not move a reader's view).
    }

    // ---- what this page is laid out at ------------------------------
    // The saved sizes, the sections, and the floor the window is held to.

    PageLayout {
        id: pageLayout
        page: page
        sidebarPane: sidebarPane
        rightPane: rightPane
        commandsSeat: commandsSeat
        graphPane: graphPane
        wipPane: wipPane
        detailsPane: detailsPane
        repoTab: repoTab
        worktreeModel: worktreeModel
        detailsModel: detailsModel
        diffModel: diffModel
    }

    /// What the window reads off the page it is showing: the floor it may not be laid out under (`Main.floorWidth` /
    /// `floorHeight`) and the two panes' own overflow, which the floor's own verb asks about.
    readonly property real floorWidth: pageLayout.floorWidth
    /// …and the same floor with the list open whether or not it is, which is the one the band's actions time their
    /// giving way against (`PageLayout.openFloorWidth`).
    readonly property real openFloorWidth: pageLayout.openFloorWidth
    readonly property real floorHeight: pageLayout.floorHeight
    readonly property bool wipBlockScrolls: pageLayout.wipBlockScrolls
    readonly property real detailsOverHeight: pageLayout.detailsOverHeight

    /// …and what it calls: the layout is pulled on the window's timer, and the two setters are the headless state
    /// check's (a person drags).
    function reportLayout() {
        pageLayout.reportLayout()
    }
    function setDetailsWidth(w) {
        pageLayout.setDetailsWidth(w)
    }
    function setGraphColumns(labels, lanes) {
        pageLayout.setGraphColumns(labels, lanes)
    }
    function setSidebarWidth(w) {
        pageLayout.setSidebarWidth(w)
    }

    Component.onCompleted: {
        // The bars that announced themselves before the watcher existed.
        page.earlyBars.forEach(b => splitWatch.holdSplitBar(b, false))
        page.earlyBars = []
        pageLayout.applySavedLayout()
        if (page.blank)
            return // no session to attach to; every model stays empty
        repoTab.attach(page.tab_id)
        if (page.pageCurrent)
            repoTab.activate()
        commandsModel.attach(page.tab_id)
        graphModel.attach(page.tab_id)
        workTree.attach(page.tab_id)
        detailsModel.attach(page.tab_id)
        diffModel.attach(page.tab_id)
        planModel.attach(page.tab_id)
        branchesModel.attachSection(page.tab_id, "branches")
        remotesModel.attachSection(page.tab_id, "remotes")
        conflictsModel.attachWorktree(page.tab_id, "conflicts")
        worktreeModel.attachWorktree(page.tab_id, "unstaged")
        stagedModel.attachWorktree(page.tab_id, "staged")
        carriedModel.attachCarried(page.tab_id)
        worktreesModel.attachSection(page.tab_id, "worktrees")
        stashesModel.attachSection(page.tab_id, "stashes")
        tagsModel.attachSection(page.tab_id, "tags")
        page.restoreDraft()
        const acts = harness.ask()
        if (acts !== null)
            acts.begin()
    }

    /// The window's focus epoch (bumped when the window regains focus) triggers a quick refresh of the visible page.
    property int focusEpoch: 0
    onFocusEpochChanged: {
        if (page.visible && repoTab.state === "open") {
            repoTab.refreshQuick()
            // The window coming back is the moment an outside change is most likely to be waiting, so the file on
            // screen is asked as well — and where the pane is standing on another copy, that copy is what the file
            // belongs to (`pollCarried` reads one, not every copy: the rest are the slow tick's, which is the whole
            // reason they are on one).
            page.pollDiff()
            page.pollCarried()
        }
    }

    /// True while the window is on screen (see Main.qml): the page shown there re-reads its repository on a tick, so a
    /// commit made in a terminal or by an agent turns up on its own.
    property bool onScreen: false
    Timer {
        interval: Metrics.pollIntervalMs
        repeat: true
        // Only the tab in front — the others catch up when switched to, and reading every open repository on every tick
        // is what makes polling expensive elsewhere.
        running: page.onScreen && page.visible && repoTab.state === "open"
        onTriggered: page.pollRepo()
    }
    // What the other working copies are carrying, on a tick of its own because it costs a `status` each — how often
    // is the reader's to set (`AppBackend.copiesIntervalMs`, zero for never). Same conditions as the tick above:
    // being on screen is what drives it, since a window kept open beside another copy is what these rows are for.
    Timer {
        interval: AppBackend.copiesIntervalMs
        repeat: true
        running: page.onScreen && page.visible && repoTab.state === "open" && AppBackend.copiesIntervalMs > 0
        onTriggered: {
            repoTab.refreshCarried()
            // And the one being read, file by file — the tallies above are all the rows need, and the pane needs the
            // list behind them (`pollCarried`).
            page.pollCarried()
        }
    }
    // The badge counting a running replay out. Its own tick because it asks its own question: two file reads off the
    // git directory, no process, so it can run at a rate a number is worth watching at — where the tick above carries
    // a whole `git status` and is ten seconds apart for it (デザイン規約 §進行中・長押しの定数, and the measured cost
    // in `ci/baseline/poll-cost-windows-x64.md`).
    //
    // While the write is out and no longer: with nothing replaying there is no number, and the status tick is what
    // says the operation ended. `replayRunning` covers the gap a handed-over plan leaves before the queue starts it.
    Timer {
        interval: Metrics.opProgressMs
        repeat: true
        running: page.visible && repoTab.state === "open" && page.replayRunning
        onTriggered: repoTab.refreshOpProgress()
    }
    /// One tick: the repository, and the file the diff pane is holding. The two are separate reads because they answer
    /// different questions — refs and status say what the tree is, the diff says what the file says.
    function pollRepo() {
        repoTab.refreshPoll()
        // The worktree listing rides this tick: one process, and what it feeds — the WORKTREES rows, the mark saying
        // a branch is another copy's — was frozen until the window was clicked without it.
        repoTab.refreshWorktrees()
        page.pollDiff()
    }

    // Where the selection stands, kept so a commit that disappears from under it can be followed to whatever took its
    // place.
    property int selectedRow: -1

    // ---- the commits being held (デザイン規約 §複数のコミットを選ぶ) ----
    /// The choice, as a set of commit ids, and how many are in it. **Ids**: a background pass rewrites the rows under
    /// the hand, and everything this page holds is re-resolved by id when one lands (`onStatsChanged`).
    /// Empty only where the working tree's row is what is shown — that row is not a commit.
    property var chosenOids: ({})
    property int chosenCount: 0
    /// The commit the last press landed on, which is what a Shift click measures its range from. **A commit**: a
    /// background pass rewrites the rows under the hand, and a row number kept across one would measure the
    /// next range from whatever commit had moved into it (デザイン規約 §複数のコミットを選ぶ). Empty before the first press.
    property string chosenAnchorOid: ""
    /// The choice as the models want it: the ids in walk order, and the rows that name them. Settled together, so the
    /// list on the right and the files under it can never be of different commits.
    property var chosenIds: []
    property var chosenRecords: []
    /// Makes one commit the whole of the choice. Every way a single row becomes what is being read comes through here
    /// — a plain click, an arrow key, a landing, the find bar — so there is one place the choice is settled from.
    function chooseOnly(oidHex) {
        const only = ({})
        if (oidHex !== "" && !GitFacts.wipOid(oidHex))
            only[oidHex] = true
        page.chosenOids = only
        page.chosenCount = Object.keys(only).length
        page.chosenIds = []
        page.chosenRecords = []
    }
    /// A left click on a row, as this page answers one. **The choice and what is being read are two answers**: a plain
    /// click makes one commit both, a modified one moves only the choice (デザイン規約 §複数のコミットを選ぶ).
    function pickRow(oidHex, atRow, modifiers) {
        const mods = modifiers === undefined ? Qt.NoModifier : modifiers
        // The working tree's row is not a commit: a modifier on it is the plain click it would be without one.
        if (mods === Qt.NoModifier || oidHex === "" || GitFacts.wipOid(oidHex)) {
            page.activateRow(oidHex, atRow)
            return
        }
        if (mods & Qt.ShiftModifier) {
            // The anchor is found again by its id: the rows may have moved since the press that set it, and an
            // anchor the graph no longer draws leaves the range to measure from the row being read.
            const anchorRow = page.chosenAnchorOid === "" ? -1 : graphModel.rowOf(page.chosenAnchorOid)
            page.chooseRange(anchorRow >= 0 ? anchorRow : page.selectedRow, atRow)
            return
        }
        // A press on a row of the graph is where the next Shift click measures from — the one press that moves the
        // anchor without moving what is read.
        page.chosenAnchorOid = oidHex
        page.chooseAlso(oidHex)
    }
    /// Puts a commit into the choice, or takes it back out — a Ctrl click. The anchor is left where it was: which
    /// presses move it is the callers' to say.
    ///
    /// **One always stays in.** An empty choice is the working tree's own state, and reaching it from a
    /// commit would leave the pane on the right describing something no row is drawn as holding.
    function chooseAlso(oidHex) {
        // A fresh object every time: the rows follow this property, and assigning the same one back changes nothing
        // for them to follow.
        const next = ({})
        for (const held in page.chosenOids)
            next[held] = true
        if (next[oidHex] === true) {
            if (page.chosenCount <= 1)
                return
            delete next[oidHex]
        } else {
            next[oidHex] = true
        }
        page.settleChoice(next)
    }
    /// The same Ctrl click, made in the list of what is held rather than in the graph (`ChosenCommitRow`). It takes a
    /// commit out and never puts one in — everything in that list is in the choice already.
    ///
    /// **The anchor stays where the graph left it**: it is a commit of the graph, and this press was not on one. A
    /// Shift click after this one measures from the last row a hand actually landed on, which is what it would
    /// measure from if the drop had been made in the graph.
    function dropFromChoice(oidHex) {
        page.chooseAlso(oidHex)
    }
    /// Reads the rows of the chosen commits again for the list on the right: what it says about them (a subject, a
    /// face) is read off the rows, so it is read again whenever the rows are.
    function refreshChosenRecords() {
        if (page.chosenIds.length > 0)
            page.chosenRecords = graphModel.chosenRows(page.chosenIds)
    }
    /// Every commit row between two places, ends included — what a Shift click reaches. **The ones scrolled past are
    /// in it too**: the anchor and the click are on screen by definition, and what lies between them usually is not.
    ///
    /// Asked of the model in one crossing (`GraphModel.oidsBetween`): a range is as long as the hand dragged it, and a
    /// call per row would put a walk of the history on a click (CLAUDE.md §性能予算). The working tree's row is dropped
    /// wherever a range sweeps over it.
    function chooseRange(from, to) {
        if (from < 0 || to < 0)
            return
        const next = ({})
        for (const oidHex of graphModel.oidsBetween(from, to)) {
            if (oidHex !== "" && !GitFacts.wipOid(oidHex))
                next[oidHex] = true
        }
        // A range that swept nothing but the working tree's row is not a choice; the anchor stays where it was.
        if (Object.keys(next).length === 0)
            return
        page.settleChoice(next)
    }
    /// The one place the set and its tally are written together, so a count and a highlight cannot disagree.
    ///
    /// **A choice that has come back down to one commit is that commit being read**, which is what every other way of
    /// landing on one row does (`activateRow`).
    function settleChoice(next) {
        const ids = Object.keys(next)
        if (ids.length === 1) {
            page.activateRow(ids[0])
            return
        }
        page.chosenOids = next
        page.chosenCount = ids.length
        page.readChoice(ids)
    }
    /// Puts the choice to the graph for its order and the rows that name it, and asks the pane on the right for what
    /// these commits hold. **Two are read as what differs between them, three or more as what all of them changed**
    /// (デザイン規約 §複数のコミットを選ぶ).
    function readChoice(ids) {
        page.chosenIds = graphModel.presentOids(ids)
        page.chosenRecords = page.chosenIds.length === 0 ? [] : graphModel.chosenRows(page.chosenIds)
        if (page.chosenIds.length === 0)
            return
        // None of what a single commit's pane says applies to several: no stash sits under a choice, and the diff
        // that was open was of one file of one commit.
        page.selectedStashRef = ""
        page.closeDiff()
        detailsModel.requestSelection(page.chosenIds, page.chosenCount === 2)
    }
    /// Drops from the choice whatever the graph no longer stands on. Run when a pass lands: a rewrite takes commits
    /// away, and a choice that goes on counting them says a number no row on screen adds up to.
    function settleChoiceAfterPass() {
        if (page.chosenCount <= 1)
            return
        const kept = graphModel.presentOids(Object.keys(page.chosenOids))
        if (kept.length === page.chosenCount) {
            // The same commits, drawn again. In the same order, what the list on the right says about them is read
            // again off the rows the pass brought (a subject reworded, a face assigned); in another order — a rewrite
            // moved one past another — the choice is read again whole, since which of two commits is the older is
            // what the pane compares from.
            if (kept.every((oidHex, at) => oidHex === page.chosenIds[at]))
                page.refreshChosenRecords()
            else
                page.readChoice(kept)
            return
        }
        // Everything the choice named is gone. What is being read answers for it — that one is followed by name
        // (`followVanishedCommit`), and the choice comes back as it.
        if (kept.length === 0) {
            page.chooseOnly(page.selectedOid)
            return
        }
        const next = ({})
        for (const oidHex of kept)
            next[oidHex] = true
        page.settleChoice(next)
    }

    // The commit the viewport is measured against between passes, so the rows a reader is on can be put back under them
    // when new ones arrive above. The WIP row is no use for that — it comes and goes with the working tree — so the
    // newest *real* commit carries the measurement.
    property string anchorOid: ""
    property int anchorRow: -1
    function rememberAnchor() {
        // **However many working-tree rows stand over it** — this window's own and one per other copy, all wearing
        // the same all-zero id, so a row skipped by that id alone leaves the anchor on another of them and the
        // measurement reads back whichever the walk left first (`GraphModel.newestCommitRow`, the one place the
        // rule is written).
        const row = graphModel.newestCommitRow()
        page.anchorOid = row < 0 ? "" : graphModel.oidAt(row)
        page.anchorRow = row
    }
    // How far the graph slid under the viewport. Zero when the anchor is gone: a rewrite deep in the history moves rows
    // by different amounts and there is no single answer, so the view is left alone.
    function anchorShift() {
        if (page.anchorRow < 0 || page.anchorOid === "")
            return 0
        const now = graphModel.rowOf(page.anchorOid)
        return now < 0 ? 0 : now - page.anchorRow
    }

    // The row the selection stood on is gone and this page owes it a landing. Held over: the status of the working
    // tree, the refs and the walk arrive as three separate messages, and the two that come first still describe the
    // repository as it was — reading the branch out of them lands on the commit that was just replaced. Resolved
    // once the graph holds where the branch points, which is only true of the refreshed pair.
    property bool pendingHeadSelect: false
    /// The first report of HEAD that may answer the landing (`WorkTreeModel.headSeq`). A write's answer names it
    /// (`RepoTab.writeAnswerHeadSeq`): the report in hand at that arming may still describe the repository as it
    /// was, with a stale HEAD that has a row to land on, and only a report numbered at or above the answer's looked
    /// after the write. An arming read out of a report already in hand (the working tree emptying, the selected
    /// commit vanishing) takes that very report.
    property int pendingHeadFromSeq: 0
    // Whether the landing is one the person here asked for, in which case the viewport goes to it as well: a commit
    // they meant to make is not an answer if it lands off screen. The other two ways this is set happen *to* the window
    // — a commit made in a terminal, a rewrite that swept the selected commit away while the poll was watching — and a
    // background pass that moves rows under a reader may not also move their view
    // (§ListView.highlightFollowsCurrentItem).
    property bool pendingHeadAsked: false
    /// Whether the working tree the last status described has nothing left in it — **a tree nobody has read has yet
    /// to answer**, so every reader of this stands behind `workTree.loaded` (the counts start at zero,
    /// which is this question's own picture of a job that is done. 規約 §UI 自動化の因果性).
    ///
    /// A property because two sides read it: the status that finds the tree empty, and the write answer that may be
    /// the half completing a stash's pair (`leaveWipWhenDone`).
    readonly property bool treeClean: workTree.stagedCount === 0 && workTree.unstagedCount === 0
                                   && workTree.untrackedCount === 0 && workTree.conflictCount === 0
    /// What operation the last status named, so that its going away can be read as an edge.
    property string seenOpText: ""
    /// The WIP face has nothing left to hold the reader with. The working tree emptied: after a commit of our own that
    /// is the end of the editor's job; when someone else committed these changes it happens with no warning, so a
    /// message being written stays on screen with its text — it is the one thing here that cannot be read back off
    /// disk. Or the operation went away, which is what takes the exit card off the face. Either way, land on the
    /// commit that now holds the changes.
    ///
    /// **A stash pressed here lands past the message.** The press *is* the decision to empty the tree, so the box
    /// is not a reason to stay: the words are still in it when the working tree comes back (the pane is hidden, not
    /// unloaded) and the entry took them for its own name on the way out. Left to the message alone, the one press that
    /// always has words in front of it — a stopped merge fills the box itself (`absorbOpMessage`) — is the one that
    /// never lands, and the pane stands over a tree it no longer describes while the highlight the working-tree row
    /// left behind is inherited by whatever slid into its place, which after this press is the entry it just made.
    ///
    /// **Every half is read off one status** (`WorkTreeModel`), and each of the two reasons is a way out that was
    /// missing. The list says `changed` only when its rows differ, so a stop that ends on a clean tree — an `edit`
    /// stop put down by `--continue`, `--skip`, `--abort` or a terminal — moves no row and would never ask this
    /// question at all. And the list is drained before the headline is (`hub::sink` pushes the nav runs first), so a
    /// question asked from there reads an `opText` one status old and hears the operation that has just gone as
    /// though it were still standing — which is every ordinary conflict landing, where the rows empty and the
    /// operation ends in the same status.
    ///
    /// `edge` is what that status moved: the tree's own counts, or the operation. Standing alone, the condition would
    /// walk a reader off the face on a poll that changed nothing under them — the message box emptied by hand is the
    /// one that would do it.
    function leaveWipWhenDone(edge) {
        // **A tree nobody has read has yet to answer.** The counts start at zero and `opText` starts
        // empty, which is this question's own picture of a job that is done — so asked before the first status it
        // answers yes about a repository it has never seen (規約 §UI 自動化の因果性). `loaded` latches on that first
        // status and never goes back (`WorkTreeModel`), so this stands in front of a page's opening moment alone.
        if (!workTree.loaded)
            return
        // **A pane about another working copy is that copy's face.** What is on screen is that copy's changes, and
        // they are still there whatever this tree has just finished — so nobody is walked off it. The landing below
        // is left armed: the moment it is about is the reader coming back to their own row.
        if (!page.wipWritable)
            return
        // Whether this window's own stash is what emptied it — asked of the press that made it, which is the only
        // thing that still knows (`ops::StashOut`). **Asked from both sides of the pair**, because the status that
        // finds the tree empty and the answer that says whose it was arrive in no fixed order; whichever comes
        // second is the one this answers on, and the side that comes first asks and gets nothing.
        const ourStash = repoTab.takeStashLanding()
        if (!page.treeClean)
            return
        // **A landing of our own is an edge in itself.** `edge` keeps a poll that changed nothing under the reader
        // from walking them off the face — the message box emptied by hand is the one that would do it — and here
        // the press itself is what moved.
        if ((!edge && !ourStash) || !page.wipShown || workTree.opText !== "")
            return
        if (!ourStash && (wipPane.subjectText !== "" || wipPane.bodyText !== ""))
            return
        page.wipShown = false
        page.pendingHeadSelect = true
        // The status this is read out of **is** the one that answers: the report of HEAD that came with it is already
        // in hand (`hub::sink` sends it ahead of the status), and that is the commit the changes went into.
        page.pendingHeadFromSeq = workTree.headSeq
        // Only ours is a landing anybody asked for, so only ours takes the viewport along.
        page.pendingHeadAsked = ourStash
    }
    function tryPendingHeadSelect() {
        if (!page.pendingHeadSelect || !workTree.headKnown)
            return
        // Where HEAD stands is the one record's, branch or not (`WorkTreeModel.headOid`), so a write that left it
        // detached lands the same as any other. Only a report counted after the arming may answer
        // (`pendingHeadFromSeq`): the one in flight at a write's answer still describes the repository as it was,
        // and its stale HEAD has a row.
        if (workTree.headSeq < page.pendingHeadFromSeq)
            return
        const row = workTree.headOid !== "" ? graphModel.rowOf(workTree.headOid) : -1
        if (row < 0)
            return
        page.pendingHeadSelect = false
        const asked = page.pendingHeadAsked
        page.pendingHeadAsked = false
        graphPane.setCurrentRow(row)
        page.activateRow(graphModel.oidAt(row))
        // Held back by an unsaved message: the question put the highlight back where it was, so there is nowhere for
        // the view to go yet.
        if (asked)
            graphPane.showRowSoon(row)
    }

    // An operation stopped part-way and this page owes it a landing on the working tree, where the conflicts and the
    // way out are. Held for the same reason `pendingHeadSelect` is: git has already written the markers by the time it
    // answers, but the status carrying those rows arrives afterwards, and the row does not exist until it does. Always
    // asked for — a stop only ever follows a press — so the viewport goes along.
    property bool pendingWipSelect: false
    function tryPendingWipSelect() {
        if (!page.pendingWipSelect)
            return
        // Row 0 is where the working tree stands once the walk has prepended it, which it does only after the status
        // has said there is something to commit; until then row 0 is still the commit that was on top, or a
        // neighbour copy's row wearing the same all-zero id. Landing on either would take the press to the wrong
        // place entirely, so the question is the graph's own word (`GraphModel.wipRow`, the one place ours is told
        // from theirs).
        if (!graphModel.wipRow)
            return
        page.pendingWipSelect = false
        graphPane.setCurrentRow(0)
        page.showWip()
        graphPane.showRowSoon(0)
    }

    // The selected commit is gone from the graph and this page did not rewrite it: an amend or a rebase run in a
    // terminal replaced it while the poll was watching. Whatever now stands where it stood is the closest thing to what
    // was being read; failing that, fall back to the branch's own commit, which is never nothing.
    function followVanishedCommit() {
        const oidHex = graphModel.oidAt(page.selectedRow)
        if (oidHex !== "" && !GitFacts.wipOid(oidHex)) {
            graphPane.setCurrentRow(page.selectedRow)
            page.activateRow(oidHex)
            return
        }
        page.selectedOid = ""
        page.pendingHeadSelect = true
        // The graph that let the commit go was rebuilt behind a read that had already reported where HEAD went, so
        // the report in hand is the one to land on.
        page.pendingHeadFromSeq = workTree.headSeq
    }

    // A reworded commit came back under a different hash: the one now standing where it stood is it, since only the
    // message changed.
    function followRewrittenCommit() {
        const row = page.rewordRow
        const oidHex = graphModel.oidAt(row)
        // Nothing there, or the working-tree row moved under it.
        if (oidHex === "" || GitFacts.wipOid(oidHex)) {
            page.rewordRow = -1
            return
        }
        graphPane.setCurrentRow(row)
        page.activateRow(oidHex)
    }

    // What a row click means: the synthetic WIP row (all-zero id) opens the working-tree view, anything else selects
    // the commit.
    /// `atRow` is where the row is, for the callers that already know (a click knows: it came from that row). **The
    /// lookup is a walk over every loaded row** (`GraphModel::row_of`), so a path that has the number and asks for it
    /// again puts a walk of the whole history on a click (CLAUDE.md §性能予算「同期処理はコミット数・refs の本数から独立」).
    /// -1 asks for the lookup, which is what a landing named only by its commit has to do.
    function activateRow(oidHex, atRow) {
        // Any new selection settles where the last rewrite left off, and answers any landing this page still owed.
        page.rewordRow = -1
        page.pendingHeadSelect = false
        page.pendingHeadAsked = false
        page.pendingWipSelect = false
        // Clicking anywhere is the way out of the name box and of a standing row question: both are offers the
        // reader may leave.
        graphPane.stopNaming()
        page.stopRowAsk()
        // And the right pane's mark is about the commit it was raised over. Every road to another commit comes through
        // here, keyboard ones included, so this is where it stops being true — the one road that keeps it raises it
        // again on the way out (`onMessageAsked`).
        detailsPane.dropAttention()
        // **A plain landing is one commit**, whatever was being held before it (デザイン規約 §複数のコミットを選ぶ).
        // Whether it was several is carried past the early return below: a click on the commit already open has
        // nothing left to ask git for *unless* the pane is showing a choice, and then everything below has to run.
        const wasChoice = page.chosenCount > 1
        page.chooseOnly(oidHex)
        const row = atRow !== undefined && atRow >= 0 ? atRow : graphModel.rowOf(oidHex)
        if (row >= 0) {
            graphPane.setCurrentRow(row)
            page.selectedRow = row
            page.chosenAnchorOid = oidHex
        }
        // **The working tree's row is not a hash**, so nothing below can be skipped for it the way it can for a
        // commit: what it shows is whatever the tree is now. Which tree is the row's own answer — several copies can
        // have a row here and they all wear the same all-zero id (`openWipFor`).
        if (GitFacts.wipOid(oidHex)) {
            page.openWipFor(row)
            return
        }
        // A commit is nobody's working copy, so the pane stops being about one.
        page.dropCarried()
        // The commit already open. **Everything below is of this commit and has been asked once**: the details and the
        // signature are answers to a hash, and a hash cannot have changed under the same row — asking again spends a
        // `git show` and a gpg run per click for an answer already on screen (the find bar says the same of landing
        // twice on one row). The second click of the rename gesture is exactly this click, so the wait it opens would
        // be spent on work nobody is waiting for (デザイン規約 §グラフ行のダブルクリック).
        if (!page.wipShown && !wasChoice && page.selectedOid === oidHex)
            return
        page.wipShown = false
        page.selectedOid = oidHex
        page.selectedStashRef = graphModel.stashRefOf(oidHex)
        detailsModel.request(oidHex)
        page.askSignature(oidHex)
        page.closeDiff()
    }

    /// Something other than the reader is picking the row this page stands on, so the default below stays out of its
    /// way — it would land first and be photographed instead (`PageAutoStart`). Written from outside and false
    /// wherever nobody wrote it, which is every window a person opens.
    property bool rowPickedElsewhere: false

    // Selection policy: restore across the tag-swap reset, and open on the working tree's own row where the tree has
    // one and on the commit HEAD stands on where it does not, so the right pane always shows something.
    function trySelectDefault() {
        if (page.selectedOid !== "" || page.wipShown || page.pendingHeadSelect || page.rowPickedElsewhere
                || graphModel.rowTotal === 0)
            return
        // Where HEAD stands is the commit to open on, and whether the tree has a row of its own decides whether a
        // commit is the landing at all — so both reads have to have answered. **Before the first status
        // `wipRowStands` is false because nothing has been asked yet** (`WorkTreeModel.loaded`), and a graph pass that
        // beat it would land on a commit and never come back: the first line above holds every later call off.
        if (!workTree.headKnown || !workTree.loaded)
            return
        let row = -1
        if (workTree.wipRowStands) {
            // **Uncommitted work is the landing, branch or no branch** (デザイン規約 §未コミット行が名乗るもの):
            // the row stands while the tree is dirty or an operation is (`graph::wip_row_stands`), and it is what
            // the reader came back to. Asked of the status, because whether the opening walk
            // carries that row is a race between the first status and the walk's own start (`session::walk`) — the
            // two disagreeing is the graph saying it is one read behind, and until the pass that knows lands, row 0
            // is still the commit that was on top (`tryPendingWipSelect` waits on the same word). Every pass calls
            // this again.
            //
            // **Asked of the graph's own word** (`GraphModel.wipRow`): every working copy's row wears the same
            // all-zero id, so the id answers yes for a pass that put a neighbour's row first and left ours out —
            // and landing there opens the pane on a copy nobody asked for and stands the page on it (`openWipFor`).
            // Which of the two a pass leaves first is the walk's to decide.
            if (!graphModel.wipRow)
                return
            row = 0
        } else {
            // **Where HEAD stands, branch or not** (`WorkTreeModel.headOid`, the record's own report — the same one
            // a write's landing is paid off, `tryPendingHeadSelect`). A detached HEAD is still a commit somebody is
            // standing on, and that is the one to open on.
            row = workTree.headOid !== "" ? graphModel.rowOf(workTree.headOid) : -1
            if (row < 0) {
                if (graphModel.loading)
                    return // the head row may still be streaming in
                // HEAD outside the window: the newest commit, which is not the newest row where stashes stand over
                // it (`GraphModel.newestCommitRow` — the rule, and the two automation drivers that ask it too). A
                // repository whose rows are all stashes has no commit to open on.
                row = graphModel.newestCommitRow()
                if (row < 0)
                    return
            }
        }
        graphPane.setCurrentRow(row)
        graphPane.anchorSoon()
        page.activateRow(graphModel.oidAt(row), row)
    }
    // Each finished pass bumps finishCount: re-resolve the selection by oid, since row numbers may have shifted. Only a
    // streaming restart bumps resetCount — that is the only case where the viewport lost its scroll position and needs
    // re-anchoring. In-place replacements keep the position, and re-centering would yank the view around.
    property int seenFinishCount: 0
    property int seenResetCount: 0
    Connections {
        target: graphModel
        function onStatsChanged() {
            if (graphModel.finishCount !== page.seenFinishCount) {
                page.seenFinishCount = graphModel.finishCount
                const resetHappened = graphModel.resetCount !== page.seenResetCount
                page.seenResetCount = graphModel.resetCount
                // A reset starts the viewport over anyway; only in-place replacements leave it pointing at rows that
                // moved.
                if (!resetHappened)
                    graphPane.shiftRows(page.anchorShift())
                page.rememberAnchor()
                // A stopped operation lands on the working tree's own row, which this pass is what puts there.
                page.tryPendingWipSelect()
                // Another copy's row moves under a rebuild the way a commit's does, and goes away when that copy
                // commits — neither of which its all-zero id can say (`settleCarriedAfterPass`).
                page.settleCarriedAfterPass()
                if (page.pendingHeadSelect) {
                    page.tryPendingHeadSelect()
                } else if (page.selectedOid !== "") {
                    const row = graphModel.rowOf(page.selectedOid)
                    if (row >= 0) {
                        page.selectedRow = row
                        graphPane.setCurrentRow(row)
                        if (resetHappened)
                            graphPane.anchorSoon()
                    } else if (page.pendingWipSelect) {
                        // **A landing the reader asked for outranks the two below.** A stopped operation owes them
                        // the working tree's row (規約 §進行中の操作から出る) and this pass may simply not have that
                        // row yet; landing anywhere else now would take the owed one away, because every landing
                        // clears it (`activateRow`).
                    } else if (page.rewordRow >= 0) {
                        page.followRewrittenCommit()
                    } else if (page.selectedRow >= 0) {
                        // A selection that was never on screen — a details jump past the walk window — has not
                        // vanished: the commit is still there, only unwalked. Leave the reader on it.
                        page.followVanishedCommit()
                    }
                }
                // After the one being read has been placed: the two follows above settle a choice of their own, and
                // this drops whatever else the pass took away (`settleChoiceAfterPass`).
                page.settleChoiceAfterPass()
            }
            page.trySelectDefault()
        }
    }
    // Where HEAD stands rides the working-tree model — the record's own report, and the landings and the default
    // selection are paid there (`workTree.onChanged`).
    Connections {
        target: branchesModel
        function onRefsSettled() {
            // The listing is the half a move onto a branch that was not there waits on: the status behind the write
            // already has HEAD on it, and until this arrives the screen still says no such branch (`moveLanding`).
            page.absorbMoveLanding()
            page.listingDrawn()
        }
    }
    /// A list has drawn what it was handed, and a delete may have been waiting on that list's own row — so **every
    /// one of them says it**. The three refs sections are handed one snapshot and draw it on three separate turns
    /// (`ops::Applied`), and the stash has a listing of its own; the row each answers for is its own, so **which
    /// list this came from stays here** — passed on, it could only be passed on wrongly. Said, because what a
    /// listing proves is the delete's owner to work out
    /// (`ops_delete::note_listing_drawn`). Withheld while a run is holding the in-between open for a picture
    /// (`holdGoneRows`).
    function listingDrawn() {
        if (!page.holdGoneRows)
            repoTab.listingDrawn()
    }
    Connections {
        target: remotesModel
        function onRefsSettled() {
            page.listingDrawn()
        }
    }
    Connections {
        target: tagsModel
        function onRefsSettled() {
            page.listingDrawn()
        }
    }
    Connections {
        target: diffModel
        // The rows are about to be swapped for a re-read of the same file — a write of ours, or an edit made outside
        // this window that the tick caught. Held here, because this is the moment the view is still standing where
        // the reader left it, and the two askers cannot both be trusted to say so: the tick asks on files that turn
        // out not to have moved, and holding a place for a swap that never comes would put the next one back
        // somewhere the reader has since left (`DiffScrollPlace`).
        function onRowsReplacing() {
            diffPane.holdScroll()
        }
    }
    // The stashes arrive on a word of their own, later than the refs — read off the refs' arrival, a dropped stash
    // would be back on screen for the whole of the graph rebuild that sits between the two (`session::write`).
    Connections {
        target: stashesModel
        function onStashesSettled() {
            page.listingDrawn()
        }
    }
    /// The `WorkTreeModel.treeRevision` the open diff was last read against — the counts of the four buckets are what
    /// a stage or an unstage moves whoever made it, and the model bumps the revision when they do.
    property int seenTreeRev: -1
    // **The tree was read.** Said by the working-tree model: the list says `changed` only when its rows differ, and
    // a status that moved no row is exactly the one this has to hear about (a second line staged out of a file
    // already on both sides moves nothing).
    Connections {
        target: workTree
        function onChanged() {
            const moved = workTree.treeRevision !== page.seenTreeRev
            page.seenTreeRev = workTree.treeRevision
            // The operation that was standing is not standing any more — the other edge the WIP face's exit turns on,
            // and the only one a clean stop ever moves (`leaveWipWhenDone`).
            const opGone = page.seenOpText !== "" && workTree.opText === ""
            page.seenOpText = workTree.opText
            // **Written down before anything asks what it means.** A write's answer and the status it published are
            // drained apart, so this may be the half that arrives first — and a status the presses waiting on one
            // never hear about is a tree emptying, or a file moving, that nothing will account for.
            if (workTree.loaded)
                repoTab.noteTreeRead(workTree.statusSeq, page.treeClean)
            // The status that follows this window's own write: the file was read when the write answered, and this
            // report is the one that write published. Measured against the number the answer named for it
            // (`ops::DiffReread`): a count of answers moves for another write answering in between without saying
            // which tree this status describes, and a read already in flight when the write ended arrives with a
            // number from before it.
            const ours = repoTab.takeDiffRead()
            // Something outside this window moved the tree, so the rows on screen — and the fingerprint the next `+`
            // would be written against — are a picture of the file as it was. Pressing one then came back with git's
            // refusal.
            if (moved && !ours)
                page.reloadDiff()
            page.absorbOpMessage()
            // ...and the other half of a move's landing: this is what carries the branch HEAD ended up on.
            page.absorbMoveLanding()
            // Whether the WIP face still has anything to stand for. **After `absorbOpMessage`**: the words a stopped
            // merge put in the box are that operation's, and an abort takes them back out — asked before it, the box it
            // has yet to empty reads as a message somebody is writing.
            page.leaveWipWhenDone(moved || opGone)
            // The report of where HEAD stands rides this model (`WorkTreeModel.headSeq`), so this is where a landing
            // owed to a write is paid — only a report counted after the arming answers (`tryPendingHeadSelect`), and
            // the first read after a write always sends one, moved or not, so a write that recorded nothing (a
            // cherry-pick of a commit the branch already has) pays the same landing as one that wrote a commit.
            page.tryPendingHeadSelect()
            page.trySelectDefault()
        }
    }
    Connections {
        // Whichever trio the pane is showing: the file the diff is on belongs to the copy those rows came from, and
        // so does the side the pane would move to when it runs out (`followEmptySide`).
        target: page.wipUnstaged
        function onChanged() {
            // The file the diff is on is still where it was, so this is the last moment its neighbour can be read (see
            // `noteDiffNeighbour`) — and the change that takes it off the side being read is the one that moves the
            // pane, with the neighbour noted by every change before it (`followEmptySide`).
            page.noteDiffNeighbour()
            page.followEmptySide()
            // **The face's own exit is the status headline's** — asked one snapshot at a time
            // (`leaveWipWhenDone`), because half the ways out of it move no row in this list at all.
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // The panes sit above the command log, which is closed until it is asked for. Splitting them vertically keeps
        // the log's height in the reader's hands and out of the panes' business.
        SplitView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            orientation: Qt.Vertical
            handle: SplitHandleBar {
                onHandChanged: (which, held) => page.holdSplitBar(which, held)
            }

            // ---- open failed ------------------------------------------
            OpenFailedScreen {
                id: failedScreen
                visible: page.openFailed
                SplitView.fillHeight: true
                SplitView.minimumHeight: pageLayout.panesMinHeight
                kind: repoTab.errorKind
                path: repoTab.errorPath
                message: repoTab.error
                pageWidth: page.width
                commandsPage: page
                onCloseRequested: page.closeTabRequested()
            }

            // ---- three-pane layout --------------------------------------
            // (repository state / search / fetch / push live in the window toolbar, next to the tabs; Reload is the app
            // menu and F5)
            SplitView {
                visible: !page.openFailed
                SplitView.fillHeight: true
                SplitView.minimumHeight: pageLayout.panesMinHeight
                orientation: Qt.Horizontal
                handle: SplitHandleBar {
                    onHandChanged: (which, held) => page.holdSplitBar(which, held)
                }

                SidebarPane {
                    id: sidebarPane
                    // The plan's freeze takes the pane whole and is meant to be read as the mode's own restriction;
                    // a replay running behind the screen holds only the doors — a switch, a delete, a name box, the
                    // `+` — and everything the pane is *read* with goes on working through it. Two states, two
                    // answers (`SidebarPane.frozen` / `doorsHeld`).
                    frozen: page.sidebarFrozen
                    doorsHeld: page.doorsHeldWhy !== ""
                    // Over the pane beside it while a name box is standing: the box reaches past this pane's edge when
                    // what is in it does not fit, and the graph is laid out after this one (`NavItemDelegate`). Only
                    // then — a pane that sat over its neighbour the rest of the time would draw its own edge over the
                    // splitter.
                    z: sidebarPane.editKey !== "" ? 1 : 0
                    repoTab: repoTab
                    workTree: workTree
                    branchesModel: branchesModel
                    remotesModel: remotesModel
                    worktreesModel: worktreesModel
                    stashesModel: stashesModel
                    tagsModel: tagsModel
                    collapsed: page.sidebarCollapsed
                    // Null on the blank page: nothing has run there (the band's `>_` answered the same way up above).
                    commandsPage: page.blank ? null : page
                    // The menus the rows raise are the page's, so only the page can say one is standing over the folded
                    // list.
                    menuOpen: page.menuStanding
                    onFoldRequested: collapse => page.foldByHand(collapse)
                    onRefActivated: oidHex => page.jumpToRef(oidHex)
                    onRefMenuRequested: (kind, name, full, oidHex) => page.openRefMenu(kind, name, full, oidHex, true)
                    onRemoteMenuRequested: name => page.openRemoteMenu(name)
                    onWorktreeActivated: path => page.openRepositoryPathRequested(path)
                    onRefSwitchRequested: (kind, name) => page.switchToRef(kind, name)
                    onBranchAtRequested: (oidHex, name) => {
                        if (name !== "")
                            repoTab.createBranch(name, oidHex, true)
                    }
                    onTagAtRequested: (oidHex, name) => {
                        if (name !== "")
                            repoTab.createTag(name, oidHex)
                    }
                    onRenameSubmitted: (kind, id, name) => page.renameRow(kind, id, name)
                    onAddRemoteRequested: publishFlow.startAddRemote()
                }

                // Center: a report over the whole of it, and under that the commit graph ⇄ file diff.
                //
                // **The report is above the pair**: it answers a write, and a write is answered wherever the reader
                // happens to be standing — a push refused while a diff is open has the same news to give
                // (デザイン規約 §答えの要らない報せ). Put in one of them it would be silent in the other, and put over them
                // it would cover what it is about.
                //
                // **So the middle steps down for it.** The bar takes its own row of the column and the pair takes what
                // is left, which is the same thing the graph did for it when the bar lived inside that pane: nothing is
                // covered, and a closed bar has no height to give.
                ColumnLayout {
                    SplitView.fillWidth: true
                    // Read off the graph: what its own columns come to once they have both given everything they
                    // can, held up to a side pane's width so the middle never reads as the thinnest of the three
                    // (`PageLayout.centreMinWidth`).
                    SplitView.minimumWidth: pageLayout.centreMinWidth
                    spacing: 0

                    NoticeBar {
                        id: noticeBar
                        Layout.fillWidth: true
                        // **The order Escape is handed out in, and the one place it is written.** Both bars can be
                        // standing at once — raising a question does not lower a report (`startRowAsk`), and a write
                        // answered while one stands does not lower the question — and **two enabled
                        // `StandardKey.Cancel` shortcuts in one window fire neither** (`tests/qml/tst_escape.qml`),
                        // so one of them gives way.
                        //
                        // **The question keeps it**: it is what is being asked of the reader, it holds the keyboard
                        // (its pill takes the focus as it opens), and it is the one thing here that stands until it
                        // is answered — a report is read and nothing follows from it. Under both is the arrival mark,
                        // last because it takes Escape as a key handler
                        // (`page.escapePressed`).
                        //
                        // Written as this bar's own line: what each bar does with Escape stays in its own
                        // declaration, and only the order between them is here.
                        yieldsEscape: graphPane.asking
                        onAcknowledged: page.hideNotice()
                    }

                    StackLayout {
                        id: centreStack
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        // The plan stands over both from the press that asked for it — before its rows exist, which is
                        // the whole of `planShown` (a diff opened earlier closes on the way in, `onPlanShownChanged`),
                        // and the graph comes back exactly as it was when the plan is put away.
                        currentIndex: page.planShown ? 2 : page.diffShown ? 1 : 0

                        GraphPane {
                            id: graphPane
                            graphModel: graphModel
                            workTree: workTree
                            blank: page.blank
                            chipListAnchor: rowHost.refListAnchor
                            rowCardOid: rowHost.rowCardOid
                            chosenOids: page.chosenOids
                            chosenCount: page.chosenCount
                            onRowActivated: (oidHex, atRow, modifiers) => page.pickRow(oidHex, atRow, modifiers)
                            // The bar moves between matches — landing on the same row twice changes nothing and
                            // costs no git.
                            onFindLanded: oidHex => {
                                if (oidHex !== "" && oidHex !== page.selectedOid)
                                    page.activateRow(oidHex)
                            }
                            onRowMenuOpenRequested: (oidHex, chip) => page.openRowMenu(oidHex, chip)
                            onRowSwitchRequested: (oidHex, chip) => page.rowDoubleClicked(oidHex, chip)
                            // The same door the WORKTREES row opens: the copy's own tab, where its changes are read
                            // and staged.
                            onCarriedOpenRequested: path => page.openRepositoryPathRequested(path)
                            onRowRenameRequested: (oidHex, chip) => page.startRename(oidHex, chip)
                            onRenameSubmitted: (kind, id, name) => page.renameRow(kind, id, name)
                            namingRefused: page.graphNameRefusedWhy !== ""
                            namingRefusedWhy: page.graphNameRefusedWhy
                            onChipExpandRequested: (oidHex, atRow, records, anchor) =>
                                rowHost.openRefList(oidHex, atRow, records, anchor)
                            onChipCollapseRequested: rowHost.closeRefListUnlessEntered()
                            onRowHoverRequested: (row, inside) => page.restOnCommit(row, inside)
                            onCreateBranchRequested: (oidHex, name) => repoTab.createBranch(name, oidHex, true)
                            // Nothing moves: a tag is left on the commit and the tree stays where it is.
                            onCreateTagRequested: (oidHex, name) => repoTab.createTag(name, oidHex)
                            onOpenRepositoryRequested: page.openRepositoryPicker()
                            onAskConfirmed: page.answerRowAsk()
                            onAskCancelled: page.stopRowAsk()
                        }

                        DiffPane {
                            id: diffPane
                            diffModel: diffModel
                            fromWorkTree: page.diffFromWt
                            staged: page.diffStaged
                            conflicted: page.diffKind === "conflicts"
                            conflictChange: page.diffChange
                            // Read where the file is, and staged only where this window is the one holding it.
                            writable: page.wipWritable
                            copyName: page.carriedName
                            // The two swap over during a rebase; the model is where that is already answered. What
                            // *this* window is in the middle of, so a copy's conflicted file falls back to git's own
                            // two words the way its rows do (`WipBucketPane`).
                            sideOurs: page.wipWritable ? workTree.sideOurs : ""
                            sideTheirs: page.wipWritable ? workTree.sideTheirs : ""
                            sideColorOurs: page.sideColorOurs
                            sideColorTheirs: page.sideColorTheirs
                            busy: page.diffSettling
                            menuStanding: page.menuStanding
                            onCloseRequested: page.closeDiffToGraph()
                            onCopyRequested: text => clipboard.copy(text)
                            onCodeMenuRequested: page.openCodeMenu()
                            onDiscardHunkRequested: hunk => page.discardHunkNow(hunk)
                            onStageFileRequested: {
                                if (page.diffStaged)
                                    repoTab.unstagePath(page.diffPath)
                                else
                                    repoTab.stagePath(page.diffPath)
                            }
                            onStageSelectionRequested: (hunk, line) => page.stageSelection(hunk, line)
                            onSplitChosen: split => page.setDiffSplit(split)
                        }

                        // The plan's face is built when it takes the seat and taken down with it: it is a whole pane
                        // of rows, a band and a menu that a page pays for otherwise, and everything it shows is the
                        // model's, so nothing is lost in the taking down (rules-refs/app-ui.md — the dialog-seat
                        // rule). The seat stands in the stack whether or not it holds a face, so the index above
                        // still names it.
                        Loader {
                            id: planSeat
                            active: page.planShown
                            sourceComponent: RebasePlanPane {
                                planModel: planModel
                                selectedOid: page.selectedOid
                                // The pane has nothing of its own yet — or the run is holding the face it had then.
                                // A read that *replaces* a standing plan is not this: those rows still answer a
                                // question somebody asked, and the pane keeps them until the newer answer lands,
                                // which is the model's own rule (`RebasePlanModel::open`).
                                waiting: (planModel.loading && !planModel.active) || page.planLoadHeld
                                discards: page.planDiscards
                                onRowPicked: oidHex => page.activateRow(oidHex)
                            }
                        }
                    }
                }

                // Right side: working tree ⇄ commit details
                Rectangle {
                    id: rightPane
                    // Where it starts; `applySavedLayout` assigns over this with the width the window is set to.
                    SplitView.preferredWidth: 400
                    SplitView.minimumWidth: pageLayout.rightMinWidth
                    color: Theme.bgSurface

                    GitVersionCorner {
                        id: gitCorner
                        // The run button takes the pane's foot while a plan stands, and the foot it takes is this
                        // corner's seat (§コミットメッセージの 2 つの枠「ペインの底は空かない」). Said through the
                        // corner's own property — a `visible` here replaces the one it draws itself by.
                        offered: !page.planActive
                        // Only one of the three panes is on screen at a time, and each measures its own file list.
                        // **The carried copy's is asked for its own room**: the tree's pane never lends the corner
                        // (its foot is the commit button's), and a carried pane read through that answer left every
                        // reader of another copy's work with no version at
                        // all.
                        roomLeft: !page.wipShown ? detailsPane.bottomRoom
                                : page.wipWritable ? wipPane.bottomRoom : carriedPane.bottomRoom
                        anchors.right: parent.right
                        anchors.bottom: parent.bottom
                        anchors.rightMargin: Theme.spaceXs
                        anchors.bottomMargin: Theme.spaceXs
                        // Declared before the panes it hangs over, so z holds it in front of whatever they draw here.
                        z: 1
                    }

                    // Another working copy's changes are a pane of their own (デザイン規約 §別の作業コピーを読む): what a
                    // reader can do with them is read them, which is what the commit pane is already shaped
                    // for.
                    CarriedPane {
                        id: carriedPane
                        anchors.fill: parent
                        visible: page.wipShown && !page.wipWritable
                        copyName: page.carriedName
                        files: carriedModel
                        readBucket: page.diffFromWt ? page.diffKind : ""
                        readPath: page.diffFromWt ? page.diffPath : ""
                        menuStanding: page.menuStanding
                        onTreeViewChosen: tree => page.setWipTreeView(tree)
                        onFileActivated: (bucket, path, origPath) => page.toggleDiff(bucket, path, origPath)
                        onFileWalked: (bucket, path, origPath) => page.openDiff(bucket, path, origPath)
                    }

                    WipPane {
                        id: wipPane
                        anchors.fill: parent
                        visible: page.wipShown && page.wipWritable
                        repoTab: repoTab
                        workTree: workTree
                        worktreeModel: worktreeModel
                        conflictsModel: conflictsModel
                        stagedModel: stagedModel
                        onTreeViewChosen: tree => page.setWipTreeView(tree)
                        amending: page.amending
                        headPublished: page.headPublished
                        menuStanding: page.menuStanding
                        onAmendToggled: on => page.amendToggled(on)
                        onCommitClicked: page.commitNow()
                        readBucket: page.diffFromWt ? page.diffKind : ""
                        readPath: page.diffFromWt ? page.diffPath : ""
                        onFileActivated: (bucket, path, origPath) => page.toggleDiff(bucket, path, origPath)
                        onFileWalked: (bucket, path, origPath) => page.openDiff(bucket, path, origPath)
                        onFileMenuRequested: (bucket, path) => page.openFileMenu(bucket, path)
                    }

                    DetailsPane {
                        id: detailsPane
                        anchors.fill: parent
                        // The run button's seat is carved off the pane's foot while a plan stands — the pane's own
                        // press-things-here edge moves up with it (§コミットメッセージの 2 つの枠「押す物はペインの底」).
                        anchors.bottomMargin: planRunBar.visible ? planRunBar.height : 0
                        visible: !page.wipShown
                        details: detailsModel
                        chosenCommits: page.chosenRecords
                        rowCardOid: rowHost.rowCardOid
                        onRowHoverRequested: (row, inside) => page.restOnCommit(row, inside)
                        onCommitDropRequested: oidHex => page.dropFromChoice(oidHex)
                        stashRef: page.selectedStashRef
                        menuStanding: page.menuStanding
                        // The one commit an amend reaches, and the line the box gives when this is not it
                        // (`page.messageEdit`). `not-head` is the silent one: it names a row the reader can see is
                        // not the top of the history, so the box stops taking typing and says nothing. The other
                        // words are the page's; the rule is core's — and while a plan row carries `reword`, the
                        // same boxes are that row's plan input, because there is one place in this app to type a
                        // message. A plain amend is held down for the plan's whole stay — it is a queued rewrite
                        // of the very history the plan is composed on, which the freeze exists to stop; on a plan
                        // row the way to type is the row's own verb.
                        editable: (page.messageEdit === "amend" && !page.planActive) || page.planReword
                        intoPlan: page.planReword
                        planDraftOid: page.planActive ? planModel.selectedOid : ""
                        planDraftSubject: planModel.selectedMsgSubject
                        planDraftBody: planModel.selectedMsgBody
                        editBlocked: page.planActive && !page.planReword && page.messageEdit !== ""
                            ? qsTr("Mark the row reword to retype its message")
                            : page.messageEdit === "stash"
                              ? qsTr("Rename it in the list on the left")
                              : page.messageEdit === "standing"
                                ? qsTr("Finish the stopped operation first")
                                : ""
                        busy: repoTab.busyCount > 0
                        published: page.selectedPublished
                        signatureKind: page.selectedSignatureKind
                        signatureCode: page.selectedSignatureCode
                        signatureSigner: page.selectedSignatureSigner
                        // Whom a rewrite would be attributed to: git keeps the author and puts the reader in as
                        // committer, so the save button wears the reader's face.
                        committerFace: repoTab.authorAvatar
                        committerFaceUrl: repoTab.authorAvatarUrl
                        signsCommits: repoTab.signsCommits
                        signingTip: repoTab.signingFormat === "ssh"
                                    ? qsTr("Signed with your ssh key")
                                    : repoTab.signingFormat === "x509"
                                      ? qsTr("Signed with your x509 certificate")
                                      : qsTr("Signed with your gpg key")
                        readPath: page.diffKind === "commit" ? page.diffPath : ""
                        onMessageSubmitted: (oidHex, subject, body) => {
                            if (page.planReword) {
                                planModel.setMessage(planModel.selectedRow, subject, body)
                                // The model took it synchronously — the typed text is the
                                // resting text now (the write path hears this from
                                // `writeReworded` instead).
                                detailsPane.noteMessageSaved()
                            } else if (!page.planActive) {
                                page.saveMessage(oidHex, subject, body)
                            }
                        }
                        onFileActivated: (path, origPath) => page.toggleDiff("commit", path, origPath)
                        onFileWalked: (path, origPath) => page.openDiff("commit", path, origPath)
                        onParentClicked: oidHex => page.jumpToRef(oidHex)
                        // The badge's press goes to the window, which owns the settings card.
                        onAvatarEditRequested: (name, email) => page.avatarSettingsRequested(name, email)
                        onCopyRequested: text => clipboard.copy(text)
                        onApplyStashRequested: selector => repoTab.applyStash(selector)
                        onPopStashRequested: selector => page.popStash(selector)
                    }

                    RebasePlanRunBar {
                        id: planRunBar
                        // The plan holds the details face up (`onPlanActiveChanged`); the WIP guard is the belt —
                        // were that face ever up, this bar would sit over the commit button.
                        visible: page.planActive && !page.wipShown
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.bottom: parent.bottom
                        anchors.leftMargin: Theme.spaceMd
                        anchors.rightMargin: Theme.spaceMd
                        plan: planModel
                        pushedCount: page.planPushed
                        tipHeldElsewhere: workTree.headReachedElsewhere
                        busy: repoTab.busyCount > 0
                    }
                }
            }

            // ---- command log ------------------------------------------
            // Raised when asked for, and by a failure. Its band is the left menu's last row, carried across.
            //
            // **Built when it is raised and taken down when it is shut**: the panel is a list with its
            // own hand, a band and a ruler, and a page whose log is down would otherwise carry all of it
            // (rules-refs/app-ui.md — the dialog-seat rule). Nothing is lost in the taking down — the rows and the
            // selection are the model's — and the panel comes back the way it always came up, on its newest row
            // (`CommandsPane.cameUp`). The seat is what stands in the split, so the split's own properties are its.
            Loader {
                id: commandsSeat
                visible: page.commandsOpen
                active: page.commandsOpen
                SplitView.preferredHeight: 280
                SplitView.minimumHeight: pageLayout.commandsMinHeight
                sourceComponent: CommandsPane {
                    curPage: page
                    commandsModel: commandsModel
                    errorText: repoTab.lastError
                    // Read off the page: the press that raises the mark is often the press that builds this seat,
                    // and a panel told afterwards would come up dark and light a frame
                    // later.
                    attention: page.commandsAttention
                    onCloseRequested: page.shutCommands()
                    onErrorCleared: repoTab.clearLastError()
                    onCopyRequested: text => clipboard.copy(text)
                }
            }
        }
    }

    // A command the user asked for failed. **The panel is not raised here, and who it belongs to is not decided
    // here** (デザイン規約 §git が言ったことを読む場所 — 開く判断は「操作」の答えで下し、コマンド 1 本の終了コードでは
    // 下さない): every command the reader asks for runs inside a write (`session::open` hands out one asked-for
    // executor and writes are all that spawn on it), one operation is several commands, and a non-zero one part way
    // through is not a failure yet. The write's own answer raises the log and says whose the panel is in the same
    // breath — `tellRefusal` for the presses that named their write, `absorbLeftoverAnswer` for the ones nobody did,
    // and `onFetchFirstFailed` for the fetch.
    //
    // **Raised from here as well, it was raised twice on two feeds in no fixed order.** The command's end and its
    // write's answer travel separate queues (`hub::Feeds`), and where this one came second the fetch's own refusal
    // was read as somebody else's news and took the panel off the fetch that raised it; where it came first, the
    // fetch found a panel already standing and left it to the reader. Either way the recovery had nothing to take
    // down, and a laptop that woke up kept the news of the network it had lost (verify-linux `fetch-recover`,
    // `open=true` under gate load, green on its own).
    //
    // What is left here is the mark, which is this row's own: a report has already said what git said, so it goes
    // back down when the row it is about finally arrives (デザイン規約 §答えの要らない報せ).
    Connections {
        target: commandsModel
        function onFailure() {
            if (page.answeredFailures > 0) {
                page.answeredFailures--
                commandsModel.noteAnswered()
            }
        }
    }

    // The split bars' refusal — collection, settling, overlay and the divider-refuse report all live in the watcher
    // (SplitBarWatch); the page keeps the source chain, since only it sees all four sources.
    SplitBarWatch {
        id: splitWatch
        refusalSource: page.refusalSource
        graphPane: graphPane
    }

    /// Bars announce themselves once, at their own creation (`SplitHandleBar.Component.onCompleted`) — which is before
    /// the watcher above exists, so the page relays and holds the early ones; the page's `Component.onCompleted` hands
    /// them over.
    property var earlyBars: []
    function holdSplitBar(bar, held) {
        if (!splitWatch) {
            page.earlyBars.push(bar)
            return
        }
        splitWatch.holdSplitBar(bar, held)
    }

    /// Automation: the drag past whichever boundary `which` names, and whether that boundary is still drawn
    /// (`PGG_AUTO_ACT=divider-refuse`; AutoActDriver calls through the page).
    function dragDividerPast(which) {
        splitWatch.dragPast(which)
    }
    function refusalLineShown(which) {
        return splitWatch.lineShown(which)
    }

    /// Whichever boundary is refusing, or null. One pointer, so the order only decides which answers in the frame where
    /// two could — and two cannot, since a hand is on one boundary at a time.
    readonly property var refusalSource:
        graphPane.refused ? graphPane.refusedAt
        : detailsPane.descRefuses ? detailsPane.descPoint
        : wipPane.descRefuses ? wipPane.descPoint
        : splitWatch.refuses ? splitWatch.at
        : null

    /// What is drawn — the one badge's own `shown` (`RefusalBadge`, on why not its `visible`).
    readonly property bool refusalShown: splitWatch.shown

    function jumpToRef(oidHex) {
        // While a plan stands the selection has two halves, and they are meant to be the one commit: **this page's**,
        // which is what the plan pane lights a row from (`RebasePlanPane.selectedOid`), and **the model's row**, which
        // is what the boxes are routed by and where a typed reword lands (`planReword` / `planDraftOid`). A jump made
        // straight into this page's half lights the new row and leaves the routing on the old one, and the screen is
        // then about two commits at once.
        //
        // So a hash pressed under a plan is one of two things. **A commit the plan holds** is a walk down the plan,
        // and is taken the way the row click takes it — the model first, the page second (`RebasePlanPane`).
        // **Anything else** is past the base, where this screen has no row to put the reader on; it is refused in
        // silence, the way the mode's other refusals are (`startFind`), because nothing was lost and a hash is not a
        // press that has to be answered. The waiting face holds no rows at all, so every hash there goes the second
        // way.
        if (page.planShown) {
            const planRow = page.planActive ? planModel.rowOf(oidHex) : -1
            if (planRow < 0)
                return
            planModel.selectRow(planRow)
            page.activateRow(oidHex)
            return
        }
        const row = graphModel.rowOf(oidHex)
        if (row >= 0)
            graphPane.jumpToRow(row)
        // A jump is one commit, whatever was being held (デザイン規約 §複数のコミットを選ぶ): the pane on the right is
        // about to describe this one, and rows lit for a choice that no longer stands would say otherwise.
        page.chooseOnly(oidHex)
        // Details resolve even outside the window.
        page.rewordRow = -1
        page.wipShown = false
        // The row travels with the oid: a landing owed later (`followVanishedCommit`) reads `selectedRow`, and a
        // stale one lands the selection on whatever now stands at the previous selection's row.
        page.selectedRow = row
        page.selectedOid = oidHex
        page.selectedStashRef = graphModel.stashRefOf(oidHex)
        detailsModel.request(oidHex)
        page.askSignature(oidHex)
        page.closeDiff()
    }
}
