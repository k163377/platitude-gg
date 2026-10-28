pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The settings screen's half of the window's PGG_AUTO_ACT harness: the verbs that reach into `settingsDialog` and
/// nothing else (no tab, page or band). The rest of the window's dialog verbs are `WindowDialogActs`.
///
/// A verb that opens the screen itself asks for it on every beat it is not up (an idempotent ask, repeated until its
/// effect shows): the screen is no stage of the verb's.
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

    /// How many sends to the chapters' foot have been made: each names its own wait for a frame drawn after it.
    property int footSends: 0
    /// Whether the chapters stand at their foot in a frame drawn after the last send (`op`'s `drawnSince`); sends
    /// them there until they do. The foot is read off the column's height, which can grow after a send: the opened
    /// screen's first layout pass lays every chapter out again, and a send made before it leaves `AVATARS` below the
    /// view.
    function chaptersAtFoot(op) {
        const sent = acts.footSends > 0
        if (sent && !op.drawnSince("chapters_foot/" + acts.footSends))
            return false
        if (sent && acts.settingsDialog.autoAtChapterFoot())
            return true
        acts.settingsDialog.autoShowChapterFoot()
        acts.footSends++
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
            acts.noteTool()
        }
        function onMergeToolChanged() {
            acts.noteTool()
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
    /// The box's whole state, as the verbs that judge it say it.
    function toolLine() {
        return "merge_editor " + acts.gitPane.toolTally()
    }
    /// The box as each arrival leaves it: a diagnosis of the run, judged by the line its completion reads.
    function noteTool() {
        if (Harness.autoAct === "settings-tools" || Harness.autoAct === "settings-tools-loading")
            Awaited.note(acts.toolLine())
    }
    function fitLine() {
        return "settings_fit reach="
               + (settingsDialog.chaptersContent <= settingsDialog.chaptersView || settingsDialog.chaptersBarShown)
               + " content=" + Math.round(settingsDialog.chaptersContent)
               + " view=" + Math.round(settingsDialog.chaptersView)
               + " bar=" + settingsDialog.chaptersBarShown
    }

    // PGG_AUTO_ACT=settings-tools / settings-tools-loading: two separately latched outputs, the real loading edge and
    // the settled choices. Opened on the git category by the menu entry's door, told to bring the list down with it.
    WindowStage {
        id: toolsTimer
        name: "settings_tools"
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
                if (!Awaited.all("settings_tools_none", { "page": PageSettled.windowSettled(window) }))
                    return
                toolsTimer.complete(() => [acts.toolLine(),
                                           "merge_editor_none this machine's stock-take named no merge editor, "
                                           + "so the list has nothing to drop"])
                return
            }
            const ready = Harness.autoAct === "settings-tools-loading"
                        ? (acts.toolLoadingSeen && acts.gitPane.toolListOpen)
                        : (acts.gitPane.toolsSettled && acts.gitPane.toolChoices.length > 0
                           && acts.gitPane.toolListOpen)
            if (!Awaited.all("settings_tools_ready", { "ready": ready,
                                                       "page": PageSettled.windowSettled(window) }))
                return
            toolsTimer.complete(() => acts.toolLine())
        }
    }

    // PGG_AUTO_ACT=settings-repo / settings-repo-pick: the git category's `REPOSITORY OVERRIDE` group, with the
    // chooser's list down for `-pick`. The argument picks a strip row other than the front one, through the same call
    // a pick from the list makes (verify-ui implement.md §壊れない動詞の実装と反復).
    WindowStage {
        id: repoSettingsTimer
        name: "repo_settings"
        /// The row of the strip chosen (the front one stays where no argument names another) and the list, where one
        /// is wanted, pressed down.
        order: ["chosen"]
        running: Harness.autoAct === "settings-repo" || Harness.autoAct === "settings-repo-pick"
        onTriggered: {
            if (!settingsDialog.opened) {
                settingsDialog.openAt("git")
                return
            }
            if (repoSettingsTimer.at("start")) {
                // The strip's rows arrive with the window.
                if (acts.repoPane.autoRepoRows === 0)
                    return
                // Wait for the front repository's read (`SettingsDialog.onOpened`) so the pick below is a switch:
                // boxes already carrying values, replaced by another repository's.
                if (!acts.repoPane.autoRepoReady)
                    return
                // The strip's answer is the press's: a row it does not have yet is chosen again.
                if (!repoSettingsTimer.input("chosen", () => {
                        if (Harness.autoActArg !== "" && !acts.repoPane.showRepoAt(Number(Harness.autoActArg)))
                            return false
                        if (Harness.autoAct === "settings-repo-pick")
                            acts.repoPane.autoOfferRepos()
                    }))
                    return
            }
            if (!Awaited.all("repo_settings_ready", {
                "read": acts.repoPane.autoRepoReady,
                "list": Harness.autoAct !== "settings-repo-pick" || acts.repoPane.autoRepoComboOpen,
                "page": PageSettled.windowSettled(window) }))
                return
            repoSettingsTimer.complete(() => ["repo_config " + acts.repoPane.repoTally(), acts.fitLine()])
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
    WindowStage {
        id: gitPathTimer
        name: "git_path"
        /// The path put in (with no argument, nothing is), then the write the way out makes.
        order: ["picked", "applied"]
        running: Harness.autoAct === "settings-git-path"
        onTriggered: {
            if (!settingsDialog.opened) {
                settingsDialog.openAt("app")
                return
            }
            if (gitPathTimer.at("start")) {
                // `in-use`: the git already running, the one path that does not offer a restart. `other`: the second
                // git the run was staged with (`--other-git`), which does — on either OS, without naming a path. Each
                // as the chooser answers it: a URL of the whole path (`Harness.fileUrl`).
                gitPathTimer.input("picked", () => {
                    if (Harness.autoActArg === "in-use")
                        acts.appPane.autoPickGitPath(Harness.fileUrl(AppBackend.gitPathInUse))
                    else if (Harness.autoActArg === "other")
                        acts.appPane.autoPickGitPath(Harness.fileUrl(Harness.otherGit))
                    else if (Harness.autoActArg !== "")
                        acts.appPane.autoPickGitPath(Harness.fileUrl(Harness.autoActArg))
                })
                return
            }
            // The screen asks as it opens, so an answer is coming either way; after a pick, this is the later ask's.
            if (!Awaited.all("git_path_answered", {
                "state": AppBackend.gitPathState !== "" && AppBackend.gitPathState !== "checking",
                "page": PageSettled.settledUnder(window) }))
                return
            // The write the way out makes, with the screen still up for the picture: the file a seeded run started
            // from is read after the run for the spelling it keeps (`verify::seed`).
            gitPathTimer.input("applied", () => settingsDialog.applyFields())
            gitPathTimer.complete(() => "git_path " + acts.appPane.gitPathTally())
        }
    }

    // PGG_AUTO_ACT=settings-processes <commands>: the application category's `GIT PROCESSES` chapter, its box typed
    // through the pane's own door (`SettingsAppPane.autoTypeProcesses`); an empty argument is an emptied box (the
    // default). An unapplied edit frames the same, so the store's own answer is the line judged.
    WindowStage {
        id: processesTimer
        name: "git_processes"
        /// The box typed; the store's answer is the count it then holds.
        order: ["typed"]
        running: Harness.autoAct === "settings-processes"
        onTriggered: {
            if (!settingsDialog.opened) {
                settingsDialog.openAt("app")
                return
            }
            if (processesTimer.at("start")) {
                processesTimer.input("typed", () => acts.appPane.autoTypeProcesses(Harness.autoActArg))
                return
            }
            // Typed nothing: the store's default stands, and the read is of that.
            if (!Awaited.all("git_processes_ready", {
                "applied": Harness.autoActArg === "" || String(AppBackend.gitConcurrency) === Harness.autoActArg,
                "page": PageSettled.windowSettled(window) }))
                return
            processesTimer.complete(() => "git_processes " + acts.appPane.processesTally())
        }
    }

    // PGG_AUTO_ACT=settings-refresh <floor>:<ceiling>:<reading>:<first>:<second>: the application category's `REFRESH`
    // chapter worked through the pane's own doors (`SettingsAppPane.autoTypeRefresh`) — this repository's two boxes, the
    // other worktrees' chooser (`auto` / `fixed` / `off`), then the boxes that reading shows (`first` and `second` are
    // the two bounds, or `first` the one interval). An empty part is left as it is. The store's own answer is the line
    // judged, with which boxes stand beside the chooser.
    WindowStage {
        id: refreshTimer
        name: "settings_refresh"
        /// The chapter worked; the store's answer is the tally standing still across a swapped frame.
        order: ["typed"]
        running: Harness.autoAct === "settings-refresh"
        onTriggered: {
            if (!settingsDialog.opened) {
                settingsDialog.openAt("app")
                return
            }
            if (refreshTimer.at("start")) {
                refreshTimer.input("typed", () => {
                    const parts = Harness.autoActArg.split(":")
                    acts.appPane.autoTypeRefresh(parts[0] || "", parts[1] || "", parts[2] || "", parts[3] || "",
                                                 parts[4] || "")
                })
                return
            }
            if (!Awaited.all("settings_refresh_ready", {
                "store": refreshTimer.still("settings_refresh_tally", acts.appPane.refreshTally()),
                "page": PageSettled.windowSettled(window) }))
                return
            refreshTimer.complete(() => "settings_refresh " + acts.appPane.refreshTally())
        }
    }

    // PGG_AUTO_ACT=settings-refresh-leave <floor>:<ceiling>:<first>:<second>: the way out taken with the `REFRESH`
    // chapter's four bound boxes holding these texts, never finished (`SettingsAppPane.autoLeaveBounds`) — an empty part
    // is an emptied box, which only the way out writes. Both pairs are first set off their defaults through the
    // chapter's own door, so an emptied box has a default to go back to. The store's answer after the close is judged.
    WindowStage {
        id: refreshLeaveTimer
        name: "refresh_leave"
        /// The pairs set off their defaults, then the way out taken.
        order: ["typed", "left"]
        running: Harness.autoAct === "settings-refresh-leave"
        onTriggered: {
            if (refreshLeaveTimer.at("start")) {
                if (!Awaited.all("refresh_leave_open", { "open": settingsDialog.opened })) {
                    settingsDialog.openAt("app")
                    return
                }
                refreshLeaveTimer.input("typed", () => acts.appPane.autoTypeRefresh("7", "20", "auto", "9", "60"))
                return
            }
            if (refreshLeaveTimer.at("typed")) {
                // The pairs typed are in the store before the way out is taken, or the leave would write over them.
                if (!Awaited.all("refresh_leave_typed", {
                        "refresh": AppBackend.refreshFloorSecs === 7 && AppBackend.refreshCeilingSecs === 20,
                        "worktrees": AppBackend.worktreesReading === "auto" && AppBackend.worktreesFloorSecs === 9
                                     && AppBackend.worktreesCeilingSecs === 60 }))
                    return
                refreshLeaveTimer.input("left", () => {
                    const parts = Harness.autoActArg.split(":")
                    acts.appPane.autoLeaveBounds(parts[0] || "", parts[1] || "", parts[2] || "", parts[3] || "")
                    settingsDialog.escapeOut()
                })
                return
            }
            // The fields are written as the close ends.
            if (!Awaited.all("refresh_leave_ready", { "shut": !settingsDialog.visible,
                                                      "page": PageSettled.windowSettled(window) }))
                return
            refreshLeaveTimer.complete(() => "settings_refresh_leave " + acts.appPane.refreshStore()
                                              + " open=" + settingsDialog.opened)
        }
    }

    // PGG_AUTO_ACT=settings-git-leave: the way out taken over a git waiting to be applied (`--other-git`), and
    // turned down. A screen that stayed frames like one nobody asked to close, so the line says the way out was taken,
    // the screen is still up, and the offer is what holds it.
    WindowStage {
        id: gitLeaveTimer
        name: "git_leave"
        /// The path put in, then the way out taken.
        order: ["picked", "left"]
        running: Harness.autoAct === "settings-git-leave"
        onTriggered: {
            if (!settingsDialog.opened) {
                settingsDialog.openAt("app")
                return
            }
            if (gitLeaveTimer.at("start")) {
                gitLeaveTimer.input("picked", () => acts.appPane.autoPickGitPath(Harness.fileUrl(Harness.otherGit)))
                return
            }
            // The offer arrives a subprocess after the pick.
            if (!Awaited.all("git_leave_offered", { "offer": AppBackend.gitPathOffersRestart }))
                return
            if (gitLeaveTimer.at("picked")) {
                gitLeaveTimer.input("left", () => settingsDialog.escapeOut())
                return
            }
            // The way out either asks about the path or has closed the screen.
            if (!Awaited.all("git_leave_asked", { "asked": settingsDialog.askingGitPath || !settingsDialog.opened,
                                                  "page": PageSettled.windowSettled(window) }))
                return
            gitLeaveTimer.complete(() => ["settings_git_leave offers=" + AppBackend.gitPathOffersRestart
                                          + " armed=" + settingsDialog.askingGitPath
                                          + " unsaved=" + settingsDialog.unsavedIdentities
                                          + " open=" + settingsDialog.opened,
                                          "git_path " + acts.appPane.gitPathTally()])
        }
    }

    // PGG_AUTO_ACT=settings-eol <true|input|false|inherited>: the `REPOSITORY OVERRIDE` group's line-ending chapter,
    // picked (`inherited` writes nothing). `core.autocrlf` is written only into the picked repository (規約 §設定の画面),
    // the only file a run may write anyway.
    //
    // Waited on: the read that fills the chooser, then the read that follows the write — the write's own `busy` falls
    // before that read lands, so stopping at it photographs the replaced value.
    WindowStage {
        id: endingsTimer
        name: "settings_eol"
        /// The row picked.
        order: ["picked"]
        running: Harness.autoAct === "settings-eol"
        /// The argument as the model spells it: the row that writes nothing is empty there.
        readonly property string wanted: Harness.autoActArg === "inherited" ? "" : Harness.autoActArg
        onTriggered: {
            if (!settingsDialog.opened) {
                settingsDialog.openAt("git")
                return
            }
            if (endingsTimer.at("start")) {
                if (acts.repoPane.autoRepoRows === 0 || !acts.repoPane.autoEndingsReady)
                    return
                // The chooser's answer is the press's: a row it does not have yet is picked again.
                if (!endingsTimer.input("picked", () => acts.repoPane.autoPickEnding(endingsTimer.wanted)))
                    return
            }
            // Last the foot, so the picture holds the chapter that was written into.
            if (!Awaited.all("settings_eol_ready", {
                "held": acts.repoPane.autoEndingsReady && acts.repoPane.autoEndingHeld === endingsTimer.wanted,
                "foot": acts.chaptersAtFoot(endingsTimer),
                "page": PageSettled.windowSettled(window) }))
                return
            endingsTimer.complete(() => "line_endings " + acts.repoPane.endingsTally())
        }
    }

    // PGG_AUTO_ACT=settings-switch: the rail, pressed from the application category onto git through the row's own
    // handler (`SettingsDialog.autoTapCategory`) — every other settings verb sets the category before the screen is
    // up and would leave a dead rail green. The report reads the panes, not `category` (that is the press itself).
    WindowStage {
        id: categorySwitchTimer
        name: "settings_switch"
        /// The rail's row pressed.
        order: ["tapped"]
        running: Harness.autoAct === "settings-switch"
        /// The application category was standing as the row was pressed — the half the picture cannot hold.
        property bool wasApp: false
        onTriggered: {
            if (categorySwitchTimer.at("start")) {
                if (!settingsDialog.opened) {
                    settingsDialog.openAt("app")
                    return
                }
                if (!acts.appPane.autoAppShown)
                    return
                // The rail's answer is the press's: a row it does not have yet is pressed again.
                if (!categorySwitchTimer.input("tapped", () => {
                        categorySwitchTimer.wasApp = acts.appPane.autoAppShown
                        return settingsDialog.autoTapCategory("git")
                    }))
                    return
            }
            if (!Awaited.all("settings_switch_ready", { "git": acts.gitPane.visible,
                                                        "page": PageSettled.windowSettled(window) }))
                return
            categorySwitchTimer.complete(() => "settings_switch was_app=" + categorySwitchTimer.wasApp
                                                + " app=" + acts.appPane.autoAppShown
                                                + " git=" + acts.gitPane.visible)
        }
    }

    // PGG_AUTO_ACT=settings-escape: the way out, through the function the `✕` and the Escape shortcut both call
    // (`SettingsDialog.escapeOut`). A closed screen frames like one that never opened, so the report's pair of states
    // is judged. The key reaching the shortcut is beyond a harness with no keyboard.
    WindowStage {
        id: escapeTimer
        name: "settings_escape"
        /// The way out taken.
        order: ["escaped"]
        running: Harness.autoAct === "settings-escape"
        /// The screen was up as the way out was taken — the half the shot cannot hold.
        property bool wasOpen: false
        onTriggered: {
            if (escapeTimer.at("start")) {
                if (!settingsDialog.opened) {
                    // The argument names the category: the git one's way out passes the chapters a Save guards.
                    settingsDialog.openAt(Harness.autoActArg === "" ? "app" : Harness.autoActArg)
                    return
                }
                // Unsaved is counted only after git has answered for the boxes, or the run photographs the read.
                if (settingsDialog.category === "git" && !acts.repoPane.autoRepoReady)
                    return
                escapeTimer.input("escaped", () => {
                    escapeTimer.wasOpen = settingsDialog.opened
                    settingsDialog.escapeOut()
                })
                return
            }
            if (!Awaited.all("settings_escape_ready", { "shut": !settingsDialog.opened,
                                                        "page": PageSettled.windowSettled(window) }))
                return
            // `global=` / `repo=` name which chapter thought it held an edit. `save=` is the global chapter's Save,
            // off the button: lit over untouched boxes, it would offer git its own answer.
            escapeTimer.complete(() => "settings_escape unsaved=" + settingsDialog.unsavedIdentities
                                        + " global=" + acts.gitPane.unsavedIsGlobal
                                        + " repo=" + acts.gitPane.unsavedIsRepo
                                        + " save=" + acts.gitPane.autoSaveOffered
                                        + " was_open=" + escapeTimer.wasOpen
                                        + " now_open=" + settingsDialog.opened)
        }
    }

    // PGG_AUTO_ACT=settings-leave: the way out taken while an identity chapter holds an edit git has not been given;
    // the screen must still be there, sent to that chapter with its `✕` armed (the screen has no foot to ask in).
    // Both halves are said: a screen that stayed frames like one never asked to go.
    WindowStage {
        id: leaveTimer
        name: "settings_leave"
        /// The edit made and the way out pressed.
        order: ["left"]
        running: Harness.autoAct === "settings-leave"
        onTriggered: {
            if (leaveTimer.at("start")) {
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
                // The typing's answer is the press's: an edit the chapter did not count is typed again.
                if (!leaveTimer.input("left", () => {
                        acts.gitPane.autoTypeIdentity("Someone Else")
                        if (settingsDialog.unsavedIdentities === 0)
                            return false
                        settingsDialog.escapeOut()
                    }))
                    return
            }
            if (!Awaited.all("settings_leave_asked", { "asked": settingsDialog.askingLeave,
                                                       "page": PageSettled.windowSettled(window) }))
                return
            // `save=`: the typing that stops the way out must also light the Save the reader is sent to.
            leaveTimer.complete(() => "settings_leave unsaved=" + settingsDialog.unsavedIdentities
                                       + " save=" + acts.gitPane.autoSaveOffered
                                       + " asked=" + settingsDialog.askingLeave
                                       + " open=" + settingsDialog.opened)
        }
    }

    // PGG_AUTO_ACT=settings-sweep [app|git]: this screen's words, taken from the air around them
    // (規約 §右のペインの字は掴める); one category per run, as only one is on screen. Waited on the category's own git
    // reads, not the screen opening: sampled before them, the column is about to grow lines.
    WindowStage {
        id: sweepTimer
        name: "settings_sweep"
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
            // git: the repository group's identity and line endings (the latter grows a line); app: the version probe.
            if (!Awaited.all("settings_sweep_ready", {
                "shown": sweepTimer.shown,
                "read": sweepTimer.cat === "git"
                    ? (acts.repoPane.autoRepoReady && acts.repoPane.autoEndingsReady)
                    : AppBackend.gitPathState !== "checking",
                "page": PageSettled.windowSettled(window) }))
                return
            // `release=`: the hand that gives the keyboard back when a press lands where nothing takes it is standing
            // (no picture can say it; that a press reaches it is `tests/qml/tst_fieldrelease.qml`'s). `version=`: the
            // app category's version chip has its grabbable field behind it (`CodeChip.grabbed`). The sweep is
            // walked as the completion reads its line.
            sweepTimer.complete(() => "settings_sweep "
                + settingsDialog.autoChapterHand.sweepAir(7, "cat=" + sweepTimer.cat
                                                             + " shown=" + sweepTimer.shown
                                                             + " release=" + settingsDialog.caretHand.stands
                                                             + " version=" + acts.appPane.autoVersionGrabbed))
        }
    }

    // PGG_AUTO_ACT=settings-tools-enter: Enter in the merge editor's box on each side of its list
    // (デザイン規約 §選ぶ欄と打つ欄「ただし一覧が降りている間の Enter は一覧のもの」): under the list the screen stands,
    // with the list shut it goes. Both in one run — either alone passes a build that never or always leaves.
    WindowStage {
        id: toolEnterTimer
        name: "tools_enter"
        /// Enter with the list down, then the list shut and Enter again. The screen and the list are read before the
        /// first alone: this run's presses can take the screen away, and a precondition read every beat would put it
        /// back up.
        order: ["under_list", "shut"]
        running: Harness.autoAct === "settings-tools-enter"
        /// Whether the screen still stood after the first Enter — read as the second goes in, a beat later, as the
        /// wrong way out closes it where it stands (`SettingsDialog.escapeOut`).
        property bool stood: false
        onTriggered: {
            if (toolEnterTimer.at("start")) {
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
                if (!acts.gitPane.toolListOpen)
                    acts.gitPane.pressToolField()
                if (!Awaited.all("tools_enter_ready", { "list": acts.gitPane.toolListOpen,
                                                        "page": PageSettled.windowSettled(window) }))
                    return
                toolEnterTimer.input("under_list", () => acts.gitPane.autoEnterTool())
                return
            }
            if (toolEnterTimer.at("under_list")) {
                toolEnterTimer.input("shut", () => {
                    toolEnterTimer.stood = settingsDialog.opened
                    acts.gitPane.autoShutToolList()
                    acts.gitPane.autoEnterTool()
                })
                return
            }
            if (!Awaited.all("tools_enter_left", { "shut": !settingsDialog.opened,
                                                   "page": PageSettled.windowSettled(window) }))
                return
            toolEnterTimer.complete(() => "merge_editor_enter stood=" + toolEnterTimer.stood + " left=true")
        }
    }

    // PGG_AUTO_ACT=avatar-enter: Enter in the candidate box opens the file picker (デザイン規約 §アバターを与える).
    // The picker is the platform's window and in neither PNG, so the report line is the whole verdict. The screen is
    // the page's dispatch's to open (`AutoActFindVerbs`), so the typing joins it.
    WindowStage {
        id: avatarEnterTimer
        name: "avatar_enter"
        /// A candidate's name put in the box (a spelled one may not be in this repository).
        order: ["typed"]
        after: ["page/act/dispatched"]
        running: Harness.autoAct === "avatar-enter"
        onTriggered: {
            if (!Awaited.all("avatar_enter_ready", { "joined": avatarEnterTimer.joined(),
                                                     "ready": settingsDialog.opened,
                                                     "page": PageSettled.windowSettled(window) }))
                return
            if (avatarEnterTimer.at("start")) {
                if (acts.appPane.authorChoices.length === 0)
                    return
                avatarEnterTimer.input("typed", () => acts.appPane.autoTypeAvatarWho(acts.appPane.authorChoices[0]))
                return
            }
            // Enter is asked again until the picker stands: it opens as the platform's window does.
            if (!acts.appPane.autoAvatarPickerOpen) {
                acts.appPane.autoEnterAvatarWho()
                return
            }
            avatarEnterTimer.complete(() => "avatar_enter who=" + (acts.appPane.chosenEmail !== "")
                                             + " picker=" + acts.appPane.autoAvatarPickerOpen)
        }
    }

    // The avatar card's four verbs: the page opens them, this finishes each on its own output — the filed row and its
    // picture, that row's `lit`, the candidate list's `opened`, and the row gone after the hold. The screen is the
    // page's dispatch's to open (`AutoActFindVerbs`), so the move joins it.
    WindowStage {
        id: avatarCardTimer
        name: "avatar_card"
        /// This verb's move made — the row lit, the list down, or the removal's hold run out — and its preconditions
        /// not read again: the removal's answer is the row going away (the wait `middle-close` describes).
        order: ["move_made"]
        after: ["page/act/dispatched"]
        running: Harness.autoAct === "avatar-settings" || Harness.autoAct === "avatar-row-lit"
                 || Harness.autoAct === "avatar-combo" || Harness.autoAct === "avatar-remove"
        /// Rows before the hold, read only as the hold is pressed.
        property int rowsBefore: -1
        onTriggered: {
            const op = avatarCardTimer
            const act = Harness.autoAct
            if (op.at("start")) {
                // A run that filed a picture on its way in waits for its row; one that filed nothing (the round-trip
                // read) takes what the store has.
                if (!Awaited.all("avatar_card_arrived", {
                        "joined": op.joined(),
                        "opened": settingsDialog.opened,
                        "painted": Harness.autoActArg === "" || acts.appPane.autoAvatarRowPainted(0),
                        "lit": act !== "avatar-row-lit" || acts.appPane.autoAvatarRowLit(0) }))
                    return
                if (act === "avatar-remove") {
                    // The hold's answer is the press's: a row or a button not there yet takes none, and is held
                    // again. Counted before the presses: their last one removes the row before the call returns.
                    if (!op.input("move_made", () => {
                            const rows = acts.appPane.autoAvatarRows
                            if (!acts.appPane.autoAvatarHoldRemove(0))
                                return false
                            op.rowsBefore = rows
                        }))
                        return
                } else {
                    // Asked again while shut: the field defers the list by a turn of the loop, and a list taken back
                    // down under an unwinding grab needs a second ask (`AppCombo.pressField`).
                    if (act === "avatar-combo" && !acts.appPane.autoAvatarComboOpen) {
                        acts.appPane.autoAvatarOfferCombo()
                        return
                    }
                    op.advance("move_made")
                }
            }
            // The removal's answer arrives after the hold: the store letting the row go. Last the foot, so the
            // picture holds `AVATARS`, the category's foot, out of the resting view's reach — not for the candidate
            // list: its popup stays where the field stood, and scrolling would leave it off its box.
            if (!Awaited.all("avatar_card_ready", {
                "removed": act !== "avatar-remove" || acts.appPane.autoAvatarRows < op.rowsBefore,
                "foot": act === "avatar-combo" || acts.chaptersAtFoot(op),
                "page": PageSettled.windowSettled(window) }))
                return
            op.complete(() => "avatar_card rows=" + acts.appPane.autoAvatarRows
                               + " painted=" + acts.appPane.autoAvatarRowPainted(0)
                               + " lit=" + acts.appPane.autoAvatarRowLit(0)
                               + " combo=" + acts.appPane.autoAvatarComboOpen
                               + " removed=" + (op.rowsBefore > acts.appPane.autoAvatarRows))
        }
    }

    // PGG_AUTO_ACT=settings-hand <chapters|tools>: the middle button's hand on the chapters or the merge editor's card,
    // pressed and left drifting for the shot (the page's `middle-hand` on this screen); waited for is the surface
    // having gone. `tools` wants `--preset mergetools`: most machines name fewer editors than the card's eight rows.
    WindowStage {
        id: handTimer
        name: "settings_hand"
        /// The hand pressed on the surface and left drifting.
        order: ["pressed"]
        running: Harness.autoAct === "settings-hand"
        property real fromAt: 0
        property bool took: false
        /// The hand's frame on the last beat: pressed once it has stood still for one, so the ring anchors in the
        /// card that is there, not one still opening.
        property string seatSize: ""
        readonly property bool tools: Harness.autoActArg === "tools"
        function hand() {
            return handTimer.tools ? acts.gitPane.toolListHand : settingsDialog.autoMiddleHand
        }
        /// Where the surface stands scrolled to.
        function scrolledTo() {
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
            if (handTimer.at("start")) {
                const wait = handTimer.waitingOn()
                if (wait !== "") {
                    // What the surface is seen waiting for, said as it changes.
                    Awaited.note("settings_hand surface=" + Harness.autoActArg + " waiting=" + wait)
                    return
                }
                const size = hand.width + "x" + hand.height
                if (size !== handTimer.seatSize) {
                    handTimer.seatSize = size
                    return
                }
                handTimer.input("pressed", () => {
                    handTimer.fromAt = handTimer.scrolledTo()
                    // The first point down the middle nothing claims: a box on the chapters keeps the middle press
                    // where the platform pastes with it (`MiddleAutoScroll.claimedAt`).
                    let point = Qt.point(hand.width / 2, hand.height / 2)
                    for (const fy of [0.5, 0.25, 0.75, 0.1, 0.9]) {
                        const p = Qt.point(point.x, hand.height * fy)
                        if (!hand.claimedAt(p)) {
                            point = p
                            break
                        }
                    }
                    handTimer.took = hand.press(point)
                    if (handTimer.took)
                        hand.drift(Qt.point(point.x, point.y + Metrics.middleScrollDeadZone + 64))
                }, () => "settings_hand surface=" + Harness.autoActArg + " pressed took=" + handTimer.took)
                return
            }
            const went = Math.abs(handTimer.scrolledTo() - handTimer.fromAt) > 8
            if (!Awaited.all("settings_hand_ready", { "went": !handTimer.took || (went && hand.ticks >= 2),
                                                      "page": PageSettled.windowSettled(window) }))
                return
            handTimer.complete(() => {
                const hand = handTimer.hand()
                const went = Math.abs(handTimer.scrolledTo() - handTimer.fromAt) > 8
                return "settings_hand surface=" + Harness.autoActArg
                       + " answered=" + (handTimer.took && went && hand.scrolling)
                       + " took=" + handTimer.took
                       + " moved=" + went
                       + " scrolling=" + hand.scrolling
                       + " ticks=" + hand.ticks
            })
        }
    }
}
