import QtQuick
import QtQuick.Dialogs
import QtQuick.Layouts
import platitude
import platitude.ui

// The settings screen's `Application` category: everything Platitude GG keeps in its own settings file, and the
// sentence that says so (規約 §設定の画面). A file of its own for the reason the other two categories have one — a
// category is one place values are stored, and this one's place is that file.
ColumnLayout {
    id: pane

    /// The tab the avatar candidates are read off: the authors of the repository being looked at. The settings
    /// themselves are the application's, but "whose commits are these" is read where the person is.
    property var curPage: null

    /// The author an avatar's badge was pressed on, handed down by the screen — which is where the entry puts it.
    property string prefillName: ""
    property string prefillEmail: ""

    /// Enter in one of the number fields. The way out belongs to the screen, so it is asked for.
    signal accepted()

    /// Stands in for the pointer on one row's Remove, which headless cannot inject.
    property int pointedAtRow: -1

    /// What the avatar verbs wait on, and the two moves they have no hand to make. The screen is the window's, so
    /// those verbs are finished by `WindowAutoActDriver`; everything here is the list's own output side — the rows the
    /// store answered the filing with, the picture inside the first of them, that row's `lit`, and the candidate
    /// list's own `opened`.
    readonly property int autoAvatarRows: avatarRepeater.count
    readonly property bool autoAvatarComboOpen: avatarWho.popup.opened
    /// Which chapters this category is actually showing, for the verb that presses the rail — the screen's `category`
    /// is the input side, and a run that read it back would be reporting its own press.
    readonly property bool autoAppShown: fetchField.visible
    /// The version git printed is in a field, so a reader can drag over it and take it away — **the one chip in this
    /// window that is a value** (`CodeChip.grabbable`). A picture cannot say it: a chip that answers a press is drawn
    /// exactly like one that does not.
    readonly property bool autoVersionGrabbed: gitVersionChip.grabbed
    function autoAvatarRowLit(at) {
        const row = avatarRepeater.itemAt(at)
        return !!row && row.lit
    }
    function autoAvatarRowPainted(at) {
        const row = avatarRepeater.itemAt(at)
        return !!row && row.pictureReady()
    }
    /// The door a press uses, so the list comes down the way it does under a hand — including the turn of the loop the
    /// field puts between the press and the list (`AppCombo.pressField`).
    function autoAvatarOfferCombo() {
        avatarWho.pressField()
    }
    /// Runs the row's hold to its end. False where the list has no such row yet, so the caller waits
    /// for it.
    function autoAvatarHoldRemove(at) {
        const row = avatarRepeater.itemAt(at)
        if (!row)
            return false
        row.holdRemove()
        return true
    }

    /// Assignments as the settings file holds them: address, name, URL — U+001F between records, U+001E between a
    /// record's fields, the same convention every packed list carries (`encode::RECORD_SEP` / `FIELD_SEP`).
    readonly property var assigned: {
        const packed = AppBackend.avatars
        if (packed === "")
            return []
        return packed.split(String.fromCharCode(31)).map(record => {
            const parts = record.split(String.fromCharCode(30))
            return { email: parts[0], name: parts[1] || parts[0], url: parts[2] }
        })
    }

    /// Who the entry offers: the authors of the repository being looked
    /// at. Read when the screen opens, because the graph
    /// keeps moving and a list that reordered itself under an open popup
    /// would be answering a question nobody asked.
    property var authorChoices: []
    function readAuthorChoices() {
        // The model dedupes, sorts and formats (`GraphModel.author_choices`); the prefill rides along so its address
        // shows once. With no page there are no rows to offer — the prefill still leads the list.
        const packed = pane.curPage
                     ? pane.curPage.pageGraph.authorChoices(pane.prefillName, pane.prefillEmail)
                     : pane.prefillEmail !== ""
                       ? pane.prefillName + " <" + pane.prefillEmail + ">" : ""
        pane.authorChoices = packed === "" ? [] : packed.split(String.fromCharCode(31))
    }

    /// The address the picker will file under, pulled back out of what the
    /// entry is showing. The list writes `Name <address>`; a person typing
    /// their own may write either half, and an address is the one with an
    /// `@` in it (`platitude_core::trailers::split_identity`).
    readonly property string chosenEmail: GitFacts.identityEmailOf(avatarWho.wanted)
    readonly property string chosenName: GitFacts.identityNameOf(avatarWho.wanted)

    /// The version git printed, in git's own spelling — the chip at the head of the line. Empty where git printed
    /// none, which is every answer that is not a version.
    readonly property string gitPathChip:
        AppBackend.gitPathState === "ok" || AppBackend.gitPathState === "old"
        ? "git " + AppBackend.gitPathVersion : ""
    /// What the git at the path said, as one sentence — empty where the chip above says the whole of it. The words
    /// are here (app-ui.md「Rust に文言を置かない」): what comes across is which of the four
    /// answers it was (`version::Probe`), and the only string passed through is git's or the OS's own.
    ///
    /// **The two `missing` sentences are different questions.** An empty box means the git on PATH, and "not on
    /// PATH" is what the startup gate says about it in those words; a path that was written down is a file that is
    /// not there, and telling the reader about PATH would send them to fix the wrong thing.
    readonly property string gitPathWord: {
        // **The warning takes the version in.** A chip alone above a paragraph reads
        // as a lead-in nobody finished; the version is the subject of what the warning has to say, so it is the
        // subject of the sentence. Two whole wordings, the way the line-ending
        // rows are written (規約 §設定の画面) — the old one has a second fact to carry and a joined-on clause reads
        // as an afterthought.
        if (AppBackend.gitPathOffersRestart) {
            return AppBackend.gitPathState === "old"
                 ? qsTr("is older than the %1 this app is built for — restart to use it anyway.")
                   .arg(AppBackend.minimumGit)
                 : qsTr("is not running yet — restart to switch to it.")
        }
        switch (AppBackend.gitPathState) {
        case "checking":
            return qsTr("Asking for the version…")
        case "ok":
            // Nothing beside the chip: with nothing else on the line the version is the whole answer, and a
            // sentence built round it would be saying "that git is" about the only thing there.
            return ""
        case "old":
            // What the `OLD GIT` badge's card says, less the version the chip is already carrying
            // (規約 §git が無い時・古い時). One spelling for the one fact, so a reader who has seen the badge reads
            // this line as the same thing said again.
            return qsTr("is older than the %1 this app is built for.").arg(AppBackend.minimumGit)
        case "missing":
            return AppBackend.gitPath === "" ? qsTr("git was not found on PATH.")
                                             : qsTr("Nothing to run at that path.")
        case "failed":
            return AppBackend.gitPathError
        }
        return ""
    }
    /// `warning` while the line is the one that costs the window it is read in, and for a git that answers and is
    /// old (nothing is stopped); `danger` for one that did not answer at all; and the help text's own ink for the
    /// rest — a version that came back is ordinary (規約 §状態: the state colours are for "this is not a state you
    /// can work in", and a settings screen is otherwise all ordinary).
    readonly property color gitPathInk: {
        if (AppBackend.gitPathOffersRestart)
            return Theme.warning
        switch (AppBackend.gitPathState) {
        case "old":
            return Theme.warning
        case "missing":
        case "failed":
            return Theme.danger
        }
        return Theme.textSecondary
    }

    /// Whether the button stands, frozen for the length of a press (規約 §フル interactive rebase, and the standing
    /// list of buttons that do not freeze: P3-確認事項 §app).
    ///
    /// **The offer moves on git's clock.** A version comes back, or a second one is asked for, and
    /// the offer can fall under a hand already holding — which would take the button out from under it mid-hold, and
    /// `ActionButton` reads `holdMs` again at the release besides. Frozen, the press finishes on the button it began
    /// on and `restart_now` decides whether it still means anything.
    ///
    /// Declared with the live value so it reads right from the start; the `Binding` is what keeps it, dropped while a
    /// press is under way with nothing put back after (`RestoreNone`) — which is the freeze itself.
    property bool gitButtonRestarts: AppBackend.gitPathOffersRestart
    /// What the button wears: `warning` for a git that answers, `danger` where that git is below the supported
    /// minimum. **The button alone goes red** — the line above it stays `warning`, because being old is a fact about
    /// the git. Read off the frozen shape, so the button stays put under a hand
    /// already holding it.
    readonly property color gitButtonTone:
        AppBackend.gitPathState === "old" ? Theme.danger : Theme.warning
    /// A gesture is under way. The pointer's is `down`; the keyboard's is the climbing fill — an armed hold
    /// takes Space itself (`HoldDriver.pressKey`), which is what puts it there.
    readonly property bool gitButtonPressing: gitButton.down || gitButton.holdProgress > 0
    Binding {
        target: pane
        property: "gitButtonRestarts"
        value: AppBackend.gitPathOffersRestart
        when: !pane.gitButtonPressing
        restoreMode: Binding.RestoreNone
    }

    /// Everything the git box is holding, in one reading: what it would write, what the store has, and what the
    /// binary at the end of it said. **None of it can be read off a picture** — a path is drawn the same whether or
    /// not anything is at the end of it, and the answer arrives a subprocess after the box does. An automation-only
    /// exposure, the same one `GraphPane.view` is (app-ui.md).
    function gitPathTally() {
        // **The four flags run together, ahead of the state.** Each row of the verb table claims a run of them and a
        // row's line is one substring (`verify::verbs`), so a `state=` in the middle would cut every claim in two.
        // Which of the two answers a version was is the machine's business anyway — a container on the supported
        // minimum and a desk on the newest both say `answered=true`.
        return "shown=" + gitPathField.text + " stored=" + AppBackend.gitPath
             + " answered=" + (AppBackend.gitPathState === "ok" || AppBackend.gitPathState === "old")
             + " offers=" + AppBackend.gitPathOffersRestart
             + " held=" + pane.gitButtonRestarts
             + " restart=" + AppBackend.restartWanted
             + " state=" + AppBackend.gitPathState
             + " version=" + AppBackend.gitPathVersion
             + " inUse=" + AppBackend.gitPathInUse
    }
    /// Opens the chooser where a git already is (`gitPicker`). The path in the box wins, because that is the one the
    /// reader is working on; with the box empty it is the placeholder's, which is the git this run spawns.
    function openGitPicker() {
        const near = GitFacts.folderUrlOf(gitPathField.text !== "" ? gitPathField.text
                                                                  : AppBackend.gitPathInUse)
        if (near !== "")
            gitPicker.currentFolder = near
        gitPicker.open()
    }

    /// The door a run types through, so the box is written the way a hand leaves it: the text, then the edit being
    /// finished with. Writing `AppBackend.gitPath` straight would photograph the wiring cut (規約 §UI 自動化の因果性).
    function autoTypeGitPath(path) {
        gitPathField.text = path
        pane.applyGitPath()
    }

    /// The door a run types the process chapter through: both boxes, each the way a hand leaves it — the text, then
    /// the edit being finished with. Either half empty is what an emptied box means: the default, and off.
    function autoTypeProcesses(concurrency, copiesSecs) {
        concurrencyField.text = concurrency
        pane.applyConcurrency()
        copiesField.text = copiesSecs
        pane.applyCopies()
    }
    /// What the store holds for that chapter, as the run reports it: the count the slots are set to and whether it is
    /// this machine's default (the number itself is the machine's — a third of its threads), and the interval in both
    /// units, since the tick reads the milliseconds and the box the seconds.
    function processesTally() {
        return "concurrency=" + AppBackend.gitConcurrency
               + " default=" + (AppBackend.gitConcurrency === AppBackend.gitConcurrencyDefault)
               + " copies_secs=" + AppBackend.copiesIntervalSecs + " copies_ms=" + AppBackend.copiesIntervalMs
    }

    /// Puts the fields back to what the store says, for the screen that just opened.
    function load() {
        fetchField.text = AppBackend.autoFetchMinutes > 0 ? String(AppBackend.autoFetchMinutes) : ""
        concurrencyField.text = String(AppBackend.gitConcurrency)
        copiesField.text = AppBackend.copiesIntervalSecs > 0 ? String(AppBackend.copiesIntervalSecs) : ""
        wholeHistoryBox.checked = AppBackend.initialCommits === 0
        commitsField.text = AppBackend.initialCommits > 0 ? String(AppBackend.initialCommits) : ""
        gitPathField.text = AppBackend.gitPath
        // The screen is open: the path is asked for its version, and nothing said here counts as a way out until it
        // closes. One `git --version`, which is the cheapest read this app makes (the eight-second one lives in the
        // other category, and asks only from the chapter that shows it).
        AppBackend.openGitPathScreen()
        pane.readAuthorChoices()
    }
    /// Opened from an avatar, the first thing left to do is name the
    /// picture, so the caret goes there. **The only caret this screen
    /// places as it opens** (`SettingsDialog.onOpened`) — a category is
    /// read before it is answered, so the caret belongs to this door.
    function focusPrefill() {
        if (pane.prefillEmail === "") {
            avatarWho.wanted = ""
            avatarWho.editText = ""
            return
        }
        const who = pane.prefillName + " <" + pane.prefillEmail + ">"
        avatarWho.wanted = who
        avatarWho.editText = who
        chooseAvatar.forceActiveFocus()
    }

    // An empty field is the off switch — nothing to type is the
    // clearest way to say "off".
    function applyFetch() {
        AppBackend.setAutoFetchMinutes(fetchField.text === "" ? 0 : Number(fetchField.text))
    }
    // An empty field is the default here — zero git at once is meaningless — and
    // the placeholder is that number. Written from here for the reason the interval is: a validator with a floor
    // calls an empty string unacceptable, so `onEditingFinished` never fires for one.
    function applyConcurrency() {
        AppBackend.setGitConcurrency(concurrencyField.text === "" ? 0 : Number(concurrencyField.text))
    }
    // The same off switch the fetch interval has: empty is never.
    function applyCopies() {
        AppBackend.setCopiesIntervalSecs(copiesField.text === "" ? 0 : Number(copiesField.text))
    }
    // Two controls, one value: the box asks for no window at all (core's `0`), and the field answers only while the
    // box is clear. Written from here for the same reason the interval is — a validator with a floor calls an empty
    // string unacceptable, so `onEditingFinished` never fires for one, and an emptied field would otherwise never be
    // written back. Empty is the default; the placeholder is that number.
    function applyCommits() {
        AppBackend.setInitialCommits(wholeHistoryBox.checked ? 0
                                     : commitsField.text === "" ? AppBackend.initialCommitsDefault
                                     : Number(commitsField.text))
    }
    // The path is written as it is finished with, and asking the binary is part of writing it: the reader is told
    // what is at the end of what they just typed, in the same breath. Empty is an answer — whichever git `PATH`
    // resolves — so there is nothing to guard against here.
    function applyGitPath() {
        AppBackend.setGitPath(gitPathField.text)
    }
    /// All of this category's fields, for the way out of the screen. The git path is written like the rest of them —
    /// **putting it in force is the button's own errand** (`AppBackend.applyGitPathNow`): a window that went down
    /// because somebody closed a settings screen would be a restart nobody pressed.
    function applyFields() {
        pane.applyFetch()
        pane.applyConcurrency()
        pane.applyCopies()
        pane.applyCommits()
        pane.applyGitPath()
    }

    Layout.fillWidth: true
    spacing: Theme.spaceXl

    // Where this category's values live, said before the chapters. The other two
    // categories say the same about themselves in their own files, and between them those sentences are the whole of
    // the difference between the categories.
    HelpText {
        text: qsTr("Kept by Platitude GG in its own settings file. Nothing here is written to your git configuration.")
    }

    // First of this category's chapters, because it is the one every other thing on the screen runs through: which
    // binary answers is upstream of how often it is asked and how much it is asked for. It also stands where the
    // category's own sentence has just said "nothing here is written to your git configuration" — which is exactly
    // what tells this chapter apart from the `Git` category next door (規約 §設定の画面).
    SettingsSection {
        caption: qsTr("GIT EXECUTABLE")
        HelpText {
            // The first sentence is the term of art every other tool names this by (規約 §設定の画面).
            text: qsTr("Path to the git executable this app runs, in every repository. Empty is the one found on PATH, which the box names.")
        }
        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceSm
            // **The box is the chooser.** The answer to "which git" is a file the reader can point at: typed, one
            // letter wrong names a binary that is not there. Pressing it opens the
            // dialog — and it keeps the ground a typing box has, which is the one exception to §選ぶ欄と打つ欄 this
            // screen carries: bare between two grounded boxes it read as disabled.
            //
            // As wide as the chapter gives it: a path has no fixed length, and an input only takes a fixed width
            // when its content does (デザイン規約 §レイアウト初期値).
            FormField {
                id: gitPathField
                Layout.fillWidth: true
                choosing: true
                // What an empty box will be read as, shown — the same job the commit count's
                // placeholder does with its number. **The resolved path**: "the git on PATH"
                // is not somewhere a reader can go and look, and the whole point of this chapter is that a machine
                // may hold several (規約 §設定の画面).
                placeholderText: AppBackend.gitPathInUse
                onPicking: pane.openGitPicker()
            }
            // **The one button here is the way to put a git in force**, and it stands only while there is one to
            // put — choosing is the box's own errand now, and a button that opened the same dialog beside it would
            // be a second door to one room. `warning`, and held, because the press costs the
            // window it is made in (規約 §設定の画面 / §長押し). The word carries the colour as well as the frame,
            // the way `push -f` does: what changes is what the press costs.
            ActionButton {
                id: gitButton
                visible: pane.gitButtonRestarts
                implicitHeight: Theme.controlHeight
                text: qsTr("Restart to apply")
                tone: pane.gitButtonTone
                // Framed, for the reason the avatar chooser is: it stands in a form row beside a
                // framed box (規約 §肯定側のボタン). The frame takes the word's own colour, which is what says the
                // press costs something.
                frameColor: pane.gitButtonTone
                holdMs: Metrics.holdMs
                activeFocusOnTab: true
                onHeld: AppBackend.applyGitPathNow()
            }
        }
        // What the git at that path said, in the slot the identity chapter says "a commit made there now would be
        // attributed to …" in: the explanation goes above the box, what is true now goes below it
        // (規約 §設定の画面). **A path cannot be checked by looking at it** — this line is the whole of the
        // difference between a box that works and a box with a typo in it.
        //
        // **The version wears the chip** (規約 §git 用語のコード表記): what git printed is git's own spelling, and
        // a line that opens with a lowercase `git` and nothing to mark it reads as a sentence with a typo at the
        // front. The rest of the sentence is ordinary text beside it.
        //
        // **Only ever one line's worth of it.** A chip and a wrapping paragraph in one row hang the second and third
        // lines off the chip's right edge, and a chip cannot flow inside wrapping
        // text without giving up its dress (its ground is measured from the word's step
        // — 同 §). So what stands beside the chip is one sentence, and anything more is the paragraph below.
        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs
            visible: pane.gitPathChip !== "" || pane.gitPathWord !== ""
            CodeChip {
                id: gitVersionChip
                visible: pane.gitPathChip !== ""
                word: pane.gitPathChip
                tint: pane.gitPathInk
                // The one chip in this window that is a value rather than a dress: it is git's own spelling of the
                // version the chosen binary answered with, and it is what a reader copies into a bug report
                // (規約 §設定の画面). Nothing else wants the press here — the sentence beside it is a field of its
                // own and the chapter's controls are elsewhere.
                grabbable: true
                // On the first line — the tail may wrap to two.
                Layout.alignment: Qt.AlignTop
            }
            CardText {
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignTop
                visible: pane.gitPathWord !== ""
                color: pane.gitPathInk
                pixelSize: Theme.fontMd
                text: pane.gitPathWord
            }
        }
        // The one thing the line above cannot carry, at the column's own margin: what a restart costs. Being told to
        // restart with nowhere to leave to, a reader's question is what they lose — and the answer is nothing, so it
        // is one short line (the chip cannot flow inside wrapping text, so the sentence it heads is kept to one line
        // and anything more comes here). Comes and goes with the button.
        CardText {
            Layout.fillWidth: true
            visible: AppBackend.gitPathOffersRestart
            color: Theme.warning
            pixelSize: Theme.fontMd
            text: qsTr("The window comes back with the same repositories open.")
        }

        FileDialog {
            id: gitPicker
            title: qsTr("Choose git")
            // Where the chooser opens. **Beside a git**: on a
            // machine with more than one git installed — which is the whole premise of this chapter — the nearest
            // useful place to start is where the one already in hand lives. The box's own path if it holds one,
            // otherwise the one behind the placeholder, which is the git this run spawns.
            //
            // Set on the way in, the same shape the repository chooser uses
            // (`Main.qml.openRepositoryPicker`): a dialog that re-homed itself while somebody was walking a tree
            // would take the folder they had got to away from them.
            // Every file is offered: a git is an `.exe` on one of the three platforms and has no extension on the
            // other two, and a wrapper script is a normal thing to point at. The version this writes back says
            // whether the choice was a git — filtering by name would only be guessing at it.
            onAccepted: {
                gitPathField.text = GitFacts.pickedPath(selectedFile.toString())
                pane.applyGitPath()
            }
        }
    }

    // **The rest of the category is held while a git waits to be applied** (規約 §設定の画面). The screen lets go
    // once the reader answers the box above; letting them go on setting other things meanwhile would be
    // offering work whose window is about to be replaced — and one of them (the graph's window) is read by the very
    // session that is going. `enabled` reaches the whole chapter, so the words go to the disabled step with the
    // boxes (§無効).
    SettingsSection {
        enabled: !AppBackend.gitPathOffersRestart
        caption: qsTr("AUTOMATIC FETCH")
        // Over the field: a sentence about what a chapter does belongs before the thing it describes,
        // which is the shape the group and category sentences already keep (規約 §設定の画面).
        HelpText {
            text: qsTr("Runs git fetch --prune on every open repository, at most once per interval. Empty means off; %1 minutes is the longest interval.")
                  .arg(AppBackend.autoFetchMaxMinutes)
        }
        LabeledField {
            caption: qsTr("Interval")
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceSm
                FormField {
                    id: fetchField
                    implicitWidth: 160
                    placeholderText: qsTr("off")
                    inputMethodHints: Qt.ImhDigitsOnly
                    validator: IntValidator {
                        bottom: 1
                        top: AppBackend.autoFetchMaxMinutes
                    }
                    // Written when the field is done with — on Enter, and
                    // on the focus leaving it. Per keystroke,
                    // it would run through "1" on the way to "10".
                    onEditingFinished: pane.applyFetch()
                    onAccepted: pane.accepted()
                }
                CardText {
                    text: qsTr("minutes")
                    color: Theme.textSecondary
                }
                Item { Layout.fillWidth: true }
            }
        }
    }

    // What the machine is asked to do at once and in the background: how many git commands may run together, and how
    // often the other working copies of a repository are read for uncommitted work. Beside the fetch interval because
    // they are the same kind of answer — about this machine and the person at it
    // (規約 §設定の画面).
    SettingsSection {
        enabled: !AppBackend.gitPathOffersRestart
        caption: qsTr("GIT PROCESSES")
        HelpText {
            text: qsTr("How many git commands run at once, in every repository. Whatever you are waiting on goes first and always finds room: the reads made in the background, and anything waiting on a network or another program, never take more than a quarter between them. %1 is the most.")
                  .arg(AppBackend.gitConcurrencyMax)
        }
        LabeledField {
            caption: qsTr("At once")
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceSm
                FormField {
                    id: concurrencyField
                    implicitWidth: 160
                    // What an empty field will be read as, shown.
                    placeholderText: String(AppBackend.gitConcurrencyDefault)
                    inputMethodHints: Qt.ImhDigitsOnly
                    validator: IntValidator {
                        bottom: 1
                        top: AppBackend.gitConcurrencyMax
                    }
                    onEditingFinished: pane.applyConcurrency()
                    onAccepted: pane.accepted()
                }
                CardText {
                    text: qsTr("commands")
                    color: Theme.textSecondary
                }
                Item { Layout.fillWidth: true }
            }
        }
        HelpText {
            text: qsTr("The repository's other working copies are read for uncommitted work on a tick of their own — one git status per copy. Empty means never; %1 seconds is the shortest interval.")
                  .arg(AppBackend.copiesIntervalMin)
        }
        LabeledField {
            caption: qsTr("Other copies every")
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceSm
                FormField {
                    id: copiesField
                    implicitWidth: 160
                    placeholderText: qsTr("never")
                    inputMethodHints: Qt.ImhDigitsOnly
                    validator: IntValidator {
                        bottom: AppBackend.copiesIntervalMin
                        top: AppBackend.copiesIntervalMax
                    }
                    onEditingFinished: pane.applyCopies()
                    onAccepted: pane.accepted()
                }
                CardText {
                    text: qsTr("seconds")
                    color: Theme.textSecondary
                }
                Item { Layout.fillWidth: true }
            }
        }
    }

    SettingsSection {
        enabled: !AppBackend.gitPathOffersRestart
        caption: qsTr("COMMIT GRAPH")
        HelpText {
            text: qsTr("How much history a graph opens with. The rest is loaded from the row at the bottom of it, a quarter of this at a time — with the whole history there is no such row. %1 is the smallest window.")
                  .arg(AppBackend.initialCommitsMin)
        }
        LabeledField {
            caption: qsTr("Initial commits")
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceSm
                FormField {
                    id: commitsField
                    implicitWidth: 160
                    // The number does not apply while the whole history is asked for, and §無効 is how that is said.
                    // The text stays put, so unchecking gives the reader their own number back.
                    enabled: !wholeHistoryBox.checked
                    // What an empty field will be read as, shown.
                    placeholderText: String(AppBackend.initialCommitsDefault)
                    inputMethodHints: Qt.ImhDigitsOnly
                    // The only ceiling is the property's own type (`session::log_limit`), so a number a person
                    // typed on purpose is one the walk answers.
                    validator: IntValidator {
                        bottom: AppBackend.initialCommitsMin
                    }
                    onEditingFinished: pane.applyCommits()
                    onAccepted: pane.accepted()
                }
                CardText {
                    text: qsTr("commits")
                    color: commitsField.enabled ? Theme.textSecondary : Theme.textMuted
                }
                Item { Layout.fillWidth: true }
            }
        }
        // The other answer a count of commits can have, and it is a yes or no — so it wears a box. An
        // empty field would have had to carry it, and an empty field already means "the default" in the box right
        // above (規約 §設定の画面).
        AppCheckBox {
            id: wholeHistoryBox
            text: qsTr("Load the whole history")
            onToggled: pane.applyCommits()
        }
    }

    SettingsSection {
        enabled: !AppBackend.gitPathOffersRestart
        caption: qsTr("AVATARS")
        HelpText {
            text: qsTr("An avatar for anyone whose commits you read. The list offers this repository's authors. Nothing is fetched: it comes from one of your own files, copied in beside these settings.")
        }
        Repeater {
            id: avatarRepeater
            /// The one column every row's name is laid into, as wide
            /// as the widest of them, so the address beside it starts
            /// on the same x down the whole list and the chapter reads
            /// as a table. The
            /// same shape `AppMenu.codeColW` uses for its chips.
            ///
            /// Settled by the rows' own seat changes:
            /// the binding form only re-ran on `count`, and a same-size
            /// reassignment replaces every row without moving it.
            property real nameColW: 0
            function settleNameColW() {
                let widest = 0
                for (let i = 0; i < avatarRepeater.count; i++) {
                    const row = avatarRepeater.itemAt(i)
                    if (row && row.nameSeat !== undefined)
                        widest = Math.max(widest, row.nameSeat)
                }
                nameColW = widest
            }
            onItemRemoved: settleNameColW()
            model: pane.assigned
            // One row per assignment (`AvatarAssignRow`); the shared name column and the pointer stand-in
            // are handed down, the rest the row reads off its own record.
            delegate: AvatarAssignRow {
                Layout.fillWidth: true
                nameColW: avatarRepeater.nameColW
                pointedAtRow: pane.pointedAtRow
                onNameSeatChanged: avatarRepeater.settleNameColW()
            }
        }
        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceSm
            AppCombo {
                id: avatarWho
                Layout.fillWidth: true
                placeholder: qsTr("name or email")
                model: pane.authorChoices
            }
            ActionButton {
                id: chooseAvatar
                implicitHeight: Theme.controlHeight
                // One word for the whole feature, and it is the word
                // this shelf already uses (デザイン規約 §アバターを
                // 与える). Bare noun: the row names a kind about to be
                // chosen (§長さ).
                text: qsTr("Choose avatar…")
                // A plain frame: bare is for the
                // answer standing beside a framed one, read as the
                // pair it is in, and this one stands in a form row
                // next to a combo (規約 §肯定側のボタン). Opened from
                // an avatar, what says which errand this is is the
                // combo beside it, already
                // carrying that name.
                frameColor: Theme.borderDefault
                activeFocusOnTab: true
                enabled: pane.chosenEmail !== ""
                onActivated: avatarPicker.open()
            }
        }
        CardText {
            Layout.fillWidth: true
            visible: AppBackend.avatarErrorKind !== ""
            color: Theme.danger
            pixelSize: Theme.fontSm
            // The words are this side's, off the kind core answered with — the numbers it takes come with it, and
            // the two failures the operating system made carry its own line under ours (`Words.avatarFailure`).
            text: Words.avatarFailure(AppBackend.avatarErrorKind,
                                      AppBackend.avatarErrorFacts,
                                      AppBackend.avatarErrorSaid)
        }
    }

    FileDialog {
        id: avatarPicker
        title: qsTr("Choose avatar")
        // The patterns come from the store so the dialog and the store
        // cannot drift apart; the words in front of them are ours, so
        // they live here (CLAUDE.md 文言規約). They
        // name the two formats, because every
        // category word for a file is a second name for the avatar
        // (デザイン規約 §アバターを与える) — and the formats are pinned
        // by that same section, so `avatar::EXTENSIONS` cannot grow one
        // this line does not know about without the rules moving first.
        nameFilters: [qsTr("PNG and JPEG (%1)").arg(AppBackend.avatarPatterns)]
        onAccepted: AppBackend.assignAvatar(pane.chosenEmail, pane.chosenName, selectedFile.toString())
    }
}
