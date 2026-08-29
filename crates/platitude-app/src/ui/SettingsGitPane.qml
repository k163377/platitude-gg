import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// The settings screen's `Git` category: everything this app writes into git's own configuration, in the two groups
// git itself keeps it in (規約 §設定の画面). A file of its own because it is the half of the screen that talks to git
// — it asks git what it would launch, hands git the identity, and nothing in it is Platitude GG's to keep.
//
// **The groups are how far a value reaches, not what it is.** `GLOBAL` is what git reads everywhere on this computer;
// `REPOSITORY OVERRIDE` is what the repository picked below writes into its own file, standing over the first. The
// chapters repeat across them where the setting exists at both levels — the identity does, the merge editor does not
// (`conflict::set_merge_tool`: which editor someone reaches for is a property of their desk).
//
// The pair is not symmetric on purpose: the second group is not a peer of the first, it sits on top of it, and the
// word `override` is what says so (規約 §設定の画面).
ColumnLayout {
    id: pane

    /// The tab git is read through. The settings are global; "what would git launch here" is not.
    property var curPage: null
    /// The strip, for the group that writes into one repository (`SettingsRepoPane`).
    required property TabsModel tabsModel
    /// This category is the one on screen, and the screen is open. What the slow candidate read waits for — a reader
    /// who opened the application category never asked for it.
    property bool showing: false
    /// The screen is open, whichever category it is on. **What the boxes follow git by** — not `showing`: git can
    /// answer while the reader is in the other category (a status refresh names the tool wherever something is
    /// conflicted), and a box that stopped following there would be written back on close as though the reader had
    /// emptied it (`applyTool`).
    property bool screenOpen: false

    /// Enter in the tool box. The way out belongs to the screen, so it is asked for rather than taken.
    signal accepted()

    // Asking is the showing category's; forgetting the latch is the closing screen's, so that neither the open edge's
    // two handlers nor their order can take away what the other just put there.
    onShowingChanged: {
        if (pane.showing)
            pane.askTools()
        else
            pane.autoToolLoadingLatched = false
    }

    // ---- the merge editor ------------------------------------------------
    readonly property string mergeTool: pane.curPage ? pane.curPage.pageWt.mergeTool : ""
    /// Names to offer, packed the way the graph's label records are.
    readonly property var toolChoices: {
        const packed = pane.curPage ? pane.curPage.pageTab.mergeTools : ""
        return packed === "" ? [] : packed.split(String.fromCharCode(31))
    }
    /// Stops a late answer from overwriting something already typed.
    property bool toolTouched: false
    // Automation can photograph both phases without racing a wall clock. The loading latch is raised only after the
    // real model reports an outstanding tool read, then keeps that observed visual state alive until grabToImage has
    // finished.
    property bool autoToolLoadingLatched: false
    readonly property bool autoToolsLoadingReady: pane.autoToolLoadingLatched && toolField.popup.opened
    readonly property bool autoToolsSettledReady: pane.curPage
        && !pane.curPage.pageTab.mergeToolsLoading
        && pane.toolChoices.length > 0 && toolField.popup.opened

    /// Asks git what it would launch, and what it could launch. The candidate read is `git mergetool --tool-help`,
    /// which is about eight seconds on Windows (規約 §conflict を外部ツールへ渡す) — hence the turning indicator, and
    /// hence asking only from the chapter that shows the answer.
    function askTools() {
        if (!pane.curPage)
            return
        // The status refresh only names the configured tool where something is conflicted, so ask for it.
        pane.curPage.pageTab.askMergeTool()
        pane.curPage.pageTab.askMergeTools()
        if (AppBackend.autoAct === "settings-tools-loading" && pane.curPage.pageTab.mergeToolsLoading)
            pane.autoToolLoadingLatched = true
    }

    // The tool is written only where it would change what git answers with. Pressing the field is not choosing
    // anything, and the write for "the same as now" is not free: an empty field asks git to unset a key, which fails
    // when the key was never there in the first place (2026-08-21 ユーザー報告 — the log raised itself over the
    // screen closing).
    function applyTool() {
        if (pane.curPage && toolField.wanted !== pane.mergeTool)
            pane.curPage.pageTab.setMergeTool(toolField.wanted)
    }
    /// Puts the box back to what git says, for the screen that just opened. The latch is not this one's to clear —
    /// it belongs to the closing screen (`onShowingChanged`), because this runs off the same edge that raises it.
    function loadTool() {
        toolField.wanted = pane.mergeTool
        pane.toolTouched = false
    }
    /// The door a press uses, so the list comes down the way it does under a hand (`AppCombo.pressField`).
    function pressToolField() {
        toolField.pressField()
    }

    /// Smoke hook. The candidates arrive in two waves and the configured name in a third, so the value has three
    /// chances to be knocked out by something that is not a person — report it at each.
    function reportTool() {
        if (AppBackend.autoAct === "settings-tools" || AppBackend.autoAct === "settings-tools-loading")
            AppBackend.report("merge_editor wanted=" + toolField.wanted + " shown=" + toolField.editText
                              + " configured=" + pane.mergeTool + " settled=" + (pane.curPage
                                  && !pane.curPage.pageTab.mergeToolsLoading)
                              + " loading=" + toolField.loading + " open=" + toolField.popup.opened
                              + " typing=" + toolField.typing
                              + " choices=" + pane.toolChoices.length)
    }
    onToolChoicesChanged: pane.reportTool()
    onMergeToolChanged: {
        // `screenOpen`, not `showing`: see the property. A box that stopped following in the other category would be
        // carrying a value git has already moved past, and `applyTool` would write it back on the way out.
        if (pane.screenOpen && !pane.toolTouched) {
            toolField.wanted = pane.mergeTool
            pane.toolTouched = false
        }
        pane.reportTool()
    }
    Connections {
        target: pane.curPage ? pane.curPage.pageTab : null
        function onMergeToolsLoadingChanged() {
            if (AppBackend.autoAct === "settings-tools-loading" && pane.curPage.pageTab.mergeToolsLoading)
                pane.autoToolLoadingLatched = true
        }
    }

    // ---- line endings ----------------------------------------------------
    // Written the moment a row is picked, the way every field on this screen but the identity is (規約 §設定の画面):
    // one key, so there is nothing for a Save to hold together. The model reads itself back afterwards, so what the
    // chooser shows is always what git holds.
    //
    // **The read is asked for when the screen opens**, not when this category shows: it is one `git config`, the same
    // order of cost as the identity read beside it, and the eight-second one (`--tool-help`) is the only thing on
    // this screen that waits for its own chapter.
    function loadEndings() {
        // No path: the user's own configuration is what git resolves outside any repository, which is where this
        // level lives (`LineEndingsModel.look`).
        globalEndings.look("")
    }
    /// The global read has answered, one way or the other. **Not `state === "ready"`** — this level is read out of
    /// whatever configuration the machine running the verb happens to have, and a run held until it answered
    /// *well* would wait out the watchdog on a machine where git cannot resolve one at all.
    readonly property bool autoEndingsAnswered: globalEndings.state === "ready"
                                                || globalEndings.state === "error"
    /// Smoke hook. Output side throughout — `held` is what git answered with, and the row the chooser is showing is
    /// what a photograph cannot tell apart from one nobody has filled in yet.
    function reportEndings() {
        AppBackend.report("line_endings scope=global state=" + globalEndings.state
                          + " held=" + globalEndings.held
                          + " shown=" + endingsField.words[endingsField.heldRow]
                          + " busy=" + globalEndings.busy
                          + " error=" + globalEndings.error)
    }

    // ---- the identity ----------------------------------------------------
    // Nothing here waits on the answer. The gate keeps a `saving` flag because a landed save is what closes it; this
    // screen was not opened to answer that one question, so it stays standing either way and the marks the fields
    // carry are the whole of what a save has to say here (`IdentityFields`).
    function loadIdentity() {
        identityFields.load()
    }
    function focusIdentity() {
        identityFields.focusName()
    }
    function submitIdentity() {
        if (!identityActions.acceptEnabled)
            return
        AppBackend.saveIdentity(identityFields.nameText, identityFields.emailText)
    }

    // ---- what the repository group answers for, forwarded ------------------
    // The second group lives in `SettingsRepoPane`; the window's harness asks the screen, the screen asks this, and
    // this passes it on — one more link than the tools have, and the same shape.
    readonly property bool autoRepoReady: repoPane.autoRepoReady
    readonly property bool autoRepoComboOpen: repoPane.autoRepoComboOpen
    readonly property int autoRepoRows: repoPane.autoRepoRows
    function autoOfferRepos() {
        repoPane.autoOfferRepos()
    }
    function autoShowRepoAt(at) {
        return repoPane.showRepoAt(at)
    }
    function reportRepo() {
        repoPane.reportRepo()
    }
    readonly property bool autoRepoEndingsReady: repoPane.autoEndingsReady
    readonly property string autoRepoEndingHeld: repoPane.autoEndingHeld
    function autoPickRepoEnding(value) {
        return repoPane.autoPickEnding(value)
    }
    function reportRepoEndings() {
        repoPane.reportEndings()
    }
    /// Lands the second group on the repository the reader is looking at, for the screen that just opened.
    function landOnFront() {
        repoPane.landOnFront()
    }

    Layout.fillWidth: true
    spacing: Theme.spaceXl

    LineEndingsModel {
        id: globalEndings
        scope: "global"
    }
    // A write here is the value the group below *inherits*, so the sentence under its chooser — what git is doing in
    // that repository right now — has just moved. Nothing else on the screen would notice: the two chapters read
    // different files, and only this one of them changed.
    Connections {
        target: globalEndings
        function onWrote() {
            repoPane.rereadEndings()
        }
    }

    SettingsGroup {
        caption: qsTr("GLOBAL")

        // Where this group's values live, said before its chapters rather than in a line at the foot — and said in
        // `warning`, because a write here reaches outside this window (規約 §状態 / §設定の画面). Both chapters
        // write with `--global` (`identity` / `conflict` in core), so what is being replaced is the configuration
        // every repository on this account is read through — the one kind of reach the state colours are for. Not
        // `fontSm`: the application category's line is help text, this one is the warning.
        Label {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            color: Theme.warning
            text: qsTr("Saved to your global git configuration, replacing what is set there. Every other git on this computer, in every repository, reads the same values.")
        }

        SettingsSection {
            caption: qsTr("IDENTITY")
            IdentityFields {
                id: identityFields
                // How far the write reaches is the warning's line above; this one is left with the pair of keys, the
                // way the merge editor's line names `merge.guitool`.
                note: qsTr("Written as user.name and user.email.")
                onSubmitted: pane.submitIdentity()
            }
            // The two keys cannot be written in one go (core.md), so this chapter keeps the button that asks for
            // them. Everything else on the screen writes as it is finished with, which is why the screen's own way
            // out is an OK and not a Save.
            DialogActions {
                id: identityActions
                acceptKind: "check"
                acceptText: AppBackend.identityBusy ? qsTr("Saving…") : qsTr("Save")
                acceptEnabled: !AppBackend.identityBusy && identityFields.filled
                onAccepted: pane.submitIdentity()
            }
        }

        SettingsSection {
            caption: qsTr("MERGE EDITOR")
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceSm
                // As wide as the chapter gives it: a tool's name has no fixed length, and an input only takes a fixed
                // width when its content does (デザイン規約 §レイアウト初期値).
                AppCombo {
                    id: toolField
                    Layout.fillWidth: true
                    placeholder: qsTr("none")
                    // The popup is opened by the screen's own `opened` edge. Loading stays latched only for the
                    // automation verb that deliberately photographs it.
                    loading: pane.autoToolLoadingLatched || (pane.curPage && pane.curPage.pageTab.mergeToolsLoading)
                    model: pane.toolChoices
                    onWantedChanged: pane.toolTouched = true
                    // A row picked from the list is a finished answer; free text waits for Enter or for the screen to
                    // close.
                    onActivated: pane.applyTool()
                    onAccepted: pane.accepted()
                }
            }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                color: Theme.textMuted
                font.pixelSize: Theme.fontSm
                // Named rather than picked from a list: the only way to enumerate them is `git mergetool
                // --tool-help`, whose output is laid out for a person to read.
                text: qsTr("Which tool opens a conflicted file. It must not need a console — this app gives git none, so vimdiff and its kind cannot run. Stored by git as merge.guitool.")
            }
        }

        SettingsSection {
            caption: qsTr("LINE ENDINGS")
            LineEndingField {
                id: endingsField
                held: globalEndings.held
                ready: globalEndings.state === "ready"
                busy: globalEndings.busy
                errorText: globalEndings.error
                // How far the write reaches is the warning's line above; this one is left with the key, the way the
                // merge editor's line names `merge.guitool`.
                note: qsTr("Stored by git as core.autocrlf. What a repository sets for itself stands over it.")
                onPicked: value => globalEndings.save(value)
            }
        }
    }

    SettingsGroup {
        caption: qsTr("REPOSITORY OVERRIDE")
        SettingsRepoPane {
            id: repoPane
            tabsModel: pane.tabsModel
            screenOpen: pane.screenOpen
            onAccepted: pane.accepted()
        }
    }
}
