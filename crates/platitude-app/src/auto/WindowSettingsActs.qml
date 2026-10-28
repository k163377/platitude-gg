pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The settings screen's half of the window's PGG_AUTO_ACT harness: the verbs that reach into `settingsDialog` and
/// nothing else (no tab, page or band). The rest of the window's dialog verbs are `WindowDialogActs`.
///
/// A verb that opens the screen itself does so as its first stage, `opened` (`openOn`): an idempotent ask, pressed
/// again each beat until the screen stands. One that needs it still up as it completes names it in its `when`.
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

    /// Opens the screen on `category` where it is not up, as the press of a verb's `opened` stage. Answers whether it
    /// stands; asked again on the next beat while it does not.
    function openOn(category) {
        if (!acts.settingsDialog.opened)
            acts.settingsDialog.openAt(category)
        return acts.settingsDialog.opened
    }

    /// Whether the chapters' column and where it stands scrolled to have stood across a polish pass (`op`'s
    /// `still`): the foot is read off the column's height, which can grow after a send — the opened screen's first
    /// layout pass lays every chapter out again, and a send made before it leaves `AVATARS` below the view. Read in
    /// the beat before each send to the foot (`sendChaptersToFoot`), so the foot is judged after a polish pass begun
    /// after the last send.
    function chaptersStill(op) {
        return op.still("chapters_foot", acts.settingsDialog.chaptersContent + "@" + acts.settingsDialog.chaptersAt)
    }
    /// The press of a verb's `foot` stage: whether the chapters stand at their foot, sending them there where they do
    /// not — pressed again on a later beat, once the column has stood (`chaptersStill`).
    function sendChaptersToFoot() {
        if (acts.settingsDialog.autoAtChapterFoot())
            return true
        acts.settingsDialog.autoShowChapterFoot()
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
            toolsTimer.note(acts.toolLine())
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
        /// Whether the box stands as the verb wants it: the loading edge seen with the list down, or the settled
        /// choices down.
        function ready() {
            return Harness.autoAct === "settings-tools-loading"
                ? (acts.toolLoadingSeen && acts.gitPane.toolListOpen)
                : (acts.gitPane.toolsSettled && acts.gitPane.toolChoices.length > 0 && acts.gitPane.toolListOpen)
        }
        onTriggered: {
            if (toolsTimer.at("start")) {
                toolsTimer.input("opened", () => {
                    if (!settingsDialog.opened)
                        settingsDialog.pressToolOnOpen = true
                    return acts.openOn("git")
                })
                return
            }
            // A stock-take that named nothing ends both verbs: an empty list is a card with nothing to drop, which
            // closes itself (`AppCombo.hasList`), so neither `opened` nor the loading edge can follow.
            if (acts.toolsCameBack && acts.gitPane.toolChoices.length === 0) {
                toolsTimer.complete({
                    "name": "settings_tools_none",
                    "when": () => ({ "opened": settingsDialog.opened, "page": PageSettled.windowSettled(window) }),
                    "report": () => [acts.toolLine(),
                                     "merge_editor_none this machine's stock-take named no merge editor, "
                                     + "so the list has nothing to drop"] })
                return
            }
            toolsTimer.complete({
                "name": "settings_tools_ready",
                "when": () => ({ "opened": settingsDialog.opened,
                                 "ready": toolsTimer.ready(),
                                 "page": PageSettled.windowSettled(window) }),
                "report": () => acts.toolLine() })
        }
    }

    // PGG_AUTO_ACT=settings-repo / settings-repo-pick: the git category's `REPOSITORY OVERRIDE` group, with the
    // chooser's list down for `-pick`. The argument picks a strip row other than the front one, through the same call
    // a pick from the list makes (verify-ui implement.md §壊れない動詞の実装と反復).
    WindowStage {
        id: repoSettingsTimer
        name: "repo_settings"
        /// The screen opened, then the row of the strip chosen (the front one stays where no argument names another)
        /// and the list, where one is wanted, pressed down.
        order: ["opened", "chosen"]
        running: Harness.autoAct === "settings-repo" || Harness.autoAct === "settings-repo-pick"
        onTriggered: {
            if (repoSettingsTimer.at("start")) {
                repoSettingsTimer.input("opened", () => acts.openOn("git"))
                return
            }
            if (repoSettingsTimer.at("opened")) {
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
            repoSettingsTimer.complete({
                "name": "repo_settings_ready",
                "when": () => ({ "opened": settingsDialog.opened,
                                 "read": acts.repoPane.autoRepoReady,
                                 "list": Harness.autoAct !== "settings-repo-pick" || acts.repoPane.autoRepoComboOpen,
                                 "page": PageSettled.windowSettled(window) }),
                "report": () => ["repo_config " + acts.repoPane.repoTally(), acts.fitLine()] })
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
        /// The screen opened, the path put in (with no argument, nothing is), then the write the way out makes.
        order: ["opened", "picked", "applied"]
        running: Harness.autoAct === "settings-git-path"
        onTriggered: {
            if (gitPathTimer.at("start")) {
                gitPathTimer.input("opened", () => acts.openOn("app"))
                return
            }
            if (gitPathTimer.at("opened")) {
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
            if (gitPathTimer.at("picked")) {
                if (!Awaited.all("git_path_answered", {
                    "state": AppBackend.gitPathState !== "" && AppBackend.gitPathState !== "checking",
                    "page": PageSettled.settledUnder(window) }))
                    return
                // The write the way out makes, with the screen still up for the picture: the file a seeded run
                // started from is read after the run for the spelling it keeps (`verify::seed`).
                if (!gitPathTimer.input("applied", () => settingsDialog.applyFields()))
                    return
            }
            // The answer was waited for before the write; the page is the completion's hold's.
            gitPathTimer.complete({
                "name": "git_path_applied",
                "when": () => ({ "opened": settingsDialog.opened }),
                "report": () => "git_path " + acts.appPane.gitPathTally() })
        }
    }

    // PGG_AUTO_ACT=settings-processes <commands>: the application category's `GIT PROCESSES` chapter, its box typed
    // through the pane's own door (`SettingsAppPane.autoTypeProcesses`); an empty argument is an emptied box (the
    // default). An unapplied edit frames the same, so the store's own answer is the line judged.
    WindowStage {
        id: processesTimer
        name: "git_processes"
        /// The screen opened and the box typed; the store's answer is the count it then holds.
        order: ["opened", "typed"]
        running: Harness.autoAct === "settings-processes"
        onTriggered: {
            if (processesTimer.at("start")) {
                processesTimer.input("opened", () => acts.openOn("app"))
                return
            }
            if (processesTimer.at("opened")) {
                processesTimer.input("typed", () => acts.appPane.autoTypeProcesses(Harness.autoActArg))
                return
            }
            // Typed nothing: the store's default stands, and the read is of that.
            processesTimer.complete({
                "name": "git_processes_ready",
                "when": () => ({ "opened": settingsDialog.opened,
                                 "applied": Harness.autoActArg === ""
                                            || String(AppBackend.gitConcurrency) === Harness.autoActArg,
                                 "page": PageSettled.windowSettled(window) }),
                "report": () => "git_processes " + acts.appPane.processesTally() })
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
        /// The screen opened and the chapter worked; the store's answer is the tally standing still across a swapped
        /// frame.
        order: ["opened", "typed"]
        running: Harness.autoAct === "settings-refresh"
        onTriggered: {
            if (refreshTimer.at("start")) {
                refreshTimer.input("opened", () => acts.openOn("app"))
                return
            }
            if (refreshTimer.at("opened")) {
                refreshTimer.input("typed", () => {
                    const parts = Harness.autoActArg.split(":")
                    acts.appPane.autoTypeRefresh(parts[0] || "", parts[1] || "", parts[2] || "", parts[3] || "",
                                                 parts[4] || "")
                })
                return
            }
            if (!refreshTimer.still("settings_refresh_tally", acts.appPane.refreshTally()))
                return
            refreshTimer.complete({
                "name": "settings_refresh_ready",
                "when": () => ({ "opened": settingsDialog.opened, "page": PageSettled.windowSettled(window) }),
                "report": () => "settings_refresh " + acts.appPane.refreshTally() })
        }
    }

    // PGG_AUTO_ACT=settings-refresh-leave <floor>:<ceiling>:<first>:<second>: the way out taken with the `REFRESH`
    // chapter's four bound boxes holding these texts, never finished (`SettingsAppPane.autoLeaveBounds`) — an empty part
    // is an emptied box, which only the way out writes. Both pairs are first set off their defaults through the
    // chapter's own door, so an emptied box has a default to go back to. The store's answer after the close is judged.
    WindowStage {
        id: refreshLeaveTimer
        name: "refresh_leave"
        /// The screen opened, the pairs set off their defaults, then the way out taken.
        order: ["opened", "typed", "left"]
        running: Harness.autoAct === "settings-refresh-leave"
        onTriggered: {
            if (refreshLeaveTimer.at("start")) {
                refreshLeaveTimer.input("opened", () => acts.openOn("app"))
                return
            }
            if (refreshLeaveTimer.at("opened")) {
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
            refreshLeaveTimer.complete({
                "name": "refresh_leave_ready",
                "when": () => ({ "shut": !settingsDialog.visible, "page": PageSettled.windowSettled(window) }),
                "report": () => "settings_refresh_leave " + acts.appPane.refreshStore()
                                + " open=" + settingsDialog.opened })
        }
    }

    // PGG_AUTO_ACT=settings-git-leave: the way out taken over a git waiting to be applied (`--other-git`), and
    // turned down. A screen that stayed frames like one nobody asked to close, so the line says the way out was taken,
    // the screen is still up, and the offer is what holds it.
    WindowStage {
        id: gitLeaveTimer
        name: "git_leave"
        /// The screen opened, the path put in, then the way out taken.
        order: ["opened", "picked", "left"]
        running: Harness.autoAct === "settings-git-leave"
        onTriggered: {
            if (gitLeaveTimer.at("start")) {
                gitLeaveTimer.input("opened", () => acts.openOn("app"))
                return
            }
            if (gitLeaveTimer.at("opened")) {
                gitLeaveTimer.input("picked", () => acts.appPane.autoPickGitPath(Harness.fileUrl(Harness.otherGit)))
                return
            }
            // The offer arrives a subprocess after the pick.
            if (gitLeaveTimer.at("picked")) {
                if (!Awaited.all("git_leave_offered", { "offer": AppBackend.gitPathOffersRestart }))
                    return
                gitLeaveTimer.input("left", () => settingsDialog.escapeOut())
                return
            }
            // The way out either asks about the path or has closed the screen — the offer still standing.
            gitLeaveTimer.complete({
                "name": "git_leave_asked",
                "when": () => ({ "offer": AppBackend.gitPathOffersRestart,
                                 "asked": settingsDialog.askingGitPath || !settingsDialog.opened,
                                 "page": PageSettled.windowSettled(window) }),
                "report": () => ["settings_git_leave offers=" + AppBackend.gitPathOffersRestart
                                 + " armed=" + settingsDialog.askingGitPath
                                 + " unsaved=" + settingsDialog.unsavedIdentities
                                 + " open=" + settingsDialog.opened,
                                 "git_path " + acts.appPane.gitPathTally()] })
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
        /// The screen opened, the row picked, then the chapters sent to their foot once the write's read is held.
        order: ["opened", "picked", "foot"]
        running: Harness.autoAct === "settings-eol"
        /// The argument as the model spells it: the row that writes nothing is empty there.
        readonly property string wanted: Harness.autoActArg === "inherited" ? "" : Harness.autoActArg
        /// The read that follows the write holding the row picked.
        function held() {
            return acts.repoPane.autoEndingsReady && acts.repoPane.autoEndingHeld === endingsTimer.wanted
        }
        onTriggered: {
            if (endingsTimer.at("start")) {
                endingsTimer.input("opened", () => acts.openOn("git"))
                return
            }
            if (endingsTimer.at("opened")) {
                if (acts.repoPane.autoRepoRows === 0 || !acts.repoPane.autoEndingsReady)
                    return
                // The chooser's answer is the press's: a row it does not have yet is picked again.
                if (!endingsTimer.input("picked", () => acts.repoPane.autoPickEnding(endingsTimer.wanted)))
                    return
            }
            // Last the foot, so the picture holds the chapter that was written into.
            if (endingsTimer.at("picked")) {
                if (!Awaited.all("settings_eol_held", { "held": endingsTimer.held() }))
                    return
                if (!acts.chaptersStill(endingsTimer) || !endingsTimer.input("foot", () => acts.sendChaptersToFoot()))
                    return
            }
            endingsTimer.complete({
                "name": "settings_eol_ready",
                "when": () => ({ "opened": settingsDialog.opened,
                                 "held": endingsTimer.held(),
                                 "foot": settingsDialog.autoAtChapterFoot(),
                                 "page": PageSettled.windowSettled(window) }),
                "report": () => "line_endings " + acts.repoPane.endingsTally() })
        }
    }

    // PGG_AUTO_ACT=settings-switch: the rail, pressed from the application category onto git through the row's own
    // handler (`SettingsDialog.autoTapCategory`) — every other settings verb sets the category before the screen is
    // up and would leave a dead rail green. The report reads the panes, not `category` (that is the press itself).
    WindowStage {
        id: categorySwitchTimer
        name: "settings_switch"
        /// The screen opened, then the rail's row pressed.
        order: ["opened", "tapped"]
        running: Harness.autoAct === "settings-switch"
        onTriggered: {
            if (categorySwitchTimer.at("start")) {
                categorySwitchTimer.input("opened", () => acts.openOn("app"))
                return
            }
            if (categorySwitchTimer.at("opened")) {
                if (!acts.appPane.autoAppShown)
                    return
                // The rail's answer is the press's: a row it does not have yet is pressed again. The application
                // category was standing as the row was pressed — the half the picture cannot hold — kept once it took.
                if (!categorySwitchTimer.input("tapped", () => {
                        const wasApp = acts.appPane.autoAppShown
                        if (!settingsDialog.autoTapCategory("git"))
                            return false
                        categorySwitchTimer.witness("wasApp", wasApp)
                    }))
                    return
            }
            categorySwitchTimer.complete({
                "name": "settings_switch_ready",
                "when": () => ({ "git": acts.gitPane.visible, "page": PageSettled.windowSettled(window) }),
                "report": () => "settings_switch was_app=" + categorySwitchTimer.witnessed("wasApp")
                                + " app=" + acts.appPane.autoAppShown
                                + " git=" + acts.gitPane.visible })
        }
    }

    // PGG_AUTO_ACT=settings-escape: the way out, through the function the `✕` and the Escape shortcut both call
    // (`SettingsDialog.escapeOut`). A closed screen frames like one that never opened, so the report's pair of states
    // is judged. The key reaching the shortcut is beyond a harness with no keyboard.
    WindowStage {
        id: escapeTimer
        name: "settings_escape"
        /// The screen opened, then the way out taken.
        order: ["opened", "escaped"]
        running: Harness.autoAct === "settings-escape"
        onTriggered: {
            if (escapeTimer.at("start")) {
                // The argument names the category: the git one's way out passes the chapters a Save guards.
                escapeTimer.input("opened", () => acts.openOn(Harness.autoActArg === "" ? "app" : Harness.autoActArg))
                return
            }
            if (escapeTimer.at("opened")) {
                // Unsaved is counted only after git has answered for the boxes, or the run photographs the read.
                if (settingsDialog.category === "git" && !acts.repoPane.autoRepoReady)
                    return
                // The screen was up as the way out was taken — the half the shot cannot hold.
                escapeTimer.input("escaped", () => {
                    escapeTimer.witness("wasOpen", settingsDialog.opened)
                    settingsDialog.escapeOut()
                })
                return
            }
            // `global=` / `repo=` name which chapter thought it held an edit. `save=` is the global chapter's Save,
            // off the button: lit over untouched boxes, it would offer git its own answer.
            escapeTimer.complete({
                "name": "settings_escape_ready",
                "when": () => ({ "shut": !settingsDialog.opened, "page": PageSettled.windowSettled(window) }),
                "report": () => "settings_escape unsaved=" + settingsDialog.unsavedIdentities
                                + " global=" + acts.gitPane.unsavedIsGlobal
                                + " repo=" + acts.gitPane.unsavedIsRepo
                                + " save=" + acts.gitPane.autoSaveOffered
                                + " was_open=" + escapeTimer.witnessed("wasOpen")
                                + " now_open=" + settingsDialog.opened })
        }
    }

    // PGG_AUTO_ACT=settings-leave: the way out taken while an identity chapter holds an edit git has not been given;
    // the screen must still be there, sent to that chapter with its `✕` armed (the screen has no foot to ask in).
    // Both halves are said: a screen that stayed frames like one never asked to go.
    WindowStage {
        id: leaveTimer
        name: "settings_leave"
        /// The screen opened, then the edit made and the way out pressed.
        order: ["opened", "left"]
        running: Harness.autoAct === "settings-leave"
        onTriggered: {
            if (leaveTimer.at("start")) {
                leaveTimer.input("opened", () => acts.openOn("git"))
                return
            }
            if (leaveTimer.at("opened")) {
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
            // `save=`: the typing that stops the way out must also light the Save the reader is sent to.
            leaveTimer.complete({
                "name": "settings_leave_asked",
                "when": () => ({ "asked": settingsDialog.askingLeave, "page": PageSettled.windowSettled(window) }),
                "report": () => "settings_leave unsaved=" + settingsDialog.unsavedIdentities
                                + " save=" + acts.gitPane.autoSaveOffered
                                + " asked=" + settingsDialog.askingLeave
                                + " open=" + settingsDialog.opened })
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
        /// What the sweep stands on, read now: the screen up on the category, its own git reads in, and the page.
        function ready() {
            return { "opened": settingsDialog.opened,
                     "shown": sweepTimer.shown,
                     "read": sweepTimer.cat === "git"
                         ? (acts.repoPane.autoRepoReady && acts.repoPane.autoEndingsReady)
                         : AppBackend.gitPathState !== "checking",
                     "page": PageSettled.windowSettled(window) }
        }
        onTriggered: {
            if (sweepTimer.at("start")) {
                sweepTimer.input("opened", () => acts.openOn(sweepTimer.cat))
                return
            }
            // git: the repository group's identity and line endings (the latter grows a line); app: the version probe.
            // `release=`: the hand that gives the keyboard back when a press lands where nothing takes it is standing
            // (no picture can say it; that a press reaches it is `tests/qml/tst_fieldrelease.qml`'s). `version=`: the
            // app category's version chip has its grabbable field behind it (`CodeChip.grabbed`). The sweep is a
            // press, walked once the category has read, and its line, with the states read beside it, is its
            // witness.
            if (sweepTimer.at("opened")) {
                if (!Awaited.all("settings_sweep_ready", sweepTimer.ready()))
                    return
                if (!sweepTimer.input("swept", () => {
                        sweepTimer.witness("swept", settingsDialog.autoChapterHand.sweepAir(7, "cat=" + sweepTimer.cat
                            + " shown=" + sweepTimer.shown
                            + " release=" + settingsDialog.caretHand.stands
                            + " version=" + acts.appPane.autoVersionGrabbed))
                    }))
                    return
            }
            sweepTimer.complete({
                "name": "settings_sweep_swept",
                "when": () => sweepTimer.ready(),
                "report": () => "settings_sweep " + sweepTimer.witnessed("swept") })
        }
    }

    // PGG_AUTO_ACT=settings-tools-enter: Enter in the merge editor's box on each side of its list
    // (デザイン規約 §選ぶ欄と打つ欄「ただし一覧が降りている間の Enter は一覧のもの」): under the list the screen stands,
    // with the list shut it goes. Both in one run — either alone passes a build that never or always leaves.
    WindowStage {
        id: toolEnterTimer
        name: "tools_enter"
        /// The screen opened and the list down, then Enter with the list down, then the list shut and Enter again.
        /// The screen and the list are read before the first Enter alone: this run's presses can take the screen
        /// away, and a precondition read every beat would put it back up.
        order: ["opened", "listed", "under_list", "shut"]
        running: Harness.autoAct === "settings-tools-enter"
        onTriggered: {
            if (toolEnterTimer.at("start")) {
                toolEnterTimer.input("opened", () => acts.openOn("git"))
                return
            }
            if (toolEnterTimer.at("opened")) {
                // A card with a loading ring is a list standing in front of the box, which alone decides whose Enter
                // it is — so no rows are needed (`AppCombo.hasList`). Waiting for them would tie the run to
                // `git mergetool --tool-help`, which can hit its timeout on a loaded machine.
                if (acts.gitPane.toolsSettled && acts.gitPane.toolChoices.length === 0)
                    return
                // Asked again while it is shut, for the turn of the loop the field puts between the press and the
                // list (`AppCombo.pressField`).
                toolEnterTimer.input("listed", () => {
                    if (!acts.gitPane.toolListOpen)
                        acts.gitPane.pressToolField()
                    return acts.gitPane.toolListOpen
                })
                return
            }
            if (toolEnterTimer.at("listed")) {
                if (!Awaited.all("tools_enter_ready", { "list": acts.gitPane.toolListOpen,
                                                        "page": PageSettled.windowSettled(window) }))
                    return
                toolEnterTimer.input("under_list", () => acts.gitPane.autoEnterTool())
                return
            }
            // Whether the screen still stood after the first Enter — read as the second goes in, a beat later, as
            // the wrong way out closes it where it stands (`SettingsDialog.escapeOut`).
            if (toolEnterTimer.at("under_list")) {
                toolEnterTimer.input("shut", () => {
                    toolEnterTimer.witness("stood", settingsDialog.opened)
                    acts.gitPane.autoShutToolList()
                    acts.gitPane.autoEnterTool()
                })
                return
            }
            toolEnterTimer.complete({
                "name": "tools_enter_left",
                "when": () => ({ "shut": !settingsDialog.opened, "page": PageSettled.windowSettled(window) }),
                "report": () => "merge_editor_enter stood=" + toolEnterTimer.witnessed("stood") + " left=true" })
        }
    }

    // PGG_AUTO_ACT=avatar-enter: Enter in the candidate box opens the file picker (デザイン規約 §アバターを与える).
    // The picker is the platform's window and in neither PNG, so the report line is the whole verdict. The screen is
    // the page's dispatch's to open (`AutoActFindVerbs`), so the typing joins it.
    WindowStage {
        id: avatarEnterTimer
        name: "avatar_enter"
        /// A candidate's name put in the box (a spelled one may not be in this repository), then Enter.
        order: ["typed", "entered"]
        after: ["page/act/dispatched"]
        running: Harness.autoAct === "avatar-enter"
        /// What every stage moves on, read now: the screen the page opened, and the page under it.
        function ready() {
            return { "joined": avatarEnterTimer.joined(),
                     "ready": settingsDialog.opened,
                     "page": PageSettled.windowSettled(window) }
        }
        onTriggered: {
            if (!Awaited.all("avatar_enter_ready", avatarEnterTimer.ready()))
                return
            if (avatarEnterTimer.at("start")) {
                if (acts.appPane.authorChoices.length === 0)
                    return
                avatarEnterTimer.input("typed", () => acts.appPane.autoTypeAvatarWho(acts.appPane.authorChoices[0]))
                return
            }
            // Enter is asked again until the picker stands: it opens as the platform's window does.
            if (avatarEnterTimer.at("typed")) {
                if (!avatarEnterTimer.input("entered", () => {
                        if (!acts.appPane.autoAvatarPickerOpen)
                            acts.appPane.autoEnterAvatarWho()
                        return acts.appPane.autoAvatarPickerOpen
                    }))
                    return
            }
            avatarEnterTimer.complete({
                "name": "avatar_enter_picked",
                "when": () => {
                    const ready = avatarEnterTimer.ready()
                    return { "joined": ready.joined, "ready": ready.ready, "page": ready.page,
                             "picker": acts.appPane.autoAvatarPickerOpen }
                },
                "report": () => "avatar_enter who=" + (acts.appPane.chosenEmail !== "")
                                + " picker=" + acts.appPane.autoAvatarPickerOpen })
        }
    }

    // The avatar card's four verbs: the page opens them, this finishes each on its own output — the filed row and its
    // picture, that row's `lit`, the candidate list's `opened`, and the row gone after the hold. The screen is the
    // page's dispatch's to open (`AutoActFindVerbs`), so the move joins it.
    WindowStage {
        id: avatarCardTimer
        name: "avatar_card"
        /// This verb's move made — the row lit, the list down, or the removal's hold run out — and its preconditions
        /// not read again: the removal's answer is the row going away (the wait `middle-close` describes). Then the
        /// chapters sent to their foot, but for the candidate list.
        order: Harness.autoAct === "avatar-combo" ? ["move_made"] : ["move_made", "foot"]
        after: ["page/act/dispatched"]
        running: Harness.autoAct === "avatar-settings" || Harness.autoAct === "avatar-row-lit"
                 || Harness.autoAct === "avatar-combo" || Harness.autoAct === "avatar-remove"
        /// The removal's answer: the store letting the row go, below the rows counted before the hold.
        function removed() {
            return Harness.autoAct !== "avatar-remove"
                || acts.appPane.autoAvatarRows < avatarCardTimer.witnessed("rowsBefore")
        }
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
                            op.witness("rowsBefore", rows)
                        }))
                        return
                } else if (act === "avatar-combo") {
                    // Asked again while shut: the field defers the list by a turn of the loop, and a list taken back
                    // down under an unwinding grab needs a second ask (`AppCombo.pressField`).
                    if (!op.input("move_made", () => {
                            if (!acts.appPane.autoAvatarComboOpen)
                                acts.appPane.autoAvatarOfferCombo()
                            return acts.appPane.autoAvatarComboOpen
                        }))
                        return
                } else {
                    op.advance("move_made")
                }
            }
            // The removal's answer arrives after the hold: the store letting the row go. Last the foot, so the
            // picture holds `AVATARS`, the category's foot, out of the resting view's reach — not for the candidate
            // list: its popup stays where the field stood, and scrolling would leave it off its box.
            if (act !== "avatar-combo" && op.at("move_made")) {
                if (!Awaited.all("avatar_card_removed", { "removed": op.removed() }))
                    return
                if (!acts.chaptersStill(op) || !op.input("foot", () => acts.sendChaptersToFoot()))
                    return
            }
            op.complete({
                "name": "avatar_card_ready",
                "when": () => ({ "removed": op.removed(),
                                 "foot": Harness.autoAct === "avatar-combo" || settingsDialog.autoAtChapterFoot(),
                                 "page": PageSettled.windowSettled(window) }),
                "report": () => "avatar_card rows=" + acts.appPane.autoAvatarRows
                                + " painted=" + acts.appPane.autoAvatarRowPainted(0)
                                + " lit=" + acts.appPane.autoAvatarRowLit(0)
                                + " combo=" + acts.appPane.autoAvatarComboOpen
                                + " removed=" + (Harness.autoAct === "avatar-remove"
                                                 && op.witnessed("rowsBefore") > acts.appPane.autoAvatarRows) })
        }
    }

    // PGG_AUTO_ACT=settings-hand <chapters|tools>: the middle button's hand on the chapters or the merge editor's card,
    // pressed and left drifting for the shot (the page's `middle-hand` on this screen); waited for is the surface
    // having gone. `tools` wants `--preset mergetools`: most machines name fewer editors than the card's eight rows.
    WindowStage {
        id: handTimer
        name: "settings_hand"
        /// The screen opened, then the hand pressed on the surface and left drifting.
        order: ["opened", "pressed"]
        running: Harness.autoAct === "settings-hand"
        readonly property bool tools: Harness.autoActArg === "tools"
        function hand() {
            return handTimer.tools ? acts.gitPane.toolListHand : settingsDialog.autoMiddleHand
        }
        /// Where the surface stands scrolled to.
        function scrolledTo() {
            return handTimer.tools ? acts.gitPane.toolListAt : settingsDialog.chaptersAt
        }
        /// Whether the surface has gone more than a few pixels from where it stood as the hand went down.
        function went() {
            return Math.abs(handTimer.scrolledTo() - handTimer.witnessed("fromAt")) > 8
        }
        /// What the surface still waits for, or "" once it holds more than room. The card's list is the hand's
        /// parent (`AppCombo`).
        function waitingOn() {
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
                // Re-asked until the screen stands; told to bring the card down where it is the surface (as
                // `settings-tools`).
                handTimer.note("settings_hand surface=" + Harness.autoActArg + " waiting=the screen")
                handTimer.input("opened", () => {
                    if (!settingsDialog.opened)
                        settingsDialog.pressToolOnOpen = handTimer.tools
                    return acts.openOn("git")
                })
                return
            }
            if (handTimer.at("opened")) {
                const wait = handTimer.waitingOn()
                if (wait !== "") {
                    // What the surface is seen waiting for, said as it changes.
                    handTimer.note("settings_hand surface=" + Harness.autoActArg + " waiting=" + wait)
                    return
                }
                // The hand's frame standing across a polish pass, so the ring anchors in the card that is there,
                // not one still opening.
                if (!handTimer.still("seat", hand.width + "x" + hand.height))
                    return
                // Where the surface stood, and whether the hand took the press, kept for the completion.
                handTimer.input("pressed", () => {
                    handTimer.witness("fromAt", handTimer.scrolledTo())
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
                    if (handTimer.witness("took", hand.press(point)))
                        hand.drift(Qt.point(point.x, point.y + Metrics.middleScrollDeadZone + 64))
                }, () => "settings_hand surface=" + Harness.autoActArg + " pressed took=" + handTimer.witnessed("took"))
                return
            }
            handTimer.complete({
                "name": "settings_hand_ready",
                "when": () => ({ "went": !handTimer.witnessed("took")
                                         || (handTimer.went() && handTimer.hand().ticks >= 2),
                                 "page": PageSettled.windowSettled(window) }),
                "report": () => {
                    const hand = handTimer.hand()
                    const went = handTimer.went()
                    const took = handTimer.witnessed("took")
                    return "settings_hand surface=" + Harness.autoActArg
                           + " answered=" + (took && went && hand.scrolling)
                           + " took=" + took
                           + " moved=" + went
                           + " scrolling=" + hand.scrolling
                           + " ticks=" + hand.ticks
                } })
        }
    }
}
