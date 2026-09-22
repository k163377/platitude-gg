pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The settings screen's half of the window's PGG_AUTO_ACT harness: the merge editor's list, the repository group,
/// the rail between the categories, and the avatar card.
///
/// A file of its own because these are the verbs that reach into one dialog and nothing else — every one of them
/// wants `settingsDialog` and none of them wants a tab, a page or the band. Built by `WindowAutoActDriver` beside
/// `WindowDialogActs`, which keeps the ones that reach into the rest of the window.
// An `Item` only because `QtObject` has no default property to hold the timers below; it is a
// sizeless holder.
Item {
    id: acts

    required property var window
    required property var settingsDialog

    /// The three panes the screen is made of, named once here. The screen
    /// hands over the two it holds and the git one hands over the third
    /// (`SettingsDialog`).
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
    /// Whether this machine's stock-take of merge editors has been made — the read went out, and it came back.
    ///
    /// **Latched off the flag's own edges, because the flag reads false on both sides of the read** and an empty
    /// list means opposite things there: before it, nobody has asked yet; after it, this machine has none. Only the
    /// second is an answer, and it is the one that ends the two verbs below.
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
            // **An inventory that named nothing is an answer, and it is the end of both of these.** The candidates'
            // second wave is a stock-take of the machine (`git mergetool --tool-help`), so a machine with no merge
            // editor on it answers with an empty list — and an empty list is a card with nothing to drop, which
            // closes itself (`AppCombo.hasList`). Neither the `opened` the settled verb waits for nor the loading
            // edge behind it can come after that, so a run that went on waiting would spend the whole watchdog in
            // silence and be read as a wedge. Said and finished: the run fails on its own report line in
            // the seconds the stock-take takes, naming the machine.
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

    // PGG_AUTO_ACT=settings-repo / settings-repo-pick: the git category's `REPOSITORY OVERRIDE` group, landed on the
    // repository the reader is looking at, and with the chooser's list down. The argument picks a row of the strip for
    // the run that wants a repository other than the front one — through the same call a pick from the list makes
    // (規約 §UI 自動化の因果性).
    //
    // Waited on: the read git answers with (`state === "ready"`), and the list's own `opened`. The category is what
    // the run set on the way in.
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
                // with the window.
                if (acts.repoPane.autoRepoRows === 0)
                    return
                // **The screen has to be showing one repository before another is picked.** The screen lands on the
                // one the reader is in as it opens (`SettingsDialog.onOpened`), and waiting for that read makes the
                // argument below a *switch* — boxes already carrying values, replaced by another
                // repository's.
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
    // having answered. The argument is the path to write, or none for the resting state — an empty box, which is
    // "whichever git PATH resolves" and is what a fresh settings directory comes up holding.
    //
    // **The picture cannot judge this one.** A path is drawn the same whether or not there is a binary at the end of
    // it, and the sentence under the box arrives a subprocess after the box does — so a run that photographed the
    // moment it typed would frame `Asking for the version…` and read as green. What is waited on is the answer
    // itself, and the line says which of the four it was.
    //
    // Typed through the box's own door (`SettingsAppPane.autoTypeGitPath` = text, then the edit being finished with),
    // so the wiring is in the picture (規約 §UI 自動化の因果性). Writing
    // the settings file is this run's to do: `verify::run` gives every run a config directory of its own, so what is
    // written here is the harness's own value (同 §).
    SampleTimer {
        id: gitPathTimer
        running: Harness.autoAct === "settings-git-path"
        /// The path has been typed. The run with no argument photographs what the screen opened on.
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

    // PGG_AUTO_ACT=settings-processes: the application category's `GIT PROCESSES` chapter, with both boxes typed. The
    // argument is `<commands>:<seconds>`, either half empty for what an emptied box means (the default, and never).
    //
    // **The picture cannot judge this one either.** A box holding `8` and a box whose edit was never applied frame
    // the same, and what the page's tick reads is the interval in milliseconds, which no box shows — so the store's
    // own answer is reported, in both units, and the line is what is judged.
    //
    // Typed through the pane's own door (`SettingsAppPane.autoTypeProcesses` = text, then the edit being finished
    // with), so the wiring is in the picture (規約 §UI 自動化の因果性).
    SampleTimer {
        id: processesTimer
        running: Harness.autoAct === "settings-processes"
        /// Both boxes have been typed; the tick after is the store's answer.
        property bool acted: false
        onTriggered: {
            if (!settingsDialog.opened) {
                settingsDialog.openAt("app")
                return
            }
            if (!processesTimer.acted) {
                const halves = Harness.autoActArg.split(":")
                acts.appPane.autoTypeProcesses(halves[0] || "", halves.length > 1 ? halves[1] : "")
                processesTimer.acted = true
                return
            }
            processesTimer.stop()
            Harness.report("git_processes " + acts.appPane.processesTally())
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=settings-git-leave: the way out taken over a git waiting to be applied, and turned down. The path
    // typed is the second git this run was staged with (`--other-git`), which is what the way out has to run into.
    //
    // **Both halves are said.** A screen that stayed is drawn exactly like one nobody asked to close, and the
    // `✕` turning is a shape a run has to be told about — so the line says that the way out was taken, that the
    // screen is still up, and that what is holding it is the offer.
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

    // PGG_AUTO_ACT=settings-eol: the `REPOSITORY OVERRIDE` group's line-ending chapter, picked. The argument is the
    // row, in git's own spelling (`true` / `input` / `false`) or `inherited` for the row that writes nothing.
    //
    // **It is the only level there is.** The screen writes `core.autocrlf` into the repository somebody picked and
    // nowhere else (規約 §設定の画面), which happens to be the only file a run may write anyway — the machine's own
    // configuration belongs to whoever is sitting at it
    // (規約 §UI 自動化の因果性 「harness の環境は … 所有する値だけを設定する」).
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
                // A repository has to be there to write into, and the strip's rows arrive with the
                // window.
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
            // Last, so the picture holds the chapter that was written into.
            settingsDialog.autoShowChapterFoot()
            Harness.report("line_endings " + acts.repoPane.endingsTally())
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=settings-switch: the rail, which is the one way between the categories from inside the screen.
    // Opened on the application category and pressed onto the other through the row's own handler
    // (`SettingsDialog.autoTapCategory`), because every other settings verb sets the category before the screen is up
    // and would leave a dead rail green. What the report reads back is the chapters — `category` is
    // the input side, and a run that read it would be reporting its own press.
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

    // PGG_AUTO_ACT=settings-escape: the way out the screen owns, taken through the same function the `✕` and the
    // Escape shortcut are one line onto (`SettingsDialog.escapeOut`). **A picture cannot answer this one** — a
    // window with no settings screen over it is drawn exactly like one where the screen never opened — so what is
    // judged is the pair of states in the report.
    //
    // It proves the road is there and ends where it says it does. The key reaching the shortcut is beyond a
    // harness with no keyboard (規約 §UI 自動化の因果性).
    SampleTimer {
        id: escapeTimer
        running: Harness.autoAct === "settings-escape"
        /// The screen was up before the way out was taken — the half the shot cannot hold.
        property bool wasOpen: false
        onTriggered: {
            if (!escapeTimer.wasOpen) {
                if (!settingsDialog.opened) {
                    // The argument names the category, because the way out is a different road from each: the git
                    // one has the two chapters a Save stands in front of, and its reads land after the screen is up.
                    settingsDialog.openAt(Harness.autoActArg === "" ? "app" : Harness.autoActArg)
                    return
                }
                // Unsaved is counted only after git has answered for the boxes — a run that pressed the
                // way out mid-read would be photographing the read.
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
            // two chapters thought it was holding an edit, and they are read out of different files. `save=` is the
            // global chapter's Save, read off the button: boxes nobody typed in hold what git holds, and a Save lit
            // over them would be offering to hand git its own answer.
            Harness.report("settings_escape unsaved=" + settingsDialog.unsavedIdentities
                              + " global=" + acts.gitPane.unsavedIsGlobal
                              + " repo=" + acts.gitPane.unsavedIsRepo
                              + " save=" + acts.gitPane.autoSaveOffered
                              + " was_open=" + escapeTimer.wasOpen
                              + " now_open=" + settingsDialog.opened)
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=settings-leave: the way out, taken while an identity chapter is holding an edit git has not been
    // given. The run types into the box the way a keystroke does, then presses the same way out the `✕` and Escape
    // press — and what has to be true afterwards is that the screen is **still there**, with the question standing
    // in its foot.
    //
    // **Both halves are said.** A screen that stayed is drawn like one that was never asked to go, and a foot
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
                // the read that had not landed yet — and boxes typed into stop following it, so the address
                // would stay empty and the Save dark over a name git was never going to be given (observed:
                // `save=false` with the address box showing its placeholder). Both reads, because the count
                // below is over both chapters.
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
            // `save=` is the other half of the edit: the typing that stops the way out is the typing that lights
            // the Save, and the button is what the reader is being sent to.
            Harness.report("settings_leave unsaved=" + settingsDialog.unsavedIdentities
                              + " save=" + acts.gitPane.autoSaveOffered
                              + " asked=" + settingsDialog.askingLeave
                              + " open=" + settingsDialog.opened)
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=settings-sweep: this screen's own words, taken from the air around them — the step between two
    // lines, the room beside a short one, the air the column leaves to the right of itself
    // (規約 §右のペインの字は掴める). The argument names the category, because the two carry different words and only
    // one of them is on screen at a time.
    //
    // **Waited on the category's own answer, not on the screen opening.** The lines here are written out of what git
    // said — the version the chosen binary printed, the identity a commit made there would carry — and a run that
    // sampled the air before those landed would be sampling a column about to grow lines.
    SampleTimer {
        id: sweepTimer
        running: Harness.autoAct === "settings-sweep"
        /// The category asked for, and the one the screen is opened on.
        readonly property string cat: Harness.autoActArg === "" ? "app" : Harness.autoActArg
        /// That category is the one showing. **The setup's own side**: `SweepPad` steps over a hidden item, so a
        /// sweep of the category that is not on screen would walk an empty column — and an empty column is not a
        /// green run, it is a run with nothing in it.
        readonly property bool shown: sweepTimer.cat === "git" ? acts.gitPane.visible
                                                               : acts.appPane.autoAppShown
        onTriggered: {
            if (!settingsDialog.opened) {
                settingsDialog.openAt(sweepTimer.cat)
                return
            }
            if (!sweepTimer.shown)
                return
            // The git category's own two reads — the repository group's identity and its line endings, which is the
            // one that grows a line when it lands; the application category's is the version probe, which stands as
            // a sentence of its own until it answers.
            if (sweepTimer.cat === "git"
                ? (!acts.repoPane.autoRepoReady || !acts.repoPane.autoEndingsReady)
                : AppBackend.gitPathState === "checking")
                return
            sweepTimer.stop()
            // `release=` is the other hand on this screen: the one that hands the keyboard back when a press lands
            // where nothing takes it. **A picture cannot say it** — a field that kept its blue through every press
            // after the drag is drawn exactly like one that was just swept — and neither can a run, which has no
            // pointer to press with. What is said here is that the hand is standing; that a press reaches it is
            // Qt's to answer, and `tests/qml/tst_fieldrelease.qml` asks with a real one.
            // `version=` is the one word on this screen that is not a sentence and not a box: git's own spelling of
            // the version the chosen binary answered with, worn as a chip. A chip that answers a press is drawn
            // exactly like one that does not, so the field behind it is said here (`CodeChip.grabbed`). Only the
            // application category has one.
            Harness.report("settings_sweep "
                + settingsDialog.autoChapterHand.sweepAir(7, "cat=" + sweepTimer.cat
                                                             + " shown=" + sweepTimer.shown
                                                             + " release=" + settingsDialog.caretHand.stands
                                                             + " version=" + acts.appPane.autoVersionGrabbed))
            window.finishAutoAct()
        }
    }

    // PGG_AUTO_ACT=settings-tools-enter: **what Enter in the merge editor's box does on each side of its own list**
    // (デザイン規約 §立っている質問は 1 か所で聞く の一覧が降りている間の Enter). Both halves are pressed in the one
    // run, and the second is what ends it: under the list the key is the list's and the screen has to stand, with
    // the list shut it is the answer and the screen goes. Either half alone passes for a build that never leaves
    // and for one that always does.
    SampleTimer {
        id: toolEnterTimer
        running: Harness.autoAct === "settings-tools-enter"
        /// The screen is up, the stock-take is in, and the list is down. Latched, because what this run presses can
        /// take the screen away and a precondition read every tick would put it straight back up.
        property bool arrived: false
        property bool pressedUnderList: false
        /// Whether the screen was still standing after that press — read on the tick after it, since the way out
        /// this press must not take closes the screen where it stands (`SettingsDialog.escapeOut`).
        property bool stood: false
        property bool pressedShut: false
        onTriggered: {
            if (!toolEnterTimer.arrived) {
                if (!settingsDialog.opened) {
                    settingsDialog.openAt("git")
                    return
                }
                // **A card with a ring in it is a list standing in front of the box**, and standing is the whole of
                // what decides whose key an Enter is — so this run takes the one the box has while the stock-take
                // is still out, and owes that read nothing (`AppCombo.hasList`: a field with a read out has
                // something to open). Waiting for the rows instead ties the run to
                // `git mergetool --tool-help`, which is eight seconds on a quiet machine and was killed at its
                // timeout on a loaded one — leaving a box with nothing to drop and a run with nothing to press
                // (observed).
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

    // PGG_AUTO_ACT=avatar-enter: **Enter in the candidate box finds the file**, which is the one thing left to do
    // with the name it holds (デザイン規約 §アバターを与える). The picker is the platform's window and is in
    // neither PNG, so the report line is the whole of it — the same reading `open-picker` takes.
    SampleTimer {
        id: avatarEnterTimer
        running: Harness.autoAct === "avatar-enter"
        /// A name has been put in the box. The candidates come with the page, so the run waits for one rather than
        /// spelling a name this repository may not carry.
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

    // The same card's avatar half, whose four shots the page opens and this finishes. Each waits on what its own verb
    // produced: the row the store answered the filing with and the picture inside it, that row's `lit`, the candidate
    // list's `opened`, and — for the removal — the row leaving the store on the far side of a hold that runs at its own
    // length (`Metrics.holdMs`). Each wait is on a state.
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
            // Last, so the picture holds the chapter these verbs are about. `AVATARS` is the foot of this category
            // and the window does not reach it from where the screen opens, so a run that photographed the resting
            // position photographed the chapters above the list — the same thing `settings-eol` scrolls for.
            //
            // **Not for the candidate list.** Its popup is placed where the field stood when it opened, and sending
            // the chapters out from under it would leave the list hanging off its own box.
            if (act !== "avatar-combo")
                settingsDialog.autoShowChapterFoot()
            Harness.report("avatar_card rows=" + acts.appPane.autoAvatarRows
                              + " painted=" + acts.appPane.autoAvatarRowPainted(0)
                              + " lit=" + acts.appPane.autoAvatarRowLit(0)
                              + " combo=" + acts.appPane.autoAvatarComboOpen
                              + " removed=" + (avatarCardTimer.rowsBefore > acts.appPane.autoAvatarRows))
            window.finishAutoAct()
        }
    }
}
