pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The dialogs' half of the window's PG_AUTO_ACT harness: the platform picker, the settings tools, the avatar
/// card, the identity question, and the two roads out of a folder that turned out not to be a repository.
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
    required property var folderDialog
    required property var settingsDialog

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

    // The tools popup has two separately latched output states: a real loading edge and the populated, settled choices.
    SampleTimer {
        running: AppBackend.autoAct === "settings-tools"
                 || AppBackend.autoAct === "settings-tools-loading"
        onTriggered: {
            const ready = AppBackend.autoAct === "settings-tools-loading"
                        ? settingsDialog.autoToolsLoadingReady
                        : settingsDialog.autoToolsSettledReady
            if (!ready)
                return
            stop()
            settingsDialog.reportTool()
            window.finishAutoAct()
        }
    }

    // The same card's avatar half, whose four shots the page opens and this finishes. Each waits on what its own verb
    // produced: the row the store answered the filing with and the picture inside it, that row's `lit`, the candidate
    // list's `opened`, and — for the removal — the row leaving the store on the far side of a hold that runs at its own
    // length (`Metrics.holdMs`). Nothing here reads a clock.
    SampleTimer {
        id: avatarCardTimer
        running: AppBackend.autoAct === "avatar-settings" || AppBackend.autoAct === "avatar-row-lit"
                 || AppBackend.autoAct === "avatar-combo" || AppBackend.autoAct === "avatar-remove"
        /// Raised once this verb's own move has been made, so nothing after it re-reads what had to be true before it.
        /// The removal's answer is a row going away, and a gate still wanting that row would never let go of it (the
        /// wait `middle-close` describes).
        property bool acted: false
        /// How many rows the hold was made against, read in the branch that presses and nowhere else.
        property int rowsBefore: -1
        onTriggered: {
            const act = AppBackend.autoAct
            if (!avatarCardTimer.acted) {
                if (!settingsDialog.opened)
                    return
                // A run that filed a picture on its way in has to have it in the list before any of this means
                // anything; one that filed nothing — the round-trip read — has whatever the store gave it.
                if (AppBackend.autoActArg !== "" && !settingsDialog.autoAvatarRowPainted(0))
                    return
                if (act === "avatar-row-lit") {
                    if (!settingsDialog.autoAvatarRowLit(0))
                        return
                } else if (act === "avatar-combo") {
                    // Asked again while it is still shut: the field defers the list by a turn of the loop, and a list
                    // taken back down under an unwinding grab has to be asked for a second time (`AppCombo.pressField`).
                    if (!settingsDialog.autoAvatarComboOpen) {
                        settingsDialog.autoAvatarOfferCombo()
                        return
                    }
                } else if (act === "avatar-remove") {
                    if (!settingsDialog.autoAvatarHoldRemove(0))
                        return
                    avatarCardTimer.rowsBefore = settingsDialog.autoAvatarRows
                }
                avatarCardTimer.acted = true
            }
            // The hold is the one move whose answer arrives after it: the store has to have let the row go.
            if (act === "avatar-remove" && settingsDialog.autoAvatarRows >= avatarCardTimer.rowsBefore)
                return
            avatarCardTimer.stop()
            AppBackend.report("avatar_card rows=" + settingsDialog.autoAvatarRows
                              + " painted=" + settingsDialog.autoAvatarRowPainted(0)
                              + " lit=" + settingsDialog.autoAvatarRowLit(0)
                              + " combo=" + settingsDialog.autoAvatarComboOpen
                              + " removed=" + (avatarCardTimer.rowsBefore > settingsDialog.autoAvatarRows))
            window.finishAutoAct()
        }
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
