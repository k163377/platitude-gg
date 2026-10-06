pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The settings screen's half of the window's PGG_AUTO_ACT harness: the verbs that reach into `settingsDialog` and
/// nothing else (no tab, page or band). The rest of the window's dialog verbs are `WindowDialogActs`.
// `Item`, not `QtObject`: rules-refs/app-ui.md「ドライバの root は `Item`」.
Item {
    id: acts

    required property var window
    required property var settingsDialog

    readonly property var appPane: acts.settingsDialog.autoAppPane
    readonly property var gitPane: acts.settingsDialog.autoGitPane
    readonly property var repoPane: acts.settingsDialog.autoGitPane.autoRepoPane

    /// Stands in for the pointer on an avatar row's Remove, which headless lacks (`SettingsAppPane.pointedAtRow`).
    Binding {
        target: acts.appPane
        property: "pointedAtRow"
        value: Harness.autoAct === "avatar-row-lit" ? 0 : -1
    }

    /// The frame count the last send to the chapters' foot was made at, or -1 before the first.
    property int footSentAtFrame: -1
    /// Whether the chapters stand at their foot in a frame drawn after the last send; sends them there until they do.
    /// The foot is read off the column's height, which can grow after a send: the opened screen's first layout pass
    /// lays every chapter out again, and a send made before it leaves `AVATARS` below the view.
    function chaptersAtFoot() {
        Awaited.at(Harness.autoAct, "foot")
        if (acts.footSentAtFrame >= 0 && acts.window.frameCounter <= acts.footSentAtFrame) {
            // `update()`, asked again each beat: a still offscreen scene swaps nothing unasked
            // (rules-refs/app-ui.md「`frameSwapped` を待つなら頼むのは `window.update()`」).
            acts.window.update()
            return false
        }
        if (acts.footSentAtFrame >= 0 && acts.settingsDialog.autoAtChapterFoot())
            return true
        acts.settingsDialog.autoShowChapterFoot()
        acts.footSentAtFrame = acts.window.frameCounter
        acts.window.update()
        return false
    }

    /// The real loading edge, raised off both `SettingsGitPane.toolsAsked` and the model's own change (the read can be
    /// out by the first, or start after it); the box holds its indicator up until the screen closes.
    property bool toolLoadingSeen: false
    function noteToolLoading() {
        if (Harness.autoAct !== "settings-tools-loading" || !acts.window.curPage)
            return
        if (!acts.window.curPage.pageTab.mergeToolsLoading)
            return
        acts.toolLoadingSeen = true
        acts.gitPane.holdToolLoading = true
    }
    Connections {
        target: acts.gitPane
        function onToolsAsked() {
            acts.noteToolLoading()
        }
        // Candidates arrive in two waves and the configured name in a third; each can knock the value out.
        function onToolChoicesChanged() {
            acts.reportTool()
        }
        function onMergeToolChanged() {
            acts.reportTool()
        }
    }
    /// Whether this machine's stock-take of merge editors went out and came back. Latched off the flag's edges: it
    /// reads false on both sides of the read, and an empty list is an answer ("none") only after it.
    property bool toolsWentOut: false
    property bool toolsCameBack: false
    function noteToolsFlight() {
        if (!acts.window.curPage)
            return
        if (acts.window.curPage.pageTab.mergeToolsLoading)
            acts.toolsWentOut = true
        else if (acts.toolsWentOut)
            acts.toolsCameBack = true
    }
    Connections {
        target: acts.window.curPage ? acts.window.curPage.pageTab : null
        function onMergeToolsLoadingChanged() {
            acts.noteToolLoading()
            acts.noteToolsFlight()
        }
    }
    function reportTool() {
        if (Harness.autoAct === "settings-tools" || Harness.autoAct === "settings-tools-loading")
            Harness.report("merge_editor " + acts.gitPane.toolTally())
    }
    function reportFit() {
        Harness.report("settings_fit reach="
                          + (settingsDialog.chaptersContent <= settingsDialog.chaptersView
                             || settingsDialog.chaptersBarShown)
                          + " content=" + Math.round(settingsDialog.chaptersContent)
                          + " view=" + Math.round(settingsDialog.chaptersView)
                          + " bar=" + settingsDialog.chaptersBarShown)
    }

    // PGG_AUTO_ACT=settings-tools / settings-tools-loading: two separately latched outputs, the real loading edge and
    // the settled choices. Opened on the git category by the menu entry's door, told to bring the list down with it.
    SampleTimer {
        running: Harness.autoAct === "settings-tools"
                 || Harness.autoAct === "settings-tools-loading"
        onTriggered: {
            if (!settingsDialog.opened) {
                settingsDialog.pressToolOnOpen = true
                settingsDialog.openAt("git")
                return
            }
            // A stock-take that named nothing ends both verbs: an empty list is a card with nothing to drop, which
            // closes itself (`AppCombo.hasList`), so neither `opened` nor the loading edge can follow.
            if (acts.toolsCameBack && acts.gitPane.toolChoices.length === 0) {
                stop()
                acts.reportTool()
                Harness.report("merge_editor_none this machine's stock-take named no merge editor, "
                                  + "so the list has nothing to drop")
                window.finishAutoAct()
                return
            }
            const ready = Harness.autoAct === "settings-tools-loading"
                        ? (acts.toolLoadingSeen && acts.gitPane.toolListOpen)
                        : (acts.gitPane.toolsSettled && acts.gitPane.toolChoices.length > 0
                           && acts.gitPane.toolListOpen)
            if (!ready)
                return
            stop()
            acts.reportTool()
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=settings-repo / settings-repo-pick: the git category's `REPOSITORY OVERRIDE` group, with the
    // chooser's list down for `-pick`. The argument picks a strip row other than the front one, through the same call
    // a pick from the list makes (verify-ui implement.md §壊れない動詞の実装と反復).
    SampleTimer {
        id: repoSettingsTimer
        running: Harness.autoAct === "settings-repo" || Harness.autoAct === "settings-repo-pick"
        /// The row of the strip has been picked and the list, where one is wanted, pressed down.
        property bool acted: false
        onTriggered: {
            if (!settingsDialog.opened) {
                settingsDialog.openAt("git")
                return
            }
            if (!repoSettingsTimer.acted) {
                // The strip's rows arrive with the window.
                if (acts.repoPane.autoRepoRows === 0)
                    return
                // Wait for the front repository's read (`SettingsDialog.onOpened`) so the pick below is a switch:
                // boxes already carrying values, replaced by another repository's.
                if (!acts.repoPane.autoRepoReady)
                    return
                if (Harness.autoActArg !== ""
                        && !acts.repoPane.showRepoAt(Number(Harness.autoActArg)))
                    return
                if (Harness.autoAct === "settings-repo-pick")
                    acts.repoPane.autoOfferRepos()
                repoSettingsTimer.acted = true
            }
            if (!acts.repoPane.autoRepoReady)
                return
            if (Harness.autoAct === "settings-repo-pick" && !acts.repoPane.autoRepoComboOpen)
                return
            repoSettingsTimer.stop()
            Harness.report("repo_config " + acts.repoPane.repoTally())
            acts.reportFit()
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=settings-git-path: the application category's `GIT EXECUTABLE` chapter, with the git at the path
    // having answered. The argument is the path to put in the box, or none for the resting (empty) box.
    //
    // The picture cannot judge this: the sentence under the box arrives a subprocess after the box, so a shot at the
    // pick would frame `Asking for the version…` and read green. The answer itself is waited on and reported.
    //
    // Writing the settings file is safe: every run has its own config directory
    // (rules/app-ui.md §UI 自動化「run は自分の環境と状態を建てる」).
    SampleTimer {
        id: gitPathTimer
        running: Harness.autoAct === "settings-git-path"
        /// The path has gone in (with no argument, nothing does).
        property bool acted: false
        onTriggered: {
            if (!settingsDialog.opened) {
                settingsDialog.openAt("app")
                return
            }
            if (!gitPathTimer.acted) {
                // `in-use`: the git already running, the one path that does not offer a restart. `other`: the second
                // git the run was staged with (`--other-git`), which does — on either OS, without naming a path. Each
                // as the chooser answers it: a URL of the whole path (`Harness.fileUrl`).
                if (Harness.autoActArg === "in-use")
                    acts.appPane.autoPickGitPath(Harness.fileUrl(AppBackend.gitPathInUse))
                else if (Harness.autoActArg === "other")
                    acts.appPane.autoPickGitPath(Harness.fileUrl(Harness.otherGit))
                else if (Harness.autoActArg !== "")
                    acts.appPane.autoPickGitPath(Harness.fileUrl(Harness.autoActArg))
                gitPathTimer.acted = true
                return
            }
            // The screen asks as it opens, so an answer is coming either way; after a pick, this is the later ask's.
            if (AppBackend.gitPathState === "" || AppBackend.gitPathState === "checking")
                return
            gitPathTimer.stop()
            // The write the way out makes, with the screen still up for the picture: the file a seeded run started
            // from is read after the run for the spelling it keeps (`verify::seed`).
            settingsDialog.applyFields()
            Harness.report("git_path " + acts.appPane.gitPathTally())
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=settings-processes <commands>: the application category's `GIT PROCESSES` chapter, its box typed
    // through the pane's own door (`SettingsAppPane.autoTypeProcesses`); an empty argument is an emptied box (the
    // default). An unapplied edit frames the same, so the store's own answer is the line judged.
    SampleTimer {
        id: processesTimer
        running: Harness.autoAct === "settings-processes"
        /// The box has been typed; the tick after is the store's answer.
        property bool acted: false
        onTriggered: {
            if (!settingsDialog.opened) {
                settingsDialog.openAt("app")
                return
            }
            if (!processesTimer.acted) {
                acts.appPane.autoTypeProcesses(Harness.autoActArg)
                processesTimer.acted = true
                return
            }
            processesTimer.stop()
            Harness.report("git_processes " + acts.appPane.processesTally())
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=settings-refresh <floor>:<ceiling>:<reading>:<first>:<second>: the application category's `REFRESH`
    // chapter worked through the pane's own doors (`SettingsAppPane.autoTypeRefresh`) — this repository's two boxes, the
    // other copies' chooser (`auto` / `fixed` / `off`), then the boxes that reading shows (`first` and `second` are the
    // two bounds, or `first` the one interval). An empty part is left as it is. The store's own answer is the line
    // judged, with which boxes stand beside the chooser.
    SampleTimer {
        id: refreshTimer
        running: Harness.autoAct === "settings-refresh"
        /// The chapter has been worked; the tick after is the store's answer.
        property bool acted: false
        onTriggered: {
            if (!settingsDialog.opened) {
                settingsDialog.openAt("app")
                return
            }
            if (!refreshTimer.acted) {
                const parts = Harness.autoActArg.split(":")
                acts.appPane.autoTypeRefresh(parts[0] || "", parts[1] || "", parts[2] || "", parts[3] || "",
                                             parts[4] || "")
                refreshTimer.acted = true
                return
            }
            refreshTimer.stop()
            Harness.report("settings_refresh " + acts.appPane.refreshTally())
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=settings-refresh-leave <floor>:<ceiling>:<first>:<second>: the way out taken with the `REFRESH`
    // chapter's four bound boxes holding these texts, never finished (`SettingsAppPane.autoLeaveBounds`) — an empty part
    // is an emptied box, which only the way out writes. Both pairs are first set off their defaults through the
    // chapter's own door, so an emptied box has a default to go back to. The store's answer after the close is judged.
    SampleTimer {
        id: refreshLeaveTimer
        running: Harness.autoAct === "settings-refresh-leave"
        /// 0: the screen is coming up; 1: the pairs are off their defaults; 2: the way out has been taken.
        property int step: 0
        onTriggered: {
            if (refreshLeaveTimer.step === 0) {
                if (!settingsDialog.opened) {
                    settingsDialog.openAt("app")
                    return
                }
                acts.appPane.autoTypeRefresh("7", "20", "auto", "9", "60")
                refreshLeaveTimer.step = 1
                return
            }
            if (refreshLeaveTimer.step === 1) {
                const parts = Harness.autoActArg.split(":")
                acts.appPane.autoLeaveBounds(parts[0] || "", parts[1] || "", parts[2] || "", parts[3] || "")
                settingsDialog.escapeOut()
                refreshLeaveTimer.step = 2
                return
            }
            // The fields are written as the close ends.
            if (settingsDialog.visible)
                return
            refreshLeaveTimer.stop()
            Harness.report("settings_refresh_leave " + acts.appPane.refreshStore() + " open=" + settingsDialog.opened)
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=settings-git-leave: the way out taken over a git waiting to be applied (`--other-git`), and
    // turned down. A screen that stayed frames like one nobody asked to close, so the line says the way out was taken,
    // the screen is still up, and the offer is what holds it.
    SampleTimer {
        id: gitLeaveTimer
        running: Harness.autoAct === "settings-git-leave"
        /// The path has gone in, and the way out has been taken.
        property bool picked: false
        property bool left: false
        onTriggered: {
            if (!settingsDialog.opened) {
                settingsDialog.openAt("app")
                return
            }
            if (!gitLeaveTimer.picked) {
                acts.appPane.autoPickGitPath(Harness.fileUrl(Harness.otherGit))
                gitLeaveTimer.picked = true
                return
            }
            // The offer arrives a subprocess after the pick.
            if (!AppBackend.gitPathOffersRestart)
                return
            if (!gitLeaveTimer.left) {
                settingsDialog.escapeOut()
                gitLeaveTimer.left = true
                return
            }
            gitLeaveTimer.stop()
            Harness.report("settings_git_leave offers=" + AppBackend.gitPathOffersRestart
                           + " armed=" + settingsDialog.askingGitPath
                           + " unsaved=" + settingsDialog.unsavedIdentities
                           + " open=" + settingsDialog.opened)
            Harness.report("git_path " + acts.appPane.gitPathTally())
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=settings-eol <true|input|false|inherited>: the `REPOSITORY OVERRIDE` group's line-ending chapter,
    // picked (`inherited` writes nothing). `core.autocrlf` is written only into the picked repository (規約 §設定の画面),
    // the only file a run may write anyway.
    //
    // Waited on: the read that fills the chooser, then the read that follows the write — the write's own `busy` falls
    // before that read lands, so stopping at it photographs the replaced value.
    SampleTimer {
        id: endingsTimer
        running: Harness.autoAct === "settings-eol"
        /// The row has been picked.
        property bool acted: false
        /// The argument as the model spells it: the row that writes nothing is empty there.
        readonly property string wanted: Harness.autoActArg === "inherited" ? "" : Harness.autoActArg
        onTriggered: {
            if (!settingsDialog.opened) {
                settingsDialog.openAt("git")
                return
            }
            if (!endingsTimer.acted) {
                if (acts.repoPane.autoRepoRows === 0 || !acts.repoPane.autoEndingsReady)
                    return
                if (!acts.repoPane.autoPickEnding(endingsTimer.wanted))
                    return
                endingsTimer.acted = true
            }
            if (!acts.repoPane.autoEndingsReady
                    || acts.repoPane.autoEndingHeld !== endingsTimer.wanted)
                return
            // Last, so the picture holds the chapter that was written into.
            if (!acts.chaptersAtFoot())
                return
            endingsTimer.stop()
            Harness.report("line_endings " + acts.repoPane.endingsTally())
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=settings-switch: the rail, pressed from the application category onto git through the row's own
    // handler (`SettingsDialog.autoTapCategory`) — every other settings verb sets the category before the screen is
    // up and would leave a dead rail green. The report reads the panes, not `category` (that is the press itself).
    SampleTimer {
        id: categorySwitchTimer
        running: Harness.autoAct === "settings-switch"
        /// The press has been made.
        property bool acted: false
        /// The application category was standing first — the half the picture cannot hold.
        property bool wasApp: false
        onTriggered: {
            if (!categorySwitchTimer.acted) {
                if (!settingsDialog.opened) {
                    settingsDialog.openAt("app")
                    return
                }
                if (!acts.appPane.autoAppShown)
                    return
                categorySwitchTimer.wasApp = true
                if (!settingsDialog.autoTapCategory("git"))
                    return
                categorySwitchTimer.acted = true
            }
            if (!acts.gitPane.visible)
                return
            categorySwitchTimer.stop()
            Harness.report("settings_switch was_app=" + categorySwitchTimer.wasApp
                              + " app=" + acts.appPane.autoAppShown
                              + " git=" + acts.gitPane.visible)
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=settings-escape: the way out, through the function the `✕` and the Escape shortcut both call
    // (`SettingsDialog.escapeOut`). A closed screen frames like one that never opened, so the report's pair of states
    // is judged. The key reaching the shortcut is beyond a harness with no keyboard.
    SampleTimer {
        id: escapeTimer
        running: Harness.autoAct === "settings-escape"
        /// The screen was up before the way out was taken — the half the shot cannot hold.
        property bool wasOpen: false
        onTriggered: {
            if (!escapeTimer.wasOpen) {
                if (!settingsDialog.opened) {
                    // The argument names the category: the git one's way out passes the chapters a Save guards.
                    settingsDialog.openAt(Harness.autoActArg === "" ? "app" : Harness.autoActArg)
                    return
                }
                // Unsaved is counted only after git has answered for the boxes, or the run photographs the read.
                if (settingsDialog.category === "git" && !acts.repoPane.autoRepoReady)
                    return
                escapeTimer.wasOpen = true
                settingsDialog.escapeOut()
                return
            }
            if (settingsDialog.opened)
                return
            escapeTimer.stop()
            // `global=` / `repo=` name which chapter thought it held an edit. `save=` is the global chapter's Save,
            // off the button: lit over untouched boxes, it would offer git its own answer.
            Harness.report("settings_escape unsaved=" + settingsDialog.unsavedIdentities
                              + " global=" + acts.gitPane.unsavedIsGlobal
                              + " repo=" + acts.gitPane.unsavedIsRepo
                              + " save=" + acts.gitPane.autoSaveOffered
                              + " was_open=" + escapeTimer.wasOpen
                              + " now_open=" + settingsDialog.opened)
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=settings-leave: the way out taken while an identity chapter holds an edit git has not been given;
    // the screen must still be there, sent to that chapter with its `✕` armed (the screen has no foot to ask in).
    // Both halves are said: a screen that stayed frames like one never asked to go.
    SampleTimer {
        id: leaveTimer
        running: Harness.autoAct === "settings-leave"
        /// The edit has been made and the way out pressed.
        property bool acted: false
        onTriggered: {
            if (!leaveTimer.acted) {
                if (!settingsDialog.opened) {
                    settingsDialog.openAt("git")
                    return
                }
                // Both chapters' reads land before a box is changed: a typed box stops following the read, so the
                // address would stay empty and the Save dark.
                if (AppBackend.identityState !== "ready" || !acts.repoPane.autoRepoReady)
                    return
                if (settingsDialog.unsavedIdentities !== 0)
                    return
                acts.gitPane.autoTypeIdentity("Someone Else")
                if (settingsDialog.unsavedIdentities === 0)
                    return
                settingsDialog.escapeOut()
                leaveTimer.acted = true
            }
            if (!settingsDialog.askingLeave)
                return
            leaveTimer.stop()
            // `save=`: the typing that stops the way out must also light the Save the reader is sent to.
            Harness.report("settings_leave unsaved=" + settingsDialog.unsavedIdentities
                              + " save=" + acts.gitPane.autoSaveOffered
                              + " asked=" + settingsDialog.askingLeave
                              + " open=" + settingsDialog.opened)
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=settings-sweep [app|git]: this screen's words, taken from the air around them
    // (規約 §右のペインの字は掴める); one category per run, as only one is on screen. Waited on the category's own git
    // reads, not the screen opening: sampled before them, the column is about to grow lines.
    SampleTimer {
        id: sweepTimer
        running: Harness.autoAct === "settings-sweep"
        readonly property string cat: Harness.autoActArg === "" ? "app" : Harness.autoActArg
        /// That category is showing: `SweepPad` steps over a hidden item, so the other would walk an empty column.
        readonly property bool shown: sweepTimer.cat === "git" ? acts.gitPane.visible
                                                               : acts.appPane.autoAppShown
        onTriggered: {
            if (!settingsDialog.opened) {
                settingsDialog.openAt(sweepTimer.cat)
                return
            }
            if (!sweepTimer.shown)
                return
            // git: the repository group's identity and line endings (the latter grows a line); app: the version probe.
            if (sweepTimer.cat === "git"
                ? (!acts.repoPane.autoRepoReady || !acts.repoPane.autoEndingsReady)
                : AppBackend.gitPathState === "checking")
                return
            sweepTimer.stop()
            // `release=`: the hand that gives the keyboard back when a press lands where nothing takes it is standing
            // (no picture can say it; that a press reaches it is `tests/qml/tst_fieldrelease.qml`'s). `version=`: the
            // app category's version chip has its grabbable field behind it (`CodeChip.grabbed`).
            Harness.report("settings_sweep "
                + settingsDialog.autoChapterHand.sweepAir(7, "cat=" + sweepTimer.cat
                                                             + " shown=" + sweepTimer.shown
                                                             + " release=" + settingsDialog.caretHand.stands
                                                             + " version=" + acts.appPane.autoVersionGrabbed))
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=settings-tools-enter: Enter in the merge editor's box on each side of its list
    // (デザイン規約 §選ぶ欄と打つ欄「ただし一覧が降りている間の Enter は一覧のもの」): under the list the screen stands,
    // with the list shut it goes. Both in one run — either alone passes a build that never or always leaves.
    SampleTimer {
        id: toolEnterTimer
        running: Harness.autoAct === "settings-tools-enter"
        /// The screen is up and the list is down. Latched: this run's presses can take the screen away, and a
        /// precondition read every tick would put it back up.
        property bool arrived: false
        property bool pressedUnderList: false
        /// Whether the screen still stood after that press — read a tick later, as the wrong way out closes it where
        /// it stands (`SettingsDialog.escapeOut`).
        property bool stood: false
        property bool pressedShut: false
        onTriggered: {
            if (!toolEnterTimer.arrived) {
                if (!settingsDialog.opened) {
                    settingsDialog.openAt("git")
                    return
                }
                // A card with a loading ring is a list standing in front of the box, which alone decides whose Enter
                // it is — so no rows are needed (`AppCombo.hasList`). Waiting for them would tie the run to
                // `git mergetool --tool-help`, which can hit its timeout on a loaded machine.
                if (acts.gitPane.toolsSettled && acts.gitPane.toolChoices.length === 0)
                    return
                // Asked again while it is shut, for the turn of the loop the field puts between the press and the
                // list (`AppCombo.pressField`).
                if (!acts.gitPane.toolListOpen) {
                    acts.gitPane.pressToolField()
                    return
                }
                toolEnterTimer.arrived = true
            }
            if (!toolEnterTimer.pressedUnderList) {
                acts.gitPane.autoEnterTool()
                toolEnterTimer.pressedUnderList = true
                return
            }
            if (!toolEnterTimer.pressedShut) {
                toolEnterTimer.stood = settingsDialog.opened
                acts.gitPane.autoShutToolList()
                acts.gitPane.autoEnterTool()
                toolEnterTimer.pressedShut = true
                return
            }
            if (settingsDialog.opened)
                return
            toolEnterTimer.stop()
            Harness.report("merge_editor_enter stood=" + toolEnterTimer.stood + " left=true")
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=avatar-enter: Enter in the candidate box opens the file picker (デザイン規約 §アバターを与える).
    // The picker is the platform's window and in neither PNG, so the report line is the whole verdict.
    SampleTimer {
        id: avatarEnterTimer
        running: Harness.autoAct === "avatar-enter"
        /// A candidate's name has been put in the box (a spelled one may not be in this repository).
        property bool typed: false
        onTriggered: {
            if (!settingsDialog.opened)
                return
            if (!avatarEnterTimer.typed) {
                if (acts.appPane.authorChoices.length === 0)
                    return
                acts.appPane.autoTypeAvatarWho(acts.appPane.authorChoices[0])
                avatarEnterTimer.typed = true
                return
            }
            if (!acts.appPane.autoAvatarPickerOpen) {
                acts.appPane.autoEnterAvatarWho()
                return
            }
            avatarEnterTimer.stop()
            Harness.report("avatar_enter who=" + (acts.appPane.chosenEmail !== "")
                              + " picker=" + acts.appPane.autoAvatarPickerOpen)
            window.finishAutoAct()
        }
    }

    // The avatar card's four verbs: the page opens them, this finishes each on its own output — the filed row and its
    // picture, that row's `lit`, the candidate list's `opened`, and the row gone after the hold.
    SampleTimer {
        id: avatarCardTimer
        running: Harness.autoAct === "avatar-settings" || Harness.autoAct === "avatar-row-lit"
                 || Harness.autoAct === "avatar-combo" || Harness.autoAct === "avatar-remove"
        /// This verb's move has been made; its preconditions are not read again — the removal's answer is the row
        /// going away (the wait `middle-close` describes).
        property bool acted: false
        /// Rows before the hold, read only in the branch that presses.
        property int rowsBefore: -1
        onTriggered: {
            const act = Harness.autoAct
            if (!avatarCardTimer.acted) {
                if (!settingsDialog.opened)
                    return
                // A run that filed a picture on its way in waits for its row; one that filed nothing (the round-trip
                // read) takes what the store has.
                if (Harness.autoActArg !== "" && !acts.appPane.autoAvatarRowPainted(0))
                    return
                if (act === "avatar-row-lit") {
                    if (!acts.appPane.autoAvatarRowLit(0))
                        return
                } else if (act === "avatar-combo") {
                    // Asked again while shut: the field defers the list by a turn of the loop, and a list taken back
                    // down under an unwinding grab needs a second ask (`AppCombo.pressField`).
                    if (!acts.appPane.autoAvatarComboOpen) {
                        acts.appPane.autoAvatarOfferCombo()
                        return
                    }
                } else if (act === "avatar-remove") {
                    if (!acts.appPane.autoAvatarHoldRemove(0))
                        return
                    avatarCardTimer.rowsBefore = acts.appPane.autoAvatarRows
                }
                avatarCardTimer.acted = true
            }
            // The removal's answer arrives after the hold: the store letting the row go.
            if (act === "avatar-remove" && acts.appPane.autoAvatarRows >= avatarCardTimer.rowsBefore)
                return
            // Last, so the picture holds `AVATARS`, the category's foot, out of the resting view's reach. Not for the
            // candidate list: its popup stays where the field stood, and scrolling would leave it off its box.
            if (act !== "avatar-combo" && !acts.chaptersAtFoot())
                return
            avatarCardTimer.stop()
            Harness.report("avatar_card rows=" + acts.appPane.autoAvatarRows
                              + " painted=" + acts.appPane.autoAvatarRowPainted(0)
                              + " lit=" + acts.appPane.autoAvatarRowLit(0)
                              + " combo=" + acts.appPane.autoAvatarComboOpen
                              + " removed=" + (avatarCardTimer.rowsBefore > acts.appPane.autoAvatarRows))
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=settings-hand <chapters|tools>: the middle button's hand on the chapters or the merge editor's card,
    // pressed and left drifting for the shot (the page's `middle-hand` on this screen); waited for is the surface
    // having gone. `tools` wants `--preset mergetools`: most machines name fewer editors than the card's eight rows.
    SampleTimer {
        id: handTimer
        running: Harness.autoAct === "settings-hand"
        /// What the surface was last seen waiting for, named each time it changes.
        property string waitingFor: ""
        property bool pressed: false
        property real fromAt: 0
        property bool took: false
        /// The hand's frame on the last tick: pressed once it has stood still for one, so the ring anchors in the
        /// card that is there, not one still opening.
        property string seatSize: ""
        readonly property bool tools: Harness.autoActArg === "tools"
        function hand() {
            return handTimer.tools ? acts.gitPane.toolListHand : settingsDialog.autoMiddleHand
        }
        function at() {
            return handTimer.tools ? acts.gitPane.toolListAt : settingsDialog.chaptersAt
        }
        /// What the surface still waits for, or "" once it holds more than room. The card's list is the hand's
        /// parent (`AppCombo`).
        function waitingOn() {
            // Re-asked until the screen stands; told to bring the card down where it is the surface (as
            // `settings-tools`).
            if (!settingsDialog.opened) {
                settingsDialog.pressToolOnOpen = handTimer.tools
                settingsDialog.openAt("git")
                return "the screen"
            }
            if (!handTimer.tools)
                return settingsDialog.chaptersContent > settingsDialog.chaptersView + 1 ? "" : "chapters past the view"
            if (!acts.gitPane.toolListOpen || !acts.gitPane.toolsSettled)
                return "the candidates"
            const list = acts.gitPane.toolListHand.parent
            return list.contentHeight > list.height + 1 ? "" : "more candidates than room"
        }
        onTriggered: {
            const hand = handTimer.hand()
            if (!handTimer.pressed) {
                const wait = handTimer.waitingOn()
                if (wait !== "") {
                    if (wait !== handTimer.waitingFor) {
                        handTimer.waitingFor = wait
                        Harness.report("settings_hand surface=" + Harness.autoActArg + " waiting=" + wait)
                    }
                    return
                }
                const size = hand.width + "x" + hand.height
                if (size !== handTimer.seatSize) {
                    handTimer.seatSize = size
                    return
                }
                handTimer.fromAt = handTimer.at()
                // The first point down the middle nothing claims: a box on the chapters keeps the middle press where
                // the platform pastes with it (`MiddleAutoScroll.claimedAt`).
                let x = hand.width / 2
                let y = hand.height / 2
                for (const fy of [0.5, 0.25, 0.75, 0.1, 0.9]) {
                    if (!hand.claimedAt(x, hand.height * fy)) {
                        y = hand.height * fy
                        break
                    }
                }
                handTimer.took = hand.press(x, y)
                if (handTimer.took)
                    hand.drift(x, y + Metrics.middleScrollDeadZone + 64)
                handTimer.pressed = true
                Harness.report("settings_hand surface=" + Harness.autoActArg + " pressed took=" + handTimer.took)
                return
            }
            const went = Math.abs(handTimer.at() - handTimer.fromAt) > 8
            if (handTimer.took && (!went || hand.ticks < 2))
                return
            handTimer.stop()
            Harness.report("settings_hand surface=" + Harness.autoActArg
                           + " answered=" + (handTimer.took && went && hand.scrolling)
                           + " took=" + handTimer.took
                           + " moved=" + went
                           + " scrolling=" + hand.scrolling
                           + " ticks=" + hand.ticks)
            window.finishAutoAct()
        }
    }
}
