pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The dialogs' half of the window's PG_AUTO_ACT harness: the platform picker, the clone box, the identity
/// question, and the two roads out of a folder that turned out not to be a repository.
///
/// The settings screen's own verbs are `WindowSettingsActs` — they reach into one dialog and nothing else,
/// which is what makes them a file rather than a section.
///
/// Built by `WindowAutoActDriver`, which is what `Main` builds when a verb was given; what these verbs act
/// on is handed down below, one property per part of the window they reach into.
// An `Item` only because `QtObject` has no default property to hold the timers below; it draws nothing and is
// never given a size.
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

    // The picker completes once its dialog is up, and `visible` is the property that says so: `FolderDialog` is
    // `QtQuick.Dialogs`' own type, not a `Popup`, so the `opened` the dialogs around it answer to is undefined here.
    SampleTimer {
        running: AppBackend.autoAct === "open-picker"
        onTriggered: {
            if (!folderDialog.visible)
                return
            stop()
            AppBackend.report("picker folder=" + folderDialog.currentFolder)
            window.finishAutoAct()
        }
    }

    // PG_AUTO_ACT=clone-dialog / clone-go / clone-refused: the box that fetches a repository, entered through the ☰'s
    // own row (`TabStrip.clickCloneRow`) so the two signal relays between the row and the window are part of what runs.
    //
    // **The far side is the repository this run opened.** A folder on this machine is a URL git takes, so no remote has
    // to be built for these and the same verb works on both operating systems (the avatar family's problem —
    // an argument that is a path cannot be handed to both). Which of the two answers comes back is decided by the
    // folder name alone: a name of its own lands beside the source, the source's own name lands **on** it, and git
    // refuses that one for the destination being taken.
    readonly property bool cloneAct: AppBackend.autoAct === "clone-dialog"
                                     || AppBackend.autoAct === "clone-go"
                                     || AppBackend.autoAct === "clone-refused"
    /// What to fetch from: whatever the verb was given, or the repository already open.
    readonly property string cloneFrom: AppBackend.autoActArg !== ""
                                        ? AppBackend.autoActArg
                                        : AppBackend.autoOpen.split(";")[0]
    SampleTimer {
        id: cloneTimer
        running: acts.cloneAct
        /// The row has been pressed; nothing re-reads what had to be true before it.
        property bool opened: false
        /// The clone has been asked for, and what the strip held when it was — the half the picture cannot hold,
        /// since a window with one more tab looks like a window that always had it.
        property bool asked: false
        property int tabsBefore: -1
        onTriggered: {
            const act = AppBackend.autoAct
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
                // The name is typed rather than left to follow the URL: one of these two has to land where the
                // source is not, and the other exactly on it.
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
            // git's answer, whichever it was: the line that quotes it, or the tab the clone became — and, for the
            // tab, the page finished reading it, since that is what the picture is of.
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
    /// What the clone verbs have to show for themselves. `dialog=` is the box's own `opened` — up for the two that
    /// stay, down for the one that landed — `said=` is git's line standing in it, and `grew=` is the strip gaining the
    /// tab the clone became.
    ///
    /// **The four the table judges are written first and in one run**: `must_say` matches a run of the line, so a
    /// field none of the verbs judges must not stand between two that they do (verbs.md).
    function reportClone() {
        AppBackend.report("clone dialog=" + cloneDialog.opened
                          + " said=" + (cloneDialog.refusal !== "")
                          + " cloning=" + cloneModel.cloning
                          + " grew=" + (cloneTimer.tabsBefore >= 0
                                        && pageRepeater.count > cloneTimer.tabsBefore)
                          + " folder=" + (cloneDialog.parentUrl !== "")
                          + " tabs=" + pageRepeater.count
                          + " name=" + cloneDialog.wantedName)
    }

    // Smoke hooks (PG_AUTO_ACT=open-not-a-repo / open-bare and the two ways back out). The picker is the platform's own
    // window, so the run enters where its answer lands — the path it accepted.
    readonly property bool pickAct: AppBackend.autoAct === "open-not-a-repo"
                                    || AppBackend.autoAct === "open-bare"
                                    || AppBackend.autoAct === "open-not-a-repo-retry"
                                    || AppBackend.autoAct === "open-not-a-repo-cancel"
                                    || AppBackend.autoAct === "open-dialog-sweep"
    property bool pickStarted: false
    SampleTimer {
        running: acts.pickAct
        onTriggered: {
            if (acts.pickStarted || !window.visible)
                return
            acts.pickStarted = true
            tabsModel.openPickedPath(AppBackend.autoActArg)
            pickAnswerTimer.start()
        }
    }
    // Poll the dialog's observable answer. The 25ms cadence is sampling only; it is not a correctness deadline.
    SampleTimer {
        id: pickAnswerTimer
        onTriggered: {
            if (!openFailedDialog.opened)
                return
            if (AppBackend.autoAct === "open-dialog-sweep") {
                pickAnswerTimer.stop()
                // The one failure with nothing behind it: a modal window carries no seat for the command log, so the
                // folder it names and git's own answer are the whole of what a reader can take away from here
                // (規約 §右のペインの字は掴める). `kind=` is what the sweep had to land on — a dialog raised on `plain`
                // has no line from git at all, and a run that swept one would be claiming less than it looked.
                AppBackend.report("open_dialog_sweep "
                    + openFailedDialog.background.pad.sweepAir(7, "kind=" + openFailedDialog.kind))
                window.finishAutoAct()
                return
            }
            if (AppBackend.autoAct === "open-not-a-repo-retry")
                openFailedDialog.retry()
            else if (AppBackend.autoAct === "open-not-a-repo-cancel")
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

    // Smoke hooks (PG_AUTO_ACT=open-fail-tab / -bare / -log): the road that keeps its tab. Nothing checks the folder
    // first there, so the page itself is what says so (`kind=` reports which). The `-log` half goes on to open the
    // command log the way the `>_` at the foot of that screen does.
    SampleTimer {
        id: failTabActTimer
        running: AppBackend.autoAct === "open-fail-tab"
                 || AppBackend.autoAct === "open-fail-tab-bare"
                 || AppBackend.autoAct === "open-fail-tab-log"
                 || AppBackend.autoAct === "open-fail-sweep"
        onTriggered: {
            if (window.curPage === null || window.curPage.pageTab.state !== "open")
                return
            failTabActTimer.stop()
            tabsModel.openRepositoryPath(AppBackend.autoActArg)
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
            if (AppBackend.autoAct === "open-fail-tab-log" && window.curPage !== null)
                if (!failTabTimer.commandsRequested) {
                    failTabTimer.commandsRequested = true
                    window.curPage.toggleCommands()
                    return
                } else if (!window.curPage.commandsShown) {
                    return
                }
            failTabTimer.stop()
            if (AppBackend.autoAct === "open-fail-sweep") {
                // The tab's own failure screen, swept instead of photographed. This one *does* carry the log's seat at
                // its foot, so git's answer is reachable there — but the folder is not, and it is the half a reader
                // needs to paste back into a shell (規約 §右のペインの字は掴める). `kind=` says which of the three
                // screens the sweep landed on, since only one of them has a line from git in it at all.
                AppBackend.report("open_fail_sweep "
                    + window.curPage.openFailedHand.sweepAir(7, "kind=" + window.curPage.pageTab.errorKind))
                window.finishAutoAct()
                return
            }
            AppBackend.report(
                "open_fail_tab tabs=" + pageRepeater.count
                + " state=" + (window.curPage !== null ? window.curPage.pageTab.state : "-")
                + " kind=" + (window.curPage !== null ? window.curPage.pageTab.errorKind : "-")
                + " commands=" + (window.curPage !== null ? window.curPage.commandsShown : "-"))
            window.finishAutoAct()
        }
    }
    /// What the run has to show for itself. `dialog=` is the dialog's own `opened` (reporting what was asked of it
    /// would go on passing with the binding cut), and `tabs=` says the refused folder never became one — which is the
    /// whole of what this verb is about.
    function reportPick() {
        AppBackend.report("open_failed kind=" + openFailedDialog.kind
                          + " dialog=" + openFailedDialog.opened
                          + " tabs=" + pageRepeater.count
                          + " active=" + tabsModel.currentIndex
                          + " near=" + openFailedDialog.near)
    }
    // PG_AUTO_ACT=identity / identity-half: "which half landed" is a pair of booleans, and a dialog that stayed open
    // because the save did not take looks exactly like one nobody has answered yet. Read the two verbs as a pair.
    SampleTimer {
        running: AppBackend.autoAct === "identity" || AppBackend.autoAct === "identity-half"
        onTriggered: {
            const wholeReady = AppBackend.autoAct === "identity"
                               && AppBackend.identityState === "missing"
                               && identityDialog.opened
            const halfReady = AppBackend.autoAct === "identity-half"
                              && AppBackend.identityState === "ready"
                              && AppBackend.identityUnsaved
                              && identityDialog.opened
            if (!wholeReady && !halfReady)
                return
            stop()
            AppBackend.report(
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

    // PG_AUTO_ACT=identity-tip: the mark's reason, read where the pointer cannot go. `tip=` is the card's own `opened`;
    // `badge=` is the group in whichever shape the width left it — reading the mark alone would fail a band that is
    // saying exactly what it should. Dismissal waits on `identityUnsaved` — the save's answer, not the open dialog.
    SampleTimer {
        running: AppBackend.autoAct === "identity-tip"
        onTriggered: {
            if (!identityDialog.opened || !AppBackend.identityUnsaved)
                return
            window.dismissIdentity()
            stop()
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
            identityTipReport.start()
        }
    }
    // The attached card intentionally has a visual tip delay. Completion is still gated by its opened property, never
    // by that duration.
    SampleTimer {
        id: identityTipReport
        onTriggered: {
            if (!topBar.stateCardOpen)
                return
            stop()
            AppBackend.report(
                "identity_tip unsaved=" + AppBackend.identityUnsaved
                + " badge=" + (topBar.stateWordsShown || topBar.stateMarkShown)
                + " tip=" + topBar.stateCardOpen
                + " rows=" + topBar.stateCardRows)
            window.finishAutoAct()
        }
    }

    // PG_AUTO_ACT=quit-waits / quit-locked: the close that arrives while git is still writing. The commit is held by
    // the repository's own pre-commit hook (`--preset slowhook` — it sleeps), so the write is provably in flight when
    // the close lands: the gate turns the close away and stands the wait dialog up. `quit-waits` photographs that
    // state; `quit-locked` tries the door a second time and then watches the held write land anyway — the wait takes
    // no answer, and it was never a kill.
    //
    // The window is closed through `window.close()`, the same call the band's ✕ and the ☰'s Exit make — and, once the
    // lock stands, **the only road left**: the modal seals both the pointer and the window's own `Shortcut`s
    // (measured, qmltestrunner — rules-refs/app-ui.md §close ゲート), while a close request still reaches `onClosing`
    // the way Alt+F4 does. That is why the second press is the honest test of a lock with no way out, and why no verb
    // here fires a write behind it: a handler called from QML would run whatever the modal is covering, and reporting
    // that it did not would be a claim about a road this harness cannot drive.
    //
    // Neither verb is a write act: the commit deliberately has not landed when `quit-waits` photographs, and
    // `quit-locked` waits the landing out in its own sampler (`AutoActCompletion`).
    SampleTimer {
        id: quitTimer
        running: AppBackend.autoAct === "quit-waits" || AppBackend.autoAct === "quit-locked"
        /// The steps already taken, so nothing re-reads what had to be true before each of them.
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
                // Pressed only while the hook provably holds the write: the preset's hook sleeps for longer than
                // the beat between this tick and the close below, so a busy count that has risen cannot have
                // fallen by the time the close lands.
                if (tab.busyCount === 0)
                    return
                quitTimer.closed = true
                window.close()
                return
            }
            if (AppBackend.autoAct === "quit-waits") {
                if (!quitWaitDialog.opened)
                    return
                stop()
                AppBackend.report("quit_wait dialog=" + quitWaitDialog.opened
                    + " window=" + window.visible
                    + " busy=" + (tab.busyCount !== 0))
                window.finishAutoAct()
                return
            }
            if (!quitTimer.retried) {
                // The same guard as the first press, for the same reason: a close that arrived after the hook let
                // go would be let through, and the gate would be right to let it. The write has to still be out for
                // the refusal to mean anything.
                if (!quitWaitDialog.opened || tab.busyCount === 0)
                    return
                quitTimer.retried = true
                window.close()
                return
            }
            // The lock never let go, the window stayed, and the write the quit was asked over still landed: the
            // sequence moving past the armed one is the write's answer being absorbed, busy falling is the queue
            // done with it.
            if (!quitWaitDialog.opened || tab.busyCount !== 0 || tab.writeSeq <= quitTimer.seqBefore)
                return
            stop()
            AppBackend.report("quit_lock dialog=" + quitWaitDialog.opened
                + " window=" + window.visible
                + " escape=" + quitWaitDialog.escapes
                + " vetoes=" + quitWaitDialog.vetoes
                + " landed=" + (tab.writeSeq > quitTimer.seqBefore))
            window.finishAutoAct()
        }
    }

    // PG_AUTO_ACT=gate-sweep: the same screen `solo` photographs, with its words taken from the air around them
    // (規約 §右のペインの字は掴める). **This is the surface with no way out to the log** — the gate stands before any
    // repository is open, so git's own answer and the path of the build already holding the settings are the whole of
    // what there is to take away, and a reader who cannot drag them retypes them.
    //
    // The path is only there while another build holds the store, which is the state the harness makes for `solo` (it
    // takes the real lock before starting this process), so the sweep is run in that one: the other way the gate comes
    // up — a git that would not answer — has no verb, since it needs a PATH without git on it.
    SampleTimer {
        id: gateSweepTimer
        running: AppBackend.autoAct === "gate-sweep"
        onTriggered: {
            if (!AppBackend.alreadyRunning || !gate.visible || AppBackend.heldElsewhere === "")
                return
            stop()
            // `held=` is what the sweep had to land on: an empty gate has air and no fields, and a run that swept one
            // would be reporting on a screen the reader never sees.
            AppBackend.report("gate_sweep "
                + gate.pad.sweepAir(7, "held=" + (AppBackend.heldElsewhere !== "")))
            window.finishAutoAct()
        }
    }
}
