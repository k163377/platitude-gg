import QtQuick
import QtQuick.Layouts
import platitude
import platitude.ui

// The `REPOSITORY OVERRIDE` group of the settings screen's `Git` category: which repository, and what it writes into
// its own configuration (規約 §設定の画面). A file of its own: a whole group with models behind it. Only the tab in
// front has a session (`RepoPageStack`), so reads and writes go through models handed a worktree path
// (`RepoConfigModel`).
ColumnLayout {
    id: pane

    /// The strip: the chooser offers its repositories under the tabs' own names (`tab_name::names_for`).
    required property TabsModel tabsModel
    /// The screen is open, whichever category is showing — what the boxes follow git by (`SettingsGitPane.screenOpen`).
    property bool screenOpen: false

    /// Enter in one of the boxes.
    signal accepted()

    // ---- which repository ------------------------------------------------
    /// One `{name, path}` per tab (`TabsModel::settle_open_repos`).
    readonly property var openRepos: pane.tabsModel.openRepos
    readonly property var repoNames: pane.openRepos.map(repo => repo.name)

    /// Shows the repository at `at` in the strip; false while there is none.
    function showRepoAt(at) {
        const repo = pane.openRepos[at]
        if (!repo)
            return false
        chooser.wanted = repo.name
        pane.touched = false
        repoConfig.look(repo.path)
        repoEndings.look(repo.path)
        return true
    }
    /// Lands on the repository the reader is looking at. Only as the screen opens, so a chosen repository stays chosen.
    function landOnFront() {
        pane.showRepoAt(pane.tabsModel.currentIndex)
    }

    // ---- the identity ----------------------------------------------------
    /// Typed since the boxes were last filled from git; keeps a late answer from overwriting the edit.
    property bool touched: false
    /// An edit somebody made that this repository's file has not been given — what the way out stops for and the Save
    /// is lit by. Guarded on the read: before it lands, an empty box looks the same as no answer.
    readonly property bool unsaved: repoConfig.state === "ready" && identityFields.dirty
    /// The whole chapter, heading included, for the way out to scroll to.
    function identityItem() {
        return identitySection
    }
    /// Puts the boxes back to what this repository's file holds, touch and all, so git's next answer fills them.
    function reloadIdentity() {
        pane.touched = false
        identityFields.load()
    }

    function submitIdentity() {
        if (!identityActions.acceptEnabled)
            return
        // The read that follows the save fills the boxes again.
        pane.touched = false
        repoConfig.save(identityFields.nameText, identityFields.emailText)
    }

    // ---- automation ------------------------------------------------------
    /// What the verbs wait on, output side only: the read has landed, the list is open, the rows it offers.
    readonly property bool autoRepoReady: repoConfig.state === "ready"
    readonly property bool autoRepoComboOpen: chooser.popup.opened
    readonly property int autoRepoRows: pane.repoNames.length
    /// The door a press uses, so the list comes down the way it does under a hand (`AppCombo.pressField`).
    function autoOfferRepos() {
        chooser.pressField()
    }
    /// The line-ending read has landed and nothing is out, so the row showing is one git named.
    readonly property bool autoEndingsReady: repoEndings.state === "ready" && !repoEndings.busy
    /// What the repository's own file holds now, in git's spelling — what a run waits on after a pick, since the
    /// write's `busy` falls before the re-read lands.
    readonly property string autoEndingHeld: repoEndings.held
    /// What the repository held before the run picked; a photograph cannot tell whether the pick moved anything.
    property string endingsWere: ""
    /// Picks the row for `value` the way the list does (`LineEndingField.pick`); false when there is no such row.
    function autoPickEnding(value) {
        const at = endingsField.rowOf(value)
        if (at < 0)
            return false
        pane.endingsWere = repoEndings.held
        endingsField.pick(at)
        return true
    }
    /// Automation: the line-ending chapter in one reading. `scope` is always local — `core.autocrlf` is written only
    /// into the picked repository (規約 §設定の画面).
    function endingsTally() {
        return "scope=local state=" + repoEndings.state
             + " were=" + pane.endingsWere
             + " held=" + repoEndings.held
             + " effective=" + repoEndings.effective
             // A photograph cannot check this: the rows' sentences look alike, and a stale one reads as right.
             + " shown=" + endingsField.words[endingsField.heldRow]
             + " busy=" + repoEndings.busy
             + " error=" + repoEndings.error
    }
    /// Automation: this chapter's Save, read off the button — its rule differs from the global chapter's
    /// (`SettingsGitPane.autoSaveOffered`): an empty box is an answer here.
    readonly property alias autoSaveOffered: identityActions.acceptEnabled
    /// Automation: the group in one reading — a photograph cannot tell whose values the boxes show. `matches=` is the
    /// plain comparison (`IdentityFields.differs`), `save=` the button's reading of it: different questions.
    function repoTally() {
        return "rows=" + pane.autoRepoRows
             + " state=" + repoConfig.state
             + " matches=" + !identityFields.differs
             + " save=" + pane.autoSaveOffered
             + " open=" + pane.autoRepoComboOpen
             + " local=" + repoConfig.localName + "|" + repoConfig.localEmail
             + " shown=" + identityFields.nameText + "|" + identityFields.emailText
             + " effective=" + repoConfig.effectiveName + "|" + repoConfig.effectiveEmail
             + " repo=" + chooser.wanted
    }

    Layout.fillWidth: true
    spacing: Theme.spaceXl

    RepoConfigModel {
        id: repoConfig
    }
    // The same repository's line endings — a model of its own, since the chapters ask git about different keys.
    LineEndingsModel {
        id: repoEndings
    }
    // git's answer fills the boxes until somebody types; every property arrives on the one signal, so the touch is
    // the guard.
    Connections {
        target: repoConfig
        function onChanged() {
            if (pane.screenOpen && !pane.touched)
                identityFields.load()
        }
    }

    // Help text, not `warning` like the group above: warning on both would say nothing on either (規約 §設定の画面).
    HelpText {
        text: qsTr("Saved in the chosen repository's own git configuration. What is set here stands over the values above, for that repository only.")
    }

    // Reachable with no repository open: the app menu opens the screen itself.
    HelpText {
        visible: pane.autoRepoRows === 0
        text: qsTr("No repository is open. These settings are about one repository, so there is nothing to show.")
    }

    LabeledField {
        visible: pane.autoRepoRows > 0
        caption: qsTr("Repository")
        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceSm
            // Picking only: this screen writes only to repositories already open (デザイン規約 §選ぶ欄と打つ欄).
            AppCombo {
                id: chooser
                Layout.fillWidth: true
                pickOnly: true
                model: pane.repoNames
                onActivated: index => pane.showRepoAt(index)
            }
        }
    }

    SettingsSection {
        id: identitySection
        visible: pane.autoRepoRows > 0
        caption: qsTr("IDENTITY")
        IdentityFields {
            id: identityFields
            heldName: repoConfig.localName
            heldEmail: repoConfig.localEmail
            // An empty box inherits the value from above.
            namePlaceholder: qsTr("inherited")
            emailPlaceholder: qsTr("inherited")
            nameMarked: repoConfig.writeUnsaved && repoConfig.writeNameSaved
            emailMarked: repoConfig.writeUnsaved && repoConfig.writeEmailSaved
            errorText: repoConfig.error !== ""
                       ? repoConfig.error
                       : repoConfig.writeUnsaved
                         ? qsTr("git still reports something else for this repository. Its configuration may have been written from elsewhere.")
                         : ""
            // The keys, and what an empty box means; the reach is the group's line above.
            note: qsTr("Written as user.name and user.email in that repository. An empty box is not written at all.")
            // The touch comes from `edited`, not a text change: a reload changes the text too (picking another
            // repository empties the boxes first, `RepoConfigModel::read_repo`), and would stop them following git —
            // leaving empty boxes that ask for the identity to be taken out.
            onEdited: pane.touched = true
            onSubmitted: pane.submitIdentity()
        }
        // What an empty box falls back to lives in files this screen does not show, so it is said here
        // (規約 §設定の画面).
        HelpText {
            visible: repoConfig.effectiveName !== "" && repoConfig.effectiveEmail !== ""
            text: qsTr("A commit made there now would be attributed to %1 <%2>.")
                  .arg(repoConfig.effectiveName).arg(repoConfig.effectiveEmail)
        }
        // Its own Save, as in the global group. No `filled`: empty is an answer here, and requiring both boxes would
        // refuse taking an override back out.
        DialogActions {
            id: identityActions
            acceptKind: "check"
            acceptText: repoConfig.writeBusy ? qsTr("Saving…") : qsTr("Save")
            acceptEnabled: !repoConfig.writeBusy && pane.unsaved
            onAccepted: pane.submitIdentity()
        }
    }

    SettingsSection {
        visible: pane.autoRepoRows > 0
        caption: qsTr("LINE ENDINGS")
        LineEndingField {
            id: endingsField
            held: repoEndings.held
            effective: repoEndings.effective
            ready: repoEndings.state === "ready"
            busy: repoEndings.busy
            errorText: repoEndings.error
            note: qsTr("Written as core.autocrlf in that repository. Inherited is not written there at all.")
            onPicked: value => repoEndings.save(value)
        }
    }
}
