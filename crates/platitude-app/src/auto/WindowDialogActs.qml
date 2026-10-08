pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The dialogs' half of the window's PGG_AUTO_ACT harness: the platform picker, the clone box, the identity
/// question, and the two roads out of a folder that turned out not to be a repository. The settings screen's own
/// verbs are `WindowSettingsActs`.
// An `Item` only because `QtObject` has no default property to hold the timers; it is sizeless.
Item {
    id: acts

    required property var window
    required property TabsModel tabsModel
    required property Repeater pageRepeater
    required property TopBar topBar
    required property Item gate
    required property OpenFailedDialog openFailedDialog
    required property IdentityDialog identityDialog
    required property CloneModel cloneModel
    required property CloneDialog cloneDialog
    required property var folderDialog
    required property var settingsDialog
    required property QuitWaitDialog quitWaitDialog

    // `visible`, not `opened`: `FolderDialog` is `QtQuick.Dialogs`' own type and has no `opened`.
    SampleTimer {
        running: Harness.autoAct === "open-picker"
        onTriggered: {
            if (!folderDialog.visible)
                return
            stop()
            Harness.report(acts.pickerLine())
            window.finishAutoAct()
        }
    }
    /// `folder=` names this machine, so `beside_worktree=` is what is judged: whether the picker came up in the folder
    /// holding the worktree the tab stands in (`RepoTab.picker_folder_url`; the cases are verbs.md `open-picker`).
    /// Compared as paths, not as text — a space arrives percent-encoded — and the worktree's folder is the product's
    /// own conversion's to find (`GitFacts.folderUrlOf`), not split here. No page, no worktree: `false`.
    function pickerLine() {
        return "picker beside_worktree=" + acts.pickerIsBesideWorktree()
                + " folder=" + folderDialog.currentFolder
    }
    function pickerIsBesideWorktree() {
        const page = window.curPage
        if (page === null || page.pageTab.repoPath === "")
            return false
        return GitFacts.pickedPath(folderDialog.currentFolder.toString())
                === GitFacts.pickedPath(GitFacts.folderUrlOf(page.pageTab.repoPath))
    }
    // Every other verb that puts the picker up says where it was pointed; nothing else says so afterwards.
    Connections {
        target: acts.window
        enabled: Harness.autoAct !== "open-picker"
        function onPickerOpened() {
            Harness.report(acts.pickerLine())
        }
    }

    // PGG_AUTO_ACT=clone-dialog / clone-go / clone-refused: entered through the ☰'s row (`TabStrip.clickCloneRow`) so
    // the relays from the row to the window run too. The source is the repository this run opened — a path works on
    // both OSes with no remote built. The folder name decides which: its own lands beside, the source's is refused.
    readonly property bool cloneAct: Harness.autoAct === "clone-dialog"
                                     || Harness.autoAct === "clone-go"
                                     || Harness.autoAct === "clone-refused"
    readonly property string cloneFrom: Harness.autoActArg !== ""
                                        ? Harness.autoActArg
                                        : Harness.autoOpen.split(";")[0]
    SampleTimer {
        id: cloneTimer
        running: acts.cloneAct
        /// The row has been pressed; nothing re-reads what had to be true before it.
        property bool opened: false
        /// The clone was asked for, and the strip's count then — a picture cannot show that a tab is new.
        property bool asked: false
        property int tabsBefore: -1
        onTriggered: {
            const act = Harness.autoAct
            if (!cloneTimer.asked) {
                if (!cloneTimer.opened) {
                    if (window.curPage === null || window.curPage.pageTab.state !== "open")
                        return
                    topBar.clickCloneRow()
                    cloneTimer.opened = true
                    return
                }
                if (!cloneDialog.opened)
                    return
                if (act === "clone-dialog") {
                    cloneTimer.stop()
                    acts.reportClone()
                    window.finishAutoAct()
                    return
                }
                cloneDialog.setFields(acts.cloneFrom,
                                      act === "clone-go"
                                      ? "cloned-here" : GitFacts.cloneFolderName(acts.cloneFrom))
                if (!cloneDialog.canSubmit)
                    return
                cloneTimer.tabsBefore = pageRepeater.count
                cloneDialog.submit()
                cloneTimer.asked = true
                return
            }
            if (cloneModel.cloning)
                return
            // git's answer: the refusal line, or the new tab with its page open (what the picture is of).
            if (act === "clone-refused"
                ? cloneDialog.refusal === ""
                : (pageRepeater.count <= cloneTimer.tabsBefore
                   || window.curPage === null || window.curPage.pageTab.state !== "open"))
                return
            cloneTimer.stop()
            acts.reportClone()
            window.finishAutoAct()
        }
    }
    /// The judged fields lead, together: `must_say` matches a run of the line (verbs.md).
    function reportClone() {
        Harness.report("clone dialog=" + cloneDialog.opened
                          + " said=" + (cloneDialog.refusal !== "")
                          + " cloning=" + cloneModel.cloning
                          + " grew=" + (cloneTimer.tabsBefore >= 0
                                        && pageRepeater.count > cloneTimer.tabsBefore)
                          + " folder=" + (cloneDialog.parentUrl !== "")
                          + " tabs=" + pageRepeater.count
                          + " name=" + cloneDialog.wantedName)
    }

    // PGG_AUTO_ACT=open-not-a-repo / open-bare and the two ways back out. The picker is the platform's own window, so
    // the run enters where its answer lands — the path it accepted.
    readonly property bool pickAct: Harness.autoAct === "open-not-a-repo"
                                    || Harness.autoAct === "open-bare"
                                    || Harness.autoAct === "open-not-a-repo-retry"
                                    || Harness.autoAct === "open-not-a-repo-cancel"
                                    || Harness.autoAct === "open-dialog-sweep"
    property bool pickStarted: false
    SampleTimer {
        running: acts.pickAct
        onTriggered: {
            if (acts.pickStarted || !window.visible)
                return
            acts.pickStarted = true
            tabsModel.openPickedPath(Harness.autoActArg)
            pickAnswerTimer.start()
        }
    }
    SampleTimer {
        id: pickAnswerTimer
        onTriggered: {
            if (!openFailedDialog.opened)
                return
            if (Harness.autoAct === "open-dialog-sweep") {
                pickAnswerTimer.stop()
                // A modal has no seat for the command log, so its words are all a reader can take away
                // (規約 §右のペインの字は掴める). `kind=`: a dialog raised on `plain` has no line from git to sweep.
                Harness.report("open_dialog_sweep "
                    + openFailedDialog.background.pad.sweepAir(7, "kind=" + openFailedDialog.kind))
                window.finishAutoAct()
                return
            }
            if (Harness.autoAct === "open-not-a-repo-retry")
                openFailedDialog.retry()
            else if (Harness.autoAct === "open-not-a-repo-cancel")
                openFailedDialog.close()
            else {
                pickAnswerTimer.stop()
                acts.reportPick()
                window.finishAutoAct()
                return
            }
            pickAnswerTimer.stop()
            pickSettleTimer.start()
        }
    }
    SampleTimer {
        id: pickSettleTimer
        onTriggered: {
            if (openFailedDialog.opened)
                return
            pickSettleTimer.stop()
            acts.reportPick()
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=open-fail-tab / -bare / -log / open-fail-sweep: the road that keeps its tab. Nothing checks the
    // folder first, so the page says what failed (`kind=`). `-log` then opens the command log as the `>_` there does.
    SampleTimer {
        id: failTabActTimer
        running: Harness.autoAct === "open-fail-tab"
                 || Harness.autoAct === "open-fail-tab-bare"
                 || Harness.autoAct === "open-fail-tab-log"
                 || Harness.autoAct === "open-fail-sweep"
        onTriggered: {
            if (window.curPage === null || window.curPage.pageTab.state !== "open")
                return
            failTabActTimer.stop()
            tabsModel.openRepositoryPath(Harness.autoActArg)
            failTabTimer.start()
        }
    }
    SampleTimer {
        id: failTabTimer
        property bool commandsRequested: false
        onTriggered: {
            if (window.curPage === null || window.curPage.pageTab.state !== "error"
                    || window.curPage.pageTab.errorKind === "")
                return
            if (Harness.autoAct === "open-fail-tab-log" && window.curPage !== null)
                if (!failTabTimer.commandsRequested) {
                    failTabTimer.commandsRequested = true
                    window.curPage.toggleCommands()
                    return
                } else if (!window.curPage.commandsShown) {
                    return
                }
            failTabTimer.stop()
            if (Harness.autoAct === "open-fail-sweep") {
                // The log's seat is here but the folder is not, and a reader pastes it into a shell
                // (規約 §右のペインの字は掴める). `kind=`: only one of the three screens has a line from git.
                Harness.report("open_fail_sweep "
                    + window.curPage.openFailedHand.sweepAir(7, "kind=" + window.curPage.pageTab.errorKind))
                window.finishAutoAct()
                return
            }
            Harness.report(
                "open_fail_tab tabs=" + pageRepeater.count
                + " state=" + (window.curPage !== null ? window.curPage.pageTab.state : "-")
                + " kind=" + (window.curPage !== null ? window.curPage.pageTab.errorKind : "-")
                + " commands=" + (window.curPage !== null ? window.curPage.commandsShown : "-"))
            window.finishAutoAct()
        }
    }
    /// `dialog=` is the dialog's own `opened` — what was asked of it would pass with the binding cut. `tabs=` says the
    /// refused folder never became a tab.
    function reportPick() {
        Harness.report("open_failed kind=" + openFailedDialog.kind
                          + " dialog=" + openFailedDialog.opened
                          + " tabs=" + pageRepeater.count
                          + " active=" + tabsModel.currentIndex
                          + " near=" + openFailedDialog.near)
    }
    // PGG_AUTO_ACT=identity / identity-half: which half landed is a pair of booleans — a dialog left open because the
    // save did not take looks like one nobody has answered.
    SampleTimer {
        running: Harness.autoAct === "identity" || Harness.autoAct === "identity-half"
        onTriggered: {
            const wholeReady = Harness.autoAct === "identity"
                               && AppBackend.identityState === "missing"
                               && identityDialog.opened
            const halfReady = Harness.autoAct === "identity-half"
                              && AppBackend.identityState === "ready"
                              && AppBackend.identityUnsaved
                              && identityDialog.opened
            if (!wholeReady && !halfReady)
                return
            stop()
            Harness.report(
                "identity state=" + AppBackend.identityState
                + " dialog=" + identityDialog.opened
                + " nameSaved=" + AppBackend.identityNameSaved
                + " emailSaved=" + AppBackend.identityEmailSaved
                + " unsaved=" + AppBackend.identityUnsaved
                + " badge=" + topBar.identityBadgeShown
                + " said=" + (AppBackend.identityError !== ""))
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=identity-tip: the mark's reason, read where the pointer cannot go. `badge=` is the group in
    // whichever shape the width left it — the mark alone would fail a band saying what it should. Dismissal waits on
    // `identityUnsaved`, the save's answer.
    //
    // Three samplers, each naming its hop (`step=`) so a run that stalls says where; a line carries only what could
    // have come out the other way. `step=` lines stand apart: `must_say` matches a run of the last line (verbs.md).
    SampleTimer {
        running: Harness.autoAct === "identity-tip"
        onTriggered: {
            if (!identityDialog.opened || !AppBackend.identityUnsaved)
                return
            window.dismissIdentity()
            stop()
            // Whether the answer took the dialog down in the same call; the guard's two cannot have changed.
            Harness.report("identity_tip step=dismissed dialog=" + identityDialog.opened)
            identityTipTimer.start()
        }
    }
    SampleTimer {
        id: identityTipTimer
        onTriggered: {
            if (identityDialog.opened)
                return
            topBar.statePointedAt = true
            stop()
            // The rest of what the card waits on: the group standing, and placed (`BandStateGroup.standInAsking`).
            Harness.report("identity_tip step=pointed"
                + " badge=" + (topBar.stateWordsShown || topBar.stateMarkShown)
                + " placed=" + topBar.statePlaced)
            identityTipReport.start()
        }
    }
    // The card opens after its tip delay.
    SampleTimer {
        id: identityTipReport
        onTriggered: {
            if (!topBar.stateCardOpen)
                return
            stop()
            // `rows=` is read without waiting for the card's layout (unlike `badges-hover`): one row cannot come out of
            // order, so `laidOut=` only says whether the picture has the card at full height.
            Harness.report("identity_tip step=carded laidOut=" + topBar.stateCardLaidOut)
            Harness.report(
                "identity_tip unsaved=" + AppBackend.identityUnsaved
                + " badge=" + (topBar.stateWordsShown || topBar.stateMarkShown)
                + " tip=" + topBar.stateCardOpen
                + " rows=" + topBar.stateCardRows)
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=quit-waits / quit-locked: the close while git is still writing — the commit is held by the sleeping
    // pre-commit hook of `--preset slowhook`, so the gate turns the close away and stands the wait dialog up.
    // `quit-waits` photographs that; `quit-locked` closes again, then watches the held write land anyway.
    //
    // Closed through `window.close()`, as the ✕ and Exit are: once the modal stands it is the only road left (the
    // modal seals the pointer and `Shortcut`s — rules-refs/app-ui.md「アプリ終了の close ゲートは 1 本」). A handler
    // called from QML would run whatever the modal covers.
    SampleTimer {
        id: quitTimer
        running: Harness.autoAct === "quit-waits" || Harness.autoAct === "quit-locked"
        /// Steps taken, so nothing re-reads what had to be true before each.
        property bool committed: false
        property bool closed: false
        property bool retried: false
        property int seqBefore: -1
        onTriggered: {
            if (window.curPage === null)
                return
            const tab = window.curPage.pageTab
            if (!quitTimer.committed) {
                if (tab.state !== "open")
                    return
                quitTimer.seqBefore = tab.writeSeq
                tab.commit("chore: held by a sleeping hook", "", false, false)
                quitTimer.committed = true
                return
            }
            if (!quitTimer.closed) {
                // Only while the hook holds the write: it sleeps longer than a beat, so a risen busy count is still up
                // when the close lands.
                if (tab.busyCount === 0)
                    return
                quitTimer.closed = true
                window.close()
                return
            }
            if (Harness.autoAct === "quit-waits") {
                if (!quitWaitDialog.opened)
                    return
                stop()
                Harness.report("quit_wait dialog=" + quitWaitDialog.opened
                    + " window=" + window.visible
                    + " busy=" + (tab.busyCount !== 0))
                window.finishAutoAct()
                return
            }
            if (!quitTimer.retried) {
                // The same guard: a close after the hook let go is rightly let through.
                if (!quitWaitDialog.opened || tab.busyCount === 0)
                    return
                quitTimer.retried = true
                window.close()
                return
            }
            // The held write landed under the lock: `writeSeq` past `seqBefore` is its answer, busy at zero the queue
            // done with it.
            if (!quitWaitDialog.opened || tab.busyCount !== 0 || tab.writeSeq <= quitTimer.seqBefore)
                return
            // And reached the screen: the answer comes before the status is read again (`session::write::run_write`),
            // and this commit takes the worktree row off: ending on the answer flips `WipTallyRow` in the census.
            const page = window.curPage
            if (page.pageWorktree.wipRowStands || !PageSettled.settled(page))
                return
            stop()
            Harness.report("quit_lock dialog=" + quitWaitDialog.opened
                + " window=" + window.visible
                + " escape=" + quitWaitDialog.escapes
                + " vetoes=" + quitWaitDialog.vetoes
                + " landed=" + (tab.writeSeq > quitTimer.seqBefore))
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=quit-save-held: the close while the app's own config save is out — the identity
    // `WindowIdentityActs` submitted, held by `--fault-hold-save` until the shutdown joins it. No session carries it,
    // so the gate's clause for the hub's saves turns the close away (`Hub::writes_settled`). After the picture the
    // run exits through the gate's yield (`WindowQuitGate`), the hold lets go at `writes-joining`, and xtask reads
    // the run's gitconfig afterwards (`verify::outcome`).
    SampleTimer {
        id: quitSaveTimer
        running: Harness.autoAct === "quit-save-held"
        property bool closed: false
        onTriggered: {
            if (!quitSaveTimer.closed) {
                if (!AppBackend.identityBusy || Harness.heldSaves() === 0)
                    return
                quitSaveTimer.closed = true
                window.close()
                return
            }
            if (!quitWaitDialog.opened)
                return
            stop()
            Harness.report("quit_save dialog=" + quitWaitDialog.opened
                + " window=" + window.visible
                + " vetoes=" + quitWaitDialog.vetoes
                + " busy=" + AppBackend.identityBusy
                + " held=" + Harness.heldSaves())
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=gate-sweep: the `solo` screen, its words taken from the air (規約 §右のペインの字は掴める) — before
    // any repository opens there is no log to reach. Run in `solo`'s state (the harness holds the real lock), the one
    // where the path shows; the other gate, a git that will not answer, needs a PATH without git and has no verb.
    SampleTimer {
        id: gateSweepTimer
        running: Harness.autoAct === "gate-sweep"
        onTriggered: {
            if (!AppBackend.alreadyRunning || !gate.visible || AppBackend.heldElsewhere === "")
                return
            stop()
            // `held=`: an empty gate has air and no fields.
            Harness.report("gate_sweep "
                + gate.pad.sweepAir(7, "held=" + (AppBackend.heldElsewhere !== "")))
            window.finishAutoAct()
        }
    }
}
