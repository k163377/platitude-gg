import QtQuick
import QtQuick.Layouts
import platitude
import platitude.ui

// The `REPOSITORY OVERRIDE` group of the settings screen's `Git` category: what one repository writes into its own
// configuration file, and which repository that is (規約 §設定の画面).
//
// A file of its own because it is a whole group with a model behind it — the group above it is two
// chapters and a sentence, this one is a chooser, a chapter, and the reads and write that follow whichever repository
// was picked. The chooser stands above the chapter: it is what the setting
// below is about, so it wears a field's caption.
//
// **The repository is whichever one was picked.** Only the tab in front has a session (`RepoPageStack`), so the reads
// and the write go through a model that is handed a work tree path (`RepoConfigModel`).
ColumnLayout {
    id: pane

    /// The strip, for the chooser: it offers exactly the repositories standing in it, called what the tabs call them —
    /// including the parents a shared name grows (`tab_name::names_for`), so two repositories of one name are told
    /// apart here the same way they are up there.
    required property TabsModel tabsModel
    /// The screen is open, whichever category it is on. **What the boxes follow git by** — the same reason the git
    /// category follows it there (`SettingsGitPane.screenOpen`): a box that stopped following would be written back on
    /// close as though the reader had emptied it.
    property bool screenOpen: false

    /// Enter in one of the boxes.
    signal accepted()

    // ---- which repository ------------------------------------------------
    /// The strip, unpacked: one record per tab, the name it is shown by and then its work tree path
    /// (`TabsModel::settle_open_repos`).
    readonly property var openRepos: {
        const packed = pane.tabsModel.openRepos
        if (packed === "")
            return []
        return packed.split(String.fromCharCode(31)).map(record => {
            const parts = record.split(String.fromCharCode(30))
            return { name: parts[0], path: parts[1] }
        })
    }
    /// Just the names, which is what the list offers.
    readonly property var repoNames: pane.openRepos.map(repo => repo.name)

    /// Shows the repository standing at `at` in the strip. Answers whether there was one — the automation waits
    /// for one.
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
    /// Lands on the repository the reader is looking at. Called when the screen opens, and only then: the screen is
    /// answering "these settings, for this repository", and the one the reader is standing in is the answer to the
    /// second half until they say otherwise.
    function landOnFront() {
        pane.showRepoAt(pane.tabsModel.currentIndex)
    }

    // ---- the identity ----------------------------------------------------
    /// Something has been typed since the boxes were last filled from git. What keeps a late answer — the read that
    /// follows a save, or a `changed` for something else entirely — from overwriting it.
    property bool touched: false
    /// The boxes hold an edit **somebody made** that this repository's file has not been given: what the way out of
    /// the screen stops for, and **what the Save is lit by** (`IdentityFields.dirty`). Guarded on the read as well:
    /// before git has answered there is nothing to disagree with, and an empty box is the same shape as a read that
    /// has not landed.
    readonly property bool unsaved: repoConfig.state === "ready" && identityFields.dirty
    /// The whole chapter, for the way out that puts the reader in front of what is holding it — the heading and its
    /// rule included, since a column landing on the boxes alone arrives with the chapter's name already off the top.
    function identityItem() {
        return identitySection
    }
    /// Puts the boxes back to what this repository's file holds, for the reader who chose to leave an edit behind.
    /// The touch goes with it — what is in the boxes is git's again, so the next answer from git may fill them.
    function reloadIdentity() {
        pane.touched = false
        identityFields.load()
    }

    function submitIdentity() {
        if (!identityActions.acceptEnabled)
            return
        // Written back by the read that follows, so the boxes stop being the reader's and start being git's again.
        pane.touched = false
        repoConfig.save(identityFields.nameText, identityFields.emailText)
    }

    // ---- automation ------------------------------------------------------
    /// What the verbs wait on: the read has landed, and the list's own `opened`. Output side throughout — the
    /// category the screen is on is the input side and a run that read it back would be reporting its own press.
    readonly property bool autoRepoReady: repoConfig.state === "ready"
    readonly property bool autoRepoComboOpen: chooser.popup.opened
    readonly property int autoRepoRows: pane.repoNames.length
    /// The door a press uses, so the list comes down the way it does under a hand (`AppCombo.pressField`).
    function autoOfferRepos() {
        chooser.pressField()
    }
    /// The line-ending read has landed and nothing is out, so the row showing is one git named.
    readonly property bool autoEndingsReady: repoEndings.state === "ready" && !repoEndings.busy
    /// What that repository's own file holds now, in git's spelling. **What a run waits on after a pick** — the
    /// write's own `busy` falls before the read that follows it lands, so a run that stopped there would report the
    /// value it had just replaced.
    readonly property string autoEndingHeld: repoEndings.held
    /// What that repository held before the run picked — the half of the claim a photograph cannot hold, since a
    /// chooser showing the picked row looks the same whether the pick moved anything or not.
    property string endingsWere: ""
    /// Picks the row for `value` through the same call a pick from the list makes (`LineEndingField.pick`). Answers
    /// whether the list holds such a row, so a run waits for one.
    function autoPickEnding(value) {
        const at = endingsField.rowOf(value)
        if (at < 0)
            return false
        pane.endingsWere = repoEndings.held
        endingsField.pick(at)
        return true
    }
    /// The line-ending chapter in one reading. An automation-only exposure, the same one `GraphPane.view` is
    /// (app-ui.md) — the values are the model's and the chooser's, and neither is on this pane as a property.
    ///
    /// `scope=local` is the whole of the chapter: this screen writes `core.autocrlf` into the repository
    /// somebody picked (規約 §設定の画面).
    function endingsTally() {
        return "scope=local state=" + repoEndings.state
             + " were=" + pane.endingsWere
             + " held=" + repoEndings.held
             + " effective=" + repoEndings.effective
             // The row the chooser is actually showing. **A photograph cannot check it** — the words are sentences of
             // the same length and shape, and one left over from another repository reads exactly like the one git
             // named.
             + " shown=" + endingsField.words[endingsField.heldRow]
             + " busy=" + repoEndings.busy
             + " error=" + repoEndings.error
    }
    /// This chapter's Save, read off the button. An automation-only exposure, the same one `GraphPane.view` is
    /// (app-ui.md), and the same one the group above keeps (`SettingsGitPane.autoSaveOffered`): **the two chapters
    /// are lit by deliberately different rules** — this one takes an empty box as an answer — so each says whether
    /// its own button is live rather than a run working either out from the boxes.
    readonly property alias autoSaveOffered: identityActions.acceptEnabled
    /// The group in one reading, the same way and for the same reason. The boxes say what git holds: **a photograph
    /// cannot check it** — they belong to whichever repository was picked last, and ones showing another
    /// repository's values, or none at all, is a screen offering to write the wrong thing. `matches=` is the plain
    /// comparison (`IdentityFields.differs`) and `save=` is what the button makes of it, which is not the same
    /// question: boxes that came from git match *and* leave the Save dark, and only one of the two says so.
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
    // The same repository's line-ending setting, read and written into the same file. A model of its own: the two
    // chapters ask git about different keys, and this is the only chapter
    // on the screen that writes `core.autocrlf` at all (`LineEndingsModel`).
    LineEndingsModel {
        id: repoEndings
    }
    // What git answers with is what the boxes say, until somebody types. Every property arrives on the one signal, so
    // the guard is the reader's touch (`SettingsGitPane.toolTouched` is the same shape).
    Connections {
        target: repoConfig
        function onChanged() {
            if (pane.screenOpen && !pane.touched)
                identityFields.load()
        }
    }

    // Where this group's values live, said before its chapter — and as help text. The group above wears `warning`
    // because it replaces what every git on the computer reads; this one reaches one repository, which is the
    // reader's own choice and undone by emptying a box. A colour that says "careful" on both would stop saying
    // anything on either.
    HelpText {
        text: qsTr("Saved in the chosen repository's own git configuration. What is set here stands over the values above, for that repository only.")
    }

    // Nothing to choose from and nothing to set. The screen is reachable with no repository open, because the app
    // menu's one row opens the screen itself (規約 §設定の画面).
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
            // Picking only: the list is the whole set of answers, because the only repositories this screen can write
            // to are the ones already open (デザイン規約 §選ぶ欄と打つ欄). As wide as the chapter gives it — a
            // repository's name has no fixed length once a namesake has grown it a parent.
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
            // What an empty box gets you: this pair has somewhere to fall
            // back to, and the box standing empty is how a reader asks for it.
            namePlaceholder: qsTr("inherited")
            emailPlaceholder: qsTr("inherited")
            nameMarked: repoConfig.writeUnsaved && repoConfig.writeNameSaved
            emailMarked: repoConfig.writeUnsaved && repoConfig.writeEmailSaved
            errorText: repoConfig.error !== ""
                       ? repoConfig.error
                       : repoConfig.writeUnsaved
                         ? qsTr("git still reports something else for this repository. Its configuration may have been written from elsewhere.")
                         : ""
            // How far the write reaches is the warning's line above; this one is left with the pair of keys, and with
            // what an empty box means — the one thing this chapter carries alone.
            note: qsTr("Written as user.name and user.email in that repository. An empty box is not written at all.")
            // **The touch comes from `edited`.** Filling the boxes from git changes their text too, so a reload
            // guarded on text would be told "the reader is typing" by its own reload — and the boxes would then stop
            // following git for good. Picking another repository empties them first (`RepoConfigModel::read_repo`),
            // which is exactly such a change, so the screen would sit with empty boxes over a repository that has an
            // identity — and an empty box asks for it to be taken out (measured, `settings-repo 0`).
            onEdited: pane.touched = true
            onSubmitted: pane.submitIdentity()
        }
        // What an empty box actually falls back to cannot be shown inside it: the value lives in a file this screen is
        // not showing, and git resolves it through more than one of them. So it is said out loud, as what a
        // commit made here would carry right now.
        HelpText {
            visible: repoConfig.effectiveName !== "" && repoConfig.effectiveEmail !== ""
            text: qsTr("A commit made there now would be attributed to %1 <%2>.")
                  .arg(repoConfig.effectiveName).arg(repoConfig.effectiveEmail)
        }
        // The two keys cannot be written in one go (core.md), so this chapter keeps the button that asks for them —
        // the same reason the git category's identity chapter has one.
        //
        // **Lit by an edit somebody made** (`unsaved`, which carries the read having landed). No `filled` here:
        // empty is an answer in this chapter, and asking for both boxes would refuse the very thing that takes an
        // override back out.
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
