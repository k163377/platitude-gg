pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The settings screen's half of the window's PG_AUTO_ACT harness: the merge editor's list, the repository group,
/// the rail between the categories, and the avatar card.
///
/// A file of its own because these are the verbs that reach into one dialog and nothing else — every one of them
/// wants `settingsDialog` and none of them wants a tab, a page or the band. Built by `WindowAutoActDriver` beside
/// `WindowDialogActs`, which keeps the ones that reach into the rest of the window.
// An `Item` only because `QtObject` has no default property to hold the timers below; it draws nothing and is
// never given a size.
Item {
    id: acts

    required property var window
    required property var settingsDialog

    /// The three panes the screen is made of, named once here rather than
    /// spelled out at every question below. The screen hands over the two
    /// it holds and the git one hands over the third (`SettingsDialog`).
    readonly property var appPane: acts.settingsDialog.autoAppPane
    readonly property var gitPane: acts.settingsDialog.autoGitPane
    readonly property var repoPane: acts.settingsDialog.autoGitPane.autoRepoPane

    /// Headless has no pointer to put on an avatar row's Remove, and the lit button is what the dim/bright pair is
    /// photographed by (`SettingsAppPane.pointedAtRow`).
    Binding {
        target: acts.appPane
        property: "pointedAtRow"
        value: Harness.autoAct === "avatar-row-lit" ? 0 : -1
    }

    /// The real loading edge, kept alive until the picture has been grabbed. Raised off the moment git was asked
    /// (`SettingsGitPane.toolsAsked`) and off the model's own change, because the read can already be out by the
    /// first and can start after it; the box is told to hold its indicator up, and the screen closing puts that down.
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
        // The candidates arrive in two waves and the configured name in a third, so the value has three chances to be
        // knocked out by something that is not a person — report it at each.
        function onToolChoicesChanged() {
            acts.reportTool()
        }
        function onMergeToolChanged() {
            acts.reportTool()
        }
    }
    Connections {
        target: acts.window.curPage ? acts.window.curPage.pageTab : null
        function onMergeToolsLoadingChanged() {
            acts.noteToolLoading()
        }
    }
    function reportTool() {
        if (Harness.autoAct === "settings-tools" || Harness.autoAct === "settings-tools-loading")
            Harness.report("merge_editor " + acts.gitPane.toolTally())
    }
    /// Whether every chapter can be got to: they fit, or the bar that sends them is standing. The claim is the
    /// harness's; the three lengths it is made of are the screen's (`SettingsDialog.chaptersContent`).
    function reportFit() {
        Harness.report("settings_fit reach="
                          + (settingsDialog.chaptersContent <= settingsDialog.chaptersView
                             || settingsDialog.chaptersBarShown)
                          + " content=" + Math.round(settingsDialog.chaptersContent)
                          + " view=" + Math.round(settingsDialog.chaptersView)
                          + " bar=" + settingsDialog.chaptersBarShown)
    }

    // The tools popup has two separately latched output states: a real loading edge and the populated, settled
    // choices. The chapter it stands in is the settings screen's git one, so the screen is opened on that category —
    // the same door the menu entry uses, told on the way in to bring the list down with it.
    SampleTimer {
        running: Harness.autoAct === "settings-tools"
                 || Harness.autoAct === "settings-tools-loading"
        onTriggered: {
            if (!settingsDialog.opened) {
                settingsDialog.pressToolOnOpen = true
                settingsDialog.openAt("git")
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

    // PG_AUTO_ACT=settings-repo / settings-repo-pick: the git category's `REPOSITORY OVERRIDE` group, landed on the
    // repository the reader is looking at, and with the chooser's list down. The argument picks a row of the strip for
    // the run that wants a repository other than the front one — through the same call a pick from the list makes,
    // not by writing the model's path (規約 §UI 自動化の因果性).
    //
    // Waited on: the read git answers with (`state === "ready"`), and the list's own `opened`. Not the category, which
    // is what the run set on the way in.
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
                // A repository has to be there to pick before anything is asked of it, and the strip's rows arrive
                // with the window rather than with the screen.
                if (acts.repoPane.autoRepoRows === 0)
                    return
                // **The screen has to be showing one repository before another is picked.** The screen lands on the
                // one the reader is in as it opens (`SettingsDialog.onOpened`), and waiting for that read makes the
                // argument below a *switch* — boxes already carrying values, replaced by another repository's —
                // rather than a first look that happens to name a row. The two are not the same road.
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

    // PG_AUTO_ACT=settings-git-path: the application category's `GIT EXECUTABLE` chapter, with the git at the path
    // having answered. The argument is the path to write, or none for the resting state — an empty box, which is
    // "whichever git PATH resolves" and is what a fresh settings directory comes up holding.
    //
    // **The picture cannot judge this one.** A path is drawn the same whether or not there is a binary at the end of
    // it, and the sentence under the box arrives a subprocess after the box does — so a run that photographed the
    // moment it typed would frame `Asking for the version…` and read as green. What is waited on is the answer
    // itself, and the line says which of the four it was.
    //
    // Typed through the box's own door (`SettingsAppPane.autoTypeGitPath` = text, then the edit being finished with),
    // never by writing `AppBackend.gitPath` — that would photograph the wiring cut (規約 §UI 自動化の因果性). Writing
    // the settings file is this run's to do: `verify::run` gives every run a config directory of its own, so what is
    // written here is the harness's own value and not the machine's (同 §).
    SampleTimer {
        id: gitPathTimer
        running: Harness.autoAct === "settings-git-path"
        /// The path has been typed. The run with no argument never types, and photographs what the screen opened on.
        property bool acted: false
        onTriggered: {
            if (!settingsDialog.opened) {
                settingsDialog.openAt("app")
                return
            }
            if (!gitPathTimer.acted) {
                // `in-use` is the path this run is already spawning — the box holding it is a box holding the git
                // already running, which is the one thing that does *not* offer a restart. `other` is the second git
                // the run was staged with (`--other-git`), which is what does: a git that answers and is not this
                // one, on either OS, without the argument naming a path.
                if (Harness.autoActArg === "in-use")
                    acts.appPane.autoTypeGitPath(AppBackend.gitPathInUse)
                else if (Harness.autoActArg === "other")
                    acts.appPane.autoTypeGitPath(Harness.otherGit)
                else if (Harness.autoActArg !== "")
                    acts.appPane.autoTypeGitPath(Harness.autoActArg)
                gitPathTimer.acted = true
                return
            }
            // The screen asks as it opens, so there is an answer coming either way; the run that typed has a second
            // one after it, and this is the state of whichever ask is outstanding.
            if (AppBackend.gitPathState === "" || AppBackend.gitPathState === "checking")
                return
            gitPathTimer.stop()
            Harness.report("git_path " + acts.appPane.gitPathTally())
            window.finishAutoAct()
        }
    }

    // PG_AUTO_ACT=settings-git-leave: the way out taken over a git waiting to be applied, and turned down. The path
    // typed is the second git this run was staged with (`--other-git`), which is what the way out has to run into.
    //
    // **Neither half is a picture.** A screen that stayed is drawn exactly like one nobody asked to close, and the
    // `✕` turning is a shape a run has to be told about — so the line says that the way out was taken, that the
    // screen is still up, and that what is holding it is the offer rather than an unsaved identity.
    SampleTimer {
        id: gitLeaveTimer
        running: Harness.autoAct === "settings-git-leave"
        /// The path has been typed, and the way out has been taken.
        property bool typed: false
        property bool left: false
        onTriggered: {
            if (!settingsDialog.opened) {
                settingsDialog.openAt("app")
                return
            }
            if (!gitLeaveTimer.typed) {
                acts.appPane.autoTypeGitPath(Harness.otherGit)
                gitLeaveTimer.typed = true
                return
            }
            // The offer is what the way out has to run into, and it arrives a subprocess after the typing.
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

    // PG_AUTO_ACT=settings-eol: the `REPOSITORY OVERRIDE` group's line-ending chapter, picked. The argument is the
    // row, in git's own spelling (`true` / `input` / `false`) or `inherited` for the row that writes nothing.
    //
    // **It is the only level there is.** The screen writes `core.autocrlf` into the repository somebody picked and
    // nowhere else (規約 §設定の画面), which happens to be the only file a run may write anyway — the machine's own
    // configuration belongs to whoever is sitting at it, not to the run
    // (規約 §UI 自動化の因果性 「harness は … その harness が所有する値だけを設定する」).
    //
    // Waited on: the read that fills the chooser (a pick before it would be picking against an empty field), then the
    // read that *follows* the write — the write's own `busy` falls before that one lands, so a run that stopped at it
    // would photograph the value it had just replaced.
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
                // A repository has to be there to write into, and the strip's rows arrive with the window rather
                // than with the screen.
                if (acts.repoPane.autoRepoRows === 0 || !acts.repoPane.autoEndingsReady)
                    return
                if (!acts.repoPane.autoPickEnding(endingsTimer.wanted))
                    return
                endingsTimer.acted = true
            }
            if (!acts.repoPane.autoEndingsReady
                    || acts.repoPane.autoEndingHeld !== endingsTimer.wanted)
                return
            endingsTimer.stop()
            // Last, so the picture holds the chapter that was written into rather than the one the screen rests on.
            settingsDialog.autoShowChapterFoot()
            Harness.report("line_endings " + acts.repoPane.endingsTally())
            window.finishAutoAct()
        }
    }

    // PG_AUTO_ACT=settings-switch: the rail, which is the one way between the categories that is not a door into
    // the screen. Opened on the application category and pressed onto the other through the row's own handler
    // (`SettingsDialog.autoTapCategory`), because every other settings verb sets the category before the screen is up
    // and would leave a dead rail green. What the report reads back is the chapters, not `category` — that is the
    // input side, and a run that read it would be reporting its own press.
    SampleTimer {
        id: categorySwitchTimer
        running: Harness.autoAct === "settings-switch"
        /// The press has been made, so what had to be true before it is not read again.
        property bool acted: false
        /// The application category was standing first — half the claim, and the half the picture cannot hold.
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

    // PG_AUTO_ACT=settings-escape: the way out the screen owns, taken through the same function the `✕` and the
    // Escape shortcut are one line onto (`SettingsDialog.escapeOut`). **A picture cannot answer this one** — a
    // window with no settings screen over it is drawn exactly like one where the screen never opened — so what is
    // judged is the pair of states in the report, not the shot.
    //
    // What it does not prove is that the key reaches the shortcut; nothing headless can post one (規約 §UI 自動化の
    // 因果性 — the harness has no keyboard). It proves the road is there and ends where it says it does.
    SampleTimer {
        id: escapeTimer
        running: Harness.autoAct === "settings-escape"
        /// The screen was up before the way out was taken — the half the shot cannot hold.
        property bool wasOpen: false
        onTriggered: {
            if (!escapeTimer.wasOpen) {
                if (!settingsDialog.opened) {
                    // The argument names the category, because the way out is not the same road from both: the git
                    // one has the two chapters a Save stands in front of, and its reads land after the screen is up.
                    settingsDialog.openAt(Harness.autoActArg === "" ? "app" : Harness.autoActArg)
                    return
                }
                // Nothing may be counted as unsaved before git has answered for the boxes — a run that pressed the
                // way out mid-read would be photographing the read rather than the way out.
                if (settingsDialog.category === "git" && !acts.repoPane.autoRepoReady)
                    return
                escapeTimer.wasOpen = true
                settingsDialog.escapeOut()
                return
            }
            if (settingsDialog.opened)
                return
            escapeTimer.stop()
            // The two halves of `unsaved` are named apart: a way out that stopped says nothing about *which* of the
            // two chapters thought it was holding an edit, and they are read out of different files.
            Harness.report("settings_escape unsaved=" + settingsDialog.unsavedIdentities
                              + " global=" + acts.gitPane.unsavedIsGlobal
                              + " repo=" + acts.gitPane.unsavedIsRepo
                              + " was_open=" + escapeTimer.wasOpen
                              + " now_open=" + settingsDialog.opened)
            window.finishAutoAct()
        }
    }

    // PG_AUTO_ACT=settings-leave: the way out, taken while an identity chapter is holding an edit git has not been
    // given. The run types into the box the way a keystroke does, then presses the same way out the `✕` and Escape
    // press — and what has to be true afterwards is that the screen is **still there**, with the question standing
    // in its foot.
    //
    // **Neither half is a picture.** A screen that stayed is drawn like one that was never asked to go, and a foot
    // carrying the question is drawn like a foot carrying anything else until it is read.
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
                // The boxes have to be holding git's answer before one of them is changed, or the "edit" is only
                // the read that had not landed yet.
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
            Harness.report("settings_leave unsaved=" + settingsDialog.unsavedIdentities
                              + " asked=" + settingsDialog.askingLeave
                              + " open=" + settingsDialog.opened)
            window.finishAutoAct()
        }
    }

    // The same card's avatar half, whose four shots the page opens and this finishes. Each waits on what its own verb
    // produced: the row the store answered the filing with and the picture inside it, that row's `lit`, the candidate
    // list's `opened`, and — for the removal — the row leaving the store on the far side of a hold that runs at its own
    // length (`Metrics.holdMs`). Nothing here reads a clock.
    SampleTimer {
        id: avatarCardTimer
        running: Harness.autoAct === "avatar-settings" || Harness.autoAct === "avatar-row-lit"
                 || Harness.autoAct === "avatar-combo" || Harness.autoAct === "avatar-remove"
        /// Raised once this verb's own move has been made, so nothing after it re-reads what had to be true before it.
        /// The removal's answer is a row going away, and a gate still wanting that row would never let go of it (the
        /// wait `middle-close` describes).
        property bool acted: false
        /// How many rows the hold was made against, read in the branch that presses and nowhere else.
        property int rowsBefore: -1
        onTriggered: {
            const act = Harness.autoAct
            if (!avatarCardTimer.acted) {
                if (!settingsDialog.opened)
                    return
                // A run that filed a picture on its way in has to have it in the list before any of this means
                // anything; one that filed nothing — the round-trip read — has whatever the store gave it.
                if (Harness.autoActArg !== "" && !acts.appPane.autoAvatarRowPainted(0))
                    return
                if (act === "avatar-row-lit") {
                    if (!acts.appPane.autoAvatarRowLit(0))
                        return
                } else if (act === "avatar-combo") {
                    // Asked again while it is still shut: the field defers the list by a turn of the loop, and a list
                    // taken back down under an unwinding grab has to be asked for a second time (`AppCombo.pressField`).
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
            // The hold is the one move whose answer arrives after it: the store has to have let the row go.
            if (act === "avatar-remove" && acts.appPane.autoAvatarRows >= avatarCardTimer.rowsBefore)
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
}
