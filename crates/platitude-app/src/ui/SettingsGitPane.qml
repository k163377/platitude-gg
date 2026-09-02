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
// `REPOSITORY OVERRIDE` is what the repository picked below writes into its own file, standing over the first. Only
// the identity is asked at both levels. The merge editor is `GLOBAL`'s alone (`conflict::set_merge_tool`: which
// editor someone reaches for is a property of their desk), and the line endings are the repository's alone — **this
// app does not write `core.autocrlf` outside a repository somebody picked**, because the machine's own configuration
// is not an application's to rewrite (規約 §設定の画面).
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
            pane.holdToolLoading = false
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
    /// Keeps the box's turning indicator up past the read that raised it. Written from outside and false wherever
    /// nobody wrote it: the read can be over before a picture of the wait has been grabbed. Put down by the screen
    /// closing (`onShowingChanged`) rather than by whoever raised it, so that neither of the open edge's two handlers
    /// nor their order can take away what the other just put there.
    property bool holdToolLoading: false
    /// The candidate list is down, and nothing is outstanding on the tool read. What a run waits on, either side of
    /// the answer: an automation-only exposure, the same one `GraphPane.view` is (app-ui.md).
    readonly property bool toolListOpen: toolField.popup.opened
    readonly property bool toolsSettled: pane.curPage !== null && !pane.curPage.pageTab.mergeToolsLoading

    /// Asks git what it would launch, and what it could launch. The candidate read is `git mergetool --tool-help`,
    /// which is about eight seconds on Windows (規約 §conflict を外部ツールへ渡す) — hence the turning indicator, and
    /// hence asking only from the chapter that shows the answer.
    function askTools() {
        if (!pane.curPage)
            return
        // The status refresh only names the configured tool where something is conflicted, so ask for it.
        pane.curPage.pageTab.askMergeTool()
        pane.curPage.pageTab.askMergeTools()
        pane.toolsAsked()
    }
    /// git has just been asked both questions. An automation-only exposure, the same one `GraphPane.view` is
    /// (app-ui.md): whether the read is still out is already on [`toolsSettled`], but *this instant* is not — the
    /// answer can land before anything that reads the pane runs again.
    signal toolsAsked()

    // The tool is written only where it would change what git answers with. Pressing the field is not choosing
    // anything, and the write for "the same as now" is not free: an empty field asks git to unset a key, which fails
    // when the key was never there in the first place (observed — the log raised itself over the
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

    /// The whole of what the box is holding, in one reading: what it would write, what it is showing, what git says
    /// is configured, whether the read has settled, and where the caret is. **None of it can be read off a picture**
    /// — the names are short and the box is the same width whatever is in it — and the candidates arrive in two waves
    /// with the configured name in a third, so the value has three chances to be knocked out by something that is not
    /// a person. An automation-only exposure, the same one `GraphPane.view` is (app-ui.md).
    function toolTally() {
        return "wanted=" + toolField.wanted + " shown=" + toolField.editText
             + " configured=" + pane.mergeTool + " settled=" + pane.toolsSettled
             + " loading=" + toolField.loading + " open=" + pane.toolListOpen
             + " typing=" + toolField.typing
             + " choices=" + pane.toolChoices.length
    }
    onMergeToolChanged: {
        // `screenOpen`, not `showing`: see the property. A box that stopped following in the other category would be
        // carrying a value git has already moved past, and `applyTool` would write it back on the way out.
        if (pane.screenOpen && !pane.toolTouched) {
            toolField.wanted = pane.mergeTool
            pane.toolTouched = false
        }
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
    /// Automation: leaves this group's name box holding an edit, so the way out has something to stop for.
    function autoTypeIdentity(text) {
        identityFields.autoTypeName(text)
    }
    function submitIdentity() {
        if (!identityActions.acceptEnabled)
            return
        AppBackend.saveIdentity(identityFields.nameText, identityFields.emailText)
    }

    // ---- what the way out has to stop for -------------------------------
    /// How many identity chapters are holding an edit git has not been given. **The only thing on this screen a
    /// reader can lose by leaving** — every other field writes as it is finished with, and these two keep a Save
    /// because git cannot be handed both of their keys at once (規約 §設定の画面). Counted rather than answered
    /// yes/no so the question can say which of the two it is about.
    readonly property int unsavedIdentities: (identityFields.dirty ? 1 : 0) + (repoPane.unsaved ? 1 : 0)
    /// Which group the question is about, for its sentence. Both is possible; the word then names neither.
    readonly property bool unsavedIsGlobal: identityFields.dirty
    readonly property bool unsavedIsRepo: repoPane.unsaved
    /// The boxes a refused way out puts the reader in front of. The group's own chapter first, since it is the one
    /// higher up the screen; null when neither is holding anything.
    function unsavedIdentityItem() {
        if (identityFields.dirty)
            return identitySection
        return repoPane.unsaved ? repoPane.identityItem() : null
    }
    /// Puts both back to what git holds, for the reader who said to leave them behind. The way out runs this rather
    /// than just closing, so a screen opened again does not come back still holding the edit that was discarded.
    function dropUnsavedIdentities() {
        identityFields.load()
        repoPane.reloadIdentity()
    }

    /// The second group, handed over whole. **Automation-only exposure**, the same one `GraphPane.view` is
    /// (app-ui.md): everything a run asks about the repository chooser and its line endings is that group's answer,
    /// and passing each question through here would be one more link that says nothing (`WindowSettingsActs`).
    readonly property alias autoRepoPane: repoPane
    /// Lands the second group on the repository the reader is looking at, for the screen that just opened.
    function landOnFront() {
        repoPane.landOnFront()
    }

    Layout.fillWidth: true
    spacing: Theme.spaceXl

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
            id: identitySection
            caption: qsTr("IDENTITY")
            IdentityFields {
                id: identityFields
                // How far the write reaches is the warning's line above; this one is left with the pair of keys, the
                // way the merge editor's line names `merge.guitool`.
                note: qsTr("Written as user.name and user.email.")
                onSubmitted: pane.submitIdentity()
            }
            // The two keys cannot be written in one go (core.md), so this chapter keeps the button that asks for
            // them. Everything else on the screen writes as it is finished with, which is why the screen itself has
            // no button that writes — its only way out is the `✕`.
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
            HelpText {
                // Named rather than picked from a list: the only way to enumerate them is `git mergetool
                // --tool-help`, whose output is laid out for a person to read.
                text: qsTr("Which tool opens a conflicted file. It must not need a console — this app gives git none, so vimdiff and its kind cannot run. Stored by git as merge.guitool.")
            }
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
                    loading: pane.holdToolLoading || (pane.curPage && pane.curPage.pageTab.mergeToolsLoading)
                    model: pane.toolChoices
                    onWantedChanged: pane.toolTouched = true
                    // A row picked from the list is a finished answer; free text waits for Enter or for the screen to
                    // close.
                    onActivated: pane.applyTool()
                    onAccepted: pane.accepted()
                }
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
