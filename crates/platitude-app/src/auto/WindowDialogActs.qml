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

    // `visible`, not `opened`: `FolderDialog` is `QtQuick.Dialogs`' own type and has no `opened`. The page's dispatch
    // puts the picker up (`AutoActFindVerbs`); nothing here presses, so nothing waits on that dispatch but the picker.
    WindowStage {
        id: pickerTimer
        name: "open_picker"
        running: Harness.autoAct === "open-picker"
        onTriggered: {
            if (!Awaited.all("open_picker_ready", { "shown": folderDialog.visible,
                                                    "page": PageSettled.windowSettled(window) }))
                return
            pickerTimer.complete(() => acts.pickerLine())
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
    // Every other verb that puts the picker up notes where it was pointed (judged by none); nothing else says so
    // afterwards.
    Connections {
        target: acts.window
        enabled: Harness.autoAct !== "open-picker"
        function onPickerOpened() {
            Awaited.note(acts.pickerLine())
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
    // The row pressed (`opened`) — nothing reads again what had to be true before it — then, past the bare dialog, the
    // clone asked for (`asked`).
    WindowStage {
        id: cloneTimer
        name: "clone"
        order: Harness.autoAct === "clone-dialog" ? ["opened"] : ["opened", "asked"]
        running: acts.cloneAct
        /// The strip's count as the clone was asked for — a picture cannot show that a tab is new.
        property int tabsBefore: -1
        onTriggered: {
            const op = cloneTimer
            const act = Harness.autoAct
            if (op.at("start")) {
                if (!Awaited.all("clone_open", { "open": window.curPage !== null
                                                         && window.curPage.pageTab.state === "open" }))
                    return
                op.input("opened", () => {
                    topBar.clickCloneRow()
                })
                return
            }
            if (op.at("opened")) {
                // The page behind the dialog whole (`PageSettled`): it is in the picture too.
                if (!Awaited.all("clone_dialog_ready", { "dialog": cloneDialog.opened,
                                                          "page": PageSettled.settled(window.curPage) }))
                    return
                if (act === "clone-dialog") {
                    op.complete(() => acts.cloneLine())
                    return
                }
                // Filled every beat until the box takes the fields: the submit goes once it can.
                op.input("asked", () => {
                    cloneDialog.setFields(acts.cloneFrom,
                                          act === "clone-go"
                                          ? "cloned-here" : GitFacts.cloneFolderName(acts.cloneFrom))
                    if (!cloneDialog.canSubmit)
                        return false
                    op.tabsBefore = pageRepeater.count
                    cloneDialog.submit()
                    return true
                })
                return
            }
            // git's answer: the refusal line, or the new tab with its page open (what the picture is of).
            if (!Awaited.all("clone_landed", {
                "done": !cloneModel.cloning,
                "answer": act === "clone-refused" ? cloneDialog.refusal !== ""
                                                  : pageRepeater.count > op.tabsBefore,
                "page": PageSettled.settledUnder(window) }))
                return
            op.complete(() => acts.cloneLine())
        }
    }
    /// The judged fields lead, together: `must_say` matches a run of the line (verbs.md).
    function cloneLine() {
        return "clone dialog=" + cloneDialog.opened
            + " said=" + (cloneDialog.refusal !== "")
            + " cloning=" + cloneModel.cloning
            + " grew=" + (cloneTimer.tabsBefore >= 0
                          && pageRepeater.count > cloneTimer.tabsBefore)
            + " folder=" + (cloneDialog.parentUrl !== "")
            + " tabs=" + pageRepeater.count
            + " name=" + cloneDialog.wantedName
    }

    // PGG_AUTO_ACT=open-not-a-repo / open-bare / open-unreadable and the two ways back out. The picker is the
    // platform's own window, so the run enters where its answer lands — the path it accepted.
    readonly property bool pickAct: Harness.autoAct === "open-not-a-repo"
                                    || Harness.autoAct === "open-bare"
                                    || Harness.autoAct === "open-unreadable"
                                    || Harness.autoAct === "open-not-a-repo-retry"
                                    || Harness.autoAct === "open-not-a-repo-cancel"
                                    || Harness.autoAct === "open-dialog-sweep"
    readonly property bool pickAnswers: Harness.autoAct === "open-not-a-repo-retry"
                                        || Harness.autoAct === "open-not-a-repo-cancel"
    // The path put in (`picked`), then the dialog's words swept (`swept`) or one of the two ways back out taken
    // (`answered`).
    WindowStage {
        id: pickTimer
        name: "open_pick"
        order: Harness.autoAct === "open-dialog-sweep" ? ["picked", "swept"]
             : acts.pickAnswers ? ["picked", "answered"] : ["picked"]
        running: acts.pickAct
        /// What the sweep came away with (`SweepPad.sweepAir`), read as it swept.
        property string swept: ""
        onTriggered: {
            const op = pickTimer
            if (op.at("start")) {
                if (!Awaited.all("open_pick_ready", { "shown": window.visible }))
                    return
                op.input("picked", () => {
                    tabsModel.openPickedPath(Harness.autoActArg)
                })
                return
            }
            if (op.at("picked")) {
                if (!Awaited.all("pick_answer_ready", { "ready": openFailedDialog.opened,
                                                        "page": PageSettled.windowSettled(window) }))
                    return
                if (Harness.autoAct === "open-dialog-sweep") {
                    // A modal has no seat for the command log, so its words are all a reader can take away
                    // (規約 §右のペインの字は掴める). `kind=`: a dialog raised on `plain` has no line from git to sweep.
                    if (!op.input("swept", () => {
                            op.swept = openFailedDialog.background.pad.sweepAir(7, "kind=" + openFailedDialog.kind)
                        }))
                        return
                    op.complete(() => "open_dialog_sweep " + op.swept)
                    return
                }
                if (acts.pickAnswers) {
                    op.input("answered", () => {
                        if (Harness.autoAct === "open-not-a-repo-retry")
                            openFailedDialog.retry()
                        else
                            openFailedDialog.close()
                    })
                    return
                }
                op.complete(() => acts.pickLine())
                return
            }
            if (!Awaited.all("pick_settle_ready", { "ready": !(openFailedDialog.opened),
                                                    "page": PageSettled.windowSettled(window) }))
                return
            op.complete(() => acts.pickLine())
        }
    }

    // PGG_AUTO_ACT=open-fail-tab / -bare / -unreadable / -log / open-fail-sweep: the road that keeps its tab. Nothing
    // checks the folder first, so the page says what failed (`kind=`). `-log` then opens the command log as the `>_`
    // there does.
    //
    // The failing tab stands (`tab`), then the log opened (`logged`) or the words swept (`swept`).
    WindowStage {
        id: failTabActTimer
        name: "fail_tab"
        order: Harness.autoAct === "open-fail-tab-log" ? ["tab", "logged"]
             : Harness.autoAct === "open-fail-sweep" ? ["tab", "swept"] : ["tab"]
        running: Harness.autoAct === "open-fail-tab"
                 || Harness.autoAct === "open-fail-tab-bare"
                 || Harness.autoAct === "open-fail-tab-unreadable"
                 || Harness.autoAct === "open-fail-tab-log"
                 || Harness.autoAct === "open-fail-sweep"
        /// What the sweep came away with (`SweepPad.sweepAir`), read as it swept.
        property string swept: ""
        onTriggered: {
            const op = failTabActTimer
            const page = window.curPage
            if (op.at("start")) {
                if (!Awaited.all("fail_tab_start", {
                        "page": page !== null,
                        "state": page !== null
                            && (page.pageTab.state === "open" || page.pageTab.state === "error") }))
                    return
                // A tab that failed as the window came up — a restored folder git now refuses, which
                // `--app-env GIT_TEST_ASSUME_DIFFERENT_OWNER=1` makes of any `--repo` — is already the
                // screen this verb reads: nothing else is opened.
                if (page.pageTab.state === "error") {
                    op.advance("tab")
                    return
                }
                op.input("tab", () => {
                    tabsModel.openRepositoryPath(Harness.autoActArg)
                })
                return
            }
            if (!Awaited.all("fail_tab_ready", {
                "ready": !(page === null || page.pageTab.state !== "error"
                    || page.pageTab.errorKind === ""),
                "page": PageSettled.windowSettled(window),
                "log": !op.at("logged") || (page !== null && page.commandsShown) }))
                return
            if (Harness.autoAct === "open-fail-tab-log" && op.at("tab")) {
                op.input("logged", () => {
                    page.toggleCommands()
                })
                return
            }
            if (Harness.autoAct === "open-fail-sweep") {
                // The log's seat is here but the folder is not, and a reader pastes it into a shell
                // (規約 §右のペインの字は掴める). `kind=`: three of the five screens have a line from git.
                if (!op.input("swept", () => {
                        op.swept = page.openFailedHand.sweepAir(7, "kind=" + page.pageTab.errorKind)
                    }))
                    return
                op.complete(() => "open_fail_sweep " + op.swept)
                return
            }
            op.complete(() => {
                const failed = window.curPage
                return "open_fail_tab tabs=" + pageRepeater.count
                    + " state=" + (failed !== null ? failed.pageTab.state : "-")
                    + " kind=" + (failed !== null ? failed.pageTab.errorKind : "-")
                    + " commands=" + (failed !== null ? failed.commandsShown : "-")
            })
        }
    }
    /// `dialog=` is the dialog's own `opened` — what was asked of it would pass with the binding cut. `tabs=` says the
    /// refused folder never became a tab.
    function pickLine() {
        return "open_failed kind=" + openFailedDialog.kind
            + " dialog=" + openFailedDialog.opened
            + " tabs=" + pageRepeater.count
            + " active=" + tabsModel.currentIndex
            + " near=" + openFailedDialog.near
    }
    // PGG_AUTO_ACT=identity / identity-half: which half landed is a pair of booleans — a dialog left open because the
    // save did not take looks like one nobody has answered.
    WindowStage {
        id: identityTimer
        name: "identity"
        running: Harness.autoAct === "identity" || Harness.autoAct === "identity-half"
        onTriggered: {
            const wholeReady = Harness.autoAct === "identity"
                               && AppBackend.identityState === "missing"
                               && identityDialog.opened
            const halfReady = Harness.autoAct === "identity-half"
                              && AppBackend.identityState === "ready"
                              && AppBackend.identityUnsaved
                              && identityDialog.opened
            if (!Awaited.all("identity_ready", { "ready": wholeReady || halfReady,
                                                 "page": PageSettled.windowSettled(window) }))
                return
            identityTimer.complete(() => "identity state=" + AppBackend.identityState
                                         + " dialog=" + identityDialog.opened
                                         + " nameSaved=" + AppBackend.identityNameSaved
                                         + " emailSaved=" + AppBackend.identityEmailSaved
                                         + " unsaved=" + AppBackend.identityUnsaved
                                         + " badge=" + topBar.identityBadgeShown
                                         + " said=" + (AppBackend.identityError !== ""))
        }
    }

    // PGG_AUTO_ACT=identity-tip: the mark's reason, read where the pointer cannot go. `badge=` is the group in
    // whichever shape the width left it — the mark alone would fail a band saying what it should. Dismissal waits on
    // `identityUnsaved`, the save's answer.
    //
    // Each hop says itself (`step=`) so a run that stalls says where; a line carries only what could have come out the
    // other way. `step=` lines stand apart: `must_say` matches a run of the last line (verbs.md).
    WindowStage {
        id: identityTipTimer
        name: "identity_tip"
        order: ["dismissed", "pointed"]
        running: Harness.autoAct === "identity-tip"
        onTriggered: {
            const op = identityTipTimer
            if (op.at("start")) {
                if (!Awaited.all("identity_tip_dismiss_ready", { "dialog": identityDialog.opened,
                                                                 "unsaved": AppBackend.identityUnsaved,
                                                                 "page": PageSettled.windowSettled(window) }))
                    return
                // Whether the answer took the dialog down in the same call; the guard's two cannot have changed.
                op.input("dismissed", () => {
                    window.dismissIdentity()
                }, () => "identity_tip step=dismissed dialog=" + identityDialog.opened)
                return
            }
            if (op.at("dismissed")) {
                if (!Awaited.all("identity_tip_ready", { "ready": !(identityDialog.opened),
                                                         "page": PageSettled.windowSettled(window) }))
                    return
                // The rest of what the card waits on: the group standing, and placed (`BandStateGroup.asking`).
                op.input("pointed", () => {
                    topBar.statePointedAt = true
                }, () => "identity_tip step=pointed"
                         + " badge=" + (topBar.stateWordsShown || topBar.stateMarkShown)
                         + " placed=" + topBar.statePlaced)
                return
            }
            // The card opens after its tip delay.
            if (!Awaited.all("identity_tip_report_ready", { "ready": topBar.stateCardOpen,
                                                            "page": PageSettled.windowSettled(window) }))
                return
            // `rows=` is read without waiting for the card's layout (unlike `badges-hover`): one row cannot come out of
            // order, so `laidOut=` only says whether the picture has the card at full height.
            op.complete(() => ["identity_tip step=carded laidOut=" + topBar.stateCardLaidOut,
                               "identity_tip unsaved=" + AppBackend.identityUnsaved
                               + " badge=" + (topBar.stateWordsShown || topBar.stateMarkShown)
                               + " tip=" + topBar.stateCardOpen
                               + " rows=" + topBar.stateCardRows])
        }
    }

    // PGG_AUTO_ACT=quit-waits / quit-locked: the close while git is still writing — the commit is held by the sleeping
    // pre-commit hook of `--preset slowhook`, so the gate turns the close away and stands the wait dialog up.
    // `quit-waits` photographs that; `quit-locked` closes again, then watches the held write land anyway.
    //
    // Closed through `window.close()`, as the ✕ and Exit are: once the modal stands it is the only road left (the
    // modal seals the pointer and `Shortcut`s — rules-refs/app-ui.md「アプリ終了の close ゲートは 1 本」). A handler
    // called from QML would run whatever the modal covers.
    //
    // Not the settled page before the landing: the commit moves it, and the landing reads it settled by name.
    WindowStage {
        id: quitTimer
        name: "quit"
        order: Harness.autoAct === "quit-locked" ? ["committed", "closed", "retried"] : ["committed", "closed"]
        running: Harness.autoAct === "quit-waits" || Harness.autoAct === "quit-locked"
        /// The write count as the commit went in: one past it is the held write's answer.
        property int seqBefore: -1
        onTriggered: {
            const op = quitTimer
            const page = window.curPage
            const tab = page !== null ? page.pageTab : null
            if (op.at("start")) {
                if (!Awaited.all("quit_ready", { "ready": !(page === null),
                                                 "open": tab !== null && tab.state === "open" }))
                    return
                op.input("committed", () => {
                    op.seqBefore = tab.writeSeq
                    tab.commit("chore: held by a sleeping hook", "", false, false)
                })
                return
            }
            if (op.at("committed")) {
                // Only while the hook holds the write: it sleeps longer than a beat, so a risen busy count is still up
                // when the close lands.
                if (!Awaited.all("quit_held", { "ready": !(page === null),
                                                "held": tab !== null && tab.busyCount !== 0 }))
                    return
                op.input("closed", () => {
                    window.close()
                })
                return
            }
            if (Harness.autoAct === "quit-waits") {
                if (!Awaited.all("quit_wait_landed", { "ready": !(page === null),
                                                       "dialog": quitWaitDialog.opened,
                                                       "page": PageSettled.windowSettled(window) }))
                    return
                op.complete(() => "quit_wait dialog=" + quitWaitDialog.opened
                    + " window=" + window.visible
                    + " busy=" + (window.curPage.pageTab.busyCount !== 0))
                return
            }
            if (op.at("closed")) {
                // The same guard: a close after the hook let go is rightly let through.
                if (!Awaited.all("quit_lock_held", { "ready": !(page === null),
                                                     "dialog": quitWaitDialog.opened,
                                                     "held": tab !== null && tab.busyCount !== 0 }))
                    return
                op.input("retried", () => {
                    window.close()
                })
                return
            }
            // The held write landed under the lock: `writeSeq` past `seqBefore` is its answer, busy at zero the queue
            // done with it. And reached the screen: the answer comes before the status is read again
            // (`session::write::run_write`), and this commit takes the worktree row off: ending on the answer flips
            // `WipTallyRow` in the census.
            if (!Awaited.all("quit_lock_landed", {
                "ready": !(page === null),
                "dialog": quitWaitDialog.opened,
                "idle": tab !== null && tab.busyCount === 0,
                "landed": tab !== null && tab.writeSeq > op.seqBefore,
                "row": page !== null && !page.pageWorktree.wipRowStands,
                "page": PageSettled.settled(page) }))
                return
            op.complete(() => "quit_lock dialog=" + quitWaitDialog.opened
                + " window=" + window.visible
                + " escape=" + quitWaitDialog.escapes
                + " vetoes=" + quitWaitDialog.vetoes
                + " landed=" + (window.curPage.pageTab.writeSeq > op.seqBefore))
        }
    }

    // PGG_AUTO_ACT=quit-save-held: the close while the app's own config save is out — the identity
    // `WindowIdentityActs` submitted, held by `--fault-hold-save` until the shutdown joins it. No session carries it,
    // so the gate's clause for the hub's saves turns the close away (`Hub::writes_settled`). After the picture the
    // run exits through the gate's yield (`WindowQuitGate`), the hold lets go at `writes-joining`, and xtask reads
    // the run's gitconfig afterwards (`verify::outcome`).
    WindowStage {
        id: quitSaveTimer
        name: "quit_save"
        order: ["closed"]
        running: Harness.autoAct === "quit-save-held"
        onTriggered: {
            const op = quitSaveTimer
            if (op.at("start")) {
                if (!Awaited.all("quit_save_held", { "busy": AppBackend.identityBusy,
                                                     "held": Harness.heldSaves() !== 0 }))
                    return
                op.input("closed", () => {
                    window.close()
                })
                return
            }
            if (!Awaited.all("quit_save_ready", { "dialog": quitWaitDialog.opened,
                                                  "page": PageSettled.windowSettled(window) }))
                return
            op.complete(() => "quit_save dialog=" + quitWaitDialog.opened
                + " window=" + window.visible
                + " vetoes=" + quitWaitDialog.vetoes
                + " busy=" + AppBackend.identityBusy
                + " held=" + Harness.heldSaves())
        }
    }

    // PGG_AUTO_ACT=gate-sweep: the `solo` screen, its words taken from the air (規約 §右のペインの字は掴める) — before
    // any repository opens there is no log to reach. Run in `solo`'s state (the harness holds the real lock), the one
    // where the path shows; the other gate, a git that will not answer, needs a PATH without git and has no verb.
    WindowStage {
        id: gateSweepTimer
        name: "gate_sweep"
        order: ["swept"]
        running: Harness.autoAct === "gate-sweep"
        /// What the sweep came away with (`SweepPad.sweepAir`), read as it swept.
        property string swept: ""
        onTriggered: {
            if (!Awaited.all("gate_sweep_ready", {
                "running": AppBackend.alreadyRunning,
                "gate": gate.visible,
                "held": AppBackend.heldElsewhere !== "",
                "page": PageSettled.windowSettled(window) }))
                return
            // `held=`: an empty gate has air and no fields.
            if (!gateSweepTimer.input("swept", () => {
                    gateSweepTimer.swept = gate.pad.sweepAir(7, "held=" + (AppBackend.heldElsewhere !== ""))
                }))
                return
            gateSweepTimer.complete(() => "gate_sweep " + gateSweepTimer.swept)
        }
    }
}
