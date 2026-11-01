import QtQuick
import QtQuick.Layouts
import platitude
import platitude.ui

// The settings screen's `Git` category: what this app writes into git's own configuration, in two groups by how far a
// value reaches — `GLOBAL`, and `REPOSITORY OVERRIDE` standing over it (規約 §設定の画面). One category, not two: both
// are git's settings and differ only in reach (user decision — ask before changing). A chapter may stand in both
// groups (`IDENTITY`) or in one: `MERGE EDITOR` is global only, `LINE ENDINGS` repository only (`LineEndingField`).
//
// The group names follow other git GUIs (the four looked at all say "global", and give the narrow side as "this
// repository overrides global"). The narrow one names the relation itself, `REPOSITORY OVERRIDE`: `local` does not say
// local to what. The pair is lopsided on purpose — the second stands on the first.
ColumnLayout {
    id: pane

    /// The tab git is read through. Settings are global; "what git launches here" is this tab's.
    property var curPage: null
    /// The strip, for the group that writes into one repository (`SettingsRepoPane`).
    required property TabsModel tabsModel
    /// This category is on screen and the screen is open — what the slow candidate read waits for.
    property bool showing: false
    /// The screen is open, whichever category is showing — what the boxes follow git by. git can answer while the
    /// other category is up (in a conflicted tree the status read writes `mergeTool` unasked), and a box that stopped
    /// following would be written back stale on close (`applyTool`) — an empty one taking `merge.guitool` out of the
    /// global file.
    property bool screenOpen: false

    /// Enter in the tool box; the screen owns the way out.
    signal accepted()

    // The latch is cleared on the way down, not in `loadTool`: both run off the open edge, in no set order.
    onShowingChanged: {
        if (pane.showing)
            pane.askTools()
        else
            pane.holdToolLoading = false
    }

    // ---- the merge editor ------------------------------------------------
    readonly property string mergeTool: pane.curPage ? pane.curPage.pageWorktree.mergeTool : ""
    /// Names to offer (`RepoTab.mergeTools`).
    readonly property var toolChoices: pane.curPage ? pane.curPage.pageTab.mergeTools : []
    /// Stops a late answer from overwriting something already typed.
    property bool toolTouched: false
    /// Keeps the box's turning indicator up past the read, which can end before a picture of the wait is grabbed.
    /// Written from outside; put down by the screen closing (`onShowingChanged`).
    property bool holdToolLoading: false
    /// Automation: the candidate list is down / the tool read has settled.
    readonly property bool toolListOpen: toolField.popup.opened
    readonly property bool toolsSettled: pane.curPage !== null && !pane.curPage.pageTab.mergeToolsLoading
    /// Automation: the candidate list's middle-button hand, and how far it has sent the rows.
    readonly property alias toolListHand: toolField.listHand
    readonly property alias toolListAt: toolField.listAt
    /// Automation: Enter in the tool box, and the list taken back down. Offscreen delivers no key, so the box's own
    /// `accepted` is raised — the signal Qt raises on Enter (`tst_appcombo`).
    function autoEnterTool() {
        toolField.accepted()
    }
    function autoShutToolList() {
        toolField.popup.close()
    }

    /// Asks git what it would launch and what it could. The candidate read (`--tool-help`) is slow
    /// (規約 §conflict を外部ツールへ渡す), so only the showing category asks: who opened `Application` did not ask
    /// for candidates.
    function askTools() {
        if (!pane.curPage)
            return
        // The status refresh only names the configured tool where something is conflicted, so ask for it.
        pane.curPage.pageTab.askMergeTool()
        pane.curPage.pageTab.askMergeTools()
        pane.toolsAsked()
    }
    /// Automation: git has just been asked. An edge, since the answer can land before anything reads
    /// [`toolsSettled`] again.
    signal toolsAsked()

    // Written only when it differs: an empty field unsets the key, which fails when it was never set. Headless:
    // `settings-tools`.
    function applyTool() {
        if (pane.curPage && toolField.wanted !== pane.mergeTool)
            pane.curPage.pageTab.setMergeTool(toolField.wanted)
    }
    /// Puts the box back to what git says, for the screen that just opened.
    function loadTool() {
        toolField.wanted = pane.mergeTool
        pane.toolTouched = false
    }
    /// The door a press uses, so the list comes down the way it does under a hand (`AppCombo.pressField`).
    function pressToolField() {
        toolField.pressField()
    }

    /// Automation: the box's whole state in one reading. None of it can be read off a picture, and the value has three
    /// arrivals (two candidate waves, then the configured name) that could knock it out.
    function toolTally() {
        return "wanted=" + toolField.wanted + " shown=" + toolField.editText
             + " configured=" + pane.mergeTool + " settled=" + pane.toolsSettled
             + " loading=" + toolField.loading + " open=" + pane.toolListOpen
             + " typing=" + toolField.typing
             + " choices=" + pane.toolChoices.length
    }
    onMergeToolChanged: {
        // `screenOpen`, not `showing`: see the property.
        if (pane.screenOpen && !pane.toolTouched) {
            toolField.wanted = pane.mergeTool
            pane.toolTouched = false
        }
    }

    // ---- the identity ----------------------------------------------------
    // Nothing here waits on the save: unlike the gate, which a landed save closes, the fields' marks say it all
    // (`IdentityFields`).
    function loadIdentity() {
        identityFields.load()
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
    /// Automation: the Save's own `enabled`, read off the button rather than worked out from the boxes.
    readonly property alias autoSaveOffered: identityActions.acceptEnabled

    // ---- what the way out has to stop for -------------------------------
    /// How many identity chapters hold an edit git has not been given — the only thing leaving can lose
    /// (`SettingsDialog.escapeOut`).
    readonly property int unsavedIdentities: (identityFields.dirty ? 1 : 0) + (repoPane.unsaved ? 1 : 0)
    /// Automation: which group holds the unsaved edit (both is possible).
    readonly property bool unsavedIsGlobal: identityFields.dirty
    readonly property bool unsavedIsRepo: repoPane.unsaved
    /// The boxes a refused way out shows: the global chapter first, being higher up; null when neither holds an edit.
    function unsavedIdentityItem() {
        if (identityFields.dirty)
            return identitySection
        return repoPane.unsaved ? repoPane.identityItem() : null
    }
    /// Puts both back to what git holds, for a way out that leaves the edits behind.
    function dropUnsavedIdentities() {
        identityFields.load()
        repoPane.reloadIdentity()
    }

    /// Automation: the second group, handed over whole (rules-refs/app-ui.md「製品の部品はハーネスへ答える相手を丸ごと渡す」).
    readonly property alias autoRepoPane: repoPane
    /// Lands the second group on the repository the reader is looking at, for the screen that just opened.
    function landOnFront() {
        repoPane.landOnFront()
    }

    Layout.fillWidth: true
    spacing: Theme.spaceXl

    SettingsGroup {
        caption: qsTr("GLOBAL")

        // In `warning`, the one warning-coloured sentence of `Git` (user decision — ask before changing): both chapters
        // write `--global`, reaching every repository on the computer, beyond this window. The repository group's line
        // stays help text — warning on both would warn on neither. It says three things (user decision): it writes the
        // global file, it replaces what is there, and every repository on this computer reads the same value.
        CardText {
            Layout.fillWidth: true
            color: Theme.warning
            text: qsTr("Saved to your global git configuration, replacing what is set there. Every other git on this computer, in every repository, reads the same values.")
        }

        SettingsSection {
            id: identitySection
            caption: qsTr("IDENTITY")
            IdentityFields {
                id: identityFields
                // Only the keys: the reach is the warning's line above.
                note: qsTr("Written as user.name and user.email.")
                onSubmitted: pane.submitIdentity()
            }
            // Its own Save, the screen's one exception to writing on a finished edit: git cannot take both keys in one
            // go (`identity::set_identity`). It stands in the chapter — at the screen's foot it would read "save this
            // screen", a lie about every other chapter. Lit by an edit (`IdentityFields.dirty`) and, unlike the
            // repository chapter's, by both boxes being filled — the global pair has nothing to fall back to.
            DialogActions {
                id: identityActions
                acceptKind: "check"
                acceptText: AppBackend.identityBusy ? qsTr("Saving…") : qsTr("Save")
                acceptEnabled: !AppBackend.identityBusy && identityFields.filled && identityFields.dirty
                onAccepted: pane.submitIdentity()
            }
        }

        // Global only: which editor opens is a property of the desk, not of a repository (`conflict::set_merge_tool`).
        // The heading names the box, so no caption (`LabeledField`). That the setting is git's is said by the category
        // it stands in; naming `merge.guitool` at the end of the help is an index for whoever comes looking from git's
        // side. Any tool keeping the merge tool's contract (`$LOCAL` / `$REMOTE` / `$BASE` in, `$MERGED` written) is
        // taken; running without a console (`CREATE_NO_WINDOW`) is the one constraint, and the words name only that.
        SettingsSection {
            caption: qsTr("MERGE EDITOR")
            HelpText {
                text: qsTr("Which tool opens a conflicted file. It must not need a console — this app gives git none, so vimdiff and its kind cannot run. Stored by git as merge.guitool.")
            }
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceSm
                AppCombo {
                    id: toolField
                    Layout.fillWidth: true
                    placeholder: qsTr("none")
                    // A read settled on no tool at all (`AppCombo.hasList` false) cannot be photographed on a desk:
                    // `--tool-help` always names one, so that picture takes a throwaway patch emptying
                    // `conflict::available_tools`.
                    loading: pane.holdToolLoading || (pane.curPage && pane.curPage.pageTab.mergeToolsLoading)
                    model: pane.toolChoices
                    onWantedChanged: pane.toolTouched = true
                    // A picked row is a finished answer; free text waits for Enter or the close. `submitted`, not
                    // `accepted`: Qt raises `accepted` under the open list too, and would shut the screen mid-choice
                    // (`settings-tools-enter`).
                    onActivated: pane.applyTool()
                    onSubmitted: pane.accepted()
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
