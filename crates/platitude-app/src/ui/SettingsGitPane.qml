import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// The settings screen's `Git` category: the two things this app writes into git's own configuration, and the sentence
// that says so (規約 §設定の画面). A file of its own because it is the half of the screen that talks to git — it asks
// git what it would launch, hands git the identity, and nothing in it is Platitude GG's to keep.
ColumnLayout {
    id: pane

    /// The tab git is read through. The settings are global; "what would git launch here" is not.
    property var curPage: null
    /// This category is the one on screen, and the screen is open. What the slow candidate read waits for — a reader
    /// who opened the application category never asked for it.
    property bool showing: false

    /// Enter in the tool box. The way out belongs to the screen, so it is asked for rather than taken.
    signal accepted()

    onShowingChanged: if (pane.showing) pane.askTools()

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
    /// Puts the box back to what git says, for the screen that just opened.
    function loadTool() {
        toolField.wanted = pane.mergeTool
        pane.toolTouched = false
        pane.autoToolLoadingLatched = false
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
        if (pane.showing && !pane.toolTouched) {
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

    // ---- the identity ----------------------------------------------------
    /// A write is in flight that this screen asked for. The notification is shared with the startup check, so nothing
    /// here reads an answer it did not ask for.
    property bool saving: false
    function loadIdentity() {
        identityFields.load()
    }
    function focusIdentity() {
        identityFields.focusName()
    }
    function submitIdentity() {
        if (!identityActions.acceptEnabled)
            return
        pane.saving = true
        AppBackend.saveIdentity(identityFields.nameText, identityFields.emailText)
    }
    Connections {
        target: AppBackend
        function onIdentityChanged() {
            if (!pane.saving || AppBackend.identityBusy)
                return
            // Nothing closes: this screen was not opened to answer that one question, so a landed save leaves it
            // standing with the marks the fields carry (`IdentityFields`).
            pane.saving = false
        }
    }

    Layout.fillWidth: true
    spacing: Theme.spaceXl

    // Where this category's values live, said before the chapters rather than in a line at the foot — and said in
    // `warning`, because a write here reaches outside this window (規約 §状態 / §設定の画面). Both chapters write
    // with `--global` (`identity` / `conflict` in core), so what is being replaced is the configuration every
    // repository on this account is read through — the one kind of reach the state colours are for. Not `fontSm`:
    // the application category's line is help text, this one is the warning.
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
            // How far the write reaches is the warning's line above; this one is left with the pair of keys, the way
            // the merge editor's line names `merge.guitool`.
            note: qsTr("Written as user.name and user.email.")
            onSubmitted: pane.submitIdentity()
        }
        // The two keys cannot be written in one go (core.md), so this chapter keeps the button that asks for them.
        // Everything else on the screen writes as it is finished with, which is why the screen's own way out is an OK
        // and not a Save.
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
            // Named rather than picked from a list: the only way to enumerate them is `git mergetool --tool-help`,
            // whose output is laid out for a person to read.
            text: qsTr("Which tool opens a conflicted file. It must not need a console — this app gives git none, so vimdiff and its kind cannot run. Stored by git as merge.guitool.")
        }
    }
}
